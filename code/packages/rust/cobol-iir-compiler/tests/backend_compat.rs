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

#[test]
fn if_else_program_accepted_by_print_backends() {
    // IF/ELSE lowers to cmp_* + jmp_if_false + jmp + label. Every print backend
    // must accept the branch structure and the comparison op.
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  N  PIC 9(3) VALUE 5.",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    IF N GREATER 3 DISPLAY \"BIG\" ELSE DISPLAY \"SMALL\".",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "if").unwrap();
    assert_accepted_by_print_backends(&m, "if/else branch");
}

#[test]
fn symbolic_relop_program_accepted_by_print_backends() {
    // A symbolic relop (`>=`) lowers to the same `cmp_*` a word relation does
    // (`>=` ≡ `cmp_ge`), so it rides the existing branch structure — no new
    // opcode. Every print backend must accept it.
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  N  PIC 9(3) VALUE 5.",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    IF N >= 5 DISPLAY \"GE\" ELSE DISPLAY \"LT\".",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "relop").unwrap();
    assert_accepted_by_print_backends(&m, "symbolic relop");
}

#[test]
fn compound_condition_program_accepted_by_print_backends() {
    // A compound `AND`/`OR` condition folds the leaf `cmp_*` booleans with the
    // bitwise `and`/`or` ops. Every print backend must accept the fold.
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  N  PIC 9(3) VALUE 5.",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    IF (N > 1 OR N > 9) AND N < 8 DISPLAY \"Y\" ELSE DISPLAY \"N\".",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "compound").unwrap();
    assert_accepted_by_print_backends(&m, "compound AND/OR condition");
}

#[test]
fn not_condition_program_accepted_by_print_backends() {
    // `NOT (…)` inverts the group's boolean with the `xor` op. This is the first
    // COBOL program to emit `xor` — every print backend must accept it.
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  N  PIC 9(3) VALUE 5.",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    IF NOT (N < 3 OR N > 9) DISPLAY \"Y\" ELSE DISPLAY \"N\".",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "not").unwrap();
    assert_accepted_by_print_backends(&m, "NOT condition (xor)");
}

#[test]
fn evaluate_program_accepted_by_print_backends() {
    // EVALUATE lowers to a cmp_eq + jmp_if_false + jmp + label cascade — the same
    // ops IF uses, no new opcode. Every print backend must accept the cascade.
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  N  PIC 9(3) VALUE 5.",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    EVALUATE N",
        "    WHEN 1 DISPLAY \"ONE\"",
        "    WHEN 5 DISPLAY \"FIVE\"",
        "    WHEN OTHER DISPLAY \"OTHER\"",
        "    END-EVALUATE.",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "eval").unwrap();
    assert_accepted_by_print_backends(&m, "EVALUATE cascade");
}

#[test]
fn level_88_condition_name_program_accepted_by_print_backends() {
    // A level-88 condition-name lowers to a plain `const` + `cmp_eq` feeding the
    // same branch structure as a relational IF — no new opcode. Every print
    // backend must accept it.
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  STATUS-CODE  PIC 9 VALUE 1.",
        "88  IS-OK  VALUE 1.",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    IF IS-OK DISPLAY \"OK\" ELSE DISPLAY \"NO\".",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "c88").unwrap();
    assert_accepted_by_print_backends(&m, "level-88 condition-name");
}

#[test]
fn level_88_multi_value_range_program_accepted_by_print_backends() {
    // A multi-value + THRU-range condition-name lowers to an OR-fold of `cmp_eq`
    // and `and(cmp_ge, cmp_le)`. This is the first COBOL program to emit the `and`
    // / `or` bitwise ops — every print backend must accept them.
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  N  PIC 99 VALUE 6.",
        "88  COND  VALUE 1 5 THRU 7 9.",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    IF COND DISPLAY \"Y\" ELSE DISPLAY \"N\".",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "c88r").unwrap();
    assert_accepted_by_print_backends(&m, "level-88 multi-value/range");
}

#[test]
fn set_condition_name_program_accepted_by_print_backends() {
    // SET cond-name TO TRUE lowers to a plain `const` store into the variable's
    // slot — no new opcode. Every print backend must accept it.
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  STATUS-CODE  PIC 9 VALUE 1.",
        "88  IS-DONE  VALUE 9.",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    SET IS-DONE TO TRUE.",
        "    DISPLAY STATUS-CODE.",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "set").unwrap();
    assert_accepted_by_print_backends(&m, "SET cond-name TO TRUE");
}

#[test]
fn scaled_multiply_divide_program_accepted_by_print_backends() {
    // Scaled MULTIPLY and DIVIDE (with ROUNDED) exercise the dividend up-scale
    // (mul by 10^e), the div, and the store rescale/rounding branches. Every
    // print backend must accept the emitted IIR.
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  R  PIC 9(2)V99 VALUE 0.",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    MULTIPLY 2.5 BY 2.5 GIVING R.",
        "    DIVIDE 3 INTO 20 GIVING R ROUNDED.",
        "    DISPLAY R.",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "muldiv").unwrap();
    assert_accepted_by_print_backends(&m, "scaled multiply/divide");
}

#[test]
fn perform_and_goto_program_accepted_by_print_backends() {
    // PERFORM (inlined range) + GO TO exercise paragraph labels, jmp, and the
    // counted-loop control. Every print backend must accept the branch structure.
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  I  PIC 9 VALUE 0.",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    PERFORM STEP UNTIL I GREATER 2.",
        "    GO TO DONE.",
        "    DISPLAY \"UNREACHED\".",
        "DONE.",
        "    STOP RUN.",
        "STEP.",
        "    ADD 1 TO I.",
        "    DISPLAY I.",
    ]);
    let m = compile_source(&src, "perform").unwrap();
    assert_accepted_by_print_backends(&m, "perform/go to");
}

#[test]
fn on_size_error_program_accepted_by_print_backends() {
    // ON SIZE ERROR adds the overflow test (cmp_ge) + handler branch on every
    // arithmetic verb; the zero-divisor guard adds another. Backends must accept it.
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  R  PIC 9(2) VALUE 1.",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    ADD 99 TO R ON SIZE ERROR DISPLAY \"OVR\".",
        "    DIVIDE 0 INTO R ON SIZE ERROR DISPLAY \"DIV0\".",
        "    DISPLAY R.",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "size").unwrap();
    assert_accepted_by_print_backends(&m, "on size error");
}

#[test]
fn compute_program_accepted_by_print_backends() {
    // COMPUTE lowers the precedence cascade + a top-level division to plain
    // scaled-i64 ops. Every print backend must accept the emitted IIR.
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  A  PIC 9(3) VALUE 20.",
        "01  B  PIC 9(3) VALUE 3.",
        "01  R  PIC 9(2)V99.",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    COMPUTE R ROUNDED = (A + B) / B.",
        "    DISPLAY R.",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "compute").unwrap();
    assert_accepted_by_print_backends(&m, "compute");
}

#[test]
fn exponentiation_program_accepted_by_print_backends() {
    // `A ** 3` unrolls to a chain of plain `mul` ops (no strings, no new opcode),
    // so it stays inside the same scaled-i64 substrate the other arithmetic uses.
    // Every print backend must accept the emitted mul-chain IIR.
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  A  PIC 9(2) VALUE 4.",
        "01  R  PIC 9(6).",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    COMPUTE R = A ** 3.",
        "    DISPLAY R.",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "pow").unwrap();
    assert_accepted_by_print_backends(&m, "pow");
}

#[test]
fn nested_division_program_accepted_by_print_backends() {
    // A division nested inside a larger COMPUTE expression lowers to a scale-12
    // quotient built from plain `const`/`mul`/`div` ops (no new opcode, no
    // strings). Every print backend must accept the emitted IIR.
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  A  PIC 9(3) VALUE 10.",
        "01  B  PIC 9(3) VALUE 3.",
        "01  C  PIC 9(3) VALUE 2.",
        "01  R  PIC 9(4)V99.",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    COMPUTE R = A / B + C.",
        "    DISPLAY R.",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "ndiv").unwrap();
    assert_accepted_by_print_backends(&m, "ndiv");
}

#[test]
fn signed_program_accepted_by_print_backends() {
    // A signed field adds the __cob_print_signed overpunch helper (a second
    // synthesized function that calls the digit printer) and the sign-keeping
    // store. Every print backend must accept both functions.
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  N  PIC S9(2) VALUE 3.",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    SUBTRACT 5 FROM N.",
        "    DISPLAY N.",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "signed").unwrap();
    assert_accepted_by_print_backends(&m, "signed overpunch");
}

#[test]
fn alphanumeric_move_and_compare_accepted_by_print_backends() {
    // Character item-to-item MOVE (str_slice / str_concat) and a space-padded
    // alphanumeric comparison (str_cmp) — the string-op substrate every print
    // backend must accept (BEAM excluded, as for any printing program).
    let src = program(&[
        "IDENTIFICATION DIVISION.",
        "PROGRAM-ID. P.",
        "DATA DIVISION.",
        "WORKING-STORAGE SECTION.",
        "01  W  PIC X(4) VALUE \"ABCD\".",
        "01  V  PIC X(2).",
        "01  U  PIC X(6).",
        "PROCEDURE DIVISION.",
        "MAIN.",
        "    MOVE W TO V.",
        "    MOVE W TO U.",
        "    IF W GREATER \"AB\" DISPLAY V U \"|\".",
        "    IF W EQUAL SPACES DISPLAY \"BLANK\".",
        "    STOP RUN.",
    ]);
    let m = compile_source(&src, "alnum").unwrap();
    assert_accepted_by_print_backends(&m, "alphanumeric move and compare");
}
