//! Structural verification of lowered modules: every module this frontend
//! produces must pass the shared SIR validator (confirming the manifest
//! declares exactly the SIR23 features the module actually uses — the
//! same ground truth `semantic-ir/src/validator.rs`'s `check_expr`
//! enforces node kind for node kind), and every module must be
//! **accepted** by the JS backend.
//!
//! `semantic-ir-to-javascript` already has real SIR23 codegen
//! (`SymApply`/`SymSymbol`/… lower to calls into the inlined
//! `__Sir.Symbolic.*` runtime, ported from `sir-runtime-symbolic` — HML01
//! Stream B rollout items 6-7, both shipped before this crate was
//! written), confirmed directly by reading `wolfram-to-semantic-ir`'s and
//! `macsyma-to-semantic-ir`'s own *current* `tests/test_validator.rs` and
//! `tests/e2e_node.rs` bodies rather than trusting either crate's module
//! doc comment (macsyma's own history: its `tests/test_validator.rs`
//! *used* to assert rejection, went stale when the backend gained SIR23
//! codegen, and had to be corrected — see that crate's `CHANGELOG.md`).
//! This file is written directly against the current, already-fixed
//! behaviour, so it carries no equivalent staleness to correct later.

use derive_to_semantic_ir::compile_source;
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
    let module = assert_valid("1 + 2\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert!(!module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_accepts(&module);
}

#[test]
fn a_bare_symbol_alone_validates_and_declares_symbolic_expr() {
    let module = assert_valid("x\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert_js_backend_accepts(&module);
}

#[test]
fn a_function_definition_and_call_validates() {
    let module = assert_valid("F(x) := x^2\nF(3)\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    // Confirms the disclosed scope boundary: Derive's grammar has no
    // pattern-matching syntax at all, so this feature is never observed,
    // even for a "function definition" construct.
    assert!(!module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_accepts(&module);
}

#[test]
fn assignment_is_pure_data_and_still_validates() {
    let module = assert_valid("x := 5\ny := x + 1\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert_js_backend_accepts(&module);
}

#[test]
fn builtin_calls_and_vectors_validate_as_symbolic_data() {
    let module = assert_valid("DIF(SIN(x), x)\nINT(x, x)\n[1, 2, 3]\n[a, b; c, d]\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert!(!module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_accepts(&module);
}

#[test]
fn a_float_literal_module_validates_and_declares_floats() {
    // A BARE float literal alone (no arithmetic wrapping it) emits no
    // SIR23 node at all -- it is just a plain SIR16 `FloatLit`, which the
    // JS backend has always accepted on its own, so it alone wouldn't
    // exercise the `Feature::SymbolicExpr` manifest assertion this test
    // also makes. Pairing it with a genuine symbolic construct keeps this
    // test consistent with the rest of the file while still confirming
    // `Feature::Floats` is declared.
    let module = assert_valid("1.5\nx + 1\n");
    assert!(module.manifest.iter().any(|f| f == Feature::Floats));
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert_js_backend_accepts(&module);
}

#[test]
fn a_complex_multi_statement_program_validates_end_to_end() {
    let module = assert_valid("F(x) := x^2\nG(x) := F(x) + 1\nresult := G(3)\n[result, F(2)]\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert!(!module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_accepts(&module);
}
