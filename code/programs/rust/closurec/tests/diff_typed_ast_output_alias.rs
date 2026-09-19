//! End-to-end parse coverage for the canonical typed-AST output flag and the
//! deprecated closurec misspelling retained by CCR-005.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn run(flag: &str) -> std::process::Output {
    Command::new(BINARY)
        .args([flag, "typed.ast", "--checks_only"])
        .output()
        .expect("run closurec")
}

#[test]
fn canonical_and_legacy_typed_ast_spellings_are_accepted() {
    for flag in [
        "--typed_ast_output_file",
        "--typed_ast_output_file__INTENRNAL_USE_ONLY",
    ] {
        let output = run(flag);
        assert!(
            output.status.success(),
            "{flag} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn conflicting_typed_ast_spellings_fail_closed() {
    let output = Command::new(BINARY)
        .args([
            "--typed_ast_output_file",
            "canonical.ast",
            "--typed_ast_output_file__INTENRNAL_USE_ONLY",
            "legacy.ast",
            "--checks_only",
        ])
        .output()
        .expect("run closurec");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("disagree"),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}
