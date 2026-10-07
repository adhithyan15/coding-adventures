//! Native Unix regressions precede the UID/GID/mode and ACL copy-or-reject repair.
use super::*;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

#[test]
fn mode_only_drift_rejects_without_identity_length_or_mtime_change() {
    let fixture = Fixture::new();
    let path = fixture.path("output");
    fs::write(&path, "original").unwrap();
    fs::set_permissions(&path, Permissions::from_mode(0o640)).unwrap();
    let original = observe(&path, false).unwrap().unwrap();
    let result = publish_with_hook(&[(path.clone(), "replacement".into())], &mut |phase, _| {
        if phase == Phase::Install {
            fs::set_permissions(&path, Permissions::from_mode(0o600))?;
        }
        Ok(())
    });
    assert!(result.is_err(), "mode-only drift was accepted");
    let current = observe(&path, false).unwrap().unwrap();
    assert_eq!(current.id, original.id);
    assert_eq!(current.len, original.len);
    assert_eq!(current.modified, original.modified);
    assert_eq!(current.permissions.mode() & 0o7777, 0o600);
    assert_eq!(fs::read(path).unwrap(), b"original");
    assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 1);
}

#[test]
fn ordinary_replacement_preserves_owner_group_and_mode() {
    let fixture = Fixture::new();
    let path = fixture.path("output");
    fs::write(&path, "original").unwrap();
    fs::set_permissions(&path, Permissions::from_mode(0o640)).unwrap();
    let original = fs::metadata(&path).unwrap();
    publish_outputs(&[(path.clone(), "replacement".into())]).unwrap();
    let installed = fs::metadata(&path).unwrap();
    assert_eq!(installed.uid(), original.uid());
    assert_eq!(installed.gid(), original.gid());
    assert_eq!(installed.mode() & 0o7777, original.mode() & 0o7777);
    assert_eq!(fs::read(path).unwrap(), b"replacement");
    assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 1);
}

#[cfg(target_os = "linux")]
fn attach_extended_acl(path: &Path, default: bool) {
    use std::ffi::c_void;
    use std::os::fd::AsRawFd;
    unsafe extern "C" {
        fn fsetxattr(
            fd: i32,
            name: *const i8,
            value: *const c_void,
            size: usize,
            flags: i32,
        ) -> i32;
    }
    let file = File::open(path).unwrap();
    let other_uid = file.metadata().unwrap().uid().checked_add(1).unwrap();
    // Linux POSIX ACL xattr v2: header, then sorted tag/permissions/ID entries.
    // The named user plus mask make this a genuine extended ACL, not mode bits.
    let mut acl = 2u32.to_le_bytes().to_vec();
    for (tag, permissions, id) in [
        (1u16, 6u16, u32::MAX),
        (2, 4, other_uid),
        (4, 0, u32::MAX),
        (16, 4, u32::MAX),
        (32, 0, u32::MAX),
    ] {
        acl.extend_from_slice(&tag.to_le_bytes());
        acl.extend_from_slice(&permissions.to_le_bytes());
        acl.extend_from_slice(&id.to_le_bytes());
    }
    let name = if default {
        c"system.posix_acl_default"
    } else {
        c"system.posix_acl_access"
    };
    // SAFETY: live borrowed descriptor, terminated constant name and initialized
    // ACL bytes are valid for this synchronous Linux-only call; no ownership moves.
    let result = unsafe {
        fsetxattr(
            file.as_raw_fd(),
            name.as_ptr(),
            acl.as_ptr().cast(),
            acl.len(),
            0,
        )
    };
    assert_eq!(
        result,
        0,
        "native ACL fixture unavailable: {}",
        io::Error::last_os_error()
    );
}

#[cfg(target_os = "linux")]
#[test]
fn extended_access_acl_rejects_before_original_mutation() {
    let fixture = Fixture::new();
    let path = fixture.path("output");
    fs::write(&path, "original").unwrap();
    attach_extended_acl(&path, false);
    let mut mutated = false;
    let result = publish_with_hook(&[(path.clone(), "replacement".into())], &mut |phase, _| {
        if phase == Phase::BackupRemove {
            mutated = true;
        }
        Ok(())
    });
    assert!(result.is_err(), "unsupported extended ACL was accepted");
    assert!(!mutated);
    assert_eq!(fs::read(path).unwrap(), b"original");
    assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 1);
}

#[cfg(target_os = "linux")]
#[test]
fn parent_default_acl_rejects_before_staging_or_missing_parent_creation() {
    let fixture = Fixture::new();
    attach_extended_acl(&fixture.dir, true);
    let path = fixture.path("missing/output");
    assert!(publish_outputs(&[(path, "body".into())]).is_err());
    assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 0);
}

#[cfg(target_os = "macos")]
#[test]
fn extended_acl_rejects_before_original_mutation() {
    let fixture = Fixture::new();
    let path = fixture.path("output");
    fs::write(&path, "original").unwrap();
    let output = std::process::Command::new("chmod")
        .args(["+a", "everyone allow read"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut mutated = false;
    let result = publish_with_hook(&[(path.clone(), "replacement".into())], &mut |phase, _| {
        if phase == Phase::BackupRemove {
            mutated = true;
        }
        Ok(())
    });
    assert!(result.is_err(), "unsupported extended ACL was accepted");
    assert!(!mutated);
    assert_eq!(fs::read(path).unwrap(), b"original");
    assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 1);
}
