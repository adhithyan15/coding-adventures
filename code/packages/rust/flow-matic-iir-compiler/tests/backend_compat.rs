//! FLOW-MATIC → IIR-to-* backend acceptance tests.
//!
//! Proves the IIR the FLOW-MATIC frontend emits is accepted by every LANG AOT
//! backend validator (wasm / jvm / clr / beam). Without these we could regress
//! the emitted IIR shape (e.g. emit an op no backend knows) unnoticed. This
//! slice is integer-only (no f64), so BEAM accepts it too.
//!
//! Pattern mirrors `dartmouth-basic-iir-compiler/tests/backend_compat.rs`.

use flow_matic_iir_compiler::compile_source;

fn assert_accepted_by_every_backend(m: &interpreter_ir::IIRModule, label: &str) {
    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(m)),
        ("jvm", iir_to_jvm_class_file::validate::validate_for_jvm(m)),
        ("clr", iir_to_cil_bytecode::validate::validate_iir_for_clr(m)),
        ("beam", iir_to_beam::validate::validate_for_beam(m)),
    ] {
        assert!(
            errs.is_empty(),
            "[{name}] validator rejected FLOW-MATIC {label}; got {} error(s): {errs:?}",
            errs.len()
        );
    }
}

/// Programs that print (`putchar`) exclude BEAM, whose validator whitelists only
/// a tiny predicate op set (`pair?`/`equal?`/`not`) — the same exclusion BASIC's
/// print programs use. The print substrate is proven on wasm/jvm/clr.
fn assert_accepted_by_print_backends(m: &interpreter_ir::IIRModule, label: &str) {
    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(m)),
        ("jvm", iir_to_jvm_class_file::validate::validate_for_jvm(m)),
        ("clr", iir_to_cil_bytecode::validate::validate_iir_for_clr(m)),
    ] {
        assert!(
            errs.is_empty(),
            "[{name}] validator rejected FLOW-MATIC {label}; got {} error(s): {errs:?}",
            errs.len()
        );
    }
}

#[test]
fn compare_branch_program_accepted_by_every_backend() {
    let src = "\
(0) INPUT INVENTORY FILE-A ; OUTPUT PRICED FILE-C .
(1) COMPARE PRODUCT-NO (A) WITH PRODUCT-NO (A) ;
    IF GREATER GO TO OPERATION 4 ; IF EQUAL GO TO OPERATION 3 ;
    OTHERWISE GO TO OPERATION 2 .
(2) STOP .
(3) MOVE UNIT-PRICE (A) TO UNIT-PRICE (C) ; STOP .
(4) STOP . (END)";
    let m = compile_source(src, "inventory").unwrap();
    assert_accepted_by_every_backend(&m, "inventory compare/branch/move");
}

#[test]
fn jump_program_accepted_by_every_backend() {
    let src = "(0) JUMP TO OPERATION 2 .\n(1) STOP .\n(2) STOP .";
    let m = compile_source(src, "jump").unwrap();
    assert_accepted_by_every_backend(&m, "jump");
}

#[test]
fn write_item_program_accepted_by_every_backend() {
    // WRITE-ITEM emits `call __fm_print_int` + `call_builtin putchar` and the two
    // synthesized digit-print helper functions — the same print substrate BASIC
    // uses. Every backend validator must accept it.
    let src = "\
(0) OUTPUT REPORT FILE-C .
(1) MOVE TOTAL (C) TO TOTAL (C) ; WRITE-ITEM FILE-C ; STOP .";
    let m = compile_source(src, "report").unwrap();
    assert_accepted_by_print_backends(&m, "write-item report");
}
