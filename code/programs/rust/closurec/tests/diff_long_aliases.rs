//! Binary-level parity tests for alternate upstream long flag spellings.
//!
//! Each alias is compared directly with its canonical spelling. This proves
//! that cli-builder resolves the spelling before closurec's configuration
//! wiring sees it: exit status, stdout, and stderr must all be identical.

use std::process::{Command, Output};

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn run(args: &[&str]) -> Output {
    Command::new(BINARY)
        .args(args)
        .output()
        .expect("run closurec")
}

fn assert_alias_matches(canonical: &[&str], alias: &[&str]) {
    let expected = run(canonical);
    let actual = run(alias);
    assert_eq!(actual.status, expected.status, "alias args: {alias:?}");
    assert_eq!(actual.stdout, expected.stdout, "alias args: {alias:?}");
    assert_eq!(actual.stderr, expected.stderr, "alias args: {alias:?}");
    assert!(
        actual.status.success(),
        "alias invocation failed: {alias:?}\n{}",
        String::from_utf8_lossy(&actual.stderr)
    );
}

#[test]
fn upstream_long_aliases_match_their_canonical_flags() {
    assert_alias_matches(&["--define=DEBUG=false"], &["--D=DEBUG=false"]);
    assert_alias_matches(&["--checks_only"], &["--checks-only"]);
    assert_alias_matches(&["--jscomp_dev_mode", "OFF"], &["--dev_mode", "OFF"]);
    assert_alias_matches(
        &["--warnings_allowlist_file", "allowlist.txt"],
        &["--warnings_whitelist_file", "allowlist.txt"],
    );
}
