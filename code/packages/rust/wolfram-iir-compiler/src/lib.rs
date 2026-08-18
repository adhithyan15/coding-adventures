//! # wolfram-iir-compiler
//!
//! Wolfram CST → `interpreter_ir::IIRModule`, **v0.1.0**.
//!
//! The sixth and final real-language frontend in this rollout (after
//! Macsyma, Derive, Reduce, Maple, and Axiom) to bridge a math language
//! in this repo onto `interpreter_ir` (IIR) — closing out Wave 5. See
//! [`wolfram-iir-vm.md`](../../../specs/wolfram-iir-vm.md) for the
//! per-language deltas from `macsyma-iir-vm.md`'s original design and
//! `lower.rs`'s module doc comment for the v0 scope (deliberately
//! narrower than the FULL grammar `wolfram-to-semantic-ir` covers — no
//! pattern matching, rules, replacement, or pure functions in v0). It
//! consumes the generic `GrammarASTNode` CST produced by
//! `coding-adventures-wolfram-parser` and emits an
//! [`interpreter_ir::IIRModule`], executable on
//! [`wolfram-vm`](../wolfram-vm) (v0 targets the VM interpreter backend
//! only).
//!
//! ## Pipeline
//!
//! ```text
//! Wolfram source
//!    │
//!    ▼  coding_adventures_wolfram_parser::try_parse_wolfram(src)
//! parser::grammar_parser::GrammarASTNode   (generic CST)
//!    │
//!    ▼  wolfram_iir_compiler::compile
//! interpreter_ir::IIRModule                (v0 subset)
//!    │
//!    ▼  wolfram_vm::run
//! dynval_runtime::LispyValue
//! ```
//!
//! ## Public API
//!
//! ```
//! use wolfram_iir_compiler::compile_source;
//! let module = compile_source("2 + 3\n", "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```

mod lower;
pub use lower::{compile, WolframIirError};

/// Recursion-depth cap `compile_source` parses under, and the enlarged
/// worker-thread stack size it parses *on*.
///
/// **Corrects an earlier, incorrect assumption in this crate**: this
/// function originally reasoned (by analogy to
/// `macsyma-iir-compiler::compile_source`) that no worker-thread stack
/// enlargement was needed here, since `MAX_RULE_DEPTH` is a property of
/// the parser, not of this lowering pass. That analogy is invalid for
/// Wolfram specifically — caught in security review, not assumed safe.
/// `wolfram-parser`'s own `MAX_RULE_DEPTH` doc comment explains why:
/// Wolfram's 20-rule-per-level precedence cascade means even ordinary
/// `(...)` nesting costs ~20 `parse_rule` frames per level, so the
/// parser's own measured bare-stack (~2 MiB) crash floor is only ~276
/// frames — about **11 real nesting levels** — regardless of
/// `MAX_RULE_DEPTH`'s cap value. `wolfram-to-semantic-ir::compile_source`
/// already solved this exact problem (it calls the same
/// `try_parse_wolfram`); this crate now mirrors that solution exactly
/// rather than the simpler, bare-stack-safe pattern
/// `macsyma-iir-compiler`/`derive-iir-compiler`/`reduce-iir-compiler`/
/// `maple-iir-compiler`/`axiom-iir-compiler` correctly use (those
/// parsers' own shallower precedence cascades really are bare-stack-safe
/// — this crate's original doc comment wrongly generalised that to
/// Wolfram too).
///
/// See `wolfram-to-semantic-ir::PARSE_STACK_SIZE`'s own doc comment for
/// the full derivation of this budget (64 MiB, not `wolfram-runtime`'s
/// 512 MiB `EVAL_STACK_SIZE` — sized down to avoid CI resource pressure
/// from many concurrent per-call thread stacks while still supporting
/// the full default `MAX_RULE_DEPTH` ceiling with comfortable margin).
const PARSE_STACK_SIZE: usize = 64 * 1024 * 1024;

/// Parse `source` as Wolfram and lower it into an
/// [`interpreter_ir::IIRModule`] in one step, mirroring
/// `wolfram-to-semantic-ir::compile_source`'s convenience wrapper —
/// including its enlarged-stack worker-thread pattern (see
/// [`PARSE_STACK_SIZE`]'s doc comment for why, unlike every sibling
/// `*-iir-compiler::compile_source` in this rollout, this one needs it).
///
/// Both thread-spawn failure and a worker-thread panic (from anywhere in
/// the parse/lower pipeline, not just a stack overflow, which is
/// unrecoverable regardless and aborts the whole process before `join`
/// ever runs) are converted into a [`WolframIirError`] rather than
/// propagated as a panic on the calling thread — mirrors
/// `wolfram-to-semantic-ir::compile_source`'s identical guarantee.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<interpreter_ir::IIRModule, WolframIirError> {
    let source = source.to_string();
    let module_name = module_name.to_string();
    let handle = std::thread::Builder::new()
        .name("wolfram-iir-compiler-compile".to_string())
        .stack_size(PARSE_STACK_SIZE)
        .spawn(move || compile_on_this_thread(&source, &module_name))
        .map_err(|e| WolframIirError {
            message: format!("failed to spawn wolfram-iir-compiler compile worker thread: {e}"),
            line: 1,
            column: 1,
        })?;
    match handle.join() {
        Ok(result) => result,
        Err(panic_payload) => Err(WolframIirError {
            message: format!(
                "wolfram-iir-compiler compile worker thread panicked: {}",
                panic_message(&panic_payload)
            ),
            line: 1,
            column: 1,
        }),
    }
}

/// Extract a human-readable message from a `std::thread::Result::Err`
/// panic payload — mirrors `wolfram-to-semantic-ir`'s identically-named
/// free function exactly.
fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

fn compile_on_this_thread(
    source: &str,
    module_name: &str,
) -> Result<interpreter_ir::IIRModule, WolframIirError> {
    let tree = coding_adventures_wolfram_parser::try_parse_wolfram(source).map_err(|msg| {
        WolframIirError {
            message: format!("parse error: {msg}"),
            line: 1,
            column: 1,
        }
    })?;
    compile(&tree, module_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wolfram_vm::run;

    fn eval_int(source: &str) -> i64 {
        let module = compile_source(source, "t").expect("compile");
        run(&module).expect("run").as_int().expect("int result")
    }

    fn eval_symbol(source: &str) -> String {
        let module = compile_source(source, "t").expect("compile");
        let v = run(&module).expect("run");
        dynval_runtime::name_of(v.as_symbol().expect("symbol result")).expect("interned")
    }

    // ---- accepted: literal arithmetic ----

    #[test]
    fn integer_literal() {
        assert_eq!(eval_int("42\n"), 42);
    }

    #[test]
    fn simple_addition() {
        assert_eq!(eval_int("2 + 3\n"), 5);
    }

    #[test]
    fn precedence() {
        assert_eq!(eval_int("2 + 3 * 4\n"), 14);
    }

    #[test]
    fn chain() {
        assert_eq!(eval_int("1 + 2 + 3 + 4\n"), 10);
    }

    #[test]
    fn unary_neg_leaf() {
        assert_eq!(eval_int("-5 + 3\n"), -2);
    }

    #[test]
    fn unary_neg_compound() {
        assert_eq!(eval_int("-(5 + 3)\n"), -8);
    }

    #[test]
    fn unary_plus_is_noop() {
        assert_eq!(eval_int("+5\n"), 5);
    }

    #[test]
    fn grouping() {
        assert_eq!(eval_int("(2 + 3) * 4\n"), 20);
    }

    #[test]
    fn exact_division() {
        assert_eq!(eval_int("20 / 4\n"), 5);
    }

    #[test]
    fn negative_literal_exact_division() {
        assert_eq!(eval_int("-4 / 2\n"), -2);
    }

    // ---- accepted: assignment ----

    #[test]
    fn assignment_and_reference() {
        assert_eq!(eval_int("x = 3\nx + 1\n"), 4);
    }

    #[test]
    fn reassignment_threading() {
        assert_eq!(eval_int("x = 3\nx = x + 1\nx\n"), 4);
    }

    #[test]
    fn two_variables() {
        assert_eq!(eval_int("a = 2\nb = 3\na * b\n"), 6);
    }

    // ---- accepted: unevaluated symbolic expressions ----

    #[test]
    fn free_symbol_addition_is_symbolic() {
        let module = compile_source("x + y\n", "t").expect("compile");
        let v = run(&module).expect("run");
        assert!(v.as_int().is_none());
    }

    #[test]
    fn free_symbol_addition_head_is_add() {
        let module = compile_source("x + y\n", "t").expect("compile");
        let v = run(&module).expect("run");
        let head = dynval_runtime::builtins::car(&[v]).unwrap();
        assert_eq!(
            dynval_runtime::name_of(head.as_symbol().unwrap()).as_deref(),
            Some("Add")
        );
    }

    #[test]
    fn mixed_concrete_and_symbolic_operand() {
        let module = compile_source("2 * x\n", "t").expect("compile");
        let v = run(&module).expect("run");
        let head = dynval_runtime::builtins::car(&[v]).unwrap();
        assert_eq!(
            dynval_runtime::name_of(head.as_symbol().unwrap()).as_deref(),
            Some("Mul")
        );
    }

    #[test]
    fn free_symbol_alone() {
        assert_eq!(eval_symbol("x\n"), "x");
    }

    // ---- rejected: explicit-error paths ----

    #[test]
    fn float_literal_rejected() {
        assert!(compile_source("1.5\n", "t").is_err());
    }

    #[test]
    fn non_exact_division_rejected() {
        let err = compile_source("7 / 2\n", "t").unwrap_err();
        assert!(
            err.message.contains("rational"),
            "message was: {}",
            err.message
        );
    }

    #[test]
    fn division_of_non_literal_concrete_rejected() {
        assert!(compile_source("x = 6\nx / 2\n", "t").is_err());
    }

    #[test]
    fn division_by_literal_zero_rejected() {
        assert!(compile_source("1 / 0\n", "t").is_err());
    }

    #[test]
    fn i64_min_divided_by_neg_one_rejected_not_panicking() {
        assert!(compile_source("(-9223372036854775807 - 1) / -1\n", "t").is_err());
    }

    #[test]
    fn function_pattern_definition_rejected() {
        let err = compile_source("f[x_] := x + 1\n", "t").unwrap_err();
        assert!(
            err.message.contains("function/pattern definition"),
            "message was: {}",
            err.message
        );
    }

    #[test]
    fn function_call_rejected() {
        assert!(compile_source("f[x]\n", "t").is_err());
    }

    #[test]
    fn part_indexing_rejected() {
        assert!(compile_source("x[[1]]\n", "t").is_err());
    }

    #[test]
    fn comparison_rejected() {
        assert!(compile_source("x == y\n", "t").is_err());
    }

    #[test]
    fn logical_and_rejected() {
        assert!(compile_source("x && y\n", "t").is_err());
    }

    #[test]
    fn power_rejected() {
        assert!(compile_source("x^2\n", "t").is_err());
    }

    #[test]
    fn list_literal_rejected() {
        assert!(compile_source("{1, 2, 3}\n", "t").is_err());
    }

    #[test]
    fn string_literal_rejected() {
        assert!(compile_source("\"hi\"\n", "t").is_err());
    }

    #[test]
    fn pattern_blank_rejected() {
        assert!(compile_source("_\n", "t").is_err());
    }

    #[test]
    fn rule_rejected() {
        assert!(compile_source("x -> y\n", "t").is_err());
    }

    #[test]
    fn replaceall_rejected() {
        assert!(compile_source("x /. y -> 1\n", "t").is_err());
    }

    #[test]
    fn pure_function_rejected() {
        assert!(compile_source("(# + 1) &\n", "t").is_err());
    }

    #[test]
    fn map_sugar_rejected() {
        assert!(compile_source("f /@ x\n", "t").is_err());
    }

    #[test]
    fn deeply_nested_parens_fail_cleanly_not_natively() {
        // Regression for a security-review finding: `compile_source`
        // originally parsed on the caller's own bare-stack thread,
        // trusting (incorrectly) that Wolfram's parser needed no
        // enlarged-stack worker thread the way the other five Wave 5
        // frontends' parsers do. `wolfram-parser`'s own measured
        // bare-stack crash floor is ~11 real nesting levels (its
        // 20-rule-per-level precedence cascade costs ~20 parse_rule
        // frames per level) -- 300 levels of `(...)` nesting is
        // trivially reachable syntactically valid input that would have
        // crashed the process natively before this fix, and must now
        // fail with a clean `Err` (either a parse-depth rejection or
        // this crate's own `MAX_EXPR_DEPTH` lowering-recursion guard)
        // instead.
        let deeply_nested = format!("{}1{}\n", "(".repeat(300), ")".repeat(300));
        assert!(compile_source(&deeply_nested, "t").is_err());
    }
}
