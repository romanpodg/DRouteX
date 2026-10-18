use super::*;
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt, symlink},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn protected_file_loader_refuses_modes_links_and_storage_locations() {
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/key-tests");
    fs::create_dir_all(&base).unwrap();
    let root = fs::canonicalize(base).unwrap().join(format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    for name in ["keys", "db", "backups"] {
        fs::DirBuilder::new()
            .mode(0o700)
            .create(root.join(name))
            .unwrap();
    }
    let path = root.join("keys/ring");
    fs::write(&path, key_bytes(1, &[(1, 7)])).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let load =
        |path: &std::path::Path| Keyring::load_file(path, &root.join("db"), &root.join("backups"));
    assert!(load(&path).is_ok());
    for mode in [0o644, 0o400, 0o1600] {
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        assert!(load(&path).is_err());
    }
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::hard_link(&path, root.join("keys/hard")).unwrap();
    assert!(load(&path).is_err());
    fs::remove_file(root.join("keys/hard")).unwrap();
    symlink(&path, root.join("keys/symlink")).unwrap();
    assert!(load(&root.join("keys/symlink")).is_err());
    symlink(root.join("keys"), root.join("alias")).unwrap();
    assert!(load(&root.join("alias/ring")).is_err());
    fs::set_permissions(root.join("keys"), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(load(&path).is_err());
    fs::set_permissions(root.join("keys"), fs::Permissions::from_mode(0o700)).unwrap();
    for directory in ["db", "backups"] {
        let target = root.join(directory).join("ring");
        fs::copy(&path, &target).unwrap();
        assert!(load(&target).is_err());
    }
    assert!(load(&root.join("keys")).is_err());
    fs::write(&path, vec![1; 2000]).unwrap();
    assert!(load(&path).is_err());
    fs::remove_dir_all(root).unwrap();
}
