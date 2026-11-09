//! Audit A3 residual, against the real generated target: a filtered run must
//! never be a green run and must never record a manifest.
//!
//! `WARRANT_EVIDENCE_FAULT=none` with `WARRANT_EVIDENCE_FILTER` used to be
//! accepted, giving an exit-0 run of a hand-picked subset of tests.  The check
//! is made by running the actual `warrant_evidence` target through cargo.

use std::path::PathBuf;
use std::process::Command;

/// `cargo test --test warrant_evidence` for this package.  Going through cargo
/// (rather than hunting for a previously built executable) guarantees the
/// target under test is rebuilt from the current sources, never a stale one.
fn warrant_evidence() -> Command {
    let mut cmd = Command::new(env!("CARGO"));
    cmd.args(["test", "-q", "--manifest-path"])
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .args(["--test", "warrant_evidence"])
        .env_remove("WARRANT_EVIDENCE_FAULT")
        .env_remove("WARRANT_EVIDENCE_FILTER")
        .env_remove("WARRANT_EVIDENCE_OUT");
    cmd
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ct-sabotage-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn fault_none_with_a_filter_is_refused_and_records_nothing() {
    let dir = scratch("none-filter");
    let out_path = dir.join("manifest.json");
    let out = warrant_evidence()
        .env("WARRANT_EVIDENCE_FAULT", "none")
        .env("WARRANT_EVIDENCE_FILTER", "/exhaustive/")
        .env("WARRANT_EVIDENCE_OUT", &out_path)
        .output()
        .expect("run target");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "must be refused, not exit 0: {stderr}");
    assert!(stderr.contains("real fault"), "{stderr}");
    assert!(!out_path.exists(), "a filtered run must not write a manifest");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn filter_without_any_fault_is_refused_and_records_nothing() {
    let dir = scratch("filter-only");
    let out_path = dir.join("manifest.json");
    let out = warrant_evidence()
        .env("WARRANT_EVIDENCE_FILTER", "/exhaustive/")
        .env("WARRANT_EVIDENCE_OUT", &out_path)
        .output()
        .expect("run target");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("WARRANT_EVIDENCE_FAULT"));
    assert!(!out_path.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_faulted_filtered_run_fails_and_records_nothing() {
    let dir = scratch("faulted");
    let out_path = dir.join("manifest.json");
    let out = warrant_evidence()
        .env("WARRANT_EVIDENCE_FAULT", "invert")
        .env("WARRANT_EVIDENCE_FILTER", "/exhaustive/")
        .env("WARRANT_EVIDENCE_OUT", &out_path)
        .output()
        .expect("run target");
    assert!(!out.status.success(), "the sabotage must be detected");
    assert!(!out_path.exists(), "a sabotaged run must not write a manifest");
    let _ = std::fs::remove_dir_all(&dir);
}
