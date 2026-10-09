//! P2.6d-3: a verified binary, confined, holding exactly the descriptors it
//! is given at 3..3+n. This is how a broker starts: its keys on those
//! descriptors, and no way to open anything else by path.
//!
//! The probe reports its descriptors with `fcntl` and `fstat`, not
//! `/proc/self/fd`: Landlock leaves `/proc` unopenable.

#![cfg(target_os = "linux")]

use capability_os_sandbox::{plan_from_json, OsFamily, SandboxPlan};
use chief_of_staff_linux_sandbox::{ConfinementError, LinuxConfinement};
use chief_of_staff_spawn_isolation::VerifiedExecutable;
use std::fs;
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

const PROBE: &str = env!("CARGO_BIN_EXE_linux-sandbox-probe");

/// The broker's plan: a manifest with no capabilities at all.
fn deny_all() -> SandboxPlan {
    plan_from_json(
        r#"{"version":1,"package":"rust/linux-sandbox-probe","capabilities":[],"justification":"Probe for inherited descriptors."}"#,
        OsFamily::Linux,
    )
    .unwrap()
}

fn digest(path: &Path) -> [u8; 32] {
    coding_adventures_sha256::sha256(&fs::read(path).unwrap())
}

fn verified(path: &Path) -> VerifiedExecutable {
    VerifiedExecutable::open(path, digest(path)).unwrap()
}

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "inherited-{}-{}",
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

/// A descriptor without close-on-exec, as a careless open would leave.
fn leaked() -> OwnedFd {
    // SAFETY: `dup` returns a new descriptor without FD_CLOEXEC, or -1.
    let fd = unsafe { libc::dup(1) };
    assert!(fd > 2);
    // SAFETY: just returned by `dup`; nothing else owns it.
    unsafe { OwnedFd::from_raw_fd(fd) }
}

fn run(confinement: &LinuxConfinement, inherited: Vec<OwnedFd>, args: &[&str]) -> Output {
    let mut command = Command::new("broker");
    command.args(args).stdin(Stdio::null()).env_clear();
    confinement
        .apply_inheriting(&mut command, inherited)
        .expect("apply");
    command.output().expect("the probe spawns")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn exactly_the_inherited_descriptors_arrive_in_order_and_open_on_exec() {
    let scratch = Scratch::new();
    let files: Vec<fs::File> = (0..3)
        .map(|index| fs::File::open(scratch.file(&format!("key-{index}"), b"k", 0o600)).unwrap())
        .collect();
    let inodes: Vec<u64> = files
        .iter()
        .map(|file| file.metadata().unwrap().ino())
        .collect();
    let _leak = leaked();
    let confinement = LinuxConfinement::prepare_verified(&deny_all(), &verified(Path::new(PROBE)))
        .expect("prepares");

    let output = run(
        &confinement,
        files.into_iter().map(OwnedFd::from).collect(),
        &["descriptors"],
    );
    assert!(output.status.success(), "{output:?}");
    let expected: Vec<String> = inodes
        .iter()
        .enumerate()
        .map(|(index, inode)| format!("{}:{inode}", 3 + index))
        .collect();
    assert_eq!(
        stdout(&output),
        format!("descriptors={}", expected.join(","))
    );
}

#[test]
fn with_nothing_inherited_nothing_arrives_above_stderr() {
    let _leak = leaked();
    let confinement =
        LinuxConfinement::prepare_verified(&deny_all(), &verified(Path::new(PROBE))).unwrap();
    let output = run(&confinement, Vec::new(), &["descriptors"]);
    assert_eq!(stdout(&output), "descriptors=");
}

#[test]
fn the_holder_of_inherited_descriptors_is_confined_like_any_agent() {
    let scratch = Scratch::new();
    let key = scratch.file("key", b"k", 0o600);
    let other = scratch.file("someone-elses-key", b"k", 0o600);
    let confinement =
        LinuxConfinement::prepare_verified(&deny_all(), &verified(Path::new(PROBE))).unwrap();
    let inherit = || vec![OwnedFd::from(fs::File::open(&key).unwrap())];

    // Another key file, by path: Landlock refuses it.
    let output = run(&confinement, inherit(), &["read", other.to_str().unwrap()]);
    assert_eq!(stdout(&output), "errno=13", "{output:?}");
    // No network, and no new process: seccomp kills both.
    for mode in ["socket", "fork"] {
        let output = run(&confinement, inherit(), &[mode]);
        assert_eq!(
            output.status.signal(),
            Some(libc::SIGSYS),
            "{mode}: {output:?}"
        );
    }
}

#[test]
fn what_runs_is_the_file_that_was_verified_not_what_is_at_its_path_later() {
    let scratch = Scratch::new();
    let copy = scratch.file("broker", &fs::read(PROBE).unwrap(), 0o755);
    let confinement = LinuxConfinement::prepare_verified(&deny_all(), &verified(&copy)).unwrap();
    fs::remove_file(&copy).unwrap();
    scratch.file("broker", b"#!/bin/sh\necho replaced\n", 0o755);
    let output = run(&confinement, Vec::new(), &["hello"]);
    assert!(stdout(&output).starts_with("hello"), "{output:?}");
}

#[test]
fn a_binary_changed_since_it_was_verified_is_refused() {
    let scratch = Scratch::new();
    let copy = scratch.file("broker", &fs::read(PROBE).unwrap(), 0o755);
    let verified = verified(&copy);
    let mut tampered = fs::OpenOptions::new().append(true).open(&copy).unwrap();
    std::io::Write::write_all(&mut tampered, b"\0appended").unwrap();
    drop(tampered);
    assert!(matches!(
        LinuxConfinement::prepare_verified(&deny_all(), &verified),
        Err(ConfinementError::Executable(_))
    ));
}

#[test]
fn more_descriptors_than_the_hook_places_refuse_and_poison_the_command() {
    let confinement =
        LinuxConfinement::prepare_verified(&deny_all(), &verified(Path::new(PROBE))).unwrap();
    let too_many: Vec<OwnedFd> = (0..65)
        .map(|_| OwnedFd::from(fs::File::open("/dev/null").unwrap()))
        .collect();
    let mut command = Command::new("broker");
    command.arg("hello").stdin(Stdio::null()).env_clear();
    assert!(matches!(
        confinement.apply_inheriting(&mut command, too_many),
        Err(ConfinementError::Inherited(_))
    ));
    let error = command
        .output()
        .expect_err("a poisoned command must not spawn");
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
}

#[test]
fn a_confined_process_may_clear_its_dumpability_and_never_set_it() {
    let confinement =
        LinuxConfinement::prepare_verified(&deny_all(), &verified(Path::new(PROBE))).unwrap();
    let output = run(&confinement, Vec::new(), &["undumpable"]);
    assert_eq!(stdout(&output), "set=0 dumpable=0", "{output:?}");
    let output = run(&confinement, Vec::new(), &["dumpable"]);
    assert_eq!(output.status.signal(), Some(libc::SIGSYS), "{output:?}");
    // Control: unconfined, the same probe sets it and survives.
    let output = Command::new(PROBE).arg("dumpable").output().unwrap();
    assert_eq!(stdout(&output), "set=0");
}
