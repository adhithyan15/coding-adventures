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
const TARGET_PASSPHRASE: &[u8] = b"e2e separate restore target passphrase";
const ITEM_PASSWORD: &[u8] = b"e2e item password stays encrypted";
const UPDATED_ITEM_PASSWORD: &[u8] = b"e2e updated password stays encrypted";
const SECURE_NOTE_BODY: &[u8] = b"e2e secure note body stays encrypted";
const EXPORT_PASSPHRASE: &[u8] = b"e2e distinct portable export passphrase";
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
        b"Audit: verified (announcements=1 commits=1 catalogs=1 revisions=0 items=0 audit_events=1)",
    );
    assert!(audit_status.success(), "audit failed: {audit_transcript}");
    assert!(audit_transcript.contains("Vault passphrase: "));
    assert!(audit_transcript.contains(
        "Audit: verified (announcements=1 commits=1 catalogs=1 revisions=0 items=0 audit_events=1)"
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

    let (note_status, note_transcript) = run_add_secure_note_in_pty(&home);
    assert!(
        note_status.success(),
        "secure note add failed: {note_transcript}"
    );
    assert!(note_transcript.contains("Title: "));
    assert!(note_transcript.contains("Note: "));
    assert!(!note_transcript.contains("e2e secure note body"));
    let note_id = extract_item_id(&note_transcript);

    let (note_show_status, note_show_transcript) =
        run_unlock_in_pty(&home, &["item", "show", &note_id], b"Body: <redacted>");
    assert!(
        note_show_status.success(),
        "secure note show failed: {note_show_transcript}"
    );
    assert!(note_show_transcript.contains("Type: vault/note/v1"));
    assert!(note_show_transcript.contains("Title: \"Recovery note\""));
    assert!(!note_show_transcript.contains("e2e secure note body"));

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
        run_unlock_in_pty(&home, &["audit", "enable"], b"Audit: already enabled.");
    assert!(
        enable_status.success(),
        "audit enable failed: {enable_transcript}"
    );
    assert_transcript_excludes_secrets(&enable_transcript);

    let (failed_add_status, failed_add_transcript) = run_empty_title_add_in_pty(&home);
    assert_eq!(failed_add_status.code(), Some(2));
    assert!(failed_add_transcript.contains("Title: "));
    assert!(failed_add_transcript.contains("vault-pm: invalid command"));
    assert_transcript_excludes_secrets(&failed_add_transcript);

    let (failed_edit_status, failed_edit_transcript) = run_empty_title_edit_in_pty(&home, &item_id);
    assert_eq!(failed_edit_status.code(), Some(2));
    assert!(failed_edit_transcript.contains("Title: "));
    assert!(failed_edit_transcript.contains("vault-pm: invalid command"));
    assert_transcript_excludes_secrets(&failed_edit_transcript);

    let (post_failure_status, post_failure_transcript) = run_unlock_in_pty(
        &home,
        &["audit", "verify"],
        b"commits=18 catalogs=6 revisions=5 items=2 audit_events=18",
    );
    assert!(
        post_failure_status.success(),
        "post-failure audit failed: {post_failure_transcript}"
    );
    assert_transcript_excludes_secrets(&post_failure_transcript);

    let (audit_list_status, audit_list_transcript) = run_unlock_in_pty(
        &home,
        &["audit", "list"],
        b"action=item_create\toutcome=failed",
    );
    assert!(
        audit_list_status.success(),
        "audit list failed: {audit_list_transcript}"
    );
    assert!(
        audit_list_transcript.contains("action=item_create\toutcome=failed"),
        "{audit_list_transcript}"
    );
    assert!(audit_list_transcript.contains("action=audit_read\toutcome=succeeded"));
    assert!(audit_list_transcript.contains("action=vault_verify\toutcome=succeeded"));
    assert_transcript_excludes_secrets(&audit_list_transcript);
    let failed_add_trace = extract_audit_trace(&audit_list_transcript, "item_create");
    let failed_edit_trace = extract_audit_trace(&audit_list_transcript, "item_update");

    let (add_show_status, add_show_transcript) = run_unlock_in_pty(
        &home,
        &["audit", "show", &failed_add_trace],
        b"action=item_create\toutcome=failed",
    );
    assert!(
        add_show_status.success(),
        "audit create show failed: {add_show_transcript}"
    );
    assert!(add_show_transcript.contains(&failed_add_trace));
    assert_transcript_excludes_secrets(&add_show_transcript);

    let (audit_show_status, audit_show_transcript) = run_unlock_in_pty(
        &home,
        &["audit", "show", &failed_edit_trace],
        b"action=item_update\toutcome=failed",
    );
    assert!(
        audit_show_status.success(),
        "audit show failed: {audit_show_transcript}"
    );
    assert!(audit_show_transcript.contains(&failed_edit_trace));
    assert_transcript_excludes_secrets(&audit_show_transcript);

    let (final_audit_status, final_audit_transcript) =
        run_unlock_in_pty(&home, &["audit", "verify"], b"audit_events=22");
    assert!(
        final_audit_status.success(),
        "final audit verification failed: {final_audit_transcript}"
    );
    assert_transcript_excludes_secrets(&final_audit_transcript);

    let export_path = home.0.join("portable-backup.vpm");
    let (export_status, export_transcript) = run_export_in_pty(&home, &export_path);
    assert!(
        export_status.success(),
        "portable export failed: {export_transcript}"
    );
    assert!(export_transcript.contains("Export passphrase: "));
    assert!(export_transcript.contains("Confirm export passphrase: "));
    assert!(export_transcript.contains("Portable export written."));
    assert!(!export_transcript
        .as_bytes()
        .windows(EXPORT_PASSPHRASE.len())
        .any(|value| value == EXPORT_PASSPHRASE));
    let artifact = fs::read(&export_path).unwrap();
    assert!(!artifact.is_empty());
    assert!(!artifact
        .windows(EXPORT_PASSPHRASE.len())
        .any(|value| value == EXPORT_PASSPHRASE));

    let (restore_init_status, restore_init_transcript) = run_target_create_in_pty(&home);
    assert!(
        restore_init_status.success(),
        "restore target init failed: {restore_init_transcript}"
    );
    assert!(restore_init_transcript.contains("Vault target created."));
    let (restore_audit_status, restore_audit_transcript) = run_unlock_with_passphrase_in_pty(
        &home,
        &["--vault", "restore", "audit", "enable"],
        b"Audit: already enabled.",
        TARGET_PASSPHRASE,
    );
    assert!(
        restore_audit_status.success(),
        "restore target audit enable failed: {restore_audit_transcript}"
    );
    let (import_status, import_transcript) = run_import_in_pty(&home, &export_path);
    assert!(
        import_status.success(),
        "portable import failed: {import_transcript}"
    );
    assert!(import_transcript.contains("Import passphrase: "));
    assert!(import_transcript.contains("Portable import complete: items=2 candidates=2."));
    assert!(!import_transcript
        .as_bytes()
        .windows(EXPORT_PASSPHRASE.len())
        .any(|value| value == EXPORT_PASSPHRASE));
    let (restore_verify_status, restore_verify_transcript) =
        run_restore_verify_in_pty(&home, &export_path);
    assert!(
        restore_verify_status.success(),
        "portable restore verification failed: {restore_verify_transcript}"
    );
    assert!(restore_verify_transcript.contains("Import passphrase: "));
    assert!(restore_verify_transcript
        .contains("Portable restore verified: items=2 candidates=2 conflicts=0."));
    assert!(!restore_verify_transcript
        .as_bytes()
        .windows(EXPORT_PASSPHRASE.len())
        .any(|value| value == EXPORT_PASSPHRASE));
    let (restore_list_status, restore_list_transcript) = run_unlock_with_passphrase_in_pty(
        &home,
        &["--vault", "restore", "item", "list"],
        b"vault/note/v1\t\"Recovery note\"",
        TARGET_PASSPHRASE,
    );
    assert!(restore_list_status.success(), "{restore_list_transcript}");
    assert_transcript_excludes_secrets(&restore_list_transcript);

    let (source_list_status, source_list_transcript) = run_unlock_in_pty(
        &home,
        &["item", "list"],
        b"vault/note/v1\t\"Recovery note\"",
    );
    assert!(source_list_status.success(), "{source_list_transcript}");
    assert!(source_list_transcript.contains("vault/login/v1\t\"Example account\""));
    assert_transcript_excludes_secrets(&source_list_transcript);

    assert_tree_excludes(&home.0, PASSPHRASE);
    assert_tree_excludes(&home.0, TARGET_PASSPHRASE);
    assert_tree_excludes(&home.0, ITEM_PASSWORD);
    assert_tree_excludes(&home.0, UPDATED_ITEM_PASSWORD);
    assert_tree_excludes(&home.0, SECURE_NOTE_BODY);
    assert_tree_excludes(&home.0, EXPORT_PASSPHRASE);
}

fn run_export_in_pty(home: &TestHome, destination: &Path) -> (ExitStatus, String) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args([
        "export",
        destination.to_str().expect("UTF-8 test export destination"),
    ]);
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
    read_until(&mut master, &mut transcript, b"Export passphrase: ");
    master.write_all(EXPORT_PASSPHRASE).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"Confirm export passphrase: ");
    master.write_all(EXPORT_PASSPHRASE).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"Portable export written.");
    drop(master);
    let status = child.wait().unwrap();
    (status, String::from_utf8_lossy(&transcript).into_owned())
}

fn run_import_in_pty(home: &TestHome, source: &Path) -> (ExitStatus, String) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args([
        "--vault",
        "restore",
        "import",
        source.to_str().expect("UTF-8 test import source"),
    ]);
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
    child
        .stdin
        .take()
        .unwrap()
        .write_all(STDIN_INJECTION)
        .unwrap();
    let mut transcript = Vec::new();
    read_until(&mut master, &mut transcript, b"Vault passphrase: ");
    master.write_all(TARGET_PASSPHRASE).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"Import passphrase: ");
    master.write_all(EXPORT_PASSPHRASE).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(
        &mut master,
        &mut transcript,
        b"Portable import complete: items=2 candidates=2.",
    );
    drop(master);
    let status = child.wait().unwrap();
    (status, String::from_utf8_lossy(&transcript).into_owned())
}

fn run_restore_verify_in_pty(home: &TestHome, source: &Path) -> (ExitStatus, String) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args([
        "--vault",
        "restore",
        "restore",
        "verify",
        source.to_str().expect("UTF-8 test restore source"),
    ]);
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
    child
        .stdin
        .take()
        .unwrap()
        .write_all(STDIN_INJECTION)
        .unwrap();
    let mut transcript = Vec::new();
    read_until(&mut master, &mut transcript, b"Vault passphrase: ");
    master.write_all(TARGET_PASSPHRASE).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"Import passphrase: ");
    master.write_all(EXPORT_PASSPHRASE).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(
        &mut master,
        &mut transcript,
        b"Portable restore verified: items=2 candidates=2 conflicts=0.",
    );
    drop(master);
    let status = child.wait().unwrap();
    (status, String::from_utf8_lossy(&transcript).into_owned())
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

fn run_add_secure_note_in_pty(home: &TestHome) -> (ExitStatus, String) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args(["item", "add", "secure-note"]);
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
    master.write_all(b"Recovery note\n").unwrap();
    read_until(&mut master, &mut transcript, b"Note: ");
    master.write_all(SECURE_NOTE_BODY).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"Item added: ");
    let item_line = transcript.len() - b"Item added: ".len();
    read_until_from(&mut master, &mut transcript, item_line, b"\n");
    drop(master);
    let status = child.wait().unwrap();
    (status, String::from_utf8_lossy(&transcript).into_owned())
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
    run_empty_title_form_in_pty(home, &["item", "edit", item_id])
}

fn run_empty_title_add_in_pty(home: &TestHome) -> (ExitStatus, String) {
    run_empty_title_form_in_pty(home, &["item", "add", "login"])
}

fn run_empty_title_form_in_pty(home: &TestHome, arguments: &[&str]) -> (ExitStatus, String) {
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

fn extract_audit_trace(transcript: &str, action: &str) -> String {
    transcript
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .find(|line| line.contains(&format!("\taction={action}\t")))
        .and_then(|line| line.split('\t').next())
        .expect("canonical audit trace")
        .to_string()
}

fn run_plain(home: &TestHome, arguments: &[&str]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args(arguments);
    home.configure(&mut command);
    command.output().unwrap()
}

fn run_init_in_pty(home: &TestHome) -> (ExitStatus, String) {
    run_new_vault_in_pty(home, &["init"], PASSPHRASE, b"Vault initialized.")
}

fn run_target_create_in_pty(home: &TestHome) -> (ExitStatus, String) {
    run_new_vault_in_pty(
        home,
        &["vault", "create", "restore"],
        TARGET_PASSPHRASE,
        b"Vault target created.",
    )
}

fn run_new_vault_in_pty(
    home: &TestHome,
    arguments: &[&str],
    passphrase: &[u8],
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
    read_until(&mut master, &mut transcript, b"New vault passphrase: ");
    master.write_all(passphrase).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"Confirm vault passphrase: ");
    master.write_all(passphrase).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, expected_output);
    drop(master);
    let status = child.wait().unwrap();
    (status, String::from_utf8_lossy(&transcript).into_owned())
}

fn run_unlock_in_pty(
    home: &TestHome,
    arguments: &[&str],
    expected_output: &[u8],
) -> (ExitStatus, String) {
    run_unlock_with_passphrase_in_pty(home, arguments, expected_output, PASSPHRASE)
}

fn run_unlock_with_passphrase_in_pty(
    home: &TestHome,
    arguments: &[&str],
    expected_output: &[u8],
    passphrase: &[u8],
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
    master.write_all(passphrase).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, expected_output);
    drain_pty(&mut master, &mut transcript);
    drop(master);
    let status = child.wait().unwrap();
    (status, String::from_utf8_lossy(&transcript).into_owned())
}

fn assert_transcript_excludes_secrets(transcript: &str) {
    assert!(!transcript
        .as_bytes()
        .windows(PASSPHRASE.len())
        .any(|value| value == PASSPHRASE));
    assert!(!transcript
        .as_bytes()
        .windows(TARGET_PASSPHRASE.len())
        .any(|value| value == TARGET_PASSPHRASE));
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
            Ok(0) => panic!(
                "pseudo-terminal closed before expected public text: {}",
                String::from_utf8_lossy(pattern)
            ),
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
            Ok(0) => panic!(
                "pseudo-terminal closed before line ending after public text: {}",
                String::from_utf8_lossy(pattern)
            ),
            Ok(_) => unreachable!(),
            Err(error) => panic!("pseudo-terminal read failed: {error}"),
        }
    }
}

fn drain_pty(master: &mut File, transcript: &mut Vec<u8>) {
    let mut bytes = [0_u8; 4096];
    loop {
        match master.read(&mut bytes) {
            Ok(0) => return,
            Ok(count) => transcript.extend_from_slice(&bytes[..count]),
            Err(error) if error.raw_os_error() == Some(libc::EIO) => return,
            Err(error) => panic!("pseudo-terminal drain failed: {error}"),
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
