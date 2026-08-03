//! # Wolfram runtime — lower M-expressions to `symbolic-ir`, evaluate via `symbolic-vm`.
//!
//! This is the **W-4** deliverable of the Wolfram-language lane (MA04 §7). The
//! W-1/W-2/W-3 items gave us a spec, a lexer, and a parser; this crate is the
//! *runtime* that finally **evaluates** Wolfram source — and it does so by
//! **reusing the shared symbolic substrate** rather than writing a bespoke
//! interpreter:
//!
//! ```text
//!   Wolfram source
//!        │
//!        ▼  coding_adventures_wolfram_parser::parse  (W-3)
//!   GrammarASTNode  (surface tree: additive, power, postfix, list, …)
//!        │
//!        ▼  crate::lower                              (this crate)
//!   symbolic_ir::IRNode   (Add, Mul, Pow, List, Rule, …)
//!        │
//!        ├─ ReplaceAll? ─► builtins::replace_all_once  (single top-down pass,
//!        │                  on cas_pattern_matching::match_pattern + substitute)
//!        ▼  symbolic_vm::VM over WolframBackend (decorates SymbolicBackend)
//!   symbolic_ir::IRNode   (evaluated)
//!        │
//!        ▼  crate::printer
//!   Wolfram surface string  (infix, f[…], {…})
//! ```
//!
//! "Everything is `head[args]`" (MA04 §1) is what makes this a *lowering* and not
//! a translation: `2 + 3` is `Plus[2, 3]` is `Add(2, 3)`, which the
//! [`SymbolicBackend`] already folds to `5`. The whole rewrite engine — numeric
//! folding, algebraic identities, the elementary-function handlers, user-defined
//! functions — is the *same* table Macsyma drives, reached through
//! `symbolic_vm::handlers::build_handler_table`. W-5's list/functional/numeric
//! built-ins (`Length`, `Map`, `Range`, …) are layered on top by
//! [`WolframBackend`], a decorator that adds those heads and delegates the rest
//! to the shared `SymbolicBackend` (MA04 §8).
//!
//! ## Public contract
//!
//! [`WolframSession::feed`] is string-in / string-out, like the Maxima/Octave
//! facades. It evaluates a chunk of source (one or more statements) and returns
//! the surface rendering of each *displayed* result. A `;` at the end of a line
//! suppresses that line's display (the notebook convention) but the statement
//! still runs and still advances the `Out[n]` counter. Bindings persist across
//! calls, so an interactive `x = 5` then `x + 1` works.
//!
//! ## Robustness at the trust boundary
//!
//! `feed` takes arbitrary user text, so — exactly as in `maxima-runtime` — it is
//! the trust boundary for the whole reused stack. The same three failure modes
//! are contained here so one crafted input can never crash or wedge a session:
//!
//! 1. **Unbounded recursion → stack overflow.** The grammar parser, the lowering,
//!    and the VM all recurse on expression nesting with no intrinsic depth limit,
//!    so deeply nested input (`((((…))))`, a long `------…x`, `1+1+1+…`) builds a
//!    correspondingly deep tree that overflows the stack while it is *built*, and
//!    again when it is *dropped*. A stack overflow is **not** a catchable panic —
//!    it aborts the process — so `catch_unwind` cannot help. Two layered caps
//!    close it: [`MAX_INPUT_LEN`] bounds total input, and a per-statement token
//!    cap ([`MAX_STATEMENT_TOKENS`]), measured against the **real** lexer token
//!    stream, bounds the tree depth (every parse-tree node consumes ≥1 token), so
//!    no stack-overflowing tree is ever constructed. The lexer is *iterative*, so
//!    it cannot itself overflow on deep nesting.
//! 2. **Unwinding panics.** Reused handlers (and the lowering's structural
//!    asserts) can panic on a surprising shape. Evaluation runs inside
//!    [`catch_unwind`] on a worker thread with a large bounded stack, and any
//!    panic becomes a clean `Err(String)`.
//! 3. **A poisoned session after a caught panic.** A panic could leave the
//!    backend env half-updated, so after catching one we **rebuild the session**,
//!    trading the lost bindings for a guaranteed-usable session next call.

mod backend;
mod builtins;
mod lower;
mod printer;

pub use backend::WolframBackend;
pub use builtins::{MAX_LIST_LENGTH, MAX_RANGE_LENGTH, MAX_STRING_LENGTH};
pub use lower::{LowerError, REPLACE_ALL};
pub use printer::print_wolfram;

use builtins::{collect_rule_list, replace_all_once};
use coding_adventures_wolfram_lexer::try_tokenize_wolfram;
use coding_adventures_wolfram_parser::try_parse_wolfram;
use lower::lower_program;
use std::panic::{catch_unwind, AssertUnwindSafe};
use symbolic_ir::{IRApply, IRNode};
use symbolic_vm::VM;

/// Maximum length, in bytes, of a single source chunk handed to [`WolframSession::feed`].
///
/// A cheap first gate bounding per-call memory/time. 64 KiB is far beyond any
/// realistic interactive submission.
pub const MAX_INPUT_LEN: usize = 64 * 1024;

/// Maximum number of lexer tokens allowed in any single top-level statement.
///
/// A statement's parse-tree depth is bounded above by its token count (every
/// node of the tree consumes at least one token), so capping tokens per statement
/// caps the recursion depth of the parser, the lowering, the VM, and the later
/// `Drop` of the tree — closing the stack-overflow-on-deep-nesting vector. The
/// count comes from the **real** Wolfram lexer (see [`check_statement_token_counts`]),
/// so there is no separately-maintained surface model to diverge from and bypass.
/// 2000 tokens in one statement is already absurd for human-written Wolfram.
pub const MAX_STATEMENT_TOKENS: usize = 2000;

/// Stack size of the worker thread that runs evaluation and printing.
///
/// With the per-statement complexity cap bounding tree depth, this is generous
/// headroom so the bounded-but-still-deep trees are built, evaluated, printed,
/// and dropped well clear of any overflow, regardless of the caller's own stack.
const EVAL_STACK_SIZE: usize = 512 * 1024 * 1024;

/// A persistent Wolfram session.
///
/// Owns the [`VM`] (and through it the [`WolframBackend`] environment), so
/// variable bindings (`x = 5`) and user-defined functions (`f[x_] := x^2`)
/// persist across calls to [`feed`](WolframSession::feed), exactly as in an
/// interactive Wolfram kernel. The `Out[n]` counter likewise persists.
pub struct WolframSession {
    vm: VM,
    /// 1-based counter of *displayed* results so far — the `Out[n]` index.
    output_index: usize,
}

impl Default for WolframSession {
    fn default() -> Self {
        Self::new()
    }
}

/// One displayed result line from a [`WolframSession::feed`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// The 1-based `Out[n]` index.
    pub index: usize,
    /// The result rendered in Wolfram surface notation.
    pub text: String,
}

impl WolframSession {
    /// Create a fresh session with an empty environment and `Out` counter.
    pub fn new() -> Self {
        WolframSession {
            vm: VM::new(Box::new(WolframBackend::new())),
            output_index: 0,
        }
    }

    /// Evaluate a chunk of Wolfram source and return the concatenated echo.
    ///
    /// Each *displayed* statement (a line not suffixed with `;`) produces one
    /// `Out[n]= «value»` line; the lines are concatenated in evaluation order,
    /// each ending in a newline. A `;`-suppressed statement still runs and still
    /// advances the `Out` counter but contributes no line. A parse or lowering
    /// error is returned as `Err` with the message.
    ///
    /// # Example
    ///
    /// ```
    /// use coding_adventures_wolfram_runtime::WolframSession;
    /// let mut s = WolframSession::new();
    /// assert_eq!(s.feed("1 + 2*3\n").unwrap(), "Out[1]= 7\n");
    /// ```
    pub fn feed(&mut self, src: &str) -> Result<String, String> {
        let outputs = self.eval_to_outputs(src)?;
        let mut out = String::new();
        for o in outputs {
            out.push_str(&format!("Out[{}]= {}\n", o.index, o.text));
        }
        Ok(out)
    }

    /// Evaluate `src` and return the structured list of displayed [`Output`]s.
    ///
    /// The lower-level cousin of [`feed`](WolframSession::feed) — a REPL that
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
        // Guard 2: reject any over-complex statement so no stack-overflowing tree
        // is ever built (see the module "Robustness" note and the const docs).
        check_statement_token_counts(src)?;

        // Guard 3 + Guard (panics): the recursive parser, the lowering, the
        // ReplaceAll pre-pass, the VM evaluation, and the printer all run on a
        // worker thread with a large bounded stack — so the bounded-but-still-deep
        // trees (capped by Guard 2) are built, walked, and dropped clear of the
        // caller's own (possibly small) stack — and any unwinding panic from the
        // reused symbolic stack is caught. Only the small result (Vec<Output>) or
        // an error message crosses back. (Guards 1 and 2 above already ran on the
        // caller thread; the iterative lexer they use cannot itself overflow.)
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
                .expect("failed to spawn wolfram evaluation thread")
                .join()
        });

        match outcome {
            // Normal path: the worker produced the outputs (or a surface error).
            Ok(Ok(Ok((outputs, next_index)))) => {
                self.output_index = next_index;
                Ok(outputs)
            }
            Ok(Ok(Err(message))) => Err(message),
            // A panic the worker caught, or one that escaped and unwound the join.
            // Either way the env may be inconsistent, so rebuild the session.
            Ok(Err(payload)) | Err(payload) => {
                self.vm = VM::new(Box::new(WolframBackend::new()));
                self.output_index = 0;
                Err(panic_message(payload))
            }
        }
    }
}

/// Evaluate every statement in `src`, returning the displayed outputs and the new
/// `Out` counter. Runs on the worker thread.
fn eval_source(vm: &mut VM, src: &str, start_index: usize) -> Result<(Vec<Output>, usize), String> {
    // The parser's error string already carries a user-readable `line:col:
    // message` position, so we forward it as-is.
    let ast = try_parse_wolfram(src)?;
    // Pair each lowered statement with whether its source line displays.
    let displays = statement_display_flags(src);
    let statements = lower_program(&ast).map_err(|e| e.to_string())?;

    let mut outputs = Vec::new();
    let mut index = start_index;
    for (i, stmt) in statements.into_iter().enumerate() {
        // Pre-pass: rewrite any `ReplaceAll` applications via the pattern matcher
        // (the VM has no ReplaceAll handler), then evaluate the rest with the VM.
        let prepared = apply_replace_all(stmt)?;
        let result = vm.eval(prepared);
        index += 1;
        // Default to display when we cannot determine the flag (more statements
        // than detected lines should not happen, but fail open to *showing*).
        let display = displays.get(i).copied().unwrap_or(true);
        if display {
            outputs.push(Output {
                index,
                text: print_wolfram(&result),
            });
        }
    }
    Ok((outputs, index))
}

/// Recursively replace every `ReplaceAll(expr, rules)` node with the result of a
/// **single top-down leftmost-outermost pass** of `rules` over `expr` (MA04
/// §21.3, via [`builtins::replace_all_once`]). `rules` may be a single
/// `Rule`/`RuleDelayed` or a `List` of them. Inner `ReplaceAll`s (a `/.` chain
/// lowers left-associative, so the *expr* side may itself be a ReplaceAll) are
/// handled by recursing into the children first.
///
/// `ReplaceAll` is *not* a VM handler — the VM has no `ReplaceAll` head — so this
/// pre-pass rewrites it at the IR level *before* evaluation; the substituted
/// result is then evaluated normally (so `g[1,2] /. g[a_,b_] -> a+b` folds to
/// `3`). The W-19 single-pass discipline replaced the prior `cas-pattern-matching`
/// fixed-point `rewrite`, which looped forever on rules like `x_Integer -> x^2`
/// (it kept re-matching the `Integer` result); the single pass yields `{1,4,9}`
/// and stops. Returning `Result` keeps the signature stable, though the bounded
/// single pass can no longer fail to converge.
fn apply_replace_all(node: IRNode) -> Result<IRNode, String> {
    match node {
        IRNode::Apply(app) => {
            // Recurse into head and args first (bottom-up), so a nested
            // `(x /. r1) /. r2` resolves the inner `/.` before the outer.
            let head = apply_replace_all(app.head)?;
            let args = app
                .args
                .into_iter()
                .map(apply_replace_all)
                .collect::<Result<Vec<_>, _>>()?;

            if let IRNode::Symbol(name) = &head {
                if name == REPLACE_ALL && args.len() == 2 {
                    let rules = collect_rule_list(&args[1]);
                    return Ok(replace_all_once(&args[0], &rules, 0));
                }
            }
            Ok(IRNode::Apply(Box::new(IRApply { head, args })))
        }
        other => Ok(other),
    }
}

/// Determine, per top-level statement, whether its result should be displayed.
///
/// A line whose statement is suffixed with `;` (outside brackets/strings) is
/// suppressed. We scan the *real* source rather than the parse tree because the
/// lowering discards the terminator. We track bracket depth and string/comment
/// state so a `;` inside `f[a; b]`-style nesting or a `"a;b"` string is not
/// mistaken for a statement-suppressing terminator. Each maximal run of source
/// that ends a statement (a top-level NEWLINE or `;`) yields one flag, in order,
/// to line up with `lower_program`'s statement list.
fn statement_display_flags(src: &str) -> Vec<bool> {
    let mut flags = Vec::new();
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut in_comment = false;
    let mut escaped = false;
    // Whether the current statement has any non-whitespace content yet.
    let mut has_content = false;

    let mut chars = src.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_comment {
            // Non-nesting `(* … *)`.
            if ch == '*' && chars.peek() == Some(&')') {
                chars.next();
                in_comment = false;
            }
            continue;
        }
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
            '(' if chars.peek() == Some(&'*') => {
                chars.next();
                in_comment = true;
            }
            '"' => {
                in_string = true;
                has_content = true;
            }
            '[' | '{' | '(' => {
                depth += 1;
                has_content = true;
            }
            ']' | '}' | ')' => {
                depth -= 1;
                has_content = true;
            }
            ';' if depth <= 0 => {
                // A top-level `;` ends a statement and *suppresses* it.
                if has_content {
                    flags.push(false);
                }
                has_content = false;
            }
            '\n' if depth <= 0 => {
                // A top-level newline ends a statement and *displays* it.
                if has_content {
                    flags.push(true);
                }
                has_content = false;
            }
            c if c.is_whitespace() => {}
            _ => has_content = true,
        }
    }
    // A trailing statement with no final newline still counts (displayed).
    if has_content {
        flags.push(true);
    }
    flags
}

/// Reject input where any single statement lexes to too many tokens.
///
/// The count is taken from the **real** Wolfram lexer, the very one the parser
/// consumes, so there is no separately-maintained lexical model to diverge from:
/// comments and whitespace are skip patterns (absent from the token stream),
/// strings are single tokens, and a top-level `NEWLINE` token marks a statement
/// boundary (the lexer drops newlines inside brackets, so a bracketed multi-line
/// form is correctly counted as one statement). A statement's parse-tree depth is
/// at most its token count, so capping tokens per statement caps the depth.
///
/// If the lexer itself errors (an untokenizable character), we return `Ok(())`
/// and let the parser surface the error uniformly — we never reject solely
/// because the *checker* could not lex something.
fn check_statement_token_counts(src: &str) -> Result<(), String> {
    let tokens = match try_tokenize_wolfram(src) {
        Ok(tokens) => tokens,
        Err(_) => return Ok(()), // unlexable — let the parser surface it
    };

    let mut count: usize = 0;
    for token in &tokens {
        // Reset on a top-level statement boundary. Match on the token *type*
        // (`NEWLINE` / `SEMI`), never on its lexeme: a STRING literal containing
        // a `;` has its quotes intact in the value but type STRING, so keying on
        // type cannot be fooled by string content.
        match token.effective_type_name() {
            "NEWLINE" | "SEMI" => {
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

/// Evaluate `src` once on a fresh [`WolframSession`] and return its echo.
///
/// Convenience for callers that do not need persistent state.
pub fn eval(src: &str) -> Result<String, String> {
    WolframSession::new().feed(src)
}

/// Recover a human-readable message from a caught panic payload.
fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Wolfram could not evaluate that input".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_folds_to_an_integer() {
        // 1 + 2*3 = 7
        assert_eq!(eval("1 + 2*3\n").unwrap(), "Out[1]= 7\n");
    }

    #[test]
    fn plus_head_application_matches_infix() {
        // Plus[1, 2, 3] = 6, the same engine as 1 + 2 + 3.
        assert_eq!(eval("Plus[1, 2, 3]\n").unwrap(), "Out[1]= 6\n");
        assert_eq!(eval("1 + 2 + 3\n").unwrap(), "Out[1]= 6\n");
    }

    #[test]
    fn power_head_application() {
        // Power[2, 10] = 1024
        assert_eq!(eval("Power[2, 10]\n").unwrap(), "Out[1]= 1024\n");
        assert_eq!(eval("2^10\n").unwrap(), "Out[1]= 1024\n");
    }

    #[test]
    fn times_and_divide() {
        assert_eq!(eval("Times[6, 7]\n").unwrap(), "Out[1]= 42\n");
        assert_eq!(eval("10 / 2\n").unwrap(), "Out[1]= 5\n");
    }

    #[test]
    fn free_symbols_stay_symbolic() {
        // Unbound x stays x; x + 0 folds to x.
        assert_eq!(eval("x + 0\n").unwrap(), "Out[1]= x\n");
        assert_eq!(eval("x*1\n").unwrap(), "Out[1]= x\n");
    }

    #[test]
    fn list_literal_evaluates_elementwise() {
        // {1 + 1, 2*3, 2^3} = {2, 6, 8}
        assert_eq!(eval("{1 + 1, 2*3, 2^3}\n").unwrap(), "Out[1]= {2, 6, 8}\n");
    }

    #[test]
    fn assignment_persists_across_calls() {
        let mut s = WolframSession::new();
        // `x = 5` is displayed (the assigned value), advancing Out to 1.
        assert_eq!(s.feed("x = 5\n").unwrap(), "Out[1]= 5\n");
        assert_eq!(s.feed("x + 1\n").unwrap(), "Out[2]= 6\n");
    }

    #[test]
    fn semicolon_suppresses_display_but_still_runs() {
        let mut s = WolframSession::new();
        // `x = 5;` runs (binds x) but displays nothing; still advances Out.
        assert_eq!(s.feed("x = 5;\n").unwrap(), "");
        // The next displayed result is Out[2].
        assert_eq!(s.feed("x*2\n").unwrap(), "Out[2]= 10\n");
    }

    #[test]
    fn sin_of_zero_folds_to_zero() {
        // Sin[0] = 0 via the shared elementary-function handler.
        assert_eq!(eval("Sin[0]\n").unwrap(), "Out[1]= 0\n");
    }

    #[test]
    fn user_defined_function() {
        // A `:=` definition binds the function; the test asserts on its
        // *application* (the Define echo shape is engine-defined).
        let mut s = WolframSession::new();
        s.feed("square[x_] := x*x;\n").unwrap();
        assert_eq!(s.feed("square[5]\n").unwrap(), "Out[2]= 25\n");
    }

    #[test]
    fn replace_all_with_a_single_rule() {
        // x /. x -> 5  =>  5
        assert_eq!(eval("x /. x -> 5\n").unwrap(), "Out[1]= 5\n");
    }

    #[test]
    fn replace_all_with_a_pattern_rule() {
        // (a + b) /. x_ -> x  is identity on the matched whole; use a structural
        // rule: f[y] /. f[t_] -> t  =>  y
        assert_eq!(eval("f[y] /. f[t_] -> t\n").unwrap(), "Out[1]= y\n");
    }

    #[test]
    fn replace_all_with_a_list_of_rules() {
        // {a, b} /. {a -> 1, b -> 2}  =>  {1, 2}
        assert_eq!(
            eval("{a, b} /. {a -> 1, b -> 2}\n").unwrap(),
            "Out[1]= {1, 2}\n"
        );
    }

    #[test]
    fn multiple_statements_in_one_feed() {
        let out = eval("1 + 1\n2 + 2\n3 + 3\n").unwrap();
        assert_eq!(out, "Out[1]= 2\nOut[2]= 4\nOut[3]= 6\n");
    }

    #[test]
    fn suppression_still_advances_the_output_index() {
        let mut s = WolframSession::new();
        assert_eq!(s.feed("10;\n").unwrap(), "");
        // The suppressed statement consumed Out[1]; next displayed is Out[2].
        assert_eq!(s.feed("20\n").unwrap(), "Out[2]= 20\n");
    }

    #[test]
    fn a_parse_error_is_returned_not_panicked() {
        let err = eval("1 +\n");
        assert!(err.is_err(), "expected a clean Err, got {err:?}");
    }

    #[test]
    fn the_one_shot_eval_helper_matches_a_session() {
        let one = eval("2 + 3\n").unwrap();
        let two = WolframSession::new().feed("2 + 3\n").unwrap();
        assert_eq!(one, two);
    }

    #[test]
    fn oversized_input_is_rejected_before_evaluation() {
        let huge = format!("{}1\n", "1+".repeat(MAX_INPUT_LEN));
        assert!(huge.len() > MAX_INPUT_LEN);
        let err = eval(&huge).unwrap_err();
        assert!(err.contains("too large"), "got {err:?}");
    }

    #[test]
    fn deeply_nested_brackets_are_rejected_not_aborted() {
        // Thousands of nested parens would overflow the stack on build + drop;
        // the complexity cap rejects them with a clean error first.
        let depth = 20_000;
        let src = format!("{}1{}\n", "(".repeat(depth), ")".repeat(depth));
        assert!(src.len() <= MAX_INPUT_LEN, "stays under the length cap");
        let err = eval(&src).unwrap_err();
        assert!(err.contains("too complex"), "got {err:?}");
    }

    #[test]
    fn long_prefix_minus_chain_is_rejected() {
        let src = format!("{}x\n", "-".repeat(5_000));
        assert!(src.len() <= MAX_INPUT_LEN);
        assert!(eval(&src).unwrap_err().contains("too complex"));
    }

    #[test]
    fn long_binary_chain_is_rejected() {
        let src = format!("{}1\n", "1+".repeat(5_000));
        assert!(src.len() <= MAX_INPUT_LEN);
        assert!(eval(&src).unwrap_err().contains("too complex"));
    }

    #[test]
    fn moderate_nesting_still_evaluates() {
        let depth = 40;
        let src = format!("{}1 + 2{}\n", "(".repeat(depth), ")".repeat(depth));
        assert_eq!(eval(&src).unwrap(), "Out[1]= 3\n");
    }

    #[test]
    fn a_string_literal_terminator_does_not_reset_the_cap() {
        // A `;` inside a string must not reset the per-statement token counter.
        let mut src = String::new();
        for i in 0..6_000 {
            src.push_str("1+");
            if i % 50 == 0 {
                src.push_str("\";\"+");
            }
        }
        src.push_str("1\n");
        assert!(src.len() <= MAX_INPUT_LEN);
        assert!(eval(&src).unwrap_err().contains("too complex"));
    }

    #[test]
    fn comment_hidden_newline_does_not_bypass_the_cap() {
        // A newline can't appear in a `(* *)` comment to fool the cap; this checks
        // a comment before a deep nest does not change the rejection.
        let depth = 20_000;
        let src = format!("(* hi *){}1{}\n", "(".repeat(depth), ")".repeat(depth));
        assert!(src.len() <= MAX_INPUT_LEN);
        assert!(eval(&src).unwrap_err().contains("too complex"));
    }

    #[test]
    fn session_recovers_after_a_parse_error() {
        let mut s = WolframSession::new();
        let _ = s.feed("1 +\n"); // parse error, not a panic
        assert_eq!(s.feed("3 + 4\n").unwrap(), "Out[1]= 7\n");
    }

    #[test]
    fn a_malformed_set_lhs_is_caught_not_aborted() {
        // `5 = 3` lowers to `Assign(5, 3)`. The reused VM's assign_handler
        // *panics* when the Set LHS is not a symbol — a malformed-AST surface the
        // security review flagged. The worker-thread `catch_unwind` must convert
        // that panic into a clean `Err`, and the session must remain usable after
        // (the env is rebuilt). This proves a crafted statement cannot abort the
        // process or wedge the session.
        let mut s = WolframSession::new();
        assert!(
            s.feed("5 = 3\n").is_err(),
            "a non-symbol Set LHS must return Err, never abort"
        );
        // The session survives and works on the next call.
        assert_eq!(s.feed("2 + 2\n").unwrap(), "Out[1]= 4\n");
    }

    // --- Self-referential-reassignment DoS guard (a security audit's
    // finding, shared with every consumer of `symbolic-vm`'s
    // `assign_handler` -- see `symbolic_vm::handlers::MAX_BOUND_VALUE_NODES`
    // / `MAX_BOUND_VALUE_DEPTH`'s own doc comments) -- `a = a * a` /
    // `a = a + a`, repeated even a handful of times, doubles the bound
    // value's node count and/or nesting depth every step, reaching millions
    // of nodes from a few hundred bytes of source. Wolfram's `Set` (`=`)
    // lowers straight to `symbolic_ir::ASSIGN` and evaluates through the
    // shared `vm.eval`, so it is protected by the same choke-point fix with
    // no crate-specific bypass work needed -- these tests prove that
    // end-to-end through a real `WolframSession`.
    // -------------------------------------------------------------------

    #[test]
    fn self_referential_multiplication_is_rejected_cleanly() {
        // The exact audited scenario, as newline-separated Wolfram
        // statements (`multiple_statements_in_one_feed` above confirms one
        // `feed` call accepts many): `a = x + y` then `a = a * a` repeated.
        // 30 repetitions is far past the real trip point (~15th step) --
        // without the guard this reaches billions of nodes; with it, a
        // clean `Err` in well under a second.
        let mut src = String::from("a = x + y\n");
        for _ in 0..30 {
            src.push_str("a = a * a\n");
        }
        assert!(src.len() < 500, "source should stay tiny: {} bytes", src.len());

        let mut s = WolframSession::new();
        let err = s.feed(&src).unwrap_err();
        assert!(err.contains("nodes"), "expected a node-count rejection, got {err:?}");
        // The worker-thread `catch_unwind` rebuilds the session on panic —
        // it must remain usable afterward.
        assert_eq!(s.feed("2 + 2\n").unwrap(), "Out[1]= 4\n");
    }

    #[test]
    fn self_referential_addition_is_rejected_cleanly() {
        // Same attack shape via the audit's other named example, `a = a +
        // a`. This one trips the DEPTH guard, not the node-count guard --
        // `Add`'s own flatten-then-left-associate canonicalization rebuilds
        // a chain whose depth equals its leaf count, and leaf count doubles
        // too. Without a depth guard this reproduces a genuine, uncatchable
        // native stack overflow on a later statement's symbol lookup, not
        // merely a slow hang.
        let mut src = String::from("a = x + y\n");
        for _ in 0..30 {
            src.push_str("a = a + a\n");
        }

        let mut s = WolframSession::new();
        let err = s.feed(&src).unwrap_err();
        assert!(
            err.contains("levels"),
            "expected a nesting-depth rejection, got {err:?}"
        );
        assert_eq!(s.feed("2 + 2\n").unwrap(), "Out[1]= 4\n");
    }

    #[test]
    fn a_handful_of_self_multiplications_under_the_cap_still_evaluate_correctly() {
        // Non-false-positive check: a FEW self-referential reassignments,
        // comfortably under the caps, must still evaluate normally.
        let mut s = WolframSession::new();
        s.feed("a = 2\na = a * a\na = a * a\n").unwrap(); // 2 -> 4 -> 16
        assert_eq!(s.feed("a\n").unwrap(), "Out[4]= 16\n");
    }

    #[test]
    fn negative_results_render() {
        assert_eq!(eval("3 - 10\n").unwrap(), "Out[1]= -7\n");
        assert_eq!(eval("-(2 + 3)\n").unwrap(), "Out[1]= -5\n");
    }

    #[test]
    fn a_small_wolfram_program_evaluates_end_to_end() {
        let mut s = WolframSession::new();
        s.feed("sq[x_] := x^2;\n").unwrap();
        let out = s.feed("sq[3] + Power[2, 3]\n").unwrap();
        // 9 + 8 = 17
        assert_eq!(out, "Out[2]= 17\n");
    }

    #[test]
    fn empty_and_whitespace_input_yields_nothing() {
        assert_eq!(eval("\n").unwrap(), "");
        assert_eq!(eval("   \n").unwrap(), "");
    }

    // --- W-11 pure functions: end-to-end through the real session ----------

    #[test]
    fn named_function_applied_immediately() {
        // Function[x, x^2][5] → 25
        assert_eq!(eval("Function[x, x^2][5]\n").unwrap(), "Out[1]= 25\n");
        // Function[{x, y}, x + y][3, 4] → 7
        assert_eq!(
            eval("Function[{x, y}, x + y][3, 4]\n").unwrap(),
            "Out[1]= 7\n"
        );
    }

    #[test]
    fn slot_function_applied_immediately() {
        // (#^2)&[5] → 25
        assert_eq!(eval("(#^2)&[5]\n").unwrap(), "Out[1]= 25\n");
        // (#1 + #2)&[3, 4] → 7
        assert_eq!(eval("(#1 + #2)&[3, 4]\n").unwrap(), "Out[1]= 7\n");
        // #&[9] → 9  (the identity pure function; # ≡ #1)
        assert_eq!(eval("#&[9]\n").unwrap(), "Out[1]= 9\n");
    }

    #[test]
    fn slot_and_named_forms_agree() {
        // The two spellings of "square the argument" give the same answer.
        assert_eq!(
            eval("(#^2)&[7]\n").unwrap(),
            eval("Function[x, x^2][7]\n").unwrap()
        );
    }

    #[test]
    fn pure_function_composes_with_map() {
        // Map[#^2 &, {1, 2, 3}] → {1, 4, 9}  (no special code in Map — the
        // backend rule fires when Map re-evals (#^2&)[x]).
        assert_eq!(
            eval("Map[#^2 &, {1, 2, 3}]\n").unwrap(),
            "Out[1]= {1, 4, 9}\n"
        );
        // The /@ sugar form is identical (parenthesised: `&` is looser than `/@`,
        // so the pure function must be grouped when used as `/@`'s left operand —
        // the same "write parentheses when mixing" convention W-6 documents).
        assert_eq!(
            eval("(#^2 &) /@ {1, 2, 3}\n").unwrap(),
            "Out[1]= {1, 4, 9}\n"
        );
    }

    #[test]
    fn pure_function_composes_with_select() {
        // Select[{1, 2, 3, 4}, Mod[#, 2] == 0 &] → {2, 4}
        assert_eq!(
            eval("Select[{1, 2, 3, 4}, Mod[#, 2] == 0 &]\n").unwrap(),
            "Out[1]= {2, 4}\n"
        );
    }

    #[test]
    fn pure_function_composes_with_nest() {
        // Nest[# + 1 &, 0, 3] → 3
        assert_eq!(eval("Nest[# + 1 &, 0, 3]\n").unwrap(), "Out[1]= 3\n");
    }

    #[test]
    fn slot_sequence_splices_all_arguments() {
        // Plus[##]& applied to three args sums them: (Plus[##]&)[1, 2, 3] → 6.
        assert_eq!(eval("Plus[##] &[1, 2, 3]\n").unwrap(), "Out[1]= 6\n");
    }

    #[test]
    fn nested_pure_functions_apply_independently() {
        // Function[x, x + 1][Function[y, y*2][3]] → (3*2) + 1 = 7. The inner
        // function is applied first (its slot/param is its own), then the outer.
        assert_eq!(
            eval("Function[x, x + 1][Function[y, y*2][3]]\n").unwrap(),
            "Out[1]= 7\n"
        );
    }

    #[test]
    fn unapplied_pure_function_is_an_inert_value() {
        // A pure function on its own is a value (Wolfram's "function object"); it
        // does not error and does not try to substitute non-existent args.
        assert!(eval("#^2 &\n").is_ok());
        assert!(eval("Function[x, x^2]\n").is_ok());
    }

    #[test]
    fn malformed_pure_function_application_does_not_abort_the_session() {
        // A two-param function applied to one arg cannot reduce; the session must
        // return cleanly (the form stays unevaluated) and remain usable.
        let mut s = WolframSession::new();
        assert!(s.feed("Function[{x, y}, x + y][1]\n").is_ok());
        assert_eq!(s.feed("2 + 2\n").unwrap(), "Out[2]= 4\n");
    }

    #[test]
    fn deeply_nested_pure_function_is_bounded_not_a_crash() {
        // A self-referential-ish deeply applied chain must not overflow the
        // worker stack — the per-statement token cap + the eval stack bound it.
        // Nest a pure function 50 times; the result is well-defined (50).
        assert_eq!(eval("Nest[# + 1 &, 0, 50]\n").unwrap(), "Out[1]= 50\n");
    }

    // --- W-12 string builtins, end-to-end through the full lex→lower→eval→print
    //     pipeline (the unit tests in builtins.rs exercise the handlers directly;
    //     these prove the surface syntax parses and the printer renders the
    //     string results with quotes). ----------------------------------------

    #[test]
    fn w12_string_builtins_end_to_end() {
        assert_eq!(eval("StringLength[\"abc\"]\n").unwrap(), "Out[1]= 3\n");
        assert_eq!(
            eval("StringJoin[\"a\", \"b\", \"c\"]\n").unwrap(),
            "Out[1]= \"abc\"\n"
        );
        assert_eq!(
            eval("StringTake[\"hello\", 3]\n").unwrap(),
            "Out[1]= \"hel\"\n"
        );
        assert_eq!(
            eval("StringTake[\"hello\", {2, 4}]\n").unwrap(),
            "Out[1]= \"ell\"\n"
        );
        assert_eq!(
            eval("StringTake[\"hello\", -2]\n").unwrap(),
            "Out[1]= \"lo\"\n"
        );
        assert_eq!(
            eval("StringDrop[\"hello\", 2]\n").unwrap(),
            "Out[1]= \"llo\"\n"
        );
        assert_eq!(
            eval("StringSplit[\"a,b,c\", \",\"]\n").unwrap(),
            "Out[1]= {\"a\", \"b\", \"c\"}\n"
        );
        assert_eq!(
            eval("StringSplit[\"a b  c\"]\n").unwrap(),
            "Out[1]= {\"a\", \"b\", \"c\"}\n"
        );
        assert_eq!(
            eval("StringReplace[\"banana\", \"a\" -> \"o\"]\n").unwrap(),
            "Out[1]= \"bonono\"\n"
        );
        assert_eq!(eval("ToString[123]\n").unwrap(), "Out[1]= \"123\"\n");
        assert_eq!(
            eval("Characters[\"ab\"]\n").unwrap(),
            "Out[1]= {\"a\", \"b\"}\n"
        );
    }

    #[test]
    fn w12_unicode_end_to_end() {
        // A multi-byte char counts as one and is never split.
        assert_eq!(eval("StringLength[\"héllo\"]\n").unwrap(), "Out[1]= 5\n");
        assert_eq!(
            eval("StringTake[\"héllo\", 2]\n").unwrap(),
            "Out[1]= \"hé\"\n"
        );
    }

    #[test]
    fn w12_malformed_input_stays_unevaluated_and_session_survives() {
        // A non-string arg and an out-of-range index both echo back unevaluated,
        // and the session keeps working afterwards (no panic).
        let mut s = WolframSession::new();
        assert_eq!(
            s.feed("StringLength[123]\n").unwrap(),
            "Out[1]= StringLength[123]\n"
        );
        assert_eq!(
            s.feed("StringTake[\"hi\", 9]\n").unwrap(),
            "Out[2]= StringTake[\"hi\", 9]\n"
        );
        assert_eq!(s.feed("2 + 2\n").unwrap(), "Out[3]= 4\n");
    }
}
