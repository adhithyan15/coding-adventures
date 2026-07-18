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

// -------------------------------------------------------------------------
// IF / ELSE + relational conditions (PR4) — vs the oracle.
// -------------------------------------------------------------------------

/// Run a program with `01 N PIC 9(3) VALUE <n>` and the given procedure body,
/// asserting the compiled output matches the oracle.
fn run_if(n: &str, body: &[&str]) -> String {
    let mut data = vec![format!("01  N  PIC 9(3) VALUE {n}.")];
    let mut proc: Vec<String> = body.iter().map(|s| s.to_string()).collect();
    let _ = &mut data;
    let _ = &mut proc;
    let data_refs: Vec<&str> = data.iter().map(|s| s.as_str()).collect();
    let proc_refs: Vec<&str> = proc.iter().map(|s| s.as_str()).collect();
    assert_matches_oracle(&wrap(&data_refs, &proc_refs))
}

#[test]
fn if_numeric_true_and_false_branches() {
    // N=5 > 3 → THEN "BIG"; N=1 not > 3 → ELSE "SMALL".
    assert_eq!(
        run_if("5", &["IF N GREATER 3 DISPLAY \"BIG\" ELSE DISPLAY \"SMALL\".", "STOP RUN."]),
        "BIG\n"
    );
    assert_eq!(
        run_if("1", &["IF N GREATER 3 DISPLAY \"BIG\" ELSE DISPLAY \"SMALL\".", "STOP RUN."]),
        "SMALL\n"
    );
}

#[test]
fn if_equal_less_and_negated() {
    assert_eq!(run_if("7", &["IF N EQUAL 7 DISPLAY \"EQ\".", "STOP RUN."]), "EQ\n");
    assert_eq!(run_if("2", &["IF N LESS 5 DISPLAY \"LT\".", "STOP RUN."]), "LT\n");
    // IS NOT GREATER: 3 is not > 5 → true.
    assert_eq!(run_if("3", &["IF N IS NOT GREATER THAN 5 DISPLAY \"OK\".", "STOP RUN."]), "OK\n");
    // A false condition with no ELSE displays nothing.
    assert_eq!(run_if("9", &["IF N LESS 5 DISPLAY \"NO\".", "STOP RUN."]), "");
}

#[test]
fn if_then_branch_runs_multiple_statements() {
    // Both then-statements run when the condition holds.
    assert_eq!(
        run_if("5", &["IF N GREATER 3 MOVE 8 TO N DISPLAY N.", "STOP RUN."]),
        "008\n"
    );
}

#[test]
fn stop_run_inside_a_branch_ends_the_program() {
    // STOP RUN in the THEN branch ends everything; the trailing DISPLAY never runs.
    assert_eq!(
        run_if("5", &["IF N GREATER 3 DISPLAY \"IN\" STOP RUN.", "DISPLAY \"AFTER\".", "STOP RUN."]),
        "IN\n"
    );
}

#[test]
fn if_on_scaled_decimal_compares_by_value() {
    // 2.50 vs 2.5 compare equal despite different receiver scales.
    let out = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(2)V99 VALUE 2.5."],
        &["IF R EQUAL 2.5 DISPLAY \"EQ\" ELSE DISPLAY \"NE\".", "STOP RUN."],
    ));
    assert_eq!(out, "EQ\n");
}

#[test]
fn nested_if_selects_the_inner_branch() {
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC 9(3) VALUE 5."],
        &[
            "IF N GREATER 3 IF N LESS 9 DISPLAY \"MID\" ELSE DISPLAY \"HI\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "MID\n");
}

// -------------------------------------------------------------------------
// Scaled-decimal ADD / SUBTRACT + item→item MOVE (PR3) — vs the oracle.
// -------------------------------------------------------------------------

#[test]
fn decimal_add_aligns_the_implied_point() {
    // R PIC 9(2)V99 starts 1.50; ADD 2.25 TO R → 3.75 → "0375".
    let out = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(2)V99 VALUE 1.5."],
        &["ADD 2.25 TO R.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "0375\n");
}

#[test]
fn add_of_higher_scale_operand_truncates_and_rounds() {
    // Operand 2.255 has more decimals than the V99 receiver.
    // Truncated: 2.25 → "0225"; ROUNDED: 2.26 → "0226".
    let trunc = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(2)V99 VALUE 0."],
        &["ADD 2.255 TO R.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(trunc, "0225\n");
    let round = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(2)V99 VALUE 0."],
        &["ADD 2.255 TO R ROUNDED.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(round, "0226\n");
}

#[test]
fn subtract_decimal_and_unsigned_magnitude() {
    // 1.5 - 2.25 = -0.75 → unsigned V99 stores magnitude 0.75 → "0075".
    let out = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(2)V99 VALUE 1.5."],
        &["SUBTRACT 2.25 FROM R.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "0075\n");
}

#[test]
fn add_between_fields_of_different_scales() {
    // A is 9(3)V9 = 12.3; B is 9(2)V99 = 1.25; ADD A TO B → 13.55 → "1355".
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC 9(3)V9 VALUE 12.3.", "01  B  PIC 9(2)V99 VALUE 1.25."],
        &["ADD A TO B.", "DISPLAY B.", "STOP RUN."],
    ));
    assert_eq!(out, "1355\n");
}

#[test]
fn item_to_item_move_reshapes_to_receiver_picture() {
    // SRC 9(3)=42 → DST 9(5) → "00042".
    let out = assert_matches_oracle(&wrap(
        &["01  SRC  PIC 9(3) VALUE 42.", "01  DST  PIC 9(5)."],
        &["MOVE SRC TO DST.", "DISPLAY DST.", "STOP RUN."],
    ));
    assert_eq!(out, "00042\n");
}

#[test]
fn item_to_item_move_rescales_the_implied_point() {
    // SRC 9(2)V9 = 12.3 → DST 9(3)V99 → 12.30 → "01230".
    let up = assert_matches_oracle(&wrap(
        &["01  SRC  PIC 9(2)V9 VALUE 12.3.", "01  DST  PIC 9(3)V99."],
        &["MOVE SRC TO DST.", "DISPLAY DST.", "STOP RUN."],
    ));
    assert_eq!(up, "01230\n");
    // Fewer decimals truncates (MOVE never rounds): 12.36 → 12.3, and
    // 9(2)V9 is three digits → "123".
    let down = assert_matches_oracle(&wrap(
        &["01  SRC  PIC 9(2)V99 VALUE 12.36.", "01  DST  PIC 9(2)V9."],
        &["MOVE SRC TO DST.", "DISPLAY DST.", "STOP RUN."],
    ));
    assert_eq!(down, "123\n");
}

// -------------------------------------------------------------------------
// ON SIZE ERROR — vs the oracle.
// -------------------------------------------------------------------------

#[test]
fn add_on_size_error_fires_on_overflow() {
    // R is 9(2) (max 99). 50 + 60 = 110 overflows → handler runs, R unchanged (50).
    let out = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(2) VALUE 50."],
        &["ADD 60 TO R ON SIZE ERROR DISPLAY \"OVER\".", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "OVER\n50\n");
}

#[test]
fn add_on_size_error_does_not_fire_when_it_fits() {
    // 50 + 40 = 90 fits 9(2) → no handler, R = 90.
    let out = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(2) VALUE 50."],
        &["ADD 40 TO R ON SIZE ERROR DISPLAY \"OVER\".", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "90\n");
}

#[test]
fn multiply_on_size_error_fires_on_overflow() {
    // 10 * 3 * ... a product that overflows the receiver runs the handler.
    // 40 * 40 = 1600 into 9(2) overflows → handler, R unchanged.
    let out = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(2) VALUE 7."],
        &["MULTIPLY 40 BY 40 GIVING R ON SIZE ERROR DISPLAY \"BIG\".", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "BIG\n07\n");
}

#[test]
fn divide_by_zero_with_on_size_error_runs_the_handler() {
    // A zero divisor is a size-error condition; with a handler it is caught.
    let out = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(3) VALUE 7."],
        &["DIVIDE 0 INTO 10 GIVING R ON SIZE ERROR DISPLAY \"DIVZERO\".", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "DIVZERO\n007\n");
}

// -------------------------------------------------------------------------
// GO TO and PERFORM — vs the oracle.
// -------------------------------------------------------------------------

/// Build a program from full procedure lines (paragraphs included) with no DATA
/// division, and assert the compiled output matches the oracle.
fn run_proc(data: &[&str], proc: &[&str]) -> String {
    assert_matches_oracle(&wrap(data, proc))
}

#[test]
fn go_to_transfers_control_and_skips_fallthrough() {
    let out = run_proc(
        &[],
        &[
            "DISPLAY \"START\".",
            "GO TO SKIP.",
            "MIDDLE.",
            "DISPLAY \"MIDDLE\".",
            "SKIP.",
            "DISPLAY \"END\".",
            "STOP RUN.",
        ],
    );
    assert_eq!(out, "START\nEND\n");
}

#[test]
fn go_to_forms_a_loop_that_terminates() {
    let out = run_proc(
        &["01  I  PIC 9 VALUE 0."],
        &[
            "MOVE 0 TO I.",
            "LOOP.",
            "ADD 1 TO I.",
            "DISPLAY I.",
            "IF I LESS 3 GO TO LOOP.",
            "STOP RUN.",
        ],
    );
    assert_eq!(out, "1\n2\n3\n");
}

#[test]
fn perform_runs_a_paragraph_then_returns() {
    let out = run_proc(
        &[],
        &[
            "PERFORM GREET.",
            "DISPLAY \"BACK\".",
            "STOP RUN.",
            "GREET.",
            "DISPLAY \"HI\".",
        ],
    );
    assert_eq!(out, "HI\nBACK\n");
}

#[test]
fn perform_n_times_and_zero_times() {
    let out = run_proc(
        &["01  COUNT  PIC 9 VALUE 0."],
        &["PERFORM TICK 3 TIMES.", "STOP RUN.", "TICK.", "ADD 1 TO COUNT.", "DISPLAY COUNT."],
    );
    assert_eq!(out, "1\n2\n3\n");
    let z = run_proc(
        &["01  N  PIC 9 VALUE 0."],
        &["PERFORM NOISE N TIMES.", "DISPLAY \"DONE\".", "STOP RUN.", "NOISE.", "DISPLAY \"X\"."],
    );
    assert_eq!(z, "DONE\n");
}

#[test]
fn perform_until_loops_and_tests_before() {
    let out = run_proc(
        &["01  I  PIC 9 VALUE 0."],
        &[
            "PERFORM STEP UNTIL I GREATER 2.",
            "DISPLAY \"DONE\".",
            "STOP RUN.",
            "STEP.",
            "ADD 1 TO I.",
            "DISPLAY I.",
        ],
    );
    assert_eq!(out, "1\n2\n3\nDONE\n");
    // Condition already true → body never runs.
    let z = run_proc(
        &["01  I  PIC 9 VALUE 5."],
        &["PERFORM NOISE UNTIL I GREATER 2.", "DISPLAY \"DONE\".", "STOP RUN.", "NOISE.", "DISPLAY \"X\"."],
    );
    assert_eq!(z, "DONE\n");
}

#[test]
fn perform_thru_runs_a_paragraph_range() {
    let out = run_proc(
        &[],
        &[
            "PERFORM A THRU C.",
            "DISPLAY \"BACK\".",
            "STOP RUN.",
            "A.",
            "DISPLAY \"A\".",
            "B.",
            "DISPLAY \"B\".",
            "C.",
            "DISPLAY \"C\".",
        ],
    );
    assert_eq!(out, "A\nB\nC\nBACK\n");
}

#[test]
fn perform_varying_counts_with_induction_variable() {
    let out = run_proc(
        &["01  I  PIC 9 VALUE 0."],
        &[
            "PERFORM SHOW VARYING I FROM 1 BY 1 UNTIL I GREATER 3.",
            "DISPLAY \"DONE\".",
            "STOP RUN.",
            "SHOW.",
            "DISPLAY I.",
        ],
    );
    assert_eq!(out, "1\n2\n3\nDONE\n");
    // Step by 2 from 0.
    let s = run_proc(
        &["01  I  PIC 9 VALUE 0."],
        &["PERFORM SHOW VARYING I FROM 0 BY 2 UNTIL I GREATER 6.", "STOP RUN.", "SHOW.", "DISPLAY I."],
    );
    assert_eq!(s, "0\n2\n4\n6\n");
}

#[test]
fn stop_run_inside_a_performed_paragraph_ends_the_program() {
    let out = run_proc(
        &[],
        &[
            "PERFORM DONE.",
            "DISPLAY \"AFTER\".",
            "STOP RUN.",
            "DONE.",
            "DISPLAY \"IN\".",
            "STOP RUN.",
        ],
    );
    assert_eq!(out, "IN\n");
}

#[test]
fn go_to_out_of_a_performed_paragraph_transfers_at_top_level() {
    let out = run_proc(
        &[],
        &[
            "PERFORM SUB.",
            "DISPLAY \"AFTER MAIN\".",
            "STOP RUN.",
            "SUB.",
            "GO TO ELSEWHERE.",
            "ELSEWHERE.",
            "DISPLAY \"ELSEWHERE\".",
            "STOP RUN.",
        ],
    );
    assert_eq!(out, "ELSEWHERE\n");
}

// -------------------------------------------------------------------------
// Scaled-decimal MULTIPLY / DIVIDE (PR3b) — vs the oracle.
// -------------------------------------------------------------------------

#[test]
fn multiply_fixed_point_truncates_into_receiver() {
    // 2.5 * 2.5 = 6.25 → into 9(3)V9 truncates to "0062".
    let out = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(3)V9."],
        &["MULTIPLY 2.5 BY 2.5 GIVING R.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "0062\n");
}

#[test]
fn multiply_giving_rounded() {
    // 6.25 into 9(2)V9: truncated 6.2, ROUNDED 6.3 (second place is 5).
    let trunc = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(2)V9 VALUE 0."],
        &["MULTIPLY 2.5 BY 2.5 GIVING R.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(trunc, "062\n");
    let round = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(2)V9 VALUE 0."],
        &["MULTIPLY 2.5 BY 2.5 GIVING R ROUNDED.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(round, "063\n");
}

#[test]
fn multiply_by_field_updates_the_by_field() {
    // MOVE 6 TO R; MULTIPLY 7 BY R → 42.
    let out = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(3) VALUE 6."],
        &["MULTIPLY 7 BY R.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "042\n");
}

#[test]
fn divide_into_giving_truncates_to_receiver_decimals() {
    // 10 / 3 = 3.333… → into 9(3)V99 truncates to 3.33 → "00333".
    let a = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(3)V99."],
        &["DIVIDE 3 INTO 10 GIVING R.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(a, "00333\n");
    // 10 / 4 = 2.5 → into 9(3) (no decimals) truncates to 2 → "002".
    let b = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(3)."],
        &["DIVIDE 4 INTO 10 GIVING R.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(b, "002\n");
}

#[test]
fn divide_giving_rounded() {
    // 20 / 3 = 6.666… → into V99: truncated 6.66, ROUNDED 6.67.
    let trunc = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(2)V99 VALUE 0."],
        &["DIVIDE 3 INTO 20 GIVING R.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(trunc, "0666\n");
    let round = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(2)V99 VALUE 0."],
        &["DIVIDE 3 INTO 20 GIVING R ROUNDED.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(round, "0667\n");
}

#[test]
fn divide_into_without_giving_updates_the_dividend() {
    // MOVE 20 TO R; DIVIDE 5 INTO R → 4.
    let out = assert_matches_oracle(&wrap(
        &["01  R  PIC 9(3) VALUE 20."],
        &["DIVIDE 5 INTO R.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "004\n");
}

#[test]
fn multiply_decimal_field_operands() {
    // A=1.5 (9(2)V9), B=2.0 (9(2)V9); MULTIPLY A BY B GIVING R(9(3)V99) = 3.00.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC 9(2)V9 VALUE 1.5.",
            "01  B  PIC 9(2)V9 VALUE 2.",
            "01  R  PIC 9(3)V99.",
        ],
        &["MULTIPLY A BY B GIVING R.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "00300\n");
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

// -------------------------------------------------------------------------
// COMPUTE — arithmetic expressions with precedence, vs the oracle.
// -------------------------------------------------------------------------

#[test]
fn compute_single_operand_moves_the_value() {
    // COMPUTE R = A with no operator is a rescale-and-store.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC 9(3) VALUE 42.", "01  R  PIC 9(5)."],
        &["COMPUTE R = A.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "00042\n");
}

#[test]
fn compute_respects_operator_precedence() {
    // A + B * C = 10 + (3 * 2) = 16 → 0016.00.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC 9(3) VALUE 10.",
            "01  B  PIC 9(3) VALUE 3.",
            "01  C  PIC 9(3) VALUE 2.",
            "01  R  PIC 9(4)V99.",
        ],
        &["COMPUTE R = A + B * C.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "001600\n");
}

#[test]
fn compute_parentheses_override_precedence() {
    // (A + B) * C = 13 * 2 = 26 → 0026.00.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC 9(3) VALUE 10.",
            "01  B  PIC 9(3) VALUE 3.",
            "01  C  PIC 9(3) VALUE 2.",
            "01  R  PIC 9(4)V99.",
        ],
        &["COMPUTE R = (A + B) * C.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "002600\n");
}

#[test]
fn compute_aligns_scaled_operands() {
    // 1.5 + 2.25 = 3.75 → 9V99 → 375.
    let out = assert_matches_oracle(&wrap(
        &["01  X  PIC 9V9 VALUE 1.5.", "01  Y  PIC 9V99 VALUE 2.25.", "01  R  PIC 9V99."],
        &["COMPUTE R = X + Y.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "375\n");
}

#[test]
fn compute_division_truncates_and_rounds() {
    // 10 / 3 = 3.333… → 9V99 truncates to 3.33.
    let trunc = assert_matches_oracle(&wrap(
        &["01  A  PIC 9(3) VALUE 10.", "01  B  PIC 9(3) VALUE 3.", "01  R  PIC 9V99."],
        &["COMPUTE R = A / B.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(trunc, "333\n");
    // 20 / 3 = 6.666… → ROUNDED to 6.67.
    let round = assert_matches_oracle(&wrap(
        &["01  A  PIC 9(3) VALUE 20.", "01  B  PIC 9(3) VALUE 3.", "01  R  PIC 9V99."],
        &["COMPUTE R ROUNDED = A / B.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(round, "667\n");
}

#[test]
fn compute_unary_minus_and_negative_magnitude() {
    // -B + A = -3 + 10 = 7 → 007.
    let pos = assert_matches_oracle(&wrap(
        &["01  A  PIC 9(3) VALUE 10.", "01  B  PIC 9(3) VALUE 3.", "01  R  PIC 9(3)."],
        &["COMPUTE R = -B + A.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(pos, "007\n");
    // B - A = -7 → an unsigned receiver keeps the magnitude, 7 → 007.
    let neg = assert_matches_oracle(&wrap(
        &["01  A  PIC 9(3) VALUE 10.", "01  B  PIC 9(3) VALUE 3.", "01  R  PIC 9(3)."],
        &["COMPUTE R = B - A.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(neg, "007\n");
}

#[test]
fn compute_on_size_error_fires_on_overflow() {
    // A*A*A = 1000 overflows PIC 9(2); the handler runs and R is unchanged.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC 9(3) VALUE 10.", "01  R  PIC 9(2) VALUE 42."],
        &["COMPUTE R = A * A * A ON SIZE ERROR DISPLAY \"OVER\".", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "OVER\n42\n");
}

#[test]
fn compute_overflow_without_handler_truncates() {
    // No handler: 1000 keeps its low two digits → 00 (COBOL's silent truncation).
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC 9(3) VALUE 10.", "01  R  PIC 9(2) VALUE 42."],
        &["COMPUTE R = A * A * A.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "00\n");
}

#[test]
fn compute_on_size_error_catches_divide_by_zero() {
    // (C - C) = 0 divisor is a size-error condition; the handler catches it.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC 9(3) VALUE 10.", "01  C  PIC 9(3) VALUE 2.", "01  R  PIC 9(3) VALUE 7."],
        &["COMPUTE R = A / (C - C) ON SIZE ERROR DISPLAY \"DIVZERO\".", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "DIVZERO\n007\n");
}
