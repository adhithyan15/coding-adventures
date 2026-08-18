//! # derive-iir-compiler
//!
//! Derive CST → `interpreter_ir::IIRModule`, **v0.1.0**.
//!
//! The second real-language frontend (after Macsyma) to bridge a math
//! language in this repo onto `interpreter_ir` (IIR) — the shared IR the
//! AOT/lang-vm chain lowers to 7 real backends (NativeAOT, LLVM, WASM,
//! JVM, CLR, VM interpreter, JIT) — rather than only the Semantic IR
//! source-to-source pipeline `derive-to-semantic-ir` already targets. See
//! [`derive-iir-vm.md`](../../../specs/derive-iir-vm.md) for the
//! per-language deltas from `macsyma-iir-vm.md`'s original design and
//! `lower.rs`'s module doc comment for the v0 scope and the `/`
//! exactness rule. It consumes the generic `GrammarASTNode` CST produced
//! by `coding-adventures-derive-parser` and emits an
//! [`interpreter_ir::IIRModule`], executable on
//! [`derive-vm`](../derive-vm) (v0 targets the VM interpreter backend
//! only).
//!
//! ## Pipeline
//!
//! ```text
//! Derive source
//!    │
//!    ▼  coding_adventures_derive_parser::try_parse_derive(src)
//! parser::grammar_parser::GrammarASTNode   (generic CST)
//!    │
//!    ▼  derive_iir_compiler::compile
//! interpreter_ir::IIRModule                (v0 subset)
//!    │
//!    ▼  derive_vm::run
//! dynval_runtime::LispyValue
//! ```
//!
//! ## Public API
//!
//! ```
//! use derive_iir_compiler::compile_source;
//! let module = compile_source("2 + 3\n", "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```

mod lower;
pub use lower::{compile, DeriveIirError};

/// Parse `source` as Derive and lower it into an
/// [`interpreter_ir::IIRModule`] in one step, mirroring
/// `derive-to-semantic-ir::compile_source`'s convenience wrapper.
///
/// Needs no worker-thread stack enlargement, for the same reason
/// `derive-to-semantic-ir::compile_source` doesn't:
/// `coding_adventures_derive_parser`'s own `MAX_RULE_DEPTH` (200) is
/// already documented safe on a bare default (~2 MiB) stack.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<interpreter_ir::IIRModule, DeriveIirError> {
    let tree = coding_adventures_derive_parser::try_parse_derive(source).map_err(|msg| {
        DeriveIirError {
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
    use derive_vm::run;

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
        assert_eq!(eval_int("x := 3\nx + 1\n"), 4);
    }

    #[test]
    fn reassignment_threading() {
        assert_eq!(eval_int("x := 3\nx := x + 1\nx\n"), 4);
    }

    #[test]
    fn two_variables() {
        assert_eq!(eval_int("a := 2\nb := 3\na * b\n"), 6);
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
        assert!(compile_source("x := 6\nx / 2\n", "t").is_err());
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
    fn function_definition_rejected() {
        let err = compile_source("F(x) := x + 1\n", "t").unwrap_err();
        assert!(
            err.message.contains("function definition"),
            "message was: {}",
            err.message
        );
    }

    #[test]
    fn function_call_rejected() {
        assert!(compile_source("SIN(x)\n", "t").is_err());
    }

    #[test]
    fn comparison_rejected() {
        assert!(compile_source("x = y\n", "t").is_err());
    }

    #[test]
    fn logical_and_rejected() {
        assert!(compile_source("x AND y\n", "t").is_err());
    }

    #[test]
    fn power_rejected() {
        assert!(compile_source("x^2\n", "t").is_err());
    }

    #[test]
    fn vector_literal_rejected() {
        assert!(compile_source("[1, 2, 3]\n", "t").is_err());
    }
}
