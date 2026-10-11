//! CV03's first trust boundary: a declared journal cannot disappear in a
//! compatibility import. These fixtures use raw JSON so duplicate decoded keys
//! survive until the real importer sees them. A Value-based fixture would erase
//! the evidence before the code under test ran.
//!
//! Absence keeps the legacy allocator contract. Presence must instead validate
//! its closed schema and complete evidence, or fail explicitly. A private
//! unchecked-import marker does not excuse discarding the journal itself.

use coding_adventures_correlation_vector::CVLog;

fn rejects_declared_journal(fields: &str) {
    let input = format!(r#"{{"enabled":true,"entries":{{}},"pass_order":[],{fields}}}"#);
    let imported = CVLog::from_json_string(&input);
    assert!(
        imported.is_err(),
        "allocator import accepted malformed journal presence and can discard it on export: {input}"
    );
}

#[test]
fn absent_journal_preserves_the_legacy_allocator_contract() {
    for enabled in [true, false] {
        let legacy = CVLog::new(enabled);
        let encoded = legacy.to_json_string().unwrap();
        assert!(!encoded.contains("\"journal\""));
        let restored = CVLog::from_json_string(&encoded).unwrap();
        assert_eq!(restored.is_enabled(), enabled);
        assert!(!restored.to_json_string().unwrap().contains("\"journal\""));
    }
}

#[test]
fn present_null_journal_is_not_absence() {
    rejects_declared_journal(r#""journal":null"#);
}

#[test]
fn present_nonobject_journal_is_not_absence() {
    rejects_declared_journal(r#""journal":[]"#);
}

#[test]
fn unsupported_journal_version_is_not_discarded() {
    rejects_declared_journal(
        r#""journal":{"version":"chronology-v99","coverage":"full","last_sequence":"0000000000000000","events":[]}"#,
    );
}

#[test]
fn numeric_journal_watermark_is_not_normalized() {
    rejects_declared_journal(
        r#""journal":{"version":"chronology-v1","coverage":"full","last_sequence":0,"events":[]}"#,
    );
}

#[test]
fn duplicate_decoded_journal_fields_are_not_ignored() {
    rejects_declared_journal(
        r#""journal":null,"\u006aournal":{"version":"chronology-v1","coverage":"full","last_sequence":"0000000000000000","events":[]}"#,
    );
}

#[test]
fn disabled_recording_does_not_launder_present_full_chronology() {
    let input = r#"{"enabled":false,"entries":{},"pass_order":[],"journal":{"version":"chronology-v1","coverage":"full","last_sequence":"0000000000000000","events":[]}}"#;
    assert!(
        CVLog::from_json_string(input).is_err(),
        "disabled allocator state cannot claim complete recorded chronology"
    );
}
