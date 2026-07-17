//! End-to-end conformance: compile a COBOL program to IIR, run it on the generic
//! JIT, capture the bytes it writes via `putchar`, and assert they are **exactly**
//! what the tree-walk `cobol-runtime` interpreter — the oracle — `DISPLAY`s.
//!
//! This is the whole point of the rung: the compiled program running on a real
//! backend must be byte-identical to the reference semantics. Each case pins a
//! specific behaviour (zero-fill, truncation, implied point, figuratives,
//! concatenation, source-text literals) so a regression names itself.

use cobol_iir_compiler::compile_source;
use coding_adventures_cobol_runtime::run_cobol;
use jit_core::core::JITCore;
use jit_core::GenericCirJit;
use std::sync::{Arc, Mutex};
use vm_core::core::VMCore;
use vm_core::value::Value;

/// Card a set of code lines into the fixed 80-column format the lexer expects
/// (6 sequence columns + a space indicator, then code from column 8).
fn program(lines: &[&str]) -> String {
    lines.iter().map(|l| format!("000000 {l}")).collect::<Vec<_>>().join("\n")
}

/// Run the compiled module on the JIT, returning everything it wrote to stdout
/// through the `putchar` builtin.
fn run_on_jit(src: &str) -> String {
    let mut module = compile_source(src, "e2e").expect("compile");
    assert!(module.validate().is_empty(), "validate: {:?}", module.validate());

    let mut vm = VMCore::new();
    let out: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    {
        let out = Arc::clone(&out);
        vm.builtins_mut().register("putchar", move |args| {
            let b = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
            out.lock().unwrap().push(b as u8);
            Ok(Value::Null)
        });
    }
    // `print_str` writes the string's bytes to the same stdout channel.
    {
        let out = Arc::clone(&out);
        vm.builtins_mut().register("print_str", move |args| {
            let s = args.first().and_then(Value::as_str).unwrap_or("");
            out.lock().unwrap().extend_from_slice(s.as_bytes());
            Ok(Value::Null)
        });
    }

    let backend = GenericCirJit::new();
    let mut jit = JITCore::new(&mut vm, Box::new(backend));
    jit.execute_with_jit(&mut vm, &mut module, "main", &[]).expect("run");
    // Bind the cloned bytes first so the MutexGuard temporary drops before `out`.
    let bytes = out.lock().unwrap().clone();
    String::from_utf8(bytes).unwrap()
}

/// The heart of every case: the compiled-and-run output equals the oracle's.
fn assert_matches_oracle(src: &str) -> String {
    let oracle = run_cobol(src).expect("oracle run");
    let jit = run_on_jit(src);
    assert_eq!(jit, oracle, "compiled output must match the cobol-runtime oracle");
    jit
}

/// Wrap DATA and PROCEDURE lines into a minimal well-formed program.
fn wrap(data: &[&str], proc: &[&str]) -> String {
    let mut lines = vec!["IDENTIFICATION DIVISION.", "PROGRAM-ID. P."];
    if !data.is_empty() {
        lines.push("DATA DIVISION.");
        lines.push("WORKING-STORAGE SECTION.");
        lines.extend_from_slice(data);
    }
    lines.push("PROCEDURE DIVISION.");
    lines.push("MAIN.");
    lines.extend_from_slice(proc);
    program(&lines)
}

#[test]
fn hello_world() {
    let out = assert_matches_oracle(&wrap(&[], &["DISPLAY \"HELLO, WORLD\".", "STOP RUN."]));
    assert_eq!(out, "HELLO, WORLD\n");
}

#[test]
fn character_move_space_pads() {
    let out = assert_matches_oracle(&wrap(
        &["01  WORD  PIC X(5)."],
        &["MOVE \"HI\" TO WORD.", "DISPLAY WORD.", "STOP RUN."],
    ));
    assert_eq!(out, "HI   \n");
}

#[test]
fn character_move_truncates_on_the_right() {
    let out = assert_matches_oracle(&wrap(
        &["01  WORD  PIC X(3)."],
        &["MOVE \"HELLO\" TO WORD.", "DISPLAY WORD.", "STOP RUN."],
    ));
    assert_eq!(out, "HEL\n");
}

#[test]
fn numeric_move_zero_fills() {
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC 9(5)."],
        &["MOVE 42 TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "00042\n");
}

#[test]
fn numeric_move_truncates_implied_point() {
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC 9(2)V9."],
        &["MOVE 123.456 TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "234\n");
}

#[test]
fn display_concatenates_with_no_separator() {
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(3) VALUE \"FOO\".", "01  B  PIC 9(2) VALUE 7."],
        &["DISPLAY A B \"!\".", "STOP RUN."],
    ));
    assert_eq!(out, "FOO07!\n");
}

#[test]
fn value_initialisation_and_figuratives() {
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC 9(3) VALUE ZERO.", "01  S  PIC X(4) VALUE SPACES."],
        &["DISPLAY N.", "DISPLAY S \"|\".", "STOP RUN."],
    ));
    assert_eq!(out, "000\n    |\n");
}

#[test]
fn display_numeric_literal_is_source_text() {
    // A numeric literal displays verbatim (42), not zero-filled — the compiler
    // must not confuse a literal with a field image.
    let out = assert_matches_oracle(&wrap(&[], &["DISPLAY 42.", "STOP RUN."]));
    assert_eq!(out, "42\n");
}

#[test]
fn multiple_moves_reassign_the_same_field() {
    // Successive literal MOVEs re-`str_const` the same register; the last wins.
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC 9(3)."],
        &["MOVE 1 TO N.", "DISPLAY N.", "MOVE 25 TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "001\n025\n");
}

#[test]
fn program_with_no_data_division() {
    let out = assert_matches_oracle(&wrap(
        &[],
        &["DISPLAY \"A\".", "DISPLAY \"B\".", "STOP RUN."],
    ));
    assert_eq!(out, "A\nB\n");
}

// -------------------------------------------------------------------------
// Integer arithmetic (PR2) — each asserted byte-identical to the oracle.
// -------------------------------------------------------------------------

#[test]
fn add_to_accumulates_into_the_receiver() {
    // R starts 10; ADD 5 3 TO R → 18 → "018".
    let out = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(3) VALUE 10."],
        &["ADD 5 3 TO R.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "018\n");
}

#[test]
fn add_giving_leaves_the_to_field_unchanged() {
    // ADD 2 3 TO A GIVING R → R = 2+3+A(100) = 105; A untouched.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC 9(3) VALUE 100.", "01  R  PIC 9(3)."],
        &["ADD 2 3 TO A GIVING R.", "DISPLAY R.", "DISPLAY A.", "STOP RUN."],
    ));
    assert_eq!(out, "105\n100\n");
}

#[test]
fn subtract_from_and_unsigned_receiver_keeps_magnitude() {
    // 10 - 3 = 7 → "007".
    let out = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(3) VALUE 10."],
        &["SUBTRACT 3 FROM R.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "007\n");
    // 3 - 5 = -2, but R is unsigned → stores magnitude 2 → "002".
    let out = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(3) VALUE 3."],
        &["SUBTRACT 5 FROM R.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "002\n");
}

#[test]
fn multiply_by_updates_the_by_field() {
    // MOVE-free: R=6; MULTIPLY 7 BY R → 42 → "042".
    let out = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(3) VALUE 6."],
        &["MULTIPLY 7 BY R.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "042\n");
}

#[test]
fn divide_into_truncates_toward_zero() {
    // 10 / 4 = 2.5 → into integer 9(3) truncates to 2 → "002".
    let out = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(3)."],
        &["DIVIDE 4 INTO 10 GIVING R.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "002\n");
    // 20 / 5 = 4, without GIVING updates the dividend R → "004".
    let out = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(3) VALUE 20."],
        &["DIVIDE 5 INTO R.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "004\n");
}

#[test]
fn overflow_without_handler_truncates_high_order() {
    // R is 9(2) (max 99). 50 + 60 = 110 overflows; with no ON SIZE ERROR handler
    // COBOL keeps the low two digits → "10".
    let out = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(2) VALUE 50."],
        &["ADD 60 TO R.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "10\n");
}

#[test]
fn arithmetic_chains_through_the_field() {
    // Successive ops read the field back, proving the i64 slot round-trips:
    // 0 +7 =7, *3 =21, -1 =20 → "20".
    let out = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(2) VALUE 0."],
        &[
            "ADD 7 TO R.",
            "MULTIPLY 3 BY R.",
            "SUBTRACT 1 FROM R.",
            "DISPLAY R.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "20\n");
}
