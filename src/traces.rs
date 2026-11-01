//! Replayable lifecycle traces, embedded so `assetq selftest` can run them.

use crate::model::{self, Event};
use serde_json::Value;

pub const TRACES: &[(&str, &str)] = &[
    ("protected-supervisor-exit.json", include_str!("../tests/traces/protected-supervisor-exit.json")),
    ("observation-unknown.json", include_str!("../tests/traces/observation-unknown.json")),
    ("execution-deadline.json", include_str!("../tests/traces/execution-deadline.json")),
    ("cancel-during-wait.json", include_str!("../tests/traces/cancel-during-wait.json")),
    ("cancel-queued.json", include_str!("../tests/traces/cancel-queued.json")),
    ("cancel-uncertain-submission.json", include_str!("../tests/traces/cancel-uncertain-submission.json")),
    ("cancel-uncertain-completed.json", include_str!("../tests/traces/cancel-uncertain-completed.json")),
    ("cancel-uncertain.json", include_str!("../tests/traces/cancel-uncertain.json")),
    ("crash-after-intent.json", include_str!("../tests/traces/crash-after-intent.json")),
    ("duplicate-events.json", include_str!("../tests/traces/duplicate-events.json")),
    ("happy-path.json", include_str!("../tests/traces/happy-path.json")),
    ("lost-reply-absent.json", include_str!("../tests/traces/lost-reply-absent.json")),
    ("lost-reply-found.json", include_str!("../tests/traces/lost-reply-found.json")),
    ("partial-import.json", include_str!("../tests/traces/partial-import.json")),
    ("stale-executor.json", include_str!("../tests/traces/stale-executor.json")),
    ("store-full.json", include_str!("../tests/traces/store-full.json")),
];

/// Replay one trace: accepted steps must apply, `reject` steps must be refused
/// with the named kind and leave the state unchanged, then `expect` must match.
pub fn run(raw: &str) -> Result<(), String> {
    let t: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let mut state = None;
    for (i, step) in t["steps"].as_array().ok_or("steps missing")?.iter().enumerate() {
        let e: Event = serde_json::from_value(step["event"].clone()).map_err(|e| format!("step {i}: {e}"))?;
        let r = model::apply(state.as_ref(), &e);
        match (step["reject"].as_str(), r) {
            (None, Ok(n)) => state = Some(n),
            (None, Err(r)) => return Err(format!("step {i}: unexpected rejection: {r}")),
            (Some(kind), Err(r)) => {
                let got = if r == model::Reject::Stale { "stale" } else { "invalid" };
                if got != kind {
                    return Err(format!("step {i}: expected {kind} rejection, got {r}"));
                }
            }
            (Some(kind), Ok(_)) => return Err(format!("step {i}: expected {kind} rejection, but it applied")),
        }
    }
    let job = serde_json::to_value(state.ok_or("empty trace")?).unwrap();
    for (k, want) in t["expect"].as_object().ok_or("expect missing")? {
        if &job[k] != want {
            return Err(format!("expect {k} = {want}, got {}", job[k]));
        }
    }
    Ok(())
}
