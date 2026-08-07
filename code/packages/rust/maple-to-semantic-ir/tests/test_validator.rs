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
//! Stream B rollout items 6-7), confirmed directly by reading
//! `reduce-to-semantic-ir`'s own *current* `tests/test_validator.rs` and
//! `tests/e2e_node.rs` bodies (which themselves note the same discipline
//! against `wolfram-to-semantic-ir`/`macsyma-to-semantic-ir`/
//! `derive-to-semantic-ir`) rather than trusting any crate's module doc
//! comment on faith. This file is written directly against that current,
//! already-shipped behaviour.
//!
//! The new `Set` head (MA09 §3/§5) has no evaluation *handler* in the
//! shared `symbolic-vm` (see `lower.rs`'s module doc comment's "`Set`"
//! section) — but that is a runtime-evaluation concern, not a SIR
//! validation or backend-*acceptance* one: it is an ordinary `SymApply`
//! node with an unusual head *name*, and the JS backend's SIR23 codegen
//! handles any `SymApply`/`SymSymbol` shape uniformly regardless of head
//! spelling (it does not special-case individual head names at all,
//! confirmed by reading `semantic-ir-to-javascript`'s SIR23 `match` arms
//! directly). So a `Set` literal is still expected to validate and be
//! accepted here.

use maple_to_semantic_ir::compile_source;
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
    let module = assert_valid("1 + 2;\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert!(!module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_accepts(&module);
}

#[test]
fn a_bare_symbol_alone_validates_and_declares_symbolic_expr() {
    let module = assert_valid("x;\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert_js_backend_accepts(&module);
}

#[test]
fn an_arrow_definition_and_call_validates() {
    let module = assert_valid("h := x -> x*x;\nh(3);\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    // Confirms the disclosed scope boundary: Maple's grammar has no
    // pattern-matching syntax in this subset, so this feature is never
    // observed, even for a function-definition construct.
    assert!(!module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_accepts(&module);
}

#[test]
fn assignment_is_pure_data_and_still_validates() {
    let module = assert_valid("x := 5;\ny := x + 1;\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert_js_backend_accepts(&module);
}

#[test]
fn lists_and_sets_both_validate_as_symbolic_data() {
    let module = assert_valid("[1, 2, 3];\n{1, 2, 3};\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert!(!module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_accepts(&module);
}

#[test]
fn a_set_literal_alone_validates_despite_no_shared_vm_handler() {
    // `Set` has no evaluation handler in the shared `symbolic-vm` (see
    // lower.rs's module doc comment) -- still expected to validate/be
    // accepted, since this is a structural concern (an ordinary SymApply
    // node), not an evaluation one.
    let module = assert_valid("{1, 2, 3};\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert_js_backend_accepts(&module);
}

#[test]
fn if_elif_else_expression_validates() {
    let module = assert_valid("if x > 0 then 1 elif x < 0 then -1 else 0 end if;\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert_js_backend_accepts(&module);
}

#[test]
fn boolean_literals_validate() {
    let module = assert_valid("true;\nfalse;\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert_js_backend_accepts(&module);
}

#[test]
fn diff_and_int_calls_validate() {
    let module = assert_valid("diff(x^2, x);\nint(x, x);\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
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
    let module = assert_valid("1.5;\nx + 1;\n");
    assert!(module.manifest.iter().any(|f| f == Feature::Floats));
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert_js_backend_accepts(&module);
}

#[test]
fn a_complex_multi_statement_program_validates_end_to_end() {
    let module = assert_valid(
        "f := x -> x*x;\ng := (x, y) -> f(x) + y;\nresult := g(3, 1);\n[result, f(2)];\n{result, f(2)};\nif result > 0 then result else 0 end if;\n",
    );
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert!(!module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_accepts(&module);
}
