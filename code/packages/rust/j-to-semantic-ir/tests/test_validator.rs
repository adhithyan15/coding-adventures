//! Structural verification of lowered modules, mirroring
//! `apl-to-semantic-ir`'s own `tests/test_validator.rs` capability-
//! acceptance pattern: every module this frontend produces must pass the
//! shared SIR validator, and (since J's 12 shared scalar atoms are, like
//! APL's, *unconditionally* `Expr::ElementwiseOp` -- there is no
//! purely-scalar escape hatch -- see `src/lower.rs`'s module doc comment)
//! a real backend must accept or reject the module according to exactly
//! which SIR22 node it uses.

use j_to_semantic_ir::compile_source;
use semantic_ir::backend::Backend;
use semantic_ir_to_javascript::JavaScriptBackend;

fn assert_valid(src: &str) -> semantic_ir::Module {
    let module = compile_source(src, "prog").unwrap_or_else(|e| panic!("lowering failed: {e}"));
    let report = semantic_ir::validate(&module);
    assert!(report.is_ok(), "SIR validation failed for `{src}`: {:?}", report.issues);
    module
}

#[test]
fn a_simple_dyadic_program_validates_and_the_js_backend_accepts_it() {
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
    let module = assert_valid("5\n");
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a pure-literal, no-operator module, got: {errors:?}"
    );
}

#[test]
fn reduce_modules_now_compile_cleanly() {
    // Regression test for the gap `apl-to-semantic-ir`'s own real lowering
    // exposed (this crate's `+/` compiles to the identical `Expr::Reduce`
    // node): `Reduce` shares `NDArrays`/`MatrixOps`/`ArrayColumnMajor` with
    // the SIR22 base cut, so `check_module()` alone (a plain feature-flag
    // check) could never distinguish "base cut only" from "also uses the
    // addendum" -- before `semantic-ir-to-javascript` gained real codegen
    // for the SIR22 "APL addendum" nodes, a dedicated tree-walk inside
    // `compile()` rejected them explicitly. Now that real codegen exists,
    // `compile()` succeeds -- see `semantic-ir-to-javascript`'s own
    // CHANGELOG and `apl-to-semantic-ir`'s `tests/e2e_node.rs` for the
    // actual node-executed proof.
    let module = assert_valid("+/1 2 3\n");
    let backend = JavaScriptBackend;
    assert!(
        backend.check_module(&module).is_empty(),
        "check_module should accept a Reduce-using module"
    );
    semantic_ir_to_javascript::compile(&module).expect("Reduce-using module now compiles");
}

#[test]
fn hook_and_fork_modules_validate_and_the_js_backend_accepts_them() {
    // Hooks/forks lower to nested ElementwiseOp/BuiltinCall applications --
    // no new SIR node at all (MA06 §5) -- so, unlike Reduce, these are
    // ordinary base-cut modules the JS backend already handles.
    let module = assert_valid("(+*)3\n");
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(errors.is_empty(), "expected the JS backend to accept a hook-using module, got: {errors:?}");
    assert!(
        semantic_ir_to_javascript::compile(&module).is_ok(),
        "expected compile() to succeed for a hook-using module"
    );

    let module = assert_valid("(+*-)3\n");
    assert!(
        semantic_ir_to_javascript::compile(&module).is_ok(),
        "expected compile() to succeed for a fork-using module"
    );
}
