#![cfg(unix)]

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::fd::FromRawFd;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);
const PASSPHRASE: &[u8] = b"e2e correct horse battery staple";
const STDIN_INJECTION: &[u8] = b"stdin injected secret\nstdin injected secret\n";

struct TestHome(PathBuf);

impl TestHome {
    fn new() -> Self {
        let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "vault-pm-program-e2e-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        let path = fs::canonicalize(path).unwrap();
        for child in ["home", "config", "data", "cache"] {
            fs::create_dir(path.join(child)).unwrap();
        }
        Self(path)
    }

    fn configure(&self, command: &mut Command) {
        command
            .env("HOME", self.0.join("home"))
            .env("XDG_CONFIG_HOME", self.0.join("config"))
            .env("XDG_DATA_HOME", self.0.join("data"))
            .env("XDG_CACHE_HOME", self.0.join("cache"));
    }
}

impl Drop for TestHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn real_cli_initializes_through_a_hidden_tty_and_survives_restart() {
    let home = TestHome::new();
    let (status, transcript) = run_init_in_pty(&home);
    assert!(status.success(), "init failed: {transcript}");
    assert!(transcript.contains("New vault passphrase: "));
    assert!(transcript.contains("Confirm vault passphrase: "));
    assert!(transcript.contains("Vault initialized."));
    assert!(!transcript
        .as_bytes()
        .windows(PASSPHRASE.len())
        .any(|value| value == PASSPHRASE));
    assert!(!transcript.contains("stdin injected secret"));

    let status = run_plain(&home, &["status", "--json"]);
    assert!(status.status.success());
    assert_eq!(
        String::from_utf8(status.stdout).unwrap(),
        "{\"state\":\"locked\"}\n"
    );
    assert!(status.stderr.is_empty());

    let doctor = run_plain(&home, &["doctor"]);
    assert_eq!(doctor.status.code(), Some(3));
    assert_eq!(
        String::from_utf8(doctor.stdout).unwrap(),
        "Doctor: authentication_required\n"
    );
    assert!(doctor.stderr.is_empty());

    let (audit_status, audit_transcript) = run_unlock_in_pty(
        &home,
        &["audit", "verify"],
        b"Audit: verified (announcements=1 commits=1 catalogs=1 revisions=0 items=0)",
    );
    assert!(audit_status.success(), "audit failed: {audit_transcript}");
    assert!(audit_transcript.contains("Vault passphrase: "));
    assert!(audit_transcript
        .contains("Audit: verified (announcements=1 commits=1 catalogs=1 revisions=0 items=0)"));
    assert_transcript_excludes_secrets(&audit_transcript);

    let (doctor_status, doctor_transcript) =
        run_unlock_in_pty(&home, &["doctor", "--unlock"], b"Doctor: healthy");
    assert!(
        doctor_status.success(),
        "authenticated doctor failed: {doctor_transcript}"
    );
    assert!(doctor_transcript.contains("Vault passphrase: "));
    assert!(doctor_transcript.contains("Doctor: healthy"));
    assert_transcript_excludes_secrets(&doctor_transcript);

    assert_tree_excludes(&home.0, PASSPHRASE);
}

fn run_plain(home: &TestHome, arguments: &[&str]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args(arguments);
    home.configure(&mut command);
    command.output().unwrap()
}

fn run_init_in_pty(home: &TestHome) -> (ExitStatus, String) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.arg("init");
    home.configure(&mut command);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::from(slave.try_clone().unwrap()))
        .stderr(Stdio::from(slave));
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 || libc::ioctl(libc::STDOUT_FILENO, tiocsctty_request(), 0) < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().unwrap();
    drop(command);
    child
        .stdin
        .take()
        .unwrap()
        .write_all(STDIN_INJECTION)
        .unwrap();
    let mut transcript = Vec::new();
    read_until(&mut master, &mut transcript, b"New vault passphrase: ");
    master.write_all(PASSPHRASE).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"Confirm vault passphrase: ");
    master.write_all(PASSPHRASE).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"Vault initialized.");
    drop(master);
    let status = child.wait().unwrap();
    (status, String::from_utf8_lossy(&transcript).into_owned())
}

fn run_unlock_in_pty(
    home: &TestHome,
    arguments: &[&str],
    expected_output: &[u8],
) -> (ExitStatus, String) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args(arguments);
    home.configure(&mut command);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::from(slave.try_clone().unwrap()))
        .stderr(Stdio::from(slave));
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 || libc::ioctl(libc::STDOUT_FILENO, tiocsctty_request(), 0) < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().unwrap();
    drop(command);
    child
        .stdin
        .take()
        .unwrap()
        .write_all(STDIN_INJECTION)
        .unwrap();
    let mut transcript = Vec::new();
    read_until(&mut master, &mut transcript, b"Vault passphrase: ");
    master.write_all(PASSPHRASE).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, expected_output);
    drop(master);
    let status = child.wait().unwrap();
    (status, String::from_utf8_lossy(&transcript).into_owned())
}

fn assert_transcript_excludes_secrets(transcript: &str) {
    assert!(!transcript
        .as_bytes()
        .windows(PASSPHRASE.len())
        .any(|value| value == PASSPHRASE));
    assert!(!transcript.contains("stdin injected secret"));
}

#[cfg(target_vendor = "apple")]
fn tiocsctty_request() -> libc::c_ulong {
    libc::TIOCSCTTY.into()
}

#[cfg(not(target_vendor = "apple"))]
fn tiocsctty_request() -> libc::c_ulong {
    libc::TIOCSCTTY
}

fn open_pty() -> (File, File) {
    let mut master = -1;
    let mut slave = -1;
    let result = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(result, 0, "openpty failed: {}", io::Error::last_os_error());
    unsafe { (File::from_raw_fd(master), File::from_raw_fd(slave)) }
}

fn read_until(master: &mut File, transcript: &mut Vec<u8>, pattern: &[u8]) {
    while !transcript
        .windows(pattern.len())
        .any(|value| value == pattern)
    {
        let mut byte = [0_u8; 1];
        match master.read(&mut byte) {
            Ok(1) => transcript.push(byte[0]),
            Ok(0) => panic!("pseudo-terminal closed before expected prompt"),
            Ok(_) => unreachable!(),
            Err(error) => panic!("pseudo-terminal read failed: {error}"),
        }
    }
}

fn assert_tree_excludes(root: &Path, forbidden: &[u8]) {
    for entry in fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if entry.file_type().unwrap().is_dir() {
            assert_tree_excludes(&path, forbidden);
        } else {
            let bytes = fs::read(path).unwrap();
            assert!(!bytes
                .windows(forbidden.len())
                .any(|value| value == forbidden));
        }
    }
}
