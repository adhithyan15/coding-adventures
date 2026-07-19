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
fn if_symbolic_relational_operators() {
    // Every symbol lowers byte-identically to the oracle (run_if asserts that).
    // N=5 across the truth table, including the range-boundary >= / <= cases and
    // an explicit NOT composing with a symbol's baseline negation (NOT >= ≡ <).
    let cases = [
        ("IF N > 3 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "Y\n"),
        ("IF N > 5 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "N\n"),
        ("IF N < 9 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "Y\n"),
        ("IF N = 5 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "Y\n"),
        ("IF N >= 5 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "Y\n"),
        ("IF N >= 6 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "N\n"),
        ("IF N <= 5 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "Y\n"),
        ("IF N <= 4 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "N\n"),
        ("IF N <> 3 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "Y\n"),
        ("IF N <> 5 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "N\n"),
        ("IF N NOT >= 6 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "Y\n"),
    ];
    for (body, want) in cases {
        assert_eq!(run_if("5", &[body, "STOP RUN."]), want, "{body}");
    }
}

#[test]
fn if_compound_and_or_and_precedence() {
    // Compound AND/OR, precedence, and parentheses — each byte-identical to the
    // oracle (run_if asserts that). AND lowers to bitwise `and`, OR to `or`.
    let cases = [
        ("IF N > 3 AND N < 9 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "Y\n"),
        ("IF N > 3 AND N > 9 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "N\n"),
        ("IF N < 3 OR N > 4 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "Y\n"),
        ("IF N < 3 OR N > 9 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "N\n"),
        // AND binds tighter than OR: N=1 OR (N>3 AND N<9) = false OR true = true.
        ("IF N = 1 OR N > 3 AND N < 9 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "Y\n"),
        // Parentheses override: (N=1 OR N>3) AND N<4 = true AND false = false.
        ("IF (N = 1 OR N > 3) AND N < 4 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "N\n"),
    ];
    for (body, want) in cases {
        assert_eq!(run_if("5", &[body, "STOP RUN."]), want, "{body}");
    }
}

#[test]
fn if_not_negates_a_condition() {
    // NOT binds tighter than AND/OR and inverts the boolean (via `xor` with 1).
    // Each case is byte-identical to the oracle (run_if asserts that).
    let cases = [
        ("IF NOT N > 3 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "N\n"), // NOT (5>3) = false
        ("IF NOT (N < 3 OR N > 9) DISPLAY \"Y\" ELSE DISPLAY \"N\".", "Y\n"), // de Morgan
        ("IF NOT N = 5 OR N > 0 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "Y\n"), // (NOT 5=5) OR 5>0
        ("IF N > 0 AND NOT N > 9 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "Y\n"), // NOT tighter than AND
        ("IF NOT (N IS NOT GREATER 3) DISPLAY \"Y\" ELSE DISPLAY \"N\".", "Y\n"), // double negation
    ];
    for (body, want) in cases {
        assert_eq!(run_if("5", &[body, "STOP RUN."]), want, "{body}");
    }
}

#[test]
fn evaluate_case_statement() {
    // EVALUATE lowers to a cmp_eq + jmp_if_false cascade — byte-identical to the
    // oracle (run_if asserts that). The subject N is matched against each WHEN.
    let eval = &[
        "EVALUATE N",
        "WHEN 1 DISPLAY \"ONE\"",
        "WHEN 5 DISPLAY \"FIVE\"",
        "WHEN OTHER DISPLAY \"OTHER\"",
        "END-EVALUATE.",
        "STOP RUN.",
    ];
    assert_eq!(run_if("5", eval), "FIVE\n"); // matches the second WHEN
    assert_eq!(run_if("1", eval), "ONE\n"); // matches the first WHEN
    assert_eq!(run_if("7", eval), "OTHER\n"); // no value matches → WHEN OTHER
    // No match and no OTHER → nothing runs; control continues after END-EVALUATE.
    assert_eq!(
        run_if("7", &["EVALUATE N", "WHEN 1 DISPLAY \"ONE\"", "END-EVALUATE.", "DISPLAY \"AFTER\".", "STOP RUN."]),
        "AFTER\n"
    );
    // A STOP RUN inside the matched WHEN ends the program.
    assert_eq!(
        run_if(
            "5",
            &["EVALUATE N", "WHEN 5 DISPLAY \"IN\" STOP RUN", "END-EVALUATE.", "DISPLAY \"AFTER\".", "STOP RUN."],
        ),
        "IN\n"
    );
}

#[test]
fn evaluate_on_a_scaled_subject() {
    // A scaled subject/value compare by value: RATE = 1.5 matches WHEN 1.5.
    let out = assert_matches_oracle(&wrap(
        &["01  RATE  PIC 9V9 VALUE 1.5."],
        &[
            "EVALUATE RATE",
            "WHEN 1.0 DISPLAY \"ONE\"",
            "WHEN 1.5 DISPLAY \"HALF\"",
            "WHEN OTHER DISPLAY \"OTHER\"",
            "END-EVALUATE.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "HALF\n");
}

#[test]
fn if_compound_condition_mixing_condition_names() {
    // A level-88 condition-name combined with a relation via AND/OR.
    let out = assert_matches_oracle(&wrap(
        &["01  STATUS-CODE  PIC 9 VALUE 1.", "88  IS-OK  VALUE 1."],
        &["IF IS-OK AND STATUS-CODE < 5 DISPLAY \"Y\" ELSE DISPLAY \"N\".", "STOP RUN."],
    ));
    assert_eq!(out, "Y\n");
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

#[test]
fn level_88_condition_name_true_and_false() {
    // STATUS-CODE = 1 makes IS-OK true (IS-OK VALUE 1); the ELSE never runs.
    let ok = assert_matches_oracle(&wrap(
        &["01  STATUS-CODE  PIC 9 VALUE 1.", "88  IS-OK  VALUE 1.", "88  IS-DONE  VALUE 9."],
        &["IF IS-OK DISPLAY \"OK\" ELSE DISPLAY \"NO\".", "STOP RUN."],
    ));
    assert_eq!(ok, "OK\n");
    // The same variable does not satisfy IS-DONE (VALUE 9) → ELSE branch.
    let done = assert_matches_oracle(&wrap(
        &["01  STATUS-CODE  PIC 9 VALUE 1.", "88  IS-OK  VALUE 1.", "88  IS-DONE  VALUE 9."],
        &["IF IS-DONE DISPLAY \"DONE\" ELSE DISPLAY \"NO\".", "STOP RUN."],
    ));
    assert_eq!(done, "NO\n");
}

#[test]
fn level_88_condition_name_after_a_move() {
    // The condition tracks its variable's live value: after MOVE 9, IS-DONE holds.
    let out = assert_matches_oracle(&wrap(
        &["01  STATUS-CODE  PIC 9 VALUE 1.", "88  IS-DONE  VALUE 9."],
        &["MOVE 9 TO STATUS-CODE.", "IF IS-DONE DISPLAY \"DONE\".", "STOP RUN."],
    ));
    assert_eq!(out, "DONE\n");
}

#[test]
fn level_88_condition_name_on_a_scaled_variable() {
    // A condition-name over a scaled item compares by value: RATE = 1.5 satisfies
    // IS-HALF (VALUE 1.5) once both are taken to the slot's scaled representation.
    let out = assert_matches_oracle(&wrap(
        &["01  RATE  PIC 9V9 VALUE 1.5.", "88  IS-HALF  VALUE 1.5."],
        &["IF IS-HALF DISPLAY \"HALF\" ELSE DISPLAY \"NO\".", "STOP RUN."],
    ));
    assert_eq!(out, "HALF\n");
}

#[test]
fn level_88_condition_name_drives_perform_until() {
    // PERFORM STEP UNTIL IS-DONE: STEP adds 2 (1→3→5→7→9); stops at 9. The
    // condition-name works anywhere a condition does, PERFORM UNTIL included.
    let out = assert_matches_oracle(&wrap(
        &["01  STATUS-CODE  PIC 9 VALUE 1.", "88  IS-DONE  VALUE 9."],
        &[
            "PERFORM STEP UNTIL IS-DONE.",
            "DISPLAY STATUS-CODE.",
            "STOP RUN.",
            "STEP.",
            "ADD 2 TO STATUS-CODE.",
        ],
    ));
    assert_eq!(out, "9\n");
}

#[test]
fn set_condition_name_to_true_assigns_first_value() {
    // SET IS-DONE TO TRUE stores 9 (IS-DONE VALUE 9) into STATUS-CODE; it then
    // displays as 9 and satisfies IS-DONE. Byte-identical to the oracle.
    let out = assert_matches_oracle(&wrap(
        &["01  STATUS-CODE  PIC 9 VALUE 1.", "88  IS-DONE  VALUE 9."],
        &[
            "SET IS-DONE TO TRUE.",
            "DISPLAY STATUS-CODE.",
            "IF IS-DONE DISPLAY \"D\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "9\nD\n");
}

#[test]
fn set_condition_name_to_true_uses_a_ranges_low_bound() {
    // 88 COND VALUE 3 THRU 6 — SET COND TO TRUE assigns the low bound 3 → "03".
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC 99 VALUE 0.", "88  COND  VALUE 3 THRU 6."],
        &["SET COND TO TRUE.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "03\n");
}

#[test]
fn level_88_multiple_values_or() {
    // 88 COND VALUE 1 3 5 — an OR of equalities (`or` of `cmp_eq`s). Hits on 3,
    // misses on 4. Byte-identical to the oracle's any-value match.
    let hit = assert_matches_oracle(&wrap(
        &["01  N  PIC 99 VALUE 3.", "88  COND  VALUE 1 3 5."],
        &["IF COND DISPLAY \"Y\" ELSE DISPLAY \"N\".", "STOP RUN."],
    ));
    assert_eq!(hit, "Y\n");
    let miss = assert_matches_oracle(&wrap(
        &["01  N  PIC 99 VALUE 4.", "88  COND  VALUE 1 3 5."],
        &["IF COND DISPLAY \"Y\" ELSE DISPLAY \"N\".", "STOP RUN."],
    ));
    assert_eq!(miss, "N\n");
}

#[test]
fn level_88_thru_range_inclusive_boundaries() {
    // 88 COND VALUE 3 THRU 6 — an inclusive range (`and` of `cmp_ge`/`cmp_le`).
    // Check both boundaries (3, 6) hold and just outside (2, 7) does not.
    for (v, want) in [("2", "N\n"), ("3", "Y\n"), ("6", "Y\n"), ("7", "N\n")] {
        let out = assert_matches_oracle(&wrap(
            &[&format!("01  N  PIC 99 VALUE {v}."), "88  COND  VALUE 3 THRU 6."],
            &["IF COND DISPLAY \"Y\" ELSE DISPLAY \"N\".", "STOP RUN."],
        ));
        assert_eq!(out, want, "N={v}");
    }
}

#[test]
fn level_88_mixed_singles_and_range() {
    // 88 COND VALUE 1 5 THRU 7 9 — {1} ∪ {5,6,7} ∪ {9}, folding cmp_eq and range
    // tests with `or`. Byte-identical to the oracle across the whole domain.
    for (v, want) in [("1", "Y\n"), ("6", "Y\n"), ("9", "Y\n"), ("4", "N\n"), ("8", "N\n")] {
        let out = assert_matches_oracle(&wrap(
            &[&format!("01  N  PIC 99 VALUE {v}."), "88  COND  VALUE 1 5 THRU 7 9."],
            &["IF COND DISPLAY \"Y\" ELSE DISPLAY \"N\".", "STOP RUN."],
        ));
        assert_eq!(out, want, "N={v}");
    }
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
fn compute_exponentiation_squares_and_cubes() {
    // A ** 2 = 25 → 0025.00; A ** 3 = 125 → 0125.00.
    let sq = assert_matches_oracle(&wrap(
        &["01  A  PIC 9(3) VALUE 5.", "01  R  PIC 9(4)V99."],
        &["COMPUTE R = A ** 2.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(sq, "002500\n");
    let cube = assert_matches_oracle(&wrap(
        &["01  A  PIC 9(3) VALUE 5.", "01  R  PIC 9(4)V99."],
        &["COMPUTE R = A ** 3.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(cube, "012500\n");
}

#[test]
fn compute_exponent_zero_is_one_and_one_is_identity() {
    // x ** 0 = 1 regardless of the base (the base is never even read); x ** 1 = x.
    let zero = assert_matches_oracle(&wrap(
        &["01  A  PIC 9(3) VALUE 7.", "01  R  PIC 9(4)."],
        &["COMPUTE R = A ** 0.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(zero, "0001\n");
    let one = assert_matches_oracle(&wrap(
        &["01  A  PIC 9(3) VALUE 7.", "01  R  PIC 9(4)."],
        &["COMPUTE R = A ** 1.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(one, "0007\n");
}

#[test]
fn compute_exponentiation_of_a_scaled_base_accumulates_scale() {
    // 1.5 ** 2 = 2.25. The base scale (1 fractional digit) is doubled by the two
    // multiplies, so the exact product carries two fractional digits → 9V99 = 225.
    let out = assert_matches_oracle(&wrap(
        &["01  X  PIC 9V9 VALUE 1.5.", "01  R  PIC 9V99."],
        &["COMPUTE R = X ** 2.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "225\n");
}

#[test]
fn compute_exponentiation_over_a_sub_expression() {
    // (A + B) ** 2 = (10 + 2) ** 2 = 144 → 0144.00.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC 9(3) VALUE 10.",
            "01  B  PIC 9(3) VALUE 2.",
            "01  R  PIC 9(4)V99.",
        ],
        &["COMPUTE R = (A + B) ** 2.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "014400\n");
}

#[test]
fn compute_exponentiation_participates_in_precedence() {
    // ** binds tighter than * : B * A ** 2 = 3 * (4 ** 2) = 3 * 16 = 48 → 0048.00.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC 9(3) VALUE 4.",
            "01  B  PIC 9(3) VALUE 3.",
            "01  R  PIC 9(4)V99.",
        ],
        &["COMPUTE R = B * A ** 2.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "004800\n");
}

#[test]
fn compute_exponentiation_truncates_into_a_narrower_receiver() {
    // 2 ** 10 = 1024; stored into 9(3) keeps the low-order three digits → 024.
    // A one-digit base keeps the compile-time bound (`int_digits · exponent`)
    // inside the 18-digit model while the value still overflows the receiver.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC 9 VALUE 2.", "01  R  PIC 9(3)."],
        &["COMPUTE R = A ** 10.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "024\n");
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
fn compute_nested_division_in_a_sum() {
    // A / B + C = 10/3 + 2 = 3.333… + 2 = 5.333…, truncated into 9(4)V99 → 5.33.
    // The oracle carries the division at scale 12, so the leading digits are exact.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC 9(3) VALUE 10.",
            "01  B  PIC 9(3) VALUE 3.",
            "01  C  PIC 9(3) VALUE 2.",
            "01  R  PIC 9(4)V99.",
        ],
        &["COMPUTE R = A / B + C.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "000533\n");
}

#[test]
fn compute_nested_division_on_the_right_of_an_operator() {
    // C + A / B — the division is the right operand of the add. Same 5.333… value,
    // proving precedence puts `/` under `+` regardless of source order.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC 9(3) VALUE 10.",
            "01  B  PIC 9(3) VALUE 3.",
            "01  C  PIC 9(3) VALUE 2.",
            "01  R  PIC 9(4)V99.",
        ],
        &["COMPUTE R = C + A / B.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "000533\n");
}

#[test]
fn compute_nested_division_then_multiply_rounds() {
    // A / B * C = 10/3 * 2 = 6.666…; ROUNDED into 9(4)V99 → 6.67.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC 9(3) VALUE 10.",
            "01  B  PIC 9(3) VALUE 3.",
            "01  C  PIC 9(3) VALUE 2.",
            "01  R  PIC 9(4)V99.",
        ],
        &["COMPUTE R ROUNDED = A / B * C.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "000667\n");
}

#[test]
fn compute_nested_division_of_a_scaled_dividend() {
    // X / Y + Z with a fractional dividend: 1.5 / 2 + 1 = 0.75 + 1 = 1.75 → 9V99.
    let out = assert_matches_oracle(&wrap(
        &["01  X  PIC 9V9 VALUE 1.5.", "01  Y  PIC 9 VALUE 2.", "01  Z  PIC 9 VALUE 1.", "01  R  PIC 9V99."],
        &["COMPUTE R = X / Y + Z.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "175\n");
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

// -------------------------------------------------------------------------
// Signed numerics (PIC S9…) — sign kept through arithmetic, overpunch on
// DISPLAY — vs the oracle.
// -------------------------------------------------------------------------

#[test]
fn signed_value_displays_with_trailing_overpunch() {
    // -123 in S9(3): magnitude "123", units 3 → 'L' (negative) → "12L".
    let neg = assert_matches_oracle(&wrap(
        &["01  N  PIC S9(3) VALUE -123."],
        &["DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(neg, "12L\n");
    // +123 → units 3 → 'C' (positive) → "12C".
    let pos = assert_matches_oracle(&wrap(
        &["01  N  PIC S9(3) VALUE 123."],
        &["DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(pos, "12C\n");
    // Zero is unsigned: units 0 → '{' → "00{".
    let zero = assert_matches_oracle(&wrap(
        &["01  N  PIC S9(3) VALUE 0."],
        &["DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(zero, "00{\n");
}

#[test]
fn signed_field_keeps_sign_through_arithmetic() {
    // 3 - 5 = -2 into a signed receiver → magnitude 2, negative → "0K".
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC S9(2) VALUE 3."],
        &["SUBTRACT 5 FROM N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "0K\n");
}

#[test]
fn signed_value_used_in_arithmetic_carries_its_sign() {
    // N = -10; ADD 4 → -6 → "0O" (units 6 → 'O', negative).
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC S9(2) VALUE -10."],
        &["ADD 4 TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "0O\n");
}

#[test]
fn moving_signed_into_unsigned_drops_the_sign() {
    // A signed source moved into an unsigned receiver keeps only magnitude.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC S9(3) VALUE -45.", "01  U  PIC 9(3) VALUE 0."],
        &["MOVE S TO U.", "DISPLAY U.", "STOP RUN."],
    ));
    assert_eq!(out, "045\n");
}

#[test]
fn compute_into_signed_receiver_shows_negative_overpunch() {
    // COMPUTE N = 2 - 9 = -7 into S9(2) → "0P" (units 7 → 'P', negative).
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC S9(2) VALUE 0."],
        &["COMPUTE N = 2 - 9.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "0P\n");
}

#[test]
fn signed_scaled_field_overpunches_the_last_fractional_digit() {
    // -1.5 in S9V9: magnitude "15", units 5 → 'N' (negative) → "1N".
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC S9V9 VALUE -1.5."],
        &["DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "1N\n");
}

// -------------------------------------------------------------------------
// Alphanumeric item-to-item MOVE + comparison — vs the oracle.
// -------------------------------------------------------------------------

#[test]
fn char_item_move_truncates_on_the_right() {
    // W is "ABCD" (X(4)); moved into V PIC X(2) keeps the leftmost two → "AB".
    let out = assert_matches_oracle(&wrap(
        &["01  W  PIC X(4) VALUE \"ABCD\".", "01  V  PIC X(2)."],
        &["MOVE W TO V.", "DISPLAY V.", "STOP RUN."],
    ));
    assert_eq!(out, "AB\n");
}

#[test]
fn char_item_move_space_pads_on_the_right() {
    // W is "AB" (X(2)); moved into V PIC X(5) left-justifies and space-pads → "AB   ".
    let out = assert_matches_oracle(&wrap(
        &["01  W  PIC X(2) VALUE \"AB\".", "01  V  PIC X(5)."],
        &["MOVE W TO V.", "DISPLAY V \"|\".", "STOP RUN."],
    ));
    assert_eq!(out, "AB   |\n");
}

#[test]
fn char_item_move_same_size_copies() {
    // Equal sizes: a straight copy of the stored image.
    let out = assert_matches_oracle(&wrap(
        &["01  W  PIC X(3) VALUE \"XyZ\".", "01  V  PIC X(3)."],
        &["MOVE W TO V.", "DISPLAY V.", "STOP RUN."],
    ));
    assert_eq!(out, "XyZ\n");
}

#[test]
fn alphanumeric_equal_and_not_equal() {
    // Item vs literal, equal (both "AB  " once padded to width 4 vs "AB" padded).
    let eq = assert_matches_oracle(&wrap(
        &["01  W  PIC X(4) VALUE \"AB\"."],
        &["IF W EQUAL \"AB\" DISPLAY \"YES\" ELSE DISPLAY \"NO\".", "STOP RUN."],
    ));
    assert_eq!(eq, "YES\n");
    // Different content → not equal.
    let ne = assert_matches_oracle(&wrap(
        &["01  W  PIC X(4) VALUE \"ABCD\"."],
        &["IF W NOT EQUAL \"AB\" DISPLAY \"DIFF\".", "STOP RUN."],
    ));
    assert_eq!(ne, "DIFF\n");
}

#[test]
fn alphanumeric_ordering_is_lexicographic() {
    // "APPLE" > "APPLY"? No — 'E'(0x45) < 'Y'(0x59), so APPLE < APPLY.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(5) VALUE \"APPLE\".", "01  B  PIC X(5) VALUE \"APPLY\"."],
        &["IF A LESS B DISPLAY \"A<B\" ELSE DISPLAY \"A>=B\".", "STOP RUN."],
    ));
    assert_eq!(out, "A<B\n");
}

#[test]
fn alphanumeric_shorter_literal_space_pads_for_compare() {
    // "AB" (item, X(4) → "AB  ") vs "AB  " literal: padded equal length, equal.
    // And "AB" item vs "ABX" literal: item pads to "AB " (width 3) < "ABX".
    let out = assert_matches_oracle(&wrap(
        &["01  W  PIC X(2) VALUE \"AB\"."],
        &["IF W LESS \"ABX\" DISPLAY \"LT\".", "STOP RUN."],
    ));
    assert_eq!(out, "LT\n");
}

#[test]
fn alphanumeric_compare_against_spaces_figurative() {
    // A blank field equals SPACES; a non-blank one does not.
    let blank = assert_matches_oracle(&wrap(
        &["01  W  PIC X(3)."],
        &["IF W EQUAL SPACES DISPLAY \"BLANK\".", "STOP RUN."],
    ));
    assert_eq!(blank, "BLANK\n");
    let filled = assert_matches_oracle(&wrap(
        &["01  W  PIC X(3) VALUE \"HI\"."],
        &["IF W NOT EQUAL SPACES DISPLAY \"FILLED\".", "STOP RUN."],
    ));
    assert_eq!(filled, "FILLED\n");
}

#[test]
fn char_move_then_compare_round_trips() {
    // Prove the moved value round-trips through the str slot and compares equal.
    let out = assert_matches_oracle(&wrap(
        &["01  W  PIC X(4) VALUE \"WXYZ\".", "01  V  PIC X(4)."],
        &["MOVE W TO V.", "IF V EQUAL W DISPLAY \"SAME\".", "STOP RUN."],
    ));
    assert_eq!(out, "SAME\n");
}
