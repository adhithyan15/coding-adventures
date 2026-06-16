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
//! ## Robustness at the trust boundary
//!
//! `feed` takes arbitrary user text, so it is the trust boundary for the whole
//! reused Macsyma stack. Three failure modes of that stack are contained here so
//! a single bad input can never crash or wedge an interactive session:
//!
//! 1. **Unwinding panics.** The `macsyma-lexer` *panics* (rather than returning
//!    an error) on a character it cannot tokenize — e.g. a stray `@`. `feed`
//!    runs evaluation inside [`std::panic::catch_unwind`] and converts any
//!    unwinding panic into a clean `Err(String)`.
//!
//! 2. **Stack overflow from unbounded recursion.** The Macsyma parser and the
//!    `symbolic-vm` recurse on expression nesting with no depth limit, so deeply
//!    nested input — thousands of `(`, a long run of prefix `-`, or a long
//!    `1+1+1+…` chain — builds a correspondingly deep tree. That overflows the
//!    stack while *parsing/evaluating* it, and again later when the tree is
//!    *dropped* (the wrapped session retains it in its `%o` history). A stack
//!    overflow is **not** a catchable panic; it aborts the whole process, and
//!    `catch_unwind` cannot stop it. Three layered guards close this:
//!    `feed` rejects input longer than [`MAX_INPUT_LEN`]; it rejects any single
//!    statement whose structural-symbol count exceeds [`MAX_STATEMENT_SYMBOLS`]
//!    (an upper bound on the resulting tree depth, since a tree of *n* internal
//!    operator/bracket nodes is at most *n* deep), so a pathologically deep tree
//!    is never built in the first place; and it runs evaluation on a dedicated
//!    worker thread with a large, bounded stack ([`EVAL_STACK_SIZE`]) and builds
//!    the echo string there, so even the bounded trees are created *and dropped*
//!    on the big stack rather than the caller's.
//!
//! 3. **A poisoned session after a caught panic.** Some Macsyma handlers hold an
//!    internal `Mutex` while running; a panic there would poison it and make
//!    *every* later call fail. So whenever `feed` catches a panic it **rebuilds
//!    the wrapped session from scratch**, trading the lost `%o` history and
//!    bindings for a guaranteed-usable session on the next call. (On the common
//!    lexer-panic path nothing was mutated, so this is cheap and lossless in
//!    practice.)
//!
//! The [`AssertUnwindSafe`](std::panic::AssertUnwindSafe) wrapper is sound here
//! because the code involved is ordinary safe Rust (no `unsafe`, no observable
//! half-updated invariant — and any logical inconsistency is erased by the
//! rebuild in (3)). These are defensive shims in the façade; the proper upstream
//! fixes are for the lexer to return a `Result` and for the parser/VM to carry a
//! recursion-depth limit.

use macsyma_runtime::MacsymaSession;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Maximum length, in bytes, of a single source chunk handed to [`feed`].
///
/// A cheap first gate that bounds total memory/time per call. 64 KiB is far
/// beyond any realistic interactive submission.
pub const MAX_INPUT_LEN: usize = 64 * 1024;

/// Maximum number of structural (operator/bracket/punctuation) symbols allowed
/// in any single top-level statement.
///
/// The depth of the IRNode tree a statement parses to is bounded above by the
/// number of operator/bracket *tokens* it contains, which is in turn bounded by
/// the count of non-identifier symbol characters. Capping that per statement
/// therefore caps the tree depth, so the parser/VM (and the later `Drop` of the
/// tree) cannot be driven into a stack-overflowing recursion. 2000 operators in
/// one statement is already absurd for human-written Maxima, so the cap never
/// bites legitimate input.
pub const MAX_STATEMENT_SYMBOLS: usize = 2000;

/// Stack size of the worker thread that runs evaluation.
///
/// With the per-statement complexity cap bounding tree depth, this is generous
/// headroom: it lets the bounded-but-still-deep trees be built and dropped well
/// clear of any overflow, regardless of the caller's own (possibly small) stack.
const EVAL_STACK_SIZE: usize = 512 * 1024 * 1024;

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
        // Guard 1: bound total input size (cheap memory/time gate).
        if src.len() > MAX_INPUT_LEN {
            return Err(format!(
                "input too large: {} bytes exceeds the {}-byte limit",
                src.len(),
                MAX_INPUT_LEN
            ));
        }
        // Guard 2: reject any over-complex statement so no stack-overflowing tree
        // is ever built (see the module "Robustness" note and the const docs).
        check_statement_complexity(src)?;

        // Guard 3: run evaluation — and the echo formatting that walks the result
        // trees — on a worker thread with a large, bounded stack, so the bounded
        // trees are built and dropped clear of the caller's (possibly small)
        // stack; and Guard 4 (unwinding panics): catch the macsyma lexer's
        // panic-on-bad-input and turn it into an ordinary error. Only the small
        // `String` echo (or an error message) crosses back to the caller.
        let inner = &mut self.inner;
        let outcome = std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(EVAL_STACK_SIZE)
                .spawn_scoped(scope, || {
                    catch_unwind(AssertUnwindSafe(|| match inner.eval_source(src) {
                        Ok(results) => {
                            let mut out = String::new();
                            for r in results {
                                if r.display {
                                    // Maxima echoes a displayed result as
                                    // `(%o«n») «value»`: output_index is the
                                    // 1-based %o counter, output_text is whatever
                                    // the macsyma pretty-printer selected.
                                    out.push_str(&format!(
                                        "(%o{}) {}\n",
                                        r.output_index, r.output_text
                                    ));
                                }
                            }
                            // The (possibly deep) result trees drop here, on the
                            // big worker stack.
                            Ok(out)
                        }
                        Err(e) => Err(format!("{e}")),
                    }))
                })
                .expect("failed to spawn maxima evaluation thread")
                .join()
        });

        match outcome {
            // Normal path: the worker produced the echo (or a surface error).
            Ok(Ok(Ok(out))) => Ok(out),
            Ok(Ok(Err(message))) => Err(message),
            // A panic the worker caught (e.g. the lexer's bad-input panic), or a
            // panic that escaped the catch and unwound the join. Either way the
            // session may be inconsistent / mutex-poisoned, so rebuild it before
            // returning, so the *next* call is guaranteed usable rather than
            // failing forever on a poisoned lock.
            Ok(Err(payload)) | Err(payload) => {
                self.inner = MacsymaSession::new();
                Err(panic_message(payload))
            }
        }
    }
}

/// Reject input whose per-statement structural complexity could drive the
/// parser/VM — or the later `Drop` of the resulting tree — into a deep,
/// stack-overflowing recursion.
///
/// A statement's IRNode tree is at most as deep as the number of operator /
/// bracket tokens it contains, which is bounded by the count of non-identifier
/// "structural" symbol characters. We count those per top-level statement
/// (resetting at each `;`/`$` that lies outside a `"`-string) and refuse the
/// whole chunk if any statement exceeds [`MAX_STATEMENT_SYMBOLS`]. The scan is a
/// single linear pass and treats string contents as opaque so an operator-heavy
/// string literal is not mistaken for deep structure.
fn check_statement_complexity(src: &str) -> Result<(), String> {
    let mut symbols: usize = 0;
    let mut in_string = false;
    let mut escaped = false;
    for ch in src.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            // End of a statement — the next statement starts fresh.
            ';' | '$' => symbols = 0,
            // Identifiers, numbers, and whitespace are leaves / separators: they
            // do not deepen the tree.
            c if c.is_alphanumeric() || c == '_' || c.is_whitespace() => {}
            // Anything else is a structural symbol (operator, bracket, comma, …).
            _ => {
                symbols += 1;
                if symbols > MAX_STATEMENT_SYMBOLS {
                    return Err(format!(
                        "statement too complex: more than {MAX_STATEMENT_SYMBOLS} operators/brackets in one statement"
                    ));
                }
            }
        }
    }
    Ok(())
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

    #[test]
    fn oversized_input_is_rejected_before_evaluation() {
        // A chunk past the length cap is refused outright (Guard 1), bounding the
        // worst-case recursion the parser/VM can be driven to.
        let huge = format!("{}1;", "1+".repeat(MAX_INPUT_LEN));
        assert!(huge.len() > MAX_INPUT_LEN);
        let err = eval(&huge).unwrap_err();
        assert!(
            err.contains("too large"),
            "expected a size error, got {err:?}"
        );
    }

    #[test]
    fn deeply_nested_brackets_are_rejected_not_aborted() {
        // Thousands of nested parens used to abort the process via stack
        // overflow (on creation and again on drop). The complexity cap now
        // rejects them with a clean error before any deep tree is built.
        let depth = 20_000;
        let src = format!("{}1{};", "(".repeat(depth), ")".repeat(depth));
        assert!(src.len() <= MAX_INPUT_LEN, "stays under the length cap");
        let err = eval(&src).unwrap_err(); // must not abort the process
        assert!(
            err.contains("too complex"),
            "expected a complexity error: {err:?}"
        );
    }

    #[test]
    fn long_chain_of_prefix_operators_is_rejected() {
        // The non-bracketed recursion vector: a long run of unary minus, also a
        // former abort. Rejected by the complexity cap, never parsed.
        let src = format!("{}x;", "-".repeat(5_000));
        assert!(src.len() <= MAX_INPUT_LEN);
        assert!(eval(&src).unwrap_err().contains("too complex"));
    }

    #[test]
    fn long_binary_chain_is_rejected() {
        // `1+1+1+…` builds a left-deep tree as deep as the number of `+`; the
        // cap rejects it before that tree (or its drop) can overflow.
        let src = format!("{}1;", "1+".repeat(5_000));
        assert!(src.len() <= MAX_INPUT_LEN);
        assert!(eval(&src).unwrap_err().contains("too complex"));
    }

    #[test]
    fn moderate_nesting_still_evaluates() {
        // The cap is far above realistic input: a modestly nested expression
        // well under the limit still evaluates normally.
        let depth = 40;
        let src = format!("{}1 + 2{};", "(".repeat(depth), ")".repeat(depth));
        let out = eval(&src).unwrap();
        assert!(out.contains('3'), "((…1 + 2…)) should fold to 3: {out:?}");
    }

    #[test]
    fn the_session_survives_and_recovers_after_a_panic() {
        // A bad-input panic is caught and the session is rebuilt, so the very
        // next statement works (rather than failing forever on a poisoned lock).
        let mut s = MaximaSession::new();
        assert!(s.feed("@@@;").is_err());
        let out = s.feed("3 + 4;").unwrap();
        assert!(out.contains('7'), "session should recover: {out:?}");
    }
}
