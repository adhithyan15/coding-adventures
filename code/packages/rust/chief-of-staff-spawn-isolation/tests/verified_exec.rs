//! `VerifiedExecutable` (D18S S-K1; P2.6d-2a): what it accepts, and that a
//! change after it was opened is seen through its descriptor. Launching one,
//! with its descriptors at 3..3+n, is `chief-of-staff-linux-sandbox`'s
//! (P2.6d-3), and is tested there.

#![cfg(target_os = "linux")]

use chief_of_staff_spawn_isolation::{VerifiedExecutable, VerifyError};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

const PROBE: &str = env!("CARGO_BIN_EXE_spawn-isolation-probe");

fn digest(path: &Path) -> [u8; 32] {
    coding_adventures_sha256::sha256(&fs::read(path).unwrap())
}

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "verified-exec-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(fs::canonicalize(path).unwrap())
    }

    fn file(&self, name: &str, bytes: &[u8], mode: u32) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_binary_is_checked_before_it_is_trusted() {
    let scratch = Scratch::new();
    let original = fs::read(PROBE).unwrap();
    let right = digest(Path::new(PROBE));

    assert_eq!(
        VerifiedExecutable::open(Path::new("relative/broker"), right).unwrap_err(),
        VerifyError::NotAbsolute
    );
    assert_eq!(
        VerifiedExecutable::open(&scratch.0, right).unwrap_err(),
        VerifyError::NotRegular
    );
    let copy = scratch.file("broker", &original, 0o755);
    assert!(VerifiedExecutable::open(&copy, right).is_ok());
    assert_eq!(
        VerifiedExecutable::open(&copy, [0; 32]).unwrap_err(),
        VerifyError::DigestMismatch
    );
    let shared = scratch.file("shared-broker", &original, 0o775);
    assert_eq!(
        VerifiedExecutable::open(&shared, right).unwrap_err(),
        VerifyError::Writable
    );
}

#[test]
fn a_binary_changed_after_it_was_verified_is_not_executed() {
    let scratch = Scratch::new();
    let copy = scratch.file("broker", &fs::read(PROBE).unwrap(), 0o755);
    let verified = VerifiedExecutable::open(&copy, digest(&copy)).unwrap();
    // Same inode, new contents: the descriptor sees the change.
    let mut tampered = fs::OpenOptions::new().append(true).open(&copy).unwrap();
    std::io::Write::write_all(&mut tampered, b"\0appended").unwrap();
    drop(tampered);
    assert_eq!(verified.verify().unwrap_err(), VerifyError::DigestMismatch);
}

#[test]
fn the_descriptor_handed_out_is_the_file_that_was_hashed() {
    let scratch = Scratch::new();
    let copy = scratch.file("broker", &fs::read(PROBE).unwrap(), 0o755);
    let verified = VerifiedExecutable::open(&copy, digest(&copy)).unwrap();
    // Replace the path: the descriptor still names the original file.
    fs::remove_file(&copy).unwrap();
    fs::write(&copy, b"something else").unwrap();
    let handed = verified.descriptor().unwrap();
    assert_ne!(
        handed.metadata().unwrap().ino(),
        fs::metadata(&copy).unwrap().ino()
    );
    assert!(verified.verify().is_ok());
}
