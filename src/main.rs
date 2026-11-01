//! `assetq`: service, executor, administration and API client in one binary.

use assetq::{Config, db, exec, store, traces};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

const USAGE: &str = "assetq <command>

Service (configuration from ASSETQ_* environment, see docs/deploy.md):
  serve [--listen 127.0.0.1:8740] [--http-threads 8]   API plus embedded executors
  work                                                 executors only (separate host)
  migrate                                              apply schema and built-in methods
  project <id> [--budget N] [--max-active N]           create or update a project
  token <project> [--role member|owner]                create a grant; prints it once
  restore-check                                        verify isolated restored metadata, manifests and payloads
  audit <job-id>                                       replay a stored job's events
  selftest [--database]                                offline checks; --database adds DB checks

Client (ASSETQ_URL, default http://127.0.0.1:8740; ASSETQ_TOKEN):
  submit --method M --operation O --key K --budget N [--text T] [--attach ID].. [--set k=JSON]..
  jobs [--phase P] [--after ID] | status <job> | events <job> | result <job> | cancel <job>
  upload <file> --type CONTENT_TYPE
  objects [--after ID] | object <id> | download <id> <out-file> | accept <id> accepted|rejected|pending
  methods | method-add <spec.json>
  recommend <use-case> [--constraints JSON] | recommend-propose <body.json> | recommend-promote <id>
  comparisons [--use-case U] | comparison-add <body.json>
  ops                                                  operational status";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn opt(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn multi(args: &[String], name: &str) -> Vec<String> {
    args.windows(2).filter(|w| w[0] == name).map(|w| w[1].clone()).collect()
}

fn pos(args: &[String], i: usize) -> Result<String, String> {
    args.get(i).filter(|a| !a.starts_with("--")).cloned().ok_or_else(|| USAGE.to_string())
}

fn print(v: &Value) {
    println!("{}", serde_json::to_string_pretty(v).unwrap());
}

fn run(args: &[String]) -> Result<(), String> {
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    let rest = &args[args.len().min(1)..];
    match cmd {
        "serve" => serve(rest),
        "work" => {
            let ctx = Arc::new(exec::Ctx::new(Config::from_env()?).map_err(|e| e.to_string())?);
            check_schema(&ctx.cfg)?;
            let handles = spawn_executors(&ctx);
            eprintln!("assetq executors running: {}", handles.len());
            for h in handles {
                let _ = h.join();
            }
            Ok(())
        }
        "migrate" => {
            let cfg = Config::from_env()?;
            db::migrate(&mut db::connect(&cfg.database_url).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            println!("schema version 1 applied; built-in methods registered");
            Ok(())
        }
        "project" => {
            let cfg = Config::from_env()?;
            let id = pos(rest, 0)?;
            let budget: i64 = opt(rest, "--budget").unwrap_or("0".into()).parse().map_err(|_| "--budget must be an integer")?;
            let max: i32 = opt(rest, "--max-active").unwrap_or("4".into()).parse().map_err(|_| "--max-active must be an integer")?;
            db::create_project(&mut db::connect(&cfg.database_url).map_err(|e| e.to_string())?, &id, budget, max)
                .map_err(|e| e.to_string())?;
            print(&json!({"project": id, "budget_credits": budget, "max_active_jobs": max}));
            Ok(())
        }
        "token" => {
            let cfg = Config::from_env()?;
            let project = pos(rest, 0)?;
            let role = opt(rest, "--role").unwrap_or("member".into());
            let t = db::create_token(&mut db::connect(&cfg.database_url).map_err(|e| e.to_string())?, &project, &role)
                .map_err(|e| e.to_string())?;
            println!("{t}");
            Ok(())
        }
        "restore-check" => {
            let cfg = Config::from_env()?;
            let store = store::Store::existing(&cfg.data_root).map_err(|e| e.to_string())?;
            let mut c = db::connect(&cfg.database_url).map_err(|e| e.to_string())?;
            print(&db::restore_check(&mut c, &store).map_err(|e| e.to_string())?);
            Ok(())
        }
        "audit" => {
            let cfg = Config::from_env()?;
            let job = db::audit_job(&mut db::connect(&cfg.database_url).map_err(|e| e.to_string())?, &pos(rest, 0)?)
                .map_err(|e| e.to_string())?;
            print(&json!({"replay": "matches stored state", "job": job}));
            Ok(())
        }
        "selftest" => selftest(rest.iter().any(|a| a == "--database")),
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            Ok(())
        }
        _ => client(cmd, rest),
    }
}

fn check_schema(cfg: &Config) -> Result<(), String> {
    let mut c = db::connect(&cfg.database_url).map_err(|e| e.to_string())?;
    let ok = c
        .query_opt("SELECT 1 FROM schema_version HAVING count(*) = 1 AND min(version) = 1", &[])
        .map_err(|_| "schema missing: run `assetq migrate`".to_string())?;
    ok.map(|_| ()).ok_or_else(|| "schema missing: run `assetq migrate`".into())
}

fn spawn_executors(ctx: &Arc<exec::Ctx>) -> Vec<std::thread::JoinHandle<()>> {
    let host = std::env::var("HOSTNAME").unwrap_or_else(|_| "host".into());
    let pid = std::process::id();
    (0..ctx.cfg.executors)
        .map(|i| {
            let ctx = ctx.clone();
            let owner = format!("{host}:{pid}:{i}");
            std::thread::spawn(move || exec::run_loop(ctx, owner, Arc::new(AtomicBool::new(false))))
        })
        .collect()
}

fn serve(rest: &[String]) -> Result<(), String> {
    let cfg = Config::from_env()?;
    check_schema(&cfg)?;
    let listen = opt(rest, "--listen").unwrap_or("127.0.0.1:8740".into());
    let threads: usize = opt(rest, "--http-threads").unwrap_or("8".into()).parse().map_err(|_| "--http-threads must be an integer")?;
    let ctx = Arc::new(exec::Ctx::new(cfg).map_err(|e| e.to_string())?);
    let server = Arc::new(assetq::api::bind(&listen).map_err(|e| format!("bind {listen}: {e}"))?);
    let addr = server.server_addr().to_ip().map(|a| a.to_string()).unwrap_or(listen);
    let mut handles = assetq::api::serve(ctx.clone(), server, threads);
    handles.extend(spawn_executors(&ctx));
    // Tests and supervisors read this line to learn the bound port.
    println!("assetq listening on {addr}");
    std::io::stdout().flush().ok();
    for h in handles {
        let _ = h.join();
    }
    Ok(())
}

fn selftest(database: bool) -> Result<(), String> {
    let mut checks = vec![];
    for (name, raw) in traces::TRACES {
        traces::run(raw).map_err(|e| format!("trace {name}: {e}"))?;
    }
    checks.push(json!({"check": "lifecycle traces", "count": traces::TRACES.len(), "outcome": "pass"}));
    let dir = std::env::temp_dir().join(format!("assetq-selftest-{}", uuid::Uuid::new_v4()));
    let s = store::Store::open(&dir).map_err(|e| e.to_string())?;
    let put = s.put(&mut &b"assetq selftest"[..], 1024, 1024).map_err(|e| e.to_string())?;
    s.verify(&put.sha256, put.size).map_err(|e| e.to_string())?;
    let full = matches!(s.put(&mut &[0u8; 64][..], 1024, 10), Err(store::StoreError::Full(_)));
    std::fs::remove_dir_all(&dir).ok();
    if !full {
        return Err("store quota did not refuse an oversized write".into());
    }
    checks.push(json!({"check": "object store hash, verify and quota", "outcome": "pass"}));
    if database {
        let cfg = Config::from_env()?;
        check_schema(&cfg)?;
        let mut c = db::connect(&cfg.database_url).map_err(|e| e.to_string())?;
        let methods: i64 = c.query_one("SELECT count(*) FROM methods", &[]).map_err(|e| e.to_string())?.get(0);
        checks.push(json!({"check": "database schema and method registry", "methods": methods, "outcome": "pass"}));
        let root = assetq::store::Store::open(&cfg.data_root).map_err(|e| format!("data root: {e}"))?;
        drop(root);
        checks.push(json!({"check": "data root directories available", "outcome": "pass"}));
    } else {
        checks.push(json!({"check": "database", "outcome": "not run (pass --database)"}));
    }
    print(&json!({"version": env!("CARGO_PKG_VERSION"), "checks": checks}));
    Ok(())
}

// ---------------------------------------------------------------- client

struct Api {
    agent: ureq::Agent,
    base: String,
    token: String,
}

impl Api {
    fn new() -> Result<Api, String> {
        let token = std::env::var("ASSETQ_TOKEN").map_err(|_| "ASSETQ_TOKEN is required")?;
        let base = std::env::var("ASSETQ_URL").unwrap_or("http://127.0.0.1:8740".into()).trim_end_matches('/').to_string();
        let agent = ureq::Agent::config_builder().http_status_as_error(false).build().into();
        Ok(Api { agent, base, token })
    }
    fn auth(&self) -> String {
        format!("Bearer {}", self.token)
    }
    fn finish(mut r: ureq::http::Response<ureq::Body>) -> Result<Value, String> {
        let status = r.status().as_u16();
        let text = r.body_mut().read_to_string().map_err(|e| format!("HTTP {status}: unreadable body: {e}"))?;
        let v: Value = serde_json::from_str(&text).map_err(|e| format!("HTTP {status}: {e}"))?;
        if status >= 400 {
            return Err(format!("HTTP {status}: {}", serde_json::to_string(&v).unwrap()));
        }
        Ok(v)
    }
    fn get(&self, path: &str) -> Result<Value, String> {
        Self::finish(self.agent.get(format!("{}{path}", self.base)).header("Authorization", self.auth()).call().map_err(|e| e.to_string())?)
    }
    fn post(&self, path: &str, body: &Value) -> Result<Value, String> {
        Self::finish(
            self.agent
                .post(format!("{}{path}", self.base))
                .header("Authorization", self.auth())
                .header("Content-Type", "application/json")
                .send(body.to_string())
                .map_err(|e| e.to_string())?,
        )
    }
}

fn read_json_file(path: &str) -> Result<Value, String> {
    serde_json::from_slice(&std::fs::read(path).map_err(|e| format!("{path}: {e}"))?).map_err(|e| format!("{path}: {e}"))
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

fn client(cmd: &str, rest: &[String]) -> Result<(), String> {
    let api = Api::new()?;
    let page = |base: &str| {
        let mut q = vec![];
        for k in ["phase", "after", "use-case"] {
            if let Some(v) = opt(rest, &format!("--{k}")) {
                q.push(format!("{}={}", k.replace('-', "_"), enc(&v)));
            }
        }
        if q.is_empty() { base.to_string() } else { format!("{base}?{}", q.join("&")) }
    };
    let v = match cmd {
        "submit" => {
            let mut constraints = serde_json::Map::new();
            for kv in multi(rest, "--set") {
                let (k, v) = kv.split_once('=').ok_or("--set takes key=JSON")?;
                constraints.insert(k.into(), serde_json::from_str(v).map_err(|e| format!("--set {k}: {e}"))?);
            }
            let budget: u64 = opt(rest, "--budget").ok_or("--budget required")?.parse().map_err(|_| "--budget must be an integer")?;
            api.post(
                "/v1/jobs",
                &json!({
                    "method": opt(rest, "--method").ok_or("--method required")?,
                    "operation": opt(rest, "--operation").ok_or("--operation required")?,
                    "idempotency_key": opt(rest, "--key").ok_or("--key required")?,
                    "text": opt(rest, "--text").unwrap_or_default(),
                    "attachments": multi(rest, "--attach"),
                    "constraints": constraints,
                    "budget": {"max_credits": budget},
                }),
            )?
        }
        "jobs" => api.get(&page("/v1/jobs"))?,
        "status" => api.get(&format!("/v1/jobs/{}", enc(&pos(rest, 0)?)))?,
        "events" => api.get(&format!("/v1/jobs/{}/events", enc(&pos(rest, 0)?)))?,
        "result" => api.get(&format!("/v1/jobs/{}/result", enc(&pos(rest, 0)?)))?,
        "cancel" => api.post(&format!("/v1/jobs/{}/cancel", enc(&pos(rest, 0)?)), &json!({}))?,
        "objects" => api.get(&page("/v1/objects"))?,
        "object" => api.get(&format!("/v1/objects/{}", enc(&pos(rest, 0)?)))?,
        "accept" => api.post(&format!("/v1/objects/{}/acceptance", enc(&pos(rest, 0)?)), &json!({"acceptance": pos(rest, 1)?}))?,
        "upload" => {
            let path = pos(rest, 0)?;
            let ct = opt(rest, "--type").ok_or("--type required")?;
            let file = std::fs::File::open(&path).map_err(|e| format!("{path}: {e}"))?;
            let len = file.metadata().map_err(|e| e.to_string())?.len();
            let name = std::path::Path::new(&path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let mut reader = file;
            let r = api
                .agent
                .post(format!("{}/v1/uploads", api.base))
                .header("Authorization", api.auth())
                .header("Content-Type", &ct)
                .header("Content-Length", len.to_string())
                .header("X-Asset-Name", enc(&name))
                .send(ureq::SendBody::from_reader(&mut reader))
                .map_err(|e| e.to_string())?;
            Api::finish(r)?
        }
        "download" => download(&api, &pos(rest, 0)?, &pos(rest, 1)?)?,
        "methods" => api.get("/v1/methods")?,
        "method-add" => api.post("/v1/methods", &read_json_file(&pos(rest, 0)?)?)?,
        "recommend" => api.get(&format!(
            "/v1/recommendations?use_case={}&constraints={}",
            enc(&pos(rest, 0)?),
            enc(&opt(rest, "--constraints").unwrap_or("{}".into()))
        ))?,
        "recommend-propose" => api.post("/v1/recommendations", &read_json_file(&pos(rest, 0)?)?)?,
        "recommend-promote" => api.post(&format!("/v1/recommendations/{}/promote", enc(&pos(rest, 0)?)), &json!({}))?,
        "comparisons" => api.get(&page("/v1/comparisons"))?,
        "comparison-add" => api.post("/v1/comparisons", &read_json_file(&pos(rest, 0)?)?)?,
        "ops" => api.get("/v1/status")?,
        _ => return Err(USAGE.into()),
    };
    print(&v);
    Ok(())
}

/// Stream to a temporary file, hash while writing, and keep the file only if
/// the hash matches both the response header and the object record.
fn download(api: &Api, id: &str, out: &str) -> Result<Value, String> {
    let meta = api.get(&format!("/v1/objects/{}", enc(id)))?;
    let r = api
        .agent
        .get(format!("{}/v1/objects/{}/content", api.base, enc(id)))
        .header("Authorization", api.auth())
        .call()
        .map_err(|e| e.to_string())?;
    if r.status().as_u16() != 200 {
        return Api::finish(r);
    }
    let header_sha = r.headers().get("X-Content-Sha256").and_then(|h| h.to_str().ok()).unwrap_or("").to_string();
    let tmp = format!("{out}.part-{}", uuid::Uuid::new_v4());
    let mut file = std::fs::File::create(&tmp).map_err(|e| format!("{tmp}: {e}"))?;
    let mut reader = r.into_body().into_reader();
    let mut hash = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut size = 0u64;
    let res: Result<(), String> = (|| {
        loop {
            let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            hash.update(&buf[..n]);
            file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            size += n as u64;
        }
        file.sync_all().map_err(|e| e.to_string())
    })();
    let got = hex::encode(hash.finalize());
    let expected = meta["sha256"].as_str().unwrap_or("");
    if let Err(e) = res.and_then(|_| {
        if got == expected && got == header_sha && Some(size) == meta["size"].as_u64() {
            Ok(())
        } else {
            Err(format!("hash mismatch: got {got}, record {expected}, header {header_sha}"))
        }
    }) {
        std::fs::remove_file(&tmp).ok();
        return Err(e);
    }
    std::fs::rename(&tmp, out).map_err(|e| e.to_string())?;
    Ok(json!({"object": id, "path": out, "sha256": got, "size": size, "verified": true}))
}
