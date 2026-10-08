//! `isolate_and_exec` and `VerifiedExecutable` (D18S S-K1, S-I3; P2.6d-2a),
//! checked from inside the child: exactly the descriptors meant, in order,
//! by inode; nothing else; the binary that ran is the one that was hashed.

#![cfg(target_os = "linux")]

use chief_of_staff_spawn_isolation::{isolate_and_exec, VerifiedExecutable, VerifyError};
use std::fs;
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

const PROBE: &str = env!("CARGO_BIN_EXE_spawn-isolation-probe");

fn digest(path: &Path) -> [u8; 32] {
    coding_adventures_sha256::sha256(&fs::read(path).unwrap())
}

fn probe() -> VerifiedExecutable {
    VerifiedExecutable::open(Path::new(PROBE), digest(Path::new(PROBE))).unwrap()
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

fn field(report: &str, name: &str) -> String {
    report
        .lines()
        .find_map(|line| line.strip_prefix(name))
        .unwrap_or_else(|| panic!("{name} missing from {report:?}"))
        .to_string()
}

/// A descriptor left without close-on-exec, as a careless open would.
fn leaked() -> OwnedFd {
    // SAFETY: `dup` returns a new descriptor without FD_CLOEXEC, or -1.
    let fd = unsafe { libc::dup(1) };
    assert!(fd > 2);
    // SAFETY: just returned by `dup`; nothing else owns it.
    unsafe { OwnedFd::from_raw_fd(fd) }
}

#[test]
fn exactly_the_inherited_descriptors_arrive_in_order() {
    let scratch = Scratch::new();
    let files: Vec<fs::File> = (0..3)
        .map(|index| {
            let path = scratch.file(&format!("key-{index}"), b"k", 0o600);
            fs::File::open(path).unwrap()
        })
        .collect();
    let inodes: Vec<u64> = files
        .iter()
        .map(|file| file.metadata().unwrap().ino())
        .collect();
    let _leak = leaked();

    let mut command = Command::new("agent-broker");
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .env_clear()
        .env("CHIEF", "1")
        .arg("--serve");
    let inherited = files.into_iter().map(OwnedFd::from).collect();
    isolate_and_exec(&mut command, &probe(), inherited).unwrap();
    let output = command.output().unwrap();
    assert!(output.status.success());
    let report = String::from_utf8(output.stdout).unwrap();

    assert_eq!(field(&report, "open_above_2="), "3,4,5", "{report}");
    let expected: Vec<String> = inodes
        .iter()
        .enumerate()
        .map(|(index, inode)| format!("{}:{inode}", 3 + index))
        .collect();
    assert_eq!(field(&report, "inodes="), expected.join(","));
    assert_eq!(field(&report, "stderr_is_dev_null="), "true");
    assert_eq!(field(&report, "session_leader="), "true");
    // argv[0] is a name only; the arguments and environment are exactly the
    // command's.
    assert_eq!(field(&report, "args="), "--serve");
    assert_eq!(field(&report, "env="), "CHIEF=1");
}

#[test]
fn with_no_descriptors_nothing_above_stderr_arrives() {
    let _leak = leaked();
    let mut command = Command::new("agent-broker");
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .env_clear();
    isolate_and_exec(&mut command, &probe(), Vec::new()).unwrap();
    let report = String::from_utf8(command.output().unwrap().stdout).unwrap();
    assert_eq!(field(&report, "open_above_2="), "");
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
    let mut command = Command::new("agent-broker");
    let error = isolate_and_exec(&mut command, &verified, Vec::new()).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn a_failed_exec_after_the_descriptors_are_placed_exits_127() {
    // A file that hashes correctly but is not a program: the exec fails in
    // the child after the descriptors are placed, so the child exits 127
    // instead of the spawn failing.
    let scratch = Scratch::new();
    let not_a_program = scratch.file("broker", b"not a program", 0o755);
    let verified = VerifiedExecutable::open(&not_a_program, digest(&not_a_program)).unwrap();
    let key = scratch.file("key", b"k", 0o600);
    let mut command = Command::new("agent-broker");
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .env_clear();
    isolate_and_exec(
        &mut command,
        &verified,
        vec![OwnedFd::from(fs::File::open(key).unwrap())],
    )
    .unwrap();
    let status = command.status().unwrap();
    assert_eq!(status.code(), Some(127));
}

#[test]
fn a_terminal_on_a_standard_descriptor_still_refuses_the_spawn() {
    let mut master = -1;
    let mut slave = -1;
    // SAFETY: `openpty` writes two descriptors, or fails.
    let opened = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(opened, 0);
    // SAFETY: both were just opened and are owned here.
    let (_master, slave) = unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) };
    let mut command = Command::new("agent-broker");
    command.stdin(Stdio::from(slave)).stdout(Stdio::null());
    isolate_and_exec(&mut command, &probe(), Vec::new()).unwrap();
    let error = command.spawn().expect_err("a terminal on stdin");
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
}

#[test]
fn an_argument_with_a_nul_is_refused_not_cut() {
    // std marks such a command and refuses it before forking; the argv this
    // crate builds would refuse it too. Either way it never runs with a
    // shortened argument.
    let mut command = Command::new("agent-broker");
    command.arg(std::ffi::OsStr::new("a\0b")).env_clear();
    let refused = match isolate_and_exec(&mut command, &probe(), Vec::new()) {
        Err(error) => error,
        Ok(command) => command.spawn().expect_err("a NUL in argv must not run"),
    };
    assert_eq!(refused.kind(), std::io::ErrorKind::InvalidInput);
}
