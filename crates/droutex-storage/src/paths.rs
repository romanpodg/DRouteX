use crate::{StorageError, StorageErrorCode};
use std::{fs::File, path::PathBuf};

pub(crate) struct Paths {
    pub directory: PathBuf,
    pub database: PathBuf,
    pub lock: File,
}

pub(crate) struct BackupTarget {
    pub pending: PathBuf,
    pub complete: PathBuf,
}

pub(crate) fn filesystem(error: std::io::Error) -> StorageError {
    StorageError::new(if error.kind() == std::io::ErrorKind::StorageFull {
        StorageErrorCode::DiskFull
    } else {
        StorageErrorCode::Filesystem
    })
}

pub(crate) fn prepare(directory: PathBuf) -> Result<Paths, StorageError> {
    #[cfg(target_os = "linux")]
    {
        linux::prepare(directory)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = directory;
        Err(StorageError::new(StorageErrorCode::UnsupportedPlatform))
    }
}

pub(crate) fn verify_files(directory: PathBuf) -> Result<(), StorageError> {
    #[cfg(target_os = "linux")]
    {
        linux::verify_files(directory)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = directory;
        Err(StorageError::new(StorageErrorCode::UnsupportedPlatform))
    }
}

pub(crate) fn backup_target(
    directory: PathBuf,
    version: i64,
) -> Result<BackupTarget, StorageError> {
    #[cfg(target_os = "linux")]
    {
        linux::backup_target(directory, version)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (directory, version);
        Err(StorageError::new(StorageErrorCode::UnsupportedPlatform))
    }
}

pub(crate) fn finish_backup(target: BackupTarget) -> Result<(), StorageError> {
    // The service owns this private directory and holds its exclusive storage lock.
    if target.complete.try_exists().map_err(filesystem)? {
        return Err(StorageError::new(StorageErrorCode::UnsafePath));
    }
    std::fs::rename(&target.pending, &target.complete).map_err(filesystem)?;
    File::open(&target.complete)
        .and_then(|file| file.sync_all())
        .map_err(filesystem)?;
    let parent = target
        .complete
        .parent()
        .ok_or_else(|| StorageError::new(StorageErrorCode::UnsafePath))?;
    File::open(parent)
        .and_then(|file| file.sync_all())
        .map_err(filesystem)
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::{
        fs::{self, DirBuilder, OpenOptions},
        io::ErrorKind,
        os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        path::Path,
        sync::atomic::{AtomicU64, Ordering},
    };

    static BACKUP_NUMBER: AtomicU64 = AtomicU64::new(0);

    fn private_file(path: &Path) -> Result<(), StorageError> {
        let metadata = fs::symlink_metadata(path).map_err(filesystem)?;
        if !metadata.is_file()
            || metadata.permissions().mode() & 0o777 != 0o600
            || metadata.nlink() != 1
        {
            return Err(StorageError::new(StorageErrorCode::UnsafePath));
        }
        Ok(())
    }

    fn open_private(path: &Path) -> Result<File, StorageError> {
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
        {
            Ok(file) => Ok(file),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                private_file(path)?;
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(path)
                    .map_err(filesystem)
            }
            Err(error) => Err(filesystem(error)),
        }
    }

    pub(super) fn prepare(directory: PathBuf) -> Result<Paths, StorageError> {
        if directory.to_str().is_none() {
            return Err(StorageError::new(StorageErrorCode::UnsafePath));
        }
        match DirBuilder::new().mode(0o700).create(&directory) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(error) => return Err(filesystem(error)),
        }
        let metadata = fs::symlink_metadata(&directory).map_err(filesystem)?;
        if !metadata.is_dir() || metadata.permissions().mode() & 0o777 != 0o700 {
            return Err(StorageError::new(StorageErrorCode::UnsafePath));
        }
        let directory = fs::canonicalize(directory).map_err(filesystem)?;
        let lock = open_private(&directory.join("droutex.lock"))?;
        lock.try_lock()
            .map_err(|_| StorageError::new(StorageErrorCode::Busy))?;
        let database = directory.join("droutex.db");
        drop(open_private(&database)?);
        verify_files(directory.clone())?;
        Ok(Paths {
            directory,
            database,
            lock,
        })
    }

    pub(super) fn verify_files(directory: PathBuf) -> Result<(), StorageError> {
        for suffix in ["", "-wal", "-shm", "-journal"] {
            let path = directory.join(format!("droutex.db{suffix}"));
            match fs::symlink_metadata(&path) {
                Ok(_) => private_file(&path)?,
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(filesystem(error)),
            }
        }
        Ok(())
    }

    pub(super) fn backup_target(
        directory: PathBuf,
        version: i64,
    ) -> Result<BackupTarget, StorageError> {
        for _ in 0..64 {
            let number = BACKUP_NUMBER.fetch_add(1, Ordering::Relaxed);
            let base = format!("migration-{version}-{}-{number}", std::process::id());
            let pending = directory.join(format!("{base}.pending"));
            let complete = directory.join(format!("{base}.backup.db"));
            if complete.try_exists().map_err(filesystem)? {
                continue;
            }
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&pending)
            {
                Ok(file) => {
                    file.sync_all().map_err(filesystem)?;
                    return Ok(BackupTarget { pending, complete });
                }
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
                Err(error) => return Err(filesystem(error)),
            }
        }
        Err(StorageError::new(StorageErrorCode::Filesystem))
    }
}
