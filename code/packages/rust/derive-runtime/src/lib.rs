//! # Derive runtime — lower Derive syntax to `symbolic-ir`, evaluate via `symbolic-vm`.
//!
//! This is the **D-4** deliverable of the Derive-language lane (MA07 §2). D-1
//! through D-3 gave us a spec, a lexer, and a parser; this crate is the
//! *runtime* that finally **evaluates** Derive source — by **reusing the
//! shared symbolic substrate** rather than writing a bespoke interpreter:
//!
//! ```text
//!   Derive source
//!        │
//!        ▼  coding_adventures_derive_parser::parse_derive  (D-3)
//!   GrammarASTNode  (surface tree: assignment, additive, power, postfix, …)
//!        │
//!        ▼  crate::lower                                   (this crate)
//!   symbolic_ir::IRNode   (Add, Mul, Pow, Assign, Define, D, Integrate, If, …)
//!        │
//!        ▼  symbolic_vm::VM over SymbolicBackend, UNCHANGED
//!   symbolic_ir::IRNode   (evaluated)
//!        │
//!        ▼  crate::printer
//!   Derive surface string  (infix, F(…), AND/OR/NOT)
//! ```
//!
//! ## No custom `Backend` needed
//!
//! Unlike `wolfram-runtime`/`macsyma-runtime` (each of which layers list,
//! string, or CAS-specific built-ins onto a bespoke `Backend` impl), D-4's
//! scope (MA07 §3/§4) needs *nothing* beyond what
//! [`symbolic_vm::SymbolicBackend`] already provides out of the box:
//! arithmetic, comparison, logic, `Assign`/`Define`/`If` (held, per
//! [`symbolic_vm::backend::BaseBackend`]), and — because `SymbolicBackend`
//! builds its handler table with `simplify: true` — the base `D`/`Integrate`
//! calculus handlers. So this crate's entire Derive-specific contribution is
//! the **lowering** (surface syntax → canonical heads, `crate::lower`) and
//! the **printer** (canonical heads → surface syntax, `crate::printer`); the
//! evaluation engine itself is reused completely unchanged, exactly the
//! "one function, three languages agreeing on its result" reuse MA07 §5
//! promises. `LIM`/`SOLVE`/`SUM`/`PRODUCT`/`TAYLOR` are deliberately not
//! wired here — MA07 §4 defers them (the shared VM has no existing handler
//! for any of them, so wiring them would be new engine code, not reuse).
//!
//! ## Public contract
//!
//! [`DeriveSession::feed`] is string-in / string-out, like the
//! Wolfram/Maxima/Octave facades. It evaluates a chunk of source (one or
//! more statements) and returns the surface rendering of each result, one
//! per line as `#n: «value»` — mirroring Derive's own numbered-worksheet
//! convention (MA07 §5). Unlike Wolfram, this subset has **no**
//! statement-suppression syntax (Derive's only `;` use is the matrix
//! row-separator, at non-zero bracket depth — MA07 §3/§4), so every
//! statement always displays. Bindings persist across calls, so an
//! interactive `x := 5` then `x + 1` works.
//!
//! ## Robustness at the trust boundary
//!
//! `feed` takes arbitrary user text, so — exactly as in
//! `wolfram-runtime`/`maxima-runtime` — it is the trust boundary for the
//! whole reused stack. Two layered guards close the two independent
//! deep-recursion vectors (per the "depth caps don't compose across
//! boundaries" lesson: a cap on one recursive walk does not protect a
//! *different* recursive walk over the same or a derived tree):
//!
//! 1. **Deeply *nested* source** (`((((…))))`, `F(F(F(…)))`). This vector is
//!    already closed by `derive-parser`'s own `MAX_RULE_DEPTH` — parsing
//!    itself rejects it with a clean `Err` before a deep tree is ever built,
//!    so this crate's lowering/printing recursion (which walks the *same*
//!    already-shallow tree the parser handed back) can never overflow on
//!    this vector. (Unlike Wolfram's parser, which has no depth cap of its
//!    own — hence `wolfram-runtime` needing a token-count gate to close
//!    *this same* vector itself.)
//! 2. **A long *flat* chain that folds into a deeply *nested* lowered tree**
//!    (`1+1+1+…` with thousands of terms). `additive`/`multiplicative` are
//!    grammar *repetitions*, not recursive rule calls, so `MAX_RULE_DEPTH`
//!    does not bound chain length at all — but [`lower_binary_chain`]
//!    left-folds an N-term chain into N-1 *nested* `Apply` applications, and
//!    the VM's own evaluation recurses through that nesting. [`MAX_STATEMENT_TOKENS`],
//!    measured against the **real** lexer token stream (so there is no
//!    separately-maintained lexical model to diverge from and bypass), closes
//!    this vector — a statement's resulting tree depth is bounded above by
//!    its token count, since every node consumes at least one token.
//! 3. **Unwinding panics** (a malformed `Assign`/`Define` LHS, or any other
//!    reused-handler panic on a surprising shape). Evaluation runs inside
//!    [`catch_unwind`] on a worker thread with a large bounded stack, and any
//!    panic becomes a clean `Err(String)`; the session is rebuilt afterward
//!    (trading lost bindings for a guaranteed-usable session next call), so
//!    one crafted statement can never abort the process or wedge a session.
//!
//! [`lower_binary_chain`]: lower

mod lower;
mod printer;

pub use lower::LowerError;
pub use printer::print_derive;

use coding_adventures_derive_lexer::try_tokenize_derive;
use coding_adventures_derive_parser::try_parse_derive;
use lower::lower_program;
use std::panic::{catch_unwind, AssertUnwindSafe};
use symbolic_vm::{SymbolicBackend, VM};

/// Maximum length, in bytes, of a single source chunk handed to
/// [`DeriveSession::feed`].
///
/// A cheap first gate bounding per-call memory/time. 64 KiB is far beyond
/// any realistic interactive submission.
pub const MAX_INPUT_LEN: usize = 64 * 1024;

/// Maximum number of lexer tokens allowed in any single top-level statement.
///
/// See the module doc comment's "Robustness" point 2 — this closes the
/// long-flat-chain vector `derive-parser`'s own `MAX_RULE_DEPTH` does not
/// (and cannot) cover. 2000 tokens in one statement is already absurd for a
/// hand-written Derive worksheet line.
pub const MAX_STATEMENT_TOKENS: usize = 2000;

/// Stack size of the worker thread that runs evaluation and printing.
///
/// With the per-statement complexity cap bounding tree depth, this is
/// generous headroom so the bounded-but-still-deep trees are built,
/// evaluated, printed, and dropped well clear of any overflow, regardless of
/// the caller's own stack.
const EVAL_STACK_SIZE: usize = 512 * 1024 * 1024;

/// A persistent Derive session.
///
/// Owns the [`VM`] (and through it the [`SymbolicBackend`] environment), so
/// variable bindings (`x := 5`) and user-defined functions (`F(x) := x^2`)
/// persist across calls to [`feed`](DeriveSession::feed), exactly as in an
/// interactive Derive worksheet. The `#n` counter likewise persists.
pub struct DeriveSession {
    vm: VM,
    /// 1-based counter of displayed results so far — the `#n` worksheet
    /// index (MA07 §5).
    output_index: usize,
}

impl Default for DeriveSession {
    fn default() -> Self {
        Self::new()
    }
}

/// One displayed result line from a [`DeriveSession::feed`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// The 1-based `#n` index.
    pub index: usize,
    /// The result rendered in Derive surface notation.
    pub text: String,
}

impl DeriveSession {
    /// Create a fresh session with an empty environment and `#n` counter.
    pub fn new() -> Self {
        DeriveSession {
            vm: VM::new(Box::new(SymbolicBackend::new())),
            output_index: 0,
        }
    }

    /// Evaluate a chunk of Derive source and return the concatenated echo.
    ///
    /// Every statement produces one `#n: «value»` line (this subset has no
    /// statement-suppression syntax); the lines are concatenated in
    /// evaluation order, each ending in a newline. A parse or lowering error
    /// is returned as `Err` with the message.
    ///
    /// # Example
    ///
    /// ```
    /// use coding_adventures_derive_runtime::DeriveSession;
    /// let mut s = DeriveSession::new();
    /// assert_eq!(s.feed("1 + 2*3\n").unwrap(), "#1: 7\n");
    /// ```
    pub fn feed(&mut self, src: &str) -> Result<String, String> {
        let outputs = self.eval_to_outputs(src)?;
        let mut out = String::new();
        for o in outputs {
            out.push_str(&format!("#{}: {}\n", o.index, o.text));
        }
        Ok(out)
    }

    /// Evaluate `src` and return the structured list of displayed [`Output`]s.
    ///
    /// The lower-level cousin of [`feed`](DeriveSession::feed) — a REPL that
    /// wants to format the prompt itself uses this and renders each `Output`.
    pub fn eval_to_outputs(&mut self, src: &str) -> Result<Vec<Output>, String> {
        // Guard 1: bound total input size (cheap memory/time gate).
        if src.len() > MAX_INPUT_LEN {
            return Err(format!(
                "input too large: {} bytes exceeds the {}-byte limit",
                src.len(),
                MAX_INPUT_LEN
            ));
        }
        // Guard 2: reject a long flat chain before it can fold into a
        // stack-overflowing lowered tree (see the module "Robustness" note,
        // point 2 — deeply *nested* source is already rejected by
        // `derive-parser`'s own `MAX_RULE_DEPTH`, a *different* vector this
        // guard does not need to re-cover).
        check_statement_token_counts(src)?;

        // Guard 3 + panics: the lowering, the VM evaluation, and the printer
        // all run on a worker thread with a large bounded stack, and any
        // unwinding panic from the reused symbolic stack is caught.
        let vm = &mut self.vm;
        let start_index = self.output_index;
        let src_owned = src.to_string();
        let outcome = std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(EVAL_STACK_SIZE)
                .spawn_scoped(scope, || {
                    catch_unwind(AssertUnwindSafe(|| {
                        eval_source(vm, &src_owned, start_index)
                    }))
                })
                .expect("failed to spawn derive evaluation thread")
                .join()
        });

        match outcome {
            // Normal path: the worker produced the outputs (or a surface error).
            Ok(Ok(Ok((outputs, next_index)))) => {
                self.output_index = next_index;
                Ok(outputs)
            }
            Ok(Ok(Err(message))) => Err(message),
            // A panic the worker caught, or one that escaped and unwound the
            // join. Either way the env may be inconsistent, so rebuild.
            Ok(Err(payload)) | Err(payload) => {
                self.vm = VM::new(Box::new(SymbolicBackend::new()));
                self.output_index = 0;
                Err(panic_message(payload))
            }
        }
    }
}

/// Evaluate every statement in `src`, returning the displayed outputs and the
/// new `#n` counter. Runs on the worker thread.
fn eval_source(vm: &mut VM, src: &str, start_index: usize) -> Result<(Vec<Output>, usize), String> {
    // The parser's error string already carries a user-readable message, so
    // we forward it as-is.
    let ast = try_parse_derive(src)?;
    let statements = lower_program(&ast).map_err(|e| e.to_string())?;

    let mut outputs = Vec::new();
    let mut index = start_index;
    for stmt in statements {
        let result = vm.eval(stmt);
        index += 1;
        outputs.push(Output {
            index,
            text: print_derive(&result),
        });
    }
    Ok((outputs, index))
}

/// Reject input where any single statement lexes to too many tokens.
///
/// The count is taken from the **real** Derive lexer, the very one the
/// parser consumes, so there is no separately-maintained lexical model to
/// diverge from: comments (none in this subset) and whitespace are skip
/// patterns (absent from the token stream), and a top-level `NEWLINE` token
/// marks a statement boundary (the lexer drops bracketed newlines, so a
/// call or vector/matrix literal spanning several physical lines is
/// correctly counted as one statement). Unlike `wolfram-runtime`'s identical
/// guard, there is no `SEMI` reset case — Derive's only `;` use is the
/// matrix row separator, which never appears at bracket depth 0.
///
/// If the lexer itself errors (an untokenizable character), we return
/// `Ok(())` and let the parser surface the error uniformly — we never
/// reject solely because the *checker* could not lex something.
fn check_statement_token_counts(src: &str) -> Result<(), String> {
    let tokens = match try_tokenize_derive(src) {
        Ok(tokens) => tokens,
        Err(_) => return Ok(()), // unlexable — let the parser surface it
    };

    let mut count: usize = 0;
    for token in &tokens {
        if token.effective_type_name() == "NEWLINE" {
            count = 0;
            continue;
        }
        count += 1;
        if count > MAX_STATEMENT_TOKENS {
            return Err(format!(
                "statement too complex: more than {MAX_STATEMENT_TOKENS} tokens in one statement"
            ));
        }
    }
    Ok(())
}

/// Evaluate `src` once on a fresh [`DeriveSession`] and return its echo.
///
/// Convenience for callers that do not need persistent state.
pub fn eval(src: &str) -> Result<String, String> {
    DeriveSession::new().feed(src)
}

/// Recover a human-readable message from a caught panic payload.
fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Derive could not evaluate that input".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_folds_to_an_integer() {
        assert_eq!(eval("1 + 2*3\n").unwrap(), "#1: 7\n");
    }

    #[test]
    fn power_and_division() {
        assert_eq!(eval("2^10\n").unwrap(), "#1: 1024\n");
        assert_eq!(eval("10 / 2\n").unwrap(), "#1: 5\n");
    }

    #[test]
    fn free_symbols_stay_symbolic() {
        assert_eq!(eval("x + 0\n").unwrap(), "#1: x\n");
    }

    #[test]
    fn assignment_persists_across_calls() {
        let mut s = DeriveSession::new();
        assert_eq!(s.feed("x := 5\n").unwrap(), "#1: 5\n");
        assert_eq!(s.feed("x + 1\n").unwrap(), "#2: 6\n");
    }

    #[test]
    fn user_defined_function_end_to_end() {
        let mut s = DeriveSession::new();
        s.feed("SQUARE(x) := x*x\n").unwrap();
        assert_eq!(s.feed("SQUARE(5)\n").unwrap(), "#2: 25\n");
    }

    #[test]
    fn dif_differentiates_via_the_shared_handler() {
        // DIF(x^2, x) -> 2*x
        assert_eq!(eval("DIF(x^2, x)\n").unwrap(), "#1: 2*x\n");
    }

    #[test]
    fn int_integrates_via_the_shared_handler() {
        // INT(x, x) -> x^2/2
        let out = eval("INT(x, x)\n").unwrap();
        assert!(out.starts_with("#1: "), "got {out:?}");
    }

    #[test]
    fn if_selects_a_branch_via_the_held_handler() {
        assert_eq!(eval("IF(1 > 0, 42, 0)\n").unwrap(), "#1: 42\n");
    }

    #[test]
    fn sin_of_zero_folds_to_zero() {
        assert_eq!(eval("SIN(0)\n").unwrap(), "#1: 0\n");
    }

    #[test]
    fn equation_stays_unevaluated_not_assignment() {
        // `x = 4` is Equal, not Assign — it does not bind x.
        let mut s = DeriveSession::new();
        assert_eq!(s.feed("x = 4\n").unwrap(), "#1: x = 4\n");
        assert_eq!(s.feed("x + 1\n").unwrap(), "#2: x + 1\n");
    }

    #[test]
    fn multiple_statements_in_one_feed() {
        let out = eval("1 + 1\n2 + 2\n3 + 3\n").unwrap();
        assert_eq!(out, "#1: 2\n#2: 4\n#3: 6\n");
    }

    #[test]
    fn a_parse_error_is_returned_not_panicked() {
        let err = eval("1 +\n");
        assert!(err.is_err(), "expected a clean Err, got {err:?}");
    }

    #[test]
    fn session_recovers_after_a_parse_error() {
        let mut s = DeriveSession::new();
        let _ = s.feed("1 +\n");
        assert_eq!(s.feed("3 + 4\n").unwrap(), "#1: 7\n");
    }

    #[test]
    fn oversized_input_is_rejected_before_evaluation() {
        let huge = format!("{}1\n", "1+".repeat(MAX_INPUT_LEN));
        assert!(huge.len() > MAX_INPUT_LEN);
        let err = eval(&huge).unwrap_err();
        assert!(err.contains("too large"), "got {err:?}");
    }

    #[test]
    fn deeply_nested_parens_are_rejected_by_the_parsers_own_cap() {
        // derive-parser's own MAX_RULE_DEPTH rejects this before the runtime
        // ever sees a tree — this test proves the session surfaces that as a
        // clean Err (not a crash), not that this crate re-implements the cap.
        let depth = 5000;
        let src = format!("{}1{}\n", "(".repeat(depth), ")".repeat(depth));
        assert!(src.len() <= MAX_INPUT_LEN, "stays under the length cap");
        assert!(eval(&src).is_err());
    }

    #[test]
    fn long_flat_chain_is_rejected_by_the_statement_token_cap() {
        // additive/multiplicative are grammar REPETITIONS, not recursive
        // rule calls, so derive-parser's MAX_RULE_DEPTH does not bound chain
        // length — this is the vector MAX_STATEMENT_TOKENS closes instead.
        let src = format!("{}1\n", "1+".repeat(5_000));
        assert!(src.len() <= MAX_INPUT_LEN);
        assert!(eval(&src).unwrap_err().contains("too complex"));
    }

    #[test]
    fn moderate_chain_still_evaluates() {
        let src = format!("{}1\n", "1+".repeat(50));
        assert_eq!(eval(&src).unwrap(), "#1: 51\n");
    }

    #[test]
    fn a_malformed_assign_lhs_is_caught_not_aborted() {
        // `5 := 3` lowers to Assign(5, 3). The reused VM's assign_handler
        // *panics* when the lhs is not a symbol — the worker-thread
        // `catch_unwind` must convert that panic into a clean `Err`, and the
        // session must remain usable afterward (the env is rebuilt).
        let mut s = DeriveSession::new();
        assert!(
            s.feed("5 := 3\n").is_err(),
            "a non-symbol Assign lhs must return Err, never abort"
        );
        assert_eq!(s.feed("2 + 2\n").unwrap(), "#1: 4\n");
    }

    // --- Self-referential-reassignment DoS guard (a security audit's
    // finding, shared with every consumer of `symbolic-vm`'s
    // `assign_handler` -- see `symbolic_vm::handlers::MAX_BOUND_VALUE_NODES`
    // / `MAX_BOUND_VALUE_DEPTH`'s own doc comments) -- `a := a * a` /
    // `a := a + a`, repeated even a handful of times, doubles the bound
    // value's node count and/or nesting depth every step, reaching
    // millions of nodes from a few hundred bytes of source. Derive lowers
    // its own `:=` straight to `symbolic_ir::ASSIGN` and evaluates through
    // the shared `vm.eval`, so it is protected by the same choke-point fix
    // with no crate-specific bypass work needed -- these tests prove that
    // end-to-end through a real `DeriveSession`.
    // -------------------------------------------------------------------

    #[test]
    fn self_referential_multiplication_worksheet_is_rejected_cleanly() {
        // The exact audited scenario, as a Derive worksheet (derive.grammar's
        // `program = { statement_line }` accepts many newline-terminated
        // statements in ONE `feed` call): `a := x + y` then `a := a * a`
        // repeated. 30 repetitions is far past the real trip point (~15th
        // step) -- without the guard this reaches billions of nodes; with
        // it, a clean `Err` in well under a second.
        let mut src = String::from("a := x + y\n");
        for _ in 0..30 {
            src.push_str("a := a * a\n");
        }
        assert!(src.len() < 500, "source should stay tiny: {} bytes", src.len());

        let mut s = DeriveSession::new();
        let err = s.feed(&src).unwrap_err();
        assert!(err.contains("nodes"), "expected a node-count rejection, got {err:?}");
        // The worker-thread `catch_unwind` rebuilds the session on panic —
        // it must remain usable afterward.
        assert_eq!(s.feed("2 + 2\n").unwrap(), "#1: 4\n");
    }

    #[test]
    fn self_referential_addition_worksheet_is_rejected_cleanly() {
        // Same attack shape via the audit's other named example, `a := a +
        // a`. This one trips the DEPTH guard, not the node-count guard --
        // `Add`'s own flatten-then-left-associate canonicalization rebuilds
        // a chain whose depth equals its leaf count, and leaf count doubles
        // too. Without a depth guard this reproduces a genuine, uncatchable
        // native stack overflow on a later statement's symbol lookup, not
        // merely a slow hang.
        let mut src = String::from("a := x + y\n");
        for _ in 0..30 {
            src.push_str("a := a + a\n");
        }

        let mut s = DeriveSession::new();
        let err = s.feed(&src).unwrap_err();
        assert!(
            err.contains("levels"),
            "expected a nesting-depth rejection, got {err:?}"
        );
        assert_eq!(s.feed("2 + 2\n").unwrap(), "#1: 4\n");
    }

    #[test]
    fn a_handful_of_self_multiplications_under_the_cap_still_evaluate_correctly() {
        // Non-false-positive check: a FEW self-referential reassignments,
        // comfortably under the caps, must still evaluate normally.
        let mut s = DeriveSession::new();
        s.feed("a := 2\na := a * a\na := a * a\n").unwrap(); // 2 -> 4 -> 16
        assert_eq!(s.feed("a\n").unwrap(), "#4: 16\n");
    }

    #[test]
    fn vector_literal_evaluates_elementwise() {
        // [1+1, 2*3, 2^3] -> [2, 6, 8] — a List's own head is data (held), but
        // each element still evaluates through the shared arithmetic handlers.
        assert_eq!(eval("[1 + 1, 2*3, 2^3]\n").unwrap(), "#1: [2, 6, 8]\n");
    }

    #[test]
    fn matrix_literal_evaluates_end_to_end() {
        assert_eq!(eval("[1 + 1, 2; 3, 4 * 2]\n").unwrap(), "#1: [2, 2; 3, 8]\n");
    }

    #[test]
    fn vector_assignment_persists_across_calls() {
        let mut s = DeriveSession::new();
        assert_eq!(s.feed("v := [1, 2, 3]\n").unwrap(), "#1: [1, 2, 3]\n");
        assert_eq!(s.feed("v\n").unwrap(), "#2: [1, 2, 3]\n");
    }

    #[test]
    fn the_one_shot_eval_helper_matches_a_session() {
        let one = eval("2 + 3\n").unwrap();
        let two = DeriveSession::new().feed("2 + 3\n").unwrap();
        assert_eq!(one, two);
    }

    #[test]
    fn empty_and_whitespace_input_yields_nothing() {
        assert_eq!(eval("\n").unwrap(), "");
    }

    #[test]
    fn a_small_derive_program_evaluates_end_to_end() {
        // A bound variable distinct from the DIF variable (`t`, not the
        // function's own parameter `y`) — `y` never appears in the body, so
        // substituting it at call time changes nothing, and `DIF(SIN(t), t)`
        // evaluates via the shared handler to `COS(t)` regardless of the
        // argument `H` is called with. (`F(x) := DIF(SIN(x), x); F(0)` would
        // NOT simplify to `cos(0)`: substituting `x` -> `0` also replaces
        // DIF's *variable* argument, leaving the well-defined-but-unevaluable
        // `DIF(0, 0)` — the same substitution-collision a Wolfram/Macsyma
        // user-defined function hits identically, not a Derive-specific bug.)
        let mut s = DeriveSession::new();
        s.feed("H(y) := DIF(SIN(t), t)\n").unwrap();
        let out = s.feed("H(0)\n").unwrap();
        assert_eq!(out, "#2: COS(t)\n");
    }
}
