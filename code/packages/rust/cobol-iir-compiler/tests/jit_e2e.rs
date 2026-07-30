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

/// Compile and run on the JIT, returning `Err` if either compilation or the run
/// itself fails (e.g. an out-of-bounds `str_slice` trap) — the compiled-side
/// mirror of the oracle's `Result`.
fn run_on_jit_result(src: &str) -> Result<String, String> {
    let mut module = compile_source(src, "e2e").map_err(|e| format!("compile: {e:?}"))?;
    if !module.validate().is_empty() {
        return Err(format!("validate: {:?}", module.validate()));
    }
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
    jit.execute_with_jit(&mut vm, &mut module, "main", &[])
        .map_err(|e| format!("run: {e:?}"))?;
    let bytes = out.lock().unwrap().clone();
    Ok(String::from_utf8(bytes).unwrap())
}

/// A computed reference modification that is out of range must **trap on both
/// engines**: the oracle returns a `RuntimeError` and the compiled `str_slice`
/// hits its VM/wasm out-of-bounds bounds check. This pins that agreement.
fn assert_both_trap(src: &str) {
    let oracle = run_cobol(src);
    assert!(oracle.is_err(), "oracle must trap, got {oracle:?}");
    let jit = run_on_jit_result(src);
    assert!(jit.is_err(), "compiled path must trap, got {jit:?}");
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
fn evaluate_multi_value_and_thru_ranges() {
    // A WHEN with several values and a WHEN with a THRU range — each byte-identical
    // to the oracle (run_if asserts that). Multi-value OR-folds cmp_eq; a range is
    // and(cmp_ge, cmp_le).
    let body = &[
        "EVALUATE N",
        "WHEN 1 2 5 DISPLAY \"SET\"",
        "WHEN 7 THRU 9 DISPLAY \"RANGE\"",
        "WHEN OTHER DISPLAY \"OTHER\"",
        "END-EVALUATE.",
        "STOP RUN.",
    ];
    assert_eq!(run_if("2", body), "SET\n"); // listed value
    assert_eq!(run_if("5", body), "SET\n"); // last of the list
    assert_eq!(run_if("7", body), "RANGE\n"); // range low boundary
    assert_eq!(run_if("9", body), "RANGE\n"); // range high boundary
    assert_eq!(run_if("6", body), "OTHER\n"); // between the sets → OTHER
    // A WHEN mixing singles and a range: WHEN 1 5 THRU 7 9 = {1} ∪ {5,6,7} ∪ {9}.
    let mixed = &[
        "EVALUATE N",
        "WHEN 1 5 THRU 7 9 DISPLAY \"Y\"",
        "WHEN OTHER DISPLAY \"N\"",
        "END-EVALUATE.",
        "STOP RUN.",
    ];
    for (n, want) in [("1", "Y\n"), ("6", "Y\n"), ("9", "Y\n"), ("4", "N\n"), ("8", "N\n")] {
        assert_eq!(run_if(n, mixed), want, "N={n}");
    }
}

#[test]
fn evaluate_on_an_alphanumeric_subject() {
    // A character subject matched with str_cmp — each case byte-identical to the
    // oracle. Single values, a THRU range, and WHEN OTHER.
    let eval = |g: &str, body: &[&str]| {
        assert_matches_oracle(&wrap(&[&format!("01  GRADE  PIC X VALUE \"{g}\".")], body))
    };
    let by_value = &[
        "EVALUATE GRADE",
        "WHEN \"A\" DISPLAY \"TOP\"",
        "WHEN \"F\" DISPLAY \"FAIL\"",
        "WHEN OTHER DISPLAY \"MID\"",
        "END-EVALUATE.",
        "STOP RUN.",
    ];
    assert_eq!(eval("A", by_value), "TOP\n");
    assert_eq!(eval("F", by_value), "FAIL\n");
    assert_eq!(eval("C", by_value), "MID\n"); // no value → OTHER
    // A THRU range over characters (byte-lexical): "A" THRU "M".
    let by_range = &[
        "EVALUATE GRADE",
        "WHEN \"A\" THRU \"M\" DISPLAY \"FIRST\"",
        "WHEN OTHER DISPLAY \"REST\"",
        "END-EVALUATE.",
        "STOP RUN.",
    ];
    assert_eq!(eval("B", by_range), "FIRST\n");
    assert_eq!(eval("M", by_range), "FIRST\n"); // high boundary
    assert_eq!(eval("Z", by_range), "REST\n"); // above the range
}

// ---------------------------------------------------------------------------
// EVALUATE with a MIXED numeric↔alphanumeric subject/WHEN. Each subject-vs-WHEN
// comparison now reuses `emit_operand_relation` — the same dispatch an
// `IF subject <relop> value` relation uses — so EVALUATE inherits IF's full
// category handling (unsigned / signed / scaled digit images, figuratives, ZERO
// routing) and its deferral set by construction. Every case is byte-identical to
// the oracle, whose `subject_in_when` already routes through `compare_operands`.
// ---------------------------------------------------------------------------

#[test]
fn evaluate_numeric_subject_alphanumeric_when() {
    // A numeric subject vs an alphanumeric WHEN value: N PIC 9(3)=42 → digit image
    // "042". WHEN "042" matches; WHEN "42" space-pads to "42 " ('0' < '4') → no
    // match — the same byte rule `IF N = "042"` uses.
    let eval = |body: &[&str]| assert_matches_oracle(&wrap(&["01  N  PIC 9(3) VALUE 42."], body));
    assert_eq!(
        eval(&[
            "EVALUATE N",
            "WHEN \"042\" DISPLAY \"HIT\"",
            "WHEN OTHER DISPLAY \"MISS\"",
            "END-EVALUATE.",
            "STOP RUN.",
        ]),
        "HIT\n"
    );
    assert_eq!(
        eval(&[
            "EVALUATE N",
            "WHEN \"42\" DISPLAY \"HIT\"",
            "WHEN OTHER DISPLAY \"MISS\"",
            "END-EVALUATE.",
            "STOP RUN.",
        ]),
        "MISS\n"
    );
}

#[test]
fn evaluate_signed_numeric_subject_alphanumeric_when() {
    // A SIGNED numeric subject compares by its overpunched image: PIC S9(3) = -123
    // → magnitude "123" with the negative sign folded into the units digit → "12L".
    // WHEN "12L" matches; the positive image "12C" does not.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC S9(3) VALUE -123."],
        &[
            "EVALUATE S",
            "WHEN \"12L\" DISPLAY \"NEG\"",
            "WHEN \"12C\" DISPLAY \"POS\"",
            "WHEN OTHER DISPLAY \"MISS\"",
            "END-EVALUATE.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "NEG\n");
}

#[test]
fn evaluate_scaled_numeric_subject_alphanumeric_when() {
    // A scaled subject uses its (int + frac) digit image — no decimal point:
    // PIC 9(2)V9 = 4.2 → "042", so `WHEN "042"` matches.
    let out = assert_matches_oracle(&wrap(
        &["01  F  PIC 9(2)V9 VALUE 4.2."],
        &[
            "EVALUATE F",
            "WHEN \"042\" DISPLAY \"HIT\"",
            "WHEN OTHER DISPLAY \"MISS\"",
            "END-EVALUATE.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "HIT\n");
}

#[test]
fn evaluate_scaled_numeric_subject_numeric_when_stays_numeric() {
    // Regression: a scaled numeric subject vs a scaled numeric WHEN value stays a
    // NUMERIC comparison with scale alignment — PIC 9(2)V9 = 4.2 matches WHEN 4.2.
    let out = assert_matches_oracle(&wrap(
        &["01  F  PIC 9(2)V9 VALUE 4.2."],
        &[
            "EVALUATE F",
            "WHEN 4.1 DISPLAY \"LO\"",
            "WHEN 4.2 DISPLAY \"HIT\"",
            "WHEN OTHER DISPLAY \"MISS\"",
            "END-EVALUATE.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "HIT\n");
}

#[test]
fn evaluate_mixed_thru_range() {
    // A THRU range with alphanumeric bounds against a numeric subject: N=42 →
    // "042"; the byte-lexical range "040" THRU "045" contains it → match. N=50 →
    // "050" is above "045" → no match.
    let eval = |val: &str, body: &[&str]| {
        assert_matches_oracle(&wrap(&[&format!("01  N  PIC 9(3) VALUE {val}.")], body))
    };
    let body = &[
        "EVALUATE N",
        "WHEN \"040\" THRU \"045\" DISPLAY \"IN\"",
        "WHEN OTHER DISPLAY \"OUT\"",
        "END-EVALUATE.",
        "STOP RUN.",
    ];
    assert_eq!(eval("42", body), "IN\n");
    assert_eq!(eval("50", body), "OUT\n");
}

#[test]
fn evaluate_numeric_subject_when_zero_stays_numeric() {
    // WHEN ZERO against a numeric subject stays a NUMERIC comparison (inherited
    // ZERO routing) — N=0 matches numerically, not via the mixed digit-image path.
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC 9(3) VALUE 0."],
        &[
            "EVALUATE N",
            "WHEN ZERO DISPLAY \"Z\"",
            "WHEN OTHER DISPLAY \"NZ\"",
            "END-EVALUATE.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "Z\n");
}

#[test]
fn evaluate_alpha_subject_numeric_literal_when_is_a_later_rung() {
    // An alphanumeric subject vs a numeric-LITERAL WHEN value is a *different*
    // pairing (numeric literal vs alphanumeric) — deferred and rejected IDENTICALLY
    // by both engines: the oracle's `compare_operands` errors on the numeric literal
    // and the compiler's `num_digit_str_operand` rejects it, exactly as the IF
    // relation defers it.
    let src = wrap(
        &["01  G  PIC X(3) VALUE \"042\"."],
        &[
            "EVALUATE G",
            "WHEN 42 DISPLAY \"HIT\"",
            "WHEN OTHER DISPLAY \"MISS\"",
            "END-EVALUATE.",
            "STOP RUN.",
        ],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a numeric-literal WHEN vs alpha subject");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a numeric-literal WHEN vs alpha subject"
    );
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
// Level-88 condition-name on an ALPHANUMERIC (`PIC X`) item (read + SET TO
// TRUE) — vs the oracle. A discrete-string VALUE reads and SETs like a MOVE;
// a THRU range or a non-string VALUE stays a later rung, rejected on both.
// -------------------------------------------------------------------------

#[test]
fn level_88_alphanumeric_read_true_branch() {
    // FLAG holds "N"; IS-N (VALUE "N") is true, so the THEN branch runs.
    let out = assert_matches_oracle(&wrap(
        &["01  FLAG  PIC X VALUE \"N\".", "88  IS-N  VALUE \"N\"."],
        &["IF IS-N DISPLAY \"yes\" ELSE DISPLAY \"no\".", "STOP RUN."],
    ));
    assert_eq!(out, "yes\n");
}

#[test]
fn level_88_alphanumeric_read_false_branch() {
    // FLAG holds "Y"; IS-N (VALUE "N") is false, so the ELSE branch runs.
    let out = assert_matches_oracle(&wrap(
        &["01  FLAG  PIC X VALUE \"Y\".", "88  IS-N  VALUE \"N\"."],
        &["IF IS-N DISPLAY \"yes\" ELSE DISPLAY \"no\".", "STOP RUN."],
    ));
    assert_eq!(out, "no\n");
}

#[test]
fn level_88_alphanumeric_set_to_true_then_display() {
    // SET IS-Y TO TRUE stores "Y" (IS-Y VALUE "Y") into FLAG; DISPLAY shows "Y".
    let out = assert_matches_oracle(&wrap(
        &["01  FLAG  PIC X VALUE \"N\".", "88  IS-Y  VALUE \"Y\"."],
        &["SET IS-Y TO TRUE.", "DISPLAY FLAG.", "STOP RUN."],
    ));
    assert_eq!(out, "Y\n");
}

#[test]
fn level_88_alphanumeric_set_to_true_then_read() {
    // After SET IS-Y TO TRUE, IS-Y holds, so the guarded DISPLAY runs.
    let out = assert_matches_oracle(&wrap(
        &["01  FLAG  PIC X VALUE \"N\".", "88  IS-Y  VALUE \"Y\"."],
        &["SET IS-Y TO TRUE.", "IF IS-Y DISPLAY \"ok\".", "STOP RUN."],
    ));
    assert_eq!(out, "ok\n");
}

#[test]
fn level_88_alphanumeric_multiple_discrete_values_or_fold() {
    // 88 VOWEL VALUE "A" "E" "I" — an OR-fold of alphanumeric equalities. FLAG="E"
    // hits; FLAG="B" misses. Byte-identical to the oracle's any-value match.
    let hit = assert_matches_oracle(&wrap(
        &["01  FLAG  PIC X VALUE \"E\".", "88  VOWEL  VALUE \"A\" \"E\" \"I\"."],
        &["IF VOWEL DISPLAY \"Y\" ELSE DISPLAY \"N\".", "STOP RUN."],
    ));
    assert_eq!(hit, "Y\n");
    let miss = assert_matches_oracle(&wrap(
        &["01  FLAG  PIC X VALUE \"B\".", "88  VOWEL  VALUE \"A\" \"E\" \"I\"."],
        &["IF VOWEL DISPLAY \"Y\" ELSE DISPLAY \"N\".", "STOP RUN."],
    ));
    assert_eq!(miss, "N\n");
    // And SET assigns the FIRST value "A", which then satisfies VOWEL.
    let set = assert_matches_oracle(&wrap(
        &["01  FLAG  PIC X VALUE \"B\".", "88  VOWEL  VALUE \"A\" \"E\" \"I\"."],
        &["SET VOWEL TO TRUE.", "DISPLAY FLAG.", "STOP RUN."],
    ));
    assert_eq!(set, "A\n");
}

#[test]
fn level_88_alphanumeric_multi_char_field_space_padding() {
    // FLAG PIC X(3) VALUE "Y" holds "Y  " (space-padded); 88 IS-Y VALUE "Y" is one
    // character. The comparison space-pads the value to the field width, so the
    // shorter VALUE still matches — byte-identical padding on both engines.
    let out = assert_matches_oracle(&wrap(
        &["01  FLAG  PIC X(3) VALUE \"Y\".", "88  IS-Y  VALUE \"Y\"."],
        &["IF IS-Y DISPLAY \"yes\" ELSE DISPLAY \"no\".", "STOP RUN."],
    ));
    assert_eq!(out, "yes\n");
    // The exact multi-character VALUE "YES" against a PIC X(3) field also matches.
    let exact = assert_matches_oracle(&wrap(
        &["01  FLAG  PIC X(3) VALUE \"YES\".", "88  IS-YES  VALUE \"YES\"."],
        &["IF IS-YES DISPLAY \"yes\" ELSE DISPLAY \"no\".", "STOP RUN."],
    ));
    assert_eq!(exact, "yes\n");
}

#[test]
fn level_88_alphanumeric_string_thru_range_read_in_range_true() {
    // GRADE holds "C"; PASSING (VALUE "A" THRU "D") is an inclusive string range
    // that contains it → the THEN branch runs. Byte-identical to the oracle.
    let out = assert_matches_oracle(&wrap(
        &["01  GRADE  PIC X VALUE \"C\".", "88  PASSING  VALUE \"A\" THRU \"D\"."],
        &["IF PASSING DISPLAY \"pass\" ELSE DISPLAY \"fail\".", "STOP RUN."],
    ));
    assert_eq!(out, "pass\n");
}

#[test]
fn level_88_alphanumeric_string_thru_range_read_out_of_range_false() {
    // A value BELOW the low bound and a value ABOVE the high bound both fall out of
    // "B" THRU "Y" → the ELSE branch runs. "A" < "B" (below); "Z" > "Y" (above).
    let below = assert_matches_oracle(&wrap(
        &["01  GRADE  PIC X VALUE \"A\".", "88  MID  VALUE \"B\" THRU \"Y\"."],
        &["IF MID DISPLAY \"in\" ELSE DISPLAY \"out\".", "STOP RUN."],
    ));
    assert_eq!(below, "out\n");
    let above = assert_matches_oracle(&wrap(
        &["01  GRADE  PIC X VALUE \"Z\".", "88  MID  VALUE \"B\" THRU \"Y\"."],
        &["IF MID DISPLAY \"in\" ELSE DISPLAY \"out\".", "STOP RUN."],
    ));
    assert_eq!(above, "out\n");
}

#[test]
fn level_88_alphanumeric_string_thru_range_boundaries_are_inclusive() {
    // The range is inclusive at BOTH ends: var == lo ("A") and var == hi ("D") each
    // satisfy PASSING (VALUE "A" THRU "D").
    let lo = assert_matches_oracle(&wrap(
        &["01  GRADE  PIC X VALUE \"A\".", "88  PASSING  VALUE \"A\" THRU \"D\"."],
        &["IF PASSING DISPLAY \"in\" ELSE DISPLAY \"out\".", "STOP RUN."],
    ));
    assert_eq!(lo, "in\n");
    let hi = assert_matches_oracle(&wrap(
        &["01  GRADE  PIC X VALUE \"D\".", "88  PASSING  VALUE \"A\" THRU \"D\"."],
        &["IF PASSING DISPLAY \"in\" ELSE DISPLAY \"out\".", "STOP RUN."],
    ));
    assert_eq!(hi, "in\n");
}

#[test]
fn level_88_alphanumeric_string_thru_range_set_stores_low_bound() {
    // SET PASSING TO TRUE stores the range's LOW bound "A" into GRADE; DISPLAY shows
    // "A". Byte-identical to the oracle's `MOVE`-into-slot store of the low bound.
    let out = assert_matches_oracle(&wrap(
        &["01  GRADE  PIC X VALUE \"C\".", "88  PASSING  VALUE \"A\" THRU \"D\"."],
        &["SET PASSING TO TRUE.", "DISPLAY GRADE.", "STOP RUN."],
    ));
    assert_eq!(out, "A\n");
}

#[test]
fn level_88_alphanumeric_range_or_folded_with_a_discrete_single() {
    // 88 X VALUE "A" THRU "C" "Z" — a range OR-folded with a discrete single. A
    // value inside the range ("B") → true; the discrete "Z" → true; a value outside
    // both ("M") → false. Byte-identical any-match on both engines.
    let in_range = assert_matches_oracle(&wrap(
        &["01  CH  PIC X VALUE \"B\".", "88  OK  VALUE \"A\" THRU \"C\" \"Z\"."],
        &["IF OK DISPLAY \"y\" ELSE DISPLAY \"n\".", "STOP RUN."],
    ));
    assert_eq!(in_range, "y\n");
    let discrete = assert_matches_oracle(&wrap(
        &["01  CH  PIC X VALUE \"Z\".", "88  OK  VALUE \"A\" THRU \"C\" \"Z\"."],
        &["IF OK DISPLAY \"y\" ELSE DISPLAY \"n\".", "STOP RUN."],
    ));
    assert_eq!(discrete, "y\n");
    let neither = assert_matches_oracle(&wrap(
        &["01  CH  PIC X VALUE \"M\".", "88  OK  VALUE \"A\" THRU \"C\" \"Z\"."],
        &["IF OK DISPLAY \"y\" ELSE DISPLAY \"n\".", "STOP RUN."],
    ));
    assert_eq!(neither, "n\n");
    // And SET stores the FIRST value item's low bound "A", which satisfies OK.
    let set = assert_matches_oracle(&wrap(
        &["01  CH  PIC X VALUE \"M\".", "88  OK  VALUE \"A\" THRU \"C\" \"Z\"."],
        &["SET OK TO TRUE.", "DISPLAY CH.", "STOP RUN."],
    ));
    assert_eq!(set, "A\n");
}

#[test]
fn level_88_alphanumeric_multi_char_string_thru_range_space_padding() {
    // A PIC X(2) field with a two-character string range "AA" THRU "ZZ": the
    // space-padded `str_cmp` ordering agrees on both engines. "MN" is inside;
    // SET stores the low bound "AA".
    let in_range = assert_matches_oracle(&wrap(
        &["01  PAIR  PIC X(2) VALUE \"MN\".", "88  OK  VALUE \"AA\" THRU \"ZZ\"."],
        &["IF OK DISPLAY \"y\" ELSE DISPLAY \"n\".", "STOP RUN."],
    ));
    assert_eq!(in_range, "y\n");
    let set = assert_matches_oracle(&wrap(
        &["01  PAIR  PIC X(2) VALUE \"MN\".", "88  OK  VALUE \"AA\" THRU \"ZZ\"."],
        &["SET OK TO TRUE.", "DISPLAY PAIR.", "STOP RUN."],
    ));
    assert_eq!(set, "AA\n");
}

#[test]
fn level_88_alphanumeric_numeric_bound_thru_range_is_a_later_rung() {
    // 88 X VALUE "A" THRU 5 on a PIC X variable — a range with a NON-string
    // (numeric) bound stays a later rung, rejected IDENTICALLY on both engines.
    let src = wrap(
        &["01  FLAG  PIC X VALUE \"C\".", "88  IN-RANGE  VALUE \"A\" THRU 5."],
        &["IF IN-RANGE DISPLAY \"yes\".", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a numeric-bound THRU 88 on a PIC X");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a numeric-bound THRU 88 on a PIC X"
    );
}

#[test]
fn level_88_alphanumeric_filler_string_thru_range_is_a_later_rung() {
    // The FILLER-88 reject still holds for a string THRU range: a level-88 on an
    // unnamed alphanumeric FILLER binds to DIFFERENT items on the two engines, so it
    // is rejected co-totally on BOTH even with a now-accepted string range.
    let src = wrap(
        &["01  FILLER  PIC X VALUE \"C\".", "88  P  VALUE \"A\" THRU \"Z\"."],
        &["IF P DISPLAY \"yes\".", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a FILLER string-range 88");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a FILLER string-range 88"
    );
}

#[test]
fn level_88_alphanumeric_numeric_value_is_a_later_rung() {
    // 88 X VALUE 5 on a PIC X variable — a non-string VALUE on an alphanumeric 88
    // stays a later rung, rejected IDENTICALLY on both engines.
    let src = wrap(
        &["01  FLAG  PIC X VALUE \"5\".", "88  IS-FIVE  VALUE 5."],
        &["IF IS-FIVE DISPLAY \"yes\".", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a numeric VALUE on an alphanumeric 88");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a numeric VALUE on an alphanumeric 88"
    );
}

#[test]
fn level_88_on_an_alphanumeric_filler_is_a_later_rung() {
    // A level-88 whose conditional variable is an UNNAMED (FILLER) alphanumeric item
    // binds to DIFFERENT items on the two engines (the compiler drops the FILLER, the
    // oracle models it), so it is rejected co-totally on BOTH — reading and setting.
    let read = wrap(
        &["01  FILLER  PIC X VALUE \"Z\".", "88  IS-B  VALUE \"B\"."],
        &["IF IS-B DISPLAY \"yes\".", "STOP RUN."],
    );
    assert!(run_cobol(&read).is_err(), "oracle must reject a FILLER alphanumeric 88 (read)");
    assert!(
        compile_source(&read, "e2e").is_err(),
        "compiler must reject a FILLER alphanumeric 88 (read)"
    );
    let set = wrap(
        &["01  FILLER  PIC X VALUE \"Z\".", "88  IS-B  VALUE \"B\"."],
        &["SET IS-B TO TRUE.", "STOP RUN."],
    );
    assert!(run_cobol(&set).is_err(), "oracle must reject a FILLER alphanumeric 88 (SET)");
    assert!(
        compile_source(&set, "e2e").is_err(),
        "compiler must reject a FILLER alphanumeric 88 (SET)"
    );
}

#[test]
fn level_88_on_a_numeric_filler_is_a_later_rung() {
    // The pre-existing latent divergence: a numeric FILLER-88 was already reachable
    // and diverging. It is now rejected co-totally on BOTH engines, reading and
    // setting — the numeric level-88 FILLER case is closed alongside the new
    // alphanumeric one.
    let read = wrap(
        &["01  FILLER  PIC 9 VALUE 5.", "88  IS-NINE  VALUE 9."],
        &["IF IS-NINE DISPLAY \"yes\".", "STOP RUN."],
    );
    assert!(run_cobol(&read).is_err(), "oracle must reject a FILLER numeric 88 (read)");
    assert!(
        compile_source(&read, "e2e").is_err(),
        "compiler must reject a FILLER numeric 88 (read)"
    );
    let set = wrap(
        &["01  FILLER  PIC 9 VALUE 5.", "88  IS-NINE  VALUE 9."],
        &["SET IS-NINE TO TRUE.", "STOP RUN."],
    );
    assert!(run_cobol(&set).is_err(), "oracle must reject a FILLER numeric 88 (SET)");
    assert!(
        compile_source(&set, "e2e").is_err(),
        "compiler must reject a FILLER numeric 88 (SET)"
    );
}

#[test]
fn level_88_after_a_filler_then_named_item_still_works() {
    // Only an 88 IMMEDIATELY following a FILLER rejects: here the 88 follows the
    // NAMED KEEP (not the preceding FILLER), so it is accepted and answers normally.
    let out = assert_matches_oracle(&wrap(
        &["01  FILLER  PIC X VALUE \"Z\".", "01  KEEP  PIC X VALUE \"Y\".", "88  IS-Y  VALUE \"Y\"."],
        &["IF IS-Y DISPLAY \"ok\" ELSE DISPLAY \"no\".", "STOP RUN."],
    ));
    assert_eq!(out, "ok\n");
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

// -------------------------------------------------------------------------
// Cross-category MOVE: unsigned-integer numeric → alphanumeric — vs the oracle.
//
// The numeric sending item is treated as though it held its digit characters
// (its zero-padded magnitude image, exactly what DISPLAY shows), then moved by
// the alphanumeric rules: LEFT-justified, space-padded on the right when the
// receiver is wider, truncated on the right when narrower.
// -------------------------------------------------------------------------

#[test]
fn numeric_to_alphanumeric_move_exact_fit() {
    // PIC 9(3)=042 → PIC X(3): the whole 3-digit image "042".
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC 9(3) VALUE 42.", "01  W  PIC X(3)."],
        &["MOVE N TO W.", "DISPLAY W.", "STOP RUN."],
    ));
    assert_eq!(out, "042\n");
}

#[test]
fn numeric_to_alphanumeric_move_space_pads_on_the_right() {
    // PIC 9(3)=042 → PIC X(5): "042" left-justified, two trailing spaces.
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC 9(3) VALUE 42.", "01  W  PIC X(5)."],
        &["MOVE N TO W.", "DISPLAY W \"|\".", "STOP RUN."],
    ));
    assert_eq!(out, "042  |\n");
}

#[test]
fn numeric_to_alphanumeric_move_truncates_on_the_right() {
    // PIC 9(3)=042 → PIC X(2): keeps the leftmost two digits "04".
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC 9(3) VALUE 42.", "01  W  PIC X(2)."],
        &["MOVE N TO W.", "DISPLAY W.", "STOP RUN."],
    ));
    assert_eq!(out, "04\n");
}

#[test]
fn numeric_to_alphanumeric_move_single_digit() {
    // PIC 9(1)=7 → PIC X(3): "7" then two spaces.
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC 9 VALUE 7.", "01  W  PIC X(3)."],
        &["MOVE N TO W.", "DISPLAY W \"|\".", "STOP RUN."],
    ));
    assert_eq!(out, "7  |\n");
}

#[test]
fn numeric_to_alphanumeric_move_of_a_computed_value() {
    // Compute the source at run time first (ADD builds 123), then MOVE its digit
    // image into the alphanumeric receiver.
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC 9(3) VALUE 100.", "01  W  PIC X(4)."],
        &["ADD 23 TO N.", "MOVE N TO W.", "DISPLAY W \"|\".", "STOP RUN."],
    ));
    assert_eq!(out, "123 |\n");
}

#[test]
fn scaled_numeric_to_alphanumeric_move_exact_fit() {
    // An UNSIGNED SCALED source `PIC 9(2)V9 = 4.2` moves its full (int + frac)
    // digit image "042" — no decimal point — exact fit into X(3).
    let out = assert_matches_oracle(&wrap(
        &["01  F  PIC 9(2)V9 VALUE 4.2.", "01  W  PIC X(3)."],
        &["MOVE F TO W.", "DISPLAY W.", "STOP RUN."],
    ));
    assert_eq!(out, "042\n");
}

#[test]
fn scaled_numeric_to_alphanumeric_move_more_fraction_digits() {
    // `PIC 9(1)V99 = 3.14` → image "314" (1 int digit + 2 frac digits), into X(3).
    let out = assert_matches_oracle(&wrap(
        &["01  F  PIC 9(1)V99 VALUE 3.14.", "01  W  PIC X(3)."],
        &["MOVE F TO W.", "DISPLAY W.", "STOP RUN."],
    ));
    assert_eq!(out, "314\n");
}

#[test]
fn scaled_numeric_to_alphanumeric_move_space_pads() {
    // Wider receiver: "042" left-justified into X(5), two trailing spaces.
    let out = assert_matches_oracle(&wrap(
        &["01  F  PIC 9(2)V9 VALUE 4.2.", "01  W  PIC X(5)."],
        &["MOVE F TO W.", "DISPLAY W \"|\".", "STOP RUN."],
    ));
    assert_eq!(out, "042  |\n");
}

#[test]
fn scaled_numeric_to_alphanumeric_move_truncates() {
    // Narrower receiver: "042" truncated on the right into X(2) → "04".
    let out = assert_matches_oracle(&wrap(
        &["01  F  PIC 9(2)V9 VALUE 4.2.", "01  W  PIC X(2)."],
        &["MOVE F TO W.", "DISPLAY W.", "STOP RUN."],
    ));
    assert_eq!(out, "04\n");
}

#[test]
fn scaled_numeric_to_alphanumeric_move_of_a_computed_value() {
    // Build the scaled value at run time first (COMPUTE 1.4 * 3 → 4.2), then MOVE
    // its digit image "042" into the alphanumeric receiver, then compare it.
    let out = assert_matches_oracle(&wrap(
        &["01  F  PIC 9(2)V9 VALUE 0.", "01  W  PIC X(3)."],
        &[
            "COMPUTE F = 1.4 * 3.",
            "MOVE F TO W.",
            "IF W = \"042\" DISPLAY \"MATCH\" ELSE DISPLAY \"NO\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "MATCH\n");
}

// -------------------------------------------------------------------------
// SIGNED numeric → alphanumeric MOVE: the image is the magnitude with a
// TRAILING SIGN OVERPUNCH on the units digit (positive `{A…I`, negative
// `}J…R`), then moved by the alphanumeric rule. Every case is byte-identical
// to the cobol-runtime oracle.
// -------------------------------------------------------------------------

#[test]
fn signed_numeric_to_alphanumeric_move_positive_exact_fit() {
    // S9(3) = +123 → magnitude "123", units 3 positive → 'C' → "12C" into X(3).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC S9(3) VALUE 123.", "01  W  PIC X(3)."],
        &["MOVE S TO W.", "DISPLAY W.", "STOP RUN."],
    ));
    assert_eq!(out, "12C\n");
}

#[test]
fn signed_numeric_to_alphanumeric_move_positive_space_pads() {
    // Into a WIDER X(5): "12C" left-justified, right space-padded → "12C  ".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC S9(3) VALUE 123.", "01  W  PIC X(5)."],
        &["MOVE S TO W.", "DISPLAY W \"|\".", "STOP RUN."],
    ));
    assert_eq!(out, "12C  |\n");
}

#[test]
fn signed_numeric_to_alphanumeric_move_truncates_on_the_right() {
    // Into a NARROWER X(2): the image "12C" keeps its leftmost two chars → "12".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC S9(3) VALUE 123.", "01  W  PIC X(2)."],
        &["MOVE S TO W.", "DISPLAY W.", "STOP RUN."],
    ));
    assert_eq!(out, "12\n");
}

#[test]
fn signed_numeric_to_alphanumeric_move_negative() {
    // S9(3) = -123 → units 3 negative → 'L' → "12L".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC S9(3) VALUE -123.", "01  W  PIC X(3)."],
        &["MOVE S TO W.", "DISPLAY W.", "STOP RUN."],
    ));
    assert_eq!(out, "12L\n");
}

#[test]
fn signed_numeric_to_alphanumeric_move_units_digit_zero() {
    // Units digit 0 selects '{' (positive) / '}' (negative).
    let neg = assert_matches_oracle(&wrap(
        &["01  S  PIC S9(3) VALUE -120.", "01  W  PIC X(3)."],
        &["MOVE S TO W.", "DISPLAY W.", "STOP RUN."],
    ));
    assert_eq!(neg, "12}\n");
    let pos = assert_matches_oracle(&wrap(
        &["01  S  PIC S9(3) VALUE 120.", "01  W  PIC X(3)."],
        &["MOVE S TO W.", "DISPLAY W.", "STOP RUN."],
    ));
    assert_eq!(pos, "12{\n");
}

#[test]
fn signed_scaled_numeric_to_alphanumeric_move() {
    // S9V9 = -4.2 → magnitude "42", overpunch units 2 negative → 'K' → "4K".
    let neg = assert_matches_oracle(&wrap(
        &["01  F  PIC S9V9 VALUE -4.2.", "01  W  PIC X(2)."],
        &["MOVE F TO W.", "DISPLAY W.", "STOP RUN."],
    ));
    assert_eq!(neg, "4K\n");
    // +4.2 → units 2 positive → 'B' → "4B".
    let pos = assert_matches_oracle(&wrap(
        &["01  F  PIC S9V9 VALUE 4.2.", "01  W  PIC X(2)."],
        &["MOVE F TO W.", "DISPLAY W.", "STOP RUN."],
    ));
    assert_eq!(pos, "4B\n");
}

#[test]
fn signed_numeric_to_alphanumeric_move_of_a_computed_value() {
    // Build a signed value by arithmetic (COMPUTE 2 - 9 = -7 into S9(2)), then MOVE
    // its overpunched image "0P" (units 7 negative → 'P') into the receiver.
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC S9(2) VALUE 0.", "01  W  PIC X(2)."],
        &[
            "COMPUTE N = 2 - 9.",
            "MOVE N TO W.",
            "IF W = \"0P\" DISPLAY \"MATCH\" ELSE DISPLAY \"NO\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "MATCH\n");
}

#[test]
fn signed_value_truncating_to_zero_magnitude_is_positive() {
    // COBOL has no negative zero: -1000 high-order-truncated into PIC S9(3) stores
    // an all-zero slot, which is POSITIVE — its overpunched image is "00{" (units 0,
    // positive), and DISPLAY of the signed field agrees. Both engines must produce
    // "00{" (the compiler's single-i64 slot collapses to a plain 0; the oracle drops
    // the sign of a stored-zero magnitude). Regression for a sign-of-zero divergence.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC S9(4) VALUE -1000.", "01  S  PIC S9(3).", "01  W  PIC X(3)."],
        &["MOVE A TO S.", "MOVE S TO W.", "DISPLAY W.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "00{\n00{\n");
}

#[test]
fn numeric_to_alphanumeric_move_result_compares_alphanumerically() {
    // The MOVE result is a genuine alphanumeric value: comparing it against a
    // string literal agrees with the oracle. PIC 9(3)=42 → "042" into X(3).
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC 9(3) VALUE 42.", "01  W  PIC X(3)."],
        &[
            "MOVE N TO W.",
            "IF W EQUAL \"042\" DISPLAY \"MATCH\" ELSE DISPLAY \"NO\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "MATCH\n");
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
fn figurative_vs_figurative_comparison() {
    // Comparing two figurative constants: each has no length to borrow, so both
    // resolve to a single fill character (`ZERO` → "0", `SPACE` → " "), matching the
    // oracle. ZERO = ZERO and SPACE = SPACE are true; ZERO ≠ SPACE and, by byte
    // ('0'=0x30 > ' '=0x20), ZERO > SPACE / SPACE < ZERO.
    let zz = assert_matches_oracle(&wrap(
        &["01  D  PIC X(1)."],
        &["IF ZERO = ZERO DISPLAY \"T\" ELSE DISPLAY \"F\".", "STOP RUN."],
    ));
    assert_eq!(zz, "T\n");
    let ss = assert_matches_oracle(&wrap(
        &["01  D  PIC X(1)."],
        &["IF SPACE = SPACE DISPLAY \"T\" ELSE DISPLAY \"F\".", "STOP RUN."],
    ));
    assert_eq!(ss, "T\n");
    let zs = assert_matches_oracle(&wrap(
        &["01  D  PIC X(1)."],
        &[
            "IF ZERO = SPACE DISPLAY \"E\" ELSE DISPLAY \"N\".",
            "IF ZERO > SPACE DISPLAY \"G\" ELSE DISPLAY \"L\".",
            "IF SPACE < ZERO DISPLAY \"S\" ELSE DISPLAY \"B\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(zs, "N\nG\nS\n");
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

// -------------------------------------------------------------------------
// Cross-category MOVE: alphanumeric → numeric (the reverse direction) — vs the
// oracle.
//
// An alphanumeric source (`PIC X(m)`) moved into an UNSIGNED INTEGER receiver
// (`PIC 9(n)`, no `S`, no `V`) is read as an unsigned integer formed from its
// characters as digits, then de-scaled into the receiver: RIGHT-justified —
// left-zero-padded when the source has fewer than `n` digits, high-order-
// truncated when more, i.e. `receiver = (integer from the m source chars) mod
// 10^n`. Both engines fold the identical per-character arithmetic
// (`value = value*10 + (byte - '0')`, left to right), so they agree byte-for-byte.
// -------------------------------------------------------------------------

#[test]
fn alphanumeric_to_numeric_move_exact_fit() {
    // PIC X(3)="042" → PIC 9(3): fold 0,4,2 → 42; DISPLAY shows "042".
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(3) VALUE \"042\".", "01  N  PIC 9(3)."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "042\n");
}

#[test]
fn alphanumeric_to_numeric_move_space_source_agrees_no_stray_sign() {
    // Regression: a SPACE source byte (0x20) is below '0', so `(b-'0')` is
    // negative and the fold goes negative. A `PIC 9` field is unsigned, so BOTH
    // engines must store the MAGNITUDE — never a stray '-' in the numeric field.
    // (An uninitialised `PIC X` is spaces, so this is a common, non-adversarial
    // case.) " " → fold -16 → magnitude 16 → PIC 9(3) "016"; the compiler and the
    // oracle must agree (assert_matches_oracle fails if they don't).
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(1) VALUE \" \".", "01  N  PIC 9(3)."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "016\n");
    // A mixed space+digit source stays consistent too: " 5" → fold -155 →
    // magnitude 155 → PIC 9(3) "155".
    let out2 = assert_matches_oracle(&wrap(
        &["01  A  PIC X(2) VALUE \" 5\".", "01  N  PIC 9(3)."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out2, "155\n");
}

#[test]
fn alphanumeric_to_numeric_move_shorter_source_zero_pads() {
    // PIC X(2)="05" → PIC 9(4): fold → 5, right-justified into 4 digits "0005".
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(2) VALUE \"05\".", "01  N  PIC 9(4)."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "0005\n");
}

#[test]
fn alphanumeric_to_numeric_move_longer_source_truncates_high_order() {
    // PIC X(5)="12345" → PIC 9(3): fold → 12345, keep the low-order 3 → 345.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(5) VALUE \"12345\".", "01  N  PIC 9(3)."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "345\n");
}

#[test]
fn alphanumeric_to_numeric_move_single_digit() {
    // PIC X(1)="7" → PIC 9(1): fold → 7.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(1) VALUE \"7\".", "01  N  PIC 9(1)."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "7\n");
}

#[test]
fn alphanumeric_to_numeric_move_then_arithmetic() {
    // The moved value is a genuine number: MOVE "40" into 9(3)=040, ADD 2 → 042.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(2) VALUE \"40\".", "01  N  PIC 9(3)."],
        &["MOVE A TO N.", "ADD 2 TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "042\n");
}

#[test]
fn alphanumeric_to_numeric_move_used_in_compute() {
    // MOVE "06" into 9(3)=006, then COMPUTE R = N * 7 → 42 → "042".
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(2) VALUE \"06\".", "01  N  PIC 9(3).", "01  R  PIC 9(3)."],
        &["MOVE A TO N.", "COMPUTE R = N * 7.", "DISPLAY R.", "STOP RUN."],
    ));
    assert_eq!(out, "042\n");
}

#[test]
fn numeric_alpha_numeric_round_trip() {
    // numeric → alphanumeric → numeric: 9(3)=42 → X(3)="042" → 9(3)=42.
    let out = assert_matches_oracle(&wrap(
        &["01  N  PIC 9(3) VALUE 42.", "01  W  PIC X(3).", "01  M  PIC 9(3)."],
        &["MOVE N TO W.", "MOVE W TO M.", "DISPLAY M.", "STOP RUN."],
    ));
    assert_eq!(out, "042\n");
}

// -------------------------------------------------------------------------
// Cross-category alphanumeric → SCALED numeric MOVE — vs the oracle.
//
// An alphanumeric source (`PIC X(m)`) moved into an UNSIGNED SCALED receiver
// `PIC 9(i)V9(d)` (`d > 0`) folds its `m` characters into an unsigned integer
// `V`; that fold IS the receiver's scaled-slot magnitude directly — it fills the
// `(i + d)` digit positions RIGHT-justified, the implied point `d` places from
// the right. So the slot is `V mod 10^(i+d)`. This is NOT the arithmetic
// decimal-align rule (`V` is not multiplied by `10^d`). DISPLAY shows the raw
// `(i + d)` digits (no point). Both engines fold the identical arithmetic and
// keep the low-order `(i + d)` digits, so they agree byte-for-byte.
// -------------------------------------------------------------------------

#[test]
fn alphanumeric_to_scaled_numeric_move_exact_fit() {
    // PIC X(3)="042" → PIC 9(2)V9: fold → 42, slot 042, reads 4.2 → DISPLAY "042".
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(3) VALUE \"042\".", "01  N  PIC 9(2)V9."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "042\n");
}

#[test]
fn alphanumeric_to_scaled_numeric_move_shorter_source_zero_pads() {
    // PIC X(2)="42" → PIC 9(2)V9: fold → 42, slot 042 (left-zero-padded to the 3
    // positions), reads 4.2 → DISPLAY "042".
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(2) VALUE \"42\".", "01  N  PIC 9(2)V9."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "042\n");
}

#[test]
fn alphanumeric_to_scaled_numeric_move_longer_source_truncates_high_order() {
    // PIC X(5)="12345" → PIC 9(2)V9: fold → 12345, keep the low-order (i+d)=3
    // digits → slot 345, reads 34.5 → DISPLAY "345".
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(5) VALUE \"12345\".", "01  N  PIC 9(2)V9."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "345\n");
}

#[test]
fn alphanumeric_to_scaled_numeric_move_more_fraction_than_source_digits() {
    // PIC X(1)="5" → PIC 9(1)V99: fold → 5, slot 005 (magnitude shorter than
    // i+d=3), reads 0.05 → DISPLAY "005".
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(1) VALUE \"5\".", "01  N  PIC 9(1)V99."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "005\n");
}

#[test]
fn alphanumeric_to_scaled_numeric_move_then_arithmetic() {
    // The moved slot is a genuine scaled number: MOVE "042" into 9(2)V9 = 4.2, then
    // ADD 1.3 → 5.5 → slot "055".
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(3) VALUE \"042\".", "01  N  PIC 9(2)V9."],
        &["MOVE A TO N.", "ADD 1.3 TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "055\n");
}

#[test]
fn alphanumeric_to_scaled_numeric_move_space_source_agrees_no_stray_sign() {
    // A SPACE source byte (0x20) is below '0', so the fold goes negative; an
    // unsigned scaled `PIC 9V9` field keeps the MAGNITUDE. " " → fold -16 →
    // magnitude 16 → slot 016 → reads 1.6. Both engines must agree.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(1) VALUE \" \".", "01  N  PIC 9(2)V9."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "016\n");
}

// -------------------------------------------------------------------------
// Mixed numeric ↔ alphanumeric comparison — vs the oracle.
//
// When a relation compares an UNSIGNED-INTEGER numeric operand with an
// ALPHANUMERIC one (a `PIC X` item or a string literal), COBOL treats the
// numeric operand as though moved to an alphanumeric field — its n-digit
// zero-padded digit image — then compares by the alphanumeric byte rule (the
// shorter side space-padded on the right, byte-by-byte). Both engines build the
// identical image and run the identical space-padded `str_cmp`, so the compiled
// program is byte-identical to the oracle. A signed / scaled numeric operand, a
// numeric literal, or a group item in a mixed relation is a clean later rung.
// -------------------------------------------------------------------------

#[test]
fn mixed_numeric_equals_matching_literal() {
    // NUM PIC 9(3)=42 → image "042"; "042" = "042" → equal.
    let out = assert_matches_oracle(&wrap(
        &["01  NUM  PIC 9(3) VALUE 42."],
        &["IF NUM = \"042\" DISPLAY \"MATCH\" ELSE DISPLAY \"NO\".", "STOP RUN."],
    ));
    assert_eq!(out, "MATCH\n");
}

#[test]
fn mixed_numeric_space_pad_mismatch() {
    // "042" vs "42" → the shorter literal space-pads to "42 "; '0' < '4' → not
    // equal. (A DISPLAY-style value comparison would wrongly call these equal —
    // this pins the alphanumeric byte rule.)
    let out = assert_matches_oracle(&wrap(
        &["01  NUM  PIC 9(3) VALUE 42."],
        &["IF NUM = \"42\" DISPLAY \"MATCH\" ELSE DISPLAY \"NO\".", "STOP RUN."],
    ));
    assert_eq!(out, "NO\n");
}

#[test]
fn mixed_numeric_greater_ordering() {
    // "042" > "040" → '2' > '0' at the last position → greater.
    let out = assert_matches_oracle(&wrap(
        &["01  NUM  PIC 9(3) VALUE 42."],
        &["IF NUM > \"040\" DISPLAY \"GT\" ELSE DISPLAY \"LE\".", "STOP RUN."],
    ));
    assert_eq!(out, "GT\n");
}

#[test]
fn mixed_numeric_on_the_right_operand() {
    // The numeric operand on the RIGHT lowers identically: "042" = "042".
    let out = assert_matches_oracle(&wrap(
        &["01  NUM  PIC 9(3) VALUE 42."],
        &["IF \"042\" = NUM DISPLAY \"MATCH\" ELSE DISPLAY \"NO\".", "STOP RUN."],
    ));
    assert_eq!(out, "MATCH\n");
}

#[test]
fn mixed_numeric_against_a_pic_x_item() {
    // The alphanumeric side is a `PIC X` item (not a literal): W = "042" equals
    // NUM's image "042".
    let out = assert_matches_oracle(&wrap(
        &["01  NUM  PIC 9(3) VALUE 42.", "01  W  PIC X(3) VALUE \"042\"."],
        &["IF NUM = W DISPLAY \"MATCH\" ELSE DISPLAY \"NO\".", "STOP RUN."],
    ));
    assert_eq!(out, "MATCH\n");
}

#[test]
fn mixed_numeric_symbolic_operators() {
    // Symbolic `>=` and `<` operators lower through the same path: "042" >= "042"
    // (equal) and "042" < "050".
    let out = assert_matches_oracle(&wrap(
        &["01  NUM  PIC 9(3) VALUE 42."],
        &[
            "IF NUM >= \"042\" DISPLAY \"GE\" ELSE DISPLAY \"LT\".",
            "IF NUM < \"050\" DISPLAY \"LT\" ELSE DISPLAY \"GE\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "GE\nLT\n");
}

#[test]
fn mixed_numeric_wider_field_space_pads_the_literal() {
    // A wider numeric field, shorter literal: NUM PIC 9(4)=42 → "0042"; the
    // literal "42" pads to "42  " (width 4) → not equal, and "0042" < "42  ".
    let out = assert_matches_oracle(&wrap(
        &["01  NUM  PIC 9(4) VALUE 42."],
        &["IF NUM < \"42\" DISPLAY \"LT\" ELSE DISPLAY \"GE\".", "STOP RUN."],
    ));
    assert_eq!(out, "LT\n");
}

// A SIGNED numeric operand in a mixed relation compares by its magnitude image
// with the operational sign folded into a TRAILING OVERPUNCH on the units digit
// (positive `{A…I`, negative `}J…R`) — the same image the signed→alphanumeric
// MOVE produces. Every case is byte-identical to the cobol-runtime oracle.

#[test]
fn mixed_signed_negative_equals_its_overpunched_image() {
    // `PIC S9(3) = -123` → magnitude "123", units 3 negative → 'L' → "12L", so
    // `IF S = "12L"` is TRUE and `IF S = "12C"` (the positive image) is FALSE.
    let eq = assert_matches_oracle(&wrap(
        &["01  S  PIC S9(3) VALUE -123."],
        &["IF S = \"12L\" DISPLAY \"T\" ELSE DISPLAY \"F\".", "STOP RUN."],
    ));
    assert_eq!(eq, "T\n");
    let ne = assert_matches_oracle(&wrap(
        &["01  S  PIC S9(3) VALUE -123."],
        &["IF S = \"12C\" DISPLAY \"T\" ELSE DISPLAY \"F\".", "STOP RUN."],
    ));
    assert_eq!(ne, "F\n");
}

#[test]
fn mixed_signed_positive_equals_its_overpunched_image() {
    // `PIC S9(3) = +123` → units 3 positive → 'C' → "12C", so `IF S = "12C"` is TRUE.
    // A signed item with an unsigned VALUE is positive (neg=false).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC S9(3) VALUE 123."],
        &["IF S = \"12C\" DISPLAY \"T\" ELSE DISPLAY \"F\".", "STOP RUN."],
    ));
    assert_eq!(out, "T\n");
}

#[test]
fn mixed_signed_units_digit_zero() {
    // Units digit 0 selects '}' (negative) / '{' (positive).
    let neg = assert_matches_oracle(&wrap(
        &["01  S  PIC S9(3) VALUE -120."],
        &["IF S = \"12}\" DISPLAY \"T\" ELSE DISPLAY \"F\".", "STOP RUN."],
    ));
    assert_eq!(neg, "T\n");
    let pos = assert_matches_oracle(&wrap(
        &["01  S  PIC S9(3) VALUE 120."],
        &["IF S = \"12{\" DISPLAY \"T\" ELSE DISPLAY \"F\".", "STOP RUN."],
    ));
    assert_eq!(pos, "T\n");
}

#[test]
fn mixed_signed_scaled_equals_its_overpunched_image() {
    // A scaled `PIC S9V9 = -4.2` → magnitude "42", units 2 negative → 'K' → "4K".
    let out = assert_matches_oracle(&wrap(
        &["01  F  PIC S9V9 VALUE -4.2."],
        &["IF F = \"4K\" DISPLAY \"T\" ELSE DISPLAY \"F\".", "STOP RUN."],
    ));
    assert_eq!(out, "T\n");
}

#[test]
fn mixed_signed_numeric_ordering() {
    // Ordering follows the byte comparison of the overpunched images: `-123` → "12L"
    // and "12L" < "12M" (L=0x4C < M=0x4D), so `IF S < "12M"` is TRUE.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC S9(3) VALUE -123."],
        &["IF S < \"12M\" DISPLAY \"T\" ELSE DISPLAY \"F\".", "STOP RUN."],
    ));
    assert_eq!(out, "T\n");
}

#[test]
fn mixed_signed_zero_magnitude_compares_positive() {
    // COBOL has no negative zero: -1000 high-order-truncated into PIC S9(3) stores an
    // all-zero POSITIVE slot, whose overpunched image is "00{" (units 0, positive).
    // Both engines must build "00{", so `IF S = "00{"` is TRUE (no reintroduced
    // negative-zero divergence).
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC S9(4) VALUE -1000.", "01  S  PIC S9(3)."],
        &[
            "MOVE A TO S.",
            "IF S = \"00{\" DISPLAY \"T\" ELSE DISPLAY \"F\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "T\n");
}

#[test]
fn signed_numeric_vs_zero_figurative_is_a_numeric_comparison() {
    // `IF S = ZERO` on a SIGNED field is a NUMERIC comparison (S vs 0), NOT an
    // alphanumeric one — so a signed field holding 0 compares EQUAL to ZERO, and a
    // negative field is LESS THAN ZERO. The ZERO figurative against a numeric operand
    // must not route the signed item through the overpunch-string path (which would
    // compare "00{"/"12L" against "000" and answer wrongly). Both engines agree.
    let zero_eq = assert_matches_oracle(&wrap(
        &["01  S  PIC S9(3) VALUE 0."],
        &["IF S = ZERO DISPLAY \"T\" ELSE DISPLAY \"F\".", "STOP RUN."],
    ));
    assert_eq!(zero_eq, "T\n");
    // A negative signed field: = ZERO is false, < ZERO is true, > ZERO is false.
    let neg = assert_matches_oracle(&wrap(
        &["01  A  PIC S9(3) VALUE 123.", "01  S  PIC S9(3)."],
        &[
            "COMPUTE S = 0 - A.",
            "IF S = ZERO DISPLAY \"E\" ELSE DISPLAY \"N\".",
            "IF S < ZERO DISPLAY \"L\" ELSE DISPLAY \"G\".",
            "IF S > ZERO DISPLAY \"P\" ELSE DISPLAY \"M\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(neg, "N\nL\nM\n");
    // Reversed operand order (ZERO on the left) behaves identically.
    let rev = assert_matches_oracle(&wrap(
        &["01  S  PIC S9(3) VALUE 0."],
        &["IF ZERO = S DISPLAY \"T\" ELSE DISPLAY \"F\".", "STOP RUN."],
    ));
    assert_eq!(rev, "T\n");
}

#[test]
fn mixed_scaled_numeric_equals_its_digit_image() {
    // An UNSIGNED SCALED operand `PIC 9(2)V9 = 4.2` compares by its (int + frac)
    // digit image "042" — no decimal point — so `IF F = "042"` is TRUE.
    let out = assert_matches_oracle(&wrap(
        &["01  F  PIC 9(2)V9 VALUE 4.2."],
        &["IF F = \"042\" DISPLAY \"MATCH\" ELSE DISPLAY \"NO\".", "STOP RUN."],
    ));
    assert_eq!(out, "MATCH\n");
}

#[test]
fn mixed_scaled_numeric_ordering() {
    // The byte rule on the same "042" image: "042" > "040" → true.
    let out = assert_matches_oracle(&wrap(
        &["01  F  PIC 9(2)V9 VALUE 4.2."],
        &["IF F > \"040\" DISPLAY \"GT\" ELSE DISPLAY \"LE\".", "STOP RUN."],
    ));
    assert_eq!(out, "GT\n");
}

#[test]
fn mixed_scaled_numeric_more_fraction_digits() {
    // `PIC 9(1)V99 = 3.14` → image "314"; `IF F = "314"` is TRUE.
    let out = assert_matches_oracle(&wrap(
        &["01  F  PIC 9(1)V99 VALUE 3.14."],
        &["IF F = \"314\" DISPLAY \"MATCH\" ELSE DISPLAY \"NO\".", "STOP RUN."],
    ));
    assert_eq!(out, "MATCH\n");
}

#[test]
fn mixed_group_item_vs_numeric_is_a_later_rung() {
    // A GROUP item on either side of a mixed relation is a later rung — the
    // compiler does not model group items, so referring to one is `Unsupported`.
    let err = compile_source(
        &wrap(
            &[
                "01  G.",
                "    05  A  PIC X(2) VALUE \"04\".",
                "    05  B  PIC X(1) VALUE \"2\".",
                "01  NUM  PIC 9(3) VALUE 42.",
            ],
            &["IF G = NUM DISPLAY \"Y\".", "STOP RUN."],
        ),
        "e2e",
    )
    .unwrap_err();
    assert!(matches!(err, cobol_iir_compiler::CompileError::Unsupported(_)), "got {err:?}");
}

// ---------------------------------------------------------------------------
// Reference modification `IDENT(start:len)` — DISPLAY and comparison contexts.
// A 1-based `start`; an omitted length runs to the end of the item. Every case
// asserts the compiled slice is byte-identical to the oracle's.
// ---------------------------------------------------------------------------

#[test]
fn refmod_display_mid_substring() {
    // WS = "ABCDE"; WS(2:3) selects positions 2..4 → "BCD".
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"ABCDE\"."],
        &["DISPLAY WS(2:3).", "STOP RUN."],
    ));
    assert_eq!(out, "BCD\n");
}

#[test]
fn refmod_display_omitted_length_runs_to_end() {
    // WS(3:) has no length → from position 3 to the end → "CDE".
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"ABCDE\"."],
        &["DISPLAY WS(3:).", "STOP RUN."],
    ));
    assert_eq!(out, "CDE\n");
}

#[test]
fn refmod_display_single_leading_char() {
    // WS(1:1) is the first character.
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"ABCDE\"."],
        &["DISPLAY WS(1:1).", "STOP RUN."],
    ));
    assert_eq!(out, "A\n");
}

#[test]
fn refmod_display_whole_item_via_full_length() {
    // A length equal to the item width selects the whole thing.
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"ABCDE\"."],
        &["DISPLAY WS(1:5).", "STOP RUN."],
    ));
    assert_eq!(out, "ABCDE\n");
}

#[test]
fn refmod_in_if_comparison_against_literal() {
    // The leading 3 characters equal "ABC" → the THEN branch.
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"ABCDE\"."],
        &[
            "IF WS(1:3) = \"ABC\" DISPLAY \"MATCH\" ELSE DISPLAY \"NO\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "MATCH\n");
}

#[test]
fn refmod_in_if_comparison_false_branch() {
    // WS(2:2) = "BC", which is not "ZZ" → the ELSE branch.
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"ABCDE\"."],
        &[
            "IF WS(2:2) = \"ZZ\" DISPLAY \"MATCH\" ELSE DISPLAY \"NO\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "NO\n");
}

#[test]
fn refmod_as_evaluate_subject() {
    // EVALUATE over WS(2:2) = "BC".
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"ABCDE\"."],
        &[
            "EVALUATE WS(2:2)",
            "WHEN \"BC\" DISPLAY \"HIT\"",
            "WHEN OTHER DISPLAY \"MISS\"",
            "END-EVALUATE.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "HIT\n");
}

#[test]
fn refmod_compared_against_another_refmod() {
    // WS(1:2) = "AB" and WS(4:2) = "DE": both slices of the same item, unequal.
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"ABCDE\"."],
        &[
            "IF WS(1:2) = WS(4:2) DISPLAY \"SAME\" ELSE DISPLAY \"DIFF\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "DIFF\n");
    // Two slices that ARE equal: WS = "ABAB", WS(1:2) = WS(3:2) = "AB".
    let eq = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(4) VALUE \"ABAB\"."],
        &[
            "IF WS(1:2) = WS(3:2) DISPLAY \"SAME\" ELSE DISPLAY \"DIFF\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(eq, "SAME\n");
}

// COMPUTED reference modification — `WS(J:K)` where the start and/or length are
// DATA-NAME (run-time integer) operands, lowered to a run-time `str_slice` over
// registers computed with `sub`/`add`. Each case runs the compiled JIT output
// against the oracle byte-for-byte; the out-of-range case pins that BOTH engines
// trap under the identical `start0 < 0 || end < start0 || end > width` rule.
// ---------------------------------------------------------------------------

#[test]
fn refmod_computed_mid_substring() {
    // WS(J:K) with J=2, K=3 over "ABCDE" → "BCD" — both indices are data items.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  WS  PIC X(5) VALUE \"ABCDE\".",
            "01  J   PIC 9 VALUE 2.",
            "01  K   PIC 9 VALUE 3.",
        ],
        &["DISPLAY WS(J:K).", "STOP RUN."],
    ));
    assert_eq!(out, "BCD\n");
}

#[test]
fn refmod_computed_omitted_length_runs_to_end() {
    // WS(J:) with J=3 has no length → from position 3 to the item end → "CDE".
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"ABCDE\".", "01  J   PIC 9 VALUE 3."],
        &["DISPLAY WS(J:).", "STOP RUN."],
    ));
    assert_eq!(out, "CDE\n");
}

#[test]
fn refmod_computed_literal_start_data_name_length() {
    // Mixed indices: a literal start with a computed length (WS(2:K), K=3) still
    // takes the run-time path because the length is a data-name → "BCD".
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"ABCDE\".", "01  K   PIC 9 VALUE 3."],
        &["DISPLAY WS(2:K).", "STOP RUN."],
    ));
    assert_eq!(out, "BCD\n");
}

#[test]
fn refmod_computed_in_if_comparison_against_literal() {
    // A computed refmod on the left of an IF comparison: WS(J:K) = "ABC" with
    // J=1, K=3 over "ABCDE" → the THEN branch.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  WS  PIC X(5) VALUE \"ABCDE\".",
            "01  J   PIC 9 VALUE 1.",
            "01  K   PIC 9 VALUE 3.",
        ],
        &[
            "IF WS(J:K) = \"ABC\" DISPLAY \"MATCH\" ELSE DISPLAY \"NO\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "MATCH\n");
}

#[test]
fn refmod_computed_as_evaluate_subject() {
    // A computed refmod as the EVALUATE subject: WS(J:2) = "BC" with J=2.
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"ABCDE\".", "01  J   PIC 9 VALUE 2."],
        &[
            "EVALUATE WS(J:2)",
            "WHEN \"BC\" DISPLAY \"HIT\"",
            "WHEN OTHER DISPLAY \"MISS\"",
            "END-EVALUATE.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "HIT\n");
}

#[test]
fn refmod_computed_index_driven_by_compute() {
    // The indices come from a COMPUTEd value: J = 1 + 1 = 2, then WS(J:2) = "BC".
    // Exercises reading a numeric slot that was written at run time, not by VALUE.
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"ABCDE\".", "01  J   PIC 9 VALUE 0."],
        &["COMPUTE J = 1 + 1.", "DISPLAY WS(J:2).", "STOP RUN."],
    ));
    assert_eq!(out, "BC\n");
}

#[test]
fn refmod_computed_same_program_in_range_and_comparison() {
    // The SAME shape (a computed slice compared against another computed slice)
    // driven with in-range indices: WS(J:2) vs WS(M:2) with J=1, M=4 over
    // "ABCDE" → "AB" vs "DE" → DIFF; then equal indices → SAME.
    let diff = assert_matches_oracle(&wrap(
        &[
            "01  WS  PIC X(5) VALUE \"ABCDE\".",
            "01  J   PIC 9 VALUE 1.",
            "01  M   PIC 9 VALUE 4.",
        ],
        &[
            "IF WS(J:2) = WS(M:2) DISPLAY \"SAME\" ELSE DISPLAY \"DIFF\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(diff, "DIFF\n");
    let same = assert_matches_oracle(&wrap(
        &[
            "01  WS  PIC X(4) VALUE \"ABAB\".",
            "01  J   PIC 9 VALUE 1.",
            "01  M   PIC 9 VALUE 3.",
        ],
        &[
            "IF WS(J:2) = WS(M:2) DISPLAY \"SAME\" ELSE DISPLAY \"DIFF\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(same, "SAME\n");
}

#[test]
fn refmod_computed_out_of_range_traps_on_both_engines() {
    // WS(J:K) with J=4, K=5 over a 5-char item runs to position 8 > 5. The oracle
    // returns a RuntimeError and the compiled str_slice hits its out-of-bounds
    // bounds check — both engines trap identically.
    assert_both_trap(&wrap(
        &[
            "01  WS  PIC X(5) VALUE \"ABCDE\".",
            "01  J   PIC 9 VALUE 4.",
            "01  K   PIC 9 VALUE 5.",
        ],
        &["DISPLAY WS(J:K).", "STOP RUN."],
    ));
}

#[test]
fn refmod_computed_zero_start_traps_on_both_engines() {
    // A start of 0 → start0 = -1 < 0 → out-of-range on both engines.
    assert_both_trap(&wrap(
        &["01  WS  PIC X(5) VALUE \"ABCDE\".", "01  J   PIC 9 VALUE 0."],
        &["DISPLAY WS(J:2).", "STOP RUN."],
    ));
}

// ---------------------------------------------------------------------------
// Reference-modification SOURCE of a MOVE — `MOVE base(start:len) TO dst` into
// an ALPHANUMERIC receiver. The slice is fit to the receiver's width by the
// ordinary alphanumeric char rule (left-justify; space-pad if wider; truncate
// if narrower), exactly as a same-category char MOVE reshapes. A numeric
// receiver stays a later rung, rejected on both engines. Every accepted case
// pins the compiled JIT output to the oracle byte-for-byte.
// ---------------------------------------------------------------------------

#[test]
fn refmod_move_source_mid_substring() {
    // WS = "ABCDE"; MOVE WS(2:3) → "BCD" into an equal-width receiver.
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"ABCDE\".", "01  DST PIC X(3)."],
        &["MOVE WS(2:3) TO DST.", "DISPLAY DST.", "STOP RUN."],
    ));
    assert_eq!(out, "BCD\n");
}

#[test]
fn refmod_move_source_omitted_length_runs_to_end() {
    // MOVE WS(3:) → from position 3 to the end → "CDE".
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"ABCDE\".", "01  DST PIC X(3)."],
        &["MOVE WS(3:) TO DST.", "DISPLAY DST.", "STOP RUN."],
    ));
    assert_eq!(out, "CDE\n");
}

#[test]
fn refmod_move_source_single_leading_char() {
    // MOVE WS(1:1) → the first character "A".
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"ABCDE\".", "01  DST PIC X(1)."],
        &["MOVE WS(1:1) TO DST.", "DISPLAY DST.", "STOP RUN."],
    ));
    assert_eq!(out, "A\n");
}

#[test]
fn refmod_move_source_into_wider_receiver_space_pads() {
    // The 2-char slice "BC" into a 5-wide receiver → left-justified, tail spaces.
    // A trailing marker proves the two padding spaces are present.
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"ABCDE\".", "01  DST PIC X(5)."],
        &["MOVE WS(2:2) TO DST.", "DISPLAY DST \"|\".", "STOP RUN."],
    ));
    assert_eq!(out, "BC   |\n");
}

#[test]
fn refmod_move_source_into_narrower_receiver_truncates() {
    // The 4-char slice "BCDE" into a 2-wide receiver → keep the leftmost two "BC".
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"ABCDE\".", "01  DST PIC X(2)."],
        &["MOVE WS(2:4) TO DST.", "DISPLAY DST.", "STOP RUN."],
    ));
    assert_eq!(out, "BC\n");
}

#[test]
fn refmod_move_source_computed_index() {
    // A computed (data-name) index takes the run-time slice-fit path: WS(J:K) with
    // J=2, K=3 → "BCD", into an equal-width receiver.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  WS  PIC X(5) VALUE \"ABCDE\".",
            "01  J   PIC 9 VALUE 2.",
            "01  K   PIC 9 VALUE 3.",
            "01  DST PIC X(3).",
        ],
        &["MOVE WS(J:K) TO DST.", "DISPLAY DST.", "STOP RUN."],
    ));
    assert_eq!(out, "BCD\n");
}

#[test]
fn refmod_move_source_computed_index_into_wider_receiver_space_pads() {
    // Run-time-length slice fit into a WIDER receiver: WS(J:2)="BC" (J=2) into a
    // 5-wide receiver → "BC   ". Exercises the run-time concat-then-truncate pad.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  WS  PIC X(5) VALUE \"ABCDE\".",
            "01  J   PIC 9 VALUE 2.",
            "01  DST PIC X(5).",
        ],
        &["MOVE WS(J:2) TO DST.", "DISPLAY DST \"|\".", "STOP RUN."],
    ));
    assert_eq!(out, "BC   |\n");
}

#[test]
fn refmod_move_source_multiple_receivers() {
    // MOVE WS(1:2) TO A B → "AB" into both receivers (the loop over receivers).
    let out = assert_matches_oracle(&wrap(
        &[
            "01  WS  PIC X(5) VALUE \"ABCDE\".",
            "01  A   PIC X(2).",
            "01  B   PIC X(4).",
        ],
        &["MOVE WS(1:2) TO A B.", "DISPLAY A \"|\".", "DISPLAY B \"|\".", "STOP RUN."],
    ));
    assert_eq!(out, "AB|\nAB  |\n");
}

#[test]
fn refmod_move_source_ascii_prefix_window_non_ascii_outside() {
    // Non-ASCII CLEANLINESS: the multi-byte char 'é' sits at the END of the source,
    // strictly OUTSIDE the (1:3) window, so byte-index == char-index within the
    // window and the byte-based (compiler) and char-based (oracle) slices coincide
    // → "abc", byte-identical. (A window covering/following a multi-byte char is
    // the pre-existing refmod byte-vs-char chip, deliberately not exercised here.)
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"abcdé\".", "01  DST PIC X(3)."],
        &["MOVE WS(1:3) TO DST.", "DISPLAY DST.", "STOP RUN."],
    ));
    assert_eq!(out, "abc\n");
}

#[test]
fn refmod_move_source_into_numeric_receiver_is_a_later_rung() {
    // The remaining boundary: a refmod MOVE source into a NUMERIC receiver stays a
    // later rung, rejected on BOTH engines (de-editing a slice into a numeric field
    // is not lowered on this rung).
    let src = wrap(
        &["01  WS  PIC X(5) VALUE \"12345\".", "01  NUM PIC 9(3)."],
        &["MOVE WS(1:3) TO NUM.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a refmod MOVE into a numeric receiver");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a refmod MOVE into a numeric receiver"
    );
}

// STRING with a reference-modification SENDING FIELD — `WS(start:len)`
// contributes its sliced substring as the field's char image, produced by the
// SAME refmod-substring evaluators DISPLAY / comparison / MOVE-source use
// (`refmod_string` in the oracle, `ref_mod_slice` in the compiler). Only
// CONSTANT (literal) indices are accepted; a computed (data-name) index stays a
// later rung, rejected IDENTICALLY on both engines. Positive tests use ASCII
// data so the byte-based (compiler) and char-based (oracle) slices coincide.
// ---------------------------------------------------------------------------

#[test]
fn string_refmod_source_mid_substring() {
    // A single refmod sending field, DELIMITED BY SIZE: WS(2:3) of "ABCDEF" is the
    // 3-char slice starting at 1-based position 2 → "BCD", left-justified into a
    // 6-wide receiver whose untouched tail stays blank.
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(6) VALUE \"ABCDEF\".", "01  DST PIC X(6) VALUE SPACES."],
        &["STRING WS(2:3) DELIMITED BY SIZE INTO DST.", "DISPLAY DST.", "STOP RUN."],
    ));
    assert_eq!(out, "BCD   \n");
}

#[test]
fn string_refmod_source_omitted_length_runs_to_end() {
    // An omitted length runs to the end of the base item: WS(3:) of "ABCDEF" →
    // "CDEF".
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(6) VALUE \"ABCDEF\".", "01  DST PIC X(6) VALUE SPACES."],
        &["STRING WS(3:) DELIMITED BY SIZE INTO DST.", "DISPLAY DST.", "STOP RUN."],
    ));
    assert_eq!(out, "CDEF  \n");
}

#[test]
fn string_refmod_source_concatenated_with_literal_and_item() {
    // The refmod image composes in the concat exactly like a plain field: WS(1:2)
    // ++ "-" ++ WS(4:2) of "ABCDEF" → "AB" ++ "-" ++ "DE" = "AB-DE".
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(6) VALUE \"ABCDEF\".", "01  DST PIC X(8) VALUE SPACES."],
        &[
            "STRING WS(1:2) \"-\" WS(4:2) DELIMITED BY SIZE INTO DST.",
            "DISPLAY DST.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "AB-DE   \n");
}

#[test]
fn string_refmod_source_under_a_delimiter() {
    // Under DELIMITED BY a single-char delimiter the refmod image is truncated at
    // its first delimiter char, like any field: WS(1:5) of "ab,cdef" is "ab,cd",
    // whose prefix up to the first "," is "ab".
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(7) VALUE \"ab,cdef\".", "01  DST PIC X(6) VALUE SPACES."],
        &["STRING WS(1:5) DELIMITED BY \",\" INTO DST.", "DISPLAY DST.", "STOP RUN."],
    ));
    assert_eq!(out, "ab    \n");
}

#[test]
fn string_refmod_source_with_pointer() {
    // The refmod image composes with WITH POINTER without special-casing: WS(2:3)
    // of "ABCDEF" = "BCD" overlaid from pointer 1 into a "......" receiver → "BCD..."
    // with the pointer written back to 1 + 3 = 4 ("04").
    let out = assert_matches_oracle(&wrap(
        &[
            "01  WS  PIC X(6) VALUE \"ABCDEF\".",
            "01  DST PIC X(6) VALUE \"......\".",
            "01  P   PIC 9(2) VALUE 1.",
        ],
        &[
            "STRING WS(2:3) DELIMITED BY SIZE INTO DST WITH POINTER P.",
            "DISPLAY DST.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "BCD...\n04\n");
}

#[test]
fn string_refmod_source_ascii_window_non_ascii_outside() {
    // Non-ASCII CLEANLINESS: 'é' sits at the END of "abcdé", strictly OUTSIDE the
    // (1:3) window, so byte-index == char-index within the window and the byte-based
    // (compiler) and char-based (oracle) slices coincide → "abc", byte-identical. (A
    // window covering/following a multi-byte char is the pre-existing refmod
    // byte-vs-char chip, deliberately not exercised here.)
    let out = assert_matches_oracle(&wrap(
        &["01  WS  PIC X(5) VALUE \"abcdé\".", "01  DST PIC X(3) VALUE SPACES."],
        &["STRING WS(1:3) DELIMITED BY SIZE INTO DST.", "DISPLAY DST.", "STOP RUN."],
    ));
    assert_eq!(out, "abc\n");
}

#[test]
fn string_refmod_source_computed_index_is_a_later_rung() {
    // A COMPUTED (data-name) index gives a run-time length the compiler's compile-
    // time STRING image contract cannot carry, so a refmod sending field with a
    // data-name index stays a later rung — rejected IDENTICALLY on BOTH engines.
    let src = wrap(
        &[
            "01  WS  PIC X(6) VALUE \"ABCDEF\".",
            "01  DST PIC X(6) VALUE SPACES.",
            "01  J   PIC 9 VALUE 2.",
            "01  K   PIC 9 VALUE 3.",
        ],
        &["STRING WS(J:K) DELIMITED BY SIZE INTO DST.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a computed-index refmod STRING source");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a computed-index refmod STRING source"
    );
}

// STRING — concatenate sending fields into an alphanumeric receiver
// (DELIMITED BY SIZE = each source taken in full). The receiver is LEFT-
// justified, truncated at its width, and — the COBOL surprise — its tail beyond
// what STRING wrote is left UNCHANGED (no space-fill). Every case pins the
// compiled JIT output to the oracle byte-for-byte.
// ---------------------------------------------------------------------------

#[test]
fn string_concatenates_two_items() {
    // "ABC" ++ "DE" = "ABCDE", left-justified into a 10-wide field that started
    // as spaces — the untouched tail stays blank.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(3) VALUE \"ABC\".",
            "01  B  PIC X(2) VALUE \"DE\".",
            "01  T  PIC X(10) VALUE SPACES.",
        ],
        &["STRING A B DELIMITED BY SIZE INTO T.", "DISPLAY T.", "STOP RUN."],
    ));
    assert_eq!(out, "ABCDE     \n");
}

#[test]
fn string_truncates_at_receiver_width() {
    // Concatenation "ABCDE" is wider than the 4-char receiver → truncated to
    // "ABCD".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(3) VALUE \"ABC\".",
            "01  B  PIC X(2) VALUE \"DE\".",
            "01  T  PIC X(4) VALUE SPACES.",
        ],
        &["STRING A B DELIMITED BY SIZE INTO T.", "DISPLAY T.", "STOP RUN."],
    ));
    assert_eq!(out, "ABCD\n");
}

#[test]
fn string_mixes_a_literal_source() {
    // A string literal between two items: "ABC" ++ "-" ++ "DE" = "ABC-DE".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(3) VALUE \"ABC\".",
            "01  B  PIC X(2) VALUE \"DE\".",
            "01  T  PIC X(8) VALUE SPACES.",
        ],
        &["STRING A \"-\" B DELIMITED BY SIZE INTO T.", "DISPLAY T.", "STOP RUN."],
    ));
    assert_eq!(out, "ABC-DE  \n");
}

#[test]
fn string_includes_a_full_width_item_with_its_spaces() {
    // DELIMITED BY SIZE takes each item in FULL — a PIC X(5) holding "HI" carries
    // its trailing spaces into the result: "HI   " ++ "!" = "HI   !".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(5) VALUE \"HI\".",
            "01  T  PIC X(8) VALUE SPACES.",
        ],
        &["STRING A \"!\" DELIMITED BY SIZE INTO T.", "DISPLAY T.", "STOP RUN."],
    ));
    assert_eq!(out, "HI   !  \n");
}

#[test]
fn string_leaves_the_untouched_tail_unchanged() {
    // The no-fill rule made visible: T starts with a non-space VALUE. STRING
    // writes only "AB" (2 chars); the remaining "ZZZZ" of the original stays put.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(2) VALUE \"AB\".",
            "01  T  PIC X(6) VALUE \"ZZZZZZ\".",
        ],
        &["STRING A DELIMITED BY SIZE INTO T.", "DISPLAY T.", "STOP RUN."],
    ));
    assert_eq!(out, "ABZZZZ\n");
}

#[test]
fn string_with_numeric_literal_source() {
    // A numeric literal contributes its source digits verbatim: "IT" ++ "42".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(2) VALUE \"IT\".",
            "01  T  PIC X(6) VALUE SPACES.",
        ],
        &["STRING A 42 DELIMITED BY SIZE INTO T.", "DISPLAY T.", "STOP RUN."],
    ));
    assert_eq!(out, "IT42  \n");
}

// ---------------------------------------------------------------------------
// STRING with a FIGURATIVE-CONSTANT sending field — SPACE/ZERO reduce to a
// single-character image (SPACE→" ", ZERO→"0"), dropping into the concat exactly
// like a 1-char string literal on BOTH engines (oracle string_source_chars,
// compiler string_source). `Fig` is closed at {Space, Zero}, both ASCII, so the
// non-ASCII-sending-field guard passes unchanged and no non-ASCII figurative can
// reach the path. Every case pins the compiled JIT output to the oracle.
// ---------------------------------------------------------------------------

#[test]
fn string_figurative_space_sending_field_delimited_by_size() {
    // SPACE contributes its single-char image " ": " " ++ "X" = " X" into a
    // PIC X(2) receiver.
    let out = assert_matches_oracle(&wrap(
        &["01  D  PIC X(2) VALUE SPACES."],
        &["STRING SPACE \"X\" DELIMITED BY SIZE INTO D.", "DISPLAY D.", "STOP RUN."],
    ));
    assert_eq!(out, " X\n");
}

#[test]
fn string_figurative_zero_sending_field() {
    // ZERO contributes its single-char image "0": "A" ++ "0" ++ "B" = "A0B" into
    // a PIC X(3) receiver.
    let out = assert_matches_oracle(&wrap(
        &["01  D  PIC X(3) VALUE SPACES."],
        &["STRING \"A\" ZERO \"B\" DELIMITED BY SIZE INTO D.", "DISPLAY D.", "STOP RUN."],
    ));
    assert_eq!(out, "A0B\n");
}

#[test]
fn string_figurative_plural_spellings() {
    // SPACE/SPACES and ZERO/ZEROS/ZEROES are the SAME figurative constant, so every
    // spelling folds to the identical single-char image — all four programs below
    // produce byte-identical output (" X" for the space spellings, "0X" for zero).
    let space_singular = assert_matches_oracle(&wrap(
        &["01  D  PIC X(2) VALUE SPACES."],
        &["STRING SPACE \"X\" DELIMITED BY SIZE INTO D.", "DISPLAY D.", "STOP RUN."],
    ));
    let space_plural = assert_matches_oracle(&wrap(
        &["01  D  PIC X(2) VALUE SPACES."],
        &["STRING SPACES \"X\" DELIMITED BY SIZE INTO D.", "DISPLAY D.", "STOP RUN."],
    ));
    assert_eq!(space_singular, " X\n");
    assert_eq!(space_plural, space_singular);

    let zero_singular = assert_matches_oracle(&wrap(
        &["01  D  PIC X(2) VALUE SPACES."],
        &["STRING ZERO \"X\" DELIMITED BY SIZE INTO D.", "DISPLAY D.", "STOP RUN."],
    ));
    let zeros = assert_matches_oracle(&wrap(
        &["01  D  PIC X(2) VALUE SPACES."],
        &["STRING ZEROS \"X\" DELIMITED BY SIZE INTO D.", "DISPLAY D.", "STOP RUN."],
    ));
    let zeroes = assert_matches_oracle(&wrap(
        &["01  D  PIC X(2) VALUE SPACES."],
        &["STRING ZEROES \"X\" DELIMITED BY SIZE INTO D.", "DISPLAY D.", "STOP RUN."],
    ));
    assert_eq!(zero_singular, "0X\n");
    assert_eq!(zeros, zero_singular);
    assert_eq!(zeroes, zero_singular);
}

#[test]
fn string_figurative_mixed_with_literal_and_item() {
    // A figurative alongside a data-name item and a string literal in one STRING
    // list: item "AB" ++ SPACE " " ++ literal "-" ++ ZERO "0" ++ item "CD" =
    // "AB -0CD" into a wide-enough PIC X(8) receiver.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(2) VALUE \"AB\".",
            "01  B  PIC X(2) VALUE \"CD\".",
            "01  T  PIC X(8) VALUE SPACES.",
        ],
        &[
            "STRING A SPACE \"-\" ZERO B DELIMITED BY SIZE INTO T.",
            "DISPLAY T.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "AB -0CD \n");
}

#[test]
fn string_figurative_with_pointer() {
    // A figurative sending field composes with WITH POINTER: SPACE places its single
    // char " " at pointer 1 (0-based 0) into a "....." receiver → " ...." with the
    // pointer advanced by 1 to 1 + 1 = 2 ("02").
    let out = assert_matches_oracle(&wrap(
        &[
            "01  T  PIC X(5) VALUE \".....\".",
            "01  P  PIC 9(2) VALUE 1.",
        ],
        &[
            "STRING SPACE DELIMITED BY SIZE INTO T WITH POINTER P.",
            "DISPLAY T.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, " ....\n02\n");
}

#[test]
fn string_figurative_delimited_by_delimiter() {
    // Under DELIMITED BY a delimiter equal to the figurative char, the 1-char image
    // is truncated at its OWN first char: SPACE's image " " scanned for the delimiter
    // " " yields the empty prefix, so the field contributes nothing and the receiver
    // keeps its VALUE. Both engines agree via assert_matches_oracle.
    let out = assert_matches_oracle(&wrap(
        &["01  D  PIC X(3) VALUE \"ZZZ\"."],
        &["STRING SPACE DELIMITED BY \" \" INTO D.", "DISPLAY D.", "STOP RUN."],
    ));
    assert_eq!(out, "ZZZ\n");
}

#[test]
fn string_figurative_all_ascii_characterization() {
    // NON-ASCII PARITY: `Fig` is closed at {Space, Zero}, both inherently ASCII, so
    // NO non-ASCII figurative operand exists to reach this path — the single-char
    // image is always ASCII and the compiler's byte scan and the oracle's char scan
    // coincide. (A non-ASCII DATA-NAME sending field is the separate, pre-existing
    // byte-vs-char chip, deliberately not exercised here.) This all-ASCII case pins
    // the figurative image byte-identical: SPACE " " ++ ZERO "0" ++ literal "z" =
    // " 0z" into a PIC X(4) receiver.
    let out = assert_matches_oracle(&wrap(
        &["01  T  PIC X(4) VALUE SPACES."],
        &["STRING SPACE ZERO \"z\" DELIMITED BY SIZE INTO T.", "DISPLAY T.", "STOP RUN."],
    ));
    assert_eq!(out, " 0z \n");
}

// ---------------------------------------------------------------------------
// STRING … DELIMITED BY a single-char delimiter — each sending field contributes
// only its PREFIX up to (not including) the first occurrence of the delimiter in
// that field; the per-field prefixes are concatenated and overlaid EXACTLY as the
// DELIMITED BY SIZE path does (leftmost min(len, width), no tail space-fill). The
// delimiter and any string-literal sending field must be ASCII (the compiler's
// prefix scan is byte-based; the oracle scans by char). Each case pins the
// compiled JIT output to the oracle byte-for-byte.
// ---------------------------------------------------------------------------

#[test]
fn string_delim_truncates_each_field_at_the_first_delimiter() {
    // Three fields, each cut at its first comma: "ab,cd"→"ab", "ef"→"ef" (no
    // comma), "gh,ij"→"gh"; concatenation "abefgh" into a 20-wide receiver.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(5) VALUE \"ab,cd\".",
            "01  B  PIC X(2) VALUE \"ef\".",
            "01  C  PIC X(5) VALUE \"gh,ij\".",
            "01  T  PIC X(20) VALUE SPACES.",
        ],
        &[
            "STRING A B C",
            "    DELIMITED BY \",\" INTO T.",
            "DISPLAY T.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "abefgh              \n");
}

#[test]
fn string_delim_field_without_the_delimiter_is_taken_whole() {
    // A field that does not contain the delimiter contributes its ENTIRE image.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(2) VALUE \"xy\".", "01  T  PIC X(10) VALUE SPACES."],
        &["STRING A DELIMITED BY \",\" INTO T.", "DISPLAY T.", "STOP RUN."],
    ));
    assert_eq!(out, "xy        \n");
}

#[test]
fn string_delim_field_starting_with_the_delimiter_contributes_nothing() {
    // A field whose first char IS the delimiter contributes the empty string, so
    // only the following field's prefix reaches the receiver.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(3) VALUE \",xy\".",
            "01  B  PIC X(2) VALUE \"AB\".",
            "01  T  PIC X(6) VALUE SPACES.",
        ],
        &[
            "STRING A B",
            "    DELIMITED BY \",\" INTO T.",
            "DISPLAY T.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "AB    \n");
}

#[test]
fn string_delim_from_a_pic_x1_identifier() {
    // The delimiter may be a PIC X(1) item (reduced by the same single_delim_code
    // UNSTRING uses): DL holds "," so "ab,cd" contributes "ab".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  DL PIC X(1) VALUE \",\".",
            "01  A  PIC X(5) VALUE \"ab,cd\".",
            "01  T  PIC X(10) VALUE SPACES.",
        ],
        &["STRING A DELIMITED BY DL INTO T.", "DISPLAY T.", "STOP RUN."],
    ));
    assert_eq!(out, "ab        \n");
}

#[test]
fn string_delim_result_longer_than_receiver_is_truncated() {
    // The prefix "abcde" (5 chars) overflows a 4-wide receiver → truncated "abcd".
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(7) VALUE \"abcde,f\".", "01  T  PIC X(4) VALUE SPACES."],
        &["STRING A DELIMITED BY \",\" INTO T.", "DISPLAY T.", "STOP RUN."],
    ));
    assert_eq!(out, "abcd\n");
}

#[test]
fn string_delim_result_shorter_than_receiver_preserves_the_tail() {
    // The prefix "ab" (2 chars) is overlaid onto a 6-wide receiver holding "ZZZZZZ";
    // STRING does NOT space-fill, so the untouched tail "ZZZZ" survives.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(5) VALUE \"ab,cd\".", "01  T  PIC X(6) VALUE \"ZZZZZZ\"."],
        &["STRING A DELIMITED BY \",\" INTO T.", "DISPLAY T.", "STOP RUN."],
    ));
    assert_eq!(out, "abZZZZ\n");
}

#[test]
fn string_delim_mixes_fields_with_and_without_the_delimiter() {
    // "p,q"→"p", "rs"→"rs" (no comma), "t,u"→"t"; concatenation "prst".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(3) VALUE \"p,q\".",
            "01  B  PIC X(2) VALUE \"rs\".",
            "01  C  PIC X(3) VALUE \"t,u\".",
            "01  T  PIC X(10) VALUE SPACES.",
        ],
        &[
            "STRING A B C",
            "    DELIMITED BY \",\" INTO T.",
            "DISPLAY T.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "prst      \n");
}

#[test]
fn string_delimited_by_size_still_takes_each_field_whole() {
    // Regression: with DELIMITED BY SIZE the delimiter char is NOT special — the
    // comma inside "ab,cd" stays, and each field is taken in full.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(5) VALUE \"ab,cd\".", "01  T  PIC X(10) VALUE SPACES."],
        &["STRING A DELIMITED BY SIZE INTO T.", "DISPLAY T.", "STOP RUN."],
    ));
    assert_eq!(out, "ab,cd     \n");
}

#[test]
fn string_delim_non_ascii_delimiter_is_a_later_rung() {
    // A single but NON-ASCII delimiter ("é" is one char / two UTF-8 bytes) would
    // make the byte-based compiler scan diverge from the char-based oracle, so it
    // is deferred — rejected on BOTH engines to stay co-total.
    let src = wrap(
        &["01  A  PIC X(3) VALUE \"abc\".", "01  T  PIC X(6) VALUE SPACES."],
        &["STRING A DELIMITED BY \"é\" INTO T.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a non-ASCII delimiter");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a non-ASCII delimiter"
    );
}

#[test]
fn string_delim_multi_char_delimiter_is_a_later_rung() {
    // Only a SINGLE-character delimiter this rung; a 2-char delimiter is deferred.
    let src = wrap(
        &["01  A  PIC X(3) VALUE \"abc\".", "01  T  PIC X(6) VALUE SPACES."],
        &["STRING A DELIMITED BY \"ab\" INTO T.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a multi-char delimiter");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a multi-char delimiter"
    );
}

#[test]
fn string_delim_non_ascii_literal_sending_field_is_a_later_rung() {
    // Under an ACTIVE delimiter a field's prefix boundary is byte-vs-char sensitive,
    // so a non-ASCII string-LITERAL sending field ("café") is deferred on BOTH
    // engines. (Under DELIMITED BY SIZE such a literal is fine — no boundary scan.)
    let src = wrap(
        &["01  T  PIC X(10) VALUE SPACES."],
        &["STRING \"café\" DELIMITED BY \",\" INTO T.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a non-ASCII literal field under a delimiter");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a non-ASCII literal field under a delimiter"
    );
}

#[test]
fn string_delim_no_overflow_runs_not_clause() {
    // A delimited STRING whose delimited concatenation FITS ⇒ no overflow, so the
    // NOT ON OVERFLOW body runs. Each field's prefix (up to ",") is "ab" then "cd" =
    // "abcd" (4) into a 6-wide receiver — fits, tail preserved. Both engines agree.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(5) VALUE \"ab,zz\".",
            "01  B  PIC X(5) VALUE \"cd,zz\".",
            "01  T  PIC X(6) VALUE \"......\".",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "STRING A B DELIMITED BY \",\" INTO T",
            "    ON OVERFLOW MOVE \"YES\" TO F",
            "    NOT ON OVERFLOW MOVE \"NON\" TO F.",
            "DISPLAY T.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "abcd..\nNON\n");
}

// ---------------------------------------------------------------------------
// STRING … WITH POINTER — overlay the concatenation into the receiver starting
// at the 1-based position held by an unsigned-integer pointer item, then write
// the pointer back to `p + chars_placed`. An out-of-range initial pointer (`p ==
// 0` or `p > size`) is ISO overflow: no transfer, receiver AND pointer left
// unchanged (ON OVERFLOW is still deferred). Receivers are preloaded with a "."
// sentinel VALUE so every "unchanged" byte is observable. Every case pins the
// compiled JIT output to the oracle byte-for-byte (receiver AND pointer).
// ---------------------------------------------------------------------------

#[test]
fn string_pointer_one_equals_no_pointer() {
    // The correctness ANCHOR: `WITH POINTER p` with `p = 1` overlays at position 0,
    // so the receiver must be IDENTICAL to the same STRING WITHOUT the phrase. The
    // pointer version then additionally writes the resume position back: 3 chars
    // placed from position 1 → p := 1 + 3 = 4 ("04").
    let with_ptr = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(3) VALUE \"abc\".",
            "01  T  PIC X(6) VALUE \"......\".",
            "01  P  PIC 9(2) VALUE 1.",
        ],
        &[
            "STRING A DELIMITED BY SIZE INTO T WITH POINTER P.",
            "DISPLAY T.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    let no_ptr = assert_matches_oracle(&wrap(
        &["01  A  PIC X(3) VALUE \"abc\".", "01  T  PIC X(6) VALUE \"......\"."],
        &["STRING A DELIMITED BY SIZE INTO T.", "DISPLAY T.", "STOP RUN."],
    ));
    // No-pointer STRING overlays "abc" over the leftmost 3, preserving the tail.
    assert_eq!(no_ptr, "abc...\n");
    // p = 1 fills the SAME receiver, then appends the resume pointer "04".
    assert_eq!(with_ptr, format!("{no_ptr}04\n"));
}

#[test]
fn string_pointer_mid_receiver_preserves_head_and_tail() {
    // p = 3 overlays at 0-based index 2; "XY" (2 chars) lands at positions 2–3, so
    // the head (0–1) and tail (4–5) keep their sentinel dots: "..XY..". p := 3 + 2
    // = 5 ("05"). This pins the overlay-at-offset with BOTH ends preserved.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(2) VALUE \"XY\".",
            "01  T  PIC X(6) VALUE \"......\".",
            "01  P  PIC 9(2) VALUE 3.",
        ],
        &[
            "STRING A DELIMITED BY SIZE INTO T WITH POINTER P.",
            "DISPLAY T.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "..XY..\n05\n");
}

#[test]
fn string_pointer_content_exactly_fills_to_the_end() {
    // p = 3 into a 5-wide receiver leaves room for exactly 3 chars (positions 2–4);
    // "XYZ" fills them precisely: "..XYZ". chars_placed = 3, p := 3 + 3 = 6 = size +
    // 1 ("06") — the boundary where the last char sits at the receiver's end.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(3) VALUE \"XYZ\".",
            "01  T  PIC X(5) VALUE \".....\".",
            "01  P  PIC 9(2) VALUE 3.",
        ],
        &[
            "STRING A DELIMITED BY SIZE INTO T WITH POINTER P.",
            "DISPLAY T.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "..XYZ\n06\n");
}

#[test]
fn string_pointer_overflow_drops_the_excess() {
    // "WXYZ" (4 chars) into a 5-wide receiver at p = 3 has room for only 3 chars
    // (positions 2–4): "WXY" is placed, "Z" is DROPPED (ISO overflow; ON OVERFLOW
    // still deferred, so no imperative runs). chars_placed = size − (p−1) = 3, so
    // p := size + 1 = 6 ("06"). Receiver "..WXY".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(4) VALUE \"WXYZ\".",
            "01  T  PIC X(5) VALUE \".....\".",
            "01  P  PIC 9(2) VALUE 3.",
        ],
        &[
            "STRING A DELIMITED BY SIZE INTO T WITH POINTER P.",
            "DISPLAY T.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "..WXY\n06\n");
}

#[test]
fn string_pointer_at_size_places_one_char() {
    // p = size (= 4) is the last in-range value: only one position (index 3) remains,
    // so just the first char "X" of "XYZ" is placed ("...X") and the rest dropped.
    // chars_placed = 1, p := 4 + 1 = 5 = size + 1 ("05").
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(3) VALUE \"XYZ\".",
            "01  T  PIC X(4) VALUE \"....\".",
            "01  P  PIC 9(2) VALUE 4.",
        ],
        &[
            "STRING A DELIMITED BY SIZE INTO T WITH POINTER P.",
            "DISPLAY T.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "...X\n05\n");
}

#[test]
fn string_pointer_past_end_is_overflow_no_transfer() {
    // p = size + 1 (= 5 > 4) is out of range: ISO overflow ⇒ NO character is
    // transferred (receiver keeps its sentinel "....") and the pointer is left
    // UNCHANGED (stays 5 → "05"). Both engines skip the whole operation identically.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(3) VALUE \"XYZ\".",
            "01  T  PIC X(4) VALUE \"....\".",
            "01  P  PIC 9(2) VALUE 5.",
        ],
        &[
            "STRING A DELIMITED BY SIZE INTO T WITH POINTER P.",
            "DISPLAY T.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "....\n05\n");
}

#[test]
fn string_pointer_zero_is_overflow_no_transfer() {
    // p = 0 would make the 0-based start −1 (underflow); it is out of range, so ISO
    // overflow ⇒ no transfer and the pointer is left unchanged (stays 0 → "00").
    // This is the guard that keeps the start computation from underflowing.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(3) VALUE \"XYZ\".",
            "01  T  PIC X(4) VALUE \"....\".",
            "01  P  PIC 9(2) VALUE 0.",
        ],
        &[
            "STRING A DELIMITED BY SIZE INTO T WITH POINTER P.",
            "DISPLAY T.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "....\n00\n");
}

#[test]
fn string_pointer_with_delimited_by_delimiter() {
    // The pointer path over the DELIMITED BY delim branch (a run-time concat). Each
    // field is cut at its first comma: "ab,cd"→"ab", "ef"→"ef"; concat "abef" (4).
    // p = 2 overlays at index 1 → ".abef...". p := 2 + 4 = 6 ("06").
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(5) VALUE \"ab,cd\".",
            "01  B  PIC X(2) VALUE \"ef\".",
            "01  T  PIC X(8) VALUE \"........\".",
            "01  P  PIC 9(2) VALUE 2.",
        ],
        &[
            "STRING A B",
            "    DELIMITED BY \",\" INTO T WITH POINTER P.",
            "DISPLAY T.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, ".abef...\n06\n");
}

#[test]
fn string_pointer_fuzz_across_full_range() {
    // Sweep the initial pointer across the WHOLE range `[0, size + 2]` = `[0, 6]` for
    // a 4-wide receiver and a 2-char sending field "XY". For every value — the two
    // out-of-range ends (0, 5, 6) and every in-range start (1..=4) — the compiled
    // JIT output must be byte-identical to the oracle (receiver AND written-back
    // pointer). This is the co-totality proof: both engines agree for EVERY value.
    for pstart in 0..=6u32 {
        let data = [
            "01  A  PIC X(2) VALUE \"XY\".".to_string(),
            "01  T  PIC X(4) VALUE \"....\".".to_string(),
            format!("01  P  PIC 9(2) VALUE {pstart}."),
        ];
        let data_refs: Vec<&str> = data.iter().map(String::as_str).collect();
        // assert_matches_oracle panics with a clear message on any divergence.
        assert_matches_oracle(&wrap(
            &data_refs,
            &[
                "STRING A DELIMITED BY SIZE INTO T WITH POINTER P.",
                "DISPLAY T.",
                "DISPLAY P.",
                "STOP RUN.",
            ],
        ));
    }
}

#[test]
fn string_pointer_signed_is_a_later_rung() {
    // The pointer must be an UNSIGNED integer. A signed pointer (`PIC S9`) is a clean
    // later rung, rejected on BOTH engines (compiler at build time, oracle at exec).
    let src = wrap(
        &[
            "01  A  PIC X(3) VALUE \"abc\".",
            "01  T  PIC X(6) VALUE \"......\".",
            "01  P  PIC S9(2) VALUE 1.",
        ],
        &["STRING A DELIMITED BY SIZE INTO T WITH POINTER P.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a signed pointer");
    assert!(compile_source(&src, "e2e").is_err(), "compiler must reject a signed pointer");
}

#[test]
fn string_pointer_fractional_is_a_later_rung() {
    // A fractional pointer (`PIC 9V9`) is not an integer position — a later rung,
    // rejected identically on both engines.
    let src = wrap(
        &[
            "01  A  PIC X(3) VALUE \"abc\".",
            "01  T  PIC X(6) VALUE \"......\".",
            "01  P  PIC 9V9 VALUE 1.",
        ],
        &["STRING A DELIMITED BY SIZE INTO T WITH POINTER P.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a fractional pointer");
    assert!(compile_source(&src, "e2e").is_err(), "compiler must reject a fractional pointer");
}

#[test]
fn string_pointer_non_numeric_is_a_later_rung() {
    // A non-numeric pointer (`PIC X`) has no integer position — a later rung,
    // rejected identically on both engines.
    let src = wrap(
        &[
            "01  A  PIC X(3) VALUE \"abc\".",
            "01  T  PIC X(6) VALUE \"......\".",
            "01  P  PIC X(2) VALUE \"12\".",
        ],
        &["STRING A DELIMITED BY SIZE INTO T WITH POINTER P.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a non-numeric pointer");
    assert!(compile_source(&src, "e2e").is_err(), "compiler must reject a non-numeric pointer");
}

// ---------------------------------------------------------------------------
// STRING … ON OVERFLOW / NOT ON OVERFLOW — run a nested imperative depending on
// whether the STRING overflowed. Overflow = the receiver filled before every
// sending character was transferred (chars dropped) OR the initial WITH POINTER
// value was out of range. The overflow BOOLEAN is computed with the identical
// comparison on both engines; each case pins the compiled JIT output (receiver +
// a flag item the imperative writes) to the oracle byte-for-byte.
// ---------------------------------------------------------------------------

#[test]
fn string_overflow_fires_when_concat_longer_than_receiver() {
    // (a) "abcd"+"efgh" = 8 chars into a 5-wide receiver: 3 chars dropped ⇒
    // overflow. The ON OVERFLOW body writes the "YES" sentinel into F; the receiver
    // holds the truncated head "abcde".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(4) VALUE \"abcd\".",
            "01  B  PIC X(4) VALUE \"efgh\".",
            "01  T  PIC X(5) VALUE \".....\".",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "STRING A B DELIMITED BY SIZE INTO T",
            "    ON OVERFLOW MOVE \"YES\" TO F",
            "    NOT ON OVERFLOW MOVE \"NON\" TO F.",
            "DISPLAY T.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "abcde\nYES\n");
}

#[test]
fn string_no_overflow_runs_not_on_overflow() {
    // (b) "ab"+"cd" = 4 chars fit in a 5-wide receiver ⇒ NO overflow. The NOT ON
    // OVERFLOW body runs, writing "NON"; the receiver keeps its untouched tail dot.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(2) VALUE \"ab\".",
            "01  B  PIC X(2) VALUE \"cd\".",
            "01  T  PIC X(5) VALUE \".....\".",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "STRING A B DELIMITED BY SIZE INTO T",
            "    ON OVERFLOW MOVE \"YES\" TO F",
            "    NOT ON OVERFLOW MOVE \"NON\" TO F.",
            "DISPLAY T.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "abcd.\nNON\n");
}

#[test]
fn string_exact_fit_is_not_overflow() {
    // Boundary: concat length EQUALS the receiver width (nothing dropped) ⇒ NOT
    // overflow, so the NOT ON OVERFLOW body runs. Pins `total == width` on the
    // not-overflow side (the `>` vs `>=` boundary both engines must agree on).
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(5) VALUE \"abcde\".",
            "01  T  PIC X(5) VALUE \".....\".",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "STRING A DELIMITED BY SIZE INTO T",
            "    ON OVERFLOW MOVE \"YES\" TO F",
            "    NOT ON OVERFLOW MOVE \"NON\" TO F.",
            "DISPLAY T.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "abcde\nNON\n");
}

#[test]
fn string_on_overflow_only_present() {
    // (c) ON OVERFLOW present, NOT ON OVERFLOW absent. Two sub-cases share the flag:
    // when overflow fires the flag flips; when it does not, the flag is UNCHANGED
    // (no NOT clause to run).
    let fires = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(8) VALUE \"abcdefgh\".",
            "01  T  PIC X(4) VALUE \"....\".",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "STRING A DELIMITED BY SIZE INTO T ON OVERFLOW MOVE \"YES\" TO F.",
            "DISPLAY T.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(fires, "abcd\nYES\n");
    let quiet = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(2) VALUE \"ab\".",
            "01  T  PIC X(4) VALUE \"....\".",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "STRING A DELIMITED BY SIZE INTO T ON OVERFLOW MOVE \"YES\" TO F.",
            "DISPLAY T.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    // No overflow and no NOT clause ⇒ F keeps its initial "no ".
    assert_eq!(quiet, "ab..\nno \n");
}

#[test]
fn string_not_on_overflow_only_present() {
    // (d) NOT ON OVERFLOW present, ON OVERFLOW absent, content fits ⇒ the NOT body
    // runs, writing "NON". The overflow-SKIPS-a-lone-NOT-clause path (jmp_if_false
    // over an empty on-branch) is the SAME skeleton the both-clause tests above
    // exercise. (Note: a bare trailing imperative is greedy — a following statement
    // would fold into its `{ statement }` — so the observation stays inside the same
    // sentence structure the passing both-clause cases use.)
    let fits = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(2) VALUE \"ab\".",
            "01  T  PIC X(4) VALUE \"....\".",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "STRING A DELIMITED BY SIZE INTO T",
            "    NOT ON OVERFLOW MOVE \"NON\" TO F.",
            "DISPLAY T.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(fits, "ab..\nNON\n");
}

#[test]
fn string_pointer_drop_fires_on_overflow() {
    // (e) WITH POINTER where the pointer causes a drop. p = 4 into a 6-wide receiver
    // leaves 3 chars of room (positions 3–5); "abcde" (5) drops 2 ⇒ overflow. The
    // receiver becomes "...abc" (head dots preserved), p := 4 + 3 = 7 ("07"), and the
    // ON OVERFLOW body writes "YES".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(5) VALUE \"abcde\".",
            "01  T  PIC X(6) VALUE \"......\".",
            "01  P  PIC 9(2) VALUE 4.",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "STRING A DELIMITED BY SIZE INTO T WITH POINTER P",
            "    ON OVERFLOW MOVE \"YES\" TO F",
            "    NOT ON OVERFLOW MOVE \"NON\" TO F.",
            "DISPLAY T.",
            "DISPLAY P.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "...abc\n07\nYES\n");
}

#[test]
fn string_pointer_zero_out_of_range_fires_on_overflow() {
    // (f) WITH POINTER out of range, p = 0. No data movement, pointer UNCHANGED, but
    // ON OVERFLOW now runs (the behaviour change this rung introduces). Receiver keeps
    // all its dots, P stays "00", F becomes "YES".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(3) VALUE \"abc\".",
            "01  T  PIC X(6) VALUE \"......\".",
            "01  P  PIC 9(2) VALUE 0.",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "STRING A DELIMITED BY SIZE INTO T WITH POINTER P",
            "    ON OVERFLOW MOVE \"YES\" TO F",
            "    NOT ON OVERFLOW MOVE \"NON\" TO F.",
            "DISPLAY T.",
            "DISPLAY P.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "......\n00\nYES\n");
}

#[test]
fn string_pointer_past_end_out_of_range_fires_on_overflow() {
    // (f cont.) WITH POINTER out of range, p > size. p = 9 into a 6-wide receiver:
    // no movement, P unchanged ("09"), ON OVERFLOW runs ⇒ F = "YES".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(3) VALUE \"abc\".",
            "01  T  PIC X(6) VALUE \"......\".",
            "01  P  PIC 9(2) VALUE 9.",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "STRING A DELIMITED BY SIZE INTO T WITH POINTER P",
            "    ON OVERFLOW MOVE \"YES\" TO F",
            "    NOT ON OVERFLOW MOVE \"NON\" TO F.",
            "DISPLAY T.",
            "DISPLAY P.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "......\n09\nYES\n");
}

#[test]
fn string_pointer_anchor_no_overflow_runs_not_clause() {
    // (g) WITH POINTER p = 1 anchor, content fits ⇒ NO overflow, so the NOT ON
    // OVERFLOW body runs. "ab" overlays at position 0 of a 5-wide receiver, tail
    // preserved ("ab..."), p := 1 + 2 = 3 ("03"), F = "NON".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(2) VALUE \"ab\".",
            "01  T  PIC X(5) VALUE \".....\".",
            "01  P  PIC 9(2) VALUE 1.",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "STRING A DELIMITED BY SIZE INTO T WITH POINTER P",
            "    ON OVERFLOW MOVE \"YES\" TO F",
            "    NOT ON OVERFLOW MOVE \"NON\" TO F.",
            "DISPLAY T.",
            "DISPLAY P.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "ab...\n03\nNON\n");
}

#[test]
fn string_on_overflow_body_propagates_control_flow() {
    // (h) The ON OVERFLOW body contains a GO TO, proving the handler's `Flow` unwinds
    // out of exec_string / the emitted block. Overflow fires, GO TO jumps to OVF,
    // which DISPLAYs and stops — the "AFTER" line after STRING is never reached.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(8) VALUE \"abcdefgh\".",
            "01  T  PIC X(4) VALUE \"....\".",
        ],
        &[
            "STRING A DELIMITED BY SIZE INTO T ON OVERFLOW GO TO OVF.",
            "DISPLAY \"AFTER\".",
            "STOP RUN.",
            "OVF.",
            "DISPLAY \"JUMPED\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "JUMPED\n");
}

#[test]
fn string_not_on_overflow_body_propagates_control_flow() {
    // (h cont.) A GO TO inside NOT ON OVERFLOW when the STRING fits — the not-overflow
    // path must ALSO unwind its handler's `Flow`. "ab" fits, so NOT runs, jumps to OK.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(2) VALUE \"ab\".", "01  T  PIC X(4) VALUE \"....\"."],
        &[
            "STRING A DELIMITED BY SIZE INTO T NOT ON OVERFLOW GO TO OKAY.",
            "DISPLAY \"AFTER\".",
            "STOP RUN.",
            "OKAY.",
            "DISPLAY \"FITS\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "FITS\n");
}

#[test]
fn string_with_no_overflow_clauses_still_works() {
    // (i) A plain STRING with NEITHER clause lowers exactly as before this rung — no
    // imperative, no branch skeleton. Regression anchor for the empty-clause path.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(3) VALUE \"abc\".", "01  T  PIC X(5) VALUE \".....\"."],
        &["STRING A DELIMITED BY SIZE INTO T.", "DISPLAY T.", "STOP RUN."],
    ));
    assert_eq!(out, "abc..\n");
}

#[test]
fn string_delim_overflow_fires_on_run_time_drop() {
    // A DELIMITED-BY-delimiter STRING whose run-time concatenation overflows the
    // receiver. Each field's prefix (up to ",") concatenates to "abXY" = 4 chars into
    // a 3-wide receiver ⇒ overflow via the run-time `clen > W` test. F becomes "YES".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(4) VALUE \"ab,z\".",
            "01  B  PIC X(4) VALUE \"XY,z\".",
            "01  T  PIC X(3) VALUE \"...\".",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "STRING A B DELIMITED BY \",\" INTO T",
            "    ON OVERFLOW MOVE \"YES\" TO F",
            "    NOT ON OVERFLOW MOVE \"NON\" TO F.",
            "DISPLAY T.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "abX\nYES\n");
}

// ---------------------------------------------------------------------------
// UNSTRING — split one alphanumeric source on a single delimiter into several
// receivers (the inverse of STRING). Each receiver — INCLUDING the last — takes
// only the field up to the NEXT delimiter; extra fields are dropped, an empty
// field (consecutive/leading delimiter) yields all spaces, and once the source
// is exhausted the remaining receivers are left UNCHANGED. Each field lands as
// an ordinary alphanumeric MOVE (left-justified, space-padded, truncated). Every
// case pins the compiled JIT output to the oracle byte-for-byte.
// ---------------------------------------------------------------------------

#[test]
fn unstring_exact_fit() {
    // "A,B,C" splits into exactly three fields for three receivers; each PIC X(3)
    // receiver is the field left-justified and space-padded.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"A,B,C\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  R3 PIC X(3) VALUE SPACES.",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2 R3.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY R3.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \nC  \n");
}

#[test]
fn unstring_drops_extra_fields() {
    // "A,B,C,D" has four fields but only three receivers — the last receiver takes
    // "C" (the field up to the NEXT delimiter, NOT the remainder) and "D" is
    // dropped (that would be ON OVERFLOW, a later rung).
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(7) VALUE \"A,B,C,D\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  R3 PIC X(3) VALUE SPACES.",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2 R3.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY R3.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \nC  \n");
}

#[test]
fn unstring_leaves_trailing_receiver_unchanged() {
    // "A,B" fills two receivers; the source is then exhausted, so R3 keeps its
    // prior VALUE "ZZZ" — it is NOT space-filled.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(3) VALUE \"A,B\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  R3 PIC X(3) VALUE \"ZZZ\".",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2 R3.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY R3.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \nZZZ\n");
}

#[test]
fn unstring_empty_field_from_consecutive_delimiters() {
    // "A,,C" — the two adjacent commas bound an EMPTY field, so R2 becomes all
    // spaces while R1 and R3 get "A" and "C".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(4) VALUE \"A,,C\".",
            "01  R1 PIC X(3) VALUE \"...\".",
            "01  R2 PIC X(3) VALUE \"...\".",
            "01  R3 PIC X(3) VALUE \"...\".",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2 R3.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY R3.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \n   \nC  \n");
}

#[test]
fn unstring_delimiter_at_start() {
    // A LEADING delimiter bounds an empty first field: ",X" → R1 = spaces, R2 = "X".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(2) VALUE \",X\".",
            "01  R1 PIC X(3) VALUE \"...\".",
            "01  R2 PIC X(3) VALUE \"...\".",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "   \nX  \n");
}

#[test]
fn unstring_space_delimiter() {
    // The delimiter can be a space: "A B C" splits on blanks into three fields.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"A B C\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  R3 PIC X(3) VALUE SPACES.",
        ],
        &[
            "UNSTRING S DELIMITED BY \" \" INTO R1 R2 R3.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY R3.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \nC  \n");
}

#[test]
fn unstring_pic_x1_item_delimiter() {
    // The delimiter may be a PIC X(1) item (its single stored character), not just
    // a literal — the compiler reads it at run time with str_index.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"A;B;C\".",
            "01  DL PIC X(1) VALUE \";\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  R3 PIC X(3) VALUE SPACES.",
        ],
        &[
            "UNSTRING S DELIMITED BY DL INTO R1 R2 R3.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY R3.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \nC  \n");
}

#[test]
fn unstring_truncates_a_long_field() {
    // A field wider than its receiver is truncated on the right (left-justified
    // MOVE): "ABCDE" into a PIC X(3) receiver keeps "ABC".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(7) VALUE \"ABCDE,Z\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "ABC\nZ  \n");
}

// ---------------------------------------------------------------------------
// UNSTRING with a LITERAL source — the field text comes from an alphanumeric
// STRING literal's OWN bytes instead of an item's storage. Only the source
// PROVIDER differs; the delimiter scan and per-receiver reshape are the SAME
// shared machinery exercised by the identifier-source cases above, so every
// splitting behaviour (exhaustion, empty fields, truncation, reshape) is
// re-pinned here against the oracle to prove the two providers agree. A
// NUMERIC-literal, a FIGURATIVE (SPACE), and a reference-modified source remain
// later rungs — rejected on BOTH engines.
// ---------------------------------------------------------------------------

#[test]
fn unstring_literal_source_three_fields() {
    // The canonical case: the source is the quoted literal "a,b,c" itself (no S
    // item at all). Three comma-delimited fields fill three PIC X(1) receivers.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  R1 PIC X(1) VALUE SPACE.",
            "01  R2 PIC X(1) VALUE SPACE.",
            "01  R3 PIC X(1) VALUE SPACE.",
        ],
        &[
            "UNSTRING \"a,b,c\" DELIMITED BY \",\" INTO R1 R2 R3.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY R3.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "a\nb\nc\n");
}

#[test]
fn unstring_literal_source_exhausted_leaves_receiver_unchanged() {
    // The literal "A,B" fills two receivers; the source is then exhausted, so R3
    // keeps its prior VALUE "ZZZ" — it is NOT space-filled (identical rule to the
    // identifier-source case, now driven by the literal's bytes).
    let out = assert_matches_oracle(&wrap(
        &[
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  R3 PIC X(3) VALUE \"ZZZ\".",
        ],
        &[
            "UNSTRING \"A,B\" DELIMITED BY \",\" INTO R1 R2 R3.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY R3.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \nZZZ\n");
}

#[test]
fn unstring_literal_source_leading_trailing_consecutive_empty_fields() {
    // Leading, consecutive, and trailing delimiters each bound an EMPTY field.
    // ",A,,B," → f1="" f2="A" f3="" f4="B" f5="" ; the four receivers take the
    // first four fields (f5 dropped), so R1/R3 become all spaces.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  R1 PIC X(3) VALUE \"...\".",
            "01  R2 PIC X(3) VALUE \"...\".",
            "01  R3 PIC X(3) VALUE \"...\".",
            "01  R4 PIC X(3) VALUE \"...\".",
        ],
        &[
            "UNSTRING \",A,,B,\" DELIMITED BY \",\" INTO R1 R2 R3 R4.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY R3.",
            "DISPLAY R4.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "   \nA  \n   \nB  \n");
}

#[test]
fn unstring_literal_source_reshapes_to_receiver_width() {
    // The literal's fields are RESHAPED to each receiver's own width: "ABCDE" is
    // wider than PIC X(3) (truncated to "ABC"), while "Z" is narrower than
    // PIC X(4) (space-padded to "Z   ").
    let out = assert_matches_oracle(&wrap(
        &["01  R1 PIC X(3) VALUE SPACES.", "01  R2 PIC X(4) VALUE SPACES."],
        &[
            "UNSTRING \"ABCDE,Z\" DELIMITED BY \",\" INTO R1 R2.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "ABC\nZ   \n");
}

#[test]
fn unstring_literal_source_single_field_no_delimiter() {
    // With the delimiter ABSENT from the literal, the whole literal is one field
    // that lands in the sole receiver (reshaped to its width).
    let out = assert_matches_oracle(&wrap(
        &["01  R1 PIC X(5) VALUE SPACES."],
        &[
            "UNSTRING \"HI\" DELIMITED BY \",\" INTO R1.",
            "DISPLAY R1.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "HI   \n");
}

#[test]
fn unstring_identifier_source_still_works_regression() {
    // Regression: an ordinary identifier source (an alphanumeric item's storage)
    // still splits exactly as before the literal-source rung was added.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"A,B,C\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  R3 PIC X(3) VALUE SPACES.",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2 R3.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY R3.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \nC  \n");
}

#[test]
fn unstring_numeric_literal_source_is_a_later_rung() {
    // Only an ALPHANUMERIC string literal is a valid literal source. A NUMERIC
    // literal source is deferred — rejected on BOTH engines (oracle at read time,
    // compiler at emit time), mirroring the identifier path's numeric-source
    // rejection intent.
    let src = wrap(
        &["01  R1 PIC X(3) VALUE SPACES.", "01  R2 PIC X(3) VALUE SPACES."],
        &["UNSTRING 123 DELIMITED BY \",\" INTO R1 R2.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a numeric-literal source");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a numeric-literal source"
    );
}

// ---------------------------------------------------------------------------
// UNSTRING with a FIGURATIVE-CONSTANT source (SPACE / ZERO). Both engines map
// the figurative to its single-character ASCII image — SPACE -> " " (0x20),
// ZERO -> "0" (0x30) — reducing to the EXISTING string-literal source scan.
// Both images are ASCII, so the non-ASCII-literal-source guard is never tripped.
// `Fig` = {Space, Zero} is closed, so no non-ASCII figurative can reach the
// path; a numeric literal and a computed ref-mod remain later rungs. Symmetric
// to the STRING sending-field and CONVERTING figurative rungs.
// ---------------------------------------------------------------------------

#[test]
fn unstring_figurative_space_source() {
    // SPACE is the single character " " with no comma, so the whole 1-char source
    // is one field that lands in A (space-padded to its X(3) width → three
    // spaces); the source is then exhausted, so B keeps its prior VALUE "ZZZ".
    let out = assert_matches_oracle(&wrap(
        &["01  A PIC X(3) VALUE \"ZZZ\".", "01  B PIC X(3) VALUE \"ZZZ\"."],
        &[
            "UNSTRING SPACE DELIMITED BY \",\" INTO A B.",
            "DISPLAY A.",
            "DISPLAY B.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "   \nZZZ\n");
}

#[test]
fn unstring_figurative_zero_source() {
    // ZERO is the single character "0" with no space delimiter, so the whole
    // 1-char source is one field → A gets "0" reshaped to its X(3) width → "0  ".
    let out = assert_matches_oracle(&wrap(
        &["01  A PIC X(3) VALUE SPACES."],
        &["UNSTRING ZERO DELIMITED BY \" \" INTO A.", "DISPLAY A.", "STOP RUN."],
    ));
    assert_eq!(out, "0  \n");
}

#[test]
fn unstring_figurative_plural_spellings() {
    // Every figurative spelling folds to the SAME single-character source: SPACE
    // and SPACES both map to " ", and ZERO / ZEROS / ZEROES all map to "0". Each
    // spelling therefore produces the identical byte-for-byte output as its
    // canonical form above.
    let space_plural = assert_matches_oracle(&wrap(
        &["01  A PIC X(3) VALUE \"ZZZ\".", "01  B PIC X(3) VALUE \"ZZZ\"."],
        &[
            "UNSTRING SPACES DELIMITED BY \",\" INTO A B.",
            "DISPLAY A.",
            "DISPLAY B.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(space_plural, "   \nZZZ\n");

    let zeros = assert_matches_oracle(&wrap(
        &["01  A PIC X(3) VALUE SPACES."],
        &["UNSTRING ZEROS DELIMITED BY \" \" INTO A.", "DISPLAY A.", "STOP RUN."],
    ));
    assert_eq!(zeros, "0  \n");

    let zeroes = assert_matches_oracle(&wrap(
        &["01  A PIC X(3) VALUE SPACES."],
        &["UNSTRING ZEROES DELIMITED BY \" \" INTO A.", "DISPLAY A.", "STOP RUN."],
    ));
    assert_eq!(zeroes, "0  \n");
}

#[test]
fn unstring_figurative_with_pointer() {
    // A figurative source under WITH POINTER p (p = 1) starts at index 0 over the
    // 1-char source " ". With no comma the whole char fills A (→ "   "), and the
    // resume pointer advances to one past the end: final cursor 2, clamped to len
    // 1 → 1, +1 → 2 ("02"). Both engines apply the same start offset and
    // write-back, so the compiled output pins to the oracle byte-for-byte.
    let out = assert_matches_oracle(&wrap(
        &["01  A PIC X(3) VALUE SPACES.", "01  P PIC 9(2) VALUE 1."],
        &[
            "UNSTRING SPACE DELIMITED BY \",\" INTO A WITH POINTER P.",
            "DISPLAY A.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "   \n02\n");
}

#[test]
fn unstring_figurative_delimiter_equals_source() {
    // The delimiter EQUALS the single source character: source " " with DELIMITED
    // BY " " splits at index 0 → the empty prefix fills A (→ spaces), the pointer
    // steps past the delimiter to the end, and the empty remainder fills B (→
    // spaces). Both receivers become all spaces; the two engines agree byte-for-
    // byte via assert_matches_oracle.
    let out = assert_matches_oracle(&wrap(
        &["01  A PIC X(3) VALUE \"ZZZ\".", "01  B PIC X(3) VALUE \"ZZZ\"."],
        &[
            "UNSTRING SPACE DELIMITED BY \" \" INTO A B.",
            "DISPLAY A.",
            "DISPLAY B.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "   \n   \n");
}

#[test]
fn unstring_figurative_all_ascii_characterization() {
    // SPACE (0x20) and ZERO (0x30) are inherently ASCII, and `Fig` = {Space, Zero}
    // is a CLOSED enum, so no non-ASCII figurative constant can ever reach the
    // source path — there is no diverging byte-vs-char case to construct here. The
    // separate pre-existing byte-vs-char chip is the non-ASCII string-LITERAL
    // source (see `unstring_non_ascii_literal_source_is_a_later_rung`). This case
    // simply pins an all-ASCII figurative source to byte-identity: ZERO → "0" with
    // no comma fills A reshaped to its X(4) width → "0   ".
    let out = assert_matches_oracle(&wrap(
        &["01  A PIC X(4) VALUE SPACES."],
        &["UNSTRING ZERO DELIMITED BY \",\" INTO A.", "DISPLAY A.", "STOP RUN."],
    ));
    assert_eq!(out, "0   \n");
}

// ---------------------------------------------------------------------------
// UNSTRING with a REFERENCE-MODIFIED source `base(start:len)`. This is a direct
// mirror of the literal-source rung: the ONLY thing that changes is the source
// character provider — the field text is the ref-mod slice of the base item
// (obtained through the SAME slice machinery DISPLAY / comparisons already use,
// so the source register is byte-identical between the oracle's `refmod_string`
// and the compiler's `ref_mod_slice`). The delimiter scan and receiver reshape
// are entirely unchanged, so every split/exhaustion rule matches the plain
// identifier source. A NUMERIC base under ref-mod stays a later rung (rejected
// by the shared slice helper on both engines).
// ---------------------------------------------------------------------------

#[test]
fn unstring_refmod_source_splits_into_receivers() {
    // The canonical case: source is the slice S(2:3) of "XA,BY" = "A,B", split on
    // "," into two receivers. The reference modification carves the field text out
    // of the middle of the base item; the split then proceeds exactly as for a
    // plain item source.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"XA,BY\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
        ],
        &[
            "UNSTRING S(2:3) DELIMITED BY \",\" INTO R1 R2.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \n");
}

#[test]
fn unstring_refmod_source_single_field_no_delimiter() {
    // The delimiter is ABSENT from the slice: S(2:3) of "HELLO" = "ELL" has no
    // comma, so the whole slice is one field, reshaped to the sole receiver's
    // width (X(5) → space-padded).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"HELLO\".", "01  R1 PIC X(5) VALUE SPACES."],
        &[
            "UNSTRING S(2:3) DELIMITED BY \",\" INTO R1.",
            "DISPLAY R1.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "ELL  \n");
}

#[test]
fn unstring_refmod_source_slice_at_start_of_base() {
    // A slice anchored at the START of the base — S(1:3) of "A,BCD" = "A,B" —
    // splits into two fields, proving the 1-based start position is honoured at
    // the very first character.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"A,BCD\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
        ],
        &[
            "UNSTRING S(1:3) DELIMITED BY \",\" INTO R1 R2.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \n");
}

#[test]
fn unstring_refmod_source_slice_at_end_of_base() {
    // A slice anchored at the END of the base — S(3:3) of "XYA,B" spans positions
    // 3..5 ("A,B"), i.e. start0+len = 2+3 = 5 = the item width exactly — splits
    // into two fields. This pins the upper slice bound at the item boundary.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"XYA,B\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
        ],
        &[
            "UNSTRING S(3:3) DELIMITED BY \",\" INTO R1 R2.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \n");
}

#[test]
fn unstring_refmod_source_with_computed_start_index() {
    // The start index is a DATA-NAME (`J` = 2), exercising the computed ref-mod
    // path (register-computed bounds) rather than the constant-folded literal
    // path. S(J:3) of "XA,BY" = "A,B" → two fields, identical to the literal-index
    // case above, so both slice paths agree with the oracle.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"XA,BY\".",
            "01  J  PIC 9  VALUE 2.",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
        ],
        &[
            "UNSTRING S(J:3) DELIMITED BY \",\" INTO R1 R2.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \n");
}

#[test]
fn unstring_refmod_source_exhausted_leaves_receiver_unchanged() {
    // The slice S(2:3) = "A,B" fills two receivers; the source is then exhausted,
    // so R3 keeps its prior VALUE "ZZZ" (NOT space-filled) — the same exhaustion
    // rule as the identifier/literal source, now driven by the slice characters.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"XA,BY\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  R3 PIC X(3) VALUE \"ZZZ\".",
        ],
        &[
            "UNSTRING S(2:3) DELIMITED BY \",\" INTO R1 R2 R3.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY R3.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \nZZZ\n");
}

#[test]
fn unstring_numeric_base_refmod_source_is_a_later_rung() {
    // A NUMERIC base item under reference modification is a later rung: the shared
    // slice helper (oracle `refmod_string`, compiler `ref_mod_slice`) rejects a
    // numeric base identically, so UNSTRING inherits that reject on BOTH engines.
    let src = wrap(
        &[
            "01  N  PIC 9(5) VALUE 12345.",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
        ],
        &["UNSTRING N(2:3) DELIMITED BY \",\" INTO R1 R2.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a numeric-base ref-mod source");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a numeric-base ref-mod source"
    );
}

#[test]
fn unstring_non_ascii_literal_source_is_a_later_rung() {
    // The oracle scans a literal source by CHARACTER while the compiler lowers it
    // to BYTE-based IIR string ops — the two agree only for ASCII (one byte per
    // char). A NON-ASCII string-literal source (here "café", whose 'é' is a
    // multi-byte character) is therefore deferred — rejected on BOTH engines so
    // they stay co-total — even though an ASCII literal source IS supported.
    let src = wrap(
        &["01  R1 PIC X(4) VALUE SPACES.", "01  R2 PIC X(4) VALUE SPACES."],
        &["UNSTRING \"café\" DELIMITED BY \",\" INTO R1 R2.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a non-ASCII literal source");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a non-ASCII literal source"
    );
}

#[test]
fn unstring_ascii_literal_source_still_works() {
    // The all-ASCII counterpart of the rejected non-ASCII case still splits and
    // reshapes exactly, byte-identical to the oracle — proving the guard is
    // scoped to non-ASCII bytes only.
    let out = assert_matches_oracle(&wrap(
        &["01  R1 PIC X(4) VALUE SPACES.", "01  R2 PIC X(4) VALUE SPACES."],
        &[
            "UNSTRING \"cafe,X\" DELIMITED BY \",\" INTO R1 R2.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "cafe\nX   \n");
}

// ---------------------------------------------------------------------------
// UNSTRING … WITH POINTER p — `p` (a `PIC 9(n)` unsigned integer) holds the
// 1-BASED character position at which the scan STARTS, and is UPDATED afterwards
// to one past the last character examined (`min(final_cursor, len) + 1`). An
// initial `p` outside `[1, len]` (either 0 or > len) is ISO's overflow: NO
// receiver is modified and `p` is left unchanged. Both engines apply the SAME
// start offset, the SAME write-back, and the SAME out-of-range rule, so each case
// pins the compiled JIT output to the tree-walk oracle byte-for-byte.
// ---------------------------------------------------------------------------

#[test]
fn unstring_pointer_one_equals_no_pointer() {
    // The correctness ANCHOR: `WITH POINTER p` with `p = 1` starts at index 0, so
    // the receivers must be IDENTICAL to the same statement WITHOUT the phrase. We
    // run both and assert the receiver lines coincide; the pointer version then
    // additionally writes the resume position back (final cursor 6 clamped to len
    // 5, +1 → 6 → "06").
    let with_ptr = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"a,b,c\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  R3 PIC X(3) VALUE SPACES.",
            "01  P  PIC 9(2) VALUE 1.",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2 R3 WITH POINTER P.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY R3.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    let no_ptr = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"a,b,c\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  R3 PIC X(3) VALUE SPACES.",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2 R3.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY R3.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(no_ptr, "a  \nb  \nc  \n");
    // p = 1 fills the SAME receivers, then appends the resume pointer "06".
    assert_eq!(with_ptr, format!("{no_ptr}06\n"));
}

#[test]
fn unstring_pointer_mid_string() {
    // The task's canonical example: source "a,b,c", p = 3 → start at 0-based index
    // 2 ("b,c"). R1="b", R2="c"; the scan's final cursor is 6, clamped to len 5,
    // +1 → the resume pointer is 6 ("06").
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"a,b,c\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  P  PIC 9(2) VALUE 3.",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2 WITH POINTER P.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "b  \nc  \n06\n");
}

#[test]
fn unstring_pointer_at_len_reads_final_char() {
    // p = len (= 5) is the last in-range value: start at 0-based index 4, the final
    // character "c". R1="c"; final cursor 6 clamped to 5, +1 → 6 ("06").
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"a,b,c\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  P  PIC 9(2) VALUE 5.",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 WITH POINTER P.",
            "DISPLAY R1.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "c  \n06\n");
}

#[test]
fn unstring_pointer_writeback_after_delimiter_terminated_field() {
    // A field terminated by a DELIMITER (not end-of-source): "AA/BB" split on "/"
    // with p = 1 fills F1="AA"; the scan stops at the delimiter (index 2), so the
    // cursor advances past it to 3 — NOT clamped (3 < len 5) — and the resume
    // pointer is 3 + 1 = 4 ("04"). This pins the non-clamped write-back path,
    // distinct from the end-of-source cases above.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"AA/BB\".",
            "01  F1 PIC X(2) VALUE SPACES.",
            "01  P  PIC 9(2) VALUE 1.",
        ],
        &[
            "UNSTRING S DELIMITED BY \"/\" INTO F1 WITH POINTER P.",
            "DISPLAY F1.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "AA\n04\n");
}

#[test]
fn unstring_pointer_source_exhausted_before_receivers_filled() {
    // With a pointer, the exhaustion rule is unchanged: "A,B" (len 3), p = 1 fills
    // R1="A" and R2="B", then the source is exhausted so R3 keeps its prior VALUE
    // "ZZZ" (NOT space-filled). The cursor ends at 4, clamped to len 3, +1 → the
    // resume pointer is 4 ("04").
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(3) VALUE \"A,B\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  R3 PIC X(3) VALUE \"ZZZ\".",
            "01  P  PIC 9(2) VALUE 1.",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2 R3 WITH POINTER P.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY R3.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \nZZZ\n04\n");
}

#[test]
fn unstring_pointer_past_end_is_overflow_no_moves() {
    // p = len + 1 (= 6 > 5) is out of range: ISO overflow ⇒ NO receiver is modified
    // (R1 keeps "ZZZ") and the pointer is left UNCHANGED (stays 6 → "06"). Both
    // engines skip the whole operation identically.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"a,b,c\".",
            "01  R1 PIC X(3) VALUE \"ZZZ\".",
            "01  P  PIC 9(2) VALUE 6.",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 WITH POINTER P.",
            "DISPLAY R1.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "ZZZ\n06\n");
}

#[test]
fn unstring_pointer_zero_is_overflow_no_moves() {
    // p = 0 would make the 0-based start −1 (underflow); it is out of range, so ISO
    // overflow ⇒ no moves and the pointer is left unchanged (stays 0 → "00"). This
    // is the guard that keeps the `usize`/`i64` start computation from underflowing.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"a,b,c\".",
            "01  R1 PIC X(3) VALUE \"ZZZ\".",
            "01  P  PIC 9(2) VALUE 0.",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 WITH POINTER P.",
            "DISPLAY R1.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "ZZZ\n00\n");
}

#[test]
fn unstring_pointer_huge_is_overflow_no_moves() {
    // A far-out-of-range pointer (9999 ≫ len 5) is still just "> len": no moves, the
    // pointer unchanged (stays 9999). Proves the guard uses the source length, not a
    // fixed bound.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"a,b,c\".",
            "01  R1 PIC X(3) VALUE \"ZZZ\".",
            "01  P  PIC 9(4) VALUE 9999.",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 WITH POINTER P.",
            "DISPLAY R1.",
            "DISPLAY P.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "ZZZ\n9999\n");
}

#[test]
fn unstring_pointer_fuzz_across_full_range() {
    // Sweep the initial pointer across the WHOLE range `[0, len + 2]` = `[0, 7]` for
    // source "a,b,c" (len 5). For every value — the two out-of-range ends (0, 6, 7)
    // and every in-range start (1..=5) — the compiled JIT output must be byte-
    // identical to the oracle (receivers AND the written-back pointer). This is the
    // co-totality proof: both engines agree for EVERY initial pointer value.
    for pstart in 0..=7u32 {
        let data = [
            "01  S  PIC X(5) VALUE \"a,b,c\".".to_string(),
            "01  R1 PIC X(3) VALUE \"ZZZ\".".to_string(),
            "01  R2 PIC X(3) VALUE \"ZZZ\".".to_string(),
            format!("01  P  PIC 9(2) VALUE {pstart}."),
        ];
        let data_refs: Vec<&str> = data.iter().map(String::as_str).collect();
        // assert_matches_oracle panics with a clear message on any divergence.
        assert_matches_oracle(&wrap(
            &data_refs,
            &[
                "UNSTRING S DELIMITED BY \",\" INTO R1 R2 WITH POINTER P.",
                "DISPLAY R1.",
                "DISPLAY R2.",
                "DISPLAY P.",
                "STOP RUN.",
            ],
        ));
    }
}

#[test]
fn unstring_pointer_signed_is_a_later_rung() {
    // The pointer must be an UNSIGNED integer. A signed pointer (`PIC S9`) is a
    // clean later rung, rejected on BOTH engines (the compiler at build time, the
    // oracle at exec time).
    let src = wrap(
        &[
            "01  S  PIC X(5) VALUE \"a,b,c\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  P  PIC S9(2) VALUE 1.",
        ],
        &["UNSTRING S DELIMITED BY \",\" INTO R1 WITH POINTER P.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a signed pointer");
    assert!(compile_source(&src, "e2e").is_err(), "compiler must reject a signed pointer");
}

#[test]
fn unstring_pointer_fractional_is_a_later_rung() {
    // A fractional pointer (`PIC 9V9`) is not an integer position — a later rung,
    // rejected identically on both engines.
    let src = wrap(
        &[
            "01  S  PIC X(5) VALUE \"a,b,c\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  P  PIC 9V9 VALUE 1.",
        ],
        &["UNSTRING S DELIMITED BY \",\" INTO R1 WITH POINTER P.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a fractional pointer");
    assert!(compile_source(&src, "e2e").is_err(), "compiler must reject a fractional pointer");
}

#[test]
fn unstring_pointer_non_numeric_is_a_later_rung() {
    // A non-numeric pointer (`PIC X`) has no integer position — a later rung,
    // rejected identically on both engines.
    let src = wrap(
        &[
            "01  S  PIC X(5) VALUE \"a,b,c\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  P  PIC X(2) VALUE \"12\".",
        ],
        &["UNSTRING S DELIMITED BY \",\" INTO R1 WITH POINTER P.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a non-numeric pointer");
    assert!(compile_source(&src, "e2e").is_err(), "compiler must reject a non-numeric pointer");
}

// ---------------------------------------------------------------------------
// UNSTRING … ON OVERFLOW / NOT ON OVERFLOW — the DIRECT sibling of STRING's
// overflow clauses. Overflow fires when every receiver is filled but the source
// is NOT exhausted (more delimited fields remain) OR the initial WITH POINTER
// value is out of range. The overflow BOOLEAN is the identical comparison on both
// engines (`p <= len` after the scan, or `true` for an out-of-range pointer); each
// case pins the compiled JIT output to the oracle byte-for-byte via
// `assert_matches_oracle`, plus the exact expected bytes.
// ---------------------------------------------------------------------------

#[test]
fn unstring_overflow_fires_more_fields_than_receivers() {
    // (a) "A,B,C" (three fields) into TWO receivers: R1="A", R2="B", then the source
    // is NOT exhausted (final cursor p = 4 ≤ len 5) ⇒ overflow. The ON OVERFLOW body
    // writes "YES"; "C" is dropped (no third receiver).
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"A,B,C\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2",
            "    ON OVERFLOW MOVE \"YES\" TO F",
            "    NOT ON OVERFLOW MOVE \"NON\" TO F.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \nYES\n");
}

#[test]
fn unstring_no_overflow_runs_not_clause() {
    // (b) "A,B" (exactly two fields) into TWO receivers: R1="A", R2="B", and the
    // source IS exhausted (the last field ran to end-of-source, p = 4 > len 3) ⇒ NO
    // overflow, so the NOT ON OVERFLOW body runs, writing "NON".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(3) VALUE \"A,B\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2",
            "    ON OVERFLOW MOVE \"YES\" TO F",
            "    NOT ON OVERFLOW MOVE \"NON\" TO F.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \nNON\n");
}

#[test]
fn unstring_trailing_delimiter_fires_overflow() {
    // (c) Trailing delimiter "A,B," into TWO receivers: R1="A", R2="B", and the
    // cursor stops AT the trailing delimiter (p = 4 = len 4) — an empty field still
    // remains ⇒ overflow (`p <= len`). The `p == len` boundary is exactly the
    // trailing-delimiter case both engines must agree on. ON OVERFLOW ⇒ "YES".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(4) VALUE \"A,B,\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2",
            "    ON OVERFLOW MOVE \"YES\" TO F",
            "    NOT ON OVERFLOW MOVE \"NON\" TO F.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \nYES\n");
}

#[test]
fn unstring_on_overflow_only_present() {
    // (d) ON OVERFLOW present, NOT ON OVERFLOW absent. When overflow fires the flag
    // flips to "YES"; when it does not, the flag is UNCHANGED (no NOT clause to run).
    let fires = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"A,B,C\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2",
            "    ON OVERFLOW MOVE \"YES\" TO F.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(fires, "A  \nB  \nYES\n");
    let quiet = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(3) VALUE \"A,B\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2",
            "    ON OVERFLOW MOVE \"YES\" TO F.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    // No overflow and no NOT clause ⇒ F keeps its initial "no ".
    assert_eq!(quiet, "A  \nB  \nno \n");
}

#[test]
fn unstring_not_on_overflow_only_present() {
    // (e) NOT ON OVERFLOW present, ON OVERFLOW absent, source exhausted ⇒ the NOT
    // body runs (jmp_if_false over an EMPTY on-branch), writing "NON".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(3) VALUE \"A,B\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2",
            "    NOT ON OVERFLOW MOVE \"NON\" TO F.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \nNON\n");
}

#[test]
fn unstring_pointer_zero_out_of_range_fires_on_overflow() {
    // (f) WITH POINTER out of range, p = 0. No data movement, pointer UNCHANGED, but
    // ON OVERFLOW now runs (the behaviour change this rung introduces). R1 keeps
    // "ZZZ", P stays "00", F becomes "YES".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"a,b,c\".",
            "01  R1 PIC X(3) VALUE \"ZZZ\".",
            "01  P  PIC 9(2) VALUE 0.",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 WITH POINTER P",
            "    ON OVERFLOW MOVE \"YES\" TO F",
            "    NOT ON OVERFLOW MOVE \"NON\" TO F.",
            "DISPLAY R1.",
            "DISPLAY P.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "ZZZ\n00\nYES\n");
}

#[test]
fn unstring_pointer_past_end_out_of_range_fires_on_overflow() {
    // (f cont.) WITH POINTER out of range, p > len. p = 6 into a 5-char source: no
    // movement, P unchanged ("06"), ON OVERFLOW runs ⇒ F = "YES".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"a,b,c\".",
            "01  R1 PIC X(3) VALUE \"ZZZ\".",
            "01  P  PIC 9(2) VALUE 6.",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 WITH POINTER P",
            "    ON OVERFLOW MOVE \"YES\" TO F",
            "    NOT ON OVERFLOW MOVE \"NON\" TO F.",
            "DISPLAY R1.",
            "DISPLAY P.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "ZZZ\n06\nYES\n");
}

#[test]
fn unstring_pointer_in_range_fields_remain_fires_overflow_with_writeback() {
    // (g) WITH POINTER p = 1, ONE receiver, source "a,b,c" has more fields left after
    // R1="a" ⇒ overflow. The write-back STILL happens before the imperative: the
    // cursor stops at the delimiter (index 1), advances to 2, so P := 2 + 1 = 3
    // ("03"). ON OVERFLOW ⇒ F = "YES". Pins overflow + a correct write-back together.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"a,b,c\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  P  PIC 9(2) VALUE 1.",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 WITH POINTER P",
            "    ON OVERFLOW MOVE \"YES\" TO F",
            "    NOT ON OVERFLOW MOVE \"NON\" TO F.",
            "DISPLAY R1.",
            "DISPLAY P.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "a  \n03\nYES\n");
}

#[test]
fn unstring_pointer_anchor_no_overflow_runs_not_clause() {
    // (h) WITH POINTER p = 1 anchor, "a,b" into TWO receivers exhausts the source ⇒
    // NO overflow, so the NOT ON OVERFLOW body runs. R1="a", R2="b", the cursor ends
    // at 4 clamped to len 3, P := 3 + 1 = 4 ("04"), F = "NON".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(3) VALUE \"a,b\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  P  PIC 9(2) VALUE 1.",
            "01  F  PIC X(3) VALUE \"no \".",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2 WITH POINTER P",
            "    ON OVERFLOW MOVE \"YES\" TO F",
            "    NOT ON OVERFLOW MOVE \"NON\" TO F.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY P.",
            "DISPLAY F.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "a  \nb  \n04\nNON\n");
}

#[test]
fn unstring_on_overflow_body_propagates_control_flow() {
    // (i) The ON OVERFLOW body contains a GO TO, proving the handler's `Flow` unwinds
    // out of exec_unstring / the emitted block. Overflow fires (three fields, two
    // receivers), GO TO jumps to OVF, which DISPLAYs and stops — the "AFTER" line
    // after UNSTRING is never reached.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"A,B,C\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2 ON OVERFLOW GO TO OVF.",
            "DISPLAY \"AFTER\".",
            "STOP RUN.",
            "OVF.",
            "DISPLAY \"JUMPED\".",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "JUMPED\n");
}

#[test]
fn unstring_with_no_overflow_clauses_still_works_regression() {
    // (j) A plain UNSTRING with NEITHER clause lowers exactly as before this rung —
    // no overflow flag, no branch skeleton. Regression anchor for the empty-clause
    // path: "A,B,C" into two receivers still fills R1="A", R2="B" and drops "C".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"A,B,C\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
        ],
        &[
            "UNSTRING S DELIMITED BY \",\" INTO R1 R2.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \n");
}

// ---------------------------------------------------------------------------
// INSPECT … TALLYING — count the (non-overlapping, left-to-right) occurrences of
// a single-character delimiter in an alphanumeric source and ADD the count to an
// integer counter (INSPECT adds; it does not clear the counter first). Each case
// pins the compiled JIT output to the tree-walk oracle byte-for-byte.
// ---------------------------------------------------------------------------

#[test]
fn inspect_counts_all_occurrences_of_a_char() {
    // "BANANA" has three A's → C = 0 + 3 = 3.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"BANANA\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"A\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "003\n");
}

#[test]
fn inspect_zero_occurrences_leaves_the_counter() {
    // No 'Z' in "HELLO" → C stays 0.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"HELLO\".", "01  C  PIC 9(2) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"Z\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "00\n");
}

#[test]
fn inspect_adds_to_a_nonzero_counter_not_replaces_it() {
    // C starts at 5; "MISSISSIPPI" has four S's → C = 5 + 4 = 9 (proves ADD, not
    // a fresh assignment).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(11) VALUE \"MISSISSIPPI\".", "01  C  PIC 9(3) VALUE 5."],
        &["INSPECT S TALLYING C FOR ALL \"S\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "009\n");
}

#[test]
fn inspect_delimiter_is_a_pic_x1_item() {
    // The delimiter may be a PIC X(1) item (its single stored character), read at
    // run time: three ';' in "A;B;C;D".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(7) VALUE \"A;B;C;D\".",
            "01  DL PIC X(1) VALUE \";\".",
            "01  C  PIC 9(2) VALUE 0.",
        ],
        &["INSPECT S TALLYING C FOR ALL DL.", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "03\n");
}

#[test]
fn inspect_every_character_matches() {
    // Every one of "AAAA" is an 'A' → C = 4.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"AAAA\".", "01  C  PIC 9(2) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"A\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "04\n");
}

#[test]
fn inspect_delimiter_at_both_ends() {
    // "*HI*" — the delimiter at both boundaries is still counted (2), and the
    // optional END-INSPECT terminator parses.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"*HI*\".", "01  C  PIC 9(2) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"*\" END-INSPECT.", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "02\n");
}

// INSPECT … TALLYING … FOR ALL … {BEFORE|AFTER} x — count only within the sub-
// slice of the source bounded by the FIRST occurrence of the single region
// delimiter `x`. BEFORE counts left of `x`; AFTER counts right of it. The ISO
// not-found asymmetry is the crux: BEFORE with `x` absent counts the WHOLE source,
// AFTER with `x` absent counts NOTHING. Each case pins the compiled window scan to
// the oracle byte-for-byte.

#[test]
fn inspect_before_counts_only_left_of_the_delimiter() {
    // "AB0CD0" — BEFORE "C" restricts to "AB0" (indices 0..3), which holds ONE '0'.
    // (The trailing '0' after 'C' is outside the region.)
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"AB0CD0\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"C\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "001\n");
}

#[test]
fn inspect_after_counts_only_right_of_the_delimiter() {
    // Same source — AFTER "C" restricts to "D0" (indices 4..6), which holds ONE '0'.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"AB0CD0\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"0\" AFTER \"C\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "001\n");
}

#[test]
fn inspect_before_absent_delimiter_counts_the_whole_source() {
    // BEFORE "Z" with no 'Z' present → the region is the ENTIRE source, so BOTH '0's
    // are counted (2). This is the BEFORE not-found rule.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"AB0CD0\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"Z\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "002\n");
}

#[test]
fn inspect_after_absent_delimiter_counts_nothing() {
    // AFTER "Z" with no 'Z' present → the region is EMPTY, so NOTHING is counted (0).
    // This is the AFTER not-found rule — the asymmetric partner of the case above.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"AB0CD0\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"0\" AFTER \"Z\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "000\n");
}

#[test]
fn inspect_after_delimiter_at_position_zero() {
    // The region delimiter is the FIRST character: AFTER "X" in "X00" → region "00"
    // (indices 1..3) → two '0's. Pins the `start = fidx + 1` edge.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"X00\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"0\" AFTER \"X\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "002\n");
}

#[test]
fn inspect_before_delimiter_at_last_position() {
    // The region delimiter is the LAST character: BEFORE "X" in "00X" → region "00"
    // (indices 0..2) → two '0's. Pins the `end = fidx` prefix edge.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"00X\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"X\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "002\n");
}

#[test]
fn inspect_region_delimiter_equals_tally_delimiter() {
    // The tally delimiter and the region delimiter are the SAME char: AFTER "A" in
    // "ABABA" → the FIRST 'A' (index 0) bounds the region to "BABA" (indices 1..5),
    // where two more 'A's remain → 2. (The bounding 'A' itself is excluded.)
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"ABABA\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"A\" AFTER \"A\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "002\n");
}

#[test]
fn inspect_before_delimiter_at_position_zero_is_an_empty_region() {
    // BEFORE "A" in "A00" → the first 'A' is at index 0, so the region is [0, 0) —
    // EMPTY — and nothing is counted (0). Pins the empty-prefix edge.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"A00\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"A\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "000\n");
}

#[test]
fn inspect_region_adds_to_a_nonzero_counter() {
    // INSPECT ADDs to the counter; a region does not change that. C starts at 5;
    // BEFORE "C" counts one '0' → C = 5 + 1 = 6.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"AB0CD0\".", "01  C  PIC 9(3) VALUE 5."],
        &["INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"C\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "006\n");
}

#[test]
fn inspect_region_delimiter_is_a_pic_x1_item() {
    // The region delimiter may itself be a PIC X(1) item, read at run time: BEFORE
    // the item DL (";") in "0;0;0" restricts to "0" (index 0..1) → one '0'.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"0;0;0\".",
            "01  DL PIC X(1) VALUE \";\".",
            "01  C  PIC 9(3) VALUE 0.",
        ],
        &["INSPECT S TALLYING C FOR ALL \"0\" BEFORE DL.", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "001\n");
}

// ---------------------------------------------------------------------------
// INSPECT … TALLYING … FOR CHARACTERS [{BEFORE|AFTER} x] — the "count every
// position" form. Instead of matching a delimiter, it ADDs the NUMBER OF CHARACTER
// POSITIONS in the region window to the counter: with no region that is length(S),
// with a `{BEFORE|AFTER} x` region it is the window length (`end - start`) of the
// SAME window `FOR ALL` uses, so it inherits the identical BEFORE→whole /
// AFTER→empty not-found asymmetry. INSPECT ADDs (does not clear). Each case pins the
// compiled JIT output to the tree-walk oracle byte-for-byte.
// ---------------------------------------------------------------------------

#[test]
fn inspect_characters_no_region_counts_the_full_length() {
    // No region → C += length("BANANA") = 6.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"BANANA\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR CHARACTERS.", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "006\n");
}

#[test]
fn inspect_characters_before_present_counts_positions_before_x() {
    // "AB0CD0" — first "C" is at index 3, so BEFORE "C" is the window [0, 3) = "AB0",
    // whose length is 3 (every position counts, not just delimiter matches).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"AB0CD0\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR CHARACTERS BEFORE \"C\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "003\n");
}

#[test]
fn inspect_characters_after_present_counts_positions_after_x() {
    // Same source; first "C" is at index 3, so AFTER "C" is the window [4, 6) = "D0",
    // whose length is 2.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"AB0CD0\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR CHARACTERS AFTER \"C\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "002\n");
}

#[test]
fn inspect_characters_before_absent_delimiter_counts_the_whole_source() {
    // BEFORE "Z" with no 'Z' in "HELLO" → not-found ⇒ WHOLE source, so C += 5.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"HELLO\".", "01  C  PIC 9(2) VALUE 0."],
        &["INSPECT S TALLYING C FOR CHARACTERS BEFORE \"Z\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "05\n");
}

#[test]
fn inspect_characters_after_absent_delimiter_counts_nothing() {
    // AFTER "Z" with no 'Z' in "HELLO" → not-found ⇒ EMPTY window, so C += 0.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"HELLO\".", "01  C  PIC 9(2) VALUE 0."],
        &["INSPECT S TALLYING C FOR CHARACTERS AFTER \"Z\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "00\n");
}

#[test]
fn inspect_characters_adds_to_a_nonzero_counter_not_replaces_it() {
    // C starts at 5; "ABCD" has length 4 → C = 5 + 4 = 9 (proves ADD, not assign).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"ABCD\".", "01  C  PIC 9(3) VALUE 5."],
        &["INSPECT S TALLYING C FOR CHARACTERS.", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "009\n");
}

#[test]
fn inspect_characters_short_source_counts_one() {
    // A single-character source → C += 1 (the short/near-empty edge).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(1) VALUE \"Q\".", "01  C  PIC 9(2) VALUE 0."],
        &["INSPECT S TALLYING C FOR CHARACTERS.", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "01\n");
}

#[test]
fn inspect_characters_alongside_for_all_leaves_the_all_path_unaffected() {
    // One INSPECT counts the A's with the ordinary `FOR ALL` path (CA = 3), a second
    // counts every position with `FOR CHARACTERS` (CC = 6). Proves the two forms
    // coexist and the ALL lowering is untouched by the CHARACTERS addition.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(6) VALUE \"BANANA\".",
            "01  CA PIC 9(3) VALUE 0.",
            "01  CC PIC 9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING CA FOR ALL \"A\".",
            "INSPECT S TALLYING CC FOR CHARACTERS.",
            "DISPLAY CA.",
            "DISPLAY CC.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "003\n006\n");
}

#[test]
fn inspect_characters_counts_bytes_not_codepoints_on_non_ascii_source() {
    // `FOR CHARACTERS` returns a LENGTH, so it is the first tally form that would
    // surface the oracle's `char` basis vs the compiler's BYTE basis directly. COBOL
    // `PIC X` positions are BYTES, so both engines must count BYTES. `PIC X(5) VALUE
    // "café"` right-pads the 4-character value to 5 CHARACTER positions — "café " (a
    // trailing space) — whose BYTE length is 6 (é is a 2-byte UTF-8 sequence, the other
    // four positions are one byte each). `assert_matches_oracle` asserts the two engines
    // AGREE (it panics on any divergence), so this pins the byte-count fix: the oracle
    // sums `len_utf8()` over the window to match the compiler's `str_len`, and both
    // report 6 — a char-based count would have wrongly reported 5 on the oracle.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"café\".", "01  C  PIC 9(2) VALUE 0."],
        &["INSPECT S TALLYING C FOR CHARACTERS.", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "06\n");
}

// INSPECT … TALLYING … FOR LEADING … {BEFORE|AFTER} x — the STANDALONE leading-run
// count restricted to a `{BEFORE|AFTER}` sub-region. The crux of this rung: the
// leading run is anchored at the WINDOW START, not source position 0. `FOR LEADING`
// counts only the maximal run of the delimiter that begins AT the window's start
// index and stops at the first non-matching char INSIDE the window (or the window
// end). With `AFTER x` and `x` absent the window is empty (count 0); with `BEFORE x`
// and `x` absent the window is the whole source. `assert_matches_oracle` re-checks
// JIT == oracle for every case.

#[test]
fn inspect_leading_after_anchors_the_run_at_the_window_start() {
    // "aaXaab" AFTER "X" narrows to "aab" (indices 3..6). The leading run of 'a'
    // there is 2 — the two a's right after the X. The "aa" BEFORE the X is outside
    // the window and must NOT contribute (that is the whole anchoring point).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"aaXaab\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR LEADING \"a\" AFTER \"X\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "002\n");
}

#[test]
fn inspect_leading_after_delimiter_at_position_zero() {
    // Delimiter is the FIRST char: "Xaab" AFTER "X" → window "aab" (1..4), leading
    // 'a' run = 2. Pins the `start = first + 1` edge for the leading scan.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"Xaab\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR LEADING \"a\" AFTER \"X\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "002\n");
}

#[test]
fn inspect_leading_after_window_starts_on_a_mismatch() {
    // "aaXbb" AFTER "X" → window "bb" (3..5). The window's FIRST char is 'b', not the
    // delimiter 'a', so the leading run is 0 even though a's appear before the X.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aaXbb\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR LEADING \"a\" AFTER \"X\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "000\n");
}

#[test]
fn inspect_leading_before_counts_the_prefix_run() {
    // "aaXaa" BEFORE "X" → window "aa" (0..2), leading 'a' run = 2. The trailing "aa"
    // after the X is outside the BEFORE window.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aaXaa\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR LEADING \"a\" BEFORE \"X\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "002\n");
}

#[test]
fn inspect_leading_before_delimiter_at_last_position() {
    // Delimiter is the LAST char: "aaX" BEFORE "X" → window "aa" (0..2), leading run
    // = 2. Pins the `end = first` prefix edge for the leading scan.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"aaX\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR LEADING \"a\" BEFORE \"X\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "002\n");
}

#[test]
fn inspect_leading_after_absent_delimiter_is_an_empty_window() {
    // AFTER "Z" with no 'Z' present → the window is EMPTY, so the leading count is 0
    // (the ISO not-found asymmetry on the AFTER side).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"aaXaab\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR LEADING \"a\" AFTER \"Z\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "000\n");
}

#[test]
fn inspect_leading_before_absent_delimiter_is_the_whole_source() {
    // BEFORE "Z" with no 'Z' present → the window is the WHOLE source "aaXaa", where
    // the leading 'a' run (from position 0) is 2 (stops at the X). The BEFORE
    // not-found rule, the asymmetric partner of the AFTER case above.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aaXaa\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR LEADING \"a\" BEFORE \"Z\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "002\n");
}

#[test]
fn inspect_leading_region_delimiter_equals_tally_delimiter() {
    // Region delimiter == tally delimiter: "aaXaa" AFTER "a" bounds on the FIRST 'a'
    // (index 0), window "aXaa" (1..5). The leading 'a' run there is 1 (position 1 is
    // 'a', position 2 is 'X'). The bounding 'a' itself is excluded.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aaXaa\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR LEADING \"a\" AFTER \"a\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "001\n");
}

#[test]
fn inspect_leading_region_adds_to_a_nonzero_counter() {
    // INSPECT ADDs to the counter; the window does not change that. C starts at 5;
    // AFTER "X" on "aaXaab" counts 2 leading a's → C = 5 + 2 = 7.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"aaXaab\".", "01  C  PIC 9(3) VALUE 5."],
        &["INSPECT S TALLYING C FOR LEADING \"a\" AFTER \"X\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "007\n");
}

#[test]
fn inspect_multi_char_region_delimiter_is_a_later_rung() {
    // A MULTI-character region delimiter is deferred — rejected on both engines, just
    // like a multi-character tally delimiter (the oracle rejects at exec, the
    // compiler at emit, both via the single-delimiter check).
    let src = wrap(
        &["01  S  PIC X(6) VALUE \"AB0CD0\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"CD\".", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a multi-char region delimiter");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a multi-char region delimiter"
    );
}

// INSPECT … TALLYING … FOR LEADING — count only the run of CONSECUTIVE delimiters
// at the START of the source, stopping at the first non-match (contrast FOR ALL,
// which counts every occurrence). Each case pins the compiled leading-run scan to
// the oracle byte-for-byte.

#[test]
fn inspect_leading_counts_the_leading_run() {
    // "000123" — three leading '0's, stop at '1' → C = 3. FOR ALL agrees here
    // (there are no other '0's), so both forms give 3 on this source.
    let lead = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"000123\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR LEADING \"0\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(lead, "003\n");

    let all = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"000123\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"0\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(all, "003\n");
}

#[test]
fn inspect_leading_stops_at_the_first_non_match() {
    // "120003" — the first character is '1', not '0', so the leading run is empty →
    // C = 0. (FOR ALL on the same source would count all three '0's = 3; LEADING's 0
    // is what distinguishes the two forms.)
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"120003\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR LEADING \"0\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "000\n");
}

#[test]
fn inspect_leading_all_characters_match() {
    // "0000" — every character is the delimiter, so the leading run is the whole
    // source → C = 4.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"0000\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR LEADING \"0\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "004\n");
}

#[test]
fn inspect_leading_blank_source_counts_zero() {
    // A blank PIC X(3) (all spaces) has no leading '0' → C = 0.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3).", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR LEADING \"0\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "000\n");
}

#[test]
fn inspect_leading_delimiter_is_a_pic_x1_item() {
    // The LEADING delimiter may be a PIC X(1) item, read at run time: two leading
    // 'A's in "AAB" → C = 2.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(3) VALUE \"AAB\".",
            "01  DL PIC X(1) VALUE \"A\".",
            "01  C  PIC 9(2) VALUE 0.",
        ],
        &["INSPECT S TALLYING C FOR LEADING DL.", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "02\n");
}

#[test]
fn inspect_leading_adds_to_a_nonzero_counter() {
    // INSPECT adds; it does not clear. C starts at 5, three leading '0's in "000X"
    // → C = 5 + 3 = 8.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"000X\".", "01  C  PIC 9(3) VALUE 5."],
        &["INSPECT S TALLYING C FOR LEADING \"0\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "008\n");
}

// INSPECT … REPLACING ALL x BY y — replace EVERY occurrence of the single
// character `x` in the alphanumeric source with the single character `y`, in
// place (same width). Each case pins the compiled per-position rebuild to the
// oracle's map, byte-for-byte.

#[test]
fn inspect_replacing_maps_a_repeated_char() {
    // "ABABA" with A→X → "XBXBX".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"ABABA\"."],
        &["INSPECT S REPLACING ALL \"A\" BY \"X\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "XBXBX\n");
}

#[test]
fn inspect_replacing_absent_char_leaves_source_unchanged() {
    // 'Z' never occurs in "HELLO" → the source is untouched.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"HELLO\"."],
        &["INSPECT S REPLACING ALL \"Z\" BY \"Q\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "HELLO\n");
}

#[test]
fn inspect_replacing_every_character() {
    // Every one of "AAAA" is an 'A' → "XXXX".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"AAAA\"."],
        &["INSPECT S REPLACING ALL \"A\" BY \"X\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "XXXX\n");
}

#[test]
fn inspect_replacing_search_and_replacement_are_pic_x1_items() {
    // Both the search and the replacement come from PIC X(1) items: O→0 in
    // "MOON" → "M00N".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(4) VALUE \"MOON\".",
            "01  X  PIC X(1) VALUE \"O\".",
            "01  Y  PIC X(1) VALUE \"0\".",
        ],
        &["INSPECT S REPLACING ALL X BY Y.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "M00N\n");
}

#[test]
fn inspect_replacing_char_at_both_ends() {
    // "*HI*" with *→- → "-HI-" (a match at both boundaries is replaced).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"*HI*\"."],
        &["INSPECT S REPLACING ALL \"*\" BY \"-\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "-HI-\n");
}

#[test]
fn inspect_replacing_end_inspect_terminator_parses() {
    // The optional END-INSPECT terminator parses; "BANANA" with A→o → "BoNoNo".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"BANANA\"."],
        &["INSPECT S REPLACING ALL \"A\" BY \"o\" END-INSPECT.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "BoNoNo\n");
}

// INSPECT … REPLACING LEADING x BY y — replace only the run of CONSECUTIVE `x`
// characters at the START of the source, stopping at the first non-`x`; positions
// after that first gap are left unchanged even if they equal `x`. The compiled
// unroll threads a runtime `active` flag (an extra `and` per position) and must
// match the oracle's stateful `in_run` map byte-for-byte.

#[test]
fn inspect_replacing_leading_replaces_the_leading_run() {
    // "000123" with LEADING 0→* → "***123": the three leading zeros are replaced,
    // the digits after are kept.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"000123\"."],
        &["INSPECT S REPLACING LEADING \"0\" BY \"*\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "***123\n");
}

#[test]
fn inspect_replacing_leading_stops_at_first_gap() {
    // "00X00" with LEADING 0→* → "**X00": the run stops at "X", so the trailing
    // "00" is NOT replaced. Contrast REPLACING ALL below, which replaces both runs.
    let lead = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"00X00\"."],
        &["INSPECT S REPLACING LEADING \"0\" BY \"*\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(lead, "**X00\n");

    // Same source, REPLACING ALL → "**X**": every "0" is replaced. The two forms
    // diverge exactly where the leading run ends.
    let all = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"00X00\"."],
        &["INSPECT S REPLACING ALL \"0\" BY \"*\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(all, "**X**\n");
}

#[test]
fn inspect_replacing_leading_no_run_leaves_source_unchanged() {
    // "120003" — the first character is not "0", so there is no leading run and the
    // source is untouched (even though interior "0"s exist).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"120003\"."],
        &["INSPECT S REPLACING LEADING \"0\" BY \"*\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "120003\n");
}

#[test]
fn inspect_replacing_leading_every_character() {
    // "0000" is all leading "0"s → the whole field becomes "****".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"0000\"."],
        &["INSPECT S REPLACING LEADING \"0\" BY \"*\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "****\n");
}

#[test]
fn inspect_replacing_leading_blank_source_unchanged() {
    // A blank PIC X(3) has no leading "0" run — the three spaces are unchanged.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3)."],
        &["INSPECT S REPLACING LEADING \"0\" BY \"*\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "   \n");
}

#[test]
fn inspect_replacing_leading_search_and_replacement_are_pic_x1_items() {
    // The search and replacement can be PIC X(1) items, not just literals:
    // LEADING 0→* on "000123" → "***123".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(6) VALUE \"000123\".",
            "01  X  PIC X(1) VALUE \"0\".",
            "01  Y  PIC X(1) VALUE \"*\".",
        ],
        &["INSPECT S REPLACING LEADING X BY Y.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "***123\n");
}

// INSPECT … REPLACING CHARACTERS BY x — overwrite EVERY position of the source with
// the single replacement char `x` (no region this rung): the WHOLE field becomes
// `x`s, its width unchanged. Both engines compute the fill on a BYTE basis so a
// non-ASCII source stays co-total (the oracle's `move_into` re-pads/truncates to the
// picture's CHAR size, exactly the compiler's `width`-many fill).
// ---------------------------------------------------------------------------

#[test]
fn inspect_replacing_characters_fills_the_whole_field() {
    // "ABABA" → REPLACING CHARACTERS BY "X" → "XXXXX": every position overwritten.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"ABABA\"."],
        &["INSPECT S REPLACING CHARACTERS BY \"X\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "XXXXX\n");
}

#[test]
fn inspect_replacing_characters_overwrites_spaces_and_mixed_content() {
    // A field with embedded spaces and mixed content is FULLY overwritten — even the
    // blanks become the replacement char. "A B C" (5 chars) → "-----".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"A B C\"."],
        &["INSPECT S REPLACING CHARACTERS BY \"-\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "-----\n");
}

#[test]
fn inspect_replacing_characters_replacement_is_a_pic_x1_item() {
    // The replacement `x` can be a PIC X(1) DATA ITEM, not just a literal:
    // "hello" filled with the item R = "*" → "*****".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"hello\".", "01  R  PIC X(1) VALUE \"*\"."],
        &["INSPECT S REPLACING CHARACTERS BY R.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "*****\n");
}

#[test]
fn inspect_replacing_characters_non_ascii_source_is_byte_co_total() {
    // The byte-basis regression. `PIC X(5) VALUE "café"` stores "café " (padded to
    // 5 CHARS = 6 BYTES). REPLACING CHARACTERS BY "Z" fills the field: the oracle
    // builds n = 6 (BYTE-length) copies then `move_into` caps to the picture's 5
    // CHARS → "ZZZZZ"; the compiler builds width = 5 copies → also "ZZZZZ". So both
    // engines land on FIVE "Z"s ("ZZZZZ\n"), byte-for-byte identical — the whole
    // point of computing the fill on a common (byte) basis. (Note: NOT six "Z"s —
    // the picture's fixed 5-char width caps the padded 6-byte image on both sides.)
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"café\"."],
        &["INSPECT S REPLACING CHARACTERS BY \"Z\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "ZZZZZ\n");
}

#[test]
fn inspect_replacing_characters_non_ascii_literal_is_a_later_rung() {
    // A single but NON-ASCII replacement LITERAL ("é" is one char / two UTF-8 bytes)
    // is deferred so the byte-based compiler stays co-total with the char-based
    // oracle — rejected on BOTH engines (guard 2).
    let src = wrap(
        &["01  S  PIC X(5) VALUE \"ABABA\"."],
        &["INSPECT S REPLACING CHARACTERS BY \"é\".", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a non-ASCII replacement");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a non-ASCII replacement"
    );
}

// INSPECT … REPLACING CHARACTERS BY x {BEFORE|AFTER} z — restrict the CHARACTERS
// overwrite to the sub-slice bounded by the FIRST occurrence of the single region
// delimiter `z`. EVERY in-window position becomes `x`; positions OUTSIDE the window
// keep their original char. The window is the SAME one the ALL/region and TALLYING
// rungs compute (shared helper), so the ISO not-found asymmetry — BEFORE with `z`
// absent overwrites the WHOLE source, AFTER with `z` absent overwrites NOTHING —
// must hold byte-for-byte on both engines (ASCII source).
// ---------------------------------------------------------------------------

#[test]
fn inspect_replacing_characters_before_replaces_only_left_of_the_delimiter() {
    // "AB,CD" — BEFORE "," restricts the overwrite to "AB" (indices 0..2): both become
    // "*", the comma and "CD" (indices 2..5, at/right of ",") are UNTOUCHED.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AB,CD\"."],
        &["INSPECT S REPLACING CHARACTERS BY \"*\" BEFORE \",\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "**,CD\n");
}

#[test]
fn inspect_replacing_characters_after_replaces_only_right_of_the_delimiter() {
    // Same source — AFTER "," restricts the overwrite to "CD" (indices 3..5): both
    // become "*"; the comma and "AB" left of it are UNTOUCHED.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AB,CD\"."],
        &["INSPECT S REPLACING CHARACTERS BY \"*\" AFTER \",\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "AB,**\n");
}

#[test]
fn inspect_replacing_characters_before_absent_delimiter_replaces_the_whole_source() {
    // BEFORE "Z" with no "Z" present → the region is the ENTIRE source, so EVERY
    // position is overwritten (the BEFORE not-found rule).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AB,CD\"."],
        &["INSPECT S REPLACING CHARACTERS BY \"*\" BEFORE \"Z\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "*****\n");
}

#[test]
fn inspect_replacing_characters_after_absent_delimiter_replaces_nothing() {
    // AFTER "Z" with no "Z" present → the region is EMPTY, so NOTHING is overwritten and
    // the source is unchanged (the AFTER not-found rule — asymmetric partner above).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AB,CD\"."],
        &["INSPECT S REPLACING CHARACTERS BY \"*\" AFTER \"Z\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "AB,CD\n");
}

#[test]
fn inspect_replacing_characters_region_replacement_is_a_pic_x1_item() {
    // The replacement `x` can be a PIC X(1) DATA ITEM with a region: R = "*", BEFORE ","
    // overwrites "AB" → "**,CD" (exercises the item path through the window).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AB,CD\".", "01  R  PIC X(1) VALUE \"*\"."],
        &["INSPECT S REPLACING CHARACTERS BY R BEFORE \",\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "**,CD\n");
}

#[test]
fn inspect_replacing_characters_region_delimiter_is_a_pic_x1_item() {
    // The region DELIMITER can be a PIC X(1) DATA ITEM (not just a literal): D = ",",
    // BEFORE D restricts the overwrite to "AB" → "**,CD" — resolved single-char via the
    // shared helper, identically to a literal delimiter.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AB,CD\".", "01  D  PIC X(1) VALUE \",\"."],
        &["INSPECT S REPLACING CHARACTERS BY \"*\" BEFORE D.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "**,CD\n");
}

#[test]
fn inspect_replacing_characters_before_delimiter_at_index_0_is_an_empty_window() {
    // Boundary: the delimiter sits at position 0, so BEFORE's window is [0,0) — EMPTY.
    // Nothing is overwritten; the source is unchanged. Verifies the compiler's byte
    // `[start,end)` and the oracle's char window agree at the left edge (start==end).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \",ABCD\"."],
        &["INSPECT S REPLACING CHARACTERS BY \"*\" BEFORE \",\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, ",ABCD\n");
}

#[test]
fn inspect_replacing_characters_after_delimiter_at_last_index_is_an_empty_window() {
    // Boundary: the delimiter sits at the LAST position, so AFTER's window is
    // [len,len) — EMPTY. Nothing is overwritten. Verifies both engines agree at the
    // right edge (start==end==len), the AFTER partner of the index-0 case above.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"ABCD,\"."],
        &["INSPECT S REPLACING CHARACTERS BY \"*\" AFTER \",\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "ABCD,\n");
}

#[test]
fn inspect_replacing_characters_region_short_moved_value_width_invariant() {
    // Width-vs-storage invariant: a value SHORTER than the picture is space-padded to
    // the full width (MOVE "AB" TO PIC X(5) → "AB   "), so the compiler's window scan
    // over str_len(S)==width and its str_slice(S,j,j+1) up to width-1 stay in bounds and
    // agree with the oracle's char rebuild. BEFORE " " restricts the overwrite to the
    // pre-space region "AB" → "**   " (the padded spaces are the AFTER-region).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE SPACES.", "01  T  PIC X(2) VALUE \"AB\"."],
        &[
            "MOVE T TO S.",
            "INSPECT S REPLACING CHARACTERS BY \"*\" BEFORE \" \".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "**   \n");
}

#[test]
fn inspect_replacing_characters_region_non_ascii_source_shares_the_reconstruction_chip() {
    // NON-ASCII SOURCE + region — the PRE-EXISTING byte-vs-char reconstruction chip
    // (task_396ba6f6), shared by every REPLACING-with-region lowering. The compiler
    // rebuilds out-of-window positions with per-position `str_slice`, which cannot
    // slice a multi-byte "é", so it TRAPS. The oracle iterates `char`s and succeeds
    // char-based (AFTER "," → char window [3,5) = "BC" → "**"; the "é" left of "," is
    // outside the window and passes through → "Aé,**"). That gap is the documented
    // pre-existing chip, NOT introduced by this rung; we pin each engine as a stable
    // characterization.
    let src = wrap(
        &["01  S  PIC X(5) VALUE \"Aé,BC\"."],
        &["INSPECT S REPLACING CHARACTERS BY \"*\" AFTER \",\".", "DISPLAY S.", "STOP RUN."],
    );
    // Compiler: the reconstruction traps on the multi-byte char (pre-existing chip).
    assert!(
        run_on_jit_result(&src).is_err(),
        "compiled CHARACTERS+region reconstruction must trap on a non-ASCII source (shared chip)"
    );
    // Oracle: succeeds char-based (the documented pre-existing chip, unchanged).
    assert_eq!(
        run_cobol(&src).expect("oracle succeeds char-based"),
        "Aé,**\n",
        "oracle characterization"
    );
}

// INSPECT … REPLACING ALL x BY y … {BEFORE|AFTER} z — restrict the ALL replacement
// to the sub-slice of the source bounded by the FIRST occurrence of the single
// region delimiter `z`. BEFORE replaces left of `z`; AFTER replaces right of it;
// positions OUTSIDE the region keep their original character. The window is the
// SAME one the TALLYING region rung computes (shared helper), so the ISO not-found
// asymmetry — BEFORE with `z` absent replaces the WHOLE source, AFTER with `z`
// absent replaces NOTHING — must hold byte-for-byte on both engines.
// ---------------------------------------------------------------------------

#[test]
fn inspect_replacing_before_replaces_only_left_of_the_delimiter() {
    // "0A0B0" — BEFORE "B" restricts the replace to "0A0" (indices 0..3): the two
    // "0"s there become "*", the trailing "0" (index 4, right of "B") is UNTOUCHED.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0A0B0\"."],
        &["INSPECT S REPLACING ALL \"0\" BY \"*\" BEFORE \"B\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "*A*B0\n");
}

#[test]
fn inspect_replacing_after_replaces_only_right_of_the_delimiter() {
    // Same source — AFTER "B" restricts the replace to "0" (index 4): only that
    // trailing "0" becomes "*"; the two "0"s left of "B" are UNTOUCHED.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0A0B0\"."],
        &["INSPECT S REPLACING ALL \"0\" BY \"*\" AFTER \"B\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "0A0B*\n");
}

#[test]
fn inspect_replacing_before_absent_delimiter_replaces_the_whole_source() {
    // BEFORE "Z" with no "Z" present → the region is the ENTIRE source, so EVERY "0"
    // is replaced (the BEFORE not-found rule — the whole subtlety of the rung).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0A0B0\"."],
        &["INSPECT S REPLACING ALL \"0\" BY \"*\" BEFORE \"Z\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "*A*B*\n");
}

#[test]
fn inspect_replacing_after_absent_delimiter_replaces_nothing() {
    // AFTER "Z" with no "Z" present → the region is EMPTY, so NOTHING is replaced and
    // the source is unchanged (the AFTER not-found rule — asymmetric partner above).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0A0B0\"."],
        &["INSPECT S REPLACING ALL \"0\" BY \"*\" AFTER \"Z\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "0A0B0\n");
}

#[test]
fn inspect_replacing_after_region_delimiter_at_position_zero() {
    // The region delimiter is the FIRST character: AFTER "B" in "B0A0" → region
    // [1, 4) = "0A0", so both "0"s become "*" → "B*A*".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"B0A0\"."],
        &["INSPECT S REPLACING ALL \"0\" BY \"*\" AFTER \"B\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "B*A*\n");
}

#[test]
fn inspect_replacing_region_delimiter_equals_search() {
    // The search char and the region delimiter are the SAME "0": AFTER "0" in "0A0B0"
    // → the FIRST "0" (index 0) bounds the region [1, 5) = "A0B0". The replace runs
    // over the ORIGINAL bytes, so the two "0"s at indices 2 and 4 become "*", while
    // the delimiter "0" at index 0 (left of the region) is KEPT → "0A*B*".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0A0B0\"."],
        &["INSPECT S REPLACING ALL \"0\" BY \"*\" AFTER \"0\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "0A*B*\n");
}

#[test]
fn inspect_replacing_region_delimiter_equals_replacement() {
    // The region delimiter and the replacement char are the SAME "*": BEFORE "*" in
    // "0*0A0" → the first "*" (index 1) bounds the region [0, 1) = "0", so only the
    // leading "0" becomes "*" → "**0A0" (the "*" delimiter and later "0"s untouched).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0*0A0\"."],
        &["INSPECT S REPLACING ALL \"0\" BY \"*\" BEFORE \"*\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "**0A0\n");
}

#[test]
fn inspect_replacing_before_delimiter_at_position_zero_is_an_empty_region() {
    // BEFORE "B" in "B0A0" → the first "B" is at index 0, so the region is [0, 0) —
    // EMPTY — and NOTHING is replaced even though "0"s follow → "B0A0" unchanged.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"B0A0\"."],
        &["INSPECT S REPLACING ALL \"0\" BY \"*\" BEFORE \"B\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "B0A0\n");
}

#[test]
fn inspect_replacing_region_delimiter_is_a_pic_x1_item() {
    // The region delimiter may itself be a PIC X(1) item, read at run time: BEFORE DL
    // (= "B") in "0A0B0" restricts to "0A0" → "*A*B0", matching the literal case.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0A0B0\".", "01  DL PIC X(1) VALUE \"B\"."],
        &["INSPECT S REPLACING ALL \"0\" BY \"*\" BEFORE DL.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "*A*B0\n");
}

// INSPECT … REPLACING LEADING x BY y … {BEFORE|AFTER} z — the STANDALONE
// leading-run substitution restricted to a `{BEFORE|AFTER}` sub-region, the exact
// analogue of `FOR LEADING … {BEFORE|AFTER}` on the count side. The run is anchored
// at the WINDOW START: characters before the window are copied through unchanged and
// neither begin nor break the run; the run begins at the window start and stops at
// the first non-`x` INSIDE the window. `AFTER z` with `z` absent is an empty window
// (no substitution); `BEFORE z` with `z` absent is the whole source.

#[test]
fn inspect_replacing_leading_after_anchors_the_run_at_the_window_start() {
    // "aaXaab" REPLACING LEADING "a" BY "*" AFTER "X" → window "aab" (3..6). Only the
    // two leading a's AFTER the X are rewritten; the "aa" before the X is untouched.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"aaXaab\"."],
        &["INSPECT S REPLACING LEADING \"a\" BY \"*\" AFTER \"X\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "aaX**b\n");
}

#[test]
fn inspect_replacing_leading_after_window_starts_on_a_mismatch() {
    // "aaXbb" AFTER "X" → window "bb" (3..5); its first char is not 'a', so nothing is
    // replaced even though a's precede the X. The source is returned unchanged.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aaXbb\"."],
        &["INSPECT S REPLACING LEADING \"a\" BY \"*\" AFTER \"X\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "aaXbb\n");
}

#[test]
fn inspect_replacing_leading_before_rewrites_the_prefix_run() {
    // "aaXaa" REPLACING LEADING "a" BY "*" BEFORE "X" → window "aa" (0..2) → the two
    // leading a's become '*'; the "aa" after the X is outside the BEFORE window.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aaXaa\"."],
        &["INSPECT S REPLACING LEADING \"a\" BY \"*\" BEFORE \"X\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "**Xaa\n");
}

#[test]
fn inspect_replacing_leading_after_delimiter_at_position_zero() {
    // Delimiter is the FIRST char: "Xaaab" AFTER "X" → window "aaab" (1..5), the three
    // leading a's are rewritten → "X***b". Pins the `start = first + 1` edge.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"Xaaab\"."],
        &["INSPECT S REPLACING LEADING \"a\" BY \"*\" AFTER \"X\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "X***b\n");
}

#[test]
fn inspect_replacing_leading_after_absent_delimiter_is_an_empty_window() {
    // AFTER "Z" with no 'Z' present → the window is EMPTY, so nothing is replaced (the
    // ISO not-found asymmetry on the AFTER side). The source is unchanged.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"aaXaab\"."],
        &["INSPECT S REPLACING LEADING \"a\" BY \"*\" AFTER \"Z\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "aaXaab\n");
}

#[test]
fn inspect_replacing_leading_before_absent_delimiter_is_the_whole_source() {
    // BEFORE "Z" with no 'Z' present → the window is the WHOLE source "aaXaa", where
    // the leading 'a' run (from position 0) is the two a's before the X → "**Xaa".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aaXaa\"."],
        &["INSPECT S REPLACING LEADING \"a\" BY \"*\" BEFORE \"Z\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "**Xaa\n");
}

#[test]
fn inspect_replacing_leading_region_delimiter_equals_search_char() {
    // Region delimiter == search char: "aaXaa" AFTER "a" bounds on the FIRST 'a'
    // (index 0), window "aXaa" (1..5). The leading 'a' run there is 1 (position 1),
    // stopping at the X → only that one 'a' is rewritten → "a*Xaa".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aaXaa\"."],
        &["INSPECT S REPLACING LEADING \"a\" BY \"*\" AFTER \"a\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "a*Xaa\n");
}

#[test]
fn inspect_replacing_multi_char_region_delimiter_is_a_later_rung() {
    // A MULTI-character region delimiter is deferred — rejected on both engines, just
    // like a multi-character search/tally delimiter (the oracle rejects at exec, the
    // compiler at emit, both via the shared single-delimiter check).
    let src = wrap(
        &["01  S  PIC X(6) VALUE \"0A0CD0\"."],
        &["INSPECT S REPLACING ALL \"0\" BY \"*\" BEFORE \"CD\".", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a multi-char region delimiter");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a multi-char region delimiter"
    );
}

// INSPECT … REPLACING ALL a BY x ALL b BY y [ALL c BY z …] — TWO OR MORE replace
// items in ONE REPLACING clause. One left-to-right pass, first-match-wins, and (the
// high-value case) NO re-chaining: a byte a replacement produces is never fed to a
// later item. Each case pins the exact rebuilt source and `assert_matches_oracle`
// independently re-checks JIT == tree-walk oracle.

#[test]
fn inspect_replacing_multi_two_items() {
    // "abcab" with a→x, b→y → "xycxy": each position takes its own item, c untouched.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"abcab\"."],
        &["INSPECT S REPLACING ALL \"a\" BY \"x\" ALL \"b\" BY \"y\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "xycxy\n");
}

#[test]
fn inspect_replacing_multi_three_items() {
    // Three items over "abcabc": a→x, b→y, c→z → "xyzxyz".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"abcabc\"."],
        &[
            "INSPECT S REPLACING ALL \"a\" BY \"x\"",
            "    ALL \"b\" BY \"y\" ALL \"c\" BY \"z\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "xyzxyz\n");
}

#[test]
fn inspect_replacing_multi_no_rechaining() {
    // THE key correctness case. `ALL "a" BY "b" ALL "b" BY "z"` over "ab" → "bz":
    // position 0's a→b STOPS (the produced 'b' is NOT then turned into 'z'), and
    // position 1's original 'b'→z. A naive sequential two-pass replace would give
    // "zz" — this pins the single-pass no-re-chaining semantics on BOTH engines.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(2) VALUE \"ab\"."],
        &["INSPECT S REPLACING ALL \"a\" BY \"b\" ALL \"b\" BY \"z\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "bz\n");
}

#[test]
fn inspect_replacing_multi_first_match_wins() {
    // Two items whose searches OVERLAP on 'a': `ALL "a" BY "x" ALL "a" BY "y"`. The
    // FIRST written item wins at every 'a' → "x", never "y".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"aaa\"."],
        &["INSPECT S REPLACING ALL \"a\" BY \"x\" ALL \"a\" BY \"y\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "xxx\n");
}

#[test]
fn inspect_replacing_multi_char_matched_by_no_item() {
    // 'Q' matches neither item → it survives unchanged among replaced neighbours.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aQbQa\"."],
        &["INSPECT S REPLACING ALL \"a\" BY \"x\" ALL \"b\" BY \"y\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "xQyQx\n");
}

#[test]
fn inspect_replacing_multi_source_all_one_item() {
    // Every character of "aaaa" is matched by the FIRST item → "xxxx" (the second
    // item never fires).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"aaaa\"."],
        &["INSPECT S REPLACING ALL \"a\" BY \"x\" ALL \"b\" BY \"y\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "xxxx\n");
}

#[test]
fn inspect_replacing_single_item_still_works() {
    // Regression: exactly ONE replace item keeps the single-item path unchanged.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"ABABA\"."],
        &["INSPECT S REPLACING ALL \"A\" BY \"X\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "XBXBX\n");
}

#[test]
fn inspect_replacing_multi_pic_x1_search_and_replacement() {
    // A multi-item list whose search/replacement operands are PIC X(1) items: O→0,
    // N→M over "MOON" → "M00M".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(4) VALUE \"MOON\".",
            "01  A  PIC X(1) VALUE \"O\".",
            "01  B  PIC X(1) VALUE \"0\".",
            "01  C  PIC X(1) VALUE \"N\".",
            "01  D  PIC X(1) VALUE \"M\".",
        ],
        &[
            "INSPECT S REPLACING ALL A BY B ALL C BY D.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "M00M\n");
}

#[test]
fn inspect_replacing_multi_leading_then_all_now_supported() {
    // THIS rung: a LEADING item inside a MULTI-item list is now SUPPORTED (the earlier
    // "several items and a LEADING item is a later rung" reject is LIFTED — this test was
    // that reject, converted to a positive). `REPLACING LEADING "a" BY "X" ALL "b" BY "Y"`
    // over "aabaa": the leading run of "a" (positions 0,1) is replaced by X; the run
    // breaks at the "b" (index 2, which ALL "b" replaces with Y); the "a"s AFTER the break
    // (indices 3,4) are NOT replaced — the leading run is dead. → "XXYaa". This is the
    // exact replace-side twin of #65's tally-multi-LEADING active-flag machine.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aabaa\"."],
        &[
            "INSPECT S REPLACING LEADING \"a\" BY \"X\" ALL \"b\" BY \"Y\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "XXYaa\n");
}

#[test]
fn inspect_replacing_multi_each_item_with_a_region() {
    // Two items, one with BEFORE and one with AFTER, over a source where the windows
    // differ. Source "a0b0a" (positions 0..5): item 1 `ALL "a" BY "x"` has NO region
    // (whole source); item 2 `ALL "0" BY "*" BEFORE "b"` fires only left of the first
    // "b" (index 2 → window [0,2)). Both "a"s become "x"; only the "0" at index 1
    // (inside [0,2)) becomes "*", the "0" at index 3 (outside) stays "0".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"a0b0a\"."],
        &[
            "INSPECT S REPLACING ALL \"a\" BY \"x\"",
            "    ALL \"0\" BY \"*\" BEFORE \"b\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "x*b0x\n");
}

#[test]
fn inspect_replacing_multi_before_and_after_windows() {
    // One item windowed BEFORE, one AFTER, over "aXaXa" (X at index 1 and 3).
    // Item 1 `ALL "a" BY "b" BEFORE "X"`: window [0,1) → only index 0's "a" → "b".
    // Item 2 `ALL "a" BY "c" AFTER "X"`: window (1,5] → indices 2 and 4's "a" → "c".
    // The "a" at index 0 is claimed by item 1 first (first-match-wins), the rest by
    // item 2. The "X"s match neither item and pass through.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aXaXa\"."],
        &[
            "INSPECT S REPLACING ALL \"a\" BY \"b\" BEFORE \"X\"",
            "    ALL \"a\" BY \"c\" AFTER \"X\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "bXcXc\n");
}

#[test]
fn inspect_replacing_multi_region_plus_regionless_item() {
    // A region item mixed with a whole-source (region-less) item. Source "0a0a0"
    // Item 1 `ALL "0" BY "*" AFTER "a"`: first "a" at index 1 → window (1,5] →
    // the "0"s at indices 2 and 4 become "*"; the "0" at index 0 (outside) stays.
    // Item 2 `ALL "a" BY "z"` (no region): both "a"s become "z" everywhere.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0a0a0\"."],
        &[
            "INSPECT S REPLACING ALL \"0\" BY \"*\" AFTER \"a\"",
            "    ALL \"a\" BY \"z\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "0z*z*\n");
}

#[test]
fn inspect_replacing_multi_after_absent_delimiter_empty_window() {
    // `AFTER x` where x is ABSENT → an EMPTY window, so that item NEVER fires; the
    // other (region-less) item still applies everywhere. Source "abab", item 1
    // `ALL "a" BY "*" AFTER "Z"` (no "Z" present → empty window, never fires),
    // item 2 `ALL "b" BY "y"` rewrites both "b"s. The "a"s stay untouched.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"abab\"."],
        &[
            "INSPECT S REPLACING ALL \"a\" BY \"*\" AFTER \"Z\"",
            "    ALL \"b\" BY \"y\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "ayay\n");
}

#[test]
fn inspect_replacing_multi_first_match_wins_overlapping_windows() {
    // FIRST-MATCH precedence when TWO items' windows both cover a position and both
    // match — the EARLIER-written item wins. Source "aaaa" with no region on either
    // (both whole-source windows): `ALL "a" BY "x"` then `ALL "a" BY "y"`. Every "a"
    // is claimed by item 1 → all "x", none "y".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"aaaa\"."],
        &[
            "INSPECT S REPLACING ALL \"a\" BY \"x\"",
            "    ALL \"a\" BY \"y\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "xxxx\n");
}

#[test]
fn inspect_replacing_multi_first_match_wins_within_overlapping_regions() {
    // Both items carry a region and both windows cover the same position, and both
    // searches match it — the earlier item still wins. Source "aXaa": item 1
    // `ALL "a" BY "p" AFTER "X"` (window (1,4] → indices 2,3), item 2
    // `ALL "a" BY "q" AFTER "X"` (same window). Index 0's "a" is outside BOTH windows
    // (before the "X") so it stays "a"; indices 2 and 3 are claimed by item 1 → "p".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"aXaa\"."],
        &[
            "INSPECT S REPLACING ALL \"a\" BY \"p\" AFTER \"X\"",
            "    ALL \"a\" BY \"q\" AFTER \"X\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "aXpp\n");
}

#[test]
fn inspect_replacing_multi_non_ascii_source_shares_the_reconstruction_chip() {
    // NON-ASCII SOURCE: this rung's MATCHING is byte-safe — a multi-byte "é" never
    // equals an ASCII search byte, so it is never falsely matched or replaced, and the
    // per-item window (computed over the ORIGINAL source, bounded by the first
    // occurrence of an ASCII region delimiter) selects the SAME ASCII positions on
    // both engines. BUT the RECONSTRUCTION is the PRE-EXISTING byte-vs-char chip shared
    // by EVERY REPLACING lowering: the byte-based compiler rebuilds the field with
    // per-position `str_slice`, which cannot slice a multi-byte char, so it traps —
    // EXACTLY as the merged single-item `REPLACING ALL` does on the same source. The
    // multi-item + per-item-region path introduces NO new non-ASCII behavior: it traps
    // identically. (The oracle iterates `char`s and succeeds char-based; that
    // divergence is the documented pre-existing chip, NOT introduced here.) We pin that
    // the MULTI path and the SINGLE-item path share the chip byte-for-byte.
    let multi = wrap(
        &["01  S  PIC X(5) VALUE \"aéaba\"."],
        &[
            "INSPECT S REPLACING ALL \"a\" BY \"x\" BEFORE \"b\"",
            "    ALL \"b\" BY \"y\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    );
    let single = wrap(
        &["01  S  PIC X(5) VALUE \"aéaba\"."],
        &["INSPECT S REPLACING ALL \"a\" BY \"x\".", "STOP RUN."],
    );
    // The compiled reconstruction traps on the multi-byte char on BOTH the multi path
    // (per-item regions) and the pre-existing single-item path — the chip is shared,
    // unchanged by this rung.
    assert!(
        run_on_jit_result(&multi).is_err(),
        "multi + per-item-region path must trap on a non-ASCII source, like single-item"
    );
    assert!(
        run_on_jit_result(&single).is_err(),
        "single-item REPLACING ALL traps on a non-ASCII source (pre-existing chip)"
    );
    // The oracle iterates chars and succeeds on the multi path: item 1
    // `ALL "a" BY "x" BEFORE "b"` (window [0, index_of_b)=[0,3)) rewrites the two "a"s
    // left of "b"; item 2 `ALL "b" BY "y"` rewrites the "b" (index 3); the "a" at
    // index 4 is outside item 1's window and is not a "b", so it stays. The "é" (a
    // non-ASCII byte) matches no ASCII search and passes through untouched → "xéxya".
    // This pins the MATCH-side byte-safety and the per-item window agreement.
    assert_eq!(run_cobol(&multi).expect("oracle succeeds char-based"), "xéxya\n");
}

// A MULTI-item REPLACING list may now include a `CHARACTERS` item (THIS rung lifts the
// multi-item CHARACTERS reject — the REPLACE twin of the tally side #81). `CHARACTERS` is
// the always-eligible catch-all: at each in-window position not already claimed by an
// EARLIER item in written order it EMITS its replacement char — NO search compare, NO
// leading-run tracking. An optional `{BEFORE|AFTER}` region narrows its window exactly
// like any other item. Each case pins the exact rebuilt field and `assert_matches_oracle`
// independently re-checks JIT == tree-walk oracle. (Only the COMBINED `TALLYING …
// REPLACING` form still defers CHARACTERS in its REPLACING half — see
// `inspect_replacing_multi_combined_with_characters_is_a_later_rung`.)

#[test]
fn inspect_replacing_multi_all_then_characters_covers_the_rest() {
    // `REPLACING ALL "A" BY "B" CHARACTERS BY "*"` over "AXAY" (X,Y not "A"). ALL "A"
    // (item 0, higher priority) claims the two "A"s at positions 0,2 → "B"; the CHARACTERS
    // catch-all (item 1) claims every OTHER position (1,3) → "*". Every position is claimed
    // by exactly one item — pins that CHARACTERS emits at exactly the positions an earlier
    // item did not.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"AXAY\"."],
        &[
            "INSPECT S REPLACING ALL \"A\" BY \"B\" CHARACTERS BY \"*\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "B*B*\n");
}

#[test]
fn inspect_replacing_multi_characters_first_shadows_the_all_item() {
    // WRITTEN-ORDER priority: `REPLACING CHARACTERS BY "*" ALL "A" BY "B"` (CHARACTERS
    // FIRST) over "AABB". The region-less CHARACTERS catch-all is eligible at EVERY
    // position, so it claims all 4 → "*", and the following ALL "A" NEVER fires
    // (first-match-per-position). Result "****", exactly as if the ALL item were absent —
    // proving CHARACTERS' position in the list is honoured (a lower-priority ALL of a
    // matching search is shadowed).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"AABB\"."],
        &[
            "INSPECT S REPLACING CHARACTERS BY \"*\" ALL \"A\" BY \"B\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "****\n");
}

#[test]
fn inspect_replacing_multi_all_then_characters_with_before_region() {
    // A CHARACTERS item WITH a `{BEFORE|AFTER}` region narrows its window; a char AFTER the
    // CHARACTERS window still gets the ALL replacement. `REPLACING ALL "A" BY "B"
    // CHARACTERS BY "*" BEFORE "X"` over "AZXA" (X at index 2):
    //   pos 0 'A' → ALL "A" → "B"
    //   pos 1 'Z' → ALL no; CHARACTERS window [0,2) contains 1 → "*"
    //   pos 2 'X' → ALL no; CHARACTERS window [0,2) excludes 2 → kept "X"
    //   pos 3 'A' → ALL "A" → "B"  (past the CHARACTERS window, still claimed by ALL)
    // Result "B*XB" — the region genuinely bounds the catch-all, and the trailing ALL item
    // is NOT shadowed outside that window.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"AZXA\"."],
        &[
            "INSPECT S REPLACING ALL \"A\" BY \"B\" CHARACTERS BY \"*\" BEFORE \"X\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "B*XB\n");
}

#[test]
fn inspect_replacing_multi_leading_then_characters_run_tracking_unaffected() {
    // A CHARACTERS item alongside a LEADING item — the LEADING run tracking is UNAFFECTED
    // by the (search-less, run-less) CHARACTERS item. `REPLACING LEADING "A" BY "L"
    // CHARACTERS BY "*" BEFORE "X"` over "AABAX" (X at index 4):
    //   pos 0 'A' → LEADING run alive → "L"
    //   pos 1 'A' → LEADING run alive → "L"
    //   pos 2 'B' → LEADING no (mismatch, run breaks here); CHARACTERS window [0,4)
    //               contains 2 → "*"
    //   pos 3 'A' → LEADING run now DEAD → no; CHARACTERS window contains 3 → "*"
    //   pos 4 'X' → LEADING dead; CHARACTERS window [0,4) excludes 4 → kept "X"
    // Result "LL**X". The LEADING run rewrites exactly its anchored run (2 chars) and
    // breaks at the first in-window mismatch, INDEPENDENTLY of the CHARACTERS item claiming
    // that same position — the active-run update never consults or is consulted by
    // CHARACTERS.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AABAX\"."],
        &[
            "INSPECT S REPLACING LEADING \"A\" BY \"L\"",
            "    CHARACTERS BY \"*\" BEFORE \"X\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "LL**X\n");
}

#[test]
fn inspect_replacing_multi_characters_in_the_middle_shadows_multiple_trailing_items() {
    // WRITTEN-ORDER priority with a region-less CHARACTERS in the MIDDLE of THREE items:
    // `REPLACING ALL "A" BY "B" CHARACTERS BY "*" ALL "C" BY "D"` over "ACAC". Item 0
    // (ALL "A") claims the two 'A's → "B"; the region-less CHARACTERS catch-all then claims
    // EVERY remaining position → "*", so item 2 (ALL "C") is UNREACHABLE and never fires.
    // Result "B*B*". This exercises the compiler's unreachable-block emission for MORE THAN
    // ONE trailing chain link after an unconditional catch-all — each dead link is a
    // self-contained block ending in its own `jmp done`, so the shadowing matches the
    // oracle's first-eligible rule byte-for-byte.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"ACAC\"."],
        &[
            "INSPECT S REPLACING ALL \"A\" BY \"B\"",
            "    CHARACTERS BY \"*\" ALL \"C\" BY \"D\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "B*B*\n");
}

#[test]
fn inspect_replacing_multi_region_less_characters_shadows_a_trailing_leading() {
    // A region-less CHARACTERS catch-all shadows a trailing LEADING item exactly as it
    // shadows a trailing ALL: `REPLACING CHARACTERS BY "*" LEADING "A" BY "L"` over "AAB"
    // → every position is claimed by CHARACTERS → "***"; the LEADING item never fires and
    // its run machinery (which the CHARACTERS item never touches) is irrelevant. Verifies
    // the unconditional-append shadow is co-total when the shadowed link carries a run.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"AAB\"."],
        &[
            "INSPECT S REPLACING CHARACTERS BY \"*\"",
            "    LEADING \"A\" BY \"L\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "***\n");
}

#[test]
fn inspect_replacing_multi_characters_empty_window_lets_a_trailing_all_fire() {
    // A CHARACTERS item with an ABSENT AFTER delimiter has an EMPTY window (start==end==len),
    // so it replaces nothing and a trailing ALL item still fires at every position:
    // `REPLACING CHARACTERS BY "*" AFTER "Q" ALL "A" BY "B"` over "AA" (no "Q") → "BB".
    // Confirms the empty-window CHARACTERS is guarded (not the region-less unconditional
    // path) and does not shadow the trailing item — co-total on both engines.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(2) VALUE \"AA\"."],
        &[
            "INSPECT S REPLACING CHARACTERS BY \"*\" AFTER \"Q\"",
            "    ALL \"A\" BY \"B\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "BB\n");
}

#[test]
fn inspect_replacing_multi_combined_with_characters_is_a_later_rung() {
    // The COMBINED `TALLYING … REPLACING` form still DEFERS a CHARACTERS item in its
    // REPLACING half — rejected identically on both engines. (A combined REPLACING is read
    // by the single-item reader, which rejects both CHARACTERS and a multi-item list
    // outright; the multi-item CHARACTERS lift does NOT leak into the combined path.)
    let src = wrap(
        &["01  S  PIC X(4) VALUE \"AABB\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"A\"",
            "    REPLACING CHARACTERS BY \"*\" ALL \"B\" BY \"C\".",
            "STOP RUN.",
        ],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject combined + multi-item CHARACTERS");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject combined + multi-item CHARACTERS"
    );
}

#[test]
fn inspect_replacing_multi_characters_non_ascii_shares_the_reconstruction_chip() {
    // NON-ASCII SOURCE + a CHARACTERS item: the RECONSTRUCTION is the PRE-EXISTING
    // byte-vs-char chip (task_396ba6f6) shared by EVERY REPLACING lowering — identical to
    // the sibling `inspect_replacing_multi_non_ascii_source_shares_the_reconstruction_chip`
    // and the single-item `REPLACING CHARACTERS` (#80). The byte-based compiler rebuilds
    // the field with per-position `str_slice`, which cannot slice a KEPT multi-byte char,
    // so it traps; the char-based oracle iterates `char`s and succeeds. The multi-item
    // CHARACTERS path introduces NO new non-ASCII behavior: it traps identically, so this
    // is a DOCUMENTED divergence characterization, NOT fixed here.
    //
    // `REPLACING ALL "a" BY "x" BEFORE "b" CHARACTERS BY "*" AFTER "b"` over "aéaba"
    // (chars a,é,a,b,a; "b" at char index 3):
    //   pos 0 'a' → ALL "a" window [0,3) → "x"
    //   pos 1 'é' → ALL no; CHARACTERS window (3,5] excludes 1 → KEPT "é"  (the trap site)
    //   pos 2 'a' → ALL "a" window [0,3) → "x"
    //   pos 3 'b' → ALL no; CHARACTERS window (3,5] excludes 3 → KEPT "b"
    //   pos 4 'a' → ALL "a" window [0,3) excludes 4; CHARACTERS window (3,5] contains 4 → "*"
    // Oracle char-based → "xéxb*".
    let src = wrap(
        &["01  S  PIC X(5) VALUE \"aéaba\"."],
        &[
            "INSPECT S REPLACING ALL \"a\" BY \"x\" BEFORE \"b\"",
            "    CHARACTERS BY \"*\" AFTER \"b\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    );
    assert!(
        run_on_jit_result(&src).is_err(),
        "multi-item CHARACTERS path traps on a non-ASCII kept char, like every REPLACING lowering"
    );
    assert_eq!(run_cobol(&src).expect("oracle succeeds char-based"), "xéxb*\n");
}

// INSPECT … REPLACING {ALL|LEADING} … {ALL|LEADING} … — a MULTI-item REPLACING list
// with one or more LEADING items (THIS rung lifts the multi-item LEADING reject). A
// LEADING item replaces only its CONSECUTIVE run of `search` anchored at its window
// start; ONE left-to-right pass carries a per-item `active` run flag (consulted only for
// LEADING items) — a LEADING item is eligible only while its run is alive, and every
// LEADING run breaks at the FIRST in-window mismatch, INDEPENDENTLY of which item won the
// position. The replace-side twin of #65's tally-multi-LEADING machine: the only
// difference is the decision loop EMITS a replacement instead of counting. Each case pins
// the exact rebuilt source and `assert_matches_oracle` re-checks JIT == tree-walk oracle.

#[test]
fn inspect_replacing_multi_leading_first_match_wins_over_all_same_delim() {
    // First-match precedence with a LEADING item FIRST vs an ALL item that also matches
    // the SAME delimiter. `LEADING "a" BY "X" ALL "a" BY "Y"` over "aab": positions 0,1
    // are claimed by the LEADING item (its run is alive) → "X", and ALL "a" never sees
    // them (first-match-wins, the position is not re-examined). The "b" matches neither
    // and stays. → "XXb". Pins that an earlier item's claim shadows a later item.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"aab\"."],
        &["INSPECT S REPLACING LEADING \"a\" BY \"X\" ALL \"a\" BY \"Y\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "XXb\n");
}

#[test]
fn inspect_replacing_multi_leading_with_after_region_anchors_run_at_window_start() {
    // A LEADING item WITH a `{BEFORE|AFTER}` region + an ALL item — the leading run is
    // anchored at the WINDOW START, not source position 0. `LEADING "a" BY "X" AFTER "Z"
    // ALL "b" BY "Y"` over "aaZaab" (Z at index 2): the LEADING window is (2,6] = indices
    // 3,4,5, so the two "a"s BEFORE the Z (indices 0,1) are OUTSIDE the window — they
    // neither begin nor break the run and stay "a". The run then replaces the two "a"s at
    // 3,4 with X (breaks at the "b" at 5, which ALL "b" replaces with Y). → "aaZXXY".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"aaZaab\"."],
        &[
            "INSPECT S REPLACING LEADING \"a\" BY \"X\" AFTER \"Z\"",
            "    ALL \"b\" BY \"Y\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "aaZXXY\n");
}

#[test]
fn inspect_replacing_multi_two_leading_items_independent_runs() {
    // Two LEADING items with DIFFERENT delimiters, no region — both runs anchor at source
    // start, but position 0 can equal only ONE delimiter, so the OTHER run breaks
    // immediately (independent run flags). `LEADING "a" BY "X" LEADING "b" BY "Y"` over
    // "aabb": LEADING "a" replaces the run at 0,1 → X; LEADING "b"'s run breaks at index 0
    // ("a" != "b"), so the "b"s at 2,3 are NOT replaced. → "XXbb".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"aabb\"."],
        &["INSPECT S REPLACING LEADING \"a\" BY \"X\" LEADING \"b\" BY \"Y\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "XXbb\n");
}

#[test]
fn inspect_replacing_multi_two_leading_items_disjoint_windows_both_fire() {
    // Two LEADING items with DIFFERENT delimiters AND disjoint windows, so BOTH runs fire
    // — each anchored at its OWN window start. `LEADING "a" BY "X" BEFORE "Z" LEADING "b"
    // BY "Y" AFTER "Z"` over "aaZbb" (Z at index 2): item 1's window is "aa" (indices 0,1
    // → X,X), item 2's window is "bb" (indices 3,4 → Y,Y); the "Z" matches neither and
    // stays. → "XXZYY". Pins two independent per-item active flags in one REPLACING list.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aaZbb\"."],
        &[
            "INSPECT S REPLACING LEADING \"a\" BY \"X\" BEFORE \"Z\"",
            "    LEADING \"b\" BY \"Y\" AFTER \"Z\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "XXZYY\n");
}

#[test]
fn inspect_replacing_multi_leading_run_breaks_on_char_claimed_by_higher_priority_item() {
    // THE run-update-independent-of-winner subtlety (mirrors #65's tally analogue). A
    // higher-priority ALL item claims position 0, whose char is NOT the LEADING item's
    // search — the LEADING run must STILL break there, even though the decision loop
    // `break`s before the LEADING item's decision link is ever evaluated (the run-update
    // is a SEPARATE pass, not folded into the decision). `ALL "X" BY "Q" LEADING "a" BY
    // "Z"` over "Xaa": at index 0 the ALL item replaces "X"→"Q" and stops the decision;
    // the separate run-update sees index 0's "X" != "a" and kills the LEADING run, so the
    // "a"s at 1,2 are NOT replaced → "Qaa". A buggy impl that decayed `active` only inside
    // the decision chain would leave the run alive and wrongly produce "QZZ".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"Xaa\"."],
        &["INSPECT S REPLACING ALL \"X\" BY \"Q\" LEADING \"a\" BY \"Z\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "Qaa\n");
}

#[test]
fn inspect_replacing_multi_leading_regression_single_item_leading_unchanged() {
    // Regression: a LONE `REPLACING LEADING` still routes through the single-item path
    // (unchanged by this rung) — the leading run of "A" is replaced, a later "A" is not.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AABAA\"."],
        &["INSPECT S REPLACING LEADING \"A\" BY \"X\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "XXBAA\n");
}

#[test]
fn inspect_replacing_multi_leading_regression_all_only_multi_unchanged() {
    // Regression: an ALL-only multi-item list (no LEADING) still behaves exactly as
    // before — first-match-wins, no re-chaining. `ALL "a" BY "x" ALL "b" BY "y"` over
    // "abab" → "xyxy".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"abab\"."],
        &["INSPECT S REPLACING ALL \"a\" BY \"x\" ALL \"b\" BY \"y\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "xyxy\n");
}

#[test]
fn inspect_replacing_multi_leading_with_filler_between_named_items_agrees() {
    // Cross-producer binding guard: an unnamed FILLER PIC X(2) sits BETWEEN the named
    // REPLACING source `S` and another named item `T` in WORKING-STORAGE. The compiler
    // DROPS FILLER items from its data model while the oracle PUSHES them, so the two
    // engines assign different physical slots; this pins that NAMED binding (the LEADING
    // multi-REPLACING resolves `S` by NAME) stays byte-identical across the two models.
    // `LEADING "a" BY "X" ALL "b" BY "Y"` over "aabaa" → "XXYaa"; `T` is untouched.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"aabaa\".",
            "01  FILLER  PIC X(2).",
            "01  T  PIC X(3) VALUE \"zzz\".",
        ],
        &[
            "INSPECT S REPLACING LEADING \"a\" BY \"X\" ALL \"b\" BY \"Y\".",
            "DISPLAY S.",
            "DISPLAY T.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "XXYaa\nzzz\n");
}

#[test]
fn inspect_replacing_multi_leading_non_ascii_source_shares_the_reconstruction_chip() {
    // NON-ASCII SOURCE — the PRE-EXISTING byte-vs-char reconstruction chip (task_396ba6f6),
    // shared by EVERY REPLACING lowering and NOT introduced by the LEADING multi path.
    // REPLACING RECONSTRUCTS the field with per-position byte `str_slice`, which cannot
    // slice a multi-byte char, so the byte-based compiler TRAPS on any multi-byte source —
    // EXACTLY as the merged single-item `REPLACING ALL` and multi-item ALL paths do. The
    // char-based oracle iterates `char`s and succeeds. This is a CHARACTERIZATION test (we
    // pin BOTH engines' outputs, NOT `assert_matches_oracle`), documenting that the LEADING
    // multi path inherits the chip identically and introduces NO new divergence.
    //
    // Source "aéaba" (chars a, é, a, b, a) with `LEADING "a" BY "X" ALL "b" BY "Y"`:
    //   i=0 'a' → X (leading run alive);  i=1 'é' ≠ "a"/"b" → kept, and it BREAKS the
    //   leading "a" run;  i=2 'a' → kept (run dead);  i=3 'b' → Y;  i=4 'a' → kept.
    // The multi-byte "é" (bytes C3 A9) matches no ASCII search, so the MATCH side is
    // byte-safe; only the RECONSTRUCTION traps. Oracle → "XéaYa".
    let src = wrap(
        &["01  S  PIC X(5) VALUE \"aéaba\"."],
        &[
            "INSPECT S REPLACING LEADING \"a\" BY \"X\" ALL \"b\" BY \"Y\".",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    );
    assert!(
        run_on_jit_result(&src).is_err(),
        "compiled LEADING-multi reconstruction traps on a non-ASCII source (pre-existing chip)"
    );
    assert_eq!(run_cobol(&src).expect("oracle succeeds char-based"), "XéaYa\n");
}

#[test]
fn inspect_combined_tallying_with_multi_replacing_is_a_later_rung() {
    // The COMBINED `TALLYING … REPLACING` form with SEVERAL replace items stays
    // rejected exactly as today — multi-item does not leak into the combined path.
    let src = wrap(
        &["01  S  PIC X(5) VALUE \"abcab\".", "01  K  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING K FOR ALL \"a\"",
            "    REPLACING ALL \"a\" BY \"x\" ALL \"b\" BY \"y\".",
            "STOP RUN.",
        ],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject combined + multi-item REPLACING");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject combined + multi-item REPLACING"
    );
}

// INSPECT … TALLYING counter FOR ALL a ALL b [ALL d …] — TWO OR MORE `FOR ALL`
// items under ONE counter. One left-to-right pass with FIRST-MATCH-PER-POSITION into
// the shared counter, so a position is counted at most once even when several (or
// duplicate) delimiters would match it — duplicates never double-count. Each case
// pins the exact counter and `assert_matches_oracle` independently re-checks JIT ==
// tree-walk oracle.

#[test]
fn inspect_tally_multi_two_delims() {
    // "abcab" counting ALL "a" ALL "b" into one counter: a,b,_,a,b → 4 (c ignored).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"abcab\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"a\" ALL \"b\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "004\n");
}

#[test]
fn inspect_tally_multi_three_delims() {
    // Three delimiters over "abcabc": every position matches one of a/b/c → 6.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"abcabc\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"a\"",
            "    ALL \"b\" ALL \"c\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "006\n");
}

#[test]
fn inspect_tally_multi_duplicate_delim_counts_each_once() {
    // THE key correctness case. `FOR ALL "a" ALL "a"` over "aa" adds 2, NOT 4: each
    // 'a' position is counted ONCE by the first item — the per-position break means
    // the second (duplicate) item never fires there. A naive "sum of independent
    // per-delimiter counts" would give 4; this pins the single-pass first-match rule.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(2) VALUE \"aa\".", "01  C  PIC 9(2) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"a\" ALL \"a\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "02\n");
}

#[test]
fn inspect_tally_multi_disjoint_delims() {
    // Disjoint delimiters over "a1b2a3": only the a's and b's match → 3, the digits
    // are ignored, and both delimiters fold into the SAME counter.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"a1b2a3\".", "01  C  PIC 9(2) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"a\" ALL \"b\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "03\n");
}

#[test]
fn inspect_tally_multi_char_matched_by_no_delim() {
    // 'Q' matches neither delimiter → it is skipped among counted neighbours: "aQbQa"
    // counts a,b,a = 3.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aQbQa\".", "01  C  PIC 9(2) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"a\" ALL \"b\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "03\n");
}

#[test]
fn inspect_tally_multi_source_all_matching() {
    // Every character of "aabb" matches one of the two delimiters → 4.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"aabb\".", "01  C  PIC 9(2) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"a\" ALL \"b\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "04\n");
}

#[test]
fn inspect_tally_single_item_still_works() {
    // Regression: exactly ONE `FOR` item keeps the single-item path unchanged (the
    // multi dispatch fires only at >= 2 items). "ABABA" has three A's → 3.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"ABABA\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"A\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "003\n");
}

#[test]
fn inspect_tally_multi_pic_x1_delimiters() {
    // The delimiters in a multi list may be PIC X(1) items (their single stored
    // character), read at run time: ALL DL1 ALL DL2 over "abcab" counts a,b,_,a,b = 4.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S   PIC X(5) VALUE \"abcab\".",
            "01  DL1 PIC X(1) VALUE \"a\".",
            "01  DL2 PIC X(1) VALUE \"b\".",
            "01  C   PIC 9(2) VALUE 0.",
        ],
        &["INSPECT S TALLYING C FOR ALL DL1 ALL DL2.", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "04\n");
}

#[test]
fn inspect_tally_multi_counter_overflow_truncates() {
    // 12 matches (six a's + six b's) added into a PIC 9(1) counter overflows: COBOL's
    // silent high-order truncation keeps the low digit → 12 mod 10 = 2.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(12) VALUE \"aaaaaabbbbbb\".", "01  C  PIC 9(1) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"a\" ALL \"b\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "2\n");
}

#[test]
fn inspect_tally_multi_adds_to_a_nonzero_counter() {
    // C starts at 5; "MISSISSIPPI" has four S's and two P's → six matched positions →
    // C = 5 + 6 = 11 (proves ADD into the shared counter, not a fresh assignment).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(11) VALUE \"MISSISSIPPI\".", "01  C  PIC 9(3) VALUE 5."],
        &["INSPECT S TALLYING C FOR ALL \"S\" ALL \"P\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "011\n");
}

// A `{BEFORE|AFTER}` region on each item of a MULTI-item tally list is now SUPPORTED
// (this rung): each item carries its OWN window over the source, and one first-match-
// per-position pass counts a position for the FIRST item whose window contains it AND
// whose delimiter matches the current char. Each case pins the exact counter and
// `assert_matches_oracle` independently re-checks JIT == tree-walk oracle.

#[test]
fn inspect_tally_multi_two_items_before_and_after() {
    // Two items, one BEFORE one AFTER, over a source where the windows differ.
    // Source "aXaXa" (X at char indices 1 and 3), length 5.
    //   item 1 `ALL "a" BEFORE "X"`: first "X" at index 1 → window [0,1) → only the
    //           "a" at index 0 is inside.
    //   item 2 `ALL "a" AFTER "X"`:  first "X" at index 1 → window [2,5) → the "a"s at
    //           indices 2 and 4 are inside.
    // Position by position: 0→item1, 2→item2, 4→item2 (the two "X"s match neither) → 3.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aXaXa\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"a\" BEFORE \"X\"",
            "    ALL \"a\" AFTER \"X\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "003\n");
}

#[test]
fn inspect_tally_multi_region_plus_regionless_item() {
    // A region item mixed with a whole-source (region-less) item. Source "0a0a0".
    //   item 1 `ALL "0" AFTER "a"`: first "a" at index 1 → window [2,5) → the "0"s at
    //           indices 2 and 4 (not the "0" at index 0, which is outside).
    //   item 2 `ALL "a"` (no region): whole source → both "a"s at indices 1 and 3.
    // Count: index 1 (item2 "a"), 2 (item1 "0"), 3 (item2 "a"), 4 (item1 "0") → 4;
    // index 0's "0" is outside item 1's window and item 2 does not match it.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0a0a0\".", "01  C  PIC 9(2) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" AFTER \"a\"",
            "    ALL \"a\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "04\n");
}

#[test]
fn inspect_tally_multi_after_absent_delimiter_empty_window() {
    // `AFTER x` where x is ABSENT → an EMPTY window, so that item contributes 0; the
    // other (region-less) item still counts everywhere. Source "abab":
    //   item 1 `ALL "a" AFTER "Z"` — no "Z" present → empty window → never fires;
    //   item 2 `ALL "b"` (no region) → the two "b"s at indices 1 and 3 → 2.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"abab\".", "01  C  PIC 9(2) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"a\" AFTER \"Z\"",
            "    ALL \"b\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "02\n");
}

#[test]
fn inspect_tally_multi_duplicate_windows_first_match_counts_once() {
    // FIRST-MATCH-PER-POSITION with DUPLICATE delimiters over DIFFERENT but OVERLAPPING
    // windows: a position matched by BOTH items is counted ONCE, by the earlier item.
    // Source "aabaa" (b at index 2), same delimiter "a" on both items:
    //   item 1 `ALL "a" BEFORE "b"`: window [0,2) → indices 0,1;
    //   item 2 `ALL "a"` (whole source): indices 0,1,3,4.
    // Indices 0 and 1 are inside BOTH windows and match BOTH items, but the per-position
    // break counts each ONCE (item 1). item 2 then claims indices 3 and 4. Total = 4,
    // NOT 6 — a naive per-item sum (2 + 4) would double-count indices 0 and 1.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aabaa\".", "01  C  PIC 9(2) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"a\" BEFORE \"b\"",
            "    ALL \"a\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "04\n");
}

#[test]
fn inspect_tally_multi_non_ascii_source_positive_parity() {
    // POSITIVE non-ASCII byte-identity parity (NOT a trap): unlike REPLACING, TALLYING
    // only COUNTS — it never reconstructs the source via `str_slice` — so there is no
    // UTF-8-boundary trap. Match-based counting of ASCII delimiters is byte-robust (a
    // multi-byte "é" = bytes 0xC3 0xA9 never equals an ASCII delimiter byte), and each
    // window is content-defined (bounded by the first occurrence of the ASCII region
    // delimiter "b"), so the char-based oracle and the byte-based compiler scan the SAME
    // substring and count the SAME matches. Source "aé0b0" (chars a,é,0,b,0):
    //   item 1 `ALL "0" BEFORE "b"`: window left of "b" → the "0" before "b" → 1;
    //   item 2 `ALL "0" AFTER "b"`:  window right of "b" → the "0" after "b"  → 1.
    // Total = 2 on BOTH engines; the "é" (and its continuation byte) matches nothing.
    // `assert_matches_oracle` asserts the DISPLAYed counter is byte-identical.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aé0b0\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"b\"",
            "    ALL \"0\" AFTER \"b\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n");
}

// A MULTI-item tally list may now MIX `ALL` and `LEADING` items (this rung lifts the
// multi-item LEADING reject). A `LEADING` item counts only its CONSECUTIVE run anchored
// at its window start; ONE left-to-right pass carries a per-item `active` run flag
// (consulted only for LEADING items) — a LEADING item is eligible only while its run is
// alive, and every LEADING run breaks at the FIRST in-window mismatch, INDEPENDENTLY of
// which item tallied the position. Each case pins the exact counter and
// `assert_matches_oracle` independently re-checks JIT == tree-walk oracle.

#[test]
fn inspect_tally_multi_leading_then_all() {
    // `FOR LEADING "a" ALL "b"` over "aabab": the leading run of "a" is positions 0,1
    // (breaks at the "b" at index 2), then ALL "b" counts the b's at 2 and 4. The "a"
    // at index 3 is NOT counted — the leading run is already dead. 2 + 2 = 4.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aabab\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR LEADING \"a\" ALL \"b\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "004\n");
}

#[test]
fn inspect_tally_multi_all_then_leading_run_breaks_at_start() {
    // `FOR ALL "b" LEADING "a"` (ALL first, DIFFERENT delims) over "baaab": the leading
    // "a" run is anchored at SOURCE START, but position 0 is "b", so the run breaks
    // immediately and LEADING "a" counts 0. ALL "b" counts the b's at 0 and 4 → 2.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"baaab\".", "01  C  PIC 9(2) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"b\" LEADING \"a\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "02\n");
}

#[test]
fn inspect_tally_multi_all_then_leading_same_delim_run_survives_claim() {
    // THE run-stays-alive subtlety, `ALL` FIRST. `FOR ALL "a" LEADING "a"` over "aab":
    // ALL "a" claims positions 0 and 1 (count 2); the LEADING "a" item never tallies
    // (ALL wins every "a"), but crucially the active-flag update KEEPS the leading run
    // alive at 0 and 1 (each char equals "a"), so a matching char claimed by a
    // higher-priority item does NOT break the leading run — the run only decays at the
    // "b" (index 2). Count = 2 (a naive "break the run whenever this item didn't tally"
    // impl would still give 2 here, but this pins the correct active-update wiring).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"aab\".", "01  C  PIC 9(2) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"a\" LEADING \"a\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "02\n");
}

#[test]
fn inspect_tally_multi_leading_then_all_same_delim() {
    // The run-stays-alive subtlety, `LEADING` FIRST. `FOR LEADING "a" ALL "a"` over
    // "aab": the LEADING item (higher priority) claims positions 0,1 as its run, and ALL
    // "a" never sees them (first-match). The run decays at the "b". Count = 2. Pins that
    // the leading eligibility gate and the run-decay compose correctly when a duplicate
    // ALL item of the SAME delimiter follows.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"aab\".", "01  C  PIC 9(2) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR LEADING \"a\" ALL \"a\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "02\n");
}

#[test]
fn inspect_tally_multi_leading_with_region_plus_all() {
    // A LEADING item WITH a region + an ALL item — the leading run is anchored at the
    // WINDOW START, not source position 0. `FOR LEADING "a" AFTER "X" ALL "b"` over
    // "aaXaab" (X at index 2): the window for the leading item is "aab" (indices 3..6),
    // so the two "a"s BEFORE the X (indices 0,1) are IGNORED and the run counts the two
    // "a"s at 3,4 (breaks at the "b" at 5). ALL "b" counts the "b" at 5. 2 + 1 = 3.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"aaXaab\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR LEADING \"a\" AFTER \"X\"",
            "    ALL \"b\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "003\n");
}

#[test]
fn inspect_tally_multi_two_leading_items_different_delims() {
    // Two LEADING items with DIFFERENT delimiters, no region. Both runs anchor at source
    // start, but position 0 can equal only ONE delimiter, so the OTHER run breaks
    // immediately. `FOR LEADING "a" LEADING "b"` over "aabb": LEADING "a" counts the run
    // at 0,1 = 2; LEADING "b"'s run breaks at index 0 ("a" != "b") → 0. Total 2.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"aabb\".", "01  C  PIC 9(2) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR LEADING \"a\" LEADING \"b\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "02\n");
}

#[test]
fn inspect_tally_multi_two_leading_items_disjoint_windows() {
    // Two LEADING items with DIFFERENT delimiters AND disjoint windows, so BOTH runs can
    // count — each anchored at its OWN window start. `FOR LEADING "a" BEFORE "X"
    // LEADING "b" AFTER "X"` over "aaXbb" (X at index 2): item 1's window is "aa" (the
    // two leading a's → 2), item 2's window is "bb" (the two leading b's → 2). Total 4.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aaXbb\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR LEADING \"a\" BEFORE \"X\"",
            "    LEADING \"b\" AFTER \"X\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "004\n");
}

#[test]
fn inspect_tally_multi_leading_non_ascii_source_positive_parity() {
    // POSITIVE non-ASCII byte-identity parity WITH a LEADING item (NOT a trap): TALLYING
    // only COUNTS, so there is no UTF-8-boundary trap. The multi-byte "é" (bytes
    // 0xC3 0xA9) equals NEITHER ASCII delimiter, and it BREAKS the leading run at the
    // SAME logical position on both engines — the oracle's char scan breaks at the "é"
    // (char index 2), the compiler's byte scan breaks at 0xC3 (byte index 2), and the
    // continuation byte 0xA9 (byte 3) also matches nothing. Source "aaébb" (chars
    // a,a,é,b,b):
    //   item 1 `LEADING "a"`: the leading run "aa" (breaks at "é") → 2;
    //   item 2 `ALL "b"`:     the two "b"s after "é" → 2.
    // Total = 4 on BOTH engines. `assert_matches_oracle` asserts the DISPLAYed counter is
    // byte-identical.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"aaébb\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR LEADING \"a\" ALL \"b\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "004\n");
}

// A MULTI-item tally list may now include a `CHARACTERS` item (THIS rung lifts the
// multi-item CHARACTERS reject). `CHARACTERS` is the always-eligible catch-all: at each
// in-window position not already claimed by an EARLIER item in written order it adds 1 to
// the shared counter — NO delimiter compare, NO leading-run tracking. An optional
// `{BEFORE|AFTER}` region narrows its window exactly like any other item. Each case pins
// the exact counter and `assert_matches_oracle` independently re-checks JIT == tree-walk
// oracle. (Only the MULTI-COUNTER and COMBINED forms still defer CHARACTERS — see
// `inspect_tally_counters_with_characters_is_a_later_rung` and
// `inspect_tally_multi_combined_with_characters_is_a_later_rung`.)

#[test]
fn inspect_tally_multi_all_then_characters_covers_the_rest() {
    // `FOR ALL "A" CHARACTERS` over "ABAB" (length 4). ALL "A" (item 0, higher priority)
    // claims the two "A"s at positions 0,2; the CHARACTERS catch-all (item 1) claims every
    // OTHER in-window position (1,3). Every position is claimed by exactly one item, so the
    // total is the SOURCE LENGTH = 4. Pins that CHARACTERS counts exactly the positions an
    // earlier item did not.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"ABAB\".", "01  C  PIC 9(2) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"A\" CHARACTERS.", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "04\n");
}

#[test]
fn inspect_tally_multi_characters_first_shadows_the_all_item() {
    // WRITTEN-ORDER priority: `FOR CHARACTERS ALL "A"` (CHARACTERS FIRST) over "ABAB". The
    // region-less CHARACTERS catch-all is eligible at EVERY position, so it claims all 4
    // and the following ALL "A" NEVER fires (first-match-per-position). Count = length = 4,
    // exactly as if the ALL item were absent — proving CHARACTERS' position in the list is
    // honoured (a lower-priority ALL of a matching delimiter is shadowed).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"ABAB\".", "01  C  PIC 9(2) VALUE 0."],
        &["INSPECT S TALLYING C FOR CHARACTERS ALL \"A\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "04\n");
}

#[test]
fn inspect_tally_multi_characters_in_the_middle_shadows_multiple_trailing_items() {
    // WRITTEN-ORDER priority with a region-less CHARACTERS in the MIDDLE of THREE items:
    // `FOR ALL "A" CHARACTERS ALL "B"` over "ABAB". Item 0 (ALL "A") claims the two 'A's;
    // the region-less CHARACTERS catch-all then claims EVERY remaining position, so item 2
    // (ALL "B") is UNREACHABLE and never fires. Count = length = 4. This exercises the
    // compiler's unreachable-block emission for MORE THAN ONE trailing chain link after an
    // unconditional catch-all — each dead link is a self-contained block ending in its own
    // `jmp cont`, so the shadowing matches the oracle's first-eligible rule byte-for-byte.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"ABAB\".", "01  C  PIC 9(2) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"A\" CHARACTERS ALL \"B\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "04\n");
}

#[test]
fn inspect_tally_multi_all_then_characters_with_before_region() {
    // A CHARACTERS item WITH a `{BEFORE|AFTER}` region narrows its window exactly like any
    // other item. `FOR ALL "A" CHARACTERS BEFORE "X"` over "ABXAB" (X at index 2):
    //   pos 0 'A' → ALL "A" claims                                   (1)
    //   pos 1 'B' → ALL no; CHARACTERS window [0,2) contains 1 → claims (2)
    //   pos 2 'X' → ALL no; CHARACTERS window [0,2) excludes 2 → NOT counted
    //   pos 3 'A' → ALL "A" claims                                   (3)
    //   pos 4 'B' → ALL no; CHARACTERS window [0,2) excludes 4 → NOT counted
    // Total = 3 (NOT the length) — the region genuinely bounds the catch-all.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"ABXAB\".", "01  C  PIC 9(2) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"A\" CHARACTERS BEFORE \"X\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "03\n");
}

#[test]
fn inspect_tally_multi_leading_then_characters_run_tracking_unaffected() {
    // A CHARACTERS item alongside a LEADING item — the LEADING run tracking is UNAFFECTED
    // by the (delimiter-less, run-less) CHARACTERS item. `FOR LEADING "A" CHARACTERS
    // BEFORE "X"` over "AABAX" (X at index 4):
    //   pos 0 'A' → LEADING run alive → claims                          (1)
    //   pos 1 'A' → LEADING run alive → claims                          (2)
    //   pos 2 'B' → LEADING no (mismatch, run breaks here); CHARACTERS window [0,4)
    //               contains 2 → claims                                 (3)
    //   pos 3 'A' → LEADING run now DEAD → no; CHARACTERS window contains 3 → claims (4)
    //   pos 4 'X' → LEADING dead; CHARACTERS window [0,4) excludes 4 → NOT counted
    // Total = 4. The LEADING run still counts exactly its anchored run (2) and breaks at
    // the first in-window mismatch, INDEPENDENTLY of the CHARACTERS item claiming that same
    // position — the active-run update never consults or is consulted by CHARACTERS.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AABAX\".", "01  C  PIC 9(2) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR LEADING \"A\" CHARACTERS BEFORE \"X\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "04\n");
}

#[test]
fn inspect_tally_multi_characters_non_ascii_outside_window_is_co_total() {
    // NON-ASCII byte-vs-char count chip (task_396ba6f6), SAME as the single `FOR
    // CHARACTERS` form (#60): a `CHARACTERS` item counts POSITIONS, and the compiler
    // iterates BYTE positions (`str_len`) while the oracle's multi-item exec iterates CHAR
    // positions (`chars.len()`). They DIVERGE if a multi-byte char falls inside a
    // CHARACTERS window. We keep this case CO-TOTAL by placing the "é" strictly OUTSIDE
    // every window so both engines agree. `FOR ALL "0" CHARACTERS BEFORE "b"` over "a0bé0"
    // (chars a,0,b,é,0 = bytes a,0,b,0xC3,0xA9,0; "b" at char/byte index 2):
    //   * CHARACTERS window is `[0, 2)` (before "b") — covers only ASCII 'a','0', where a
    //     byte count and a char count coincide;
    //   * the "é" (char 3 / bytes 3,4) sits AFTER "b", outside the CHARACTERS window, and
    //     equals neither ALL "0" nor the region delimiter, so NEITHER engine counts it.
    // Both engines count: 'a' (CHARACTERS) + '0'@1 (ALL) + '0'@last (ALL) = 3.
    // `assert_matches_oracle` panics on any divergence, so this pins the co-total agreement
    // (and documents the chip that a multi-byte char INSIDE a CHARACTERS window would
    // surface — that divergence is pre-existing and NOT fixed here).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"a0bé0\".", "01  C  PIC 9(2) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" CHARACTERS BEFORE \"b\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "03\n");
}

#[test]
fn inspect_tally_multi_combined_with_characters_is_a_later_rung() {
    // The COMBINED `TALLYING … REPLACING` form still DEFERS a CHARACTERS item in its tally
    // half — rejected identically on both engines. (A combined tally is read by the
    // single-item reader, which rejects a multi-item list outright; the multi-item
    // CHARACTERS lift does NOT leak into the combined path.)
    let src = wrap(
        &["01  S  PIC X(4) VALUE \"AABB\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"A\" CHARACTERS",
            "    REPLACING ALL \"B\" BY \"C\".",
            "STOP RUN.",
        ],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject combined + multi-item CHARACTERS");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject combined + multi-item CHARACTERS"
    );
}

// INSPECT … TALLYING c1 FOR ALL a [ALL b …] c2 FOR ALL d … — TWO OR MORE `tally_for`
// groups, each with its OWN counter. ISO COMBINED-priority-list-ACROSS-COUNTERS
// semantics: ALL delimiters of ALL groups form ONE ordered list scanned in a SINGLE
// left-to-right pass; at each position the first matching delimiter (group-1's items
// first, then group-2's, …) bumps ITS OWN counter and the scan advances — so a position
// CLAIMED by an earlier group NEVER reaches a later group. Each case pins the exact
// counters and `assert_matches_oracle` independently re-checks JIT == tree-walk oracle.

#[test]
fn inspect_tally_counters_two_distinct_delims() {
    // Two counters, disjoint delimiters over "abcab": C1 counts a,a = 2 and C2 counts
    // b,b = 2 (c matches neither). Each delimiter bumps its OWN counter.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S   PIC X(5) VALUE \"abcab\".",
            "01  C1  PIC 9(3) VALUE 0.",
            "01  C2  PIC 9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL \"a\"",
            "    C2 FOR ALL \"b\".",
            "DISPLAY C1.",
            "DISPLAY C2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n002\n");
}

#[test]
fn inspect_tally_counters_same_delim_first_group_wins() {
    // THE crux case. Two counters, the SAME delimiter "a" over "aa": the combined
    // priority list is [g0:a, g1:a]; each 'a' position matches g0 FIRST and the scan
    // advances, so g1 never fires. C1 += 2, C2 += 0 — NOT 2 and 2. This pins that an
    // earlier group consumes the position across counters.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S   PIC X(2) VALUE \"aa\".",
            "01  C1  PIC 9(2) VALUE 0.",
            "01  C2  PIC 9(2) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL \"a\"",
            "    C2 FOR ALL \"a\".",
            "DISPLAY C1.",
            "DISPLAY C2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "02\n00\n");
}

#[test]
fn inspect_tally_counters_group_with_multiple_items() {
    // A group carrying TWO items (`C1 FOR ALL "a" ALL "b"`) plus a second counter
    // (`C2 FOR ALL "c"`) over "abcabc": C1 counts a,b,a,b = 4 and C2 counts c,c = 2.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S   PIC X(6) VALUE \"abcabc\".",
            "01  C1  PIC 9(3) VALUE 0.",
            "01  C2  PIC 9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL \"a\" ALL \"b\"",
            "    C2 FOR ALL \"c\".",
            "DISPLAY C1.",
            "DISPLAY C2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "004\n002\n");
}

#[test]
fn inspect_tally_counters_starves_later_group() {
    // The ISO "aba" example: `C1 FOR ALL "a" ALL "b" C2 FOR ALL "a"` over "aba". Group 1
    // (items a,b) claims ALL three positions → C1 += 3; C2's "a" never reaches a
    // position → C2 += 0.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S   PIC X(3) VALUE \"aba\".",
            "01  C1  PIC 9(3) VALUE 0.",
            "01  C2  PIC 9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL \"a\" ALL \"b\"",
            "    C2 FOR ALL \"a\".",
            "DISPLAY C1.",
            "DISPLAY C2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "003\n000\n");
}

#[test]
fn inspect_tally_counters_three_counters() {
    // Three counters, one delimiter each, over "abcabc": C1=a,a=2, C2=b,b=2, C3=c,c=2.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S   PIC X(6) VALUE \"abcabc\".",
            "01  C1  PIC 9(3) VALUE 0.",
            "01  C2  PIC 9(3) VALUE 0.",
            "01  C3  PIC 9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL \"a\"",
            "    C2 FOR ALL \"b\"",
            "    C3 FOR ALL \"c\".",
            "DISPLAY C1.",
            "DISPLAY C2.",
            "DISPLAY C3.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n002\n002\n");
}

#[test]
fn inspect_tally_counters_char_matched_by_no_group() {
    // 'X' matches no group's delimiter → it advances with no increment: "aXbXa" gives
    // C1 (a) = 2 and C2 (b) = 1.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S   PIC X(5) VALUE \"aXbXa\".",
            "01  C1  PIC 9(3) VALUE 0.",
            "01  C2  PIC 9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL \"a\"",
            "    C2 FOR ALL \"b\".",
            "DISPLAY C1.",
            "DISPLAY C2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n001\n");
}

#[test]
fn inspect_tally_counters_overflow_truncates_per_counter() {
    // Each counter truncates INDEPENDENTLY via its own store path: 12 a's into a
    // PIC 9(1) C1 overflows (12 mod 10 = 2); 3 b's into a PIC 9(3) C2 = 003.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S   PIC X(15) VALUE \"aaaaaaaaaaaabbb\".",
            "01  C1  PIC 9(1) VALUE 0.",
            "01  C2  PIC 9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL \"a\"",
            "    C2 FOR ALL \"b\".",
            "DISPLAY C1.",
            "DISPLAY C2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "2\n003\n");
}

#[test]
fn inspect_tally_counters_add_to_nonzero_counters() {
    // Distinct counters ACCUMULATE independently onto their starting values (INSPECT
    // adds; it does not clear). C1 starts 5, C2 starts 10; "MISSISSIPPI" has four S's
    // and two P's → C1 = 5 + 4 = 9, C2 = 10 + 2 = 12.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S   PIC X(11) VALUE \"MISSISSIPPI\".",
            "01  C1  PIC 9(3) VALUE 5.",
            "01  C2  PIC 9(3) VALUE 10.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL \"S\"",
            "    C2 FOR ALL \"P\".",
            "DISPLAY C1.",
            "DISPLAY C2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "009\n012\n");
}

#[test]
fn inspect_tally_counters_pic_x1_delimiters_across_groups() {
    // The delimiters may be PIC X(1) items read at run time, one per group: ALL DL1 into
    // C1, ALL DL2 into C2, over "abcab" → C1 = 2 (a), C2 = 2 (b).
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S   PIC X(5) VALUE \"abcab\".",
            "01  DL1 PIC X(1) VALUE \"a\".",
            "01  DL2 PIC X(1) VALUE \"b\".",
            "01  C1  PIC 9(3) VALUE 0.",
            "01  C2  PIC 9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL DL1",
            "    C2 FOR ALL DL2.",
            "DISPLAY C1.",
            "DISPLAY C2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n002\n");
}

#[test]
fn inspect_tally_counters_same_counter_name_in_two_groups() {
    // The SAME counter name may appear in two groups — a rare but legal form. Both
    // groups' matches ADD to that ONE item: `C FOR ALL "a" C FOR ALL "b"` over "abab"
    // gives 2 a's + 2 b's = C = 4 (each group resolves the counter by name at its add).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"abab\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"a\"",
            "    C FOR ALL \"b\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "004\n");
}

#[test]
fn inspect_tally_counters_single_counter_still_works() {
    // Regression: exactly ONE `tally_for` keeps the single-counter paths unchanged (the
    // multi-COUNTER dispatch fires only at >= 2 groups). One item → the `Inspect` path.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"ABABA\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"A\".", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "003\n");
}

#[test]
fn inspect_tally_counters_with_leading_item_is_a_later_rung() {
    // A LEADING item in ANY group of the multi-COUNTER path is deferred — rejected on
    // BOTH engines (a LONE `FOR LEADING` is still supported via the single path).
    let src = wrap(
        &[
            "01  S   PIC X(4) VALUE \"aabb\".",
            "01  C1  PIC 9(3) VALUE 0.",
            "01  C2  PIC 9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL \"a\"",
            "    C2 FOR LEADING \"b\".",
            "STOP RUN.",
        ],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a multi-counter LEADING item");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a multi-counter LEADING item"
    );
}

// INSPECT … TALLYING c1 FOR ALL a [{BEFORE|AFTER} p] [ALL b …] c2 FOR ALL d … — the
// several-counters form where each item may now carry its OWN `{BEFORE|AFTER}` region
// window (this rung LIFTS the multi-counter region reject). Each item's window narrows
// the positions its delimiter is eligible at; the combined priority list still scans in
// ONE left-to-right pass, first in-window match across counters wins. Each case pins the
// exact counters and `assert_matches_oracle` independently re-checks JIT == oracle.

#[test]
fn inspect_tally_counters_two_items_mixed_before_after() {
    // Two counter groups, one BEFORE one AFTER, over "aXaXa" (X at char indices 1 and 3):
    //   C1 `ALL "a" BEFORE "X"`: first X at index 1 → window [0,1) → only the "a" at 0.
    //   C2 `ALL "a" AFTER "X"`:  first X at index 1 → window [2,5) → the "a"s at 2 and 4.
    // Position by position (combined list [g0, g1]): 0→C1, 2→C2, 4→C2. So C1=1, C2=2.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S   PIC X(5) VALUE \"aXaXa\".",
            "01  C1  PIC 9(3) VALUE 0.",
            "01  C2  PIC 9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL \"a\" BEFORE \"X\"",
            "    C2 FOR ALL \"a\" AFTER \"X\".",
            "DISPLAY C1.",
            "DISPLAY C2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "001\n002\n");
}

#[test]
fn inspect_tally_counters_region_item_plus_regionless_item() {
    // A region-carrying item in one group + a region-LESS item in another. Source "0a0a0":
    //   C1 `ALL "0" AFTER "a"`: first "a" at index 1 → window [2,5) → the "0"s at 2 and 4.
    //   C2 `ALL "a"` (no region): whole source → the "a"s at indices 1 and 3.
    // Walk: 1→C2, 2→C1, 3→C2, 4→C1; index 0's "0" is outside C1's window and C2 doesn't
    // match it. So C1=2, C2=2.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S   PIC X(5) VALUE \"0a0a0\".",
            "01  C1  PIC 9(3) VALUE 0.",
            "01  C2  PIC 9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL \"0\" AFTER \"a\"",
            "    C2 FOR ALL \"a\".",
            "DISPLAY C1.",
            "DISPLAY C2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n002\n");
}

#[test]
fn inspect_tally_counters_after_absent_delimiter_empty_window() {
    // `AFTER x` where x is ABSENT → an EMPTY window, so that group's item contributes 0;
    // the other (region-less) group still counts. Source "abab":
    //   C1 `ALL "a" AFTER "Z"` — no "Z" → empty window → never fires → C1=0;
    //   C2 `ALL "b"` (no region) → the two "b"s at 1 and 3 → C2=2.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S   PIC X(4) VALUE \"abab\".",
            "01  C1  PIC 9(3) VALUE 0.",
            "01  C2  PIC 9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL \"a\" AFTER \"Z\"",
            "    C2 FOR ALL \"b\".",
            "DISPLAY C1.",
            "DISPLAY C2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "000\n002\n");
}

#[test]
fn inspect_tally_counters_earlier_window_starves_later_group() {
    // CROSS-COUNTER first-match with a WINDOW: an earlier group's IN-WINDOW delimiter
    // claims a position, starving a later group's delimiter of it. Source "aZa" (Z at
    // index 1), both groups matching "a":
    //   C1 `ALL "a" BEFORE "Z"`: first Z at index 1 → window [0,1) → only index 0;
    //   C2 `ALL "a"` (whole source): indices 0 and 2.
    // Walk: index 0's "a" is inside C1's window → C1 claims it (C2 never sees it); index
    // 2's "a" is OUTSIDE C1's window → falls to C2. So C1=1, C2=1 — NOT C2=2. The proof
    // that C1's in-window delimiter starved C2 of index 0.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S   PIC X(3) VALUE \"aZa\".",
            "01  C1  PIC 9(3) VALUE 0.",
            "01  C2  PIC 9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL \"a\" BEFORE \"Z\"",
            "    C2 FOR ALL \"a\".",
            "DISPLAY C1.",
            "DISPLAY C2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "001\n001\n");
}

#[test]
fn inspect_tally_counters_same_counter_two_groups_with_regions() {
    // The SAME counter name in two groups, EACH with its own region — both regions' hits
    // ADD to that one item. Source "0b0" (b at index 1):
    //   group0 `C FOR ALL "0" BEFORE "b"`: window [0,1) → the "0" at index 0;
    //   group1 `C FOR ALL "0" AFTER "b"`:  window [2,3) → the "0" at index 2.
    // Each group's accumulator = 1; both add to C → C = 0 + 1 + 1 = 2.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"0b0\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"b\"",
            "    C FOR ALL \"0\" AFTER \"b\".",
            "DISPLAY C.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n");
}

#[test]
fn inspect_tally_counters_non_ascii_source_positive_parity() {
    // POSITIVE non-ASCII byte-identity parity (NOT a trap): TALLYING only COUNTS — it
    // never reconstructs the source via `str_slice` — so there is no UTF-8-boundary trap.
    // ASCII delimiters never equal a multi-byte continuation byte, and each item's window
    // is content-defined (bounded by the first "b"), so the char-based oracle and the
    // byte-based compiler scan the SAME substring and count the SAME matches. Source
    // "aé0b0" (chars a, é, 0, b, 0), two counter groups with per-item regions:
    //   C1 `ALL "0" BEFORE "b"`: window left of "b" → the "0" before "b" → 1;
    //   C2 `ALL "0" AFTER "b"`:  window right of "b" → the "0" after "b"  → 1.
    // C1=1, C2=1 on BOTH engines; the "é" (and its continuation byte) matches nothing.
    // `assert_matches_oracle` asserts the DISPLAYed counters are byte-identical.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S   PIC X(5) VALUE \"aé0b0\".",
            "01  C1  PIC 9(3) VALUE 0.",
            "01  C2  PIC 9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL \"0\" BEFORE \"b\"",
            "    C2 FOR ALL \"0\" AFTER \"b\".",
            "DISPLAY C1.",
            "DISPLAY C2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "001\n001\n");
}

#[test]
fn inspect_tally_counters_with_characters_is_a_later_rung() {
    // A `CHARACTERS` item in any group is deferred on BOTH engines.
    let src = wrap(
        &[
            "01  S   PIC X(4) VALUE \"aabb\".",
            "01  C1  PIC 9(3) VALUE 0.",
            "01  C2  PIC 9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL \"a\"",
            "    C2 FOR CHARACTERS.",
            "STOP RUN.",
        ],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a multi-counter CHARACTERS item");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a multi-counter CHARACTERS item"
    );
}

#[test]
fn inspect_tally_counters_signed_counter_is_a_later_rung() {
    // A signed (`PIC S9`) counter in ANY group is deferred — every counter must be an
    // unsigned integer, validated identically on BOTH engines.
    let src = wrap(
        &[
            "01  S   PIC X(4) VALUE \"aabb\".",
            "01  C1  PIC 9(3) VALUE 0.",
            "01  C2  PIC S9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL \"a\"",
            "    C2 FOR ALL \"b\".",
            "STOP RUN.",
        ],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a signed counter");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a signed counter"
    );
}

#[test]
fn inspect_tally_counters_fractional_counter_is_a_later_rung() {
    // A fractional (`PIC 9V9`) counter in any group is deferred on BOTH engines.
    let src = wrap(
        &[
            "01  S   PIC X(4) VALUE \"aabb\".",
            "01  C1  PIC 9(3) VALUE 0.",
            "01  C2  PIC 9V9 VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL \"a\"",
            "    C2 FOR ALL \"b\".",
            "STOP RUN.",
        ],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a fractional counter");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a fractional counter"
    );
}

#[test]
fn inspect_combined_several_counters_with_replacing_is_a_later_rung() {
    // The COMBINED `TALLYING … REPLACING` form with SEVERAL counters stays rejected
    // exactly as today — the multi-COUNTER relaxation is confined to the LONE TALLYING
    // form and does not leak into the combined path (which still routes through the
    // several-counters reject in `read_inspect_tally_all`/`inspect_tally_all`).
    let src = wrap(
        &[
            "01  S   PIC X(5) VALUE \"abcab\".",
            "01  C1  PIC 9(3) VALUE 0.",
            "01  C2  PIC 9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL \"a\" C2 FOR ALL \"b\"",
            "    REPLACING ALL \"a\" BY \"x\".",
            "STOP RUN.",
        ],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject combined + several counters");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject combined + several counters"
    );
}

#[test]
fn inspect_combined_multi_tallying_with_replacing_is_a_later_rung() {
    // The COMBINED `TALLYING … REPLACING` form with SEVERAL tally items stays rejected
    // exactly as today — the multi-item tally relaxation does not leak into the
    // combined path.
    let src = wrap(
        &["01  S  PIC X(5) VALUE \"abcab\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"a\" ALL \"b\"",
            "    REPLACING ALL \"a\" BY \"x\".",
            "STOP RUN.",
        ],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject combined + multi-item TALLYING");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject combined + multi-item TALLYING"
    );
}

// INSPECT … TALLYING … REPLACING with a per-half `{BEFORE|AFTER}` region — the
// combined form now accepts an INDEPENDENT single-char region on EACH half. Because
// the tally does not mutate the source, BOTH windows (the count's and the
// replacement's) are computed over the SAME original bytes, using the shared window
// helper — so the JIT output stays byte-identical to the tree-walk oracle. Every
// case pins the exact counter/source result; `assert_matches_oracle` independently
// re-checks JIT == oracle.

#[test]
fn inspect_combined_region_tally_region_only() {
    // Only the TALLYING half is narrowed: BEFORE "C" over "AB0CD0" counts the single
    // "0" in "AB0" → C = 001. The REPLACING half has NO region, so it maps BOTH "0"s.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"AB0CD0\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"C\"",
            "    REPLACING ALL \"0\" BY \"*\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "001\nAB*CD*\n");
}

#[test]
fn inspect_combined_region_replace_region_only() {
    // Only the REPLACING half is narrowed: the region-less TALLYING counts all three
    // "0"s (3), then REPLACING ALL "0" BEFORE "B" restricts to "0A0" → "*A*B0".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0A0B0\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\"",
            "    REPLACING ALL \"0\" BY \"*\" BEFORE \"B\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "003\n*A*B0\n");
}

#[test]
fn inspect_combined_region_both_before_distinct_delimiters() {
    // BOTH halves BEFORE, DIFFERENT delimiters over "0A0B0C0": tally BEFORE "B"
    // → region "0A0" → 2; replace BEFORE "C" → region "0A0B0" → the three "0"s at
    // indices 0/2/4 become "*" → "*A*B*C0".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(7) VALUE \"0A0B0C0\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"B\"",
            "    REPLACING ALL \"0\" BY \"*\" BEFORE \"C\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n*A*B*C0\n");
}

#[test]
fn inspect_combined_region_before_and_after_different_kinds() {
    // DIFFERENT kinds over "0A0B0": tally BEFORE "B" → region "0A0" → 2; replace
    // AFTER "B" → region "0" (index 4) → only the trailing "0" → "0A0B*".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0A0B0\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"B\"",
            "    REPLACING ALL \"0\" BY \"*\" AFTER \"B\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n0A0B*\n");
}

#[test]
fn inspect_combined_region_both_after() {
    // BOTH halves AFTER over "0A0B0": tally AFTER "A" → region "0B0" → two "0"s (2);
    // replace AFTER "B" → region "0" → only index 4 → "0A0B*".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0A0B0\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" AFTER \"A\"",
            "    REPLACING ALL \"0\" BY \"*\" AFTER \"B\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n0A0B*\n");
}

#[test]
fn inspect_combined_region_tally_after_not_found_replace_before_not_found() {
    // The per-half not-found asymmetry, BOTH exercised at once over "0A0": tally
    // AFTER "Z" (absent) → EMPTY window → 0; replace BEFORE "Z" (absent) → WHOLE
    // source → both "0"s → "*A*".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"0A0\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" AFTER \"Z\"",
            "    REPLACING ALL \"0\" BY \"*\" BEFORE \"Z\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "000\n*A*\n");
}

#[test]
fn inspect_combined_region_tally_before_not_found_whole_source() {
    // Tally BEFORE "Z" (absent) → WHOLE source → both "0"s (2); replace AFTER "A"
    // → region "0" (index 2) → only the trailing "0" → "0A*".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"0A0\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"Z\"",
            "    REPLACING ALL \"0\" BY \"*\" AFTER \"A\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n0A*\n");
}

#[test]
fn inspect_combined_region_replace_after_not_found_empty() {
    // Replace AFTER "Z" (absent) → EMPTY window → NOTHING replaced, source unchanged;
    // the region-less tally still counts both "0"s (2).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"0A0\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\"",
            "    REPLACING ALL \"0\" BY \"*\" AFTER \"Z\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n0A0\n");
}

#[test]
fn inspect_combined_region_delimiter_at_last_position() {
    // Region delimiter is the LAST character over "0A0X": tally BEFORE "X" → region
    // "0A0" → 2; replace BEFORE "X" → region "0A0" → both "0"s → "*A*X".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"0A0X\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"X\"",
            "    REPLACING ALL \"0\" BY \"*\" BEFORE \"X\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n*A*X\n");
}

#[test]
fn inspect_combined_region_delimiter_equals_search_char_each_half() {
    // Each half's region delimiter EQUALS its own search char over "0A0B": tally
    // AFTER "0" → first "0" at index 0 bounds region "A0B" → one "0"; replace AFTER
    // "0" → same region → index 2 → "0A*B". The two halves are independent but land
    // on the same window here.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"0A0B\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" AFTER \"0\"",
            "    REPLACING ALL \"0\" BY \"*\" AFTER \"0\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "001\n0A*B\n");
}

#[test]
fn inspect_combined_region_pic_x1_delimiter() {
    // The TALLYING half's region delimiter is a PIC X(1) item read at run time:
    // BEFORE DL (="C") over "AB0CD0" → region "AB0" → one "0"; the region-less
    // REPLACING maps both "0"s → "AB*CD*".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S   PIC X(6) VALUE \"AB0CD0\".",
            "01  C   PIC 9(3) VALUE 0.",
            "01  DL  PIC X(1) VALUE \"C\".",
        ],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" BEFORE DL",
            "    REPLACING ALL \"0\" BY \"*\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "001\nAB*CD*\n");
}

#[test]
fn inspect_combined_region_adds_to_a_nonzero_counter() {
    // The combined region form still ADDs to a preloaded counter: C starts at 5,
    // tally BEFORE "A" over "0A0" → region "0" → one "0" → C = 5 + 1 = 006; the
    // region-less REPLACING maps both "0"s → "*A*".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"0A0\".", "01  C  PIC 9(3) VALUE 5."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"A\"",
            "    REPLACING ALL \"0\" BY \"*\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "006\n*A*\n");
}

#[test]
fn inspect_combined_region_delimiter_at_position_zero_empty_before() {
    // BEFORE with the delimiter at index 0 gives an EMPTY window on that half: tally
    // BEFORE "0" over "0A0B0" → first "0" at index 0 → region [0,0) → 0; replace
    // BEFORE "B" → region "0A0" → the two "0"s at 0/2 → "*A*B0".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0A0B0\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"0\"",
            "    REPLACING ALL \"0\" BY \"*\" BEFORE \"B\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "000\n*A*B0\n");
}

#[test]
fn inspect_combined_for_leading_before_region_is_now_supported() {
    // (Case 1) `FOR LEADING` carrying a region on the combined TALLYING half is NOW
    // SUPPORTED (this rung lifts the old deferral). It composes the SAME standalone
    // `FOR LEADING … BEFORE/AFTER` routine the lone form uses. Over "00A0B", the
    // TALLYING window BEFORE "A" is "00" and the LEADING run anchored at the window
    // start is 2 → C = 002. The region-less REPLACING ALL "0" then rewrites EVERY "0"
    // (all three) → "**A*B". The tally's window-limited 2 vs. the replace's all-three
    // is the proof the LEADING half's region genuinely bounds only the count.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"00A0B\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR LEADING \"0\" BEFORE \"A\"",
            "    REPLACING ALL \"0\" BY \"*\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n**A*B\n");
}

#[test]
fn inspect_combined_replacing_leading_before_region_is_now_supported() {
    // (Case 3, BEFORE twin) `REPLACING LEADING` carrying a region on the combined
    // REPLACING half is NOW SUPPORTED — the substitution-side analogue. Over "00A0B",
    // the region-less TALLYING FOR ALL "0" counts all three "0"s → C = 003; the
    // REPLACING LEADING "0" BEFORE "A" then rewrites only the LEADING run inside the
    // window "00" → "**A0B" (the "0" at index 3 is outside the window and no longer
    // part of a leading run). Both halves reuse the standalone LEADING+region routines.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"00A0B\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\"",
            "    REPLACING LEADING \"0\" BY \"*\" BEFORE \"A\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "003\n**A0B\n");
}

#[test]
fn inspect_combined_multi_char_region_delimiter_is_a_later_rung() {
    // A MULTI-character region delimiter on a combined half is deferred — rejected on
    // both engines via the shared single-delimiter check (oracle at exec, compiler at
    // emit), exactly like the lone forms.
    let src = wrap(
        &["01  S  PIC X(6) VALUE \"AB0CD0\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"CD\"",
            "    REPLACING ALL \"0\" BY \"*\".",
            "STOP RUN.",
        ],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a multi-char region delimiter (combined)");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a multi-char region delimiter (combined)"
    );
}

// INSPECT … TALLYING … REPLACING — one INSPECT carrying BOTH phrases. Per ISO the
// statement runs "as though an INSPECT TALLYING were specified, followed by an
// INSPECT REPLACING": count FIRST (into the counter, over the ORIGINAL bytes),
// replace SECOND (rewriting the source). The compiler composes its two existing
// lowerings in that order and the oracle its two exec halves, so the JIT output
// stays byte-identical to the reference.

#[test]
fn inspect_combined_distinct_chars() {
    // TALLYING counts "L" (two in "HELLO") into C; REPLACING then maps O→0.
    // The two phrases touch different characters, so this pins the plain
    // count-and-substitute composition: C = 002, S = "HELL0".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"HELLO\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"L\" REPLACING ALL \"O\" BY \"0\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\nHELL0\n");
}

#[test]
fn inspect_combined_shared_char_tallies_before_replacing() {
    // The tallied delimiter and the replaced search are the SAME char ("S"). The
    // count must see the ORIGINAL source: "SASSY" has three S's → C = 003, and
    // only AFTERWARDS are they replaced by "Z" → "ZAZZY". If the tally ran after
    // the replace it would count zero — so the 3 proves tally-before-replace.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"SASSY\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"S\" REPLACING ALL \"S\" BY \"Z\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "003\nZAZZY\n");
}

#[test]
fn inspect_combined_adds_to_a_nonzero_counter() {
    // C starts at 5; "BANANA" has three A's → C = 5 + 3 = 008 (INSPECT ADDS, it
    // does not clear first — the combined form preserves that). REPLACING then
    // maps N→n → "BAnAnA".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(6) VALUE \"BANANA\".", "01  C  PIC 9(3) VALUE 5."],
        &[
            "INSPECT S TALLYING C FOR ALL \"A\" REPLACING ALL \"N\" BY \"n\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "008\nBAnAnA\n");
}

#[test]
fn inspect_combined_chars_at_both_ends() {
    // The tallied char sits at BOTH ends of the source and the replaced char in
    // between: "ABCBA" → TALLYING "A" counts the two end bytes (C = 002),
    // REPLACING "B" BY "-" rewrites the interior → "A-C-A". Exercises the loop's
    // first and last positions for both halves.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"ABCBA\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"A\" REPLACING ALL \"B\" BY \"-\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\nA-C-A\n");
}

// INSPECT … TALLYING … FOR LEADING … REPLACING ALL — the combined form's TALLYING
// half may count only the LEADING run of the delimiter (the REPLACING half here is
// ALL; the mirror REPLACING-LEADING half is exercised in the next section). Tally
// still runs FIRST over the original bytes, so a shared delimiter/search is counted
// before it is substituted. The compiler reuses the lone-TALLYING leading lowering
// and the oracle its leading count path, so the JIT output stays byte-identical.

#[test]
fn inspect_combined_for_leading_counts_only_the_leading_run() {
    // "000X0": the LEADING run of "0" is 3 (stops at 'X'), so C = 003 — NOT the 4
    // that FOR ALL would count. The delimiter and search are the SAME char, and the
    // tally runs first, so REPLACING ALL "0" BY "*" then rewrites every "0" →
    // "***X*". The 3 (not 4) is what proves the leading-run count.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"000X0\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR LEADING \"0\" REPLACING ALL \"0\" BY \"*\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "003\n***X*\n");
}

#[test]
fn inspect_combined_for_leading_no_leading_run() {
    // The first character is not the delimiter, so the LEADING run is empty →
    // C = 000 (FOR ALL would count the two later "0"s). REPLACING ALL still rewrites
    // every "0" → "X**X". Pins the "no run" boundary of the combined form.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"X00X\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR LEADING \"0\" REPLACING ALL \"0\" BY \"*\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "000\nX**X\n");
}

#[test]
fn inspect_combined_for_leading_all_characters_match() {
    // Every character is the delimiter, so the leading run spans the whole field
    // (C = 004) and REPLACING ALL rewrites all of it → "****".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"0000\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR LEADING \"0\" REPLACING ALL \"0\" BY \"*\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "004\n****\n");
}

// INSPECT … TALLYING … REPLACING LEADING — the combined form's REPLACING half may
// now be LEADING too: after the tally counts (over the ORIGINAL bytes), the rebuild
// rewrites only the consecutive run of `search` at the START, stopping at the first
// non-match. The two halves' leading flags are INDEPENDENT — either, both, or
// neither may be LEADING. The compiler reuses the lone-REPLACING-LEADING `active`
// run unroll and the oracle its leading-run map, so the JIT output stays
// byte-identical.

#[test]
fn inspect_combined_replacing_leading_all_tally() {
    // "00X00": TALLYING FOR ALL "0" counts EVERY "0" (4 — two leading, two trailing)
    // into C FIRST, THEN REPLACING LEADING "0" rewrites only the leading run (2,
    // stops at 'X') → "**X00". delim == search: the count is 4 (all zeros) even
    // though only the two leading zeros are ultimately replaced — proof the tally
    // saw the original bytes before the leading replace overwrote any of them.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"00X00\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" REPLACING LEADING \"0\" BY \"*\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "004\n**X00\n");
}

#[test]
fn inspect_combined_both_halves_leading() {
    // Both halves LEADING: TALLYING FOR LEADING "0" counts only the leading run (2)
    // into C, THEN REPLACING LEADING "0" rewrites that same leading run → "**X00".
    // The two leading flags are threaded independently through the combined form.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"00X00\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR LEADING \"0\" REPLACING LEADING \"0\" BY \"*\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n**X00\n");
}

#[test]
fn inspect_combined_replacing_leading_no_run() {
    // No leading run: the first character is not `search`, so REPLACING LEADING
    // changes nothing even though later "0"s exist. TALLYING FOR ALL still counts
    // every "0" (2) → source unchanged "X00X". Pins the "no run" boundary.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"X00X\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" REPLACING LEADING \"0\" BY \"*\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\nX00X\n");
}

// INSPECT … TALLYING/REPLACING where a LEADING half ALSO carries a BEFORE/AFTER
// region — the rung this file targets. Both engines already own the full
// LEADING+region machinery per half (the STANDALONE `FOR LEADING …`/`REPLACING
// LEADING … BEFORE/AFTER` routines); the combined exec/emit already COMPOSE those
// routines in ISO order (tally FIRST over the ORIGINAL bytes, THEN replace). This
// rung merely lifted the combined-form deferral guard, so the combination is now
// byte-identical to the oracle. The count anchors at its window start; the replace
// anchors at ITS (independent) window start.

#[test]
fn inspect_combined_for_leading_after_region_anchors_the_count() {
    // (Case 2) AFTER-window anchoring on the TALLYING LEADING half. Over "0X00A",
    // the TALLYING window AFTER "X" is "00A" (indices 2..5); the LEADING run of "0"
    // anchored at THAT window start is 2 → C = 002. The leading "0" at index 0 is
    // BEFORE the window and must NOT contribute — the anchoring point. REPLACING ALL
    // "0" (no region) then rewrites every "0" (indices 0, 2, 3) → "*X**A".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0X00A\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR LEADING \"0\" AFTER \"X\"",
            "    REPLACING ALL \"0\" BY \"*\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n*X**A\n");
}

#[test]
fn inspect_combined_replacing_leading_after_region_anchors_the_run() {
    // (Case 3) AFTER-window anchoring on the REPLACING LEADING half. Over "0X00A",
    // the region-less TALLYING FOR ALL "0" counts all three "0"s → C = 003. The
    // REPLACING LEADING "0" AFTER "X" then rewrites only the LEADING run inside the
    // window "00A" (indices 2..5) → "0X**A"; the leading "0" at index 0 is outside
    // the window and is left untouched.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0X00A\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\"",
            "    REPLACING LEADING \"0\" BY \"*\" AFTER \"X\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "003\n0X**A\n");
}

#[test]
fn inspect_combined_both_halves_leading_independent_regions() {
    // (Case 4) BOTH halves LEADING, each with its OWN independent region. Over
    // "00A00B00": the TALLYING FOR LEADING "0" BEFORE "A" counts the prefix run in
    // window "00" (indices 0..2) → C = 002; the REPLACING LEADING "0" AFTER "B"
    // rewrites the leading run in window "00" (indices 6..8) → "00A00B**". The two
    // windows are disjoint and the two leading flags are threaded independently, yet
    // both compose the same standalone LEADING+region routines.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(8) VALUE \"00A00B00\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR LEADING \"0\" BEFORE \"A\"",
            "    REPLACING LEADING \"0\" BY \"*\" AFTER \"B\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n00A00B**\n");
}

#[test]
fn inspect_combined_leading_region_tally_sees_original_bytes_before_replace() {
    // (Case 5) tally-then-replace ORDERING under LEADING+region with delim == search.
    // Over "0000A", the TALLYING FOR LEADING "0" BEFORE "A" counts the run in window
    // "0000" → C = 004, and the REPLACING LEADING "0" BEFORE "A" rewrites that same
    // run → "****A". If the replace had run FIRST, the source would be "****A" and the
    // subsequent tally would count 0 — so C = 004 (not 0) is the proof the tally saw
    // the ORIGINAL bytes before the replace overwrote them, exactly the ISO order.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0000A\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR LEADING \"0\" BEFORE \"A\"",
            "    REPLACING LEADING \"0\" BY \"*\" BEFORE \"A\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "004\n****A\n");
}

#[test]
fn inspect_combined_leading_region_filler_between_source_and_counter() {
    // (Case 7) CROSS-PRODUCER binding: source `S`, an intervening `FILLER`, and the
    // counter `C` bind by NAME, not by storage adjacency. A FILLER sits between them
    // in WORKING-STORAGE; the combined LEADING+region statement still resolves S and C
    // correctly and produces the Case-1 result (C = 002, S = "**A*B").
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"00A0B\".",
            "01  FILLER PIC X(3) VALUE \"ZZZ\".",
            "01  C  PIC 9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C FOR LEADING \"0\" BEFORE \"A\"",
            "    REPLACING ALL \"0\" BY \"*\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n**A*B\n");
}

#[test]
fn inspect_combined_leading_region_all_both_halves_still_supported() {
    // (Case 6, regression) An ALREADY-supported combined form — ALL+region on BOTH
    // halves, no LEADING — is untouched by this rung. Over "0A0B0" with DISTINCT
    // window delimiters: tally FOR ALL "0" BEFORE "B" → window "0A0" → 2; replace ALL
    // "0" AFTER "B" → window "0" (index 4) → only the trailing "0" → "0A0B*".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0A0B0\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"B\"",
            "    REPLACING ALL \"0\" BY \"*\" AFTER \"B\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n0A0B*\n");
}

#[test]
fn inspect_combined_leading_no_region_still_supported() {
    // (Case 6, regression) A combined LEADING half WITHOUT a region — already
    // supported before this rung — stays byte-identical. Over "000X0": FOR LEADING
    // "0" counts the leading run 3 (stops at 'X') → C = 003; REPLACING ALL "0"
    // rewrites every "0" → "***X*".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"000X0\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR LEADING \"0\" REPLACING ALL \"0\" BY \"*\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "003\n***X*\n");
}

#[test]
fn inspect_combined_characters_before_region_with_leading_replace() {
    // (Formerly a later-rung regression; THIS rung LIFTS the deferral.) A combined
    // `FOR CHARACTERS` tally half is now supported, and it composes with EVERY existing
    // REPLACING-half capability — here a region-narrowed CHARACTERS count beside a
    // window-anchored `REPLACING LEADING`. "00A0B": the tally window is BEFORE "A" =
    // "00" (2 positions) → C = 002 (CHARACTERS counts positions, not a delimiter), then
    // `REPLACING LEADING "0" BY "*" AFTER "A"` narrows to the run beginning after the
    // "A" (index 3): S[3]='0'→'*', S[4]='B' stops → "00A*B". The tally saw the ORIGINAL
    // bytes before the replace overwrote any of them (ISO tally-then-replace order).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"00A0B\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR CHARACTERS BEFORE \"A\"",
            "    REPLACING LEADING \"0\" BY \"*\" AFTER \"A\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\n00A*B\n");
}

#[test]
fn inspect_combined_characters_replacing_all() {
    // The canonical new form: `TALLYING C FOR CHARACTERS REPLACING ALL "A" BY "B"`.
    // CHARACTERS counts the FULL length of the source (positions, NOT delimiter
    // matches), so over "XAYAZ" C = 005 — even though only TWO characters are "A".
    // The REPLACING half then rewrites every "A" → "B" over the whole (still-original)
    // source → "XBYBZ". Verifying BOTH the counter DISPLAY (005, the length) and the
    // rewritten source (XBYBZ) proves the tally half counted positions while the
    // replace half matched the delimiter, in ISO tally-then-replace order.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"XAYAZ\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR CHARACTERS REPLACING ALL \"A\" BY \"B\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "005\nXBYBZ\n");
}

#[test]
fn inspect_combined_characters_before_region_replacing_all() {
    // `TALLYING C FOR CHARACTERS BEFORE "X" REPLACING ALL "A" BY "B"`: the tally count
    // is NARROWED by the region while the replace runs over the WHOLE source. Over
    // "AAXAA" the CHARACTERS window BEFORE "X" is "AA" (2 positions) → C = 002, but
    // `REPLACING ALL "A"` is un-regioned, so EVERY "A" (both before and after the "X")
    // becomes "B" → "BBXBB". The count's window and the replace's reach are independent
    // — the crux of this case.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AAXAA\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR CHARACTERS BEFORE \"X\"",
            "    REPLACING ALL \"A\" BY \"B\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "002\nBBXBB\n");
}

#[test]
fn inspect_combined_characters_replacing_leading() {
    // `TALLYING C FOR CHARACTERS REPLACING LEADING "A" BY "B"`: a CHARACTERS tally
    // composed with a LEADING replace. Over "AAXAA" the CHARACTERS count is the full
    // length (C = 005), then `REPLACING LEADING "A"` rewrites only the consecutive run
    // of "A" at the START (the two leading A's), stopping at "X" → "BBXAA". The two
    // trailing A's are left intact — proof the LEADING run map still governs the
    // replace half unchanged.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AAXAA\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR CHARACTERS REPLACING LEADING \"A\" BY \"B\".",
            "DISPLAY C.",
            "DISPLAY S.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "005\nBBXAA\n");
}

#[test]
fn inspect_combined_characters_non_ascii_shares_the_reconstruction_chip() {
    // NON-ASCII characterization. The TALLYING-half CHARACTERS count is POSITION-based:
    // the compiler counts BYTE positions (`str_len`) while the oracle counts CHAR
    // positions (`chars.len()`) — the PRE-EXISTING byte-vs-char count chip
    // (task_396ba6f6), identical to the standalone `FOR CHARACTERS` (#60) and multi-item
    // CHARACTERS (#81). Here the "é" sits strictly OUTSIDE the tally window (BEFORE "X"),
    // so the CHARACTERS count itself WOULD agree byte-vs-char. BUT the REPLACING half
    // RECONSTRUCTS the whole field with per-position `str_slice`, which cannot slice a
    // multi-byte char — so the compiler traps, EXACTLY the shared reconstruction chip
    // every combined non-ASCII case hits (see
    // `inspect_combined_leading_region_non_ascii_shares_the_reconstruction_chip`). This
    // rung introduces NO new divergence. `PIC X(5) VALUE "AAXé"` right-pads the
    // 4-character value to 5 char-positions — "AAXé " (a trailing space; 6 bytes, since
    // "é" is a 2-byte UTF-8 sequence) — with "é" strictly AFTER the "X" tally window.
    // The oracle iterates chars and succeeds; the compiler traps — the documented,
    // pre-existing gap.
    let src = wrap(
        &["01  S  PIC X(5) VALUE \"AAXé\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR CHARACTERS BEFORE \"X\"",
            "    REPLACING ALL \"A\" BY \"B\".",
            "STOP RUN.",
        ],
    );
    // Compiler traps in the REPLACING reconstruction (pre-existing byte-vs-char chip)…
    assert!(
        run_on_jit_result(&src).is_err(),
        "compiled combined CHARACTERS reconstruction must trap on a non-ASCII source (shared chip)"
    );
    // …while the oracle iterates chars and succeeds — the documented, pre-existing gap.
    assert!(
        run_cobol(&src).is_ok(),
        "oracle succeeds char-based on the non-ASCII source (the pre-existing chip, unchanged)"
    );
}

#[test]
fn inspect_combined_leading_region_non_ascii_shares_the_reconstruction_chip() {
    // (Case 8) NON-ASCII: the LEADING+region MATCHING and COUNTING are byte-safe — a
    // multi-byte "é" never equals an ASCII delimiter/search byte, so the tally and the
    // window selection agree on both engines. BUT the REPLACING half RECONSTRUCTS the
    // whole field with per-position `str_slice`, which cannot slice a multi-byte char,
    // so the compiler traps — EXACTLY the PRE-EXISTING reconstruction chip every
    // REPLACING lowering shares (task_396ba6f6). This rung introduces NO new
    // divergence: the combined LEADING+region path traps identically. The oracle
    // iterates `char`s and succeeds char-based; that gap is the documented chip, not
    // anything new here. `PIC X(5) VALUE "00Xé"` stores "00Xé " (5 chars / 6 bytes),
    // with "é" strictly OUTSIDE every window.
    let src = wrap(
        &["01  S  PIC X(5) VALUE \"00Xé\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR LEADING \"0\" BEFORE \"X\"",
            "    REPLACING LEADING \"0\" BY \"*\" BEFORE \"X\".",
            "STOP RUN.",
        ],
    );
    // Compiler traps in the reconstruction (pre-existing byte-vs-char chip)…
    assert!(
        run_on_jit_result(&src).is_err(),
        "compiled combined LEADING+region reconstruction must trap on a non-ASCII source (shared chip)"
    );
    // …while the oracle iterates chars and succeeds — the documented, pre-existing gap.
    assert!(
        run_cobol(&src).is_ok(),
        "oracle succeeds char-based on the non-ASCII source (the pre-existing chip, unchanged)"
    );
}

#[test]
fn inspect_combined_characters_is_still_a_later_rung() {
    // The still-deferred combined sub-form: `REPLACING CHARACTERS BY` on the REPLACE
    // half rewrites every position unconditionally — a DIFFERENT node
    // (`InspectReplacingCharacters` / `emit_inspect_replacing_characters`) from the
    // TALLYING-half CHARACTERS this rung lifted. It stays a later rung, rejected
    // CO-TOTAL on both engines: the combined REPLACING half flows through
    // `inspect_replacing_all`, which rejects a `CHARACTERS` item on both sides.
    let src = wrap(
        &["01  S  PIC X(5) VALUE \"AABBB\".", "01  C  PIC 9(3) VALUE 0."],
        &[
            "INSPECT S TALLYING C FOR ALL \"B\" REPLACING CHARACTERS BY \"X\".",
            "STOP RUN.",
        ],
    );
    // Oracle rejects…
    assert!(run_cobol(&src).is_err(), "oracle must still reject combined REPLACING CHARACTERS");
    // …and the compiler rejects with the same later-rung `Unsupported` diagnostic.
    let err = compile_source(&src, "insp_combined_repl_chars").unwrap_err();
    assert!(matches!(err, cobol_iir_compiler::CompileError::Unsupported(_)), "got {err:?}");
}

#[test]
fn inspect_combined_second_tally_item_is_a_later_rung() {
    // A combined statement whose TALLYING half has a second FOR-phrase item
    // (`FOR ALL "A" ALL "B"`) is still a later rung — it parses, but the combined
    // gate does not admit the deferred sub-forms, so it is a clean Unsupported.
    let err = compile_source(
        &wrap(
            &["01  S  PIC X(5) VALUE \"ABABA\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"A\" ALL \"B\" REPLACING ALL \"B\" BY \"X\".",
                "STOP RUN.",
            ],
        ),
        "insp_combined_two_items",
    )
    .unwrap_err();
    assert!(matches!(err, cobol_iir_compiler::CompileError::Unsupported(_)), "got {err:?}");
}

// INSPECT … CONVERTING from TO to — translate every character of the source
// through a per-character table built from the two equal-length string literals:
// a character equal to from[k] becomes to[k] (first k wins if from repeats), and
// a character in no table entry is left unchanged (in place, same width). Each
// case pins the compiled per-position rebuild to the oracle's char→char map,
// byte-for-byte.
// ---------------------------------------------------------------------------

#[test]
fn inspect_converting_simple_table() {
    // A→X, B→Y, C→Z applied to "CAB" → "ZXY".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"CAB\"."],
        &["INSPECT S CONVERTING \"ABC\" TO \"XYZ\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "ZXY\n");
}

#[test]
fn inspect_converting_multichar_vowel_table() {
    // "AEIOU" → "12345": A→1,E→2,I→3,O→4,U→5. "BEAN" → B,2,1,N = "B21N".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"BEAN\"."],
        &["INSPECT S CONVERTING \"AEIOU\" TO \"12345\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "B21N\n");
}

#[test]
fn inspect_converting_char_not_in_from_is_unchanged() {
    // Only the vowels of "HELLO" map (E→2, O→4); H, L, L are in no table entry and
    // pass through unchanged → "H2LL4".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"HELLO\"."],
        &["INSPECT S CONVERTING \"AEIOU\" TO \"12345\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "H2LL4\n");
}

#[test]
fn inspect_converting_every_character_is_converted() {
    // Every character of "ABAB" is in the table (A→X, B→Y) → "XYXY".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"ABAB\"."],
        &["INSPECT S CONVERTING \"AB\" TO \"XY\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "XYXY\n");
}

#[test]
fn inspect_converting_single_char_from_and_to() {
    // A one-entry table O→0 on "MOON" → "M00N" (the CONVERTING form of a single
    // substitution, and the optional END-INSPECT terminator parses).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"MOON\"."],
        &["INSPECT S CONVERTING \"O\" TO \"0\" END-INSPECT.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "M00N\n");
}

#[test]
fn inspect_converting_duplicate_from_leftmost_wins() {
    // `from` repeats 'A': "AAB" → "XYZ". The FIRST occurrence wins, so A→X (not the
    // later A→Y); B→Z. "AAB" → "XXZ" (if the rightmost won it would be "YYZ").
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"AAB\"."],
        &["INSPECT S CONVERTING \"AAB\" TO \"XYZ\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "XXZ\n");
}

// INSPECT … CONVERTING later-rung forms — these parse (the grammar accepts the
// broad surface) but the compiler rejects them as a clean Unsupported.

#[test]
fn inspect_converting_unequal_lengths_is_a_later_rung() {
    // A from/to pair of different lengths has no well-defined table → later rung.
    let err = compile_source(
        &wrap(
            &["01  S  PIC X(3) VALUE \"ABC\"."],
            &["INSPECT S CONVERTING \"AB\" TO \"XYZ\".", "STOP RUN."],
        ),
        "insp_conv_unequal",
    )
    .unwrap_err();
    assert!(matches!(err, cobol_iir_compiler::CompileError::Unsupported(_)), "got {err:?}");
}

// ---------------------------------------------------------------------------
// INSPECT … CONVERTING with a DATA-NAME `from`/`to` operand — the translation
// SET may come from a `PIC X` item's CURRENT storage instead of a string literal.
// Either or both of `from`/`to` may be an item (mixing freely with a literal on
// the other side). The item's declared width supplies the equal-length check at
// compile time; its bytes are read ONCE (loop-invariant) before the per-position
// loop, so the per-position first-match-wins chain is byte-identical to the
// literal path — pinned against the oracle here on ASCII operands.
// ---------------------------------------------------------------------------

#[test]
fn inspect_converting_data_name_from_literal_to() {
    // `from` is the item F (= "ABC"), `to` the literal "XYZ": A→X, B→Y, C→Z applied
    // to "CAB" → "ZXY" — identical to the both-literal simple table.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"CAB\".", "01  F  PIC X(3) VALUE \"ABC\"."],
        &["INSPECT S CONVERTING F TO \"XYZ\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "ZXY\n");
}

#[test]
fn inspect_converting_literal_from_data_name_to() {
    // Reverse mix: `from` the literal "ABC", `to` the item T (= "XYZ"). Same table,
    // same "CAB" → "ZXY".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"CAB\".", "01  T  PIC X(3) VALUE \"XYZ\"."],
        &["INSPECT S CONVERTING \"ABC\" TO T.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "ZXY\n");
}

#[test]
fn inspect_converting_both_data_names() {
    // BOTH sides are items: F = "AEIOU", T = "12345". "BEAN" → "B21N" (B, N pass
    // through). The full table is built from two runtime storages.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(4) VALUE \"BEAN\".",
            "01  F  PIC X(5) VALUE \"AEIOU\".",
            "01  T  PIC X(5) VALUE \"12345\".",
        ],
        &["INSPECT S CONVERTING F TO T.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "B21N\n");
}

#[test]
fn inspect_converting_data_name_with_a_region() {
    // A data-name table narrowed by a `{BEFORE|AFTER}` region: table F="A"→T="0";
    // BEFORE "Y" in "AXAYA" restricts the translate to "AXA" → "0X0YA" (the trailing
    // "A" right of "Y" is untouched) — the region guard composes with runtime table
    // reads exactly as with a literal table.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"AXAYA\".",
            "01  F  PIC X(1) VALUE \"A\".",
            "01  T  PIC X(1) VALUE \"0\".",
        ],
        &["INSPECT S CONVERTING F TO T BEFORE \"Y\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "0X0YA\n");
}

#[test]
fn inspect_converting_data_name_first_occurrence_wins() {
    // The `from` ITEM repeats a char: F = "AAB", T = "XYZ". First occurrence wins, so
    // A→X (not the later A→Y); B→Z. "AAB" → "XXZ" — the leftmost-wins rule holds for a
    // runtime-read table just as for a literal one.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(3) VALUE \"AAB\".",
            "01  F  PIC X(3) VALUE \"AAB\".",
            "01  T  PIC X(3) VALUE \"XYZ\".",
        ],
        &["INSPECT S CONVERTING F TO T.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "XXZ\n");
}

#[test]
fn inspect_converting_data_name_char_not_in_from_passes_through() {
    // A source char in NO table entry is unchanged: table F="AEIOU"→T="12345" on
    // "HELLO" maps only E→2, O→4; H, L, L pass through → "H2LL4".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"HELLO\".",
            "01  F  PIC X(5) VALUE \"AEIOU\".",
            "01  T  PIC X(5) VALUE \"12345\".",
        ],
        &["INSPECT S CONVERTING F TO T.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "H2LL4\n");
}

#[test]
fn inspect_converting_from_item_aliases_the_source() {
    // The `from` item IS the source S itself: INSPECT S CONVERTING S TO T. The table
    // must be built from the ORIGINAL bytes of S — read up front, before the loop
    // overwrites S. S = "ABAB", T = "XYCD" (only the FIRST occurrence of each S char
    // matters): the table maps A→X (index 0 wins over index 2), B→Y (index 1 wins
    // over index 3). "ABAB" → "XYXY". If S were read AFTER a partial rewrite the
    // result would diverge; parity here pins the up-front read.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"ABAB\".", "01  T  PIC X(4) VALUE \"XYCD\"."],
        &["INSPECT S CONVERTING S TO T.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "XYXY\n");
}

#[test]
fn inspect_converting_filler_between_source_and_table_binds_by_name() {
    // A FILLER sits BETWEEN the named source S and the named table items F/T. The
    // compiler DROPS the FILLER while the oracle PUSHES it, so the two engines lay
    // their items out differently — but both bind the source and table operands by
    // NAME, so the CONVERTING still resolves S, F, T identically. Table F="AB"→T="XY"
    // on "ABAB" → "XYXY".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S       PIC X(4) VALUE \"ABAB\".",
            "01  FILLER  PIC X(2) VALUE \"##\".",
            "01  F       PIC X(2) VALUE \"AB\".",
            "01  T       PIC X(2) VALUE \"XY\".",
        ],
        &["INSPECT S CONVERTING F TO T.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "XYXY\n");
}

#[test]
fn inspect_converting_unequal_width_data_name_is_a_later_rung() {
    // A data-name `from`/`to` whose DECLARED WIDTHS differ has no well-defined table —
    // the equal-length check (now over item widths) rejects it as a later rung on BOTH
    // engines (the compiler at build time, the oracle at exec time).
    let src = wrap(
        &[
            "01  S  PIC X(3) VALUE \"ABC\".",
            "01  F  PIC X(2) VALUE \"AB\".",
            "01  T  PIC X(3) VALUE \"XYZ\".",
        ],
        &["INSPECT S CONVERTING F TO T.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject unequal-width data-name from/to");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject unequal-width data-name from/to"
    );
}

#[test]
fn inspect_converting_numeric_item_as_table_is_a_later_rung() {
    // A NUMERIC item as the `from` table is a later rung on BOTH engines — CONVERTING's
    // table must be an alphanumeric (`PIC X`) operand.
    let src = wrap(
        &["01  S  PIC X(3) VALUE \"ABC\".", "01  N  PIC 9(3) VALUE 123."],
        &["INSPECT S CONVERTING N TO \"XYZ\".", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a numeric table item");
    assert!(compile_source(&src, "e2e").is_err(), "compiler must reject a numeric table item");
}

// A non-ASCII data-name `from`/`to` operand is the PRE-EXISTING byte-vs-char operand
// chip (task_396ba6f6): the compiler compares raw bytes, the oracle compares chars,
// so a multibyte item byte can diverge and is NOT statically rejectable. This
// CHARACTERIZATION test pins EACH engine's behaviour independently (NOT via
// assert_matches_oracle) so a change is caught — it deliberately does not assert the
// two agree. All operands here keep the SOURCE ASCII; only the data-name table
// carries a multibyte char.
#[test]
fn inspect_converting_non_ascii_data_name_table_is_the_byte_vs_char_chip() {
    // F holds a 1-char (but 2-byte) "é" table key; T holds "0". The SOURCE stays
    // ASCII ("XYZ") so only the TABLE is multibyte. No source char matches the table
    // key on either engine (the oracle compares the char 'é', the compiler compares
    // 'é''s first byte — neither equals X/Y/Z), so both leave the source unchanged.
    // We pin each engine to "XYZ\n" as a stable characterization of the shared chip,
    // WITHOUT asserting the two agree by construction.
    let src = wrap(
        &["01  S  PIC X(3) VALUE \"XYZ\".", "01  F  PIC X(1) VALUE \"é\".", "01  T  PIC X(1) VALUE \"0\"."],
        &["INSPECT S CONVERTING F TO T.", "DISPLAY S.", "STOP RUN."],
    );
    // Oracle: characterization pin.
    let oracle = run_cobol(&src).expect("oracle run");
    assert_eq!(oracle, "XYZ\n", "oracle characterization");
    // Compiled: characterization pin (independent — this is the shared byte-vs-char chip,
    // so we do NOT route through assert_matches_oracle).
    let jit = run_on_jit(&src);
    assert_eq!(jit, "XYZ\n", "compiled characterization");
}

// The TO side of the same chip has a DIFFERENT failure MODE than the FROM side. A
// non-ASCII TO item is sliced per position with `str_slice(item, k, k+1)` (BYTE
// offsets); on a multibyte char that slice cuts a UTF-8 boundary, so the byte-based
// compiler TRAPS while the char-based oracle produces a valid char-mapped result.
// Both engines confine the disagreement to the non-ASCII-operand chip zone
// (task_396ba6f6); this CHARACTERIZATION test pins each side independently (compiler
// traps, oracle succeeds), documenting the trap so a change is caught. The supported
// ASCII surface is unaffected.
#[test]
fn inspect_converting_non_ascii_to_data_name_traps_the_compiler_reconstruction_chip() {
    // F = "A" (ASCII key), T = "é" (1 char / 2 bytes). Source "AYZ": the oracle maps
    // A→"é" → "éYZ"; the compiler's up-front `str_slice(T, 0, 1)` cuts the 2-byte "é"
    // and traps. Deterministic (the trap is in the up-front table build, independent
    // of whether any source char matches).
    let src = wrap(
        &["01  S  PIC X(3) VALUE \"AYZ\".", "01  F  PIC X(1) VALUE \"A\".", "01  T  PIC X(1) VALUE \"é\"."],
        &["INSPECT S CONVERTING F TO T.", "DISPLAY S.", "STOP RUN."],
    );
    // Oracle: char-based, succeeds with the mapped multibyte char.
    assert_eq!(run_cobol(&src).expect("oracle run"), "éYZ\n", "oracle characterization");
    // Compiled: byte-based `str_slice` on the multibyte TO item traps (pre-existing chip).
    assert!(
        run_on_jit_result(&src).is_err(),
        "compiled non-ASCII TO-item reconstruction traps (pre-existing byte-vs-char chip)"
    );
}

// ---------------------------------------------------------------------------
// INSPECT … CONVERTING with a CONSTANT reference-modified `from`/`to` operand —
// the translation set may be a slice `base(start:len)` (or `base(start:)`) of an
// alphanumeric item, provided both indices are LITERALS. A const refmod's slice
// length is compile-time-known, so it reduces to the data-name (`Item`) case: the
// compiler materialises the slice register up front via the shared `ref_mod_slice`
// and treats it as a fixed-width item; the oracle resolves it up front via the
// shared `refmod_string`. A COMPUTED refmod (any data-name index) has a run-time
// length and stays a later rung, rejected on BOTH engines by the same const-index
// predicate the MOVE/STRING refmod rungs use (#67). Pinned against the oracle here
// on ASCII operands.
// ---------------------------------------------------------------------------

#[test]
fn inspect_converting_const_refmod_from_literal_to() {
    // `from` is the const slice F(2:3) = "ABC" (F = "ZABCZ"); `to` the literal "XYZ".
    // A→X, B→Y, C→Z applied to "CAB" → "ZXY" — identical to the both-literal table.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"CAB\".", "01  F  PIC X(5) VALUE \"ZABCZ\"."],
        &["INSPECT S CONVERTING F(2:3) TO \"XYZ\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "ZXY\n");
}

#[test]
fn inspect_converting_literal_from_const_refmod_to() {
    // Reverse mix: `from` the literal "ABC", `to` the const slice T(2:3) = "XYZ"
    // (T = "ZXYZZ"). Same table, same "CAB" → "ZXY".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"CAB\".", "01  T  PIC X(5) VALUE \"ZXYZZ\"."],
        &["INSPECT S CONVERTING \"ABC\" TO T(2:3).", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "ZXY\n");
}

#[test]
fn inspect_converting_both_const_refmods() {
    // BOTH sides are const slices: F(2:5) = "AEIOU" (F = "ZAEIOU"), T(2:5) = "12345"
    // (T = "Z12345"). "BEAN" → "B21N" (B, N pass through). The full table is built
    // from two compile-time-length slice registers.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(4) VALUE \"BEAN\".",
            "01  F  PIC X(6) VALUE \"ZAEIOU\".",
            "01  T  PIC X(6) VALUE \"Z12345\".",
        ],
        &["INSPECT S CONVERTING F(2:5) TO T(2:5).", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "B21N\n");
}

#[test]
fn inspect_converting_const_refmod_omitted_len() {
    // The `base(start:)` form (omitted length runs to the end of the item): F(2:) on
    // F = "ZABC" (PIC X(4)) is the 3-char slice "ABC" — its static length is
    // `width - start + 1 = 4 - 2 + 1 = 3`, so it drops into the equal-length check
    // against the 3-char "XYZ" just like a data-name's declared width. "CAB" → "ZXY".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"CAB\".", "01  F  PIC X(4) VALUE \"ZABC\"."],
        &["INSPECT S CONVERTING F(2:) TO \"XYZ\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "ZXY\n");
}

#[test]
fn inspect_converting_const_refmod_with_a_region() {
    // A const-slice table narrowed by a `{BEFORE|AFTER}` region: table F(2:1)="A"→"0";
    // BEFORE "Y" in "AXAYA" restricts the translate to "AXA" → "0X0YA" (the trailing
    // "A" right of "Y" is untouched) — the region guard composes with the slice
    // register exactly as with a data-name or literal table.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AXAYA\".", "01  F  PIC X(3) VALUE \"ZAZ\"."],
        &["INSPECT S CONVERTING F(2:1) TO \"0\" BEFORE \"Y\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "0X0YA\n");
}

#[test]
fn inspect_converting_const_refmod_first_occurrence_wins() {
    // The `from` const slice repeats a char: F(2:3) = "AAB" (F = "ZAAB"). First
    // occurrence wins, so A→X (not the later A→Y); B→Z. "AAB" → "XXZ" — the
    // leftmost-wins rule holds for a slice-register table just as for the others.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"AAB\".", "01  F  PIC X(4) VALUE \"ZAAB\"."],
        &["INSPECT S CONVERTING F(2:3) TO \"XYZ\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "XXZ\n");
}

#[test]
fn inspect_converting_const_refmod_aliases_the_source() {
    // The `from` refmod BASE is the source S itself: INSPECT S CONVERTING S(2:2) TO T.
    // The slice must be built from the ORIGINAL bytes of S — `ref_mod_slice`
    // materialises the slice register up front, before the loop overwrites S. S =
    // "ABAB", S(2:2) = "BA" (original bytes), T = "XY": table B→X, A→Y. "ABAB" →
    // "YXYX". If the slice were read AFTER a partial rewrite the result would diverge;
    // parity here pins the up-front slice.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"ABAB\".", "01  T  PIC X(2) VALUE \"XY\"."],
        &["INSPECT S CONVERTING S(2:2) TO T.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "YXYX\n");
}

#[test]
fn inspect_converting_const_refmod_filler_between_binds_by_name() {
    // A FILLER sits BETWEEN the named source S and the named refmod base F. The
    // compiler DROPS the FILLER while the oracle PUSHES it, so item layouts differ —
    // but both bind the source and refmod base by NAME, so the slice F(1:2)="AB"
    // resolves identically. "ABAB" → "XYXY".
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S       PIC X(4) VALUE \"ABAB\".",
            "01  FILLER  PIC X(2) VALUE \"##\".",
            "01  F       PIC X(2) VALUE \"AB\".",
        ],
        &["INSPECT S CONVERTING F(1:2) TO \"XY\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "XYXY\n");
}

#[test]
fn inspect_converting_unequal_length_const_refmod_is_a_later_rung() {
    // A const-slice `from`/`to` whose STATIC lengths differ has no well-defined table —
    // the equal-length check (now over slice lengths) rejects it as a later rung on
    // BOTH engines (the compiler at build time, the oracle at exec time). F(1:2) = "AB"
    // (len 2) vs "XYZ" (len 3).
    let src = wrap(
        &["01  S  PIC X(3) VALUE \"ABC\".", "01  F  PIC X(2) VALUE \"AB\"."],
        &["INSPECT S CONVERTING F(1:2) TO \"XYZ\".", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject unequal-length const-refmod from/to");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject unequal-length const-refmod from/to"
    );
}

#[test]
fn inspect_converting_computed_refmod_from_is_a_later_rung() {
    // A COMPUTED refmod `from` — the start index is a data-name J, so the slice length
    // is only known at run time. The compile-time table contract cannot carry it, so
    // it stays a later rung, rejected on BOTH engines (the same const-index predicate
    // #67 uses for STRING/MOVE refmod sending fields).
    let src = wrap(
        &[
            "01  S  PIC X(3) VALUE \"CAB\".",
            "01  F  PIC X(5) VALUE \"ZABCZ\".",
            "01  J  PIC 9   VALUE 2.",
        ],
        &["INSPECT S CONVERTING F(J:3) TO \"XYZ\".", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a computed refmod from");
    assert!(compile_source(&src, "e2e").is_err(), "compiler must reject a computed refmod from");
}

#[test]
fn inspect_converting_computed_refmod_to_is_a_later_rung() {
    // The TO side of the same rule: a computed refmod `to` (length index is a
    // data-name K) is a run-time length the table contract cannot carry — a later
    // rung on BOTH engines.
    let src = wrap(
        &[
            "01  S  PIC X(3) VALUE \"CAB\".",
            "01  T  PIC X(5) VALUE \"ZXYZZ\".",
            "01  K  PIC 9   VALUE 3.",
        ],
        &["INSPECT S CONVERTING \"ABC\" TO T(2:K).", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a computed refmod to");
    assert!(compile_source(&src, "e2e").is_err(), "compiler must reject a computed refmod to");
}

#[test]
fn inspect_converting_numeric_base_const_refmod_is_a_later_rung() {
    // A refmod whose BASE is a NUMERIC item is a later rung on BOTH engines — the
    // slice evaluator rejects a numeric base identically (`ref_mod_slice` at build
    // time, `refmod_string` at exec time), so CONVERTING inherits that gate unchanged.
    let src = wrap(
        &["01  S  PIC X(3) VALUE \"CAB\".", "01  N  PIC 9(3) VALUE 123."],
        &["INSPECT S CONVERTING N(1:2) TO \"XY\".", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a numeric refmod base");
    assert!(compile_source(&src, "e2e").is_err(), "compiler must reject a numeric refmod base");
}

#[test]
fn inspect_converting_non_ascii_char_outside_const_refmod_window() {
    // POSITIVE non-ASCII case: the multibyte char is STRICTLY OUTSIDE the slice window
    // (and the source stays ASCII), so byte offsets (compiler `str_slice`) and char
    // offsets (oracle `refmod_string`) coincide within `[0, end)`. F = "ABCé": the
    // slice F(1:3) = "ABC" cuts BEFORE the 2-byte "é" (char index 3, byte offset 3),
    // so both engines see the ASCII "ABC" table. "CAB" → "ZXY". The byte-vs-char chip
    // (task_396ba6f6) only bites when the slice — or the source reconstruction —
    // straddles a multibyte char, which this case avoids; that hazard is documented on
    // the data-name non-ASCII characterization tests above.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"CAB\".", "01  F  PIC X(4) VALUE \"ABCé\"."],
        &["INSPECT S CONVERTING F(1:3) TO \"XYZ\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "ZXY\n");
}

// ---------------------------------------------------------------------------
// INSPECT … CONVERTING with a FIGURATIVE-CONSTANT `from`/`to` operand — SPACE and
// ZERO (the only figuratives in the model) are accepted, each mapped to the
// single-character literal `" "` (0x20) / `"0"` (0x30). Both are ASCII, so the
// mapping reduces to the EXISTING single-char Literal path in every downstream
// respect: the equal-length check, the ASCII-literal guard, the convert loop, and
// the BEFORE/AFTER region all apply unchanged. Pinned against the oracle here.
// ---------------------------------------------------------------------------

#[test]
fn inspect_converting_figurative_space_from() {
    // `from` is the figurative SPACE (→ " "), `to` the literal "_": every space of
    // "A B C" becomes "_" → "A_B_C". A figurative FROM reduces to the single-char
    // literal path.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"A B C\"."],
        &["INSPECT S CONVERTING SPACE TO \"_\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "A_B_C\n");
}

#[test]
fn inspect_converting_figurative_zero_to() {
    // `from` the literal "O", `to` the figurative ZERO (→ "0"): O→0 applied to "MOON"
    // → "M00N". A figurative TO reduces to the single-char literal path.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"MOON\"."],
        &["INSPECT S CONVERTING \"O\" TO ZERO.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "M00N\n");
}

#[test]
fn inspect_converting_figurative_space_to() {
    // `to` is the figurative SPACE (→ " "): X→space applied to "XAXA" → " A A" (a
    // space wherever an X sat). Confirms a figurative TO maps to the blank correctly.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"XAXA\"."],
        &["INSPECT S CONVERTING \"X\" TO SPACE.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, " A A\n");
}

#[test]
fn inspect_converting_figurative_plural_spellings() {
    // The reader folds every spelling of a figurative to the same `Fig`: SPACES == SPACE,
    // ZEROS == ZEROES == ZERO. `SPACES TO ZEROS` and `SPACES TO ZEROES` therefore build
    // the identical " "→"0" table: each space of "A B C" → "0" → "A0B0C".
    let out_s = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"A B C\"."],
        &["INSPECT S CONVERTING SPACES TO ZEROS.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out_s, "A0B0C\n");
    let out_es = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"A B C\"."],
        &["INSPECT S CONVERTING SPACES TO ZEROES.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out_es, "A0B0C\n");
}

#[test]
fn inspect_converting_figurative_with_a_region() {
    // A figurative operand narrowed by a `{BEFORE|AFTER}` region: SPACE→"_" but only
    // BEFORE "Z". "A B Z" — the two spaces left of "Z" (indices 1 and 3) convert; the
    // "Z" and anything right of it are untouched → "A_B_Z". The region guard composes
    // with the figurative-as-literal table exactly as with a plain literal.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"A B Z\"."],
        &["INSPECT S CONVERTING SPACE TO \"_\" BEFORE \"Z\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "A_B_Z\n");
}

#[test]
fn inspect_converting_figurative_unequal_length_is_a_later_rung() {
    // A figurative (length 1) paired with a length-2 literal has no well-defined table —
    // the EXISTING equal-length check rejects it as a later rung on BOTH engines. "AB"
    // (len 2) vs ZERO (→ "0", len 1).
    let src = wrap(
        &["01  S  PIC X(3) VALUE \"ABC\"."],
        &["INSPECT S CONVERTING \"AB\" TO ZERO.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a figurative paired with an unequal-length literal");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a figurative paired with an unequal-length literal"
    );
}

#[test]
fn inspect_converting_figurative_all_ascii_characterization() {
    // NON-ASCII PARITY note. SPACE and ZERO are the ONLY figuratives in the model, and
    // both map to an inherently ASCII single-character literal (" " = 0x20, "0" = 0x30),
    // so there is NO non-ASCII figurative OPERAND to exercise — the figurative operand
    // this rung adds is always ASCII. A non-ASCII byte can still enter only through the
    // SOURCE, and that is the PRE-EXISTING byte-vs-char SOURCE chip (task_396ba6f6): the
    // compiler reconstructs each pass-through position with `str_slice(s, j, j+1)` on
    // BYTE offsets, which traps on a multibyte char, while the char-based oracle
    // succeeds — a disagreement OWNED by the shared source-reconstruction chip and
    // UNCHANGED by this rung (it predates and is orthogonal to the figurative operand).
    // The figurative rung's parity surface is therefore all-ASCII; this test pins the
    // representative all-ASCII figurative-on-both-sides case through assert_matches_oracle
    // — an all-space source "     " CONVERTING SPACE TO ZERO → "00000" (both `from` and
    // `to` are figuratives, both mapped to single-char ASCII literals).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE SPACES."],
        &["INSPECT S CONVERTING SPACE TO ZERO.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "00000\n");
}

#[test]
fn inspect_converting_combined_with_replacing_is_rejected() {
    // CONVERTING is a STANDALONE INSPECT alternative — the grammar does not let it
    // sit beside a REPLACING clause in one statement — so a combined form is a
    // clean compile-time rejection (a parse error, since the two are mutually
    // exclusive alternatives), never a silent mis-compile.
    let err = compile_source(
        &wrap(
            &["01  S  PIC X(5) VALUE \"ABABA\"."],
            &[
                "INSPECT S CONVERTING \"A\" TO \"X\" REPLACING ALL \"B\" BY \"Y\".",
                "STOP RUN.",
            ],
        ),
        "insp_conv_combined",
    )
    .unwrap_err();
    assert!(matches!(err, cobol_iir_compiler::CompileError::Parse(_)), "got {err:?}");
}

// ---------------------------------------------------------------------------
// INSPECT … CONVERTING from TO to … {BEFORE|AFTER} z — restrict the translation
// to the sub-slice of the source bounded by the FIRST occurrence of the single
// region delimiter `z`. BEFORE translates left of `z`; AFTER translates right of
// it; positions OUTSIDE the region keep their original character. The window is
// the SAME one the TALLYING/REPLACING region rungs compute (shared helper), so the
// ISO not-found asymmetry — BEFORE with `z` absent translates the WHOLE source,
// AFTER with `z` absent translates NOTHING — must hold byte-for-byte on both
// engines.
// ---------------------------------------------------------------------------

#[test]
fn inspect_converting_before_translates_only_left_of_the_delimiter() {
    // "AXAYA" — table A→0; BEFORE "Y" restricts the translate to "AXA" (indices
    // 0..3): the two "A"s there become "0", the trailing "A" (index 4, right of "Y")
    // is UNTOUCHED → "0X0Y A" without the space = "0X0YA".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AXAYA\"."],
        &["INSPECT S CONVERTING \"A\" TO \"0\" BEFORE \"Y\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "0X0YA\n");
}

#[test]
fn inspect_converting_after_translates_only_right_of_the_delimiter() {
    // Same source and table — AFTER "Y" restricts the translate to "A" (index 4):
    // only that trailing "A" becomes "0"; the two "A"s left of "Y" are UNTOUCHED.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AXAYA\"."],
        &["INSPECT S CONVERTING \"A\" TO \"0\" AFTER \"Y\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "AXAY0\n");
}

#[test]
fn inspect_converting_before_absent_delimiter_translates_the_whole_source() {
    // BEFORE "Z" with no "Z" present → the region is the ENTIRE source, so EVERY "A"
    // is translated (the BEFORE not-found rule — the whole subtlety of the rung).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AXAYA\"."],
        &["INSPECT S CONVERTING \"A\" TO \"0\" BEFORE \"Z\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "0X0Y0\n");
}

#[test]
fn inspect_converting_after_absent_delimiter_translates_nothing() {
    // AFTER "Z" with no "Z" present → the region is EMPTY, so NOTHING is translated
    // and the source is unchanged (the AFTER not-found rule — asymmetric partner).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AXAYA\"."],
        &["INSPECT S CONVERTING \"A\" TO \"0\" AFTER \"Z\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "AXAYA\n");
}

#[test]
fn inspect_converting_after_region_delimiter_at_position_zero() {
    // The region delimiter is the FIRST character: AFTER "Y" in "YABA" → region
    // [1, 4) = "ABA", so both "A"s become "0" (B unmapped) → "Y0B0".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"YABA\"."],
        &["INSPECT S CONVERTING \"A\" TO \"0\" AFTER \"Y\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "Y0B0\n");
}

#[test]
fn inspect_converting_before_region_delimiter_as_last_char() {
    // The region delimiter is the LAST character: BEFORE "Y" in "AXAY" → region
    // [0, 3) = "AXA", so both "A"s become "0"; the trailing "Y" is left → "0X0Y".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"AXAY\"."],
        &["INSPECT S CONVERTING \"A\" TO \"0\" BEFORE \"Y\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "0X0Y\n");
}

#[test]
fn inspect_converting_region_delimiter_is_also_in_the_from_set() {
    // The region delimiter "A" is ALSO a `from` character: AFTER "A" in "AXAYA" → the
    // FIRST "A" (index 0) bounds the region [1, 5) = "XAYA". The translate runs over
    // the ORIGINAL bytes, so the "A"s at indices 2 and 4 become "0", while the
    // delimiter "A" at index 0 (left of the region) is KEPT → "AX0Y0".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AXAYA\"."],
        &["INSPECT S CONVERTING \"A\" TO \"0\" AFTER \"A\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "AX0Y0\n");
}

#[test]
fn inspect_converting_before_delimiter_at_position_zero_is_an_empty_region() {
    // BEFORE "Y" in "YAXA" → the first "Y" is at index 0, so the region is [0, 0) —
    // EMPTY — and NOTHING is translated even though "A"s follow → "YAXA" unchanged.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"YAXA\"."],
        &["INSPECT S CONVERTING \"A\" TO \"0\" BEFORE \"Y\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "YAXA\n");
}

#[test]
fn inspect_converting_multichar_table_with_a_region() {
    // A multi-entry table (A→1, E→2) narrowed by a region: "AEYAE" — BEFORE "Y" limits
    // the translate to "AE" (indices 0..2) → "12"; the trailing "AE" (right of "Y") is
    // UNTOUCHED → "12YAE".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AEYAE\"."],
        &["INSPECT S CONVERTING \"AE\" TO \"12\" BEFORE \"Y\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "12YAE\n");
}

#[test]
fn inspect_converting_region_delimiter_is_a_pic_x1_item() {
    // The region delimiter may itself be a PIC X(1) item, read at run time: BEFORE DL
    // (= "Y") in "AXAYA" restricts to "AXA" → "0X0YA", matching the literal case.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"AXAYA\".", "01  DL PIC X(1) VALUE \"Y\"."],
        &["INSPECT S CONVERTING \"A\" TO \"0\" BEFORE DL.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "0X0YA\n");
}

#[test]
fn inspect_converting_region_shorter_than_the_from_set() {
    // The region window is SHORTER than the `from` set: table A→1,E→2,I→3 but the
    // BEFORE "Y" window is just "A" (index 0). Only that "A" translates → "1YEI"
    // (the "E","I" right of "Y" keep their originals though they are in `from`).
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"AYEI\"."],
        &["INSPECT S CONVERTING \"AEI\" TO \"123\" BEFORE \"Y\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "1YEI\n");
}

#[test]
fn inspect_converting_multi_char_region_delimiter_is_a_later_rung() {
    // A MULTI-character region delimiter is deferred — rejected on both engines, just
    // like a multi-character search/tally delimiter (the oracle rejects at exec, the
    // compiler at emit, both via the shared single-delimiter check).
    let src = wrap(
        &["01  S  PIC X(6) VALUE \"AXAYAB\"."],
        &["INSPECT S CONVERTING \"A\" TO \"0\" BEFORE \"YB\".", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a multi-char region delimiter");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a multi-char region delimiter"
    );
}

#[test]
fn mixed_unsigned_numeric_vs_space_figurative_agrees() {
    // An UNSIGNED numeric vs the SPACE figurative is supported and byte-identical:
    // NUM=42 → image "042"; SPACE expands to the image width → "   "; '0' (0x30) >
    // ' ' (0x20), so `= SPACE` is false → "NO". (A SIGNED numeric vs SPACE is a
    // later rung, rejected identically on both engines — see the oracle-unit test.)
    let out = assert_matches_oracle(&wrap(
        &["01  NUM  PIC 9(3) VALUE 42."],
        &["IF NUM = SPACE DISPLAY \"MATCH\" ELSE DISPLAY \"NO\".", "STOP RUN."],
    ));
    assert_eq!(out, "NO\n");
}


// -------------------------------------------------------------------------
// Cross-category MOVE: alphanumeric → SIGNED numeric — completes the
// Char↔Numeric × signed/unsigned MOVE matrix. vs the cobol-runtime oracle.
//
// An alphanumeric source (`PIC X(m)`) carries NO operational sign — COBOL does
// not read an overpunch from a plain `PIC X` source — so a MOVE into a SIGNED
// receiver (`PIC S9(i)V9(d)`) stores the folded MAGNITUDE and its sign is ALWAYS
// POSITIVE. The fold and scale placement are IDENTICAL to the unsigned-receiver
// path (fold `V = V*10 + (byte - '0')` left→right; slot `V mod 10^(i+d)`); the
// only difference is that DISPLAY of a signed field overpunches the units digit
// on its POSITIVE row (`{A…I` for units 0-9), so the visible output differs from
// the unsigned case even though the magnitude is identical. Both engines reuse
// their existing signed-aware store path (compiler `store_scaled`/`reapply_sign`;
// oracle `move_into` with `Decimal { neg: false }`), so they agree byte-for-byte.
// -------------------------------------------------------------------------

#[test]
fn alphanumeric_to_signed_numeric_move_exact_fit_positive() {
    // PIC X(3)="123" → PIC S9(3): fold 123, positive; units 3 → '{A…I' row → 'C'.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(3) VALUE \"123\".", "01  N  PIC S9(3)."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "12C\n");
}

#[test]
fn alphanumeric_to_signed_numeric_move_units_digit_zero_overpunch() {
    // PIC X(3)="120" → PIC S9(3): magnitude 120, units 0 → positive-row '{' → "12{".
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(3) VALUE \"120\".", "01  N  PIC S9(3)."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "12{\n");
}

#[test]
fn alphanumeric_to_signed_numeric_move_shorter_source_zero_pads() {
    // PIC X(2)="05" → PIC S9(4): fold 5, magnitude 0005; units 5 → positive 'E'.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(2) VALUE \"05\".", "01  N  PIC S9(4)."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "000E\n");
}

#[test]
fn alphanumeric_to_signed_numeric_move_longer_source_truncates_high_order() {
    // PIC X(5)="12345" → PIC S9(3): keep low-order 3 → 345; units 5 → positive 'E'.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(5) VALUE \"12345\".", "01  N  PIC S9(3)."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "34E\n");
}

#[test]
fn alphanumeric_to_signed_scaled_numeric_move() {
    // PIC X(3)="042" → PIC S9(2)V9: fold 42, slot 042 (scale d=1); DISPLAY shows
    // the raw 3 digits with the units overpunched, positive → units 2 → 'B'.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(3) VALUE \"042\".", "01  N  PIC S9(2)V9."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "04B\n");
}

#[test]
fn alphanumeric_to_signed_numeric_move_space_source_no_stray_sign() {
    // A SPACE source (0x20) is below '0', so `(b-'0')` goes negative and the fold
    // goes negative — but an alphanumeric source has NO operational sign, so BOTH
    // engines take the MAGNITUDE and store it POSITIVE (never a stray '-'), even
    // into a SIGNED receiver. " " → fold -16 → magnitude 016; units 6 → positive
    // 'F'. assert_matches_oracle fails if the two engines disagree.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(1) VALUE \" \".", "01  N  PIC S9(3)."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "01F\n");
}

#[test]
fn alphanumeric_to_signed_numeric_move_is_a_genuine_number() {
    // The moved value is a real (positive) number, not just bytes: MOVE "40" into
    // S9(3) → 040, then ADD 2 → 42; DISPLAY shows units 2 overpunched positive 'B'.
    let out = assert_matches_oracle(&wrap(
        &["01  A  PIC X(2) VALUE \"40\".", "01  N  PIC S9(3)."],
        &["MOVE A TO N.", "ADD 2 TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "04B\n");
}

#[test]
fn alphanumeric_to_signed_numeric_move_with_filler_between_agrees() {
    // Cross-producer binding guard: an unnamed FILLER PIC X(2) sits BETWEEN the
    // named source and the named signed receiver in WORKING-STORAGE. The compiler
    // DROPS FILLER items from its data model while the oracle PUSHES them, so the
    // two engines assign different physical slots; this pins that NAMED binding
    // (A → N) stays byte-identical across the two data-division models.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(3) VALUE \"123\".",
            "01  FILLER  PIC X(2).",
            "01  N  PIC S9(3).",
        ],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    ));
    assert_eq!(out, "12C\n");
}

#[test]
fn alphanumeric_to_signed_numeric_move_non_ascii_is_the_preexisting_byte_char_chip() {
    // Non-ASCII documentation + regression guard. A multi-byte source char exposes
    // the PRE-EXISTING byte-vs-char chip that this rung does NOT touch: the oracle
    // folds the FULL byte storage (`chars.bytes()` over the char-padded value),
    // while the compiler's `emit_str_to_int` folds only the item's `width()` =
    // CHAR-COUNT leading bytes. When the source holds a multi-byte char these two
    // read a DIFFERENT number of bytes, so the two engines already disagree — and
    // they disagree IDENTICALLY on the UNSIGNED path, proving the divergence is not
    // introduced by the signed-receiver relaxation.
    //
    // `PIC X(2) VALUE "é"`: "é" is 1 char / 2 UTF-8 bytes, char-padded to 2 chars →
    // "é " → bytes `C3 A9 20`. The oracle folds all three; the compiler folds the
    // first two (`width()` == 2). Hence the magnitudes differ (oracle 894, compiler
    // 591) on BOTH receiver signednesses. This is deferred to the non-ASCII chip; we
    // do NOT use `assert_matches_oracle` here because a clean parity assertion is
    // impossible for a multi-byte fold. Instead we pin all four outputs so a future
    // regression in EITHER the fold or the sign relaxation is caught, and we assert
    // the sign relaxation is orthogonal: signed == unsigned with the units digit
    // overpunched on the POSITIVE row (4→'D', 1→'A'), and nothing else changed.
    let unsigned_src = wrap(
        &["01  A  PIC X(2) VALUE \"é\".", "01  N  PIC 9(3)."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    );
    let signed_src = wrap(
        &["01  A  PIC X(2) VALUE \"é\".", "01  N  PIC S9(3)."],
        &["MOVE A TO N.", "DISPLAY N.", "STOP RUN."],
    );
    // Oracle: full-byte fold. Unsigned 894; signed adds the positive overpunch (4→'D').
    assert_eq!(run_cobol(&unsigned_src).expect("oracle unsigned"), "894\n");
    assert_eq!(run_cobol(&signed_src).expect("oracle signed"), "89D\n");
    // Compiler: char-count-byte fold. Unsigned 591; signed adds the overpunch (1→'A').
    assert_eq!(run_on_jit(&unsigned_src), "591\n");
    assert_eq!(run_on_jit(&signed_src), "59A\n");
}

// ---------------------------------------------------------------------------
// Figurative constant SPACE / ZERO as a SINGLE-CHARACTER delimiter / search /
// replace / region operand. Wherever a single-character operand is taken through
// the shared delimiter helpers — a DELIMITED BY delimiter (STRING, UNSTRING), an
// INSPECT TALLYING FOR ALL delimiter, an INSPECT REPLACING search AND replace
// char, and an INSPECT BEFORE/AFTER region delimiter — a figurative SPACE or ZERO
// now resolves to its single ASCII character (SPACE → " " 0x20, ZERO → "0" 0x30),
// reducing to the existing single-character-literal path. Co-total across all
// three shared helpers (oracle single_delim_char; compiler single_delim_code for
// the byte scan and single_delim_str for the 1-char replace string). Both images
// are ASCII, so the pre-existing non-ASCII byte-vs-char behaviour is untouched.
// Completes the figurative operand-class arc (CONVERTING, STRING sending field,
// UNSTRING source).
// ---------------------------------------------------------------------------

#[test]
fn unstring_figurative_space_delimiter() {
    // DELIMITED BY SPACE folds to the " " delimiter: "HI YOU" splits at the space
    // into "HI" and "YOU", each left-justified and space-padded to its X(3) width.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(6) VALUE \"HI YOU\".",
            "01  A  PIC X(3) VALUE SPACES.",
            "01  B  PIC X(3) VALUE SPACES.",
        ],
        &[
            "UNSTRING S DELIMITED BY SPACE INTO A B.",
            "DISPLAY A.",
            "DISPLAY B.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "HI \nYOU\n");
}

#[test]
fn unstring_figurative_zero_delimiter() {
    // DELIMITED BY ZERO folds to the "0" delimiter: "AB0CD" splits at the '0' into
    // "AB" and "CD", each exactly filling its X(2) receiver.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"AB0CD\".",
            "01  A  PIC X(2) VALUE SPACES.",
            "01  B  PIC X(2) VALUE SPACES.",
        ],
        &[
            "UNSTRING S DELIMITED BY ZERO INTO A B.",
            "DISPLAY A.",
            "DISPLAY B.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "AB\nCD\n");
}

#[test]
fn string_figurative_space_delimiter() {
    // STRING … DELIMITED BY SPACE folds to the " " delimiter, truncating each
    // sending field at its first space: "HI YOU" contributes only its prefix "HI",
    // overlaid leftmost into D (the X(5) tail keeps its VALUE SPACES).
    let out = assert_matches_oracle(&wrap(
        &["01  S1 PIC X(6) VALUE \"HI YOU\".", "01  D  PIC X(5) VALUE SPACES."],
        &["STRING S1 DELIMITED BY SPACE INTO D.", "DISPLAY D.", "STOP RUN."],
    ));
    assert_eq!(out, "HI   \n");
}

#[test]
fn inspect_tallying_figurative_space_delimiter() {
    // FOR ALL SPACE folds to the " " delimiter: "A B C" holds two spaces, so the
    // counter goes 0 + 2 = 2.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"A B C\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL SPACE.", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "002\n");
}

#[test]
fn inspect_replacing_figurative_search() {
    // REPLACING ALL SPACE folds the SEARCH char to " " (single_delim_code): every
    // space of "A B C" is replaced by "_" → "A_B_C".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"A B C\"."],
        &["INSPECT S REPLACING ALL SPACE BY \"_\".", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "A_B_C\n");
}

#[test]
fn inspect_replacing_figurative_replace() {
    // REPLACING … BY ZERO folds the REPLACE char to "0" (single_delim_str, the
    // 1-char replace-string helper): every 'X' of "XYXY" becomes "0" → "0Y0Y".
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"XYXY\"."],
        &["INSPECT S REPLACING ALL \"X\" BY ZERO.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "0Y0Y\n");
}

#[test]
fn inspect_figurative_region_delimiter() {
    // AFTER SPACE folds the REGION delimiter to " ": "XX XX" restricts the tally
    // window to everything right of the first space → "XX", which holds two 'X's,
    // so the counter goes 0 + 2 = 2.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"XX XX\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL \"X\" AFTER SPACE.", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "002\n");
}

#[test]
fn figurative_delimiter_plural_spellings() {
    // SPACE/SPACES and ZERO/ZEROS/ZEROES are the SAME figurative constant, so every
    // spelling folds to the identical single delimiter char in a delimiter context.
    // SPACES → " " counting the two spaces of "A B C" → 002.
    let spaces = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"A B C\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL SPACES.", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(spaces, "002\n");

    // ZEROS → "0" counting the three zeros of "0A0B0" → 003.
    let zeros = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0A0B0\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL ZEROS.", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(zeros, "003\n");

    // ZEROES → "0" folds identically to ZEROS: same source, same 003.
    let zeroes = assert_matches_oracle(&wrap(
        &["01  S  PIC X(5) VALUE \"0A0B0\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL ZEROES.", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(zeroes, "003\n");
    assert_eq!(zeros, zeroes, "ZEROS and ZEROES fold to the same single delimiter char");
}

#[test]
fn figurative_delimiter_all_ascii_characterization() {
    // NON-ASCII PARITY: `Fig` is a CLOSED enum {Space, Zero}, both inherently ASCII
    // (SPACE 0x20, ZERO 0x30), so NO non-ASCII figurative constant can ever reach a
    // delimiter helper — the single-char image is always one ASCII byte, and the
    // compiler's byte scan and the oracle's char scan coincide. There is no
    // diverging non-ASCII figurative case to construct. The separate pre-existing
    // byte-vs-char concern is a non-ASCII single-char LITERAL/ITEM delimiter
    // (task_396ba6f6), deliberately not exercised here. This case simply pins an
    // all-ASCII figurative delimiter to byte-identity: FOR ALL ZERO over "1020304"
    // counts the three '0' bytes → 003.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(7) VALUE \"1020304\".", "01  C  PIC 9(3) VALUE 0."],
        &["INSPECT S TALLYING C FOR ALL ZERO.", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "003\n");
}

#[test]
fn inspect_replacing_characters_by_figurative() {
    // REPLACING CHARACTERS BY ZERO folds the replace char to "0" through the
    // `single_delim_str` CHARACTERS path (the replace-string helper): EVERY position
    // of "ABC" becomes "0" → "000". Exercises the CHARACTERS-BY call site directly.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(3) VALUE \"ABC\"."],
        &["INSPECT S REPLACING CHARACTERS BY ZERO.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "000\n");
}

#[test]
fn inspect_replacing_multi_item_figurative() {
    // A MULTI-ITEM REPLACING list where BOTH the search char (via `single_delim_code`)
    // and the replace char (via `single_delim_str`) are figuratives, on two separate
    // items: ALL "A" BY SPACE, ALL "B" BY ZERO over "AABB" → "  00". Exercises the
    // multi-item search+replace pairing, proving the two lifted helpers stay co-total
    // together.
    let out = assert_matches_oracle(&wrap(
        &["01  S  PIC X(4) VALUE \"AABB\"."],
        &["INSPECT S REPLACING ALL \"A\" BY SPACE ALL \"B\" BY ZERO.", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "  00\n");
}

#[test]
fn inspect_tallying_multi_counter_figurative() {
    // A MULTI-COUNTER TALLYING where one counter tallies a figurative delimiter
    // (SPACE, via the multi-counter `single_delim_code` path) and the other a literal:
    // "X X" has 1 space and 2 'X' → C1 = 1, C2 = 2. Exercises the multi-counter delim
    // call site.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S   PIC X(3) VALUE \"X X\".",
            "01  C1  PIC 9(3) VALUE 0.",
            "01  C2  PIC 9(3) VALUE 0.",
        ],
        &[
            "INSPECT S TALLYING C1 FOR ALL SPACE C2 FOR ALL \"X\".",
            "DISPLAY C1.",
            "DISPLAY C2.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "001\n002\n");
}
// ---------------------------------------------------------------------------
// CONSTANT reference-modified single-character delimiter / search / replace
// operand. A refmod `BASE(start:len)` with LITERAL indices and slice-length
// exactly 1 reduces to the same single ASCII character the single-char-literal /
// figurative path already handles — so `D(2:1)` is a first-class delimiter,
// search char, and replace char wherever a single-character operand is taken
// through the three shared helpers (oracle `single_delim_char`; compiler
// `single_delim_code` for the byte scan and `single_delim_str` for the 1-char
// replace string). The slice is carved by the SAME machinery DISPLAY /
// comparison / MOVE-source use (`refmod_string` in the oracle, `ref_mod_slice`
// in the compiler), so the reconstructed char is byte-identical on ASCII bases.
// A length != 1 constant refmod is a multi-character delimiter (later rung); a
// COMPUTED (data-name index) refmod is a computed delimiter (later rung) — both
// rejected co-total. Completes the delimiter/search/replace operand-class arc
// (literal, item, figurative #78, refmod).
// ---------------------------------------------------------------------------

#[test]
fn unstring_const_refmod_delimiter() {
    // DELIMITED BY D(2:1) where D = "X,Y" slices out "," (the middle char), so
    // "A,B,C" splits into three fields for three PIC X(3) receivers — identical to
    // the plain `DELIMITED BY ","` rung, only the delimiter is a const-refmod slice.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"A,B,C\".",
            "01  D  PIC X(3) VALUE \"X,Y\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  R3 PIC X(3) VALUE SPACES.",
        ],
        &[
            "UNSTRING S DELIMITED BY D(2:1) INTO R1 R2 R3.",
            "DISPLAY R1.",
            "DISPLAY R2.",
            "DISPLAY R3.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "A  \nB  \nC  \n");
}

#[test]
fn inspect_tallying_const_refmod_delimiter() {
    // FOR ALL D(2:1) where D = "XAY" slices out "A"; "ABABA" holds three 'A's, so
    // the counter goes 0 + 3 = 3. Exercises `single_delim_code` via INSPECT.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"ABABA\".",
            "01  D  PIC X(3) VALUE \"XAY\".",
            "01  C  PIC 9(3) VALUE 0.",
        ],
        &["INSPECT S TALLYING C FOR ALL D(2:1).", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "003\n");
}

#[test]
fn string_const_refmod_delimiter() {
    // STRING A B DELIMITED BY D(2:1) where D = "X,Y" slices out ",", truncating each
    // sending field at its first comma: "ab,cd" -> "ab", "ef" -> "ef" (no comma);
    // "abef" overlaid leftmost into the X(6) receiver. Exercises `single_delim_code`
    // via STRING.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  A  PIC X(5) VALUE \"ab,cd\".",
            "01  B  PIC X(2) VALUE \"ef\".",
            "01  D  PIC X(3) VALUE \"X,Y\".",
            "01  T  PIC X(6) VALUE SPACES.",
        ],
        &[
            "STRING A B DELIMITED BY D(2:1) INTO T.",
            "DISPLAY T.",
            "STOP RUN.",
        ],
    ));
    assert_eq!(out, "abef  \n");
}

#[test]
fn inspect_replacing_const_refmod_search_and_replace() {
    // REPLACING ALL D(2:1) BY E(1:1): the SEARCH char comes through `single_delim_code`
    // (D = "XAY" -> "A") and the REPLACE char through `single_delim_str` (E = "ZBC" ->
    // "Z"), so every 'A' of "ABABA" becomes 'Z' -> "ZBZBZ". One test hits BOTH lifted
    // compiler helpers, proving they stay co-total together.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"ABABA\".",
            "01  D  PIC X(3) VALUE \"XAY\".",
            "01  E  PIC X(3) VALUE \"ZBC\".",
        ],
        &["INSPECT S REPLACING ALL D(2:1) BY E(1:1).", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "ZBZBZ\n");
}

#[test]
fn const_refmod_multi_char_delimiter_is_a_later_rung() {
    // A constant refmod of slice-length != 1 is a MULTI-CHARACTER delimiter: D(1:2)
    // of "X,Y" is "X," (two chars), deferred and rejected on BOTH engines to stay
    // co-total (`SliceLen::Const(_ != 1)` in the compiler; a two-char slice in the
    // oracle's `[c]` match).
    let src = wrap(
        &[
            "01  S  PIC X(5) VALUE \"A,B,C\".",
            "01  D  PIC X(3) VALUE \"X,Y\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
        ],
        &["UNSTRING S DELIMITED BY D(1:2) INTO R1 R2.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a length-2 const-refmod delimiter");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a length-2 const-refmod delimiter"
    );
}

#[test]
fn computed_refmod_delimiter_is_a_later_rung() {
    // A COMPUTED refmod delimiter — one whose start index is a DATA-NAME (D(J:1)) —
    // has a run-time length the compile-time contract cannot carry, so it stays a
    // later rung, rejected on BOTH engines (the oracle's `const_ix` predicate is
    // false; the compiler's `ref_mod_slice` yields `SliceLen::Runtime`). Mirrors the
    // Const/Runtime split #74 established for the CONVERTING refmod.
    let src = wrap(
        &[
            "01  S  PIC X(5) VALUE \"A,B,C\".",
            "01  D  PIC X(3) VALUE \"X,Y\".",
            "01  J  PIC 9   VALUE 2.",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
        ],
        &["UNSTRING S DELIMITED BY D(J:1) INTO R1 R2.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a computed-refmod delimiter");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a computed-refmod delimiter"
    );
}

#[test]
fn const_refmod_delimiter_ascii_window_non_ascii_outside() {
    // Non-ASCII CLEANLINESS: the base D = ",bcdé" carries a multi-byte char 'é', but
    // the (1:1) window selects only the FIRST char ",", strictly BEFORE 'é'. Within
    // that window byte-index == char-index, so the compiler's byte-based slice and
    // the oracle's char-based slice coincide on the ASCII ",", and the tally over
    // "X,Y,Z" (two commas) agrees byte-for-byte -> 002. (A window covering or
    // following the multi-byte char is the pre-existing refmod byte-vs-char chip,
    // task_396ba6f6, deliberately not exercised here.)
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"X,Y,Z\".",
            "01  D  PIC X(5) VALUE \",bcdé\".",
            "01  C  PIC 9(3) VALUE 0.",
        ],
        &["INSPECT S TALLYING C FOR ALL D(1:1).", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "002\n");
}

#[test]
fn group_base_refmod_delimiter_is_a_later_rung() {
    // A GROUP base is rejected on BOTH engines: the compiler's `ref_mod_slice`
    // rejects a group base via `item_index`, and the oracle's `single_delim_char`
    // now rejects a group base up front (rather than slicing `group_image`) so the
    // delimiter site stays co-total. (Without that oracle guard the oracle would
    // ACCEPT `G(2:1)` while the compiler rejected it — the pre-existing group-refmod
    // chip, here fenced off at the new delimiter site.)
    let src = wrap(
        &[
            "01  S  PIC X(5) VALUE \"A,B,C\".",
            "01  G.",
            "    05 GA PIC X(3) VALUE \"x,y\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
        ],
        &["UNSTRING S DELIMITED BY G(2:1) INTO R1 R2.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a group-base refmod delimiter");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a group-base refmod delimiter"
    );
}

#[test]
fn numeric_base_refmod_delimiter_is_a_later_rung() {
    // A NUMERIC base is rejected on BOTH engines: the compiler's `ref_mod_slice`
    // (via `item_index`→`Numeric` arm) and the oracle's `refmod_string` both reject
    // "reference modification of a numeric item", reached through the lifted
    // `RefMod` arm. Co-total.
    let src = wrap(
        &[
            "01  S  PIC X(5) VALUE \"A,B,C\".",
            "01  D  PIC 9(3) VALUE 123.",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
        ],
        &["UNSTRING S DELIMITED BY D(2:1) INTO R1 R2.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a numeric-base refmod delimiter");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a numeric-base refmod delimiter"
    );
}

#[test]
fn const_refmod_omitted_length_delimiter() {
    // An OMITTED length `D(3:)` runs to the end of the 3-char base "ab," — that is
    // the single char "," (length 1), so it joins the single-char path. This is the
    // one accepted path whose length is NOT a literal: the compiler's
    // `const_refmod_len` computes `width - start0` and the oracle slices to `width`.
    // Tallying "," over "X,Y,Z" agrees byte-for-byte -> 002.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"X,Y,Z\".",
            "01  D  PIC X(3) VALUE \"ab,\".",
            "01  C  PIC 9(3) VALUE 0.",
        ],
        &["INSPECT S TALLYING C FOR ALL D(3:).", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "002\n");
}

#[test]
fn const_refmod_omitted_length_multichar_is_a_later_rung() {
    // An OMITTED length that runs to more than one character — `D(2:)` on the 3-char
    // "ab," is "b," (length 2) — is a MULTI-CHARACTER delimiter, rejected on BOTH
    // engines (`SliceLen::Const(2)` in the compiler; a two-char slice in the oracle's
    // `[c]` match). Co-total.
    let src = wrap(
        &[
            "01  S  PIC X(5) VALUE \"A,B,C\".",
            "01  D  PIC X(3) VALUE \"ab,\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
        ],
        &["UNSTRING S DELIMITED BY D(2:) INTO R1 R2.", "STOP RUN."],
    );
    assert!(run_cobol(&src).is_err(), "oracle must reject a length-2 omitted-length refmod delimiter");
    assert!(
        compile_source(&src, "e2e").is_err(),
        "compiler must reject a length-2 omitted-length refmod delimiter"
    );
}

#[test]
fn const_refmod_region_delimiter() {
    // A refmod as an INSPECT BEFORE/AFTER REGION delimiter (oracle site
    // `single_delim_char`@region; compiler `single_delim_code` via the region-window
    // emitter). D(2:1) of "x,y" is ",". The BEFORE-"," region of "aa,aa" is "aa", in
    // which "a" occurs twice -> 002. Positive parity, exercising the region site.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(5) VALUE \"aa,aa\".",
            "01  D  PIC X(3) VALUE \"x,y\".",
            "01  C  PIC 9(3) VALUE 0.",
        ],
        &["INSPECT S TALLYING C FOR ALL \"a\" BEFORE D(2:1).", "DISPLAY C.", "STOP RUN."],
    ));
    assert_eq!(out, "002\n");
}

#[test]
fn const_refmod_replacing_characters_by() {
    // A refmod as the REPLACING CHARACTERS BY replacement (oracle site
    // `single_delim_char`@1593; compiler `single_delim_str`@CHARACTERS-BY). E(1:1) of
    // "*!" is "*", so every character of "ABC" becomes "*" -> "***". Positive parity,
    // exercising the `single_delim_str` refmod path via CHARACTERS BY.
    let out = assert_matches_oracle(&wrap(
        &[
            "01  S  PIC X(3) VALUE \"ABC\".",
            "01  E  PIC X(2) VALUE \"*!\".",
        ],
        &["INSPECT S REPLACING CHARACTERS BY E(1:1).", "DISPLAY S.", "STOP RUN."],
    ));
    assert_eq!(out, "***\n");
}
