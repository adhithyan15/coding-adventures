//! Structural verification of lowered modules: every module this frontend
//! produces must pass the shared SIR validator (confirming the manifest
//! declares exactly the SIR23 features the module actually uses — the same
//! ground truth `semantic-ir/src/validator.rs`'s `check_expr` enforces node
//! kind for node kind), and every module must be **accepted** by the JS
//! backend.
//!
//! # This file's assertions used to say the opposite — and that was a bug
//!
//! Until this crate's `semantic-ir-to-javascript` dependency gained real
//! SIR23 codegen (`SymApply`/`SymPatternBlank`/`SymPatternNamed`/`SymRule`/
//! `SymReplaceAll` all lower to calls into the inlined `__Sir.Symbolic.*`
//! runtime, ported from `sir-runtime-symbolic` — HML01 Stream B rollout
//! item 7), every module this frontend produced was necessarily *rejected*
//! by that backend, since this frontend retargets `macsyma-compiler`'s own
//! "everything is symbolic data" design (see `lower.rs`'s module doc
//! comment) — even the most trivial literal arithmetic (`1 + 2`) emits at
//! least one `SymApply` node. This file's tests asserted exactly that
//! rejection, mirroring `wolfram-to-semantic-ir`'s own (now similarly
//! fixed) `tests/test_validator.rs`, which originally established the
//! pattern for SIR23.
//!
//! Once `sir-runtime-symbolic` and its JS-backend codegen landed, that
//! assertion became **false** for every test below — the JS backend now
//! successfully compiles every one of these modules — but nobody had
//! updated this file to match, so all seven tests here started failing on
//! `origin/main` (confirmed directly: `cargo test -p macsyma-to-semantic-ir`
//! failed 7/7 in `test_validator.rs` before this fix, each with
//! `expected the JS backend to reject a module using SIR23 features`).
//! This file corrects that stale assumption: `assert_js_backend_accepts`
//! now asserts `check_module` returns *no* errors, and `tests/e2e_node.rs`
//! (new, this same fix) goes one step further and actually runs the
//! compiled JS through `node`, proving the generated code is not just
//! statically accepted but genuinely executable.

use semantic_ir::backend::Backend;
use semantic_ir::Feature;
use semantic_ir_to_javascript::JavaScriptBackend;
use macsyma_to_semantic_ir::compile_source;

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
    let module = assert_valid("1 + 2$\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert!(!module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_accepts(&module);
}

#[test]
fn a_bare_symbol_alone_validates_and_declares_symbolic_expr() {
    let module = assert_valid("x$\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert_js_backend_accepts(&module);
}

#[test]
fn a_function_definition_and_call_validates() {
    let module = assert_valid("f(x) := x^2$\nf(3)$\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    // Confirms the disclosed scope boundary: no pattern-matching syntax
    // exists in this grammar at all, so this feature is never observed,
    // even for a "function definition" construct.
    assert!(!module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_accepts(&module);
}

#[test]
fn assignment_is_pure_data_and_still_validates() {
    let module = assert_valid("x : 5$\ny : x + 1$\n");
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert_js_backend_accepts(&module);
}

#[test]
fn control_flow_constructs_validate_as_symbolic_data() {
    let module = assert_valid(
        "if x > 0 then 1 else -1$\nwhile x do x : x - 1$\nfor i in [1, 2, 3] do i$\n\
         block([total : 0], total)$\nreturn(5)$\n",
    );
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
    let module = assert_valid("1.5$\nx + 1$\n");
    assert!(module.manifest.iter().any(|f| f == Feature::Floats));
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert_js_backend_accepts(&module);
}

#[test]
fn a_complex_multi_statement_program_validates_end_to_end() {
    let module = assert_valid(
        "f(x) := x^2$\ng(x) := f(x) + 1$\nresult : g(3)$\n[result, f(2)]$\n",
    );
    assert!(module.manifest.iter().any(|f| f == Feature::SymbolicExpr));
    assert!(!module.manifest.iter().any(|f| f == Feature::PatternMatching));
    assert_js_backend_accepts(&module);
}
