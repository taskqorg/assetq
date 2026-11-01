//! Isolated-fixture proofs against a real PostgreSQL server (A3-A7).
//! `ASSETQ_TEST_PG` is a libpq keyword string for a disposable cluster, for
//! example `host=/tmp/x port=5544 user=me`; `scripts/check.py` starts one.
//! Every test creates its own database, data root and synthetic provider dir.
//! The provider is `synthetic.fake`: these tests prove recovery semantics,
//! never real generation.

use assetq::db::{self, Principal};
use assetq::exec::{self, Ctx};
use assetq::provider::{FakeProvider, Output, Poll, Provider, Submit};
use assetq::{Config, api};
use postgres::Client;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

struct Fixture {
    url: String,
    dir: PathBuf,
}

fn fixture() -> Fixture {
    let admin =
        std::env::var("ASSETQ_TEST_PG").expect("ASSETQ_TEST_PG must name a disposable PostgreSQL cluster (run python3 scripts/check.py)");
    let name = format!("assetq_t_{}", uuid::Uuid::new_v4().simple());
    let mut c = Client::connect(&format!("{admin} dbname=postgres"), postgres::NoTls).expect("connect test cluster");
    c.batch_execute(&format!("CREATE DATABASE {name}")).unwrap();
    let url = format!("{admin} dbname={name}");
    db::migrate(&mut db::connect(&url).unwrap()).unwrap();
    db::migrate(&mut db::connect(&url).unwrap()).unwrap(); // idempotent
    let dir = std::env::temp_dir().join(format!("assetq-it-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&dir).unwrap();
    Fixture { url, dir }
}

impl Fixture {
    fn config(&self, quota: u64) -> Config {
        Config {
            database_url: self.url.clone(),
            data_root: self.dir.join("data"),
            quota_bytes: quota,
            max_upload_bytes: 1 << 20,
            temp_quota_bytes: 1 << 30,
            metadata_quota_bytes: 1 << 30,
            free_space_floor_bytes: 0,
            metadata_admission_headroom_bytes: 4 << 20,
            metadata_recovery_headroom_bytes: 16 << 20,
            max_execution_ms: 86_400_000,
            content_types: assetq::DEFAULT_TYPES.split(',').map(String::from).collect(),
            executors: 1,
            poll_ms: 0,
            lease_ms: 60_000,
            retention_days: 7,
            fake_provider_dir: Some(self.dir.join("fake")),
            image_worker_root: None,
        }
    }
    fn ctx(&self, quota: u64) -> Ctx {
        Ctx::new(self.config(quota)).unwrap()
    }
    fn client(&self) -> Client {
        db::connect(&self.url).unwrap()
    }
    fn fake(&self) -> FakeProvider {
        FakeProvider::new(self.dir.join("fake")).unwrap()
    }
    fn fault(&self, name: &str) {
        std::fs::write(self.dir.join("fake/faults").join(name), "").unwrap();
    }
}

fn project(c: &mut Client, id: &str, budget: i64, max_active: i32) -> (Principal, String) {
    db::create_project(c, id, budget, max_active).unwrap();
    let t = db::create_token(c, id, "member").unwrap();
    (db::authenticate(c, &t).unwrap(), t)
}

fn req(key: &str, constraints: Value) -> Value {
    json!({"method": "synthetic.fake", "operation": "synthetic.generate", "idempotency_key": key,
           "text": "original stylized wooden crate", "constraints": constraints, "budget": {"max_credits": 1}})
}

const NO_PROVIDERS: &[&str] = &[];
const FAKE: &[&str] = &["synthetic.fake"];
fn limits() -> db::Limits {
    db::Limits { quota_bytes: 1 << 30 }
}

// R4/R6/R7: an observation outage must remain visible and reserve capacity;
// terminal interruption distinguishes an API cancel from a worker failure.
#[test]
fn image_worker_observation_failure_and_terminal_cancel_keep_capacity() {
    use std::sync::atomic::{AtomicBool, Ordering};
    struct Interrupted {
        fake: FakeProvider,
        terminal: Arc<AtomicBool>,
    }
    impl Provider for Interrupted {
        fn name(&self) -> &str {
            self.fake.name()
        }
        fn submit(&self, e: &str, r: &Value) -> Submit {
            self.fake.submit(e, r)
        }
        fn lookup(&self, e: &str) -> Result<Option<String>, String> {
            self.fake.lookup(e)
        }
        fn poll(&self, _: &str) -> Result<Poll, String> {
            if self.terminal.load(Ordering::Relaxed) {
                Ok(Poll::Cancelled)
            } else {
                Err("native observation unavailable; no replay".into())
            }
        }
        fn fetch(&self, o: &str, i: usize) -> Result<Output, String> {
            self.fake.fetch(o, i)
        }
        fn cancel(&self, _: &str) -> Result<bool, String> {
            Ok(false)
        }
    }
    for requested in [false, true] {
        let f = fixture();
        let mut c = f.client();
        let (p, _) = project(&mut c, "worker-outage", 3, 1);
        let terminal = Arc::new(AtomicBool::new(false));
        let mut ctx = f.ctx(1 << 30);
        ctx.providers = vec![Arc::new(Interrupted { fake: f.fake(), terminal: terminal.clone() })];
        let request = req("one", json!({}));
        let (j, _) = db::submit_job(&mut c, &p, &request, &limits(), FAKE).unwrap();
        let id = j["id"].as_str().unwrap();
        exec::tick(&mut c, &ctx, "worker").unwrap();
        exec::tick(&mut c, &ctx, "observer").unwrap();
        let unknown = db::get_job(&mut c, &p.project, id).unwrap();
        assert_eq!(unknown["phase"], "waiting");
        assert_eq!(unknown["reason"], "native observation unavailable; no replay");
        assert_eq!(f.fake().submissions(), 1);
        assert_eq!(db::submit_job(&mut c, &p, &request, &limits(), FAKE).unwrap().0["id"], id);
        assert_eq!(db::submit_job(&mut c, &p, &req("two", json!({})), &limits(), FAKE).unwrap_err().code, "concurrency_limit");
        if requested {
            db::cancel_job(&mut c, &p.project, id).unwrap();
            exec::tick(&mut c, &ctx, "canceller").unwrap();
            assert_eq!(db::get_job(&mut c, &p.project, id).unwrap()["cancel"], "uncertain");
        }
        terminal.store(true, Ordering::Relaxed);
        exec::tick(&mut c, &ctx, "observer").unwrap();
        let done = db::get_job(&mut c, &p.project, id).unwrap();
        assert_eq!(done["phase"], if requested { "cancelled" } else { "failed" });
        assert!(done["usage"]["credits"].is_null());
        assert_eq!(f.fake().submissions(), 1);
        db::submit_job(&mut c, &p, &req("two", json!({})), &limits(), FAKE).unwrap();
        db::audit_job(&mut c, id).unwrap();
    }
}

/// Drive in-process executors until `done(job)` or a step budget runs out.
fn drive(c: &mut Client, ctx: &Ctx, project: &str, id: &str, done: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..400 {
        let job = db::get_job(c, project, id).unwrap();
        if done(&job) {
            return job;
        }
        if !exec::tick(c, ctx, "it-executor").unwrap() {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    panic!("job {id} did not reach the expected state: {}", db::get_job(c, project, id).unwrap());
}

fn phase(p: &'static str) -> impl Fn(&Value) -> bool {
    move |j| j["phase"] == p
}

fn event_types(c: &mut Client, project: &str, id: &str) -> Vec<String> {
    db::job_events(c, project, id).unwrap().into_iter().map(|(_, e, _)| e["type"].as_str().unwrap().to_string()).collect()
}

#[test]
fn admission_idempotency_scope_and_typed_limits() {
    let f = fixture();
    let mut c = f.client();
    let (a, _) = project(&mut c, "alpha", 2, 2);
    let (b, _) = project(&mut c, "beta", 10, 4);
    let (j1, created) = db::submit_job(&mut c, &a, &req("k1", json!({})), &limits(), FAKE).unwrap();
    assert!(created);
    assert_eq!(j1["method"], "synthetic.fake@1");
    let (j1b, created) = db::submit_job(&mut c, &a, &req("k1", json!({})), &limits(), FAKE).unwrap();
    assert!(!created);
    assert_eq!(j1b["id"], j1["id"]);
    let err = |r: Result<(Value, bool), assetq::Error>| r.unwrap_err().code;
    assert_eq!(err(db::submit_job(&mut c, &a, &req("k1", json!({"outputs": 2})), &limits(), FAKE)), "idempotency_conflict");
    // The same key in another project is independent.
    assert!(db::submit_job(&mut c, &b, &req("k1", json!({})), &limits(), FAKE).unwrap().1);
    // Cross-project reads look like missing objects.
    let id = j1["id"].as_str().unwrap();
    assert_eq!(db::get_job(&mut c, "beta", id).unwrap_err().status, 404);
    // Typed constraints, unknown fields, disabled methods, missing capability.
    assert_eq!(err(db::submit_job(&mut c, &a, &req("k2", json!({"outputs": "two"})), &limits(), FAKE)), "bad_constraints");
    assert_eq!(err(db::submit_job(&mut c, &a, &req("k2", json!({"outputs": 9})), &limits(), FAKE)), "bad_constraints");
    assert_eq!(err(db::submit_job(&mut c, &a, &req("k2", json!({"model": "x"})), &limits(), FAKE)), "bad_constraints");
    let mut r = req("k2", json!({}));
    r["permissions"] = json!(["admin"]);
    assert_eq!(err(db::submit_job(&mut c, &a, &r, &limits(), FAKE)), "bad_request");
    let mut r = req("k2", json!({}));
    r["project"] = json!("beta");
    assert_eq!(err(db::submit_job(&mut c, &a, &r, &limits(), FAKE)), "forbidden");
    let mut r = req("k2", json!({}));
    r["method"] = json!("image.worker.v1@1");
    assert_eq!(err(db::submit_job(&mut c, &a, &r, &limits(), FAKE)), "method_disabled");
    assert_eq!(err(db::submit_job(&mut c, &a, &req("k2", json!({})), &limits(), NO_PROVIDERS)), "capability_unavailable");
    let mut r = req("k2", json!({}));
    r["budget"]["max_credits"] = json!(0);
    assert_eq!(err(db::submit_job(&mut c, &a, &r, &limits(), FAKE)), "budget_too_low");
    let mut r = req("k2", json!({}));
    r["attachments"] = json!([uuid::Uuid::new_v4().to_string()]);
    assert_eq!(err(db::submit_job(&mut c, &a, &r, &limits(), FAKE)), "attachment_not_found");
    // Concurrency: alpha allows 2 active jobs.
    db::submit_job(&mut c, &a, &req("k3", json!({})), &limits(), FAKE).unwrap();
    assert_eq!(err(db::submit_job(&mut c, &a, &req("k4", json!({})), &limits(), FAKE)), "concurrency_limit");
    // Budget: beta has 10 credits; reservations of 1 each are counted.
    db::create_project(&mut c, "alpha", 2, 10).unwrap();
    assert_eq!(err(db::submit_job(&mut c, &a, &req("k5", json!({})), &limits(), FAKE)), "budget_exhausted");
    // Store full refuses new jobs with a reason.
    let tiny = db::Limits { quota_bytes: 0 };
    assert_eq!(err(db::submit_job(&mut c, &b, &req("k6", json!({})), &tiny, FAKE)), "store_full");
}

#[test]
fn concurrent_duplicate_submissions_create_one_job() {
    let f = fixture();
    let mut c = f.client();
    let (p, _) = project(&mut c, "gamma", 100, 50);
    let ids: Vec<String> = (0..8)
        .map(|_| {
            let (url, p) = (f.url.clone(), p.clone());
            std::thread::spawn(move || {
                let mut c = db::connect(&url).unwrap();
                db::submit_job(&mut c, &p, &req("same", json!({})), &limits(), FAKE).unwrap().0["id"].as_str().unwrap().to_string()
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect();
    assert!(ids.iter().all(|i| *i == ids[0]));
    let n: i64 = c.query_one("SELECT count(*) FROM jobs", &[]).unwrap().get(0);
    assert_eq!(n, 1);
}

#[test]
fn manifest_commit_is_atomic_and_fenced() {
    let f = fixture();
    let ctx = f.ctx(1 << 30);
    let mut c = f.client();
    let (p, _) = project(&mut c, "delta", 10, 4);
    let (j, _) = db::submit_job(&mut c, &p, &req("m", json!({})), &limits(), FAKE).unwrap();
    let id = j["id"].as_str().unwrap();
    let job = drive(&mut c, &ctx, "delta", id, phase("succeeded"));
    assert_eq!(job["submissions"], 1);
    let fence = job["fence"].as_i64().unwrap();
    // A stale executor's manifest write rolls back with its rejected event.
    let mut tx = db::write_tx(&mut c, true).unwrap();
    tx.execute("DELETE FROM manifests WHERE job_id = $1", &[&id]).unwrap();
    let stale = assetq::model::Event::ManifestCommitted { fence: fence - 1, items: 1 };
    assert_eq!(db::apply_event(&mut tx, id, &stale, 1000, 0).unwrap_err().code, "invalid_transition");
    drop(tx);
    let n: i64 = c.query_one("SELECT count(*) FROM manifests WHERE job_id = $1", &[&id]).unwrap().get(0);
    assert_eq!(n, 1, "rollback restored the committed manifest");
    let r = db::job_result(&mut c, "delta", id).unwrap();
    assert_eq!(r["status"], "succeeded");
    assert_eq!(r["manifest"]["items"][0]["synthetic"], true);
    db::audit_job(&mut c, id).unwrap();
}

#[test]
fn crash_after_intent_reconciles_before_one_resubmission() {
    let f = fixture();
    let ctx = f.ctx(1 << 30);
    let mut c = f.client();
    let (p, _) = project(&mut c, "eps", 10, 4);
    let (j, _) = db::submit_job(&mut c, &p, &req("c", json!({})), &limits(), FAKE).unwrap();
    let id = j["id"].as_str().unwrap();
    // A dead executor persisted dispatch intent and never called the provider.
    let mut tx = db::write_tx(&mut c, true).unwrap();
    db::apply_event(&mut tx, id, &assetq::model::Event::Claimed { owner: "dead".into(), fence: 1 }, 60_000, 0).unwrap();
    db::apply_event(&mut tx, id, &assetq::model::Event::DispatchIntended { fence: 1, effect_id: "dead-effect".into() }, 60_000, 0).unwrap();
    db::commit_write(tx).unwrap();
    c.execute("UPDATE jobs SET lease_until = clock_timestamp() - interval '1 second' WHERE id = $1", &[&id]).unwrap();
    let job = drive(&mut c, &ctx, "eps", id, phase("succeeded"));
    assert_eq!(f.fake().submissions(), 1, "exactly one provider submission after confirmed absence");
    assert_eq!(job["submissions"], 2, "one recorded intent was reconciled as never received");
    let ev = event_types(&mut c, "eps", id);
    let pos = |t: &str| ev.iter().position(|e| e == t).unwrap_or_else(|| panic!("{t} missing in {ev:?}"));
    assert!(pos("lease_expired") < pos("submit_uncertain"));
    assert!(pos("submit_uncertain") < pos("reconciled"));
    db::audit_job(&mut c, id).unwrap();
}

#[test]
fn lost_reply_adopts_original_operation() {
    let f = fixture();
    let ctx = f.ctx(1 << 30);
    let mut c = f.client();
    let (p, _) = project(&mut c, "zeta", 10, 4);
    f.fault("lose_reply");
    let (j, _) = db::submit_job(&mut c, &p, &req("l", json!({"report_usage": false})), &limits(), FAKE).unwrap();
    let id = j["id"].as_str().unwrap();
    let job = drive(&mut c, &ctx, "zeta", id, phase("succeeded"));
    assert_eq!(f.fake().submissions(), 1, "no blind second paid submission");
    assert_eq!(job["submissions"], 1);
    assert_eq!(job["usage"]["credits"], Value::Null);
    assert_eq!(job["usage"]["unknown_reason"], "provider did not report usage");
    let ev = event_types(&mut c, "zeta", id);
    assert!(ev.contains(&"submit_uncertain".to_string()) && ev.contains(&"reconciled".to_string()));
    let op: Option<String> = c.query_one("SELECT op_id FROM effects WHERE job_id = $1", &[&id]).unwrap().get(0);
    assert_eq!(op.as_deref(), job["provider_operation"].as_str());
    db::audit_job(&mut c, id).unwrap();
}

#[test]
fn partial_import_resumes_and_full_store_preserves_work() {
    let f = fixture();
    let mut c = f.client();
    let (p, _) = project(&mut c, "eta", 10, 4);
    // Partial import: the second output stream breaks once.
    let ctx = f.ctx(1 << 30);
    f.fault("fail_fetch");
    let (j, _) = db::submit_job(&mut c, &p, &req("p", json!({"outputs": 2})), &limits(), FAKE).unwrap();
    let id = j["id"].as_str().unwrap();
    let job = drive(&mut c, &ctx, "eta", id, |j| j["reason"].as_str().is_some_and(|r| r.contains("output-1")));
    assert_eq!(job["phase"], "active");
    assert_eq!(db::job_result(&mut c, "eta", id).unwrap()["status"], "incomplete");
    let objs: i64 = c.query_one("SELECT count(*) FROM objects WHERE job_id = $1", &[&id]).unwrap().get(0);
    assert_eq!(objs, 1, "the first output stays committed; no partial object");
    let job = drive(&mut c, &ctx, "eta", id, phase("succeeded"));
    assert_eq!(job["submissions"], 1);
    assert_eq!(db::job_result(&mut c, "eta", id).unwrap()["manifest"]["items"].as_array().unwrap().len(), 2);
    let tmp_left = std::fs::read_dir(f.dir.join("data/tmp")).unwrap().count();
    assert_eq!(tmp_left, 0);

    // Full store: the import pauses with a reason; committed objects stay.
    let used = db::used_bytes(&mut c).unwrap();
    let full = f.ctx(used + 10);
    let (j2, _) = db::submit_job(&mut c, &p, &req("full", json!({})), &limits(), FAKE).unwrap();
    let id2 = j2["id"].as_str().unwrap();
    let job2 = drive(&mut c, &full, "eta", id2, |j| j["reason"].as_str().is_some_and(|r| r.contains("store_full")));
    assert_eq!(job2["phase"], "active");
    for o in db::job_result(&mut c, "eta", id).unwrap()["manifest"]["items"].as_array().unwrap() {
        full.store.verify(o["sha256"].as_str().unwrap(), o["size"].as_u64().unwrap()).unwrap();
    }
    // With the quota at its limit a new job stops before any provider effect.
    let at_limit = f.ctx(used);
    let (j3, _) = db::submit_job(&mut c, &p, &req("full2", json!({})), &limits(), FAKE).unwrap();
    let id3 = j3["id"].as_str().unwrap();
    let before = f.fake().submissions();
    let job3 = drive(&mut c, &at_limit, "eta", id3, |j| j["reason"].as_str().is_some_and(|r| r.starts_with("store_full")));
    assert_eq!(job3["submissions"], 0);
    assert_eq!(f.fake().submissions(), before);
    // Space returns: both resume and complete.
    let roomy = f.ctx(1 << 30);
    c.execute("UPDATE jobs SET wake_at = now()", &[]).unwrap();
    drive(&mut c, &roomy, "eta", id2, phase("succeeded"));
    drive(&mut c, &roomy, "eta", id3, phase("succeeded"));
}

// R6: blocking fetch/read must outlive a lease without starving import;
// an actually expired lease must still refuse metadata and manifest commits.
#[test]
fn slow_import_renews_live_leases_without_reviving_expired_ones() {
    struct SlowProvider {
        fake: FakeProvider,
        expire: Option<String>,
        delay: Duration,
    }
    struct SlowRead {
        inner: Box<dyn std::io::Read + Send>,
        expire: Option<String>,
        op: String,
        first: bool,
        delay: Duration,
    }
    impl std::io::Read for SlowRead {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.first {
                self.first = false;
                if let Some(url) = &self.expire {
                    db::connect(url)
                        .unwrap()
                        .execute(
                            "UPDATE jobs SET lease_until = clock_timestamp() - interval '1 second' WHERE state->>'op_id' = $1",
                            &[&self.op],
                        )
                        .unwrap();
                }
                std::thread::sleep(self.delay);
            }
            self.inner.read(buf)
        }
    }
    impl Provider for SlowProvider {
        fn name(&self) -> &str {
            self.fake.name()
        }
        fn submit(&self, effect: &str, request: &Value) -> Submit {
            self.fake.submit(effect, request)
        }
        fn lookup(&self, effect: &str) -> Result<Option<String>, String> {
            self.fake.lookup(effect)
        }
        fn poll(&self, op: &str) -> Result<Poll, String> {
            self.fake.poll(op)
        }
        fn cancel(&self, op: &str) -> Result<bool, String> {
            self.fake.cancel(op)
        }
        fn fetch(&self, op: &str, index: usize) -> Result<Output, String> {
            std::thread::sleep(self.delay);
            let out = self.fake.fetch(op, index)?;
            Ok(Output {
                evidence: out.evidence,
                content_type: out.content_type,
                reader: Box::new(SlowRead {
                    inner: out.reader,
                    expire: self.expire.clone(),
                    op: op.into(),
                    first: true,
                    delay: self.delay,
                }),
            })
        }
    }
    for expire in [false, true] {
        let f = fixture();
        let mut c = f.client();
        let (p, _) = project(&mut c, "slow", 10, 1);
        let mut ctx = f.ctx(1 << 30);
        // Native metadata scans can exceed 250ms. Preserve both slow boundaries
        // and forced-expiry assertions while calibrating to this real fixture.
        let began = Instant::now();
        db::commit_write(db::write_tx(&mut c, true).unwrap()).unwrap();
        let guarded_ms = began.elapsed().as_millis() as u64;
        ctx.cfg.lease_ms = guarded_ms.saturating_mul(8).max(250);
        assert!(ctx.cfg.lease_ms <= 10_000, "guarded fixture exceeds the bounded calibration budget");
        let delay = Duration::from_millis(ctx.cfg.lease_ms + 150);
        eprintln!("R6 fixture: guarded_ms={guarded_ms} lease_ms={} each_slow_step_ms={}", ctx.cfg.lease_ms, delay.as_millis());
        ctx.providers = vec![Arc::new(SlowProvider { fake: f.fake(), expire: expire.then(|| f.url.clone()), delay })];
        let (j, _) = db::submit_job(&mut c, &p, &req("slow", json!({})), &limits(), FAKE).unwrap();
        let id = j["id"].as_str().unwrap();
        drive(&mut c, &ctx, &p.project, id, phase("active"));
        exec::tick(&mut c, &ctx, "slow-importer").unwrap();
        let j = db::get_job(&mut c, &p.project, id).unwrap();
        if expire {
            assert_eq!(j["phase"], "active");
            let objects: i64 = c.query_one("SELECT count(*) FROM objects WHERE job_id = $1", &[&id]).unwrap().get(0);
            assert_eq!(objects, 0, "an expired lease cannot commit imported metadata");
            assert_eq!(db::job_result(&mut c, &p.project, id).unwrap()["status"], "incomplete");
        } else {
            assert_eq!(j["phase"], "succeeded", "fetch and stream each exceed the calibrated lease");
            assert_eq!(j["attempts"], 2);
            let result = db::job_result(&mut c, &p.project, id).unwrap();
            let item = &result["manifest"]["items"][0];
            ctx.store.verify(item["sha256"].as_str().unwrap(), item["size"].as_u64().unwrap()).unwrap();
        }
        assert_eq!(j["submissions"], 1);
        assert_eq!(f.fake().submissions(), 1);
        db::audit_job(&mut c, id).unwrap();
    }
}

// R4/R7: an uncertain cancel is still external work, so a max-one project
// cannot admit a replacement until the same operation is observed terminal.
#[test]
fn uncertain_cancellation_retains_capacity_until_terminal_observation() {
    for outcome in ["done", "failed", "confirmed"] {
        let f = fixture();
        let ctx = f.ctx(1 << 30);
        let mut c = f.client();
        let (p, _) = project(&mut c, "bounded", 10, 1);
        let (j, _) = db::submit_job(&mut c, &p, &req("first", json!({"fail": outcome == "failed"})), &limits(), FAKE).unwrap();
        let id = j["id"].as_str().unwrap();
        drive(&mut c, &ctx, &p.project, id, phase("waiting"));
        f.fault("cancel_unconfirmed");
        db::cancel_job(&mut c, &p.project, id).unwrap();
        exec::tick(&mut c, &ctx, "cancel-observer").unwrap();
        assert_eq!(db::get_job(&mut c, &p.project, id).unwrap()["cancel"], "uncertain");
        let replacement = req("replacement", json!({}));
        assert_eq!(db::submit_job(&mut c, &p, &replacement, &limits(), FAKE).unwrap_err().code, "concurrency_limit");
        c.execute("UPDATE projects SET max_active_jobs = 2, budget_credits = 1 WHERE id = $1", &[&p.project]).unwrap();
        assert_eq!(db::submit_job(&mut c, &p, &replacement, &limits(), FAKE).unwrap_err().code, "budget_exhausted");
        c.execute("UPDATE projects SET max_active_jobs = 1, budget_credits = 10 WHERE id = $1", &[&p.project]).unwrap();
        let j = db::get_job(&mut c, &p.project, id).unwrap();
        assert_eq!(j["phase"], "waiting");
        assert_eq!(j["leased"], false);
        assert_eq!(j["usage"]["credits"], Value::Null);
        let op = j["provider_operation"].clone();
        // New context/session observes the persisted operation after restart.
        let mut c = f.client();
        if outcome == "confirmed" {
            let mut tx = db::write_tx(&mut c, true).unwrap();
            db::apply_event(
                &mut tx,
                id,
                &assetq::model::Event::CancelConfirmed { op_id: op.as_str().unwrap().into() },
                ctx.cfg.lease_ms,
                0,
            )
            .unwrap();
            db::commit_write(tx).unwrap();
        } else {
            exec::tick(&mut c, &f.ctx(1 << 30), "restarted-observer").unwrap();
        }
        let j = db::get_job(&mut c, &p.project, id).unwrap();
        assert_eq!(j["phase"], if outcome == "failed" { "failed" } else { "cancelled" });
        assert_eq!(j["provider_operation"], op);
        assert_eq!(j["submissions"], 1);
        if outcome == "done" {
            assert_eq!(j["cancel"], "locally_stopped");
            assert_eq!(j["usage"]["credits"], 1);
        }
        assert!(db::job_result(&mut c, &p.project, id).unwrap()["manifest"].is_null());
        assert_eq!(f.fake().submissions(), 1);
        assert!(db::submit_job(&mut c, &p, &replacement, &limits(), FAKE).unwrap().1);
        db::audit_job(&mut c, id).unwrap();
    }
}

#[test]
fn cancellation_states_and_unknown_charges() {
    let f = fixture();
    let ctx = f.ctx(1 << 30);
    let mut c = f.client();
    let (p, _) = project(&mut c, "theta", 10, 4);
    let (j, _) = db::submit_job(&mut c, &p, &req("q", json!({})), &limits(), FAKE).unwrap();
    let id = j["id"].as_str().unwrap();
    let job = db::cancel_job(&mut c, "theta", id).unwrap();
    assert_eq!((job["phase"].as_str(), job["cancel"].as_str()), (Some("cancelled"), Some("locally_stopped")));

    let (j, _) = db::submit_job(&mut c, &p, &req("w", json!({"polls": 100000})), &limits(), FAKE).unwrap();
    let id = j["id"].as_str().unwrap();
    drive(&mut c, &ctx, "theta", id, phase("waiting"));
    let job = db::cancel_job(&mut c, "theta", id).unwrap();
    assert_eq!((job["phase"].as_str(), job["cancel"].as_str()), (Some("waiting"), Some("requested")));
    let job = drive(&mut c, &ctx, "theta", id, phase("cancelled"));
    assert_eq!(job["cancel"], "provider_confirmed");

    f.fault("cancel_unconfirmed");
    let (j, _) = db::submit_job(&mut c, &p, &req("u", json!({"polls": 100000})), &limits(), FAKE).unwrap();
    let id = j["id"].as_str().unwrap();
    drive(&mut c, &ctx, "theta", id, phase("waiting"));
    db::cancel_job(&mut c, "theta", id).unwrap();
    let job = drive(&mut c, &ctx, "theta", id, |j| j["cancel"] == "uncertain");
    assert_eq!(job["cancel"], "uncertain");
    assert_eq!(job["usage"]["credits"], Value::Null);
    assert!(job["usage"]["unknown_reason"].as_str().unwrap().starts_with("charge unknown"));
    // Unknown charges keep their reservation in the budget.
    let st = db::status(&mut c, "theta", 1 << 30, FAKE).unwrap();
    assert_eq!(st["providers"][0]["health"], "unknown");
    assert_eq!(st["backup"]["state"], "unknown");
}

#[test]
fn waiting_jobs_release_executor_capacity() {
    let f = fixture();
    let ctx = f.ctx(1 << 30);
    let mut c = f.client();
    let (p, _) = project(&mut c, "iota", 10, 10);
    let ids: Vec<String> = (0..3)
        .map(|i| {
            db::submit_job(&mut c, &p, &req(&format!("cap{i}"), json!({"polls": 100000})), &limits(), FAKE).unwrap().0["id"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    // One executor identity serves all three: each wait releases its lease.
    for id in &ids {
        drive(&mut c, &ctx, "iota", id, phase("waiting"));
    }
    for id in &ids {
        let j = db::get_job(&mut c, "iota", id).unwrap();
        assert_eq!((j["phase"].as_str(), j["leased"].as_bool()), (Some("waiting"), Some(false)));
    }
    let leased: i64 = c.query_one("SELECT count(*) FROM jobs WHERE lease_owner IS NOT NULL", &[]).unwrap().get(0);
    assert_eq!(leased, 0);
}

#[test]
fn registry_recommendations_and_comparisons() {
    let f = fixture();
    let mut c = f.client();
    let (member, _) = project(&mut c, "kappa", 10, 4);
    let owner_token = db::create_token(&mut c, "kappa", "owner").unwrap();
    let owner = db::authenticate(&mut c, &owner_token).unwrap();
    let methods = db::list_methods(&mut c, "kappa").unwrap();
    assert_eq!(methods.as_array().unwrap().len(), 5);
    assert_eq!(db::resolve_method(&mut c, "kappa", "image.worker.v1@2").unwrap().2, assetq::image_worker::legacy_method());
    assert_eq!(db::resolve_method(&mut c, "kappa", "image.worker.v1@3").unwrap().2, assetq::image_worker::method());
    let mut spec = db::builtin_methods()[0].3.clone();
    spec["pipeline_revision"] = json!("2");
    let body = json!({"id": "synthetic.fake", "status": "experimental", "spec": spec});
    assert_eq!(db::create_method_version(&mut c, &member, &body).unwrap_err().code, "forbidden");
    let v2 = db::create_method_version(&mut c, &owner, &body).unwrap();
    assert_eq!(v2["version"], 2);
    let mut bad = body.clone();
    bad["spec"]["steps"] = json!(["shell.exec"]);
    assert_eq!(db::create_method_version(&mut c, &owner, &bad).unwrap_err().code, "bad_method");
    // Unversioned requests now resolve to v2; a running v1 job keeps v1.
    assert_eq!(db::resolve_method(&mut c, "kappa", "synthetic.fake").unwrap().1, 2);

    let none = db::recommendations(&mut c, "kappa", "prop-image", &json!({"style":"stylized"})).unwrap();
    assert_eq!(none["recommended"], Value::Null);
    assert!(none["none_reason"].is_string());
    let rec = db::propose_recommendation(
        &mut c,
        &member,
        &json!({
        "use_case": "prop-image", "method": "synthetic.fake@2", "constraints": {"style": "stylized"},
        "evidence": {"comparison": "fixture"}, "observed_at": "2026-11-01T00:00:00Z"}),
    )
    .unwrap();
    assert_eq!(rec["state"], "proposed");
    assert_eq!(
        db::recommendations(&mut c, "kappa", "prop-image", &json!({"style":"stylized"})).unwrap()["recommended"],
        Value::Null,
        "proposals are not recommendations"
    );
    let id = rec["id"].as_str().unwrap();
    assert_eq!(db::promote_recommendation(&mut c, &member, id).unwrap_err().code, "forbidden");
    db::promote_recommendation(&mut c, &owner, id).unwrap();
    assert_eq!(db::recommendations(&mut c, "kappa", "prop-image", &json!({"style":"realistic"})).unwrap()["recommended"], Value::Null);
    assert_eq!(
        db::recommendations(&mut c, "kappa", "prop-image", &json!({"style":"stylized"})).unwrap()["recommended"]["method"],
        "synthetic.fake@2"
    );

    db::add_comparison(
        &mut c,
        &member,
        &json!({"use_case": "prop-image", "dataset_version": "crate-v1", "evaluator_version": "eval-1",
        "method_versions": ["synthetic.fake@1", "synthetic.fake@2"], "results": {"synthetic.fake@1": {"failure_rate": 0}},
        "missing_cases": ["synthetic.fake@2: no samples"]}),
    )
    .unwrap();
    assert_eq!(
        db::add_comparison(
            &mut c,
            &member,
            &json!({"use_case": "x", "dataset_version": "d", "evaluator_version": "e", "method_versions": ["nope@1"], "results": {}})
        )
        .unwrap_err()
        .code,
        "bad_comparison"
    );
    let list = db::list_comparisons(&mut c, "kappa", Some("prop-image")).unwrap();
    assert_eq!(list[0]["missing_cases"][0], "synthetic.fake@2: no samples");
}

// R3: an owner token cannot inspect or promote another project's evidence.
#[test]
fn registry_scope_and_expired_fences() {
    let f = fixture();
    let mut c = f.client();
    let (a, _) = project(&mut c, "private-a", 10, 4);
    let (b, _) = project(&mut c, "private-b", 10, 4);
    let owner_a = Principal { project: a.project.clone(), role: "owner".into() };
    let owner_b = Principal { project: b.project.clone(), role: "owner".into() };
    let body = json!({"id": "private-method", "spec": db::builtin_methods()[0].3});
    db::create_method_version(&mut c, &owner_a, &body).unwrap();
    assert!(db::resolve_method(&mut c, &b.project, "private-method@1").is_err());
    assert_eq!(db::list_methods(&mut c, &b.project).unwrap().as_array().unwrap().len(), 5);
    let rec = db::propose_recommendation(
        &mut c,
        &a,
        &json!({"use_case": "private", "method": "private-method@1",
        "evidence": {"comparison": "private"}, "observed_at": "2026-11-01T00:00:00Z"}),
    )
    .unwrap();
    let rid = rec["id"].as_str().unwrap();
    assert_eq!(db::promote_recommendation(&mut c, &owner_b, rid).unwrap_err().status, 404);
    assert!(db::recommendations(&mut c, &b.project, "private", &json!({})).unwrap()["records"].as_array().unwrap().is_empty());
    db::add_comparison(
        &mut c,
        &a,
        &json!({"use_case":"private", "dataset_version":"d1", "evaluator_version":"e1",
        "method_versions":["private-method@1"], "results": {"state":"unknown"}, "missing_cases":["no run"]}),
    )
    .unwrap();
    assert!(db::list_comparisons(&mut c, &b.project, None).unwrap().as_array().unwrap().is_empty());
    // R6: the deadline, not only a successor's claim, fences a stale writer.
    let (j, _) = db::submit_job(&mut c, &a, &req("expired", json!({})), &limits(), FAKE).unwrap();
    let id = j["id"].as_str().unwrap();
    let mut tx = db::write_tx(&mut c, true).unwrap();
    db::apply_event(&mut tx, id, &assetq::model::Event::Claimed { owner: "old".into(), fence: 1 }, 60_000, 0).unwrap();
    db::commit_write(tx).unwrap();
    c.execute("UPDATE jobs SET lease_until = clock_timestamp() - interval '1 second' WHERE id = $1", &[&id]).unwrap();
    let mut tx = db::write_tx(&mut c, true).unwrap();
    assert_eq!(
        db::apply_event(&mut tx, id, &assetq::model::Event::DispatchIntended { fence: 1, effect_id: "stale".into() }, 1000, 0)
            .unwrap_err()
            .code,
        "stale_fence"
    );
    drop(tx);
    let ctx = f.ctx(1 << 30);
    drive(&mut c, &ctx, &a.project, id, phase("succeeded"));
    let attempts: i64 = c.query_one("SELECT count(DISTINCT id) FROM attempts WHERE job_id = $1", &[&id]).unwrap().get(0);
    assert!(attempts >= 2);
    db::audit_job(&mut c, id).unwrap();
    // R7: an elapsed execution budget cancels without a provider effect.
    let (j, _) = db::submit_job(&mut c, &a, &req("deadline", json!({})), &limits(), FAKE).unwrap();
    let id = j["id"].as_str().unwrap();
    c.execute("UPDATE jobs SET created_at = now() - interval '2 days' WHERE id = $1", &[&id]).unwrap();
    let done = drive(&mut c, &ctx, &a.project, id, phase("cancelled"));
    assert_eq!(done["submissions"], 0);
}

// R7: a reservation is exclusive across database sessions and quota failure
// preserves committed work, including when crash leftovers consume temp space.
#[test]
fn admission_reservation_and_metadata_limit() {
    let f = fixture();
    let mut c = f.client();
    let mut other = f.client();
    db::with_admission(&mut c, 1 << 30, |_| {
        assert_eq!(db::with_admission(&mut other, 1 << 30, |_| Ok(())).unwrap_err().code, "admission_busy");
        Ok(())
    })
    .unwrap();
    assert_eq!(db::with_admission(&mut other, 0, |_| Ok(())).unwrap_err().code, "metadata_full");
    db::with_admission(&mut other, 1 << 30, |_| Ok(())).unwrap();
}

// ---------------------------------------------------------------- HTTP and process level

fn http() -> ureq::Agent {
    ureq::Agent::config_builder().http_status_as_error(false).build().into()
}

#[test]
fn http_streaming_upload_download_and_scope() {
    let f = fixture();
    let mut cfg = f.config(1 << 30);
    cfg.max_upload_bytes = 4 << 20;
    let ctx = Arc::new(Ctx::new(cfg).unwrap());
    let mut c = f.client();
    let (_, ta) = project(&mut c, "lam", 10, 4);
    let (_, tb) = project(&mut c, "mu", 10, 4);
    let server = Arc::new(api::bind("127.0.0.1:0").unwrap());
    let base = format!("http://{}", server.server_addr().to_ip().unwrap());
    api::serve(ctx.clone(), server, 4);
    let h = http();
    let auth = |t: &str| format!("Bearer {t}");
    // 3 MiB streamed upload.
    let body: Vec<u8> = (0..3u32 << 20).map(|i| (i % 251) as u8).collect();
    let mut r = h
        .post(format!("{base}/v1/uploads"))
        .header("Authorization", auth(&ta))
        .header("Content-Type", "image/png")
        .header("X-Asset-Name", "../../etc/passwd")
        .send(&body[..])
        .unwrap();
    assert_eq!(r.status().as_u16(), 201);
    let o: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
    assert_eq!(o["sha256"], db::sha256_hex(&body));
    let oid = o["id"].as_str().unwrap();
    // The caller name is metadata; the stored path is the content address.
    assert!(f.dir.join("data/objects").join(&db::sha256_hex(&body)[..2]).exists());
    assert!(!f.dir.join("etc").exists());
    let mut r = h.get(format!("{base}/v1/objects/{oid}/content")).header("Authorization", auth(&ta)).call().unwrap();
    assert_eq!(r.headers()["X-Content-Sha256"].to_str().unwrap(), o["sha256"].as_str().unwrap());
    assert_eq!(r.headers()["Content-Disposition"], "attachment");
    assert_eq!(r.headers()["Cache-Control"], "private, no-store");
    let got = r.body_mut().with_config().limit(10 << 20).read_to_vec().unwrap();
    assert_eq!(db::sha256_hex(&got), o["sha256"].as_str().unwrap());
    // Foreign project, missing token, bad type, oversize.
    assert_eq!(h.get(format!("{base}/v1/objects/{oid}/content")).header("Authorization", auth(&tb)).call().unwrap().status().as_u16(), 404);
    assert_eq!(h.get(format!("{base}/v1/objects/{oid}")).call().unwrap().status().as_u16(), 401);
    assert_eq!(
        h.post(format!("{base}/v1/uploads"))
            .header("Authorization", auth(&ta))
            .header("Content-Type", "text/html")
            .send(&b"<x>"[..])
            .unwrap()
            .status()
            .as_u16(),
        415
    );
    let big = vec![0u8; 5 << 20];
    assert_eq!(
        h.post(format!("{base}/v1/uploads"))
            .header("Authorization", auth(&ta))
            .header("Content-Type", "image/png")
            .send(&big[..])
            .unwrap()
            .status()
            .as_u16(),
        413
    );
    // Path traversal in routes never reaches the filesystem.
    assert_eq!(
        h.get(format!("{base}/v1/objects/..%2F..%2Fetc/content")).header("Authorization", auth(&ta)).call().unwrap().status().as_u16(),
        404
    );
    // Attach the upload to a job and page through objects.
    let mut r = h.post(format!("{base}/v1/jobs")).header("Authorization", auth(&ta)).header("Content-Type", "application/json")
        .send(json!({"method": "synthetic.fake@1", "operation": "synthetic.generate", "idempotency_key": "h1", "attachments": [oid], "budget": {"max_credits": 1}}).to_string()).unwrap();
    assert_eq!(r.status().as_u16(), 201, "{}", r.body_mut().read_to_string().unwrap());
    let mut r = h.post(format!("{base}/v1/jobs")).header("Authorization", auth(&tb)).header("Content-Type", "application/json")
        .send(json!({"method": "synthetic.fake@1", "operation": "synthetic.generate", "idempotency_key": "h1", "attachments": [oid], "budget": {"max_credits": 1}}).to_string()).unwrap();
    assert_eq!(r.status().as_u16(), 404, "{}", r.body_mut().read_to_string().unwrap());
    let mut r = h.get(format!("{base}/v1/objects?limit=1")).header("Authorization", auth(&ta)).call().unwrap();
    let page: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
}

struct Service {
    child: Child,
    url: String,
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_assetq")
}

fn env_cmd(f: &Fixture) -> Command {
    let mut cmd = Command::new(bin());
    cmd.env("ASSETQ_DATABASE_URL", &f.url)
        .env("ASSETQ_DATA_ROOT", f.dir.join("data"))
        .env("ASSETQ_FAKE_PROVIDER_DIR", f.dir.join("fake"))
        .env("ASSETQ_POLL_MS", "50")
        .env("ASSETQ_LEASE_MS", "2000")
        .env("ASSETQ_EXECUTORS", "1");
    cmd
}

fn start(f: &Fixture) -> Service {
    let mut child = env_cmd(f).args(["serve", "--listen", "127.0.0.1:0"]).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn().unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap()).read_line(&mut line).unwrap();
    let addr = line.trim().strip_prefix("assetq listening on ").unwrap_or_else(|| panic!("unexpected: {line}")).to_string();
    Service { child, url: format!("http://{addr}") }
}

// R7: configured bounds must reach the native service and administrative paths.
#[test]
fn resource_bounds_floor_refuses_native_upload() {
    let f = fixture();
    let (_, token) = project(&mut f.client(), "floor", 10, 4);
    let mut child = env_cmd(&f)
        .env("ASSETQ_FREE_SPACE_FLOOR_BYTES", i64::MAX.to_string())
        .args(["serve", "--listen", "127.0.0.1:0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap()).read_line(&mut line).unwrap();
    let s = Service { child, url: format!("http://{}", line.trim().strip_prefix("assetq listening on ").unwrap()) };
    let r = http()
        .post(format!("{}/v1/uploads", s.url))
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "text/plain")
        .send(b"preserve floor")
        .unwrap();
    assert_eq!(r.status().as_u16(), 507, "R7: native service must enforce configured free-space floor");
    assert_eq!(f.client().query_one("SELECT count(*) FROM objects", &[]).unwrap().get::<_, i64>(0), 0);
}

#[test]
fn resource_bounds_migration_refuses_exhaustion() {
    let f = fixture();
    let out = env_cmd(&f).env("ASSETQ_METADATA_QUOTA_BYTES", "0").arg("migrate").output().unwrap();
    assert!(!out.status.success(), "R7: even repeat migrations must enforce metadata bounds");
    assert!(String::from_utf8_lossy(&out.stderr).contains("metadata_full"));
    let name = format!("assetq_m_{}", uuid::Uuid::new_v4().simple());
    let mut c = f.client();
    c.batch_execute(&format!("CREATE DATABASE {name}")).unwrap();
    let url = format!("{} dbname={name}", std::env::var("ASSETQ_TEST_PG").unwrap());
    let out = env_cmd(&f).env("ASSETQ_DATABASE_URL", &url).env("ASSETQ_METADATA_QUOTA_BYTES", "0").arg("migrate").output().unwrap();
    assert!(!out.status.success());
    let mut empty = db::connect(&url).unwrap();
    assert_eq!(empty.query_one("SELECT to_regclass('schema_version')::text", &[]).unwrap().get::<_, Option<String>>(0), None);
    db::migrate(&mut empty).unwrap();
    assert_eq!(empty.query_one("SELECT version FROM schema_version", &[]).unwrap().get::<_, i32>(0), 1);
}

// R7: a native process cannot stream while a surviving OS reservation is held,
// even after the reserving database session has ended.
#[test]
fn resource_bounds_native_process_respects_surviving_store_lock() {
    let f = fixture();
    let (_, token) = project(&mut f.client(), "process", 10, 4);
    let ctx = f.ctx(1 << 30);
    let session = f.client();
    let lock = ctx.store.writer_lock().unwrap();
    drop(session);
    let service = start(&f);
    let upload = || {
        http()
            .post(format!("{}/v1/uploads", service.url))
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "text/plain")
            .send(b"process lock")
            .unwrap()
    };
    assert_eq!(upload().status().as_u16(), 507);
    drop(lock);
    assert_eq!(upload().status().as_u16(), 201);
    drop(service);
    let mut cfg = f.config(1 << 30);
    cfg.free_space_floor_bytes = i64::MAX as u64;
    let blocked = Ctx::new(cfg).unwrap();
    let mut c = f.client();
    let p = db::authenticate(&mut c, &token).unwrap();
    let (job, _) = db::submit_job(&mut c, &p, &req("dispatch-floor", json!({})), &limits(), FAKE).unwrap();
    exec::tick(&mut c, &blocked, "floor").unwrap();
    assert_eq!(db::get_job(&mut c, &p.project, job["id"].as_str().unwrap()).unwrap()["submissions"], 0);
    assert_eq!(c.query_one("SELECT count(*) FROM effects", &[]).unwrap().get::<_, i64>(0), 0);
}

#[test]
fn resource_bounds_recovery_refuses_exhaustion() {
    let f = fixture();
    let mut c = f.client();
    let (p, _) = project(&mut c, "recovery", 10, 4);
    db::submit_job(&mut c, &p, &req("recover", json!({})), &limits(), FAKE).unwrap();
    let mut cfg = f.config(1 << 30);
    cfg.metadata_quota_bytes = 0;
    let ctx = Ctx::new(cfg).unwrap();
    assert_eq!(exec::tick(&mut c, &ctx, "bounded").unwrap_err().code, "metadata_full");
    assert_eq!(c.query_one("SELECT count(*) FROM attempts", &[]).unwrap().get::<_, i64>(0), 0);
}

// R7/R4: admission closure cannot spend recovery capacity on repeated claims,
// and cancellation/reconciliation retain the original operation until terminal.
#[test]
fn resource_bounds_recovery_reserve_preserves_operations() {
    let f = fixture();
    let mut c = f.client();
    let (p, _) = project(&mut c, "reserve", 10, 4);
    let (job, _) = db::submit_job(&mut c, &p, &req("paid", json!({"polls": 50})), &limits(), FAKE).unwrap();
    let id = job["id"].as_str().unwrap();
    let waiting = drive(&mut c, &f.ctx(1 << 30), &p.project, id, phase("waiting"));
    let (queued, _) = db::submit_job(&mut c, &p, &req("queued", json!({})), &limits(), FAKE).unwrap();
    let mut cfg = f.config(1 << 30);
    cfg.metadata_quota_bytes =
        db::metadata_bytes(&mut c).unwrap() + cfg.metadata_admission_headroom_bytes + cfg.metadata_recovery_headroom_bytes / 2;
    db::configure(&mut c, &cfg).unwrap();
    assert_eq!(db::create_token(&mut c, &p.project, "member").unwrap_err().code, "metadata_full");
    assert_eq!(db::observe(&mut c, "health", "test", &json!({})).unwrap_err().code, "metadata_full");
    assert_eq!(db::migrate(&mut c).unwrap_err().code, "metadata_full");
    assert_eq!(db::metadata_status(&mut c)["admission"], "exhausted");
    assert_eq!(db::metadata_status(&mut c)["recovery"], "available");
    let before: i64 = c.query_one("SELECT count(*) FROM attempts", &[]).unwrap().get(0);
    let ctx = Ctx::new(cfg.clone()).unwrap();
    for _ in 0..5 {
        exec::tick(&mut c, &ctx, "reserve").unwrap();
    }
    assert_eq!(c.query_one("SELECT count(*) FROM attempts", &[]).unwrap().get::<_, i64>(0), before);
    db::cancel_job(&mut c, &p.project, id).unwrap();
    let cancelled = drive(&mut c, &ctx, &p.project, id, phase("cancelled"));
    assert_eq!(cancelled["provider_operation"], waiting["provider_operation"]);
    assert_eq!(cancelled["submissions"], 1);
    db::audit_job(&mut c, id).unwrap();
    cfg.metadata_quota_bytes = 0;
    db::configure(&mut c, &cfg).unwrap();
    let queued_id = queued["id"].as_str().unwrap();
    assert_eq!(db::cancel_job(&mut c, &p.project, queued_id).unwrap_err().code, "metadata_full");
    assert_eq!(db::metadata_status(&mut c)["recovery"], "exhausted");
    cfg.metadata_quota_bytes = 1 << 30;
    let restored = Ctx::new(cfg).unwrap();
    drive(&mut c, &restored, &p.project, queued_id, phase("succeeded"));
    db::audit_job(&mut c, queued_id).unwrap();
}

// R7: a second session must remeasure physical indexes/TOAST after the first
// writer commits, rather than reuse a pre-lock admission observation.
#[test]
fn resource_bounds_concurrent_metadata_counts_toast_and_indexes() {
    let f = fixture();
    let mut c = f.client();
    project(&mut c, "physical", 10, 4);
    let mut cfg = f.config(1 << 30);
    let before = db::metadata_bytes(&mut c).unwrap();
    cfg.metadata_quota_bytes = before + cfg.metadata_admission_headroom_bytes + cfg.metadata_recovery_headroom_bytes;
    db::configure(&mut c, &cfg).unwrap();
    let mut tx = db::write_tx(&mut c, false).unwrap();
    let value = json!({"data": (0..1024).map(|i| db::sha256_hex(format!("invented fixture {i}").as_bytes())).collect::<String>()});
    db::charge(&mut tx, value.to_string().len()).unwrap();
    tx.execute("INSERT INTO observations (kind, subject, value) VALUES ('fixture', 'toast', $1)", &[&value]).unwrap();
    let (ready, started) = std::sync::mpsc::channel();
    let (send, recv) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let mut other = db::connect_config(&cfg).unwrap();
        ready.send(()).unwrap();
        send.send(db::create_token(&mut other, "physical", "member").unwrap_err().code).unwrap();
    });
    started.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(recv.recv_timeout(Duration::from_millis(50)).is_err(), "concurrent metadata writer bypassed reservation");
    db::commit_write(tx).unwrap();
    assert_eq!(recv.recv_timeout(Duration::from_secs(5)).unwrap(), "metadata_full");
    worker.join().unwrap();
    assert!(db::metadata_bytes(&mut c).unwrap() > before);
    let toast: i64 =
        c.query_one("SELECT pg_total_relation_size(reltoastrelid) FROM pg_class WHERE oid = 'observations'::regclass", &[]).unwrap().get(0);
    assert!(toast > 8192, "fixture must create real TOAST and index allocations");
    assert_eq!(c.query_one("SELECT count(*) FROM tokens", &[]).unwrap().get::<_, i64>(0), 1);
    db::configure(&mut c, &f.config(1 << 30)).unwrap();
    assert_eq!(db::observe(&mut c, "fixture", "oversized", &json!({"data": "x".repeat(1 << 20)})).unwrap_err().code, "metadata_full");
    assert_eq!(c.query_one("SELECT count(*) FROM observations", &[]).unwrap().get::<_, i64>(0), 1);
}

// R6/R7: local generation has no provider effect identity; persisted progress
// must still finish using recovery headroom after admission closes.
#[test]
fn resource_bounds_local_progress_resumes_at_admission_ceiling() {
    use assetq::model::Event;
    let f = fixture();
    let mut c = f.client();
    let (p, _) = project(&mut c, "local", 10, 4);
    let owner = Principal { project: p.project.clone(), role: "owner".into() };
    let mut spec = db::builtin_methods()[0].3.clone();
    spec["steps"] = json!(["synthetic.generate", "validate"]);
    db::create_method_version(&mut c, &owner, &json!({"id": "local.fixture", "spec": spec})).unwrap();
    let mut request = req("local-recovery", json!({}));
    request["method"] = json!("local.fixture");
    let (job, _) = db::submit_job(&mut c, &p, &request, &limits(), FAKE).unwrap();
    let id = job["id"].as_str().unwrap();
    let ctx = f.ctx(1 << 30);
    let stored = ctx.store.put(&mut &b"retained local output"[..], 100, 100).unwrap();
    let mut tx = db::write_tx(&mut c, true).unwrap();
    db::apply_event(&mut tx, id, &Event::Claimed { owner: "fixture".into(), fence: 1 }, 60_000, 0).unwrap();
    db::insert_object(
        &mut tx,
        &db::NewObject {
            project: &p.project,
            sha256: &stored.sha256,
            size: stored.size,
            content_type: "application/x-assetq-synthetic",
            kind: "output",
            job_id: Some(id),
            import_key: Some("output-0"),
            provenance: json!({"import_key": "output-0"}),
            synthetic: true,
            retention_days: None,
        },
    )
    .unwrap();
    db::apply_event(&mut tx, id, &Event::ObjectImported { fence: 1, key: "output-0".into() }, 60_000, 0).unwrap();
    db::apply_event(&mut tx, id, &Event::StepCompleted { fence: 1, step: 0 }, 60_000, 0).unwrap();
    db::apply_event(&mut tx, id, &Event::Paused { fence: 1, reason: "invented restart fixture".into() }, 60_000, 0).unwrap();
    db::commit_write(tx).unwrap();
    let mut cfg = f.config(1 << 30);
    cfg.metadata_quota_bytes =
        db::metadata_bytes(&mut c).unwrap() + cfg.metadata_admission_headroom_bytes + cfg.metadata_recovery_headroom_bytes / 2;
    let resumed = Ctx::new(cfg).unwrap();
    let done = drive(&mut c, &resumed, &p.project, id, phase("succeeded"));
    assert_eq!(done["submissions"], 0);
    db::audit_job(&mut c, id).unwrap();
}

fn cli(f: &Fixture, s: &Service, token: &str, args: &[&str]) -> Value {
    let out = env_cmd(f).env("ASSETQ_URL", &s.url).env("ASSETQ_TOKEN", token).args(args).output().unwrap();
    assert!(out.status.success(), "assetq {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).unwrap()
}

fn wait_for(f: &Fixture, s: &Service, token: &str, job: &str, want: &str) -> Value {
    let start = Instant::now();
    loop {
        let j = cli(f, s, token, &["status", job]);
        if j["phase"] == want {
            return j;
        }
        assert!(start.elapsed() < Duration::from_secs(60), "job never reached {want}: {j}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn service_restart_recovers_job_through_cli() {
    let f = fixture();
    let admin = |args: &[&str]| {
        let out = env_cmd(&f).args(args).output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8(out.stdout).unwrap()
    };
    admin(&["migrate"]);
    admin(&["project", "nu", "--budget", "5", "--max-active", "4"]);
    let token = admin(&["token", "nu"]).trim().to_string();
    let s = start(&f);
    let input = f.dir.join("brief.txt");
    std::fs::write(&input, "wooden crate with blue metal corners").unwrap();
    let up = cli(&f, &s, &token, &["upload", input.to_str().unwrap(), "--type", "text/plain"]);
    let att = up["id"].as_str().unwrap();
    let submit = [
        "submit",
        "--method",
        "synthetic.fake",
        "--operation",
        "synthetic.generate",
        "--key",
        "restart-1",
        "--budget",
        "1",
        "--text",
        "crate",
        "--attach",
        att,
        "--set",
        "polls=40",
        "--set",
        "outputs=2",
    ];
    let j = cli(&f, &s, &token, &submit);
    assert_eq!(j["created"], true);
    let id = j["job"]["id"].as_str().unwrap().to_string();
    let waiting = wait_for(&f, &s, &token, &id, "waiting");
    let op = waiting["provider_operation"].as_str().unwrap().to_string();
    // Kill the service mid-wait (no graceful shutdown) and restart it.
    drop(s);
    let s = start(&f);
    let again = cli(&f, &s, &token, &submit);
    assert_eq!((again["created"].as_bool(), again["job"]["id"].as_str()), (Some(false), Some(id.as_str())));
    let done = wait_for(&f, &s, &token, &id, "succeeded");
    assert_eq!(done["provider_operation"], op.as_str(), "the original provider operation survived the restart");
    assert_eq!(done["method"], "synthetic.fake@1");
    assert_eq!(done["submissions"], 1);
    assert_eq!(f.fake().submissions(), 1);
    let result = cli(&f, &s, &token, &["result", &id]);
    let items = result["manifest"]["items"].as_array().unwrap().clone();
    assert_eq!(items.len(), 2);
    for it in &items {
        let out = f.dir.join(format!("{}.bin", it["import_key"].as_str().unwrap()));
        let d = cli(&f, &s, &token, &["download", it["object_id"].as_str().unwrap(), out.to_str().unwrap()]);
        assert_eq!(d["sha256"], it["sha256"]);
        assert!(std::fs::read_to_string(&out).unwrap().starts_with("SYNTHETIC ASSETQ OUTPUT"));
        let o = cli(&f, &s, &token, &["object", it["object_id"].as_str().unwrap()]);
        assert_eq!(o["provenance"]["parents"][0], att);
        assert_eq!(o["provenance"]["provider_operation"], op.as_str());
        assert_eq!(o["synthetic"], true);
        assert_eq!(o["acceptance"], "pending");
    }
    let ops = cli(&f, &s, &token, &["ops"]);
    assert_eq!(ops["jobs"]["by_phase"]["succeeded"], 1);
    assert_eq!(ops["backup"]["state"], "unknown");
    let audit = admin(&["audit", &id]);
    assert!(audit.contains("matches stored state"));
    let methods = cli(&f, &s, &token, &["methods"]);
    assert_eq!(methods.as_array().unwrap().len(), 5);
    let rec = cli(&f, &s, &token, &["recommend", "prop image"]);
    assert_eq!(rec["recommended"], Value::Null);
    // A corrupted download is refused and leaves no output file.
    let blob = f
        .dir
        .join("data/objects")
        .join(&items[0]["sha256"].as_str().unwrap()[..2])
        .join(&items[0]["sha256"].as_str().unwrap()[2..4])
        .join(items[0]["sha256"].as_str().unwrap());
    let mut perms = std::fs::metadata(&blob).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    std::fs::set_permissions(&blob, perms).unwrap();
    let len = std::fs::metadata(&blob).unwrap().len() as usize;
    std::fs::write(&blob, vec![b'x'; len]).unwrap();
    let bad_out = f.dir.join("bad.bin");
    let out = env_cmd(&f)
        .env("ASSETQ_URL", &s.url)
        .env("ASSETQ_TOKEN", &token)
        .args(["download", items[0]["object_id"].as_str().unwrap(), bad_out.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("hash mismatch"));
    assert!(!Path::new(&bad_out).exists());
}

#[test]
fn http_temp_full_preserves_existing_downloads() {
    let f = fixture();
    let mut cfg = f.config(16);
    cfg.max_upload_bytes = 16;
    cfg.temp_quota_bytes = 8;
    let ctx = Arc::new(Ctx::new(cfg).unwrap());
    let (_, token) = project(&mut f.client(), "quota", 10, 4);
    let server = Arc::new(api::bind("127.0.0.1:0").unwrap());
    let base = format!("http://{}", server.server_addr().to_ip().unwrap());
    api::serve(ctx, server, 2);
    let h = http();
    let upload = |bytes: &[u8]| {
        h.post(format!("{base}/v1/uploads"))
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "text/plain")
            .send(bytes)
            .unwrap()
    };
    let mut response = upload(b"hello");
    assert_eq!(response.status().as_u16(), 201);
    let object: Value = serde_json::from_str(&response.body_mut().read_to_string().unwrap()).unwrap();
    std::fs::write(f.dir.join("data/tmp/crash-leftover"), b"12345678").unwrap();
    let mut response = upload(b"x");
    assert_eq!(response.status().as_u16(), 507);
    assert!(response.body_mut().read_to_string().unwrap().contains("store_full"));
    let mut response = h
        .get(format!("{base}/v1/objects/{}/content", object["id"].as_str().unwrap()))
        .header("Authorization", format!("Bearer {token}"))
        .call()
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(db::sha256_hex(&response.body_mut().read_to_vec().unwrap()), object["sha256"]);
}

// A8: a genuine SQL dump/restored database plus copied objects must agree
// through the restored service, not only through a filesystem hash helper.
#[test]
fn restored_database_and_objects_pass_cli_and_api() {
    let f = fixture();
    let (_, token) = project(&mut f.client(), "restore", 5, 4);
    let s = start(&f);
    let job = cli(
        &f,
        &s,
        &token,
        &["submit", "--method", "synthetic.fake", "--operation", "synthetic.generate", "--key", "restore-1", "--budget", "1"],
    );
    let id = job["job"]["id"].as_str().unwrap();
    wait_for(&f, &s, &token, id, "succeeded");
    let result = cli(&f, &s, &token, &["result", id]);
    drop(s);
    let binaries = PathBuf::from(std::env::var("ASSETQ_TEST_PG_BIN").expect("runner must provide PostgreSQL binaries"));
    let dump = f.dir.join("backup.sql");
    let output = Command::new(binaries.join("pg_dump"))
        .args(["--no-owner", "--no-privileges", "--dbname", &f.url])
        .arg("--file")
        .arg(&dump)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let name = format!("assetq_restore_{}", uuid::Uuid::new_v4().simple());
    let admin = std::env::var("ASSETQ_TEST_PG").unwrap();
    let mut c = Client::connect(&format!("{admin} dbname=postgres"), postgres::NoTls).unwrap();
    c.batch_execute(&format!("CREATE DATABASE {name}")).unwrap();
    let restored = Fixture { url: format!("{admin} dbname={name}"), dir: f.dir.join("restored") };
    std::fs::create_dir_all(&restored.dir).unwrap();
    let output = Command::new(binaries.join("psql"))
        .args(["-X", "-v", "ON_ERROR_STOP=1", "--dbname", &restored.url])
        .arg("--file")
        .arg(&dump)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    fn copy_tree(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let dest = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &dest);
            } else {
                std::fs::copy(entry.path(), dest).unwrap();
            }
        }
    }
    copy_tree(&f.dir.join("data"), &restored.dir.join("data"));
    let check = env_cmd(&restored).arg("restore-check").output().unwrap();
    assert!(check.status.success(), "{}", String::from_utf8_lossy(&check.stderr));
    let check: Value = serde_json::from_slice(&check.stdout).unwrap();
    assert_eq!(check["succeeded"], 1);
    let s = start(&restored);
    assert_eq!(cli(&restored, &s, &token, &["status", id])["phase"], "succeeded");
    assert_eq!(cli(&restored, &s, &token, &["result", id])["manifest"], result["manifest"]);
    let item = &result["manifest"]["items"][0];
    let out = restored.dir.join("download.bin");
    assert_eq!(
        cli(&restored, &s, &token, &["download", item["object_id"].as_str().unwrap(), out.to_str().unwrap()])["sha256"],
        item["sha256"]
    );
    // Missing payload is a failed restore, never a healthy empty catalog.
    drop(s);
    let sha = item["sha256"].as_str().unwrap();
    std::fs::remove_file(restored.dir.join("data/objects").join(&sha[..2]).join(&sha[2..4]).join(sha)).unwrap();
    assert!(!env_cmd(&restored).arg("restore-check").output().unwrap().status.success());
}
