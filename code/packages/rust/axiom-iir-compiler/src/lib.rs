//! # axiom-iir-compiler
//!
//! Axiom CST → `interpreter_ir::IIRModule`, **v0.1.0**.
//!
//! The fifth real-language frontend (after Macsyma, Derive, Reduce, and
//! Maple) to bridge a math language in this repo onto `interpreter_ir`
//! (IIR). See [`axiom-iir-vm.md`](../../../specs/axiom-iir-vm.md) for the
//! per-language deltas from `macsyma-iir-vm.md`'s original design and
//! `lower.rs`'s module doc comment for the v0 scope. It consumes the
//! generic `GrammarASTNode` CST produced by
//! `coding-adventures-axiom-parser` and emits an
//! [`interpreter_ir::IIRModule`], executable on
//! [`axiom-vm`](../axiom-vm) (v0 targets the VM interpreter backend
//! only).
//!
//! **`program` is a SINGLE expression** — `axiom.grammar`'s own `program
//! = expr` (Axiom is modeled as a numbered, per-line interactive
//! session), unlike every sibling frontend's multi-statement worksheet
//! loop. See `lower.rs`'s module doc comment for the full consequence
//! (no `env`/binding-threading needed at all).
//!
//! ## Pipeline
//!
//! ```text
//! Axiom source (one expression)
//!    │
//!    ▼  coding_adventures_axiom_parser::try_parse_axiom(src)
//! parser::grammar_parser::GrammarASTNode   (generic CST)
//!    │
//!    ▼  axiom_iir_compiler::compile
//! interpreter_ir::IIRModule                (v0 subset)
//!    │
//!    ▼  axiom_vm::run
//! dynval_runtime::LispyValue
//! ```
//!
//! ## Public API
//!
//! ```
//! use axiom_iir_compiler::compile_source;
//! let module = compile_source("2 + 3", "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```

mod lower;
pub use lower::{compile, AxiomIirError};

/// Parse `source` as a single Axiom expression and lower it into an
/// [`interpreter_ir::IIRModule`] in one step, mirroring
/// `axiom-to-semantic-ir::compile_source`'s convenience wrapper.
///
/// Needs no worker-thread stack enlargement, for the same reason
/// `axiom-to-semantic-ir::compile_source` doesn't:
/// `coding_adventures_axiom_parser`'s own `MAX_RULE_DEPTH` (140) is
/// already documented safe on a bare default (~2 MiB) stack.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<interpreter_ir::IIRModule, AxiomIirError> {
    let tree =
        coding_adventures_axiom_parser::try_parse_axiom(source).map_err(|msg| AxiomIirError {
            message: format!("parse error: {msg}"),
            line: 1,
            column: 1,
        })?;
    compile(&tree, module_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_vm::run;

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
        assert_eq!(eval_int("42"), 42);
    }

    #[test]
    fn simple_addition() {
        assert_eq!(eval_int("2 + 3"), 5);
    }

    #[test]
    fn precedence() {
        assert_eq!(eval_int("2 + 3 * 4"), 14);
    }

    #[test]
    fn chain() {
        assert_eq!(eval_int("1 + 2 + 3 + 4"), 10);
    }

    #[test]
    fn unary_neg_leaf() {
        assert_eq!(eval_int("-5 + 3"), -2);
    }

    #[test]
    fn unary_neg_compound() {
        assert_eq!(eval_int("-(5 + 3)"), -8);
    }

    #[test]
    fn grouping() {
        assert_eq!(eval_int("(2 + 3) * 4"), 20);
    }

    #[test]
    fn exact_division() {
        assert_eq!(eval_int("20 / 4"), 5);
    }

    #[test]
    fn negative_literal_exact_division() {
        assert_eq!(eval_int("-4 / 2"), -2);
    }

    // ---- accepted: assignment (single expression -- see the module doc
    // comment: there is no second statement to reference the binding). ----

    #[test]
    fn assignment_returns_the_assigned_value() {
        assert_eq!(eval_int("x := 3"), 3);
    }

    #[test]
    fn assignment_with_an_arithmetic_rhs() {
        assert_eq!(eval_int("x := 2 + 3"), 5);
    }

    // ---- accepted: unevaluated symbolic expressions ----

    #[test]
    fn free_symbol_addition_is_symbolic() {
        let module = compile_source("x + y", "t").expect("compile");
        let v = run(&module).expect("run");
        assert!(v.as_int().is_none());
    }

    #[test]
    fn free_symbol_addition_head_is_add() {
        let module = compile_source("x + y", "t").expect("compile");
        let v = run(&module).expect("run");
        let head = dynval_runtime::builtins::car(&[v]).unwrap();
        assert_eq!(
            dynval_runtime::name_of(head.as_symbol().unwrap()).as_deref(),
            Some("Add")
        );
    }

    #[test]
    fn mixed_concrete_and_symbolic_operand() {
        let module = compile_source("2 * x", "t").expect("compile");
        let v = run(&module).expect("run");
        let head = dynval_runtime::builtins::car(&[v]).unwrap();
        assert_eq!(
            dynval_runtime::name_of(head.as_symbol().unwrap()).as_deref(),
            Some("Mul")
        );
    }

    #[test]
    fn free_symbol_alone() {
        assert_eq!(eval_symbol("x"), "x");
    }

    // ---- rejected: explicit-error paths ----

    #[test]
    fn float_literal_rejected() {
        assert!(compile_source("1.5", "t").is_err());
    }

    #[test]
    fn string_literal_rejected() {
        assert!(compile_source("\"hi\"", "t").is_err());
    }

    #[test]
    fn non_exact_division_rejected() {
        let err = compile_source("7 / 2", "t").unwrap_err();
        assert!(
            err.message.contains("rational"),
            "message was: {}",
            err.message
        );
    }

    #[test]
    fn division_of_a_concrete_but_non_literal_value_rejected() {
        // Axiom's single-expression v0 has no `env`, so there is no
        // `x := 6 / x / 2`-style test available the way sibling crates
        // have. `9223372036854775807 + 1` overflows `checked_add`, so
        // `combine`'s literal-tracking degrades to `concrete: true,
        // literal: None` (the real `call_builtin "+"` is still emitted
        // for the VM to execute) -- dividing that by a literal exercises
        // the "concrete but not a direct literal" rejection path, the one
        // shape of this case reachable without assignment threading.
        assert!(compile_source("(9223372036854775807 + 1) / 2", "t").is_err());
    }

    #[test]
    fn division_by_literal_zero_rejected() {
        assert!(compile_source("1 / 0", "t").is_err());
    }

    #[test]
    fn i64_min_divided_by_neg_one_rejected_not_panicking() {
        assert!(compile_source("(-9223372036854775807 - 1) / -1", "t").is_err());
    }

    #[test]
    fn function_definition_rejected() {
        assert!(compile_source("f(x: Integer): Integer == x + 1", "t").is_err());
    }

    #[test]
    fn function_call_rejected() {
        assert!(compile_source("f(x)", "t").is_err());
    }

    #[test]
    fn comparison_rejected() {
        assert!(compile_source("x = y", "t").is_err());
    }

    #[test]
    fn power_rejected() {
        assert!(compile_source("x^2", "t").is_err());
    }

    #[test]
    fn list_literal_rejected() {
        assert!(compile_source("[1, 2, 3]", "t").is_err());
    }

    #[test]
    fn declaration_rejected() {
        assert!(compile_source("x : Integer", "t").is_err());
    }

    #[test]
    fn coercion_rejected() {
        assert!(compile_source("x :: Float", "t").is_err());
    }

    #[test]
    fn has_query_rejected() {
        assert!(compile_source("Integer has Ring", "t").is_err());
    }

    #[test]
    fn if_expr_rejected() {
        assert!(compile_source("if x > 0 then 1 else 0", "t").is_err());
    }
}
