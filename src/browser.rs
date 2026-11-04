//! Browser identity adapter; durable state remains in the shared API (R3/R9).
use crate::{Error, Result, db};
use postgres::Client;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tiny_http::{Method, Request};

pub const ORIGIN: &str = "https://assets.ufobe.com";
const COOKIE: &str = "__Host-assetq";
const TTL: Duration = Duration::from_secs(1800);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    guard_secret: String,
    subjects: BTreeMap<String, Vec<String>>,
}

struct Session {
    subject: String,
    csrf: String,
    expires: Instant,
}

pub struct Browser {
    settings: Settings,
    sessions: Mutex<BTreeMap<String, Session>>,
}

fn denied() -> Error {
    Error::new(403, "browser_denied", "verified host sign-in and an explicit project mapping are required")
}

fn unique_header(req: &Request, name: &str) -> Result<String> {
    let mut values = req.headers().iter().filter(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name));
    let value = values.next().ok_or_else(denied)?.value.as_str().to_string();
    if values.next().is_some() {
        return Err(denied());
    }
    Ok(value)
}

fn equal_secret(a: &str, b: &str) -> bool {
    // Compare fixed-size digests so timing does not expose credential prefixes.
    use sha2::{Digest, Sha256};
    Sha256::digest(a.as_bytes()).iter().zip(Sha256::digest(b.as_bytes())).fold(0u8, |v, (a, b)| v | (a ^ b)) == 0
}

fn session_id(req: &Request) -> Result<String> {
    let cookie = unique_header(req, "Cookie")?;
    let mut entries = cookie.split(';').filter_map(|p| p.trim().split_once('=')).filter(|(k, _)| *k == COOKIE);
    let id = entries.next().ok_or_else(denied)?.1;
    if entries.next().is_some() || id.len() != 32 {
        return Err(denied());
    }
    Ok(id.into())
}

impl Browser {
    pub fn from_env() -> std::result::Result<Option<Arc<Self>>, String> {
        let Some(path) = std::env::var_os("ASSETQ_BROWSER_CONFIG") else {
            return Ok(None);
        };
        let meta = std::fs::symlink_metadata(&path).map_err(|_| "browser configuration unreadable")?;
        if !meta.is_file() || meta.len() > 65536 {
            return Err("browser configuration must be a regular file at most 64 KiB".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if meta.permissions().mode() & 0o077 != 0 {
                return Err("browser configuration must be owner-only".into());
            }
        }
        let raw = std::fs::read_to_string(&path).map_err(|_| "browser configuration unreadable")?;
        Self::from_json(&raw).map(|v| Some(Arc::new(v)))
    }

    pub fn from_json(raw: &str) -> std::result::Result<Self, String> {
        let settings: Settings = serde_json::from_str(raw).map_err(|_| "invalid browser configuration")?;
        if settings.guard_secret.len() < 32
            || settings.guard_secret.len() > 256
            || settings.subjects.is_empty()
            || settings.subjects.len() > 128
            || settings.subjects.iter().any(|(s, tokens)| {
                s.is_empty()
                    || s.len() > 200
                    || s.chars().any(char::is_control)
                    || tokens.is_empty()
                    || tokens.len() > 32
                    || tokens.iter().any(|t| t.is_empty() || t.len() > 256)
            })
        {
            return Err("invalid bounded browser identity mappings".into());
        }
        Ok(Self { settings, sessions: Mutex::new(BTreeMap::new()) })
    }

    pub fn identity(&self, req: &Request) -> Result<String> {
        if unique_header(req, "Host")? != "assets.ufobe.com"
            || !equal_secret(&unique_header(req, "X-AssetQ-Guard")?, &self.settings.guard_secret)
        {
            return Err(denied());
        }
        let subject = unique_header(req, "X-AssetQ-Subject")?;
        if !self.settings.subjects.contains_key(&subject) {
            return Err(denied());
        }
        Ok(subject)
    }

    fn grants(&self, c: &mut Client, subject: &str) -> Result<BTreeMap<String, db::Principal>> {
        let mut grants = BTreeMap::new();
        for token in self.settings.subjects.get(subject).ok_or_else(denied)? {
            match db::authenticate(c, token) {
                Ok(p) => {
                    if grants.insert(p.project.clone(), p).is_some() {
                        return Err(Error::new(503, "browser_configuration", "duplicate project mapping"));
                    }
                }
                Err(e) if e.status == 401 => {} // Revocation removes authority immediately.
                Err(e) => return Err(e),
            }
        }
        if grants.is_empty() {
            return Err(denied());
        }
        Ok(grants)
    }

    pub fn open_session(&self, c: &mut Client, req: &Request) -> Result<(Value, String)> {
        let subject = self.identity(req)?;
        let grants = self.grants(c, &subject)?;
        let mut sessions = self.sessions.lock().map_err(|_| Error::new(503, "browser_session", "session store unavailable"))?;
        let now = Instant::now();
        sessions.retain(|_, s| s.expires > now);
        let existing = session_id(req).ok().filter(|id| sessions.get(id).is_some_and(|s| s.subject == subject));
        let id = match existing {
            Some(id) => id,
            None => {
                if sessions.len() >= 1024 {
                    return Err(Error::new(503, "browser_session", "session capacity reached"));
                }
                let id = uuid::Uuid::new_v4().simple().to_string();
                sessions.insert(id.clone(), Session { subject, csrf: uuid::Uuid::new_v4().simple().to_string(), expires: now + TTL });
                id
            }
        };
        let session = &sessions[&id];
        let ttl = session.expires.saturating_duration_since(now).as_secs();
        Ok((
            json!({"csrf":session.csrf, "expires_in_seconds":ttl,
            "projects":grants.values().map(|p| json!({"id":p.project,"role":p.role})).collect::<Vec<_>>()}),
            format!("{COOKIE}={id}; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age={ttl}"),
        ))
    }

    fn check_session(&self, req: &Request) -> Result<String> {
        let subject = self.identity(req)?;
        let sessions = self.sessions.lock().map_err(|_| Error::new(503, "browser_session", "session store unavailable"))?;
        let id = session_id(req)?;
        let s = sessions.get(&id).filter(|s| s.subject == subject && s.expires > Instant::now()).ok_or_else(denied)?;
        if req.method() != &Method::Get
            && (unique_header(req, "Origin")? != ORIGIN || !equal_secret(&unique_header(req, "X-CSRF-Token")?, &s.csrf))
        {
            return Err(Error::new(403, "csrf", "matching session CSRF token and canonical origin required"));
        }
        Ok(subject)
    }

    pub fn principal(&self, c: &mut Client, req: &Request) -> Result<db::Principal> {
        let subject = self.check_session(req)?;
        let query = crate::api::query(req.url());
        let mut projects = query.iter().filter(|(k, _)| k == "project");
        let project = &projects.next().ok_or_else(denied)?.1;
        if projects.next().is_some() {
            return Err(denied());
        }
        self.grants(c, &subject)?.remove(project).ok_or_else(denied)
    }

    pub fn logout(&self, req: &Request) -> Result<String> {
        self.check_session(req)?;
        self.sessions.lock().map_err(|_| Error::new(503, "browser_session", "session store unavailable"))?.remove(&session_id(req)?);
        Ok(format!("{COOKIE}=; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=0"))
    }
}
