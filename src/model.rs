//! The one executable job/attempt/effect model (R6).
//!
//! Every persisted job change is an [`Event`] applied by [`apply`]; the database
//! stores the resulting [`Job`] next to the append-only event log, so a stored
//! job can always be replayed from its events. Trace files under `tests/traces`
//! replay crash, duplicate, cancellation, uncertain-submission and
//! partial-import histories through the same function.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Accepted, never claimed.
    Queued,
    /// Between or inside local steps; claimable when `lease` is empty.
    Active,
    /// External provider work outstanding; holds no executor capacity.
    Waiting,
    Succeeded,
    Failed,
    Cancelled,
}

impl Phase {
    pub fn terminal(self) -> bool {
        matches!(self, Phase::Succeeded | Phase::Failed | Phase::Cancelled)
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::Queued => "queued",
            Phase::Active => "active",
            Phase::Waiting => "waiting",
            Phase::Succeeded => "succeeded",
            Phase::Failed => "failed",
            Phase::Cancelled => "cancelled",
        }
    }
}

/// State of the current external submission (one possibly paid effect).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectState {
    /// No submission outstanding; a dispatch is allowed.
    None,
    /// Dispatch intent persisted; the provider may or may not have the request.
    Intended,
    /// Provider receipt (operation ID) recorded.
    Submitted,
    /// The submission reply was lost; reconcile before any replay.
    Uncertain,
    Done,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancelState {
    None,
    Requested,
    ProviderConfirmed,
    LocallyStopped,
    Uncertain,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub phase: Phase,
    pub method: String,
    pub steps: Vec<String>,
    pub step: usize,
    pub fence: i64,
    pub lease: Option<String>,
    pub attempts: u32,
    pub effect: EffectState,
    pub effect_id: Option<String>,
    pub op_id: Option<String>,
    pub submissions: u32,
    pub outputs: Option<usize>,
    pub imported: BTreeSet<String>,
    pub cancel: CancelState,
    /// Provider-reported charge; `None` stays unknown, explained by `usage_note`.
    pub usage_credits: Option<u64>,
    pub usage_note: Option<String>,
    pub reason: Option<String>,
    pub manifest: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Accepted {
        method: String,
        steps: Vec<String>,
    },
    Claimed {
        owner: String,
        fence: i64,
    },
    /// The lease holder vanished (crash or expiry); work becomes claimable.
    LeaseExpired,
    LeaseReleased {
        fence: i64,
    },
    StepCompleted {
        fence: i64,
        step: usize,
    },
    DispatchIntended {
        fence: i64,
        effect_id: String,
    },
    SubmitAcked {
        fence: i64,
        effect_id: String,
        op_id: String,
    },
    SubmitUncertain {
        fence: i64,
        effect_id: String,
        reason: String,
    },
    /// Provider lookup by our effect ID: `Some` adopts the operation, `None`
    /// means the provider confirms it never received the request.
    Reconciled {
        effect_id: String,
        op_id: Option<String>,
    },
    ProviderCompleted {
        op_id: String,
        outputs: usize,
        credits: Option<u64>,
    },
    ProviderFailed {
        op_id: String,
        reason: String,
        credits: Option<u64>,
    },
    ObservationUnknown {
        op_id: String,
        reason: String,
    },
    ObjectImported {
        fence: i64,
        key: String,
    },
    /// Release the lease with a reason (partial import, full store); imported
    /// objects stay and a later claim resumes.
    Paused {
        fence: i64,
        reason: String,
    },
    /// The provider definitively refused the request (nothing was created).
    SubmitRefused {
        fence: i64,
        effect_id: String,
        reason: String,
    },
    ManifestCommitted {
        fence: i64,
        items: usize,
    },
    Failed {
        fence: i64,
        reason: String,
    },
    CancelRequested,
    DeadlineExpired,
    CancelConfirmed {
        op_id: String,
    },
    CancelStoppedLocally,
    CancelUncertain {
        reason: String,
    },
}

impl Event {
    pub fn fence(&self) -> Option<i64> {
        match self {
            Self::LeaseReleased { fence }
            | Self::StepCompleted { fence, .. }
            | Self::DispatchIntended { fence, .. }
            | Self::SubmitAcked { fence, .. }
            | Self::SubmitUncertain { fence, .. }
            | Self::ObjectImported { fence, .. }
            | Self::Paused { fence, .. }
            | Self::SubmitRefused { fence, .. }
            | Self::ManifestCommitted { fence, .. }
            | Self::Failed { fence, .. } => Some(*fence),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Reject {
    /// The event carries an old fence or no lease: a stale executor.
    Stale,
    Invalid(String),
}

impl std::fmt::Display for Reject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Reject::Stale => write!(f, "stale fence"),
            Reject::Invalid(m) => write!(f, "invalid transition: {m}"),
        }
    }
}

fn bad<T>(m: impl Into<String>) -> Result<T, Reject> {
    Err(Reject::Invalid(m.into()))
}

pub const MAX_STEPS: usize = 16;
pub const STEP_KINDS: &[&str] = &["synthetic.generate", "provider.submit", "provider.import", "validate"];

/// Apply one event. A missing state accepts only `Accepted`.
pub fn apply(state: Option<&Job>, event: &Event) -> Result<Job, Reject> {
    let Some(s) = state else {
        return match event {
            Event::Accepted { method, steps } => {
                if steps.is_empty() || steps.len() > MAX_STEPS {
                    return bad("steps must be 1..=16");
                }
                if let Some(k) = steps.iter().find(|k| !STEP_KINDS.contains(&k.as_str())) {
                    return bad(format!("unknown step kind {k}"));
                }
                Ok(Job {
                    phase: Phase::Queued,
                    method: method.clone(),
                    steps: steps.clone(),
                    step: 0,
                    fence: 0,
                    lease: None,
                    attempts: 0,
                    effect: EffectState::None,
                    effect_id: None,
                    op_id: None,
                    submissions: 0,
                    outputs: None,
                    imported: BTreeSet::new(),
                    cancel: CancelState::None,
                    usage_credits: None,
                    usage_note: Some("no provider usage observed".into()),
                    reason: None,
                    manifest: false,
                })
            }
            _ => bad("job does not exist"),
        };
    };
    let mut n = s.clone();
    let fenced = |fence: &i64| -> Result<(), Reject> { if *fence != s.fence || s.lease.is_none() { Err(Reject::Stale) } else { Ok(()) } };
    let op_matches = |op: &str| s.op_id.as_deref() == Some(op);
    let current = s.steps.get(s.step).map(String::as_str);

    if s.phase.terminal() {
        // Duplicate or late events are idempotent no-ops (R6).
        return match event {
            Event::ProviderCompleted { op_id, .. } | Event::ProviderFailed { op_id, .. } if op_matches(op_id) => Ok(n),
            Event::CancelRequested | Event::DeadlineExpired | Event::LeaseExpired => Ok(n),
            _ => bad(format!("job is {}", s.phase.as_str())),
        };
    }

    match event {
        Event::DeadlineExpired => {
            n = apply(Some(s), &Event::CancelRequested)?;
            n.reason = Some("execution deadline exceeded".into());
        }
        Event::Accepted { .. } => return bad("already accepted"),
        Event::Claimed { owner, fence } => {
            if s.lease.is_some() {
                return bad("lease held");
            }
            if *fence != s.fence + 1 {
                return bad("claim must advance the fence by one");
            }
            if s.phase == Phase::Waiting {
                return bad("waiting on the provider; pollers observe it without a lease");
            }
            n.fence = *fence;
            n.lease = Some(owner.clone());
            n.attempts += 1;
            if s.phase == Phase::Queued {
                n.phase = Phase::Active;
            }
        }
        Event::LeaseExpired => n.lease = None,
        Event::LeaseReleased { fence } => {
            fenced(fence)?;
            n.lease = None;
        }
        Event::StepCompleted { fence, step } => {
            fenced(fence)?;
            if *step != s.step || current.is_none() {
                return bad("step out of order");
            }
            if current == Some("provider.submit") {
                return bad("provider.submit completes through the provider");
            }
            if current == Some("provider.import") && Some(s.imported.len()) < s.outputs {
                return bad("import incomplete");
            }
            n.step += 1;
        }
        Event::DispatchIntended { fence, effect_id } => {
            fenced(fence)?;
            if current != Some("provider.submit") {
                return bad("current step is not provider.submit");
            }
            if s.effect != EffectState::None {
                // Never blindly repeat a possibly paid submission (R4).
                return bad(format!("effect is {:?}; reconcile first", s.effect));
            }
            if s.cancel != CancelState::None {
                return bad("cancel requested");
            }
            n.effect = EffectState::Intended;
            n.effect_id = Some(effect_id.clone());
            n.submissions += 1;
        }
        Event::SubmitAcked { fence, effect_id, op_id } => {
            fenced(fence)?;
            if s.effect != EffectState::Intended || s.effect_id.as_ref() != Some(effect_id) {
                return bad("no matching dispatch intent");
            }
            n.effect = EffectState::Submitted;
            n.op_id = Some(op_id.clone());
            n.phase = Phase::Waiting;
            n.lease = None; // release executor capacity during the provider wait
        }
        Event::SubmitUncertain { fence, effect_id, reason } => {
            fenced(fence)?;
            if s.effect != EffectState::Intended || s.effect_id.as_ref() != Some(effect_id) {
                return bad("no matching dispatch intent");
            }
            n.effect = EffectState::Uncertain;
            n.phase = Phase::Waiting;
            n.lease = None;
            n.reason = Some(reason.clone());
        }
        Event::Reconciled { effect_id, op_id } => {
            if !matches!(s.effect, EffectState::Intended | EffectState::Uncertain) || s.effect_id.as_ref() != Some(effect_id) {
                return bad("nothing to reconcile");
            }
            if s.lease.is_some() {
                return bad("reconcile only unleased work");
            }
            match op_id {
                Some(op) => {
                    n.effect = EffectState::Submitted;
                    n.op_id = Some(op.clone());
                    n.phase = Phase::Waiting;
                }
                None => {
                    n.effect = EffectState::None;
                    n.effect_id = None;
                    n.phase = Phase::Active;
                    if s.cancel == CancelState::Requested {
                        n.cancel = CancelState::LocallyStopped;
                        n.phase = Phase::Cancelled;
                    }
                }
            }
            n.reason = None;
        }
        Event::ProviderCompleted { op_id, outputs, credits } => {
            if s.effect == EffectState::Done && op_matches(op_id) {
                return Ok(n); // duplicate webhook or poll
            }
            if s.effect != EffectState::Submitted || !op_matches(op_id) {
                return bad("completion for an unknown operation");
            }
            if *outputs == 0 || *outputs > 16 {
                return bad("completion must have 1..=16 outputs");
            }
            n.effect = EffectState::Done;
            n.reason = None;
            n.outputs = Some(*outputs);
            set_usage(&mut n, *credits);
            if matches!(s.cancel, CancelState::Requested | CancelState::Uncertain) {
                // Work finished before the cancel reached the provider.
                n.phase = Phase::Cancelled;
                n.cancel = CancelState::LocallyStopped;
            } else {
                n.phase = Phase::Active;
                n.step += 1;
            }
        }
        Event::ProviderFailed { op_id, reason, credits } => {
            if s.effect == EffectState::Failed && op_matches(op_id) {
                return Ok(n);
            }
            if s.effect != EffectState::Submitted || !op_matches(op_id) {
                return bad("failure for an unknown operation");
            }
            n.effect = EffectState::Failed;
            set_usage(&mut n, *credits);
            n.phase = Phase::Failed;
            n.lease = None;
            n.reason = Some(reason.clone());
        }
        Event::ObservationUnknown { op_id, reason } => {
            if s.effect != EffectState::Submitted || !op_matches(op_id) || s.lease.is_some() {
                return bad("observation for an unknown or leased operation");
            }
            n.reason = Some(reason.clone());
        }
        Event::ObjectImported { fence, key } => {
            fenced(fence)?;
            if !matches!(current, Some("provider.import" | "synthetic.generate")) {
                return bad("current step does not import");
            }
            n.imported.insert(key.clone());
        }
        Event::Paused { fence, reason } => {
            fenced(fence)?;
            // Imported objects stay; no manifest; a later claim resumes (R6, R7).
            n.lease = None;
            n.reason = Some(reason.clone());
        }
        Event::SubmitRefused { fence, effect_id, reason } => {
            fenced(fence)?;
            if s.effect != EffectState::Intended || s.effect_id.as_ref() != Some(effect_id) {
                return bad("no matching dispatch intent");
            }
            n.effect = EffectState::None;
            n.phase = Phase::Failed;
            n.lease = None;
            n.reason = Some(format!("provider refused: {reason}"));
        }
        Event::ManifestCommitted { fence, items } => {
            fenced(fence)?;
            if s.step != s.steps.len() {
                return bad("steps incomplete");
            }
            if *items == 0 || *items != s.imported.len() {
                return bad("manifest must list every imported object");
            }
            if s.cancel != CancelState::None {
                return bad("cancel requested");
            }
            n.manifest = true;
            n.phase = Phase::Succeeded;
            n.lease = None;
            n.reason = None;
        }
        Event::Failed { fence, reason } => {
            fenced(fence)?;
            if matches!(s.effect, EffectState::Intended | EffectState::Uncertain) {
                return bad("an uncertain submission must be reconciled before failing");
            }
            n.phase = Phase::Failed;
            n.lease = None;
            n.reason = Some(reason.clone());
        }
        Event::CancelRequested => {
            if s.cancel != CancelState::None {
                return Ok(n);
            }
            n.cancel = CancelState::Requested;
            if s.effect == EffectState::None && s.lease.is_none() {
                n.cancel = CancelState::LocallyStopped;
                n.phase = Phase::Cancelled;
            }
        }
        Event::CancelConfirmed { op_id } => {
            if !matches!(s.cancel, CancelState::Requested | CancelState::Uncertain) || !op_matches(op_id) {
                return bad("no provider cancel outstanding");
            }
            n.cancel = CancelState::ProviderConfirmed;
            n.phase = Phase::Cancelled;
            n.lease = None;
        }
        Event::CancelStoppedLocally => {
            if s.cancel != CancelState::Requested {
                return bad("cancel not requested");
            }
            if matches!(s.effect, EffectState::Intended | EffectState::Submitted | EffectState::Uncertain) {
                return bad("external work outstanding; cancel at the provider");
            }
            n.cancel = CancelState::LocallyStopped;
            n.phase = Phase::Cancelled;
            n.lease = None;
        }
        Event::CancelUncertain { reason } => {
            if s.cancel != CancelState::Requested || s.effect != EffectState::Submitted {
                return bad("cancel not requested");
            }
            n.cancel = CancelState::Uncertain;
            n.phase = Phase::Waiting;
            n.lease = None;
            n.reason = Some(reason.clone());
            if n.usage_credits.is_none() {
                n.usage_note = Some(format!("charge unknown: {reason}"));
            }
        }
    }
    Ok(n)
}

fn set_usage(n: &mut Job, credits: Option<u64>) {
    n.usage_credits = credits;
    n.usage_note = credits.is_none().then(|| "provider did not report usage".into());
}

/// Fold a whole history; used by trace replay and stored-job audits.
pub fn replay(events: &[Event]) -> Result<Job, (usize, Reject)> {
    let mut state: Option<Job> = None;
    for (i, e) in events.iter().enumerate() {
        state = Some(apply(state.as_ref(), e).map_err(|r| (i, r))?);
    }
    state.ok_or((0, Reject::Invalid("empty history".into())))
}
