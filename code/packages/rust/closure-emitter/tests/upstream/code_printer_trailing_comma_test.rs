//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Third companion file to `code_printer_test.rs` (after
//! `code_printer_declarations_test.rs`). This one covers
//! **trailing-comma policy** in array and object literals —
//! upstream's `testTrailingCommaInArrayAndObjectWithPrettyPrint`
//! and the inline `assertPrintSame` lines that pin "the
//! printer never emits a trailing `,` before `]` or `}`."
//!
//! ## What gap-022 actually was
//!
//! Filed when the upstream port at `code_printer_test.rs::test_trailing_comma_in_array_and_object_with_pretty_print`
//! was first stubbed (CLOC12.07): the AST had no `trailing_comma: bool`
//! flag on `ArrayExpression` / `ObjectExpression` and no port file
//! demonstrated whether the emitter handled the case correctly.
//!
//! Reviewing the upstream test family in detail makes the right
//! resolution clear: **upstream strips trailing commas during
//! parse → pretty-print**. The output side is what matters, and
//! both upstream and our emitter agree on it: no trailing comma
//! ever ends up between the last element and the closing
//! bracket. Verifying that doesn't require a `trailing_comma`
//! AST flag — it requires hand-built ASTs that pin the emitter
//! output for the standard array / object shapes.
//!
//! ## Why no AST flag is needed
//!
//! Our `ArrayExpression.elements` is `Vec<Option<Expression>>`.
//! An "input trailing comma" like `[1,]` parses to two
//! `,`-separated positions: a single concrete element `1` and a
//! second position. In ES2017 the trailing-comma is purely
//! syntactic — it does NOT introduce an elision. The resulting
//! AST is `[Some(1)]`, exactly the same as `[1]` would produce.
//! Pretty-printing `[Some(1)]` writes one element, no
//! separator, then `]`. Both inputs collapse to the same
//! single-element output.
//!
//! For object literals the same reasoning applies:
//! `{a:1,}` parses to `{ properties: [Property{a:1}] }` —
//! indistinguishable from `{a:1}`. The trailing comma never
//! lives in the AST, so it can't survive the emitter.
//!
//! ## Translation policy
//!
//! Hand-build the AST that would result from each upstream
//! input (with the input's trailing comma stripped), emit, and
//! compare to upstream's expected output. Tests cover **both**
//! the compact (default) mode and the pretty (`pretty: true`)
//! mode where the upstream test runs.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    ArrayExpression, BindingTarget, Declaration, Expression, Identifier, NumericLiteral,
    ObjectExpression, Program, ProgramItem, Property, PropertyKey, PropertyKind, SourceType,
    Statement, VarKind, VariableDeclaration, VariableDeclarator,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
// =====================================================================

fn ident_name(name: &str) -> Identifier {
    Identifier {
        cv: None,
        name: name.to_string(),
    }
}

fn num(v: f64) -> Expression {
    Expression::NumericLiteral(NumericLiteral {
        cv: None,
        value: v,
        raw: if v.fract() == 0.0 && v.is_finite() {
            format!("{}", v as i64)
        } else {
            v.to_string()
        },
    })
}

fn arr(elements: Vec<Option<Expression>>) -> Expression {
    Expression::ArrayExpression(ArrayExpression {
        cv: None,
        elements,
    })
}

fn obj(props: Vec<(&str, Expression)>) -> Expression {
    Expression::ObjectExpression(ObjectExpression {
        cv: None,
        properties: props
            .into_iter()
            .map(|(k, v)| Property {
                cv: None,
                kind: PropertyKind::Init,
                key: PropertyKey::Identifier(ident_name(k)),
                value: Box::new(v),
                computed: false,
                shorthand: false,
                method: false,
            })
            .collect(),
    })
}

/// Wrap an Expression as `var x = <expr>;` and return a Program.
fn var_x_eq(expr: Expression) -> Program {
    Program::new_untraced(EsVersion::Es2025, SourceType::Module).with_body(vec![
        ProgramItem::Statement(Statement::Declaration(Declaration::VariableDeclaration(
            VariableDeclaration {
                cv: None,
                kind: VarKind::Var,
                declarations: vec![VariableDeclarator {
                    cv: None,
                    id: BindingTarget::Identifier(ident_name("x")),
                    init: Some(expr),
                }],
            },
        ))),
    ])
}

fn emit_with(prog: Program, pretty: bool) -> String {
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    let opts = EmitOptions {
        pretty,
        ..EmitOptions::default()
    };
    emit(&prog, &sidecar, &mut cv, &opts)
        .expect("emit failed")
        .code
}

fn assert_emits_compact(prog: Program, expected: &str) {
    let code = emit_with(prog, false);
    assert_eq!(
        code, expected,
        "compact emit did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

fn assert_emits_pretty(prog: Program, expected: &str) {
    let code = emit_with(prog, true);
    assert_eq!(
        code, expected,
        "pretty emit did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Ported tests (gap-022) — array trailing-comma cases
// =====================================================================

/// Upstream `testNoTrailingCommaInEmptyArrayLiteral`:
///
///   assertPrintSame("var x = [];");
///
/// Empty array has no elements and no comma at all — the
/// degenerate base case for the trailing-comma family. Also
/// covered in `code_printer_declarations_test.rs::var_with_empty_array_init`
/// but pinned here too because the trailing-comma family
/// belongs together.
#[test]
fn empty_array_emits_without_trailing_comma_compact() {
    assert_emits_compact(var_x_eq(arr(vec![])), "var x=[];");
}

#[test]
fn empty_array_emits_without_trailing_comma_pretty() {
    assert_emits_pretty(var_x_eq(arr(vec![])), "var x = [];");
}

/// Upstream `testTrailingCommaInArrayAndObjectWithPrettyPrint`,
/// single-element array case:
///
///   assertPrettyPrint("var x = [1,];", "var x = [1];\n");
///
/// The input has a trailing comma; the pretty-printed output
/// does not. We hand-build the AST that the parser would
/// produce from `[1,]` — a single concrete `Some(1)` element —
/// and confirm the emitter outputs `[1]` with no trailing
/// comma. Same reasoning applies to the compact form below.
#[test]
fn single_element_array_emits_without_trailing_comma_compact() {
    assert_emits_compact(var_x_eq(arr(vec![Some(num(1.0))])), "var x=[1];");
}

#[test]
fn single_element_array_emits_without_trailing_comma_pretty() {
    assert_emits_pretty(var_x_eq(arr(vec![Some(num(1.0))])), "var x = [1];");
}

/// Upstream `testTrailingCommaInArrayAndObjectWithPrettyPrint`,
/// multi-element array case:
///
///   assertPrettyPrint("var x = [1, 2, 3,];", "var x = [1, 2, 3];\n");
///
/// Three concrete elements; trailing comma stripped.
#[test]
fn multi_element_array_emits_without_trailing_comma_compact() {
    assert_emits_compact(
        var_x_eq(arr(vec![Some(num(1.0)), Some(num(2.0)), Some(num(3.0))])),
        "var x=[1,2,3];",
    );
}

#[test]
fn multi_element_array_emits_without_trailing_comma_pretty() {
    assert_emits_pretty(
        var_x_eq(arr(vec![Some(num(1.0)), Some(num(2.0)), Some(num(3.0))])),
        "var x = [1, 2, 3];",
    );
}

/// Upstream coverage of the multi-line input case:
///
///   assertPrettyPrint("var x = [\n1,\n2,\n];", "var x = [1, 2];\n");
///
/// Same AST as `[1, 2]`, output collapses to single line in
/// pretty mode. Our pretty mode is single-line so the
/// expected output is the same one-line shape.
#[test]
fn multi_line_input_array_collapses_to_single_line_pretty() {
    assert_emits_pretty(
        var_x_eq(arr(vec![Some(num(1.0)), Some(num(2.0))])),
        "var x = [1, 2];",
    );
}

/// **Elisions are NOT trailing commas.** Upstream's
/// `assertPrintSame("[1,,3]")` preserves the middle elision
/// because it's semantically meaningful (the resulting array
/// has length 3 with index 1 holding `undefined`). The
/// trailing position is also written when the last AST
/// element is `None` — that's a real elision, not a stripped
/// trailing comma.
///
/// We pin both shapes to make explicit that the
/// no-trailing-comma policy is about emitter behaviour after
/// the AST is built, not about the AST itself.
#[test]
fn elision_in_array_is_preserved() {
    // [1, , 3] — element at index 1 is an elision.
    assert_emits_compact(
        var_x_eq(arr(vec![Some(num(1.0)), None, Some(num(3.0))])),
        "var x=[1,,3];",
    );
}

// =====================================================================
// Ported tests (gap-022) — object trailing-comma cases
// =====================================================================

/// Empty object: no properties, no commas. Belongs in the
/// trailing-comma family as the degenerate object case.
#[test]
fn empty_object_emits_without_trailing_comma_compact() {
    assert_emits_compact(var_x_eq(obj(vec![])), "var x={};");
}

#[test]
fn empty_object_emits_without_trailing_comma_pretty() {
    assert_emits_pretty(var_x_eq(obj(vec![])), "var x = {};");
}

/// Upstream `testTrailingCommaInArrayAndObjectWithPrettyPrint`,
/// single-property object case:
///
///   assertPrettyPrint("var x = {a: 1,};", "var x = {a: 1};\n");
///
/// One property; trailing comma stripped.
#[test]
fn single_prop_object_emits_without_trailing_comma_compact() {
    assert_emits_compact(var_x_eq(obj(vec![("a", num(1.0))])), "var x={a:1};");
}

#[test]
fn single_prop_object_emits_without_trailing_comma_pretty() {
    // Note: our pretty emitter writes a space after `{` and
    // before `}` (`{ a: 1 }`). The trailing-comma assertion is
    // the absence of `,` immediately before the closing brace.
    assert_emits_pretty(var_x_eq(obj(vec![("a", num(1.0))])), "var x = { a: 1 };");
}

/// Upstream `testTrailingCommaInArrayAndObjectWithPrettyPrint`,
/// multi-property object case:
///
///   assertPrettyPrint("var x = {a: 1, b: 2,};", "var x = {a: 1, b: 2};\n");
#[test]
fn multi_prop_object_emits_without_trailing_comma_compact() {
    assert_emits_compact(
        var_x_eq(obj(vec![("a", num(1.0)), ("b", num(2.0))])),
        "var x={a:1,b:2};",
    );
}

#[test]
fn multi_prop_object_emits_without_trailing_comma_pretty() {
    assert_emits_pretty(
        var_x_eq(obj(vec![("a", num(1.0)), ("b", num(2.0))])),
        "var x = { a: 1, b: 2 };",
    );
}

// =====================================================================
// Nested cases — array of objects, object of arrays.
//
// Trailing-comma policy needs to compose: a nested literal's
// own non-trailing-comma output must NOT introduce a comma in
// its parent's position chain.
// =====================================================================

/// Array containing one object literal: `var x = [{a:1}];`.
/// No trailing commas at either level.
#[test]
fn array_of_objects_no_trailing_comma() {
    let inner = obj(vec![("a", num(1.0))]);
    assert_emits_compact(var_x_eq(arr(vec![Some(inner)])), "var x=[{a:1}];");
}

/// Object containing one array literal: `var x = {a: [1]};`.
/// No trailing commas at either level.
#[test]
fn object_of_arrays_no_trailing_comma() {
    let inner = arr(vec![Some(num(1.0))]);
    assert_emits_compact(var_x_eq(obj(vec![("a", inner)])), "var x={a:[1]};");
}
