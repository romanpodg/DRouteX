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

pub(in crate::deployment) struct FileStore {
    root: PathBuf,
    uid: u32,
    _lock: File,
}
impl FileStore {
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
        DirBuilder::new().mode(0o700).create(root).map_err(io)?;
        let lock = create(&root.join("store.lock"), uid)?;
        lock.try_lock().map_err(|_| Error(ErrorCode::Busy))?;
        let store = Self {
            root: root.to_path_buf(),
            uid,
            _lock: lock,
        };
        store.commit(journal)?;
        sync_directory(parent)?;
        Ok(store)
    }
    pub(super) fn open(root: &Path, node: NodeScopeId) -> Result<(Self, Journal)> {
        safe_absolute(root)?;
        if fs::canonicalize(root).map_err(io)? != root {
            return Err(unsafe_path());
        }
        let uid = fs::metadata("/proc/self").map_err(io)?.uid();
        check_directory(root, uid)?;
        let lock = open_file(&root.join("store.lock"), uid, 0, true)?;
        lock.try_lock().map_err(|_| Error(ErrorCode::Busy))?;
        let store = Self {
            root: root.to_path_buf(),
            uid,
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
    pub(super) fn stage(
        &self,
        candidate: Generation,
        descriptor: &Descriptor,
        trust: &SnapshotTrust,
        cancel: &Cancellation,
    ) -> Result<Generation> {
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
        Generation::restore(envelope, Artifacts::new(entries)?, trust, descriptor)
    }
    pub(super) fn commit(&self, journal: &Journal) -> Result<()> {
        let bytes = serde_json::to_vec(journal).map_err(|_| Error(ErrorCode::State))?;
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(Error(ErrorCode::Size));
        }
        let next = self.root.join("manifest.next");
        self.remove_file_if_present(&next)?;
        write(&next, self.uid, &bytes)?;
        checkpoint("manifest-prepared")?;
        // Any error from the rename onward has an uncertain durable outcome.
        fs::rename(&next, self.root.join("manifest.json"))
            .map_err(|_| Error(ErrorCode::UncertainCommit))?;
        checkpoint("manifest-renamed").map_err(|_| Error(ErrorCode::UncertainCommit))?;
        sync_directory(&self.root).map_err(|_| Error(ErrorCode::UncertainCommit))?;
        checkpoint("manifest-durable").map_err(|_| Error(ErrorCode::UncertainCommit))?;
        Ok(())
    }
    pub(super) fn recover(&self, journal: &Journal) -> Result<()> {
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
        sync_directory(&self.root)
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
        for (count, entry) in fs::read_dir(path).map_err(io)?.enumerate() {
            if count > MAX_RESOURCE_PINS {
                return Err(Error(ErrorCode::Size));
            }
            let entry = entry.map_err(io)?;
            let name = entry.file_name();
            let name = name.to_str().ok_or_else(unsafe_path)?;
            if name != "snapshot" {
                parse_filename(name)?;
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
