//! HTTP API: project-scoped JSON endpoints plus streaming upload/download.
//! Bind it to a local or otherwise protected address; it adds no public route.

use crate::db::{self, NewObject, Principal};
use crate::exec::Ctx;
use crate::{Error, Result};
use postgres::Client;
use serde_json::{Value, json};
use std::io::Read;
use std::sync::Arc;
use tiny_http::{Header, Method, Request, Response, StatusCode};

const MAX_JSON: u64 = 1 << 20;

pub fn bind(listen: &str) -> std::io::Result<tiny_http::Server> {
    let address: std::net::SocketAddr = listen.parse().map_err(std::io::Error::other)?;
    if !address.ip().is_loopback() {
        return Err(std::io::Error::other("API must bind to loopback"));
    }
    tiny_http::Server::http(listen).map_err(std::io::Error::other)
}

/// Serve requests on `threads` workers; each keeps its own database session.
pub fn serve(ctx: Arc<Ctx>, server: Arc<tiny_http::Server>, threads: usize) -> Vec<std::thread::JoinHandle<()>> {
    (0..threads.max(1))
        .map(|_| {
            let (ctx, server) = (ctx.clone(), server.clone());
            std::thread::spawn(move || {
                let mut client: Option<Client> = None;
                while let Ok(req) = server.recv() {
                    handle(&ctx, &mut client, req);
                }
            })
        })
        .collect()
}

fn header(req: &Request, name: &str) -> Option<String> {
    req.headers().iter().find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name)).map(|h| h.value.as_str().to_string())
}

fn json_response(status: u16, v: &Value) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(serde_json::to_vec_pretty(v).unwrap())
        .with_status_code(StatusCode(status))
        .with_header(Header::from_bytes("Content-Type", "application/json").unwrap())
        .with_header(Header::from_bytes("Cache-Control", "private, no-store").unwrap())
}

fn handle(ctx: &Ctx, client: &mut Option<Client>, mut req: Request) {
    if client.as_ref().is_none_or(|c| c.is_closed()) {
        *client = db::connect_config(&ctx.cfg).ok();
    }
    let result = match client.as_mut() {
        Some(c) => {
            if req.method() == &Method::Post {
                // Cancellation and recovery must remain available at capacity.
                if req.url().split('?').next().is_some_and(|p| p.ends_with("/cancel")) {
                    route(ctx, c, &mut req)
                } else {
                    principal(c, &req).and_then(|_| db::with_admission(c, ctx.cfg.metadata_quota_bytes, |c| route(ctx, c, &mut req)))
                }
            } else {
                route(ctx, c, &mut req)
            }
        }
        None => Err(Error::new(503, "database", "database unavailable")),
    };
    let _ = match result {
        Ok(Reply::Json(status, v)) => req.respond(json_response(status, &v)),
        Ok(Reply::File { file, size, sha256, content_type }) => {
            let headers = vec![
                Header::from_bytes("Content-Type", content_type.as_bytes()).unwrap(),
                Header::from_bytes("X-Content-Sha256", sha256.as_bytes()).unwrap(),
                Header::from_bytes("Content-Disposition", "attachment").unwrap(),
                Header::from_bytes("X-Content-Type-Options", "nosniff").unwrap(),
                Header::from_bytes("Cache-Control", "private, no-store").unwrap(),
            ];
            req.respond(Response::new(StatusCode(200), headers, file, Some(size as usize), None))
        }
        Err(e) => req.respond(json_response(e.status, &e.json())),
    };
}

enum Reply {
    Json(u16, Value),
    File { file: std::fs::File, size: u64, sha256: String, content_type: String },
}

fn ok(v: Value) -> Result<Reply> {
    Ok(Reply::Json(200, v))
}

fn query(url: &str) -> Vec<(String, String)> {
    url.split_once('?')
        .map(|(_, q)| q)
        .unwrap_or("")
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (decode(k), decode(v))
        })
        .collect()
}

fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                match std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()).ok_or(()) {
                    Ok(v) => {
                        out.push(v);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn read_json(req: &mut Request) -> Result<Value> {
    let mut buf = Vec::new();
    req.as_reader().take(MAX_JSON + 1).read_to_end(&mut buf).map_err(|e| Error::new(400, "bad_body", e.to_string()))?;
    if buf.len() as u64 > MAX_JSON {
        return Err(Error::new(413, "too_large", "JSON body exceeds 1 MiB"));
    }
    let v: Value = serde_json::from_slice(&buf).map_err(|e| Error::new(400, "bad_json", e.to_string()))?;
    if !v.is_object() {
        return Err(Error::new(400, "bad_json", "body must be an object"));
    }
    Ok(v)
}

fn principal(c: &mut Client, req: &Request) -> Result<Principal> {
    let auth = header(req, "Authorization").ok_or_else(|| Error::new(401, "unauthenticated", "bearer token required"))?;
    let token = auth.strip_prefix("Bearer ").ok_or_else(|| Error::new(401, "unauthenticated", "bearer token required"))?;
    db::authenticate(c, token.trim())
}

fn route(ctx: &Ctx, c: &mut Client, req: &mut Request) -> Result<Reply> {
    let url = req.url().to_string();
    let path = url.split('?').next().unwrap_or("");
    let segs: Vec<&str> = path.trim_matches('/').split('/').collect();
    let q = query(&url);
    let param = |k: &str| q.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
    let limit = param("limit").and_then(|l| l.parse::<i64>().ok()).unwrap_or(50).clamp(1, 500);
    let method = req.method().clone();

    if method == Method::Get && segs == ["v1", "health"] {
        let db_ok = c.simple_query("SELECT 1").is_ok();
        return ok(
            json!({"service": "up", "version": env!("CARGO_PKG_VERSION"), "database": if db_ok { "ok" } else { "unknown: query failed" }}),
        );
    }
    let p = principal(c, req)?;
    let pid = p.project.clone();
    let providers = ctx.provider_names();
    match (&method, segs.as_slice()) {
        (Method::Post, ["v1", "uploads"]) => upload(ctx, c, &p, req),
        (Method::Get, ["v1", "objects"]) => ok(db::list_objects(c, &pid, param("after").as_deref(), limit)?),
        (Method::Get, ["v1", "objects", id]) => ok(db::get_object(c, &pid, id)?),
        (Method::Get, ["v1", "objects", id, "content"]) => {
            let o = db::get_object(c, &pid, id)?;
            let sha = o["sha256"].as_str().unwrap().to_string();
            let file = ctx.store.open_object(&sha).map_err(|e| Error::new(500, "store_io", format!("committed object unreadable: {e}")))?;
            Ok(Reply::File {
                file,
                size: o["size"].as_u64().unwrap(),
                sha256: sha,
                content_type: o["content_type"].as_str().unwrap().into(),
            })
        }
        (Method::Post, ["v1", "objects", id, "acceptance"]) => {
            let body = read_json(req)?;
            ok(db::set_acceptance(c, &p, id, body["acceptance"].as_str().unwrap_or(""))?)
        }
        (Method::Post, ["v1", "jobs"]) => {
            let body = read_json(req)?;
            let (job, created) = db::submit_job(c, &p, &body, &db::Limits { quota_bytes: ctx.cfg.quota_bytes }, &providers)?;
            Ok(Reply::Json(if created { 201 } else { 200 }, json!({"job": job, "created": created})))
        }
        (Method::Get, ["v1", "jobs"]) => ok(db::list_jobs(c, &pid, param("phase").as_deref(), param("after").as_deref(), limit)?),
        (Method::Get, ["v1", "jobs", id]) => ok(db::get_job(c, &pid, id)?),
        (Method::Get, ["v1", "jobs", id, "events"]) => {
            let events = db::job_events(c, &pid, id)?;
            ok(Value::Array(events.into_iter().map(|(seq, e, at)| json!({"seq": seq, "event": e, "at": at})).collect()))
        }
        (Method::Get, ["v1", "jobs", id, "result"]) => ok(db::job_result(c, &pid, id)?),
        (Method::Post, ["v1", "jobs", id, "cancel"]) => ok(db::cancel_job(c, &pid, id)?),
        (Method::Get, ["v1", "methods"]) => ok(db::list_methods(c, &pid)?),
        (Method::Post, ["v1", "methods"]) => {
            let body = read_json(req)?;
            Ok(Reply::Json(201, db::create_method_version(c, &p, &body)?))
        }
        (Method::Get, ["v1", "recommendations"]) => {
            let uc = param("use_case").ok_or_else(|| Error::new(422, "bad_request", "use_case required"))?;
            let constraints: Value = serde_json::from_str(&param("constraints").unwrap_or("{}".into()))
                .map_err(|_| Error::new(422, "bad_constraints", "constraints must be a JSON object"))?;
            if !constraints.is_object() {
                return Err(Error::new(422, "bad_constraints", "constraints must be an object"));
            }
            ok(db::recommendations(c, &pid, &uc, &constraints)?)
        }
        (Method::Post, ["v1", "recommendations"]) => {
            let body = read_json(req)?;
            Ok(Reply::Json(201, db::propose_recommendation(c, &p, &body)?))
        }
        (Method::Post, ["v1", "recommendations", id, "promote"]) => ok(db::promote_recommendation(c, &p, id)?),
        (Method::Get, ["v1", "comparisons"]) => ok(db::list_comparisons(c, &pid, param("use_case").as_deref())?),
        (Method::Post, ["v1", "comparisons"]) => {
            let body = read_json(req)?;
            Ok(Reply::Json(201, db::add_comparison(c, &p, &body)?))
        }
        (Method::Get, ["v1", "status"]) => {
            let mut status = db::status(c, &pid, ctx.cfg.quota_bytes, &providers)?;
            status["store"]["physical_inventory"] = match ctx.store.inventory() {
                Ok((objects, temp)) => json!({"state": "observed", "object_bytes": objects, "temp_bytes": temp,
                    "full": objects.saturating_add(temp) >= ctx.cfg.quota_bytes || temp >= ctx.cfg.temp_quota_bytes}),
                Err(_) => json!({"state": "unknown", "reason": "physical inventory unavailable"}),
            };
            status["admission"] = json!({"writers": 1, "temp_quota_bytes": ctx.cfg.temp_quota_bytes,
                "metadata_quota_bytes": ctx.cfg.metadata_quota_bytes, "max_execution_ms": ctx.cfg.max_execution_ms,
                "free_space_floor_bytes": ctx.cfg.free_space_floor_bytes});
            status["admission"]["metadata"] = db::metadata_status(c);
            status["store"]["free_space"] = match ctx.store.available() {
                Ok(bytes) => json!({"state": "observed", "available_bytes": bytes, "floor_bytes": ctx.cfg.free_space_floor_bytes,
                    "write_overhead_bytes": 64 << 10,
                    "admission": if bytes.saturating_sub(ctx.cfg.free_space_floor_bytes) > 64 << 10 { "available" } else { "exhausted" }}),
                Err(_) => json!({"state": "unknown", "reason": "filesystem free-space observation unavailable"}),
            };
            ok(status)
        }
        _ => Err(Error::new(404, "no_route", format!("{method} {path}"))),
    }
}

fn upload(ctx: &Ctx, c: &mut Client, p: &Principal, req: &mut Request) -> Result<Reply> {
    let ct = header(req, "Content-Type").unwrap_or_default();
    let ct = ct.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    if !ctx.cfg.content_types.contains(&ct) {
        return Err(Error::new(415, "unsupported_type", format!("content type {ct:?} is not allowed")));
    }
    if let Some(len) = req.body_length() {
        if len as u64 > ctx.cfg.max_upload_bytes {
            return Err(Error::new(413, "too_large", format!("limit {} bytes", ctx.cfg.max_upload_bytes)));
        }
    }
    // A caller-supplied name is metadata only; storage keys stay server-owned.
    let name = header(req, "X-Asset-Name").map(|n| n.chars().filter(|c| !c.is_control()).take(200).collect::<String>());
    let _writer = ctx.store.writer_lock()?;
    let room = ctx.room()?;
    let stored = ctx.store.put_locked(req.as_reader(), ctx.cfg.max_upload_bytes, room)?;
    let mut tx = db::write_tx(c, false)?;
    let id = db::insert_object(
        &mut tx,
        &NewObject {
            project: &p.project,
            sha256: &stored.sha256,
            size: stored.size,
            content_type: &ct,
            kind: "upload",
            job_id: None,
            import_key: None,
            provenance: json!({"source": "upload", "name": name}),
            synthetic: false,
            retention_days: Some(ctx.cfg.retention_days),
        },
    )?;
    db::commit_write(tx)?;
    Ok(Reply::Json(201, db::get_object(c, &p.project, &id)?))
}
