//! # Scilab Runtime — a tree-walking evaluator over `array-runtime`.
//!
//! This is item **MA-10d** of the Scilab frontend (spec
//! `code/specs/MA10-scilab-language.md`): the runtime that makes the Scilab
//! lexer/parser (MA-10b/MA-10c) executable. It parses with
//! [`coding_adventures_scilab_parser`] and walks the resulting tree with a
//! recursive [`Interpreter`], computing [`ScilabValue`]s over
//! [`array_runtime`].
//!
//! ```text
//! Scilab source
//!    │
//!    ▼  coding_adventures_scilab_parser::try_parse_scilab   (MA-10c)
//! GrammarASTNode
//!    │
//!    ▼  crate::eval::Interpreter::run                        (this crate)
//! ScilabValue  (Num(Array) | Str(String))
//! ```
//!
//! ## Why its own evaluator, not `matlab-runtime`
//!
//! MA10 §1/§2/§5 is explicit: Scilab forks MATLAB's *grammar shape* (matrix
//! literals, ranges, the precedence cascade, indexing) but not its
//! *semantics* — the decisive divergence being that `+` means numeric
//! addition on strings in MATLAB and concatenation in Scilab (§1 finding 1).
//! Reusing `matlab_runtime::MatValue` would silently reuse MATLAB's answer to
//! "what does an operator mean on this variant," exactly the trap this crate
//! is built to avoid — so [`ScilabValue`] is its own value enum (`value.rs`),
//! and this crate has no dependency on `matlab-runtime` at all. What *does*
//! transfer unchanged, per MA10 §5's "zero substrate work" conclusion: the
//! entire `array-runtime` numeric core (dense matrices, elementwise ops with
//! scalar broadcasting, matmul, transpose, ranges, reductions, comparisons).
//!
//! Four things this crate's evaluator does that `matlab-runtime`'s never
//! needed at all: Scilab's own `select`/`case` multi-way conditional (no
//! MATLAB `switch`/`otherwise` analogue), `break`/`continue`, user-defined
//! `function`/`endfunction` with multiple return values, and `$`-based
//! last-index resolution (Scilab's classic/preferred alternative to MATLAB's
//! context-sensitive `end`-as-index — MA10 §1 finding 5). See `eval.rs`'s own
//! module doc comment for the evaluator's internal design (including why its
//! methods take `&mut self` throughout, unlike `matlab-runtime`'s `&self`)
//! and each of these four additions' own doc comments for their specific
//! semantics.
//!
//! ## Robustness at the trust boundary
//!
//! [`Interpreter::feed`] takes arbitrary user text, so — following
//! `maple-runtime`/`reduce-runtime`/`derive-runtime`/`wolfram-runtime`/
//! `maxima-runtime`'s established, more rigorous pattern rather than
//! `matlab-runtime`'s older, simpler one (MA10 §6's own instruction: a
//! brand-new crate should start from this repo's accumulated lessons, not
//! replicate an earlier sibling's possibly-incomplete safety net) — this
//! crate closes the same two independent vectors those crates document:
//!
//! 1. **Deeply *nested* source** (parenthesised, matrix-literal, or
//!    function-call-argument nesting; a flat `^`/unary-prefix/chained-
//!    assignment chain; nested `if`/`select`/`while`/`for`/`function`) is
//!    already rejected by `scilab-parser`'s own `MAX_RULE_DEPTH` (125,
//!    independently measured against seven distinct recursive shapes — see
//!    that constant's own doc comment) — before this crate's evaluation
//!    recursion (which walks the *same*, already-shallow tree the parser
//!    handed back) ever runs.
//!
//! 2. **A long *flat* chain that folds into a deeply *nested* evaluated
//!    tree at runtime.** `scilab-parser`'s own `MAX_RULE_DEPTH` doc comment
//!    proves (by direct inspection of the shared `parser::grammar_parser`
//!    engine) that every EBNF `{ x }`-repetition production —
//!    `logical_or`/`logical_and`/`bit_or`/`bit_and`/`comparison`/`additive`/
//!    `multiplicative`, `{ elseif_clause }`, `{ case_clause }`, `arg_list`,
//!    `name_list`, `matrix_rows` — costs the *parser* zero native stack
//!    regardless of width, so `MAX_RULE_DEPTH` does not (and structurally
//!    cannot) bound a flat chain's length. **The question this raises for
//!    every reused-substrate CAS-family runtime in this repo is whether
//!    *this* crate's own evaluator re-introduces the vector by recursively
//!    folding such a chain into a nested tree.** Checked directly, not
//!    assumed: `matlab_runtime::eval::Interpreter::eval_binary_chain` — the
//!    identical-shaped chain production for MATLAB's own identically-flat
//!    grammar — is a plain iterative loop (a `for` over `node.children`
//!    accumulating a running total), **not** a self-recursive
//!    `eval_binary_chain(&operands[1..])`-style fold. This crate's own
//!    `eval::Interpreter::eval_binary_chain` mirrors that exact iterative
//!    shape (see its own doc comment for the full accounting, including
//!    every *other* flat-repetition production in this grammar and where
//!    each is walked by a plain loop rather than a recursive helper).
//!    **Conclusion: confirmed iterative, no gap** — this crate needs no
//!    `MAX_STATEMENT_TOKENS`-style guard the way `maple-runtime`/
//!    `reduce-runtime`/`derive-runtime` each need for *their* languages
//!    (where the analogous lowering step genuinely does fold a flat
//!    repetition into a nested tree). Verified by this crate's own
//!    `long_flat_arithmetic_chain_evaluates_without_a_dedicated_guard` test
//!    (several thousand `+`-joined terms, evaluated successfully in bounded
//!    time and default stack space).
//!
//! 3. **A genuinely new third vector, unique to this crate among the
//!    reused-substrate languages: recursive *function calls* at runtime.**
//!    None of `matlab-runtime`/`maple-runtime`/`reduce-runtime`/
//!    `derive-runtime` evaluate user-defined functions that can call
//!    themselves — `scilab-runtime` does (MA10 §4's multiple-return
//!    surface). A Scilab function with no base case (or one written to
//!    recurse further than intended) creates native-stack recursion that has
//!    nothing to do with either vector above — the *source* for a three-line
//!    recursive function is shallow; it is the runtime *call* depth that
//!    grows unboundedly. `eval::MAX_DEPTH` (2000, deliberately more generous
//!    than `matlab-runtime`'s 512, since it must also budget for legitimate
//!    recursive Scilab functions — see that constant's own doc comment for
//!    the full accounting) closes this vector, verified empirically by this
//!    crate's own `runaway_recursion_is_a_clean_error_not_a_crash` test.
//!
//! 4. **Unwinding panics** (an internal invariant violation, an unexpected
//!    AST shape, a wrong-arity builtin call, ...). Unlike `matlab-runtime`
//!    (which predates this pattern and has no panic-safety net of its own —
//!    a wrong shape there would abort the whole process), [`Interpreter::feed`]
//!    runs parsing *and* evaluation inside [`catch_unwind`] on a dedicated
//!    worker thread with a large bounded stack ([`EVAL_STACK_SIZE`], 512
//!    MiB, matching `maple-runtime`'s own choice), so any panic becomes a
//!    clean `Err(String)` rather than an abort. On a panic, the session is
//!    rebuilt (`Interpreter::new()`) — trading the lost variable/function
//!    bindings for a guaranteed-usable session on the next call, the same
//!    tradeoff `maple-runtime::MapleSession::eval_to_outputs` makes
//!    explicitly. Note that `catch_unwind` and `MAX_DEPTH` are *not*
//!    interchangeable: a real native-stack overflow (from runaway recursion
//!    that a depth cap failed to catch in time) is **not** a catchable panic
//!    at all — the process aborts directly, no unwind — so `MAX_DEPTH`
//!    (point 3 above) is the mechanism that actually prevents that class of
//!    crash; `catch_unwind` here is what protects against everything else
//!    (out-of-bounds access, arithmetic invariant violations, `unwrap`/
//!    `expect` failures, ...).

mod builtins;
mod eval;
mod value;

pub use eval::Interpreter;
pub use value::ScilabValue;

use coding_adventures_scilab_parser::try_parse_scilab;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Maximum length, in bytes, of a single source chunk handed to
/// [`Interpreter::feed`]. A cheap first gate bounding per-call memory/time —
/// mirrors `maple_runtime::MAX_INPUT_LEN`'s identical role. 64 KiB is far
/// beyond any realistic interactive submission (or hand-written script).
pub const MAX_INPUT_LEN: usize = 64 * 1024;

/// Stack size of the worker thread that runs parsing and evaluation.
///
/// With `scilab-parser`'s own `MAX_RULE_DEPTH` bounding parse-time recursion
/// and `eval::MAX_DEPTH` bounding evaluation-time recursion (including
/// function-call recursion), this is generous headroom so the bounded-but-
/// still-meaningfully-deep trees this crate supports are parsed, evaluated,
/// and dropped well clear of any overflow, regardless of the caller's own
/// stack size. Matches `maple_runtime::EVAL_STACK_SIZE`.
const EVAL_STACK_SIZE: usize = 512 * 1024 * 1024;

impl Interpreter {
    /// Parse and evaluate a chunk of Scilab source, returning the
    /// concatenated prompt echo of every unsuppressed result. Variables and
    /// function definitions persist across calls (see `eval.rs`'s own
    /// `Interpreter::run` doc comment for how top-level `function`
    /// definitions are registered).
    ///
    /// Runs on a dedicated worker thread inside [`catch_unwind`] — see the
    /// crate doc comment's "Robustness at the trust boundary" section for
    /// the full rationale. On a panic, the session is rebuilt from scratch
    /// (bindings are lost, but the session remains usable for the next
    /// call).
    ///
    /// # Example
    ///
    /// ```
    /// use coding_adventures_scilab_runtime::Interpreter;
    /// let mut s = Interpreter::new();
    /// assert!(s.feed("2 + 2\n").unwrap().contains("4"));
    /// ```
    pub fn feed(&mut self, source: &str) -> Result<String, String> {
        if source.len() > MAX_INPUT_LEN {
            return Err(format!(
                "scilab-runtime: input too large: {} bytes exceeds the {}-byte limit",
                source.len(),
                MAX_INPUT_LEN
            ));
        }

        let src_owned = source.to_string();
        let mut panicked = false;
        let interp: &mut Interpreter = self;
        let result: Result<String, String> = std::thread::scope(|scope| {
            let handle = match std::thread::Builder::new()
                .stack_size(EVAL_STACK_SIZE)
                .spawn_scoped(scope, move || {
                    catch_unwind(AssertUnwindSafe(|| eval_in_place(interp, &src_owned)))
                }) {
                Ok(handle) => handle,
                Err(io_err) => {
                    return Err(format!(
                        "scilab-runtime: failed to spawn the evaluation thread: {io_err}"
                    ));
                }
            };
            match handle.join() {
                Ok(Ok(result)) => result,
                Ok(Err(payload)) | Err(payload) => {
                    panicked = true;
                    Err(panic_message(payload))
                }
            }
        });

        if panicked {
            *self = Interpreter::new();
        }
        result
    }
}

/// Parse and run `source` against an existing session. Runs on the worker
/// thread, inside `catch_unwind` (see [`Interpreter::feed`]).
fn eval_in_place(interp: &mut Interpreter, source: &str) -> Result<String, String> {
    let tree = try_parse_scilab(source)?;
    interp.run(&tree)
}

/// Recover a human-readable message from a caught panic payload.
fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "scilab-runtime: could not evaluate that input".to_string()
    }
}

/// Evaluate Scilab source in a fresh session and return its display output.
pub fn eval(source: &str) -> Result<String, String> {
    Interpreter::new().feed(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Evaluate and return the trimmed display output.
    fn run(src: &str) -> String {
        eval(src).unwrap_or_else(|e| panic!("eval failed for {src:?}: {e}"))
    }

    /// Evaluate `expr` (suppressed) and read back the bound variable's data
    /// via a follow-up echo — here we just parse the echoed scalar.
    fn scalar(src: &str) -> f64 {
        let out = run(src);
        out.rsplit('=')
            .next()
            .unwrap()
            .trim()
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("not a scalar echo: {out:?}"))
    }

    fn scalar_in(m: &mut Interpreter, src: &str) -> f64 {
        let out = m
            .feed(src)
            .unwrap_or_else(|e| panic!("eval failed for {src:?}: {e}"));
        out.rsplit('=')
            .next()
            .unwrap()
            .trim()
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("not a scalar echo: {out:?}"))
    }

    // --- arithmetic and display ------------------------------------------

    #[test]
    fn scalar_arithmetic_echoes_ans() {
        assert_eq!(scalar("2 + 3 * 4\n"), 14.0);
        assert_eq!(scalar("2 ^ 10\n"), 1024.0);
        assert_eq!(scalar("-2 ^ 2\n"), -4.0); // unary looser than power
    }

    #[test]
    fn semicolon_suppresses_display() {
        assert_eq!(run("x = 5;\n"), "");
        assert!(run("x = 5\n").contains("x = 5"));
    }

    #[test]
    fn assignment_persists_in_a_session() {
        let mut m = Interpreter::new();
        m.feed("a = 10;\n").unwrap();
        m.feed("b = 20;\n").unwrap();
        assert!(m.feed("a + b\n").unwrap().contains("30"));
    }

    // --- matrices, ranges, `$`-indexing -----------------------------------

    #[test]
    fn matrix_literal_and_indexing() {
        let mut m = Interpreter::new();
        m.feed("A = [1 2; 3 4];\n").unwrap();
        assert_eq!(scalar_in(&mut m, "A(2, 1)\n"), 3.0);
        assert_eq!(scalar_in(&mut m, "A(3)\n"), 2.0);
    }

    #[test]
    fn dollar_last_index_resolves_correctly() {
        let mut m = Interpreter::new();
        m.feed("A = [10 20 30 40];\n").unwrap();
        assert_eq!(scalar_in(&mut m, "A($)\n"), 40.0);
        assert_eq!(scalar_in(&mut m, "A($-1)\n"), 30.0);
    }

    #[test]
    fn dollar_used_outside_an_index_is_an_error() {
        assert!(eval("$\n").is_err());
    }

    #[test]
    fn ranges_and_for_loop() {
        let mut m = Interpreter::new();
        m.feed("s = 0;\nfor i = 1:5\n  s = s + i;\nend\n").unwrap();
        assert_eq!(scalar_in(&mut m, "s\n"), 15.0);
    }

    #[test]
    fn while_loop() {
        let mut m = Interpreter::new();
        m.feed("n = 0;\nwhile n < 10\n  n = n + 1;\nend\n").unwrap();
        assert_eq!(scalar_in(&mut m, "n\n"), 10.0);
    }

    // --- if/elseif/else, select/case/else ---------------------------------

    #[test]
    fn if_elseif_else() {
        let mut m = Interpreter::new();
        m.feed("x = 5;\nif x > 10\n  y = 1;\nelseif x > 3\n  y = 2;\nelse\n  y = 3;\nend\n")
            .unwrap();
        assert_eq!(scalar_in(&mut m, "y\n"), 2.0);
    }

    #[test]
    fn if_then_and_bare_comma_forms_evaluate_identically() {
        // stmt_sep already collapsed the linker-keyword-vs-punctuation
        // distinction at PARSE time (MA10 §3) -- the TREE is identical
        // either way, so one confirming test suffices for the runtime.
        let mut a = Interpreter::new();
        a.feed("x = 5;\n").unwrap();
        let out_then = a.feed("if x>0 then y=1, end\n").unwrap();
        let mut b = Interpreter::new();
        b.feed("x = 5;\n").unwrap();
        let out_comma = b.feed("if x>0, y=1, end\n").unwrap();
        assert_eq!(out_then, out_comma);
        assert_eq!(scalar_in(&mut a, "y\n"), scalar_in(&mut b, "y\n"));
    }

    #[test]
    fn select_case_matches_first_equal_case() {
        let mut m = Interpreter::new();
        m.feed(
            "x = 2;\nselect x\n case 1 then y = 10;\n case 2 then y = 20;\n else y = 0;\n end\n",
        )
        .unwrap();
        assert_eq!(scalar_in(&mut m, "y\n"), 20.0);
    }

    #[test]
    fn select_case_falls_to_else_when_nothing_matches() {
        let mut m = Interpreter::new();
        m.feed("x = 99;\nselect x\n case 1 then y = 10;\n else y = 0;\n end\n")
            .unwrap();
        assert_eq!(scalar_in(&mut m, "y\n"), 0.0);
    }

    #[test]
    fn select_case_with_no_match_and_no_else_does_nothing() {
        let mut m = Interpreter::new();
        m.feed("y = 7;\n").unwrap();
        m.feed("x = 99;\nselect x\n case 1 then y = 10;\n end\n")
            .unwrap();
        assert_eq!(scalar_in(&mut m, "y\n"), 7.0); // untouched
    }

    // --- break/continue -----------------------------------------------------

    #[test]
    fn break_stops_a_while_loop_early() {
        let mut m = Interpreter::new();
        m.feed("n = 0;\nwhile %t\n  n = n + 1;\n  if n == 3 then break, end\nend\n")
            .unwrap();
        assert_eq!(scalar_in(&mut m, "n\n"), 3.0);
    }

    #[test]
    fn continue_skips_to_the_next_iteration() {
        let mut m = Interpreter::new();
        m.feed(
            "s = 0;\nfor i = 1:5\n  if i == 3 then continue, end\n  s = s + i;\nend\n",
        )
        .unwrap();
        // 1+2+4+5 = 12 (3 is skipped)
        assert_eq!(scalar_in(&mut m, "s\n"), 12.0);
    }

    #[test]
    fn break_outside_a_loop_is_an_error() {
        assert!(eval("break\n").is_err());
    }

    // --- functions, single and multiple return values -----------------------

    #[test]
    fn function_with_single_return_value() {
        let mut m = Interpreter::new();
        m.feed("function y = square(x)\n y = x * x;\nendfunction\n")
            .unwrap();
        assert_eq!(scalar_in(&mut m, "square(5)\n"), 25.0);
    }

    #[test]
    fn function_with_multiple_return_values() {
        let mut m = Interpreter::new();
        m.feed("function [s, p] = sumprod(a, b)\n s = a + b;\n p = a * b;\nendfunction\n")
            .unwrap();
        m.feed("[s, p] = sumprod(3, 4);\n").unwrap();
        assert_eq!(scalar_in(&mut m, "s\n"), 7.0);
        assert_eq!(scalar_in(&mut m, "p\n"), 12.0);
    }

    #[test]
    fn function_can_be_called_before_its_own_definition_in_the_same_feed() {
        let mut m = Interpreter::new();
        m.feed("y = double_it(4);\nfunction r = double_it(x)\n r = x * 2;\nendfunction\n")
            .unwrap();
        assert_eq!(scalar_in(&mut m, "y\n"), 8.0);
    }

    #[test]
    fn nested_function_definition_is_a_clean_error_not_a_silent_no_op() {
        // MA10 §4 never lists nested function definitions in scope. A
        // `func_def` textually nested inside another block is NOT silently
        // ignored (it is not registered, since only `run`'s top-level pass 1
        // does that) -- it is an honest, clean `Err`.
        assert!(eval(
            "if %t then\n function y = f(x)\n  y = x;\n endfunction\nend\n"
        )
        .is_err());
    }

    #[test]
    fn function_gets_a_fresh_workspace_no_shared_variables() {
        let mut m = Interpreter::new();
        m.feed("x = 100;\n").unwrap();
        m.feed("function y = f(x)\n y = x + 1;\nendfunction\n")
            .unwrap();
        assert_eq!(scalar_in(&mut m, "f(1)\n"), 2.0); // not 101
        assert_eq!(scalar_in(&mut m, "x\n"), 100.0); // caller's x untouched
    }

    #[test]
    fn recursive_function_evaluates_correctly() {
        let mut m = Interpreter::new();
        m.feed(
            "function y = fact(n)\n if n <= 1 then y = 1;\n else y = n * fact(n - 1);\n end\nendfunction\n",
        )
        .unwrap();
        assert_eq!(scalar_in(&mut m, "fact(6)\n"), 720.0);
    }

    // --- special constants ---------------------------------------------------

    #[test]
    fn all_eight_percent_constants_evaluate() {
        assert!((scalar("%pi\n") - std::f64::consts::PI).abs() < 1e-12);
        assert!((scalar("%e\n") - std::f64::consts::E).abs() < 1e-12);
        assert!(scalar("%inf\n").is_infinite());
        assert!(scalar("%nan\n").is_nan());
        assert_eq!(scalar("%eps\n"), f64::EPSILON);
        assert_eq!(scalar("%t\n"), 1.0);
        assert_eq!(scalar("%f\n"), 0.0);
        assert!(eval("%i\n").is_err()); // complex numbers deferred, MA10 §4
    }

    // --- strings: assignment/display/equality only --------------------------

    #[test]
    fn string_assignment_and_display() {
        assert!(run("s = 'hello'\n").contains("hello"));
        assert!(run("s = \"world\"\n").contains("world"));
    }

    #[test]
    fn string_equality_and_inequality() {
        assert_eq!(scalar("'abc' == 'abc'\n"), 1.0);
        assert_eq!(scalar("'abc' == 'xyz'\n"), 0.0);
        assert_eq!(scalar("'abc' ~= 'xyz'\n"), 1.0);
        assert_eq!(scalar("'abc' <> 'xyz'\n"), 1.0);
    }

    #[test]
    fn string_plus_string_is_a_clean_error_not_concatenation_or_addition() {
        // The one thing this cut must NOT do (MA10 §4's explicit scope cut):
        // `+` over strings must not silently concatenate (Scilab's real
        // answer) OR silently compute ASCII-code numeric addition (MATLAB's
        // real, different answer) -- it must be an honest Err.
        assert!(eval("'a' + 'b'\n").is_err());
        assert!(eval("\"a\" + \"b\"\n").is_err());
    }

    #[test]
    fn string_minus_or_ordering_comparison_is_also_an_error() {
        assert!(eval("'a' - 'b'\n").is_err());
        assert!(eval("'a' < 'b'\n").is_err());
    }

    // --- flat-chain safety (point 5) -----------------------------------------

    #[test]
    fn long_flat_arithmetic_chain_evaluates_without_a_dedicated_guard() {
        // Confirmed-iterative eval_binary_chain (see the crate/eval.rs doc
        // comments): a flat chain of thousands of `+`-joined terms must
        // evaluate successfully (not need, and not trip, any special guard),
        // proving the vector is closed by construction.
        let src = format!("{}1\n", "1+".repeat(20_000));
        assert_eq!(scalar(&src), 20_001.0);
    }

    #[test]
    fn moderate_flat_chain_evaluates_correctly() {
        let src = format!("{}1\n", "1+".repeat(50));
        assert_eq!(scalar(&src), 51.0);
    }

    // --- runaway function-call recursion (point 3 in the crate doc comment) -

    #[test]
    fn runaway_recursion_is_a_clean_error_not_a_crash() {
        // A function with NO base case at all -- unbounded call-depth
        // recursion, the vector unique to this crate among the reused-
        // substrate CAS-family runtimes (see the crate doc comment). Run on
        // a worker thread with the DEFAULT (unboosted, ~a few MiB) stack
        // size, so this proves `eval::MAX_DEPTH` trips before even a modest
        // stack would overflow -- not merely that the generous 512 MiB
        // `EVAL_STACK_SIZE` production stack absorbs it.
        let handle = std::thread::spawn(|| {
            let mut m = Interpreter::new();
            m.feed("function y = loop(n)\n y = loop(n + 1);\nendfunction\n")
                .unwrap();
            m.feed("loop(0)\n")
        });
        let result = handle.join().expect("must not crash the thread");
        assert!(result.is_err(), "runaway recursion must be a clean Err");
    }

    // --- panic-safety (point 4 in the crate doc comment) ---------------------

    #[test]
    fn a_parse_error_is_returned_not_panicked() {
        assert!(eval("r = 1 +\n").is_err());
    }

    #[test]
    fn session_recovers_after_a_parse_error() {
        let mut m = Interpreter::new();
        let _ = m.feed("r = 1 +\n");
        assert!(m.feed("3 + 4\n").unwrap().contains('7'));
    }

    #[test]
    fn oversized_input_is_rejected_before_evaluation() {
        let huge = format!("{}1\n", "1+".repeat(MAX_INPUT_LEN));
        assert!(huge.len() > MAX_INPUT_LEN);
        let err = eval(&huge).unwrap_err();
        assert!(err.contains("too large"), "got {err:?}");
    }

    #[test]
    fn dimension_mismatch_and_oob_are_errors() {
        assert!(eval("[1 2] * [3 4]\n").is_err());
        let mut m = Interpreter::new();
        m.feed("A = [1 2 3];\n").unwrap();
        assert!(m.feed("A(9)\n").is_err());
        assert!(eval("zeros(1e18)\n").is_err());
        assert!(eval("undefined_thing\n").is_err());
    }

    #[test]
    fn constructor_rejects_an_astronomical_element_product() {
        // Security regression: each dimension alone (67,108,864) is within
        // count()'s own per-dimension cap, but their PRODUCT (~4.5e15
        // elements, ~36 petabytes) is not -- this must be a clean error, not
        // an attempted allocation that aborts the process.
        assert!(eval("zeros(67108864, 67108864)\n").is_err());
        assert!(eval("eye(67108864)\n").is_err());
        // A genuinely small, in-bounds construction still works.
        assert!(eval("zeros(3, 4)\n").is_ok());
    }

    #[test]
    fn matrix_self_concatenation_cannot_double_past_the_element_cap() {
        // Security regression: `[A A]`/`[A; A]` repeated doubles the element
        // count each time with no individually-large input -- only the
        // ACCUMULATED result grows exponentially. 24 repetitions alone
        // already reaches 2^24 elements; this must error out well before an
        // attacker-reachable number of repetitions produces an
        // allocation-aborting size.
        let mut m = Interpreter::new();
        m.feed("A = 1;\n").unwrap();
        let mut last_ok = true;
        for _ in 0..40 {
            last_ok = m.feed("A = [A A];\n").is_ok();
            if !last_ok {
                break;
            }
        }
        assert!(!last_ok, "40 doublings (up to 2^40 elements) must be rejected before completing");
    }

    #[test]
    fn deeply_nested_input_errors_instead_of_crashing() {
        let src = format!("{}1{}\n", "(".repeat(2000), ")".repeat(2000));
        assert!(eval(&src).is_err());
    }

    #[test]
    fn the_one_shot_eval_helper_matches_a_session() {
        let one = eval("2 + 3\n").unwrap();
        let two = Interpreter::new().feed("2 + 3\n").unwrap();
        assert_eq!(one, two);
    }
}
