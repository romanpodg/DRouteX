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

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod linux {
    use super::*;
    use crate::keyring::MAX_KEYRING_BYTES;
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
        // Trusted Linux procfs supplies the current service identity without FFI.
        // A non-dumpable process whose proc ownership changes to root fails closed.
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
