//! End-to-end integration tests for the Wolfram runtime.
//!
//! These exercise the *whole* pipeline — parse → lower → evaluate (via the shared
//! `symbolic-vm`) → pretty-print — through the public [`WolframSession`] /
//! [`eval`] surface, mirroring how a REPL or embedder uses the crate. The unit
//! tests inside `src/` pin the individual stages; these assert the observable
//! string-in / string-out contract a user sees.

use coding_adventures_wolfram_runtime::{eval, WolframSession};

/// A representative `1 + 2*3 → 7` end-to-end (the canonical "arithmetic works"
/// check from the W-4 brief).
#[test]
fn arithmetic_precedence_end_to_end() {
    assert_eq!(eval("1 + 2*3\n").unwrap(), "Out[1]= 7\n");
    assert_eq!(eval("(1 + 2)*3\n").unwrap(), "Out[1]= 9\n");
    assert_eq!(eval("2^10\n").unwrap(), "Out[1]= 1024\n");
}

/// The explicit head-applications evaluate identically to their infix forms —
/// the head-name bridge (Plus→Add, Times→Mul, Power→Pow) is the crux of W-4.
#[test]
fn head_applications_match_infix() {
    assert_eq!(eval("Plus[1, 2, 3]\n").unwrap(), "Out[1]= 6\n");
    assert_eq!(eval("Times[2, 3, 4]\n").unwrap(), "Out[1]= 24\n");
    assert_eq!(eval("Power[2, 10]\n").unwrap(), "Out[1]= 1024\n");
    assert_eq!(eval("Subtract[10, 3]\n").unwrap(), "Out[1]= 7\n");
}

/// Symbols stay symbolic; algebraic identities fold (the `SymbolicBackend`).
#[test]
fn symbolic_evaluation() {
    assert_eq!(eval("x + 0\n").unwrap(), "Out[1]= x\n");
    assert_eq!(eval("x*1\n").unwrap(), "Out[1]= x\n");
    assert_eq!(eval("x^1\n").unwrap(), "Out[1]= x\n");
    assert_eq!(eval("x^0\n").unwrap(), "Out[1]= 1\n");
}

/// List literals evaluate element-wise and round-trip to brace notation.
#[test]
fn lists() {
    assert_eq!(eval("{1, 2, 3}\n").unwrap(), "Out[1]= {1, 2, 3}\n");
    assert_eq!(eval("{1 + 1, 2*2, 3^2}\n").unwrap(), "Out[1]= {2, 4, 9}\n");
    assert_eq!(eval("{}\n").unwrap(), "Out[1]= {}\n");
    assert_eq!(
        eval("{1, {2, 3}, 4}\n").unwrap(),
        "Out[1]= {1, {2, 3}, 4}\n"
    );
}

/// Built-in elementary functions reach the shared handlers.
#[test]
fn elementary_functions() {
    assert_eq!(eval("Sin[0]\n").unwrap(), "Out[1]= 0\n");
    assert_eq!(eval("Cos[0]\n").unwrap(), "Out[1]= 1\n");
    // An unknown head passes through unevaluated (Mathematica semantics).
    assert_eq!(eval("g[1, 2]\n").unwrap(), "Out[1]= g[1, 2]\n");
}

/// Assignment binds in the session env and persists across `feed` calls.
#[test]
fn stateful_assignment() {
    let mut s = WolframSession::new();
    assert_eq!(s.feed("a = 10\n").unwrap(), "Out[1]= 10\n");
    assert_eq!(s.feed("b = a + 5\n").unwrap(), "Out[2]= 15\n");
    assert_eq!(s.feed("a*b\n").unwrap(), "Out[3]= 150\n");
}

/// A user-defined function via `:=`, then applied.
#[test]
fn user_functions() {
    let mut s = WolframSession::new();
    s.feed("cube[x_] := x^3;\n").unwrap();
    assert_eq!(s.feed("cube[2]\n").unwrap(), "Out[2]= 8\n");
    assert_eq!(s.feed("cube[5]\n").unwrap(), "Out[3]= 125\n");
}

/// `/.` replacement with single rules, list rules, and pattern rules.
#[test]
fn replace_all() {
    assert_eq!(eval("x /. x -> 9\n").unwrap(), "Out[1]= 9\n");
    assert_eq!(
        eval("{a, b} /. {a -> 1, b -> 2}\n").unwrap(),
        "Out[1]= {1, 2}\n"
    );
    // A pattern rule that captures and reuses the bound name on the RHS.
    assert_eq!(eval("h[5] /. h[n_] -> n + 1\n").unwrap(), "Out[1]= 6\n");
}

/// Comparisons and logic fold when fully numeric, stay symbolic otherwise.
#[test]
fn comparisons_and_logic() {
    // `1 < 2` folds to True; an open comparison stays symbolic.
    let out = eval("1 < 2\n").unwrap();
    assert!(out.contains("True"), "got {out:?}");
    assert_eq!(eval("a == a\n").unwrap(), "Out[1]= True\n");
}

/// Multi-statement feed numbers `Out` in order; `;` suppresses display.
#[test]
fn output_numbering_and_suppression() {
    let mut s = WolframSession::new();
    assert_eq!(s.feed("1\n2\n").unwrap(), "Out[1]= 1\nOut[2]= 2\n");
    // A suppressed statement consumes an Out index but prints nothing.
    assert_eq!(s.feed("3;\n").unwrap(), "");
    assert_eq!(s.feed("4\n").unwrap(), "Out[4]= 4\n");
}

/// Errors are returned cleanly (never a panic) and the session survives.
#[test]
fn errors_are_clean_and_recoverable() {
    let mut s = WolframSession::new();
    assert!(s.feed("1 +\n").is_err());
    assert!(s.feed("f[x\n").is_err()); // unclosed bracket
    assert_eq!(s.feed("2 + 2\n").unwrap(), "Out[1]= 4\n");
}

/// A short program exercising several features at once.
#[test]
fn a_small_program() {
    let mut s = WolframSession::new();
    s.feed("area[r_] := Pi*r^2;\n").unwrap();
    s.feed("nums = {1, 2, 3};\n").unwrap();
    let out = s.feed("Power[2, 5] + Times[3, 4]\n").unwrap();
    // 32 + 12 = 44
    assert_eq!(out, "Out[3]= 44\n");
}
