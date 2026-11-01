//! Immutable filesystem object store (R5, R7; storage section).
//!
//! Keys are server-owned content addresses: `objects/<aa>/<bb>/<sha256>`.
//! Writes stream into `tmp/`, are hashed and fsynced, then renamed into place.
//! A caller never supplies a path. Crash leftovers in `tmp/` are retained and
//! never appear in results; nothing here deletes a committed object.

use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum StoreError {
    /// Quota or filesystem space exhausted; existing objects are untouched.
    Full(String),
    TooLarge(u64),
    /// The source stream failed (for example a partial provider download).
    Source(String),
    Io(io::Error),
    Corrupt(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Full(m) => write!(f, "store_full: {m}"),
            StoreError::TooLarge(n) => write!(f, "too_large: limit {n} bytes"),
            StoreError::Source(m) => write!(f, "source_failed: {m}"),
            StoreError::Io(e) => write!(f, "io: {e}"),
            StoreError::Corrupt(m) => write!(f, "corrupt: {m}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stored {
    pub sha256: String,
    pub size: u64,
}

#[derive(Clone)]
pub struct Store {
    root: PathBuf,
}

pub fn valid_sha(sha: &str) -> bool {
    sha.len() == 64 && sha.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

impl Store {
    pub fn open(root: &Path) -> io::Result<Store> {
        fs::create_dir_all(root.join("tmp"))?;
        fs::create_dir_all(root.join("objects"))?;
        Ok(Store { root: root.to_path_buf() })
    }

    pub fn existing(root: &Path) -> io::Result<Store> {
        if !root.join("objects").is_dir() || !root.join("tmp").is_dir() {
            return Err(io::Error::other("existing object and temp directories required"));
        }
        Ok(Store { root: root.to_path_buf() })
    }

    fn path(&self, sha: &str) -> PathBuf {
        assert!(valid_sha(sha), "server-owned key must be a sha256");
        self.root.join("objects").join(&sha[..2]).join(&sha[2..4]).join(sha)
    }

    /// Stream `src` into the store. `limit` caps this object; `room` is the
    /// remaining quota. Exceeding `room` is a full store, never a partial object.
    pub fn put(&self, src: &mut dyn Read, limit: u64, room: u64) -> Result<Stored, StoreError> {
        let tmp = self.root.join("tmp").join(uuid::Uuid::new_v4().to_string());
        let result = self.write_tmp(&tmp, src, limit, room);
        let stored = match result {
            Ok(s) => s,
            Err(e) => {
                let _ = fs::remove_file(&tmp); // our own uncommitted temp file
                return Err(e);
            }
        };
        let dest = self.path(&stored.sha256);
        if dest.exists() {
            let _ = fs::remove_file(&tmp);
            self.verify(&stored.sha256, stored.size)?;
            return Ok(stored);
        }
        fs::create_dir_all(dest.parent().unwrap()).map_err(map_io)?;
        let mut perms = fs::metadata(&tmp).map_err(map_io)?.permissions();
        perms.set_readonly(true); // committed objects are immutable
        fs::set_permissions(&tmp, perms).map_err(map_io)?;
        fs::rename(&tmp, &dest).map_err(map_io)?;
        // Newly created shard directories must survive a host crash too.
        let shard = dest.parent().unwrap();
        for directory in [shard.to_path_buf(), shard.parent().unwrap().to_path_buf(), self.root.join("objects"), self.root.clone()] {
            File::open(directory).and_then(|d| d.sync_all()).map_err(map_io)?;
        }
        Ok(stored)
    }

    fn write_tmp(&self, tmp: &Path, src: &mut dyn Read, limit: u64, room: u64) -> Result<Stored, StoreError> {
        let mut out = File::create(tmp).map_err(map_io)?;
        let mut hash = Sha256::new();
        let mut size = 0u64;
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = match src.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(StoreError::Source(e.to_string())),
            };
            size += n as u64;
            if size > limit {
                return Err(StoreError::TooLarge(limit));
            }
            if size > room {
                return Err(StoreError::Full(format!("quota leaves {room} bytes")));
            }
            hash.update(&buf[..n]);
            out.write_all(&buf[..n]).map_err(map_io)?;
        }
        out.sync_all().map_err(map_io)?;
        Ok(Stored { sha256: hex::encode(hash.finalize()), size })
    }

    /// Count physical files, including orphan blobs and crash leftovers (R7).
    /// The caller holds the database admission lock during inventory and write.
    pub fn inventory(&self) -> io::Result<(u64, u64)> {
        fn bytes(path: &Path) -> io::Result<u64> {
            let mut total = 0u64;
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let kind = entry.file_type()?;
                total = total
                    .checked_add(if kind.is_dir() {
                        bytes(&entry.path())?
                    } else if kind.is_file() {
                        entry.metadata()?.len()
                    } else {
                        return Err(io::Error::other("unexpected symlink or special file in store"));
                    })
                    .ok_or_else(|| io::Error::other("store size overflow"))?;
            }
            Ok(total)
        }
        Ok((bytes(&self.root.join("objects"))?, bytes(&self.root.join("tmp"))?))
    }

    pub fn open_object(&self, sha: &str) -> io::Result<File> {
        if !valid_sha(sha) {
            return Err(io::Error::other("invalid object key"));
        }
        File::open(self.path(sha))
    }

    /// Re-read and hash a committed object.
    pub fn verify(&self, sha: &str, size: u64) -> Result<(), StoreError> {
        let mut f = self.open_object(sha).map_err(StoreError::Io)?;
        let mut hash = Sha256::new();
        let n = io::copy(&mut f, &mut hash).map_err(StoreError::Io)?;
        let got = hex::encode(hash.finalize());
        if n != size || got != sha {
            return Err(StoreError::Corrupt(format!("{sha}: read {n} bytes hashing to {got}")));
        }
        Ok(())
    }
}

fn map_io(e: io::Error) -> StoreError {
    // ENOSPC (28) and EDQUOT (69 macOS, 122 Linux) mean a full store.
    match e.raw_os_error() {
        Some(28 | 69 | 122) => StoreError::Full(e.to_string()),
        _ => StoreError::Io(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_is_content_addressed_and_quota_bounded() {
        let dir = std::env::temp_dir().join(format!("assetq-store-{}", uuid::Uuid::new_v4()));
        let s = Store::open(&dir).unwrap();
        let a = s.put(&mut &b"hello"[..], 100, 100).unwrap();
        assert_eq!(a.sha256, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
        assert_eq!(s.put(&mut &b"hello"[..], 100, 100).unwrap(), a);
        s.verify(&a.sha256, 5).unwrap();
        assert!(matches!(s.put(&mut &b"0123456789"[..], 100, 4), Err(StoreError::Full(_))));
        assert!(matches!(s.put(&mut &b"0123456789"[..], 4, 100), Err(StoreError::TooLarge(4))));
        assert_eq!(fs::read_dir(dir.join("tmp")).unwrap().count(), 0);
        s.verify(&a.sha256, 5).unwrap();
        // R5: deduplication must not accept a same-size corrupted payload.
        let path = s.path(&a.sha256);
        let mut perms = fs::metadata(&path).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        fs::set_permissions(&path, perms).unwrap();
        fs::write(&path, b"xxxxx").unwrap();
        assert!(matches!(s.put(&mut &b"hello"[..], 100, 100), Err(StoreError::Corrupt(_))));
        fs::remove_dir_all(dir).unwrap();
    }
}
