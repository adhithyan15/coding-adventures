//! # javascript-to-semantic-ir
//!
//! JavaScript CST → narrow-waist [Semantic IR](semantic_ir).
//!
//! The SIR19 JavaScript frontend (after Twig in SIR11, Python in SIR17,
//! and Ruby).  It consumes the generic
//! [`GrammarASTNode`](parser::grammar_parser::GrammarASTNode) produced by
//! the [`javascript-parser`](coding_adventures_javascript_parser) crate
//! and emits a [`semantic_ir::Module`] suitable for any SIR backend.
//!
//! ## Pipeline
//!
//! ```text
//! JavaScript source
//!    │
//!    ▼  javascript_parser::parse_javascript(source, "es2020")
//! parser::grammar_parser::GrammarASTNode  (generic CST)
//!    │
//!    ▼  javascript_to_semantic_ir::compile_source     ← THIS CRATE
//! semantic_ir::Module
//!    │
//!    ▼  semantic-ir-to-{rust, typescript, go, python}
//! target source
//! ```
//!
//! We deliberately lower from the **generic** `GrammarASTNode` (the CST),
//! not from the parser's typed-AST bridge — that's the contract SIR19
//! sets, mirroring the Ruby and Twig frontends.
//!
//! ## Milestone status — M2 (variables, assignment, operators)
//!
//! This build implements literals (M1) **plus**:
//!
//! - JS number → [`IntLit`](semantic_ir::Expr::IntLit) when the literal
//!   text is an integer (no `.`/exponent), else
//!   [`FloatLit`](semantic_ir::Expr::FloatLit).
//! - `true`/`false` → [`BoolLit`](semantic_ir::Expr::BoolLit).
//! - `null` **and** `undefined` → [`NilLit`](semantic_ir::Expr::NilLit)
//!   (the JS distinction is intentionally lost in v0).
//! - string → [`StrLit`](semantic_ir::Expr::StrLit).
//! - variable reference → [`VarRef`](semantic_ir::Expr::VarRef) with a
//!   resolved [`Scope`](semantic_ir::Scope) (M2 has one flat scope, so
//!   `Scope::Local`); an undeclared name is a positioned error.
//! - `let`/`const`/`var x = e;` → a sequential binding statement
//!   ([`Stmt::LetStarBinding`](semantic_ir::Stmt::LetStarBinding));
//!   re-assignment `x = e;` → [`Stmt::Assign`](semantic_ir::Stmt::Assign).
//! - arithmetic / comparison operators → `BuiltinCall`; `&&`/`||` →
//!   short-circuit [`LogicalAnd`](semantic_ir::Expr::LogicalAnd) /
//!   [`LogicalOr`](semantic_ir::Expr::LogicalOr); unary `!`→`not`,
//!   `-`→`neg`; both `==`/`===` → `BuiltinCall("=")` and both `!=`/`!==`
//!   → `BuiltinCall("!=")` (strict normalisation — a documented semantic
//!   change for the loose-equality coercion cases).
//!
//! All other syntax (control flow, functions, collections, member
//! access, template literals) is **deferred** to later milestones and
//! currently produces a clear [`JsLowerError`].  See the crate
//! `CHANGELOG.md` "Deferred" section for the roadmap.
//!
//! ## Public API
//!
//! ```ignore
//! use javascript_to_semantic_ir::{compile_source, JsLowerError};
//!
//! let module = compile_source("42;", "demo")?;
//! // `module` is a `semantic_ir::Module` with a `main` whose body
//! // value is `IntLit { value: 42 }`.
//! ```

use coding_adventures_javascript_parser::parse_javascript;

mod lower;

pub use lower::{compile, JsLowerError};

/// The ECMAScript edition we parse against.  `es2020` is broad enough to
/// admit modern syntax (arrow functions, `let`/`const`, template
/// literals); the frontend then rejects the parts it doesn't yet lower.
const JS_VERSION: &str = "es2020";

/// Parse JavaScript source and lower it to SIR in a single call.
///
/// Wraps [`parse_javascript`] (with the [`JS_VERSION`] grammar) followed
/// by [`compile`].  A parse failure is surfaced as a [`JsLowerError`]
/// with the parser's reported position when available, so callers get a
/// single uniform error type for both phases.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, JsLowerError> {
    let tree = parse_javascript(source, JS_VERSION).map_err(|e| parse_error_to_lower(&e))?;
    compile(&tree, module_name)
}

/// Translate the parser's free-form error string into a [`JsLowerError`].
///
/// The parser reports errors as `"… at L:C: …"`; we best-effort extract
/// the `L:C` so the error carries a usable position.  When extraction
/// fails we fall back to `0:0` — the message is still preserved.
fn parse_error_to_lower(msg: &str) -> JsLowerError {
    let (line, column) = extract_line_col(msg).unwrap_or((0, 0));
    JsLowerError {
        message: msg.to_string(),
        line,
        column,
    }
}

/// Pull the first `"<line>:<col>"` pair out of a parser error message.
fn extract_line_col(msg: &str) -> Option<(usize, usize)> {
    // Look for the " at " marker the parser uses, then parse "L:C".
    let after = msg.split(" at ").nth(1)?;
    let coords = after.split(':');
    let parts: Vec<&str> = coords.take(2).collect();
    if parts.len() == 2 {
        let line = parts[0].trim().parse().ok()?;
        let col = parts[1].trim().parse().ok()?;
        Some((line, col))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use semantic_ir::{Expr, Feature, FeatureManifest};

    /// Lower `src`, asserting success, and return the module.
    fn lower(src: &str) -> semantic_ir::Module {
        compile_source(src, "test").expect("lowering succeeded")
    }

    /// Fetch the `main` function's body block.
    fn main_value(m: &semantic_ir::Module) -> &Expr {
        let f = m
            .functions
            .iter()
            .find(|f| f.name == "main")
            .expect("main function present");
        &f.body.value
    }

    /// Helper: a module must always pass the SIR validator.
    fn assert_valid(m: &semantic_ir::Module) {
        let r = semantic_ir::validate(m);
        assert!(r.is_ok(), "validation failed: {:?}", r.issues);
    }

    // ── one positive test per literal kind ─────────────────────────

    #[test]
    fn integer_number_becomes_int_lit() {
        let m = lower("42;");
        assert!(matches!(main_value(&m), Expr::IntLit { value: 42, .. }));
        assert_valid(&m);
        // No string/float features ⇒ an empty manifest.
        assert_eq!(m.manifest, FeatureManifest::new());
    }

    #[test]
    fn decimal_number_becomes_float_lit() {
        let m = lower("3.25;");
        match main_value(&m) {
            Expr::FloatLit { value, .. } => assert!((value - 3.25).abs() < 1e-12),
            other => panic!("expected FloatLit, got {other:?}"),
        }
        assert!(m.manifest.contains(Feature::Floats));
        assert_valid(&m);
    }

    #[test]
    fn exponent_number_becomes_float_lit() {
        // `1e3` has an exponent marker ⇒ float, even with no decimal pt.
        let m = lower("1e3;");
        match main_value(&m) {
            Expr::FloatLit { value, .. } => assert!((value - 1000.0).abs() < 1e-9),
            other => panic!("expected FloatLit, got {other:?}"),
        }
        assert_valid(&m);
    }

    #[test]
    fn true_becomes_bool_lit() {
        let m = lower("true;");
        assert!(matches!(main_value(&m), Expr::BoolLit { value: true, .. }));
        assert_valid(&m);
    }

    #[test]
    fn false_becomes_bool_lit() {
        let m = lower("false;");
        assert!(matches!(
            main_value(&m),
            Expr::BoolLit { value: false, .. }
        ));
        assert_valid(&m);
    }

    #[test]
    fn null_becomes_nil_lit() {
        let m = lower("null;");
        assert!(matches!(main_value(&m), Expr::NilLit { .. }));
        assert_valid(&m);
    }

    #[test]
    fn undefined_also_becomes_nil_lit() {
        // JS-specific: `undefined` is an identifier, but we collapse it
        // to the same NilLit as `null`. The distinction is lost in v0.
        let m = lower("undefined;");
        assert!(matches!(main_value(&m), Expr::NilLit { .. }));
        assert_valid(&m);
    }

    #[test]
    fn double_quoted_string_becomes_str_lit() {
        let m = lower("\"hello\";");
        match main_value(&m) {
            Expr::StrLit { value, .. } => assert_eq!(value, "hello"),
            other => panic!("expected StrLit, got {other:?}"),
        }
        assert!(m.manifest.contains(Feature::Strings));
        assert_valid(&m);
    }

    #[test]
    fn single_quoted_string_becomes_str_lit() {
        let m = lower("'world';");
        match main_value(&m) {
            Expr::StrLit { value, .. } => assert_eq!(value, "world"),
            other => panic!("expected StrLit, got {other:?}"),
        }
        assert_valid(&m);
    }

    // ── compile_source / structural tests ──────────────────────────

    #[test]
    fn empty_program_compiles_to_nil_main() {
        // `compile_source` on empty input must still produce a valid
        // module with a nil-valued `main`.
        let m = lower("");
        assert_eq!(m.name, "test");
        assert!(matches!(main_value(&m), Expr::NilLit { .. }));
        // Exactly one function (`main`) and it is exported.
        assert_eq!(m.functions.len(), 1);
        assert!(m.exports.iter().any(|e| e.name == "main"));
        assert_valid(&m);
    }

    #[test]
    fn metadata_records_source_language_and_sir_version() {
        let m = lower("1;");
        assert_eq!(m.metadata.source_language.as_deref(), Some("javascript"));
        assert_eq!(
            m.metadata.sir_version.as_deref(),
            Some(semantic_ir::CURRENT_SIR_VERSION)
        );
    }

    #[test]
    fn last_literal_is_the_block_value() {
        // Multiple top-level literal statements: the final one is the
        // block's tail value (earlier pure literals are unobservable).
        let m = lower("1;\n2;\n3;\n");
        assert!(matches!(main_value(&m), Expr::IntLit { value: 3, .. }));
        assert_valid(&m);
    }

    #[test]
    fn validate_round_trip_on_mixed_literals() {
        // A string literal forces the `Strings` feature; the module must
        // still validate (used-but-undeclared would otherwise error).
        let m = lower("\"abc\";");
        assert_valid(&m);
        // And declaring exactly the observed feature: no extra warnings.
        let r = semantic_ir::validate(&m);
        assert!(r.warnings().next().is_none(), "unexpected warnings");
    }

    // ── M2: variables, binding, assignment ─────────────────────────

    use semantic_ir::{Scope, Stmt};

    /// Fetch the `main` function's body block.
    fn main_block(m: &semantic_ir::Module) -> &semantic_ir::Block {
        &m.functions
            .iter()
            .find(|f| f.name == "main")
            .expect("main function present")
            .body
    }

    #[test]
    fn let_binding_emits_let_star_and_resolves_reference() {
        // `let x = 1; x;` → one binding stmt, tail value a local var-ref.
        let m = lower("let x = 1; x;");
        let b = main_block(&m);
        assert_eq!(b.stmts.len(), 1);
        match &b.stmts[0] {
            Stmt::LetStarBinding { name, value, .. } => {
                assert_eq!(name, "x");
                assert!(matches!(value, Expr::IntLit { value: 1, .. }));
            }
            other => panic!("expected LetStarBinding, got {other:?}"),
        }
        match &b.value {
            Expr::VarRef { name, scope: Scope::Local, .. } => assert_eq!(name, "x"),
            other => panic!("expected local VarRef, got {other:?}"),
        }
        assert_valid(&m);
    }

    #[test]
    fn const_and_var_both_lower_to_bindings() {
        let m = lower("const a = 1; var b = 2;");
        let b = main_block(&m);
        assert_eq!(b.stmts.len(), 2);
        assert!(matches!(&b.stmts[0], Stmt::LetStarBinding { name, .. } if name == "a"));
        assert!(matches!(&b.stmts[1], Stmt::LetStarBinding { name, .. } if name == "b"));
        assert_valid(&m);
    }

    #[test]
    fn cross_referencing_consecutive_bindings_validate() {
        // The parallel-`let` trap: `const y = x + 1` references the prior
        // `x`.  Sequential `let*` makes this validate.
        let m = lower("let x = 1; const y = x + 1; y;");
        assert_valid(&m);
        assert!(matches!(main_block(&m).value, Expr::VarRef { .. }));
    }

    #[test]
    fn reassignment_emits_assign_after_binding() {
        // `let x = 1; x = 2;` — second is a re-assignment, not a binding.
        let m = lower("let x = 1; x = 2;");
        let b = main_block(&m);
        assert_eq!(b.stmts.len(), 2);
        assert!(matches!(&b.stmts[0], Stmt::LetStarBinding { .. }));
        match &b.stmts[1] {
            Stmt::Assign { name, scope: Scope::Local, value, .. } => {
                assert_eq!(name, "x");
                assert!(matches!(value, Expr::IntLit { value: 2, .. }));
            }
            other => panic!("expected Assign, got {other:?}"),
        }
        assert!(m.manifest.contains(Feature::MutableBindings));
        assert_valid(&m);
    }

    #[test]
    fn first_bare_assignment_creates_a_binding() {
        // `x = 5; x;` — no declarator, first sighting still binds (JS
        // implicitly creates the global); the reference then resolves.
        let m = lower("x = 5; x;");
        let b = main_block(&m);
        assert!(matches!(&b.stmts[0], Stmt::LetStarBinding { name, .. } if name == "x"));
        assert!(matches!(&b.value, Expr::VarRef { .. }));
        assert_valid(&m);
    }

    #[test]
    fn unresolved_name_is_a_positioned_error() {
        let err = compile_source("nope;", "test").expect_err("should reject unknown name");
        assert!(
            err.message.contains("unresolved name"),
            "unexpected message: {}",
            err.message
        );
    }

    // ── M2: binary operators → BuiltinCall ─────────────────────────

    /// Lower `let a = 1; let b = 2; <op>;` and return the tail-value
    /// `BuiltinCall` name, asserting the module validates.
    fn binop_name(src_op: &str) -> String {
        let m = lower(&format!("let a = 1; let b = 2; {src_op};"));
        assert_valid(&m);
        match &main_block(&m).value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(args.len(), 2, "binary op must have 2 args");
                name.clone()
            }
            other => panic!("expected BuiltinCall for `{src_op}`, got {other:?}"),
        }
    }

    #[test]
    fn arithmetic_operators_lower_to_builtins() {
        assert_eq!(binop_name("a + b"), "+");
        assert_eq!(binop_name("a - b"), "-");
        assert_eq!(binop_name("a * b"), "*");
        assert_eq!(binop_name("a / b"), "/");
        assert_eq!(binop_name("a % b"), "%");
    }

    #[test]
    fn comparison_operators_lower_to_builtins() {
        assert_eq!(binop_name("a < b"), "<");
        assert_eq!(binop_name("a > b"), ">");
        assert_eq!(binop_name("a <= b"), "<=");
        assert_eq!(binop_name("a >= b"), ">=");
    }

    #[test]
    fn equality_is_normalised_to_strict() {
        // Both loose and strict equality collapse to the SIR `=`/`!=`.
        assert_eq!(binop_name("a == b"), "=");
        assert_eq!(binop_name("a === b"), "=");
        assert_eq!(binop_name("a != b"), "!=");
        assert_eq!(binop_name("a !== b"), "!=");
    }

    #[test]
    fn left_associative_chain_folds_left() {
        // `a + b + a` → BuiltinCall("+", [BuiltinCall("+", [a, b]), a]).
        let m = lower("let a = 1; let b = 2; a + b + a;");
        assert_valid(&m);
        match &main_block(&m).value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "+");
                assert!(matches!(&args[0], Expr::BuiltinCall { name, .. } if name == "+"));
                assert!(matches!(&args[1], Expr::VarRef { .. }));
            }
            other => panic!("expected nested BuiltinCall, got {other:?}"),
        }
    }

    #[test]
    fn precedence_is_preserved_by_the_cst() {
        // `a + b * a` must nest the `*` inside the `+`'s second arg.
        let m = lower("let a = 1; let b = 2; a + b * a;");
        assert_valid(&m);
        match &main_block(&m).value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "+");
                assert!(matches!(&args[1], Expr::BuiltinCall { name, .. } if name == "*"));
            }
            other => panic!("expected `+` at root, got {other:?}"),
        }
    }

    // ── M2: logical short-circuit ──────────────────────────────────

    #[test]
    fn logical_and_is_a_short_circuit_node() {
        let m = lower("let a = 1; let b = 2; a && b;");
        assert!(matches!(main_block(&m).value, Expr::LogicalAnd { .. }));
        assert!(m.manifest.contains(Feature::ShortCircuit));
        assert_valid(&m);
    }

    #[test]
    fn logical_or_is_a_short_circuit_node() {
        let m = lower("let a = 1; let b = 2; a || b;");
        assert!(matches!(main_block(&m).value, Expr::LogicalOr { .. }));
        assert!(m.manifest.contains(Feature::ShortCircuit));
        assert_valid(&m);
    }

    #[test]
    fn logical_operators_are_not_builtins() {
        // Guard against a regression that lowers `&&` to BuiltinCall.
        let m = lower("let a = 1; let b = 2; a && b;");
        assert!(!matches!(main_block(&m).value, Expr::BuiltinCall { .. }));
    }

    // ── M2: unary operators ────────────────────────────────────────

    #[test]
    fn unary_not_lowers_to_builtin_not() {
        let m = lower("let c = true; !c;");
        match &main_block(&m).value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "not");
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected BuiltinCall(not), got {other:?}"),
        }
        assert_valid(&m);
    }

    #[test]
    fn unary_minus_on_variable_lowers_to_neg() {
        let m = lower("let d = 1; -d;");
        match &main_block(&m).value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "neg");
                assert!(matches!(&args[0], Expr::VarRef { .. }));
            }
            other => panic!("expected BuiltinCall(neg), got {other:?}"),
        }
        assert_valid(&m);
    }

    #[test]
    fn unary_minus_on_literal_is_constant_folded() {
        // `-5` folds to IntLit(-5); `-3.25` to FloatLit(-3.25).
        let m = lower("-5;");
        assert!(matches!(main_value(&m), Expr::IntLit { value: -5, .. }));
        assert_valid(&m);

        let m = lower("-3.25;");
        match main_value(&m) {
            Expr::FloatLit { value, .. } => assert!((value + 3.25).abs() < 1e-12),
            other => panic!("expected FloatLit(-3.25), got {other:?}"),
        }
    }

    // ── M2: structural / round-trip ────────────────────────────────

    #[test]
    fn operators_add_no_features_but_validate() {
        // A pure arithmetic program declares no Strings/Floats/etc.
        let m = lower("let a = 1; let b = 2; a + b;");
        assert!(!m.manifest.contains(Feature::ShortCircuit));
        assert!(!m.manifest.contains(Feature::MutableBindings));
        assert_valid(&m);
        let r = semantic_ir::validate(&m);
        assert!(r.warnings().next().is_none(), "unexpected warnings");
    }

    #[test]
    fn mixed_program_validates_round_trip() {
        let m = lower("let x = 10; let y = x * 2; x = y + 1; x >= y && y != 0;");
        assert_valid(&m);
        assert!(m.manifest.contains(Feature::ShortCircuit));
        assert!(m.manifest.contains(Feature::MutableBindings));
    }

    #[test]
    fn parse_failure_surfaces_as_lower_error_with_position() {
        // `@` is not valid JS ⇒ the parser errors; we must surface it as
        // a `JsLowerError` (not panic) with a best-effort position.
        let err = compile_source("@;", "test").expect_err("should fail to parse");
        assert!(!err.message.is_empty());
    }

    #[test]
    fn extract_line_col_parses_parser_message() {
        // Unit-test the position extractor directly.
        let got = extract_line_col("Parse error at 3:7: something");
        assert_eq!(got, Some((3, 7)));
        assert_eq!(extract_line_col("no position here"), None);
    }
}
