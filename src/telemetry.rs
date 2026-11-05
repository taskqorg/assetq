//! Pipe observations in the installation's one CoreQ local store (pipe.md § Telemetry).
//!
//! Error classes are `&'static str`, so a raw reason, prompt or path cannot reach a record.
//! A failed write never changes a job (R2); `state` reads unknown instead (R4).

use crate::provider::{Poll, Submit};
use agentq_runtime::agent::{Context, Run};
use coreq::telemetry::{Budget, Event, INSTALLATION_BYTES, PEER_BYTES, Store};
use serde_json::{Map, Value, json};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const PRODUCT: &str = "pipe";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const NO_CALLER: &str = "no-caller-context";
// ponytail: a 200 ms idle loop would sync 432000 rows a day per executor; identical passes fold, as CoreQ polls do.
const FOLD: Duration = Duration::from_secs(60);

/// Telemetry outcome and stable error class.
pub type Outcome = (&'static str, &'static str);
pub const OK: Outcome = ("ok", "none");

/// The quantity on a record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Units {
    None,
    Credits(Option<u64>),
    Items(u64),
}

/// The identity a job's records share; every value is a code or null with a reason.
#[derive(Clone, Default)]
pub struct Scope {
    trace: Option<String>,
    project: Option<String>,
    pipeline: Option<String>,
    revision: Option<String>,
    runtime: Option<String>,
}

impl Scope {
    pub fn job(id: &str, project: &str, method: &str, version: i32, provider: Option<&str>) -> Scope {
        Scope {
            trace: code(id.replace('-', "")),
            project: project_code(project),
            pipeline: code(method.to_string()),
            revision: Some(version.to_string()),
            runtime: provider.and_then(|name| code(name.to_string())),
        }
    }
}

/// When an operation started: wall clock for the record, monotonic for its duration.
pub struct Started {
    at_ms: u64,
    clock: Instant,
}

impl Started {
    pub fn now() -> Started {
        Started { at_ms: SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |t| t.as_millis() as u64), clock: Instant::now() }
    }
    fn elapsed_us(&self) -> u64 {
        self.clock.elapsed().as_micros() as u64
    }
}

struct Fold {
    started: Started,
    outcome: Outcome,
    passes: u64,
    duration_us: u64,
}

/// Consecutive passes that handled no item and share one outcome.
#[derive(Default)]
pub struct Passes {
    fold: Option<Fold>,
}

pub struct Telemetry {
    store: Result<Store, String>,
    lost: AtomicU64,
    losses: PathBuf,
}

/// The store path below a data root.
pub fn path(data_root: &Path) -> PathBuf {
    data_root.join("telemetry").join("observations.sqlite3")
}

impl Telemetry {
    /// An unavailable store reads unknown in `state`; it never stops Pipe.
    pub fn open(data_root: &Path) -> Telemetry {
        let store = open_store(data_root);
        if let Err(error) = &store {
            eprintln!("telemetry unavailable: {error}");
        }
        let losses = data_root.join("telemetry/lost");
        if let Ok(_lock) = loss_lock(&losses) {
            if let Ok(mut file) = OpenOptions::new().create_new(true).write(true).open(&losses) {
                let _ = file.write_all(&0u64.to_le_bytes()).and_then(|_| file.sync_all());
            }
        }
        Telemetry { store, lost: AtomicU64::new(0), losses }
    }

    pub(crate) fn lost(&self, error: &str) {
        let _ = loss_count(&self.losses, true);
        if self.lost.fetch_add(1, Ordering::Relaxed) == 0 {
            eprintln!("telemetry record lost: {error}");
        }
    }

    fn write(&self, value: Value) {
        let written = match &self.store {
            Ok(store) => Event::observation(value).and_then(|event| store.append(&[event])).map(|_| ()),
            Err(error) => Err(error.clone()),
        };
        if let Err(error) = written {
            self.lost(&error);
        }
    }

    /// One generation step (`pipe.step`).
    pub fn step(&self, scope: &Scope, step: &str, started: &Started, outcome: Outcome) {
        let mut value = record("stage", "pipe.step", started, started.elapsed_us(), outcome, scope, Units::None);
        value["step"] = code(step.to_string()).map_or(Value::Null, Value::String);
        self.write(value);
    }

    /// One provider call (`provider.<call>`).
    pub fn provider(&self, scope: &Scope, operation: &'static str, started: &Started, outcome: Outcome, units: Units) {
        self.write(record("operation", operation, started, started.elapsed_us(), outcome, scope, units));
    }

    /// One executor pass. `Ok(true)` handled an item and is its own record; the rest fold until the outcome changes.
    pub fn pass(&self, passes: &mut Passes, started: Started, result: Result<bool, &'static str>) {
        if result == Ok(true) {
            self.flush(passes);
            self.write(pass_record(&started, started.elapsed_us(), OK, 1, 1));
            return;
        }
        let outcome = result.map_or_else(|class| ("failed", class), |_| OK);
        if passes.fold.as_ref().is_some_and(|fold| fold.outcome != outcome) {
            self.flush(passes);
        }
        let duration_us = started.elapsed_us();
        let fold = passes.fold.get_or_insert(Fold { started, outcome, passes: 0, duration_us: 0 });
        fold.passes += 1;
        fold.duration_us += duration_us;
        if fold.started.clock.elapsed() >= FOLD {
            self.flush(passes);
        }
    }

    /// Writes the folded passes; the executor calls it before it stops.
    pub fn flush(&self, passes: &mut Passes) {
        if let Some(fold) = passes.fold.take() {
            self.write(pass_record(&fold.started, fold.duration_us, fold.outcome, fold.passes, 0));
        }
    }

    /// Hands one finished agent run to `agentq-runtime`, which owns `agent.turn` and `agent.call` (R11).
    pub fn agent_run(&self, run: &mut Run, project: &str) {
        let Ok(store) = &self.store else {
            return self.lost("store unavailable");
        };
        let project = project_code(project).unwrap_or_else(|| "other".into());
        let context = Context {
            product: PRODUCT,
            version: VERSION,
            project: &project,
            task: None,
            route: Some("image.worker"),
            account: "inherited",
        };
        match run.record(&context, store) {
            Ok(report) => {
                if let Err(error) = report.recorded {
                    self.lost(&error);
                }
            }
            Err(error) => self.lost(&error),
        }
    }

    /// The `telemetry` part of `GET /v1/status`.
    pub fn state(&self) -> Value {
        let Ok(store) = &self.store else {
            return json!({"state": "unknown", "reason": "local store unavailable"});
        };
        let shipper = store.shipper().unwrap_or_else(|_| json!({"state": "unknown", "reason": "shipper state unreadable"}));
        match loss_count(&self.losses, false) {
            Ok(0) if self.lost.load(Ordering::Relaxed) == 0 => json!({"state": "observed", "lost": 0, "shipper": shipper}),
            Ok(lost) if lost > 0 => json!({"state": "unknown", "reason": "observations lost", "lost": lost, "shipper": shipper}),
            _ => json!({"state": "unknown", "reason": "loss counter unreadable", "lost": null, "shipper": shipper}),
        }
    }
}

// Shared health metadata, not an observation store. Never reset a missing or corrupt counter.
fn loss_lock(path: &Path) -> std::io::Result<File> {
    let lock = File::options().create(true).truncate(false).write(true).open(path.with_extension("lock"))?;
    lock.lock()?;
    Ok(lock)
}

fn loss_count(path: &Path, increment: bool) -> std::io::Result<u64> {
    let _lock = loss_lock(path)?;
    let mut file = File::options().read(true).write(true).open(path)?;
    let mut bytes = [0u8; 8];
    file.read_exact(&mut bytes)?;
    let count = u64::from_le_bytes(bytes);
    if increment {
        file.seek(SeekFrom::Start(0))?;
        file.write_all(&count.saturating_add(1).to_le_bytes())?;
        file.sync_all()?;
    }
    Ok(count)
}

fn open_store(data_root: &Path) -> Result<Store, String> {
    let dir = data_root.join("telemetry");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dir = dir.canonicalize().map_err(|e| e.to_string())?;
    let budget = Budget { lock: dir.join("budget.lock"), peers: vec![], total: INSTALLATION_BYTES, reserved: PEER_BYTES };
    Store::open_bounded(&dir.join("observations.sqlite3"), budget)
}

fn pass_record(started: &Started, duration_us: u64, outcome: Outcome, calls: u64, items: u64) -> Value {
    let mut value = record("service.pass", "pipe.exec.pass", started, duration_us, outcome, &Scope::default(), Units::Items(items));
    value["calls"] = json!(calls);
    value["unknown"]["trace"] = json!("no-job");
    value["unknown"]["project"] = json!("no-job");
    value["unknown"]["pipeline"] = json!("no-job");
    value["unknown"]["pipeline_revision"] = json!("no-job");
    value["unknown"]["runtime"] = json!("no-job");
    value
}

fn record(kind: &str, operation: &str, started: &Started, duration_us: u64, outcome: Outcome, scope: &Scope, units: Units) -> Value {
    let mut unknown = Map::new();
    for name in ["task", "attempt", "actor"] {
        unknown.insert(name.into(), json!(NO_CALLER));
    }
    unknown.insert("span".into(), json!("not-traced"));
    let mut text = |name: &str, value: &Option<String>, reason: &str| {
        if value.is_none() {
            unknown.insert(name.into(), json!(reason));
        }
        json!(value)
    };
    let trace = text("trace", &scope.trace, "invalid-code");
    let project = text("project", &scope.project, "invalid-code");
    let pipeline = text("pipeline", &scope.pipeline, "invalid-code");
    let revision = text("pipeline_revision", &scope.revision, "invalid-code");
    let runtime = text("runtime", &scope.runtime, "no-provider");
    let host = text("host", &host(), "not-reported");
    let (quantity, unit) = match units {
        Units::None => (Value::Null, None),
        Units::Credits(credits) => (json!(credits), Some("credits")),
        Units::Items(items) => (json!(items), Some("items")),
    };
    if units == Units::Credits(None) {
        unknown.insert("units".into(), json!("provider-unreported"));
    }
    json!({
        "product": PRODUCT, "version": VERSION, "kind": kind, "operation": operation,
        "trace": trace, "span": null, "started_ms": started.at_ms, "duration_us": duration_us,
        "outcome": outcome.0, "error": outcome.1, "project": project, "task": null, "attempt": null, "actor": null,
        "pipeline": pipeline, "pipeline_revision": revision, "runtime": runtime, "host": host,
        "units": quantity, "unit_kind": unit, "unknown": unknown,
    })
}

fn code(text: String) -> Option<String> {
    (!text.is_empty() && text.len() <= 128 && text.bytes().all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))).then_some(text)
}

/// A Pipe project as a CoreQ project code; a bare ID gets the `pipe/` owner (pipe.md § Telemetry).
pub fn project_code(id: &str) -> Option<String> {
    let name = if id.contains('/') { id.to_string() } else { format!("pipe/{id}") };
    (name.len() <= 128 && name.split('/').all(|part| !matches!(part, "." | "..") && code(part.to_string()).is_some())).then_some(name)
}

fn host() -> Option<String> {
    #[cfg(windows)]
    let name = std::env::var("COMPUTERNAME").ok();
    #[cfg(unix)]
    let name = {
        let mut bytes = [0u8; 256];
        // SAFETY: the buffer holds the length passed.
        (unsafe { libc::gethostname(bytes.as_mut_ptr().cast(), bytes.len()) } == 0)
            .then(|| String::from_utf8_lossy(&bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())]).into_owned())
    };
    name.and_then(code)
}

/// A stable class for a reason, by its fixed prefix; the text after the prefix is never stored.
pub fn class(reason: &str, fallback: &'static str) -> &'static str {
    const KNOWN: &[(&str, &str)] = &[
        ("capability_unavailable", "capability_unavailable"),
        ("store_full", "store_full"),
        ("admission_busy", "admission_busy"),
        ("metadata_full", "metadata_full"),
        ("validation failed", "validation-failed"),
        ("fetch failed", "fetch-failed"),
        ("import of", "import-failed"),
    ];
    KNOWN.iter().find(|(prefix, _)| reason.starts_with(prefix)).map_or(fallback, |(_, class)| class)
}

pub fn submit_outcome(reply: &Submit) -> (Outcome, Units) {
    let outcome = match reply {
        Submit::Accepted(_) => OK,
        Submit::Uncertain(_) => ("unknown", "submit-uncertain"),
        Submit::Refused(reason) => ("refused", class(reason, "submit-refused")),
    };
    (outcome, Units::None)
}

pub fn lookup_outcome(reply: &Result<Option<String>, String>) -> (Outcome, Units) {
    let outcome = match reply {
        Ok(_) => OK,
        Err(reason) => ("unknown", class(reason, "lookup-unknown")),
    };
    (outcome, Units::None)
}

pub fn poll_outcome(reply: &Result<Poll, String>) -> (Outcome, Units) {
    match reply {
        Ok(Poll::Pending) => (OK, Units::None),
        Ok(Poll::Done { credits, .. }) => (OK, Units::Credits(*credits)),
        Ok(Poll::Failed { reason, credits }) => (("failed", class(reason, "provider-failed")), Units::Credits(*credits)),
        Ok(Poll::Cancelled) => (("cancelled", "provider-cancelled"), Units::None),
        Err(reason) => (("unknown", class(reason, "poll-unknown")), Units::None),
    }
}

pub fn cancel_outcome(reply: &Result<bool, String>) -> (Outcome, Units) {
    let outcome = match reply {
        Ok(true) => OK,
        Ok(false) => ("unknown", "cancel-unconfirmed"),
        Err(reason) => ("unknown", class(reason, "cancel-failed")),
    };
    (outcome, Units::None)
}

pub fn validate_outcome(reply: &Result<(), String>) -> (Outcome, Units) {
    let outcome = match reply {
        Ok(()) => OK,
        Err(reason) => ("failed", class(reason, "validation-failed")),
    };
    (outcome, Units::None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use coreq::telemetry::Reader;

    fn fixture() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pipe-telemetry-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn rows(dir: &Path) -> Vec<Value> {
        Reader::open(&path(dir)).unwrap().export(0, 0, 1000).unwrap()["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["observation"].clone())
            .collect()
    }

    // R4/R11: a missing value reads null with a reason, a reported zero stays zero and credits are never guessed.
    #[test]
    fn telemetry_units_and_missing_values_read_null_with_a_reason() {
        let dir = fixture();
        let telemetry = Telemetry::open(&dir);
        let scope = Scope::job("0a1b2c3d-0000-4000-8000-000000000001", "csgo", "image.worker.v1", 5, Some("agentq.image"));
        for credits in [Some(15), Some(0), None] {
            let (outcome, units) = poll_outcome(&Ok(Poll::Done { outputs: 1, credits }));
            telemetry.provider(&scope, "provider.poll", &Started::now(), outcome, units);
        }
        let rows = rows(&dir);
        assert_eq!(rows.len(), 3);
        for (row, units) in rows.iter().zip([json!(15), json!(0), Value::Null]) {
            assert_eq!(row["units"], units);
            assert_eq!(row["unit_kind"], "credits");
            assert_eq!(row["schema"], 1);
            assert_eq!(row["pipeline"], "image.worker.v1");
            assert_eq!(row["pipeline_revision"], "5");
            assert_eq!(row["runtime"], "agentq.image");
            assert_eq!(row["project"], "pipe/csgo");
            assert_eq!(row["trace"], "0a1b2c3d000040008000000000000001");
            for name in ["task", "attempt", "actor"] {
                assert_eq!(row[name], Value::Null);
                assert_eq!(row["unknown"][name], NO_CALLER);
            }
            assert_eq!(row["unknown"]["entry"], "not-collected");
        }
        assert_eq!(rows[2]["unknown"]["units"], "provider-unreported");
        assert!(rows[1]["unknown"].get("units").is_none(), "a reported zero has no unknown reason");
        assert_eq!(telemetry.state()["state"], "observed");
        drop(telemetry);
        std::fs::remove_dir_all(dir).unwrap();
    }

    // R11: a reason can hold a path or provider text; only its fixed class reaches a record.
    #[test]
    fn telemetry_reason_text_never_reaches_a_record() {
        let private = "C:\\private\\prompt invented-secret";
        assert_eq!(class(private, "poll-unknown"), "poll-unknown");
        assert_eq!(class(&format!("store_full: {private}"), "x"), "store_full");
        assert_eq!(class(&format!("validation failed: {private}"), "x"), "validation-failed");
        let dir = fixture();
        let telemetry = Telemetry::open(&dir);
        let scope = Scope::job("id", "owner/name", "method with spaces", 1, None);
        let (outcome, units) = poll_outcome(&Err(private.into()));
        telemetry.provider(&scope, "provider.poll", &Started::now(), outcome, units);
        let row = rows(&dir).remove(0);
        assert!(!row.to_string().contains("invented-secret") && !row.to_string().contains("private"));
        assert_eq!(row["outcome"], "unknown");
        assert_eq!(row["project"], "owner/name");
        assert_eq!(row["pipeline"], Value::Null, "an ID that is not a code reads null");
        assert_eq!(row["unknown"]["pipeline"], "invalid-code");
        assert_eq!(row["runtime"], Value::Null);
        assert_eq!(row["unknown"]["runtime"], "no-provider");
        drop(telemetry);
        std::fs::remove_dir_all(dir).unwrap();
    }

    // Service passes: a pass with an item stands alone; idle and failing passes fold by outcome and keep their count.
    #[test]
    fn telemetry_passes_fold_by_outcome_and_keep_their_count() {
        let dir = fixture();
        let telemetry = Telemetry::open(&dir);
        let mut passes = Passes::default();
        for result in [Ok(false), Ok(false), Ok(true), Err("database"), Err("database"), Err("other"), Ok(false)] {
            telemetry.pass(&mut passes, Started::now(), result);
        }
        telemetry.flush(&mut passes);
        let rows = rows(&dir);
        let shape: Vec<_> =
            rows.iter().map(|r| (r["outcome"].clone(), r["error"].clone(), r["calls"].clone(), r["units"].clone())).collect();
        assert_eq!(
            shape,
            [
                (json!("ok"), json!("none"), json!(2), json!(0)),
                (json!("ok"), json!("none"), json!(1), json!(1)),
                (json!("failed"), json!("database"), json!(2), json!(0)),
                (json!("failed"), json!("other"), json!(1), json!(0)),
                (json!("ok"), json!("none"), json!(1), json!(0)),
            ]
        );
        assert!(rows.iter().all(|r| r["kind"] == "service.pass" && r["unit_kind"] == "items" && r["unknown"]["trace"] == "no-job"));
        drop(telemetry);
        std::fs::remove_dir_all(dir).unwrap();
    }

    // R4/R11: the API and a restarted service must see losses from a separate worker.
    #[test]
    fn telemetry_worker_loss_survives_exit_and_is_visible_to_other_processes() {
        let dir = fixture();
        let api = Telemetry::open(&dir);
        let worker = Telemetry::open(&dir);
        worker.write(json!({"product":"invalid product"}));
        assert_eq!(worker.state()["lost"], 1);
        assert_eq!(api.state()["state"], "unknown");
        assert_eq!(api.state()["lost"], 1);
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| Telemetry::open(&dir).write(json!({"product":"invalid product"})));
            }
        });
        assert_eq!(api.state()["lost"], 9, "concurrent workers cannot overwrite losses");
        drop(worker);
        drop(api);
        let restarted = Telemetry::open(&dir);
        assert_eq!(restarted.state()["lost"], 9);
        assert_eq!(restarted.state()["state"], "unknown");
        drop(restarted);
        std::fs::write(dir.join("telemetry/lost"), b"bad").unwrap();
        let corrupt = Telemetry::open(&dir);
        assert_eq!(corrupt.state()["state"], "unknown");
        assert_eq!(corrupt.state()["lost"], Value::Null);
        drop(corrupt);
        std::fs::remove_dir_all(dir).unwrap();
    }

    // R2/R4: an unavailable store loses the record, reads unknown and never stops the caller.
    #[test]
    fn telemetry_unavailable_store_reads_unknown() {
        let dir = fixture();
        std::fs::write(dir.join("telemetry"), "invented file blocks the directory").unwrap();
        let telemetry = Telemetry::open(&dir);
        assert_eq!(telemetry.state()["state"], "unknown");
        telemetry.step(&Scope::default(), "validate", &Started::now(), OK);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
