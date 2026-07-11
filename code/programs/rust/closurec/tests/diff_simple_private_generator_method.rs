//! Integration test for the `tests/diff/simple-private-generator-method/` fixture.
//!
//! Exercises a **private generator method** (`*#g(){…}`, a `MethodDefinition`
//! with a `PropertyKey::PrivateName` key whose `value` [`FunctionExpression`] has
//! `generator: true`) end-to-end at SIMPLE — the CLOC12.182 bridge of the `*`
//! generator marker in `convert_private_method_definition`. Private accessors
//! (`get #x()`/`set #x()`, CLOC12.179) and public generator methods (`*gen()`,
//! CLOC12.181) already bridged; the private *generator* form still declined until
//! now. Only the private *async* form (`async #m()`) still declines.
//!
//! The fixture is `class C { *#g(){ return 1 + 2 } }` compiled at SIMPLE.
//! Two facts prove the whole pipeline ran through the private generator method:
//!   1. the class round-trips with a `*#g` head — proving the bridge set the
//!      `generator` flag (and the emitter reprinted the `*` before the private
//!      key), not a WHITESPACE_ONLY fallback; and
//!   2. the body folds — `return 1 + 2` → `return 3` — proving the SIMPLE
//!      pipeline descended INTO the private generator's body. A WHITESPACE_ONLY
//!      fallback would leave `1+2` intact.
//! Before this bridge change a private generator method DECLINED, dropping the
//! file to WHITESPACE_ONLY (`class C{*#g(){return 1+2}};`) and assertion (2)
//! failed.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-private-generator-method/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_private_generator_method_folds_body() {
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
    let expected =
        std::fs::read_to_string("tests/diff/simple-private-generator-method/expected.stdout")
            .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let a = actual.replace(' ', "");
    // (1) the class round-tripped with a `*#g` private-generator head.
    assert!(
        (a.contains("classC{") || a.contains("class C{")) && a.contains("*#g("),
        "private generator method did not round-trip with a `*#g` head: {actual}"
    );
    // (2) the body folded — proving the pipeline descended INTO the method body
    //     (`return 1+2`→`return 3`). A WHITESPACE_ONLY fallback would leave the
    //     arithmetic intact.
    assert!(
        a.contains("return3"),
        "private generator method body did not fold to `return 3`: {actual}"
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
