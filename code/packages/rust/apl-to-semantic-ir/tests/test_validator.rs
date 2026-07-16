//! Structural verification of lowered modules, mirroring
//! `matlab-to-semantic-ir`'s own `tests/test_validator.rs` capability-
//! acceptance pattern: every module this frontend produces must pass the
//! shared SIR validator, and (since **every** APL program here emits at
//! least one SIR22/SIR22-addendum node -- there is no purely-scalar escape
//! hatch the way MATLAB's literal-only subset has, see `src/lower.rs`'s
//! module doc comment point 1) a real backend must accept or reject the
//! module according to exactly which SIR22 node it uses.
//!
//! `semantic-ir-to-javascript` now implements real codegen for the SIR22
//! *base cut* (`ElementwiseOp` among them, which is all a simple dyadic
//! program like `3+4` needs), so those modules are accepted. The SIR22
//! "APL addendum" nodes (`Reduce`/`Scan`/`OuterProduct`/...) remain
//! deferred — this crate's own lowering is exactly what motivated adding
//! `semantic-ir-to-javascript`'s dedicated tree-walk rejection for them
//! (see that crate's `find_unimplemented_sir22_addendum_node`), since they
//! share `NDArrays`/`MatrixOps`/`ArrayColumnMajor` with the now-accepted
//! base cut and so are NOT caught by the plain `Backend::check_module`
//! feature check alone — the `reduce_and_outer_product_modules...` test
//! below calls `compile()`, not `check_module()` directly, for exactly
//! that reason.

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
fn a_simple_dyadic_program_validates_and_the_js_backend_accepts_it() {
    // Even the simplest possible dyadic program -- `3+4` -- lowers to an
    // `ElementwiseOp`, which is now real SIR22-base-cut codegen.
    let module = assert_valid("3+4\n");
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept an ElementwiseOp-using module, got: {errors:?}"
    );
}

#[test]
fn a_pure_scalar_literal_with_no_operator_at_all_still_validates() {
    // The one shape of APL program that emits *no* SIR22 node at all: a
    // bare literal with nothing applied to it. `print` is a plain builtin
    // every backend already implements.
    let module = assert_valid("5\n");
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a pure-literal, no-operator module, got: {errors:?}"
    );
}

#[test]
fn reduce_and_outer_product_modules_validate_but_compile_still_rejects_them() {
    // `Reduce`/`OuterProduct` share `NDArrays`/`MatrixOps`/`ArrayColumnMajor`
    // with the SIR22 base cut the JS backend now accepts, so
    // `check_module()` alone (a plain feature-flag check) no longer
    // catches these -- only the dedicated tree-walk inside `compile()`
    // does. Confirms both that the module still fails cleanly (not a
    // panic) AND that the coarse feature check by itself is genuinely
    // insufficient here (documenting, not just asserting, the gap).
    let module = assert_valid("+/1 2 3\n");
    let backend = JavaScriptBackend;
    assert!(
        backend.check_module(&module).is_empty(),
        "check_module alone no longer rejects Reduce -- it shares features with the accepted base cut"
    );
    let err = semantic_ir_to_javascript::compile(&module)
        .expect_err("compile() should still cleanly reject a Reduce-using module");
    assert_eq!(err.kind, semantic_ir::BackendErrorKind::UnsupportedFeature);

    let module = assert_valid("1∘.×2\n");
    let err = semantic_ir_to_javascript::compile(&module)
        .expect_err("compile() should still cleanly reject an OuterProduct-using module");
    assert_eq!(err.kind, semantic_ir::BackendErrorKind::UnsupportedFeature);
}
