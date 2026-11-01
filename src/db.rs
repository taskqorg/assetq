//! PostgreSQL metadata: projects, grants, jobs and their event log, effects,
//! objects, manifests and the method/recommendation registry.

use crate::model::{self, Event, Job};
use crate::{Error, Result};
use postgres::{Client, GenericClient, NoTls, Transaction};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub fn connect(url: &str) -> Result<Client> {
    Client::connect(url, NoTls).map_err(Error::from)
}

pub fn connect_config(cfg: &crate::Config) -> Result<Client> {
    let mut c = connect(&cfg.database_url)?;
    configure(&mut c, cfg)?;
    Ok(c)
}

pub fn configure(c: &mut Client, cfg: &crate::Config) -> Result<()> {
    if cfg.metadata_admission_headroom_bytes < 4 << 20 {
        return Err(Error::new(422, "bad_config", "metadata transaction headroom must be at least 4 MiB"));
    }
    c.query_one("SELECT set_config('assetq.metadata_quota', $1, false), set_config('assetq.metadata_headroom', $2, false), set_config('assetq.metadata_recovery', $3, false)",
        &[&cfg.metadata_quota_bytes.to_string(), &cfg.metadata_admission_headroom_bytes.to_string(), &cfg.metadata_recovery_headroom_bytes.to_string()])?;
    Ok(())
}

const METADATA_LOCK: i64 = 0x415353455452;

fn metadata_limits(c: &mut impl GenericClient) -> Result<(u64, u64, u64)> {
    let r = c.query_one(
        "SELECT coalesce(nullif(current_setting('assetq.metadata_quota', true), ''), '1073741824')::bigint,
        coalesce(nullif(current_setting('assetq.metadata_headroom', true), ''), '4194304')::bigint,
        coalesce(nullif(current_setting('assetq.metadata_recovery', true), ''), '16777216')::bigint",
        &[],
    )?;
    Ok((r.get::<_, i64>(0) as u64, r.get::<_, i64>(1) as u64, r.get::<_, i64>(2) as u64))
}

pub fn metadata_bytes(c: &mut impl GenericClient) -> Result<u64> {
    // Includes all relations, indexes, TOAST and retained dead/aborted allocations.
    let size: i64 = c.query_one("SELECT pg_database_size(current_database())", &[])?.get(0);
    u64::try_from(size).map_err(|_| Error::new(503, "metadata_unknown", "negative database size"))
}

pub fn metadata_check(c: &mut impl GenericClient, recovery: bool) -> Result<()> {
    let (quota, headroom, reserve) = metadata_limits(c)?;
    let ceiling = if recovery { quota } else { quota.saturating_sub(reserve) };
    if metadata_bytes(c)?.checked_add(headroom).is_none_or(|n| n > ceiling) {
        return Err(Error::new(
            507,
            "metadata_full",
            if recovery {
                "metadata recovery headroom exhausted"
            } else {
                "metadata admission headroom exhausted; recovery reserve retained"
            },
        ));
    }
    Ok(())
}

/// Acquire before row/table locks; never hold across a stream or provider call.
pub fn write_tx(c: &mut Client, recovery: bool) -> Result<Transaction<'_>> {
    let mut tx = c.transaction()?;
    tx.batch_execute("SET LOCAL lock_timeout = '5s'")?;
    tx.query_one("SELECT pg_advisory_xact_lock($1)", &[&METADATA_LOCK])?;
    metadata_check(&mut tx, recovery)?;
    let (quota, _, reserve) = metadata_limits(&mut tx)?;
    let start = metadata_bytes(&mut tx)?;
    let ceiling = if recovery { quota } else { quota.saturating_sub(reserve) };
    tx.query_one("SELECT set_config('assetq.write_start', $1, true), set_config('assetq.write_ceiling', $2, true), set_config('assetq.write_charge', '0', true)",
        &[&start.to_string(), &ceiling.to_string()])?;
    Ok(tx)
}

/// Cap cumulative payload and row-allocation allowance before issuing mutations.
pub fn charge(tx: &mut Transaction, bytes: usize) -> Result<()> {
    let (_, headroom, _) = metadata_limits(tx)?;
    let old: String = tx
        .query_one("SELECT current_setting('assetq.write_charge', true)", &[])?
        .get::<_, Option<String>>(0)
        .ok_or_else(|| Error::new(503, "metadata_unknown", "unguarded metadata transaction"))?;
    let previous = old.parse::<u64>().map_err(|_| Error::new(503, "metadata_unknown", "invalid transaction charge"))?;
    let cost = (bytes as u64)
        .checked_mul(16)
        .and_then(|n| n.checked_add(64 << 10))
        .and_then(|n| n.checked_add(previous))
        .filter(|n| *n <= headroom)
        .ok_or_else(|| Error::new(507, "metadata_full", "metadata transaction payload allowance exhausted"))?;
    tx.query_one("SELECT set_config('assetq.write_charge', $1, true)", &[&cost.to_string()])?;
    Ok(())
}

pub fn commit_write(mut tx: Transaction) -> Result<()> {
    let r = tx.query_one("SELECT current_setting('assetq.write_start')::bigint, current_setting('assetq.write_ceiling')::bigint", &[])?;
    let used = metadata_bytes(&mut tx)?;
    let (_, headroom, _) = metadata_limits(&mut tx)?;
    if used > r.get::<_, i64>(1) as u64 || used.saturating_sub(r.get::<_, i64>(0) as u64) > headroom {
        return Err(Error::new(507, "metadata_full", "physical metadata growth exceeded transaction allowance; transaction refused"));
    }
    tx.commit()?;
    Ok(())
}

pub fn metadata_status(c: &mut Client) -> Value {
    match (metadata_limits(c), metadata_bytes(c)) {
        (Ok((quota, headroom, reserve)), Ok(used)) => json!({"state": "observed", "used_bytes": used,
            "quota_bytes": quota, "transaction_headroom_bytes": headroom, "recovery_reserve_bytes": reserve,
            "admission": if used.checked_add(headroom).is_some_and(|n| n <= quota.saturating_sub(reserve)) { "available" } else { "exhausted" },
            "recovery": if used.checked_add(headroom).is_some_and(|n| n <= quota) { "available" } else { "exhausted" }}),
        _ => json!({"state": "unknown", "reason": "physical metadata measurement unavailable"}),
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

const SCHEMA: &str = include_str!("schema.sql");

/// Apply schema version 1 once and register the built-in method versions.
pub fn migrate(c: &mut Client) -> Result<()> {
    let mut tx = write_tx(c, false)?;
    tx.batch_execute("CREATE TABLE IF NOT EXISTS schema_version (version int PRIMARY KEY, applied_at timestamptz NOT NULL DEFAULT now()); LOCK TABLE schema_version IN EXCLUSIVE MODE;")?;
    if tx.query_opt("SELECT 1 FROM schema_version WHERE version <> 1 LIMIT 1", &[])?.is_some() {
        return Err(Error::new(409, "schema_mismatch", "this candidate supports only schema 1"));
    }
    let applied = tx.query_opt("SELECT 1 FROM schema_version WHERE version = 1", &[])?.is_some();
    if !applied {
        tx.batch_execute(SCHEMA)?;
        tx.execute("INSERT INTO schema_version (version) VALUES (1)", &[])?;
    }
    commit_write(tx)?;
    Ok(())
}

pub fn builtin_methods() -> Vec<(&'static str, i32, &'static str, Value)> {
    vec![
        (
            "synthetic.fake",
            1,
            "experimental",
            json!({
                "operation": "synthetic.generate",
                "inputs": ["text", "attachments"],
                "outputs": ["application/x-assetq-synthetic"],
                "provider": "synthetic.fake",
                "model": "none",
                "api_revision": "fake-1",
                "pipeline_revision": "1",
                "parameters": {
                    "outputs": {"type": "integer", "min": 1, "max": 4},
                    "polls": {"type": "integer", "min": 0, "max": 100000},
                    "report_usage": {"type": "boolean"},
                    "fail": {"type": "boolean"}
                },
                "capabilities": ["synthetic.fake"],
                "limitations": ["Synthetic: proves durable-job and recovery semantics only; never real generation."],
                "provenance": "AssetQ built-in fixture",
                "validation": ["sha256 re-read", "size > 0", "declared content type"],
                "steps": ["provider.submit", "provider.import", "validate"],
                "max_credits": 1,
                "synthetic": true
            }),
        ),
        (
            "image.worker.v1",
            1,
            "disabled",
            json!({
                "operation": "image.generate",
                "inputs": ["text", "attachments"],
                "outputs": ["image/png"],
                "provider": "agentq.codex-image",
                "model": "unqualified",
                "api_revision": "unqualified",
                "pipeline_revision": "0",
                "parameters": {},
                "capabilities": ["codex.image_generation"],
                "limitations": ["Not implemented in this build; attached by a later task after real tool qualification."],
                "provenance": "assetq.md initial generation methods",
                "validation": ["bitmap decode", "dimensions", "sha256"],
                "steps": ["provider.submit", "provider.import", "validate"],
                "max_credits": 0
            }),
        ),
        (
            "model.meshy-prop.v1",
            1,
            "disabled",
            json!({
                "operation": "model.image_to_3d",
                "inputs": ["image attachment"],
                "outputs": ["model/gltf-binary", "image/png"],
                "provider": "meshy",
                "model": "meshy-t2",
                "api_revision": "unverified",
                "pipeline_revision": "0",
                "parameters": {},
                "capabilities": ["meshy.image_to_3d", "blender.headless"],
                "limitations": ["Not implemented in this build; provider support and cost need fresh verification before dispatch."],
                "provenance": "docs/meshy-baseline.md",
                "validation": ["GLB parse", "triangle count", "Blender render"],
                "steps": ["provider.submit", "provider.import", "validate"],
                "max_credits": 0
            }),
        ),
        ("image.worker.v1", 2, "experimental", crate::image_worker::legacy_method()),
        ("image.worker.v1", 3, "experimental", crate::image_worker::method()),
    ]
}

// shortcut: one admitting writer across processes; use byte reservations if concurrent streaming is needed.
const ADMISSION_LOCK: i64 = 0x415353455451;
pub fn with_admission<T>(c: &mut Client, quota: u64, f: impl FnOnce(&mut Client) -> Result<T>) -> Result<T> {
    with_writer(c, quota, false, f)
}

pub fn with_recovery<T>(c: &mut Client, quota: u64, f: impl FnOnce(&mut Client) -> Result<T>) -> Result<T> {
    with_writer(c, quota, true, f)
}

fn with_writer<T>(c: &mut Client, quota: u64, recovery: bool, f: impl FnOnce(&mut Client) -> Result<T>) -> Result<T> {
    let locked: bool = c.query_one("SELECT pg_try_advisory_lock($1)", &[&ADMISSION_LOCK])?.get(0);
    if !locked {
        return Err(Error::new(429, "admission_busy", "another writer holds the admission reservation"));
    }
    let result = (|| {
        let (_, headroom, reserve) = metadata_limits(c)?;
        if metadata_bytes(c)?
            .checked_add(headroom)
            .and_then(|n| n.checked_add(if recovery { 0 } else { reserve }))
            .is_none_or(|n| n > quota)
        {
            return Err(Error::new(507, "metadata_full", "database admission quota exhausted"));
        }
        f(c)
    })();
    c.query_one("SELECT pg_advisory_unlock($1)", &[&ADMISSION_LOCK])?;
    result
}

pub fn lock_job(c: &mut impl GenericClient, id: &str) -> Result<bool> {
    Ok(c.query_one("SELECT pg_try_advisory_lock(hashtextextended($1, 42))", &[&id])?.get(0))
}
pub fn unlock_job(c: &mut Client, id: &str) -> Result<()> {
    c.query_one("SELECT pg_advisory_unlock(hashtextextended($1, 42))", &[&id])?;
    Ok(())
}

// ---------------------------------------------------------------- grants

#[derive(Clone, Debug)]
pub struct Principal {
    pub project: String,
    pub role: String,
}

impl Principal {
    pub fn require_owner(&self) -> Result<()> {
        if self.role == "owner" { Ok(()) } else { Err(Error::new(403, "forbidden", "owner grant required")) }
    }
}

pub fn authenticate(c: &mut impl GenericClient, token: &str) -> Result<Principal> {
    let row = c.query_opt("SELECT project, role FROM tokens WHERE hash = $1 AND revoked_at IS NULL", &[&sha256_hex(token.as_bytes())])?;
    let row = row.ok_or_else(|| Error::new(401, "unauthenticated", "unknown or revoked token"))?;
    Ok(Principal { project: row.get(0), role: row.get(1) })
}

pub fn create_project(c: &mut Client, id: &str, budget: i64, max_active: i32) -> Result<()> {
    let mut tx = write_tx(c, false)?;
    charge(&mut tx, id.len())?;
    tx.execute(
        "INSERT INTO projects (id, budget_credits, max_active_jobs) VALUES ($1, $2, $3)
         ON CONFLICT (id) DO UPDATE SET budget_credits = $2, max_active_jobs = $3",
        &[&id, &budget, &max_active],
    )
    .map_err(|e| Error::new(400, "bad_project", e.to_string()))?;
    for (mid, version, status, spec) in builtin_methods() {
        charge(&mut tx, spec.to_string().len())?;
        tx.execute(
            "INSERT INTO methods (project, id, version, status, spec) VALUES ($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING",
            &[&id, &mid, &version, &status, &spec],
        )?;
    }
    commit_write(tx)?;
    Ok(())
}

/// Create a grant and return the secret once; only its hash is stored.
pub fn create_token(c: &mut Client, project: &str, role: &str) -> Result<String> {
    let token = format!("aq_{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
    let mut tx = write_tx(c, false)?;
    charge(&mut tx, project.len() + role.len() + token.len())?;
    tx.execute("INSERT INTO tokens (hash, project, role) VALUES ($1, $2, $3)", &[&sha256_hex(token.as_bytes()), &project, &role])
        .map_err(|e| Error::new(400, "bad_grant", e.to_string()))?;
    commit_write(tx)?;
    Ok(token)
}

// ---------------------------------------------------------------- objects

pub fn used_bytes(c: &mut impl GenericClient) -> Result<u64> {
    let v: i64 = c.query_one("SELECT coalesce(sum(size), 0)::bigint FROM blobs", &[])?.get(0);
    Ok(v as u64)
}

pub struct NewObject<'a> {
    pub project: &'a str,
    pub sha256: &'a str,
    pub size: u64,
    pub content_type: &'a str,
    pub kind: &'a str,
    pub job_id: Option<&'a str>,
    pub import_key: Option<&'a str>,
    pub provenance: Value,
    pub synthetic: bool,
    pub retention_days: Option<u32>,
}

/// Record a committed blob and its project-scoped object row. Idempotent per
/// `(job_id, import_key)` so a resumed import reuses the earlier object.
pub fn insert_object(c: &mut Transaction, o: &NewObject) -> Result<String> {
    charge(c, o.provenance.to_string().len() + o.content_type.len() + o.project.len() + o.import_key.map(str::len).unwrap_or(0))?;
    c.execute("INSERT INTO blobs (sha256, size) VALUES ($1, $2) ON CONFLICT DO NOTHING", &[&o.sha256, &(o.size as i64)])?;
    if let (Some(job), Some(key)) = (o.job_id, o.import_key) {
        if let Some(r) = c.query_opt("SELECT id, sha256 FROM objects WHERE job_id = $1 AND import_key = $2", &[&job, &key])? {
            let sha: String = r.get(1);
            if sha != o.sha256 {
                return Err(Error::new(409, "import_mismatch", format!("{key} was imported with a different hash")));
            }
            return Ok(r.get(0));
        }
    }
    let id = new_id();
    let days = o.retention_days.map(|d| d as i32);
    c.execute(
        "INSERT INTO objects (id, project, sha256, size, content_type, kind, job_id, import_key, provenance, synthetic, retention_until)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, CASE WHEN $11::int IS NULL THEN NULL ELSE now() + make_interval(days => $11::int) END)",
        &[&id, &o.project, &o.sha256, &(o.size as i64), &o.content_type, &o.kind, &o.job_id, &o.import_key, &o.provenance, &o.synthetic, &days],
    )?;
    Ok(id)
}

const OBJECT_COLS: &str =
    "id, sha256, size, content_type, kind, job_id, provenance, synthetic, acceptance, retention_until::text, created_at::text";

fn object_json(r: &postgres::Row) -> Value {
    json!({
        "id": r.get::<_, String>(0),
        "sha256": r.get::<_, String>(1),
        "size": r.get::<_, i64>(2),
        "content_type": r.get::<_, String>(3),
        "kind": r.get::<_, String>(4),
        "job_id": r.get::<_, Option<String>>(5),
        "provenance": r.get::<_, Value>(6),
        "synthetic": r.get::<_, bool>(7),
        "acceptance": r.get::<_, String>(8),
        "retention_until": r.get::<_, Option<String>>(9),
        "created_at": r.get::<_, String>(10),
    })
}

/// A missing object and a foreign project's object read the same (no oracle).
pub fn get_object(c: &mut impl GenericClient, project: &str, id: &str) -> Result<Value> {
    let r = c.query_opt(&format!("SELECT {OBJECT_COLS} FROM objects WHERE id = $1 AND project = $2"), &[&id, &project])?;
    r.map(|r| object_json(&r)).ok_or_else(|| Error::new(404, "not_found", "no such object in this project"))
}

pub fn list_objects(c: &mut impl GenericClient, project: &str, after: Option<&str>, limit: i64) -> Result<Value> {
    let rows = c.query(
        &format!("SELECT {OBJECT_COLS} FROM objects WHERE project = $1 AND ($2::text IS NULL OR id > $2) ORDER BY id LIMIT $3"),
        &[&project, &after, &limit],
    )?;
    let items: Vec<Value> = rows.iter().map(object_json).collect();
    let next = (items.len() as i64 == limit).then(|| items.last().map(|v| v["id"].clone())).flatten();
    Ok(json!({"items": items, "next": next}))
}

pub fn set_acceptance(c: &mut Client, p: &Principal, id: &str, value: &str) -> Result<Value> {
    p.require_owner()?;
    if !matches!(value, "pending" | "accepted" | "rejected") {
        return Err(Error::new(422, "bad_acceptance", "acceptance is pending, accepted or rejected"));
    }
    let mut tx = write_tx(c, false)?;
    charge(&mut tx, id.len() + value.len())?;
    let n = tx.execute("UPDATE objects SET acceptance = $3 WHERE id = $1 AND project = $2", &[&id, &p.project, &value])?;
    if n == 0 {
        return Err(Error::new(404, "not_found", "no such object in this project"));
    }
    commit_write(tx)?;
    get_object(c, &p.project, id)
}

// ---------------------------------------------------------------- methods

const METHOD_STATUSES: &[&str] = &["experimental", "recommended", "deprecated", "disabled"];

pub fn list_methods(c: &mut impl GenericClient, project: &str) -> Result<Value> {
    let rows =
        c.query("SELECT id, version, status, spec, created_at::text FROM methods WHERE project = $1 ORDER BY id, version", &[&project])?;
    Ok(Value::Array(rows.iter().map(method_json).collect()))
}

fn method_json(r: &postgres::Row) -> Value {
    json!({"id": r.get::<_, String>(0), "version": r.get::<_, i32>(1), "status": r.get::<_, String>(2), "spec": r.get::<_, Value>(3), "created_at": r.get::<_, String>(4)})
}

/// Resolve `id` or `id@version`. Unversioned picks the newest recommended or
/// experimental version; disabled versions are refused, never substituted.
pub fn resolve_method(c: &mut impl GenericClient, project: &str, name: &str) -> Result<(String, i32, Value)> {
    let row = match name.split_once('@') {
        Some((id, v)) => {
            let v: i32 = v.parse().map_err(|_| Error::new(422, "bad_method", "version must be an integer"))?;
            c.query_opt("SELECT id, version, status, spec FROM methods WHERE id = $1 AND version = $2 AND project = $3", &[&id, &v, &project])?
        }
        None => c.query_opt(
            "SELECT id, version, status, spec FROM methods WHERE id = $1 AND project = $2 AND status IN ('recommended', 'experimental') ORDER BY version DESC LIMIT 1",
            &[&name, &project],
        )?,
    };
    let r = row.ok_or_else(|| Error::new(422, "unknown_method", format!("no usable method {name}")))?;
    let status: String = r.get(2);
    if status == "disabled" {
        return Err(Error::new(422, "method_disabled", format!("{name} is disabled")));
    }
    Ok((r.get(0), r.get(1), r.get(3)))
}

/// Adding or editing a method creates a new version (R5); old versions stay.
pub fn create_method_version(c: &mut Client, p: &Principal, body: &Value) -> Result<Value> {
    p.require_owner()?;
    let id = body["id"].as_str().filter(|s| !s.is_empty() && s.len() <= 100).ok_or_else(|| Error::new(422, "bad_method", "id required"))?;
    let status = body["status"].as_str().unwrap_or("experimental");
    if !METHOD_STATUSES.contains(&status) {
        return Err(Error::new(422, "bad_method", "unknown status"));
    }
    let spec = &body["spec"];
    validate_spec(spec)?;
    let mut tx = write_tx(c, false)?;
    charge(&mut tx, body.to_string().len())?;
    tx.execute("LOCK TABLE methods IN SHARE ROW EXCLUSIVE MODE", &[])?;
    let v: i32 =
        tx.query_one("SELECT coalesce(max(version), 0) + 1 FROM methods WHERE id = $1 AND project = $2", &[&id, &p.project])?.get(0);
    let r = tx.query_one(
        "INSERT INTO methods (id, version, status, spec, project) VALUES ($1, $2, $3, $4, $5) RETURNING id, version, status, spec, created_at::text",
        &[&id, &v, &status, spec, &p.project],
    )?;
    commit_write(tx)?;
    Ok(method_json(&r))
}

fn validate_spec(spec: &Value) -> Result<()> {
    let need = [
        "operation",
        "inputs",
        "outputs",
        "provider",
        "model",
        "api_revision",
        "pipeline_revision",
        "parameters",
        "capabilities",
        "limitations",
        "provenance",
        "validation",
        "steps",
        "max_credits",
    ];
    if let Some(k) = need.iter().find(|k| spec.get(**k).is_none()) {
        return Err(Error::new(422, "bad_method", format!("spec.{k} required")));
    }
    let steps: Vec<String> =
        serde_json::from_value(spec["steps"].clone()).map_err(|_| Error::new(422, "bad_method", "steps must be strings"))?;
    model::apply(None, &Event::Accepted { method: "check".into(), steps }).map_err(|e| Error::new(422, "bad_method", e.to_string()))?;
    if spec["max_credits"].as_u64().is_none_or(|n| n > i64::MAX as u64) {
        return Err(Error::new(422, "bad_method", "max_credits must be in 0..=i64::MAX"));
    }
    let steps = spec["steps"].as_array().unwrap();
    if steps != &vec![json!("provider.submit"), json!("provider.import"), json!("validate")]
        && steps != &vec![json!("synthetic.generate"), json!("validate")]
    {
        return Err(Error::new(422, "bad_method", "unsupported generation/import/validation sequence"));
    }
    if spec["provider"] == "synthetic.fake" && spec["synthetic"] != true {
        return Err(Error::new(422, "bad_method", "synthetic provider must retain synthetic provenance"));
    }
    if steps[0] == "synthetic.generate" && spec["provider"] != "synthetic.fake" {
        return Err(Error::new(422, "bad_method", "local generation is synthetic only"));
    }
    for k in ["operation", "provider", "model", "api_revision", "pipeline_revision"] {
        if spec[k].as_str().is_none_or(str::is_empty) {
            return Err(Error::new(422, "bad_method", format!("{k} must be a nonempty string")));
        }
    }
    for k in ["inputs", "outputs", "capabilities", "limitations", "validation"] {
        if spec[k].as_array().is_none_or(|a| a.is_empty() || a.iter().any(|v| v.as_str().is_none_or(str::is_empty))) {
            return Err(Error::new(422, "bad_method", format!("{k} must be nonempty strings")));
        }
    }
    if !spec["parameters"].is_object() {
        return Err(Error::new(422, "bad_method", "parameters must be an object"));
    }
    Ok(())
}

/// Typed constraints: every key must be declared with a matching type.
fn validate_constraints(params: &Value, constraints: &Value) -> Result<()> {
    let Some(obj) = constraints.as_object() else {
        return Err(Error::new(422, "bad_constraints", "constraints must be an object"));
    };
    for (k, v) in obj {
        let p = params.get(k).ok_or_else(|| Error::new(422, "bad_constraints", format!("{k} is not a declared parameter")))?;
        let ok = match p["type"].as_str() {
            Some("integer") => {
                v.as_i64().is_some_and(|n| p["min"].as_i64().is_none_or(|m| n >= m) && p["max"].as_i64().is_none_or(|m| n <= m))
            }
            Some("number") => v.is_number(),
            Some("string") => v.is_string(),
            Some("boolean") => v.is_boolean(),
            _ => false,
        };
        if !ok {
            return Err(Error::new(422, "bad_constraints", format!("{k} has the wrong type or range")));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- jobs

pub struct Limits {
    pub quota_bytes: u64,
}

const MAX_TEXT: usize = 64 * 1024;
const MAX_ATTACHMENTS: usize = 16;

/// Durable admission (R2, R7). Returns `(job, created)`.
pub fn submit_job(c: &mut Client, p: &Principal, req: &Value, limits: &Limits, providers: &[&str]) -> Result<(Value, bool)> {
    let bad = |m: &str| Error::new(422, "bad_request", m.to_string());
    let key = req["idempotency_key"]
        .as_str()
        .filter(|k| !k.is_empty() && k.len() <= 200)
        .ok_or_else(|| bad("idempotency_key (1..200 chars) required"))?;
    if let Some(proj) = req.get("project") {
        if proj.as_str() != Some(p.project.as_str()) {
            return Err(Error::new(403, "forbidden", "request project differs from the grant"));
        }
    }
    let method_name = req["method"].as_str().ok_or_else(|| bad("method required"))?;
    let operation = req["operation"].as_str().ok_or_else(|| bad("operation required"))?;
    let text = req.get("text").map(|t| t.as_str().ok_or_else(|| bad("text must be a string"))).transpose()?.unwrap_or("");
    if text.len() > MAX_TEXT {
        return Err(Error::new(413, "too_large", "text exceeds 65536 bytes"));
    }
    let attachments: Vec<String> =
        serde_json::from_value(req.get("attachments").cloned().unwrap_or(json!([]))).map_err(|_| bad("attachments must be object IDs"))?;
    if attachments.len() > MAX_ATTACHMENTS {
        return Err(Error::new(413, "too_large", "at most 16 attachments"));
    }
    let constraints = req.get("constraints").cloned().unwrap_or(json!({}));
    let budget = req["budget"]["max_credits"].as_u64().ok_or_else(|| bad("budget.max_credits required"))?;
    if let Some(k) = req.as_object().unwrap().keys().find(|k| {
        !["project", "operation", "method", "text", "attachments", "constraints", "budget", "idempotency_key"].contains(&k.as_str())
    }) {
        // A request cannot carry permissions or configuration (R3).
        return Err(bad(&format!("unknown field {k}")));
    }
    let canonical = json!({"operation": operation, "method": method_name, "text": text, "attachments": attachments, "constraints": constraints, "budget": {"max_credits": budget}});
    let hash = sha256_hex(canonical.to_string().as_bytes());

    let mut tx = write_tx(c, false)?;
    // Serialize admission per project so limits and idempotency are exact.
    let prow = tx.query_one("SELECT budget_credits, max_active_jobs FROM projects WHERE id = $1 FOR UPDATE", &[&p.project])?;
    if let Some(r) = tx.query_opt("SELECT id, request_hash FROM jobs WHERE project = $1 AND idempotency_key = $2", &[&p.project, &key])? {
        let existing: String = r.get(1);
        if existing != hash {
            return Err(Error::new(409, "idempotency_conflict", "this idempotency key was used with a different request"));
        }
        let id: String = r.get(0);
        tx.commit()?;
        return Ok((get_job(c, &p.project, &id)?, false));
    }
    let (mid, mver, spec) = resolve_method(&mut tx, &p.project, method_name)?;
    if spec["operation"].as_str() != Some(operation) {
        return Err(Error::new(422, "operation_mismatch", format!("{mid}@{mver} performs {}", spec["operation"])));
    }
    let provider = spec["provider"].as_str().unwrap_or("");
    if !providers.contains(&provider) {
        return Err(Error::new(422, "capability_unavailable", format!("provider {provider} is not configured on this service")));
    }
    validate_constraints(&spec["parameters"], &constraints)?;
    for a in &attachments {
        get_object(&mut tx, &p.project, a)
            .map_err(|_| Error::new(404, "attachment_not_found", format!("attachment {a} is not in this project")))?;
    }
    let reserve = i64::try_from(spec["max_credits"].as_u64().unwrap_or(0)).map_err(|_| bad("method budget exceeds i64::MAX"))?;
    let budget = i64::try_from(budget).map_err(|_| bad("budget exceeds i64::MAX"))?;
    if budget < reserve {
        return Err(Error::new(422, "budget_too_low", format!("method may charge up to {reserve} credits")));
    }
    let (project_budget, max_active): (i64, i32) = (prow.get(0), prow.get(1));
    let active: i64 = tx
        .query_one("SELECT count(*) FROM jobs WHERE project = $1 AND phase NOT IN ('succeeded', 'failed', 'cancelled')", &[&p.project])?
        .get(0);
    if active >= max_active as i64 {
        return Err(Error::new(429, "concurrency_limit", format!("{active} active jobs; limit {max_active}")));
    }
    // Unknown usage keeps its full reservation (R4).
    let spent: i64 = tx.query_one(
        "SELECT coalesce(sum(CASE
            WHEN phase IN ('succeeded', 'failed', 'cancelled') AND state->>'usage_credits' IS NOT NULL THEN (state->>'usage_credits')::bigint
            WHEN phase IN ('succeeded', 'failed', 'cancelled') AND state->>'submissions' = '0' THEN 0
            ELSE reserved_credits END), 0)::bigint FROM jobs WHERE project = $1",
        &[&p.project],
    )?.get(0);
    if spent.checked_add(reserve).is_none_or(|v| v > project_budget) {
        return Err(Error::new(402, "budget_exhausted", format!("spent or reserved {spent} of {project_budget} credits")));
    }
    let used = used_bytes(&mut tx)?;
    if used >= limits.quota_bytes {
        return Err(Error::new(507, "store_full", format!("store uses {used} of {} bytes", limits.quota_bytes)));
    }
    let id = new_id();
    let steps: Vec<String> =
        serde_json::from_value(spec["steps"].clone()).map_err(|_| Error::new(500, "bad_method", "stored steps invalid"))?;
    let method = format!("{mid}@{mver}");
    let event = Event::Accepted { method, steps };
    let state = model::apply(None, &event)?;
    charge(&mut tx, canonical.to_string().len() + serde_json::to_string(&state).unwrap().len() + key.len())?;
    tx.execute(
        "INSERT INTO jobs (id, project, idempotency_key, request_hash, request, method_id, method_version, phase, state, reserved_credits, events)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 1)",
        &[&id, &p.project, &key, &hash, &canonical, &mid, &mver, &state.phase.as_str(), &serde_json::to_value(&state).unwrap(), &reserve],
    )?;
    tx.execute("INSERT INTO job_events (job_id, seq, event) VALUES ($1, 1, $2)", &[&id, &serde_json::to_value(&event).unwrap()])?;
    commit_write(tx)?;
    Ok((get_job(c, &p.project, &id)?, true))
}

fn job_json(r: &postgres::Row) -> Value {
    let state: Value = r.get(3);
    json!({
        "id": r.get::<_, String>(0),
        "method": format!("{}@{}", r.get::<_, String>(1), r.get::<_, i32>(2)),
        "phase": state["phase"],
        "step": state["step"],
        "steps": state["steps"],
        "effect": state["effect"],
        "provider_operation": state["op_id"],
        "submissions": state["submissions"],
        "cancel": state["cancel"],
        "usage": {"credits": state["usage_credits"], "unknown_reason": state["usage_note"]},
        "reserved_credits": r.get::<_, i64>(4),
        "reason": state["reason"],
        "attempts": state["attempts"],
        "fence": state["fence"],
        "leased": !state["lease"].is_null(),
        "request": r.get::<_, Value>(5),
        "created_at": r.get::<_, String>(6),
        "updated_at": r.get::<_, String>(7),
    })
}

const JOB_COLS: &str = "id, method_id, method_version, state, reserved_credits, request, created_at::text, updated_at::text";

pub fn get_job(c: &mut impl GenericClient, project: &str, id: &str) -> Result<Value> {
    let r = c.query_opt(&format!("SELECT {JOB_COLS} FROM jobs WHERE id = $1 AND project = $2"), &[&id, &project])?;
    r.map(|r| job_json(&r)).ok_or_else(|| Error::new(404, "not_found", "no such job in this project"))
}

pub fn list_jobs(c: &mut impl GenericClient, project: &str, phase: Option<&str>, after: Option<&str>, limit: i64) -> Result<Value> {
    let rows = c.query(
        &format!("SELECT {JOB_COLS} FROM jobs WHERE project = $1 AND ($2::text IS NULL OR phase = $2) AND ($3::text IS NULL OR id > $3) ORDER BY id LIMIT $4"),
        &[&project, &phase, &after, &limit],
    )?;
    let items: Vec<Value> = rows.iter().map(job_json).collect();
    let next = (items.len() as i64 == limit).then(|| items.last().map(|v| v["id"].clone())).flatten();
    Ok(json!({"items": items, "next": next}))
}

pub fn job_events(c: &mut impl GenericClient, project: &str, id: &str) -> Result<Vec<(i32, Value, String)>> {
    get_job(c, project, id)?;
    let rows = c.query("SELECT seq, event, at::text FROM job_events WHERE job_id = $1 ORDER BY seq", &[&id])?;
    Ok(rows.iter().map(|r| (r.get(0), r.get(1), r.get(2))).collect())
}

/// Replay a stored job's events through the model and compare with its row.
pub fn audit_job(c: &mut impl GenericClient, id: &str) -> Result<Job> {
    let stored: Value = c.query_one("SELECT state FROM jobs WHERE id = $1", &[&id])?.get(0);
    let rows = c.query("SELECT event FROM job_events WHERE job_id = $1 ORDER BY seq", &[&id])?;
    let events: Vec<Event> = rows
        .iter()
        .map(|r| serde_json::from_value(r.get::<_, Value>(0)))
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| Error::new(500, "corrupt_events", e.to_string()))?;
    let replayed = model::replay(&events).map_err(|(i, r)| Error::new(500, "corrupt_events", format!("event {i}: {r}")))?;
    if serde_json::to_value(&replayed).unwrap() != stored {
        return Err(Error::new(500, "replay_mismatch", format!("job {id} state differs from its event replay")));
    }
    Ok(replayed)
}

pub fn load_job(tx: &mut Transaction, id: &str) -> Result<Job> {
    let r = tx.query_opt("SELECT state FROM jobs WHERE id = $1 FOR UPDATE", &[&id])?;
    let r = r.ok_or_else(|| Error::new(404, "not_found", "no such job"))?;
    serde_json::from_value(r.get(0)).map_err(|e| Error::new(500, "corrupt_state", e.to_string()))
}

/// Apply one model event inside `tx`: the only way a job row changes.
/// `wake_ms` schedules the next executor or poller look.
pub fn apply_event(tx: &mut Transaction, id: &str, event: &Event, lease_ms: u64, wake_ms: u64) -> Result<Job> {
    let cur = load_job(tx, id)?;
    let next = model::apply(Some(&cur), event)?;
    if event.fence().is_some() {
        let live: bool = tx
            .query_one("SELECT lease_until > clock_timestamp() FROM jobs WHERE id = $1", &[&id])?
            .get::<_, Option<bool>>(0)
            .unwrap_or(false);
        if !live {
            return Err(Error::new(409, "stale_fence", "lease expired"));
        }
    }
    if next == cur {
        return Ok(next); // idempotent duplicate: nothing to record
    }
    charge(tx, serde_json::to_string(&next).unwrap().len() + serde_json::to_string(event).unwrap().len())?;
    if let Event::SubmitAcked { effect_id, op_id, .. } | Event::Reconciled { effect_id, op_id: Some(op_id) } = event {
        tx.execute("UPDATE effects SET op_id = $2, updated_at = now() WHERE id = $1", &[effect_id, op_id])?;
    }
    if let Event::Claimed { owner, fence } = event {
        tx.execute("INSERT INTO attempts (id, job_id, fence, owner) VALUES ($1,$2,$3,$4)", &[&new_id(), &id, fence, owner])?;
    }
    if cur.lease.is_some() && next.lease.is_none() {
        tx.execute("UPDATE attempts SET ended_at = now() WHERE job_id = $1 AND fence = $2", &[&id, &cur.fence])?;
    }
    let lease_ms = lease_ms as i64;
    let wake_ms = wake_ms as i64;
    tx.execute(
        "UPDATE jobs SET state = $2, phase = $3, fence = $4, lease_owner = $5,
           lease_until = CASE WHEN $5::text IS NULL THEN NULL WHEN $8 THEN now() + make_interval(secs => $6::bigint / 1000.0) ELSE lease_until END,
           wake_at = now() + make_interval(secs => $7::bigint / 1000.0), events = events + 1, updated_at = now()
         WHERE id = $1",
        &[&id, &serde_json::to_value(&next).unwrap(), &next.phase.as_str(), &next.fence, &next.lease, &lease_ms, &wake_ms, &(next.fence != cur.fence)],
    )?;
    tx.execute(
        "INSERT INTO job_events (job_id, seq, event) SELECT $1, events, $2 FROM jobs WHERE id = $1",
        &[&id, &serde_json::to_value(event).unwrap()],
    )?;
    Ok(next)
}

pub fn cancel_job(c: &mut Client, project: &str, id: &str) -> Result<Value> {
    get_job(c, project, id)?;
    let mut tx = write_tx(c, true)?;
    apply_event(&mut tx, id, &Event::CancelRequested, 0, 0)?;
    commit_write(tx)?;
    get_job(c, project, id)
}

/// Result: only a committed, verified manifest is a success (R6).
pub fn job_result(c: &mut impl GenericClient, project: &str, id: &str) -> Result<Value> {
    let job = get_job(c, project, id)?;
    let m = c.query_opt("SELECT items, committed_at::text FROM manifests WHERE job_id = $1", &[&id])?;
    match (job["phase"].as_str(), m) {
        (Some("succeeded"), Some(m)) => Ok(
            json!({"job": job, "status": "succeeded", "manifest": {"items": m.get::<_, Value>(0), "committed_at": m.get::<_, String>(1)}}),
        ),
        (Some(phase @ ("failed" | "cancelled")), _) => Ok(json!({"job": job, "status": phase, "manifest": null, "reason": job["reason"]})),
        (phase, _) => {
            Ok(json!({"job": job, "status": "incomplete", "manifest": null, "reason": format!("job is {}", phase.unwrap_or("unknown"))}))
        }
    }
}

// ---------------------------------------------------------------- recommendations and comparisons

pub fn propose_recommendation(c: &mut Client, p: &Principal, body: &Value) -> Result<Value> {
    let use_case =
        body["use_case"].as_str().filter(|s| !s.is_empty()).ok_or_else(|| Error::new(422, "bad_recommendation", "use_case required"))?;
    let (mid, mver, _) = resolve_method(
        c,
        &p.project,
        body["method"]
            .as_str()
            .filter(|m| m.contains('@'))
            .ok_or_else(|| Error::new(422, "bad_recommendation", "method must be id@version"))?,
    )?;
    let evidence = body
        .get("evidence")
        .filter(|e| e.is_object() || e.is_array())
        .ok_or_else(|| Error::new(422, "bad_recommendation", "evidence required"))?;
    let observed = body["observed_at"].as_str().ok_or_else(|| Error::new(422, "bad_recommendation", "observed_at required"))?;
    let constraints = body.get("constraints").cloned().unwrap_or(json!({}));
    if !constraints.is_object() || evidence.as_object().is_some_and(|v| v.is_empty()) || evidence.as_array().is_some_and(|v| v.is_empty()) {
        return Err(Error::new(422, "bad_recommendation", "object constraints and nonempty evidence required"));
    }
    let id = new_id();
    let mut tx = write_tx(c, false)?;
    charge(&mut tx, body.to_string().len())?;
    tx.execute(
        "INSERT INTO recommendations (id, use_case, constraints, method_id, method_version, evidence, observed_at, state, proposed_by, project)
         VALUES ($1, $2, $3, $4, $5, $6, $7::text::timestamptz, 'proposed', $8, $8)",
        &[&id, &use_case, &constraints, &mid, &mver, evidence, &observed, &p.project],
    )
    .map_err(|e| Error::new(422, "bad_recommendation", e.to_string()))?;
    commit_write(tx)?;
    get_recommendation(c, &p.project, &id)
}

/// Promotion follows the owner-approved policy: an owner grant (R1, methods section).
pub fn promote_recommendation(c: &mut Client, p: &Principal, id: &str) -> Result<Value> {
    p.require_owner()?;
    let mut tx = write_tx(c, false)?;
    let r = tx.query_opt(
        "SELECT use_case, constraints FROM recommendations WHERE id = $1 AND project = $2 AND state = 'proposed' FOR UPDATE",
        &[&id, &p.project],
    )?;
    let r = r.ok_or_else(|| Error::new(404, "not_found", "no proposed recommendation"))?;
    let use_case: String = r.get(0);
    let constraints: Value = r.get(1);
    let previous = tx.query(
        "SELECT id FROM recommendations WHERE use_case = $1 AND project = $2 AND constraints = $3 AND state = 'promoted'",
        &[&use_case, &p.project, &constraints],
    )?;
    for _ in &previous {
        charge(&mut tx, 0)?;
    }
    charge(&mut tx, constraints.to_string().len())?;
    tx.query_one("SELECT 1 FROM projects WHERE id = $1 FOR UPDATE", &[&p.project])?;
    tx.execute(
        "UPDATE recommendations SET state = 'withdrawn' WHERE use_case = $1 AND project = $2 AND constraints = $3 AND state = 'promoted'",
        &[&use_case, &p.project, &constraints],
    )?;
    tx.execute("UPDATE recommendations SET state = 'promoted', promoted_by = $2 WHERE id = $1", &[&id, &p.project])?;
    commit_write(tx)?;
    get_recommendation(c, &p.project, id)
}

const REC_COLS: &str = "r.id, r.use_case, r.constraints, r.method_id, r.method_version, r.evidence, r.observed_at::text, r.state, r.proposed_by, r.promoted_by, m.status";

fn rec_json(r: &postgres::Row) -> Value {
    json!({
        "id": r.get::<_, String>(0), "use_case": r.get::<_, String>(1), "constraints": r.get::<_, Value>(2),
        "method": format!("{}@{}", r.get::<_, String>(3), r.get::<_, i32>(4)), "evidence": r.get::<_, Value>(5),
        "observed_at": r.get::<_, String>(6), "state": r.get::<_, String>(7), "proposed_by": r.get::<_, String>(8),
        "promoted_by": r.get::<_, Option<String>>(9), "method_status": r.get::<_, String>(10),
    })
}

fn get_recommendation(c: &mut impl GenericClient, project: &str, id: &str) -> Result<Value> {
    let r = c.query_one(
        &format!(
            "SELECT {REC_COLS} FROM recommendations r JOIN methods m ON m.id = r.method_id AND m.version = r.method_version AND m.project = r.project WHERE r.id = $1 AND r.project = $2"
        ),
        &[&id, &project],
    )?;
    Ok(rec_json(&r))
}

/// One qualified route for the use case, or an explicit "none qualified".
pub fn recommendations(c: &mut impl GenericClient, project: &str, use_case: &str, constraints: &Value) -> Result<Value> {
    let rows = c.query(
        &format!("SELECT {REC_COLS} FROM recommendations r JOIN methods m ON m.id = r.method_id AND m.version = r.method_version AND m.project = r.project WHERE r.use_case = $1 AND r.project = $2 ORDER BY r.created_at DESC"),
        &[&use_case, &project],
    )?;
    let all: Vec<Value> = rows.iter().map(rec_json).collect();
    let qualified = all
        .iter()
        .find(|r| {
            r["state"] == "promoted"
                && &r["constraints"] == constraints
                && r["method_status"] != "disabled"
                && r["method_status"] != "deprecated"
        })
        .cloned();
    let reason =
        if qualified.is_some() { Value::Null } else { json!("no promoted recommendation with an enabled method for this use case") };
    Ok(json!({"use_case": use_case, "recommended": qualified, "none_reason": reason, "records": all,
              "note": "recommendations are evidence records, separate from provider health"}))
}

pub fn add_comparison(c: &mut Client, p: &Principal, body: &Value) -> Result<Value> {
    let s = |k: &str| body[k].as_str().filter(|v| !v.is_empty()).ok_or_else(|| Error::new(422, "bad_comparison", format!("{k} required")));
    let (use_case, dataset, evaluator) = (s("use_case")?, s("dataset_version")?, s("evaluator_version")?);
    let methods = body
        .get("method_versions")
        .filter(|v| v.as_array().is_some_and(|a| !a.is_empty()))
        .ok_or_else(|| Error::new(422, "bad_comparison", "method_versions required"))?;
    for m in methods.as_array().unwrap() {
        let name =
            m.as_str().filter(|m| m.contains('@')).ok_or_else(|| Error::new(422, "bad_comparison", "method_versions are id@version"))?;
        let (id, v) = name.split_once('@').unwrap();
        let v: i32 = v.parse().map_err(|_| Error::new(422, "bad_comparison", "bad version"))?;
        if c.query_opt("SELECT 1 FROM methods WHERE id = $1 AND version = $2 AND project = $3", &[&id, &v, &p.project])?.is_none() {
            return Err(Error::new(422, "bad_comparison", format!("unknown method {name}")));
        }
    }
    let results = body.get("results").cloned().ok_or_else(|| Error::new(422, "bad_comparison", "results required"))?;
    let missing = body.get("missing_cases").cloned().unwrap_or(json!([]));
    let id = new_id();
    let mut tx = write_tx(c, false)?;
    charge(&mut tx, body.to_string().len())?;
    tx.execute(
        "INSERT INTO comparisons (id, use_case, dataset_version, evaluator_version, method_versions, results, missing_cases, created_by) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        &[&id, &use_case, &dataset, &evaluator, methods, &results, &missing, &p.project],
    )?;
    commit_write(tx)?;
    Ok(json!({"id": id}))
}

pub fn list_comparisons(c: &mut impl GenericClient, project: &str, use_case: Option<&str>) -> Result<Value> {
    let rows = c.query(
        "SELECT id, use_case, dataset_version, evaluator_version, method_versions, results, missing_cases, created_by, created_at::text FROM comparisons WHERE created_by = $2 AND ($1::text IS NULL OR use_case = $1) ORDER BY created_at",
        &[&use_case, &project],
    )?;
    Ok(Value::Array(
        rows.iter()
            .map(|r| {
                json!({
                    "id": r.get::<_, String>(0), "use_case": r.get::<_, String>(1), "dataset_version": r.get::<_, String>(2),
                    "evaluator_version": r.get::<_, String>(3), "method_versions": r.get::<_, Value>(4), "results": r.get::<_, Value>(5),
                    "missing_cases": r.get::<_, Value>(6), "created_by": r.get::<_, String>(7), "created_at": r.get::<_, String>(8),
                })
            })
            .collect(),
    ))
}

pub fn observe(c: &mut Client, kind: &str, subject: &str, value: &Value) -> Result<()> {
    let mut tx = write_tx(c, false)?;
    charge(&mut tx, kind.len() + subject.len() + value.to_string().len())?;
    tx.execute(
        "INSERT INTO observations (kind, subject, value) VALUES ($1, $2, $3) ON CONFLICT (kind, subject) DO UPDATE SET value = $3, observed_at = now()",
        &[&kind, &subject, value],
    )?;
    commit_write(tx)?;
    Ok(())
}

/// Operational status. Missing observations read as unknown with a reason.
pub fn status(c: &mut impl GenericClient, project: &str, quota: u64, providers: &[&str]) -> Result<Value> {
    let mut phases = serde_json::Map::new();
    for r in c.query("SELECT phase, count(*) FROM jobs WHERE project = $1 GROUP BY phase", &[&project])? {
        phases.insert(r.get(0), json!(r.get::<_, i64>(1)));
    }
    let uncertain: i64 = c.query_one("SELECT count(*) FROM jobs WHERE project = $1 AND state->>'effect' IN ('uncertain', 'intended') AND phase NOT IN ('succeeded','failed','cancelled')", &[&project])?.get(0);
    let leased: i64 = c.query_one("SELECT count(*) FROM jobs WHERE project = $1 AND lease_owner IS NOT NULL", &[&project])?.get(0);
    let mut methods = vec![];
    for r in c.query("SELECT method_id || '@' || method_version, phase, count(*), count(*) FILTER (WHERE state->>'usage_credits' IS NULL AND (state->>'submissions')::int > 0), coalesce(sum((state->>'usage_credits')::bigint), 0)::bigint FROM jobs WHERE project = $1 GROUP BY 1, 2 ORDER BY 1, 2", &[&project])? {
        methods.push(json!({"method": r.get::<_, String>(0), "phase": r.get::<_, String>(1), "jobs": r.get::<_, i64>(2), "usage_unknown": r.get::<_, i64>(3), "credits_reported": r.get::<_, i64>(4)}));
    }
    let used = used_bytes(c)?;
    let mut obs = serde_json::Map::new();
    for r in c.query(
        "SELECT kind || ':' || subject, value, observed_at::text FROM observations WHERE observed_at > now() - interval '5 minutes'",
        &[],
    )? {
        obs.insert(r.get(0), json!({"value": r.get::<_, Value>(1), "observed_at": r.get::<_, String>(2)}));
    }
    let provider_health: Vec<Value> = providers
        .iter()
        .map(|p| {
            obs.get(&format!("provider:{p}"))
                .cloned()
                .unwrap_or_else(|| json!({"provider": p, "health": "unknown", "reason": "no fresh health observation recorded"}))
        })
        .collect();
    let backup = obs
        .get("backup:store")
        .cloned()
        .unwrap_or_else(|| json!({"state": "unknown", "reason": "no fresh backup or restore observation recorded"}));
    let integrity = obs
        .get("integrity:store")
        .cloned()
        .unwrap_or_else(|| json!({"state": "unknown", "reason": "no fresh full integrity scan recorded"}));
    Ok(json!({
        "project": project,
        "jobs": {"by_phase": phases, "uncertain_submissions": uncertain, "leased": leased},
        "methods": methods,
        "providers": provider_health,
        "store": {"used_bytes": used, "quota_bytes": quota, "full": used >= quota, "integrity": integrity},
        "backup": backup,
        "states": "generation success (job phase), asset validation (manifest) and owner acceptance (object acceptance) are separate fields",
    }))
}

/// Read-only schema-specific restored-state verification (R5, R6, A8).
/// Run against a quiescent isolated restore, then qualify the API separately.
pub fn restore_check(c: &mut Client, store: &crate::store::Store) -> Result<Value> {
    let mut tx = c.build_transaction().read_only(true).isolation_level(postgres::IsolationLevel::RepeatableRead).start()?;
    let versions: Vec<i32> = tx.query("SELECT version FROM schema_version ORDER BY version", &[])?.iter().map(|r| r.get(0)).collect();
    if versions != [1] {
        return Err(Error::new(409, "schema_mismatch", "expected exactly schema version 1"));
    }
    let blobs = tx.query("SELECT sha256, size FROM blobs", &[])?;
    for b in &blobs {
        store.verify(&b.get::<_, String>(0), b.get::<_, i64>(1) as u64)?;
    }
    let jobs = tx.query("SELECT id, project FROM jobs ORDER BY id", &[])?;
    let mut succeeded = 0;
    for row in &jobs {
        let (id, project): (String, String) = (row.get(0), row.get(1));
        let state = audit_job(&mut tx, &id)?;
        let manifest = tx.query_opt("SELECT items FROM manifests WHERE job_id = $1", &[&id])?;
        if state.manifest != manifest.is_some() || state.manifest != (state.phase == model::Phase::Succeeded) {
            return Err(Error::new(409, "manifest_mismatch", format!("job {id}: success and manifest disagree")));
        }
        if let Some(manifest) = manifest {
            let items: Value = manifest.get(0);
            let items = items.as_array().ok_or_else(|| Error::new(409, "manifest_mismatch", "manifest items not an array"))?;
            let mut keys = std::collections::BTreeSet::new();
            for item in items {
                let object = get_object(&mut tx, &project, item["object_id"].as_str().unwrap_or(""))?;
                if object["sha256"] != item["sha256"]
                    || object["size"] != item["size"]
                    || object["content_type"] != item["content_type"]
                    || object["job_id"] != id
                    || object["provenance"]["import_key"] != item["import_key"]
                {
                    return Err(Error::new(409, "manifest_mismatch", format!("job {id}: object record differs")));
                }
                keys.insert(item["import_key"].as_str().unwrap_or("").to_owned());
            }
            if items.len() != keys.len() || keys != state.imported || keys.is_empty() {
                return Err(Error::new(409, "manifest_mismatch", format!("job {id}: missing or duplicate manifest items")));
            }
            succeeded += 1;
        }
    }
    let methods: i64 = tx.query_one("SELECT count(*) FROM methods", &[])?.get(0);
    let attempts: i64 = tx.query_one("SELECT count(*) FROM attempts", &[])?.get(0);
    tx.commit()?;
    let (objects, temp) = store.inventory().map_err(|e| Error::new(503, "store_unknown", e.to_string()))?;
    Ok(json!({"schema": 1, "jobs": jobs.len(), "succeeded": succeeded, "blobs": blobs.len(), "methods": methods,
        "attempts": attempts, "physical_object_bytes": objects, "retained_temp_bytes": temp, "verified": true}))
}
