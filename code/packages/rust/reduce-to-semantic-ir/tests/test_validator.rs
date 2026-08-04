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
//! `derive-to-semantic-ir`'s own *current* `tests/test_validator.rs` and
//! `tests/e2e_node.rs` bodies (which themselves note the same discipline
//! against `wolfram-to-semantic-ir`/`macsyma-to-semantic-ir`) rather than
//! trusting any crate's module doc comment on faith. This file is written
//! directly against that current, already-shipped behaviour.
//!
//! `CompoundExpression`/`Cons`/`First`/`Second`/`Third`/`Rest`/`Part`/
//! `Append`/`Reverse` have no evaluation *handler* in the shared
//! `symbolic-vm` (see `lower.rs`'s module doc comment's "REAL gap"
//! section) — but that is a runtime-evaluation concern, not a SIR
//! validation or backend-*acceptance* one: they are ordinary `SymApply`
//! nodes with an unusual head *name*, and the JS backend's SIR23 codegen
//! handles any `SymApply`/`SymSymbol` shape uniformly regardless of head
//! spelling (it does not special-case individual head names at all,
//! confirmed by reading `semantic-ir-to-javascript`'s SIR23 `match` arms
//! directly). So every construct below — including the ones with no
//! shared-VM handler — is still expected to validate and be accepted
//! here.

use reduce_to_semantic_ir::compile_source;
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
fn a_procedure_definition_and_call_validates() {
    let module = assert_valid("h(x) := x*x;\nh(3);\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    // Confirms the disclosed scope boundary: Reduce's grammar has no
    // pattern-matching syntax in this subset, so this feature is never
    // observed, even for a "procedure definition" construct.
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
fn list_accessor_calls_and_lists_validate_as_symbolic_data() {
    let module = assert_valid("first(l);\nrest(l);\nappend(l1, l2);\n{1, 2, 3};\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert!(!module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_accepts(&module);
}

#[test]
fn if_expression_validates() {
    let module = assert_valid("if x > 0 then 1 else -1;\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert_js_backend_accepts(&module);
}

#[test]
fn group_statement_validates_even_with_no_compound_expression_handler() {
    // CompoundExpression has no evaluation handler in the shared
    // symbolic-vm (lower.rs's module doc comment's "REAL gap" section) --
    // still expected to validate/be accepted, since this is a structural
    // concern (an ordinary SymApply node), not an evaluation one.
    let module = assert_valid("<< x := 1; x + 1 >>;\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert_js_backend_accepts(&module);
}

#[test]
fn a_bare_cons_that_does_not_fold_still_validates() {
    // a . b (b not structurally a literal list) lowers to a bare `Cons`
    // head with no shared-VM handler either -- same "structural
    // acceptance regardless of runtime evaluability" story.
    let module = assert_valid("a . b;\n");
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
        "h(x) := x*x;\ng(x) := h(x) + 1;\nresult := g(3);\n{result, h(2)};\nif result > 0 then result else 0;\n",
    );
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert!(!module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_accepts(&module);
}
