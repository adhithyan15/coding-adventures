//! Structural verification of lowered modules: every module this frontend
//! produces must pass the shared SIR validator, and any module using the
//! SIR22 array/matrix domain must be correctly *accepted* by the
//! `semantic-ir-to-javascript` backend — mirroring exactly the
//! capability-acceptance verification pattern used to land SIR22/SIR23
//! themselves (a real `Module`, checked through the actual
//! `Backend::check_module` path, not an isolated unit assertion).
//!
//! `semantic-ir-to-javascript` now implements real codegen for the SIR22
//! *base cut* (`ArrayLit`/`Range`/`MatMul`/`ElementwiseOp`/`Transpose`/
//! `IndexGet`/`IndexSet` all lower to calls into its inlined
//! `__Sir.Array.*` runtime — see that backend's `emit.rs`/`runtime.rs`),
//! so these tests confirm the *gate* now lets these modules through; a
//! genuine array/matrix round-trip through real execution lives in
//! `e2e_node.rs`.
//!
//! Note on scope: the "scalar fast path" described in `lower.rs`'s module
//! doc comment only recognises operands built *purely from literals* as
//! provably scalar — any *variable* (a function parameter, a loop counter,
//! an accumulator, ...) is never provably scalar without real shape
//! inference, so ordinary variable arithmetic (`total + i`, `x * x` for a
//! parameter `x`, ...) always takes the `ElementwiseOp`/`MatMul` path and
//! therefore always needs `MatrixOps`/`ArrayColumnMajor` declared — even
//! though, at runtime, such values are almost always genuine scalars.
//! `control_flow_and_loops_validate` below documents this by asserting the
//! manifest directly, rather than relying on backend rejection to make it
//! concrete (the backend now accepts these features too).

use matlab_to_semantic_ir::compile_source;
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
    // Every operand on every arithmetic op here is a literal (or built
    // transitively from one), so `expr_is_known_scalar` holds throughout
    // and no SIR22 node is ever emitted -- the one shape of MATLAB program
    // this frontend can currently take all the way through the JS backend.
    let module = assert_valid("function r = seven()\n  r = 3 + 4;\nend\ndisp(seven());\n");
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(
        errors.is_empty(),
        "expected the JS backend to accept a purely-literal module, got: {errors:?}"
    );
}

#[test]
fn a_float_literal_program_validates_and_declares_floats() {
    // Regression test for a confirmed, previously-shipped bug:
    // `number_literal_expr` never called `self.observed.add(Feature::
    // Floats)` (it was a free function with no access to `observed`), so a
    // float-literal-only module failed `semantic_ir::validate()` even
    // though `check_expr` requires the feature for every `Expr::FloatLit`
    // node. Found while implementing `macsyma-to-semantic-ir`, fixed here.
    let module = assert_valid("function r = half()\n  r = 1.5;\nend\ndisp(half());\n");
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
fn a_logical_and_program_validates_and_declares_short_circuit() {
    // Regression test for a confirmed, previously-shipped bug: `try_logical`
    // (`src/lower.rs`) built `Expr::LogicalAnd`/`Expr::LogicalOr` nodes for
    // `&&`/`||`/`&`/`|` without ever calling `self.observed.add(Feature::
    // ShortCircuit)`, so any MATLAB program using them failed
    // `semantic_ir::validate()` outright with "manifest does not declare
    // feature short-circuit but module uses it" -- even though the lowering
    // itself was otherwise correct. Found while scoping `tests/oracle.rs`'s
    // corpus (probe: `x > 3 && y > 5`), fixed here alongside it.
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
    // See the module doc comment above: `total + i` involves two
    // variables, never provably scalar, so this ordinary accumulator loop
    // already needs the array-domain features declared.
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
