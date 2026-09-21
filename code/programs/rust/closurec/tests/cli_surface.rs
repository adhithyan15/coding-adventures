//! Offline verifier for the pinned Closure Compiler command-line surface.

#[path = "cli_surface_support/mod.rs"]
mod cli_surface_support;

use cli_surface_support::{verify_report, AuditReport};
use serde_json::{json, Value};

const REPORT: &str = "tests/cli-surface/v20260915-audit.json";

fn spec_bytes() -> Vec<u8> {
    std::fs::read("cli.spec.json").expect("read cli.spec.json")
}

fn report() -> AuditReport {
    let bytes = std::fs::read(REPORT).expect("read CLI surface report");
    serde_json::from_slice(&bytes).expect("parse strict CLI surface report")
}

fn assert_has(errors: &[String], needle: &str) {
    assert!(
        errors.iter().any(|error| error.contains(needle)),
        "expected error containing {needle:?}, got {errors:#?}"
    );
}

#[test]
fn checked_in_surface_is_complete_and_current() {
    let errors = verify_report(&report(), &spec_bytes());
    assert!(
        errors.is_empty(),
        "CLI surface audit failed:\n{}",
        errors.join("\n")
    );
}

#[test]
fn rejects_bad_source_pin() {
    let mut mutant = report();
    mutant.upstream_source.sha256 = "0".repeat(64);
    assert_has(&verify_report(&mutant, &spec_bytes()), "source pin");
}

#[test]
fn rejects_unclassified_upstream_drift() {
    let mut mutant = report();
    mutant
        .classification
        .direct_upstream_flags
        .retain(|name| name != "angular_pass");
    assert_has(
        &verify_report(&mutant, &spec_bytes()),
        "upstream classification mismatch",
    );
}

#[test]
fn rejects_duplicate_classification_ownership() {
    let mut mutant = report();
    mutant
        .classification
        .generated_flags
        .push("angular_pass".to_string());
    let errors = verify_report(&mutant, &spec_bytes());
    assert_has(&errors, "generated flag classification drifted");
    assert_has(&errors, "classification owners");
}

#[test]
fn rejects_duplicate_entries_within_one_classification() {
    let mut mutant = report();
    mutant
        .classification
        .direct_upstream_flags
        .insert(0, "angular_pass".to_string());
    assert_has(
        &verify_report(&mutant, &spec_bytes()),
        "strictly sorted and unique",
    );
}

#[test]
fn rejects_unclassified_local_flag() {
    let mut root: Value = serde_json::from_slice(&spec_bytes()).expect("parse spec");
    root["flags"]
        .as_array_mut()
        .expect("flags array")
        .push(json!({
            "id": "surprise",
            "long": "surprise",
            "type": "boolean",
            "description": "mutant"
        }));
    let mutant_spec = serde_json::to_vec(&root).expect("serialize mutant spec");
    let mut mutant_report = report();
    mutant_report.local_spec_sha256 = coding_adventures_sha256::sha256_hex(&mutant_spec);
    assert_has(&verify_report(&mutant_report, &mutant_spec), "unclassified");
}

#[test]
fn rejects_stale_deprecated_alias() {
    let mut mutant = report();
    mutant.classification.deprecated_aliases[0].canonical = "debug".to_string();
    assert_has(
        &verify_report(&mutant, &spec_bytes()),
        "deprecated alias classification drifted",
    );
}

#[test]
fn rejects_unreviewed_upstream_alias() {
    let mut root: Value = serde_json::from_slice(&spec_bytes()).expect("parse spec");
    let flags = root["flags"].as_array_mut().expect("flags array");
    let checks = flags
        .iter_mut()
        .find(|flag| flag["id"] == "checks_only")
        .expect("checks_only flag");
    checks["long_aliases"] = json!([]);
    let mutant_spec = serde_json::to_vec(&root).expect("serialize mutant spec");
    let mut mutant_report = report();
    mutant_report.local_spec_sha256 = coding_adventures_sha256::sha256_hex(&mutant_spec);
    assert_has(
        &verify_report(&mutant_report, &mutant_spec),
        "--checks-only",
    );
}

#[test]
fn rejects_unclassified_local_long_alias() {
    let mut root: Value = serde_json::from_slice(&spec_bytes()).expect("parse spec");
    let flags = root["flags"].as_array_mut().expect("flags array");
    let define = flags
        .iter_mut()
        .find(|flag| flag["id"] == "define")
        .expect("define flag");
    define["long_aliases"] = json!(["D", "surprise"]);
    let mutant_spec = serde_json::to_vec(&root).expect("serialize mutant spec");
    let mut mutant_report = report();
    mutant_report.local_spec_sha256 = coding_adventures_sha256::sha256_hex(&mutant_spec);
    assert_has(
        &verify_report(&mutant_report, &mutant_spec),
        "unclassified local long aliases",
    );
}

#[test]
fn rejects_changed_supported_alias_mapping() {
    let mut mutant = report();
    mutant
        .upstream_options
        .iter_mut()
        .find(|option| option.name == "compilation_level")
        .expect("compilation level option")
        .aliases[0] = "-D".to_string();
    let errors = verify_report(&mutant, &spec_bytes());
    assert_has(&errors, "surface SHA-256 mismatch");
    assert_has(&errors, "alias map does not match");
}

#[test]
fn rejects_stale_counts() {
    let mut mutant = report();
    mutant.counts.local_spec_flags += 1;
    assert_has(
        &verify_report(&mutant, &spec_bytes()),
        "counts do not match",
    );
}
