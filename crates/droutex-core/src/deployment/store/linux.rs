use super::*;
use crate::deployment::{
    Artifacts, MAX_MANIFEST_BYTES, MAX_RESOURCE_BYTES, artifacts::parse_filename,
};
use droutex_config::snapshot::{MAX_RESOURCE_PINS, MAX_SNAPSHOT_BYTES};
use std::{
    fs::{self, DirBuilder, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Component, PathBuf},
};

fn io(_: std::io::Error) -> Error {
    Error(ErrorCode::Filesystem)
}
fn unsafe_path() -> Error {
    Error(ErrorCode::UnsafePath)
}
fn lock_error(error: fs::TryLockError) -> Error {
    match error {
        fs::TryLockError::WouldBlock => Error(ErrorCode::Busy),
        fs::TryLockError::Error(error) => io(error),
    }
}

pub(in crate::deployment) struct FileStore {
    root: PathBuf,
    uid: u32,
    identity: (u64, u64),
    _lock: File,
}
impl Drop for FileStore {
    fn drop(&mut self) {
        // File close alone retains flock while a duplicated/inherited open-file
        // description remains alive (including a concurrent spawn before exec).
        // This non-Clone owner releases its lock explicitly before closing. Drop
        // cannot report an unlock error; closing the owned file remains the fallback.
        let _ = self._lock.unlock();
    }
}
impl FileStore {
    #[cfg(test)]
    pub(in crate::deployment) fn duplicate_lock_for_test(&self) -> std::io::Result<File> {
        self._lock.try_clone()
    }
    pub(super) fn provision(
        root: &Path,
        database: &Path,
        backup: &Path,
        journal: &Journal,
    ) -> Result<Self> {
        safe_absolute(root)?;
        for cp in [database, backup] {
            let cp = fs::canonicalize(cp).map_err(io)?;
            if root.starts_with(&cp) || cp.starts_with(root) {
                return Err(unsafe_path());
            }
        }
        let parent = root.parent().ok_or_else(unsafe_path)?;
        if fs::canonicalize(parent).map_err(io)? != parent {
            return Err(unsafe_path());
        }
        let uid = fs::metadata("/proc/self").map_err(io)?.uid();
        if uid == 0 {
            return Err(unsafe_path());
        }
        check_ancestors(parent, uid)?;
        DirBuilder::new().mode(0o700).create(root).map_err(io)?;
        let lock = create(&root.join("store.lock"), uid)?;
        lock.try_lock().map_err(lock_error)?;
        let store = Self {
            root: root.to_path_buf(),
            uid,
            identity: directory_identity(root)?,
            _lock: lock,
        };
        store.commit(journal, &Cancellation::default())?;
        sync_directory(parent)?;
        Ok(store)
    }
    pub(super) fn open(root: &Path, node: NodeScopeId) -> Result<(Self, Journal)> {
        safe_absolute(root)?;
        if fs::canonicalize(root).map_err(io)? != root {
            return Err(unsafe_path());
        }
        let uid = fs::metadata("/proc/self").map_err(io)?.uid();
        check_ancestors(root.parent().ok_or_else(unsafe_path)?, uid)?;
        check_directory(root, uid)?;
        let lock = open_file(&root.join("store.lock"), uid, 0, true)?;
        lock.try_lock().map_err(lock_error)?;
        let store = Self {
            root: root.to_path_buf(),
            uid,
            identity: directory_identity(root)?,
            _lock: lock,
        };
        let bytes = read(&root.join("manifest.json"), uid, MAX_MANIFEST_BYTES)?;
        let journal: Journal =
            serde_json::from_slice(&bytes).map_err(|_| Error(ErrorCode::State))?;
        journal.validate(node)?;
        // Refuse unexpected root objects rather than growing or deleting arbitrary paths.
        for entry in fs::read_dir(root).map_err(io)? {
            let entry = entry.map_err(io)?;
            if !matches!(
                entry.file_name().to_str(),
                Some(
                    "manifest.json"
                        | "manifest.next"
                        | "store.lock"
                        | "slot-0"
                        | "slot-1"
                        | "slot-2"
                )
            ) {
                return Err(unsafe_path());
            }
        }
        Ok((store, journal))
    }
    fn slot(&self, slot: u8) -> Result<PathBuf> {
        if slot > 2 {
            return Err(Error(ErrorCode::State));
        }
        Ok(self.root.join(format!("slot-{slot}")))
    }
    fn validate_root(&self) -> Result<()> {
        check_directory(&self.root, self.uid)?;
        if directory_identity(&self.root)? != self.identity {
            return Err(unsafe_path());
        }
        check_ancestors(self.root.parent().ok_or_else(unsafe_path)?, self.uid)
    }
    pub(super) fn stage(
        &self,
        candidate: Generation,
        descriptor: &Descriptor,
        trust: &SnapshotTrust,
        cancel: &Cancellation,
    ) -> Result<Generation> {
        self.validate_root()?;
        let path = self.slot(descriptor.slot)?;
        self.remove_slot(&path)?;
        cancel.check()?;
        DirBuilder::new().mode(0o700).create(&path).map_err(io)?;
        checkpoint("stage-directory")?;
        write(&path.join("snapshot"), self.uid, &candidate.envelope)?;
        checkpoint("stage-envelope")?;
        for (name, (_, bytes)) in &candidate.artifacts.entries {
            cancel.check()?;
            write(&path.join(name), self.uid, bytes)?;
            checkpoint("stage-resource")?;
        }
        sync_directory(&path)?;
        sync_directory(&self.root)?;
        checkpoint("stage-durable")?;
        // Do not publish the in-memory input if the persisted copy was changed or truncated.
        self.restore(descriptor, trust)
    }
    pub(super) fn restore(
        &self,
        descriptor: &Descriptor,
        trust: &SnapshotTrust,
    ) -> Result<Generation> {
        self.validate_root()?;
        let (envelope, artifacts) = self.read_slot(descriptor)?;
        Generation::restore(envelope, artifacts, trust, descriptor)
    }
    fn read_slot(&self, descriptor: &Descriptor) -> Result<(Vec<u8>, Artifacts)> {
        let path = self.slot(descriptor.slot)?;
        check_directory(&path, self.uid)?;
        let envelope = read(&path.join("snapshot"), self.uid, MAX_SNAPSHOT_BYTES)?;
        let mut entries = Vec::new();
        let mut total = 0_usize;
        for entry in fs::read_dir(&path).map_err(io)? {
            let name = entry.map_err(io)?.file_name();
            let name = name.to_str().ok_or_else(unsafe_path)?;
            if name == "snapshot" {
                continue;
            }
            if entries.len() == MAX_RESOURCE_PINS {
                return Err(Error(ErrorCode::Size));
            }
            let resource = parse_filename(name)?;
            let bytes = read(&path.join(name), self.uid, MAX_RESOURCE_BYTES - total)?;
            total = total
                .checked_add(bytes.len())
                .ok_or(Error(ErrorCode::Size))?;
            entries.push((resource, bytes));
        }
        Ok((envelope, Artifacts::new(entries)?))
    }
    pub(super) fn commit(&self, journal: &Journal, cancel: &Cancellation) -> Result<()> {
        self.validate_root()?;
        cancel.check()?;
        let bytes = serde_json::to_vec(journal).map_err(|_| Error(ErrorCode::State))?;
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(Error(ErrorCode::Size));
        }
        let next = self.root.join("manifest.next");
        self.remove_file_if_present(&next)?;
        write(&next, self.uid, &bytes)?;
        checkpoint("manifest-prepared")?;
        cancel.check()?;
        self.validate_root()?;
        // Any error from the rename onward has an uncertain durable outcome.
        fs::rename(&next, self.root.join("manifest.json"))
            .map_err(|_| Error(ErrorCode::UncertainCommit))?;
        checkpoint("manifest-renamed").map_err(|_| Error(ErrorCode::UncertainCommit))?;
        sync_directory(&self.root).map_err(|_| Error(ErrorCode::UncertainCommit))?;
        checkpoint("manifest-durable").map_err(|_| Error(ErrorCode::UncertainCommit))?;
        Ok(())
    }
    pub(super) fn recover(&self, journal: &Journal) -> Result<()> {
        self.validate_root()?;
        // Retention has no admission authority, but every retained byte/path still
        // obeys D36. A revoked previous signer is deliberately not required here.
        if let Some(previous) = &journal.previous {
            self.read_slot(previous)?;
        }
        // A visible post-rename manifest may still be the outcome of an uncertain
        // commit. Make this selection durable before deleting anything that the
        // former durable manifest could have retained as active/previous.
        checkpoint("recovery-before-sync")?;
        sync_directory(&self.root)?;
        checkpoint("recovery-durable")?;
        self.remove_file_if_present(&self.root.join("manifest.next"))?;
        for slot in 0..3 {
            if journal
                .active
                .as_ref()
                .is_none_or(|active| active.slot != slot)
                && journal
                    .previous
                    .as_ref()
                    .is_none_or(|previous| previous.slot != slot)
            {
                self.remove_slot(&self.slot(slot)?)?;
            }
        }
        sync_directory(&self.root)?;
        checkpoint("recovery-cleaned")
    }
    fn remove_file_if_present(&self, path: &Path) -> Result<()> {
        match fs::symlink_metadata(path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(io(error)),
            Ok(_) => {
                open_file(path, self.uid, MAX_RESOURCE_BYTES, false)?;
                fs::remove_file(path).map_err(io)
            }
        }
    }
    fn remove_slot(&self, path: &Path) -> Result<()> {
        match fs::symlink_metadata(path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(io(error)),
            Ok(_) => check_directory(path, self.uid)?,
        }
        // Flat, bounded, typed entries only. Never recurse through a substituted link.
        let mut total = 0_usize;
        for (count, entry) in fs::read_dir(path).map_err(io)?.enumerate() {
            if count > MAX_RESOURCE_PINS {
                return Err(Error(ErrorCode::Size));
            }
            let entry = entry.map_err(io)?;
            let name = entry.file_name();
            let name = name.to_str().ok_or_else(unsafe_path)?;
            let max = if name == "snapshot" {
                MAX_SNAPSHOT_BYTES
            } else {
                MAX_RESOURCE_BYTES - total
            };
            let file = open_file(&path.join(name), self.uid, max, false)?;
            if name != "snapshot" {
                parse_filename(name)?;
                total = total
                    .checked_add(file.metadata().map_err(io)?.len() as usize)
                    .filter(|total| *total <= MAX_RESOURCE_BYTES)
                    .ok_or(Error(ErrorCode::Size))?;
            }
            self.remove_file_if_present(&path.join(name))?;
        }
        fs::remove_dir(path).map_err(io)?;
        sync_directory(&self.root)
    }
}

fn safe_absolute(path: &Path) -> Result<()> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::RootDir | Component::Normal(_)))
    {
        return Err(unsafe_path());
    }
    Ok(())
}
fn directory_identity(path: &Path) -> Result<(u64, u64)> {
    let metadata = fs::symlink_metadata(path).map_err(io)?;
    Ok((metadata.dev(), metadata.ino()))
}
fn check_ancestors(path: &Path, uid: u32) -> Result<()> {
    // Trust root/service ownership only. Root-owned sticky shared directories
    // (e.g. /tmp) cannot have service-owned children renamed by another UID.
    // A non-sticky group/world-writable ancestor is never a safe store location.
    for ancestor in path.ancestors() {
        let metadata = fs::symlink_metadata(ancestor).map_err(io)?;
        let writable = metadata.mode() & 0o022 != 0;
        let sticky = metadata.mode() & 0o1000 != 0;
        if !metadata.is_dir()
            || (metadata.uid() != 0 && metadata.uid() != uid)
            || (writable && !sticky)
        {
            return Err(unsafe_path());
        }
    }
    Ok(())
}
fn check_directory(path: &Path, uid: u32) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(io)?;
    if uid == 0 || !metadata.is_dir() || metadata.uid() != uid || metadata.mode() & 0o7777 != 0o700
    {
        return Err(unsafe_path());
    }
    Ok(())
}
fn check_file(metadata: &fs::Metadata, uid: u32, max: usize) -> Result<()> {
    if !metadata.is_file()
        || metadata.uid() != uid
        || metadata.mode() & 0o7777 != 0o600
        || metadata.nlink() != 1
        || metadata.len() > max as u64
    {
        return Err(unsafe_path());
    }
    Ok(())
}
fn create(path: &Path, uid: u32) -> Result<File> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(0x20000 | 0x800)
        .open(path)
        .map_err(io)?;
    check_file(&file.metadata().map_err(io)?, uid, 0)?;
    Ok(file)
}
fn open_file(path: &Path, uid: u32, max: usize, writable: bool) -> Result<File> {
    let before = fs::symlink_metadata(path).map_err(io)?;
    let file = OpenOptions::new()
        .read(true)
        .write(writable)
        .custom_flags(0x20000 | 0x800)
        .open(path)
        .map_err(io)?;
    let metadata = file.metadata().map_err(io)?;
    check_file(&metadata, uid, max)?;
    if !before.is_file() || before.ino() != metadata.ino() || before.dev() != metadata.dev() {
        return Err(unsafe_path());
    }
    Ok(file)
}
fn read(path: &Path, uid: u32, max: usize) -> Result<Vec<u8>> {
    let file = open_file(path, uid, max, false)?;
    let mut bytes = Vec::new();
    file.take((max + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() > max {
        return Err(Error(ErrorCode::Size));
    }
    Ok(bytes)
}
fn write(path: &Path, uid: u32, bytes: &[u8]) -> Result<()> {
    let mut file = create(path, uid)?;
    checkpoint("file-created")?;
    file.write_all(bytes).map_err(io)?;
    checkpoint("file-written")?;
    file.sync_all().map_err(io)?;
    checkpoint("file-synced")
}
fn sync_directory(path: &Path) -> Result<()> {
    File::open(path).map_err(io)?.sync_all().map_err(io)
}

// Fault/crash hooks exist only in the unit-test binary, never in a service build.
fn checkpoint(phase: &str) -> Result<()> {
    #[cfg(test)]
    {
        super::super::tests::checkpoint(phase)?;
    }
    #[cfg(not(test))]
    {
        let _ = phase;
    }
    Ok(())
}
