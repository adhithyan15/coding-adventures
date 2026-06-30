//! # python-to-semantic-ir
//!
//! Python CST → narrow-waist Semantic IR (SIR17), **milestone M3**.
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
//! ## M3 scope
//!
//! M1 lowered **literals only**; M2 added variable references,
//! assignment, and unary/binary operators.  M3 adds **control flow**,
//! still wrapped in the synthesised `main` function:
//!
//! - **`if` / `elif` / `else`** → `Expr::If` (an `elif` chain folds
//!   right-to-left into nested `If`s; a missing `else` yields an empty
//!   nil-valued block).  Since `if` is a SIR *expression*, a trailing
//!   `if` becomes the block value; otherwise it is a `Stmt::ExprStmt`;
//! - **`while c: body`** → `Stmt::While { cond, body }`;
//! - **`for x in range(...): body`** → `Stmt::ForRange` (1/2/3-arg
//!   `range` mapped to `start`/`stop`/`step`; wrong arity rejected);
//! - **`for x in <iter>: body`** → `Stmt::ForEach`.
//!
//! Each loop / branch suite lowers to a `Block`; the loop variable is a
//! `Scope::Local` bound inside the body only, and block-local bindings do
//! not leak (matching the validator).  Loops declare `Feature::Loops`;
//! `if` adds no feature.
//!
//! Functions/`def`/`lambda`, calls, and collections are deferred to
//! later milestones; unhandled forms return a clear `PythonLowerError`.
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

    // ── M3: control flow — if / elif / else ───────────────────────────

    #[test]
    fn trailing_if_becomes_block_value_with_both_branches() {
        // `if c: 1 else: 2` is the last (and only executable) statement, so
        // the `If` expression is `main`'s block value.
        let m = lower("c = True\nif c:\n    1\nelse:\n    2\n");
        match main_value(&m) {
            Expr::If { cond, then_branch, else_branch, .. } => {
                assert!(matches!(**cond, Expr::VarRef { scope: Scope::Local, .. }));
                assert!(matches!(then_branch.value, Expr::IntLit { value: 1, .. }));
                assert!(matches!(else_branch.value, Expr::IntLit { value: 2, .. }));
            }
            other => panic!("expected If, got {other:?}"),
        }
        // `if` is a SIR v0 construct — it adds no manifest feature.
        assert!(!m.manifest.contains(Feature::Loops));
    }

    #[test]
    fn if_without_else_synthesizes_nil_else_branch() {
        let m = lower("c = True\nif c:\n    1\n");
        match main_value(&m) {
            Expr::If { then_branch, else_branch, .. } => {
                assert!(matches!(then_branch.value, Expr::IntLit { value: 1, .. }));
                // No `else` in source → empty block whose value is NilLit.
                assert!(else_branch.stmts.is_empty());
                assert!(matches!(else_branch.value, Expr::NilLit { .. }));
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn elif_chain_nests_if_in_else_branch() {
        // if c: x=1 elif d: x=2 else: x=3
        //   ⇒ If(c){ then:[x=1], else: Block{ If(d){ then:[x=2], else:[x=3] } } }
        let m = lower(
            "c = True\nd = False\nif c:\n    x = 1\nelif d:\n    x = 2\nelse:\n    x = 3\n",
        );
        let outer = main_value(&m);
        match outer {
            Expr::If { cond, then_branch, else_branch, .. } => {
                assert!(matches!(&**cond, Expr::VarRef { name, .. } if name == "c"));
                // then-branch binds x = 1.
                assert!(matches!(
                    &then_branch.stmts[0],
                    Stmt::LetStarBinding { value: Expr::IntLit { value: 1, .. }, .. }
                ));
                // else-branch is a block whose *value* is the nested elif If.
                assert!(else_branch.stmts.is_empty());
                match &else_branch.value {
                    Expr::If { cond, then_branch, else_branch, .. } => {
                        assert!(matches!(&**cond, Expr::VarRef { name, .. } if name == "d"));
                        assert!(matches!(
                            &then_branch.stmts[0],
                            Stmt::LetStarBinding { value: Expr::IntLit { value: 2, .. }, .. }
                        ));
                        // Final else binds x = 3.
                        assert!(matches!(
                            &else_branch.stmts[0],
                            Stmt::LetStarBinding { value: Expr::IntLit { value: 3, .. }, .. }
                        ));
                    }
                    other => panic!("expected nested If for elif, got {other:?}"),
                }
            }
            other => panic!("expected outer If, got {other:?}"),
        }
    }

    #[test]
    fn elif_without_else_nests_if_with_nil_else() {
        // if c: x=1 elif d: x=2   (no trailing else)
        let m = lower("c = True\nd = False\nif c:\n    x = 1\nelif d:\n    x = 2\n");
        match main_value(&m) {
            Expr::If { else_branch, .. } => match &else_branch.value {
                Expr::If { else_branch: inner_else, .. } => {
                    // The innermost else is the synthesized nil branch.
                    assert!(inner_else.stmts.is_empty());
                    assert!(matches!(inner_else.value, Expr::NilLit { .. }));
                }
                other => panic!("expected nested elif If, got {other:?}"),
            },
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn if_as_statement_with_trailing_value_becomes_exprstmt() {
        // An `if` followed by a later bare expression is in *statement*
        // position, so it becomes an `ExprStmt` wrapping the `If`.
        let m = lower("c = True\nif c:\n    1\n42\n");
        let stmts = main_stmts(&m);
        // last stmt is the ExprStmt(If …); the block value is IntLit(42).
        assert!(matches!(main_value(&m), Expr::IntLit { value: 42, .. }));
        assert!(stmts
            .iter()
            .any(|s| matches!(s, Stmt::ExprStmt { expr: Expr::If { .. }, .. })));
    }

    // ── M3: while ─────────────────────────────────────────────────────

    #[test]
    fn while_lowers_to_while_stmt_and_sets_loops_feature() {
        let m = lower("c = True\nwhile c:\n    x = 1\n");
        match &main_stmts(&m)[1] {
            Stmt::While { cond, body, .. } => {
                assert!(matches!(cond, Expr::VarRef { scope: Scope::Local, .. }));
                assert!(matches!(
                    &body.stmts[0],
                    Stmt::LetStarBinding { name, .. } if name == "x"
                ));
            }
            other => panic!("expected While, got {other:?}"),
        }
        assert!(m.manifest.contains(Feature::Loops));
    }

    #[test]
    fn while_body_can_reassign_outer_local() {
        // `x` is bound before the loop; a `x = x + 1` inside re-assigns it.
        let m = lower("x = 0\nc = True\nwhile c:\n    x = x + 1\n");
        match &main_stmts(&m)[2] {
            Stmt::While { body, .. } => {
                assert!(matches!(&body.stmts[0], Stmt::Assign { name, .. } if name == "x"));
            }
            other => panic!("expected While, got {other:?}"),
        }
        assert!(m.manifest.contains(Feature::MutableBindings));
    }

    // ── M3: for-range (all three arities) ─────────────────────────────

    #[test]
    fn for_range_one_arg_defaults_start_zero_step_one() {
        let m = lower("for i in range(5):\n    s = i\n");
        match &main_stmts(&m)[0] {
            Stmt::ForRange { var, start, stop, step, body, .. } => {
                assert_eq!(var, "i");
                assert!(matches!(start, Expr::IntLit { value: 0, .. }));
                assert!(matches!(stop, Expr::IntLit { value: 5, .. }));
                assert!(matches!(step, Expr::IntLit { value: 1, .. }));
                // Inside the body, `i` resolves as a Local (the loop var).
                assert!(matches!(
                    &body.stmts[0],
                    Stmt::LetStarBinding { value: Expr::VarRef { scope: Scope::Local, .. }, .. }
                ));
            }
            other => panic!("expected ForRange, got {other:?}"),
        }
        assert!(m.manifest.contains(Feature::Loops));
    }

    #[test]
    fn for_range_two_args_sets_start_and_stop_step_one() {
        let m = lower("for i in range(2, 9):\n    s = i\n");
        match &main_stmts(&m)[0] {
            Stmt::ForRange { start, stop, step, .. } => {
                assert!(matches!(start, Expr::IntLit { value: 2, .. }));
                assert!(matches!(stop, Expr::IntLit { value: 9, .. }));
                assert!(matches!(step, Expr::IntLit { value: 1, .. }));
            }
            other => panic!("expected ForRange, got {other:?}"),
        }
    }

    #[test]
    fn for_range_three_args_sets_all_three() {
        let m = lower("for i in range(2, 10, 3):\n    s = i\n");
        match &main_stmts(&m)[0] {
            Stmt::ForRange { start, stop, step, .. } => {
                assert!(matches!(start, Expr::IntLit { value: 2, .. }));
                assert!(matches!(stop, Expr::IntLit { value: 10, .. }));
                assert!(matches!(step, Expr::IntLit { value: 3, .. }));
            }
            other => panic!("expected ForRange, got {other:?}"),
        }
    }

    #[test]
    fn for_range_accepts_variable_bounds() {
        // `range(a, b)` with non-literal bounds lowers them as VarRefs.
        let m = lower("a = 1\nb = 10\nfor i in range(a, b):\n    s = i\n");
        match &main_stmts(&m)[2] {
            Stmt::ForRange { start, stop, .. } => {
                assert!(matches!(start, Expr::VarRef { name, .. } if name == "a"));
                assert!(matches!(stop, Expr::VarRef { name, .. } if name == "b"));
            }
            other => panic!("expected ForRange, got {other:?}"),
        }
    }

    #[test]
    fn for_range_zero_args_is_rejected() {
        let err = compile_source("for i in range():\n    s = i\n", "t")
            .expect_err("range() rejected");
        assert!(
            err.message.contains("range") && err.message.contains("arity"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn for_range_four_args_is_rejected() {
        let err = compile_source("for i in range(1, 2, 3, 4):\n    s = i\n", "t")
            .expect_err("range(...,4) rejected");
        assert!(err.message.contains("arity"), "got: {}", err.message);
    }

    // ── M3: for-each ──────────────────────────────────────────────────

    #[test]
    fn for_each_over_iterable_lowers_to_foreach() {
        let m = lower("xs = 1\nfor x in xs:\n    p = x\n");
        match &main_stmts(&m)[1] {
            Stmt::ForEach { var, iter, body, .. } => {
                assert_eq!(var, "x");
                assert!(matches!(iter, Expr::VarRef { name, .. } if name == "xs"));
                // `x` resolves as a Local inside the body.
                assert!(matches!(
                    &body.stmts[0],
                    Stmt::LetStarBinding { value: Expr::VarRef { scope: Scope::Local, .. }, .. }
                ));
            }
            other => panic!("expected ForEach, got {other:?}"),
        }
        assert!(m.manifest.contains(Feature::Loops));
    }

    #[test]
    fn for_each_iterable_resolved_before_loop_var_bound() {
        // The iterable `x` here is a *pre-existing* local, distinct from the
        // loop var `x` shadowing it inside the body — but the iterable must
        // resolve against the outer scope (it does, because we classify and
        // lower the iterable before binding the loop var).
        let m = lower("x = 1\nfor x in x:\n    p = x\n");
        match &main_stmts(&m)[1] {
            Stmt::ForEach { iter, .. } => {
                assert!(matches!(iter, Expr::VarRef { name, scope: Scope::Local, .. } if name == "x"));
            }
            other => panic!("expected ForEach, got {other:?}"),
        }
    }

    // ── M3: loop-variable scoping ─────────────────────────────────────

    #[test]
    fn loop_var_does_not_leak_past_the_loop() {
        // `i` is bound only inside the for body; referencing it afterwards
        // is an unresolved-name error (the scope is rewound on body exit).
        let err = compile_source("for i in range(3):\n    s = i\ni\n", "t")
            .expect_err("leaked loop var rejected");
        assert!(
            err.message.contains("unresolved name `i`"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn name_bound_inside_branch_does_not_leak_past_if() {
        // `y` is first-bound inside the then-branch; a later top-level
        // reference must be unresolved (branch scope is rewound).
        let err = compile_source("c = True\nif c:\n    y = 1\ny\n", "t")
            .expect_err("leaked branch local rejected");
        assert!(
            err.message.contains("unresolved name `y`"),
            "got: {}",
            err.message
        );
    }

    // ── M3: nested control flow ───────────────────────────────────────

    #[test]
    fn nested_if_inside_while_lowers_and_validates() {
        let m = lower("c = True\nwhile c:\n    if c:\n        x = 1\n");
        match &main_stmts(&m)[1] {
            Stmt::While { body, .. } => {
                // The body's value is the nested `If` (a trailing if-suite).
                assert!(matches!(body.value, Expr::If { .. }));
            }
            other => panic!("expected While, got {other:?}"),
        }
        assert!(semantic_ir::validate(&m).is_ok());
    }

    #[test]
    fn for_inside_for_lowers_and_validates() {
        let m = lower("for i in range(3):\n    for j in range(3):\n        s = i\n");
        match &main_stmts(&m)[0] {
            Stmt::ForRange { var, body, .. } => {
                assert_eq!(var, "i");
                // The inner for is the trailing statement of the outer body.
                assert!(body.stmts.iter().any(|s| matches!(s, Stmt::ForRange { var, .. } if var == "j")));
            }
            other => panic!("expected outer ForRange, got {other:?}"),
        }
        assert!(semantic_ir::validate(&m).is_ok());
    }

    // ── M3: deferred constructs stay rejected ─────────────────────────

    #[test]
    fn def_is_still_unsupported() {
        let err = compile_source("def f():\n    x = 1\n", "t").expect_err("def rejected");
        assert!(err.message.contains("unsupported"), "got: {}", err.message);
    }

    #[test]
    fn with_statement_is_still_unsupported() {
        let err = compile_source("with a:\n    x = 1\n", "t").expect_err("with rejected");
        assert!(err.message.contains("unsupported"), "got: {}", err.message);
    }

    // ── M3: validator round-trip over control-flow programs ───────────

    #[test]
    fn control_flow_modules_pass_the_validator() {
        for src in [
            "c = True\nif c:\n    x = 1\n",
            "c = True\nif c:\n    x = 1\nelse:\n    x = 2\n",
            "c = True\nd = False\nif c:\n    x = 1\nelif d:\n    x = 2\nelse:\n    x = 3\n",
            "c = True\nwhile c:\n    x = 1\n",
            "x = 0\nc = True\nwhile c:\n    x = x + 1\n",
            "for i in range(5):\n    s = i\n",
            "for i in range(2, 9):\n    s = i\n",
            "for i in range(2, 10, 3):\n    s = i\n",
            "xs = 1\nfor x in xs:\n    p = x\n",
            "c = True\nwhile c:\n    if c:\n        x = 1\n",
            "for i in range(3):\n    for j in range(3):\n        s = i\n",
        ] {
            let m = lower(src);
            let r = semantic_ir::validate(&m);
            assert!(r.is_ok(), "module for {src:?} failed validation: {:?}", r.issues);
        }
    }
}
