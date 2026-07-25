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
// half may count only the LEADING run of the delimiter while the REPLACING half
// stays ALL. Tally still runs FIRST over the original bytes, so a shared
// delimiter/search is counted before it is substituted. The compiler reuses the
// lone-TALLYING leading lowering and the oracle its leading count path, so the JIT
// output stays byte-identical.

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

#[test]
fn inspect_combined_replacing_leading_is_a_later_rung() {
    // The combined form's REPLACING half stays ALL only: a combined
    // `TALLYING … REPLACING LEADING` is still a later rung, rejected identically on
    // both engines (a lone REPLACING LEADING, by contrast, is supported).
    let err = compile_source(
        &wrap(
            &["01  S  PIC X(5) VALUE \"AABBB\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"B\" REPLACING LEADING \"A\" BY \"X\".",
                "STOP RUN.",
            ],
        ),
        "insp_combined_repl_leading",
    )
    .unwrap_err();
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

#[test]
fn inspect_converting_item_operand_is_a_later_rung() {
    // A PIC X item as the `from` operand (rather than a string literal) is a later
    // rung this slice.
    let err = compile_source(
        &wrap(
            &["01  S  PIC X(3) VALUE \"ABC\".", "01  F  PIC X(3) VALUE \"ABC\"."],
            &["INSPECT S CONVERTING F TO \"XYZ\".", "STOP RUN."],
        ),
        "insp_conv_item",
    )
    .unwrap_err();
    assert!(matches!(err, cobol_iir_compiler::CompileError::Unsupported(_)), "got {err:?}");
}

#[test]
fn inspect_converting_before_region_is_a_later_rung() {
    // A BEFORE/AFTER region restricting the conversion parses but is a later rung.
    let err = compile_source(
        &wrap(
            &["01  S  PIC X(5) VALUE \"ABABA\"."],
            &["INSPECT S CONVERTING \"A\" TO \"X\" BEFORE \"B\".", "STOP RUN."],
        ),
        "insp_conv_before",
    )
    .unwrap_err();
    assert!(matches!(err, cobol_iir_compiler::CompileError::Unsupported(_)), "got {err:?}");
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
