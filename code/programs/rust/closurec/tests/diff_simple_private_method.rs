//! Integration test for the `tests/diff/simple-private-method/` fixture.
//!
//! Exercises a **private class method** (`#m(){}`, a `ClassMember::Method` whose
//! key is a `PropertyKey::PrivateName`) end-to-end at SIMPLE — the CLOC12.178
//! bridge of the `private_method_definition` grammar node, on top of the
//! `PropertyKey::PrivateName` node + emit arms (CLOC12.177).
//!
//! The fixture is `class C { #m(){ return 1 + 2 } }` compiled at SIMPLE.
//! Two facts prove the whole pipeline ran through the private method:
//!   1. the class round-trips with a `#`-prefixed method key — proving the bridge
//!      lowered the `private_method_definition` node to a `ClassMember::Method`
//!      the emitter can print, not a WHITESPACE_ONLY fallback; and
//!   2. the method body folds — `1 + 2` → `3` — proving the SIMPLE pipeline
//!      descended INTO the private method's body. A WHITESPACE_ONLY fallback
//!      would leave `1+2` intact.
//! Were the bridge to decline the private method, the file would drop to
//! WHITESPACE_ONLY (`class C{#m(){return 1+2}};`) and assertion (2) would fail.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-private-method/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_private_method_folds_body() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-private-method/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // Guard the *point* of the fixture. Strip spaces so the checks are
    // insensitive to inter-token whitespace.
    let a = actual.replace(' ', "");
    // (1) the class round-tripped with a private `#m` method key — proving the
    //     bridge lowered the `private_method_definition` node.
    assert!(
        (a.contains("classC{") || a.contains("class C{")) && a.contains("#m("),
        "private-method class did not round-trip with a `#m(` key: {actual}"
    );
    // (2) the method body folded — proving the pipeline descended INTO the private
    //     method's body (`1+2`→`3`). A WHITESPACE_ONLY fallback would leave the
    //     arithmetic intact.
    assert!(
        a.contains("return3"),
        "private method body did not fold to `return 3`: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "unfolded arithmetic present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
    // (3) a class *declaration* terminates with `};` when it is the LAST
    //     program item (oracle-verified: the real Closure emits
    //     `class C{m(){return 3}};`) and is NOT paren-wrapped (unlike the
    //     class *expression* form `(class …);`). NOTE: the trailing `;` used
    //     to double as a WHITESPACE_ONLY-fallback detector; it no longer can,
    //     since both paths now end in `;`. The constant-fold assertion above
    //     is the real fallback discriminator — a whitespace-only pass cannot
    //     fold, so a folded body proves the optimizing pipeline ran.
    let t = actual.trim_end_matches('\n');
    assert!(
        t.ends_with("};") && !t.starts_with('('),
        "class declaration must terminate with a semicolon as the last program item and not be paren-wrapped: {actual}"
    );
}
