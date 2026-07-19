//! Integration test for the `tests/diff/simple-inline-variables/` fixture.
//!
//! Exercises CLOC13.H — the `inline-variables` pass propagates a top-level
//! `const = literal` to its use sites. It is a value-copying pass (it does not
//! delete the source declaration) and runs at SIMPLE; the fixed-point
//! `constant-fold` sweep then folds the now-concrete arithmetic it exposes.
//! What does NOT run at SIMPLE is `remove-unused-vars`, so the emptied
//! declaration is KEPT (open-world). `const RATE = 2; total(base * RATE);
//! margin(RATE + 1);` becomes `const RATE=2;total(base*2);margin(3);`. Under
//! ADVANCED the now-unused `const RATE = 2` is dropped, giving
//! `total(base*2);margin(3);`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-inline-variables/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_inline_variables_fixture_matches_expected_stdout() {
    let flags = read_flags();
    let out = Command::new(BINARY)
        .args(&flags)
        .output()
        .expect("run closurec");

    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );

    let actual = String::from_utf8_lossy(&out.stdout);
    let expected = std::fs::read_to_string("tests/diff/simple-inline-variables/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
