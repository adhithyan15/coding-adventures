//! # python-to-semantic-ir
//!
//! Python CST → narrow-waist Semantic IR (SIR17), **milestone M4**.
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
//! ## M4 scope
//!
//! M1 lowered **literals only**; M2 added variable references,
//! assignment, and unary/binary operators; M3 added **control flow**
//! (`if`/`elif`/`else`, `while`, `for`).  M4 adds **functions, calls,
//! and closures**:
//!
//! - **`def f(params): suite`** → a top-level `Function` named `f`.  A
//!   two-pass design collects every function name first so calls (and
//!   mutual recursion) resolve regardless of textual order.
//! - **`return expr`** (tail position only) sets the function body's
//!   block `value`; falling off the end yields `NilLit` (Python's
//!   implicit `None`).  A **non-tail** (early) `return` is rejected with
//!   a positioned error.
//! - **`lambda params: expr`** and **nested `def`** are lifted to
//!   top-level synthesised functions with computed **captures**, and the
//!   definition site emits an `Expr::MakeClosure`.
//! - **calls** `f(args)` → `DirectCall` (known function), `BuiltinCall`
//!   (`print`/`len`/`range`), or `IndirectCall` (a closure value).
//!
//! Collections / comprehensions / decorators / `*args` & default
//! arguments are deferred to later milestones; unhandled forms return a
//! clear `PythonLowerError`.
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

    /// Find a function by name in a lowered module.
    fn func<'a>(m: &'a semantic_ir::Module, name: &str) -> &'a semantic_ir::Function {
        m.functions
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("function `{name}` exists"))
    }

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

    // ══════════════════════════════════════════════════════════════════
    // M4: functions, calls, closures
    // ══════════════════════════════════════════════════════════════════

    // ── def → top-level Function ──────────────────────────────────────

    #[test]
    fn def_lifts_to_top_level_function_with_params() {
        let m = lower("def add(a, b):\n    return a + b\n");
        let f = func(&m, "add");
        assert_eq!(f.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        assert!(f.captures.is_empty());
        // Tail `return a + b` → body value is the `+` builtin call.
        match &f.body.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "+");
                assert!(matches!(args[0], Expr::VarRef { scope: Scope::Param, .. }));
                assert!(matches!(args[1], Expr::VarRef { scope: Scope::Param, .. }));
            }
            other => panic!("expected BuiltinCall(+), got {other:?}"),
        }
        // A def with params declares DynamicTyping (no annotations).
        assert!(m.manifest.contains(Feature::DynamicTyping));
        // `main` exists alongside.
        assert!(m.functions.iter().any(|f| f.name == "main"));
    }

    #[test]
    fn def_no_params_no_return_yields_nil_body() {
        // Falling off the end with no `return` ⇒ implicit `None` (NilLit).
        let m = lower("def g():\n    x = 1\n");
        let f = func(&m, "g");
        assert!(f.params.is_empty());
        assert!(matches!(f.body.value, Expr::NilLit { .. }));
        // The single assignment is a let* statement in the body.
        assert!(matches!(&f.body.stmts[0], Stmt::LetStarBinding { name, .. } if name == "x"));
    }

    #[test]
    fn def_bare_return_yields_nil_body() {
        let m = lower("def h():\n    return\n");
        let f = func(&m, "h");
        assert!(matches!(f.body.value, Expr::NilLit { .. }));
    }

    #[test]
    fn def_tail_return_value_is_block_value() {
        let m = lower("def k():\n    return 42\n");
        let f = func(&m, "k");
        assert!(matches!(f.body.value, Expr::IntLit { value: 42, .. }));
    }

    #[test]
    fn def_param_resolves_as_param_scope() {
        let m = lower("def f(x):\n    return x\n");
        let f = func(&m, "f");
        assert!(matches!(f.body.value, Expr::VarRef { scope: Scope::Param, .. }));
    }

    #[test]
    fn def_statements_then_tail_return() {
        // A body with leading statements and a tail return.
        let m = lower("def f(n):\n    x = n + 1\n    return x\n");
        let f = func(&m, "f");
        assert_eq!(f.body.stmts.len(), 1);
        assert!(matches!(&f.body.stmts[0], Stmt::LetStarBinding { name, .. } if name == "x"));
        assert!(matches!(&f.body.value, Expr::VarRef { name, scope: Scope::Local, .. } if name == "x"));
    }

    // ── early-return rejection ────────────────────────────────────────

    #[test]
    fn early_return_is_rejected_with_position() {
        // A `return` that is not the final statement is an early return.
        let err = compile_source("def f(x):\n    return x\n    y = 1\n", "t")
            .expect_err("early return rejected");
        assert!(
            err.message.contains("early return"),
            "got: {}",
            err.message
        );
        assert_eq!(err.line, 2, "error points at the offending return");
    }

    #[test]
    fn return_inside_if_branch_is_early_return() {
        // A `return` nested inside an `if` branch is never the function
        // tail — rejected.
        let err = compile_source(
            "def f(x):\n    if x:\n        return 1\n    return 2\n",
            "t",
        )
        .expect_err("return-in-branch rejected");
        assert!(err.message.contains("early return"), "got: {}", err.message);
    }

    // ── calls: DirectCall / BuiltinCall / IndirectCall ────────────────

    #[test]
    fn call_known_function_is_direct_call() {
        let m = lower("def f(a):\n    return a\n\nf(1)\n");
        match main_value(&m) {
            Expr::DirectCall { fn_name, args, .. } => {
                assert_eq!(fn_name, "f");
                assert!(matches!(args[0], Expr::IntLit { value: 1, .. }));
            }
            other => panic!("expected DirectCall, got {other:?}"),
        }
    }

    #[test]
    fn call_forward_reference_resolves_via_two_pass() {
        // `g` is called before it is defined — the name-collection pass
        // makes this a DirectCall.
        let m = lower("def f():\n    return g()\n\ndef g():\n    return 1\n");
        let f = func(&m, "f");
        assert!(matches!(&f.body.value, Expr::DirectCall { fn_name, .. } if fn_name == "g"));
    }

    #[test]
    fn builtin_calls_lower_to_builtin_call() {
        for (src, name) in [
            ("print(1)\n", "print"),
            ("xs = 1\nlen(xs)\n", "len"),
            ("range(5)\n", "range"),
        ] {
            let m = lower(src);
            match main_value(&m) {
                Expr::BuiltinCall { name: got, .. } => assert_eq!(got, name, "for {src:?}"),
                other => panic!("expected BuiltinCall({name}) for {src:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn call_through_local_value_is_indirect_call() {
        // `f` is a captured/local closure handle (a parameter), so calling
        // it is an IndirectCall through the value.
        let m = lower("def apply(fn, x):\n    return fn(x)\n");
        let f = func(&m, "apply");
        match &f.body.value {
            Expr::IndirectCall { target, args, .. } => {
                assert!(matches!(&**target, Expr::VarRef { name, scope: Scope::Param, .. } if name == "fn"));
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected IndirectCall, got {other:?}"),
        }
        assert!(m.manifest.contains(Feature::Closures));
    }

    // ── lambda + capture ──────────────────────────────────────────────

    #[test]
    fn lambda_lowers_to_make_closure_and_synthesised_function() {
        let m = lower("f = lambda a: a + 1\n");
        // A synthesised `__lambda_0` function exists.
        let lam = func(&m, "__lambda_0");
        assert_eq!(lam.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["a"]);
        assert!(lam.captures.is_empty());
        // The binding's value is a MakeClosure referencing it.
        match &main_stmts(&m)[0] {
            Stmt::LetStarBinding { value: Expr::MakeClosure { fn_name, captures, .. }, .. } => {
                assert_eq!(fn_name, "__lambda_0");
                assert!(captures.is_empty());
            }
            other => panic!("expected LetStarBinding(MakeClosure), got {other:?}"),
        }
        assert!(m.manifest.contains(Feature::Closures));
    }

    #[test]
    fn lambda_captures_enclosing_local() {
        // `n` is a top-level local; the lambda body reads it → it is
        // captured (Scope::Capture inside the synthesised function).
        let m = lower("n = 10\nf = lambda x: x + n\n");
        let lam = func(&m, "__lambda_0");
        assert_eq!(lam.captures.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["n"]);
        // Inside the body, `n` resolves as a capture, `x` as a param.
        match &lam.body.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "+");
                assert!(matches!(&args[0], Expr::VarRef { name, scope: Scope::Param, .. } if name == "x"));
                assert!(matches!(&args[1], Expr::VarRef { name, scope: Scope::Capture, .. } if name == "n"));
            }
            other => panic!("expected BuiltinCall(+), got {other:?}"),
        }
        // The MakeClosure threads `n`'s enclosing value.
        match &main_stmts(&m)[1] {
            Stmt::LetStarBinding { value: Expr::MakeClosure { captures, .. }, .. } => {
                assert_eq!(captures.len(), 1);
                assert_eq!(captures[0].name, "n");
                assert!(matches!(&captures[0].value, Expr::VarRef { name, scope: Scope::Local, .. } if name == "n"));
            }
            other => panic!("expected MakeClosure capturing n, got {other:?}"),
        }
    }

    #[test]
    fn lambda_does_not_capture_globals_or_functions() {
        // A reference to a top-level function name inside a lambda is NOT
        // a capture — it is reachable directly.
        let m = lower("def g(y):\n    return y\n\nf = lambda x: g(x)\n");
        let lam = func(&m, "__lambda_0");
        assert!(lam.captures.is_empty(), "g should not be captured");
        assert!(matches!(&lam.body.value, Expr::DirectCall { fn_name, .. } if fn_name == "g"));
    }

    // ── nested def + capture ──────────────────────────────────────────

    #[test]
    fn nested_def_captures_enclosing_param_and_returns_closure() {
        // The canonical closure-adder: outer(n) returns inner, which
        // captures n.
        let m = lower(
            "def outer(n):\n    def inner(x):\n        return x + n\n    return inner\n",
        );
        // `inner` is lifted to a top-level function capturing `n`.
        let inner = func(&m, "inner");
        assert_eq!(inner.captures.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["n"]);
        match &inner.body.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "+");
                assert!(matches!(&args[0], Expr::VarRef { scope: Scope::Param, .. }));
                assert!(matches!(&args[1], Expr::VarRef { name, scope: Scope::Capture, .. } if name == "n"));
            }
            other => panic!("expected BuiltinCall(+), got {other:?}"),
        }
        // `outer`'s tail `return inner` is a MakeClosure threading `n`.
        let outer = func(&m, "outer");
        match &outer.body.value {
            Expr::MakeClosure { fn_name, captures, .. } => {
                assert_eq!(fn_name, "inner");
                assert_eq!(captures.len(), 1);
                assert_eq!(captures[0].name, "n");
                assert!(matches!(&captures[0].value, Expr::VarRef { name, scope: Scope::Param, .. } if name == "n"));
            }
            other => panic!("expected MakeClosure(inner), got {other:?}"),
        }
        assert!(m.manifest.contains(Feature::Closures));
    }

    // ── mutual recursion ──────────────────────────────────────────────

    #[test]
    fn mutual_recursion_sets_feature() {
        let m = lower(
            "def is_even(n):\n    return is_odd(n)\n\ndef is_odd(n):\n    return is_even(n)\n",
        );
        assert!(
            m.manifest.contains(Feature::MutualRecursion),
            "two functions calling each other ⇒ MutualRecursion"
        );
    }

    #[test]
    fn self_recursion_is_not_mutual_recursion() {
        let m = lower("def fact(n):\n    return fact(n)\n");
        assert!(
            !m.manifest.contains(Feature::MutualRecursion),
            "self-recursion alone is not mutual recursion"
        );
    }

    // ── deferred constructs stay rejected ─────────────────────────────

    #[test]
    fn default_parameter_is_rejected() {
        let err = compile_source("def f(a=1):\n    return a\n", "t")
            .expect_err("default param rejected");
        assert!(err.message.contains("default"), "got: {}", err.message);
    }

    #[test]
    fn list_literal_is_still_unsupported() {
        let err = compile_source("xs = [1, 2, 3]\n", "t").expect_err("list rejected");
        assert!(err.message.contains("unsupported"), "got: {}", err.message);
    }

    // ── validator round-trip over M4 programs ─────────────────────────

    #[test]
    fn m4_modules_pass_the_validator() {
        for src in [
            "def f():\n    return 1\n",
            "def f(a, b):\n    return a + b\n",
            "def g():\n    x = 1\n",
            "def h():\n    return\n",
            "def f(a):\n    return a\n\nf(1)\n",
            "def f():\n    return g()\n\ndef g():\n    return 1\n",
            "print(1)\n",
            "f = lambda a: a + 1\n",
            "n = 10\nf = lambda x: x + n\n",
            "def apply(fn, x):\n    return fn(x)\n",
            "def outer(n):\n    def inner(x):\n        return x + n\n    return inner\n",
            "def is_even(n):\n    return is_odd(n)\n\ndef is_odd(n):\n    return is_even(n)\n",
            // factorial (recursion + if/else tail)
            "def fact(n):\n    if n < 2:\n        return 1\n    else:\n        return n * fact(n - 1)\n",
            // fibonacci (while loop + mutation, tail return)
            "def fib(n):\n    a = 0\n    b = 1\n    i = 0\n    while i < n:\n        t = a + b\n        a = b\n        b = t\n        i = i + 1\n    return a\n",
        ] {
            let m = lower(src);
            let r = semantic_ir::validate(&m);
            assert!(r.is_ok(), "module for {src:?} failed validation: {:?}", r.issues);
        }
    }

    // ══════════════════════════════════════════════════════════════════
    // M4: end-to-end — Python → SIR → Python → execute
    // ══════════════════════════════════════════════════════════════════
    //
    // These tests close the loop: lower a golden Python program to SIR,
    // emit *fresh* Python via the `semantic-ir-to-python` backend, run it
    // with the system `python`, and assert on stdout.  They are gated on
    // `python` being available so CI hosts without an interpreter still
    // pass (the lowering + validate assertions above already cover
    // correctness structurally; this is the behavioural confirmation).

    /// Locate a Python interpreter, or `None` if none is on `PATH`.
    fn python_exe() -> Option<&'static str> {
        ["python3", "python"].into_iter().find(|cand| {
            std::process::Command::new(cand)
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
    }

    /// Lower `src` → SIR → Python, execute it, and return trimmed stdout.
    /// Returns `None` when no interpreter is available (test should skip).
    fn run_roundtrip(src: &str) -> Option<String> {
        let py = python_exe()?;
        let module = compile_source(src, "golden").expect("lowering succeeded");
        // The lowered module must validate before we trust the emit.
        let v = semantic_ir::validate(&module);
        assert!(v.is_ok(), "golden module failed validation: {:?}", v.issues);

        let artifact =
            semantic_ir_to_python::compile(&module).expect("emit python from SIR");

        let out = std::process::Command::new(py)
            .arg("-c")
            .arg(&artifact.source)
            .output()
            .expect("python ran");
        assert!(
            out.status.success(),
            "python execution failed:\nstderr:\n{}\n--- emitted ---\n{}",
            String::from_utf8_lossy(&out.stderr),
            artifact.source
        );
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    #[test]
    fn e2e_factorial() {
        // Recursion + tail-position if/else (returns in both branches).
        let src = "\
def fact(n):
    if n < 2:
        return 1
    else:
        return n * fact(n - 1)

print(fact(5))
";
        if let Some(out) = run_roundtrip(src) {
            assert_eq!(out, "120", "factorial(5) should print 120");
        }
    }

    #[test]
    fn e2e_fibonacci() {
        // While loop + mutation + tail return.
        let src = "\
def fib(n):
    a = 0
    b = 1
    i = 0
    while i < n:
        t = a + b
        a = b
        b = t
        i = i + 1
    return a

print(fib(10))
";
        if let Some(out) = run_roundtrip(src) {
            assert_eq!(out, "55", "fib(10) should print 55");
        }
    }

    #[test]
    fn e2e_closure_adder() {
        // A closure that captures its enclosing parameter `n`.
        let src = "\
def adder(n):
    def add(x):
        return x + n
    return add

a = adder(10)
print(a(5))
";
        if let Some(out) = run_roundtrip(src) {
            assert_eq!(out, "15", "adder(10)(5) should print 15");
        }
    }

    #[test]
    fn e2e_lambda_closure() {
        // A lambda capturing an enclosing local, invoked indirectly.
        let src = "\
n = 100
f = lambda x: x + n
print(f(23))
";
        if let Some(out) = run_roundtrip(src) {
            assert_eq!(out, "123", "lambda capturing n should print 123");
        }
    }

    #[test]
    fn e2e_mutual_recursion() {
        // is_even / is_odd call each other.
        let src = "\
def is_even(n):
    if n == 0:
        return True
    else:
        return is_odd(n - 1)

def is_odd(n):
    if n == 0:
        return False
    else:
        return is_even(n - 1)

print(is_even(10))
";
        if let Some(out) = run_roundtrip(src) {
            // The backend renders booleans in the SIR runtime's own
            // display form (`#t`/`#f`), not Python's `True`/`False` — the
            // value is what matters: `is_even(10)` is true.
            assert!(
                out == "True" || out == "#t",
                "is_even(10) should be truthy, got {out:?}"
            );
        }
    }
}
