//! # python-to-semantic-ir
//!
//! Python CST → narrow-waist Semantic IR (SIR17), **milestone M2**.
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
//! ## M2 scope
//!
//! M1 lowered **literals only**.  M2 adds, still wrapped in a
//! synthesised `main` function:
//!
//! - **variable references** — a bare `x` becomes a `VarRef` whose
//!   `scope` is resolved (`Local` when bound, error otherwise);
//! - **assignment** — `x = expr` is *first-occurrence* detected: the
//!   first binding of a name declares it (`LetStarBinding`), a later
//!   assignment re-binds it (`Assign`);
//! - **operators** — arithmetic (`+ - * / %`) and comparison
//!   (`== != < > <= >=`) lower to `BuiltinCall`; unary `not`/`-` to
//!   `BuiltinCall("not"/"neg")` (with `-<literal>` constant-folded);
//!   `and`/`or` to the short-circuit `LogicalAnd`/`LogicalOr` nodes.
//!
//! Control flow, functions/`def`/`lambda`, calls, and collections are
//! deferred to later milestones; unhandled forms return a clear
//! `PythonLowerError`.
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
    use semantic_ir::{Expr, Feature, Scope, Stmt};

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

    /// Fetch `main`'s block statements.
    fn main_stmts(m: &semantic_ir::Module) -> &[Stmt] {
        &m.functions
            .iter()
            .find(|f| f.name == "main")
            .expect("main exists")
            .body
            .stmts
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
        for src in [
            "",
            "42\n",
            "3.25\n",
            "True\n",
            "None\n",
            "\"hi\"\n",
            "-7\n",
            "1\n2\n",
            // M2 constructs:
            "x = 1\n",
            "x = 1\nx = 2\n",
            "x = 1\ny = x + 1\n",
            "x = 5\ny = x * 2 + 3\n",
            "a = 1\nb = 2\nc = a < b\n",
            "p = True\nq = False\nr = p and q\n",
            "m = 1\nn = not m\n",
            "x = 10\ny = -x\n",
            "a = 1\nb = a == a\n",
            "a = 1\nb = a % 2\n",
        ] {
            let m = lower(src);
            let r = semantic_ir::validate(&m);
            assert!(r.is_ok(), "module for {src:?} failed validation: {:?}", r.issues);
        }
    }

    // ── M2: variable references + assignment ──────────────────────────

    #[test]
    fn first_assignment_is_let_star_binding() {
        let m = lower("x = 1\n");
        match &main_stmts(&m)[0] {
            Stmt::LetStarBinding { name, value, .. } => {
                assert_eq!(name, "x");
                assert!(matches!(value, Expr::IntLit { value: 1, .. }));
            }
            other => panic!("expected LetStarBinding, got {other:?}"),
        }
        // A trailing assignment yields a NilLit block value.
        assert!(matches!(main_value(&m), Expr::NilLit { .. }));
    }

    #[test]
    fn reassignment_becomes_assign_and_sets_mutable_feature() {
        let m = lower("x = 1\nx = 2\n");
        let stmts = main_stmts(&m);
        assert!(matches!(stmts[0], Stmt::LetStarBinding { .. }));
        match &stmts[1] {
            Stmt::Assign { name, scope, value, .. } => {
                assert_eq!(name, "x");
                assert_eq!(*scope, Scope::Local);
                assert!(matches!(value, Expr::IntLit { value: 2, .. }));
            }
            other => panic!("expected Assign, got {other:?}"),
        }
        assert!(
            m.manifest.contains(Feature::MutableBindings),
            "reassignment must declare MutableBindings"
        );
    }

    #[test]
    fn variable_reference_resolves_to_local() {
        // `x` is bound, then referenced as the trailing value.
        let m = lower("x = 1\nx\n");
        match main_value(&m) {
            Expr::VarRef { name, scope, .. } => {
                assert_eq!(name, "x");
                assert_eq!(*scope, Scope::Local);
            }
            other => panic!("expected VarRef, got {other:?}"),
        }
    }

    #[test]
    fn let_then_reference_in_rhs_resolves() {
        // `y = x + 1` must see the prior `x` binding (sequential let*).
        let m = lower("x = 2\ny = x + 1\n");
        match &main_stmts(&m)[1] {
            Stmt::LetStarBinding { name, value, .. } => {
                assert_eq!(name, "y");
                match value {
                    Expr::BuiltinCall { name, args, .. } => {
                        assert_eq!(name, "+");
                        assert!(matches!(args[0], Expr::VarRef { scope: Scope::Local, .. }));
                        assert!(matches!(args[1], Expr::IntLit { value: 1, .. }));
                    }
                    other => panic!("expected BuiltinCall(+), got {other:?}"),
                }
            }
            other => panic!("expected LetStarBinding, got {other:?}"),
        }
    }

    #[test]
    fn unresolved_name_is_an_error() {
        let err = compile_source("x\n", "t").expect_err("unresolved name rejected");
        assert!(
            err.message.contains("unresolved name `x`"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn self_reference_first_binding_is_unresolved() {
        // `x = x` — the RHS `x` is not yet bound, so it's unresolved.
        let err = compile_source("x = x\n", "t").expect_err("self-ref rejected");
        assert!(err.message.contains("unresolved name"), "got: {}", err.message);
    }

    // ── M2: binary arithmetic operators ───────────────────────────────

    #[test]
    fn arithmetic_operators_lower_to_builtin_calls() {
        for (src, op) in [
            ("a = 1\na + 1\n", "+"),
            ("a = 1\na - 1\n", "-"),
            ("a = 1\na * 2\n", "*"),
            ("a = 1\na / 2\n", "/"),
            ("a = 1\na % 2\n", "%"),
        ] {
            let m = lower(src);
            match main_value(&m) {
                Expr::BuiltinCall { name, args, .. } => {
                    assert_eq!(name, op, "for {src:?}");
                    assert_eq!(args.len(), 2);
                }
                other => panic!("expected BuiltinCall({op}) for {src:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn arithmetic_is_left_associative() {
        // `a - b - c` → ((a - b) - c).
        let m = lower("a = 1\nb = 1\nc = 1\na - b - c\n");
        match main_value(&m) {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "-");
                // lhs is itself a (a - b) call; rhs is c.
                assert!(matches!(&args[0], Expr::BuiltinCall { name, .. } if name == "-"));
                assert!(matches!(&args[1], Expr::VarRef { name, .. } if name == "c"));
            }
            other => panic!("expected nested BuiltinCall(-), got {other:?}"),
        }
    }

    #[test]
    fn precedence_multiply_binds_tighter_than_add() {
        // `a + b * c` → a + (b * c).
        let m = lower("a = 1\nb = 1\nc = 1\na + b * c\n");
        match main_value(&m) {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "+");
                assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "a"));
                assert!(matches!(&args[1], Expr::BuiltinCall { name, .. } if name == "*"));
            }
            other => panic!("expected BuiltinCall(+), got {other:?}"),
        }
    }

    // ── M2: comparison operators ──────────────────────────────────────

    #[test]
    fn comparison_operators_lower_to_builtin_calls() {
        for (src, op) in [
            ("a = 1\na < 1\n", "<"),
            ("a = 1\na > 1\n", ">"),
            ("a = 1\na <= 1\n", "<="),
            ("a = 1\na >= 1\n", ">="),
            ("a = 1\na == 1\n", "="),  // == maps to "="
            ("a = 1\na != 1\n", "!="),
        ] {
            let m = lower(src);
            match main_value(&m) {
                Expr::BuiltinCall { name, args, .. } => {
                    assert_eq!(name, op, "for {src:?}");
                    assert_eq!(args.len(), 2);
                }
                other => panic!("expected BuiltinCall({op}) for {src:?}, got {other:?}"),
            }
        }
    }

    // ── M2: unary operators ───────────────────────────────────────────

    #[test]
    fn unary_not_lowers_to_builtin_not() {
        let m = lower("c = True\nnot c\n");
        match main_value(&m) {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "not");
                assert_eq!(args.len(), 1);
                assert!(matches!(args[0], Expr::VarRef { .. }));
            }
            other => panic!("expected BuiltinCall(not), got {other:?}"),
        }
    }

    #[test]
    fn unary_minus_on_variable_lowers_to_neg() {
        let m = lower("d = 1\n-d\n");
        match main_value(&m) {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "neg");
                assert_eq!(args.len(), 1);
                assert!(matches!(args[0], Expr::VarRef { .. }));
            }
            other => panic!("expected BuiltinCall(neg), got {other:?}"),
        }
    }

    #[test]
    fn unary_minus_on_literal_still_constant_folds() {
        // Carried from M1: `-7` folds to IntLit(-7), not BuiltinCall(neg).
        let m = lower("-7\n");
        assert!(matches!(main_value(&m), Expr::IntLit { value: -7, .. }));
    }

    #[test]
    fn unary_plus_is_identity() {
        let m = lower("e = 1\n+e\n");
        // `+e` returns the operand unchanged — a VarRef, no builtin.
        assert!(matches!(main_value(&m), Expr::VarRef { name, .. } if name == "e"));
    }

    // ── M2: short-circuit logical operators ───────────────────────────

    #[test]
    fn logical_and_lowers_to_short_circuit_node() {
        let m = lower("a = True\nb = False\na and b\n");
        match main_value(&m) {
            Expr::LogicalAnd { lhs, rhs, .. } => {
                assert!(matches!(**lhs, Expr::VarRef { .. }));
                assert!(matches!(**rhs, Expr::VarRef { .. }));
            }
            other => panic!("expected LogicalAnd, got {other:?}"),
        }
        assert!(m.manifest.contains(Feature::ShortCircuit));
    }

    #[test]
    fn logical_or_lowers_to_short_circuit_node() {
        let m = lower("a = True\nb = False\na or b\n");
        assert!(matches!(main_value(&m), Expr::LogicalOr { .. }));
        assert!(m.manifest.contains(Feature::ShortCircuit));
    }

    #[test]
    fn logical_and_is_left_nested() {
        // `a and b and c` → (a and b) and c.
        let m = lower("a = True\nb = True\nc = True\na and b and c\n");
        match main_value(&m) {
            Expr::LogicalAnd { lhs, rhs, .. } => {
                assert!(matches!(**lhs, Expr::LogicalAnd { .. }));
                assert!(matches!(&**rhs, Expr::VarRef { name, .. } if name == "c"));
            }
            other => panic!("expected nested LogicalAnd, got {other:?}"),
        }
    }

    #[test]
    fn no_operator_program_declares_no_short_circuit() {
        let m = lower("x = 1\nx + 1\n");
        assert!(!m.manifest.contains(Feature::ShortCircuit));
        assert!(!m.manifest.contains(Feature::MutableBindings));
    }

    // ── error paths ──────────────────────────────────────────────────

    #[test]
    fn global_statement_is_unsupported() {
        let err = compile_source("global x\n", "t").expect_err("global rejected");
        assert!(err.message.contains("unsupported"), "got: {}", err.message);
    }

    #[test]
    fn parse_error_is_surfaced() {
        let err = compile_source("def\n", "t").expect_err("parse error rejected");
        assert!(err.message.contains("parse error"), "got: {}", err.message);
    }

    #[test]
    fn error_carries_position() {
        // A reference on line 2 should report line 2.
        let err = compile_source("x = 1\nzzz\n", "t").unwrap_err();
        assert_eq!(err.line, 2, "got {}:{}", err.line, err.column);
        assert!(err.column >= 1);
    }
}
