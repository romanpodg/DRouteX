use crate::{SecretError, SecretErrorCode};
use std::path::Path;

pub(crate) fn read(path: &Path, database: &Path, backup: &Path) -> Result<Vec<u8>, SecretError> {
    read_bounded(path, database, backup, crate::keyring::MAX_KEYRING_BYTES)
}

pub(crate) fn read_bounded(
    path: &Path,
    database: &Path,
    backup: &Path,
    max_bytes: usize,
) -> Result<Vec<u8>, SecretError> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        linux::read(path, database, backup, max_bytes)
    }
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    {
        let _ = (path, database, backup, max_bytes);
        Err(SecretError(SecretErrorCode::UnsupportedPlatform))
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod linux {
    use super::*;
    use std::{
        fs::{self, OpenOptions},
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

    pub(super) fn read(
        path: &Path,
        database: &Path,
        backup: &Path,
        max_bytes: usize,
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
        // Linux preserves effective ownership of the world-readable/executable
        // process directory even for non-dumpable tasks (task_dump_owner exception).
        let uid = fs::metadata("/proc/self").map_err(io)?.uid();
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
            || metadata.len() > max_bytes as u64
            || !before.is_file()
            || before.ino() != metadata.ino()
            || before.dev() != metadata.dev()
        {
            return Err(unsafe_path());
        }
        let mut bytes = Zeroizing::new(Vec::with_capacity(max_bytes + 1));
        file.take((max_bytes + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(io)?;
        if bytes.len() > max_bytes {
            return Err(unsafe_path());
        }
        Ok(std::mem::take(&mut *bytes))
    }
}
