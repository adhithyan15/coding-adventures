//! # Maple runtime — lower Maple syntax to `symbolic-ir`, evaluate via `symbolic-vm`.
//!
//! This is the **MP-4** deliverable of the Maple-language lane (MA09 §2).
//! MP-1 through MP-3 gave us a spec, a lexer, and a parser; this crate is the
//! *runtime* that finally **evaluates** Maple source — by **reusing the
//! shared symbolic substrate** rather than writing a bespoke interpreter,
//! the same reuse story `derive-runtime`/`reduce-runtime` already
//! demonstrated for the other two Wave-5 CAS-family languages:
//!
//! ```text
//!   Maple source
//!        │
//!        ▼  coding_adventures_maple_parser::try_parse_maple  (MP-3)
//!   GrammarASTNode  (surface tree: statement, if_expr, assignment, arrow_def,
//!        │           logical_or, comparison, additive, postfix, …)
//!        ▼  crate::lower                                        (this crate)
//!   symbolic_ir::IRNode   (Add, Mul, Pow, Assign, Define, If, List, Set, …)
//!        │
//!        ▼  symbolic_vm::VM over SymbolicBackend, UNCHANGED
//!   symbolic_ir::IRNode   (evaluated)
//!        │
//!        ▼  crate::printer
//!   Maple surface string  (infix, [a,b,c], {a,b,c}, and/or/not, if...then...end if)
//! ```
//!
//! ## No custom `Backend` needed — verified against the real handler table
//!
//! Per MA09 §2/§5, MP-4 reuses [`symbolic_vm::SymbolicBackend`] *unchanged*
//! — no bespoke `Backend` the way `wolfram-runtime`/`macsyma-runtime` each
//! layer their own list/string built-ins onto. This claim was checked
//! directly against the source, not assumed from either crate's own spec
//! prose (the exact discipline MA08 §5 itself insists on, after disclosing
//! that its own *original* wording overclaimed a few heads as "already
//! implemented"): grepping `symbolic_vm::handlers::build_handler_table`
//! confirms handlers exist for `Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`,
//! `Equal`/`NotEqual`/`Less`/`Greater`/`LessEqual`/`GreaterEqual`,
//! `And`/`Or`/`Not`, `Assign`/`Define`/`If`, `List`, and — since
//! `SymbolicBackend::new` always builds the table with `simplify: true`
//! (`symbolic_vm::backends`) — `D`/`Integrate`. Grepping
//! `symbolic_vm::backend::BaseBackend::new` confirms the held-heads set is
//! exactly `{Assign, Define, If, Assume, Forget}` — every head this
//! subset's `Assign`/`Define`/`If` lowering relies on being held (so
//! `assign_handler`'s lhs, `define_handler`'s body, and `if_handler`'s
//! not-taken branch never pre-evaluate) is confirmed present, unchanged.
//!
//! ## `Set` — the one genuinely new head, and the one disclosed gap
//!
//! Maple is the first language in this repo with **two** distinct bracketed
//! aggregate literals — `[a, b, c]` (`List`, already fully handled) and
//! `{a, b, c}` (`Set`, MA09 §3/§5). There is **no** handler for a `Set` head
//! anywhere in the shared table (confirmed by the same grep above — no
//! language before Maple has asked for one), so [`lower::SET`] is defined
//! *locally to this crate* rather than added to `symbolic-ir`/`symbolic-vm`
//! — mirroring `reduce-runtime`'s identical treatment of its own new
//! `CompoundExpression`/`Cons`/list-accessor heads. `Set` is not a held
//! head, so its elements evaluate (any `Assign` inside one still fires), but
//! the call itself stays structurally correct-but-unevaluated: real Maple's
//! unordered/duplicate-removing set semantics are **not yet enforced** —
//! see [`lower`]'s own module doc comment for the full accounting. This is a
//! disclosed gap, not a silent bug, ready for a future item to close by
//! adding a real handler (to the shared table, or a narrowly-scoped Maple
//! `Backend`) with no lowering change required.
//!
//! `diff`/`int` are thin calls into the already-shared `D`/`Integrate`
//! handlers — the same ones Derive's `DIF`/`INT` and Wolfram's own
//! `D`/`Integrate` already call — so this crate adds no calculus code of
//! its own.
//!
//! ## Public contract
//!
//! [`MapleSession::feed`] is string-in / string-out. It evaluates a chunk of
//! source (one or more `;`/`:`-terminated statements) and returns the
//! surface rendering of each *displayed* result, one per line — MA09 §3's
//! own statement-separator row is explicit that `;` displays and `:`
//! suppresses ("a display flag on the surrounding session, not an IR node"),
//! so [`lower::Display::Suppress`]-tagged statements still evaluate (side
//! effects like `x := 5:` really do bind `x`) but contribute no output line.
//! Like `reduce-runtime`'s own `ReduceSession` (and unlike Derive's/
//! Wolfram's numbered worksheet convention), there is no `#n:`/`In[n]:=`
//! prefix — real Maple's own interactive session has no equivalent either
//! (MA09 §5). Bindings persist across calls, so an interactive `x := 5;`
//! then `x + 1;` works.
//!
//! ## Robustness at the trust boundary
//!
//! `feed` takes arbitrary user text, so — exactly as in
//! `reduce-runtime`/`derive-runtime`/`wolfram-runtime`/`maxima-runtime` — it
//! is the trust boundary for the whole reused stack. Two layered guards
//! close the two independent deep-recursion vectors (per the "depth caps
//! don't compose across boundaries" lesson: a cap on one recursive walk
//! does not protect a *different* recursive walk over the same or a derived
//! tree):
//!
//! 1. **Deeply *nested* source** (parenthesised, list/set-literal nesting,
//!    `not`/unary-minus prefix chains, a flat `^` chain, or nested
//!    `if`/`end if`/`fi`) — already rejected by `maple-parser`'s own
//!    `MAX_RULE_DEPTH`, before this crate's lowering/evaluation/printing
//!    recursion (which walks the *same* already-shallow tree the parser
//!    handed back) ever runs.
//! 2. **A long *flat* chain that folds into a deeply *nested* lowered
//!    tree.** `maple-parser`'s own `MAX_RULE_DEPTH` doc comment proves,
//!    with an independent measurement per shape, that its EBNF
//!    `{ x }`-repetition productions cost the parser *zero* native stack
//!    regardless of width — so `MAX_RULE_DEPTH` does not (and structurally
//!    cannot) bound a flat chain's length. Two shapes in this grammar fold a
//!    flat repetition into a nested `Apply` tree at *lowering* time, not
//!    parse time: [`lower::lower_binary_chain`] (`additive`/
//!    `multiplicative` — the same vector `reduce-runtime`/`derive-runtime`
//!    already guard) and, genuinely new to this grammar (no `reduce.grammar`
//!    analogue — REDUCE has no `elif`), [`lower::lower_if`]'s own
//!    right-to-left `elif`-chain fold. [`MAX_STATEMENT_TOKENS`], measured
//!    against the **real** lexer token stream (so there is no separately-
//!    maintained lexical model to diverge from and bypass), closes this
//!    vector.
//!
//!    Unlike `reduce-runtime`'s identical-in-spirit guard — which must track
//!    bracket *nesting depth* and only reset on a `;`/`$` at depth 0,
//!    because REDUCE's `<< s1; s2; ... >>` group statement can embed a `;`
//!    lexically *inside* a parenthesised construct that is itself just one
//!    operand of a much larger enclosing chain — this crate's reset is
//!    unconditional on every `SEMI`/`COLON` token. This is a genuine
//!    grammar-shape difference, verified directly against
//!    `maple-parser`'s own compiled grammar (`grep -n 'SEMI\|COLON'
//!    coding-adventures-maple-parser/src/_grammar.rs`), not assumed by
//!    resemblance to Reduce: `SEMI`/`COLON` are referenced in exactly ONE
//!    place in the entire grammar — `statement_line`'s own terminator
//!    alternation — so every `SEMI`/`COLON` token in a valid parse is
//!    unambiguously a genuine top-level statement boundary; there is no
//!    Maple construct (this subset has no bare compound-statement grouping
//!    the way REDUCE's `<< ... >>` is — MA09 §4 defers bare expression
//!    sequences entirely) that could ever embed one lexically inside a
//!    nested operand.
//! 3. **Unwinding panics** (e.g. a wrong-arity `diff(x)`/`int(f)` call
//!    reaching `derivative_handler`/`integrate_handler`'s own arity panic —
//!    Maple's grammar-enforced bare-`NAME` `Assign`/`Define` left-hand side
//!    means, unlike Reduce's, there is no malformed-`Assign`-lhs panic
//!    vector at all here). Evaluation runs inside [`catch_unwind`] on a
//!    worker thread with a large bounded stack, and any panic becomes a
//!    clean `Err(String)`; the session is rebuilt afterward (trading lost
//!    bindings for a guaranteed-usable session next call), so one crafted
//!    statement can never abort the process or wedge a session.
//!
//! ## Out-of-scope constructs (MA09 §4) are rejected at PARSE time, not here
//!
//! `proc(...) ... end proc` (block-structured procedures) and `for`/`while`
//! loops are not represented anywhere in `maple.grammar` at all — `proc`,
//! `for`, `while`, `do` are ordinary `NAME` tokens (not reserved words in
//! `maple.tokens`), so `proc(x) x^2 end proc` parses only as far as the
//! ordinary call `proc(x)` (ordinary function application, ambiguous with a
//! ordinary function named `proc`), after which the leftover `x^2 end proc`
//! has nowhere left to attach and `statement_line`'s own terminator check
//! fails cleanly; similarly `for i from 1 to 10 do ... end do` parses only
//! as far as the bare symbol `for`, after which `i` has nowhere to attach.
//! Both therefore surface as an ordinary `try_parse_maple` `Err`, which this
//! crate's own `eval_source` forwards as-is — no special-casing needed in
//! this crate at all, exactly like `reduce-runtime`'s identical "a parse
//! error is returned, not panicked" contract for its own out-of-scope
//! constructs. See this crate's own test suite for confirmation that this
//! is a genuine parse-time rejection, not silent mis-evaluation.

mod lower;
mod printer;

pub use lower::{Display, LowerError, LoweredStatement, SET};
pub use printer::print_maple;

use coding_adventures_maple_lexer::try_tokenize_maple;
use coding_adventures_maple_parser::try_parse_maple;
use lower::lower_program;
use std::panic::{catch_unwind, AssertUnwindSafe};
use symbolic_vm::{SymbolicBackend, VM};

/// Maximum length, in bytes, of a single source chunk handed to
/// [`MapleSession::feed`].
///
/// A cheap first gate bounding per-call memory/time. 64 KiB is far beyond
/// any realistic interactive submission.
pub const MAX_INPUT_LEN: usize = 64 * 1024;

/// Maximum number of lexer tokens allowed in any single statement (a run of
/// tokens between two `SEMI`/`COLON` separators, or between a separator and
/// the start/end of input).
///
/// See the crate doc comment's "Robustness" point 2 — this closes the
/// long-flat-chain vector `maple-parser`'s own `MAX_RULE_DEPTH` does not
/// (and cannot) cover. 2000 tokens in one statement is already absurd for a
/// hand-written Maple program line, matching `reduce-runtime`'s/
/// `derive-runtime`'s identical choice.
pub const MAX_STATEMENT_TOKENS: usize = 2000;

/// Stack size of the worker thread that runs evaluation and printing.
///
/// With the per-statement complexity cap bounding tree depth, this is
/// generous headroom so the bounded-but-still-deep trees are built,
/// evaluated, printed, and dropped well clear of any overflow, regardless of
/// the caller's own stack.
const EVAL_STACK_SIZE: usize = 512 * 1024 * 1024;

/// A persistent Maple session.
///
/// Owns the [`VM`] (and through it the [`SymbolicBackend`] environment), so
/// variable bindings (`x := 5`) and user-defined functions (`f := x -> x*x`)
/// persist across calls to [`feed`](MapleSession::feed), exactly as in an
/// interactive Maple session.
pub struct MapleSession {
    vm: VM,
}

impl Default for MapleSession {
    fn default() -> Self {
        Self::new()
    }
}

/// One displayed result line from a [`MapleSession::feed`] call.
///
/// Only produced for `;`-terminated ([`Display::Show`]) statements — MA09
/// §3's own statement-separator row (`:` suppresses). Maple's own session
/// transcript has no numbered-input convention either (MA09 §2/§5), so —
/// like `reduce-runtime::Output` and unlike `derive-runtime::Output`'s `#n`
/// index — this carries only the rendered text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// The result rendered in Maple surface notation.
    pub text: String,
}

impl MapleSession {
    /// Create a fresh session with an empty environment.
    pub fn new() -> Self {
        MapleSession {
            vm: VM::new(Box::new(SymbolicBackend::new())),
        }
    }

    /// Evaluate a chunk of Maple source and return the concatenated echo.
    ///
    /// Every *displayed* statement (`;`-terminated, or the optional final
    /// terminator-less statement) produces one plain output line (no
    /// numbering — see the module/[`Output`] doc comments); `:`-suppressed
    /// statements still evaluate (so their side effects persist) but
    /// contribute no line. Lines are concatenated in evaluation order, each
    /// ending in a newline. A parse or lowering error is returned as `Err`
    /// with the message.
    ///
    /// # Example
    ///
    /// ```
    /// use coding_adventures_maple_runtime::MapleSession;
    /// let mut s = MapleSession::new();
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

    /// Evaluate `src` and return the structured list of *displayed*
    /// [`Output`]s (`:`-suppressed statements still evaluate for their side
    /// effects, but contribute no `Output`).
    ///
    /// The lower-level cousin of [`feed`](MapleSession::feed) — a REPL that
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
        // Guard 2 + panics: reject a long flat chain (arithmetic OR an elif
        // chain) before it can fold into a stack-overflowing lowered tree
        // (see the crate "Robustness" doc, point 2 — deeply *nested* source
        // is already rejected by `maple-parser`'s own `MAX_RULE_DEPTH`, a
        // *different* vector this guard does not need to re-cover). This
        // check runs INSIDE the worker thread's `catch_unwind`, not on the
        // caller thread — `/security-review` flagged an earlier draft that
        // ran it before `std::thread::scope`, which would have let a
        // hypothetical panic inside the tokenizer it calls unwind straight
        // through `feed`/the REPL's `run` loop to `main`, contradicting this
        // crate's own "one crafted statement can never abort the process"
        // guarantee. Guard 3 + panics: the lowering, the VM evaluation, and
        // the printer all run on the SAME worker thread with a large bounded
        // stack, and any unwinding panic from the reused symbolic stack is
        // caught — mirroring `reduce-runtime`'s identical worker-thread
        // design (including its own `/security-review` fix: a thread-spawn
        // failure is folded into the ordinary `Err` path, never a
        // caller-thread panic).
        let vm = &mut self.vm;
        let src_owned = src.to_string();
        let mut panicked = false;
        let result: Result<Vec<Output>, String> = std::thread::scope(|scope| {
            let handle = match std::thread::Builder::new()
                .stack_size(EVAL_STACK_SIZE)
                .spawn_scoped(scope, || {
                    catch_unwind(AssertUnwindSafe(|| {
                        check_statement_token_counts(&src_owned)?;
                        eval_source(vm, &src_owned)
                    }))
                }) {
                Ok(handle) => handle,
                Err(io_err) => {
                    return Err(format!(
                        "failed to spawn the Maple evaluation thread: {io_err}"
                    ));
                }
            };
            match handle.join() {
                Ok(Ok(Ok(outputs))) => Ok(outputs),
                Ok(Ok(Err(message))) => Err(message),
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

/// Evaluate every statement in `src`, returning the *displayed* outputs.
/// Runs on the worker thread.
fn eval_source(vm: &mut VM, src: &str) -> Result<Vec<Output>, String> {
    // The parser's error string already carries a user-readable message
    // (this is also where MA09 §4's out-of-scope constructs — `proc`,
    // `for`/`while` — fail: see the crate doc comment's own section on
    // this), so we forward it as-is.
    let ast = try_parse_maple(src)?;
    let statements = lower_program(&ast).map_err(|e| e.to_string())?;

    let mut outputs = Vec::new();
    for stmt in statements {
        let result = vm.eval(stmt.node);
        if matches!(stmt.display, lower::Display::Show) {
            outputs.push(Output {
                text: print_maple(&result),
            });
        }
    }
    Ok(outputs)
}

/// Reject input where any single statement lexes to too many tokens.
///
/// The count is taken from the **real** Maple lexer, the very one the
/// parser consumes, so there is no separately-maintained lexical model to
/// diverge from: a `SEMI`/`COLON` token (`;`/`:`) marks a statement boundary
/// and resets the running count.
///
/// Unlike `reduce-runtime::check_statement_token_counts`, this reset is
/// **unconditional** — no bracket-nesting depth needs to be tracked. See the
/// crate doc comment's "Robustness" point 2 for the full, grep-verified
/// argument: `SEMI`/`COLON` appear in exactly one place in the entire
/// `maple.grammar` (`statement_line`'s own terminator), so every occurrence
/// in a valid token stream is unambiguously a genuine top-level statement
/// boundary — there is no Maple construct that could embed one lexically
/// inside a nested operand the way REDUCE's `<< ... >>` group statement
/// could.
///
/// If the lexer itself errors (an untokenizable character), we return
/// `Ok(())` and let the parser surface the error uniformly — we never
/// reject solely because the *checker* could not lex something.
fn check_statement_token_counts(src: &str) -> Result<(), String> {
    let tokens = match try_tokenize_maple(src) {
        Ok(tokens) => tokens,
        Err(_) => return Ok(()), // unlexable — let the parser surface it
    };

    let mut count: usize = 0;
    for token in &tokens {
        match token.effective_type_name() {
            "SEMI" | "COLON" => {
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

/// Evaluate `src` once on a fresh [`MapleSession`] and return its echo.
///
/// Convenience for callers that do not need persistent state.
pub fn eval(src: &str) -> Result<String, String> {
    MapleSession::new().feed(src)
}

/// Recover a human-readable message from a caught panic payload.
fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Maple could not evaluate that input".to_string()
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
    }

    #[test]
    fn free_symbols_stay_symbolic() {
        assert_eq!(eval("x + 0;\n").unwrap(), "x\n");
    }

    #[test]
    fn assignment_persists_across_calls() {
        let mut s = MapleSession::new();
        assert_eq!(s.feed("x := 5;\n").unwrap(), "5\n");
        assert_eq!(s.feed("x + 1;\n").unwrap(), "6\n");
    }

    #[test]
    fn user_defined_arrow_function_end_to_end() {
        let mut s = MapleSession::new();
        s.feed("f := x -> x*x;\n").unwrap();
        assert_eq!(s.feed("f(5);\n").unwrap(), "25\n");
    }

    #[test]
    fn multi_parameter_arrow_function_end_to_end() {
        let mut s = MapleSession::new();
        s.feed("g := (x, y) -> x + y;\n").unwrap();
        assert_eq!(s.feed("g(2, 3);\n").unwrap(), "5\n");
    }

    // --- Display suppression: `;` shows, `:` suppresses (MA09 §3) ----------

    #[test]
    fn colon_suppresses_output_but_side_effect_still_happens() {
        let mut s = MapleSession::new();
        assert_eq!(s.feed("x := 5:\n").unwrap(), "");
        assert_eq!(s.feed("x + 1;\n").unwrap(), "6\n");
    }

    #[test]
    fn semicolon_displays_output() {
        assert_eq!(eval("x := 5;\n").unwrap(), "5\n");
    }

    #[test]
    fn mixed_terminators_in_one_feed_only_display_the_semicolon_lines() {
        let out = eval("1: 2; 3:\n").unwrap();
        assert_eq!(out, "2\n");
    }

    // --- `if`/`elif`/`else`/`end if` (MA09 §3) -------------------------------

    #[test]
    fn if_selects_a_branch_via_the_held_handler() {
        assert_eq!(eval("if 1 > 0 then 42 else 0 end if;\n").unwrap(), "42\n");
        assert_eq!(eval("if 0 > 1 then 42 else 7 end if;\n").unwrap(), "7\n");
    }

    #[test]
    fn if_with_no_else_returns_false_on_a_failing_condition() {
        assert_eq!(eval("if 0 > 1 then 42 end if;\n").unwrap(), "false\n");
    }

    #[test]
    fn fi_closing_spelling_evaluates_identically_to_end_if() {
        assert_eq!(
            eval("if 1 > 0 then 42 else 0 fi;\n").unwrap(),
            eval("if 1 > 0 then 42 else 0 end if;\n").unwrap()
        );
    }

    #[test]
    fn elif_chain_evaluates_the_first_true_branch() {
        assert_eq!(
            eval("if false then 1 elif true then 2 else 3 end if;\n").unwrap(),
            "2\n"
        );
    }

    #[test]
    fn if_unresolved_on_a_free_variable_prints_back_as_if_syntax() {
        assert_eq!(
            eval("if x > 0 then 1 else -1 end if;\n").unwrap(),
            "if x > 0 then 1 else -1 end if\n"
        );
    }

    // --- Lists / sets (MA09 §3/§5) --------------------------------------------

    #[test]
    fn list_literal_evaluates_elementwise() {
        assert_eq!(eval("[1 + 1, 2*3, 2^3];\n").unwrap(), "[2, 6, 8]\n");
    }

    #[test]
    fn list_assignment_persists_across_calls() {
        let mut s = MapleSession::new();
        assert_eq!(s.feed("v := [1, 2, 3];\n").unwrap(), "[1, 2, 3]\n");
        assert_eq!(s.feed("v;\n").unwrap(), "[1, 2, 3]\n");
    }

    #[test]
    fn set_literal_evaluates_its_elements_but_stays_structurally_unresolved() {
        // Disclosed gap (crate/`lower` doc comments): no shared `Set`
        // handler exists yet, so this must not crash or hang, and
        // duplicates are NOT removed (real Maple's dedup semantics aren't
        // enforced at evaluation time in this subset).
        assert_eq!(eval("{1 + 1, 2*3};\n").unwrap(), "{2, 6}\n");
        assert_eq!(eval("{1, 1, 2};\n").unwrap(), "{1, 1, 2}\n");
    }

    #[test]
    fn set_assignment_persists_across_calls() {
        let mut s = MapleSession::new();
        assert_eq!(s.feed("v := {1, 2, 3};\n").unwrap(), "{1, 2, 3}\n");
        assert_eq!(s.feed("v;\n").unwrap(), "{1, 2, 3}\n");
    }

    // --- Booleans / logic (MA09 §3) -------------------------------------------

    #[test]
    fn boolean_literals_evaluate() {
        assert_eq!(eval("true;\n").unwrap(), "true\n");
        assert_eq!(eval("false;\n").unwrap(), "false\n");
    }

    #[test]
    fn boolean_logic_evaluates() {
        assert_eq!(eval("1 > 0 and 2 > 1;\n").unwrap(), "true\n");
        assert_eq!(eval("not (1 > 2);\n").unwrap(), "true\n");
    }

    #[test]
    fn not_equal_operator_evaluates() {
        assert_eq!(eval("1 <> 2;\n").unwrap(), "true\n");
        assert_eq!(eval("1 <> 1;\n").unwrap(), "false\n");
    }

    // --- diff/int bridge to the shared D/Integrate handlers (MA09 §2/§5) ----

    #[test]
    fn diff_evaluates_via_the_shared_derivative_handler() {
        assert_eq!(eval("diff(x^2, x);\n").unwrap(), "2*x\n");
    }

    #[test]
    fn int_evaluates_via_the_shared_integrate_handler() {
        assert_eq!(eval("int(x, x);\n").unwrap(), "1/2*x^2\n");
    }

    // --- Explicit `*` requirement (MA09 §3/§4) --------------------------------

    #[test]
    fn juxtaposition_without_an_operator_is_a_parse_error() {
        assert!(eval("a b;\n").is_err());
    }

    // --- Out-of-scope constructs are rejected at parse time (MA09 §4) -------

    #[test]
    fn proc_block_structured_procedures_are_rejected() {
        // `proc` is an ordinary NAME (no grammar production at all for
        // block-structured procedures) -- `proc(x) x^2 end proc` parses
        // only as far as the call `proc(x)`, then fails to find a
        // SEMI/COLON terminator before `x^2`.
        assert!(eval("proc(x) x^2 end proc;\n").is_err());
        assert!(eval("proc() 1 end proc;\n").is_err());
    }

    #[test]
    fn for_loops_are_rejected() {
        assert!(eval("for i from 1 to 10 do i end do;\n").is_err());
    }

    #[test]
    fn while_loops_are_rejected() {
        assert!(eval("while x > 0 do x := x - 1 end do;\n").is_err());
    }

    #[test]
    fn remember_table_spelling_is_rejected() {
        // MA09 §1/§4: `f(x) := expr` is real Maple's narrower remember-table
        // mechanism, deliberately excluded from this subset -- confirmed
        // all the way through this crate's own `eval`, not just at the
        // parser layer.
        assert!(eval("f(x) := 1;\n").is_err());
    }

    // --- Robustness ------------------------------------------------------------

    #[test]
    fn a_parse_error_is_returned_not_panicked() {
        let err = eval("1 +\n");
        assert!(err.is_err(), "expected a clean Err, got {err:?}");
    }

    #[test]
    fn session_recovers_after_a_parse_error() {
        let mut s = MapleSession::new();
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
        // maple-parser's own MAX_RULE_DEPTH rejects this before the runtime
        // ever sees a tree -- this test proves the session surfaces that as
        // a clean Err (not a crash), not that this crate re-implements the
        // cap.
        let depth = 5000;
        let src = format!("{}1{};\n", "(".repeat(depth), ")".repeat(depth));
        assert!(src.len() <= MAX_INPUT_LEN, "stays under the length cap");
        assert!(eval(&src).is_err());
    }

    #[test]
    fn long_flat_additive_chain_is_rejected_by_the_statement_token_cap() {
        // additive/multiplicative are grammar REPETITIONS, not recursive
        // rule calls, so maple-parser's MAX_RULE_DEPTH does not bound
        // chain length -- this is the vector MAX_STATEMENT_TOKENS closes.
        let src = format!("{}1;\n", "1+".repeat(5_000));
        assert!(src.len() <= MAX_INPUT_LEN);
        assert!(eval(&src).unwrap_err().contains("too complex"));
    }

    #[test]
    fn long_elif_chain_is_rejected_by_the_statement_token_cap() {
        // Genuinely new relative to reduce-runtime/derive-runtime: an
        // `elif` chain is ALSO a grammar repetition (not recursion), and
        // `lower_if` folds it into a deeply nested `If(...)` tree, exactly
        // the same shape of vector as the additive-chain case above.
        let mut src = String::from("if false then 0\n");
        for _ in 0..600 {
            src.push_str("elif false then 0\n");
        }
        src.push_str("else 1 end if;\n");
        assert!(
            src.len() <= MAX_INPUT_LEN,
            "the PoC must fit under the input-size cap on its own, so only \
             the statement-token-count guard is what's actually being tested"
        );
        assert!(eval(&src).unwrap_err().contains("too complex"));
    }

    #[test]
    fn moderate_chain_still_evaluates() {
        let src = format!("{}1;\n", "1+".repeat(50));
        assert_eq!(eval(&src).unwrap(), "51\n");
    }

    #[test]
    fn a_wrong_arity_diff_call_is_caught_not_aborted() {
        // `diff(x)` lowers to `D(x)` (1 arg) -- the reused
        // `derivative_handler` *panics* on the wrong arity ("D expects 2
        // arguments, got 1"). The worker-thread `catch_unwind` must convert
        // that panic into a clean `Err`, and the session must remain usable
        // afterward (the env is rebuilt).
        let mut s = MapleSession::new();
        assert!(
            s.feed("diff(x);\n").is_err(),
            "a wrong-arity diff call must return Err, never abort"
        );
        assert_eq!(s.feed("2 + 2;\n").unwrap(), "4\n");
    }

    #[test]
    fn the_one_shot_eval_helper_matches_a_session() {
        let one = eval("2 + 3;\n").unwrap();
        let two = MapleSession::new().feed("2 + 3;\n").unwrap();
        assert_eq!(one, two);
    }

    #[test]
    fn empty_and_whitespace_input_yields_nothing() {
        assert_eq!(eval("\n").unwrap(), "");
    }

    #[test]
    fn a_small_maple_program_evaluates_end_to_end() {
        let mut s = MapleSession::new();
        s.feed("h := x -> x - 2*x;\n").unwrap();
        let out = s.feed("h(5);\n").unwrap();
        assert_eq!(out, "-5\n");
    }
}
