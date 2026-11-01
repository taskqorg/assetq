//! Executors claim fenced work, run bounded steps and import outputs. Pollers
//! observe provider operations without holding executor capacity (R2, R6).

use crate::db::{self, NewObject};
use crate::model::{CancelState, EffectState, Event, Job};
use crate::provider::{Poll, Provider, Submit};
use crate::store::Store;
use crate::{Config, Error, Result};
use postgres::Client;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub struct Ctx {
    pub cfg: Config,
    pub store: Store,
    pub providers: Vec<Arc<dyn Provider>>,
}

impl Ctx {
    pub fn new(cfg: Config) -> std::io::Result<Ctx> {
        let store = Store::open(&cfg.data_root)?;
        let mut providers: Vec<Arc<dyn Provider>> = vec![];
        if let Some(dir) = &cfg.fake_provider_dir {
            providers.push(Arc::new(crate::provider::FakeProvider::new(dir.clone())?));
        }
        Ok(Ctx { cfg, store, providers })
    }
    pub fn provider_names(&self) -> Vec<&str> {
        self.providers.iter().map(|p| p.name()).collect()
    }
    fn provider(&self, name: &str) -> Result<&Arc<dyn Provider>> {
        self.providers
            .iter()
            .find(|p| p.name() == name)
            .ok_or_else(|| Error::new(503, "capability_unavailable", format!("provider {name} is not configured")))
    }
    pub fn room(&self) -> Result<u64> {
        let (objects, temp) = self.store.inventory().map_err(|_| Error::new(503, "store_unknown", "physical inventory unavailable"))?;
        Ok(self.cfg.quota_bytes.saturating_sub(objects.saturating_add(temp)).min(self.cfg.temp_quota_bytes.saturating_sub(temp)))
    }
}

fn event(c: &mut Client, ctx: &Ctx, id: &str, e: &Event, wake_ms: u64) -> Result<Job> {
    let mut tx = c.transaction()?;
    let j = db::apply_event(&mut tx, id, e, ctx.cfg.lease_ms, wake_ms)?;
    tx.commit()?;
    Ok(j)
}

/// Run executor and poller duties until `stop` is set.
pub fn run_loop(ctx: Arc<Ctx>, owner: String, stop: Arc<AtomicBool>) {
    let mut client: Option<Client> = None;
    while !stop.load(Ordering::Relaxed) {
        if client.as_ref().is_none_or(|c| c.is_closed()) {
            client = db::connect(&ctx.cfg.database_url).map_err(|e| eprintln!("executor {owner}: {e}")).ok();
        }
        let busy = match client.as_mut() {
            Some(c) => tick(c, &ctx, &owner).unwrap_or_else(|e| {
                eprintln!("executor {owner}: {e}");
                false
            }),
            None => false,
        };
        if !busy {
            std::thread::sleep(Duration::from_millis(ctx.cfg.poll_ms.min(200)));
        }
    }
}

/// One unit of work: a claimed job or one provider observation.
pub fn tick(c: &mut Client, ctx: &Ctx, owner: &str) -> Result<bool> {
    if let Some((id, job)) = claim(c, ctx, owner)? {
        let result = run_claimed(c, ctx, &id, job);
        db::unlock_job(c, &id)?;
        if let Err(e) = result {
            eprintln!("job {id}: {e}");
        }
        return Ok(true);
    }
    poll_due(c, ctx)
}

fn claim(c: &mut Client, ctx: &Ctx, owner: &str) -> Result<Option<(String, Job)>> {
    let mut tx = c.transaction()?;
    let row = tx.query_opt(
        "SELECT id, lease_owner IS NOT NULL FROM jobs WHERE phase IN ('queued', 'active') AND wake_at <= now()
           AND (lease_owner IS NULL OR lease_until < now()) ORDER BY wake_at LIMIT 1 FOR UPDATE SKIP LOCKED",
        &[],
    )?;
    let Some(row) = row else { return Ok(None) };
    let id: String = row.get(0);
    if !db::lock_job(&mut tx, &id)? {
        return Ok(None);
    }
    let result = (|| {
        if row.get::<_, bool>(1) {
            db::apply_event(&mut tx, &id, &Event::LeaseExpired, ctx.cfg.lease_ms, 0)?;
        }
        let fence = db::load_job(&mut tx, &id)?.fence + 1;
        let job = db::apply_event(&mut tx, &id, &Event::Claimed { owner: owner.into(), fence }, ctx.cfg.lease_ms, 0)?;
        tx.commit()?;
        Ok(Some((id.clone(), job)))
    })();
    if result.is_err() {
        db::unlock_job(c, &id)?;
    }
    result
}

fn renew(c: &mut Client, ctx: &Ctx, id: &str, job: &Job) -> Result<()> {
    let n = c.execute(
        "UPDATE jobs SET lease_until = now() + make_interval(secs => $3::bigint / 1000.0) WHERE id = $1 AND fence = $2 AND lease_owner IS NOT NULL AND lease_until > clock_timestamp()",
        &[&id, &job.fence, &(ctx.cfg.lease_ms as i64)],
    )?;
    if n == 0 { Err(Error::new(409, "stale_fence", "lease lost")) } else { Ok(()) }
}

struct JobInfo {
    project: String,
    request: Value,
    spec: Value,
}

fn info(c: &mut Client, id: &str) -> Result<JobInfo> {
    let r = c.query_one(
        "SELECT j.project, j.request, m.spec FROM jobs j JOIN methods m ON m.id = j.method_id AND m.version = j.method_version AND m.project = j.project WHERE j.id = $1",
        &[&id],
    )?;
    Ok(JobInfo { project: r.get(0), request: r.get(1), spec: r.get(2) })
}

fn run_claimed(c: &mut Client, ctx: &Ctx, id: &str, mut job: Job) -> Result<()> {
    let info = info(c, id)?;
    let f = job.fence;
    loop {
        let mut tx = c.transaction()?;
        job = db::load_job(&mut tx, id)?;
        tx.commit()?;
        if job.phase.terminal() {
            return Ok(());
        }
        if expired(c, ctx, id)? && job.cancel == CancelState::None {
            job = event(c, ctx, id, &Event::DeadlineExpired, 0)?;
        }
        if job.cancel == CancelState::Requested && matches!(job.effect, EffectState::None | EffectState::Done | EffectState::Failed) {
            event(c, ctx, id, &Event::CancelStoppedLocally, 0)?;
            return Ok(());
        }
        if job.effect == EffectState::Intended {
            // A previous executor persisted intent but not the reply.
            let effect_id = job.effect_id.clone().unwrap();
            event(
                c,
                ctx,
                id,
                &Event::SubmitUncertain { fence: f, effect_id, reason: "executor stopped before recording the provider reply".into() },
                0,
            )?;
            return Ok(());
        }
        if job.step == job.steps.len() {
            return commit_manifest(c, ctx, id, f);
        }
        renew(c, ctx, id, &job)?;
        match job.steps[job.step].as_str() {
            "provider.submit" => return submit(c, ctx, id, &job, &info),
            "provider.import" => match import(c, ctx, id, &job, &info)? {
                Some(j) => j,
                None => return Ok(()),
            },
            "synthetic.generate" => {
                let body = format!(
                    "SYNTHETIC ASSETQ LOCAL OUTPUT - not a real generation\ntext={}\n",
                    info.request["text"].as_str().unwrap_or("")
                );
                let mut src = std::io::Cursor::new(body.into_bytes());
                if !job.imported.contains("output-0") {
                    match put_output(c, ctx, id, &job, &info, "output-0", "application/x-assetq-synthetic", &mut src, None)? {
                        Some(j) => job = j,
                        None => return Ok(()),
                    }
                }
                event(c, ctx, id, &Event::StepCompleted { fence: f, step: job.step }, 0)?
            }
            "validate" => {
                let allowed: Vec<String> = serde_json::from_value(info.spec["outputs"].clone()).unwrap_or_default();
                let rows = c.query("SELECT sha256, size, content_type, import_key FROM objects WHERE job_id = $1", &[&id])?;
                let mut problem = None;
                for r in &rows {
                    let (sha, size, ct, key): (String, i64, String, String) = (r.get(0), r.get(1), r.get(2), r.get(3));
                    if size <= 0 {
                        problem = Some(format!("{key} is empty"));
                    } else if !allowed.contains(&ct) {
                        problem = Some(format!("{key} has undeclared type {ct}"));
                    } else if let Err(e) = ctx.store.verify(&sha, size as u64) {
                        problem = Some(format!("{key}: {e}"));
                    }
                }
                if rows.len() != job.imported.len() {
                    problem = Some("imported objects and records differ".into());
                }
                if let Some(p) = problem {
                    event(c, ctx, id, &Event::Failed { fence: f, reason: format!("validation failed: {p}") }, 0)?;
                    return Ok(());
                }
                event(c, ctx, id, &Event::StepCompleted { fence: f, step: job.step }, 0)?
            }
            other => {
                event(c, ctx, id, &Event::Failed { fence: f, reason: format!("unsupported step {other}") }, 0)?;
                return Ok(());
            }
        };
    }
}

fn expired(c: &mut Client, ctx: &Ctx, id: &str) -> Result<bool> {
    Ok(c.query_one(
        "SELECT created_at + make_interval(secs => $2::bigint / 1000.0) < clock_timestamp() FROM jobs WHERE id = $1",
        &[&id, &(ctx.cfg.max_execution_ms as i64)],
    )?
    .get(0))
}

fn submit(c: &mut Client, ctx: &Ctx, id: &str, job: &Job, info: &JobInfo) -> Result<()> {
    match db::with_admission(c, ctx.cfg.metadata_quota_bytes, |c| submit_admitted(c, ctx, id, job, info)) {
        Err(e) if matches!(e.code, "admission_busy" | "metadata_full") => {
            event(c, ctx, id, &Event::Paused { fence: job.fence, reason: e.code.into() }, ctx.cfg.poll_ms)?;
            Ok(())
        }
        other => other,
    }
}

fn submit_admitted(c: &mut Client, ctx: &Ctx, id: &str, job: &Job, info: &JobInfo) -> Result<()> {
    let f = job.fence;
    if ctx.room()? == 0 {
        event(
            c,
            ctx,
            id,
            &Event::Paused { fence: f, reason: "store_full: no new provider effects until space is available".into() },
            ctx.cfg.poll_ms,
        )?;
        return Ok(());
    }
    let provider = match ctx.provider(info.spec["provider"].as_str().unwrap_or("")) {
        Ok(p) => p,
        Err(e) => {
            event(c, ctx, id, &Event::Paused { fence: f, reason: e.reason }, ctx.cfg.poll_ms)?;
            return Ok(());
        }
    };
    let effect_id = db::new_id();
    // Persist dispatch intent before the external effect (R2 durable execution).
    let mut tx = c.transaction()?;
    tx.execute(
        "INSERT INTO effects (id, job_id, provider, request, attempt_id) SELECT $1, $2, $3, $4, id FROM attempts WHERE job_id = $2 AND fence = $5",
        &[&effect_id, &id, &provider.name(), &info.request, &f],
    )?;
    db::apply_event(&mut tx, id, &Event::DispatchIntended { fence: f, effect_id: effect_id.clone() }, ctx.cfg.lease_ms, 0)?;
    tx.commit()?;
    renew(c, ctx, id, job)?;
    let e = match provider.submit(&effect_id, &info.request) {
        Submit::Accepted(op) => {
            c.execute("UPDATE effects SET op_id = $2, updated_at = now() WHERE id = $1", &[&effect_id, &op])?;
            Event::SubmitAcked { fence: f, effect_id, op_id: op }
        }
        Submit::Uncertain(reason) => Event::SubmitUncertain { fence: f, effect_id, reason },
        Submit::Refused(reason) => Event::SubmitRefused { fence: f, effect_id, reason },
    };
    event(c, ctx, id, &e, ctx.cfg.poll_ms)?;
    Ok(())
}

fn import(c: &mut Client, ctx: &Ctx, id: &str, job: &Job, info: &JobInfo) -> Result<Option<Job>> {
    let provider = ctx.provider(info.spec["provider"].as_str().unwrap_or(""))?;
    let op = job.op_id.clone().ok_or_else(|| Error::new(500, "no_operation", "import without a provider operation"))?;
    let mut job = job.clone();
    for i in 0..job.outputs.unwrap_or(0) {
        let key = format!("output-{i}");
        if job.imported.contains(&key) {
            continue;
        }
        renew(c, ctx, id, &job)?;
        let out = match provider.fetch(&op, i) {
            Ok(o) => o,
            Err(e) => {
                event(c, ctx, id, &Event::Paused { fence: job.fence, reason: format!("fetch failed: {e}") }, ctx.cfg.poll_ms)?;
                return Ok(None);
            }
        };
        let mut reader = out.reader;
        match put_output(c, ctx, id, &job, info, &key, &out.content_type, &mut reader, Some(&op))? {
            Some(j) => job = j,
            None => return Ok(None),
        }
    }
    Ok(Some(event(c, ctx, id, &Event::StepCompleted { fence: job.fence, step: job.step }, 0)?))
}

#[allow(clippy::too_many_arguments)]
fn put_output(
    c: &mut Client,
    ctx: &Ctx,
    id: &str,
    job: &Job,
    info: &JobInfo,
    key: &str,
    content_type: &str,
    src: &mut dyn std::io::Read,
    op: Option<&str>,
) -> Result<Option<Job>> {
    let result = db::with_admission(c, ctx.cfg.metadata_quota_bytes, |c| {
        let room = ctx.room()?;
        let stored = match ctx.store.put(src, ctx.cfg.max_upload_bytes, room) {
            Ok(s) => s,
            Err(e) => {
                event(c, ctx, id, &Event::Paused { fence: job.fence, reason: format!("import of {key} failed: {e}") }, ctx.cfg.poll_ms)?;
                return Ok(None);
            }
        };
        let synthetic = info.spec["synthetic"].as_bool().unwrap_or(false);
        let provenance = json!({
            "job_id": id, "method": job.method, "provider": info.spec["provider"], "provider_operation": op,
            "parents": info.request["attachments"], "request": info.request, "import_key": key, "synthetic": synthetic,
        });
        let mut tx = c.transaction()?;
        db::insert_object(
            &mut tx,
            &NewObject {
                project: &info.project,
                sha256: &stored.sha256,
                size: stored.size,
                content_type,
                kind: "output",
                job_id: Some(id),
                import_key: Some(key),
                provenance,
                synthetic,
                retention_days: None,
            },
        )?;
        let j = db::apply_event(&mut tx, id, &Event::ObjectImported { fence: job.fence, key: key.into() }, ctx.cfg.lease_ms, 0)?;
        tx.commit()?;
        Ok(Some(j))
    });
    match result {
        Err(e) if matches!(e.code, "admission_busy" | "metadata_full") => {
            event(c, ctx, id, &Event::Paused { fence: job.fence, reason: e.code.into() }, ctx.cfg.poll_ms)?;
            Ok(None)
        }
        other => other,
    }
}

/// Manifest rows and the success transition commit in one transaction (R6).
fn commit_manifest(c: &mut Client, ctx: &Ctx, id: &str, fence: i64) -> Result<()> {
    let mut tx = c.transaction()?;
    db::load_job(&mut tx, id)?;
    let rows = tx
        .query("SELECT id, sha256, size, content_type, import_key, synthetic FROM objects WHERE job_id = $1 ORDER BY import_key", &[&id])?;
    for r in &rows {
        ctx.store.verify(&r.get::<_, String>(1), r.get::<_, i64>(2) as u64)?;
    }
    let items: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "object_id": r.get::<_, String>(0), "sha256": r.get::<_, String>(1), "size": r.get::<_, i64>(2),
                "content_type": r.get::<_, String>(3), "import_key": r.get::<_, String>(4), "synthetic": r.get::<_, bool>(5),
            })
        })
        .collect();
    tx.execute("INSERT INTO manifests (job_id, items) VALUES ($1, $2)", &[&id, &Value::Array(items.clone())])?;
    db::apply_event(&mut tx, id, &Event::ManifestCommitted { fence, items: items.len() }, ctx.cfg.lease_ms, 0)?;
    tx.commit()?;
    Ok(())
}

/// Observe one due provider operation: reconcile, poll or cancel.
fn poll_due(c: &mut Client, ctx: &Ctx) -> Result<bool> {
    let mut tx = c.transaction()?;
    let row = tx.query_opt(
        "SELECT id FROM jobs WHERE phase = 'waiting' AND wake_at <= now() ORDER BY wake_at LIMIT 1 FOR UPDATE SKIP LOCKED",
        &[],
    )?;
    let Some(row) = row else { return Ok(false) };
    let id: String = row.get(0);
    if !db::lock_job(&mut tx, &id)? {
        return Ok(false);
    }
    let job = (|| -> Result<Job> {
        tx.execute(
            "UPDATE jobs SET wake_at = now() + make_interval(secs => $2::bigint / 1000.0) WHERE id = $1",
            &[&id, &(ctx.cfg.poll_ms as i64)],
        )?;
        let job = db::load_job(&mut tx, &id)?;
        tx.commit()?;
        Ok(job)
    })();
    let result = job.and_then(|job| observe(c, ctx, &id, job));
    db::unlock_job(c, &id)?;
    result
}

fn observe(c: &mut Client, ctx: &Ctx, id: &str, mut job: Job) -> Result<bool> {
    if expired(c, ctx, id)? && job.cancel == CancelState::None {
        job = event(c, ctx, id, &Event::DeadlineExpired, 0)?;
    }
    let info = info(c, id)?;
    let provider = match ctx.provider(info.spec["provider"].as_str().unwrap_or("")) {
        Ok(p) => p,
        Err(_) => return Ok(true), // stays waiting; status shows the capability gap
    };
    let e = match job.effect {
        EffectState::Intended | EffectState::Uncertain => {
            let effect_id = job.effect_id.clone().unwrap();
            match provider.lookup(&effect_id) {
                Ok(op) => {
                    if let Some(op) = &op {
                        c.execute("UPDATE effects SET op_id = $2, updated_at = now() WHERE id = $1", &[&effect_id, op])?;
                    }
                    Event::Reconciled { effect_id, op_id: op }
                }
                Err(_) => return Ok(true), // still uncertain; never replay blindly
            }
        }
        EffectState::Submitted => {
            let op = job.op_id.clone().unwrap();
            if job.cancel == CancelState::Requested {
                match provider.cancel(&op) {
                    Ok(true) => Event::CancelConfirmed { op_id: op },
                    Ok(false) => Event::CancelUncertain { reason: "provider did not confirm the cancel".into() },
                    Err(e) => Event::CancelUncertain { reason: format!("provider cancel failed: {e}") },
                }
            } else {
                match provider.poll(&op) {
                    Ok(Poll::Pending) | Err(_) => return Ok(true),
                    Ok(Poll::Done { outputs, credits }) => Event::ProviderCompleted { op_id: op, outputs, credits },
                    Ok(Poll::Failed { reason, credits }) => Event::ProviderFailed { op_id: op, reason, credits },
                }
            }
        }
        _ => return Ok(true),
    };
    event(c, ctx, &id, &e, 0)?;
    Ok(true)
}
