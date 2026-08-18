//! # macsyma-iir-compiler
//!
//! Macsyma CST → `interpreter_ir::IIRModule`, **v0.1.0**.
//!
//! This is the first frontend to bridge a math language in this repo onto
//! `interpreter_ir` (IIR) — the shared IR the AOT/lang-vm chain lowers to
//! 7 real backends (NativeAOT, LLVM, WASM, JVM, CLR, VM interpreter, JIT)
//! — rather than only the Semantic IR source-to-source pipeline
//! `macsyma-to-semantic-ir` already targets. See
//! [`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md) for the full
//! design and `lower.rs`'s module doc comment for the v0 scope and the
//! `/` exactness rule. It consumes the generic `GrammarASTNode` CST
//! produced by `coding-adventures-macsyma-parser` and emits an
//! [`interpreter_ir::IIRModule`], executable on
//! [`macsyma-vm`](../../macsyma-vm) (v0 targets the VM interpreter
//! backend only — see the spec's Wave 4 for native codegen).
//!
//! ## Pipeline
//!
//! ```text
//! Macsyma source
//!    │
//!    ▼  coding_adventures_macsyma_parser::create_macsyma_parser(src).parse()
//! parser::grammar_parser::GrammarASTNode   (generic CST)
//!    │
//!    ▼  macsyma_iir_compiler::compile
//! interpreter_ir::IIRModule                (v0 subset)
//!    │
//!    ▼  macsyma_vm::run
//! dynval_runtime::LispyValue
//! ```
//!
//! ## Public API
//!
//! ```
//! use macsyma_iir_compiler::compile_source;
//! let module = compile_source("2 + 3$\n", "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```

mod lower;
pub use lower::{compile, MacsymaIirError};

/// Parse `source` as Macsyma and lower it into an
/// [`interpreter_ir::IIRModule`] in one step, mirroring
/// `macsyma-to-semantic-ir::compile_source`'s convenience wrapper.
///
/// Needs no worker-thread stack enlargement, for the same reason
/// `macsyma-to-semantic-ir::compile_source` doesn't:
/// `coding_adventures_macsyma_parser`'s own `MAX_RULE_DEPTH` (200) is
/// already documented safe on a bare default (~2 MiB) stack.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<interpreter_ir::IIRModule, MacsymaIirError> {
    let mut parser = coding_adventures_macsyma_parser::create_macsyma_parser(source);
    let tree = parser.parse().map_err(|err| MacsymaIirError {
        message: format!("parse error: {}", err.message),
        line: err.token.line,
        column: err.token.column,
    })?;
    compile(&tree, module_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use macsyma_vm::run;

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
        assert_eq!(eval_int("42$"), 42);
    }

    #[test]
    fn simple_addition() {
        assert_eq!(eval_int("2 + 3$"), 5);
    }

    #[test]
    fn precedence() {
        assert_eq!(eval_int("2 + 3 * 4$"), 14);
    }

    #[test]
    fn chain() {
        assert_eq!(eval_int("1 + 2 + 3 + 4$"), 10);
    }

    #[test]
    fn unary_neg_leaf() {
        assert_eq!(eval_int("-5 + 3$"), -2);
    }

    #[test]
    fn unary_neg_compound() {
        assert_eq!(eval_int("-(5 + 3)$"), -8);
    }

    #[test]
    fn unary_plus_is_noop() {
        assert_eq!(eval_int("+5$"), 5);
    }

    #[test]
    fn grouping() {
        assert_eq!(eval_int("(2 + 3) * 4$"), 20);
    }

    #[test]
    fn exact_division() {
        assert_eq!(eval_int("20 / 4$"), 5);
    }

    #[test]
    fn negative_literal_exact_division() {
        assert_eq!(eval_int("-4 / 2$"), -2);
    }

    // ---- accepted: assignment ----

    #[test]
    fn assignment_and_reference() {
        assert_eq!(eval_int("x: 3$ x + 1$"), 4);
    }

    #[test]
    fn reassignment_threading() {
        assert_eq!(eval_int("x: 3$ x: x + 1$ x$"), 4);
    }

    #[test]
    fn two_variables() {
        assert_eq!(eval_int("a: 2$ b: 3$ a * b$"), 6);
    }

    // ---- accepted: unevaluated symbolic expressions ----

    #[test]
    fn free_symbol_addition_is_symbolic() {
        // x + y with both free: result is a symbolic Apply node, not an int.
        let module = compile_source("x + y$", "t").expect("compile");
        let v = run(&module).expect("run");
        assert!(v.as_int().is_none());
    }

    #[test]
    fn free_symbol_addition_head_is_add() {
        let module = compile_source("x + y$", "t").expect("compile");
        let v = run(&module).expect("run");
        let head = dynval_runtime::builtins::car(&[v]).unwrap();
        assert_eq!(
            dynval_runtime::name_of(head.as_symbol().unwrap()).as_deref(),
            Some("Add")
        );
    }

    #[test]
    fn mixed_concrete_and_symbolic_operand() {
        // 2*x: one concrete literal, one free symbol -> symbolic Mul.
        let module = compile_source("2 * x$", "t").expect("compile");
        let v = run(&module).expect("run");
        let head = dynval_runtime::builtins::car(&[v]).unwrap();
        assert_eq!(
            dynval_runtime::name_of(head.as_symbol().unwrap()).as_deref(),
            Some("Mul")
        );
    }

    #[test]
    fn free_symbol_alone() {
        assert_eq!(eval_symbol("x$"), "x");
    }

    // ---- rejected: explicit-error paths ----

    #[test]
    fn float_literal_rejected() {
        assert!(compile_source("1.5$", "t").is_err());
    }

    #[test]
    fn non_exact_division_rejected() {
        let err = compile_source("7 / 2$", "t").unwrap_err();
        assert!(
            err.message.contains("Rational"),
            "message was: {}",
            err.message
        );
    }

    #[test]
    fn division_of_non_literal_concrete_rejected() {
        // x: 6$ x/2 -- x is concrete but not a *direct* literal at this
        // node, so exactness can't be verified at compile time.
        assert!(compile_source("x: 6$ x / 2$", "t").is_err());
    }

    #[test]
    fn division_by_literal_zero_rejected() {
        assert!(compile_source("1 / 0$", "t").is_err());
    }

    #[test]
    fn i64_min_divided_by_neg_one_rejected_not_panicking() {
        // Regression: plain `a % b`/`a / b` on i64 panics (in every build
        // profile) for exactly this case. i64::MIN is reachable here via
        // checked_sub without ever tripping an overflow error
        // (`-9223372036854775807 - 1` is in-range), so this must return a
        // clean Err rather than crash the compiler on adversarial input.
        assert!(compile_source("(-9223372036854775807 - 1) / -1$", "t").is_err());
    }

    #[test]
    fn function_definition_rejected() {
        assert!(compile_source("f(x) := x + 1$", "t").is_err());
    }

    #[test]
    fn if_rejected() {
        assert!(compile_source("if x > 0 then 1 else 0$", "t").is_err());
    }

    #[test]
    fn while_rejected() {
        assert!(compile_source("while x < 5 do x$", "t").is_err());
    }

    #[test]
    fn list_literal_rejected() {
        assert!(compile_source("[1, 2, 3]$", "t").is_err());
    }

    #[test]
    fn function_call_rejected() {
        assert!(compile_source("sin(x)$", "t").is_err());
    }

    #[test]
    fn comparison_rejected() {
        assert!(compile_source("x = y$", "t").is_err());
    }

    #[test]
    fn logical_and_rejected() {
        assert!(compile_source("x and y$", "t").is_err());
    }

    #[test]
    fn power_rejected() {
        assert!(compile_source("x^2$", "t").is_err());
    }

    #[test]
    fn string_literal_rejected() {
        assert!(compile_source("\"hi\"$", "t").is_err());
    }
}
