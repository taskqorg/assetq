//! One operator-bound Meshy run. Receipts are not a second job state machine.
use crate::provider::{Output, Poll, Provider, Submit};
use crate::{
    Config, db,
    store::{Store, durable_rename},
};
use base64::Engine;
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const PROVIDER: &str = "meshy";
pub const SOURCE: &str = "fe2077d15b0c50617b84c6267277d46339c6a8cedbd6334e923cfa5475529baa";
const PRICE: u64 = 15;
const RECEIPT_LIMIT: u64 = 2 << 20;
const RENDER_RESERVE: u64 = 32 << 20;
const OUTPUT_RESERVE: u64 = 640 << 20;
type Res<T> = Result<T, String>;

pub fn recipe() -> Value {
    json!({"model_type":"smart-topology","ai_model":"meshy-t2","topology":"triangle","target_polycount":3000,
        "should_texture":true,"enable_pbr":true,"texture_resolution":"2k","target_formats":["glb"]})
}

pub fn method() -> Value {
    json!({"operation":"model.image_to_3d","inputs":["one verified original crate PNG"],
        "outputs":["model/gltf-binary","image/png","image/jpeg","application/octet-stream","application/json"],"provider":PROVIDER,
        "model":"meshy-t2","api_revision":"openapi/v1/image-to-3d;verified-2026-11-04","pipeline_revision":"2",
        "parameters":{},"recipe":recipe(),"capabilities":["meshy.image_to_3d","blender.headless","operator-bound qualified executor"],
        "limitations":["Only the operator-bound initial job may dispatch; unknown prior spend refuses; no cancellation DELETE or POST replay; owner visual acceptance separate."],
        "provenance":"docs/meshy-pipeline.md; docs/meshy-baseline.md","validation":["GLB bounds","Blender import/render","triangles/components/dimensions/UV/PBR","sha256"],
        "steps":["provider.submit","provider.import","validate"],"max_credits":PRICE,"synthetic":false})
}

pub fn preflight(blender: Option<&Path>) -> Res<Value> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let memory = native_memory_bound();
    let blender_hash =
        blender.map(|path| File::open(path).map_err(|_| "Blender executable unavailable".to_string()).and_then(sha_reader)).transpose();
    Ok(json!({"dispatch_enabled":false,"assetq_binary_sha256":sha_reader(File::open(executable).map_err(|e|e.to_string())?)?,
        "memory_limit_bytes":memory.as_ref().ok(),"memory_unknown_reason":memory.err(),
        "blender_sha256":blender_hash.as_ref().ok().and_then(|h|h.as_ref()),"blender_unknown_reason":blender_hash.err().or_else(||blender.is_none().then(||"Blender executable not supplied".into())),
        "protected_meshy_key":if std::env::var("MESHY_API_KEY").ok().is_some_and(|k|!k.is_empty()) {"present"} else {"absent"},
        "source_sha256":SOURCE,"expected_recipe":recipe(),"expected_credits":PRICE,"cumulative_cap":null,"spend_policy":"uncapped_initial_testing",
        "ledger":"unknown until independent operator reconciliation","provider_qualification":"unproved"}))
}

pub struct Meshy {
    cfg: Config,
    store: Store,
    root: PathBuf,
    binding: Option<Value>,
    key: Option<String>,
    agent: ureq::Agent,
    base: String,
}

impl Meshy {
    /// Observation can resume with the existing protected account configuration.
    /// This never enables another POST.
    pub fn observer(cfg: Config) -> Res<Self> {
        let store = Store::existing(&cfg.data_root).map_err(|e| e.to_string())?.with_bounds(
            cfg.quota_bytes,
            cfg.temp_quota_bytes,
            cfg.free_space_floor_bytes,
        );
        let root = cfg.data_root.join("tmp/meshy");
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .timeout_global(Some(Duration::from_secs(30)))
            .build()
            .into();
        Ok(Self {
            cfg,
            store,
            root,
            binding: None,
            key: std::env::var("MESHY_API_KEY").ok().filter(|s| !s.is_empty()),
            agent,
            base: "https://api.meshy.ai".into(),
        })
    }

    pub fn bound(cfg: Config, job: &str, profile: &Path) -> Res<Self> {
        // The worker prepares code/evidence; the existing operator owns secrets and qualification.
        if std::env::var_os("TASKQ_ATTEMPT").is_some() {
            return Err("operator action required: Meshy paid qualification cannot run in a TaskQ worker".into());
        }
        if cfg.max_upload_bytes == 0 || cfg.max_upload_bytes > 64 << 20 {
            return Err("bounded Meshy qualification requires a positive upload limit at most 64 MiB".into());
        }
        let mut p = Self::observer(cfg)?;
        let binding = read_json(profile)?;
        binding_check(&binding, now()?)?;
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        if binding["assetq_binary_sha256"] != sha_reader(File::open(executable).map_err(|e| e.to_string())?)? {
            return Err("AssetQ binary differs from the operator qualification candidate".into());
        }
        let memory = native_memory_bound()?;
        if binding["memory_limit_bytes"].as_u64() != Some(memory) {
            return Err("native memory readback differs from qualified executor receipt; no paid effect".into());
        }
        if binding["job_id"] != job || p.key.is_none() {
            return Err("Meshy requires the exact operator-bound job and protected MESHY_API_KEY".into());
        }
        let blender = binding["blender"].as_str().ok_or("qualified Blender executable required")?;
        let binary = File::open(blender).map_err(|_| "qualified Blender executable unavailable")?;
        let digest = sha_reader(binary)?;
        if binding["blender_sha256"] != digest {
            return Err("Blender artifact differs from operator qualification".into());
        }
        let mut c = db::connect_config(&p.cfg).map_err(|e| e.to_string())?;
        let row = c.query_one("SELECT request, method_id, method_version FROM jobs WHERE id=$1", &[&job]).map_err(|e| e.to_string())?;
        if row.get::<_, String>(1) != "model.meshy-prop.v1" || row.get::<_, i32>(2) != 2 {
            return Err("operator binding requires model.meshy-prop.v1@2".into());
        }
        p.input(&row.get::<_, Value>(0), &binding)?;
        let methods = c
            .query_one(
                "SELECT spec FROM methods WHERE id='model.meshy-prop.v1' AND version=2 AND project=(SELECT project FROM jobs WHERE id=$1)",
                &[&job],
            )
            .map_err(|e| e.to_string())?;
        if methods.get::<_, Value>(0) != method() {
            return Err("bound method differs from the compiled immutable Meshy recipe".into());
        }
        let _lock = p.store.writer_lock().map_err(|e| e.to_string())?;
        p.seal(&p.root.join("bound-job.json"), &binding)?;
        p.binding = Some(binding);
        Ok(p)
    }

    fn dir(&self, id: &str) -> Res<PathBuf> {
        if uuid::Uuid::parse_str(id).map(|x| x.to_string()).as_deref() != Ok(id) {
            return Err("invalid Meshy identity".into());
        }
        Ok(self.root.join(id))
    }

    // All receipt writes share the counted store writer lock supplied by the caller.
    fn save(&self, path: &Path, v: &Value) -> Res<()> {
        let bytes = serde_json::to_vec(v).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > RECEIPT_LIMIT || self.store.room().map_err(|e| e.to_string())? < bytes.len() as u64 + 65536 {
            return Err("store_full: Meshy receipt capacity unavailable".into());
        }
        let tmp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        let mut f = private_file(&tmp)?;
        f.write_all(&bytes).and_then(|_| f.sync_all()).map_err(|e| e.to_string())?;
        durable_rename(&tmp, path).map_err(|e| e.to_string())
    }
    fn seal(&self, path: &Path, v: &Value) -> Res<()> {
        if path.exists() {
            return if read_json(path)? == *v { Ok(()) } else { Err("Meshy binding already exists; replacement refused".into()) };
        }
        self.save(path, v)
    }

    fn http(&self, path: &str, body: Option<&Value>) -> Res<Value> {
        let auth = format!("Bearer {}", self.key.as_ref().ok_or("protected Meshy credentials unavailable; observation unknown")?);
        let result = if let Some(body) = body {
            self.agent
                .post(format!("{}{path}", self.base))
                .header("Authorization", &auth)
                .header("Content-Type", "application/json")
                .send(body.to_string())
        } else {
            self.agent.get(format!("{}{path}", self.base)).header("Authorization", &auth).call()
        };
        let mut r = result.map_err(|_| "Meshy transport failed; original operation/charge unknown")?;
        let status = r.status().as_u16();
        if !matches!(status, 200 | 201 | 202) {
            return Err(format!("Meshy HTTP {status}; response retained only by provider, outcome requires reconciliation"));
        }
        let bytes = r
            .body_mut()
            .with_config()
            .limit(RECEIPT_LIMIT)
            .read_to_vec()
            .map_err(|_| "Meshy response unavailable or above receipt bound")?;
        serde_json::from_slice(&bytes).map_err(|_| "Meshy returned invalid JSON; outcome unknown".into())
    }

    fn input(&self, request: &Value, binding: &Value) -> Res<(Vec<u8>, Value)> {
        let ids = request["attachments"].as_array().ok_or("one immutable attachment required")?;
        if ids.len() != 1
            || ids[0] != binding["source_object_id"]
            || request["constraints"] != json!({})
            || request["operation"] != "model.image_to_3d"
        {
            return Err("request differs from fixed crate recipe/binding".into());
        }
        let mut c = db::connect_config(&self.cfg).map_err(|e| e.to_string())?;
        let job = binding["job_id"].as_str().ok_or("job binding absent")?;
        let row = c.query_one("SELECT project, request FROM jobs WHERE id=$1", &[&job]).map_err(|e| e.to_string())?;
        if row.get::<_, Value>(1) != *request {
            return Err("request differs from durable job".into());
        }
        let meta =
            db::get_object(&mut c, &row.get::<_, String>(0), ids[0].as_str().ok_or("source ID absent")?).map_err(|e| e.to_string())?;
        if meta["sha256"] != SOURCE || meta["content_type"] != "image/png" || meta["synthetic"] == true {
            return Err("source is not the retained original crate".into());
        }
        let bytes = read_limit(self.store.open_object(SOURCE).map_err(|e| e.to_string())?, 20 << 20)?;
        if db::sha256_hex(&bytes) != SOURCE {
            return Err("source hash mismatch".into());
        }
        let decoded = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).map_err(|_| "source PNG decode failed")?;
        if decoded.width() != 1254 || decoded.height() != 1254 {
            return Err("source dimensions mismatch".into());
        }
        Ok((
            bytes,
            json!({"original_asset_id":"2c887345-9f66-491f-8a0c-81d81cf0c4af","local_object_id":ids[0],"sha256":SOURCE,"width":1254,"height":1254}),
        ))
    }

    fn dispatch(&self, effect: &str, request: &Value) -> Res<String> {
        let binding = self.binding.as_ref().ok_or("capability_unavailable: Meshy dispatch needs the explicit operator binding")?;
        binding_check(binding, now()?)?;
        if binding["memory_limit_bytes"].as_u64() != Some(native_memory_bound()?) {
            return Err("qualified native memory boundary changed before paid dispatch".into());
        }
        let (bytes, source) = self.input(request, binding)?;
        let mut c = db::connect_config(&self.cfg).map_err(|e| e.to_string())?;
        let row =
            c.query_one("SELECT job_id, request FROM effects WHERE id=$1 AND provider='meshy'", &[&effect]).map_err(|e| e.to_string())?;
        if row.get::<_, String>(0) != binding["job_id"].as_str().unwrap_or("") || row.get::<_, Value>(1) != *request {
            return Err("effect is not bound to this operator job".into());
        }
        // Serialize all first-workstream effects, across projects/processes, in its shared store.
        let _lock = self.store.writer_lock().map_err(|e| e.to_string())?;
        let selector = self.root.join("initial-effect.json");
        if selector.exists() {
            return Err("initial Meshy dispatch already intended; reconcile original, never POST again".into());
        }
        let others: i64 =
            c.query_one("SELECT count(*) FROM effects WHERE provider='meshy' AND id<>$1", &[&effect]).map_err(|e| e.to_string())?.get(0);
        if others != 0 {
            return Err("other Meshy effects exist; workstream ledger requires operator reconciliation".into());
        }
        let balance = self.http("/openapi/v1/balance", None)?;
        spend_check(binding["prior_credits"].as_u64(), balance["balance"].as_u64(), binding["balance"].as_u64())?;
        let dir = self.dir(effect)?;
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        self.save(
            &dir.join("preflight.json"),
            &json!({"source":source,"recipe":recipe(),"balance":balance,"binding":binding,"expected_credits":PRICE}),
        )?;
        self.reserve_outputs(&dir)?;
        binding_check(binding, now()?)?;
        let mut payload = recipe();
        payload["image_url"] = json!(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)));
        self.post_once(effect, &binding["job_id"], &payload)
    }

    fn reserve_outputs(&self, dir: &Path) -> Res<()> {
        if self.store.room().map_err(|e| e.to_string())? < 2 * OUTPUT_RESERVE {
            return Err("store_full: bounded Meshy output and temporary capacity unavailable before paid effect".into());
        }
        let file = private_file(&dir.join("output-reservation"))?;
        file.set_len(OUTPUT_RESERVE).and_then(|_| file.sync_all()).map_err(|e| e.to_string())
    }

    // Caller holds the shared workstream/store lock through POST and receipt fsync.
    fn post_once(&self, effect: &str, job: &Value, payload: &Value) -> Res<String> {
        let selector = self.root.join("initial-effect.json");
        if selector.exists() {
            return Err("initial Meshy dispatch already intended; no paid replay".into());
        }
        self.seal(&selector, &json!({"effect":effect,"job":job,"source_sha256":SOURCE}))?;
        // No network mutation precedes the durable global selector.
        let receipt = self.http("/openapi/v1/image-to-3d", Some(&payload))?;
        let op = receipt["result"].as_str().ok_or("POST receipt has no original provider ID")?;
        self.dir(op)?;
        self.save(&self.dir(effect)?.join("submitted.json"), &json!({"effect":effect,"provider_id":op,"receipt":receipt}))?;
        Ok(op.into())
    }

    fn effect_for(&self, op: &str) -> Res<String> {
        self.dir(op)?;
        let selector = read_json(&self.root.join("initial-effect.json"))?;
        let effect = selector["effect"].as_str().ok_or("effect selector missing")?;
        if read_json(&self.dir(effect)?.join("submitted.json"))?["provider_id"] != op {
            return Err("original provider ID mismatch".into());
        }
        Ok(effect.into())
    }

    fn completed(&self, op: &str) -> Res<Value> {
        let effect = self.effect_for(op)?;
        read_json(&self.dir(&effect)?.join("outputs.json"))
    }

    fn release_output_room(&self, effect: &str) -> Res<()> {
        let path = self.dir(effect)?.join("output-reservation");
        let _lock = self.store.writer_lock().map_err(|e| e.to_string())?;
        if path.is_file() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn download(&self, url: &str, ct: &str, label: &str) -> Res<Value> {
        asset_url(url)?;
        // Never send API credentials to an asset host; redirects are refused.
        let r = self.agent.get(url).call().map_err(|_| "Meshy asset transport unavailable")?;
        if r.status().as_u16() != 200 {
            return Err("Meshy asset HTTP response unavailable".into());
        }
        let mut reader = r.into_body().into_reader();
        self.retain_download(&mut reader, ct, label)
    }

    fn retain_download(&self, reader: &mut dyn Read, ct: &str, label: &str) -> Res<Value> {
        let stored = self.store.put(reader, self.cfg.max_upload_bytes, self.store.room().map_err(|e| e.to_string())?).map_err(|e| {
            if matches!(e, crate::store::StoreError::TooLarge(_)) { format!("validation failed: {e}") } else { e.to_string() }
        })?;
        let actual = if ct == "image/png" {
            let mut prefix = vec![];
            self.store.open_object(&stored.sha256).and_then(|f| f.take(8).read_to_end(&mut prefix)).map_err(|e| e.to_string())?;
            if prefix == b"\x89PNG\r\n\x1a\n" {
                "image/png"
            } else if prefix.starts_with(&[0xff, 0xd8, 0xff]) {
                "image/jpeg"
            } else {
                "application/octet-stream"
            }
        } else {
            ct
        };
        Ok(json!({"sha256":stored.sha256,"size":stored.size,"content_type":actual,"role":label}))
    }

    fn prepare_outputs(&self, op: &str, task: &Value) -> Res<Value> {
        let effect = self.effect_for(op)?;
        let dir = self.dir(&effect)?;
        let (urls, mut shape_problem) = match output_urls(task) {
            Ok(urls) => (urls, None),
            Err(reason) => {
                // A malformed completion is definitive. Retain any valid original GLB and publish a failed validation report.
                let urls = task["model_urls"]["glb"]
                    .as_str()
                    .filter(|u| asset_url(u).is_ok())
                    .map(|u| vec![(u.into(), "model/gltf-binary".into(), "original-glb".into())])
                    .unwrap_or_default();
                (urls, Some(reason))
            }
        };
        let mut outputs = vec![];
        for (i, (url, ct, label)) in urls.iter().enumerate() {
            let path = dir.join(format!("download-{i}.json"));
            let item = if path.exists() {
                read_json(&path)?
            } else {
                let item = match self.download(url, ct, label) {
                    Ok(item) => item,
                    Err(reason) if reason.starts_with("validation failed:") => {
                        shape_problem = Some(reason);
                        break;
                    }
                    Err(reason) => return Err(reason),
                };
                let _lock = self.store.writer_lock().map_err(|e| e.to_string())?;
                self.save(&path, &item)?;
                item
            };
            self.store
                .verify(item["sha256"].as_str().ok_or("download hash missing")?, item["size"].as_u64().ok_or("download size missing")?)
                .map_err(|e| e.to_string())?;
            outputs.push(item);
        }
        let report = if let Some(reason) = shape_problem {
            json!({"valid":false,"reason":reason,"views":[]})
        } else {
            match self.blender(&dir, &outputs) {
                Ok(v) => v,
                Err(reason) if reason.starts_with("store_full:") || reason.starts_with("validation_unknown:") => return Err(reason),
                Err(reason) => json!({"valid":false,"reason":reason,"views":[]}),
            }
        };
        for name in ["view-front.png", "view-back.png", "view-side.png"] {
            let path = dir.join(name);
            if report["valid"] == true && path.exists() {
                let mut f = File::open(&path).map_err(|e| e.to_string())?;
                let stored = self.store.put(&mut f, 4 << 20, self.store.room().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
                outputs.push(json!({"sha256":stored.sha256,"size":stored.size,"content_type":"image/png","role":name}));
            }
        }
        let preflight = read_json(&dir.join("preflight.json")).unwrap_or(Value::Null);
        let prior = preflight["binding"]["prior_credits"].as_u64();
        let credits = task["consumed_credits"].as_u64();
        let evidence = json!({"provider":PROVIDER,"provider_id":op,"source_sha256":SOURCE,"recipe":recipe(),
            "consumed_credits":task.get("consumed_credits"),"texture_model":task.get("texture_model"),
            "ledger":{"cap":null,"spend_policy":"uncapped_initial_testing","prior_credits":prior,"expected_credits":PRICE,"actual_credits":credits,
                "cumulative_credits":prior.zip(credits).and_then(|(a,b)|a.checked_add(b)),"balance_before":preflight["balance"]["balance"],
                "balance_after":"unknown until independent account reconciliation","no_purchases":true},
            "texture_model_unknown_reason":"unreported provider defaults are not inferred","validation":report,
            "validator_sha256":db::sha256_hex(include_bytes!("../scripts/validate_meshy.py")),"outputs":outputs});
        let mut bytes = std::io::Cursor::new(serde_json::to_vec(&evidence).map_err(|e| e.to_string())?);
        let stored = self.store.put(&mut bytes, RECEIPT_LIMIT, self.store.room().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        outputs
            .push(json!({"sha256":stored.sha256,"size":stored.size,"content_type":"application/json","role":"validation-and-provenance"}));
        let result = json!({"outputs":outputs,"evidence":evidence});
        let _lock = self.store.writer_lock().map_err(|e| e.to_string())?;
        self.save(&dir.join("outputs.json"), &result)?;
        // Only our capacity placeholder is released; retained source/output bytes remain charged.
        if dir.join("output-reservation").is_file() {
            fs::remove_file(dir.join("output-reservation")).map_err(|e| e.to_string())?;
        }
        Ok(result)
    }

    fn blender(&self, dir: &Path, outputs: &[Value]) -> Res<Value> {
        if let Ok(report) = read_json(&dir.join("validation.json")) {
            return Ok(report);
        }
        validation_may_start(dir)?;
        let binding = read_json(&self.root.join("bound-job.json"))?;
        if native_memory_bound().ok() != binding["memory_limit_bytes"].as_u64() || binding["memory_limit_bytes"].is_null() {
            return Err("validation_unknown: qualified inherited native memory boundary unavailable; no child launch".into());
        }
        let executable = binding["blender"].as_str().ok_or("qualified Blender unavailable after restart")?;
        if sha_reader(File::open(executable).map_err(|_| "qualified Blender unavailable")?)?
            != binding["blender_sha256"].as_str().unwrap_or("")
        {
            return Err("Blender artifact changed".into());
        }
        let sha = outputs.first().and_then(|v| v["sha256"].as_str()).ok_or("source GLB missing")?;
        let glb_path = self.cfg.data_root.join("objects").join(&sha[..2]).join(&sha[2..4]).join(sha);
        let glb = read_limit(self.store.open_object(sha).map_err(|e| e.to_string())?, self.cfg.max_upload_bytes)?;
        glb_check(&glb)?;
        for output in outputs.iter().filter(|v| v["role"].as_str().is_some_and(|r| r.starts_with("texture-"))) {
            let bytes = read_limit(
                self.store.open_object(output["sha256"].as_str().ok_or("texture hash absent")?).map_err(|e| e.to_string())?,
                32 << 20,
            )?;
            if texture_dimensions(&bytes)? != (2048, 2048) {
                return Err("original texture dimensions differ from 2048-by-2048 recipe".into());
            }
        }
        let _lock = self.store.writer_lock().map_err(|e| e.to_string())?;
        if self.store.room().map_err(|e| e.to_string())? < 2 * RENDER_RESERVE {
            return Err("store_full: bounded Blender workspace unavailable".into());
        }
        let inputs:Vec<Value>=outputs.iter().map(|o| {
            let hash=o["sha256"].as_str().unwrap();
            json!({"role":o["role"],"sha256":hash,"path":self.cfg.data_root.join("objects").join(&hash[..2]).join(&hash[2..4]).join(hash)})
        }).collect();
        self.save(&dir.join("inputs.json"), &json!(inputs))?;
        let script = dir.join("validate.py");
        if script.exists() {
            if fs::read(&script).map_err(|e| e.to_string())? != include_bytes!("../scripts/validate_meshy.py") {
                return Err("validator changed after interruption".into());
            }
        } else {
            let mut f = private_file(&script)?;
            f.write_all(include_bytes!("../scripts/validate_meshy.py")).and_then(|_| f.sync_all()).map_err(|e| e.to_string())?;
        }
        let attempt = uuid::Uuid::new_v4().to_string();
        let start_receipt = json!({"attempt":attempt,"source_sha256":sha,"validator_sha256":db::sha256_hex(include_bytes!("../scripts/validate_meshy.py"))});
        self.save(&dir.join(format!("validation-start-{attempt}.json")), &start_receipt)?;
        self.save(&dir.join("validation-started.json"), &start_receipt)?;
        let mut command = Command::new(executable);
        command.env_clear();
        command.env("MALLOC_ARENA_MAX", "2").env("OIDN_NUM_THREADS", "2");
        for name in ["PATH", "SYSTEMROOT", "WINDIR", "TEMP", "TMP"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let child = command
            .args(["--background", "--factory-startup", "--disable-autoexec", "--python-exit-code", "1", "--python"])
            .arg(&script)
            .arg("--")
            .arg(&glb_path)
            .arg(dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        let mut child = match child {
            Ok(child) => child,
            Err(_) => {
                let report = json!({"valid":false,"reason":"headless Blender could not start; no native process","views":[]});
                self.save(&dir.join("validation.json"), &report)?;
                return Ok(report);
            }
        };
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().map_err(|_| "validation_unknown: Blender exit readback failed")? {
                if !status.success() {
                    let report = json!({"valid":false,"reason":"headless Blender refused the source GLB; child exit confirmed","views":[]});
                    self.save(&dir.join("validation.json"), &report)?;
                    return Ok(report);
                }
                break;
            }
            if start.elapsed() > Duration::from_secs(120) {
                child.kill().map_err(|_| "validation_unknown: Blender stop unconfirmed")?;
                child.wait().map_err(|_| "validation_unknown: Blender exit unconfirmed")?;
                let report = json!({"valid":false,"reason":"headless Blender exceeded 120-second validation budget; child stop confirmed","views":[]});
                self.save(&dir.join("validation.json"), &report)?;
                return Ok(report);
            }
            if self.store.room().is_err() || self.store.room().is_ok_and(|n| n < RENDER_RESERVE) {
                child.kill().map_err(|_| "validation_unknown: Blender stop unconfirmed")?;
                child.wait().map_err(|_| "validation_unknown: Blender exit unconfirmed")?;
                self.save(
                    &dir.join(format!("validation-stop-{attempt}.json")),
                    &json!({"attempt":attempt,"exit_confirmed":true,"reason":"storage capacity"}),
                )?;
                return Err("store_full: Blender stopped after storage capacity observation failed".into());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        read_json(&dir.join("validation.json"))
    }
}

impl Provider for Meshy {
    fn name(&self) -> &str {
        PROVIDER
    }
    fn submit(&self, effect: &str, request: &Value) -> Submit {
        match self.dispatch(effect, request) {
            Ok(op) => Submit::Accepted(op),
            Err(reason)
                if self.root.join("initial-effect.json").exists()
                    && read_json(&self.root.join("initial-effect.json")).ok().is_none_or(|v| v["effect"] == effect) =>
            {
                Submit::Uncertain(reason)
            }
            Err(reason) => match self.release_output_room(effect) {
                Ok(()) => Submit::Refused(reason),
                Err(_) => Submit::Refused(format!("{reason}; unused capacity-placeholder cleanup unknown")),
            },
        }
    }
    fn lookup(&self, effect: &str) -> Res<Option<String>> {
        let receipt = read_json(&self.dir(effect)?.join("submitted.json"))?;
        let op = receipt["provider_id"].as_str().ok_or("original POST receipt missing; no sealed absence or paid replay")?;
        self.effect_for(op)?;
        Ok(Some(op.into()))
    }
    fn poll(&self, op: &str) -> Res<Poll> {
        self.effect_for(op)?;
        if let Ok(result) = self.completed(op) {
            for item in result["outputs"].as_array().ok_or("completed output list invalid")? {
                self.store
                    .verify(item["sha256"].as_str().ok_or("completed hash absent")?, item["size"].as_u64().ok_or("completed size absent")?)
                    .map_err(|e| e.to_string())?;
            }
            self.release_output_room(&self.effect_for(op)?)?;
            return Ok(Poll::Done {
                outputs: result["outputs"].as_array().unwrap().len(),
                credits: result["evidence"]["consumed_credits"].as_u64(),
            });
        }
        let task = self.http(&format!("/openapi/v1/image-to-3d/{op}"), None)?;
        if task["id"] != op {
            return Err("Meshy task identity mismatch".into());
        }
        let effect = self.effect_for(op)?;
        {
            let _lock = self.store.writer_lock().map_err(|e| e.to_string())?;
            self.save(&self.dir(&effect)?.join("observed.json"), &task)?;
        }
        let credits = task["consumed_credits"].as_u64();
        match task["status"].as_str() {
            Some("PENDING" | "IN_PROGRESS") => Ok(Poll::Pending),
            Some("SUCCEEDED") => {
                let outputs = if let Ok(v) = self.completed(op) { v } else { self.prepare_outputs(op, &task)? };
                Ok(Poll::Done { outputs: outputs["outputs"].as_array().ok_or("outputs receipt invalid")?.len(), credits })
            }
            Some("FAILED" | "CANCELED") => {
                self.release_output_room(&effect)?;
                Ok(Poll::Failed {
                    reason: format!("Meshy terminal status {}; provider details retained privately", task["status"]),
                    credits,
                })
            }
            _ => Err("unknown Meshy status; operation and usage retained".into()),
        }
    }
    fn fetch(&self, op: &str, index: usize) -> Res<Output> {
        let result = self.completed(op)?;
        let item = result["outputs"].get(index).ok_or("output index absent")?;
        let sha = item["sha256"].as_str().ok_or("output hash absent")?;
        self.store.verify(sha, item["size"].as_u64().ok_or("output size absent")?).map_err(|e| e.to_string())?;
        Ok(Output {
            content_type: item["content_type"].as_str().ok_or("output type absent")?.into(),
            reader: Box::new(self.store.open_object(sha).map_err(|e| e.to_string())?),
            evidence: json!({"role":item["role"],"source_sha256":SOURCE,"recipe":recipe(),"provider_id":op,"consumed_credits":result["evidence"]["consumed_credits"],"validation":result["evidence"]["validation"]}),
        })
    }
    fn cancel(&self, op: &str) -> Res<bool> {
        self.effect_for(op)?;
        Ok(false)
    }
    fn validate(&self, op: &str) -> Res<()> {
        let result = self.completed(op)?;
        if result["evidence"]["validation"]["valid"] == true {
            Ok(())
        } else {
            Err(format!("Meshy output validation failed: {}", result["evidence"]["validation"]["reason"]))
        }
    }
}

fn private_file(path: &Path) -> Res<File> {
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(path).map_err(|e| e.to_string())
}
fn read_limit(mut reader: impl Read, limit: u64) -> Res<Vec<u8>> {
    let mut bytes = vec![];
    reader.by_ref().take(limit + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit { Err("file exceeds declared limit".into()) } else { Ok(bytes) }
}
fn read_json(path: &Path) -> Res<Value> {
    serde_json::from_slice(&read_limit(File::open(path).map_err(|_| "Meshy receipt unavailable; outcome unknown")?, RECEIPT_LIMIT)?)
        .map_err(|_| "Meshy receipt invalid; outcome unknown".into())
}
fn sha_reader(mut reader: impl Read) -> Res<String> {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    let mut buf = [0; 65536];
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(hex::encode(hash.finalize()))
}
fn now() -> Res<u64> {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).map_err(|e| e.to_string())
}
fn binding_check(v: &Value, time: u64) -> Res<()> {
    let at = v["verified_unix_seconds"].as_u64().ok_or("fresh operator verification required")?;
    if at > time
        || time - at > 300
        || v["all_attempts_reconciled"] != true
        || v["exclusive_workstream"] != true
        || v["source_sha256"] != SOURCE
    {
        return Err("current qualified executor, exclusive workstream and reconciled ledger required; no dispatch".into());
    }
    for field in
        ["executor_evidence_sha256", "ledger_evidence_sha256", "official_api_evidence_sha256", "blender_sha256", "assetq_binary_sha256"]
    {
        if !v[field].as_str().is_some_and(|s| s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit())) {
            return Err(format!("operator {field} evidence required"));
        }
    }
    spend_check(v["prior_credits"].as_u64(), v["balance"].as_u64(), v["balance"].as_u64())
}
fn spend_check(prior: Option<u64>, balance: Option<u64>, expected: Option<u64>) -> Res<()> {
    let (Some(prior), Some(balance), Some(expected)) = (prior, balance, expected) else {
        return Err("Meshy prior spend or balance unknown; no paid effect".into());
    };
    if prior.checked_add(PRICE).is_none() || balance < PRICE || balance != expected {
        return Err("Meshy ledger overflow, insufficient balance or balance changed; reconcile before spend".into());
    }
    Ok(())
}
fn asset_url(url: &str) -> Res<()> {
    let uri: ureq::http::Uri = url.parse().map_err(|_| "invalid provider asset URL")?;
    if uri.scheme_str() != Some("https")
        || uri.host() != Some("assets.meshy.ai")
        || uri.port_u16().is_some_and(|p| p != 443)
        || uri.authority().is_some_and(|a| a.as_str().contains('@'))
    {
        return Err("provider asset URL outside the qualified HTTPS host; no credential forwarding".into());
    }
    Ok(())
}
fn output_urls(task: &Value) -> Res<Vec<(String, String, String)>> {
    let glb = task["model_urls"]["glb"].as_str().ok_or("completed task missing original GLB")?;
    let mut out: Vec<(String, String, String)> = vec![(glb.into(), "model/gltf-binary".into(), "original-glb".into())];
    let sets = task["texture_urls"].as_array().ok_or("completed task missing texture maps")?;
    for (i, set) in sets.iter().enumerate() {
        let maps = set.as_object().ok_or("invalid texture map set")?;
        for (role, url) in maps {
            let url = url.as_str().ok_or("invalid texture locator")?;
            if url.is_empty() {
                continue;
            }
            if !["base_color", "metallic", "roughness", "normal", "emission"].contains(&role.as_str()) {
                return Err("unrecognized texture role; no silent omission".into());
            }
            out.push((url.into(), "image/png".into(), format!("texture-{i}-{role}")));
        }
    }
    if !(2..=9).contains(&out.len()) {
        return Err("texture count outside bounded pipeline".into());
    }
    for (url, _, _) in &out {
        asset_url(url)?;
    }
    Ok(out)
}
fn glb_check(bytes: &[u8]) -> Res<()> {
    if bytes.len() < 20
        || &bytes[..4] != b"glTF"
        || u32::from_le_bytes(bytes[4..8].try_into().unwrap()) != 2
        || u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize != bytes.len()
        || &bytes[16..20] != b"JSON"
    {
        return Err("invalid GLB v2 container".into());
    }
    let len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    if len > RECEIPT_LIMIT as usize || len % 4 != 0 {
        return Err("GLB JSON exceeds bounded aligned chunk".into());
    }
    let chunk = bytes.get(20..20 + len).ok_or("GLB JSON out of bounds")?;
    let v: Value = serde_json::from_slice(chunk).map_err(|_| "GLB JSON invalid")?;
    for kind in ["buffers", "images"] {
        for entry in v[kind].as_array().into_iter().flatten() {
            if entry.get("uri").is_some() {
                return Err("external GLB resources refused before Blender import".into());
            }
        }
    }
    let mut position = 20 + len;
    let mut binary = 0;
    let mut binary_start = 0;
    while position < bytes.len() {
        let header = bytes.get(position..position + 8).ok_or("GLB chunk header truncated")?;
        let length = u32::from_le_bytes(header[..4].try_into().unwrap()) as usize;
        if length % 4 != 0 || &header[4..8] != b"BIN\0" || binary != 0 {
            return Err("unsupported or unaligned GLB chunk".into());
        }
        binary_start = position + 8;
        position = position.checked_add(8 + length).filter(|n| *n <= bytes.len()).ok_or("GLB binary chunk out of bounds")?;
        binary = length;
    }
    let buffers = v["buffers"].as_array().ok_or("GLB buffers absent")?;
    if buffers.len() != 1 || buffers[0]["byteLength"].as_u64().is_none_or(|n| n > binary as u64) {
        return Err("GLB buffer exceeds retained binary".into());
    }
    for view in v["bufferViews"].as_array().into_iter().flatten() {
        if view["buffer"] != 0
            || view["byteLength"]
                .as_u64()
                .zip(view["byteOffset"].as_u64().or(Some(0)))
                .and_then(|(length, offset)| length.checked_add(offset))
                .is_none_or(|n| n > binary as u64)
        {
            return Err("GLB buffer view out of bounds".into());
        }
    }
    let mut count = 0u64;
    if v["nodes"].as_array().is_some_and(|n| n.len() > 64) || v["scenes"].as_array().is_some_and(|n| n.len() > 4) {
        return Err("GLB scene instances exceed native safety bounds".into());
    }
    let images = v["images"].as_array().cloned().unwrap_or_default();
    if images.len() > 8 {
        return Err("GLB embedded image count exceeds native safety bound".into());
    }
    for image in images {
        let index = image["bufferView"].as_u64().ok_or("GLB image must be embedded in a buffer view")? as usize;
        let view = v["bufferViews"].get(index).ok_or("GLB image view absent")?;
        let start = view["byteOffset"].as_u64().unwrap_or(0) as usize + binary_start;
        let size = view["byteLength"].as_u64().ok_or("GLB image length absent")? as usize;
        if texture_dimensions(bytes.get(start..start + size).ok_or("GLB image bounds invalid")?)? != (2048, 2048) {
            return Err("GLB embedded texture dimensions differ from 2048-by-2048 recipe".into());
        }
    }
    for accessor in v["accessors"].as_array().into_iter().flatten() {
        count = count.checked_add(accessor["count"].as_u64().ok_or("GLB accessor count absent")?).ok_or("GLB accessor count overflow")?;
    }
    if count > 2_000_000 || v["meshes"].as_array().is_none_or(|m| m.is_empty() || m.len() > 64) {
        return Err("GLB geometry exceeds native safety bounds".into());
    }
    Ok(())
}

fn texture_dimensions(bytes: &[u8]) -> Res<(u32, u32)> {
    if bytes.len() >= 24 && &bytes[..8] == b"\x89PNG\r\n\x1a\n" && &bytes[12..16] == b"IHDR" {
        return Ok((u32::from_be_bytes(bytes[16..20].try_into().unwrap()), u32::from_be_bytes(bytes[20..24].try_into().unwrap())));
    }
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return Err("texture is not a bounded PNG/JPEG".into());
    }
    let mut position = 2;
    while position + 4 <= bytes.len() {
        if bytes[position] != 0xff {
            return Err("invalid JPEG marker".into());
        }
        let marker = bytes[position + 1];
        if marker == 0xff {
            position += 1;
            continue;
        }
        if marker == 0xda || marker == 0xd9 {
            break;
        }
        if marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            position += 2;
            continue;
        }
        let length = u16::from_be_bytes(bytes[position + 2..position + 4].try_into().unwrap()) as usize;
        if length < 2 || position + 2 + length > bytes.len() {
            return Err("JPEG segment out of bounds".into());
        }
        if [0xc0, 0xc1, 0xc2].contains(&marker) {
            if length < 8 {
                return Err("JPEG frame truncated".into());
            }
            return Ok((
                u16::from_be_bytes(bytes[position + 7..position + 9].try_into().unwrap()) as u32,
                u16::from_be_bytes(bytes[position + 5..position + 7].try_into().unwrap()) as u32,
            ));
        }
        position += 2 + length;
    }
    Err("JPEG dimensions unavailable before native decode".into())
}

// Read the existing executor boundary; never install or change host limits here.
fn native_memory_bound() -> Res<u64> {
    #[cfg(unix)]
    let bound = {
        let mut limit = std::mem::MaybeUninit::<libc::rlimit>::uninit();
        if unsafe { libc::getrlimit(libc::RLIMIT_AS, limit.as_mut_ptr()) } != 0 {
            return Err("native memory boundary readback unavailable".into());
        }
        unsafe { limit.assume_init() }.rlim_cur as u64
    };
    #[cfg(windows)]
    let bound = {
        let mut bound = u64::MAX;
        use windows_sys::Win32::System::JobObjects::*;
        let mut limits = std::mem::MaybeUninit::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>::zeroed();
        if unsafe {
            QueryInformationJobObject(
                std::ptr::null_mut(),
                JobObjectExtendedLimitInformation,
                limits.as_mut_ptr().cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err("native memory boundary readback unavailable; qualified Windows job required".into());
        }
        let limits = unsafe { limits.assume_init() };
        if limits.BasicLimitInformation.LimitFlags & JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK != 0 {
            return Err("native memory boundary permits silent child breakaway; no qualified inheritance".into());
        }
        if limits.BasicLimitInformation.LimitFlags & JOB_OBJECT_LIMIT_PROCESS_MEMORY != 0 {
            bound = bound.min(limits.ProcessMemoryLimit as u64);
        }
        if limits.BasicLimitInformation.LimitFlags & JOB_OBJECT_LIMIT_JOB_MEMORY != 0 {
            bound = bound.min(limits.JobMemoryLimit as u64);
        }
        bound
    };
    #[cfg(not(any(unix, windows)))]
    let bound = u64::MAX;
    if !(512 << 20..=2 << 30).contains(&bound) {
        return Err("qualified inherited native memory boundary must be 512 MiB..2 GiB; no limit is changed".into());
    }
    Ok(bound)
}

fn validation_may_start(dir: &Path) -> Res<()> {
    if !dir.join("validation-started.json").exists() {
        return Ok(());
    }
    let started = read_json(&dir.join("validation-started.json"))?;
    let attempt = started["attempt"].as_str().ok_or("validation_unknown: Blender attempt identity missing")?;
    if uuid::Uuid::parse_str(attempt).map(|id| id.to_string()).as_deref() != Ok(attempt) {
        return Err("validation_unknown: invalid local attempt identity".into());
    }
    if read_json(&dir.join(format!("validation-stop-{attempt}.json")))
        .ok()
        .is_none_or(|v| v["exit_confirmed"] != true || v["attempt"] != attempt)
    {
        return Err("validation_unknown: retained Blender execution has no completion report; no replacement".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Meshy {
        let root = std::env::temp_dir().join(format!("aq-meshy-{}", uuid::Uuid::new_v4()));
        Store::open(&root).unwrap();
        let cfg = Config {
            database_url: "unused-invented-fixture".into(),
            data_root: root,
            quota_bytes: 1 << 30,
            max_upload_bytes: 64 << 20,
            temp_quota_bytes: 1 << 30,
            metadata_quota_bytes: 1 << 30,
            free_space_floor_bytes: 0,
            metadata_admission_headroom_bytes: 4 << 20,
            metadata_recovery_headroom_bytes: 16 << 20,
            max_execution_ms: 600000,
            content_types: vec![],
            executors: 1,
            poll_ms: 0,
            lease_ms: 60000,
            retention_days: 7,
            fake_provider_dir: None,
            image_worker_root: None,
        };
        let mut p = Meshy::observer(cfg).unwrap();
        p.key = Some("invented-not-a-real-api-key".into());
        p
    }
    fn seed(p: &Meshy, effect: &str, op: &str) {
        let _lock = p.store.writer_lock().unwrap();
        fs::create_dir_all(p.dir(effect).unwrap()).unwrap();
        p.seal(&p.root.join("initial-effect.json"), &json!({"effect":effect,"job":"invented"})).unwrap();
        p.save(&p.dir(effect).unwrap().join("submitted.json"), &json!({"effect":effect,"provider_id":op})).unwrap();
    }
    fn glb(v: Value) -> Vec<u8> {
        let mut json = serde_json::to_vec(&v).unwrap();
        while json.len() % 4 != 0 {
            json.push(b' ');
        }
        let length = (28 + json.len()) as u32;
        let mut bytes = b"glTF".to_vec();
        bytes.extend(2u32.to_le_bytes());
        bytes.extend(length.to_le_bytes());
        bytes.extend((json.len() as u32).to_le_bytes());
        bytes.extend(b"JSON");
        bytes.extend(json);
        bytes.extend(0u32.to_le_bytes());
        bytes.extend(b"BIN\0");
        bytes
    }

    // A2/A6, R3/R7: absent/stale evidence and unknown spend must never authorize a paid call.
    #[test]
    fn meshy_recipe_spend_and_input_boundaries() {
        assert_eq!(
            recipe(),
            json!({"ai_model":"meshy-t2","model_type":"smart-topology","topology":"triangle","target_polycount":3000,
            "should_texture":true,"enable_pbr":true,"texture_resolution":"2k","target_formats":["glb"]})
        );
        assert!(spend_check(Some(985), Some(15), Some(15)).is_ok());
        assert!(spend_check(Some(5000), Some(15), Some(15)).is_ok(), "A6 permits reconciled testing beyond the old cap");
        for (prior, balance, expected) in [
            (None, Some(1000), Some(1000)),
            (Some(0), None, None),
            (Some(u64::MAX), Some(1000), Some(1000)),
            (Some(0), Some(14), Some(14)),
            (Some(0), Some(99), Some(100)),
        ] {
            assert!(spend_check(prior, balance, expected).is_err());
        }
        assert!(binding_check(&json!({}), 1000).is_err());
        let mut binding = json!({"verified_unix_seconds":1000,"all_attempts_reconciled":true,"exclusive_workstream":true,
            "source_sha256":SOURCE,"prior_credits":0,"balance":1000,"executor_evidence_sha256":"a".repeat(64),
            "ledger_evidence_sha256":"b".repeat(64),"official_api_evidence_sha256":"c".repeat(64),"blender_sha256":"d".repeat(64),"assetq_binary_sha256":"e".repeat(64)});
        assert!(binding_check(&binding, 1300).is_ok());
        assert!(binding_check(&binding, 1301).is_err());
        assert!(binding_check(&binding, 999).is_err());
        binding["all_attempts_reconciled"] = json!(false);
        assert!(binding_check(&binding, 1000).is_err());
        assert!(asset_url("https://assets.meshy.ai/file.glb?invented=1").is_ok());
        for url in [
            "http://assets.meshy.ai/file",
            "https://assets.meshy.ai.evil.invalid/file",
            "https://evil.invalid/file",
            "https://assets.meshy.ai:444/file",
            "https://user@assets.meshy.ai/file",
        ] {
            assert!(asset_url(url).is_err());
        }
        let good = json!({"asset":{"version":"2.0"},"buffers":[{"byteLength":0}],"meshes":[{}]});
        assert!(glb_check(&glb(good.clone())).is_ok());
        let mut foreign = good.clone();
        foreign["images"] = json!([{"uri":"file:///invented-private-file"}]);
        assert!(glb_check(&glb(foreign)).is_err());
        let mut huge = good;
        huge["accessors"] = json!([{"count":2_000_001}]);
        assert!(glb_check(&glb(huge)).is_err());
        assert!(glb_check(b"not a GLB").is_err());
    }

    // A3, R4/R5: Meshy's HTTP 202 acceptance must retain the original ID across restart without a second POST.
    #[test]
    fn meshy_accepted_post_retains_original_id_after_restart() {
        for status in [200, 201, 202] {
            let mut p = fixture();
            let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
            p.base = format!("http://{}", server.server_addr());
            let effect = uuid::Uuid::new_v4().to_string();
            let op = uuid::Uuid::new_v4().to_string();
            fs::create_dir_all(p.dir(&effect).unwrap()).unwrap();
            let reply = json!({"result":op});
            let thread = std::thread::spawn(move || {
                let req = server.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
                assert_eq!(req.method(), &tiny_http::Method::Post);
                assert_eq!(req.url(), "/openapi/v1/image-to-3d");
                req.respond(tiny_http::Response::from_string(reply.to_string()).with_status_code(status)).unwrap();
                server
            });
            let accepted = {
                let _lock = p.store.writer_lock().unwrap();
                p.post_once(&effect, &json!("invented-job"), &recipe())
            };
            let server = thread.join().unwrap();
            assert_eq!(accepted.unwrap(), op, "HTTP {status} must preserve provider acceptance");
            let restarted = Meshy::observer(p.cfg.clone()).unwrap();
            assert_eq!(restarted.lookup(&effect).unwrap(), Some(op));
            {
                let _lock = restarted.store.writer_lock().unwrap();
                assert!(restarted.post_once(&effect, &json!("invented-job"), &recipe()).is_err());
            }
            assert!(server.recv_timeout(Duration::from_millis(50)).unwrap().is_none(), "accepted POST cannot replay");
            fs::remove_dir_all(&p.cfg.data_root).unwrap();
        }
    }

    // A3, R4/R6: lost POST reply and failed receipt persistence both preserve uncertainty across restart.
    #[test]
    fn meshy_post_once_survives_lost_reply_and_receipt_failure() {
        for receipt_failure in [false, true] {
            let mut p = fixture();
            let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
            p.base = format!("http://{}", server.server_addr());
            let effect = uuid::Uuid::new_v4().to_string();
            let op = uuid::Uuid::new_v4().to_string();
            let dir = p.dir(&effect).unwrap();
            fs::create_dir_all(&dir).unwrap();
            if receipt_failure {
                fs::create_dir(dir.join("submitted.json")).unwrap();
            }
            let reply = if receipt_failure { json!({"result":op}).to_string() } else { "lost-success-body".into() };
            let thread = std::thread::spawn(move || {
                let mut req = server.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
                assert_eq!(req.method(), &tiny_http::Method::Post);
                assert_eq!(req.url(), "/openapi/v1/image-to-3d");
                let mut body = String::new();
                req.as_reader().read_to_string(&mut body).unwrap();
                assert_eq!(serde_json::from_str::<Value>(&body).unwrap(), recipe());
                req.respond(tiny_http::Response::from_string(reply)).unwrap();
                server
            });
            {
                let _lock = p.store.writer_lock().unwrap();
                assert!(p.post_once(&effect, &json!("invented-job"), &recipe()).is_err());
            }
            let server = thread.join().unwrap();
            let restarted = Meshy::observer(p.cfg.clone()).unwrap();
            assert!(restarted.lookup(&effect).is_err());
            assert!(matches!(restarted.submit(&effect, &json!({})), Submit::Uncertain(_)));
            {
                let _lock = p.store.writer_lock().unwrap();
                assert!(p.post_once(&effect, &json!("invented-job"), &recipe()).is_err());
            }
            assert!(server.recv_timeout(Duration::from_millis(50)).unwrap().is_none(), "no second HTTP mutation");
            fs::remove_dir_all(&p.cfg.data_root).unwrap();
        }
    }

    // A3/A4, R4-R6: poll original ID after restart, retain a definitive validation refusal and observed charge; cancel never DELETEs.
    #[test]
    fn meshy_original_poll_cancel_and_completed_refusal() {
        let mut p = fixture();
        let effect = uuid::Uuid::new_v4().to_string();
        let op = uuid::Uuid::new_v4().to_string();
        seed(&p, &effect, &op);
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        p.base = format!("http://{}", server.server_addr());
        let expected_op = op.clone();
        let thread = std::thread::spawn(move || {
            for status in ["IN_PROGRESS", "SUCCEEDED"] {
                let req = server.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
                assert_eq!(req.method(), &tiny_http::Method::Get);
                assert_eq!(req.url(), format!("/openapi/v1/image-to-3d/{expected_op}"));
                req.respond(tiny_http::Response::from_string(json!({"id":expected_op,"status":status,"consumed_credits":15}).to_string()))
                    .unwrap();
            }
            server
        });
        assert_eq!(p.lookup(&effect).unwrap(), Some(op.clone()));
        assert!(matches!(p.poll(&op).unwrap(), Poll::Pending));
        assert!(!p.cancel(&op).unwrap());
        assert!(matches!(p.poll(&op).unwrap(), Poll::Done { outputs: 1, credits: Some(15) }));
        assert!(p.validate(&op).unwrap_err().contains("missing original GLB"));
        let mut output = p.fetch(&op, 0).unwrap();
        let mut bytes = vec![];
        output.reader.read_to_end(&mut bytes).unwrap();
        let evidence: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(evidence["consumed_credits"], 15);
        assert_eq!(evidence["validation"]["valid"], false);
        let restarted = Meshy::observer(p.cfg.clone()).unwrap();
        assert_eq!(restarted.lookup(&effect).unwrap(), Some(op.clone()));
        assert!(restarted.validate(&op).is_err());
        let server = thread.join().unwrap();
        assert!(server.recv_timeout(Duration::from_millis(50)).unwrap().is_none(), "cancel does not issue DELETE");
        fs::remove_dir_all(&p.cfg.data_root).unwrap();
    }

    // R4/R6: retained terminal bytes/usage must recover even when the account or provider is unavailable.
    #[test]
    fn meshy_retained_completion_does_not_need_provider() {
        let mut p = fixture();
        let effect = uuid::Uuid::new_v4().to_string();
        let op = uuid::Uuid::new_v4().to_string();
        seed(&p, &effect, &op);
        let bytes = b"invented retained original";
        let stored = p.store.put(&mut &bytes[..], 1024, 1024).unwrap();
        {
            let _lock = p.store.writer_lock().unwrap();
            p.save(&p.dir(&effect).unwrap().join("outputs.json"),&json!({"outputs":[{"sha256":stored.sha256,"size":stored.size,"content_type":"model/gltf-binary"}],"evidence":{"consumed_credits":15,"validation":{"valid":true}}})).unwrap();
        }
        p.key = None;
        p.base = "http://127.0.0.1:1".into();
        assert!(matches!(p.poll(&op).unwrap(), Poll::Done { outputs: 1, credits: Some(15) }));
        assert!(
            matches!(p.submit(&uuid::Uuid::new_v4().to_string(), &json!({})), Submit::Refused(_)),
            "another job is definitely refused, never relabeled as an uncertain paid operation"
        );
        fs::remove_dir_all(&p.cfg.data_root).unwrap();
    }

    // R6/R7: definitive size/format errors fail validation; short files remain retained and native texture allocation is bounded before decoding.
    #[test]
    fn meshy_download_refusals_and_native_allocation_bounds() {
        let mut p = fixture();
        assert!(p.reserve_outputs(&p.root).unwrap_err().starts_with("store_full:"));
        assert!(!p.root.join("initial-effect.json").exists(), "capacity failure precedes the paid-intent barrier");
        p.cfg.max_upload_bytes = 8;
        let error = p.retain_download(&mut &b"invented-oversize-file"[..], "model/gltf-binary", "original-glb").unwrap_err();
        assert!(error.starts_with("validation failed:"));
        let short = p.retain_download(&mut &b"bad"[..], "image/png", "texture-0-base_color").unwrap();
        assert_eq!(short["content_type"], "application/octet-stream");
        p.store.verify(short["sha256"].as_str().unwrap(), 3).unwrap();
        assert!(texture_dimensions(b"bad").is_err());
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend(2048u32.to_be_bytes());
        png.extend(2048u32.to_be_bytes());
        assert_eq!(texture_dimensions(&png).unwrap(), (2048, 2048));
        let jpeg = [0xff, 0xd8, 0xff, 0xc0, 0, 8, 8, 8, 0, 8, 0, 0];
        assert_eq!(texture_dimensions(&jpeg).unwrap(), (2048, 2048));
        assert!(texture_dimensions(&jpeg[..8]).is_err());
        let many_nodes = json!({"asset":{"version":"2.0"},"buffers":[{"byteLength":0}],"meshes":[{}],"nodes":vec![json!({"mesh":0});65]});
        assert!(glb_check(&glb(many_nodes)).unwrap_err().contains("instances"));
        fs::remove_dir_all(&p.cfg.data_root).unwrap();
    }

    // R6/R7: only a matching confirmed stop permits local validation recovery after full storage.
    #[test]
    fn meshy_validation_restart_requires_original_stop_readback() {
        let p = fixture();
        let dir = &p.root;
        let attempt = uuid::Uuid::new_v4().to_string();
        assert!(validation_may_start(dir).is_ok());
        {
            let _lock = p.store.writer_lock().unwrap();
            p.save(&dir.join("validation-started.json"), &json!({"attempt":attempt})).unwrap();
        }
        assert!(validation_may_start(dir).is_err());
        let path = dir.join(format!("validation-stop-{attempt}.json"));
        for (confirmed, identity, allowed) in
            [(false, attempt.clone(), false), (true, uuid::Uuid::new_v4().to_string(), false), (true, attempt.clone(), true)]
        {
            {
                let _lock = p.store.writer_lock().unwrap();
                p.save(&path, &json!({"attempt":identity,"exit_confirmed":confirmed})).unwrap();
            }
            assert_eq!(validation_may_start(dir).is_ok(), allowed);
        }
        fs::remove_dir_all(&p.cfg.data_root).unwrap();
    }
}
