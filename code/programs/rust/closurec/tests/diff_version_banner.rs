//! Integration test for the `tests/diff/version-banner/` fixture.
//!
//! Exercises CLOC11.55 — `--version` Closure-Compiler-style
//! banner. The output must contain both the `Closure Compiler `
//! marker (so toolchains recognise this as a drop-in) and the
//! `Version: <semver>` line (so version-extracting scripts keep
//! working).
//!
//! We don't pin byte-for-byte because the embedded version
//! string changes with every release; instead, the test
//! asserts the structural invariants.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

#[test]
fn version_flag_emits_cc_style_banner() {
    let out = Command::new(BINARY)
        .arg("--version")
        .output()
        .expect("run closurec");

    assert!(
        out.status.success(),
        "version flag should exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let text = String::from_utf8_lossy(&out.stdout);

    // First line: `Closure Compiler ...` marker. Tools that grep
    // upstream CC's stdout for this string keep working.
    assert!(
        text.starts_with("Closure Compiler "),
        "expected `Closure Compiler ` prefix, got:\n{text}"
    );

    // Second line: `Version: <semver>`. Standard hook for
    // version-extracting scripts.
    assert!(
        text.contains("\nVersion: "),
        "expected `Version: ` line, got:\n{text}"
    );

    // The embedded semver from Cargo must appear so users
    // running --version can confirm which build they have.
    assert!(
        text.contains(env!("CARGO_PKG_VERSION")),
        "expected embedded version {} in output, got:\n{text}",
        env!("CARGO_PKG_VERSION")
    );

    // Clean trailing newline (no blank line drift).
    assert!(text.ends_with("\n"));
    assert!(
        !text.ends_with("\n\n"),
        "should not end with a blank line:\n{text}"
    );
}
