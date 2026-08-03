//! # Reduce runtime — lower Reduce syntax to `symbolic-ir`, evaluate via `symbolic-vm`.
//!
//! This is the **R-4** deliverable of the Reduce-language lane (MA08 §2). R-1
//! through R-3 gave us a spec, a lexer, and a parser; this crate is the
//! *runtime* that finally **evaluates** Reduce source — by **reusing the
//! shared symbolic substrate** rather than writing a bespoke interpreter,
//! exactly the reuse story [`derive-runtime`](../derive-runtime) already
//! demonstrated for the other Wave-5 CAS-family language:
//!
//! ```text
//!   Reduce source
//!        │
//!        ▼  coding_adventures_reduce_parser::try_parse_reduce  (R-3)
//!   GrammarASTNode  (surface tree: assignment, if_expr, additive, postfix, …)
//!        │
//!        ▼  crate::lower                                        (this crate)
//!   symbolic_ir::IRNode   (Add, Mul, Pow, Assign, Define, If, List, …)
//!        │
//!        ▼  symbolic_vm::VM over SymbolicBackend, UNCHANGED
//!   symbolic_ir::IRNode   (evaluated)
//!        │
//!        ▼  crate::printer
//!   Reduce surface string  (infix, {a,b,c}, and/or/not, if...then...else)
//! ```
//!
//! ## No custom `Backend` needed — and what that costs (MA08 §5)
//!
//! Per MA08 §2/§5, R-4 reuses [`symbolic_vm::SymbolicBackend`] *unchanged* —
//! no bespoke `Backend` the way `wolfram-runtime`/`macsyma-runtime` each
//! layer their own list/string built-ins onto. `SymbolicBackend` (built
//! with `simplify: true`) already provides everything MA08 §3 needs for
//! arithmetic (`Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`), comparison (`Equal`/
//! `Less`/`Greater`/`LessEqual`/`GreaterEqual`/`NotEqual`), logic (`And`/
//! `Or`/`Not`), the held `Assign`/`Define`/`If` forms, and `List` — so this
//! crate's entire Reduce-specific contribution is the **lowering**
//! (surface syntax → canonical heads, [`lower`]) and the **printer**
//! (canonical heads → surface syntax, [`printer`]); the evaluation engine
//! itself is reused completely unchanged.
//!
//! What MA08 §5 gets *wrong*, confirmed by grepping the actual crates
//! (see [`lower`]'s module doc comment for the full accounting): the
//! *shared* handler table has **no** handler at all for `CompoundExpression`
//! (Reduce's `<< ... >>`), `First`/`Second`/`Third`/`Rest`/`Part`/`Append`/
//! `Reverse` (the list accessors/constructors), or `Cons` — Macsyma's list
//! functions and Wolfram's `CompoundExpression` are real, but wired through
//! a *bespoke* `Backend` specific to that language, which is exactly what
//! R-4 is not supposed to build. This crate still lowers to the
//! structurally-correct heads MA08 §3 documents (so a future item that adds
//! real handlers needs no lowering changes), but evaluating one of these
//! calls does not perform the operation — arguments evaluate (so `Assign`/
//! `Define` side effects inside a `<< ... >>` still happen, in order), the
//! call itself just stays unevaluated, exactly like an undefined user
//! function. Also disclosed: MA08 §3's own prose describes the arithmetic
//! heads as `Plus`/`Subtract`/`Times`/`Power` (and even expands `/`/unary
//! `-` into `Times`/`Power` applications) — none of those spellings exist
//! in `symbolic-ir` either; the real, already-reused heads are `Add`/`Sub`/
//! `Mul`/`Div`/`Pow`/`Neg`, which is what this crate actually lowers to.
//!
//! ## Public contract
//!
//! [`ReduceSession::feed`] is string-in / string-out. It evaluates a chunk
//! of source (one or more `;`/`$`-terminated statements) and returns the
//! surface rendering of each result, one per line. Unlike
//! [`derive-runtime`](../derive-runtime) or Wolfram's `In[n]:=`/`Out[n]=`,
//! **Reduce's own session transcript has no numbered-input convention**
//! (MA08 §2/§5) — so, unlike [`derive_runtime::Output`]'s `#n` index,
//! [`Output`] here carries only the rendered text, and `feed` never prints
//! any prefix. Bindings persist across calls, so an interactive `x := 5`
//! then `x + 1` works.
//!
//! [`derive_runtime::Output`]: https://docs.rs/coding-adventures-derive-runtime
//!
//! ## Robustness at the trust boundary
//!
//! `feed` takes arbitrary user text, so — exactly as in
//! `derive-runtime`/`wolfram-runtime`/`maxima-runtime` — it is the trust
//! boundary for the whole reused stack. Two layered guards close the two
//! independent deep-recursion vectors (per the "depth caps don't compose
//! across boundaries" lesson: a cap on one recursive walk does not protect
//! a *different* recursive walk over the same or a derived tree):
//!
//! 1. **Deeply *nested* source**, whether parenthesised (`((((…))))`),
//!    right-recursive assignment/`if`-`else`/cons/power chains, or any
//!    other shape `reduce-parser`'s own `MAX_RULE_DEPTH` measures — this
//!    vector is already closed by the *parser*, before this crate's
//!    lowering/printing recursion (which walks the *same* already-shallow
//!    tree the parser handed back) ever runs.
//! 2. **A long *flat* chain that folds into a deeply *nested* lowered
//!    tree.** `reduce-parser`'s own `MAX_RULE_DEPTH` doc comment proves,
//!    with a throwaway probe grammar, that its EBNF `{ x }`-repetition
//!    productions (`additive`, `multiplicative`, a chained `postfix` call
//!    `f(x)(y)(z)…`, `logical_or`/`logical_and`, `arglist`) cost the parser
//!    *zero* native stack regardless of width — so `MAX_RULE_DEPTH` does not
//!    (and structurally cannot) bound a flat chain's length. Of those,
//!    [`lower::lower_binary_chain`] (additive/multiplicative) and
//!    [`lower::lower_postfix`]'s call-chaining loop both left-fold an
//!    N-repetition into an N-1-deep *nested* `Apply` tree — `logical_or`/
//!    `logical_and`/`arglist` do not (they fold flat/n-ary, or lower each
//!    element independently — see their own doc comments), so they need no
//!    guarding here. [`MAX_STATEMENT_TOKENS`], measured against the
//!    **real** lexer token stream (so there is no separately-maintained
//!    lexical model to diverge from and bypass), closes this vector — a
//!    statement's resulting tree depth is bounded above by its token count,
//!    since every node consumes at least one token. Unlike
//!    `derive-runtime`'s identical-in-spirit guard (which resets its
//!    running count on a `NEWLINE` token, since Derive's statement
//!    separator IS the newline), this crate resets on Reduce's own real
//!    separators, `SEMI`/`DOLLAR` (`;`/`$`) — but **only when they mark a
//!    genuine top-level statement boundary**, i.e. at bracket depth 0. An
//!    earlier version reset unconditionally, including on a `;`/`$`
//!    lexically inside a `<< ... >>` group statement, reasoning that
//!    `group_expr`'s own statements lower flat ([`lower::lower_group_expr`])
//!    so bounding each sub-statement's own token run independently was fine
//!    — but a `/security-review` finding caught the gap that reasoning
//!    missed: a *parenthesized* group construct can itself be embedded as
//!    just **one operand** of a much larger enclosing additive/
//!    multiplicative/postfix chain (`1 + 1 + (<<0;0>>) + 1 + 1 + ...`), and
//!    the `;` inside it reset the counter for that *outer* chain too, even
//!    though the outer chain still folds every operand into one deeply
//!    nested tree regardless. [`check_statement_token_counts`] now tracks
//!    bracket nesting (`LPAREN`/`RPAREN`, `LBRACE`/`RBRACE`, `GROUP_OPEN`/
//!    `GROUP_CLOSE`) and only resets at depth 0 — see that function's own
//!    doc comment for the full accounting, including the deliberate (and
//!    safe) trade-off of being slightly more conservative than the strict
//!    minimum in one unusual case.
//! 3. **Unwinding panics** (a malformed `Assign`/`Define` LHS, or any other
//!    reused-handler panic on a surprising shape). Evaluation runs inside
//!    [`catch_unwind`] on a worker thread with a large bounded stack, and any
//!    panic becomes a clean `Err(String)`; the session is rebuilt afterward
//!    (trading lost bindings for a guaranteed-usable session next call), so
//!    one crafted statement can never abort the process or wedge a session.
//!    (A thread-spawn failure itself — OS thread-count/memory pressure — is
//!    handled as an ordinary `Err` too, not a caller-thread panic; an
//!    earlier version `.expect()`-unwrapped that outside this boundary,
//!    another `/security-review` finding.)

mod lower;
mod printer;

pub use lower::LowerError;
pub use printer::print_reduce;

use coding_adventures_reduce_lexer::try_tokenize_reduce;
use coding_adventures_reduce_parser::try_parse_reduce;
use lower::lower_program;
use std::panic::{catch_unwind, AssertUnwindSafe};
use symbolic_vm::{SymbolicBackend, VM};

/// Maximum length, in bytes, of a single source chunk handed to
/// [`ReduceSession::feed`].
///
/// A cheap first gate bounding per-call memory/time. 64 KiB is far beyond
/// any realistic interactive submission.
pub const MAX_INPUT_LEN: usize = 64 * 1024;

/// Maximum number of lexer tokens allowed in any single statement (a run of
/// tokens between two `SEMI`/`DOLLAR` separators, or between a separator and
/// the start/end of input).
///
/// See the module doc comment's "Robustness" point 2 — this closes the
/// long-flat-chain vector `reduce-parser`'s own `MAX_RULE_DEPTH` does not
/// (and cannot) cover. 2000 tokens in one statement is already absurd for a
/// hand-written Reduce program line, matching `derive-runtime`'s identical
/// choice of cap.
pub const MAX_STATEMENT_TOKENS: usize = 2000;

/// Stack size of the worker thread that runs evaluation and printing.
///
/// With the per-statement complexity cap bounding tree depth, this is
/// generous headroom so the bounded-but-still-deep trees are built,
/// evaluated, printed, and dropped well clear of any overflow, regardless of
/// the caller's own stack.
const EVAL_STACK_SIZE: usize = 512 * 1024 * 1024;

/// A persistent Reduce session.
///
/// Owns the [`VM`] (and through it the [`SymbolicBackend`] environment), so
/// variable bindings (`x := 5`) and user-defined operators (`h(l, m) := l +
/// m`) persist across calls to [`feed`](ReduceSession::feed), exactly as in
/// an interactive Reduce session.
pub struct ReduceSession {
    vm: VM,
}

impl Default for ReduceSession {
    fn default() -> Self {
        Self::new()
    }
}

/// One displayed result line from a [`ReduceSession::feed`] call.
///
/// Reduce's own session transcript has no numbered-input convention (MA08
/// §2/§5, unlike Derive's `#n:` or Wolfram's `In[n]:=`), so — unlike
/// `derive-runtime::Output` — this carries only the rendered text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// The result rendered in Reduce surface notation.
    pub text: String,
}

impl ReduceSession {
    /// Create a fresh session with an empty environment.
    pub fn new() -> Self {
        ReduceSession {
            vm: VM::new(Box::new(SymbolicBackend::new())),
        }
    }

    /// Evaluate a chunk of Reduce source and return the concatenated echo.
    ///
    /// Every statement produces one plain output line (no numbering — see
    /// the module/[`Output`] doc comments); the lines are concatenated in
    /// evaluation order, each ending in a newline. A parse or lowering error
    /// is returned as `Err` with the message.
    ///
    /// # Example
    ///
    /// ```
    /// use coding_adventures_reduce_runtime::ReduceSession;
    /// let mut s = ReduceSession::new();
    /// assert_eq!(s.feed("1 + 2*3;\n").unwrap(), "7\n");
    /// ```
    pub fn feed(&mut self, src: &str) -> Result<String, String> {
        let outputs = self.eval_to_outputs(src)?;
        let mut out = String::new();
        for o in outputs {
            out.push_str(&o.text);
            out.push('\n');
        }
        Ok(out)
    }

    /// Evaluate `src` and return the structured list of displayed [`Output`]s.
    ///
    /// The lower-level cousin of [`feed`](ReduceSession::feed) — a REPL that
    /// wants to format its own prompt uses this and renders each `Output`.
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
        // `reduce-parser`'s own `MAX_RULE_DEPTH`, a *different* vector this
        // guard does not need to re-cover).
        check_statement_token_counts(src)?;

        // Guard 3 + panics: the lowering, the VM evaluation, and the printer
        // all run on a worker thread with a large bounded stack, and any
        // unwinding panic from the reused symbolic stack is caught.
        //
        // `spawn_scoped` itself can fail (OS thread-count/memory pressure —
        // exactly the kind of adversarial-load condition this crate's own
        // "trust boundary" doc anticipates) — a previous version unwrapped
        // that with `.expect(...)`, panicking on the *calling* thread,
        // entirely outside the `catch_unwind` below (a `/security-review`
        // finding). Resolved to a plain `Result<Vec<Output>, String>`
        // *inside* the scoped closure instead (a `ScopedJoinHandle` can't
        // outlive it), with `panicked` — mutated only on this, the calling,
        // thread, never touched from the spawned one — flagging whether
        // `self.vm` needs rebuilding after the scope returns.
        let vm = &mut self.vm;
        let src_owned = src.to_string();
        let mut panicked = false;
        let result: Result<Vec<Output>, String> = std::thread::scope(|scope| {
            let handle = match std::thread::Builder::new()
                .stack_size(EVAL_STACK_SIZE)
                .spawn_scoped(scope, || {
                    catch_unwind(AssertUnwindSafe(|| eval_source(vm, &src_owned)))
                }) {
                Ok(handle) => handle,
                Err(io_err) => {
                    // Nothing ran on another thread at all, so there is no
                    // VM state to rebuild.
                    return Err(format!(
                        "failed to spawn the Reduce evaluation thread: {io_err}"
                    ));
                }
            };
            // `handle.join()`'s outer `Result` is join's own (a panic that
            // somehow escaped the `catch_unwind` below); its `Ok` carries
            // `catch_unwind`'s own `Result` (a panic it *did* catch); *that*
            // `Ok` finally carries `eval_source`'s own `Result<Vec<Output>,
            // String>` (an ordinary surface error, not a panic at all).
            match handle.join() {
                Ok(Ok(Ok(outputs))) => Ok(outputs),
                Ok(Ok(Err(message))) => Err(message),
                // A panic the worker caught, or one that escaped and unwound
                // the join. Either way the env may be inconsistent, so flag
                // it for a rebuild once we're back on this thread.
                Ok(Err(payload)) | Err(payload) => {
                    panicked = true;
                    Err(panic_message(payload))
                }
            }
        });

        if panicked {
            self.vm = VM::new(Box::new(SymbolicBackend::new()));
        }
        result
    }
}

/// Evaluate every statement in `src`, returning the displayed outputs. Runs
/// on the worker thread.
fn eval_source(vm: &mut VM, src: &str) -> Result<Vec<Output>, String> {
    // The parser's error string already carries a user-readable message, so
    // we forward it as-is.
    let ast = try_parse_reduce(src)?;
    let statements = lower_program(&ast).map_err(|e| e.to_string())?;

    let mut outputs = Vec::new();
    for stmt in statements {
        let result = vm.eval(stmt);
        outputs.push(Output {
            text: print_reduce(&result),
        });
    }
    Ok(outputs)
}

/// Reject input where any single statement lexes to too many tokens.
///
/// The count is taken from the **real** Reduce lexer, the very one the
/// parser consumes, so there is no separately-maintained lexical model to
/// diverge from: a `SEMI`/`DOLLAR` token (`;`/`$`, Reduce's own
/// interchangeable statement terminators — manual §5.1, unlike
/// `derive-runtime`'s significant-`NEWLINE` reset) marks a statement
/// boundary and resets the running count.
///
/// **Only at bracket depth 0**, though — a `/security-review` finding caught
/// a bypass in an earlier version of this guard that reset unconditionally,
/// including on a `;`/`$` lexically inside a `<< ... >>` group statement.
/// `lower_group_expr` does lower a group's *own* statements flat, but a
/// group construct can itself be parenthesized and embedded as **one
/// operand inside a much larger enclosing `additive`/`multiplicative`/
/// `postfix` chain** (`reduce.grammar`'s `atom = ... | group`, `group =
/// LPAREN expr RPAREN`) — e.g. `1 + 1 + (<<0;0>>) + 1 + 1 + ...`. The `;`
/// inside `(<<0;0>>)` would reset the counter for the *outer* chain too,
/// even though [`lower::lower_binary_chain`] still left-folds every operand
/// of that outer chain — before, at, and after the embedded reset point —
/// into one single nested `Apply` tree, since it is one contiguous grammar
/// repetition with no true top-level statement boundary at each reset
/// point. Empirically, a 32,000-term `+` chain with a trivial `+(<<0;0>>)`
/// spliced in every 900 terms sailed through with no error at all, while a
/// plain uninterrupted chain of the same length was correctly rejected —
/// 16× the documented cap, undetected.
///
/// The fix: track nesting depth over `LPAREN`/`RPAREN`, `LBRACE`/`RBRACE`,
/// and `GROUP_OPEN`/`GROUP_CLOSE` (every bracket-like construct this
/// grammar has), and only treat a `SEMI`/`DOLLAR` as a genuine reset point
/// when depth is back to 0. This is slightly more conservative than
/// strictly necessary — a single *top-level* `<< s1; s2; ...; sN >>` with
/// many short, independently-flat sub-statements no longer resets between
/// them either, since depth stays 1 throughout the group — but that only
/// means such an (unusual) statement hits the cap sooner than the absolute
/// minimum required, never that a genuinely deep-folding chain slips
/// through uncounted. A negative depth from malformed/unbalanced brackets
/// (which the real parser will separately reject) only makes future
/// `;`/`$` tokens *not* reset, i.e. more conservative still — never less.
///
/// If the lexer itself errors (an untokenizable character), we return
/// `Ok(())` and let the parser surface the error uniformly — we never
/// reject solely because the *checker* could not lex something.
fn check_statement_token_counts(src: &str) -> Result<(), String> {
    let tokens = match try_tokenize_reduce(src) {
        Ok(tokens) => tokens,
        Err(_) => return Ok(()), // unlexable — let the parser surface it
    };

    let mut count: usize = 0;
    let mut depth: i32 = 0;
    for token in &tokens {
        match token.effective_type_name() {
            "LPAREN" | "LBRACE" | "GROUP_OPEN" => depth += 1,
            "RPAREN" | "RBRACE" | "GROUP_CLOSE" => depth -= 1,
            "SEMI" | "DOLLAR" if depth == 0 => {
                count = 0;
                continue;
            }
            _ => {}
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

/// Evaluate `src` once on a fresh [`ReduceSession`] and return its echo.
///
/// Convenience for callers that do not need persistent state.
pub fn eval(src: &str) -> Result<String, String> {
    ReduceSession::new().feed(src)
}

/// Recover a human-readable message from a caught panic payload.
fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Reduce could not evaluate that input".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_folds_to_an_integer() {
        assert_eq!(eval("1 + 2*3;\n").unwrap(), "7\n");
    }

    #[test]
    fn power_and_division() {
        assert_eq!(eval("2^10;\n").unwrap(), "1024\n");
        assert_eq!(eval("10 / 2;\n").unwrap(), "5\n");
        assert_eq!(eval("2**10;\n").unwrap(), "1024\n");
    }

    #[test]
    fn free_symbols_stay_symbolic() {
        assert_eq!(eval("x + 0;\n").unwrap(), "x\n");
    }

    #[test]
    fn assignment_persists_across_calls() {
        let mut s = ReduceSession::new();
        assert_eq!(s.feed("x := 5;\n").unwrap(), "5\n");
        assert_eq!(s.feed("x + 1;\n").unwrap(), "6\n");
    }

    #[test]
    fn user_defined_operator_end_to_end() {
        let mut s = ReduceSession::new();
        s.feed("h(x) := x*x;\n").unwrap();
        assert_eq!(s.feed("h(5);\n").unwrap(), "25\n");
    }

    #[test]
    fn if_selects_a_branch_via_the_held_handler() {
        assert_eq!(eval("if 1 > 0 then 42 else 0;\n").unwrap(), "42\n");
        assert_eq!(eval("if 0 > 1 then 42 else 7;\n").unwrap(), "7\n");
    }

    #[test]
    fn if_with_no_else_returns_false_on_a_failing_condition() {
        assert_eq!(eval("if 0 > 1 then 42;\n").unwrap(), "False\n");
    }

    #[test]
    fn equation_stays_unevaluated_not_assignment() {
        // `x = 4` is Equal, not Assign — it does not bind x.
        let mut s = ReduceSession::new();
        assert_eq!(s.feed("x = 4;\n").unwrap(), "x = 4\n");
        assert_eq!(s.feed("x + 1;\n").unwrap(), "x + 1\n");
    }

    #[test]
    fn multiple_statements_in_one_feed() {
        let out = eval("1 + 1; 2 + 2; 3 + 3;\n").unwrap();
        assert_eq!(out, "2\n4\n6\n");
    }

    #[test]
    fn semi_and_dollar_terminators_are_interchangeable() {
        let out = eval("1 + 1$ 2 + 2;\n").unwrap();
        assert_eq!(out, "2\n4\n");
    }

    #[test]
    fn list_literal_evaluates_elementwise() {
        assert_eq!(
            eval("{1 + 1, 2*3, 2^3};\n").unwrap(),
            "{2, 6, 8}\n"
        );
    }

    #[test]
    fn list_assignment_persists_across_calls() {
        let mut s = ReduceSession::new();
        assert_eq!(s.feed("v := {1, 2, 3};\n").unwrap(), "{1, 2, 3}\n");
        assert_eq!(s.feed("v;\n").unwrap(), "{1, 2, 3}\n");
    }

    #[test]
    fn cons_onto_a_literal_list_folds_and_evaluates() {
        assert_eq!(eval("1 . {2, 3};\n").unwrap(), "{1, 2, 3}\n");
    }

    #[test]
    fn boolean_logic_evaluates() {
        assert_eq!(eval("1 > 0 and 2 > 1;\n").unwrap(), "True\n");
        assert_eq!(eval("not (1 > 2);\n").unwrap(), "True\n");
    }

    #[test]
    fn a_parse_error_is_returned_not_panicked() {
        let err = eval("1 +\n");
        assert!(err.is_err(), "expected a clean Err, got {err:?}");
    }

    #[test]
    fn session_recovers_after_a_parse_error() {
        let mut s = ReduceSession::new();
        let _ = s.feed("1 +\n");
        assert_eq!(s.feed("3 + 4;\n").unwrap(), "7\n");
    }

    #[test]
    fn oversized_input_is_rejected_before_evaluation() {
        let huge = format!("{}1;\n", "1+".repeat(MAX_INPUT_LEN));
        assert!(huge.len() > MAX_INPUT_LEN);
        let err = eval(&huge).unwrap_err();
        assert!(err.contains("too large"), "got {err:?}");
    }

    #[test]
    fn deeply_nested_parens_are_rejected_by_the_parsers_own_cap() {
        // reduce-parser's own MAX_RULE_DEPTH rejects this before the runtime
        // ever sees a tree — this test proves the session surfaces that as a
        // clean Err (not a crash), not that this crate re-implements the cap.
        let depth = 5000;
        let src = format!("{}1{};\n", "(".repeat(depth), ")".repeat(depth));
        assert!(src.len() <= MAX_INPUT_LEN, "stays under the length cap");
        assert!(eval(&src).is_err());
    }

    #[test]
    fn long_flat_additive_chain_is_rejected_by_the_statement_token_cap() {
        // additive/multiplicative are grammar REPETITIONS, not recursive
        // rule calls, so reduce-parser's MAX_RULE_DEPTH does not bound
        // chain length — this is the vector MAX_STATEMENT_TOKENS closes.
        let src = format!("{}1;\n", "1+".repeat(5_000));
        assert!(src.len() <= MAX_INPUT_LEN);
        assert!(eval(&src).unwrap_err().contains("too complex"));
    }

    #[test]
    fn long_flat_postfix_call_chain_is_rejected_by_the_statement_token_cap() {
        // f(x)(x)(x)... -- `postfix`'s own call-chaining loop folds each
        // successive call into a nested Apply, another repetition-driven
        // (not recursion-driven) vector MAX_RULE_DEPTH does not bound.
        let src = format!("f{};\n", "(x)".repeat(5_000));
        assert!(src.len() <= MAX_INPUT_LEN);
        assert!(eval(&src).unwrap_err().contains("too complex"));
    }

    #[test]
    fn a_long_chain_inside_a_group_statement_is_still_capped() {
        // The token-count reset on SEMI/DOLLAR happens even lexically
        // inside `<< ... >>` (see the module doc comment) -- this proves a
        // dangerous chain hidden as one of a group statement's own
        // sub-statements is still caught.
        let src = format!("<< 1; {}1 >>;\n", "1+".repeat(5_000));
        assert!(eval(&src).unwrap_err().contains("too complex"));
    }

    #[test]
    fn a_group_statement_embedded_inside_a_larger_chain_cannot_hide_the_chains_true_length() {
        // `/security-review` finding: resetting on ANY `;`/`$` regardless of
        // bracket nesting let a `;` lexically inside a *parenthesized* group
        // construct — itself just one operand of a much larger enclosing
        // additive chain — reset the counter for that outer chain too, even
        // though `lower_binary_chain` still folds every operand (before, at,
        // and after the embedded reset point) into one single nested `Apply`
        // tree, since the whole thing is one contiguous grammar repetition
        // with no true top-level statement boundary at each reset point.
        // Splicing a trivial `(<<0;0>>)` into an otherwise-uninterrupted
        // chain every 900 terms previously let a chain 16x the documented
        // cap sail through with no error at all; the fix tracks bracket
        // depth and only resets at depth 0, so the true (much longer than
        // MAX_STATEMENT_TOKENS) chain length must still be rejected.
        let mut src = String::new();
        for i in 0..32_000 {
            if i > 0 && i % 900 == 0 {
                src.push_str("+(<<0;0>>)");
            }
            src.push_str("+1");
        }
        src.push_str(";\n");
        assert!(
            src.len() <= MAX_INPUT_LEN,
            "the PoC must fit under the input-size cap on its own, so only \
             the statement-token-count guard is what's actually being tested"
        );
        assert!(
            eval(&src).unwrap_err().contains("too complex"),
            "a chain far longer than MAX_STATEMENT_TOKENS must not be hidden \
             by an embedded group statement's own internal reset"
        );
    }

    #[test]
    fn moderate_chain_still_evaluates() {
        let src = format!("{}1;\n", "1+".repeat(50));
        assert_eq!(eval(&src).unwrap(), "51\n");
    }

    #[test]
    fn a_malformed_assign_lhs_is_caught_not_aborted() {
        // `5 := 3` lowers to Assign(5, 3). The reused VM's assign_handler
        // *panics* when the lhs is not a symbol — the worker-thread
        // `catch_unwind` must convert that panic into a clean `Err`, and the
        // session must remain usable afterward (the env is rebuilt).
        let mut s = ReduceSession::new();
        assert!(
            s.feed("5 := 3;\n").is_err(),
            "a non-symbol Assign lhs must return Err, never abort"
        );
        assert_eq!(s.feed("2 + 2;\n").unwrap(), "4\n");
    }

    // --- Self-referential-reassignment DoS guard (a security audit's
    // finding, shared with every consumer of `symbolic-vm`'s
    // `assign_handler` -- see `symbolic_vm::handlers::MAX_BOUND_VALUE_NODES`
    // / `MAX_BOUND_VALUE_DEPTH`'s own doc comments) -- `a := a * a` /
    // `a := a + a`, repeated even a handful of times, doubles the bound
    // value's node count and/or nesting depth every step, reaching millions
    // of nodes from a few hundred bytes of source. Reduce lowers its own
    // `:=` straight to `symbolic_ir::ASSIGN` and evaluates through the
    // shared `vm.eval`, so it is protected by the same choke-point fix with
    // no crate-specific bypass work needed -- these tests prove that
    // end-to-end through a real `ReduceSession`.
    // -------------------------------------------------------------------

    #[test]
    fn self_referential_multiplication_worksheet_is_rejected_cleanly() {
        // The exact audited scenario, as a Reduce worksheet
        // (reduce.grammar's `program = { statement_line } [ statement ]`
        // accepts many `;`-terminated statements in ONE `feed` call):
        // `a := x + y` then `a := a * a` repeated. 30 repetitions is far
        // past the real trip point (~15th step) -- without the guard this
        // reaches billions of nodes; with it, a clean `Err` in well under a
        // second.
        let mut src = String::from("a := x + y;\n");
        for _ in 0..30 {
            src.push_str("a := a * a;\n");
        }
        assert!(src.len() < 500, "source should stay tiny: {} bytes", src.len());

        let mut s = ReduceSession::new();
        let err = s.feed(&src).unwrap_err();
        assert!(err.contains("nodes"), "expected a node-count rejection, got {err:?}");
        // The worker-thread `catch_unwind` rebuilds the session on panic —
        // it must remain usable afterward.
        assert_eq!(s.feed("2 + 2;\n").unwrap(), "4\n");
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
        let mut src = String::from("a := x + y;\n");
        for _ in 0..30 {
            src.push_str("a := a + a;\n");
        }

        let mut s = ReduceSession::new();
        let err = s.feed(&src).unwrap_err();
        assert!(
            err.contains("levels"),
            "expected a nesting-depth rejection, got {err:?}"
        );
        assert_eq!(s.feed("2 + 2;\n").unwrap(), "4\n");
    }

    #[test]
    fn a_handful_of_self_multiplications_under_the_cap_still_evaluate_correctly() {
        // Non-false-positive check: a FEW self-referential reassignments,
        // comfortably under the caps, must still evaluate normally.
        let mut s = ReduceSession::new();
        s.feed("a := 2;\na := a * a;\na := a * a;\n").unwrap(); // 2 -> 4 -> 16
        assert_eq!(s.feed("a;\n").unwrap(), "16\n");
    }

    #[test]
    fn the_one_shot_eval_helper_matches_a_session() {
        let one = eval("2 + 3;\n").unwrap();
        let two = ReduceSession::new().feed("2 + 3;\n").unwrap();
        assert_eq!(one, two);
    }

    #[test]
    fn empty_and_whitespace_input_yields_nothing() {
        assert_eq!(eval("\n").unwrap(), "");
    }

    #[test]
    fn a_small_reduce_program_evaluates_end_to_end() {
        let mut s = ReduceSession::new();
        s.feed("h(x) := x - 2*x;\n").unwrap();
        let out = s.feed("h(5);\n").unwrap();
        assert_eq!(out, "-5\n");
    }

    #[test]
    fn group_statement_executes_side_effects_in_order() {
        // MA08 §5 gap (see the module/`lower` doc comments): the group's
        // OWN result stays the unevaluated CompoundExpression(1, 2) rather
        // than collapsing to just `2`, since the shared handler table has
        // no handler for that head -- but the ASSIGN side effect inside it
        // genuinely fires, in order, since arguments are still evaluated.
        let mut s = ReduceSession::new();
        let out = s.feed("<< a := 1; a + 1 >>;\n").unwrap();
        assert_eq!(out, "<< 1; 2 >>\n");
        assert_eq!(s.feed("a;\n").unwrap(), "1\n");
    }

    #[test]
    fn list_accessor_calls_do_not_crash_even_though_unwired() {
        // See the crate/`lower` doc comments' disclosed gap: no shared
        // handler exists yet, so this evaluates the argument and leaves the
        // call itself unevaluated -- it must not panic or hang.
        assert_eq!(eval("first({1, 2, 3});\n").unwrap(), "first({1, 2, 3})\n");
    }
}
