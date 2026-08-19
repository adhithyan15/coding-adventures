#![cfg(unix)]

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);
const PASSPHRASE: &[u8] = b"e2e correct horse battery staple";
const TARGET_PASSPHRASE: &[u8] = b"e2e separate restore target passphrase";
const ROTATED_PASSPHRASE: &[u8] = b"e2e rotated correct horse battery staple";
const ITEM_PASSWORD: &[u8] = b"e2e item password stays encrypted";
const UPDATED_ITEM_PASSWORD: &[u8] = b"e2e updated password stays encrypted";
const LOGIN_NOTES: &[u8] = b"e2e login notes stay encrypted 91d603be";
const UPDATED_LOGIN_NOTES: &[u8] = b"e2e updated login notes stay encrypted 7a425cd1";
const SECURE_NOTE_BODY: &[u8] = b"e2e secure note body stays encrypted";
const CARD_NUMBER: &[u8] = b"4242424242424242";
const CARD_CVV: &[u8] = b"7391";
const API_KEY_TOKEN: &[u8] = b"vlt_e2e_d83f71a5c82b46a3910ec7fd2146b90a";
const DATABASE_PASSWORD: &[u8] = b"db_e2e_61f2c0bdb52049d7a77e71a3e68943ae";
const TOTP_BASE32: &[u8] = b"GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
const TOTP_RAW_SECRET: &[u8] = b"12345678901234567890";
const EXPORT_PASSPHRASE: &[u8] = b"e2e distinct portable export passphrase";
const SEARCH_QUERY: &[u8] = b"accounts.example.test";
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
            .env("XDG_CACHE_HOME", self.0.join("cache"))
            // VLT-PM46 §4.1 detects a clipboard from these two variables on
            // every non-macOS host. Removing them makes every real-process
            // test here deterministically clipboard-free, so a `--copy` run
            // exercises the fail-closed path on a developer's desktop exactly
            // as it does on a headless CI runner — and, just as importantly,
            // never reaches out and overwrites the developer's own clipboard.
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY");
    }
}

impl Drop for TestHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The shipped executable must never contain VLT-PM41 crash injection.
///
/// The drill needs a `vault-pm` it can kill at a chosen durable write, and the
/// obvious way to get one — enabling the `crash-injection` feature through
/// this crate's `dev-dependencies` — quietly fails. Cargo resolves features
/// per package across a build graph, so `cargo build --release --all-targets`
/// would pull dev-dependencies in and uplift the instrumented binary to
/// `target/release/vault-pm`, the exact path a packaging step copies from. A
/// password manager would then ship an environment-variable kill switch that
/// fires between durable writes.
///
/// The drill therefore lives in a separate crate,
/// `code/programs/rust/vault-pm-cli-drill`, whose binary is `vault-pm-drill`.
/// This test is the guard rail on that decision: it reads the binary this
/// crate actually produced and fails if either injection variable name appears
/// anywhere in it. It runs in a build that does have dev-dependencies
/// resolved, which is precisely the configuration the mistake shows up in.
#[test]
fn the_shipped_executable_contains_no_crash_injection() {
    let binary = fs::read(env!("CARGO_BIN_EXE_vault-pm")).unwrap();
    for forbidden in [b"VAULT_PM_CRASH_AT".as_slice(), b"VAULT_PM_CRASH_TRACE"] {
        assert!(
            !binary
                .windows(forbidden.len())
                .any(|value| value == forbidden),
            "the shipped vault-pm binary contains {}; see VLT-PM41 section 4.6",
            String::from_utf8_lossy(forbidden)
        );
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
    assert!(add_transcript.contains("URL count (0-16): "));
    assert_eq!(add_transcript.matches("URL: ").count(), 2);
    assert!(add_transcript.contains("Notes (optional): "));
    assert!(!add_transcript.contains("e2e item password"));
    assert!(!add_transcript.contains("e2e login notes"));
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

    let search_query = std::str::from_utf8(SEARCH_QUERY).unwrap();
    let (search_status, search_transcript) = run_unlock_in_pty(
        &home,
        &["search", search_query],
        format!("{item_id}\tvault/login/v1\t\"Example account\"").as_bytes(),
    );
    assert!(
        search_status.success(),
        "item search failed: {search_transcript}"
    );
    assert!(search_transcript.contains(&item_id));
    assert!(search_transcript.contains("vault/login/v1\t\"Example account\""));
    assert!(!search_transcript.contains(search_query));
    assert_transcript_excludes_secrets(&search_transcript);

    let (show_status, show_transcript) =
        run_unlock_in_pty(&home, &["item", "show", &item_id], b"Password: <redacted>");
    assert!(show_status.success(), "item show failed: {show_transcript}");
    assert!(show_transcript.contains("Title: \"Example account\""));
    assert!(show_transcript.contains("Username: \"ada@example.test\""));
    assert!(show_transcript.contains("URL: \"https://example.test/login\""));
    assert!(show_transcript.contains("URL: \"https://accounts.example.test\""));
    assert!(show_transcript.contains("Notes: present"));
    assert!(!show_transcript.contains("e2e item password"));
    assert!(!show_transcript.contains("e2e login notes"));

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
    assert!(updated_transcript.contains("URL: \"https://updated.example.test\""));
    assert!(updated_transcript.contains("URL: \"https://backup.example.test\""));
    assert!(updated_transcript.contains("Notes: present"));
    assert!(!updated_transcript.contains("e2e updated password"));
    assert!(!updated_transcript.contains("e2e updated login notes"));

    let (reveal_status, reveal_transcript, reveal_stdout) =
        run_secret_reveal_in_pty(&home, &item_id, "login-password", UPDATED_ITEM_PASSWORD);
    assert!(
        reveal_status.success(),
        "secret reveal failed: {reveal_transcript}"
    );
    assert!(reveal_transcript.contains("Reveal secret on this terminal? Type yes to continue: "));
    assert!(reveal_transcript.contains("Secret: \"e2e updated password stays encrypted\""));
    assert!(reveal_stdout.is_empty(), "secret entered process stdout");
    assert_transcript_excludes_secrets(&reveal_transcript);

    let (notes_status, notes_transcript, notes_stdout) =
        run_secret_reveal_in_pty(&home, &item_id, "login-notes", UPDATED_LOGIN_NOTES);
    assert!(
        notes_status.success(),
        "login notes reveal failed: {notes_transcript}"
    );
    assert!(
        notes_transcript.contains("Secret: \"e2e updated login notes stay encrypted 7a425cd1\"")
    );
    assert!(
        notes_stdout.is_empty(),
        "login notes entered process stdout"
    );
    assert_transcript_excludes_secrets(&notes_transcript);

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

    let (denied_candidate_status, denied_candidate_transcript, denied_candidate_stdout) =
        run_conflict_reveal_failure_in_pty(
            &home,
            &item_id,
            &original_revision,
            b"no",
            b"vault-pm: invalid command",
        );
    assert_eq!(denied_candidate_status.code(), Some(2));
    assert!(denied_candidate_stdout.is_empty());
    assert_transcript_excludes_secrets(&denied_candidate_transcript);

    let (failed_candidate_status, failed_candidate_transcript, failed_candidate_stdout) =
        run_conflict_reveal_failure_in_pty(
            &home,
            &item_id,
            &original_revision,
            b"yes",
            b"vault-pm: recovery or conflict required",
        );
    assert_eq!(failed_candidate_status.code(), Some(5));
    assert!(failed_candidate_stdout.is_empty());
    assert_transcript_excludes_secrets(&failed_candidate_transcript);

    let (failed_merge_status, failed_merge_transcript) = run_unlock_in_pty(
        &home,
        &["conflict", "merge", "login", &item_id, &original_revision],
        b"vault-pm: recovery or conflict required",
    );
    assert_eq!(failed_merge_status.code(), Some(5));
    assert!(!failed_merge_transcript.contains("Title: "));
    assert_transcript_excludes_secrets(&failed_merge_transcript);

    let (failed_note_merge_status, failed_note_merge_transcript) = run_unlock_in_pty(
        &home,
        &[
            "conflict",
            "merge",
            "secure-note",
            &item_id,
            &original_revision,
        ],
        b"vault-pm: recovery or conflict required",
    );
    assert_eq!(failed_note_merge_status.code(), Some(5));
    assert!(!failed_note_merge_transcript.contains("Title: "));
    assert_transcript_excludes_secrets(&failed_note_merge_transcript);

    let (failed_card_merge_status, failed_card_merge_transcript) = run_unlock_in_pty(
        &home,
        &["conflict", "merge", "card", &item_id, &original_revision],
        b"vault-pm: recovery or conflict required",
    );
    assert_eq!(failed_card_merge_status.code(), Some(5));
    assert!(!failed_card_merge_transcript.contains("Title: "));
    assert!(!failed_card_merge_transcript.contains("Card number: "));
    assert_transcript_excludes_secrets(&failed_card_merge_transcript);

    let (failed_api_key_merge_status, failed_api_key_merge_transcript) = run_unlock_in_pty(
        &home,
        &["conflict", "merge", "api-key", &item_id, &original_revision],
        b"vault-pm: recovery or conflict required",
    );
    assert_eq!(failed_api_key_merge_status.code(), Some(5));
    assert!(!failed_api_key_merge_transcript.contains("Label: "));
    assert!(!failed_api_key_merge_transcript.contains("Token: "));
    assert_transcript_excludes_secrets(&failed_api_key_merge_transcript);

    let (failed_database_merge_status, failed_database_merge_transcript) = run_unlock_in_pty(
        &home,
        &[
            "conflict",
            "merge",
            "database-credential",
            &item_id,
            &original_revision,
        ],
        b"vault-pm: recovery or conflict required",
    );
    assert_eq!(failed_database_merge_status.code(), Some(5));
    assert!(!failed_database_merge_transcript.contains("Engine: "));
    assert!(!failed_database_merge_transcript.contains("Password: "));
    assert_transcript_excludes_secrets(&failed_database_merge_transcript);

    let (failed_totp_merge_status, failed_totp_merge_transcript) = run_unlock_in_pty(
        &home,
        &["conflict", "merge", "totp", &item_id, &original_revision],
        b"vault-pm: recovery or conflict required",
    );
    assert_eq!(failed_totp_merge_status.code(), Some(5));
    assert!(!failed_totp_merge_transcript.contains("Algorithm (SHA1/SHA256/SHA512): "));
    assert!(!failed_totp_merge_transcript.contains("Secret (Base32): "));
    assert_transcript_excludes_secrets(&failed_totp_merge_transcript);

    let (failed_opaque_merge_status, failed_opaque_merge_transcript) = run_unlock_in_pty(
        &home,
        &["conflict", "merge", "opaque", &item_id, &original_revision],
        b"vault-pm: recovery or conflict required",
    );
    assert_eq!(failed_opaque_merge_status.code(), Some(5));
    assert!(!failed_opaque_merge_transcript.contains("Payload (hex): "));
    assert_transcript_excludes_secrets(&failed_opaque_merge_transcript);

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
    assert!(restored_transcript.contains("URL: \"https://example.test/login\""));
    assert!(restored_transcript.contains("URL: \"https://accounts.example.test\""));
    assert!(restored_transcript.contains("Notes: present"));
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

    let (post_failure_status, post_failure_transcript) =
        run_unlock_in_pty(&home, &["audit", "verify"], b"Audit: verified (");
    assert!(
        post_failure_status.success(),
        "post-failure audit failed: {post_failure_transcript}"
    );
    assert!(
        post_failure_transcript
            .contains("commits=30 catalogs=6 revisions=5 items=2 audit_events=30"),
        "unexpected post-failure audit totals: {post_failure_transcript}"
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
    assert!(audit_list_transcript.contains("action=item_search\toutcome=succeeded"));
    assert!(audit_list_transcript.contains("action=item_read\toutcome=denied"));
    assert!(audit_list_transcript.contains("action=item_read\toutcome=failed"));
    assert!(audit_list_transcript.contains("action=item_conflict_merge\toutcome=failed"));
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
        run_unlock_in_pty(&home, &["audit", "verify"], b"Audit: verified (");
    assert!(
        final_audit_status.success(),
        "final audit verification failed: {final_audit_transcript}"
    );
    assert!(
        final_audit_transcript.contains("audit_events=34"),
        "unexpected final audit total: {final_audit_transcript}"
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
    let (restore_status, restore_transcript) = run_restore_in_pty(&home, &export_path);
    assert!(
        restore_status.success(),
        "portable restore failed: {restore_transcript}"
    );
    assert!(restore_transcript.contains("Import passphrase: "));
    assert!(restore_transcript
        .contains("Portable restore completed and verified: items=2 candidates=2 conflicts=0."));
    assert!(!restore_transcript.contains("Portable import complete:"));
    assert!(!restore_transcript
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
    assert_tree_excludes(&home.0, LOGIN_NOTES);
    assert_tree_excludes(&home.0, UPDATED_LOGIN_NOTES);
    assert_tree_excludes(&home.0, SECURE_NOTE_BODY);
    assert_tree_excludes(&home.0, EXPORT_PASSPHRASE);
    assert_tree_excludes(&home.0, SEARCH_QUERY);
}

/// One real process, one passphrase, many commands.
///
/// This is the property the interactive shell exists for, proved against the
/// real executable rather than the library: a session unlocks once, runs
/// several commands without prompting again, forgets its authenticator on an
/// explicit `lock`, prompts again afterwards, and ends cleanly on `exit`.
///
/// It also proves the properties the shell must *not* have weakened. The
/// child's standard input is the same injected pipe every other end-to-end
/// test uses, so a piped stdin still cannot drive an unlocked session, and the
/// hidden item-password ceremony still happens on the terminal with echo off.
#[test]
fn real_cli_shell_unlocks_once_locks_on_demand_and_exits_cleanly() {
    let home = TestHome::new();
    let (init_status, init_transcript) = run_init_in_pty(&home);
    assert!(init_status.success(), "init failed: {init_transcript}");

    let (mut master, mut child) = spawn_shell_in_pty(&home);
    let mut transcript = Vec::new();
    read_until(&mut master, &mut transcript, b"vault-pm> ");

    // First authenticated command: the session has no authenticator yet.
    master.write_all(b"item list\n").unwrap();
    read_until(&mut master, &mut transcript, b"Vault passphrase: ");
    master.write_all(PASSPHRASE).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"No items.");

    // Second and third authenticated commands: no prompt may appear between
    // them, which is the whole point of the session.
    let quiet = transcript.len();
    master.write_all(b"item list\n").unwrap();
    read_until_from(&mut master, &mut transcript, quiet, b"No items.");
    master.write_all(b"search absent\n").unwrap();
    read_until_from(&mut master, &mut transcript, quiet, b"No matches.");
    assert!(
        !String::from_utf8_lossy(&transcript[quiet..]).contains("Vault passphrase: "),
        "a retained session re-prompted: {}",
        String::from_utf8_lossy(&transcript[quiet..])
    );

    // A secret-bearing command still runs its hidden ceremony inside the shell.
    let hidden = transcript.len();
    master.write_all(b"item add login\n").unwrap();
    read_until_from(&mut master, &mut transcript, hidden, b"Title: ");
    master.write_all(b"Shell account\n").unwrap();
    read_until_from(&mut master, &mut transcript, hidden, b"Username: ");
    master.write_all(b"shell.user\n").unwrap();
    read_until_from(&mut master, &mut transcript, hidden, b"Password: ");
    master.write_all(ITEM_PASSWORD).unwrap();
    master.write_all(b"\n").unwrap();
    read_until_from(&mut master, &mut transcript, hidden, b"URL count (0-16): ");
    master.write_all(b"0\n").unwrap();
    read_until_from(&mut master, &mut transcript, hidden, b"Notes (optional): ");
    master.write_all(b"\n").unwrap();
    read_until_from(&mut master, &mut transcript, hidden, b"Item added: ");
    assert!(
        !String::from_utf8_lossy(&transcript[hidden..]).contains("Vault passphrase: "),
        "the item ceremony re-prompted for the vault passphrase"
    );

    // An explicit lock forgets the authenticator; the next command must ask.
    let relock = transcript.len();
    master.write_all(b"lock\n").unwrap();
    read_until_from(&mut master, &mut transcript, relock, b"Locked.");
    master.write_all(b"item list\n").unwrap();
    read_until_from(&mut master, &mut transcript, relock, b"Vault passphrase: ");
    master.write_all(PASSPHRASE).unwrap();
    master.write_all(b"\n").unwrap();
    read_until_from(&mut master, &mut transcript, relock, b"Shell account");

    master.write_all(b"exit\n").unwrap();
    drain_pty(&mut master, &mut transcript);
    drop(master);
    let status = child.wait().unwrap();
    let transcript = String::from_utf8_lossy(&transcript).into_owned();
    assert!(status.success(), "shell exit failed: {transcript}");
    assert_transcript_excludes_secrets(&transcript);
    assert!(!transcript.contains("e2e item password"));
    assert_tree_excludes(&home.0, PASSPHRASE);
    assert_tree_excludes(&home.0, ITEM_PASSWORD);
}

/// End of input ends the session, and refused verbs never end it.
#[test]
fn real_cli_shell_rejects_lifecycle_verbs_and_ends_on_end_of_input() {
    let home = TestHome::new();
    let (init_status, init_transcript) = run_init_in_pty(&home);
    assert!(init_status.success(), "init failed: {init_transcript}");

    let (mut master, mut child) = spawn_shell_in_pty(&home);
    let mut transcript = Vec::new();
    read_until(&mut master, &mut transcript, b"vault-pm> ");

    // Vault lifecycle and vault reselection are refused, and a refusal keeps
    // the session alive rather than terminating it.
    for refused in ["init\n", "vault create work\n", "--vault work item list\n"] {
        let start = transcript.len();
        master.write_all(refused.as_bytes()).unwrap();
        read_until_from(
            &mut master,
            &mut transcript,
            start,
            b"vault-pm: invalid command",
        );
    }

    // Ctrl-D on an empty line is the terminal's end of input.
    master.write_all(&[0x04]).unwrap();
    drain_pty(&mut master, &mut transcript);
    drop(master);
    let status = child.wait().unwrap();
    let transcript = String::from_utf8_lossy(&transcript).into_owned();
    assert!(
        status.success(),
        "end of input did not end the session cleanly: {transcript}"
    );
    assert_transcript_excludes_secrets(&transcript);
}

/// Start `vault-pm shell` on a pseudo-terminal with an injected piped stdin.
fn spawn_shell_in_pty(home: &TestHome) -> (File, std::process::Child) {
    let (master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.arg("shell");
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
    (master, child)
}

#[test]
fn real_cli_creates_redacts_and_separately_reveals_a_payment_card() {
    let home = TestHome::new();
    let (init_status, init_transcript) = run_init_in_pty(&home);
    assert!(init_status.success(), "init failed: {init_transcript}");

    let (add_status, add_transcript) = run_add_card_in_pty(&home);
    assert!(add_status.success(), "card add failed: {add_transcript}");
    for prompt in [
        "Title: ",
        "Cardholder: ",
        "Card number: ",
        "Expiry month (1-12): ",
        "Expiry year (YYYY): ",
        "CVV: ",
        "Billing postal code (optional): ",
    ] {
        assert!(add_transcript.contains(prompt), "{add_transcript}");
    }
    assert!(!add_transcript.contains(core::str::from_utf8(CARD_NUMBER).unwrap()));
    assert!(!add_transcript.contains(core::str::from_utf8(CARD_CVV).unwrap()));
    let item_id = extract_item_id(&add_transcript);

    let (show_status, show_transcript) = run_unlock_in_pty(
        &home,
        &["item", "show", &item_id],
        b"Billing postal code: present",
    );
    assert!(show_status.success(), "card show failed: {show_transcript}");
    assert!(show_transcript.contains("Cardholder: \"Ada Lovelace\""));
    assert!(show_transcript.contains("Last four: \"4242\""));
    assert!(show_transcript.contains("Expiry: 12/2030"));
    assert!(show_transcript.contains("Card number: <redacted>"));
    assert!(show_transcript.contains("CVV: <redacted>"));
    assert!(!show_transcript.contains(core::str::from_utf8(CARD_NUMBER).unwrap()));
    assert!(!show_transcript.contains("CVV: \"7391\""));
    assert!(!show_transcript.contains("Billing postal code: \"94107\""));

    for (field, secret) in [("card-number", CARD_NUMBER), ("card-cvv", CARD_CVV)] {
        let (status, transcript, stdout) = run_secret_reveal_in_pty(&home, &item_id, field, secret);
        assert!(status.success(), "{field} reveal failed: {transcript}");
        assert!(transcript.contains(&format!(
            "Secret: {:?}",
            core::str::from_utf8(secret).unwrap()
        )));
        assert!(stdout.is_empty(), "{field} entered process stdout");
        assert_transcript_excludes_secrets(&transcript);
    }

    let (audit_status, audit_transcript) = run_unlock_in_pty(
        &home,
        &["audit", "list"],
        b"action=item_create\toutcome=succeeded",
    );
    assert!(
        audit_status.success(),
        "audit list failed: {audit_transcript}"
    );
    assert!(audit_transcript.contains("action=item_read\toutcome=succeeded"));
    assert!(!audit_transcript.contains(core::str::from_utf8(CARD_NUMBER).unwrap()));
    assert_audit_rows_have_only_closed_fields(&audit_transcript);

    let (verify_status, verify_transcript) = run_unlock_in_pty(
        &home,
        &["audit", "verify"],
        b"commits=6 catalogs=2 revisions=1 items=1 audit_events=6",
    );
    assert!(
        verify_status.success(),
        "card audit verification failed: {verify_transcript}"
    );
    assert_tree_excludes(&home.0, CARD_NUMBER);
}

#[test]
fn real_cli_creates_redacts_and_separately_reveals_an_api_key() {
    let home = TestHome::new();
    let (init_status, init_transcript) = run_init_in_pty(&home);
    assert!(init_status.success(), "init failed: {init_transcript}");

    let (add_status, add_transcript) = run_add_api_key_in_pty(&home);
    assert!(add_status.success(), "API-key add failed: {add_transcript}");
    for prompt in [
        "Label: ",
        "Service: ",
        "Token: ",
        "Scopes (comma-separated, optional): ",
        "Expiry Unix seconds (optional): ",
    ] {
        assert!(add_transcript.contains(prompt), "{add_transcript}");
    }
    assert!(!add_transcript.contains(core::str::from_utf8(API_KEY_TOKEN).unwrap()));
    let item_id = extract_item_id(&add_transcript);

    let (show_status, show_transcript) =
        run_unlock_in_pty(&home, &["item", "show", &item_id], b"Token: <redacted>");
    assert!(
        show_status.success(),
        "API-key show failed: {show_transcript}"
    );
    assert!(show_transcript.contains("Label: \"Issue automation\""));
    assert!(show_transcript.contains("Service: \"api.example.test\""));
    assert!(show_transcript.contains("Scope: \"read:issues\""));
    assert!(show_transcript.contains("Scope: \"write:comments\""));
    assert!(show_transcript.contains("Expiry: 1893456000"));
    assert!(show_transcript.contains("Token: <redacted>"));
    assert!(!show_transcript.contains(core::str::from_utf8(API_KEY_TOKEN).unwrap()));

    let (reveal_status, reveal_transcript, stdout) =
        run_secret_reveal_in_pty(&home, &item_id, "api-key-token", API_KEY_TOKEN);
    assert!(
        reveal_status.success(),
        "API-key token reveal failed: {reveal_transcript}"
    );
    assert!(reveal_transcript.contains(&format!(
        "Secret: {:?}",
        core::str::from_utf8(API_KEY_TOKEN).unwrap()
    )));
    assert!(stdout.is_empty(), "API-key token entered process stdout");
    assert_transcript_excludes_secrets(&reveal_transcript);

    let (audit_status, audit_transcript) = run_unlock_in_pty(
        &home,
        &["audit", "list"],
        b"action=item_create\toutcome=succeeded",
    );
    assert!(
        audit_status.success(),
        "audit list failed: {audit_transcript}"
    );
    assert!(audit_transcript.contains("action=item_read\toutcome=succeeded"));
    assert!(!audit_transcript.contains(core::str::from_utf8(API_KEY_TOKEN).unwrap()));
    assert!(!audit_transcript.contains("Issue automation"));
    assert!(!audit_transcript.contains("read:issues"));
    assert_audit_rows_have_only_closed_fields(&audit_transcript);

    let (verify_status, verify_transcript) = run_unlock_in_pty(
        &home,
        &["audit", "verify"],
        b"commits=5 catalogs=2 revisions=1 items=1 audit_events=5",
    );
    assert!(
        verify_status.success(),
        "API-key audit verification failed: {verify_transcript}"
    );
    assert!(
        verify_transcript.contains("commits=5"),
        "{verify_transcript}"
    );
    assert!(
        verify_transcript.contains("revisions=1"),
        "{verify_transcript}"
    );
    assert!(verify_transcript.contains("items=1"), "{verify_transcript}");
    assert_tree_excludes(&home.0, API_KEY_TOKEN);
}

#[test]
fn real_cli_creates_redacts_and_separately_reveals_a_database_credential() {
    let home = TestHome::new();
    assert!(run_init_in_pty(&home).0.success());
    let (add_status, add_transcript) = run_add_database_in_pty(&home);
    assert!(
        add_status.success(),
        "database add failed: {add_transcript}"
    );
    for prompt in [
        "Label: ",
        "Engine: ",
        "Host: ",
        "Port: ",
        "Database (optional): ",
        "Username: ",
        "Password: ",
    ] {
        assert!(add_transcript.contains(prompt), "{add_transcript}");
    }
    assert!(!add_transcript.contains(core::str::from_utf8(DATABASE_PASSWORD).unwrap()));
    let item_id = extract_item_id(&add_transcript);

    let (show_status, show) =
        run_unlock_in_pty(&home, &["item", "show", &item_id], b"Password: <redacted>");
    assert!(show_status.success(), "database show failed: {show}");
    for field in [
        "Label: \"Production reporting\"",
        "Engine: \"postgres\"",
        "Host: \"db.internal.example\"",
        "Port: 5432",
        "Database: \"analytics\"",
        "Username: \"reporter\"",
        "Lease: absent",
        "Expiry: none",
        "Password: <redacted>",
    ] {
        assert!(show.contains(field), "{show}");
    }
    assert!(!show.contains(core::str::from_utf8(DATABASE_PASSWORD).unwrap()));

    let (reveal_status, reveal, stdout) =
        run_secret_reveal_in_pty(&home, &item_id, "database-password", DATABASE_PASSWORD);
    assert!(reveal_status.success(), "database reveal failed: {reveal}");
    assert!(stdout.is_empty());
    assert_transcript_excludes_secrets(&reveal);

    let (audit_status, audit) = run_unlock_in_pty(
        &home,
        &["audit", "list"],
        b"action=item_create\toutcome=succeeded",
    );
    assert!(audit_status.success(), "database audit failed: {audit}");
    assert!(audit.contains("action=item_read\toutcome=succeeded"));
    assert!(!audit.contains(core::str::from_utf8(DATABASE_PASSWORD).unwrap()));
    assert!(!audit.contains("Production reporting"));
    assert!(!audit.contains("db.internal.example"));
    assert_audit_rows_have_only_closed_fields(&audit);

    let (verify_status, verify) = run_unlock_in_pty(
        &home,
        &["audit", "verify"],
        b"commits=5 catalogs=2 revisions=1 items=1 audit_events=5",
    );
    assert!(verify_status.success(), "database verify failed: {verify}");
    assert_tree_excludes(&home.0, DATABASE_PASSWORD);
}

#[test]
fn real_cli_creates_redacts_and_separately_reveals_a_totp_seed() {
    let home = TestHome::new();
    assert!(run_init_in_pty(&home).0.success());
    let (add_status, add_transcript) = run_add_totp_in_pty(&home);
    assert!(add_status.success(), "TOTP add failed: {add_transcript}");
    for prompt in [
        "Label: ",
        "Issuer (optional): ",
        "Secret (Base32): ",
        "Algorithm (SHA1/SHA256/SHA512): ",
        "Digits (6 or 8): ",
        "Period seconds (1-3600): ",
    ] {
        assert!(add_transcript.contains(prompt), "{add_transcript}");
    }
    assert!(!add_transcript.contains(core::str::from_utf8(TOTP_BASE32).unwrap()));
    let item_id = extract_item_id(&add_transcript);

    let (show_status, show) =
        run_unlock_in_pty(&home, &["item", "show", &item_id], b"Secret: <redacted>");
    assert!(show_status.success(), "TOTP show failed: {show}");
    for field in [
        "Label: \"GitHub ada@example.com\"",
        "Issuer: \"GitHub\"",
        "Algorithm: SHA1",
        "Digits: 6",
        "Period: 30",
        "Secret: <redacted>",
    ] {
        assert!(show.contains(field), "{show}");
    }
    assert!(!show.contains(core::str::from_utf8(TOTP_BASE32).unwrap()));

    let (reveal_status, reveal, stdout) =
        run_secret_reveal_in_pty(&home, &item_id, "totp-secret", TOTP_BASE32);
    assert!(reveal_status.success(), "TOTP reveal failed: {reveal}");
    assert!(stdout.is_empty());
    assert_transcript_excludes_secrets(&reveal);

    let (audit_status, audit) = run_unlock_in_pty(
        &home,
        &["audit", "list"],
        b"action=item_create\toutcome=succeeded",
    );
    assert!(audit_status.success(), "TOTP audit failed: {audit}");
    assert!(audit.contains("action=item_read\toutcome=succeeded"));
    assert!(!audit.contains(core::str::from_utf8(TOTP_BASE32).unwrap()));
    assert!(!audit.contains("GitHub ada@example.com"));
    assert_audit_rows_have_only_closed_fields(&audit);

    let (verify_status, verify) = run_unlock_in_pty(
        &home,
        &["audit", "verify"],
        b"commits=5 catalogs=2 revisions=1 items=1 audit_events=5",
    );
    assert!(verify_status.success(), "TOTP verify failed: {verify}");
    assert_tree_excludes(&home.0, TOTP_BASE32);
    assert_tree_excludes(&home.0, TOTP_RAW_SECRET);
}

/// VLT-PM45 §9 gates 2, 3, 7, and 11, against the real executable.
///
/// The unit tests pin the code against a frozen clock; this one cannot, because
/// the real binary reads the real one. So it recomputes the answer for the step
/// the process must have been in, from the same VLT05 engine — which is proven
/// against the full RFC 6238 Appendix B table in its own crate — and checks the
/// executable agrees. What that adds over the unit tests is the wiring: that
/// the stored seed, algorithm, period, and digit count survive encryption,
/// storage, decryption, and a real PTY unchanged.
///
/// It also checks the one property no in-process test can: that the two output
/// channels really are separate file descriptors, with the code on `/dev/tty`
/// and the validity line on standard output, and that neither carries the
/// other's content.
#[test]
fn real_cli_shows_the_current_totp_code_on_the_terminal_and_its_window_on_stdout() {
    use coding_adventures_vault_auth::{TotpAlgorithm, TotpAuthenticator};
    use std::time::{SystemTime, UNIX_EPOCH};

    let home = TestHome::new();
    assert!(run_init_in_pty(&home).0.success());
    let (add_status, add_transcript) = run_add_totp_in_pty(&home);
    assert!(add_status.success(), "TOTP add failed: {add_transcript}");
    let item_id = extract_item_id(&add_transcript);

    // Without a clipboard, `--copy` fails before anything else happens, so it
    // needs no terminal at all: a plain run with no PTY would fail at the
    // passphrase prompt instead if the availability check were not first.
    if clipboard_is_absent() {
        let copied = run_plain(&home, &["totp", "code", &item_id, "--copy"]);
        assert_eq!(copied.status.code(), Some(8), "{copied:?}");
        assert!(copied.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&copied.stderr),
            "vault-pm: unsupported capability\n"
        );
    }

    let before = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let (status, transcript, stdout) = run_totp_code_in_pty(&home, &item_id, b"yes", true);
    let after = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(status.success(), "{transcript}");

    let code = extract_revealed_secret(&transcript);
    assert_eq!(code.len(), 6, "{transcript}");
    assert!(code.bytes().all(|byte| byte.is_ascii_digit()), "{code}");

    // The process read its clock somewhere between these two readings, so the
    // acceptable answers are the codes for every second it could have been in.
    // Comparing against a window rather than a point is what keeps this test
    // from failing once every thirty seconds on a period boundary.
    let authenticator =
        TotpAuthenticator::new(TOTP_RAW_SECRET.to_vec(), TotpAlgorithm::Sha1, 30, 6, 0).unwrap();
    let acceptable: Vec<String> = (before..=after)
        .map(|second| authenticator.formatted_code_at(second).unwrap().to_string())
        .collect();
    assert!(
        acceptable.contains(&code),
        "the executable produced {code}, which is no code for {before}..={after}"
    );

    // Standard output carries the window and only the window.
    let line = String::from_utf8(stdout).unwrap();
    let remaining: u64 = line
        .strip_prefix("Code valid for ")
        .and_then(|rest| rest.strip_suffix(" more seconds\n"))
        .unwrap_or_else(|| panic!("unexpected standard output: {line:?}"))
        .parse()
        .unwrap();
    assert!(
        (1..=30).contains(&remaining),
        "a window outside 1..=30 is not a window into a 30-second step"
    );
    assert!(!line.contains(&code), "the code must never reach stdout");
    assert_transcript_excludes_secrets(&transcript);
    assert!(!transcript.contains(core::str::from_utf8(TOTP_BASE32).unwrap()));

    // Two runs inside one step agree, which is the observable difference
    // between computing from a clock and drawing from a random source.
    let repeat_before = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let (repeat_status, repeat_transcript, repeat_stdout) =
        run_totp_code_in_pty(&home, &item_id, b"yes", true);
    let repeat_after = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(repeat_status.success(), "{repeat_transcript}");
    let repeat = extract_revealed_secret(&repeat_transcript);
    if before / 30 == repeat_after / 30 {
        assert_eq!(
            code, repeat,
            "two runs inside one step must produce the same code"
        );
    }
    let repeat_acceptable: Vec<String> = (repeat_before..=repeat_after)
        .map(|second| authenticator.formatted_code_at(second).unwrap().to_string())
        .collect();
    assert!(repeat_acceptable.contains(&repeat), "{repeat_transcript}");
    assert!(String::from_utf8(repeat_stdout)
        .unwrap()
        .starts_with("Code valid for "));

    // Refusal releases nothing on either channel.
    let (denied_status, denied_transcript, denied_stdout) =
        run_totp_code_in_pty(&home, &item_id, b"no", false);
    assert_eq!(denied_status.code(), Some(2), "{denied_transcript}");
    assert!(denied_stdout.is_empty());
    assert!(
        !denied_transcript.contains("Secret: \""),
        "{denied_transcript}"
    );

    // The audit chain records the reads and refuses to record their content.
    let (audit_status, audit) = run_unlock_in_pty(
        &home,
        &["audit", "list"],
        b"action=item_read\toutcome=denied",
    );
    assert!(audit_status.success(), "{audit}");
    assert_eq!(
        audit
            .lines()
            .filter(|row| row.contains("action=item_read\toutcome=succeeded"))
            .count(),
        2,
        "{audit}"
    );
    for forbidden in [
        code.as_str(),
        repeat.as_str(),
        core::str::from_utf8(TOTP_BASE32).unwrap(),
        "GitHub ada@example.com",
        "SHA1",
    ] {
        assert!(
            !audit.contains(forbidden),
            "{forbidden} leaked into the audit"
        );
    }
    assert_audit_rows_have_only_closed_fields(&audit);
    assert_tree_excludes(&home.0, TOTP_RAW_SECRET);
}

/// VLT-PM44 §8 gate 9, against the real executable.
///
/// The generator is the one command in this product that must work before
/// `vault-pm init` has ever run, because the most common moment to want a
/// generated password is while signing up for something. This test therefore
/// starts from an untouched home, generates twice, and checks four things the
/// unit tests cannot: that the process really does reach `/dev/tty` for both
/// the confirmation and the delivery, that its ordinary standard output stays
/// empty, that no vault state appears on disk, and that two runs of the same
/// command produce different passwords — which is the only end-to-end evidence
/// that the operating-system CSPRNG is genuinely wired in.
#[test]
fn real_cli_generates_a_password_without_a_vault_and_delivers_it_only_on_the_terminal() {
    let home = TestHome::new();
    let (status, transcript, stdout) =
        run_password_generate_in_pty(&home, &["password", "generate", "--reveal"], b"yes");
    assert!(status.success(), "{transcript}");
    assert!(
        stdout.is_empty(),
        "a generated password must never reach standard output"
    );

    let generated = extract_revealed_secret(&transcript);
    assert_eq!(generated.len(), 24, "{transcript}");
    assert!(generated.is_ascii());
    for character in generated.chars() {
        assert!(
            character.is_ascii_graphic() && character != '"' && character != '\\',
            "{character} is outside the generated alphabet"
        );
    }
    assert_transcript_excludes_secrets(&transcript);

    // Nothing about the request is echoed, and no vault was created or opened.
    assert!(!transcript.contains("vault-pm:"), "{transcript}");
    for child in ["config", "data", "cache"] {
        assert_eq!(
            fs::read_dir(home.0.join(child)).unwrap().count(),
            0,
            "the generator must leave {child} untouched"
        );
    }

    // A second run draws again from the operating-system CSPRNG.
    let (second_status, second_transcript, second_stdout) =
        run_password_generate_in_pty(&home, &["password", "generate", "--reveal"], b"yes");
    assert!(second_status.success(), "{second_transcript}");
    assert!(second_stdout.is_empty());
    let second = extract_revealed_secret(&second_transcript);
    assert_ne!(
        generated, second,
        "two runs producing the same password would mean the CSPRNG is not wired in"
    );

    // A narrowed policy is honoured end to end.
    let (narrow_status, narrow_transcript, narrow_stdout) = run_password_generate_in_pty(
        &home,
        &[
            "password",
            "generate",
            "--length",
            "40",
            "--no-symbols",
            "--exclude-ambiguous",
            "--reveal",
        ],
        b"yes",
    );
    assert!(narrow_status.success(), "{narrow_transcript}");
    assert!(narrow_stdout.is_empty());
    let narrowed = extract_revealed_secret(&narrow_transcript);
    assert_eq!(narrowed.len(), 40);
    for character in narrowed.chars() {
        assert!(character.is_ascii_alphanumeric(), "{character} is a symbol");
        assert!(!"01IOl|".contains(character), "{character} is ambiguous");
    }
}

/// A refused confirmation mints nothing, and an under-strength policy is
/// refused before the terminal is ever touched.
#[test]
fn real_cli_refuses_weak_password_policies_and_unconfirmed_reveals() {
    let home = TestHome::new();

    let (status, transcript, stdout) =
        run_password_generate_in_pty(&home, &["password", "generate", "--reveal"], b"no");
    assert_eq!(status.code(), Some(2), "{transcript}");
    assert!(stdout.is_empty());
    assert!(
        !transcript.contains("Secret: "),
        "a refused reveal must deliver nothing: {transcript}"
    );

    // Parse-time refusals never reach the terminal at all, so they need no
    // pseudo-terminal to observe.
    let weak = run_plain(
        &home,
        &["password", "generate", "--length", "12", "--reveal"],
    );
    assert_eq!(weak.status.code(), Some(2));
    assert!(weak.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&weak.stderr),
        "vault-pm: password policy below the minimum entropy floor\n"
    );

    // One character more clears the 80-bit floor, so the refusal is the floor
    // talking and not a blanket rejection of `--length`.
    let all_classes_disabled = run_plain(
        &home,
        &[
            "password",
            "generate",
            "--no-lowercase",
            "--no-uppercase",
            "--no-digits",
            "--no-symbols",
            "--reveal",
        ],
    );
    assert_eq!(all_classes_disabled.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&all_classes_disabled.stderr),
        "vault-pm: invalid command\n"
    );

    // `--copy` has an adapter behind it now (VLT-PM46), and this host has no
    // clipboard for it to reach, so it fails closed in the same place the old
    // blanket refusal did — before any prompt.
    if clipboard_is_absent() {
        let copied = run_plain(&home, &["password", "generate", "--copy"]);
        assert_eq!(copied.status.code(), Some(8));
        assert!(copied.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&copied.stderr),
            "vault-pm: unsupported capability\n"
        );
    }

    // The detached half of `--copy` is a real verb of this grammar, and it
    // takes its parameters from a pipe rather than from arguments. Run by
    // hand, with nothing on standard input, it reads zero bytes and refuses.
    let clear = run_plain(&home, &["clipboard", "clear"]);
    assert_eq!(clear.status.code(), Some(2), "{clear:?}");
    assert!(clear.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&clear.stderr),
        "vault-pm: invalid command\n"
    );
    for rejected in [
        vec!["clipboard"],
        vec!["clipboard", "wipe"],
        vec!["clipboard", "clear", "30"],
        vec!["--vault", "personal", "clipboard", "clear"],
    ] {
        let output = run_plain(&home, &rejected);
        assert_eq!(output.status.code(), Some(2), "{rejected:?}");
    }

    // A selector names a target this command never opens.
    let selected = run_plain(
        &home,
        &["--vault", "personal", "password", "generate", "--reveal"],
    );
    assert_eq!(selected.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&selected.stderr),
        "vault-pm: invalid command\n"
    );

    for child in ["config", "data", "cache"] {
        assert_eq!(fs::read_dir(home.0.join(child)).unwrap().count(), 0);
    }
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

fn run_restore_in_pty(home: &TestHome, source: &Path) -> (ExitStatus, String) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args([
        "--vault",
        "restore",
        "restore",
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
    let verify_prompt_start = transcript.len();
    read_until_from(
        &mut master,
        &mut transcript,
        verify_prompt_start,
        b"Vault passphrase: ",
    );
    master.write_all(TARGET_PASSPHRASE).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(
        &mut master,
        &mut transcript,
        b"Portable restore completed and verified: items=2 candidates=2 conflicts=0.",
    );
    drop(master);
    let status = child.wait().unwrap();
    (status, String::from_utf8_lossy(&transcript).into_owned())
}

fn run_add_login_in_pty(home: &TestHome) -> (ExitStatus, String) {
    run_login_form_in_pty(
        home,
        &["item", "add", "login"],
        LoginFormInput {
            title: b"Example account",
            username: b"ada@example.test",
            password: ITEM_PASSWORD,
            urls: &[
                b"https://example.test/login",
                b"https://accounts.example.test",
            ],
            notes: LOGIN_NOTES,
        },
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

fn run_add_card_in_pty(home: &TestHome) -> (ExitStatus, String) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args(["item", "add", "card"]);
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
    master.write_all(b"Personal Visa\n").unwrap();
    read_until(&mut master, &mut transcript, b"Cardholder: ");
    master.write_all(b"Ada Lovelace\n").unwrap();
    read_until(&mut master, &mut transcript, b"Card number: ");
    master.write_all(CARD_NUMBER).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"Expiry month (1-12): ");
    master.write_all(b"12\n").unwrap();
    read_until(&mut master, &mut transcript, b"Expiry year (YYYY): ");
    master.write_all(b"2030\n").unwrap();
    read_until(&mut master, &mut transcript, b"CVV: ");
    master.write_all(CARD_CVV).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(
        &mut master,
        &mut transcript,
        b"Billing postal code (optional): ",
    );
    master.write_all(b"94107\n").unwrap();
    read_until(&mut master, &mut transcript, b"Item added: ");
    let item_line = transcript.len() - b"Item added: ".len();
    read_until_from(&mut master, &mut transcript, item_line, b"\n");
    drop(master);
    let status = child.wait().unwrap();
    (status, String::from_utf8_lossy(&transcript).into_owned())
}

fn run_add_api_key_in_pty(home: &TestHome) -> (ExitStatus, String) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args(["item", "add", "api-key"]);
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
    read_until(&mut master, &mut transcript, b"Label: ");
    master.write_all(b"Issue automation\n").unwrap();
    read_until(&mut master, &mut transcript, b"Service: ");
    master.write_all(b"api.example.test\n").unwrap();
    read_until(&mut master, &mut transcript, b"Token: ");
    master.write_all(API_KEY_TOKEN).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(
        &mut master,
        &mut transcript,
        b"Scopes (comma-separated, optional): ",
    );
    master.write_all(b"read:issues,write:comments\n").unwrap();
    read_until(
        &mut master,
        &mut transcript,
        b"Expiry Unix seconds (optional): ",
    );
    master.write_all(b"1893456000\n").unwrap();
    read_until(&mut master, &mut transcript, b"Item added: ");
    let item_line = transcript.len() - b"Item added: ".len();
    read_until_from(&mut master, &mut transcript, item_line, b"\n");
    drop(master);
    let status = child.wait().unwrap();
    (status, String::from_utf8_lossy(&transcript).into_owned())
}

fn run_add_database_in_pty(home: &TestHome) -> (ExitStatus, String) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args(["item", "add", "database-credential"]);
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
    for (prompt, value) in [
        (&b"Vault passphrase: "[..], PASSPHRASE),
        (&b"Label: "[..], &b"Production reporting"[..]),
        (&b"Engine: "[..], &b"postgres"[..]),
        (&b"Host: "[..], &b"db.internal.example"[..]),
        (&b"Port: "[..], &b"5432"[..]),
        (&b"Database (optional): "[..], &b"analytics"[..]),
        (&b"Username: "[..], &b"reporter"[..]),
        (&b"Password: "[..], DATABASE_PASSWORD),
    ] {
        read_until(&mut master, &mut transcript, prompt);
        master.write_all(value).unwrap();
        master.write_all(b"\n").unwrap();
    }
    read_until(&mut master, &mut transcript, b"Item added: ");
    let item_line = transcript.len() - b"Item added: ".len();
    read_until_from(&mut master, &mut transcript, item_line, b"\n");
    drop(master);
    let status = child.wait().unwrap();
    (status, String::from_utf8_lossy(&transcript).into_owned())
}

fn run_add_totp_in_pty(home: &TestHome) -> (ExitStatus, String) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args(["item", "add", "totp"]);
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
    for (prompt, value) in [
        (&b"Vault passphrase: "[..], PASSPHRASE),
        (&b"Label: "[..], &b"GitHub ada@example.com"[..]),
        (&b"Issuer (optional): "[..], &b"GitHub"[..]),
        (&b"Secret (Base32): "[..], TOTP_BASE32),
        (&b"Algorithm (SHA1/SHA256/SHA512): "[..], &b"SHA1"[..]),
        (&b"Digits (6 or 8): "[..], &b"6"[..]),
        (&b"Period seconds (1-3600): "[..], &b"30"[..]),
    ] {
        read_until(&mut master, &mut transcript, prompt);
        master.write_all(value).unwrap();
        master.write_all(b"\n").unwrap();
    }
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
        LoginFormInput {
            title: b"Updated account",
            username: b"grace@example.test",
            password: UPDATED_ITEM_PASSWORD,
            urls: &[
                b"https://updated.example.test",
                b"https://backup.example.test",
            ],
            notes: UPDATED_LOGIN_NOTES,
        },
        b"Item updated: ",
    )
}

fn run_secret_reveal_in_pty(
    home: &TestHome,
    item_id: &str,
    field: &str,
    expected_secret: &[u8],
) -> (ExitStatus, String, Vec<u8>) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args(["item", "reveal", item_id, field]);
    home.configure(&mut command);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(slave));
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 || libc::ioctl(libc::STDERR_FILENO, tiocsctty_request(), 0) < 0 {
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
    read_until(
        &mut master,
        &mut transcript,
        b"Reveal secret on this terminal? Type yes to continue: ",
    );
    master.write_all(b"yes\n").unwrap();
    let expected_line = format!(
        "Secret: {:?}",
        core::str::from_utf8(expected_secret).unwrap()
    );
    read_until(&mut master, &mut transcript, expected_line.as_bytes());
    drain_pty(&mut master, &mut transcript);
    drop(master);
    let status = child.wait().unwrap();
    let mut stdout = Vec::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut stdout)
        .unwrap();
    (
        status,
        String::from_utf8_lossy(&transcript).into_owned(),
        stdout,
    )
}

fn run_conflict_reveal_failure_in_pty(
    home: &TestHome,
    item_id: &str,
    revision_id: &str,
    confirmation: &[u8],
    expected_error: &[u8],
) -> (ExitStatus, String, Vec<u8>) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args(["conflict", "reveal", item_id, revision_id, "login-password"]);
    home.configure(&mut command);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(slave));
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 || libc::ioctl(libc::STDERR_FILENO, tiocsctty_request(), 0) < 0 {
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
    read_until(
        &mut master,
        &mut transcript,
        b"Reveal secret on this terminal? Type yes to continue: ",
    );
    master.write_all(confirmation).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, expected_error);
    let error_line = transcript.len() - expected_error.len();
    read_until_from(&mut master, &mut transcript, error_line, b"\n");
    drop(master);
    let status = child.wait().unwrap();
    let mut stdout = Vec::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut stdout)
        .unwrap();
    (
        status,
        String::from_utf8_lossy(&transcript).into_owned(),
        stdout,
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

struct LoginFormInput<'a> {
    title: &'a [u8],
    username: &'a [u8],
    password: &'a [u8],
    urls: &'a [&'a [u8]],
    notes: &'a [u8],
}

fn run_login_form_in_pty(
    home: &TestHome,
    arguments: &[&str],
    input: LoginFormInput<'_>,
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
    master.write_all(input.title).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"Username: ");
    master.write_all(input.username).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"Password: ");
    master.write_all(input.password).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"URL count (0-16): ");
    master
        .write_all(input.urls.len().to_string().as_bytes())
        .unwrap();
    master.write_all(b"\n").unwrap();
    for url in input.urls {
        read_until(&mut master, &mut transcript, b"URL: ");
        master.write_all(url).unwrap();
        master.write_all(b"\n").unwrap();
    }
    read_until(&mut master, &mut transcript, b"Notes (optional): ");
    master.write_all(input.notes).unwrap();
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

/// Run one `password generate --reveal` against a real controlling terminal.
///
/// Unlike every other pseudo-terminal helper here this one never sends a
/// passphrase, because the command it drives never asks for one: VLT-PM44 §1
/// makes the generator vault-free, so the only terminal interaction is the
/// reveal confirmation.
/// Drive `vault-pm totp code ITEM --reveal` through a real controlling
/// terminal, with standard output captured separately.
///
/// Standard output is piped rather than pointed at the terminal because this
/// command deliberately writes to both channels — the code to `/dev/tty` and
/// the non-secret validity line to stdout — and the whole point of the test is
/// that the two never swap.
fn run_totp_code_in_pty(
    home: &TestHome,
    item_id: &str,
    confirmation: &[u8],
    expect_code: bool,
) -> (ExitStatus, String, Vec<u8>) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args(["totp", "code", item_id, "--reveal"]);
    home.configure(&mut command);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(slave));
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 || libc::ioctl(libc::STDERR_FILENO, tiocsctty_request(), 0) < 0 {
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
    read_until(
        &mut master,
        &mut transcript,
        b"Reveal secret on this terminal? Type yes to continue: ",
    );
    master.write_all(confirmation).unwrap();
    master.write_all(b"\n").unwrap();
    if expect_code {
        // The code cannot be predicted here, so wait for the opening of the
        // quoted line rather than for a known value.
        read_until(&mut master, &mut transcript, b"Secret: \"");
    }
    drain_pty(&mut master, &mut transcript);
    drop(master);
    let status = child.wait().unwrap();
    let mut stdout = Vec::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut stdout)
        .unwrap();
    (
        status,
        String::from_utf8_lossy(&transcript).into_owned(),
        stdout,
    )
}

fn run_password_generate_in_pty(
    home: &TestHome,
    arguments: &[&str],
    confirmation: &[u8],
) -> (ExitStatus, String, Vec<u8>) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args(arguments);
    home.configure(&mut command);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(slave));
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 || libc::ioctl(libc::STDERR_FILENO, tiocsctty_request(), 0) < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().unwrap();
    drop(command);
    // A generated password must not be influenced by, or leak through, an
    // attacker-controlled standard input.
    child
        .stdin
        .take()
        .unwrap()
        .write_all(STDIN_INJECTION)
        .unwrap();
    let mut transcript = Vec::new();
    read_until(
        &mut master,
        &mut transcript,
        b"Reveal secret on this terminal? Type yes to continue: ",
    );
    master.write_all(confirmation).unwrap();
    master.write_all(b"\n").unwrap();
    drain_pty(&mut master, &mut transcript);
    drop(master);
    let status = child.wait().unwrap();
    let mut stdout = Vec::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut stdout)
        .unwrap();
    (
        status,
        String::from_utf8_lossy(&transcript).into_owned(),
        stdout,
    )
}

/// Pull the one quoted secret line out of a terminal transcript.
///
/// The generator's alphabet contains neither `"` nor `\`, so the debug quoting
/// the host applies is exactly a pair of quotes around the raw value and the
/// closing quote is unambiguous.
fn extract_revealed_secret(transcript: &str) -> String {
    let start = transcript
        .find("Secret: \"")
        .expect("the terminal must receive one quoted secret line")
        + "Secret: \"".len();
    let rest = &transcript[start..];
    let end = rest.find('"').expect("the secret line must be closed");
    rest[..end].to_owned()
}

/// Whether a `--copy` run from these tests will find no clipboard at all.
///
/// [`TestHome::configure`] removes `DISPLAY` and `WAYLAND_DISPLAY`, so every
/// non-macOS host — CI included — is deterministically clipboard-free and the
/// fail-closed path is what runs. macOS reaches its pasteboard through
/// `pbcopy`, which no environment variable can take away; there the
/// fail-closed assertions are skipped rather than made to pass by writing a
/// generated password into the developer's actual clipboard. The real
/// platform round trip is proved in `vault-pm-cli-host`, behind an explicit
/// `VAULT_PM_CLIPBOARD_E2E` opt-in, for the same reason.
fn clipboard_is_absent() -> bool {
    !cfg!(target_os = "macos")
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

fn assert_audit_rows_have_only_closed_fields(transcript: &str) {
    let mut rows = 0;
    for line in transcript.lines().filter(|line| line.contains("\taction=")) {
        rows += 1;
        let fields = line.split('\t').skip(1).collect::<Vec<_>>();
        assert!(fields.len() >= 4, "malformed audit row: {line}");
        for field in fields {
            let name = field
                .split_once('=')
                .map(|(name, _)| name)
                .expect("audit field must be named");
            assert!(
                matches!(
                    name,
                    "counter" | "action" | "outcome" | "time" | "item" | "selected" | "result"
                ),
                "unexpected audit field {name}: {line}"
            );
        }
    }
    assert!(rows > 0, "expected at least one audit row: {transcript}");
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

/// Per-`poll` idle bound for every blocking PTY read below.
///
/// Every helper here reads one byte (or one buffer) at a time from a real
/// child process's terminal, and the original versions called the blocking
/// `File::read` directly with no bound at all: if the child ever wrote
/// something that didn't byte-for-byte contain the pattern being waited on —
/// a race, a buffering difference between an interactive terminal and a
/// CI-allocated PTY, a subtly different prompt, or a genuine deadlock in the
/// CLI — the read blocked forever. That turned a single wrong byte into a
/// 150-minute CI job timeout with zero diagnostic output (see PR #12042's
/// "Build and test affected packages" hang).
///
/// 60 seconds is generous for a single real Argon2id unlock plus process
/// spawn on a slow, oversubscribed CI runner, but still bounded: a timeout
/// here fails the specific `read` that stalled, in seconds, with the pattern
/// that never arrived and the transcript captured so far — not the whole job,
/// two and a half hours later, with nothing to look at.
const PTY_READ_TIMEOUT_MS: libc::c_int = 60_000;

/// Block until `fd` has data (or EOF/hangup) ready, or `PTY_READ_TIMEOUT_MS`
/// passes with nothing arriving. Returns `true` when a following `read` is
/// safe to call without blocking, `false` on timeout.
fn poll_pty_readable(fd: RawFd) -> bool {
    let mut poller = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    loop {
        // SAFETY: one initialized `pollfd` describing a file descriptor the
        // caller owns for the duration of this call.
        let ready = unsafe { libc::poll(&mut poller, 1, PTY_READ_TIMEOUT_MS) };
        if ready == 0 {
            return false;
        }
        if ready < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            panic!("pseudo-terminal poll failed: {error}");
        }
        // Any of POLLIN/POLLHUP/POLLERR means the next `read` will return
        // promptly (data, EOF, or an error) rather than block.
        return true;
    }
}

fn read_until(master: &mut File, transcript: &mut Vec<u8>, pattern: &[u8]) {
    while !transcript
        .windows(pattern.len())
        .any(|value| value == pattern)
    {
        if !poll_pty_readable(master.as_raw_fd()) {
            panic!(
                "timed out after {PTY_READ_TIMEOUT_MS}ms waiting for {:?}; transcript so far: {:?}",
                String::from_utf8_lossy(pattern),
                String::from_utf8_lossy(transcript)
            );
        }
        let mut byte = [0_u8; 1];
        match master.read(&mut byte) {
            Ok(1) => transcript.push(byte[0]),
            Ok(0) => panic!(
                "pseudo-terminal closed before expected public text: {}",
                String::from_utf8_lossy(pattern)
            ),
            Ok(_) => unreachable!(),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => panic!("pseudo-terminal read failed: {error}"),
        }
    }
}

fn read_until_from(master: &mut File, transcript: &mut Vec<u8>, start: usize, pattern: &[u8]) {
    while !transcript[start..]
        .windows(pattern.len())
        .any(|value| value == pattern)
    {
        if !poll_pty_readable(master.as_raw_fd()) {
            panic!(
                "timed out after {PTY_READ_TIMEOUT_MS}ms waiting for {:?}; transcript so far: {:?}",
                String::from_utf8_lossy(pattern),
                String::from_utf8_lossy(transcript)
            );
        }
        let mut byte = [0_u8; 1];
        match master.read(&mut byte) {
            Ok(1) => transcript.push(byte[0]),
            Ok(0) => panic!(
                "pseudo-terminal closed before line ending after public text: {}",
                String::from_utf8_lossy(pattern)
            ),
            Ok(_) => unreachable!(),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => panic!("pseudo-terminal read failed: {error}"),
        }
    }
}

fn drain_pty(master: &mut File, transcript: &mut Vec<u8>) {
    let mut bytes = [0_u8; 4096];
    loop {
        if !poll_pty_readable(master.as_raw_fd()) {
            panic!(
                "pseudo-terminal drain timed out after {PTY_READ_TIMEOUT_MS}ms waiting for more \
                 output or EOF; transcript so far: {:?}",
                String::from_utf8_lossy(transcript)
            );
        }
        match master.read(&mut bytes) {
            Ok(0) => return,
            Ok(count) => transcript.extend_from_slice(&bytes[..count]),
            Err(error) if error.raw_os_error() == Some(libc::EIO) => return,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => panic!("pseudo-terminal drain failed: {error}"),
        }
    }
}

/// Every encrypted repository object under `root`, keyed by absolute path.
///
/// VLT-PM43 §7 gate 1. The `objects` directory is `LocalVaultPaths`' object
/// root, and it holds nothing but sealed `ObjectFrameV1` records: item
/// revisions, catalogs, commits, device certificates, and audit events. The
/// walk starts from the whole test home rather than a computed path because
/// `LocalVaultPaths::resolve` is platform-dependent, and a gate that quietly
/// looked at an empty directory would pass for the wrong reason — which is why
/// the caller also asserts the map is non-empty.
fn encrypted_object_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut tree = BTreeMap::new();
    let Ok(entries) = fs::read_dir(root) else {
        return tree;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            tree.extend(encrypted_object_tree(&path));
        } else if path
            .components()
            .any(|component| component.as_os_str() == "objects")
        {
            tree.insert(path.clone(), fs::read(&path).unwrap());
        }
    }
    tree
}

fn run_passphrase_rotate_in_pty(
    home: &TestHome,
    current: &[u8],
    replacement: &[u8],
) -> (ExitStatus, String) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args(["passphrase", "rotate"]);
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
    master.write_all(current).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"New vault passphrase: ");
    master.write_all(replacement).unwrap();
    master.write_all(b"\n").unwrap();
    read_until(&mut master, &mut transcript, b"Confirm vault passphrase: ");
    master.write_all(replacement).unwrap();
    master.write_all(b"\n").unwrap();
    drain_pty(&mut master, &mut transcript);
    drop(master);
    let status = child.wait().unwrap();
    (status, String::from_utf8_lossy(&transcript).into_owned())
}

/// VLT-PM43. The real executable, over a real pseudo-terminal, across process
/// restarts.
///
/// The load-bearing assertion is the object-tree comparison: §14.8 does not ask
/// only that a rotation *work*, it asks that it not re-encrypt every item body,
/// and the only way to know that is to look at the bytes on disk. Every object
/// present before the rotation must still be present and byte-for-byte
/// unchanged; a CLI vault is audit-first from generation zero, so the rotation's
/// own audit-only commit is the one thing allowed to appear.
#[test]
fn real_cli_rotates_the_passphrase_without_re_encrypting_item_bodies() {
    let home = TestHome::new();
    let (status, transcript) = run_init_in_pty(&home);
    assert!(status.success(), "{transcript}");
    let (status, transcript) = run_add_login_in_pty(&home);
    assert!(status.success(), "{transcript}");
    let item_id = extract_item_id(&transcript);

    let before = encrypted_object_tree(&home.0);
    assert!(
        !before.is_empty(),
        "the fixture must have written encrypted objects"
    );

    let (status, transcript) = run_passphrase_rotate_in_pty(&home, PASSPHRASE, ROTATED_PASSPHRASE);
    assert!(status.success(), "{transcript}");
    assert!(
        transcript.contains("Vault passphrase rotated."),
        "{transcript}"
    );
    assert_transcript_excludes_secrets(&transcript);
    assert!(!transcript.contains("e2e rotated"), "{transcript}");

    let after = encrypted_object_tree(&home.0);
    for (path, bytes) in &before {
        assert_eq!(
            after.get(path),
            Some(bytes),
            "rotation rewrote an encrypted object: {path:?}"
        );
    }
    assert!(after.len() > before.len(), "the audit commit must appear");

    // A new process, the retired passphrase: refused with the authentication
    // class, and nothing about the vault disclosed.
    let (status, transcript) =
        run_unlock_with_passphrase_in_pty(&home, &["item", "list"], b"vault-pm: ", PASSPHRASE);
    assert_eq!(status.code(), Some(3), "{transcript}");
    assert!(!transcript.contains("Example account"), "{transcript}");

    // A new process, the new passphrase: the same item, decrypted by the same
    // unchanged root key.
    let (status, transcript) = run_unlock_with_passphrase_in_pty(
        &home,
        &["item", "list"],
        b"Example account",
        ROTATED_PASSPHRASE,
    );
    assert!(status.success(), "{transcript}");
    assert!(transcript.contains(&item_id), "{transcript}");

    // And the audit chain carried across the rotation, with the rotation in it.
    let (status, transcript) = run_unlock_with_passphrase_in_pty(
        &home,
        &["audit", "list"],
        b"action=passphrase_rotate",
        ROTATED_PASSPHRASE,
    );
    assert!(status.success(), "{transcript}");
    assert!(
        transcript.contains("action=passphrase_rotate\toutcome=succeeded"),
        "{transcript}"
    );
    assert_audit_rows_have_only_closed_fields(&transcript);

    let (status, transcript) = run_unlock_with_passphrase_in_pty(
        &home,
        &["audit", "verify"],
        b"Audit: verified",
        ROTATED_PASSPHRASE,
    );
    assert!(status.success(), "{transcript}");

    assert_tree_excludes(&home.0, PASSPHRASE);
    assert_tree_excludes(&home.0, ROTATED_PASSPHRASE);
    assert_tree_excludes(&home.0, ITEM_PASSWORD);
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

/// One deterministic attachment payload larger than a single 64 KiB chunk.
///
/// The length is deliberately not a chunk multiple, so the short final chunk
/// is exercised. A payload that happened to be an exact multiple would leave
/// the tail path untested and make "it round-tripped" a weaker statement than
/// it looks (VLT-PM47 §9.1).
fn attachment_payload() -> Vec<u8> {
    (0..(2 * 65_536 + 1_234))
        .map(|index| (index % 251) as u8)
        .collect()
}

fn run_attachment_add_in_pty(
    home: &TestHome,
    item_id: &str,
    source: &Path,
) -> (ExitStatus, String, Vec<u8>) {
    run_attachment_command_in_pty(
        home,
        &[
            "attachment",
            "add",
            item_id,
            source.to_str().expect("UTF-8 attachment source"),
        ],
        None,
    )
}

fn run_attachment_list_in_pty(home: &TestHome, item_id: &str) -> (ExitStatus, String, Vec<u8>) {
    run_attachment_command_in_pty(home, &["attachment", "list", item_id], None)
}

fn run_attachment_export_in_pty(
    home: &TestHome,
    item_id: &str,
    attachment_id: &str,
    destination: &Path,
    answer: &[u8],
) -> (ExitStatus, String, Vec<u8>) {
    run_attachment_command_in_pty(
        home,
        &[
            "attachment",
            "export",
            item_id,
            attachment_id,
            destination.to_str().expect("UTF-8 attachment destination"),
        ],
        Some(answer),
    )
}

/// Drive one attachment command through the real executable.
///
/// Standard output stays a clean pipe and the controlling terminal is on
/// standard error, the same split `run_totp_code_in_pty` uses, so the test can
/// assert on what a shell would capture separately from what the person sees.
fn run_attachment_command_in_pty(
    home: &TestHome,
    arguments: &[&str],
    confirmation: Option<&[u8]>,
) -> (ExitStatus, String, Vec<u8>) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args(arguments);
    home.configure(&mut command);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(slave));
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 || libc::ioctl(libc::STDERR_FILENO, tiocsctty_request(), 0) < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().unwrap();
    // Every other `run_*_in_pty` helper in this file drops `command` right
    // after `spawn` for exactly this reason: `Command` keeps its own copy of
    // the `Stdio` it was given for `stderr` — here, the pty slave — alive for
    // as long as the `Command` value lives, which without an explicit drop is
    // the end of this function's scope, *after* `drain_pty` below. A pty
    // master only sees EOF once every open reference to the slave is closed,
    // so an un-dropped `command` is a second, silent holder of the slave that
    // keeps `drain_pty`'s `read` waiting forever for a hang-up the real child
    // already produced. This was exactly PR #12042's CI hang: the real
    // `vault-pm attachment add` process runs to completion and exits, but the
    // test's own leaked fd keeps the terminal looking "still open" to the
    // read loop that is waiting to drain it.
    drop(command);
    child
        .stdin
        .take()
        .unwrap()
        .write_all(STDIN_INJECTION)
        .unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut transcript = Vec::new();
    read_until(&mut master, &mut transcript, b"Vault passphrase: ");
    master.write_all(PASSPHRASE).unwrap();
    master.write_all(b"\n").unwrap();
    if let Some(answer) = confirmation {
        read_until(
            &mut master,
            &mut transcript,
            b"Write this attachment's contents to a plaintext file? Type yes to continue: ",
        );
        master.write_all(answer).unwrap();
        master.write_all(b"\n").unwrap();
    }
    let mut captured = Vec::new();
    stdout.read_to_end(&mut captured).unwrap();
    drain_pty(&mut master, &mut transcript);
    let status = child.wait().unwrap();
    (
        status,
        String::from_utf8_lossy(&transcript).into_owned(),
        captured,
    )
}

/// VLT-PM47 §9. The whole attachment ceremony through the real executable: a
/// multi-chunk file goes in, its metadata comes back, and the bytes come out
/// identical — with nothing of the file or its name on disk in clear, and the
/// refusal path releasing nothing.
#[test]
fn the_real_cli_round_trips_a_multi_chunk_attachment_byte_identically() {
    let home = TestHome::new();
    assert!(run_init_in_pty(&home).0.success());
    let (add_status, add_transcript) = run_add_login_in_pty(&home);
    assert!(add_status.success(), "login add failed: {add_transcript}");
    let item_id = extract_item_id(&add_transcript);

    let payload = attachment_payload();
    let source = home.0.join("recovery-codes.bin");
    fs::write(&source, &payload).unwrap();

    let (status, transcript, stdout) = run_attachment_add_in_pty(&home, &item_id, &source);
    assert!(status.success(), "{transcript}");
    let announced = String::from_utf8(stdout).unwrap();
    let attachment_id = announced
        .strip_prefix("Attachment added: ")
        .and_then(|rest| rest.strip_suffix('\n'))
        .unwrap_or_else(|| panic!("unexpected standard output: {announced:?}"))
        .to_owned();

    // The listing is metadata, so it belongs on ordinary standard output.
    let (list_status, list_transcript, list_stdout) = run_attachment_list_in_pty(&home, &item_id);
    assert!(list_status.success(), "{list_transcript}");
    let listed = String::from_utf8(list_stdout).unwrap();
    assert!(listed.contains(&attachment_id), "{listed:?}");
    assert!(listed.contains("name=\"recovery-codes.bin\""), "{listed:?}");
    assert!(
        listed.contains(&format!("bytes={}", payload.len())),
        "{listed:?}"
    );

    // A refusal at the prompt writes no file at all.
    let refused_destination = home.0.join("refused.bin");
    let (refused_status, refused_transcript, refused_stdout) =
        run_attachment_export_in_pty(&home, &item_id, &attachment_id, &refused_destination, b"no");
    assert_eq!(refused_status.code(), Some(2), "{refused_transcript}");
    assert!(refused_stdout.is_empty());
    assert!(
        !refused_destination.exists(),
        "a refused export wrote a file"
    );

    let destination = home.0.join("exported.bin");
    let (export_status, export_transcript, export_stdout) =
        run_attachment_export_in_pty(&home, &item_id, &attachment_id, &destination, b"yes");
    assert!(export_status.success(), "{export_transcript}");
    assert_eq!(
        String::from_utf8(export_stdout).unwrap(),
        "Attachment written.\n"
    );
    assert_eq!(
        fs::read(&destination).unwrap(),
        payload,
        "the exported file must be byte-identical to the source"
    );

    // The store holds ciphertext: a distinctive interior run of the payload,
    // and the file name, appear nowhere under the platform roots. The exported
    // copy is deliberately outside those roots.
    for child in ["config", "data", "cache"] {
        assert_tree_excludes(&home.0.join(child), &payload[65_536..65_536 + 64]);
        assert_tree_excludes(&home.0.join(child), b"recovery-codes.bin");
    }

    // Every access is recorded, and the chain carries neither the name nor any
    // byte of the file.
    let (audit_status, audit) = run_unlock_in_pty(
        &home,
        &["audit", "list"],
        b"action=item_read\toutcome=denied",
    );
    assert!(audit_status.success(), "{audit}");
    assert!(
        audit.contains("action=item_update\toutcome=succeeded"),
        "{audit}"
    );
    assert!(
        audit.contains("action=item_read\toutcome=denied"),
        "the refused export must leave a row: {audit}"
    );
    assert_eq!(
        audit
            .lines()
            .filter(|row| row.contains("action=item_read\toutcome=succeeded"))
            .count(),
        2,
        "{audit}"
    );
    assert!(!audit.contains("recovery-codes.bin"), "{audit}");
    assert_audit_rows_have_only_closed_fields(&audit);

    let (verify_status, verify) =
        run_unlock_in_pty(&home, &["audit", "verify"], b"Audit: verified");
    assert!(verify_status.success(), "{verify}");
}

// ---------------------------------------------------------------------------
// VLT-PM48: local agent, IPC, and auto-lock.
// ---------------------------------------------------------------------------

/// Stops whatever agent is running for `home` when the test ends, including
/// on panic.
///
/// Every agent test spawns a real detached background process. Without this,
/// a failing assertion would leave that process running for the lifetime of
/// the CI runner (or a developer's machine), holding a socket open under a
/// temporary directory this test is about to delete out from under it.
struct AgentGuard<'a> {
    home: &'a TestHome,
}

impl Drop for AgentGuard<'_> {
    fn drop(&mut self) {
        let _ = run_plain(self.home, &["agent", "stop"]);
    }
}

/// Run one command over a real pseudo-terminal, feeding it nothing.
///
/// Used to prove a command does *not* prompt: if it needed a passphrase, the
/// prompt text would already be sitting in the transcript, and the child
/// would then block reading `/dev/tty` — which nothing here ever writes to —
/// so `drain_pty`'s bounded read fails loudly after
/// [`PTY_READ_TIMEOUT_MS`] rather than either hanging this test suite forever
/// or, worse, silently passing. Standard input is `Stdio::null()` rather than
/// piped and closed, because VLT-PM08 §2 already establishes that secret
/// collection never reads process stdin at all — the prompt this test is
/// checking for lives on the controlling terminal, not on this stream.
fn run_without_prompting_in_pty(home: &TestHome, arguments: &[&str]) -> (ExitStatus, String) {
    let (mut master, slave) = open_pty();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vault-pm"));
    command.args(arguments);
    home.configure(&mut command);
    command
        .stdin(Stdio::null())
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
    let mut transcript = Vec::new();
    drain_pty(&mut master, &mut transcript);
    drop(master);
    let status = child.wait().unwrap();
    (status, String::from_utf8_lossy(&transcript).into_owned())
}

fn stdout_string(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The complete agent lifecycle through the real executable, end to end.
///
/// This is the one test that proves the whole point of VLT-PM48: a person
/// who runs `agent unlock` once answers no further passphrase prompts for
/// ordinary authenticated commands until they explicitly lock, stop the
/// agent, or the configured idle bound elapses — while every step in between
/// remains the same real `vault-pm` binary, the same real filesystem vault,
/// and the same real Unix domain socket a second local process would have to
/// reach.
#[test]
fn real_agent_unlock_removes_the_prompt_until_locked_or_stopped() {
    let home = TestHome::new();
    assert!(run_init_in_pty(&home).0.success());

    // Nothing is running yet, and asking is not an error.
    let not_running = run_plain(&home, &["agent", "status"]);
    assert!(not_running.status.success());
    assert_eq!(stdout_string(&not_running), "Agent: not running.\n");

    // A command that needs unlocking still falls back to the ordinary prompt
    // when no agent exists — VLT-PM48 §2 requirement 4, checked before the
    // agent is even started so a regression here cannot hide behind "the
    // agent happened to be running already."
    let (status, transcript) = run_unlock_in_pty(&home, &["item", "list"], b"No items.");
    assert!(status.success(), "{transcript}");

    let started = run_plain(&home, &["agent", "start"]);
    assert!(started.status.success(), "{:?}", started);
    assert_eq!(stdout_string(&started), "Agent: started.\n");
    let _guard = AgentGuard { home: &home };

    // Starting an already-running agent is idempotent, not an error.
    let started_again = run_plain(&home, &["agent", "start"]);
    assert!(started_again.status.success());
    assert_eq!(stdout_string(&started_again), "Agent: already running.\n");

    let running_empty = run_plain(&home, &["agent", "status"]);
    assert!(running_empty.status.success());
    assert_eq!(
        stdout_string(&running_empty),
        "Agent: running. No vaults retained.\n"
    );

    // One real passphrase prompt, on a real pseudo-terminal, handed to a real
    // running agent process over its real socket.
    let (unlock_status, unlock_transcript) = run_unlock_with_passphrase_in_pty(
        &home,
        &["agent", "unlock"],
        b"Agent: unlocked.",
        PASSPHRASE,
    );
    assert!(unlock_status.success(), "{unlock_transcript}");
    assert_transcript_excludes_secrets(&unlock_transcript);

    let running_with_vault = run_plain(&home, &["agent", "status"]);
    assert!(running_with_vault.status.success());
    let status_text = stdout_string(&running_with_vault);
    assert!(
        status_text.starts_with("Agent: running.\n"),
        "{status_text}"
    );
    assert!(
        status_text.contains("personal: unlocked ("),
        "{status_text}"
    );
    assert!(status_text.contains("s remaining)\n"), "{status_text}");

    // `--vault` filtered status reports the same thing about one named vault.
    let filtered = run_plain(&home, &["--vault", "personal", "agent", "status", "--json"]);
    assert!(filtered.status.success());
    let filtered_text = stdout_string(&filtered);
    assert!(
        filtered_text.contains("\"vault\":\"personal\""),
        "{filtered_text}"
    );
    assert!(
        filtered_text.contains("\"unlocked\":true"),
        "{filtered_text}"
    );

    // The heart of the feature: a one-shot authenticated command run with
    // nothing at all on its controlling terminal still succeeds, because it
    // opportunistically reused the agent's retained passphrase instead of
    // prompting for one.
    let (reused_status, reused_transcript) = run_without_prompting_in_pty(&home, &["item", "list"]);
    assert!(reused_status.success(), "{reused_transcript}");
    assert!(
        !reused_transcript.contains("Vault passphrase"),
        "an unlocked agent must remove the prompt entirely: {reused_transcript}"
    );
    assert!(
        reused_transcript.contains("No items."),
        "{reused_transcript}"
    );

    // `agent lock` forgets it, and the very next command prompts again.
    let locked = run_plain(&home, &["agent", "lock"]);
    assert!(locked.status.success());
    assert_eq!(stdout_string(&locked), "Agent: locked.\n");

    let after_lock_status = run_plain(&home, &["agent", "status"]);
    assert!(after_lock_status.status.success());
    assert_eq!(
        stdout_string(&after_lock_status),
        "Agent: running. No vaults retained.\n"
    );

    let (reprompted_status, reprompted_transcript) =
        run_unlock_in_pty(&home, &["item", "list"], b"No items.");
    assert!(reprompted_status.success(), "{reprompted_transcript}");

    // `agent stop` tears the socket down; the next status call reports the
    // agent absent rather than erroring, and stopping an already-stopped
    // agent is equally harmless.
    let stopped = run_plain(&home, &["agent", "stop"]);
    assert!(stopped.status.success());
    assert_eq!(stdout_string(&stopped), "Agent: stopped.\n");

    let stopped_again = run_plain(&home, &["agent", "stop"]);
    assert!(stopped_again.status.success());
    assert_eq!(stdout_string(&stopped_again), "Agent: not running.\n");

    let finally_not_running = run_plain(&home, &["agent", "status"]);
    assert!(finally_not_running.status.success());
    assert_eq!(stdout_string(&finally_not_running), "Agent: not running.\n");
}

/// A successful `passphrase rotate` invalidates whatever the agent had
/// cached for that vault immediately, not only after the idle bound elapses.
#[test]
fn real_agent_cache_is_forgotten_immediately_after_a_passphrase_rotation() {
    let home = TestHome::new();
    assert!(run_init_in_pty(&home).0.success());

    let started = run_plain(&home, &["agent", "start"]);
    assert!(started.status.success());
    let _guard = AgentGuard { home: &home };

    let (unlock_status, unlock_transcript) = run_unlock_with_passphrase_in_pty(
        &home,
        &["agent", "unlock"],
        b"Agent: unlocked.",
        PASSPHRASE,
    );
    assert!(unlock_status.success(), "{unlock_transcript}");

    // Confirm the opportunistic path is live before rotating, so the assertion
    // after rotation is about the rotation and not about the agent never
    // having been reached at all.
    let (before_status, before_transcript) = run_without_prompting_in_pty(&home, &["item", "list"]);
    assert!(before_status.success(), "{before_transcript}");

    // The rotation itself still needs the (still-cached, still-correct) old
    // passphrase and a fresh one on a real terminal.
    let (rotate_status, rotate_transcript) =
        run_passphrase_rotate_in_pty(&home, PASSPHRASE, ROTATED_PASSPHRASE);
    assert!(rotate_status.success(), "{rotate_transcript}");

    // The agent's cached value is now the old, wrong passphrase. Because
    // `passphrase_rotate` forgets it immediately on success, the very next
    // command falls back to a prompt — for the *new* passphrase — rather than
    // silently trying the stale one and failing with `Locked`.
    let (after_status, after_transcript) = run_unlock_with_passphrase_in_pty(
        &home,
        &["item", "list"],
        b"No items.",
        ROTATED_PASSPHRASE,
    );
    assert!(after_status.success(), "{after_transcript}");
}

/// VLT-PM49 §9 gate 6, through the real executable with a real pseudo-
/// terminal: a Bitwarden JSON export on disk becomes a real, redacted,
/// separately-reachable vault-pm item, and the plaintext password never
/// reaches stdout, stderr, or a durable audit row.
#[test]
fn real_cli_imports_bitwarden_json_and_leaks_no_secret_anywhere() {
    const BITWARDEN_PASSWORD: &[u8] = b"e2e-bitwarden-import-secret-4f21c9";

    let home = TestHome::new();
    assert!(run_init_in_pty(&home).0.success());

    let source = home.0.join("bitwarden-export.json");
    let mut fixture = Vec::new();
    fixture.extend_from_slice(br#"{"items":[{"type":1,"name":"Imported GitHub","notes":null,"login":{"username":"e2e-imported-user","password":""#);
    fixture.extend_from_slice(BITWARDEN_PASSWORD);
    fixture.extend_from_slice(br#"","uris":[{"uri":"https://github.com"}]}}]}"#);
    fs::write(&source, &fixture).unwrap();

    let (imported_status, imported_transcript) = run_unlock_in_pty(
        &home,
        &[
            "import",
            "bitwarden",
            source.to_str().expect("UTF-8 test source path"),
        ],
        b"Import complete: created=1 skipped=0 failed=0",
    );
    assert!(imported_status.success(), "{imported_transcript}");
    assert!(!imported_transcript
        .as_bytes()
        .windows(BITWARDEN_PASSWORD.len())
        .any(|value| value == BITWARDEN_PASSWORD));
    assert_transcript_excludes_secrets(&imported_transcript);

    let (listed_status, listed_transcript) =
        run_unlock_in_pty(&home, &["item", "list"], b"Imported GitHub");
    assert!(listed_status.success(), "{listed_transcript}");
    assert!(!listed_transcript
        .as_bytes()
        .windows(BITWARDEN_PASSWORD.len())
        .any(|value| value == BITWARDEN_PASSWORD));

    let (audit_status, audit_transcript) = run_unlock_in_pty(&home, &["audit", "list"], b"action=");
    assert!(audit_status.success(), "{audit_transcript}");
    assert!(!audit_transcript
        .as_bytes()
        .windows(BITWARDEN_PASSWORD.len())
        .any(|value| value == BITWARDEN_PASSWORD));
    assert!(!audit_transcript.contains("Imported GitHub"));
    assert_audit_rows_have_only_closed_fields(&audit_transcript);

    // KDBX always fails closed before opening its source (VLT-PM49 §8) --
    // an absent path fails the identical way a real one would.
    let kdbx_result = run_plain(
        &home,
        &[
            "import",
            "kdbx",
            home.0.join("absent.kdbx").to_str().unwrap(),
        ],
    );
    assert!(!kdbx_result.status.success());
    assert!(stdout_string(&kdbx_result).is_empty());
}
