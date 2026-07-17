//! COBOL → IIR-to-* backend acceptance tests.
//!
//! Proves the IIR the COBOL frontend emits is accepted by every LANG AOT backend
//! validator (wasm / jvm / clr / beam). Without these we could regress the
//! emitted IIR shape (e.g. emit an op no backend knows) unnoticed. This slice is
//! integer-and-string I/O (no f64).
//!
//! Pattern mirrors `flow-matic-iir-compiler/tests/backend_compat.rs`.

use cobol_iir_compiler::compile_source;

/// Card code lines into the fixed 80-column format.
fn program(lines: &[&str]) -> String {
    lines.iter().map(|l| format!("000000 {l}")).collect::<Vec<_>>().join("\n")
}

fn assert_accepted_by_every_backend(m: &interpreter_ir::IIRModule, label: &str) {
    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(m)),
        ("jvm", iir_to_jvm_class_file::validate::validate_for_jvm(m)),
        ("clr", iir_to_cil_bytecode::validate::validate_iir_for_clr(m)),
        ("beam", iir_to_beam::validate::validate_for_beam(m)),
    ] {
        assert!(
            errs.is_empty(),
            "[{name}] validator rejected COBOL {label}; got {} error(s): {errs:?}",
            errs.len()
        );
    }
}

/// Programs that print (`str_const`/`print_str`/`putchar`) exclude BEAM, whose
/// validator whitelists only a tiny predicate op set — the same exclusion the
/// BASIC and FLOW-MATIC print programs use. The print substrate is proven on
/// wasm/jvm/clr.
fn assert_accepted_by_print_backends(m: &interpreter_ir::IIRModule, label: &str) {
    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(m)),
        ("jvm", iir_to_jvm_class_file::validate::validate_for_jvm(m)),
        ("clr", iir_to_cil_bytecode::validate::validate_iir_for_clr(m)),
    ] {
        assert!(
            errs.is_empty(),
            "[{name}] validator rejected COBOL {label}; got {} error(s): {errs:?}",
            errs.len()
        );
    }
}

#[test]
fn stop_only_program_accepted_by_every_backend() {
    // No DISPLAY → no print substrate → BEAM accepts it too.
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "stop").unwrap();
    assert_accepted_by_every_backend(&m, "stop-only");
}

#[test]
fn display_program_accepted_by_print_backends() {
    // DISPLAY of a literal and of numeric/character items — the str_const +
    // print_str + putchar substrate every print backend must accept.
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  N  PIC 9(5) VALUE 42.",
        "01  W  PIC X(4) VALUE \"HI\".",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    DISPLAY \"START\".",
        "    MOVE 7 TO N.",
        "    DISPLAY N W \"!\".",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "display").unwrap();
    assert_accepted_by_print_backends(&m, "display literals and items");
}

#[test]
fn arithmetic_program_accepted_by_print_backends() {
    // ADD/MULTIPLY/SUBTRACT on i64 slots + the numeric-DISPLAY digit helper
    // (add/sub/mul/mod/cmp_lt/jmp_if_false/call). Every print backend must
    // accept the emitted IIR (BEAM excluded, as for any printing program).
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  R  PIC 9(3) VALUE 0.",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    ADD 5 3 TO R.",
        "    MULTIPLY 2 BY R.",
        "    SUBTRACT 1 FROM R.",
        "    DISPLAY R.",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "arith").unwrap();
    assert_accepted_by_print_backends(&m, "integer arithmetic");
}

#[test]
fn scaled_decimal_program_accepted_by_print_backends() {
    // Scaled ADD with ROUNDED and an item-to-item MOVE exercise the rescale +
    // sign-aware rounding-bias branches (extra cmp_lt/jmp/label/div/mul). Every
    // print backend must accept the emitted IIR.
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  A  PIC 9(2)V99 VALUE 1.5.",
        "01  B  PIC 9(3)V9.",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    ADD 2.255 TO A ROUNDED.",
        "    MOVE A TO B.",
        "    DISPLAY A B.",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "scaled").unwrap();
    assert_accepted_by_print_backends(&m, "scaled-decimal arithmetic");
}
