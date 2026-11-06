//! R3/R4/R10: one protected consumer path, including non-generating qualification.
use super::capacity::{Capacity, Event};
use super::*;
use agentq_runtime::agent::{ProtectedAppServer, ProtectedHost, ProtectedPreparation};
use ring::signature::Ed25519KeyPair;
use serde::Deserialize;
use std::io::{BufRead, BufReader};
use std::sync::mpsc;

// AgentQ #266 macOS arm64 adapter, natively qualified for transport through Pipe #61.
pub const ADAPTER_SHA256: &str = "a1cbb7b88959532f2be28ea8d696c4cc1ae17b7c97205d6549f7156164c82a58";
const ADAPTER_PUBLIC_KEY: &str = "7f739abab8625718e2e5dbedf0deb5a98373260d3c22ecd20a1148f42bf6d584";
const ADAPTER_SIGNATURE: &str =
    "b1c2afbaf90ac266027564fc9d40887a1fafc7dfb31a1964678ec652be7ed5794c08013652468a1d916ea190e0ed3e4169bf085428d869d3d671133b1c907d04";
// The caller the existing signed host policy already authorizes; using it needs no policy or grant edit.
const PERMIT_CALLER: &str = "assetq-52-qualification";
const PERMIT_SECONDS: u64 = 120;

/// Expected identities the operator read independently of this operation's receipts.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Expected {
    adapter: PathBuf,
    profile_sha256: String,
    executable: PathBuf,
    executable_sha256: String,
    account_home: PathBuf,
    approver: String,
}

/// Both consumers must observe owned host cancellation before closing native stdin.
pub(super) fn with_run<T>(
    mut opened: Res<Run>,
    work: impl FnOnce(&mut Run) -> Res<T>,
    cleanup: impl FnOnce() -> Res<Value>,
) -> (Res<T>, Res<Value>) {
    let result = opened.as_mut().map_err(|e| e.clone()).and_then(work);
    let cleanup = cleanup();
    drop(opened);
    (result, cleanup)
}

pub(super) fn signed_host(adapter: &Path) -> Res<ProtectedHost> {
    ProtectedHost::signed(adapter, ADAPTER_SHA256, ADAPTER_PUBLIC_KEY, ADAPTER_SIGNATURE)
}

#[cfg(target_os = "macos")]
fn boot_time() -> Res<i64> {
    let mut time = std::mem::MaybeUninit::<libc::timeval>::zeroed();
    let mut size = std::mem::size_of::<libc::timeval>();
    let mut mib = [libc::CTL_KERN, libc::KERN_BOOTTIME];
    // SAFETY: the buffer and its length describe one timeval, which sysctl fills.
    if unsafe { libc::sysctl(mib.as_mut_ptr(), 2, time.as_mut_ptr().cast(), &mut size, std::ptr::null_mut(), 0) } != 0 {
        return Err("host boot unknown; no permit".into());
    }
    // SAFETY: zeroed is a valid timeval, whether or not sysctl wrote to it.
    Ok(unsafe { time.assume_init() }.tv_sec)
        .and_then(|seconds| if seconds > 0 { Ok(seconds) } else { Err("host boot unknown; no permit".into()) })
}

#[cfg(not(target_os = "macos"))]
fn boot_time() -> Res<i64> {
    Err("capability_unavailable: protected permit requires the existing Mac authority".into())
}

fn issuer_key() -> Res<Vec<u8>> {
    let path = std::env::var_os("ASSETQ_PERMIT_ISSUER_KEY")
        .ok_or("capability_unavailable: ASSETQ_PERMIT_ISSUER_KEY is not configured; no permit")?;
    fs::read(path).map_err(|_| "capability_unavailable: protected permit issuer key unreadable; no permit".into())
}

/// The signed payload keeps the binding's own field order, as the host reads it back.
fn issue_permit(pkcs8: &[u8], root: &Path, op: &str, owner: &coreq::process::Identity, boot: i64, now: u64) -> Res<Value> {
    let pair = Ed25519KeyPair::from_pkcs8(pkcs8).map_err(|_| "capability_unavailable: protected permit issuer key invalid; no permit")?;
    let key = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
    let binding = json!({"schema":"pipe.permits/1","caller":PERMIT_CALLER,"op_id":op,"root":root,"runtime":"codex",
        "pid":owner.pid,"started":owner.started,"boot":boot,"expires_at":now+PERMIT_SECONDS});
    let payload = json!({"binding":binding,"key_sha256":db::sha256_hex(key.as_bytes())});
    let mut message = b"pipe.permits/1\0".to_vec();
    message.extend_from_slice(payload.to_string().as_bytes());
    Ok(json!({"binding":binding,"key":key,"signature":hex::encode(pair.sign(&message).as_ref())}))
}

fn capacity_of(dir: &Path) -> Res<(Capacity, String)> {
    let scope = read_json(&dir.join("permit-capacity.json"))?;
    Ok((
        Capacity(PathBuf::from(scope["path"].as_str().ok_or("protected capacity path missing")?)),
        scope["op"].as_str().ok_or("protected capacity operation missing")?.into(),
    ))
}

/// Hold-first ordering: the permit exists only for an operation that owns capacity.
pub(super) fn prepare_launch(
    root: &Path,
    dir: &Path,
    op: &str,
    adapter: &Path,
    capacity: &Path,
    expected: Option<&Expected>,
) -> Res<(ProtectedAppServer, PathBuf)> {
    if !cfg!(target_os = "macos") {
        return Err("capability_unavailable: protected Workspace adapter is qualified on Mac only; no Windows grant or fallback".into());
    }
    let host = signed_host(adapter)?;
    let pkcs8 = issuer_key()?;
    let owner = coreq::process::identity(std::process::id()).map_err(|_| "runner identity unknown")?.ok_or("runner absent")?;
    save(&dir.join("runner.json"), &json!({"pid":owner.pid,"started":owner.started}))?;
    let root = root.canonicalize().map_err(|_| "qualification project unavailable")?;
    let at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|_| "permit clock unknown")?.as_secs();
    let permit = issue_permit(&pkcs8, &root, op, &owner, boot_time()?, at)?;
    seal(&dir.join("permit-capacity.json"), &json!({"path":capacity,"op":op}))?;
    let (capacity, _) = capacity_of(dir)?;
    capacity.apply(Event::Acquire { op: op.into() })?;
    seal(&dir.join("permit.json"), &permit)?;
    let preparation = host.prepare(&permit, Instant::now() + Duration::from_secs(5))?;
    save(&dir.join("preparation.json"), &preparation.checkpoint())?;
    save(&dir.join("host-request.json"), preparation.request())?;
    if let Some(expected) = expected {
        expected_matches(&preparation.checkpoint(), expected)?;
    }
    // Prepared authority is resolved by the pinned host before launch, never copied from an operation observation.
    let home = PathBuf::from(
        preparation.checkpoint()["receipt"]["prepared"]["authority"]["account_home"].as_str().ok_or("prepared account home unknown")?,
    );
    capacity.apply(Event::LaunchIntended { op: op.into() })?;
    let marker = private_file(&dir.join("protected-launch")).map_err(|_| "protected launch already attempted; no replay")?;
    marker.sync_all().map_err(|e| e.to_string())?;
    sync_spool(dir)?;
    let mut server = host.launch_prepared(&preparation, Intent::Inherit, Path::new("/dev/null"))?;
    let until = Instant::now() + Duration::from_secs(10);
    let verified = loop {
        match server.verify(until) {
            Ok(receipt) => break Ok(receipt),
            Err(_) if Instant::now() < until => std::thread::sleep(Duration::from_millis(50)),
            Err(reason) => break Err(reason),
        }
    };
    match verified {
        Ok(receipt) => {
            if let Err(error) = save(&dir.join("authority.json"), &receipt) {
                let _ = cancel_prepared(dir);
                return Err(error);
            }
        }
        Err(error) => {
            let _ = cancel_prepared(dir);
            return Err(error);
        }
    }
    Ok((server, home))
}

/// R3: the host resolves authority; the operator's independent reading must agree before launch.
fn expected_matches(checkpoint: &Value, expected: &Expected) -> Res<()> {
    let authority = &checkpoint["receipt"]["prepared"]["authority"];
    if authority["profile_sha256"] != expected.profile_sha256
        || authority["executable"] != json!(expected.executable)
        || authority["executable_sha256"] != expected.executable_sha256
        || authority["account_home"] != json!(expected.account_home)
        || authority["saved"]["grant"]["by"] != expected.approver
    {
        return Err("prepared authority differs from independent readback; no native launch".into());
    }
    Ok(())
}

/// Operations started through the original adapter recover only through authenticated retained bytes.
fn started_through_taskq(dir: &Path) -> bool {
    dir.join("slot-request.json").exists()
}

fn retained(dir: &Path, cancel: bool) -> Res<Value> {
    let adapter = std::env::var_os("ASSETQ_PROTECTED_ADAPTER")
        .ok_or("ASSETQ_PROTECTED_ADAPTER is not configured; retained operation stays unknown")?;
    let host = signed_host(Path::new(&adapter))?;
    let text = |name: &str| fs::read_to_string(dir.join(name));
    let checkpoint = text("preparation.json").map_err(|_| "retained checkpoint missing; operation stays unknown")?;
    let receipt =
        text("reconciliation.json").or_else(|_| text("authority.json")).map_err(|_| "retained receipt missing; operation stays unknown")?;
    let until = Instant::now() + Duration::from_secs(5);
    let answer =
        if cancel { host.cancel_retained(&checkpoint, &receipt, until)? } else { host.reconcile_retained(&checkpoint, &receipt, until)? };
    save(&dir.join(if cancel { "retained-cancel.json" } else { "retained-reconciliation.json" }), &answer)?;
    Ok(answer)
}

/// R4: verified completion cannot stand in for the original slot's own release acknowledgment.
fn original_release_acknowledged(dir: &Path) -> Res<bool> {
    if !dir.join("slot-release-intent").exists() || !dir.join("slot-release.json").exists() {
        return Ok(false);
    }
    let (request, receipt) = (read_json(&dir.join("slot-request.json"))?, read_json(&dir.join("slot-release.json"))?);
    Ok(receipt["caller"] == request["binding"]["caller"]
        && receipt["op_id"] == request["binding"]["op_id"]
        && receipt["state"] == "released"
        && receipt["occupied"] == false)
}

fn prepared(dir: &Path) -> Res<(ProtectedHost, ProtectedPreparation)> {
    let scope = read_json(&dir.join("qualification.json"))?;
    let adapter = Path::new(scope["adapter"].as_str().ok_or("original adapter missing")?);
    let host = signed_host(adapter)?;
    let preparation = host.recover_preparation(&read_json(&dir.join("preparation.json"))?)?;
    if *preparation.request() != read_json(&dir.join("host-request.json"))?
        || preparation.request()["permit"] != read_json(&dir.join("permit.json"))?
    {
        return Err("original preparation/request/permit mismatch; no replay or release".into());
    }
    Ok((host, preparation))
}

pub(super) fn prepared_home(dir: &Path) -> Res<PathBuf> {
    let checkpoint = if started_through_taskq(dir) { read_json(&dir.join("preparation.json"))? } else { prepared(dir)?.1.checkpoint() };
    Ok(PathBuf::from(checkpoint["receipt"]["prepared"]["authority"]["account_home"].as_str().ok_or("prepared home missing")?))
}

pub(super) fn cancel_prepared(dir: &Path) -> Res<Value> {
    if started_through_taskq(dir) {
        return retained(dir, true);
    }
    let (host, preparation) = prepared(dir)?;
    let receipt = host.cancel(preparation.request(), Instant::now() + Duration::from_secs(5))?;
    save(&dir.join("host-cancel.json"), &receipt)?;
    Ok(receipt)
}

fn release_capacity(dir: &Path, capacity: &Capacity, op: &str) -> Res<()> {
    let repeated = dir.join("permit-release-intent").exists();
    if !repeated && !capacity.holds(op)? {
        return Err("protected permit held by another operation; nothing released".into());
    }
    seal(&dir.join("permit-release-intent"), &json!(true))?;
    // A repeat after a crash may find the permit already free or held by the next operation.
    if repeated && !capacity.holds(op)? {
        return Ok(());
    }
    capacity.apply(Event::Release { op: op.into() })
}

pub(super) fn release_unlaunched(dir: &Path) -> Res<()> {
    if dir.join("protected-launch").exists() {
        return Err("launch may have occurred; reconcile original preparation".into());
    }
    let (capacity, op) = capacity_of(dir)?;
    release_capacity(dir, &capacity, &op)
}

pub(super) fn reconcile_release(dir: &Path) -> Res<()> {
    if started_through_taskq(dir) {
        if retained(dir, false)?["state"] != "completed" {
            return Err("retained native/descendant completion unknown; operation stays unknown".into());
        }
        if !original_release_acknowledged(dir)? {
            return Err("retained completion verified but the original slot release was never acknowledged; Pipe has no TaskQ path; operation stays unknown".into());
        }
        return Ok(());
    }
    let (host, preparation) = prepared(dir)?;
    let (capacity, op) = capacity_of(dir)?;
    reconcile_then_release(
        dir,
        Instant::now() + Duration::from_secs(15),
        |until| host.reconcile(&preparation, until),
        || {
            capacity
                .apply(Event::CompletionVerified { op: op.clone() })
                .or_else(|e| if dir.join("permit-release-intent").exists() { Ok(()) } else { Err(e) })
        },
        || release_capacity(dir, &capacity, &op),
    )
}

fn reconcile_then_release(
    dir: &Path,
    until: Instant,
    mut reconcile: impl FnMut(Instant) -> Res<Value>,
    verified: impl FnOnce() -> Res<()>,
    release: impl FnOnce() -> Res<()>,
) -> Res<()> {
    loop {
        // The callback is AgentQ's certificate-validating reconcile, never raw host output.
        let receipt = reconcile(until)?;
        save(&dir.join("reconciliation.json"), &receipt)?;
        if receipt["state"] == "completed" {
            break;
        }
        if Instant::now() >= until {
            return Err("protected native/descendant completion unknown; retain original permit".into());
        }
        std::thread::sleep(Duration::from_millis(250).min(until.saturating_duration_since(Instant::now())));
    }
    verified()?;
    release()
}

/// Operator entry: release only what proof allows; a launch intent always needs reconciliation.
pub fn release(dir: &Path) -> Res<()> {
    if !cfg!(target_os = "macos") {
        return Err("capability_unavailable: protected adapter requires Mac".into());
    }
    if dir.join("protected-launch").exists() || started_through_taskq(dir) { reconcile_release(dir) } else { release_unlaunched(dir) }
}

/// Recovery uses the original operation; a dead consumer never authorizes replay/release.
pub fn observe(dir: &Path, cancel: bool) -> Res<Value> {
    if !cfg!(target_os = "macos") {
        return Err("capability_unavailable: protected adapter requires Mac".into());
    }
    if started_through_taskq(dir) {
        return retained(dir, cancel);
    }
    if dir.join("control.json").exists() && !dir.join("permit.json").exists() {
        return Err("original consumer retained no preparation checkpoint; operation stays unknown; no replay".into());
    }
    let (host, preparation) = prepared(dir)?;
    let until = Instant::now() + Duration::from_secs(5);
    let receipt = if cancel { host.cancel(preparation.request(), until)? } else { host.observe(preparation.request(), until)? };
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
        .ok_or("qualification deadline; retain original permit")?;
    rx.recv_timeout(left).map_err(|_| "qualification input ended/deadline; retain original permit")?
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

/// The operator drives this one real consumer over private stdio; its permit stays held
/// until `image-observe --release` verifies the original operation.
pub fn qualify(dir: &Path) -> Res<()> {
    if !cfg!(target_os = "macos") {
        return Err("capability_unavailable: composed protected qualification requires the existing approved Mac Workspace host".into());
    }
    issuer_key()?;
    let data_root = std::env::var_os("ASSETQ_DATA_ROOT").ok_or("capability_unavailable: ASSETQ_DATA_ROOT is not configured; no permit")?;
    let capacity = PathBuf::from(data_root).join("tmp/image-worker");
    fs::create_dir_all(&capacity).map_err(|e| e.to_string())?;
    let capacity = capacity.join("protected-capacity.json");
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
        &json!({"state":"awaiting-control","pid":owner.pid,"started":owner.started,"root":root,"runtime":PIN,"adapter_sha256":ADAPTER_SHA256}),
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
        let requirements: Expected = serde_json::from_value(control.clone()).map_err(|_| "invalid protected launch match requirements")?;
        save(&dir.join("control.json"), &control)?;
        save(&dir.join("qualification.json"), &json!({"adapter":requirements.adapter}))?;
        let mut log = OpenOptions::new().create_new(true).write(true).open(dir.join("native.jsonl")).map_err(|e| e.to_string())?;
        let op = format!("pipe-qualify-{}", uuid::Uuid::new_v4());
        let prepared = prepare_launch(&root, &dir, &op, &requirements.adapter, &capacity, Some(&requirements));
        if prepared.is_err() && !dir.join("protected-launch").exists() && dir.join("permit-capacity.json").exists() {
            let _ = release_unlaunched(&dir);
        }
        let (server, home) = prepared?;
        let runtime = agent::runtime("codex").ok_or("Codex runtime unavailable")?;
        let (work, cancelled) = with_run(
            server.open(None, &runtime_launch(&root, &home, until)),
            |run| {
                let messages = runtime_progress(runtime, run, &mut log)?;
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
                                catalogue_progress(runtime, run, &mut log, &id, until)?
                            } else {
                                runtime_progress(runtime, run, &mut log)?
                            };
                            emit(&json!({"events":events}))?;
                        }
                        Some("request") => {
                            let method = non_generating(&frame)?;
                            if method == "account/read" {
                                let account = runtime.account(run).map_err(|e| e.to_string())?;
                                emit(&json!({"login":format!("{:?}",account.login),"plan":account.plan}))?;
                            } else {
                                // Runtime owns the allowed catalogue reads; unsupported probes refuse before write.
                                journal(&mut log, "runtime-intent", &frame)?;
                                runtime.send(run, &json!({"method":method,"params":frame["params"]}))?;
                                let checkpoint = runtime.checkpoint(run).map_err(|e| e.to_string())?;
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
            },
            || observe(&dir, true),
        );
        let supervisor_exit = json!({"unknown":"Runtime owns transport; supervisor exit is not whole-operation completion"});
        save(
            &dir.join("cancel.json"),
            &json!({"receipt":cancelled.as_ref().ok(),"unknown":cancelled.as_ref().err(),"supervisor_exit":supervisor_exit,"permit_release":"not attempted; run image-observe --release"}),
        )?;
        let recovered = observe(&dir, false);
        journal(&mut log, "recovery", &json!({"receipt":recovered.as_ref().ok(),"unknown":recovered.as_ref().err()}))?;
        work?;
        cancelled?;
        emit(
            &json!({"state":"cancel-requested","permit_release":"not attempted","image_tool":"unknown","usage":"no model turn submitted; image usage unobserved"}),
        )
    })();
    if let Err(ref reason) = attempt {
        save(&dir.join("unknown.json"), &json!({"reason":reason,"no_replay":true,"permit_release":"not attempted"}))?;
    }
    attempt
}

#[cfg(test)]
mod tests {
    use super::*;

    // R3/R10: an unverified adapter refuses before any spool, capacity or launch effect.
    #[test]
    fn image_worker_prepare_refuses_before_any_effect() {
        let dir = std::env::temp_dir().join(format!("pipe-never-launch-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&dir).unwrap();
        let (adapter, capacity) = (dir.join("invented-adapter"), dir.join("capacity.json"));
        fs::write(&adapter, b"invented adapter bytes").unwrap();
        let error = prepare_launch(&dir, &dir, "invented-op", &adapter, &capacity, None).err().unwrap();
        let expected = if cfg!(target_os = "macos") { "protected" } else { "Mac only" };
        assert!(error.contains(expected), "{error}");
        assert!(!capacity.exists() && !dir.join("permit-capacity.json").exists() && !dir.join("protected-launch").exists());
        assert!(signed_host(&adapter).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    fn invented_issuer() -> (Vec<u8>, Vec<u8>) {
        use ring::signature::KeyPair;
        let document = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
        let public = Ed25519KeyPair::from_pkcs8(document.as_ref()).unwrap().public_key().as_ref().to_vec();
        (document.as_ref().to_vec(), public)
    }

    // R3/R7: the host's own verifier accepts the permit; a changed binding or foreign key refuses.
    #[test]
    fn image_worker_permit_verifies_under_issuer_and_binds_operation() {
        let (pkcs8, public) = invented_issuer();
        let owner = coreq::process::Identity { pid: 42, started: "invented-start".into() };
        let permit = issue_permit(&pkcs8, Path::new("/invented/root"), "invented-op", &owner, 7, 1000).unwrap();
        let binding = &permit["binding"];
        assert_eq!(binding["expires_at"], 1000 + PERMIT_SECONDS);
        assert_eq!(binding["caller"], PERMIT_CALLER);
        assert_eq!(permit["key"].as_str().unwrap().len(), 64);
        assert_eq!(
            binding.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(),
            ["schema", "caller", "op_id", "root", "runtime", "pid", "started", "boot", "expires_at"]
        );
        let payload = |binding: &Value| {
            json!({"binding":binding,"key_sha256":db::sha256_hex(permit["key"].as_str().unwrap().as_bytes())}).to_string()
        };
        let verify = |key: &[u8], text: String| {
            agentq_runtime::agent::host::verify_signature(
                &hex::encode(key),
                permit["signature"].as_str().unwrap(),
                "pipe.permits/1",
                text.as_bytes(),
            )
        };
        verify(&public, payload(binding)).unwrap();
        let mut changed = binding.clone();
        changed["op_id"] = json!("invented-other");
        assert!(verify(&public, payload(&changed)).is_err());
        assert!(verify(&invented_issuer().1, payload(binding)).is_err());
        assert!(issue_permit(b"invented bad key", Path::new("/invented/root"), "invented-op", &owner, 7, 1000).is_err());
    }

    // R3: an independent reading that disagrees with prepared authority stops before launch.
    #[test]
    fn image_worker_expected_identity_must_match_preparation() {
        let expected = || Expected {
            adapter: "/invented/adapter".into(),
            profile_sha256: "invented-profile".into(),
            executable: "/invented/native".into(),
            executable_sha256: "invented-native-hash".into(),
            account_home: "/invented/home".into(),
            approver: "invented-approver".into(),
        };
        let checkpoint = json!({"receipt":{"prepared":{"authority":{"profile_sha256":"invented-profile","executable":"/invented/native",
            "executable_sha256":"invented-native-hash","account_home":"/invented/home","saved":{"grant":{"by":"invented-approver"}}}}}});
        expected_matches(&checkpoint, &expected()).unwrap();
        for change in [
            |e: &mut Expected| e.profile_sha256 = "other".into(),
            |e: &mut Expected| e.executable = "/other".into(),
            |e: &mut Expected| e.executable_sha256 = "other".into(),
            |e: &mut Expected| e.account_home = "/other".into(),
            |e: &mut Expected| e.approver = "other".into(),
        ] {
            let mut other = expected();
            change(&mut other);
            assert!(expected_matches(&checkpoint, &other).is_err());
        }
        assert!(serde_json::from_value::<Expected>(json!({"grant":"full","adapter_sha256":"forged"})).is_err());
    }

    // R3/R4/R7: no release before AgentQ completion, and a repeat after a crash cannot free the next holder.
    #[test]
    fn image_worker_permit_release_follows_verified_completion() {
        let dir = std::env::temp_dir().join(format!("aq-permit-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&dir).unwrap();
        let capacity = Capacity(dir.join("capacity.json"));
        let op = "invented-op";
        capacity.apply(Event::Acquire { op: op.into() }).unwrap();
        capacity.apply(Event::LaunchIntended { op: op.into() }).unwrap();
        let verified = || capacity.apply(Event::CompletionVerified { op: op.into() });
        let released = || release_capacity(&dir, &capacity, op);
        assert!(reconcile_then_release(&dir, Instant::now(), |_| Ok(json!({"state":"unknown"})), verified, released).is_err());
        assert!(capacity.holds(op).unwrap() && !dir.join("permit-release-intent").exists());
        assert!(reconcile_then_release(&dir, Instant::now(), |_| Err("certificate mismatched".into()), verified, released).is_err());
        assert!(capacity.holds(op).unwrap() && !dir.join("permit-release-intent").exists());
        assert!(released().is_err(), "launch intent without completion keeps the permit");
        assert!(capacity.holds(op).unwrap());
        reconcile_then_release(&dir, Instant::now(), |_| Ok(json!({"state":"completed"})), verified, released).unwrap();
        assert!(!capacity.0.exists() && dir.join("permit-release-intent").exists());
        capacity.apply(Event::Acquire { op: "invented-next".into() }).unwrap();
        released().unwrap();
        assert!(capacity.holds("invented-next").unwrap());
        let foreign = dir.join("foreign");
        fs::create_dir(&foreign).unwrap();
        assert!(release_capacity(&foreign, &capacity, op).is_err());
        assert!(capacity.holds("invented-next").unwrap() && !foreign.join("permit-release-intent").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    // R4/R10: operations started through the original adapter never publish on unacknowledged release.
    #[test]
    fn image_worker_taskq_era_operation_needs_its_own_release_receipt() {
        let dir = std::env::temp_dir().join(format!("aq-retained-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&dir).unwrap();
        assert!(!started_through_taskq(&dir));
        save(&dir.join("slot-request.json"), &json!({"binding":{"caller":"invented","op_id":"invented-op"},"key":"invented"})).unwrap();
        assert!(started_through_taskq(&dir));
        assert!(!original_release_acknowledged(&dir).unwrap());
        save(&dir.join("slot-release.json"), &json!({"caller":"invented","op_id":"invented-op","state":"released","occupied":false}))
            .unwrap();
        assert!(!original_release_acknowledged(&dir).unwrap(), "receipt without durable intent");
        save(&dir.join("slot-release-intent"), &json!(true)).unwrap();
        assert!(original_release_acknowledged(&dir).unwrap());
        for receipt in [
            json!({"caller":"invented","op_id":"invented-op","state":"releasing","occupied":false}),
            json!({"caller":"invented","op_id":"invented-op","state":"released","occupied":true}),
            json!({"caller":"foreign","op_id":"invented-op","state":"released","occupied":false}),
        ] {
            save(&dir.join("slot-release.json"), &receipt).unwrap();
            assert!(!original_release_acknowledged(&dir).unwrap());
        }
        // An operation started through TaskQ never takes the permit path, and refuses without the operator's adapter.
        assert!(prepared_home(&dir).is_err());
        if std::env::var_os("ASSETQ_PROTECTED_ADAPTER").is_none() {
            assert!(reconcile_release(&dir).unwrap_err().contains("ASSETQ_PROTECTED_ADAPTER"));
            assert!(cancel_prepared(&dir).unwrap_err().contains("ASSETQ_PROTECTED_ADAPTER"));
        }
        assert!(release_unlaunched(&dir).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    // R3/R4/R6: dropping Run closes native stdin, so even a work error must keep
    // the original transport until host cancellation can observe the live operation.
    #[test]
    fn image_worker_cancel_readback_precedes_native_eof() {
        for outcome in ["success", "work-error", "cleanup-error"] {
            let dir = std::env::temp_dir().join(format!("assetq-cleanup-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&dir).unwrap();
            let trace = dir.join("wire.jsonl");
            let eof = PathBuf::from(format!("{}.eof", trace.display()));
            let mut launch = runtime_launch(&dir, &dir, Instant::now() + Duration::from_secs(10));
            launch.install = Some(dir.join("absent-optimizer"));
            launch.command = vec![
                if cfg!(windows) { "python" } else { "python3" }.into(),
                "-u".into(),
                Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/image-runtime.py").into_os_string(),
                trace.clone().into_os_string(),
                "chatgpt".into(),
                "openai".into(),
                "wait".into(),
            ];
            let runtime = agent::runtime("codex").unwrap();
            let (work, cleanup) = with_run(
                runtime.launch(&launch),
                |run| {
                    assert_eq!(runtime.account(run).unwrap().login, Login::Subscription);
                    if outcome == "work-error" { Err("invented work error".into()) } else { Ok(()) }
                },
                || {
                    let until = Instant::now() + Duration::from_secs(1);
                    while Instant::now() < until {
                        if eof.exists() {
                            return Err("native EOF before cancel readback".into());
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    if outcome == "cleanup-error" {
                        Err("invented cancellation unknown".into())
                    } else {
                        Ok(json!({"state":"cancel-requested"}))
                    }
                },
            );
            let until = Instant::now() + Duration::from_secs(3);
            while !eof.exists() && Instant::now() < until {
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(eof.exists(), "owned fixture input did not close after cleanup");
            remove_fixture_dir(&dir);
            assert_eq!(work.is_err(), outcome == "work-error");
            if outcome == "cleanup-error" {
                assert_eq!(cleanup.unwrap_err(), "invented cancellation unknown");
            } else {
                assert_eq!(cleanup.unwrap()["state"], "cancel-requested");
            }
        }
        let (work, cleanup) =
            with_run::<()>(Err("invented open failure".into()), |_| panic!("failed open ran work"), || Ok(json!("cleanup attempted")));
        assert_eq!(work.unwrap_err(), "invented open failure");
        assert_eq!(cleanup.unwrap(), json!("cleanup attempted"));
    }

    fn remove_fixture_dir(dir: &Path) {
        // EOF precedes Python exit; Windows can still hold the fixture's working directory.
        let until = Instant::now() + Duration::from_secs(3);
        loop {
            match fs::remove_dir_all(dir) {
                Err(e) if cfg!(windows) && e.raw_os_error() == Some(32) && Instant::now() < until => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                result => {
                    result.unwrap();
                    break;
                }
            }
        }
    }

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
            drop(run);
            drop(log);
            remove_fixture_dir(&dir);
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
