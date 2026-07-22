//! Structural verification of lowered modules, mirroring
//! `j-to-semantic-ir`'s/`apl-to-semantic-ir`'s own `tests/test_validator.rs`
//! capability-acceptance pattern: every module this frontend produces must
//! pass the shared SIR validator, and a real backend must accept the module
//! (including the genuinely new function-literal machinery -- `Function`/
//! `DirectCall`/`MakeClosure`/`IndirectCall` -- this crate is the first
//! SIR22-array-domain frontend to ever emit).

use q_to_semantic_ir::compile_source;
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
    assert!(errors.is_empty(), "expected the JS backend to accept it, got: {errors:?}");
    assert!(semantic_ir_to_javascript::compile(&module).is_ok());
}

#[test]
fn a_pure_scalar_literal_with_no_operator_at_all_still_validates() {
    let module = assert_valid("5\n");
    let backend = JavaScriptBackend;
    assert!(backend.check_module(&module).is_empty());
}

#[test]
fn reduce_and_scan_modules_validate_and_compile() {
    let module = assert_valid("+/1 2 3\n");
    let backend = JavaScriptBackend;
    assert!(backend.check_module(&module).is_empty());
    assert!(semantic_ir_to_javascript::compile(&module).is_ok());

    let module = assert_valid("+\\1 2 3\n");
    assert!(semantic_ir_to_javascript::compile(&module).is_ok());
}

#[test]
fn til_using_index_generator_validates_and_compiles() {
    let module = assert_valid("!5\n");
    assert!(semantic_ir_to_javascript::compile(&module).is_ok());
}

#[test]
fn dyadic_comma_reusing_catenate_validates_and_compiles() {
    let module = assert_valid("1,2 3\n");
    assert!(semantic_ir_to_javascript::compile(&module).is_ok());
}

#[test]
fn a_new_q_specific_builtin_validates_and_compiles() {
    // The SIR validator has no fixed whitelist of BuiltinCall names (it
    // accepts any string), so a genuinely new name like `q_first` passes
    // validation and compiles regardless of whether the shared JS
    // runtime's dispatch table happens to recognise it -- that is an
    // execution-time (not validation-time) concern, exercised instead by
    // `tests/e2e_node.rs`.
    let module = assert_valid("*1 2 3\n");
    let backend = JavaScriptBackend;
    assert!(backend.check_module(&module).is_empty());
    assert!(semantic_ir_to_javascript::compile(&module).is_ok());
}

#[test]
fn function_literal_modules_validate_and_the_js_backend_accepts_them() {
    // The one genuinely new lowering surface (MA11 §2/§3 bullet 1): a real
    // multi-function module (main + a synthesized Function), exercising
    // DirectCall.
    let module = assert_valid("f:{x+y}\n2 f 3\n");
    assert_eq!(module.functions.len(), 2);
    let backend = JavaScriptBackend;
    let errors = backend.check_module(&module);
    assert!(errors.is_empty(), "expected the JS backend to accept a function-literal module, got: {errors:?}");
    assert!(semantic_ir_to_javascript::compile(&module).is_ok());
}

#[test]
fn higher_order_function_value_modules_validate_and_compile() {
    // Exercises MakeClosure + IndirectCall together (the genuinely dynamic
    // call-site case, see src/lower.rs's module doc comment).
    let module = assert_valid("apply:{[g] g 5}\ninc:{x+1}\napply inc\n");
    assert_eq!(module.functions.len(), 3, "apply, inc, main");
    let backend = JavaScriptBackend;
    assert!(backend.check_module(&module).is_empty());
    assert!(semantic_ir_to_javascript::compile(&module).is_ok());
}

#[test]
fn a_global_read_from_inside_a_function_body_validates_and_compiles() {
    let module = assert_valid("n:10\nf:{x+n}\nf 5\n");
    assert_eq!(module.globals.len(), 1);
    assert_eq!(module.globals[0].name, "n");
    let backend = JavaScriptBackend;
    assert!(backend.check_module(&module).is_empty());
    assert!(semantic_ir_to_javascript::compile(&module).is_ok());
}

#[test]
fn list_literal_modules_validate_and_compile() {
    let module = assert_valid("(1;2;3)\n");
    assert!(semantic_ir_to_javascript::compile(&module).is_ok());
}
