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
//! ## Milestone status — M1 (literals)
//!
//! This build implements **literal lowering only**:
//!
//! - JS number → [`IntLit`](semantic_ir::Expr::IntLit) when the literal
//!   text is an integer (no `.`/exponent), else
//!   [`FloatLit`](semantic_ir::Expr::FloatLit).
//! - `true`/`false` → [`BoolLit`](semantic_ir::Expr::BoolLit).
//! - `null` **and** `undefined` → [`NilLit`](semantic_ir::Expr::NilLit)
//!   (the JS distinction is intentionally lost in v0).
//! - string → [`StrLit`](semantic_ir::Expr::StrLit).
//!
//! All other syntax (variables, operators, control flow, functions,
//! collections, template literals) is **deferred** to later milestones
//! and currently produces a clear [`JsLowerError`].  See the crate
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

    // ── error paths ────────────────────────────────────────────────

    #[test]
    fn operator_expression_is_rejected_in_m1() {
        // `1 + 2` is a non-literal expression ⇒ out of M1 scope.
        let err = compile_source("1 + 2;", "test").expect_err("should reject operators");
        assert!(
            err.message.contains("out of scope") || err.message.contains("M1"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bare_identifier_reference_is_rejected_in_m1() {
        // A variable reference (not `undefined`) is deferred to M2.
        let err = compile_source("x;", "test").expect_err("should reject var refs");
        assert!(
            err.message.contains("variable reference") || err.message.contains("out of scope"),
            "unexpected message: {}",
            err.message
        );
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
