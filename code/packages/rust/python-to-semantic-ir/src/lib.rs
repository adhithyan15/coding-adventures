//! # python-to-semantic-ir
//!
//! Python CST → narrow-waist Semantic IR (SIR17), **milestone M1**.
//!
//! This is the second frontend for the SIR10 narrow-waist IR (after
//! [`twig-to-semantic-ir`]).  It consumes the generic
//! [`GrammarASTNode`] CST produced by the
//! [`coding_adventures_python_parser`] crate and emits a
//! [`semantic_ir::Module`].
//!
//! ## Pipeline
//!
//! ```text
//! Python source
//!    │
//!    ▼  coding_adventures_python_parser::parse_python(src, "3.10")
//! parser::grammar_parser::GrammarASTNode   (generic CST)
//!    │
//!    ▼  python_to_semantic_ir::compile
//! semantic_ir::Module                      (per SIR10 + SIR16)
//! ```
//!
//! ## Public API
//!
//! ```ignore
//! use python_to_semantic_ir::{compile_source, PythonLowerError};
//! let module = compile_source("42\n", "demo")?;
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```
//!
//! ## M1 scope
//!
//! M1 lowers **literals only** — int, float, `True`/`False`, `None`,
//! and strings — wrapped in a synthesised `main` function.  Variable
//! references, assignment, operators, control flow, functions, and
//! collections are deferred to later milestones; unhandled forms
//! return a clear `PythonLowerError` (`"unsupported in M1: …"`).
//!
//! See `code/specs/SIR17-python-to-semantic-ir.md` for the full
//! lowering table and the deferred-form roadmap.

mod lower;

pub use lower::{compile, PythonLowerError};

/// Convenience: parse Python source (version `"3.10"`) and lower it to
/// SIR in one call.
///
/// Parse errors and lower errors are both surfaced as
/// [`PythonLowerError`].  The underlying parser returns a flat
/// `String` for parse failures (with no structured position), so parse
/// errors are reported at `1:1` with the parser's message inlined.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, PythonLowerError> {
    let tree = coding_adventures_python_parser::parse_python(source, "3.10").map_err(|msg| {
        PythonLowerError {
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
    use semantic_ir::{Expr, Feature, Stmt};

    /// Lower a snippet, asserting success, and return the module.
    fn lower(src: &str) -> semantic_ir::Module {
        compile_source(src, "test").expect("lowering succeeded")
    }

    /// Fetch `main`'s block value expression.
    fn main_value(m: &semantic_ir::Module) -> &Expr {
        &m.functions
            .iter()
            .find(|f| f.name == "main")
            .expect("main exists")
            .body
            .value
    }

    // ── one positive test per literal kind ────────────────────────────

    #[test]
    fn int_literal_lowers_to_int_lit() {
        let m = lower("42\n");
        match main_value(&m) {
            Expr::IntLit { value, .. } => assert_eq!(*value, 42),
            other => panic!("expected IntLit, got {other:?}"),
        }
    }

    #[test]
    fn negative_int_literal_constant_folds() {
        let m = lower("-7\n");
        match main_value(&m) {
            Expr::IntLit { value, .. } => assert_eq!(*value, -7),
            other => panic!("expected IntLit(-7), got {other:?}"),
        }
    }

    #[test]
    fn float_literal_lowers_to_float_lit_and_sets_feature() {
        let m = lower("3.25\n");
        match main_value(&m) {
            Expr::FloatLit { value, .. } => assert!((*value - 3.25).abs() < 1e-12),
            other => panic!("expected FloatLit, got {other:?}"),
        }
        assert!(
            m.manifest.contains(Feature::Floats),
            "float literal must declare the Floats feature"
        );
    }

    #[test]
    fn negative_float_literal_constant_folds() {
        let m = lower("-2.5\n");
        match main_value(&m) {
            Expr::FloatLit { value, .. } => assert!((*value + 2.5).abs() < 1e-12),
            other => panic!("expected FloatLit(-2.5), got {other:?}"),
        }
    }

    #[test]
    fn true_literal_lowers_to_bool_lit() {
        let m = lower("True\n");
        match main_value(&m) {
            Expr::BoolLit { value, .. } => assert!(*value),
            other => panic!("expected BoolLit(true), got {other:?}"),
        }
    }

    #[test]
    fn false_literal_lowers_to_bool_lit() {
        let m = lower("False\n");
        match main_value(&m) {
            Expr::BoolLit { value, .. } => assert!(!*value),
            other => panic!("expected BoolLit(false), got {other:?}"),
        }
    }

    #[test]
    fn none_literal_lowers_to_nil_lit() {
        let m = lower("None\n");
        match main_value(&m) {
            Expr::NilLit { .. } => {}
            other => panic!("expected NilLit, got {other:?}"),
        }
    }

    #[test]
    fn string_literal_lowers_to_str_lit_and_sets_feature() {
        let m = lower("\"hi\"\n");
        match main_value(&m) {
            Expr::StrLit { value, .. } => assert_eq!(value, "hi"),
            other => panic!("expected StrLit, got {other:?}"),
        }
        assert!(
            m.manifest.contains(Feature::Strings),
            "string literal must declare the Strings feature"
        );
    }

    #[test]
    fn single_quoted_string_literal_lowers() {
        let m = lower("'world'\n");
        match main_value(&m) {
            Expr::StrLit { value, .. } => assert_eq!(value, "world"),
            other => panic!("expected StrLit, got {other:?}"),
        }
    }

    // ── top-level structure ──────────────────────────────────────────

    #[test]
    fn empty_program_yields_main_returning_nil() {
        let m = lower("");
        assert!(m.functions.iter().any(|f| f.name == "main"));
        match main_value(&m) {
            Expr::NilLit { .. } => {}
            other => panic!("expected NilLit for empty program, got {other:?}"),
        }
    }

    #[test]
    fn multiple_statements_last_is_value_earlier_are_exprstmts() {
        let m = lower("1\n2\n3\n");
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        // Final expression is the block value …
        match &main.body.value {
            Expr::IntLit { value, .. } => assert_eq!(*value, 3),
            other => panic!("expected IntLit(3), got {other:?}"),
        }
        // … and the two earlier ones are ExprStmts.
        assert_eq!(main.body.stmts.len(), 2);
        for stmt in &main.body.stmts {
            assert!(matches!(stmt, Stmt::ExprStmt { .. }));
        }
    }

    #[test]
    fn metadata_records_python_and_sir_version() {
        let m = lower("42\n");
        assert_eq!(m.metadata.source_language.as_deref(), Some("python"));
        assert_eq!(
            m.metadata.sir_version.as_deref(),
            Some(semantic_ir::CURRENT_SIR_VERSION)
        );
    }

    #[test]
    fn minimal_program_declares_no_extra_features() {
        // An int-only program needs no feature flags.
        let m = lower("42\n");
        assert!(!m.manifest.contains(Feature::Floats));
        assert!(!m.manifest.contains(Feature::Strings));
    }

    // ── validator round-trip ─────────────────────────────────────────

    #[test]
    fn lowered_modules_pass_the_validator() {
        for src in ["", "42\n", "3.25\n", "True\n", "None\n", "\"hi\"\n", "-7\n", "1\n2\n"] {
            let m = lower(src);
            let r = semantic_ir::validate(&m);
            assert!(r.is_ok(), "module for {src:?} failed validation: {:?}", r.issues);
        }
    }

    // ── error paths ──────────────────────────────────────────────────

    #[test]
    fn assignment_is_unsupported_in_m1() {
        let err = compile_source("x = 1\n", "t").expect_err("assignment rejected");
        assert!(
            err.message.contains("unsupported in M1"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn bare_name_reference_is_unsupported_in_m1() {
        let err = compile_source("x\n", "t").expect_err("name ref rejected");
        assert!(err.message.contains("unsupported in M1"), "got: {}", err.message);
    }

    #[test]
    fn operator_expression_is_unsupported_in_m1() {
        let err = compile_source("1 + 2\n", "t").expect_err("operator rejected");
        assert!(err.message.contains("unsupported in M1"), "got: {}", err.message);
    }

    #[test]
    fn parse_error_is_surfaced() {
        let err = compile_source("def\n", "t").expect_err("parse error rejected");
        assert!(err.message.contains("parse error"), "got: {}", err.message);
    }

    #[test]
    fn error_carries_position() {
        // The error position is taken from the offending node; just
        // assert it is populated (1-based) rather than a fixed value.
        let err = compile_source("x = 1\n", "t").unwrap_err();
        assert!(err.line >= 1);
        assert!(err.column >= 1);
    }
}
