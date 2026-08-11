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
const ITEM_PASSWORD: &[u8] = b"e2e item password stays encrypted";
const UPDATED_ITEM_PASSWORD: &[u8] = b"e2e updated password stays encrypted";
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
        b"Audit: verified (announcements=1 commits=1 catalogs=1 revisions=0 items=0 audit_events=0)",
    );
    assert!(audit_status.success(), "audit failed: {audit_transcript}");
    assert!(audit_transcript.contains("Vault passphrase: "));
    assert!(audit_transcript.contains(
        "Audit: verified (announcements=1 commits=1 catalogs=1 revisions=0 items=0 audit_events=0)"
    ));
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

    let (add_status, add_transcript) = run_add_login_in_pty(&home);
    assert!(add_status.success(), "item add failed: {add_transcript}");
    assert!(add_transcript.contains("Title: "));
    assert!(add_transcript.contains("Username: "));
    assert!(add_transcript.contains("Password: "));
    assert!(add_transcript.contains("URL (optional): "));
    assert!(!add_transcript.contains("e2e item password"));
    let item_id = extract_item_id(&add_transcript);

    let (list_status, list_transcript) = run_unlock_in_pty(
        &home,
        &["item", "list"],
        b"vault/login/v1\t\"Example account\"",
    );
    assert!(list_status.success(), "item list failed: {list_transcript}");
    assert!(list_transcript.contains(&item_id));
    assert!(!list_transcript.contains("e2e item password"));

    let (show_status, show_transcript) =
        run_unlock_in_pty(&home, &["item", "show", &item_id], b"Password: <redacted>");
    assert!(show_status.success(), "item show failed: {show_transcript}");
    assert!(show_transcript.contains("Title: \"Example account\""));
    assert!(show_transcript.contains("Username: \"ada@example.test\""));
    assert!(show_transcript.contains("URL: \"https://example.test\""));
    assert!(!show_transcript.contains("e2e item password"));

    let (edit_status, edit_transcript) = run_edit_login_in_pty(&home, &item_id);
    assert!(edit_status.success(), "item edit failed: {edit_transcript}");
    assert!(edit_transcript.contains(&format!("Item updated: {item_id}")));
    assert!(!edit_transcript.contains("e2e updated password"));

    let (updated_status, updated_transcript) =
        run_unlock_in_pty(&home, &["item", "show", &item_id], b"Password: <redacted>");
    assert!(
        updated_status.success(),
        "updated item show failed: {updated_transcript}"
    );
    assert!(updated_transcript.contains("Title: \"Updated account\""));
    assert!(updated_transcript.contains("Username: \"grace@example.test\""));
    assert!(updated_transcript.contains("URL: none"));
    assert!(!updated_transcript.contains("e2e updated password"));

    let (history_status, history_transcript) = run_unlock_in_pty(
        &home,
        &["history", "list", &item_id],
        b"vault/login/v1\t\"Example account\"",
    );
    assert!(
        history_status.success(),
        "history list failed: {history_transcript}"
    );
    assert!(history_transcript.contains("\tlive\tparents=1\tupdated="));
    assert!(history_transcript.contains("vault/login/v1\t\"Updated account\""));
    assert!(history_transcript.contains("\tlive\tparents=0\tupdated="));
    assert_transcript_excludes_secrets(&history_transcript);
    assert!(!history_transcript.contains("e2e item password"));
    assert!(!history_transcript.contains("e2e updated password"));
    let original_revision = extract_history_revision(&history_transcript, "Example account");

    let expected_delete = format!("Item deleted: {item_id}");
    let (delete_status, delete_transcript) = run_unlock_in_pty(
        &home,
        &["item", "delete", &item_id],
        expected_delete.as_bytes(),
    );
    assert!(
        delete_status.success(),
        "item delete failed: {delete_transcript}"
    );
    assert!(delete_transcript.contains(&format!("Item deleted: {item_id}")));
    assert_transcript_excludes_secrets(&delete_transcript);

    let (deleted_show_status, deleted_show_transcript) =
        run_unlock_in_pty(&home, &["item", "show", &item_id], b"vault-pm: not found");
    assert_eq!(deleted_show_status.code(), Some(4));
    assert_transcript_excludes_secrets(&deleted_show_transcript);

    let (deleted_history_status, deleted_history_transcript) = run_unlock_in_pty(
        &home,
        &["history", "list", &item_id],
        b"vault/login/v1\t\"Example account\"",
    );
    assert!(deleted_history_status.success());
    assert!(deleted_history_transcript.contains("\tdeleted\tparents=1\tdeleted="));
    assert_transcript_excludes_secrets(&deleted_history_transcript);

    let expected_restore = format!("Item restored: {item_id}");
    let (restore_status, restore_transcript) = run_unlock_in_pty(
        &home,
        &["history", "restore", &item_id, &original_revision],
        expected_restore.as_bytes(),
    );
    assert!(
        restore_status.success(),
        "history restore failed: {restore_transcript}"
    );
    assert!(restore_transcript.contains(&format!("Item restored: {item_id}")));
    assert_transcript_excludes_secrets(&restore_transcript);

    let (restored_status, restored_transcript) =
        run_unlock_in_pty(&home, &["item", "show", &item_id], b"Password: <redacted>");
    assert!(restored_status.success());
    assert!(restored_transcript.contains("Title: \"Example account\""));
    assert_transcript_excludes_secrets(&restored_transcript);
    assert!(!restored_transcript.contains("e2e item password"));
    assert!(!restored_transcript.contains("e2e updated password"));

    let (enable_status, enable_transcript) =
        run_unlock_in_pty(&home, &["audit", "enable"], b"Audit: enabled.");
    assert!(
        enable_status.success(),
        "audit enable failed: {enable_transcript}"
    );
    assert_transcript_excludes_secrets(&enable_transcript);

    let (failed_edit_status, failed_edit_transcript) = run_empty_title_edit_in_pty(&home, &item_id);
    assert_eq!(failed_edit_status.code(), Some(2));
    assert!(failed_edit_transcript.contains("Title: "));
    assert!(failed_edit_transcript.contains("vault-pm: invalid command"));
    assert_transcript_excludes_secrets(&failed_edit_transcript);

    let (post_failure_status, post_failure_transcript) = run_unlock_in_pty(
        &home,
        &["audit", "verify"],
        b"commits=7 catalogs=5 revisions=4 items=1 audit_events=2",
    );
    assert!(
        post_failure_status.success(),
        "post-failure audit failed: {post_failure_transcript}"
    );
    assert_transcript_excludes_secrets(&post_failure_transcript);

    assert_tree_excludes(&home.0, PASSPHRASE);
    assert_tree_excludes(&home.0, ITEM_PASSWORD);
    assert_tree_excludes(&home.0, UPDATED_ITEM_PASSWORD);
}

fn run_add_login_in_pty(home: &TestHome) -> (ExitStatus, String) {
    run_login_form_in_pty(
        home,
        &["item", "add", "login"],
        b"Example account",
        b"ada@example.test",
        ITEM_PASSWORD,
        b"https://example.test",
        b"Item added: ",
    )
}

fn run_edit_login_in_pty(home: &TestHome, item_id: &str) -> (ExitStatus, String) {
    run_login_form_in_pty(
        home,
        &["item", "edit", item_id],
        b"Updated account",
        b"grace@example.test",
        UPDATED_ITEM_PASSWORD,
        b"",
        b"Item updated: ",
    )
}

fn run_empty_title_edit_in_pty(home: &TestHome, item_id: &str) -> (ExitStatus, String) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args(["item", "edit", item_id]);
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
    read_until(&mut master, &mut transcript, b"Title: ");
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"vault-pm: invalid command");
    let error_line = transcript.len() - b"vault-pm: invalid command".len();
    read_until_from(&mut master, &mut transcript, error_line, b"\n");
    drop(master);
    let status = child.wait().unwrap();
    (status, String::from_utf8_lossy(&transcript).into_owned())
}

fn run_login_form_in_pty(
    home: &TestHome,
    arguments: &[&str],
    title: &[u8],
    username: &[u8],
    password: &[u8],
    url: &[u8],
    completion: &[u8],
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
    read_until(&mut master, &mut transcript, b"Title: ");
    master.write_all(title).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"Username: ");
    master.write_all(username).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"Password: ");
    master.write_all(password).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"URL (optional): ");
    master.write_all(url).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, completion);
    let item_line = transcript.len() - completion.len();
    read_until_from(&mut master, &mut transcript, item_line, b"\n");
    drop(master);
    let status = child.wait().unwrap();
    (status, String::from_utf8_lossy(&transcript).into_owned())
}

fn extract_item_id(transcript: &str) -> String {
    let marker = "Item added: ";
    let start = transcript.find(marker).expect("item-add marker") + marker.len();
    transcript[start..]
        .lines()
        .next()
        .expect("item-add ID")
        .trim_end_matches('\r')
        .to_string()
}

fn extract_history_revision(transcript: &str, title: &str) -> String {
    transcript
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .find(|line| line.contains("\tlive\t") && line.ends_with(&format!("\"{title}\"")))
        .and_then(|line| line.split('\t').next())
        .expect("canonical live history revision")
        .to_string()
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

fn read_until_from(master: &mut File, transcript: &mut Vec<u8>, start: usize, pattern: &[u8]) {
    while !transcript[start..]
        .windows(pattern.len())
        .any(|value| value == pattern)
    {
        let mut byte = [0_u8; 1];
        match master.read(&mut byte) {
            Ok(1) => transcript.push(byte[0]),
            Ok(0) => panic!("pseudo-terminal closed before expected line ending"),
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
