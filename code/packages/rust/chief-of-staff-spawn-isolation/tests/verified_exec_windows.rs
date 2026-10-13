//! `VerifiedExecutable` on Windows (D18S S-K1, step 8): while it is held,
//! the file it hashed can be neither changed, renamed nor deleted, so the
//! bytes at its path are the bytes that were hashed. Launching it is
//! `chief-of-staff-windows-sandbox`'s, and is tested there.

#![cfg(windows)]

use chief_of_staff_spawn_isolation::{VerifiedExecutable, VerifyError};
use std::fs;
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
        Self(path)
    }

    fn copy_of_probe(&self) -> PathBuf {
        let path = self.0.join("probe.exe");
        fs::copy(PROBE, &path).unwrap();
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
    let path = scratch.copy_of_probe();
    let pinned = digest(&path);
    assert!(VerifiedExecutable::open(&path, pinned).is_ok());

    let mut wrong = pinned;
    wrong[0] ^= 1;
    assert_eq!(
        VerifiedExecutable::open(&path, wrong).err(),
        Some(VerifyError::DigestMismatch)
    );
    assert_eq!(
        VerifiedExecutable::open(Path::new("relative.exe"), pinned).err(),
        Some(VerifyError::NotAbsolute)
    );
    assert_eq!(
        VerifiedExecutable::open(&scratch.0.join("absent.exe"), pinned).err(),
        Some(VerifyError::Unreadable)
    );
    // A directory opens (with backup semantics it would), but is no binary.
    assert!(VerifiedExecutable::open(&scratch.0, pinned).is_err());
}

#[test]
fn a_held_binary_cannot_be_changed_renamed_or_deleted() {
    let scratch = Scratch::new();
    let path = scratch.copy_of_probe();
    let held = VerifiedExecutable::open(&path, digest(&path)).unwrap();

    // Each is a sharing violation while the handle is held.
    assert!(fs::OpenOptions::new().write(true).open(&path).is_err());
    assert!(fs::rename(&path, scratch.0.join("moved.exe")).is_err());
    assert!(fs::remove_file(&path).is_err());
    // Reading is still allowed: that is what a launch does.
    assert!(fs::read(&path).is_ok());
    assert!(held.verify().is_ok());

    drop(held);
    // Released, the file is an ordinary file again.
    assert!(fs::OpenOptions::new().write(true).open(&path).is_ok());
}

#[test]
fn the_handle_handed_out_is_the_file_that_was_hashed() {
    let scratch = Scratch::new();
    let path = scratch.copy_of_probe();
    let held = VerifiedExecutable::open(&path, digest(&path)).unwrap();
    let copy = held.descriptor().unwrap();
    assert_eq!(
        copy.metadata().unwrap().len(),
        fs::metadata(&path).unwrap().len()
    );
    assert_eq!(held.path(), path.as_path());
    assert_eq!(held.digest(), digest(&path));
}
