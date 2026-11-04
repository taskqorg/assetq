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
    Unknown(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Full(m) => write!(f, "store_full: {m}"),
            StoreError::TooLarge(n) => write!(f, "too_large: limit {n} bytes"),
            StoreError::Source(m) => write!(f, "source_failed: {m}"),
            StoreError::Io(e) => write!(f, "io: {e}"),
            StoreError::Corrupt(m) => write!(f, "corrupt: {m}"),
            StoreError::Unknown(m) => write!(f, "store_unknown: {m}"),
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
    quota: u64,
    temp_quota: u64,
    floor: u64,
}

pub fn valid_sha(sha: &str) -> bool {
    sha.len() == 64 && sha.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

impl Store {
    pub fn open(root: &Path) -> io::Result<Store> {
        fs::create_dir_all(root.join("tmp"))?;
        fs::create_dir_all(root.join("objects"))?;
        Ok(Store { root: root.to_path_buf(), quota: u64::MAX, temp_quota: u64::MAX, floor: 0 })
    }

    pub fn existing(root: &Path) -> io::Result<Store> {
        if !root.join("objects").is_dir() || !root.join("tmp").is_dir() {
            return Err(io::Error::other("existing object and temp directories required"));
        }
        Ok(Store { root: root.to_path_buf(), quota: u64::MAX, temp_quota: u64::MAX, floor: 0 })
    }

    pub fn with_bounds(mut self, quota: u64, temp_quota: u64, floor: u64) -> Store {
        self.quota = quota;
        self.temp_quota = temp_quota;
        self.floor = floor;
        self
    }

    pub fn available(&self) -> io::Result<u64> {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            use std::os::unix::fs::MetadataExt;
            let device = fs::metadata(&self.root)?.dev();
            if [self.root.join("tmp"), self.root.join("objects")].iter().any(|p| fs::metadata(p).map(|m| m.dev() != device).unwrap_or(true))
            {
                return Err(io::Error::other("data root, objects and tmp must share one observed filesystem"));
            }
            let path = std::ffi::CString::new(self.root.as_os_str().as_bytes())?;
            let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
            // statvfs initializes the structure on success; bavail respects the service user's reserve.
            if unsafe { libc::statvfs(path.as_ptr(), stat.as_mut_ptr()) } != 0 {
                return Err(io::Error::last_os_error());
            }
            let stat = unsafe { stat.assume_init() };
            (stat.f_bavail as u64).checked_mul(stat.f_frsize as u64).ok_or_else(|| io::Error::other("available space overflow"))
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetVolumePathNameW};
            let mut volume = None;
            let mut available = u64::MAX;
            for directory in [&self.root, &self.root.join("tmp"), &self.root.join("objects")] {
                let path: Vec<u16> = directory.canonicalize()?.as_os_str().encode_wide().chain(Some(0)).collect();
                let mut mount = vec![0u16; 32768];
                // Both APIs initialize their outputs on success; junctions resolve to the target volume.
                if unsafe { GetVolumePathNameW(path.as_ptr(), mount.as_mut_ptr(), mount.len() as u32) } == 0 {
                    return Err(io::Error::last_os_error());
                }
                mount.truncate(mount.iter().position(|c| *c == 0).ok_or_else(|| io::Error::other("unterminated volume path"))?);
                if volume.as_ref().is_some_and(|v| v != &mount) {
                    return Err(io::Error::other("data root, objects and tmp must share one observed filesystem"));
                }
                volume = Some(mount);
                let mut bytes = 0;
                if unsafe { GetDiskFreeSpaceExW(path.as_ptr(), &mut bytes, std::ptr::null_mut(), std::ptr::null_mut()) } == 0 {
                    return Err(io::Error::last_os_error());
                }
                available = available.min(bytes);
            }
            Ok(available)
        }
        #[cfg(not(any(unix, windows)))]
        Err(io::Error::other("free-space observation is unsupported on this platform"))
    }

    pub fn room(&self) -> Result<u64, StoreError> {
        self.room_with(self.available().map_err(|e| StoreError::Unknown(e.to_string()))?)
    }

    fn room_with(&self, available: u64) -> Result<u64, StoreError> {
        let (objects, temp) = self.inventory().map_err(|e| StoreError::Unknown(e.to_string()))?;
        Ok(self
            .quota
            .saturating_sub(objects.saturating_add(temp))
            .min(self.temp_quota.saturating_sub(temp))
            .min(available.saturating_sub(self.floor).saturating_sub(64 << 10)))
    }

    /// Independent of the DB session: its loss must not release a live stream's reservation.
    pub fn writer_lock(&self) -> Result<File, StoreError> {
        let file = fs::OpenOptions::new().create(true).truncate(false).write(true).open(self.root.join("writer.lock")).map_err(map_io)?;
        file.try_lock().map_err(|e| StoreError::Full(format!("store writer reservation unavailable: {e}")))?;
        Ok(file)
    }

    fn path(&self, sha: &str) -> PathBuf {
        assert!(valid_sha(sha), "server-owned key must be a sha256");
        self.root.join("objects").join(&sha[..2]).join(&sha[2..4]).join(sha)
    }

    /// Stream `src` into the store. `limit` caps this object; `room` is the
    /// remaining quota. Exceeding `room` is a full store, never a partial object.
    pub fn put(&self, src: &mut dyn Read, limit: u64, room: u64) -> Result<Stored, StoreError> {
        let _writer = self.writer_lock()?;
        self.put_locked(src, limit, room)
    }

    pub(crate) fn put_locked(&self, src: &mut dyn Read, limit: u64, room: u64) -> Result<Stored, StoreError> {
        if room == 0 || self.room()? == 0 {
            return Err(StoreError::Full("quota or free-space floor exhausted".into()));
        }
        let tmp = self.root.join("tmp").join(uuid::Uuid::new_v4().to_string());
        let result = self.write_tmp(&tmp, src, limit, room, || self.available());
        let stored = match result {
            Ok(s) => s,
            Err(e) => {
                let _ = fs::remove_file(&tmp); // our own uncommitted temp file
                return Err(e);
            }
        };
        let dest = self.path(&stored.sha256);
        let space = self.available().map_err(|e| StoreError::Unknown(e.to_string())).and_then(|bytes| {
            if bytes < self.floor { Err(StoreError::Full("free-space floor reached before object installation".into())) } else { Ok(()) }
        });
        if let Err(e) = space {
            let _ = fs::remove_file(&tmp);
            return Err(e);
        }
        if dest.exists() {
            let _ = fs::remove_file(&tmp);
            self.verify(&stored.sha256, stored.size)?;
            return Ok(stored);
        }
        fs::create_dir_all(dest.parent().unwrap()).map_err(map_io)?;
        let mut perms = fs::metadata(&tmp).map_err(map_io)?.permissions();
        perms.set_readonly(true); // committed objects are immutable
        fs::set_permissions(&tmp, perms).map_err(map_io)?;
        durable_rename(&tmp, &dest).map_err(map_io)?;
        #[cfg(unix)]
        for directory in [dest.parent().unwrap().parent().unwrap().to_path_buf(), self.root.join("objects"), self.root.clone()] {
            File::open(directory).and_then(|d| d.sync_all()).map_err(map_io)?;
        }
        Ok(stored)
    }

    fn write_tmp(
        &self,
        tmp: &Path,
        src: &mut dyn Read,
        limit: u64,
        room: u64,
        available: impl Fn() -> io::Result<u64>,
    ) -> Result<Stored, StoreError> {
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
            if n as u64 > self.room_with(available().map_err(|e| StoreError::Unknown(e.to_string()))?)? {
                return Err(StoreError::Full("quota or free-space floor reached during stream".into()));
            }
            hash.update(&buf[..n]);
            out.write_all(&buf[..n]).map_err(map_io)?;
            if available().map_err(|e| StoreError::Unknown(e.to_string()))? < self.floor {
                return Err(StoreError::Full("free-space floor reached during stream".into()));
            }
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

/// Windows directory handles cannot be flushed through File::open; the native
/// move supplies write-through durability. Unix flushes the parent directory.
pub fn durable_rename(from: &Path, to: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW};
        let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) } == 0 {
            return Err(io::Error::last_os_error());
        }
    }
    #[cfg(not(windows))]
    fs::rename(from, to)?;
    #[cfg(unix)]
    File::open(to.parent().unwrap())?.sync_all()?;
    Ok(())
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

    // R7: each chunk observes changing free space and real temp/orphan quota;
    // unknown observations stop the stream without modifying committed bytes.
    #[test]
    fn resource_bounds_stream_rechecks_space_and_preserves_objects() {
        let dir = std::env::temp_dir().join(format!("assetq-stream-{}", uuid::Uuid::new_v4()));
        let s = Store::open(&dir).unwrap().with_bounds(1 << 30, 1 << 30, 1024);
        let original = s.put(&mut &b"retained"[..], 100, 100).unwrap();
        let calls = std::cell::Cell::new(0);
        let available = || {
            let n = calls.get();
            calls.set(n + 1);
            Ok(if n < 2 { 1 << 30 } else { 1024 })
        };
        let tmp = dir.join("tmp/changing-space");
        assert!(matches!(s.write_tmp(&tmp, &mut &vec![1; 128 << 10][..], 1 << 20, 1 << 20, available), Err(StoreError::Full(_))));
        assert_eq!(fs::metadata(&tmp).unwrap().len(), 64 << 10, "second chunk must not reach disk");
        fs::remove_file(&tmp).unwrap();
        assert!(matches!(
            s.write_tmp(&tmp, &mut &b"x"[..], 100, 100, || Err(io::Error::other("invented unavailable observation"))),
            Err(StoreError::Unknown(_))
        ));
        fs::remove_file(tmp).unwrap();
        let locked = s.writer_lock().unwrap();
        assert!(matches!(s.put(&mut &b"x"[..], 100, 100), Err(StoreError::Full(_))));
        drop(locked);
        fs::write(dir.join("tmp/crash-leftover"), b"12345678").unwrap();
        let tiny = s.clone().with_bounds(16, 8, 0);
        assert!(matches!(tiny.put(&mut &b"x"[..], 100, 100), Err(StoreError::Full(_))));
        s.verify(&original.sha256, original.size).unwrap();
        fs::remove_dir_all(dir).unwrap();
    }

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
