//! End-to-end tests of the public Maxima façade, exercised exactly as an
//! external caller (e.g. `maxima-repl`) would: through `MaximaSession::feed`
//! and the one-shot `eval`. These complement the in-module unit tests by
//! pinning the crate's *public* contract.

use coding_adventures_maxima_runtime::{eval, MaximaSession};

#[test]
fn a_multi_statement_feed_echoes_each_displayed_result() {
    // Two displayed statements in a single feed → two echo lines, %o1 then %o2.
    let out = eval("1 + 1; diff(x^2, x);").unwrap();
    assert!(out.contains("(%o1) "), "first echo missing: {out:?}");
    assert!(out.contains("(%o2) "), "second echo missing: {out:?}");
    assert!(out.contains('2'), "diff(x^2,x)=2*x should appear: {out:?}");
}

#[test]
fn bindings_persist_across_feeds() {
    let mut s = MaximaSession::new();
    assert_eq!(s.feed("a : 10$").unwrap(), "");
    assert_eq!(s.feed("b : 20$").unwrap(), "");
    let out = s.feed("a + b;").unwrap();
    assert!(
        out.contains("30"),
        "expected 30 from persisted a+b: {out:?}"
    );
}

#[test]
fn integration_genuinely_reduces() {
    // A real symbolic reduction round-trips through the façade unchanged.
    let out = eval("integrate(x^2, x);").unwrap();
    assert!(out.contains('x') && out.contains('3'), "got {out:?}");
}

#[test]
fn bad_input_is_an_error_not_a_crash() {
    // The macsyma lexer panics on `@`; the façade must turn that into an Err.
    assert!(eval("@@@;").is_err());
    // And a session survives a bad feed and keeps working.
    let mut s = MaximaSession::new();
    let _ = s.feed("@@@;");
    assert!(s.feed("2 + 2;").unwrap().contains('4'));
}
