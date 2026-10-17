use crate::{SecretError, SecretErrorCode};
use std::path::Path;

pub(crate) fn read(path: &Path, database: &Path, backup: &Path) -> Result<Vec<u8>, SecretError> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        linux::read(path, database, backup)
    }
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    {
        let _ = (path, database, backup);
        Err(SecretError(SecretErrorCode::UnsupportedPlatform))
    }
}

#[cfg(any(test, all(target_os = "linux", target_arch = "x86_64")))]
pub(crate) fn parse_effective_uid(status: &str) -> Result<u32, SecretError> {
    let invalid = || SecretError(SecretErrorCode::UnsafePath);
    let mut lines = status.lines().filter_map(|line| line.strip_prefix("Uid:"));
    let values = lines.next().ok_or_else(invalid)?;
    if lines.next().is_some() {
        return Err(invalid());
    }
    let mut values = values.split_ascii_whitespace();
    let mut ids = [0_u32; 4];
    for id in &mut ids {
        *id = values
            .next()
            .ok_or_else(invalid)?
            .parse()
            .map_err(|_| invalid())?;
    }
    if values.next().is_some() {
        return Err(invalid());
    }
    Ok(ids[1])
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod linux {
    use super::*;
    use crate::keyring::MAX_KEYRING_BYTES;
    use std::{
        fs::{self, File, OpenOptions},
        io::Read,
        os::unix::fs::{MetadataExt, OpenOptionsExt},
        path::Component,
    };
    use zeroize::Zeroizing;

    fn io(_: std::io::Error) -> SecretError {
        SecretError(SecretErrorCode::Filesystem)
    }
    fn unsafe_path() -> SecretError {
        SecretError(SecretErrorCode::UnsafePath)
    }

    fn effective_uid() -> Result<u32, SecretError> {
        let mut status = String::new();
        File::open("/proc/self/status")
            .map_err(io)?
            .take(65_537)
            .read_to_string(&mut status)
            .map_err(io)?;
        if status.len() > 65_536 {
            return Err(unsafe_path());
        }
        super::parse_effective_uid(&status)
    }

    pub(super) fn read(
        path: &Path,
        database: &Path,
        backup: &Path,
    ) -> Result<Vec<u8>, SecretError> {
        if !path.is_absolute()
            || path
                .components()
                .any(|c| !matches!(c, Component::RootDir | Component::Normal(_)))
        {
            return Err(unsafe_path());
        }
        let parent = path.parent().ok_or_else(unsafe_path)?;
        if fs::canonicalize(parent).map_err(io)? != parent {
            return Err(unsafe_path());
        }
        for root in [database, backup] {
            let root = fs::canonicalize(root).map_err(io)?;
            if path.starts_with(root) {
                return Err(unsafe_path());
            }
        }
        // Proc inode ownership can become root when dumpability is disabled.
        // The trusted kernel status fields retain the effective service UID.
        let uid = effective_uid()?;
        let directory = fs::symlink_metadata(parent).map_err(io)?;
        if uid == 0
            || !directory.is_dir()
            || directory.uid() != uid
            || directory.mode() & 0o7777 != 0o700
        {
            return Err(unsafe_path());
        }
        let before = fs::symlink_metadata(path).map_err(io)?;
        // Linux ABI O_NOFOLLOW|O_NONBLOCK prevents following a final symlink and
        // blocking on a substituted FIFO before opened-handle validation.
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(0x20000 | 0x800)
            .open(path)
            .map_err(io)?;
        let metadata = file.metadata().map_err(io)?;
        if !metadata.is_file()
            || metadata.uid() != uid
            || metadata.mode() & 0o7777 != 0o600
            || metadata.nlink() != 1
            || metadata.len() > MAX_KEYRING_BYTES as u64
            || !before.is_file()
            || before.ino() != metadata.ino()
            || before.dev() != metadata.dev()
        {
            return Err(unsafe_path());
        }
        let mut bytes = Zeroizing::new(Vec::with_capacity(MAX_KEYRING_BYTES + 1));
        file.take((MAX_KEYRING_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(io)?;
        if bytes.len() > MAX_KEYRING_BYTES {
            return Err(unsafe_path());
        }
        Ok(std::mem::take(&mut *bytes))
    }
}
