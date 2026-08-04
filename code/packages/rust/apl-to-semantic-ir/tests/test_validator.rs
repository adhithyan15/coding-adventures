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
//! program like `3+4` needs) AND for the SIR22 "APL addendum" (`Reduce`/
//! `Scan`/`OuterProduct`/`Shape`/`Reshape`/`IndexGenerator`/`IndexOf`/
//! `Ravel`/`Catenate`) — this crate's own lowering is exactly what
//! motivated adding that codegen (the addendum nodes share `NDArrays`/
//! `MatrixOps`/`ArrayColumnMajor` with the base cut, so a plain
//! feature-flag `Backend::check_module` check alone could never
//! distinguish "base cut only" from "also uses the addendum" — real
//! codegen for both closes that gap by making the distinction moot).
//! `reduce_and_outer_product_modules_now_compile_cleanly` below is the
//! regression test for the OLD rejection behavior: it now asserts
//! `compile()` SUCCEEDS. Real, node-executed behavioral proof (not just
//! "doesn't error") lives in `tests/e2e_node.rs`.

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
fn reduce_and_outer_product_modules_now_compile_cleanly() {
    // Regression test for the gap this crate's own real lowering exposed:
    // `Reduce`/`OuterProduct` share `NDArrays`/`MatrixOps`/
    // `ArrayColumnMajor` with the SIR22 base cut, so `check_module()`
    // alone (a plain feature-flag check) was never able to distinguish
    // "base cut only" from "also uses the addendum" -- before the JS
    // backend gained real codegen for these nine nodes, that meant a
    // dedicated tree-walk had to reject them explicitly inside
    // `compile()` (a belt-and-suspenders check `check_module()` alone
    // could not provide). Now that real codegen exists, both
    // `check_module()` AND `compile()` accept these modules -- the
    // distinction the old test had to document (coarse feature check vs.
    // dedicated tree-walk) no longer needs to exist at all.
    let module = assert_valid("+/1 2 3\n");
    let backend = JavaScriptBackend;
    assert!(
        backend.check_module(&module).is_empty(),
        "check_module should accept a Reduce-using module"
    );
    semantic_ir_to_javascript::compile(&module).expect("Reduce-using module now compiles");

    let module = assert_valid("1∘.×2\n");
    semantic_ir_to_javascript::compile(&module).expect("OuterProduct-using module now compiles");
}
