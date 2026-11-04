//! AssetQ: durable asset-generation jobs over PostgreSQL metadata and an
//! immutable filesystem object store. See `assetq.md` and `docs/service.md`.

pub mod api;
pub mod db;
pub mod exec;
pub mod image_worker;
pub mod meshy;
pub mod model;
pub mod provider;
pub mod store;
pub mod traces;

use std::path::PathBuf;

/// One error shape for API, CLI and executor paths: an HTTP status, a stable
/// machine code and a human reason.
#[derive(Debug, Clone)]
pub struct Error {
    pub status: u16,
    pub code: &'static str,
    pub reason: String,
}

impl Error {
    pub fn new(status: u16, code: &'static str, reason: impl Into<String>) -> Error {
        Error { status, code, reason: reason.into() }
    }
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({"error": self.code, "reason": self.reason})
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}): {}", self.code, self.status, self.reason)
    }
}

impl From<postgres::Error> for Error {
    fn from(e: postgres::Error) -> Error {
        // The message never contains connection credentials.
        Error::new(503, "database", e.to_string())
    }
}

impl From<model::Reject> for Error {
    fn from(r: model::Reject) -> Error {
        match r {
            model::Reject::Stale => Error::new(409, "stale_fence", "a newer executor owns this job"),
            model::Reject::Invalid(m) => Error::new(409, "invalid_transition", m),
        }
    }
}

impl From<store::StoreError> for Error {
    fn from(e: store::StoreError) -> Error {
        use store::StoreError::*;
        match &e {
            Full(_) => Error::new(507, "store_full", e.to_string()),
            TooLarge(_) => Error::new(413, "too_large", e.to_string()),
            Source(_) => Error::new(400, "upload_interrupted", e.to_string()),
            Unknown(_) => Error::new(503, "store_unknown", e.to_string()),
            Io(_) | Corrupt(_) => Error::new(500, "store_io", e.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Runtime configuration. Secrets arrive through the environment from
/// protected host configuration, never through arguments or Git (R3).
#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub data_root: PathBuf,
    pub quota_bytes: u64,
    pub max_upload_bytes: u64,
    pub temp_quota_bytes: u64,
    pub metadata_quota_bytes: u64,
    pub free_space_floor_bytes: u64,
    pub metadata_admission_headroom_bytes: u64,
    pub metadata_recovery_headroom_bytes: u64,
    pub max_execution_ms: u64,
    pub content_types: Vec<String>,
    pub executors: usize,
    pub poll_ms: u64,
    pub lease_ms: u64,
    pub retention_days: u32,
    /// Enables the explicitly synthetic file-backed provider (tests and selftest).
    pub fake_provider_dir: Option<PathBuf>,
    pub image_worker_root: Option<PathBuf>,
}

pub const DEFAULT_TYPES: &str =
    "image/png,image/jpeg,image/webp,model/gltf-binary,text/plain,application/json,application/octet-stream,application/x-assetq-synthetic";

fn env_num(name: &str, default: u64) -> std::result::Result<u64, String> {
    match std::env::var(name) {
        Ok(v) => {
            v.parse::<u64>().ok().filter(|n| *n <= i64::MAX as u64).ok_or_else(|| format!("{name} must be an integer in 0..=i64::MAX"))
        }
        Err(_) => Ok(default),
    }
}

impl Config {
    pub fn from_env() -> std::result::Result<Config, String> {
        let database_url = std::env::var("ASSETQ_DATABASE_URL").map_err(|_| "ASSETQ_DATABASE_URL is required".to_string())?;
        let data_root = PathBuf::from(std::env::var("ASSETQ_DATA_ROOT").map_err(|_| "ASSETQ_DATA_ROOT is required".to_string())?);
        Ok(Config {
            database_url,
            data_root,
            quota_bytes: env_num("ASSETQ_QUOTA_BYTES", 50 << 30)?,
            max_upload_bytes: env_num("ASSETQ_MAX_UPLOAD_BYTES", 512 << 20)?,
            temp_quota_bytes: env_num("ASSETQ_TEMP_QUOTA_BYTES", 1 << 30)?,
            metadata_quota_bytes: env_num("ASSETQ_METADATA_QUOTA_BYTES", 1 << 30)?,
            free_space_floor_bytes: env_num("ASSETQ_FREE_SPACE_FLOOR_BYTES", 1 << 30)?,
            metadata_admission_headroom_bytes: env_num("ASSETQ_METADATA_ADMISSION_HEADROOM_BYTES", 4 << 20)?,
            metadata_recovery_headroom_bytes: env_num("ASSETQ_METADATA_RECOVERY_HEADROOM_BYTES", 16 << 20)?,
            max_execution_ms: env_num("ASSETQ_MAX_EXECUTION_MS", 86_400_000)?,
            content_types: std::env::var("ASSETQ_CONTENT_TYPES")
                .unwrap_or_else(|_| DEFAULT_TYPES.into())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            executors: env_num("ASSETQ_EXECUTORS", 2)? as usize,
            poll_ms: env_num("ASSETQ_POLL_MS", 30_000)?,
            lease_ms: env_num("ASSETQ_LEASE_MS", 120_000)?,
            retention_days: u32::try_from(env_num("ASSETQ_RETENTION_DAYS", 30)?)
                .ok()
                .filter(|n| *n <= i32::MAX as u32)
                .ok_or("ASSETQ_RETENTION_DAYS must fit a nonnegative PostgreSQL interval")?,
            fake_provider_dir: std::env::var("ASSETQ_FAKE_PROVIDER_DIR").ok().map(PathBuf::from),
            image_worker_root: std::env::var("ASSETQ_IMAGE_WORKER_ROOT").ok().map(PathBuf::from),
        })
    }
}
