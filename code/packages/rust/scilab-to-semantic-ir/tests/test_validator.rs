//! Structural verification of lowered modules: every module this frontend
//! produces must pass the shared SIR validator, and any module using the
//! SIR22 array/matrix domain must be correctly *accepted* by the
//! `semantic-ir-to-javascript` backend -- mirroring
//! `matlab-to-semantic-ir/tests/test_validator.rs`'s own capability-
//! acceptance verification pattern exactly (a real `Module`, checked
//! through the actual `Backend::check_module` path, not an isolated unit
//! assertion).

use scilab_to_semantic_ir::compile_source;
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
    let module = assert_valid("function r = seven()\n  r = 3 + 4;\nendfunction\ndisp(seven());\n");
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a purely-literal module, got: {errors:?}"
    );
}

#[test]
fn a_float_literal_program_validates_and_declares_floats() {
    let module = assert_valid("y = 1.5;\ndisp(y);\n");
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
    let module = assert_valid("s = 'hello';\ndisp(s);\n");
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
fn a_logical_and_program_validates_and_declares_short_circuit() {
    let module = assert_valid("x = 5;\ny = 10;\nif x > 3 && y > 5\n  disp(1);\nend\n");
    assert!(module
        .manifest
        .iter()
        .any(|f| f == semantic_ir::Feature::ShortCircuit));
}

#[test]
fn control_flow_with_a_variable_accumulator_validates_but_needs_array_features() {
    let module = assert_valid(
        "total = 0;\nfor i = 1:10\n  if i > 5\n    total = total + i;\n  end\nend\ndisp(total);\n",
    );
    assert!(module
        .manifest
        .iter()
        .any(|f| f == semantic_ir::Feature::MatrixOps));
}

#[test]
fn a_matrix_program_validates_and_the_js_backend_accepts_it() {
    let module = assert_valid("A = [1 2; 3 4];\nB = A * A;\ndisp(B);\n");
    assert!(module
        .manifest
        .iter()
        .any(|f| f == semantic_ir::Feature::MatrixOps));
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a module using SIR22 array/matrix features, got: {errors:?}"
    );
}

#[test]
fn an_indexing_program_validates_and_the_js_backend_accepts_it() {
    let module = assert_valid("A = [1 2 3];\nA(2) = 9;\ndisp(A(2));\n");
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a module using IndexGet/IndexSet, got: {errors:?}"
    );
}

#[test]
fn a_range_and_transpose_program_validates_and_the_js_backend_accepts_it() {
    let module = assert_valid("A = [1 2; 3 4];\nv = 1:5;\nB = A';\ndisp(B);\n");
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a module using Range/Transpose, got: {errors:?}"
    );
}

#[test]
fn a_select_case_program_desugars_to_valid_if_chain_and_validates() {
    // `y` is pre-declared before the `select` so each branch re-*assigns*
    // it rather than introducing it -- see `tests/e2e_node.rs`'s identical
    // comment on why a branch-local `LetStarBinding` does not survive the
    // SIR validator's own lexical block scoping.
    let module = assert_valid(
        "x = 2;\ny = 0;\nselect x\n  case 1\n    y = 10;\n  case 2\n    y = 20;\n  else\n    y = 0;\nend\ndisp(y);\n",
    );
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a desugared select/case module, got: {errors:?}"
    );
}

#[test]
fn a_percent_constant_program_validates_and_the_js_backend_accepts_it() {
    let module = assert_valid("y = %pi * 2;\ndisp(y);\n");
    assert!(module
        .manifest
        .iter()
        .any(|f| f == semantic_ir::Feature::Floats));
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a module using a %-constant, got: {errors:?}"
    );
}

#[test]
fn for_loop_reusing_an_already_assigned_variable_as_the_counter_is_rejected() {
    // Round 3 review had this idiom lowering "successfully" (validating
    // and JS-backend-accepting). Round 5 review found that was actually
    // UNSOUND: the shared `semantic-ir-to-javascript` backend's
    // `ForRange` codegen JS-block-scopes the loop variable, so reading a
    // reused counter after the loop WITHOUT first reassigning it silently
    // returns the stale pre-loop value, not the loop's true final value
    // (confirmed via `node`: prints the pre-loop value, not the
    // post-loop one). Round 3's own test masked this by always
    // reassigning before reading. Since this frontend has no control over
    // the JS backend's codegen choice, this is now a clean, disclosed
    // rejection instead of a "supported but sometimes silently wrong"
    // feature.
    let err = compile_source("y = 1;\nfor y = 1:3\n  disp(y);\nend\n", "prog")
        .expect_err("reusing an existing variable as a for-loop counter should be rejected");
    assert!(err.message.contains("for-loop counter"));
}
