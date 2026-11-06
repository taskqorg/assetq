//! assetq#61 native non-generating qualification harness; pinned AgentQ a610cf7. One launch, never retried.
use agentq_runtime::{agent::{ProtectedHost,ProtectedPreparation},optimization::Intent};
use ring::{rand::{SecureRandom,SystemRandom},signature::Ed25519KeyPair};
use serde_json::{Value,json};
use std::{fs::{self,File,OpenOptions},io::Write,os::unix::fs::OpenOptionsExt,path::{Path,PathBuf},time::{Duration,Instant,SystemTime,UNIX_EPOCH}};
type Res<T>=Result<T,String>;
const CALLER:&str="assetq-52-qualification";
const OP:&str="pipe-52-corrected-mac-transport-a1cbb7b8";
const ROOT:&str="/Users/kirs/Projects/assetq";
const ADAPTER_SHA:&str="a1cbb7b88959532f2be28ea8d696c4cc1ae17b7c97205d6549f7156164c82a58";
const PUBLIC:&str="7f739abab8625718e2e5dbedf0deb5a98373260d3c22ecd20a1148f42bf6d584";
fn hex(b:&[u8])->String {b.iter().map(|v|format!("{v:02x}")).collect()}
fn sha(b:&[u8])->String {hex(ring::digest::digest(&ring::digest::SHA256,b).as_ref())}
fn install()->PathBuf {PathBuf::from(std::env::var("HOME").unwrap()).join(".codex/agentq-host")}
fn dir()->PathBuf {install().join("qualification/pipe-52-corrected-mac-transport-a1cbb7b8")}
/// Existing issuer and sealed-policy input: read only, never copied or printed.
fn retained()->PathBuf {install().join("qualification/assetq-52")}
fn sync_dir(p:&Path)->Res<()> {File::open(p).and_then(|f|f.sync_all()).map_err(|e|format!("dir sync: {e}"))}
/// Exclusive durable evidence: never overwritten.
fn once(name:&str,bytes:&[u8])->Res<()> {
    let p=dir().join(name);
    let mut f=OpenOptions::new().write(true).create_new(true).mode(0o600).open(&p).map_err(|e|format!("{name}: {e}"))?;
    f.write_all(bytes).and_then(|_|f.sync_all()).map_err(|e|format!("{name}: {e}"))?;sync_dir(&dir())
}
fn save(name:&str,v:&Value)->Res<()> {once(name,v.to_string().as_bytes())}
fn append(f:&mut File,v:&Value)->Res<()> {writeln!(f,"{v}").and_then(|_|f.sync_all()).map_err(|e|format!("log: {e}"))}
fn boot()->Res<i64> {
    let mut t=std::mem::MaybeUninit::<libc::timeval>::zeroed();let mut size=std::mem::size_of::<libc::timeval>();
    let mut mib=[libc::CTL_KERN,libc::KERN_BOOTTIME];
    if unsafe{libc::sysctl(mib.as_mut_ptr(),2,t.as_mut_ptr().cast(),&mut size,std::ptr::null_mut(),0)}!=0 {return Err("boot unknown".into());}
    Ok(unsafe{t.assume_init()}.tv_sec)
}
fn permit()->Res<Value> {
    let pair=Ed25519KeyPair::from_pkcs8(&fs::read(retained().join("permit-issuer.pk8")).map_err(|e|e.to_string())?).map_err(|_|"issuer invalid")?;
    let me=coreq::process::identity(std::process::id()).map_err(|_|"identity unknown")?.ok_or("identity absent")?;
    let now=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let binding=json!({"schema":"pipe.permits/1","caller":CALLER,"op_id":OP,"root":ROOT,"runtime":"codex","pid":me.pid,"started":me.started,"boot":boot()?,"expires_at":now+3600});
    let mut raw=[0u8;32];SystemRandom::new().fill(&mut raw).map_err(|_|"random failed")?;let key=hex(&raw);
    let payload=json!({"binding":binding,"key_sha256":sha(key.as_bytes())});
    let mut message=b"pipe.permits/1\0".to_vec();message.extend_from_slice(payload.to_string().as_bytes());
    Ok(json!({"binding":binding,"key":key,"signature":hex(pair.sign(&message).as_ref())}))
}
/// Independent pre-launch resolution: the exact sealed policy read locally plus the host's documented additions.
fn expected(adapter:&Path)->Res<Value> {
    let env:Value=serde_json::from_slice(&fs::read(retained().join("policy-input.json")).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    let a=&env["authority"];let install=install().canonicalize().map_err(|e|e.to_string())?;
    let bin=PathBuf::from(a["executable"].as_str().ok_or("exe")?);
    let control=install.join("native-scopes").join(&sha(format!("{CALLER}\0{OP}").as_bytes())[..32]);
    let mut profile=env["profile"].as_str().ok_or("profile")?.to_string();
    for p in [install.as_path(),adapter.parent().ok_or("parent")?,bin.parent().ok_or("parent")?] {profile.push_str(&format!("(deny file-write* (subpath {}))\n",json!(p)));}
    profile.push_str(&format!("(deny file* (subpath {}))\n",json!(control)));
    Ok(json!({"project":ROOT,"runtime":"codex","account":a["account"],"account_home":a["account_home"],"executable":a["executable"],"executable_sha256":a["executable_sha256"],
        "requested":a["requested"],"resolved":a["resolved"],"saved":a["saved"],"profile_sha256":sha(profile.as_bytes()),"policy_sha256":sha(env.to_string().as_bytes()),
        "scope_directory":control,"adapter_executable":adapter,"adapter_sha256":ADAPTER_SHA}))
}
fn compare(prepared:&Value,e:&Value)->Res<()> {
    let a=&prepared["authority"];
    let pairs=[("project",&a["project"]),("runtime",&a["runtime"]),("account",&a["account"]),("account_home",&a["account_home"]),("executable",&a["executable"]),("executable_sha256",&a["executable_sha256"]),
        ("requested",&a["requested"]),("resolved",&a["resolved"]),("saved",&a["saved"]),("profile_sha256",&a["profile_sha256"]),("policy_sha256",&a["policy_sha256"]),("scope_directory",&a["scope_directory"]),
        ("adapter_executable",&a["adapter"]["executable"]),("adapter_sha256",&a["adapter"]["sha256"])];
    for (name,seen) in pairs {if *seen!=e[name] {return Err(format!("pre-launch identity mismatch: {name}"));}}
    Ok(())
}
fn safe(r:&Value)->Value {
    let l=&r["launch"];let s=&l["scope"];
    json!({"state":r["state"],"reason":r["reason"],"launch_state":l["state"],"launch_reason":l["reason"],"frontend":l["frontend"],"supervisor":l["supervisor"],"native":l["native"],"coalition":s["binding"],
        "input_closed":l["input_closed"],"cancel_requested":l["cancel_requested"],"native_ok":l["native_ok"],"dispatch_ack":s["dispatch_ack"],"certificate":s["certificate"],
        "cleanup_intent":s["cleanup_intent"],"bootout_ack":s["bootout_ack"],"cleanup_verified":s["cleanup_verified"],"last_counts":s["last_counts"],"verified_effective":r["verified_effective"],"label":s["label"]})
}
fn say(step:&str,v:&Value) {println!("{}",json!({"step":step,"at":SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),"readback":v}));let _=std::io::stdout().flush();}
fn qualify()->Res<()> {
    for n in ["permit.json","checkpoint.json","launch-intent.json"] {if dir().join(n).exists() {return Err(format!("{n} exists; one-shot consumed, never launch again"));}}
    let adapter=install().join("stage/a1cbb7b88959532f/agentq").canonicalize().map_err(|e|e.to_string())?;
    let manifest:Value=serde_json::from_slice(&fs::read(install().join("trust.json")).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    if manifest["sha256"]!=ADAPTER_SHA||manifest["public_key"]!=PUBLIC {return Err("trust.json is not the published manifest".into());}
    let host=ProtectedHost::signed(&adapter,ADAPTER_SHA,PUBLIC,manifest["signature"].as_str().ok_or("signature")?)?;
    let e=expected(&adapter)?;
    let permit=permit()?;save("permit.json",&permit)?;
    let preparation=host.prepare(&permit,Instant::now()+Duration::from_secs(5))?;
    let checkpoint=preparation.checkpoint();save("checkpoint.json",&checkpoint)?;
    compare(&checkpoint["receipt"]["prepared"],&e)?;
    let prepared=&checkpoint["receipt"]["prepared"];
    say("prepared",&json!({"preparation":checkpoint["receipt"]["preparation"],"policy_revision":checkpoint["receipt"]["policy_revision"],"account":checkpoint["receipt"]["account"],"approver":prepared["authority"]["saved"]["grant"]["by"],"grant_mode":prepared["authority"]["saved"]["grant"]["mode"],"profile_sha256":prepared["authority"]["profile_sha256"],"adapter_sha256":prepared["authority"]["adapter"]["sha256"],"native_sha256":prepared["authority"]["executable_sha256"],"host_instance_sha256":sha(prepared["host_instance"].to_string().as_bytes()),"independent_identities":"match"}));
    // Durable one-shot intent, persisted before the single launch.
    save("launch-intent.json",&json!({"operation":OP,"caller":CALLER,"preparation":checkpoint["receipt"]["preparation"],"one_shot":true}))?;
    let log=dir().join("frontend-private.log");
    let mut checks=OpenOptions::new().create_new(true).append(true).mode(0o600).open(dir().join("readback.jsonl")).map_err(|e|e.to_string())?;
    let launched=host.launch_prepared(&preparation,Intent::Inherit,&log);
    let mut server=match launched {Ok(s)=>s,Err(reason)=>{save("launch-error.json",&json!({"reason":reason}))?;return reconcile_only(&host,&mut checks,&format!("launch error, operation retained: {reason}"));}};
    say("launched",&json!({"frontend_child":server.id()}));
    // Verified live boundary before any input.
    let until=Instant::now()+Duration::from_secs(45);
    let verified=loop {
        match server.verify(Instant::now()+Duration::from_secs(5)) {
            Ok(r)=>break Ok(r),
            Err(reason) if Instant::now()<until&&!reason.contains("frontend exited")=>{append(&mut checks,&json!({"verify_pending":reason}))?;std::thread::sleep(Duration::from_millis(50));},
            Err(reason)=>break Err(reason),
        }
    };
    let mut fault=None;
    match verified {
        Ok(receipt)=>{
            save("receipt-verified.json",&receipt)?;say("verified",&safe(&receipt));
            let init=json!({"method":"initialize","id":1,"params":{"clientInfo":{"name":"pipe61_qualification","version":"0.0.0"}}});
            match server.send(&init,Instant::now()+Duration::from_secs(10)) {
                Err(reason)=>fault=Some(format!("initialize send: {reason}")),
                Ok(())=>{
                    let until=Instant::now()+Duration::from_secs(30);
                    let mut native=OpenOptions::new().create_new(true).append(true).mode(0o600).open(dir().join("native.jsonl")).map_err(|e|e.to_string())?;
                    let mut result=None;
                    while Instant::now()<until&&result.is_none() {
                        match server.receive(Instant::now()+Duration::from_secs(1)) {
                            Ok(Some(ev))=>{append(&mut native,&ev)?;if ev["id"]==1 {result=Some(ev);}},
                            Ok(None)=>{},
                            Err(reason)=>{fault=Some(format!("initialize receive: {reason}"));break;}
                        }
                    }
                    match result {
                        Some(ev) if !ev["result"].is_null()=>{
                            say("initialized",&json!({"keys":ev["result"].as_object().map(|o|o.keys().cloned().collect::<Vec<_>>()),"user_agent":ev["result"]["userAgent"]}));
                            if let Err(reason)=server.send(&json!({"method":"initialized"}),Instant::now()+Duration::from_secs(5)) {fault=Some(format!("initialized send: {reason}"));}
                            else {
                                match server.verify(Instant::now()+Duration::from_secs(5)) {
                                    Ok(r)=>{save("receipt-post-initialize.json",&r)?;say("post-initialize",&safe(&r));},
                                    Err(reason)=>fault=Some(format!("post-initialize verify: {reason}")),
                                }
                            }
                        },
                        Some(ev)=>fault=Some(format!("initialize error: {}",ev["error"])),
                        None=>{if fault.is_none() {fault=Some("initialize response absent".into());}}
                    }
                }
            }
        },
        Err(reason)=>fault=Some(format!("verify: {reason}")),
    }
    if let Some(reason)=&fault {save("fault.json",&json!({"reason":reason}))?;say("fault",&json!({"reason":reason}));}
    // Original-operation cancellation; transport stays open until its readback completes.
    let cancel=server.cancel(Instant::now()+Duration::from_secs(5));
    save("cancel.json",&json!({"receipt":cancel.as_ref().ok(),"unknown":cancel.as_ref().err()}))?;
    say("cancel",&match &cancel {Ok(r)=>safe(r),Err(reason)=>json!({"unknown":reason})});
    let until=Instant::now()+Duration::from_secs(20);
    let request=preparation.request().clone();
    loop {
        match host.observe(&request,Instant::now()+Duration::from_secs(5)) {
            Ok(r)=>{append(&mut checks,&json!({"cancel_readback":safe(&r)}))?;if r["launch"]["input_closed"]==true {save("receipt-cancel-readback.json",&r)?;say("cancel-readback",&safe(&r));break;}},
            Err(reason)=>append(&mut checks,&json!({"cancel_readback_unknown":reason}))?,
        }
        if Instant::now()>=until {say("cancel-readback",&json!({"unknown":"input not observed closed before deadline"}));break;}
        std::thread::sleep(Duration::from_millis(100));
    }
    let outcome=reconcile_only(&host,&mut checks,"recovery from retained checkpoint");
    // Drain and close only after readback. Frontend exit is not evidence of completion.
    let end=Instant::now()+Duration::from_secs(10);
    let mut exit=None;
    while Instant::now()<end {match server.poll() {Ok(Some(c))=>{exit=Some(c);break;},Ok(None)=>std::thread::sleep(Duration::from_millis(50)),Err(_)=>break}}
    say("frontend",&json!({"exit":exit}));
    drop(server);
    outcome
}
/// Fresh recovery from the persisted checkpoint bytes only. Never launches.
fn reconcile_only(host:&ProtectedHost,checks:&mut File,why:&str)->Res<()> {
    let checkpoint:Value=serde_json::from_slice(&fs::read(dir().join("checkpoint.json")).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    let recovered:ProtectedPreparation=host.recover_preparation(&checkpoint)?;
    say("recover_preparation",&json!({"why":why,"preparation":checkpoint["receipt"]["preparation"],"matches":recovered.checkpoint()==checkpoint}));
    let until=Instant::now()+Duration::from_secs(120);let mut n=0;
    loop {
        n+=1;
        match host.reconcile(&recovered,Instant::now()+Duration::from_secs(5)) {
            Ok(r)=>{
                append(checks,&json!({"reconcile":safe(&r)}))?;
                if r["state"]=="completed" {
                    save("receipt-completed.json",&r)?;say("reconcile-completed",&json!({"attempts":n,"receipt":safe(&r)}));
                    let label=&r["launch"]["scope"]["label"];let domain=&r["launch"]["scope"]["domain"];
                    let printed=std::process::Command::new("/bin/launchctl").args(["print",&format!("{}/{}",domain.as_str().unwrap_or(""),label.as_str().unwrap_or(""))]).output().map(|o|o.status.success());
                    let f=host.reconcile(&recovered,Instant::now()+Duration::from_secs(5));
                    let o=host.observe(recovered.request(),Instant::now()+Duration::from_secs(5));
                    save("receipt-final.json",&json!({"reconcile":f.as_ref().ok(),"reconcile_unknown":f.as_ref().err(),"observe":o.as_ref().ok(),"observe_unknown":o.as_ref().err()}))?;
                    say("final-readback",&json!({"reconcile":f.as_ref().map(safe).map_err(|e|e.clone()),"observe_state":o.as_ref().map(|v|v["state"].clone()).map_err(|e|e.clone()),"launchd_job_still_loaded":printed.as_ref().ok()}));
                    return Ok(());
                }
                if Instant::now()>=until {save("receipt-unknown.json",&r)?;say("reconcile-unknown",&safe(&r));return Err("completion unproved at deadline; occupancy retained".into());}
            },
            Err(reason)=>{
                append(checks,&json!({"reconcile_unknown":reason}))?;
                if Instant::now()>=until {say("reconcile-unknown",&json!({"reason":reason}));return Err("reconcile unknown at deadline; occupancy retained".into());}
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}
/// Fresh-process recovery from retained checkpoint bytes only. No launch path.
fn recover()->Res<()> {
    let adapter=install().join("stage/a1cbb7b88959532f/agentq").canonicalize().map_err(|e|e.to_string())?;
    let manifest:Value=serde_json::from_slice(&fs::read(install().join("trust.json")).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    let host=ProtectedHost::signed(&adapter,ADAPTER_SHA,PUBLIC,manifest["signature"].as_str().ok_or("signature")?)?;
    let checkpoint:Value=serde_json::from_slice(&fs::read(dir().join("checkpoint.json")).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    let recovered=host.recover_preparation(&checkpoint)?;
    let until=|| Instant::now()+Duration::from_secs(5);
    let o=host.observe(recovered.request(),until());
    let r=host.reconcile(&recovered,until());
    save("recovery-fresh-process.json",&json!({"observe":o.as_ref().ok(),"observe_unknown":o.as_ref().err(),"reconcile":r.as_ref().ok(),"reconcile_unknown":r.as_ref().err()}))?;
    say("fresh-recovery",&json!({"checkpoint_matches":recovered.checkpoint()==checkpoint,"observe":o.as_ref().map(safe).map_err(|e|e.clone()),"reconcile":r.as_ref().map(safe).map_err(|e|e.clone())}));
    Ok(())
}
fn main() {
    let a:Vec<_>=std::env::args().skip(1).collect();
    let r=match a.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["qualify-one-shot"]=>qualify(),
        ["recover"]=>recover(),
        _=>Err("usage: a61 qualify-one-shot|recover".into()),
    };
    if let Err(reason)=r {eprintln!("refused: {reason}");std::process::exit(2);}
}
