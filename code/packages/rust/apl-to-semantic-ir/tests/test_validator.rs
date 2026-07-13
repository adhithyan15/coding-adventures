//! Structural verification of lowered modules, mirroring
//! `matlab-to-semantic-ir`'s own `tests/test_validator.rs` capability-
//! rejection pattern: every module this frontend produces must pass the
//! shared SIR validator, and (since **every** APL program here emits at
//! least one SIR22/SIR22-addendum node -- there is no purely-scalar escape
//! hatch the way MATLAB's literal-only subset has, see `src/lower.rs`'s
//! module doc comment point 1) a real backend that doesn't implement SIR22
//! codegen yet must cleanly *reject* the module rather than silently
//! producing wrong output.
//!
//! `semantic-ir-to-javascript` does not implement codegen for any SIR22 or
//! SIR22-addendum node (see that backend's `emit.rs`, which panics with
//! "not accepted yet" if one is ever reached post-check -- the panic only
//! guards the capability check never drifting, since `Feature::NDArrays`/
//! `MatrixOps` are not in that backend's `ACCEPTED_FEATURES`), so these
//! tests confirm the *gate* works, not that codegen runs.

use apl_to_semantic_ir::compile_source;
use semantic_ir::backend::Backend;
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

#[test]
fn every_apl_program_validates_but_the_js_backend_rejects_it() {
    // Even the simplest possible dyadic program -- `3+4` -- lowers to an
    // `ElementwiseOp`, which is a SIR22 node no backend accepts yet.
    let module = assert_valid("3+4\n");
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        !errors.is_empty(),
        "expected the JS backend to reject an ElementwiseOp-using module"
    );
}

#[test]
fn a_pure_scalar_literal_with_no_operator_at_all_still_validates() {
    // The one shape of APL program that emits *no* SIR22 node at all: a
    // bare literal with nothing applied to it. `print` is a plain builtin
    // every backend already implements, so this is the sole case that could
    // in principle run through the JS backend today.
    let module = assert_valid("5\n");
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a pure-literal, no-operator module, got: {errors:?}"
    );
}

#[test]
fn reduce_and_outer_product_modules_validate_but_are_rejected_by_js() {
    let module = assert_valid("+/1 2 3\n");
    let backend = JavaScriptBackend;
    assert!(!backend.check_module(&module).is_empty());

    let module = assert_valid("1∘.×2\n");
    assert!(!backend.check_module(&module).is_empty());
}
