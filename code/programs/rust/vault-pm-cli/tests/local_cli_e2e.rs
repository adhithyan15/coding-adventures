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
