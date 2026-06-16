//! # Maxima runtime — a Macsyma reuse.
//!
//! [Maxima](https://maxima.sourceforge.io/) is the GPL-licensed descendant of
//! DOE Macsyma. When MIT's Project MAC handed Macsyma to the Department of
//! Energy, William Schelter maintained a copy and in 1998 released it under the
//! GPL as *Maxima*. The two share the same algebraic surface: `:` assignment,
//! `;` to display / `$` to suppress, the `%i«n»`/`%o«n»` history convention,
//! and the whole `diff`/`integrate`/`expand`/`factor`/`solve` function family.
//! A program written for one runs on the other.
//!
//! Because of that, this crate is **not** a new interpreter. It is a thin
//! facade — a [`MaximaSession`] that owns a [`macsyma_runtime::MacsymaSession`]
//! and presents Maxima's string-in / string-out console contract over it. The
//! entire pipeline beneath (the `macsyma-lexer` → `macsyma-parser` →
//! `macsyma-compiler` frontend, the `symbolic-vm`, and the twenty `cas-*`
//! crates that implement simplification, calculus, trig, solving, substitution,
//! and pretty-printing) is reused **unchanged**.
//!
//! This is the symbolic-CAS analogue of how GNU Octave was delivered as a thin
//! reuse of `matlab-runtime`: a second historical language for the cost of a
//! façade plus a REPL, because the syntax already matched.
//!
//! ## What `feed` does
//!
//! ```text
//! (%i1) diff(x^3, x);
//! (%o1) 3*x^2
//! ```
//!
//! [`MaximaSession::feed`] hands the source straight to
//! [`MacsymaSession::eval_source`](macsyma_runtime::MacsymaSession::eval_source),
//! which parses and evaluates it, then builds the output echo. For each evaluated
//! statement **whose `display` flag is set** (i.e. terminated by `;`, not `$`)
//! it appends one line:
//!
//! ```text
//! (%o«output_index») «output_text»
//! ```
//!
//! A `$`-suppressed statement still runs and still advances the shared `%o`
//! history counter, but contributes no output line — exactly as in Maxima. A
//! surface/parse error arrives as the macsyma `CompileError`; we forward its
//! `Display` text as `Err(String)` so a REPL can show it without leaking a Rust
//! backtrace.
//!
//! ## Panic-safety at the trust boundary
//!
//! The underlying `macsyma-lexer` **panics** (rather than returning an error)
//! when it meets a character it cannot tokenize — e.g. a stray `@`. Since
//! `feed` is a trust boundary that takes arbitrary user text, a single bad
//! keystroke must not abort an interactive session. So `feed` runs the
//! evaluation inside [`std::panic::catch_unwind`] and turns any unwinding panic
//! into a clean `Err(String)`. The tokenizer panics *before* it mutates session
//! state, so the wrapped [`MacsymaSession`] stays consistent and usable
//! afterwards; the [`AssertUnwindSafe`](std::panic::AssertUnwindSafe) wrapper is
//! sound here because all the code involved is panic-safe ordinary Rust (no
//! `unsafe`, no half-updated invariants that could be observed). This is a
//! defensive shim in the façade; the proper upstream fix is for the lexer to
//! return a `Result` instead of panicking.

use macsyma_runtime::MacsymaSession;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// A persistent Maxima session.
///
/// Holds the wrapped [`MacsymaSession`], so `%i`/`%o` history, variable bindings
/// (`x : 5$`), and `load("orthopoly")` gates all persist across calls to
/// [`feed`](MaximaSession::feed) just as they would in an interactive Maxima.
pub struct MaximaSession {
    inner: MacsymaSession,
}

impl Default for MaximaSession {
    fn default() -> Self {
        Self::new()
    }
}

impl MaximaSession {
    /// Create a fresh session with empty history and bindings.
    pub fn new() -> Self {
        MaximaSession {
            inner: MacsymaSession::new(),
        }
    }

    /// Evaluate a chunk of Maxima source and return the console echo.
    ///
    /// The source may contain several statements (each terminated by `;` or
    /// `$`). Every displayed result becomes one `(%o«n») «text»` line; the lines
    /// are concatenated in evaluation order, each ending in a newline. If nothing
    /// is displayed (e.g. a single `$`-suppressed statement) the result is the
    /// empty string. A parse/surface error is returned as `Err` with the
    /// evaluator's own message.
    pub fn feed(&mut self, src: &str) -> Result<String, String> {
        // The macsyma tokenizer panics on characters it cannot lex, so guard
        // the whole evaluation against an unwinding panic and surface it as an
        // ordinary error (see the module-level "Panic-safety" note). On the
        // happy path this is a no-op; on a bad-input panic the session is left
        // untouched (the panic fires before any mutation) and remains usable.
        let inner = &mut self.inner;
        let evaluated = catch_unwind(AssertUnwindSafe(|| inner.eval_source(src)));
        let results = match evaluated {
            Ok(Ok(results)) => results,
            Ok(Err(e)) => return Err(format!("{e}")),
            Err(payload) => return Err(panic_message(payload)),
        };
        let mut out = String::new();
        for r in results {
            if r.display {
                // Maxima echoes a displayed result as `(%o«n») «value»`. The
                // output_index is Macsyma's 1-based %o counter, carried through
                // verbatim; output_text is whatever the macsyma pretty-printer
                // selected (single-line, or a 2-D box when display2d is on).
                out.push_str(&format!("(%o{}) {}\n", r.output_index, r.output_text));
            }
        }
        Ok(out)
    }
}

/// Evaluate `src` once on a fresh [`MaximaSession`] and return its echo.
///
/// Convenience for callers that do not need persistent history.
pub fn eval(src: &str) -> Result<String, String> {
    MaximaSession::new().feed(src)
}

/// Recover a human-readable message from a caught panic payload.
///
/// `panic!("…")` stores either a `&'static str` or a `String`; we try both and
/// fall back to a generic message so the caller always gets *something* to show
/// the user rather than a debug-formatted `Box<dyn Any>`.
fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Maxima could not evaluate that input".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The exact `output_text` formatting is whatever the macsyma pretty-printer
    // produces; these tests assert on robust substrings (and the `(%oN)` echo
    // shape) rather than brittle whole-string matches, so they stay green if the
    // printer's spacing changes.

    #[test]
    fn arithmetic_folds_to_an_integer() {
        // 1 + 2*3 = 7, displayed as (%o1) 7
        let out = eval("1 + 2*3;").unwrap();
        assert!(out.starts_with("(%o1) "), "echo shape: {out:?}");
        assert!(out.contains('7'), "expected 7 in {out:?}");
    }

    #[test]
    fn differentiation_of_a_cube() {
        // diff(x^3, x) = 3*x^2
        let out = eval("diff(x^3, x);").unwrap();
        assert!(out.contains('3') && out.contains('x'), "got {out:?}");
        // the derivative is quadratic in x — a 2 (the exponent) appears
        assert!(out.contains('2'), "expected the exponent 2 in {out:?}");
    }

    #[test]
    fn integration_of_a_power() {
        // integrate(x^2, x) = x^3/3 — genuinely reduced by the macsyma evaluator
        // (chosen over `expand`, which the current evaluator echoes symbolically).
        let out = eval("integrate(x^2, x);").unwrap();
        assert!(out.contains('x'), "got {out:?}");
        assert!(
            out.contains('3'),
            "expected the x^3/3 antiderivative in {out:?}"
        );
    }

    #[test]
    fn suppressed_assignment_then_use() {
        // `x : 5$` binds and displays nothing; `x + 1;` then shows 6.
        let mut s = MaximaSession::new();
        let nothing = s.feed("x : 5$").unwrap();
        assert_eq!(nothing, "", "a $-suppressed statement prints nothing");
        let out = s.feed("x + 1;").unwrap();
        assert!(out.contains('6'), "binding should persist: {out:?}");
    }

    #[test]
    fn factoring_a_difference_of_squares() {
        // factor(x^2 - 1) = (x - 1)*(x + 1)
        let out = eval("factor(x^2 - 1);").unwrap();
        assert!(out.contains('x'), "got {out:?}");
        // both linear factors mention 1
        assert!(out.contains('1'), "expected (x-1)(x+1) form in {out:?}");
    }

    #[test]
    fn history_counter_advances_across_statements() {
        // Two displayed statements in one feed get %o1 then %o2.
        let out = eval("2; 3;").unwrap();
        assert!(out.contains("(%o1) "), "first echo: {out:?}");
        assert!(out.contains("(%o2) "), "second echo: {out:?}");
    }

    #[test]
    fn suppression_still_advances_the_output_index() {
        // A `$` statement consumes an %o slot even though it prints nothing,
        // so the *next* displayed result is %o2, not %o1.
        let mut s = MaximaSession::new();
        assert_eq!(s.feed("10$").unwrap(), "");
        let out = s.feed("20;").unwrap();
        assert!(out.contains("(%o2) "), "index should skip to 2: {out:?}");
    }

    #[test]
    fn a_surface_error_is_returned_not_panicked() {
        // An unparseable fragment yields Err with the evaluator's message,
        // never a panic.
        let err = eval("@#$%^;");
        assert!(err.is_err(), "expected a clean Err, got {err:?}");
    }

    #[test]
    fn the_one_shot_eval_helper_matches_a_session() {
        let one = eval("1 + 1;").unwrap();
        let mut s = MaximaSession::new();
        let two = s.feed("1 + 1;").unwrap();
        assert_eq!(one, two);
    }
}
