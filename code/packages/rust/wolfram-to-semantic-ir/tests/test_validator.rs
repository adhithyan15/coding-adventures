//! Structural verification of lowered modules: every module this frontend
//! produces must pass the shared SIR validator (confirming the manifest
//! declares exactly the SIR23 features the module actually uses — the same
//! ground truth `semantic-ir/src/validator.rs`'s `check_expr` enforces node
//! kind for node kind), and every module must currently be *rejected* by
//! the JS backend — mirroring the capability-rejection verification
//! pattern used to land SIR22/SIR23 themselves and `matlab-to-semantic-ir`.
//!
//! Unlike `matlab-to-semantic-ir` (which has one purely-literal subset that
//! *is* accepted, since scalar-only MATLAB arithmetic never touches SIR22
//! at all), **every** test below is expected to be rejected: under this
//! frontend's "everything is data" design (see `lower.rs`'s module doc
//! comment), even the most trivial literal arithmetic (`1 + 2`) emits at
//! least one `SymApply` node, and `semantic-ir-to-javascript` does not
//! implement SIR23 codegen at all yet (`sir-runtime-symbolic`, the runtime
//! library it would depend on, is future work — Stream B rollout item 6).
//! There is therefore no e2e `node`-execution test in this crate at all;
//! see `lower.rs`'s module doc comment for the full disclosure.

use semantic_ir::backend::Backend;
use semantic_ir::Feature;
use semantic_ir_to_javascript::JavaScriptBackend;
use wolfram_to_semantic_ir::compile_source;

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

fn assert_js_backend_rejects(module: &semantic_ir::Module) {
    let backend = JavaScriptBackend;
    let errors = backend.check_module(module);
    assert!(
        !errors.is_empty(),
        "expected the JS backend to reject a module using SIR23 features \
         (codegen for them isn't implemented there yet)"
    );
}

#[test]
fn bare_arithmetic_validates_and_declares_symbolic_expr() {
    let module = assert_valid("1 + 2\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert!(!module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_rejects(&module);
}

#[test]
fn a_float_literal_module_validates_and_declares_floats() {
    // Regression test for a confirmed, previously-shipped bug: `lower_token`'s
    // number-literal handling never called `self.observed.add(Feature::
    // Floats)` (it delegated to a free function with no access to
    // `observed`), so a float-literal-only module failed
    // `semantic_ir::validate()` even though `check_expr` requires the
    // feature for every `Expr::FloatLit` node. Found while implementing
    // `macsyma-to-semantic-ir`, fixed here.
    //
    // Paired with `x +` (a genuine `SymApply`) rather than a bare `1.5`
    // alone: a bare float literal is a plain SIR16 node the JS backend
    // already accepts, so it wouldn't exercise this file's own "every
    // module is rejected" capability-gate assertion below.
    let module = assert_valid("x + 1.5\n");
    assert!(module.manifest.iter().any(|f| f == Feature::Floats));
    assert_js_backend_rejects(&module);
}

#[test]
fn a_bare_symbol_alone_validates_and_declares_symbolic_expr() {
    let module = assert_valid("x\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert_js_backend_rejects(&module);
}

#[test]
fn a_pattern_program_validates_and_declares_pattern_matching() {
    let module = assert_valid("f[x_] := x^2\nf[3]\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert!(module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_rejects(&module);
}

#[test]
fn a_replaceall_program_validates_and_declares_pattern_matching() {
    let module = assert_valid("{1, 2, 3} /. a_ -> a + 1\n");
    assert!(module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_rejects(&module);
}

#[test]
fn a_replacerepeated_program_validates() {
    let module = assert_valid("x //. a -> b\n");
    assert!(module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_rejects(&module);
}

#[test]
fn a_condition_alternatives_patterntest_program_validates() {
    let module = assert_valid("x_ /; x > 2\n_?IntegerQ\na | b\n");
    assert!(module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_rejects(&module);
}

#[test]
fn a_pure_function_program_validates() {
    let module = assert_valid("Map[#^2 &, {1, 2, 3}]\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert_js_backend_rejects(&module);
}

#[test]
fn assignment_is_pure_data_and_still_validates() {
    let module = assert_valid("x = 5\ny = x + 1\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert_js_backend_rejects(&module);
}

#[test]
fn a_complex_multi_statement_program_validates_end_to_end() {
    let module = assert_valid(
        "f[x_] := x^2\ng[x_] := f[x] + 1\nresult = g[3] /. Null -> 0\n{result, f[2]}\n",
    );
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert!(module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_rejects(&module);
}
