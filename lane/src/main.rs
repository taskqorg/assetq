//! pipe-read: the Pipe read lane (pipe.md § Read lane, docs/read-lane.md).
use bytes::Bytes;
use coreq::enroll::{self, Backoff, Delivery};
use coreq::telemetry::{Event, OBSERVATION_SCHEMA, Reader, Store};
use http_body_util::{BodyExt, Full, Limited};
use hyper::body::Incoming;
use hyper::header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue, RETRY_AFTER};
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_rustls::HttpsConnector;
use hyper_util::client::legacy::{Client, connect::HttpConnector};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::graceful::GracefulShutdown;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering::SeqCst};
use std::sync::mpsc::{SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::Semaphore;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const REVISION: &str = "1";
const REQUEST_BYTES: usize = 1 << 20;
const REFUSED_BYTES: usize = 64 << 10;
const REFUSED_PATIENCE: Duration = Duration::from_secs(2);
const PROVIDER_BYTES: usize = 16 << 20;
const RECORD_QUEUE: usize = 4096;
const SYSTEMD_FIRST_FD: i32 = 3;

type Body = Full<Bytes>;
type Res<T> = Result<T, String>;

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

fn sha256(text: &str) -> String {
    Sha256::digest(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// CoreQ code alphabet: telemetry holds metadata only, never free text.
fn is_code(text: &str) -> bool {
    !text.is_empty() && text.len() <= 64 && text.bytes().all(|c| c.is_ascii_alphanumeric() || b"._:-".contains(&c))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, path)
}

// ---------------------------------------------------------------- configuration

struct Config {
    listen: Option<String>,
    host: String,
    snapshot: PathBuf,
    state_dir: PathBuf,
    provider_url: String,
    concurrency: usize,
    queue: usize,
    request_timeout: Duration,
    drain_timeout: Duration,
    alert_command: Vec<String>,
    coreq_connection: Option<PathBuf>,
}

impl Config {
    fn load(path: &str) -> Res<Self> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("config {path}: {e}"))?;
        let v: Value = serde_json::from_str(&text).map_err(|e| format!("config {path}: {e}"))?;
        let text_of = |name: &str| v[name].as_str().map(str::to_owned).ok_or(format!("config {name} is required"));
        let number = |name: &str, default: u64| v[name].as_u64().unwrap_or(default);
        let config = Self {
            listen: v["listen"].as_str().map(str::to_owned),
            host: text_of("host")?,
            snapshot: text_of("snapshot")?.into(),
            state_dir: text_of("state_dir")?.into(),
            provider_url: v["provider_url"].as_str().unwrap_or("https://api.tavily.com").trim_end_matches('/').to_owned(),
            concurrency: number("concurrency", 16) as usize,
            queue: number("queue", 64) as usize,
            request_timeout: Duration::from_millis(number("request_timeout_ms", 30_000)),
            drain_timeout: Duration::from_millis(number("drain_timeout_ms", 35_000)),
            alert_command: v["alert_command"]
                .as_array()
                .map_or(Vec::new(), |a| a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect()),
            coreq_connection: v["coreq_connection"].as_str().map(PathBuf::from),
        };
        if config.concurrency == 0 {
            return Err("config concurrency must be positive".into());
        }
        Ok(config)
    }

    fn store_path(&self) -> PathBuf {
        self.state_dir.join("telemetry.db")
    }
}

// ---------------------------------------------------------------- last-known-good snapshot

struct Snapshot {
    revision: String,
    generated_ms: u64,
    grants: HashMap<String, String>,
}

impl Snapshot {
    fn parse(bytes: &[u8]) -> Res<Self> {
        let v: Value = serde_json::from_slice(bytes).map_err(|e| format!("not JSON: {e}"))?;
        if v["schema"] != 1 {
            return Err("unsupported schema".into());
        }
        let generated_ms = v["generated_ms"].as_u64().ok_or("generated_ms is missing")?;
        let mut grants = HashMap::new();
        for grant in v["grants"].as_array().ok_or("grants is missing")? {
            let (Some(hash), Some(project)) = (grant["token_sha256"].as_str(), grant["project"].as_str()) else {
                return Err("grant is incomplete".into());
            };
            if hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
                return Err("grant hash is not SHA-256".into());
            }
            if project.len() > 128 || !project.contains('/') || !project.split('/').all(|p| is_code(p) && p != "." && p != "..") {
                return Err("grant project is not owner/repository".into());
            }
            grants.insert(hash.to_owned(), project.to_owned());
        }
        let revision = Sha256::digest(bytes).iter().take(6).map(|b| format!("{b:02x}")).collect();
        Ok(Self { revision, generated_ms, grants })
    }
}

/// The control file when valid, else this lane's own retained copy of the last valid one.
fn load_snapshot(config: &Config) -> Res<Snapshot> {
    let good = config.state_dir.join("snapshot.good.json");
    let control = std::fs::read(&config.snapshot).map_err(|e| e.to_string()).and_then(|b| Snapshot::parse(&b).map(|s| (b, s)));
    match control {
        Ok((bytes, snapshot)) => {
            write_atomic(&good, &bytes).map_err(|e| format!("retain snapshot: {e}"))?;
            Ok(snapshot)
        }
        Err(control_error) => {
            let bytes = std::fs::read(&good).map_err(|e| format!("no usable snapshot: control: {control_error}; retained: {e}"))?;
            eprintln!("pipe-read: control snapshot refused ({control_error}); serving the retained copy");
            Snapshot::parse(&bytes).map_err(|e| format!("no usable snapshot: control: {control_error}; retained: {e}"))
        }
    }
}

// ---------------------------------------------------------------- telemetry

/// Records leave the request path through a bounded channel. Whatever cannot be queued or stored is counted.
struct Telemetry {
    sender: Mutex<Option<SyncSender<Value>>>,
    lost: Arc<AtomicU64>,
    shipper: Arc<Mutex<String>>,
}

impl Telemetry {
    fn start(config: &Config) -> (Self, std::thread::JoinHandle<()>) {
        let lost_path = config.state_dir.join("telemetry-lost");
        let prior = std::fs::read_to_string(&lost_path).ok().and_then(|t| t.trim().parse().ok()).unwrap_or(0);
        let lost = Arc::new(AtomicU64::new(prior));
        let shipper = Arc::new(Mutex::new(if config.coreq_connection.is_some() { "starting" } else { "off" }.to_owned()));
        let (sender, receiver) = sync_channel::<Value>(RECORD_QUEUE);
        let path = config.store_path();
        let connection = config.coreq_connection.clone();
        let (lost_writer, shipper_state) = (lost.clone(), shipper.clone());
        let writer = std::thread::spawn(move || {
            // ponytail: unbounded-API store with CoreQ's 28 MiB cap; Store::open_bounded when an installation budget lock exists here.
            let mut store: Option<Arc<Store>> = None;
            let mut shipping = false;
            let prepare = |store: &mut Option<Arc<Store>>, shipping: &mut bool| {
                if store.is_none() {
                    *store = Store::open(&path).map(Arc::new).map_err(|e| eprintln!("pipe-read: telemetry store: {e}")).ok();
                }
                if let (Some(open), Some(connection), false) = (&*store, &connection, *shipping) {
                    *shipping = true;
                    start_shipper(open.clone(), connection.clone(), shipper_state.clone());
                }
            };
            // At start, not at the first call: records kept from the last run must ship from an idle lane too.
            prepare(&mut store, &mut shipping);
            while let Ok(first) = receiver.recv() {
                let mut batch = vec![first];
                batch.extend(receiver.try_iter().take(256));
                prepare(&mut store, &mut shipping);
                let events: Vec<Event> = batch
                    .iter()
                    .filter_map(|v| Event::observation(v.clone()).map_err(|e| eprintln!("pipe-read: record: {e}")).ok())
                    .collect();
                let mut dropped = (batch.len() - events.len()) as u64;
                match &store {
                    Some(open) => {
                        if let Err(error) = open.append(&events) {
                            eprintln!("pipe-read: telemetry append: {error}");
                            dropped += events.len() as u64;
                        }
                    }
                    None => dropped += events.len() as u64,
                }
                if dropped > 0 {
                    lost_writer.fetch_add(dropped, SeqCst);
                }
                let _ = write_atomic(&lost_path, lost_writer.load(SeqCst).to_string().as_bytes());
            }
        });
        (Self { sender: Mutex::new(Some(sender)), lost, shipper }, writer)
    }

    fn record(&self, record: Value) {
        let guard = self.sender.lock().unwrap();
        let Some(sender) = guard.as_ref() else {
            self.lost.fetch_add(1, SeqCst);
            return;
        };
        if let Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) = sender.try_send(record) {
            self.lost.fetch_add(1, SeqCst);
        }
    }

    fn close(&self) {
        self.sender.lock().unwrap().take();
    }
}

fn start_shipper(store: Arc<Store>, connection: PathBuf, state: Arc<Mutex<String>>) {
    std::thread::spawn(move || {
        let mut backoff = Backoff::default();
        let mut wire = enroll::Https { browser: false };
        loop {
            // Nothing enrolled is not a healthy delivery; Store::ship would report an empty pass as success.
            if matches!(enroll::active(&connection), Ok(None)) {
                *state.lock().unwrap() = "off".into();
                std::thread::sleep(Duration::from_secs(30));
                continue;
            }
            let wait = match store.ship(&connection, &mut backoff, &mut wire) {
                Ok(shipped) => {
                    *state.lock().unwrap() = "ok".into();
                    if shipped.more { 1 } else { 30 }
                }
                Err(Delivery::Retry(seconds)) => {
                    *state.lock().unwrap() = "retrying".into();
                    seconds
                }
                Err(Delivery::Stopped(reason)) => {
                    *state.lock().unwrap() = format!("stopped: {reason}");
                    60
                }
            };
            std::thread::sleep(Duration::from_secs(wait));
        }
    });
}

// ---------------------------------------------------------------- provider

struct Key {
    value: String,
    id: String,
    disabled: AtomicBool,
}

/// One warm client and one bounded queue per provider. Tavily is the only provider so far.
struct Provider {
    client: Client<HttpsConnector<HttpConnector>, Body>,
    url: String,
    keys: Vec<Key>,
    current: AtomicUsize,
    admitted: Arc<Semaphore>,
    running: Arc<Semaphore>,
    capacity: usize,
    timeout: Duration,
}

/// The outcome of one provider call, ready for the caller and for the record.
struct Answer {
    status: StatusCode,
    body: Bytes,
    retry_after: Option<HeaderValue>,
    outcome: &'static str,
    error: String,
    key: Option<String>,
    units: Option<u64>,
    /// A key disabled by this call: (key id, HTTP status).
    disabled: Option<(String, u16)>,
}

fn refusal(status: StatusCode, outcome: &'static str, error: &str, reason: &str) -> Answer {
    Answer {
        status,
        body: json!({"error": error, "reason": reason}).to_string().into(),
        retry_after: None,
        outcome,
        error: error.to_owned(),
        key: None,
        units: None,
        disabled: None,
    }
}

impl Provider {
    fn new(config: &Config, disabled: &BTreeSet<String>) -> Res<Self> {
        let credentials =
            std::env::var("CREDENTIALS_DIRECTORY").map_err(|_| "CREDENTIALS_DIRECTORY is unset: keys arrive only by LoadCredential")?;
        let text =
            std::fs::read_to_string(Path::new(&credentials).join("tavily-keys")).map_err(|e| format!("tavily-keys credential: {e}"))?;
        let keys: Vec<Key> = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|value| {
                let id = sha256(value)[..12].to_owned();
                Key { value: value.to_owned(), disabled: AtomicBool::new(disabled.contains(&id)), id }
            })
            .collect();
        if keys.is_empty() {
            return Err("tavily-keys credential holds no key".into());
        }
        let connector = hyper_rustls::HttpsConnectorBuilder::new().with_webpki_roots().https_or_http().enable_http1().build();
        let client = Client::builder(TokioExecutor::new()).pool_idle_timeout(Duration::from_secs(90)).build(connector);
        Ok(Self {
            client,
            url: config.provider_url.clone(),
            keys,
            current: AtomicUsize::new(0),
            admitted: Arc::new(Semaphore::new(config.concurrency + config.queue)),
            running: Arc::new(Semaphore::new(config.concurrency)),
            capacity: config.concurrency + config.queue,
            timeout: config.request_timeout,
        })
    }

    fn next_enabled(&self, from: usize) -> Option<usize> {
        (0..self.keys.len()).map(|step| (from + step) % self.keys.len()).find(|&index| !self.keys[index].disabled.load(SeqCst))
    }

    async fn call(&self, path: &str, body: Bytes) -> Answer {
        let Ok(_admitted) = self.admitted.clone().try_acquire_owned() else {
            return refusal(StatusCode::SERVICE_UNAVAILABLE, "refused", "queue-full", "the provider queue is full; retry later");
        };
        let Ok(_running) = self.running.acquire().await else {
            return refusal(StatusCode::SERVICE_UNAVAILABLE, "failed", "queue-closed", "the provider queue is closed");
        };
        let mut rotated = false;
        let mut last_failure = String::new();
        for _ in 0..self.keys.len() {
            let current = self.current.load(SeqCst);
            let Some(index) = self.next_enabled(current) else { break };
            let key = &self.keys[index];
            let request = Request::post(format!("{}{path}", self.url))
                .header(AUTHORIZATION, format!("Bearer {}", key.value))
                .header(CONTENT_TYPE, "application/json")
                .body(Full::new(body.clone()))
                .expect("static request shape");
            let sent = tokio::time::timeout(self.timeout, self.client.request(request)).await;
            let response = match sent {
                Err(_) => {
                    let mut answer =
                        refusal(StatusCode::GATEWAY_TIMEOUT, "failed", "provider-timeout", "the provider did not answer in time");
                    answer.key = Some(key.id.clone());
                    return answer;
                }
                // The only reason to rotate: no HTTP answer arrived on this key.
                Ok(Err(error)) => {
                    last_failure = if error.is_connect() { "connect".into() } else { "transport".into() };
                    let _ = self.current.compare_exchange(current, (index + 1) % self.keys.len(), SeqCst, SeqCst);
                    rotated = true;
                    continue;
                }
                Ok(Ok(response)) => response,
            };
            let status = response.status();
            let retry_after = response.headers().get(RETRY_AFTER).cloned();
            let read = tokio::time::timeout(self.timeout, Limited::new(response.into_body(), PROVIDER_BYTES).collect()).await;
            let Ok(Ok(collected)) = read else {
                let mut answer =
                    refusal(StatusCode::BAD_GATEWAY, "failed", "provider-body", "the provider reply was cut off, too large or too slow");
                answer.key = Some(key.id.clone());
                return answer;
            };
            let bytes = collected.to_bytes();
            if matches!(status.as_u16(), 401 | 402 | 403) {
                key.disabled.store(true, SeqCst);
                let reason = format!("the provider refused key {} with HTTP {}; the key is disabled", key.id, status.as_u16());
                let mut answer = refusal(StatusCode::BAD_GATEWAY, "failed", &format!("key-disabled-{}", status.as_u16()), &reason);
                answer.key = Some(key.id.clone());
                answer.disabled = Some((key.id.clone(), status.as_u16()));
                return answer;
            }
            // Success and every other provider reply, quota and 429 included, reach the caller unchanged.
            let units = serde_json::from_slice::<Value>(&bytes).ok().and_then(|v| v["usage"]["credits"].as_u64());
            let (outcome, error) = match (status.is_success(), rotated) {
                (true, true) => ("retried", "none".to_owned()),
                (true, false) => ("ok", "none".to_owned()),
                (false, _) => ("failed", format!("provider-{}", status.as_u16())),
            };
            return Answer { status, body: bytes, retry_after, outcome, error, key: Some(key.id.clone()), units, disabled: None };
        }
        if rotated {
            let reason = format!("no key reached the provider; last failure: {last_failure}");
            refusal(StatusCode::BAD_GATEWAY, "failed", "provider-unreachable", &reason)
        } else {
            refusal(StatusCode::SERVICE_UNAVAILABLE, "failed", "no-usable-key", "every provider key is disabled; an operator must act")
        }
    }
}

// ---------------------------------------------------------------- service

struct Lane {
    config: Config,
    provider: Provider,
    snapshot: RwLock<Arc<Snapshot>>,
    rejected_snapshots: AtomicU64,
    telemetry: Telemetry,
    draining: AtomicBool,
    disabled_write: Mutex<()>,
    started: Instant,
}

struct Caller {
    project: Option<String>,
    label: Option<String>,
    runtime: Option<String>,
}

fn reply(status: StatusCode, body: impl Into<Bytes>) -> Response<Body> {
    Response::builder().status(status).header(CONTENT_TYPE, "application/json").body(Full::new(body.into())).expect("static response shape")
}

impl Lane {
    fn record(&self, operation: &str, step: &str, caller: &Caller, started_ms: u64, took: Duration, answer: &Answer) {
        let mut unknown = serde_json::Map::new();
        unknown.insert("trace".into(), json!("no-caller-trace"));
        unknown.insert("span".into(), json!("no-caller-trace"));
        let task = caller.label.as_deref().and_then(|label| label.parse::<u64>().ok());
        if task.is_none() {
            unknown.insert("task".into(), json!(if caller.label.is_some() { "non-numeric-label" } else { "no-label" }));
        }
        if caller.project.is_none() {
            unknown.insert("project".into(), json!("unauthenticated"));
        }
        if answer.units.is_none() {
            unknown.insert("units".into(), json!(if answer.key.is_some() { "provider-unreported" } else { "no-provider-call" }));
        }
        if caller.runtime.is_none() {
            unknown.insert("runtime".into(), json!("caller-unreported"));
        }
        self.telemetry.record(json!({
            "product": "pipe-read", "version": VERSION, "kind": "operation", "operation": operation,
            "trace": null, "span": null, "started_ms": started_ms, "duration_us": took.as_micros() as u64,
            "outcome": answer.outcome, "error": answer.error, "project": caller.project, "task": task,
            "pipeline": operation, "pipeline_revision": REVISION, "step": step, "route": "tavily",
            "account": answer.key, "source": caller.label, "runtime": caller.runtime, "runtime_version": null,
            "units": answer.units, "unit_kind": answer.units.map(|_| "tavily_credits"), "host": self.config.host, "unknown": unknown,
        }));
    }

    /// Persist, record and alert on one key's transition to disabled.
    async fn on_disabled(self: &Arc<Self>, id: &str, status: u16) {
        {
            let _one = self.disabled_write.lock().unwrap();
            let path = self.config.state_dir.join("disabled-keys");
            let mut known: BTreeSet<String> = std::fs::read_to_string(&path).unwrap_or_default().lines().map(str::to_owned).collect();
            known.insert(id.to_owned());
            let text = known.into_iter().collect::<Vec<_>>().join("\n") + "\n";
            if let Err(error) = write_atomic(&path, text.as_bytes()) {
                eprintln!("pipe-read: disabled key {id} not persisted: {error}");
            }
        }
        let message = format!("pipe-read on {}: Tavily key {id} disabled after HTTP {status}", self.config.host);
        eprintln!("{message}");
        self.telemetry.record(json!({
            "product": "pipe-read", "version": VERSION, "kind": "check", "operation": "read.key_state",
            "trace": null, "span": null, "started_ms": now_ms(), "outcome": "failed", "error": format!("key-disabled-{status}"),
            "route": "tavily", "account": id, "host": self.config.host,
            "unknown": {"trace": "no-caller-trace", "span": "no-caller-trace"},
        }));
        if let Some((program, arguments)) = self.config.alert_command.split_first() {
            let mut command = tokio::process::Command::new(program);
            command.args(arguments).arg(&message).kill_on_drop(true);
            // The caller already has its answer to wait for; a stuck alert command must not hold it.
            tokio::spawn(async move {
                match tokio::time::timeout(Duration::from_secs(10), command.status()).await {
                    Ok(Ok(status)) if status.success() => {}
                    other => eprintln!("pipe-read: alert command failed: {other:?}"),
                }
            });
        }
    }

    fn health(&self) -> Value {
        let snapshot = self.snapshot.read().unwrap().clone();
        let total = self.provider.keys.len();
        let disabled = self.provider.keys.iter().filter(|k| k.disabled.load(SeqCst)).count();
        let draining = self.draining.load(SeqCst);
        let status = if draining {
            "draining"
        } else if disabled == total {
            "no-usable-key"
        } else {
            "ok"
        };
        json!({
            "status": status, "version": VERSION, "uptime_s": self.started.elapsed().as_secs(),
            "snapshot": {"revision": snapshot.revision, "age_s": now_ms().saturating_sub(snapshot.generated_ms) / 1000,
                         "grants": snapshot.grants.len(), "rejected": self.rejected_snapshots.load(SeqCst)},
            "keys": {"total": total, "disabled": disabled},
            "queue": {"capacity": self.provider.capacity, "in_use": self.provider.capacity - self.provider.admitted.available_permits()},
            "telemetry": {"lost": self.telemetry.lost.load(SeqCst), "shipper": self.telemetry.shipper.lock().unwrap().clone()},
        })
    }

    fn authenticate(&self, request: &Request<Incoming>) -> Option<String> {
        let token = request.headers().get(AUTHORIZATION)?.to_str().ok()?.strip_prefix("Bearer ")?.trim();
        self.snapshot.read().unwrap().grants.get(&sha256(token)).cloned()
    }

    async fn handle(self: Arc<Self>, request: Request<Incoming>) -> Result<Response<Body>, std::convert::Infallible> {
        let route = (request.method().clone(), request.uri().path().to_owned());
        let (operation, field, path) = match (&route.0, route.1.as_str()) {
            (&Method::GET, "/healthz") => return Ok(reply(StatusCode::OK, self.health().to_string())),
            (&Method::POST, "/v1/read/web_search") => ("web_search", "query", "/search"),
            (&Method::POST, "/v1/read/web_extract") => ("web_extract", "urls", "/extract"),
            _ => return Ok(reply(StatusCode::NOT_FOUND, json!({"error": "not-found", "reason": "unknown route"}).to_string())),
        };
        let (started_ms, begun) = (now_ms(), Instant::now());
        let header = |name: &str| request.headers().get(name).and_then(|v| v.to_str().ok()).map(str::to_owned);
        let (label, runtime) = (header("x-pipe-task"), header("x-pipe-runtime"));
        let project = self.authenticate(&request);
        let code_or_none = |value: &Option<String>| value.clone().filter(|v| is_code(v));
        let caller = Caller { project: project.clone(), label: code_or_none(&label), runtime: code_or_none(&runtime) };
        let step = if operation == "web_search" { "tavily.search" } else { "tavily.extract" };
        // Read the body before any refusal: closing on unread bytes resets the connection and loses the caller's reason.
        // Unauthenticated callers get a small, short read, so a slow or large body cannot hold the lane.
        // ponytail: no cap on open connections or header read time; entry is loopback or the guarded Caddy route, add both if local abuse appears.
        let (limit, patience) =
            if project.is_some() { (REQUEST_BYTES, self.config.request_timeout) } else { (REFUSED_BYTES, REFUSED_PATIENCE) };
        let collected = tokio::time::timeout(patience, Limited::new(request.into_body(), limit).collect()).await;
        let answer = if project.is_none() {
            refusal(StatusCode::UNAUTHORIZED, "refused", "unauthenticated", "a valid project token is required")
        } else if caller.label != label || caller.runtime != runtime {
            refusal(
                StatusCode::BAD_REQUEST,
                "refused",
                "bad-label",
                "X-Pipe-Task and X-Pipe-Runtime hold at most 64 characters of A-Za-z0-9._:-",
            )
        } else {
            match collected.map(|read| read.map(|b| serde_json::from_slice::<Value>(&b.to_bytes()))) {
                Ok(Ok(Ok(Value::Object(mut body)))) if body.contains_key(field) => {
                    // The caller never selects a key, and every call reports its credits.
                    body.remove("api_key");
                    body.insert("include_usage".into(), json!(true));
                    self.provider.call(path, Value::Object(body).to_string().into()).await
                }
                _ => refusal(
                    StatusCode::BAD_REQUEST,
                    "refused",
                    "bad-request",
                    &format!("the body must be a JSON object of at most 1 MiB with {field}"),
                ),
            }
        };
        if let Some((id, status)) = &answer.disabled {
            self.on_disabled(id, *status).await;
        }
        self.record(operation, step, &caller, started_ms, begun.elapsed(), &answer);
        let mut response = reply(answer.status, answer.body);
        if let Some(value) = answer.retry_after {
            response.headers_mut().insert(RETRY_AFTER, value);
        }
        Ok(response)
    }

    /// Adopt a newer valid snapshot; refuse an invalid one and keep serving the current one.
    fn refresh_snapshot(&self, seen: &mut Option<SystemTime>) {
        let Ok(modified) = std::fs::metadata(&self.config.snapshot).and_then(|m| m.modified()) else { return };
        if *seen == Some(modified) {
            return;
        }
        *seen = Some(modified);
        let good = self.config.state_dir.join("snapshot.good.json");
        match std::fs::read(&self.config.snapshot).map_err(|e| e.to_string()).and_then(|b| Snapshot::parse(&b).map(|s| (b, s))) {
            Ok((bytes, snapshot)) => {
                if let Err(error) = write_atomic(&good, &bytes) {
                    eprintln!("pipe-read: retain snapshot: {error}");
                }
                *self.snapshot.write().unwrap() = Arc::new(snapshot);
            }
            Err(error) => {
                self.rejected_snapshots.fetch_add(1, SeqCst);
                eprintln!("pipe-read: snapshot refused, keeping the last good one: {error}");
            }
        }
    }
}

/// A socket-activated listener keeps the port open while a restart drains; `listen` serves tests and manual runs.
fn listener(config: &Config) -> Res<std::net::TcpListener> {
    let activated = std::env::var("LISTEN_PID").ok().and_then(|p| p.parse::<u32>().ok()) == Some(std::process::id())
        && std::env::var("LISTEN_FDS").ok().as_deref() == Some("1");
    let listener = if activated {
        // SAFETY: systemd passes the one socket at this descriptor and nothing else owns it.
        unsafe { std::net::TcpListener::from_raw_fd(SYSTEMD_FIRST_FD) }
    } else {
        let address = config.listen.as_deref().ok_or("no activated socket and no listen address")?;
        std::net::TcpListener::bind(address).map_err(|e| format!("bind {address}: {e}"))?
    };
    let address = listener.local_addr().map_err(|e| e.to_string())?;
    if !address.ip().is_loopback() {
        return Err(format!("the lane binds only to loopback, not {address}"));
    }
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    Ok(listener)
}

async fn serve(config: Config) -> Res<()> {
    std::fs::create_dir_all(&config.state_dir).map_err(|e| format!("state directory: {e}"))?;
    let disabled: BTreeSet<String> =
        std::fs::read_to_string(config.state_dir.join("disabled-keys")).unwrap_or_default().lines().map(str::to_owned).collect();
    let provider = Provider::new(&config, &disabled)?;
    let snapshot = load_snapshot(&config)?;
    let listener = TcpListener::from_std(listener(&config)?).map_err(|e| e.to_string())?;
    let (telemetry, writer) = Telemetry::start(&config);
    let lane = Arc::new(Lane {
        config,
        provider,
        snapshot: RwLock::new(Arc::new(snapshot)),
        rejected_snapshots: AtomicU64::new(0),
        telemetry,
        draining: AtomicBool::new(false),
        disabled_write: Mutex::new(()),
        started: Instant::now(),
    });
    let watcher = lane.clone();
    tokio::spawn(async move {
        let mut seen = std::fs::metadata(&watcher.config.snapshot).and_then(|m| m.modified()).ok();
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        loop {
            tick.tick().await;
            watcher.refresh_snapshot(&mut seen);
        }
    });
    let mut terminate = signal(SignalKind::terminate()).map_err(|e| e.to_string())?;
    let graceful = GracefulShutdown::new();
    println!("pipe-read: serving on {}", listener.local_addr().map_err(|e| e.to_string())?);
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let Ok((stream, _)) = accepted else { continue };
                let lane = lane.clone();
                let connection = hyper::server::conn::http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service_fn(move |request| lane.clone().handle(request)));
                let watched = graceful.watch(connection);
                tokio::spawn(async move { let _ = watched.await; });
            }
            _ = terminate.recv() => break,
        }
    }
    lane.draining.store(true, SeqCst);
    drop(listener);
    if tokio::time::timeout(lane.config.drain_timeout, graceful.shutdown()).await.is_err() {
        eprintln!("pipe-read: drain timed out; remaining calls are cut");
    }
    lane.telemetry.close();
    tokio::task::spawn_blocking(move || writer.join()).await.ok();
    println!("pipe-read: drained, lost records: {}", lane.telemetry.lost.load(SeqCst));
    Ok(())
}

// ---------------------------------------------------------------- operator commands

/// Compare Tavily's per-key usage since the first run with the credits recorded locally for that key.
async fn reconcile(config: Config) -> Res<Value> {
    let provider = Provider::new(&config, &BTreeSet::new())?;
    let baseline_path = config.state_dir.join("usage-baseline.json");
    let mut baseline: Value = std::fs::read(&baseline_path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or(json!({"since_ms": now_ms(), "keys": {}}));
    let since_ms = baseline["since_ms"].as_i64().ok_or("usage baseline is invalid")?;
    let (mut recorded, mut unknown_units) = (HashMap::<String, u64>::new(), 0u64);
    let mut runtimes = std::collections::BTreeMap::<String, u64>::new();
    let store = config.store_path();
    if store.exists() {
        let reader = Reader::open(&store).map_err(|e| format!("telemetry store: {e}"))?;
        let mut after = 0;
        loop {
            let page = reader.export(since_ms, after, 1024).map_err(|e| format!("telemetry export: {e}"))?;
            for event in page["events"].as_array().into_iter().flatten() {
                let observation = &event["observation"];
                if observation["kind"] != "operation" {
                    continue;
                }
                *runtimes.entry(observation["runtime"].as_str().unwrap_or("unreported").to_owned()).or_default() += 1;
                let Some(key) = observation["account"].as_str() else { continue };
                match observation["units"].as_u64() {
                    Some(units) => *recorded.entry(key.to_owned()).or_default() += units,
                    None => unknown_units += 1,
                }
            }
            if page["more"] != true {
                break;
            }
            after = page["next"].as_i64().ok_or("telemetry export has no cursor")?;
        }
    }
    let mut rows = Vec::new();
    for key in &provider.keys {
        let request = Request::get(format!("{}/usage", provider.url))
            .header(AUTHORIZATION, format!("Bearer {}", key.value))
            .body(Body::default())
            .expect("static request shape");
        let observed = match provider.client.request(request).await {
            Ok(response) if response.status().is_success() => {
                let body = response.into_body().collect().await.map_err(|e| format!("usage body: {e}"))?.to_bytes();
                serde_json::from_slice::<Value>(&body)
                    .ok()
                    .and_then(|v| v["key"]["usage"].as_u64())
                    .ok_or("usage reply has no key.usage".to_owned())
            }
            Ok(response) => Err(format!("usage HTTP {}", response.status().as_u16())),
            Err(error) => Err(format!("usage request: {error}")),
        };
        let local = recorded.get(&key.id).copied().unwrap_or(0);
        rows.push(match observed {
            Ok(usage) => {
                let start = baseline["keys"]
                    .as_object_mut()
                    .ok_or("usage baseline is invalid")?
                    .entry(key.id.clone())
                    .or_insert(json!(usage))
                    .as_u64()
                    .unwrap_or(usage);
                let delta = usage.saturating_sub(start);
                json!({"key": key.id, "tavily_delta": delta, "recorded": local, "difference": delta as i64 - local as i64})
            }
            // Unknown is never a match.
            Err(reason) => json!({"key": key.id, "tavily_delta": null, "recorded": local, "difference": null, "unknown": reason}),
        });
    }
    write_atomic(&baseline_path, baseline.to_string().as_bytes()).map_err(|e| format!("usage baseline: {e}"))?;
    Ok(json!({"since_ms": since_ms, "calls_without_units": unknown_units, "runtimes": runtimes, "keys": rows}))
}

fn enroll_command(config: &Config, destination: &str) -> Res<()> {
    let connection = config.coreq_connection.as_ref().ok_or("config coreq_connection is required")?;
    let request = enroll::Request {
        installation: None,
        // DashQ grants by its product allowlist, where Pipe's shipper is `assetq`; each record still names `pipe-read`.
        product: "assetq",
        host_label: &config.host,
        schemas: &[OBSERVATION_SCHEMA],
        scope: None,
        capabilities: &[],
    };
    enroll::pair(connection, destination, &request, &mut enroll::Https { browser: false })?;
    println!("pipe-read: telemetry connection is active");
    Ok(())
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let run = || -> Res<()> {
        if arguments == ["selftest"] {
            let _ = hyper_rustls::HttpsConnectorBuilder::new().with_webpki_roots().https_only().enable_http1().build();
            Snapshot::parse(br#"{"schema":1,"generated_ms":1,"grants":[]}"#)?;
            println!("pipe-read {VERSION}: selftest ok");
            return Ok(());
        }
        let [command, path, rest @ ..] = arguments.as_slice() else {
            return Err("usage: pipe-read selftest | serve|reconcile <config.json> | enroll <config.json> <https-url>".into());
        };
        let config = Config::load(path)?;
        let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        match (command.as_str(), rest) {
            ("serve", []) => runtime.block_on(serve(config)),
            ("reconcile", []) => {
                println!("{}", runtime.block_on(reconcile(config))?);
                Ok(())
            }
            ("enroll", [url]) => enroll_command(&config, url),
            _ => Err("unknown command".into()),
        }
    };
    if let Err(error) = run() {
        eprintln!("pipe-read: {error}");
        std::process::exit(2);
    }
}
