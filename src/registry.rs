//! Immutable evaluation inputs and bounded comparisons over ordinary jobs (R5/R7).
use crate::{Error, Result, db};
use db::Principal;
use postgres::{Client, GenericClient};
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn bad(reason: impl Into<String>) -> Error {
    Error::new(422, "bad_comparison", reason)
}

fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key].as_str().filter(|s| !s.is_empty() && s.len() <= 65536).ok_or_else(|| bad(format!("{key} required")))
}

pub fn register(c: &mut Client, p: &Principal, kind: &str, body: &Value) -> Result<Value> {
    p.require_owner()?;
    let id = text(body, "id")?;
    if id.contains('@') || id.len() > 100 || !matches!(kind, "dataset" | "evaluator") {
        return Err(bad("invalid revision identity"));
    }
    let version =
        body["version"].as_i64().filter(|v| *v > 0 && *v <= i32::MAX as i64).ok_or_else(|| bad("positive version required"))? as i32;
    let doc = &body["document"];
    if !doc.is_object() || doc.to_string().len() > 128 * 1024 {
        return Err(bad("document must be an object at most 128 KiB"));
    }
    if kind == "dataset" {
        text(doc, "operation")?;
        let cases = doc["cases"].as_array().filter(|a| !a.is_empty() && a.len() <= 16).ok_or_else(|| bad("1-16 cases required"))?;
        let mut ids = BTreeSet::new();
        for case in cases {
            if !ids.insert(text(case, "id")?) || case["text"].as_str().is_none_or(|s| s.len() > 65536) {
                return Err(bad("unique case IDs and bounded text required"));
            }
            let attachments: Vec<String> =
                serde_json::from_value(case["attachments"].clone()).map_err(|_| bad("attachments must be object IDs"))?;
            if attachments.len() > 16 {
                return Err(bad("at most 16 attachments per case"));
            }
            for a in attachments {
                db::get_object(c, &p.project, &a)?;
            }
            if case.as_object().unwrap().keys().any(|k| !["id", "text", "attachments"].contains(&k.as_str())) {
                return Err(bad("case contains unsupported fields"));
            }
        }
    } else {
        text(doc, "decision_rule")?;
        text(doc, "quality_rubric")?;
    }
    let hash = db::sha256_hex(doc.to_string().as_bytes());
    let mut tx = db::write_tx(c, false)?;
    db::charge(&mut tx, body.to_string().len())?;
    if let Some(r) = tx.query_opt(
        "SELECT sha256 FROM benchmark_revisions WHERE project=$1 AND kind=$2 AND id=$3 AND version=$4",
        &[&p.project, &kind, &id, &version],
    )? {
        if r.get::<_, String>(0) != hash {
            return Err(Error::new(409, "immutable_revision", "revision already has different contents"));
        }
    } else {
        tx.execute(
            "INSERT INTO benchmark_revisions (project,kind,id,version,document,sha256) VALUES ($1,$2,$3,$4,$5,$6)",
            &[&p.project, &kind, &id, &version, doc, &hash],
        )?;
    }
    db::commit_write(tx)?;
    Ok(json!({"id": id, "version": version, "sha256": hash, "document": doc}))
}

fn revision(c: &mut impl GenericClient, project: &str, kind: &str, name: &str) -> Result<Value> {
    let (id, version) = name.split_once('@').ok_or_else(|| bad("revision must be id@version"))?;
    let version: i32 = version.parse().map_err(|_| bad("invalid version"))?;
    let row = c
        .query_opt(
            "SELECT document,sha256 FROM benchmark_revisions WHERE project=$1 AND kind=$2 AND id=$3 AND version=$4",
            &[&project, &kind, &id, &version],
        )?
        .ok_or_else(|| Error::new(404, "not_found", "no such revision in this project"))?;
    Ok(json!({"name": name, "document": row.get::<_, Value>(0), "sha256": row.get::<_, String>(1)}))
}

pub fn revisions(c: &mut Client, project: &str, kind: &str, after: &str, limit: i64) -> Result<Value> {
    let rows = c.query("SELECT id,version,document,sha256 FROM benchmark_revisions WHERE project=$1 AND kind=$2 AND id || '@' || version > $3 ORDER BY id || '@' || version LIMIT $4", &[&project, &kind, &after, &limit])?;
    Ok(Value::Array(rows.iter().map(|r| json!({"id": r.get::<_, String>(0), "version": r.get::<_, i32>(1), "document": r.get::<_, Value>(2), "sha256": r.get::<_, String>(3)})).collect()))
}

/// Freeze the complete matrix before admitting any sample; versions never float.
pub fn prepare(c: &mut Client, p: &Principal, body: &Value) -> Result<Value> {
    let plan = &body["plan"];
    let dataset = revision(c, &p.project, "dataset", text(plan, "dataset")?)?;
    let evaluator = revision(c, &p.project, "evaluator", text(plan, "evaluator")?)?;
    if body["dataset_version"] != plan["dataset"] || body["evaluator_version"] != plan["evaluator"] {
        return Err(bad("plan and comparison revision identities differ"));
    }
    let repeats = plan["repeats"].as_u64().filter(|n| (1..=8).contains(n)).ok_or_else(|| bad("repeats must be 1-8"))?;
    let max_jobs = plan["max_jobs"].as_u64().filter(|n| (1..=32).contains(n)).ok_or_else(|| bad("max_jobs must be 1-32"))?;
    let max_credits = plan["max_credits"].as_u64().filter(|n| *n <= i64::MAX as u64).ok_or_else(|| bad("max_credits required"))?;
    let constraints = &plan["constraints"];
    let methods =
        body["method_versions"].as_array().filter(|a| !a.is_empty() && a.len() <= 32).ok_or_else(|| bad("1-32 methods required"))?;
    let cases = dataset["document"]["cases"].as_array().unwrap();
    if cases.len() as u64 * methods.len() as u64 * repeats > max_jobs {
        return Err(bad("matrix exceeds max_jobs"));
    }
    let mut samples = vec![];
    let mut cost = 0u64;
    let mut seen = BTreeSet::new();
    for method in methods {
        let name = method.as_str().filter(|m| m.contains('@')).ok_or_else(|| bad("exact method versions required"))?;
        if !seen.insert(name) {
            return Err(bad("duplicate candidate"));
        }
        let (_, _, spec) = db::resolve_method(c, &p.project, name)?;
        if spec["operation"] != dataset["document"]["operation"] {
            return Err(bad("candidate and dataset operations differ"));
        }
        db::validate_constraints(&spec["parameters"], constraints)?;
        for case in cases {
            let mut inputs = vec![];
            for a in case["attachments"].as_array().unwrap() {
                let o = db::get_object(c, &p.project, a.as_str().unwrap())?;
                inputs.push(json!({"id": o["id"], "sha256": o["sha256"], "size": o["size"]}));
            }
            for repeat in 0..repeats {
                let credits = spec["max_credits"].as_u64().unwrap();
                cost = cost.checked_add(credits).filter(|n| *n <= max_credits).ok_or_else(|| bad("matrix exceeds credit budget"))?;
                samples.push(json!({"sample": samples.len().to_string(), "case": case["id"], "repeat": repeat, "method": name,
                    "method_sha256": db::sha256_hex(spec.to_string().as_bytes()), "inputs": inputs,
                    "synthetic": spec["provider"] == "synthetic.fake" && spec["synthetic"] == true,
                    "request": {"method": name, "operation": spec["operation"], "text": case["text"], "attachments": case["attachments"],
                    "constraints": constraints, "budget": {"max_credits": credits}}}));
            }
        }
    }
    Ok(json!({"dataset": dataset, "evaluator": evaluator, "operation": dataset["document"]["operation"], "use_case": body["use_case"],
        "constraints": constraints, "repeats": repeats, "max_jobs": max_jobs, "max_credits": max_credits, "reserved_max_credits": cost, "samples": samples}))
}

fn plan(c: &mut Client, project: &str, id: &str) -> Result<Value> {
    c.query_opt("SELECT plan FROM benchmark_plans WHERE comparison=$1 AND project=$2", &[&id, &project])?
        .map(|r| r.get(0))
        .ok_or_else(|| Error::new(404, "not_found", "no executable comparison in this project"))
}

fn sample_key(id: &str, sample: &Value) -> String {
    format!("comparison:{id}:{}", sample["sample"].as_str().unwrap())
}

pub fn run(c: &mut Client, p: &Principal, id: &str, quota: u64, providers: &[&str]) -> Result<Value> {
    p.require_owner()?;
    let plan = plan(c, &p.project, id)?;
    if plan["samples"].as_array().unwrap().iter().any(|s| s["synthetic"] != true) {
        return Err(Error::new(
            403,
            "experiment_not_authorized",
            "real-provider comparisons need operator qualification and a freshly reconciled approved experiment ledger; no paid budget is granted here",
        ));
    }
    let mut refusal = Value::Null;
    for sample in plan["samples"].as_array().unwrap() {
        let mut req = sample["request"].clone();
        req["idempotency_key"] = json!(sample_key(id, sample));
        if let Err(e) = db::submit_job(c, p, &req, &db::Limits { quota_bytes: quota }, providers) {
            refusal = e.json();
            break;
        }
    }
    let mut result = result(c, &p.project, id)?;
    result["admission_refusal"] = refusal;
    Ok(result)
}

pub fn result(c: &mut Client, project: &str, id: &str) -> Result<Value> {
    let plan = match c.query_opt("SELECT plan FROM benchmark_plans WHERE comparison=$1 AND project=$2", &[&id, &project])? {
        Some(row) => row.get::<_, Value>(0),
        None => return db::get_comparison(c, project, id),
    };
    let mut samples = vec![];
    let mut missing = vec![];
    let (mut failures, mut terminal, mut reported, mut unknown, mut judged) = (0u64, 0u64, 0u64, 0u64, 0u64);
    for sample in plan["samples"].as_array().unwrap() {
        let key = sample_key(id, sample);
        let row = c.query_opt(
            "SELECT id, (extract(epoch FROM updated_at-created_at)*1000)::bigint FROM jobs WHERE project=$1 AND idempotency_key=$2",
            &[&project, &key],
        )?;
        let mut item = sample.clone();
        let score = c
            .query_opt(
                "SELECT judgment || jsonb_build_object('observed_at',observed_at::text) FROM benchmark_scores WHERE comparison=$1 AND sample=$2",
                &[&id, &sample["sample"].as_str().unwrap()],
            )?
            .map(|r| r.get::<_, Value>(0))
            .unwrap_or(Value::Null);
        item["human_quality"] = score.clone();
        if let Some(row) = row {
            let job = db::get_job(c, project, &row.get::<_, String>(0))?;
            if job["request"] != sample["request"] || job["method"] != sample["method"] {
                return Err(Error::new(409, "sample_conflict", "sample key belongs to different immutable inputs"));
            }
            let phase = job["phase"].as_str().unwrap();
            let done = matches!(phase, "succeeded" | "failed" | "cancelled");
            terminal += u64::from(done);
            failures += u64::from(matches!(phase, "failed" | "cancelled"));
            if !done {
                missing.push(json!({"sample": sample["sample"], "reason": "job not terminal"}));
            }
            let credits = job["usage"]["credits"].as_u64();
            reported += credits.unwrap_or(0);
            if credits.is_none() {
                unknown += 1;
                missing.push(json!({"sample": sample["sample"], "reason": "usage unknown"}));
            }
            if phase == "succeeded" && score.is_null() {
                missing.push(json!({"sample": sample["sample"], "reason": "human judgment missing"}));
            }
            judged += u64::from(!score.is_null());
            item["technical"] = db::job_result(c, project, job["id"].as_str().unwrap())?;
            item["technical_score"] = if done { json!(u8::from(phase == "succeeded")) } else { Value::Null };
            item["duration_ms"] = if done { json!(row.get::<_, i64>(1)) } else { Value::Null };
            item["job"] = job;
        } else {
            unknown += 1;
            missing.push(json!({"sample": sample["sample"], "reason": "not admitted"}));
            item["job"] = Value::Null;
            item["duration_ms"] = Value::Null;
            item["technical_score"] = Value::Null;
        }
        samples.push(item);
    }
    let count = samples.len() as u64;
    let names: BTreeSet<&str> = samples.iter().map(|s| s["method"].as_str().unwrap()).collect();
    let metrics: Vec<Value> = names.into_iter().map(|method| {
        let group: Vec<&Value> = samples.iter().filter(|s| s["method"] == method).collect();
        let finished = group.iter().filter(|s| matches!(s["job"]["phase"].as_str(), Some("succeeded" | "failed" | "cancelled"))).count();
        let failed = group.iter().filter(|s| matches!(s["job"]["phase"].as_str(), Some("failed" | "cancelled"))).count();
        let costs: Vec<u64> = group.iter().filter_map(|s| s["job"]["usage"]["credits"].as_u64()).collect();
        let scores: Vec<u64> = group.iter().filter_map(|s| s["human_quality"]["score"].as_u64()).collect();
        let durations: Vec<i64> = group.iter().filter_map(|s| s["duration_ms"].as_i64()).collect();
        json!({"method": method, "samples": group.len(), "terminal": finished, "failures": failed,
            "failure_rate": if finished == group.len() {json!(failed as f64 / group.len() as f64)} else {Value::Null},
            "technical_valid": group.iter().filter(|s| s["technical"]["status"] == "succeeded").count(),
            "credits_reported": costs.iter().sum::<u64>(), "credits": if costs.len()==group.len() {json!(costs.iter().sum::<u64>())} else {Value::Null},
            "usage_unknown": group.len()-costs.len(), "duration_ms": if durations.len()==group.len() {json!(durations.iter().sum::<i64>())} else {Value::Null},
            "human_scores": scores, "human_score_mean": if scores.is_empty() {Value::Null} else {json!(scores.iter().sum::<u64>() as f64 / scores.len() as f64)},
            "human_judgments_missing": group.iter().filter(|s| s["job"]["phase"] == "succeeded" && s["human_quality"].is_null()).count()})
    }).collect();
    let mut output = json!({"id": id, "plan": plan, "samples": samples, "missing_cases": missing,
        "method_metrics": metrics,
        "state": if missing.is_empty() { "complete" } else { "incomplete" }, "terminal": terminal, "failures": failures,
        "failure_rate": if terminal == count { json!(failures as f64 / count as f64) } else { Value::Null },
        "credits_reported": reported, "credits": if unknown == 0 { json!(reported) } else { Value::Null }, "usage_unknown": unknown,
        "human_judgments": judged, "winner": null, "winner_reason": "owner assessment of the frozen decision rule required; no automatic promotion"});
    output["revision"] = json!(db::sha256_hex(output.to_string().as_bytes()));
    Ok(output)
}

pub fn score(c: &mut Client, p: &Principal, id: &str, body: &Value) -> Result<Value> {
    let sample = text(body, "sample")?;
    let score = body["score"].as_u64().filter(|n| *n <= 100).ok_or_else(|| bad("human score must be 0-100"))?;
    let judgment = text(body, "judgment")?;
    let reviewer = text(body, "reviewer")?;
    let mut tx = db::write_tx(c, false)?;
    let plan = tx
        .query_opt("SELECT plan FROM benchmark_plans WHERE comparison=$1 AND project=$2", &[&id, &p.project])?
        .ok_or_else(|| Error::new(404, "not_found", "no comparison in this project"))?
        .get::<_, Value>(0);
    let entry = plan["samples"].as_array().unwrap().iter().find(|s| s["sample"] == sample).ok_or_else(|| bad("unknown sample"))?;
    let key = sample_key(id, entry);
    if tx.query_opt("SELECT 1 FROM jobs WHERE project=$1 AND idempotency_key=$2 AND phase='succeeded'", &[&p.project, &key])?.is_none() {
        return Err(bad("human judgment requires a successful original sample"));
    }
    let record = json!({"score": score, "judgment": judgment, "reviewer": reviewer, "grant_project": p.project,
        "evaluator_sha256": plan["evaluator"]["sha256"]});
    db::charge(&mut tx, record.to_string().len())?;
    if let Some(row) = tx.query_opt("SELECT judgment FROM benchmark_scores WHERE comparison=$1 AND sample=$2", &[&id, &sample])? {
        if row.get::<_, Value>(0) != record {
            return Err(Error::new(409, "immutable_judgment", "existing judgment differs; create a new comparison"));
        }
    } else {
        tx.execute("INSERT INTO benchmark_scores (comparison,sample,judgment) VALUES ($1,$2,$3)", &[&id, &sample, &record])?;
    }
    db::commit_write(tx)?;
    result(c, &p.project, id)
}

/// Receipt-backed metadata, never provider health or dispatch authority.
pub fn qualification(method: &str) -> Value {
    let data: Value = serde_json::from_str(include_str!("method-evidence.json")).expect("compiled receipt records");
    data[method].clone()
}

pub fn qualified_spec(method: &str, spec: &Value) -> Value {
    let expected = match method {
        "image.worker.v1@2" => crate::image_worker::legacy_method(),
        "image.worker.v1@5" => crate::image_worker::method(),
        "model.meshy-prop.v1@2" => crate::meshy::method(),
        _ => return Value::Null,
    };
    if spec != &expected {
        return Value::Null;
    }
    let mut evidence = qualification(method);
    evidence["method_spec_sha256"] = json!(db::sha256_hex(spec.to_string().as_bytes()));
    evidence
}
