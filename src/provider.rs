//! Provider boundary. Real image and 3D providers attach in later tasks; this
//! build ships only `synthetic.fake`, an explicitly synthetic provider whose
//! state lives in files so it survives service restarts in tests.
//!
//! Fault injection: create `<dir>/faults/<name>`; each file is consumed once.
//! `lose_reply` (create the operation, then lose the reply), `fail_fetch`
//! (break the second output stream), `cancel_unconfirmed`.

use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::io::{self, Read};
use std::path::PathBuf;

pub enum Submit {
    Accepted(String),
    /// The provider may have the request; the caller must reconcile.
    Uncertain(String),
    Refused(String),
}

pub enum Poll {
    Pending,
    Done { outputs: usize, credits: Option<u64> },
    Failed { reason: String, credits: Option<u64> },
    Cancelled,
}

pub struct Output {
    pub content_type: String,
    pub reader: Box<dyn Read + Send>,
    pub evidence: Value,
}

pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn submit(&self, effect_id: &str, request: &Value) -> Submit;
    /// `Ok(None)` is a sealed absence: no delayed submission for this effect
    /// can still be accepted. Eventually consistent/missing lookups return Err.
    /// Real adapters must implement the same barrier or stay uncertain (R4).
    fn lookup(&self, effect_id: &str) -> Result<Option<String>, String>;
    fn poll(&self, op_id: &str) -> Result<Poll, String>;
    fn fetch(&self, op_id: &str, index: usize) -> Result<Output, String>;
    /// `Ok(true)` confirmed stopped, `Ok(false)` provider could not confirm.
    fn cancel(&self, op_id: &str) -> Result<bool, String>;
    /// Definitive output validation after originals have been retained/imported.
    fn validate(&self, _op_id: &str) -> Result<(), String> {
        Ok(())
    }
}

pub struct FakeProvider {
    dir: PathBuf,
}

impl FakeProvider {
    pub fn new(dir: PathBuf) -> io::Result<FakeProvider> {
        fs::create_dir_all(dir.join("ops"))?;
        fs::create_dir_all(dir.join("faults"))?;
        fs::create_dir_all(dir.join("effects"))?;
        Ok(FakeProvider { dir })
    }
    fn fault(&self, name: &str) -> bool {
        fs::remove_file(self.dir.join("faults").join(name)).is_ok()
    }
    fn effect_path(&self, effect: &str) -> Result<PathBuf, String> {
        if effect.is_empty() || effect.len() > 100 || !effect.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            return Err("malformed effect id".into());
        }
        Ok(self.dir.join("effects").join(effect))
    }
    fn seal(&self, effect: &str, receipt: &str) -> Result<(), String> {
        let path = self.effect_path(effect)?;
        let mut f = fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|e| e.to_string())?;
        f.write_all(receipt.as_bytes()).and_then(|_| f.sync_all()).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        fs::File::open(self.dir.join("effects")).and_then(|f| f.sync_all()).map_err(|e| e.to_string())?;
        Ok(())
    }
    fn op_path(&self, op: &str) -> Result<PathBuf, String> {
        if op.len() != 36 || !op.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') {
            return Err("malformed operation id".into());
        }
        Ok(self.dir.join("ops").join(format!("{op}.json")))
    }
    fn read(&self, op: &str) -> Result<Value, String> {
        let raw = fs::read(self.op_path(op)?).map_err(|e| e.to_string())?;
        serde_json::from_slice(&raw).map_err(|e| e.to_string())
    }
    fn write(&self, op: &str, v: &Value) -> Result<(), String> {
        let path = self.op_path(op)?;
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, v.to_string()).and_then(|_| fs::rename(&tmp, &path)).map_err(|e| e.to_string())
    }
    /// Count of submissions the provider received (a paid call in real life).
    pub fn submissions(&self) -> usize {
        fs::read_dir(self.dir.join("ops"))
            .map(|d| d.filter(|e| e.as_ref().is_ok_and(|e| e.path().extension().is_some_and(|x| x == "json"))).count())
            .unwrap_or(0)
    }
}

impl Provider for FakeProvider {
    fn name(&self) -> &str {
        "synthetic.fake"
    }
    fn submit(&self, effect_id: &str, request: &Value) -> Submit {
        let op = uuid::Uuid::new_v4().to_string();
        if let Err(e) = self.seal(effect_id, &op) {
            return match self.effect_path(effect_id).ok().and_then(|p| fs::read_to_string(p).ok()) {
                Some(s) if s == "absent" => Submit::Refused("effect sealed absent; delayed dispatch refused".into()),
                _ => Submit::Uncertain(format!("effect already reserved or receipt unavailable: {e}")),
            };
        }
        let c = &request["constraints"];
        let state = json!({
            "op": op, "effect_id": effect_id, "text": request["text"],
            "outputs": c["outputs"].as_u64().unwrap_or(1),
            "polls_left": c["polls"].as_u64().unwrap_or(0),
            "report_usage": c["report_usage"].as_bool().unwrap_or(true),
            "fail": c["fail"].as_bool().unwrap_or(false),
            "cancelled": false,
        });
        if let Err(e) = self.write(&op, &state) {
            return Submit::Uncertain(e);
        }
        if self.fault("lose_reply") {
            return Submit::Uncertain("synthetic fault: reply lost after the provider accepted".into());
        }
        Submit::Accepted(op)
    }
    fn lookup(&self, effect_id: &str) -> Result<Option<String>, String> {
        let path = self.effect_path(effect_id)?;
        if !path.exists() {
            // Atomic tombstone prevents a paused old executor submitting later.
            if self.seal(effect_id, "absent").is_ok() {
                return Ok(None);
            }
        }
        let receipt = fs::read_to_string(path).map_err(|e| e.to_string())?;
        if receipt == "absent" {
            return Ok(None);
        }
        let operation = self.read(&receipt)?; // partial receipts remain unknown
        if operation["effect_id"] != effect_id {
            return Err("effect receipt mismatch".into());
        }
        Ok(Some(receipt))
    }
    fn poll(&self, op: &str) -> Result<Poll, String> {
        let mut v = self.read(op)?;
        if v["cancelled"] == true {
            return Ok(Poll::Failed { reason: "cancelled".into(), credits: Some(0) });
        }
        let left = v["polls_left"].as_u64().unwrap_or(0);
        if left > 0 {
            v["polls_left"] = json!(left - 1);
            self.write(op, &v)?;
            return Ok(Poll::Pending);
        }
        let credits = (v["report_usage"] == true).then_some(1);
        if v["fail"] == true {
            return Ok(Poll::Failed { reason: "synthetic failure requested".into(), credits });
        }
        Ok(Poll::Done { outputs: v["outputs"].as_u64().unwrap_or(1) as usize, credits })
    }
    fn fetch(&self, op: &str, index: usize) -> Result<Output, String> {
        let v = self.read(op)?;
        let body =
            format!("SYNTHETIC ASSETQ OUTPUT - not a real generation\nop={op}\nindex={index}\ntext={}\n", v["text"].as_str().unwrap_or(""));
        let bytes = body.into_bytes();
        if index == 1 && self.fault("fail_fetch") {
            let half = bytes[..bytes.len() / 2].to_vec();
            return Ok(Output {
                content_type: "application/x-assetq-synthetic".into(),
                reader: Box::new(io::Cursor::new(half).chain(Broken)),
                evidence: Value::Null,
            });
        }
        Ok(Output {
            content_type: "application/x-assetq-synthetic".into(),
            reader: Box::new(io::Cursor::new(bytes)),
            evidence: Value::Null,
        })
    }
    fn cancel(&self, op: &str) -> Result<bool, String> {
        if self.fault("cancel_unconfirmed") {
            return Ok(false);
        }
        let mut v = self.read(op)?;
        v["cancelled"] = json!(true);
        self.write(op, &v)?;
        Ok(true)
    }
}

struct Broken;
impl Read for Broken {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::ConnectionReset, "synthetic fault: stream reset"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sealed_absence_refuses_late_dispatch_and_receipt_survives_restart() {
        let dir = std::env::temp_dir().join(format!("aq-provider-{}", uuid::Uuid::new_v4()));
        let p = FakeProvider::new(dir.clone()).unwrap();
        assert_eq!(p.lookup("old-effect").unwrap(), None);
        assert!(matches!(p.submit("old-effect", &json!({})), Submit::Refused(_)));
        assert_eq!(p.submissions(), 0);
        let Submit::Accepted(op) = p.submit("new-effect", &json!({})) else { panic!("new effect refused") };
        let p = FakeProvider::new(dir.clone()).unwrap();
        assert_eq!(p.lookup("new-effect").unwrap(), Some(op));
        assert!(matches!(p.submit("new-effect", &json!({})), Submit::Uncertain(_)));
        assert_eq!(p.submissions(), 1);
        fs::remove_dir_all(dir).unwrap();
    }
}
