//! Dartmouth BASIC → IIR-to-* backend acceptance tests.
//!
//! Proves BASIC's emitted IIR is accepted by the LANG-FULL AOT backends
//! (wasm / jvm / clr; BEAM only for no-float modules). Without these tests we
//! could regress BASIC's IIR shape (e.g. emit an op no backend knows) without
//! anyone noticing until users try to AOT-compile.
//!
//! Pattern mirrors `twig-ir-compiler/tests/backend_compat.rs` and
//! `oct-iir-compiler/tests/backend_compat.rs`.

use dartmouth_basic_iir_compiler::compile_source;

/// Helper: run every backend's validator on a module.  Panics on any
/// rejection, attributing the failure to the named backend.
fn assert_accepted_by_every_backend(m: &interpreter_ir::IIRModule, label: &str) {
    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(m)),
        ("jvm",  iir_to_jvm_class_file::validate::validate_for_jvm(m)),
        ("clr",  iir_to_cil_bytecode::validate::validate_iir_for_clr(m)),
        ("beam", iir_to_beam::validate::validate_for_beam(m)),
    ] {
        assert!(errs.is_empty(),
            "[{name}] validator rejected BASIC {label}; got {} error(s): {errs:?}",
            errs.len());
    }
}

/// BA7 scalar BASIC emits f64. BEAM still has no f64 lowering, and LANG-FULL's
/// contract excludes BEAM; keep these checks on the f64-capable AOT validators.
fn assert_accepted_by_lang_full_backends(m: &interpreter_ir::IIRModule, label: &str) {
    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(m)),
        ("jvm",  iir_to_jvm_class_file::validate::validate_for_jvm(m)),
        ("clr",  iir_to_cil_bytecode::validate::validate_iir_for_clr(m)),
    ] {
        assert!(errs.is_empty(),
            "[{name}] validator rejected BASIC {label}; got {} error(s): {errs:?}",
            errs.len());
    }
}

// ---------------------------------------------------------------------------
// Group 1: minimal shapes
// ---------------------------------------------------------------------------

/// The smallest possible BASIC program: `10 END`.
#[test]
fn basic_minimal_end_accepted_by_every_backend() {
    let m = compile_source("10 END\n", "compat").expect("BASIC must compile");
    assert_accepted_by_every_backend(&m, "`10 END`");
}

/// `LET` binding — exercises `const` + `mov` (BA7 typed f64 scalar).
#[test]
fn basic_let_binding_accepted_by_every_backend() {
    let src = "10 LET A = 42\n\
               20 END\n";
    let m = compile_source(src, "compat").expect("BASIC must compile");
    assert_accepted_by_lang_full_backends(&m, "`LET A = 42`");
}

// ---------------------------------------------------------------------------
// Group 2: arithmetic
// ---------------------------------------------------------------------------

/// Binary `+` — typed `add`.
#[test]
fn basic_typed_add_accepted_by_every_backend() {
    let src = "10 LET A = 30\n\
               20 LET B = 12\n\
               30 LET C = A + B\n\
               40 END\n";
    let m = compile_source(src, "compat").expect("BASIC must compile");
    assert_accepted_by_lang_full_backends(&m, "`C = A + B`");
}

/// Binary `*` — typed `mul`.
#[test]
fn basic_typed_mul_accepted_by_every_backend() {
    let src = "10 LET A = 6\n\
               20 LET B = 7\n\
               30 LET C = A * B\n\
               40 END\n";
    let m = compile_source(src, "compat").expect("BASIC must compile");
    assert_accepted_by_lang_full_backends(&m, "`C = A * B`");
}

// ---------------------------------------------------------------------------
// Group 3: control flow
// ---------------------------------------------------------------------------

/// `IF … THEN <line>` — branches via `cmp_*` + `jmp_if_*` + line-label.
#[test]
fn basic_if_then_goto_accepted_by_every_backend() {
    let src = "10 LET A = 7\n\
               20 IF A > 5 THEN 100\n\
               30 END\n\
               100 END\n";
    let m = compile_source(src, "compat").expect("BASIC must compile");
    assert_accepted_by_lang_full_backends(&m, "`IF A > 5 THEN 100`");
}

/// `FOR … NEXT` loop — exercises backward `jmp` + counter mutation.
#[test]
fn basic_for_next_loop_accepted_by_every_backend() {
    let src = "10 FOR I = 1 TO 3\n\
               20 NEXT I\n\
               30 END\n";
    let m = compile_source(src, "compat").expect("BASIC must compile");
    assert_accepted_by_lang_full_backends(&m, "`FOR I = 1 TO 3 / NEXT I`");
}

/// `GOTO` — unconditional jump to a labeled line.
#[test]
fn basic_goto_accepted_by_every_backend() {
    let src = "10 LET A = 1\n\
               20 GOTO 100\n\
               30 LET A = 999\n\
               100 END\n";
    let m = compile_source(src, "compat").expect("BASIC must compile");
    assert_accepted_by_lang_full_backends(&m, "`GOTO 100`");
}

// ---------------------------------------------------------------------------
// Group 4: function-status invariant
// ---------------------------------------------------------------------------

/// Every BASIC function in the module must be `FullyTyped` — without
/// that, JITCore's threshold-zero compile path doesn't fire and the
/// IIR-to-* backends emit untyped fallbacks (or reject).
#[test]
fn basic_main_is_fully_typed() {
    let src = "10 LET A = 1\n\
               20 LET B = A + 1\n\
               30 END\n";
    let m = compile_source(src, "compat").expect("BASIC must compile");
    let main = m.functions.iter().find(|f| f.name == "main")
        .expect("module must have main");
    assert_eq!(
        main.type_status,
        interpreter_ir::function::FunctionTypeStatus::FullyTyped,
        "BASIC main should be FullyTyped; got: {:?}",
        main.type_status,
    );
}
