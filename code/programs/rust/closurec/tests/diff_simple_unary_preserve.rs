//! Integration test for the `tests/diff/simple-unary-preserve/` fixture.
//!
//! Regression oracle for the **prefix-unary-operator-drop miscompile**. The
//! bridge (`javascript-parser/src/bridge.rs`) discriminated the two
//! `unary_expression` grammar alternatives by counting AST child *nodes*; the
//! operator is a *token* (filtered out by `node_children`), so every
//! prefix-operator form looked like a pass-through and the operator was
//! silently dropped — `!a` → `a`, `-b` → `b`, `~c` → `c`. That is a
//! **miscompile** at SIMPLE/ADVANCED (WHITESPACE_ONLY kept the operators
//! because it never runs the bridge).
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=first();var b=second();var c=third();report(!a,-b,~c,!(a == b));
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-unary-preserve/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_unary_preserve_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-unary-preserve/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// The prefix operators must all survive into the output. Before the bridge
/// fix each of these vanished, so asserting their presence is the direct
/// regression guard against the operator-drop miscompile.
#[test]
fn simple_unary_preserve_keeps_every_prefix_operator() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("!a"), "logical-not operator dropped; got:\n{actual}");
    assert!(actual.contains("-b"), "unary-minus operator dropped; got:\n{actual}");
    assert!(actual.contains("~c"), "bitwise-not operator dropped; got:\n{actual}");
    // `!(a == b)` must keep its parens — `!a == b` would reparse as
    // `(!a) == b`, a different program.
    assert!(
        actual.contains("!(a == b)"),
        "negated-equality lost its parentheses (precedence miscompile); got:\n{actual}",
    );
}

/// Regression guard: the output must NOT be the WHITESPACE_ONLY fallback. The
/// unused `var dead = 4 + 5;` is removed only by the typed pipeline, so its
/// absence proves the SIMPLE optimizer ran (and therefore the bridge ran, and
/// the operators survived *it*). WHITESPACE_ONLY keeps `dead` verbatim.
#[test]
fn simple_unary_preserve_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains("dead"),
        "expected the unused `dead` binding to be removed by the typed pipeline \
         (proving this is the SIMPLE optimizer, not the whitespace fallback); \
         got:\n{actual}",
    );
}
