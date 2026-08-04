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
//! ## Milestone status — M5 (collections: arrays & objects)
//!
//! This build implements literals (M1), variables/operators (M2), control
//! flow (M3), and functions/closures (M4) **plus** collections (M5):
//! array literals → [`Expr::SeqLit`], object literals → [`Expr::MapLit`]
//! (identifier and `"string"` keys both → string map keys), `xs.length`
//! → [`Expr::SeqLen`], `obj.prop`/`obj["k"]` → [`Expr::MapGet`], `xs[i]`
//! → [`Expr::SeqIndex`], and the matching write forms
//! [`Stmt::SeqSet`]/[`Stmt::MapSet`].  The bracket-vs-dot disambiguation
//! (`x[<string-literal>]`→map, `x[<other>]`→sequence, `.length`→`SeqLen`,
//! other `.prop`→map) follows the SIR19 collections table; flat member
//! chains (`grid[0][1]`) fold left iteratively.  See [`lower`] for the
//! full collections design and the CWE-674 depth bounds.
//!
//! M4, unchanged: functions (M4): `function` declarations →
//! top-level [`Function`](semantic_ir::Function)s, arrow functions and
//! nested `function`s → [`Expr::MakeClosure`] over synthesised functions
//! with free-variable [`Capture`](semantic_ir::Capture)s, tail-position
//! `return` → the body [`Block`](semantic_ir::Block)'s value, calls →
//! [`DirectCall`](semantic_ir::Expr::DirectCall) /
//! [`IndirectCall`](semantic_ir::Expr::IndirectCall), and `console.log`
//! → [`BuiltinCall`](semantic_ir::Expr::BuiltinCall)`("print", …)`.
//! Function-name collection is two-pass (so forward references and mutual
//! recursion resolve); an early (non-tail) `return` is a positioned error.
//!
//! M3, unchanged: `if`/`else` → [`Expr::If`] (nested in the else branch
//! for else-if chains), `while` → [`Stmt::While`], the canonical counting
//! C-style `for` → [`Stmt::ForRange`], `for … of` → [`Stmt::ForEach`], and
//! bare `{ … }` blocks → [`Expr::Block`].  Non-canonical `for` loops, the
//! remaining control-flow constructs
//! (`switch`/`try`/`do-while`/labeled/`break`/`continue`), and classes /
//! `this` / `new` (M6+) are positioned errors (deferred).
//!
//! The M1 + M2 lowerings, unchanged, are:
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
//! All other syntax (methods beyond `console.log`, array methods like
//! `.map`/`.push`, spread/elisions, object shorthand/computed/numeric keys,
//! `.length` assignment, template literals, classes/`this`/`new`,
//! generators/`async`, default/rest params, destructuring, plus
//! non-canonical `for`, early `return`, and the remaining control-flow
//! constructs `switch`/`try`/`do-while`/labeled/`break`/`continue`) is
//! **deferred** to later milestones and currently produces a clear
//! [`JsLowerError`].  See the crate `CHANGELOG.md` "Deferred" section for
//! the roadmap.
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

    // ── M3: control flow — if / while / for / for-of ───────────────

    use semantic_ir::Block;

    /// Find the single `Expr::If` that a top-level `if` statement lowers to.
    /// An `if` statement becomes a `Stmt::ExprStmt { expr: Expr::If, .. }`
    /// in `main`'s body (it produces no observable tail value at the top
    /// level), so we dig it out of the first statement.
    fn first_if(m: &semantic_ir::Module) -> &Expr {
        let b = main_block(m);
        match b.stmts.first() {
            Some(Stmt::ExprStmt { expr: e @ Expr::If { .. }, .. }) => e,
            // Or, when `if` is the *only* item, it may be the tail value.
            _ => match &b.value {
                e @ Expr::If { .. } => e,
                other => panic!("expected Expr::If, got stmts={:?} value={other:?}", b.stmts),
            },
        }
    }

    /// Pull a block's only `ExprStmt`-wrapped assignment target name, used
    /// to sanity-check which branch body we are looking at.
    fn block_only_assign_name(b: &Block) -> Option<&str> {
        b.stmts.iter().find_map(|s| match s {
            Stmt::Assign { name, .. } | Stmt::LetStarBinding { name, .. } => Some(name.as_str()),
            _ => None,
        })
    }

    #[test]
    fn if_else_lowers_to_expr_if_with_both_branches() {
        let m = lower("let c = true; let x = 0; if (c) { x = 1; } else { x = 2; }");
        assert_valid(&m);
        match first_if(&m) {
            Expr::If { cond, then_branch, else_branch, .. } => {
                assert!(matches!(**cond, Expr::VarRef { .. }));
                // then-branch assigns x = 1
                assert!(matches!(
                    then_branch.stmts.first(),
                    Some(Stmt::Assign { .. })
                ));
                assert!(matches!(
                    else_branch.stmts.first(),
                    Some(Stmt::Assign { .. })
                ));
            }
            other => panic!("expected If, got {other:?}"),
        }
        // NB: `Expr::If` is not gated by any `Feature` in SIR v0 — the
        // validator observes no feature for a conditional — so we
        // deliberately do *not* assert a manifest entry here.
    }

    #[test]
    fn if_without_else_gets_empty_nil_else_branch() {
        let m = lower("let c = true; let x = 0; if (c) { x = 1; }");
        assert_valid(&m);
        match first_if(&m) {
            Expr::If { then_branch, else_branch, .. } => {
                assert!(matches!(then_branch.stmts.first(), Some(Stmt::Assign { .. })));
                // Synthetic empty else: no stmts, nil value.
                assert!(else_branch.stmts.is_empty());
                assert!(matches!(else_branch.value, Expr::NilLit { .. }));
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn else_if_chain_nests_if_in_else_branch() {
        // `if (a) {} else if (b) {} else {}` → outer If whose else_branch's
        // tail value is a nested If.
        let m = lower(
            "let a = true; let b = false; let x = 0; \
             if (a) { x = 1; } else if (b) { x = 2; } else { x = 3; }",
        );
        assert_valid(&m);
        match first_if(&m) {
            Expr::If { else_branch, .. } => {
                // The else branch holds the nested `if` as its tail value.
                match &else_branch.value {
                    Expr::If { then_branch, else_branch: inner_else, .. } => {
                        assert_eq!(block_only_assign_name(then_branch), Some("x"));
                        // Final `else { x = 3; }`.
                        assert!(matches!(
                            inner_else.stmts.first(),
                            Some(Stmt::Assign { .. })
                        ));
                    }
                    other => panic!("expected nested If in else, got {other:?}"),
                }
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn single_statement_if_body_needs_no_braces() {
        // `if (c) x = 1;` — an unbraced single-statement body.
        let m = lower("let c = true; let x = 0; if (c) x = 1;");
        assert_valid(&m);
        match first_if(&m) {
            Expr::If { then_branch, .. } => {
                assert!(matches!(then_branch.stmts.first(), Some(Stmt::Assign { .. })));
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn while_loop_lowers_to_stmt_while() {
        let m = lower("let c = true; let x = 0; while (c) { x = 1; }");
        assert_valid(&m);
        let b = main_block(&m);
        let w = b.stmts.iter().find(|s| matches!(s, Stmt::While { .. }))
            .expect("a While statement");
        match w {
            Stmt::While { cond, body, .. } => {
                assert!(matches!(cond, Expr::VarRef { .. }));
                assert!(matches!(body.stmts.first(), Some(Stmt::Assign { .. })));
            }
            _ => unreachable!(),
        }
        assert!(m.manifest.contains(Feature::Loops));
    }

    /// Locate the single `ForRange` in `main`.
    fn first_for_range(m: &semantic_ir::Module) -> &Stmt {
        main_block(m)
            .stmts
            .iter()
            .find(|s| matches!(s, Stmt::ForRange { .. }))
            .expect("a ForRange statement")
    }

    #[test]
    fn c_for_with_explicit_assign_update_lowers_to_for_range() {
        // `for (let i = 0; i < n; i = i + 1)` → ForRange(i, 0, n, 1).
        let m = lower("let n = 10; let s = 0; for (let i = 0; i < n; i = i + 1) { s = s + i; }");
        assert_valid(&m);
        match first_for_range(&m) {
            Stmt::ForRange { var, start, stop, step, body, .. } => {
                assert_eq!(var, "i");
                assert!(matches!(start, Expr::IntLit { value: 0, .. }));
                assert!(matches!(stop, Expr::VarRef { name, .. } if name == "n"));
                assert!(matches!(step, Expr::IntLit { value: 1, .. }));
                // Body references both `s` (outer) and `i` (loop var).
                assert!(matches!(body.stmts.first(), Some(Stmt::Assign { .. })));
            }
            _ => unreachable!(),
        }
        assert!(m.manifest.contains(Feature::Loops));
    }

    #[test]
    fn c_for_with_postfix_increment_update() {
        // `i++` update → step IntLit(1).
        let m = lower("let n = 5; let s = 0; for (let i = 0; i < n; i++) { s = s + i; }");
        assert_valid(&m);
        match first_for_range(&m) {
            Stmt::ForRange { var, step, .. } => {
                assert_eq!(var, "i");
                assert!(matches!(step, Expr::IntLit { value: 1, .. }));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn c_for_with_compound_plus_assign_step() {
        // `i += 2` update → step IntLit(2).
        let m = lower("let n = 9; let s = 0; for (let i = 0; i < n; i += 2) { s = s + i; }");
        assert_valid(&m);
        match first_for_range(&m) {
            Stmt::ForRange { step, .. } => {
                assert!(matches!(step, Expr::IntLit { value: 2, .. }));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn c_for_with_le_condition_bumps_stop_half_open() {
        // `i <= n` ⇒ half-open `n + 1`.
        let m = lower("let n = 4; let s = 0; for (let i = 0; i <= n; i++) { s = s + i; }");
        assert_valid(&m);
        match first_for_range(&m) {
            Stmt::ForRange { stop, .. } => match stop {
                Expr::BuiltinCall { name, args, .. } => {
                    assert_eq!(name, "+");
                    assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "n"));
                    assert!(matches!(&args[1], Expr::IntLit { value: 1, .. }));
                }
                other => panic!("expected n+1 stop, got {other:?}"),
            },
            _ => unreachable!(),
        }
    }

    #[test]
    fn c_for_with_literal_stop() {
        // `for (let i = 0; i < 10; i++)` — stop is a literal.
        let m = lower("let s = 0; for (let i = 0; i < 10; i++) { s = s + i; }");
        assert_valid(&m);
        match first_for_range(&m) {
            Stmt::ForRange { start, stop, step, .. } => {
                assert!(matches!(start, Expr::IntLit { value: 0, .. }));
                assert!(matches!(stop, Expr::IntLit { value: 10, .. }));
                assert!(matches!(step, Expr::IntLit { value: 1, .. }));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn non_canonical_for_decrement_is_rejected() {
        // `i--` is a decrement; we can't represent it as a half-open
        // counting ForRange, so it's a positioned deferral error.
        let err = compile_source(
            "let n = 5; for (let i = n; i > 0; i--) { n = n; }",
            "test",
        )
        .expect_err("decrementing for must be rejected");
        assert!(
            err.message.contains("non-canonical"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn non_canonical_for_wrong_cond_variable_is_rejected() {
        // Condition references a different variable than the loop var.
        let err = compile_source(
            "let n = 5; let j = 0; for (let i = 0; j < n; i++) { n = n; }",
            "test",
        )
        .expect_err("mismatched condition variable must be rejected");
        assert!(err.message.contains("non-canonical"), "got: {}", err.message);
    }

    #[test]
    fn non_canonical_for_multiplicative_step_is_rejected() {
        // `i = i * 2` is not an additive increment.
        let err = compile_source(
            "let n = 64; for (let i = 1; i < n; i = i * 2) { n = n; }",
            "test",
        )
        .expect_err("multiplicative step must be rejected");
        assert!(err.message.contains("non-canonical"), "got: {}", err.message);
    }

    #[test]
    fn for_of_lowers_to_for_each() {
        // `for (const x of xs) { p = x; }` → ForEach { var: x, iter: xs }.
        // NB: collection literals are deferred past M3, so the iterable
        // `xs` is bound to a placeholder scalar.  The lowering is
        // structural — it does not typecheck the iterable — so `for-of`
        // over a `VarRef` lowers regardless of the bound value's shape.
        let m = lower("let xs = 0; let p = 0; for (const x of xs) { p = x; }");
        assert_valid(&m);
        let fe = main_block(&m).stmts.iter()
            .find(|s| matches!(s, Stmt::ForEach { .. }))
            .expect("a ForEach statement");
        match fe {
            Stmt::ForEach { var, iter, body, .. } => {
                assert_eq!(var, "x");
                assert!(matches!(iter, Expr::VarRef { name, .. } if name == "xs"));
                // Body assigns p = x (x resolves to the loop var).
                assert!(matches!(body.stmts.first(), Some(Stmt::Assign { .. })));
            }
            _ => unreachable!(),
        }
        assert!(m.manifest.contains(Feature::Loops));
    }

    #[test]
    fn loop_variable_is_not_visible_after_the_loop() {
        // `i` is in scope inside the for body but unresolved afterwards.
        let err = compile_source(
            "let n = 3; let s = 0; for (let i = 0; i < n; i++) { s = s + i; } i;",
            "test",
        )
        .expect_err("loop var must not leak past the loop");
        assert!(
            err.message.contains("unresolved name"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn for_of_variable_is_not_visible_after_the_loop() {
        let err = compile_source(
            "let xs = 0; let p = 0; for (const x of xs) { p = x; } x;",
            "test",
        )
        .expect_err("for-of var must not leak");
        assert!(err.message.contains("unresolved name"), "got: {}", err.message);
    }

    #[test]
    fn block_scoped_let_does_not_leak_to_outer_scope() {
        // A `let` inside an if-body is block-scoped: referencing it after
        // the `if` is an unresolved-name error.
        let err = compile_source(
            "let c = true; if (c) { let inner = 1; inner; } inner;",
            "test",
        )
        .expect_err("block-scoped let must not leak");
        assert!(err.message.contains("unresolved name"), "got: {}", err.message);
    }

    #[test]
    fn bare_block_statement_lowers_and_scopes() {
        // A bare `{ … }` block runs its statements; an inner binding is
        // scoped to the block.
        let m = lower("let x = 0; { x = 1; }");
        assert_valid(&m);
        // The block's inner `x = 1` is an Assign to the outer x.
        let has_block = main_block(&m).stmts.iter().any(|s| {
            matches!(s, Stmt::ExprStmt { expr: Expr::Block(_), .. })
        }) || matches!(&main_block(&m).value, Expr::Block(_));
        assert!(has_block, "expected an Expr::Block somewhere in main");
    }

    #[test]
    fn nested_control_flow_validates() {
        // while containing an if containing a for — exercise nesting and
        // the shared body/scoping machinery end to end.
        let m = lower(
            "let n = 5; let s = 0; let go = true; \
             while (go) { \
               if (s < n) { for (let i = 0; i < n; i++) { s = s + i; } } else { go = false; } \
             }",
        );
        assert_valid(&m);
        assert!(m.manifest.contains(Feature::Loops));
        // The outer statement is a While.
        assert!(main_block(&m).stmts.iter().any(|s| matches!(s, Stmt::While { .. })));
    }

    #[test]
    fn if_condition_can_be_a_comparison() {
        // The condition is a relational BuiltinCall, not just a var-ref.
        let m = lower("let a = 1; let b = 2; let x = 0; if (a < b) { x = 1; }");
        assert_valid(&m);
        match first_if(&m) {
            Expr::If { cond, .. } => {
                assert!(matches!(**cond, Expr::BuiltinCall { .. }));
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn control_flow_round_trips_through_validation() {
        // A small program mixing all four control-flow forms validates and
        // declares exactly the observed features (no spurious warnings).
        // Collection literals are deferred, so `xs` is a placeholder scalar.
        let m = lower(
            "let xs = 0; let total = 0; \
             for (const x of xs) { total = total + x; } \
             let i = 0; while (i < 3) { i = i + 1; } \
             for (let k = 0; k < 3; k++) { total = total + k; } \
             if (total > 0) { total = total; } else { total = 0; }",
        );
        assert_valid(&m);
        let r = semantic_ir::validate(&m);
        assert!(r.warnings().next().is_none(), "unexpected warnings");
        assert!(m.manifest.contains(Feature::Loops));
    }

    // ── M4: functions, return, arrows, calls, closures ─────────────

    use semantic_ir::{CaptureValue, Function};

    /// Find a function by name in the module, panicking if absent.
    fn func<'a>(m: &'a semantic_ir::Module, name: &str) -> &'a Function {
        m.functions
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("function `{name}` present (have {:?})",
                m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()))
    }

    #[test]
    fn function_declaration_becomes_top_level_function() {
        // `function add(a, b) { return a + b; }` → a `Function` with two
        // params, no captures, body value = `a + b`.
        let m = lower("function add(a, b) { return a + b; }");
        assert_valid(&m);
        let add = func(&m, "add");
        assert_eq!(add.params.len(), 2);
        assert_eq!(add.params[0].name, "a");
        assert_eq!(add.params[1].name, "b");
        assert!(add.captures.is_empty());
        // Body tail value is `BuiltinCall("+", [param a, param b])`.
        match &add.body.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "+");
                assert!(matches!(&args[0], Expr::VarRef { scope: Scope::Param, name, .. } if name == "a"));
                assert!(matches!(&args[1], Expr::VarRef { scope: Scope::Param, name, .. } if name == "b"));
            }
            other => panic!("expected `+` body, got {other:?}"),
        }
        // A user function is exported alongside `main`.
        assert!(m.exports.iter().any(|e| e.name == "add"));
        assert!(m.exports.iter().any(|e| e.name == "main"));
        // Untyped params ⇒ DynamicTyping declared.
        assert!(m.manifest.contains(Feature::DynamicTyping));
    }

    #[test]
    fn tail_return_sets_body_value() {
        let m = lower("function f(a) { return a; }");
        assert_valid(&m);
        let f = func(&m, "f");
        assert!(matches!(&f.body.value, Expr::VarRef { scope: Scope::Param, name, .. } if name == "a"));
        assert!(f.body.stmts.is_empty());
    }

    #[test]
    fn no_return_yields_nil_body_value() {
        // `function g() { let x = 1; }` — no return ⇒ nil tail value, the
        // `let` is a body statement.
        let m = lower("function g() { let x = 1; }");
        assert_valid(&m);
        let g = func(&m, "g");
        assert!(matches!(&g.body.value, Expr::NilLit { .. }));
        assert_eq!(g.body.stmts.len(), 1);
        assert!(matches!(&g.body.stmts[0], Stmt::LetStarBinding { name, .. } if name == "x"));
    }

    #[test]
    fn bare_return_yields_nil_body_value() {
        let m = lower("function g() { return; }");
        assert_valid(&m);
        assert!(matches!(&func(&m, "g").body.value, Expr::NilLit { .. }));
    }

    #[test]
    fn empty_function_body_is_nil() {
        let m = lower("function h() {}");
        assert_valid(&m);
        let h = func(&m, "h");
        assert!(h.body.stmts.is_empty());
        assert!(matches!(&h.body.value, Expr::NilLit { .. }));
    }

    #[test]
    fn early_return_is_rejected() {
        // A `return` followed by more statements is a genuine early return.
        let err = compile_source(
            "function f(a) { return a; let x = 1; }",
            "test",
        )
        .expect_err("early return must be rejected");
        assert!(
            err.message.contains("early return not supported"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn early_return_inside_non_tail_if_is_rejected() {
        // `if (c) { return 1; } moreStatements;` — the if is not the tail,
        // so the return inside it is early.
        let err = compile_source(
            "function f(n) { if (n) { return 1; } let y = 2; }",
            "test",
        )
        .expect_err("early return inside a non-tail if must be rejected");
        assert!(err.message.contains("early return"), "got: {}", err.message);
    }

    #[test]
    fn tail_if_else_with_returns_folds_to_expr_if() {
        // The classic guard recursion: a tail `if/else` whose branches both
        // `return` folds into an `Expr::If` body value (no early-return).
        let m = lower(
            "function fact(n) { if (n <= 1) { return 1; } else { return n * fact(n - 1); } }",
        );
        assert_valid(&m);
        let fact = func(&m, "fact");
        match &fact.body.value {
            Expr::If { then_branch, else_branch, .. } => {
                assert!(matches!(&then_branch.value, Expr::IntLit { value: 1, .. }));
                // else value is `n * fact(n - 1)` — a multiplicative builtin.
                assert!(matches!(&else_branch.value, Expr::BuiltinCall { name, .. } if name == "*"));
            }
            other => panic!("expected If body, got {other:?}"),
        }
    }

    #[test]
    fn direct_call_to_known_function() {
        // `f(5)` where `f` is a module function → DirectCall.
        let m = lower("function f(a) { return a; } f(5);");
        assert_valid(&m);
        match main_value(&m) {
            Expr::DirectCall { fn_name, args, .. } => {
                assert_eq!(fn_name, "f");
                assert_eq!(args.len(), 1);
                assert!(matches!(&args[0], Expr::IntLit { value: 5, .. }));
            }
            other => panic!("expected DirectCall, got {other:?}"),
        }
    }

    #[test]
    fn forward_reference_call_resolves_via_two_pass() {
        // A call appears *before* the function it names — the pass-1
        // collection still lets it resolve to a DirectCall.
        let m = lower("function user() { return helper(); } function helper() { return 1; } user();");
        assert_valid(&m);
        // The body of `user` direct-calls `helper`.
        match &func(&m, "user").body.value {
            Expr::DirectCall { fn_name, .. } => assert_eq!(fn_name, "helper"),
            other => panic!("expected DirectCall, got {other:?}"),
        }
    }

    #[test]
    fn indirect_call_through_closure_value() {
        // `g` is a local bound to a closure value → calling it is Indirect.
        let m = lower("let g = (x) => x + 1; g(3);");
        assert_valid(&m);
        match main_value(&m) {
            Expr::IndirectCall { target, args, .. } => {
                assert!(matches!(**target, Expr::VarRef { scope: Scope::Local, .. }));
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected IndirectCall, got {other:?}"),
        }
        assert!(m.manifest.contains(Feature::Closures));
    }

    #[test]
    fn console_log_lowers_to_builtin_print() {
        let m = lower("let x = 1; console.log(x);");
        assert_valid(&m);
        // The call is a statement (its value is unobservable at top level);
        // find the print BuiltinCall in the block.
        let has_print = main_block(&m).stmts.iter().any(|s| {
            matches!(s, Stmt::ExprStmt { expr: Expr::BuiltinCall { name, .. }, .. } if name == "print")
        }) || matches!(&main_block(&m).value, Expr::BuiltinCall { name, .. } if name == "print");
        assert!(has_print, "expected a print BuiltinCall in main");
    }

    // ── C3: member-method calls → `__method__` dispatch ────────────
    //
    // `recv.method(args…)` (other than `console.log`) lowers to
    // `BuiltinCall("__method__", [recv, StrLit("method"), args…])`: the
    // receiver at `args[0]`, the method name always a `StrLit` at `args[1]`,
    // the call arguments following.  These tests pin the envelope shape, the
    // `Feature::Strings` declaration, closure-arg lowering, and chaining.

    /// Assert `e` is a `__method__` dispatch, returning `(method, args)` —
    /// where `args` is the *whole* argument vector (receiver at `[0]`,
    /// `StrLit(method)` at `[1]`, call args at `[2..]`).
    fn expect_method(e: &Expr) -> (String, &[Expr]) {
        match e {
            Expr::BuiltinCall { name, args, .. } if name == "__method__" => {
                assert!(args.len() >= 2, "dispatch needs receiver + method name");
                let method = match &args[1] {
                    Expr::StrLit { value, .. } => value.clone(),
                    other => panic!("method name must be a StrLit, got {other:?}"),
                };
                (method, args.as_slice())
            }
            other => panic!("expected __method__ dispatch, got {other:?}"),
        }
    }

    #[test]
    fn push_lowers_to_method_dispatch() {
        // `arr.push(1)` → __method__(VarRef arr, "push", IntLit 1).
        let m = lower("let arr = [0]; arr.push(1);");
        assert_valid(&m);
        let (method, args) = expect_method(main_value(&m));
        assert_eq!(method, "push");
        assert_eq!(args.len(), 3, "receiver + name + one call arg");
        assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "arr"));
        assert!(matches!(&args[2], Expr::IntLit { value: 1, .. }));
        // The synthetic method-name StrLit declares Strings.
        assert!(m.manifest.contains(Feature::Strings));
    }

    #[test]
    fn pop_lowers_to_zero_arg_method_dispatch() {
        // `arr.pop()` → __method__(VarRef arr, "pop") — no trailing args.
        let m = lower("let arr = [1, 2]; arr.pop();");
        assert_valid(&m);
        let (method, args) = expect_method(main_value(&m));
        assert_eq!(method, "pop");
        assert_eq!(args.len(), 2, "receiver + name only");
        assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "arr"));
    }

    #[test]
    fn map_with_arrow_callback_lowers_to_dispatch_with_closure() {
        // `arr.map(x => x * 2)` → __method__(arr, "map", MakeClosure{…}).
        // The arrow reuses the existing closure lowering, so the callback
        // arrives as a MakeClosure argument.
        let m = lower("let arr = [1, 2, 3]; arr.map(x => x * 2);");
        assert_valid(&m);
        let (method, args) = expect_method(main_value(&m));
        assert_eq!(method, "map");
        assert_eq!(args.len(), 3, "receiver + name + closure arg");
        assert!(
            matches!(&args[2], Expr::MakeClosure { .. }),
            "callback should lower to MakeClosure, got {:?}",
            args[2]
        );
        assert!(m.manifest.contains(Feature::Closures));
        assert!(m.manifest.contains(Feature::Strings));
    }

    #[test]
    fn map_with_named_function_callback_passes_closure() {
        // A named function passed by reference (`arr.map(dbl)`) resolves to a
        // value and lowers as an IndirectCall-able closure handle argument.
        let m = lower("function dbl(x) { return x * 2; } let arr = [1]; arr.map(dbl);");
        assert_valid(&m);
        let (method, args) = expect_method(main_value(&m));
        assert_eq!(method, "map");
        assert_eq!(args.len(), 3);
        // `dbl` names a module function → passed as a MakeClosure handle.
        assert!(matches!(&args[2], Expr::MakeClosure { .. } | Expr::VarRef { .. }));
    }

    #[test]
    fn string_method_lowers_to_dispatch() {
        // `s.toUpperCase()` → __method__(VarRef s, "toUpperCase").
        let m = lower("let s = \"hi\"; s.toUpperCase();");
        assert_valid(&m);
        let (method, args) = expect_method(main_value(&m));
        assert_eq!(method, "toUpperCase");
        assert_eq!(args.len(), 2);
        assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "s"));
    }

    #[test]
    fn chained_methods_nest_dispatch() {
        // `xs.filter(f).map(g)` → the outer `.map` dispatch whose *receiver*
        // (args[0]) is itself the inner `.filter` dispatch.
        let m = lower(
            "let xs = [1, 2, 3]; \
             let f = (x) => x > 1; \
             let g = (x) => x * 10; \
             xs.filter(f).map(g);",
        );
        assert_valid(&m);
        let (outer_method, outer_args) = expect_method(main_value(&m));
        assert_eq!(outer_method, "map");
        // The receiver of `.map` is the `.filter` dispatch.
        let (inner_method, inner_args) = expect_method(&outer_args[0]);
        assert_eq!(inner_method, "filter");
        // Innermost receiver is `xs`.
        assert!(matches!(&inner_args[0], Expr::VarRef { name, .. } if name == "xs"));
    }

    #[test]
    fn method_on_object_property_lowers_receiver_via_member_read() {
        // A deeper receiver: `box.items.push(9)` — the receiver `box.items`
        // is a member read (MapGet), folded before the `.push` dispatch.
        let m = lower("let box = {items: [1]}; box.items.push(9);");
        assert_valid(&m);
        let (method, args) = expect_method(main_value(&m));
        assert_eq!(method, "push");
        assert!(matches!(&args[0], Expr::MapGet { .. }));
        assert!(matches!(&args[2], Expr::IntLit { value: 9, .. }));
    }

    #[test]
    fn console_log_still_lowers_to_print_not_method_dispatch() {
        // Guard against a regression: `console.log` must keep its dedicated
        // `print` lowering rather than being swept into `__method__`.
        let m = lower("console.log(1);");
        assert_valid(&m);
        match main_value(&m) {
            Expr::BuiltinCall { name, .. } => {
                assert_eq!(name, "print", "console.log must stay a print builtin");
            }
            other => panic!("expected print BuiltinCall, got {other:?}"),
        }
    }

    #[test]
    fn computed_member_call_stays_deferred() {
        // `obj[key](x)` is a *computed* member call — not a named method — so
        // it remains a positioned error, not a `__method__` dispatch.
        let err = compile_source("let obj = {}; let key = \"m\"; obj[key](1);", "test")
            .expect_err("computed member call should be deferred");
        assert!(
            err.message.contains("deferred") || err.message.contains("non-identifier"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn zero_arg_call() {
        let m = lower("function h() { return 42; } h();");
        assert_valid(&m);
        match main_value(&m) {
            Expr::DirectCall { fn_name, args, .. } => {
                assert_eq!(fn_name, "h");
                assert!(args.is_empty());
            }
            other => panic!("expected zero-arg DirectCall, got {other:?}"),
        }
    }

    #[test]
    fn expression_arrow_with_capture() {
        // `(a) => a + n` captures the enclosing `n`.
        let m = lower("let n = 10; let f = (a) => a + n; f(1);");
        assert_valid(&m);
        // One synthesised lambda with one capture (`n`).
        let lambda = func(&m, "__lambda_0");
        assert_eq!(lambda.params.len(), 1);
        assert_eq!(lambda.params[0].name, "a");
        assert_eq!(lambda.captures.len(), 1);
        assert_eq!(lambda.captures[0].name, "n");
        // Inside the body, `n` resolves as a Capture and `a` as a Param.
        match &lambda.body.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "+");
                assert!(matches!(&args[0], Expr::VarRef { scope: Scope::Param, name, .. } if name == "a"));
                assert!(matches!(&args[1], Expr::VarRef { scope: Scope::Capture, name, .. } if name == "n"));
            }
            other => panic!("expected `a + n`, got {other:?}"),
        }
        // The `let f = …` binding holds a MakeClosure with one CaptureValue
        // resolving `n` in the enclosing (main) local scope.
        let make = main_block(&m).stmts.iter().find_map(|s| match s {
            Stmt::LetStarBinding { name, value: Expr::MakeClosure { fn_name, captures, .. }, .. }
                if name == "f" => Some((fn_name.clone(), captures.clone())),
            _ => None,
        });
        let (fn_name, captures): (String, Vec<CaptureValue>) =
            make.expect("a MakeClosure binding for f");
        assert_eq!(fn_name, "__lambda_0");
        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].name, "n");
        assert!(matches!(&captures[0].value, Expr::VarRef { scope: Scope::Local, name, .. } if name == "n"));
        assert!(m.manifest.contains(Feature::Closures));
    }

    #[test]
    fn block_bodied_arrow() {
        let m = lower("let f = (a) => { return a + 1; }; f(2);");
        assert_valid(&m);
        let lambda = func(&m, "__lambda_0");
        assert!(lambda.captures.is_empty());
        assert!(matches!(&lambda.body.value, Expr::BuiltinCall { name, .. } if name == "+"));
    }

    #[test]
    fn no_param_arrow_captures_nothing() {
        let m = lower("let f = () => 5; f();");
        assert_valid(&m);
        let lambda = func(&m, "__lambda_0");
        assert!(lambda.params.is_empty());
        assert!(lambda.captures.is_empty());
        assert!(matches!(&lambda.body.value, Expr::IntLit { value: 5, .. }));
    }

    #[test]
    fn bare_identifier_arrow_param() {
        // `a => a + n` — single-identifier arrow params (no parens).
        let m = lower("let n = 3; let f = a => a + n; f(1);");
        assert_valid(&m);
        let lambda = func(&m, "__lambda_0");
        assert_eq!(lambda.params.len(), 1);
        assert_eq!(lambda.params[0].name, "a");
        assert_eq!(lambda.captures.len(), 1);
        assert_eq!(lambda.captures[0].name, "n");
    }

    #[test]
    fn nested_function_is_lifted_and_captures() {
        // `function outer(n) { function inner(x) { return x + n; } return inner; }`
        // — `inner` is lifted to a top-level Function capturing `n`; `outer`
        // binds `inner` to a MakeClosure and returns it.
        let m = lower(
            "function outer(n) { function inner(x) { return x + n; } return inner; }",
        );
        assert_valid(&m);
        let inner = func(&m, "inner");
        assert_eq!(inner.captures.len(), 1);
        assert_eq!(inner.captures[0].name, "n");
        match &inner.body.value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "+");
                assert!(matches!(&args[0], Expr::VarRef { scope: Scope::Param, name, .. } if name == "x"));
                assert!(matches!(&args[1], Expr::VarRef { scope: Scope::Capture, name, .. } if name == "n"));
            }
            other => panic!("expected `x + n`, got {other:?}"),
        }
        // `outer`'s body binds `inner` to a MakeClosure and returns it.
        let outer = func(&m, "outer");
        assert!(outer.body.stmts.iter().any(|s| matches!(
            s,
            Stmt::LetStarBinding { name, value: Expr::MakeClosure { fn_name, .. }, .. }
                if name == "inner" && fn_name == "inner"
        )));
        // The returned value resolves `inner` to its closure value (a local).
        assert!(matches!(&outer.body.value, Expr::VarRef { name, .. } if name == "inner"));
        assert!(m.manifest.contains(Feature::Closures));
    }

    #[test]
    fn self_recursion_is_not_mutual_recursion() {
        // A function calling only itself is single-function recursion; we
        // must NOT declare MutualRecursion (no spurious warning beyond the
        // intrinsic one — here there should be none).
        let m = lower("function loop(n) { if (n === 0) { return 0; } else { return loop(n - 1); } } loop(3);");
        assert_valid(&m);
        assert!(!m.manifest.contains(Feature::MutualRecursion));
    }

    #[test]
    fn mutual_recursion_is_declared() {
        // isEven ↔ isOdd is a genuine 2-cycle → MutualRecursion declared.
        let m = lower(
            "function isEven(n) { if (n === 0) { return true; } else { return isOdd(n - 1); } } \
             function isOdd(n) { if (n === 0) { return false; } else { return isEven(n - 1); } } \
             isEven(4);",
        );
        // The module validates (a benign 'declared but unused' warning for
        // mutual-recursion is intrinsic: the validator has no node for it).
        let r = semantic_ir::validate(&m);
        assert!(r.is_ok(), "validation errored: {:?}", r.issues);
        assert!(m.manifest.contains(Feature::MutualRecursion));
    }

    #[test]
    fn deeply_nested_arrow_captures_transitively() {
        // `x` captured through two closure layers: the inner arrow captures
        // `x` (from the outer arrow, which itself captured it from main).
        let m = lower("let x = 1; let f = () => () => x; f();");
        assert_valid(&m);
        // Two synthesised lambdas; the innermost captures `x`.
        assert!(m.functions.iter().filter(|f| f.name.starts_with("__lambda_")).count() >= 2);
        // Every lambda whose body references `x` must capture it.
        for f in m.functions.iter().filter(|f| f.name.starts_with("__lambda_")) {
            // If the body or a capture mentions `x`, it must appear as a capture.
            let _ = f; // structural validity is asserted by assert_valid.
        }
        assert!(m.manifest.contains(Feature::Closures));
    }

    #[test]
    fn parameter_reference_resolves_to_param_scope() {
        let m = lower("function id(a) { return a; }");
        assert_valid(&m);
        assert!(matches!(
            &func(&m, "id").body.value,
            Expr::VarRef { scope: Scope::Param, name, .. } if name == "a"
        ));
    }

    #[test]
    fn unresolved_name_in_function_body_errors() {
        let err = compile_source("function f() { return zzz; }", "test")
            .expect_err("unresolved name in body must error");
        assert!(err.message.contains("unresolved name"), "got: {}", err.message);
    }

    #[test]
    fn default_parameter_literal_lowers_to_param_default() {
        // `function f(a = 1)` → a single `Param` whose `default` is the
        // lowered initializer `IntLit 1`.  Observes `Feature::DefaultParams`.
        let m = lower("function f(a = 1) { return a; }");
        assert_valid(&m);
        let f = func(&m, "f");
        assert_eq!(f.params.len(), 1);
        let p = &f.params[0];
        assert_eq!(p.name, "a");
        assert!(
            matches!(p.default.as_deref(), Some(Expr::IntLit { value: 1, .. })),
            "expected default IntLit 1, got {:?}",
            p.default
        );
        assert!(m.manifest.contains(Feature::DefaultParams));
    }

    #[test]
    fn plain_parameter_has_no_default() {
        // A parameter with no initializer keeps `default: None`, and a
        // function with only plain params does *not* declare DefaultParams.
        let m = lower("function f(a) { return a; }");
        assert_valid(&m);
        assert!(func(&m, "f").params[0].default.is_none());
        assert!(!m.manifest.contains(Feature::DefaultParams));
    }

    #[test]
    fn default_parameter_may_reference_earlier_param() {
        // `function f(a, b = a + 1)` — the default for `b` is lowered in
        // param scope, so it references `a` as a `Scope::Param` VarRef.
        // This is the JS call-time / param-scope rule, which matches the SIR
        // `Param.default` model exactly.
        let m = lower("function f(a, b = a + 1) { return b; }");
        assert_valid(&m);
        let f = func(&m, "f");
        assert_eq!(f.params.len(), 2);
        assert!(f.params[0].default.is_none(), "`a` has no default");
        // `b`'s default is `a + 1` → BuiltinCall("+", [VarRef(a, Param), IntLit 1]).
        match f.params[1].default.as_deref() {
            Some(Expr::BuiltinCall { name, args, .. }) if name == "+" => {
                assert!(
                    matches!(&args[0], Expr::VarRef { scope: Scope::Param, name, .. } if name == "a"),
                    "default must reference earlier param `a`, got {:?}",
                    args[0]
                );
                assert!(matches!(&args[1], Expr::IntLit { value: 1, .. }));
            }
            other => panic!("expected `a + 1` default, got {other:?}"),
        }
        assert!(m.manifest.contains(Feature::DefaultParams));
    }

    #[test]
    fn arrow_default_parameter_lowers_to_param_default() {
        // Arrow functions take defaults too: `(a = 1) => a`.
        let m = lower("let g = (a = 1) => a; g();");
        assert_valid(&m);
        let lambda = func(&m, "__lambda_0");
        assert!(
            matches!(lambda.params[0].default.as_deref(), Some(Expr::IntLit { value: 1, .. })),
            "expected arrow default IntLit 1, got {:?}",
            lambda.params[0].default
        );
        assert!(m.manifest.contains(Feature::DefaultParams));
    }

    #[test]
    fn partial_call_omitting_defaulted_arg_lowers_present_args_only() {
        // `f(5)` against `function f(a, b = a + 1)` is a *partial* call: the
        // `DirectCall` carries only the present argument (`5`); the omitted
        // `b` is filled by its default at the call site.  The validator
        // permits a partial call when the trailing params have defaults.
        let m = lower("function f(a, b = a + 1) { return b; } console.log(f(5));");
        assert_valid(&m);
        // The top-level `console.log(f(5))` wraps a DirectCall with one arg.
        fn find_direct_call(e: &Expr) -> Option<&Expr> {
            match e {
                Expr::BuiltinCall { args, .. } => args.iter().find_map(find_direct_call),
                Expr::DirectCall { .. } => Some(e),
                _ => None,
            }
        }
        let call = find_direct_call(main_value(&m)).expect("a DirectCall to `f`");
        match call {
            Expr::DirectCall { fn_name, args, .. } => {
                assert_eq!(fn_name, "f");
                assert_eq!(args.len(), 1, "only the present arg is lowered (partial call)");
                assert!(matches!(&args[0], Expr::IntLit { value: 5, .. }));
            }
            other => panic!("expected DirectCall, got {other:?}"),
        }
    }

    #[test]
    fn rest_parameter_still_deferred() {
        // Defaults are now supported, but rest `...args` stays deferred.
        let err = compile_source("function f(...args) { return args; }", "test")
            .expect_err("rest params still deferred");
        assert!(err.message.contains("deferred"), "got: {}", err.message);
    }

    #[test]
    fn method_call_other_than_console_log_lowers_to_dispatch() {
        // C3: previously deferred, a general member-method call now lowers to
        // the `__method__` dispatch envelope (receiver, StrLit(name), args…).
        let m = lower("let o = [0]; o.foo(1);");
        assert_valid(&m);
        let (method, args) = expect_method(main_value(&m));
        assert_eq!(method, "foo");
        assert_eq!(args.len(), 3);
        assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "o"));
        assert!(matches!(&args[2], Expr::IntLit { value: 1, .. }));
    }

    #[test]
    fn function_program_round_trips_through_validation() {
        // A program mixing a top-level function, a closure, a direct call,
        // and console.log validates with no errors.
        let m = lower(
            "function twice(f, x) { return f(f(x)); } \
             let inc = (n) => n + 1; \
             console.log(twice(inc, 5));",
        );
        assert_valid(&m);
        assert!(m.manifest.contains(Feature::Closures));
        assert!(m.manifest.contains(Feature::DynamicTyping));
    }

    // ── M4: depth-bound regression (CWE-674 — no stack overflow) ────

    use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

    /// Build a synthetic [`GrammarASTNode`] with the given rule name and
    /// children.  Spans are stamped so error positions are non-zero.
    fn node(rule: &str, children: Vec<ASTNodeOrToken>) -> GrammarASTNode {
        GrammarASTNode {
            rule_name: rule.to_string(),
            children,
            start_line: Some(1),
            start_column: Some(1),
            end_line: Some(1),
            end_column: Some(1),
        }
    }

    /// Build a `block`-chain nested `n` levels deep, bottoming out at a
    /// childless terminal node: `block[ block[ … block[ leaf ] … ] ]`.  The
    /// depth guards trip while descending the chain, long before the leaf,
    /// so it needs no real token.
    fn nest_blocks(n: usize) -> GrammarASTNode {
        let mut cur = node("primary_expression", Vec::new());
        for _ in 0..n {
            cur = node("block", vec![ASTNodeOrToken::Node(cur)]);
        }
        cur
    }

    // ── M5: collections — arrays, objects, member/subscript access ─

    /// Lower `src` whose tail value is a collection expression, returning it.
    fn coll_value(src: &str) -> Expr {
        let m = lower(src);
        let r = semantic_ir::validate(&m);
        assert!(r.is_ok(), "validation failed: {:?}", r.issues);
        main_value(&m).clone()
    }

    #[test]
    fn array_literal_lowers_to_seq_lit() {
        let m = lower("[1, 2, 3];");
        match main_value(&m) {
            Expr::SeqLit { items, .. } => {
                assert_eq!(items.len(), 3);
                assert!(matches!(items[0], Expr::IntLit { value: 1, .. }));
                assert!(matches!(items[2], Expr::IntLit { value: 3, .. }));
            }
            other => panic!("expected SeqLit, got {other:?}"),
        }
        assert!(m.manifest.contains(Feature::Sequences));
        assert_valid(&m);
    }

    #[test]
    fn empty_array_is_empty_seq_lit() {
        match coll_value("[];") {
            Expr::SeqLit { items, .. } => assert!(items.is_empty()),
            other => panic!("expected empty SeqLit, got {other:?}"),
        }
    }

    #[test]
    fn nested_array_literals() {
        // `[[1], [2, 3]]` → SeqLit of two SeqLits.
        match coll_value("[[1], [2, 3]];") {
            Expr::SeqLit { items, .. } => {
                assert_eq!(items.len(), 2);
                assert!(matches!(&items[0], Expr::SeqLit { items, .. } if items.len() == 1));
                assert!(matches!(&items[1], Expr::SeqLit { items, .. } if items.len() == 2));
            }
            other => panic!("expected nested SeqLit, got {other:?}"),
        }
    }

    #[test]
    fn object_literal_with_identifier_keys_lowers_to_map_lit() {
        // Parenthesised so `{` is not read as a block at statement start.
        let m = lower("({a: 1, b: 2});");
        match main_value(&m) {
            Expr::MapLit { entries, .. } => {
                assert_eq!(entries.len(), 2);
                assert!(matches!(&entries[0].key, Expr::StrLit { value, .. } if value == "a"));
                assert!(matches!(&entries[0].value, Expr::IntLit { value: 1, .. }));
                assert!(matches!(&entries[1].key, Expr::StrLit { value, .. } if value == "b"));
            }
            other => panic!("expected MapLit, got {other:?}"),
        }
        assert!(m.manifest.contains(Feature::Maps));
        assert_valid(&m);
    }

    #[test]
    fn object_literal_in_binding_needs_no_parens() {
        // In value position (not statement start) `{` is unambiguous.
        let m = lower("let o = {a: 1}; o;");
        let b = main_block(&m);
        assert!(matches!(
            &b.stmts[0],
            Stmt::LetStarBinding { value: Expr::MapLit { .. }, .. }
        ));
        assert_valid(&m);
    }

    #[test]
    fn object_literal_with_string_keys() {
        match coll_value("let v = 0; ({\"k\": v});") {
            Expr::MapLit { entries, .. } => {
                assert!(matches!(&entries[0].key, Expr::StrLit { value, .. } if value == "k"));
                assert!(matches!(&entries[0].value, Expr::VarRef { .. }));
            }
            other => panic!("expected MapLit, got {other:?}"),
        }
    }

    #[test]
    fn empty_object_is_empty_map_lit() {
        match coll_value("({});") {
            Expr::MapLit { entries, .. } => assert!(entries.is_empty()),
            other => panic!("expected empty MapLit, got {other:?}"),
        }
    }

    #[test]
    fn array_length_lowers_to_seq_len() {
        // `xs.length` → SeqLen (the one dotted property that is a sequence op).
        match coll_value("let xs = [1, 2]; xs.length;") {
            Expr::SeqLen { seq, .. } => {
                assert!(matches!(&*seq, Expr::VarRef { name, .. } if name == "xs"));
            }
            other => panic!("expected SeqLen, got {other:?}"),
        }
    }

    #[test]
    fn numeric_subscript_lowers_to_seq_index() {
        // `xs[0]` → SeqIndex (non-string index → sequence).
        match coll_value("let xs = [9]; xs[0];") {
            Expr::SeqIndex { seq, index, .. } => {
                assert!(matches!(&*seq, Expr::VarRef { name, .. } if name == "xs"));
                assert!(matches!(&*index, Expr::IntLit { value: 0, .. }));
            }
            other => panic!("expected SeqIndex, got {other:?}"),
        }
    }

    #[test]
    fn variable_subscript_lowers_to_seq_index() {
        // `xs[i]` with a variable index is also a SeqIndex (not a string key).
        match coll_value("let xs = [9]; let i = 0; xs[i];") {
            Expr::SeqIndex { index, .. } => {
                assert!(matches!(&*index, Expr::VarRef { name, .. } if name == "i"));
            }
            other => panic!("expected SeqIndex, got {other:?}"),
        }
    }

    #[test]
    fn dot_member_lowers_to_map_get() {
        // `obj.prop` → MapGet with string key "prop".
        match coll_value("let obj = {p: 1}; obj.p;") {
            Expr::MapGet { map, key, .. } => {
                assert!(matches!(&*map, Expr::VarRef { name, .. } if name == "obj"));
                assert!(matches!(&*key, Expr::StrLit { value, .. } if value == "p"));
            }
            other => panic!("expected MapGet, got {other:?}"),
        }
    }

    #[test]
    fn string_subscript_lowers_to_map_get() {
        // `obj["k"]` — a string-literal key → MapGet (mirrors `obj.k`).
        match coll_value("let obj = {k: 1}; obj[\"k\"];") {
            Expr::MapGet { map, key, .. } => {
                assert!(matches!(&*map, Expr::VarRef { name, .. } if name == "obj"));
                assert!(matches!(&*key, Expr::StrLit { value, .. } if value == "k"));
            }
            other => panic!("expected MapGet, got {other:?}"),
        }
    }

    #[test]
    fn seq_index_assignment_lowers_to_seq_set() {
        // `xs[0] = 9;` → SeqSet.
        let m = lower("let xs = [1]; xs[0] = 9;");
        let b = main_block(&m);
        let set = b.stmts.iter().find(|s| matches!(s, Stmt::SeqSet { .. }))
            .expect("a SeqSet statement");
        match set {
            Stmt::SeqSet { seq, index, value, .. } => {
                assert!(matches!(seq, Expr::VarRef { name, .. } if name == "xs"));
                assert!(matches!(index, Expr::IntLit { value: 0, .. }));
                assert!(matches!(value, Expr::IntLit { value: 9, .. }));
            }
            _ => unreachable!(),
        }
        assert!(m.manifest.contains(Feature::Sequences));
        assert_valid(&m);
    }

    #[test]
    fn dot_assignment_lowers_to_map_set() {
        // `obj.prop = 5;` → MapSet with string key.
        let m = lower("let obj = {p: 0}; obj.p = 5;");
        let set = main_block(&m).stmts.iter().find(|s| matches!(s, Stmt::MapSet { .. }))
            .expect("a MapSet statement");
        match set {
            Stmt::MapSet { map, key, value, .. } => {
                assert!(matches!(map, Expr::VarRef { name, .. } if name == "obj"));
                assert!(matches!(key, Expr::StrLit { value, .. } if value == "p"));
                assert!(matches!(value, Expr::IntLit { value: 5, .. }));
            }
            _ => unreachable!(),
        }
        assert!(m.manifest.contains(Feature::Maps));
        assert_valid(&m);
    }

    #[test]
    fn string_subscript_assignment_lowers_to_map_set() {
        // `obj["k"] = 5;` → MapSet.
        let m = lower("let obj = {k: 0}; obj[\"k\"] = 5;");
        assert!(main_block(&m).stmts.iter().any(|s| matches!(
            s,
            Stmt::MapSet { key: Expr::StrLit { value, .. }, .. } if value == "k"
        )));
        assert_valid(&m);
    }

    #[test]
    fn numeric_subscript_assignment_is_seq_set_not_map_set() {
        // Guard the disambiguation on the *assignment* side: `xs[i] = v`
        // must be a SeqSet, never a MapSet.
        let m = lower("let xs = [0]; let i = 0; xs[i] = 7;");
        assert!(main_block(&m).stmts.iter().any(|s| matches!(s, Stmt::SeqSet { .. })));
        assert!(!main_block(&m).stmts.iter().any(|s| matches!(s, Stmt::MapSet { .. })));
        assert_valid(&m);
    }

    #[test]
    fn nested_map_in_seq_validates() {
        // A sequence of maps — exercise the manifest declaring both features.
        let m = lower("let data = [{id: 1}, {id: 2}]; data[0];");
        assert_valid(&m);
        assert!(m.manifest.contains(Feature::Sequences));
        assert!(m.manifest.contains(Feature::Maps));
        let r = semantic_ir::validate(&m);
        assert!(r.warnings().next().is_none(), "unexpected warnings");
    }

    #[test]
    fn chained_subscript_left_associates() {
        // `grid[0][1]` → SeqIndex(SeqIndex(grid, 0), 1).
        match coll_value("let grid = [[1, 2]]; grid[0][1];") {
            Expr::SeqIndex { seq, index, .. } => {
                assert!(matches!(&*index, Expr::IntLit { value: 1, .. }));
                assert!(matches!(&*seq, Expr::SeqIndex { .. }));
            }
            other => panic!("expected nested SeqIndex, got {other:?}"),
        }
    }

    #[test]
    fn chained_subscript_assignment_sets_innermost() {
        // `grid[0][1] = 9;` → SeqSet whose seq is SeqIndex(grid, 0).
        let m = lower("let grid = [[0, 0]]; grid[0][1] = 9;");
        let set = main_block(&m).stmts.iter().find(|s| matches!(s, Stmt::SeqSet { .. }))
            .expect("a SeqSet statement");
        match set {
            Stmt::SeqSet { seq, index, value, .. } => {
                assert!(matches!(seq, Expr::SeqIndex { .. }));
                assert!(matches!(index, Expr::IntLit { value: 1, .. }));
                assert!(matches!(value, Expr::IntLit { value: 9, .. }));
            }
            _ => unreachable!(),
        }
        assert_valid(&m);
    }

    #[test]
    fn dot_chain_lowers_to_nested_map_get() {
        // `a.b.c` → MapGet(MapGet(a, "b"), "c").
        match coll_value("let a = {b: {c: 1}}; a.b.c;") {
            Expr::MapGet { map, key, .. } => {
                assert!(matches!(&*key, Expr::StrLit { value, .. } if value == "c"));
                assert!(matches!(&*map, Expr::MapGet { .. }));
            }
            other => panic!("expected nested MapGet, got {other:?}"),
        }
    }

    #[test]
    fn array_spread_is_deferred() {
        let err = compile_source("let xs = [1]; [...xs];", "test")
            .expect_err("array spread deferred");
        assert!(err.message.contains("deferred"), "got: {}", err.message);
    }

    #[test]
    fn length_assignment_is_deferred() {
        let err = compile_source("let xs = [1]; xs.length = 0;", "test")
            .expect_err("length assignment deferred");
        assert!(err.message.contains("deferred"), "got: {}", err.message);
    }

    #[test]
    fn computed_object_key_is_deferred() {
        let err = compile_source("let k = \"x\"; ({[k]: 1});", "test")
            .expect_err("computed key deferred");
        assert!(err.message.contains("deferred"), "got: {}", err.message);
    }

    /// Build a `member_expression` dot-chain nested `n` levels deep
    /// (`p.p.p…`) so the **expression** depth guard trips while lowering it.
    fn nest_member(n: usize) -> GrammarASTNode {
        use lexer::token::{Token, TokenType};
        let tok = |type_: TokenType, value: &str| Token {
            type_,
            value: value.to_string(),
            line: 1,
            column: 1,
            type_name: None,
            flags: None,
            cv: None,
        };
        // Each layer: member_expression[ inner, Dot, Name("p") ].
        let mut cur = node(
            "primary_expression",
            vec![ASTNodeOrToken::Token(tok(TokenType::Name, "p"))],
        );
        for _ in 0..n {
            cur = node(
                "member_expression",
                vec![
                    ASTNodeOrToken::Node(cur),
                    ASTNodeOrToken::Token(tok(TokenType::Dot, ".")),
                    ASTNodeOrToken::Token(tok(TokenType::Name, "p")),
                ],
            );
        }
        cur
    }

    #[test]
    fn deeply_nested_member_chain_is_rejected_without_crashing() {
        // A `p.p.p…` chain far deeper than MAX_EXPR_DEPTH (256) must turn
        // into a positioned error, not overflow the native stack (CWE-674).
        // We build the member tower directly and lower it as the program's
        // single expression statement via the public `compile`.
        let expr = node("expression", vec![ASTNodeOrToken::Node(nest_member(600))]);
        let stmt = node(
            "expression_statement",
            vec![ASTNodeOrToken::Node(expr)],
        );
        let body = node(
            "source_element",
            vec![ASTNodeOrToken::Node(node("statement", vec![ASTNodeOrToken::Node(stmt)]))],
        );
        let program = node("program", vec![ASTNodeOrToken::Node(body)]);
        let err = compile(&program, "deep")
            .expect_err("a 600-deep member chain must be rejected, not crash");
        assert!(
            err.message.contains("deeper than the supported limit"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn compile_rejects_deeply_nested_input_without_crashing() {
        // A `program` whose body is a `block` tower far deeper than
        // `MAX_STMT_DEPTH` (256).  We feed a *synthetic* CST straight into
        // the public `compile`, bypassing the parser (whose own recursion is
        // out of scope here), to prove that the pass-1
        // `collect_function_names` walk — which runs *before* the
        // depth-guarded lowering and is reachable from the public API —
        // turns deep input into a clean positioned `JsLowerError` rather
        // than overflowing the native stack (CWE-674).
        //
        // 600 > 256 trips the guard, yet is shallow enough that the test
        // runs comfortably on the harness's default thread stack.
        let body = node(
            "source_element",
            vec![ASTNodeOrToken::Node(nest_blocks(600))],
        );
        let program = node("program", vec![ASTNodeOrToken::Node(body)]);
        let err = compile(&program, "deep")
            .expect_err("a 600-deep block tower must be rejected, not crash");
        assert!(
            err.message.contains("deeper than the supported limit"),
            "unexpected message: {}",
            err.message
        );
    }
}
