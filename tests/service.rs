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
use std::sync::atomic::{AtomicUsize, Ordering};
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

// A3/A4, R5/R6: a definitive geometry refusal retains source objects and charge,
// releases the job, publishes no manifest and cannot redispatch after restart.
#[test]
fn meshy_validation_retains_originals_without_success_manifest() {
    struct MeshFixture(FakeProvider);
    impl Provider for MeshFixture {
        fn name(&self) -> &str {
            "meshy"
        }
        fn submit(&self, effect: &str, request: &Value) -> Submit {
            self.0.submit(effect, request)
        }
        fn lookup(&self, effect: &str) -> Result<Option<String>, String> {
            self.0.lookup(effect)
        }
        fn poll(&self, _op: &str) -> Result<Poll, String> {
            Ok(Poll::Done { outputs: 1, credits: Some(15) })
        }
        fn fetch(&self, _op: &str, _index: usize) -> Result<Output, String> {
            Ok(Output {
                content_type: "model/gltf-binary".into(),
                reader: Box::new(std::io::Cursor::new(b"invented invalid GLB fixture".to_vec())),
                evidence: json!({"fixture":true,"validation":{"valid":false}}),
            })
        }
        fn cancel(&self, _op: &str) -> Result<bool, String> {
            Ok(false)
        }
        fn validate(&self, _op: &str) -> Result<(), String> {
            Err("Meshy output validation failed: invented invalid GLB fixture".into())
        }
    }
    let f = fixture();
    let mut c = f.client();
    let (p, _) = project(&mut c, "meshy-fixture", 1000, 1);
    let mut ctx = f.ctx(1 << 30);
    ctx.providers.push(Arc::new(MeshFixture(f.fake())));
    let mut spec = assetq::meshy::method();
    spec["synthetic"] = json!(true);
    let owner = Principal { project: p.project.clone(), role: "owner".into() };
    let method = db::create_method_version(&mut c, &owner, &json!({"id":"model.meshy-prop.v1","spec":spec})).unwrap();
    let request = json!({"method":format!("model.meshy-prop.v1@{}",method["version"]),"operation":"model.image_to_3d",
        "text":"invented fixture","attachments":[],"constraints":{},"budget":{"max_credits":15},"idempotency_key":"meshy-refusal"});
    let (job, created) = db::submit_job(&mut c, &p, &request, &limits(), &ctx.provider_names()).unwrap();
    assert!(created);
    let id = job["id"].as_str().unwrap();
    for _ in 0..8 {
        exec::tick(&mut c, &ctx, "meshy-fixture").unwrap();
    }
    let failed = db::get_job(&mut c, &p.project, id).unwrap();
    assert_eq!(failed["phase"], "failed");
    assert_eq!(failed["usage"]["credits"], 15);
    assert_eq!(failed["leased"], false);
    assert!(failed["reason"].as_str().unwrap().contains("invented invalid GLB fixture"));
    assert_eq!(failed["submissions"], 1);
    let objects = c.query("SELECT id, sha256, size FROM objects WHERE job_id=$1", &[&id]).unwrap();
    assert_eq!(objects.len(), 1);
    let object = &objects[0];
    ctx.store.verify(&object.get::<_, String>(1), object.get::<_, i64>(2) as u64).unwrap();
    assert_eq!(db::get_object(&mut c, &p.project, &object.get::<_, String>(0)).unwrap()["job_id"], id);
    assert_eq!(c.query_one("SELECT count(*) FROM manifests WHERE job_id=$1", &[&id]).unwrap().get::<_, i64>(0), 0);
    assert!(db::audit_job(&mut c, id).is_ok());
    assert!(!db::submit_job(&mut c, &p, &request, &limits(), &ctx.provider_names()).unwrap().1);
    let mut restarted = f.ctx(1 << 30);
    restarted.providers.push(Arc::new(MeshFixture(f.fake())));
    exec::tick(&mut c, &restarted, "restarted").unwrap();
    assert_eq!(f.fake().submissions(), 1);
}

fn limits() -> db::Limits {
    db::Limits { quota_bytes: 1 << 30 }
}

// R3/R5: the capability-named provider and retained legacy versions both reach
// the same pre-launch refusal, without rewriting old method/provider identities.
#[test]
fn image_worker_provider_names_retain_versions_and_refuse_dispatch() {
    let f = fixture();
    let mut config = f.config(1 << 30);
    config.image_worker_root = Some(f.dir.clone());
    let ctx = Ctx::new(config).unwrap();
    let mut c = f.client();
    let (p, _) = project(&mut c, "worker-names", 1, 1);
    for version in [2, 3, 4, 5] {
        let mut request = req(&format!("version-{version}"), json!({}));
        request["project"] = json!(p.project);
        request["method"] = json!(format!("image.worker.v1@{version}"));
        request["operation"] = json!("image.generate");
        request["budget"]["max_credits"] = json!(0);
        let (job, _) = db::submit_job(&mut c, &p, &request, &limits(), &ctx.provider_names()).unwrap();
        let id = job["id"].as_str().unwrap();
        let done = drive(&mut c, &ctx, &p.project, id, phase("failed"));
        assert!(done["reason"].as_str().unwrap().contains("capability_unavailable: protected project/profile grant"));
        assert_eq!(db::submit_job(&mut c, &p, &request, &limits(), &ctx.provider_names()).unwrap().0["id"], id);
        db::audit_job(&mut c, id).unwrap();
    }
    assert_eq!(std::fs::read_dir(f.dir.join("data/tmp/image-worker")).unwrap().count(), 0);
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

/// Scripts what a provider reports after the fake accepted the submission.
struct ReportingProvider {
    fake: FakeProvider,
    unknown_polls: AtomicUsize,
    outputs: usize,
    body: Vec<u8>,
}

impl ReportingProvider {
    fn new(fake: FakeProvider, unknown_polls: usize, outputs: usize, body: Vec<u8>) -> Arc<ReportingProvider> {
        Arc::new(ReportingProvider { fake, unknown_polls: AtomicUsize::new(unknown_polls), outputs, body })
    }
}

impl Provider for ReportingProvider {
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
        if self.unknown_polls.load(Ordering::SeqCst) > 0 {
            self.unknown_polls.fetch_sub(1, Ordering::SeqCst);
            return Err("no completion report".into());
        }
        Ok(match self.fake.poll(op)? {
            Poll::Done { credits, .. } => Poll::Done { outputs: self.outputs, credits },
            other => other,
        })
    }
    fn fetch(&self, _: &str, _: usize) -> Result<Output, String> {
        Ok(Output { content_type: "image/png".into(), reader: Box::new(std::io::Cursor::new(self.body.clone())), evidence: Value::Null })
    }
    fn cancel(&self, op: &str) -> Result<bool, String> {
        self.fake.cancel(op)
    }
}

/// A PNG signature and IHDR chunk, which is all the import gate reads.
fn png_header(width: u32, height: u32) -> Vec<u8> {
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    png.extend(width.to_be_bytes());
    png.extend(height.to_be_bytes());
    png.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
    png
}

/// Registers `synthetic.fake@2`: PNG output, size constraints, three credits.
fn register_png_method(c: &mut Client, project: &str) {
    let token = db::create_token(c, project, "owner").unwrap();
    let owner = db::authenticate(c, &token).unwrap();
    let mut spec = db::builtin_methods()[0].3.clone();
    spec["outputs"] = json!(["image/png"]);
    spec["max_credits"] = json!(3);
    for limit in ["max_width", "max_height"] {
        spec["parameters"][limit] = json!({"type": "integer", "min": 1, "max": 4096});
    }
    db::create_method_version(c, &owner, &json!({"id": "synthetic.fake", "status": "experimental", "spec": spec})).unwrap();
}

fn png_req(key: &str, limit: u32) -> Value {
    json!({"method": "synthetic.fake@2", "operation": "synthetic.generate", "idempotency_key": key, "text": "blue wedge",
           "constraints": {"max_width": limit, "max_height": limit}, "budget": {"max_credits": 3}})
}

// R4/R6/R7: the assetq#4 run stayed `waiting` with its reservations after a
// completed turn returned a 1254 by 1254 PNG against a 1024 limit. A missing
// report stays unknown; a completed report is definitive and never replays.
#[test]
fn completed_turn_with_oversize_png_fails_and_releases_reservations() {
    let f = fixture();
    let mut c = f.client();
    // Budget 4: the replacement fits only if spend falls from 3 reserved to 1 reported.
    let (p, _) = project(&mut c, "oversize", 4, 1);
    register_png_method(&mut c, "oversize");
    let mut ctx = f.ctx(1 << 30);
    ctx.providers = vec![ReportingProvider::new(f.fake(), 2, 1, png_header(1254, 1254))];
    let (j, _) = db::submit_job(&mut c, &p, &png_req("oversize", 1024), &limits(), FAKE).unwrap();
    let id = j["id"].as_str().unwrap();
    drive(&mut c, &ctx, &p.project, id, phase("waiting"));

    exec::tick(&mut c, &ctx, "it-executor").unwrap();
    assert_eq!(db::get_job(&mut c, &p.project, id).unwrap()["phase"], "waiting", "no completion report stays unknown");
    let held = db::submit_job(&mut c, &p, &png_req("replacement", 2048), &limits(), FAKE).unwrap_err();
    assert_eq!(held.code, "concurrency_limit", "unknown keeps its reservations");

    let failed = drive(&mut c, &ctx, &p.project, id, phase("failed"));
    let reason = failed["reason"].as_str().unwrap();
    assert!(reason.contains("validation failed") && reason.contains("PNG dimensions 1254x1254 exceed"), "{reason}");
    assert_eq!(failed["submissions"], 1);
    assert_eq!(failed["usage"]["credits"], 1);
    assert_eq!(f.fake().submissions(), 1, "a refused output never resubmits");
    let result = db::job_result(&mut c, &p.project, id).unwrap();
    assert_eq!(result["status"], "failed");
    assert!(result["manifest"].is_null());
    let retained: i64 = c.query_one("SELECT count(*) FROM objects WHERE job_id = $1", &[&id]).unwrap().get(0);
    assert_eq!(retained, 1, "the refused output stays retained");
    db::audit_job(&mut c, id).unwrap();

    // The same bytes under a larger request limit: only the limit refused them.
    let (next, _) = db::submit_job(&mut c, &p, &png_req("replacement", 2048), &limits(), FAKE).unwrap();
    let next = next["id"].as_str().unwrap();
    drive(&mut c, &ctx, &p.project, next, phase("succeeded"));
    assert_eq!(db::job_result(&mut c, &p.project, next).unwrap()["manifest"]["items"][0]["content_type"], "image/png");
}

// R4/R6: every other refusal of a completed report is also terminal, so none of
// them can pause or wait until the execution deadline.
#[test]
fn completed_turn_with_unimportable_output_fails_with_its_reason() {
    for (key, outputs, body, limit, want) in [
        ("invalid-png", 1, b"this file is not a PNG at all".to_vec(), 1 << 20, "invalid PNG"),
        ("too-large", 1, png_header(16, 16), 8, "too_large"),
        ("no-outputs", 0, vec![], 1 << 20, "1..=16 outputs"),
        ("many-outputs", 17, vec![], 1 << 20, "1..=16 outputs"),
    ] {
        let f = fixture();
        let mut c = f.client();
        let (p, _) = project(&mut c, "refused", 4, 1);
        register_png_method(&mut c, "refused");
        let mut ctx = f.ctx(1 << 30);
        ctx.cfg.max_upload_bytes = limit;
        ctx.providers = vec![ReportingProvider::new(f.fake(), 0, outputs, body)];
        let (j, _) = db::submit_job(&mut c, &p, &png_req(key, 1024), &limits(), FAKE).unwrap();
        let id = j["id"].as_str().unwrap();
        let failed = drive(&mut c, &ctx, &p.project, id, phase("failed"));
        let reason = failed["reason"].as_str().unwrap();
        assert!(reason.contains(want), "{key}: {reason}");
        assert_eq!(failed["submissions"], 1, "{key}");
        assert_eq!(f.fake().submissions(), 1, "{key}");
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
fn migration_renames_runtime_specific_image_method_and_keeps_jobs() {
    let f = fixture();
    let mut c = f.client();
    let (a, _) = project(&mut c, "mu", 10, 4);
    let (job, _) = db::submit_job(&mut c, &a, &req("m1", json!({})), &limits(), FAKE).unwrap();
    // Rows seeded by an earlier build named the runtime instead of the capability.
    c.execute(
        "UPDATE methods SET spec = jsonb_set(jsonb_set(spec, '{provider}', '\"agentq.codex-image\"'), '{capabilities}', '[\"codex.image_generation\"]') WHERE id = 'image.worker.v1' AND version = 1",
        &[],
    )
    .unwrap();
    db::migrate(&mut c).unwrap();
    let spec = &db::list_methods(&mut c, "mu")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == "image.worker.v1" && m["version"] == 1)
        .unwrap()["spec"]
        .clone();
    assert_eq!(spec["provider"], "agentq.image");
    assert_eq!(spec["capabilities"], json!(["image_generation"]));
    // Versions 2-4 are historical registry entries that keep their original runtime-specific spec (R5).
    assert!(
        !db::builtin_methods().iter().filter(|m| m.0 != "image.worker.v1" || m.1 == 1 || m.1 == 5).any(|m| m
            .3
            .to_string()
            .to_lowercase()
            .contains("codex")
            || m.3.to_string().to_lowercase().contains("claude"))
    );
    let id = job["id"].as_str().unwrap();
    assert_eq!(db::get_job(&mut c, "mu", id).unwrap()["method"], json!("synthetic.fake@1"));
}

#[test]
fn registry_recommendations_and_comparisons() {
    let f = fixture();
    let mut c = f.client();
    let (member, _) = project(&mut c, "kappa", 10, 4);
    let owner_token = db::create_token(&mut c, "kappa", "owner").unwrap();
    let owner = db::authenticate(&mut c, &owner_token).unwrap();
    let methods = db::list_methods(&mut c, "kappa").unwrap();
    assert_eq!(methods.as_array().unwrap().len(), 8);
    assert_eq!(db::resolve_method(&mut c, "kappa", "image.worker.v1@2").unwrap().2, assetq::image_worker::legacy_method());
    assert_eq!(db::resolve_method(&mut c, "kappa", "image.worker.v1@3").unwrap().2, assetq::image_worker::protected_method_v3());
    assert_eq!(db::resolve_method(&mut c, "kappa", "image.worker.v1@4").unwrap().2, assetq::image_worker::protected_method_v4());
    assert_eq!(db::resolve_method(&mut c, "kappa", "image.worker.v1@5").unwrap().2, assetq::image_worker::method());
    // A1/R5 adds Meshy v2; migrate old catalogs without rewriting disabled v1 or another project's registry.
    let old_meshy = db::list_methods(&mut c, "kappa")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == "model.meshy-prop.v1" && m["version"] == 1)
        .unwrap()["spec"]
        .clone();
    c.execute("DELETE FROM methods WHERE project='kappa' AND id='model.meshy-prop.v1' AND version=2", &[]).unwrap();
    db::migrate(&mut c).unwrap();
    assert_eq!(db::resolve_method(&mut c, "kappa", "model.meshy-prop.v1@2").unwrap().2, assetq::meshy::method());
    assert_eq!(db::resolve_method(&mut c, "kappa", "model.meshy-prop.v1@1").unwrap_err().code, "method_disabled");
    assert_eq!(
        db::list_methods(&mut c, "kappa")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == "model.meshy-prop.v1" && m["version"] == 1)
            .unwrap()["spec"],
        old_meshy
    );
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
// Task #7 A2-A7/R5/R7: immutable inputs, bounded repeated jobs, restart-safe
// identities and human judgment must survive registry and recommendation edits.
#[test]
fn registry_bounded_comparison_jobs_and_pins() {
    use assetq::registry;
    let f = fixture();
    let mut c = f.client();
    let (member, _) = project(&mut c, "bench", 64, 1);
    let owner = Principal { project: member.project.clone(), role: "owner".into() };
    let (foreign, _) = project(&mut c, "foreign-bench", 0, 1);
    let dataset = json!({"id":"crate", "version":1, "document":{"operation":"synthetic.generate", "cases":[{"id":"blue", "text":"invented blue crate", "attachments":[]}]}});
    assert_eq!(registry::register(&mut c, &member, "dataset", &dataset).unwrap_err().status, 403);
    let frozen = registry::register(&mut c, &owner, "dataset", &dataset).unwrap();
    assert_eq!(registry::register(&mut c, &owner, "dataset", &dataset).unwrap(), frozen);
    let mut changed = dataset.clone();
    changed["document"]["cases"][0]["text"] = json!("changed input");
    assert_eq!(registry::register(&mut c, &owner, "dataset", &changed).unwrap_err().code, "immutable_revision");
    registry::register(&mut c, &owner, "evaluator", &json!({"id":"quality", "version":1, "document":{"decision_rule":"technical validity, then owner quality, then lower measured credits", "quality_rubric":"0 unusable; 100 accepted invented crate"}})).unwrap();
    let rec1 = db::propose_recommendation(
        &mut c,
        &member,
        &json!({"use_case":"crate", "method":"synthetic.fake@1", "evidence":{"fixture":"invented"}, "observed_at":"2026-11-04T00:00:00Z"}),
    )
    .unwrap();
    db::promote_recommendation(&mut c, &owner, rec1["id"].as_str().unwrap()).unwrap();
    let mut request = req("recommended-pin", json!({}));
    request["method"] = json!("synthetic.fake@1");
    request["recommendation"] = rec1["id"].clone();
    let pinned = db::submit_job(&mut c, &member, &request, &limits(), FAKE).unwrap().0;
    let mut spec = db::builtin_methods()[0].3.clone();
    spec["pipeline_revision"] = json!("2");
    db::create_method_version(&mut c, &owner, &json!({"id":"synthetic.fake", "spec":spec})).unwrap();
    let rec2 = db::propose_recommendation(&mut c, &member, &json!({"use_case":"crate", "method":"synthetic.fake@2", "evidence":{"fixture":"invented revision 2"}, "observed_at":"2026-11-04T01:00:00Z"})).unwrap();
    db::promote_recommendation(&mut c, &owner, rec2["id"].as_str().unwrap()).unwrap();
    assert_eq!(db::submit_job(&mut c, &member, &request, &limits(), FAKE).unwrap().0["id"], pinned["id"]);
    let ctx = f.ctx(1 << 30);
    drive(&mut c, &ctx, &owner.project, pinned["id"].as_str().unwrap(), phase("succeeded"));
    assert_eq!(db::get_job(&mut c, &owner.project, pinned["id"].as_str().unwrap()).unwrap()["method"], "synthetic.fake@1");
    let body = json!({"use_case":"crate", "dataset_version":"crate@1", "evaluator_version":"quality@1", "method_versions":["synthetic.fake@1", "synthetic.fake@2"],
        "plan":{"dataset":"crate@1", "evaluator":"quality@1", "constraints":{}, "repeats":2, "max_jobs":4, "max_credits":4}});
    let mut over = body.clone();
    over["plan"]["max_jobs"] = json!(3);
    assert_eq!(db::add_comparison(&mut c, &member, &over).unwrap_err().code, "bad_comparison");
    over = body.clone();
    over["plan"]["max_credits"] = json!(3);
    assert_eq!(db::add_comparison(&mut c, &member, &over).unwrap_err().code, "bad_comparison");
    let cmp = db::add_comparison(&mut c, &member, &body).unwrap();
    let id = cmp["id"].as_str().unwrap();
    assert_eq!(registry::result(&mut c, &foreign.project, id).unwrap_err().status, 404);
    assert_eq!(registry::run(&mut c, &member, id, 1 << 30, FAKE).unwrap_err().status, 403);
    let first = registry::run(&mut c, &owner, id, 1 << 30, FAKE).unwrap();
    assert_eq!(first["state"], "incomplete");
    assert_eq!(first["admission_refusal"]["error"], "concurrency_limit");
    assert_eq!(first["samples"][0]["request"]["text"], "invented blue crate");
    changed["version"] = json!(2);
    registry::register(&mut c, &owner, "dataset", &changed).unwrap();
    let mut c = f.client(); // A7: resume from persisted plan and jobs, no in-memory run state.
    for sample in 0..4 {
        let state = registry::run(&mut c, &owner, id, 1 << 30, FAKE).unwrap();
        let job = state["samples"][sample]["job"]["id"].as_str().unwrap();
        drive(&mut c, &ctx, &owner.project, job, phase("succeeded"));
        registry::score(
            &mut c,
            &member,
            id,
            &json!({"sample":sample.to_string(),"score":80,"judgment":"invented fixture accepted","reviewer":"invented fixture reviewer"}),
        )
        .unwrap();
    }
    let result = registry::result(&mut c, &owner.project, id).unwrap();
    assert_eq!(result["state"], "complete");
    assert_eq!(result["failure_rate"], 0.0);
    assert_eq!(result["credits"], 4);
    assert_eq!(result["samples"][0]["technical_score"], 1);
    assert!(result["samples"][0]["human_quality"]["observed_at"].is_string());
    assert_eq!(result["plan"]["dataset"]["sha256"], frozen["sha256"]);
    assert_eq!(result["human_judgments"], 4);
    assert_eq!(result["method_metrics"][0]["human_score_mean"], 80.0);
    assert!(result["samples"][0]["duration_ms"].as_i64().unwrap() >= 0);
    assert_eq!(
        registry::result(&mut c, &owner.project, id).unwrap(),
        result,
        "read-only reproducibility includes original jobs and manifest hashes"
    );
    assert_eq!(registry::run(&mut c, &owner, id, 1 << 30, FAKE).unwrap()["samples"], result["samples"]);
    assert_eq!(
        registry::score(
            &mut c,
            &member,
            id,
            &json!({"sample":"0","score":80,"judgment":"invented fixture accepted","reviewer":"invented fixture reviewer"})
        )
        .unwrap(),
        result
    );
    assert_eq!(f.fake().submissions(), 5, "one pinned job plus four samples; resuming never replays");
    assert_eq!(
        registry::score(&mut c, &member, id, &json!({"sample":"0","score":10,"judgment":"changed","reviewer":"invented"}))
            .unwrap_err()
            .code,
        "immutable_judgment"
    );
    assert_eq!(
        registry::score(&mut c, &foreign, id, &json!({"sample":"0","score":80,"judgment":"invented","reviewer":"invented"}))
            .unwrap_err()
            .status,
        404
    );
    let none = db::scoped_recommendations(&mut c, &owner.project, "crate", &json!({}), Some("image.generate")).unwrap();
    assert!(none["recommended"].is_null());
    assert!(!none["qualification_evidence"].as_array().unwrap().is_empty());
    assert!(none["observed_at"].is_string());
    assert!(none["revision"].is_string());
    let filtered =
        db::filter_methods(&mut c, &owner.project, &json!({"capability":"synthetic.fake","constraints":{"outputs":5}}), "", 500).unwrap();
    assert!(filtered["items"].as_array().unwrap().is_empty());
    let filtered =
        db::filter_methods(&mut c, &owner.project, &json!({"operation":"synthetic.generate","constraints":{"outputs":2}}), "", 1).unwrap();
    assert_eq!(filtered["items"].as_array().unwrap().len(), 1);
    assert!(filtered["next"].is_string());
    let status = db::status(&mut c, &owner.project, 1 << 30, FAKE).unwrap();
    assert_eq!(status["restore"]["state"], "unknown");
    assert!(status["revisions"]["registry"].is_string());
    assert!(status["revisions"]["comparisons"].is_string());
}

// Task #7 A3/A4: cross-operation rankings cannot exist; an incomplete sample
// or unreported usage cannot produce a perfect failure/cost result.
#[test]
fn registry_operation_separation_and_unknown_comparisons() {
    use assetq::registry;
    let f = fixture();
    let mut c = f.client();
    let (member, _) = project(&mut c, "bench-unknown", 16, 4);
    let owner = Principal { project: member.project.clone(), role: "owner".into() };
    let mixed = json!({"use_case":"mixed", "dataset_version":"invented-1", "evaluator_version":"invented-1", "method_versions":["image.worker.v1@5","model.meshy-prop.v1@2"],"results":{}});
    assert_eq!(db::add_comparison(&mut c, &member, &mixed).unwrap_err().reason, "candidate operations differ");
    registry::register(&mut c, &owner, "dataset", &json!({"id":"d", "version":1, "document":{"operation":"synthetic.generate","cases":[{"id":"a","text":"invented","attachments":[]}]}})).unwrap();
    registry::register(
        &mut c,
        &owner,
        "evaluator",
        &json!({"id":"e","version":1,"document":{"decision_rule":"retain failures and unknowns","quality_rubric":"invented 0-100"}}),
    )
    .unwrap();
    for constraints in [json!({"report_usage":false}), json!({"fail":true})] {
        let body = json!({"use_case":"unknown", "dataset_version":"d@1", "evaluator_version":"e@1", "method_versions":["synthetic.fake@1"],
            "plan":{"dataset":"d@1","evaluator":"e@1","constraints":constraints,"repeats":1,"max_jobs":1,"max_credits":1}});
        let cmp = db::add_comparison(&mut c, &member, &body).unwrap();
        let id = cmp["id"].as_str().unwrap();
        let before = registry::result(&mut c, &owner.project, id).unwrap();
        assert!(before["failure_rate"].is_null());
        assert!(before["credits"].is_null());
        let run = registry::run(&mut c, &owner, id, 1 << 30, FAKE).unwrap();
        let job = run["samples"][0]["job"]["id"].as_str().unwrap();
        drive(&mut c, &f.ctx(1 << 30), &owner.project, job, phase(if constraints["fail"] == true { "failed" } else { "succeeded" }));
        let after = registry::result(&mut c, &owner.project, id).unwrap();
        if constraints["fail"] == true {
            assert_eq!(after["failures"], 1);
            assert_eq!(after["failure_rate"], 1.0);
        } else {
            assert!(after["credits"].is_null());
            assert_eq!(after["usage_unknown"], 1);
            assert_eq!(after["state"], "incomplete");
            assert!(after["samples"][0]["human_quality"].is_null());
        }
    }
    registry::register(&mut c,&owner,"dataset",&json!({"id":"paid","version":1,"document":{"operation":"model.image_to_3d","cases":[{"id":"a","text":"invented","attachments":[]}]}})).unwrap();
    let paid=db::add_comparison(&mut c,&member,&json!({"use_case":"paid","dataset_version":"paid@1","evaluator_version":"e@1","method_versions":["model.meshy-prop.v1@2"],"plan":{"dataset":"paid@1","evaluator":"e@1","constraints":{},"repeats":1,"max_jobs":1,"max_credits":15}})).unwrap();
    assert_eq!(
        registry::run(&mut c, &owner, paid["id"].as_str().unwrap(), 1 << 30, &["meshy"]).unwrap_err().code,
        "experiment_not_authorized"
    );
    assert_eq!(registry::result(&mut c, &owner.project, paid["id"].as_str().unwrap()).unwrap()["terminal"], 0);
    // A newer model/status is not a promotion, and identical use cases remain operation-scoped.
    for (name, operation) in [("invented-image", "image.generate"), ("invented-mesh", "model.image_to_3d")] {
        let mut spec = db::builtin_methods()[0].3.clone();
        spec["operation"] = json!(operation);
        db::create_method_version(&mut c, &owner, &json!({"id":name,"status":"recommended","spec":spec})).unwrap();
        assert!(
            db::scoped_recommendations(&mut c, &owner.project, "shared", &json!({}), Some(operation)).unwrap()["recommended"].is_null()
        );
        let rec = db::propose_recommendation(&mut c,&member,&json!({"use_case":"shared","method":format!("{name}@1"),"evidence":{"fixture":"invented"},"observed_at":"2026-11-04T00:00:00Z"})).unwrap();
        db::promote_recommendation(&mut c, &owner, rec["id"].as_str().unwrap()).unwrap();
    }
    assert!(db::recommendations(&mut c, &owner.project, "shared", &json!({})).unwrap()["recommended"].is_null());
    for op in ["image.generate", "model.image_to_3d"] {
        assert_eq!(
            db::scoped_recommendations(&mut c, &owner.project, "shared", &json!({}), Some(op)).unwrap()["recommended"]["operation"],
            op
        );
    }
    let evidence = registry::qualification("model.meshy-prop.v1@2");
    assert_eq!(evidence["owner_judgment"], "accepted as a pipeline check, not as a finished game asset");
    assert_eq!(evidence["credits"], 15);
    assert_eq!(evidence["general_dispatch"], "refused");
    let receipt: Value = serde_json::from_str(include_str!("../docs/evidence/4/native-receipt.json")).unwrap();
    let evidence = registry::qualification("image.worker.v1@2");
    assert_eq!(evidence["original_sha256"], receipt["original_sha256"]);
    assert_eq!(evidence["job"], receipt["job_id"]);
    assert!(evidence["credits"].is_null());
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
    assert_eq!(db::list_methods(&mut c, &b.project).unwrap().as_array().unwrap().len(), 8);
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

// Task #7 A4-A6/R4/R5: bounded histories cannot hide a promoted route or
// old comparisons, and unrelated specs cannot borrow real qualification.
#[test]
fn registry_publication_limits_and_receipt_identity() {
    use assetq::registry;
    let f = fixture();
    let mut c = f.client();
    let (member, _) = project(&mut c, "registry-limits", 8, 4);
    let owner = Principal { project: member.project.clone(), role: "owner".into() };
    let mut defects = vec![];
    c.execute("DELETE FROM methods WHERE project=$1 AND id='model.meshy-prop.v1' AND version=2", &[&owner.project]).unwrap();
    let custom = db::create_method_version(&mut c, &owner, &json!({"id":"model.meshy-prop.v1","spec":db::builtin_methods()[0].3})).unwrap();
    assert_eq!(custom["version"], 2);
    db::migrate(&mut c).unwrap();
    let rows = db::list_methods(&mut c, &owner.project).unwrap();
    let custom = rows.as_array().unwrap().iter().find(|m| m["id"] == "model.meshy-prop.v1" && m["version"] == 2).unwrap();
    if !custom["qualification"].is_null() {
        defects.push("different spec borrowed a real receipt");
    }
    let rec = db::propose_recommendation(
        &mut c,
        &member,
        &json!({"use_case":"limit","method":"synthetic.fake@1","evidence":{"fixture":"invented"},"observed_at":"2026-11-04T00:00:00Z"}),
    )
    .unwrap();
    let rid = rec["id"].as_str().unwrap();
    db::promote_recommendation(&mut c, &owner, rid).unwrap();
    // Invented fixture history, installed in one SQL statement to avoid 501 redundant writer setups.
    c.execute("INSERT INTO recommendations (id,project,use_case,constraints,method_id,method_version,evidence,observed_at,state,proposed_by,created_at)
        SELECT 'invented-proposal-' || n,r.project,r.use_case,r.constraints,r.method_id,r.method_version,r.evidence,r.observed_at,'proposed',r.proposed_by,now()+interval '1 second' FROM recommendations r CROSS JOIN generate_series(1,501) n WHERE r.id=$1", &[&rid]).unwrap();
    if db::recommendations(&mut c, &owner.project, "limit", &json!({})).unwrap()["recommended"]["id"] != rid {
        defects.push("history page hid promoted recommendation");
    }
    c.execute("INSERT INTO comparisons (id,use_case,dataset_version,evaluator_version,method_versions,results,missing_cases,created_by)
        SELECT 'invented-history-' || lpad(n::text,4,'0'),'history','invented-d','invented-e','[\"synthetic.fake@1\"]'::jsonb,'{}'::jsonb,'[\"unmeasured\"]'::jsonb,$1 FROM generate_series(1,501) n", &[&owner.project]).unwrap();
    let first = db::comparison_page(&mut c, &owner.project, Some("history"), "", 500).unwrap();
    assert_eq!(first["items"].as_array().unwrap().len(), 500);
    let last = db::comparison_page(&mut c, &owner.project, Some("history"), first["next"].as_str().unwrap(), 500).unwrap();
    assert_eq!(last["items"].as_array().unwrap().len(), 1);
    assert!(last["next"].is_null());
    assert_eq!(
        registry::result(&mut c, &owner.project, last["items"][0]["id"].as_str().unwrap()).unwrap()["missing_cases"][0],
        "unmeasured"
    );
    registry::register(&mut c,&owner,"dataset",&json!({"id":"d","version":1,"document":{"operation":"synthetic.generate","cases":[{"id":"a","text":"invented","attachments":[]}]}})).unwrap();
    registry::register(
        &mut c,
        &owner,
        "evaluator",
        &json!({"id":"e","version":1,"document":{"decision_rule":"observe only","quality_rubric":"invented"}}),
    )
    .unwrap();
    let cmp = db::add_comparison(&mut c,&member,&json!({"use_case":"revision","dataset_version":"d@1","evaluator_version":"e@1","method_versions":["synthetic.fake@1"],"plan":{"dataset":"d@1","evaluator":"e@1","constraints":{},"repeats":1,"max_jobs":1,"max_credits":1}})).unwrap();
    let id = cmp["id"].as_str().unwrap();
    let before = db::status(&mut c, &owner.project, 1 << 30, FAKE).unwrap()["revisions"]["comparisons"].clone();
    let run = registry::run(&mut c, &owner, id, 1 << 30, FAKE).unwrap();
    let admitted = db::status(&mut c, &owner.project, 1 << 30, FAKE).unwrap()["revisions"]["comparisons"].clone();
    if before == admitted {
        defects.push("admission did not change control revision");
    }
    drive(&mut c, &f.ctx(1 << 30), &owner.project, run["samples"][0]["job"]["id"].as_str().unwrap(), phase("succeeded"));
    let completed = db::status(&mut c, &owner.project, 1 << 30, FAKE).unwrap()["revisions"]["comparisons"].clone();
    if admitted == completed {
        defects.push("completion did not change control revision");
    }
    registry::score(&mut c, &member, id, &json!({"sample":"0","score":50,"judgment":"invented","reviewer":"invented"})).unwrap();
    let scored = db::status(&mut c, &owner.project, 1 << 30, FAKE).unwrap()["revisions"]["comparisons"].clone();
    if completed == scored {
        defects.push("judgment did not change control revision");
    }
    assert!(defects.is_empty(), "R4/R5 publication defects: {defects:?}");
}

// Task #7 A1/A6: CLI, API and agent JSON share the same registry/comparison
// records; anonymous and foreign callers cannot obtain project evidence.
#[test]
fn registry_cli_api_projection() {
    let f = fixture();
    let mut c = f.client();
    let (_, member_token) = project(&mut c, "registry-cli", 4, 4);
    let owner_token = db::create_token(&mut c, "registry-cli", "owner").unwrap();
    let (_, foreign_token) = project(&mut c, "registry-cli-foreign", 0, 1);
    // This fixture measures API projections, not two-second lease performance.
    let mut child = env_cmd(&f)
        .env("ASSETQ_LEASE_MS", "60000")
        .args(["serve", "--listen", "127.0.0.1:0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap()).read_line(&mut line).unwrap();
    let s = Service { child, url: format!("http://{}", line.trim().strip_prefix("assetq listening on ").unwrap()) };
    let file = f.dir.join("invented.json");
    for (command, body) in [
        (
            "dataset-add",
            json!({"id":"d","version":1,"document":{"operation":"synthetic.generate","cases":[{"id":"a","text":"invented fixture","attachments":[]}]}}),
        ),
        (
            "evaluator-add",
            json!({"id":"e","version":1,"document":{"decision_rule":"observe technical and human quality","quality_rubric":"invented 0-100"}}),
        ),
    ] {
        std::fs::write(&file, body.to_string()).unwrap();
        assert_eq!(cli(&f, &s, &owner_token, &[command, file.to_str().unwrap()])["version"], 1);
    }
    assert_eq!(cli(&f, &s, &member_token, &["datasets"])[0]["id"], "d");
    assert_eq!(cli(&f, &s, &member_token, &["evaluators"])[0]["id"], "e");
    let methods = cli(
        &f,
        &s,
        &member_token,
        &["methods", "--operation", "synthetic.generate", "--capability", "synthetic.fake", "--constraints", "{\"outputs\":1}"],
    );
    assert_eq!(methods["items"].as_array().unwrap().len(), 1);
    let body = json!({"use_case":"crate","dataset_version":"d@1","evaluator_version":"e@1","method_versions":["synthetic.fake@1"],"plan":{"dataset":"d@1","evaluator":"e@1","constraints":{},"repeats":1,"max_jobs":1,"max_credits":1}});
    // A2/A3/A7, R5: aliases must fail before admission, not strand resolved sample jobs.
    let h = http();
    let mut alias_statuses = vec![];
    for methods in [
        json!(["synthetic.fake@01"]),
        json!(["synthetic.fake@+1"]),
        json!(["synthetic.fake@1", "synthetic.fake@01"]),
        json!(["synthetic.fake@1", "synthetic.fake@+1"]),
        json!(["synthetic.fake@01", "synthetic.fake@+1"]),
    ] {
        let mut invalid = body.clone();
        invalid["method_versions"] = methods;
        invalid["plan"]["max_jobs"] = json!(2);
        invalid["plan"]["max_credits"] = json!(2);
        let response = h
            .post(format!("{}/v1/comparisons", s.url))
            .header("Authorization", format!("Bearer {member_token}"))
            .send(invalid.to_string())
            .unwrap();
        alias_statuses.push(response.status().as_u16());
    }
    assert_eq!(alias_statuses, vec![422; 5], "noncanonical pins, including duplicate aliases, must refuse before admission");
    assert!(db::list_comparisons(&mut c, "registry-cli", None).unwrap().as_array().unwrap().is_empty());
    assert_eq!(c.query_one("SELECT count(*) FROM jobs WHERE project='registry-cli'", &[]).unwrap().get::<_, i64>(0), 0);
    std::fs::write(&file, body.to_string()).unwrap();
    let cmp = cli(&f, &s, &member_token, &["comparison-add", file.to_str().unwrap()]);
    let id = cmp["id"].as_str().unwrap();
    assert_eq!(cli(&f, &s, &member_token, &["comparisons", "--limit", "1"])["items"][0]["id"], id);
    let run = cli(&f, &s, &owner_token, &["comparison-run", id]);
    wait_for(&f, &s, &member_token, run["samples"][0]["job"]["id"].as_str().unwrap(), "succeeded");
    assert_eq!(cli(&f, &s, &member_token, &["comparison", id])["state"], "incomplete");
    std::fs::write(&file, json!({"sample":"0","score":70,"judgment":"invented fixture","reviewer":"invented reviewer"}).to_string())
        .unwrap();
    let scored = cli(&f, &s, &member_token, &["comparison-score", id, file.to_str().unwrap()]);
    assert_eq!(scored["state"], "complete");
    assert_eq!(cli(&f, &s, &member_token, &["comparison", id]), scored);
    let ops = cli(&f, &s, &member_token, &["ops"]);
    assert!(ops["revisions"]["recommendations"].is_string());
    assert_eq!(ops["restore"]["state"], "unknown");
    let h = http();
    let base = &s.url;
    for path in ["status".to_string(), format!("comparisons/{id}")] {
        assert_eq!(h.get(format!("{base}/v1/{path}")).call().unwrap().status().as_u16(), 401);
    }
    assert_eq!(
        h.get(format!("{base}/v1/comparisons/{id}"))
            .header("Authorization", format!("Bearer {foreign_token}"))
            .call()
            .unwrap()
            .status()
            .as_u16(),
        404
    );
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

// R3/R5/R9: browser proof, sessions and CSRF cannot widen the shared API grant;
// search and preview must protect private objects before any file access.
#[test]
fn management_browser_scope_csrf_search_preview_and_durable_request() {
    let f = fixture();
    let ctx = Arc::new(f.ctx(64 << 20));
    let mut c = f.client();
    let (_, member) = project(&mut c, "web-a", 20, 8);
    let (_, foreign) = project(&mut c, "web-b", 20, 8);
    let owner = db::create_token(&mut c, "web-a", "owner").unwrap();
    let guard = uuid::Uuid::new_v4().simple().to_string();
    let browser = Arc::new(
        assetq::browser::Browser::from_json(
            &json!({"guard_secret":guard,
        "subjects":{"invented-member":[member],"invented-owner":[owner],"invented-foreign":[foreign]}})
            .to_string(),
        )
        .unwrap(),
    );
    let server = Arc::new(api::bind("127.0.0.1:0").unwrap());
    let base = format!("http://{}", server.server_addr().to_ip().unwrap());
    api::serve_with_browser(ctx.clone(), server, 3, Some(browser));
    let h = http();
    let get = |path: &str, subject: &str, cookie: &str| {
        h.get(format!("{base}{path}"))
            .header("Host", "assets.ufobe.com")
            .header("X-AssetQ-Guard", &guard)
            .header("X-AssetQ-Subject", subject)
            .header("Cookie", cookie)
    };
    let post = |path: &str, subject: &str, cookie: &str, csrf: &str| {
        h.post(format!("{base}{path}"))
            .header("Host", "assets.ufobe.com")
            .header("X-AssetQ-Guard", &guard)
            .header("X-AssetQ-Subject", subject)
            .header("Cookie", cookie)
            .header("Origin", assetq::browser::ORIGIN)
            .header("X-CSRF-Token", csrf)
    };
    let mut r = get("/browser/session", "invented-member", "").call().unwrap();
    assert_eq!(r.status().as_u16(), 200);
    let set_cookie = r.headers()["Set-Cookie"].to_str().unwrap().to_owned();
    assert!(set_cookie.contains("Secure; HttpOnly; SameSite=Strict"));
    let cookie = set_cookie.split(';').next().unwrap();
    let s: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
    assert_eq!(s["projects"], json!([{"id":"web-a","role":"member"}]));
    let csrf = s["csrf"].as_str().unwrap();
    for path in ["/assets", "/jobs/new", "/benchmarks/new", "/ui/app.js", "/ui/style.css"] {
        let response = get(path, "invented-member", cookie).call().unwrap();
        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(response.headers()["Cache-Control"], "private, no-store");
        assert!(response.headers()["Content-Security-Policy"].to_str().unwrap().contains("frame-ancestors 'none'"));
    }
    assert_eq!(h.get(format!("{base}/browser/session")).call().unwrap().status().as_u16(), 403);
    assert_eq!(get("/browser/session", "not-mapped", cookie).call().unwrap().status().as_u16(), 403);
    assert_eq!(get("/browser/v1/objects?project=web-a", "invented-owner", cookie).call().unwrap().status().as_u16(), 403);
    assert_eq!(get("/browser/v1/objects?project=web-b", "invented-member", cookie).call().unwrap().status().as_u16(), 403);
    assert_eq!(get("/browser/v1/objects?project=web-a&project=web-b", "invented-member", cookie).call().unwrap().status().as_u16(), 403);
    assert_eq!(get("/v1/objects", "invented-member", cookie).call().unwrap().status().as_u16(), 401);
    // Existing published render bytes, imported into an isolated fixture only.
    let png = include_bytes!("../docs/evidence/5/operator-run/view-front.png");
    assert_eq!(
        post("/browser/v1/uploads?project=web-a", "invented-member", cookie, "wrong")
            .header("Content-Type", "image/png")
            .send(png.as_slice())
            .unwrap()
            .status()
            .as_u16(),
        403
    );
    assert_eq!(
        post("/browser/v1/uploads?project=web-a", "invented-member", cookie, csrf)
            .header("Origin", "https://invented.invalid")
            .header("Content-Type", "image/png")
            .send(png.as_slice())
            .unwrap()
            .status()
            .as_u16(),
        403
    );
    let mut r = post("/browser/v1/uploads?project=web-a", "invented-member", cookie, csrf)
        .header("Content-Type", "image/png")
        .header("X-Asset-Name", "Crate front.png")
        .send(png.as_slice())
        .unwrap();
    assert_eq!(r.status().as_u16(), 201);
    let o: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
    let id = o["id"].as_str().unwrap();
    assert_eq!(o["sha256"], db::sha256_hex(png));
    let mut r = get("/browser/v1/objects?project=web-a&q=CRATE&type=image", "invented-member", cookie).call().unwrap();
    let found: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
    assert_eq!(found["items"].as_array().unwrap().len(), 1);
    assert_eq!(found["items"][0]["id"], id);
    assert_eq!(db::search_objects(&mut c, "web-a", None, 50, "%", "").unwrap()["items"], json!([]));
    assert_eq!(db::search_objects(&mut c, "web-b", None, 50, "crate", "").unwrap()["items"], json!([]));
    assert_eq!(db::search_objects(&mut c, "web-a", None, 50, "crate", "model").unwrap()["items"], json!([]));
    assert_eq!(db::search_objects(&mut c, "web-a", None, 50, &"x".repeat(257), "").unwrap_err().code, "bad_search");
    let mut r = get(&format!("/browser/v1/objects/{id}/preview?project=web-a"), "invented-member", cookie).call().unwrap();
    assert_eq!(r.status().as_u16(), 200);
    let bytes = r.body_mut().read_to_vec().unwrap();
    let decoded = image::load_from_memory(&bytes).unwrap();
    assert!(decoded.width() <= 512 && decoded.height() <= 512);
    let mut r = get(&format!("/browser/v1/objects/{id}/content?project=web-a"), "invented-member", cookie).call().unwrap();
    assert_eq!(r.headers()["Content-Disposition"], "attachment");
    assert_eq!(r.body_mut().read_to_vec().unwrap(), png);
    // An upload's MIME label alone never makes it safe to preview.
    let mut r = post("/browser/v1/uploads?project=web-a", "invented-member", cookie, csrf)
        .header("Content-Type", "image/png")
        .send(&b"invented invalid PNG"[..])
        .unwrap();
    let bad: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
    assert_eq!(
        get(&format!("/browser/v1/objects/{}/preview?project=web-a", bad["id"].as_str().unwrap()), "invented-member", cookie)
            .call()
            .unwrap()
            .status()
            .as_u16(),
        415
    );
    let r = get("/browser/session", "invented-foreign", "").call().unwrap();
    let fc = r.headers()["Set-Cookie"].to_str().unwrap().split(';').next().unwrap().to_owned();
    for suffix in ["", "/preview", "/related", "/content"] {
        assert_eq!(
            get(&format!("/browser/v1/objects/{id}{suffix}?project=web-b"), "invented-foreign", &fc).call().unwrap().status().as_u16(),
            404
        );
    }
    let body = json!({"method":"synthetic.fake@1","operation":"synthetic.generate","text":"literal source needle","attachments":[id],"constraints":{},"budget":{"max_credits":1},"idempotency_key":"browser-original"});
    let mut r = post("/browser/v1/jobs?project=web-a", "invented-member", cookie, csrf)
        .header("Content-Type", "application/json")
        .send(body.to_string())
        .unwrap();
    assert_eq!(r.status().as_u16(), 201);
    let job: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
    let jid = job["job"]["id"].as_str().unwrap();
    assert_eq!(db::job_by_key(&mut c, "web-a", "browser-original").unwrap()["id"], jid);
    assert_eq!(db::job_by_key(&mut c, "web-b", "browser-original").unwrap_err().status, 404);
    assert_eq!(db::get_method(&mut c, "web-a", "image.worker.v1", "1").unwrap()["status"], "disabled");
    assert_eq!(db::get_method(&mut c, "web-a", "image.worker.v1", "01").unwrap_err().status, 422);
    assert_eq!(
        post("/browser/v1/jobs?project=web-a", "invented-member", cookie, csrf)
            .header("Content-Type", "application/json")
            .send(body.to_string())
            .unwrap()
            .status()
            .as_u16(),
        200
    );
    let mut changed = body.clone();
    changed["text"] = json!("changed");
    assert_eq!(
        post("/browser/v1/jobs?project=web-a", "invented-member", cookie, csrf)
            .header("Content-Type", "application/json")
            .send(changed.to_string())
            .unwrap()
            .status()
            .as_u16(),
        409
    );
    assert_eq!(
        post("/browser/v1/methods?project=web-a", "invented-member", cookie, csrf)
            .header("Content-Type", "application/json")
            .send("{}")
            .unwrap()
            .status()
            .as_u16(),
        403
    );
    for _ in 0..4 {
        exec::tick(&mut c, &ctx, "browser-fixture").unwrap();
    }
    assert_eq!(db::get_job(&mut c, "web-a", jid).unwrap()["phase"], "succeeded");
    let related = db::related_objects(&mut c, "web-a", id, None, 1).unwrap();
    assert_eq!(related["children"].as_array().unwrap().len(), 1);
    assert!(related["next_children"].is_string());
    assert_eq!(db::search_objects(&mut c, "web-a", None, 50, "SOURCE NEEDLE", "").unwrap()["items"].as_array().unwrap().len(), 1);
    assert_eq!(post("/browser/logout", "invented-member", cookie, csrf).send("").unwrap().status().as_u16(), 200);
    assert_eq!(get("/browser/v1/jobs?project=web-a", "invented-member", cookie).call().unwrap().status().as_u16(), 403);
    // Ending/reopening a browser session never ends or repeats the durable job.
    let r = get("/browser/session", "invented-member", "").call().unwrap();
    let new_cookie = r.headers()["Set-Cookie"].to_str().unwrap().split(';').next().unwrap().to_owned();
    assert_eq!(
        get(&format!("/browser/v1/jobs/{jid}?project=web-a"), "invented-member", &new_cookie).call().unwrap().status().as_u16(),
        200
    );
    c.execute("UPDATE tokens SET revoked_at=now() WHERE hash=$1", &[&db::sha256_hex(member.as_bytes())]).unwrap();
    assert_eq!(get("/browser/v1/objects?project=web-a", "invented-member", &new_cookie).call().unwrap().status().as_u16(), 403);
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
    // Grow beyond the relcache init file; its removal must not cancel this fixture's net physical growth.
    let value = json!({"data": (0..3072).map(|i| db::sha256_hex(format!("invented fixture {i}").as_bytes())).collect::<String>()});
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
    let after = db::metadata_bytes(&mut c).unwrap();
    assert!(after > before, "physical fixture before={before} after={after}");
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
    assert_eq!(methods.as_array().unwrap().len(), 8);
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
