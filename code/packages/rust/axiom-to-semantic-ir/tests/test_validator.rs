//! Structural verification of lowered modules: every module this frontend
//! produces must pass the shared SIR validator (confirming the manifest
//! declares exactly the SIR23 features the module actually uses — the same
//! ground truth `semantic-ir/src/validator.rs`'s `check_expr` enforces node
//! kind for node kind), and every module must be **accepted** by the JS
//! backend.
//!
//! The three reserved head names this crate introduces
//! (`__axiom_declare`/`__axiom_coerce`/`__axiom_has`, see `src/lower.rs`'s
//! module doc comment for the full design decision) have no evaluation
//! handler in any backend today — but that is a runtime-evaluation concern,
//! not a SIR validation or backend-*acceptance* one: each is an ordinary
//! `SymApply` node with an unusual head *name*, and the JS backend's SIR23
//! codegen handles any `SymApply`/`SymSymbol` shape uniformly regardless of
//! head spelling (confirmed directly by reading `semantic-ir-to-javascript`'s
//! SIR23 `match` arms — the same fact `maple-to-semantic-ir`'s own `Set` and
//! `reduce-to-semantic-ir`'s own `CompoundExpression`/`Cons`/… already lean
//! on). So every construct here is still expected to validate and be
//! accepted.

use coding_adventures_axiom_to_semantic_ir::compile_source;
use semantic_ir::backend::Backend;
use semantic_ir::Feature;
use semantic_ir_to_javascript::JavaScriptBackend;

fn assert_valid(src: &str) -> semantic_ir::Module {
    let module = compile_source(src, "prog").unwrap_or_else(|e| panic!("lowering failed: {e}"));
    let report = semantic_ir::validate(&module);
    assert!(
        report.is_ok(),
        "SIR validation failed for `{src}`: {:?}",
        report.issues
    );
    module
}

fn assert_js_backend_accepts(module: &semantic_ir::Module) {
    let backend = JavaScriptBackend;
    let errors = backend.check_module(module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a module using only SIR23 features \
         (codegen for them has been implemented since HML01 Stream B rollout item 7): {errors:?}"
    );
}

#[test]
fn bare_arithmetic_validates_and_declares_symbolic_expr() {
    let module = assert_valid("1 + 2");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert!(!module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_accepts(&module);
}

#[test]
fn a_string_literal_validates_and_declares_strings() {
    let module = assert_valid("\"hello\"");
    assert!(module.manifest.iter().any(|f| f == Feature::Strings));
    assert_js_backend_accepts(&module);
}

#[test]
fn a_declared_function_definition_and_call_validates() {
    let module = assert_valid("power(x: Integer, n: NonNegativeInteger): Integer == x ** n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert!(!module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_accepts(&module);
}

#[test]
fn an_undeclared_function_definition_validates() {
    let module = assert_valid("f x == x * x");
    assert_js_backend_accepts(&module);
}

#[test]
fn assignment_validates() {
    let module = assert_valid("x := 5");
    assert_js_backend_accepts(&module);
}

#[test]
fn a_list_literal_validates() {
    let module = assert_valid("[1, 2, 3]");
    assert_js_backend_accepts(&module);
}

#[test]
fn if_then_else_validates() {
    let module = assert_valid("if x > 0 then 1 else -1");
    assert_js_backend_accepts(&module);
}

#[test]
fn a_multi_statement_block_validates() {
    let module = assert_valid("(a := 1; a + 1)");
    assert_js_backend_accepts(&module);
}

#[test]
fn a_declaration_validates_despite_no_shared_evaluator_for_axiom_declare() {
    let module = assert_valid("a : PositiveInteger");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert_js_backend_accepts(&module);
}

#[test]
fn a_tuple_declaration_validates() {
    let module = assert_valid("(a, b, c) : Integer");
    assert_js_backend_accepts(&module);
}

#[test]
fn a_coercion_validates_despite_no_shared_evaluator_for_axiom_coerce() {
    let module = assert_valid("3 :: Fraction(Integer)");
    assert_js_backend_accepts(&module);
}

#[test]
fn a_has_query_validates_despite_no_shared_evaluator_for_axiom_has() {
    let module = assert_valid("Polynomial(Integer) has Ring");
    assert_js_backend_accepts(&module);
}

#[test]
fn a_deeply_nested_type_constructor_validates() {
    let module = assert_valid("a : List(Matrix(Polynomial(Integer)))");
    assert_js_backend_accepts(&module);
}

#[test]
fn a_float_literal_module_validates_and_declares_floats() {
    let module = assert_valid("1.5");
    assert!(module.manifest.iter().any(|f| f == Feature::Floats));
    assert_js_backend_accepts(&module);
}

#[test]
fn a_complex_multi_construct_program_validates_end_to_end() {
    let module = assert_valid(
        "(a : PositiveInteger; a := 5; f(x: Integer): Integer == if x > 0 then x else -x; f(a))",
    );
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert!(!module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_accepts(&module);
}
