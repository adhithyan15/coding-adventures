//! Integration test for the `tests/diff/simple-void-drop/` fixture.
//!
//! End-to-end oracle for the `void`-operator drop in statement position
//! (`closure-pass-dce`): `void <impure>;` as an expression statement drops the
//! redundant `void` wrapper, keeping the operand (`void f();` -> `f();`). A
//! `void` whose result is observed (a non-statement position such as
//! `h(void g())`) is kept. A pure `void <lit>;` is declined here.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! f();a.b();new C;a();b();h(void g());
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/simple-void-drop/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_void_drop_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-void-drop/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
