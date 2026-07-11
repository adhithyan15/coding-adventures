//! Integration test for the `tests/diff/simple-computed-key/` fixture.
//!
//! Exercises a **computed class-member key** (`[k]`, a `PropertyKey::Expression`
//! with `computed: true`) end-to-end at SIMPLE — the CLOC12.180 bridge of the
//! computed `[expr]` key form in `convert_property_key`. The typed AST
//! (`PropertyKey::Expression`) and emitter (`emit_property_key` brackets it)
//! already supported computed keys; only the bridge declined them.
//!
//! The fixture is `class C { [k] = 1 + 2 }` compiled at SIMPLE.
//! Two facts prove the whole pipeline ran through the computed key:
//!   1. the class round-trips with a bracketed `[k]` key — proving the bridge
//!      lowered the computed key to `PropertyKey::Expression`, not a
//!      WHITESPACE_ONLY fallback; and
//!   2. the initializer folds — `1 + 2` → `3` — proving the SIMPLE pipeline
//!      descended INTO the field's initializer. A WHITESPACE_ONLY fallback would
//!      leave `1+2` intact.
//! Before this bridge change a computed key DECLINED, dropping the file to
//! WHITESPACE_ONLY (`class C{[k]=1+2};`) and assertion (2) failed.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-computed-key/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_computed_key_folds_initializer() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-computed-key/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let a = actual.replace(' ', "");
    // (1) the class round-tripped with a bracketed `[k]` computed key.
    assert!(
        (a.contains("classC{") || a.contains("class C{")) && a.contains("[k]"),
        "computed-key class did not round-trip with a `[k]` key: {actual}"
    );
    // (2) the initializer folded — proving the pipeline descended INTO the field
    //     initializer (`1+2`→`3`). A WHITESPACE_ONLY fallback would leave the
    //     arithmetic intact.
    assert!(
        a.contains("[k]=3"),
        "computed-key field initializer did not fold to `3`: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "unfolded arithmetic present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
    // (3) a class *declaration* emits bare — NO wrapping paren (a WHITESPACE_ONLY
    //     fallback for a class *expression* would wrap; a declaration must not).
    let t = actual.trim_end_matches('\n');
    assert!(
        t.ends_with('}') && !t.starts_with('('),
        "class declaration must emit bare (no wrap): {actual}"
    );
}
