//! Native-size import reuses a completed operation; it cannot launch a runtime.
use super::*;

pub(super) struct Source {
    pub job: String,
    pub effect: String,
    pub project: String,
    pub request: Value,
    pub home: PathBuf,
    pub item: Value,
    pub report: Value,
    pub attachments: Vec<Value>,
    pub sha256: String,
}

pub fn method() -> Value {
    let mut spec = super::method();
    spec["operation"] = json!("image.import_native");
    spec["pipeline_revision"] = json!("6");
    spec["parameters"] = json!({"source_job":{"type":"string"}});
    spec["capabilities"] = json!(["retained.native.image", "native.savedPath", "completed source operation"]);
    spec["inputs"] = json!(["operator-bound completed generation", "verified original text/PNG attachment IDs"]);
    spec["limitations"] = json!([
        "Retained native-size original only; no generation, downscale or <=1024 guarantee; 4096-axis/16-million-pixel/64-MiB safety bounds."
    ]);
    spec
}

impl ImageWorker {
    /// Administrative preparation requires fresh original-operation reconciliation.
    /// It has no Runtime launch/input, grant edit or automatic generation fallback.
    pub fn retained_job(cfg: Config, root: PathBuf, source_job: &str, expected_sha256: &str) -> Res<Self> {
        if expected_sha256.len() != 64 || !expected_sha256.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
            return Err("expected retained PNG SHA-256 required".into());
        }
        let mut worker = Self::new(cfg.clone(), root).map_err(|e| e.to_string())?;
        let mut c = db::connect_config(&cfg).map_err(|e| e.to_string())?;
        let row = c
            .query_one(
                "SELECT j.project, j.request, e.id FROM jobs j JOIN effects e ON e.job_id=j.id WHERE j.id=$1 AND e.provider=$2",
                &[&source_job, &PROVIDER],
            )
            .map_err(|e| e.to_string())?;
        let project: String = row.get(0);
        let request: Value = row.get(1);
        let effect: String = row.get(2);
        let dir = worker.dir(&effect)?;
        if runner_alive(&dir)? {
            return Err("original runner still live; observe it; never replay".into());
        }
        let receipt = read_json(&dir.join("request.json"))?;
        if receipt["effect"] != effect
            || receipt["runtime"] != PIN
            || receipt["request"] != request
            || receipt["root"] != json!(worker.root)
        {
            return Err("original effect/runtime/request/root mismatch; no import".into());
        }
        protected::reconcile_release(&dir)?;
        let home = protected::prepared_home(&dir)?;
        let report = completed_report(&cfg, &dir, &worker.root, &home, &project)?;
        let item = native_image(&dir, report["thread"].as_str(), report["turn"].as_str())?.ok_or("original native image item missing")?;
        let mut attachments = vec![];
        let store = Store::existing(&cfg.data_root).map_err(|e| e.to_string())?;
        for id in request["attachments"].as_array().ok_or("source attachments missing")? {
            let object = db::get_object(&mut c, &project, id.as_str().ok_or("source attachment invalid")?).map_err(|e| e.to_string())?;
            let sha = object["sha256"].as_str().ok_or("source attachment hash missing")?;
            store.verify(sha, object["size"].as_u64().ok_or("source attachment size missing")?).map_err(|e| e.to_string())?;
            attachments.push(json!({"object_id":id,"sha256":sha,"size":object["size"]}));
        }
        let (bytes, _) = original_png(&cfg, &home, &item, &json!({})).map_err(|e| e.reason)?;
        let sha256 = db::sha256_hex(&bytes);
        if sha256 != expected_sha256 {
            return Err("retained original differs from expected native PNG hash; no import".into());
        }
        // Reconstruct the old observation against its unchanged limits, without input.
        let old = complete(&cfg, &dir, &home, &item, report.clone(), &attachments, &request["constraints"])?;
        retain_observation(&dir, &Ok(old))?;
        recover_native_with(&dir, || Ok(()))?;
        let spec = method();
        match db::resolve_method(&mut c, &project, "image.worker.v1@6") {
            Ok((_, _, existing)) if existing == spec => (),
            Ok(_) => return Err("existing method version 6 differs; never rewrite it".into()),
            Err(error) if error.code == "unknown_method" => {
                // The administrative CLI already holds the database authority, as migrate/project do.
                let owner = db::Principal { project: project.clone(), role: "owner".into() };
                let created =
                    db::create_method_version(&mut c, &owner, &json!({"id":"image.worker.v1","status":"experimental","spec":spec}))
                        .map_err(|e| e.to_string())?;
                if created["version"] != 6 {
                    return Err("native-size version allocation differs; no import".into());
                }
            }
            Err(error) => return Err(error.to_string()),
        }
        worker.retained = Some(Source { job: source_job.into(), effect, project, request, home, item, report, attachments, sha256 });
        Ok(worker)
    }
}

fn matches(source: &Source, request: &Value) -> bool {
    request["method"] == "image.worker.v1@6"
        && request["operation"] == "image.import_native"
        && request["constraints"] == json!({"source_job":source.job})
        && request["budget"] == json!({"max_credits":0})
        && request["text"] == source.request["text"]
        && request["attachments"] == source.request["attachments"]
}

fn authorize(worker: &ImageWorker, effect: &str, request: &Value) -> Res<()> {
    let source = worker.retained.as_ref().ok_or("retained source is not configured")?;
    if !matches(source, request) {
        return Err("retained import differs from operator-bound source; no generation".into());
    }
    let mut c = db::connect_config(&worker.cfg).map_err(|e| e.to_string())?;
    let row = c
        .query_one(
            "SELECT j.project,e.request FROM effects e JOIN jobs j ON j.id=e.job_id WHERE e.id=$1 AND e.provider=$2",
            &[&effect, &PROVIDER],
        )
        .map_err(|e| e.to_string())?;
    if row.get::<_, String>(0) != source.project || row.get::<_, Value>(1) != *request {
        return Err("retained effect/project/request mismatch".into());
    }
    Ok(())
}

pub(super) fn submit(worker: &ImageWorker, effect: &str, request: &Value) -> Submit {
    if let Err(reason) = authorize(worker, effect, request) {
        return Submit::Refused(reason);
    }
    let result = (|| -> Res<()> {
        let dir = worker.reserve(effect, request)?;
        spool(&worker.cfg, &dir, worker.retained.as_ref().unwrap())
    })();
    match result {
        Ok(()) => Submit::Accepted(effect.into()),
        Err(reason) => Submit::Uncertain(reason),
    }
}

pub(super) fn resume(worker: &ImageWorker, effect: &str) -> Res<()> {
    let dir = worker.dir(effect)?;
    let request = read_json(&dir.join("request.json"))?;
    if request["effect"] != effect || request["runtime"] != PIN {
        return Err("retained receipt differs; no replacement".into());
    }
    authorize(worker, effect, &request["request"])?;
    spool(&worker.cfg, &dir, worker.retained.as_ref().unwrap())
}

fn spool(cfg: &Config, dir: &Path, source: &Source) -> Res<()> {
    seal(&dir.join("retained-source.json"), &json!({"job":source.job,"effect":source.effect,"sha256":source.sha256}))?;
    let mut report = source.report.clone();
    report["retained_import"] = json!(true);
    report["usage_scope"] = json!("source_generation; this import submitted no Runtime input");
    report["source_generation"] = json!({"job":source.job,"effect":source.effect,"request":source.request,"sha256":source.sha256,"runtime":PIN,"usage":source.report["usage"],"charge":null});
    let mut result = complete(cfg, dir, &source.home, &source.item, report, &source.attachments, &json!({}))?;
    if result["status"] == "completed" && result["outputs"][0]["sha256"] != source.sha256 {
        return Err("retained original changed since preparation; no replacement".into());
    }
    result["exit_confirmed"] = json!(true);
    if result["status"] == "completed" {
        verify_outputs(dir, &result)?;
    }
    save(&dir.join("result.json"), &result)?;
    File::options().write(true).open(dir.join("reservation")).and_then(|f| f.set_len(0)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // R3/R5/R6: native-size import must keep bytes, parentage and unknown charge,
    // reject source/request substitution, and never invoke another generation.
    #[test]
    fn image_worker_retained_import_keeps_native_size_and_refuses_substitution() {
        let (dir, cfg, mut worker) = super::super::tests::fixture();
        assert!(
            ImageWorker::retained_job(cfg.clone(), dir.clone(), "invented-job", "invalid")
                .err()
                .unwrap()
                .contains("expected retained PNG SHA-256 required")
        );
        let home = dir.join("native-home");
        fs::create_dir_all(home.join("generated_images")).unwrap();
        let path = home.join("generated_images/invented.png");
        image::RgbImage::new(1254, 1254).save(&path).unwrap();
        let bytes = fs::read(&path).unwrap();
        let source = Source {
            job: db::new_id(),
            effect: db::new_id(),
            project: "invented-project".into(),
            request: json!({"text":"invented retained brief","method":"image.worker.v1@5","attachments":["invented-attachment"],"constraints":{"max_width":1024,"max_height":1024}}),
            home,
            item: json!({"id":"invented-native-tool","type":"imageGeneration","status":"completed","failure":null,"savedPath":path}),
            report: json!({"thread":"invented-thread","turn":"invented-turn","usage":{"total_tokens":152203}}),
            attachments: vec![json!({"object_id":"invented-attachment","sha256":"invented-hash"})],
            sha256: db::sha256_hex(&bytes),
        };
        let request = json!({"method":"image.worker.v1@6","operation":"image.import_native","text":source.request["text"],"attachments":source.request["attachments"],
            "budget":{"max_credits":0},"constraints":{"source_job":source.job}});
        assert!(matches(&source, &request));
        let effect = db::new_id();
        let output = worker.reserve(&effect, &request).unwrap();
        spool(&cfg, &output, &source).unwrap();
        assert!(matches!(worker.poll(&effect), Ok(Poll::Done { outputs: 3, credits: Some(0) })));
        let result = worker.result(&effect).unwrap();
        assert_eq!(result["outputs"][0]["evidence"]["dimensions"], json!({"width":1254,"height":1254}));
        assert_eq!(result["outputs"][0]["evidence"]["source_generation"]["request"], source.request);
        let mut original = vec![];
        worker.fetch(&effect, 0).unwrap().reader.read_to_end(&mut original).unwrap();
        assert_eq!(original, bytes);
        assert_eq!(fs::metadata(output.join("reservation")).unwrap().len(), 0);
        assert!(worker.reserve(&effect, &request).is_err());
        for (key, value) in [
            ("method", json!("image.worker.v1@5")),
            ("operation", json!("image.generate")),
            ("text", json!("other")),
            ("attachments", json!([])),
            ("constraints", json!({"source_job":"foreign"})),
            ("budget", json!({"max_credits":1})),
        ] {
            let mut substituted = request.clone();
            substituted[key] = value;
            assert!(!matches(&source, &substituted), "{key}");
        }
        fs::write(&path, b"not the retained original").unwrap();
        let other = worker.reserve(&db::new_id(), &request).unwrap();
        // A changed file can fail decode; it cannot produce a successful image manifest.
        spool(&cfg, &other, &source).unwrap();
        assert_eq!(read_json(&other.join("result.json")).unwrap()["status"], "failed");
        worker.retained = Some(source);
        assert!(matches!(worker.submit(&db::new_id(), &json!({"method":"image.worker.v1@5"})), Submit::Refused(_)));
        assert_eq!(method()["pipeline_revision"], "6");
        assert_eq!(super::super::method()["pipeline_revision"], "5");
        fs::remove_dir_all(dir).unwrap();
    }
}
