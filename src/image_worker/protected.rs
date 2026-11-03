//! R3/R4: one protected consumer path, including non-generating qualification.
use super::*;
use agentq_runtime::agent::{ProtectedAppServer, ProtectedHost, ProtectedIdentity};
use serde::Deserialize;
use std::io::{BufRead, BufReader};
use std::sync::mpsc;

// Installed signed TaskQ 4.6.66 candidate for non-generating qualification; native proof remains open.
pub const ADAPTER_SHA256: &str = "bd6dc0de5024ff7b6b0ae0a987f5467aa0c5196b005c7cdfd434d043709c7e42";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Launch {
    adapter: PathBuf,
    request: Value,
    profile_sha256: String,
    executable: PathBuf,
    executable_sha256: String,
    account_home: PathBuf,
    approver: String,
}

pub(super) fn launch(root: &Path, dir: &Path, launch: Launch) -> Res<(ProtectedAppServer, PathBuf)> {
    if !cfg!(target_os = "macos") {
        return Err("capability_unavailable: protected Workspace adapter is qualified on Mac only; no Windows grant or fallback".into());
    }
    let root = root.canonicalize().map_err(|_| "protected consumer project unavailable")?;
    if launch.request["slot"]["binding"]["root"] != json!(root) {
        return Err("protected consumer project differs from slot; no native launch".into());
    }
    let host = ProtectedHost::new(&launch.adapter, ADAPTER_SHA256)?;
    let expected = ProtectedIdentity {
        profile_sha256: launch.profile_sha256,
        executable: launch.executable,
        executable_sha256: launch.executable_sha256,
        account_home: launch.account_home,
        approver: launch.approver,
    };
    let home = expected.account_home.clone();
    // Match requirements are retained for recovery, never accepted as a grant.
    let marker = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(dir.join("protected-launch"))
        .map_err(|_| "protected launch already attempted; observe original operation, never replay")?;
    marker.sync_all().map_err(|e| e.to_string())?;
    save(&dir.join("host-request.json"), &launch.request)?;
    sync_spool(dir)?;
    let mut checks = OpenOptions::new().create_new(true).write(true).open(dir.join("host-readback.jsonl")).map_err(|e| e.to_string())?;
    let mut server = host.launch(launch.request, expected, Intent::Inherit, Path::new("/dev/null"))?;
    let verified = (|| -> Res<()> {
        let ready_until = Instant::now() + Duration::from_secs(10);
        let receipt = loop {
            match server.verify(ready_until) {
                Ok(receipt) => break receipt,
                Err(reason) if Instant::now() < ready_until => {
                    journal(&mut checks, "unknown", &json!({"reason":reason}))?;
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(reason) => return Err(reason),
            }
        };
        save(&dir.join("authority.json"), &receipt)
    })();
    if let Err(reason) = verified {
        let cancel = server.cancel(Instant::now() + Duration::from_secs(5));
        let persisted = save(&dir.join("host-cancel.json"), &json!({"receipt":cancel.as_ref().ok(),"unknown":cancel.as_ref().err()}));
        return Err(match persisted {
            Ok(()) => reason,
            Err(error) => format!("{reason}; cancellation receipt persistence unknown: {error}"),
        });
    }
    Ok((server, home))
}

/// Recovery uses the original operation; a dead consumer never authorizes replay/release.
pub fn observe(dir: &Path, cancel: bool) -> Res<Value> {
    if !cfg!(target_os = "macos") {
        return Err("capability_unavailable: protected adapter requires Mac".into());
    }
    let control: Launch =
        serde_json::from_value(read_json(&dir.join("control.json"))?).map_err(|_| "protected recovery control invalid")?;
    if read_json(&dir.join("host-request.json"))? != control.request {
        return Err("protected recovery request differs from original operation".into());
    }
    let host = ProtectedHost::new(&control.adapter, ADAPTER_SHA256)?;
    let until = Instant::now() + Duration::from_secs(5);
    let receipt = if cancel { host.cancel(&control.request, until)? } else { host.observe(&control.request, until)? };
    save(&dir.join(if cancel { "recovery-cancel.json" } else { "recovery-observe.json" }), &receipt)?;
    Ok(receipt)
}

fn emit(value: &Value) -> Res<()> {
    let mut out = std::io::stdout().lock();
    serde_json::to_writer(&mut out, value).map_err(|e| e.to_string())?;
    out.write_all(b"\n").and_then(|_| out.flush()).map_err(|e| e.to_string())
}

fn next_frame(rx: &mpsc::Receiver<Res<Value>>, until: Instant) -> Res<Value> {
    let left = until
        .checked_duration_since(Instant::now())
        .filter(|left| !left.is_zero())
        .ok_or("qualification deadline; retain original slot")?;
    rx.recv_timeout(left).map_err(|_| "qualification input ended/deadline; retain original slot")?
}

fn non_generating(frame: &Value) -> Res<&'static str> {
    match frame["method"].as_str() {
        Some("account/read") if frame["params"] == json!({"refreshToken":false}) => Ok("account/read"),
        Some("config/read") => Ok("config/read"),
        Some("model/list") => Ok("model/list"),
        Some("command/exec")
            if frame["params"]["timeoutMs"].as_u64().is_some_and(|n| n > 0 && n <= 10_000)
                && frame["params"]["outputBytesCap"].as_u64().is_some_and(|n| n > 0 && n <= 65_536)
                && frame["params"]["processId"].as_str().is_some_and(|s| !s.is_empty() && s.len() <= 100) =>
        {
            Ok("command/exec")
        }
        Some("command/exec/terminate") => Ok("command/exec/terminate"),
        _ => Err("qualification permits bounded non-generating reads/probes only; no model turn or configuration write".into()),
    }
}

fn catalogue_progress(runtime: &dyn Runtime, run: &mut Run, log: &mut File, id: &Value, until: Instant) -> Res<Vec<Value>> {
    let until = until.min(Instant::now() + Duration::from_secs(10));
    let mut events = vec![];
    loop {
        if Instant::now() >= until {
            return Err("catalogue response unobserved before deadline; original request retained; no resend".into());
        }
        events.extend(runtime_progress(runtime, run, log)?);
        if events
            .iter()
            .any(|m| m.get("id") == Some(id) && m.get("method").is_none() && (m.get("result").is_some() || m.get("error").is_some()))
        {
            return Ok(events);
        }
        std::thread::sleep(Duration::from_millis(20).min(until.saturating_duration_since(Instant::now())));
    }
}

/// PM drives this one real consumer over private stdio; no development-board task
/// is created for its native operation and no slot is released by this command.
pub fn qualify(dir: &Path) -> Res<()> {
    if !cfg!(target_os = "macos") {
        return Err("capability_unavailable: composed protected qualification requires the existing approved Mac Workspace host".into());
    }
    let root = std::env::current_dir().and_then(|p| p.canonicalize()).map_err(|e| e.to_string())?;
    let parent = dir
        .parent()
        .ok_or("qualification directory needs a workspace parent")?
        .canonicalize()
        .map_err(|_| "qualification parent unavailable")?;
    if !parent.starts_with(&root) || dir.file_name().is_none_or(|n| n == "." || n == "..") {
        return Err("qualification directory must be a new directory inside the current project".into());
    }
    let dir = parent.join(dir.file_name().unwrap());
    let builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = builder;
        builder.mode(0o700);
        builder.create(&dir).map_err(|_| "qualification directory exists or cannot be created; never replay")?;
    }
    #[cfg(not(unix))]
    builder.create(&dir).map_err(|_| "qualification directory exists or cannot be created; never replay")?;
    let owner = coreq::process::identity(std::process::id()).map_err(|e| e.to_string())?.ok_or("consumer identity unknown")?;
    save(&dir.join("consumer.json"), &json!({"pid":owner.pid,"started":owner.started,"root":root,"runtime":PIN}))?;
    emit(
        &json!({"state":"awaiting-existing-slot","pid":owner.pid,"started":owner.started,"root":root,"runtime":PIN,"adapter_sha256":ADAPTER_SHA256}),
    )?;
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut input = BufReader::new(std::io::stdin());
        loop {
            let mut bytes = vec![];
            let frame = match input.by_ref().take(32_769).read_until(b'\n', &mut bytes) {
                Ok(0) => break,
                Ok(_) if bytes.len() <= 32_768 && bytes.ends_with(b"\n") => {
                    serde_json::from_slice::<Value>(&bytes).map_err(|_| "invalid qualification JSON".to_string())
                }
                _ => Err("qualification input incomplete or exceeds 32 KiB".into()),
            };
            let failed = frame.is_err();
            if tx.send(frame).is_err() || failed {
                break;
            }
        }
    });
    let until = Instant::now() + Duration::from_secs(120);
    let attempt = (|| -> Res<()> {
        let control = next_frame(&rx, until)?;
        let requirements: Launch = serde_json::from_value(control.clone()).map_err(|_| "invalid protected launch match requirements")?;
        save(&dir.join("control.json"), &control)?;
        let mut log = OpenOptions::new().create_new(true).write(true).open(dir.join("native.jsonl")).map_err(|e| e.to_string())?;
        let (server, home) = launch(&root, &dir, requirements)?;
        let runtime = agent::runtime("codex").ok_or("Codex runtime unavailable")?;
        let work = (|| -> Res<()> {
            let mut run = server.open(None, &runtime_launch(&root, &home, until))?;
            let messages = runtime_progress(runtime, &mut run, &mut log)?;
            save(&dir.join("handshake.json"), &json!({"progress":messages}))?;
            emit(&json!({"state":"initialized","image_tool":"unknown: no current native image-tool capability receipt"}))?;
            let mut catalogue = None;
            loop {
                let frame = next_frame(&rx, until)?;
                match frame["action"].as_str() {
                    Some("cancel") => break,
                    Some("verify") => {
                        let receipt = observe(&dir, false)?;
                        journal(&mut log, "authority-observation", &receipt)?;
                        emit(&receipt)?;
                    }
                    Some("receive") => {
                        let events = if let Some(id) = catalogue.take() {
                            catalogue_progress(runtime, &mut run, &mut log, &id, until)?
                        } else {
                            runtime_progress(runtime, &mut run, &mut log)?
                        };
                        emit(&json!({"events":events}))?;
                    }
                    Some("request") => {
                        let method = non_generating(&frame)?;
                        if method == "account/read" {
                            let account = runtime.account(&mut run).map_err(|e| e.to_string())?;
                            emit(&json!({"login":format!("{:?}",account.login),"plan":account.plan}))?;
                        } else {
                            // Runtime owns the allowed catalogue reads; unsupported probes refuse before write.
                            journal(&mut log, "runtime-intent", &frame)?;
                            runtime.send(&mut run, &json!({"method":method,"params":frame["params"]}))?;
                            let checkpoint = runtime.checkpoint(&run).map_err(|e| e.to_string())?;
                            journal(&mut log, "runtime-checkpoint", &checkpoint)?;
                            if method == "model/list" {
                                // Observe the pinned Runtime's allocated ID; never allocate or override a native ID.
                                let pending = &checkpoint["app"]["pending"];
                                if pending[1] != method || !pending[0].is_u64() {
                                    return Err("catalogue request identity unknown; no resend".into());
                                }
                                catalogue = Some(pending[0].clone());
                            }
                            emit(&json!({"state":"read-submitted"}))?;
                        }
                    }
                    _ => return Err("unknown qualification action".into()),
                }
            }
            Ok(())
        })();
        let cancelled = observe(&dir, true);
        let supervisor_exit = json!({"unknown":"Runtime owns transport; supervisor exit is not whole-operation completion"});
        save(
            &dir.join("cancel.json"),
            &json!({"receipt":cancelled.as_ref().ok(),"unknown":cancelled.as_ref().err(),"supervisor_exit":supervisor_exit,"slot_release":"not attempted; PM must reconcile all probe descendants"}),
        )?;
        let recovered = observe(&dir, false);
        journal(&mut log, "recovery", &json!({"receipt":recovered.as_ref().ok(),"unknown":recovered.as_ref().err()}))?;
        work?;
        cancelled?;
        emit(
            &json!({"state":"cancel-requested","slot_release":"not attempted","image_tool":"unknown","usage":"no model turn submitted; image usage unobserved"}),
        )
    })();
    if let Err(ref reason) = attempt {
        save(&dir.join("unknown.json"), &json!({"reason":reason,"no_replay":true,"slot_release":"not attempted"}))?;
    }
    attempt
}

#[cfg(test)]
mod tests {
    use super::*;

    // R3/R4/R8: delayed catalogue replies need a matching ID; notifications, foreign
    // replies and empty reads cannot complete qualification or authorize a resend.
    #[test]
    fn image_worker_qualification_waits_for_catalogue_response() {
        for outcome in ["catalogue", "catalogue-empty", "catalogue-error", "catalogue-missing"] {
            let dir = std::env::temp_dir().join(format!("assetq-catalogue-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&dir).unwrap();
            let trace = dir.join("wire.jsonl");
            let log_path = dir.join("native.jsonl");
            let mut log = File::create(&log_path).unwrap();
            let mut launch = runtime_launch(&dir, &dir, Instant::now() + Duration::from_secs(10));
            launch.install = Some(dir.join("absent-optimizer"));
            launch.command = vec![
                if cfg!(windows) { "python" } else { "python3" }.into(),
                "-u".into(),
                Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/image-runtime.py").into_os_string(),
                trace.clone().into_os_string(),
                "chatgpt".into(),
                "openai".into(),
                outcome.into(),
            ];
            let runtime = agent::runtime("codex").unwrap();
            let mut run = runtime.launch(&launch).unwrap();
            runtime_progress(runtime, &mut run, &mut log).unwrap();
            runtime.send(&mut run, &json!({"method":"model/list","params":{}})).unwrap();
            let id = runtime.checkpoint(&run).unwrap()["app"]["pending"][0].clone();
            let release = format!("{}.release", trace.display());
            let releaser = std::thread::spawn(move || {
                let until = Instant::now() + Duration::from_secs(2);
                while !fs::read_to_string(&log_path).unwrap().contains("fixture/catalogue-waiting") && Instant::now() < until {
                    std::thread::sleep(Duration::from_millis(5));
                }
                fs::write(release, b"invented release").unwrap();
            });
            let result = catalogue_progress(runtime, &mut run, &mut log, &id, Instant::now() + Duration::from_secs(3));
            releaser.join().unwrap();
            runtime.cancel(&mut run).unwrap();
            let messages: Vec<Value> = fs::read_to_string(&trace).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect();
            assert_eq!(messages.iter().filter(|m| m["method"] == "model/list").count(), 1);
            assert!(!messages.iter().any(|m| m["method"] == "turn/start"));
            drop(log);
            fs::remove_dir_all(&dir).unwrap();
            if outcome == "catalogue-missing" {
                assert!(result.unwrap_err().contains("catalogue response unobserved"));
            } else {
                let events = result.unwrap();
                let reply =
                    events.iter().find(|m| m["id"] == id && m.get("method").is_none()).expect("matching catalogue response missing");
                assert!(events.iter().any(|m| m["method"] == "fixture/catalogue-waiting"));
                if outcome == "catalogue-error" {
                    assert!(reply.get("error").is_some());
                } else if outcome == "catalogue-empty" {
                    assert_eq!(reply["result"]["data"], json!([]));
                } else {
                    assert_eq!(reply["result"]["data"][0]["id"], "invented-inherited-model");
                }
            }
        }
    }
    // R3/R4: qualification cannot become a model turn, config write or unbounded command.
    #[test]
    fn image_worker_qualification_refuses_generation_and_unbounded_probes() {
        for method in ["turn/start", "thread/start", "config/value/write", "command/exec", "unknown"] {
            assert!(non_generating(&json!({"method":method,"params":{}})).is_err(), "{method}");
        }
        assert!(non_generating(&json!({"method":"account/read","params":{"refreshToken":true}})).is_err());
        for (timeout, cap, accepted) in [(1, 1, true), (10_000, 65_536, true), (10_001, 64, false), (100, 65_537, false), (0, 64, false)] {
            assert_eq!(
                non_generating(
                    &json!({"method":"command/exec","params":{"timeoutMs":timeout,"outputBytesCap":cap,"processId":"invented-probe"}})
                )
                .is_ok(),
                accepted
            );
        }
        assert!(serde_json::from_value::<Launch>(json!({"grant":"full","adapter_sha256":"forged"})).is_err());
        let (tx, rx) = mpsc::channel();
        tx.send(Ok(json!({"action":"receive"}))).unwrap();
        assert!(next_frame(&rx, Instant::now() - Duration::from_secs(1)).unwrap_err().contains("deadline"));
        assert_eq!(next_frame(&rx, Instant::now() + Duration::from_secs(1)).unwrap()["action"], "receive");
        if !cfg!(target_os = "macos") {
            assert!(qualify(Path::new("must-not-exist")).unwrap_err().contains("existing approved Mac Workspace"));
            assert!(!Path::new("must-not-exist").exists());
        }
    }
}
