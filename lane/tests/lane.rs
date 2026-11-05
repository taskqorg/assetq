//! Read-lane behavior through the real binary and a scripted provider (pipe.md § Read lane; A2, A4, A5).
//! Risk covered: a queue, key or snapshot rule that passes in isolation but fails in the running process.
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering::SeqCst};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const TOKEN: &str = "invented-project-token";
static NEXT: AtomicU64 = AtomicU64::new(0);

fn sha256(text: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// A scripted provider: `script(key)` gives (status, delay ms); status 0 cuts the connection without an answer.
struct Provider {
    url: String,
    seen: Arc<Mutex<Vec<(String, String)>>>,
}

fn read_http(stream: &mut TcpStream) -> Option<(String, String)> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let end = buffer.windows(4).position(|w| w == b"\r\n\r\n");
        if let Some(end) = end {
            let head = String::from_utf8_lossy(&buffer[..end]).to_string();
            let length: usize =
                head.lines().find_map(|l| l.to_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse().unwrap())).unwrap_or(0);
            if buffer.len() >= end + 4 + length {
                return Some((head, String::from_utf8_lossy(&buffer[end + 4..end + 4 + length]).to_string()));
            }
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return None,
            Ok(n) => buffer.extend_from_slice(&chunk[..n]),
        }
    }
}

fn provider(script: impl Fn(&str) -> (u16, u64) + Send + Sync + 'static) -> Provider {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (script, log) = (Arc::new(script), seen.clone());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let (script, log, mut stream) = (script.clone(), log.clone(), stream.unwrap());
            std::thread::spawn(move || {
                while let Some((head, body)) = read_http(&mut stream) {
                    let key = head.lines().find_map(|l| l.strip_prefix("authorization: Bearer ")).unwrap_or("").to_owned();
                    log.lock().unwrap().push((key.clone(), body));
                    let (status, delay) = script(&key);
                    std::thread::sleep(Duration::from_millis(delay));
                    if status == 0 {
                        return;
                    }
                    let reply = json!({"results": [{"title": "invented"}], "usage": {"credits": 2}}).to_string();
                    let extra = if status == 429 { "Retry-After: 7\r\n" } else { "" };
                    let text = format!("HTTP/1.1 {status} X\r\n{extra}Content-Length: {}\r\n\r\n{reply}", reply.len());
                    if stream.write_all(text.as_bytes()).is_err() {
                        return;
                    }
                }
            });
        }
    });
    Provider { url, seen }
}

struct Fixture {
    dir: PathBuf,
    port: u16,
    child: Option<Child>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn write_snapshot(dir: &Path, grants: &[(&str, &str)]) {
    let grants: Vec<Value> = grants.iter().map(|(t, p)| json!({"token_sha256": sha256(t), "project": p})).collect();
    let text = json!({"schema": 1, "generated_ms": 1, "grants": grants}).to_string();
    std::fs::write(dir.join("snapshot.tmp"), text).unwrap();
    std::fs::rename(dir.join("snapshot.tmp"), dir.join("snapshot.json")).unwrap();
}

impl Fixture {
    fn new(url: &str, keys: &[&str], extra: Value) -> Self {
        let serial = NEXT.fetch_add(1, SeqCst);
        let dir = std::env::temp_dir().join(format!("pipe-read-{}-{serial}", std::process::id()));
        std::fs::create_dir_all(dir.join("cred")).unwrap();
        std::fs::write(dir.join("cred/tavily-keys"), keys.join("\n")).unwrap();
        write_snapshot(&dir, &[(TOKEN, "invented/project")]);
        // Below the ephemeral range: a freed port :0 could go to a parallel test's listener before the lane binds it.
        let port = 20000 + (std::process::id() % 9000) as u16 + serial as u16;
        let mut config = json!({"listen": format!("127.0.0.1:{port}"), "host": "invented-host", "snapshot": dir.join("snapshot.json"),
                                "state_dir": dir.join("state"), "provider_url": url});
        for (name, value) in extra.as_object().unwrap() {
            config[name] = value.clone();
        }
        std::fs::write(dir.join("config.json"), config.to_string()).unwrap();
        let mut fixture = Self { dir, port, child: None };
        fixture.start();
        fixture
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_pipe-read"));
        command
            .args(["serve"])
            .arg(self.dir.join("config.json"))
            .env("CREDENTIALS_DIRECTORY", self.dir.join("cred"))
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command
    }

    fn start(&mut self) {
        self.child = Some(self.command().spawn().unwrap());
        self.wait_ready();
    }

    fn wait_ready(&self) {
        let end = Instant::now() + Duration::from_secs(10);
        while Instant::now() < end {
            if TcpStream::connect(("127.0.0.1", self.port)).is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("lane did not listen");
    }

    fn stop(&mut self) -> std::process::ExitStatus {
        let mut child = self.child.take().unwrap();
        unsafe { libc::kill(child.id() as i32, libc::SIGTERM) };
        child.wait().unwrap()
    }

    fn health(&self) -> Value {
        call(self.port, "GET", "/healthz", &[], "").1
    }

    fn search(&self) -> (u16, Value) {
        call(
            self.port,
            "POST",
            "/v1/read/web_search",
            &[("Authorization", &format!("Bearer {TOKEN}")), ("X-Pipe-Task", "49"), ("X-Pipe-Runtime", "claude")],
            r#"{"query":"q"}"#,
        )
    }

    /// Stored records, read after the lane stopped so its writer has flushed.
    fn records(&self) -> Vec<Value> {
        let reader = coreq::telemetry::Reader::open(&self.dir.join("state/telemetry.db")).unwrap();
        reader.export(0, 0, 1024).unwrap()["events"].as_array().unwrap().iter().map(|e| e["observation"].clone()).collect()
    }
}

fn call(port: u16, method: &str, path: &str, headers: &[(&str, &str)], body: &str) -> (u16, Value) {
    let (status, _, value) = call_raw(port, method, path, headers, body);
    (status, value)
}

fn call_raw(port: u16, method: &str, path: &str, headers: &[(&str, &str)], body: &str) -> (u16, String, Value) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
    let extra: String = headers.iter().map(|(k, v)| format!("{k}: {v}\r\n")).collect();
    write!(stream, "{method} {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n{extra}Content-Length: {}\r\n\r\n{body}", body.len())
        .unwrap();
    let mut text = String::new();
    stream.read_to_string(&mut text).unwrap();
    let (head, payload) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    (status, head.to_owned(), serde_json::from_str(payload).unwrap_or(Value::Null))
}

fn ok_provider() -> Provider {
    provider(|_| (200, 0))
}

#[test]
fn call_is_authenticated_forwarded_and_fully_recorded() {
    let upstream = ok_provider();
    let mut lane = Fixture::new(&upstream.url, &["invented-key-a"], json!({}));
    let (status, _) = call(lane.port, "POST", "/v1/read/web_search", &[("Authorization", "Bearer wrong")], r#"{"query":"q"}"#);
    assert_eq!(status, 401);
    let auth = [("Authorization", "Bearer invented-project-token")];
    assert_eq!(call(lane.port, "POST", "/v1/read/web_search", &auth, r#"{"nothing":1}"#).0, 400);
    let (status, body) = call(
        lane.port,
        "POST",
        "/v1/read/web_extract",
        &[auth[0], ("X-Pipe-Task", "opaque:label-7")],
        r#"{"urls":["https://x.invalid"],"api_key":"caller"}"#,
    );
    assert_eq!((status, body["results"][0]["title"].as_str()), (200, Some("invented")));
    assert_eq!(lane.search().0, 200);
    let seen = upstream.seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 2, "refused calls must not reach the provider");
    assert_eq!(lane.health()["telemetry"]["lost"], 0);
    let sent: Value = serde_json::from_str(&seen[0].1).unwrap();
    assert_eq!((sent["include_usage"].as_bool(), sent.get("api_key").is_none()), (Some(true), true));
    assert_eq!(lane.stop().code(), Some(0));
    let records = lane.records();
    assert_eq!(records.len(), 4);
    let search = &records[3];
    for (field, value) in [
        ("pipeline", json!("web_search")),
        ("pipeline_revision", json!("1")),
        ("step", json!("tavily.search")),
        ("project", json!("invented/project")),
        ("task", json!(49)),
        ("route", json!("tavily")),
        ("units", json!(2)),
        ("unit_kind", json!("tavily_credits")),
        ("outcome", json!("ok")),
        ("runtime", json!("claude")),
    ] {
        assert_eq!(search[field], value, "{field}");
    }
    assert!(search["duration_us"].is_u64());
    let opaque = &records[2];
    assert_eq!(
        (opaque["source"].as_str(), opaque["task"].is_null(), opaque["unknown"]["task"].as_str()),
        (Some("opaque:label-7"), true, Some("non-numeric-label"))
    );
    assert_eq!((records[0]["outcome"].as_str(), records[0]["error"].as_str()), (Some("refused"), Some("unauthenticated")));
    assert_eq!(records[0]["unknown"]["units"], "no-provider-call");
}

#[test]
fn full_queue_refuses_at_once_with_reason() {
    let upstream = provider(|_| (200, 1500));
    let lane = Fixture::new(&upstream.url, &["invented-key-a"], json!({"concurrency": 1, "queue": 1}));
    let port = lane.port;
    let held: Vec<_> = (0..2)
        .map(|_| {
            std::thread::spawn(move || {
                call(port, "POST", "/v1/read/web_search", &[("Authorization", "Bearer invented-project-token")], r#"{"query":"q"}"#).0
            })
        })
        .collect();
    let end = Instant::now() + Duration::from_secs(5);
    while lane.health()["queue"]["in_use"] != 2 && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(5));
    }
    let begun = Instant::now();
    let (status, body) = lane.search();
    assert!(begun.elapsed() < Duration::from_millis(500), "a full queue must not wait");
    assert_eq!((status, body["error"].as_str()), (503, Some("queue-full")));
    assert_eq!(held.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>(), [200, 200]);
}

#[test]
fn key_rotates_only_on_connection_failure() {
    let upstream = provider(|key| if key == "invented-key-a" { (0, 0) } else { (200, 0) });
    let mut lane = Fixture::new(&upstream.url, &["invented-key-a", "invented-key-b"], json!({}));
    assert_eq!(lane.search().0, 200);
    assert_eq!(lane.search().0, 200);
    assert_eq!(lane.health()["keys"]["disabled"], 0);
    let keys: Vec<String> = upstream.seen.lock().unwrap().iter().map(|s| s.0.clone()).collect();
    assert_eq!(keys, ["invented-key-a", "invented-key-b", "invented-key-b"], "rotation sticks and the failed key is tried once");
    lane.stop();
    let records = lane.records();
    assert_eq!((records[0]["outcome"].as_str(), records[1]["outcome"].as_str()), (Some("retried"), Some("ok")));
}

#[test]
fn quota_and_429_pass_through_without_rotation() {
    for status in [429u16, 432, 433] {
        let upstream = provider(move |_| (status, 0));
        let lane = Fixture::new(&upstream.url, &["invented-key-a", "invented-key-b"], json!({}));
        let auth = [("Authorization", "Bearer invented-project-token")];
        let (seen_status, head, body) = call_raw(lane.port, "POST", "/v1/read/web_search", &auth, r#"{"query":"q"}"#);
        assert_eq!(seen_status, status);
        assert_eq!(body["usage"]["credits"], 2, "the provider body reaches the caller");
        assert_eq!(status != 429 || head.to_lowercase().contains("retry-after: 7"), true);
        lane.search();
        assert_eq!(lane.health()["keys"]["disabled"], 0);
        assert!(upstream.seen.lock().unwrap().iter().all(|s| s.0 == "invented-key-a"), "no rotation on {status}");
    }
}

#[test]
fn refused_key_is_disabled_alerted_and_stays_disabled() {
    for status in [401u16, 402, 403] {
        let upstream = provider(move |key| if key == "invented-key-a" { (status, 0) } else { (200, 0) });
        let alert = std::env::temp_dir().join(format!("pipe-read-alert-{}-{status}", std::process::id()));
        let _ = std::fs::remove_file(&alert);
        let script = format!("printf '%s' \"$1\" > {}", alert.display());
        let mut lane =
            Fixture::new(&upstream.url, &["invented-key-a", "invented-key-b"], json!({"alert_command": ["sh", "-c", script, "alert"]}));
        let (code, body) = lane.search();
        assert_eq!((code, body["error"].as_str()), (502, Some(format!("key-disabled-{status}").as_str())));
        assert!(!body.to_string().contains("invented-key"), "a reply never holds a key");
        assert_eq!(lane.search().0, 200, "later calls use the next enabled key");
        assert_eq!(lane.health()["keys"]["disabled"], 1);
        let end = Instant::now() + Duration::from_secs(5);
        while !alert.exists() && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(10));
        }
        let message = std::fs::read_to_string(&alert).unwrap();
        assert!(message.contains(&format!("HTTP {status}")) && !message.contains("invented-key"), "{message}");
        std::fs::remove_file(&alert).unwrap();
        lane.stop();
        assert!(lane.records().iter().any(|r| r["kind"] == "check" && r["error"] == format!("key-disabled-{status}")));
        lane.start();
        assert_eq!(lane.health()["keys"]["disabled"], 1, "a restart keeps the key disabled");
        assert_eq!(lane.search().0, 200);
        assert!(upstream.seen.lock().unwrap().iter().skip(1).all(|s| s.0 == "invented-key-b"));
    }
}

#[test]
fn every_key_disabled_refuses_with_reason() {
    let upstream = provider(|_| (401, 0));
    let lane = Fixture::new(&upstream.url, &["invented-key-a"], json!({}));
    lane.search();
    let (status, body) = lane.search();
    assert_eq!((status, body["error"].as_str()), (503, Some("no-usable-key")));
    assert_eq!(lane.health()["status"], "no-usable-key");
}

#[test]
fn snapshot_loss_keeps_last_known_good() {
    let upstream = ok_provider();
    let mut lane = Fixture::new(&upstream.url, &["invented-key-a"], json!({}));
    let path = lane.dir.join("snapshot.json");
    // Control publishes garbage: the lane keeps serving and counts the refusal.
    std::fs::write(&path, "{broken").unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    while lane.health()["snapshot"]["rejected"] != 1 && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(lane.health()["snapshot"]["rejected"], 1);
    assert_eq!(lane.search().0, 200);
    // A restart with the control side still broken or gone serves the retained copy.
    lane.stop();
    lane.start();
    assert_eq!(lane.search().0, 200);
    lane.stop();
    std::fs::remove_file(&path).unwrap();
    lane.start();
    assert_eq!(lane.search().0, 200);
    // A new valid publication takes effect, revocation included.
    write_snapshot(&lane.dir, &[("invented-other-token", "invented/other")]);
    let end = Instant::now() + Duration::from_secs(5);
    while lane.search().0 != 401 && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(lane.search().0, 401);
    assert_eq!(
        call(lane.port, "POST", "/v1/read/web_search", &[("Authorization", "Bearer invented-other-token")], r#"{"query":"q"}"#).0,
        200
    );
}

#[test]
fn unusable_store_counts_loss_and_calls_keep_working() {
    let upstream = ok_provider();
    let mut lane = Fixture::new(&upstream.url, &["invented-key-a"], json!({}));
    lane.stop();
    std::fs::create_dir_all(lane.dir.join("state")).unwrap();
    let store = lane.dir.join("state/telemetry.db");
    std::fs::remove_file(&store).ok();
    std::fs::write(&store, "not a database, longer than a sqlite header would need to be to fail the read").unwrap();
    lane.start();
    assert_eq!(lane.search().0, 200);
    let end = Instant::now() + Duration::from_secs(5);
    while lane.health()["telemetry"]["lost"] != 1 && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(lane.health()["telemetry"]["lost"], 1);
    lane.stop();
    lane.start();
    assert_eq!(lane.health()["telemetry"]["lost"], 1, "the counter survives a restart");
}

/// The socket unit's contract: the port stays open, an in-flight call finishes, a call during the gap is served by the next process.
#[test]
fn restart_under_activated_socket_drains_and_refuses_nobody() {
    let upstream = provider(|_| (200, 700));
    let mut lane = Fixture::new(&upstream.url, &["invented-key-a"], json!({}));
    lane.stop();
    let socket = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = socket.local_addr().unwrap().port();
    // LISTEN_PID must name the service itself, so a shell sets it before exec.
    let activated = |fixture: &Fixture| -> Child {
        let fd = socket.as_raw_fd();
        let service = fixture.command();
        let mut shell = Command::new("sh");
        shell.arg("-c").arg("LISTEN_PID=$$ LISTEN_FDS=1 exec \"$0\" \"$@\"").arg(service.get_program()).args(service.get_args());
        shell.env("CREDENTIALS_DIRECTORY", fixture.dir.join("cred")).stdout(Stdio::null()).stderr(Stdio::null());
        unsafe {
            shell.pre_exec(move || if libc::dup2(fd, 3) < 0 { Err(std::io::Error::last_os_error()) } else { Ok(()) });
        }
        shell.spawn().unwrap()
    };
    let mut first = activated(&lane);
    let auth = [("Authorization", "Bearer invented-project-token")];
    let call_once = move || call(port, "POST", "/v1/read/web_search", &auth, r#"{"query":"q"}"#).0;
    let end = Instant::now() + Duration::from_secs(5);
    while call(port, "GET", "/healthz", &[], "").0 != 200 && Instant::now() < end {}
    let in_flight = std::thread::spawn(call_once);
    std::thread::sleep(Duration::from_millis(200));
    unsafe { libc::kill(first.id() as i32, libc::SIGTERM) };
    // The old process has stopped accepting: this connection waits in the kernel's backlog for the next one.
    std::thread::sleep(Duration::from_millis(100));
    let in_gap = std::thread::spawn(call_once);
    assert_eq!(first.wait().unwrap().code(), Some(0), "the stop drains and exits cleanly");
    assert_eq!(in_flight.join().unwrap(), 200, "the in-flight call finishes");
    let mut second = activated(&lane);
    assert_eq!(in_gap.join().unwrap(), 200, "a call during the gap is served, not refused");
    unsafe { libc::kill(second.id() as i32, libc::SIGTERM) };
    assert_eq!(second.wait().unwrap().code(), Some(0));
}
