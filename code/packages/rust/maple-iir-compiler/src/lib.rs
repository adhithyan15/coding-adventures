//! # maple-iir-compiler
//!
//! Maple CST → `interpreter_ir::IIRModule`, **v0.1.0**.
//!
//! The fourth real-language frontend (after Macsyma, Derive, and Reduce)
//! to bridge a math language in this repo onto `interpreter_ir` (IIR).
//! See [`maple-iir-vm.md`](../../../specs/maple-iir-vm.md) for the
//! per-language deltas from `macsyma-iir-vm.md`'s original design and
//! `lower.rs`'s module doc comment for the v0 scope. It consumes the
//! generic `GrammarASTNode` CST produced by
//! `coding-adventures-maple-parser` and emits an
//! [`interpreter_ir::IIRModule`], executable on
//! [`maple-vm`](../maple-vm) (v0 targets the VM interpreter backend
//! only).
//!
//! ## Pipeline
//!
//! ```text
//! Maple source
//!    │
//!    ▼  coding_adventures_maple_parser::try_parse_maple(src)
//! parser::grammar_parser::GrammarASTNode   (generic CST)
//!    │
//!    ▼  maple_iir_compiler::compile
//! interpreter_ir::IIRModule                (v0 subset)
//!    │
//!    ▼  maple_vm::run
//! dynval_runtime::LispyValue
//! ```
//!
//! ## Public API
//!
//! ```
//! use maple_iir_compiler::compile_source;
//! let module = compile_source("2 + 3;\n", "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```

mod lower;
pub use lower::{compile, MapleIirError};

/// Parse `source` as Maple and lower it into an
/// [`interpreter_ir::IIRModule`] in one step, mirroring
/// `maple-to-semantic-ir::compile_source`'s convenience wrapper.
///
/// Needs no worker-thread stack enlargement, for the same reason
/// `maple-to-semantic-ir::compile_source` doesn't:
/// `coding_adventures_maple_parser`'s own `MAX_RULE_DEPTH` (150) is
/// already documented safe on a bare default (~2 MiB) stack.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<interpreter_ir::IIRModule, MapleIirError> {
    let tree =
        coding_adventures_maple_parser::try_parse_maple(source).map_err(|msg| MapleIirError {
            message: format!("parse error: {msg}"),
            line: 1,
            column: 1,
        })?;
    compile(&tree, module_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maple_vm::run;

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
        assert_eq!(eval_int("42;\n"), 42);
    }

    #[test]
    fn simple_addition() {
        assert_eq!(eval_int("2 + 3;\n"), 5);
    }

    #[test]
    fn precedence() {
        assert_eq!(eval_int("2 + 3 * 4;\n"), 14);
    }

    #[test]
    fn chain() {
        assert_eq!(eval_int("1 + 2 + 3 + 4;\n"), 10);
    }

    #[test]
    fn unary_neg_leaf() {
        assert_eq!(eval_int("-5 + 3;\n"), -2);
    }

    #[test]
    fn unary_neg_compound() {
        assert_eq!(eval_int("-(5 + 3);\n"), -8);
    }

    #[test]
    fn grouping() {
        assert_eq!(eval_int("(2 + 3) * 4;\n"), 20);
    }

    #[test]
    fn exact_division() {
        assert_eq!(eval_int("20 / 4;\n"), 5);
    }

    #[test]
    fn negative_literal_exact_division() {
        assert_eq!(eval_int("-4 / 2;\n"), -2);
    }

    // ---- accepted: assignment ----

    #[test]
    fn assignment_and_reference() {
        assert_eq!(eval_int("x := 3;\nx + 1;\n"), 4);
    }

    #[test]
    fn reassignment_threading() {
        assert_eq!(eval_int("x := 3;\nx := x + 1;\nx;\n"), 4);
    }

    #[test]
    fn two_variables() {
        assert_eq!(eval_int("a := 2;\nb := 3;\na * b;\n"), 6);
    }

    #[test]
    fn colon_statement_terminator_also_works() {
        // `;` (display) vs `:` (suppress) is a session-display concept
        // this frontend deliberately ignores -- both compile identically.
        assert_eq!(eval_int("x := 3:\nx + 1:\n"), 4);
    }

    // ---- accepted: unevaluated symbolic expressions ----

    #[test]
    fn free_symbol_addition_is_symbolic() {
        let module = compile_source("x + y;\n", "t").expect("compile");
        let v = run(&module).expect("run");
        assert!(v.as_int().is_none());
    }

    #[test]
    fn free_symbol_addition_head_is_add() {
        let module = compile_source("x + y;\n", "t").expect("compile");
        let v = run(&module).expect("run");
        let head = dynval_runtime::builtins::car(&[v]).unwrap();
        assert_eq!(
            dynval_runtime::name_of(head.as_symbol().unwrap()).as_deref(),
            Some("Add")
        );
    }

    #[test]
    fn mixed_concrete_and_symbolic_operand() {
        let module = compile_source("2 * x;\n", "t").expect("compile");
        let v = run(&module).expect("run");
        let head = dynval_runtime::builtins::car(&[v]).unwrap();
        assert_eq!(
            dynval_runtime::name_of(head.as_symbol().unwrap()).as_deref(),
            Some("Mul")
        );
    }

    #[test]
    fn free_symbol_alone() {
        assert_eq!(eval_symbol("x;\n"), "x");
    }

    // ---- rejected: explicit-error paths ----

    #[test]
    fn float_literal_rejected() {
        assert!(compile_source("1.5;\n", "t").is_err());
    }

    #[test]
    fn non_exact_division_rejected() {
        let err = compile_source("7 / 2;\n", "t").unwrap_err();
        assert!(
            err.message.contains("rational"),
            "message was: {}",
            err.message
        );
    }

    #[test]
    fn division_of_non_literal_concrete_rejected() {
        assert!(compile_source("x := 6;\nx / 2;\n", "t").is_err());
    }

    #[test]
    fn division_by_literal_zero_rejected() {
        assert!(compile_source("1 / 0;\n", "t").is_err());
    }

    #[test]
    fn i64_min_divided_by_neg_one_rejected_not_panicking() {
        assert!(compile_source("(-9223372036854775807 - 1) / -1;\n", "t").is_err());
    }

    #[test]
    fn arrow_function_definition_rejected() {
        let err = compile_source("f := x -> x + 1;\n", "t").unwrap_err();
        assert!(
            err.message.contains("arrow function definition"),
            "message was: {}",
            err.message
        );
    }

    #[test]
    fn function_call_rejected() {
        assert!(compile_source("diff(x, x);\n", "t").is_err());
    }

    #[test]
    fn comparison_rejected() {
        assert!(compile_source("x = y;\n", "t").is_err());
    }

    #[test]
    fn logical_and_rejected() {
        assert!(compile_source("x and y;\n", "t").is_err());
    }

    #[test]
    fn power_rejected() {
        assert!(compile_source("x^2;\n", "t").is_err());
    }

    #[test]
    fn list_literal_rejected() {
        assert!(compile_source("[1, 2, 3];\n", "t").is_err());
    }

    #[test]
    fn set_literal_rejected() {
        assert!(compile_source("{1, 2, 3};\n", "t").is_err());
    }

    #[test]
    fn boolean_literal_rejected() {
        assert!(compile_source("true;\n", "t").is_err());
    }

    #[test]
    fn if_expr_rejected() {
        assert!(compile_source("if x > 0 then 1 else 0 end if;\n", "t").is_err());
    }

    #[test]
    fn postfix_call_is_not_chainable() {
        // maple.grammar's own `postfix` has a single OPTIONAL call
        // suffix, so `f(x)(y)` is a parse error, not a lowering error.
        assert!(compile_source("f(x)(y);\n", "t").is_err());
    }
}
