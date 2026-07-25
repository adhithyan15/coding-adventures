//! Structural verification of lowered modules: every module this frontend
//! produces must pass the shared SIR validator, and any module using the
//! SIR22 array/matrix domain or KW1 keyword-argument vocabulary must be
//! correctly *accepted* by the `semantic-ir-to-javascript` backend --
//! mirrors `scilab-to-semantic-ir/tests/test_validator.rs`'s own
//! capability-acceptance verification pattern exactly (a real `Module`,
//! checked through the actual `Backend::check_module` path, not an
//! isolated unit assertion).

use coding_adventures_idl_to_semantic_ir::compile_source;
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
fn a_purely_literal_program_validates_and_the_js_backend_accepts_it() {
    let module = assert_valid("FUNCTION seven\n RETURN, 3 + 4\nEND\nPRINT, seven()\n");
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a purely-literal module, got: {errors:?}"
    );
}

#[test]
fn a_float_literal_program_validates_and_declares_floats() {
    let module = assert_valid("y = 1.5\nPRINT, y\n");
    assert!(module
        .manifest
        .iter()
        .any(|f| f == semantic_ir::Feature::Floats));
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a purely-literal float module, got: {errors:?}"
    );
}

#[test]
fn a_string_literal_program_declares_strings_and_the_js_backend_accepts_it() {
    let module = assert_valid("s = 'hello'\nPRINT, s\n");
    assert!(module
        .manifest
        .iter()
        .any(|f| f == semantic_ir::Feature::Strings));
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a module using a string literal, got: {errors:?}"
    );
}

#[test]
fn control_flow_with_a_variable_accumulator_validates_but_needs_array_features() {
    let module = assert_valid(
        "total = 0\nFOR i = 1, 10 DO BEGIN\n IF i GT 5 THEN total = total + i\nENDFOR\nPRINT, total\n",
    );
    assert!(module
        .manifest
        .iter()
        .any(|f| f == semantic_ir::Feature::MatrixOps));
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept ordinary control flow with a variable accumulator, \
         got: {errors:?}"
    );
}

#[test]
fn a_matrix_product_program_validates_and_the_js_backend_accepts_it() {
    let module = assert_valid("a = [1,2]\nb = [3,4]\nc = a ## b\nPRINT, c\n");
    assert!(module
        .manifest
        .iter()
        .any(|f| f == semantic_ir::Feature::MatrixOps));
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a module using SIR22 MatMul, got: {errors:?}"
    );
}

#[test]
fn an_indexing_program_validates_and_the_js_backend_accepts_it() {
    let module = assert_valid("a = [1,2,3]\na[1] = 9\nPRINT, a[1]\n");
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a module using IndexGet/IndexSet, got: {errors:?}"
    );
}

#[test]
fn a_range_program_validates_and_the_js_backend_accepts_it() {
    let module = assert_valid("a = [0,1,2,3,4,5]\ny = a[1:3]\nPRINT, y\n");
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a module using Range/IndexArg::Range, got: {errors:?}"
    );
}

#[test]
fn a_transpose_program_validates_and_the_js_backend_accepts_it() {
    let module = assert_valid("a = [1,2,3]\nb = TRANSPOSE(a)\nPRINT, b\n");
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a module using Transpose, got: {errors:?}"
    );
}

#[test]
fn an_indgen_program_validates_and_the_js_backend_accepts_it() {
    let module = assert_valid("a = INDGEN(5)\nPRINT, a\n");
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a module using INDGEN's Range lowering, got: {errors:?}"
    );
}

#[test]
fn a_pro_function_call_program_validates_and_the_js_backend_accepts_it() {
    let module = assert_valid("FUNCTION square, x\n RETURN, x * x\nEND\nPRINT, square(5)\n");
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a module with a DirectCall to a user FUNCTION, got: \
         {errors:?}"
    );
}

#[test]
fn a_keyword_argument_program_validates_and_the_js_backend_accepts_it() {
    // KW1/KW4: the JS backend's own ACCEPTED_FEATURES list includes
    // Feature::KeywordParams today (see lower.rs's own module doc comment,
    // "Keyword arguments", for the verification that this crate's own
    // module doc previously flagged as needing a direct check rather than
    // trusting semantic_ir::manifest::Feature's own stale doc comment).
    let module = assert_valid(
        "FUNCTION plot_it, x, COLOR=hue\n RETURN, x + hue\nEND\nPRINT, plot_it(1, COLOR=10)\n",
    );
    assert!(module
        .manifest
        .iter()
        .any(|f| f == semantic_ir::Feature::KeywordParams));
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a module using KW1 keyword parameters/arguments, \
         got: {errors:?}"
    );
}

#[test]
fn a_boolean_keyword_shorthand_program_validates_and_the_js_backend_accepts_it() {
    let module = assert_valid("PRO plot_it, x, YLOG=ylog\n PRINT, x\nEND\nplot_it, 1, /YLOG\n");
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a module using the /KEYWORD boolean shorthand, got: \
         {errors:?}"
    );
}

#[test]
fn a_two_namespace_program_validates_and_the_js_backend_accepts_it() {
    let module = assert_valid(
        "PRO DOIT, x\n PRINT, x\nEND\nFUNCTION DOIT, x\n RETURN, x\nEND\nDOIT, 5\nPRINT, DOIT(5)\n",
    );
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a module with a same-named PRO and FUNCTION (mangled \
         to distinct SIR function names), got: {errors:?}"
    );
}

#[test]
fn a_repeat_until_program_validates_and_the_js_backend_accepts_it() {
    let module = assert_valid("x = 0\nREPEAT x = x + 1 UNTIL x GE 3\nPRINT, x\n");
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a desugared REPEAT...UNTIL module, got: {errors:?}"
    );
}

#[test]
fn for_loop_reusing_an_already_assigned_variable_as_the_counter_is_rejected() {
    // Mirrors scilab-to-semantic-ir's own identical, hard-won fix: the
    // shared JS backend's ForRange codegen JS-block-scopes the loop
    // variable, so reusing an already-known variable as the counter (and
    // reading its final value afterward without reassigning) would
    // silently read the stale pre-loop value -- rejected outright instead.
    let err = compile_source("y = 1\nFOR y = 1, 3 DO PRINT, y\n", "prog")
        .expect_err("reusing an existing variable as a FOR-loop counter should be rejected");
    assert!(err.message.contains("FOR-loop counter"));
}
