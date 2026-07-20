//! # COBOL runtime — running COBOL, where the quirks live.
//!
//! A tree-walking interpreter for COBOL-60, built on the `cobol-parser` CST. It
//! turns WORKING-STORAGE into a **PICTURE-typed data model** and executes the
//! PROCEDURE DIVISION, capturing everything `DISPLAY`ed. See
//! [PL08](../../../specs/PL08-cobol-runtime.md).
//!
//! It implements a *small but fully correct* slice — `MOVE` / `DISPLAY` /
//! `STOP RUN`, fixed-point decimal `ADD` / `SUBTRACT` / `MULTIPLY` / `DIVIDE`
//! (with `ROUNDED` and `ON SIZE ERROR`),
//! `COMPUTE` (precedence-correct arithmetic expressions with `+ - * / **`, unary
//! sign and parentheses, `ROUNDED`, and `ON SIZE ERROR`), `IF … ELSE`
//! (numeric and alphanumeric comparison),
//! `PERFORM para [THRU para2] [n TIMES | UNTIL cond | VARYING id FROM x BY y UNTIL cond]`
//! (out-of-line paragraph or paragraph-range invocation — fixed-count,
//! conditional, and counted loops), and `GO TO para` (unconditional transfer,
//! including back-edge loops) over numeric-display (`9`/`V`, and
//! signed `S` with trailing-overpunch display) and character pictures — and
//! returns a descriptive error for anything not yet modelled, rather than
//! producing wrong output. The roadmap toward full COBOL (the `SIGN` clause and
//! `SEPARATE`/`LEADING` variants, editing pictures, `PERFORM … THRU`/`UNTIL`/
//! `VARYING`, `GO TO … DEPENDING`, tables, files, later standards) is in PL08.
//!
//! ```
//! use coding_adventures_cobol_runtime::run_cobol;
//! // Card columns 1–6 are the sequence area; code begins in column 8.
//! let src = "\
//! 000010 IDENTIFICATION DIVISION.
//! 000020 PROGRAM-ID. HELLO.
//! 000030 PROCEDURE DIVISION.
//! 000040 MAIN.
//! 000050     DISPLAY \"HELLO, WORLD\".
//! 000060     STOP RUN.";
//! assert_eq!(run_cobol(src).unwrap(), "HELLO, WORLD\n");
//! ```

use coding_adventures_cobol_parser::try_parse_cobol;

mod error;
mod interp;
mod picture;
mod program;
mod value;

pub use error::RuntimeError;

// The PICTURE-typed data model's building blocks, re-exported so a *compiler*
// (not just this tree-walk interpreter) can reuse COBOL's exact picture and
// fixed-point-value logic. `cobol-iir-compiler` lowers COBOL to IIR and depends
// on these to format literals into their stored picture image at compile time —
// so its output is byte-identical to this oracle's `DISPLAY` (PL09).
pub use interp::COMPUTE_DIV_SCALE;
pub use picture::Picture;
pub use value::{move_into_char, move_into_numeric, Decimal, MAX_POW_EXP};

/// Parse and run a COBOL program, returning everything it `DISPLAY`ed (the
/// captured console, each `DISPLAY` terminated by a newline).
///
/// Lexical and parse errors surface as [`RuntimeError::Parse`]; constructs the
/// v0.1 runtime does not yet model surface as descriptive runtime errors.
pub fn run_cobol(source: &str) -> Result<String, RuntimeError> {
    let cst = try_parse_cobol(source).map_err(RuntimeError::Parse)?;
    let program = program::read_program(&cst)?;
    let machine = interp::Machine::new(&program)?;
    machine.run(&program)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Assemble a program from code lines, carding each (6 sequence columns + a
    /// space indicator, then the code beginning in column 8).
    fn program(lines: &[&str]) -> String {
        lines.iter().map(|l| format!("000000 {l}")).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn hello_world() {
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. HELLO.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    DISPLAY \"HELLO, WORLD\".",
            "    STOP RUN.",
        ]))
        .unwrap();
        assert_eq!(out, "HELLO, WORLD\n");
    }

    #[test]
    fn character_move_space_pads_and_displays_stored_width() {
        // MOVE "HI" TO a PIC X(5) → stored "HI   "; DISPLAY shows all 5 columns.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  WORD  PIC X(5).",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    MOVE \"HI\" TO WORD.",
            "    DISPLAY WORD.",
            "    STOP RUN.",
        ]))
        .unwrap();
        assert_eq!(out, "HI   \n");
    }

    #[test]
    fn character_move_truncates_on_the_right() {
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  WORD  PIC X(3).",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    MOVE \"HELLO\" TO WORD.",
            "    DISPLAY WORD.",
            "    STOP RUN.",
        ]))
        .unwrap();
        assert_eq!(out, "HEL\n");
    }

    #[test]
    fn numeric_move_zero_fills_and_displays_raw_digits() {
        // MOVE 42 TO PIC 9(5) → "00042"; DISPLAY shows the raw digits.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  N  PIC 9(5).",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    MOVE 42 TO N.",
            "    DISPLAY N.",
            "    STOP RUN.",
        ]))
        .unwrap();
        assert_eq!(out, "00042\n");
    }

    #[test]
    fn numeric_move_truncates_and_implied_decimal_has_no_point() {
        // MOVE 123.456 TO PIC 9(2)V9 → integer keeps "23", fraction keeps "4"
        // → stored "234"; DISPLAY shows "234" (no decimal point).
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  N  PIC 9(2)V9.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    MOVE 123.456 TO N.",
            "    DISPLAY N.",
            "    STOP RUN.",
        ]))
        .unwrap();
        assert_eq!(out, "234\n");
    }

    #[test]
    fn display_concatenates_operands_with_no_separator() {
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  A  PIC X(3) VALUE \"FOO\".",
            "01  B  PIC 9(2) VALUE 7.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    DISPLAY A B \"!\".",
            "    STOP RUN.",
        ]))
        .unwrap();
        // "FOO" + "07" + "!" — no spaces between operands.
        assert_eq!(out, "FOO07!\n");
    }

    #[test]
    fn value_initialization_and_figuratives() {
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  N  PIC 9(3) VALUE ZERO.",
            "01  S  PIC X(4) VALUE SPACES.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    DISPLAY N.",
            "    DISPLAY S \"|\".",
            "    STOP RUN.",
        ]))
        .unwrap();
        assert_eq!(out, "000\n    |\n");
    }

    #[test]
    fn group_item_displays_concatenation_of_children() {
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  FULL-NAME.",
            "    02  FIRST  PIC X(3) VALUE \"AMY\".",
            "    02  LAST   PIC X(4) VALUE \"LEE\".",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    DISPLAY FULL-NAME \"|\".",
            "    STOP RUN.",
        ]))
        .unwrap();
        // FIRST "AMY" + LAST "LEE " (space-padded to 4) → "AMYLEE " then "|".
        assert_eq!(out, "AMYLEE |\n");
    }

    #[test]
    fn item_to_item_move_reshapes_to_receiver_picture() {
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  SRC  PIC 9(3) VALUE 42.",
            "01  DST  PIC 9(5).",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    MOVE SRC TO DST.",
            "    DISPLAY DST.",
            "    STOP RUN.",
        ]))
        .unwrap();
        // SRC holds "042"; moved into 9(5) → "00042".
        assert_eq!(out, "00042\n");
    }

    // ----------------------------------------------------------------------
    // Fixed-point decimal arithmetic
    // ----------------------------------------------------------------------

    /// Run a program whose WORKING-STORAGE is one field `R PIC <pic>`, execute
    /// `body`, then `DISPLAY R` — returns R's displayed digits.
    fn compute(pic: &str, body: &[&str], extra_ws: &[&str]) -> String {
        let mut lines = vec![
            "IDENTIFICATION DIVISION.".to_string(),
            "PROGRAM-ID. P.".to_string(),
            "DATA DIVISION.".to_string(),
            "WORKING-STORAGE SECTION.".to_string(),
            format!("01  R  PIC {pic}."),
        ];
        lines.extend(extra_ws.iter().map(|s| s.to_string()));
        lines.push("PROCEDURE DIVISION.".to_string());
        lines.push("MAIN.".to_string());
        lines.extend(body.iter().map(|s| format!("    {s}")));
        lines.push("    DISPLAY R.".to_string());
        lines.push("    STOP RUN.".to_string());
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        run_cobol(&program(&refs)).unwrap()
    }

    #[test]
    fn add_to_accumulates_into_the_receiver() {
        // R starts 10; ADD 5 3 TO R → 18.
        assert_eq!(compute("9(3)", &["MOVE 10 TO R.", "ADD 5 3 TO R."], &[]), "018\n");
    }

    #[test]
    fn add_giving_leaves_the_to_field_unchanged() {
        // ADD 2 3 TO A GIVING R → R = 2+3+A(=100) = 105; A untouched.
        let out = compute(
            "9(3)",
            &["ADD 2 3 TO A GIVING R."],
            &["01  A  PIC 9(3) VALUE 100."],
        );
        assert_eq!(out, "105\n");
    }

    #[test]
    fn subtract_from_and_unsigned_receiver_keeps_magnitude() {
        // SUBTRACT 3 FROM R(=10) → 7.
        assert_eq!(compute("9(3)", &["MOVE 10 TO R.", "SUBTRACT 3 FROM R."], &[]), "007\n");
        // SUBTRACT 5 FROM R(=3) → -2, but R is unsigned → stores magnitude 2.
        assert_eq!(compute("9(3)", &["MOVE 3 TO R.", "SUBTRACT 5 FROM R."], &[]), "002\n");
    }

    #[test]
    fn multiply_fixed_point_truncates_into_receiver() {
        // 2.5 * 2.5 = 6.25 → into PIC 9(3)V9 truncates to "0062".
        let out = compute("9(3)V9", &["MULTIPLY 2.5 BY 2.5 GIVING R."], &[]);
        assert_eq!(out, "0062\n");
    }

    #[test]
    fn multiply_by_updates_the_by_field_without_giving() {
        // MOVE 6 TO R; MULTIPLY 7 BY R → R = 42.
        assert_eq!(compute("9(3)", &["MOVE 6 TO R.", "MULTIPLY 7 BY R."], &[]), "042\n");
    }

    #[test]
    fn decimal_add_aligns_the_implied_point() {
        // R PIC 9(2)V99 (4 digits) starts 1.50; ADD 2.25 TO R → 3.75 → "0375".
        assert_eq!(compute("9(2)V99", &["MOVE 1.5 TO R.", "ADD 2.25 TO R."], &[]), "0375\n");
    }

    #[test]
    fn divide_into_giving_truncates_to_receiver_decimals() {
        // 10 / 3 = 3.333… → into PIC 9(3)V99 truncates to 3.33 → "00333".
        assert_eq!(compute("9(3)V99", &["DIVIDE 3 INTO 10 GIVING R."], &[]), "00333\n");
        // 10 / 4 = 2.5 → into PIC 9(3) (no decimals) truncates to 2 → "002".
        assert_eq!(compute("9(3)", &["DIVIDE 4 INTO 10 GIVING R."], &[]), "002\n");
    }

    #[test]
    fn divide_into_without_giving_updates_the_dividend() {
        // MOVE 20 TO R; DIVIDE 5 INTO R → R = 20/5 = 4.
        assert_eq!(compute("9(3)", &["MOVE 20 TO R.", "DIVIDE 5 INTO R."], &[]), "004\n");
    }

    #[test]
    fn divide_by_zero_is_a_clear_error() {
        let err = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  R  PIC 9(3).",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    DIVIDE 0 INTO 10 GIVING R.",
            "    STOP RUN.",
        ]))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::DivideByZero), "got {err:?}");
    }

    // ----------------------------------------------------------------------
    // ROUNDED / ON SIZE ERROR on the arithmetic verbs
    // ----------------------------------------------------------------------

    #[test]
    fn divide_giving_rounded_rounds_the_receiver() {
        // 10 / 3 = 3.333… → into V99. Truncated: 3.33; ROUNDED: still 3.33 (3rd
        // place < 5). Use 20 / 3 = 6.666… → truncated 6.66, ROUNDED 6.67.
        let prog = |rounded: &str| {
            run_cobol(&program(&[
                "IDENTIFICATION DIVISION.",
                "PROGRAM-ID. P.",
                "DATA DIVISION.",
                "WORKING-STORAGE SECTION.",
                "01  R  PIC 9(2)V99 VALUE 0.",
                "PROCEDURE DIVISION.",
                "MAIN.",
                &format!("    DIVIDE 3 INTO 20 GIVING R{rounded}."),
                "    DISPLAY R.",
                "    STOP RUN.",
            ]))
            .unwrap()
        };
        assert_eq!(prog(""), "0666\n"); // truncated 6.66
        assert_eq!(prog(" ROUNDED"), "0667\n"); // rounded 6.67
    }

    #[test]
    fn multiply_giving_rounded() {
        // 2.5 * 2.5 = 6.25 into 9(2)V9: truncated 6.2, ROUNDED 6.3 (2nd place 5).
        let prog = |rounded: &str| {
            run_cobol(&program(&[
                "IDENTIFICATION DIVISION.",
                "PROGRAM-ID. P.",
                "DATA DIVISION.",
                "WORKING-STORAGE SECTION.",
                "01  R  PIC 9(2)V9 VALUE 0.",
                "PROCEDURE DIVISION.",
                "MAIN.",
                &format!("    MULTIPLY 2.5 BY 2.5 GIVING R{rounded}."),
                "    DISPLAY R.",
                "    STOP RUN.",
            ]))
            .unwrap()
        };
        assert_eq!(prog(""), "062\n");
        assert_eq!(prog(" ROUNDED"), "063\n");
    }

    #[test]
    fn add_on_size_error_fires_on_overflow() {
        // R is 9(2) (max 99). ADD 50 TO R twice → 100 overflows on the second;
        // the handler runs and R is left unchanged (still 50).
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  R  PIC 9(2) VALUE 50.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    ADD 60 TO R",
            "        ON SIZE ERROR DISPLAY \"OVER\".",
            "    DISPLAY R.",
            "    STOP RUN.",
        ]))
        .unwrap();
        // 50 + 60 = 110 overflows 9(2) → handler runs, R unchanged at 50.
        assert_eq!(out, "OVER\n50\n");
    }

    #[test]
    fn divide_by_zero_with_on_size_error_runs_the_handler() {
        // A zero divisor is a size-error condition; with a handler it is caught.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  R  PIC 9(3) VALUE 7.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    DIVIDE 0 INTO 10 GIVING R",
            "        ON SIZE ERROR DISPLAY \"DIVZERO\".",
            "    DISPLAY R.",
            "    STOP RUN.",
        ]))
        .unwrap();
        assert_eq!(out, "DIVZERO\n007\n");
    }

    // ----------------------------------------------------------------------
    // IF — conditions and branching
    // ----------------------------------------------------------------------

    /// Run a program with `01 N PIC 9(3) VALUE <n>` and the given procedure body.
    fn run_if(n: &str, body: &[&str]) -> String {
        let mut lines = vec![
            "IDENTIFICATION DIVISION.".to_string(),
            "PROGRAM-ID. P.".to_string(),
            "DATA DIVISION.".to_string(),
            "WORKING-STORAGE SECTION.".to_string(),
            format!("01  N  PIC 9(3) VALUE {n}."),
            "PROCEDURE DIVISION.".to_string(),
            "MAIN.".to_string(),
        ];
        lines.extend(body.iter().map(|s| format!("    {s}")));
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        run_cobol(&program(&refs)).unwrap()
    }

    #[test]
    fn if_numeric_true_and_false_branches() {
        // N=5 > 3 → THEN.
        assert_eq!(
            run_if("5", &["IF N GREATER 3 DISPLAY \"BIG\" ELSE DISPLAY \"SMALL\".", "STOP RUN."]),
            "BIG\n"
        );
        // N=1 not > 3 → ELSE.
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
        // N=5 against each symbol: the whole truth table.
        let t = |body: &str| run_if("5", &[body, "STOP RUN."]);
        assert_eq!(t("IF N > 3 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "Y\n"); // 5 > 3
        assert_eq!(t("IF N > 5 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "N\n"); // 5 > 5 false
        assert_eq!(t("IF N < 9 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "Y\n"); // 5 < 9
        assert_eq!(t("IF N = 5 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "Y\n"); // 5 = 5
        assert_eq!(t("IF N >= 5 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "Y\n"); // 5 >= 5 (boundary)
        assert_eq!(t("IF N >= 6 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "N\n"); // 5 >= 6 false
        assert_eq!(t("IF N <= 5 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "Y\n"); // 5 <= 5 (boundary)
        assert_eq!(t("IF N <= 4 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "N\n"); // 5 <= 4 false
        assert_eq!(t("IF N <> 3 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "Y\n"); // 5 <> 3
        assert_eq!(t("IF N <> 5 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "N\n"); // 5 <> 5 false
        // An explicit NOT composes with a symbol's baseline negation: NOT >= ≡ <.
        assert_eq!(t("IF N NOT >= 6 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "Y\n"); // 5 < 6
    }

    #[test]
    fn if_compound_and_or_and_precedence() {
        // N=5 through AND/OR, precedence, and parenthesised grouping.
        let t = |body: &str| run_if("5", &[body, "STOP RUN."]);
        // AND: both hold → true; one fails → false.
        assert_eq!(t("IF N > 3 AND N < 9 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "Y\n");
        assert_eq!(t("IF N > 3 AND N > 9 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "N\n");
        // OR: one holds → true; neither → false.
        assert_eq!(t("IF N < 3 OR N > 4 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "Y\n");
        assert_eq!(t("IF N < 3 OR N > 9 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "N\n");
        // Precedence: AND binds tighter than OR. `N = 1 OR N > 3 AND N < 9`
        // parses as `N=1 OR (N>3 AND N<9)` = false OR true = true.
        assert_eq!(t("IF N = 1 OR N > 3 AND N < 9 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "Y\n");
        // Parentheses override: `(N = 1 OR N > 3) AND N < 4` = true AND false = false.
        assert_eq!(t("IF (N = 1 OR N > 3) AND N < 4 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "N\n");
    }

    #[test]
    fn if_not_negates_a_condition() {
        // N=5. `NOT` binds tighter than AND/OR and negates the following simple
        // condition (relation, parenthesised group, or condition-name).
        let t = |body: &str| run_if("5", &[body, "STOP RUN."]);
        // NOT over a relation: NOT (5 > 3) = false.
        assert_eq!(t("IF NOT N > 3 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "N\n");
        // NOT over a parenthesised group (de Morgan): NOT (5<3 OR 5>9) = true.
        assert_eq!(t("IF NOT (N < 3 OR N > 9) DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "Y\n");
        // Precedence: NOT tighter than OR. `NOT N = 5 OR N > 0` = (NOT 5=5) OR 5>0
        // = false OR true = true.
        assert_eq!(t("IF NOT N = 5 OR N > 0 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "Y\n");
        // NOT tighter than AND: `N > 0 AND NOT N > 9` = true AND (NOT false) = true.
        assert_eq!(t("IF N > 0 AND NOT N > 9 DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "Y\n");
        // A negation-level NOT composes with a relop-level NOT (double negation):
        // NOT (5 IS NOT > 3) = NOT (NOT true) = true.
        assert_eq!(t("IF NOT (N IS NOT GREATER 3) DISPLAY \"Y\" ELSE DISPLAY \"N\"."), "Y\n");
    }

    /// The `EVALUATE N WHEN 1 … WHEN 5 … WHEN OTHER … END-EVALUATE` body one card
    /// per line, wrapped around `01 N PIC 9(3) VALUE {n}`.
    const EVAL_BODY: &[&str] = &[
        "EVALUATE N",
        "WHEN 1 DISPLAY \"ONE\"",
        "WHEN 5 DISPLAY \"FIVE\"",
        "WHEN OTHER DISPLAY \"OTHER\"",
        "END-EVALUATE.",
        "STOP RUN.",
    ];

    #[test]
    fn evaluate_runs_the_first_matching_when() {
        // N=5 matches the second WHEN → "FIVE"; no fall-through to OTHER.
        assert_eq!(run_if("5", EVAL_BODY), "FIVE\n");
        // N=1 matches the first WHEN → "ONE".
        assert_eq!(run_if("1", EVAL_BODY), "ONE\n");
    }

    #[test]
    fn evaluate_falls_through_to_when_other() {
        // N=7 matches no value → the WHEN OTHER branch.
        assert_eq!(run_if("7", EVAL_BODY), "OTHER\n");
    }

    #[test]
    fn evaluate_with_no_match_and_no_other_does_nothing() {
        assert_eq!(
            run_if(
                "7",
                &["EVALUATE N", "WHEN 1 DISPLAY \"ONE\"", "END-EVALUATE.", "DISPLAY \"AFTER\".", "STOP RUN."],
            ),
            "AFTER\n"
        );
    }

    #[test]
    fn evaluate_branch_may_stop_run() {
        // A STOP RUN inside the matched WHEN ends the program (the trailing DISPLAY
        // never runs) — the branch's Flow propagates.
        assert_eq!(
            run_if(
                "5",
                &[
                    "EVALUATE N",
                    "WHEN 5 DISPLAY \"IN\" STOP RUN",
                    "WHEN OTHER DISPLAY \"OTHER\"",
                    "END-EVALUATE.",
                    "DISPLAY \"AFTER\".",
                    "STOP RUN.",
                ],
            ),
            "IN\n"
        );
    }

    #[test]
    fn evaluate_with_thousands_of_whens_iterates_not_recurses() {
        // A crafted EVALUATE with thousands of WHEN branches must evaluate by
        // iteration, never recursion — no stack overflow. The subject matches the
        // last value; branches are one card each so the 80-column format is fine.
        let mut lines = vec![
            "IDENTIFICATION DIVISION.".to_string(),
            "PROGRAM-ID. P.".to_string(),
            "DATA DIVISION.".to_string(),
            "WORKING-STORAGE SECTION.".to_string(),
            "01  N  PIC 9(4) VALUE 2000.".to_string(),
            "PROCEDURE DIVISION.".to_string(),
            "MAIN.".to_string(),
            "    EVALUATE N".to_string(),
        ];
        for i in 1..=2000 {
            lines.push(format!("    WHEN {i} DISPLAY \"HIT\""));
        }
        lines.push("    END-EVALUATE.".to_string());
        lines.push("    STOP RUN.".to_string());
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        assert_eq!(run_cobol(&program(&refs)).unwrap(), "HIT\n");
    }

    /// `EVALUATE N WHEN 1 2 5 DISPLAY "SET" WHEN 7 THRU 9 DISPLAY "RANGE" WHEN
    /// OTHER DISPLAY "OTHER" END-EVALUATE` — a multi-value WHEN and a THRU range.
    const EVAL_RANGES: &[&str] = &[
        "EVALUATE N",
        "WHEN 1 2 5 DISPLAY \"SET\"",
        "WHEN 7 THRU 9 DISPLAY \"RANGE\"",
        "WHEN OTHER DISPLAY \"OTHER\"",
        "END-EVALUATE.",
        "STOP RUN.",
    ];

    #[test]
    fn evaluate_matches_a_multi_value_when() {
        // Any listed value matches the first branch.
        for n in ["1", "2", "5"] {
            assert_eq!(run_if(n, EVAL_RANGES), "SET\n", "N={n}");
        }
    }

    #[test]
    fn evaluate_matches_a_thru_range_inclusive() {
        // 7..=9 match the range branch; 6 falls through to OTHER.
        assert_eq!(run_if("7", EVAL_RANGES), "RANGE\n"); // low boundary
        assert_eq!(run_if("9", EVAL_RANGES), "RANGE\n"); // high boundary
        assert_eq!(run_if("6", EVAL_RANGES), "OTHER\n"); // just below → OTHER
        assert_eq!(run_if("3", EVAL_RANGES), "OTHER\n"); // between the sets → OTHER
    }

    #[test]
    fn evaluate_a_when_may_mix_values_and_a_range() {
        // WHEN 1 5 THRU 7 9 = {1} ∪ {5,6,7} ∪ {9}.
        let body = &[
            "EVALUATE N",
            "WHEN 1 5 THRU 7 9 DISPLAY \"Y\"",
            "WHEN OTHER DISPLAY \"N\"",
            "END-EVALUATE.",
            "STOP RUN.",
        ];
        for (n, want) in [("1", "Y\n"), ("6", "Y\n"), ("9", "Y\n"), ("4", "N\n"), ("8", "N\n")] {
            assert_eq!(run_if(n, body), want, "N={n}");
        }
    }

    /// Build a program whose `01 GRADE PIC X VALUE "{g}"` drives an alphanumeric
    /// EVALUATE, then run `body`.
    fn run_alpha_evaluate(g: &str, body: &[&str]) -> Result<String, RuntimeError> {
        let mut lines = vec![
            "IDENTIFICATION DIVISION.".to_string(),
            "PROGRAM-ID. P.".to_string(),
            "DATA DIVISION.".to_string(),
            "WORKING-STORAGE SECTION.".to_string(),
            format!("01  GRADE  PIC X VALUE \"{g}\"."),
            "PROCEDURE DIVISION.".to_string(),
            "MAIN.".to_string(),
        ];
        lines.extend(body.iter().map(|s| format!("    {s}")));
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        run_cobol(&program(&refs))
    }

    #[test]
    fn evaluate_on_an_alphanumeric_subject() {
        // A char subject matched against string literals — space-padded compare.
        let body = &[
            "EVALUATE GRADE",
            "WHEN \"A\" DISPLAY \"TOP\"",
            "WHEN \"F\" DISPLAY \"FAIL\"",
            "WHEN OTHER DISPLAY \"MID\"",
            "END-EVALUATE.",
            "STOP RUN.",
        ];
        assert_eq!(run_alpha_evaluate("A", body).unwrap(), "TOP\n");
        assert_eq!(run_alpha_evaluate("F", body).unwrap(), "FAIL\n");
        assert_eq!(run_alpha_evaluate("C", body).unwrap(), "MID\n"); // no value → OTHER
    }

    #[test]
    fn evaluate_on_an_alphanumeric_thru_range() {
        // A THRU range over characters: "A" THRU "M" (byte-lexical). B and M are in
        // range; Z is above.
        let body = &[
            "EVALUATE GRADE",
            "WHEN \"A\" THRU \"M\" DISPLAY \"FIRST-HALF\"",
            "WHEN OTHER DISPLAY \"REST\"",
            "END-EVALUATE.",
            "STOP RUN.",
        ];
        assert_eq!(run_alpha_evaluate("B", body).unwrap(), "FIRST-HALF\n");
        assert_eq!(run_alpha_evaluate("M", body).unwrap(), "FIRST-HALF\n"); // high boundary
        assert_eq!(run_alpha_evaluate("Z", body).unwrap(), "REST\n"); // above the range
    }

    #[test]
    fn if_then_branch_runs_multiple_statements() {
        // THEN branch has two statements; both run when the condition holds.
        assert_eq!(
            run_if("5", &["IF N GREATER 3 MOVE 8 TO N DISPLAY N.", "STOP RUN."]),
            "008\n"
        );
    }

    #[test]
    fn a_huge_flat_and_chain_evaluates_by_iteration_not_recursion() {
        // A crafted `A AND A AND … (thousands)` is grammar *repetition* — flat
        // siblings, not depth-capped nesting — so it builds one `Cond::And` with a
        // long `Vec`. Evaluation iterates that list, so it must NOT overflow the
        // stack. (Before the n-ary fix this folded into a depth-N tree and blew the
        // stack.) One `AND` term per card so the 80-column format doesn't truncate
        // the statement (it flows freely across cards). 5000 terms all hold → THEN.
        let mut lines = vec![
            "IDENTIFICATION DIVISION.".to_string(),
            "PROGRAM-ID. P.".to_string(),
            "DATA DIVISION.".to_string(),
            "WORKING-STORAGE SECTION.".to_string(),
            "01  N  PIC 9 VALUE 5.".to_string(),
            "PROCEDURE DIVISION.".to_string(),
            "MAIN.".to_string(),
            "    IF N > 0".to_string(),
        ];
        for _ in 0..5000 {
            lines.push("    AND N > 0".to_string());
        }
        lines.push("    DISPLAY \"OK\".".to_string());
        lines.push("    STOP RUN.".to_string());
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        assert_eq!(run_cobol(&program(&refs)).unwrap(), "OK\n");
    }

    #[test]
    fn if_alphanumeric_comparison_space_pads() {
        // "AB" vs "AB " (space-padded) compare equal.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  W  PIC X(4) VALUE \"AB\".",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    IF W EQUAL \"AB\" DISPLAY \"MATCH\" ELSE DISPLAY \"NO\".",
            "    STOP RUN.",
        ]))
        .unwrap();
        assert_eq!(out, "MATCH\n");
    }

    /// Build a program whose `01 STATUS-CODE PIC 9 VALUE {v}` carries two
    /// level-88 condition-names, then run `body`.
    fn run_level88(v: &str, body: &[&str]) -> Result<String, RuntimeError> {
        let mut lines = vec![
            "IDENTIFICATION DIVISION.".to_string(),
            "PROGRAM-ID. P.".to_string(),
            "DATA DIVISION.".to_string(),
            "WORKING-STORAGE SECTION.".to_string(),
            format!("01  STATUS-CODE  PIC 9 VALUE {v}."),
            "88  IS-OK  VALUE 1.".to_string(),
            "88  IS-DONE  VALUE 9.".to_string(),
            "PROCEDURE DIVISION.".to_string(),
            "MAIN.".to_string(),
        ];
        lines.extend(body.iter().map(|s| format!("    {s}")));
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        run_cobol(&program(&refs))
    }

    #[test]
    fn level_88_condition_name_tests_its_variable() {
        // STATUS-CODE = 1 makes IS-OK true, IS-DONE false.
        assert_eq!(
            run_level88("1", &["IF IS-OK DISPLAY \"OK\" ELSE DISPLAY \"NO\".", "STOP RUN."]).unwrap(),
            "OK\n"
        );
        assert_eq!(
            run_level88("1", &["IF IS-DONE DISPLAY \"DONE\" ELSE DISPLAY \"NO\".", "STOP RUN."]).unwrap(),
            "NO\n"
        );
        // STATUS-CODE = 9 flips them.
        assert_eq!(
            run_level88("9", &["IF IS-DONE DISPLAY \"DONE\" ELSE DISPLAY \"NO\".", "STOP RUN."]).unwrap(),
            "DONE\n"
        );
    }

    #[test]
    fn level_88_condition_name_drives_perform_until() {
        // PERFORM STEP UNTIL IS-DONE — STEP adds 2 to STATUS-CODE (1→3→5→7→9),
        // so it runs while IS-DONE is false and stops once STATUS-CODE reaches 9.
        let out = run_level88(
            "1",
            &[
                "PERFORM STEP UNTIL IS-DONE.",
                "DISPLAY STATUS-CODE.",
                "STOP RUN.",
                "STEP.",
                "ADD 2 TO STATUS-CODE.",
            ],
        )
        .unwrap();
        assert_eq!(out, "9\n");
    }

    #[test]
    fn level_88_on_a_non_numeric_item_is_a_later_rung() {
        // A condition-name whose conditional variable is alphanumeric is deferred
        // — a clean error, never a wrong answer.
        let err = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  FLAG  PIC X VALUE \"Y\".",
            "88  IS-YES  VALUE \"Y\".",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    IF IS-YES DISPLAY \"YES\".",
            "    STOP RUN.",
        ]))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
    }

    /// Build a program with `01 N PIC 99 VALUE {v}` plus a single level-88 line,
    /// then test `IF COND`.
    fn run_level88_cond(v: &str, eighty_eight: &str) -> Result<String, RuntimeError> {
        run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            &format!("01  N  PIC 99 VALUE {v}."),
            eighty_eight,
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    IF COND DISPLAY \"Y\" ELSE DISPLAY \"N\".",
            "    STOP RUN.",
        ]))
    }

    #[test]
    fn level_88_multiple_values_is_an_or() {
        // 88 COND VALUE 1 3 5 — true for any listed value, false otherwise.
        for (v, want) in [("1", "Y\n"), ("3", "Y\n"), ("5", "Y\n"), ("2", "N\n"), ("4", "N\n")] {
            assert_eq!(run_level88_cond(v, "88  COND  VALUE 1 3 5.").unwrap(), want, "N={v}");
        }
    }

    #[test]
    fn level_88_thru_range_is_inclusive() {
        // 88 COND VALUE 3 THRU 6 — true for 3..=6, false just outside (both ends).
        for (v, want) in [("2", "N\n"), ("3", "Y\n"), ("5", "Y\n"), ("6", "Y\n"), ("7", "N\n")] {
            assert_eq!(run_level88_cond(v, "88  COND  VALUE 3 THRU 6.").unwrap(), want, "N={v}");
        }
    }

    #[test]
    fn level_88_mixes_singles_and_ranges() {
        // 88 COND VALUE 1 5 THRU 7 9 — {1} ∪ {5,6,7} ∪ {9}.
        for (v, want) in [("1", "Y\n"), ("5", "Y\n"), ("7", "Y\n"), ("9", "Y\n"), ("4", "N\n"), ("8", "N\n")] {
            assert_eq!(run_level88_cond(v, "88  COND  VALUE 1 5 THRU 7 9.").unwrap(), want, "N={v}");
        }
    }

    #[test]
    fn multi_value_on_a_plain_item_is_rejected() {
        // A multi-value / range VALUE is only meaningful on a level-88 entry; on a
        // plain item it is a clean error.
        let err = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  N  PIC 99 VALUE 1 2 3.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    STOP RUN.",
        ]))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn set_condition_name_to_true_assigns_the_first_value() {
        // SET IS-DONE TO TRUE stores 9 (IS-DONE VALUE 9) into STATUS-CODE, which
        // then displays as 9 and satisfies IS-DONE.
        let out = run_level88(
            "1",
            &[
                "SET IS-DONE TO TRUE.",
                "DISPLAY STATUS-CODE.",
                "IF IS-DONE DISPLAY \"D\".",
                "STOP RUN.",
            ],
        )
        .unwrap();
        assert_eq!(out, "9\nD\n");
    }

    #[test]
    fn set_condition_name_to_true_uses_a_ranges_low_bound() {
        // 88 COND VALUE 3 THRU 6 — SET COND TO TRUE assigns the low bound 3.
        let out = run_level88_cond("0", "88  COND  VALUE 3 THRU 6.").unwrap();
        // (The helper's IF prints N/Y for COND on N=0 — before the SET, N=0 is
        // false.) Re-run with a program that SETs then displays.
        assert_eq!(out, "N\n");
        let set = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  N  PIC 99 VALUE 0.",
            "88  COND  VALUE 3 THRU 6.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    SET COND TO TRUE.",
            "    DISPLAY N.",
            "    STOP RUN.",
        ]))
        .unwrap();
        assert_eq!(set, "03\n");
    }

    #[test]
    fn set_an_undeclared_condition_name_errors() {
        let err = run_level88("1", &["SET NOPE TO TRUE.", "STOP RUN."]).unwrap_err();
        assert!(matches!(err, RuntimeError::UndefinedName(_)), "got {err:?}");
    }

    #[test]
    fn stop_run_inside_a_branch_ends_the_program() {
        // The STOP RUN is inside the THEN branch; the trailing DISPLAY never runs.
        assert_eq!(
            run_if("5", &["IF N GREATER 3 DISPLAY \"IN\" STOP RUN.", "DISPLAY \"AFTER\".", "STOP RUN."]),
            "IN\n"
        );
    }

    #[test]
    fn deeply_nested_if_errors_rather_than_overflowing_the_stack() {
        // A crafted source can nest `IF`s far past anything real COBOL does.
        // Parsing that recurses once per level; without the parser's depth cap
        // it overflows the *native* stack and aborts the process — uncatchable,
        // not a RuntimeError. `cobol-parser` opts into the cap, so end-to-end
        // this comes back as a clean parse error. One `IF` per card so the
        // fixed 80-column format doesn't truncate them (a statement flows
        // freely across cards, so the nest is real). 4096 is far past the cap.
        let mut lines: Vec<String> = vec![
            "IDENTIFICATION DIVISION.".into(),
            "PROGRAM-ID. P.".into(),
            "DATA DIVISION.".into(),
            "WORKING-STORAGE SECTION.".into(),
            "01  N  PIC 9(3) VALUE 5.".into(),
            "PROCEDURE DIVISION.".into(),
            "MAIN.".into(),
        ];
        for _ in 0..4096 {
            lines.push("    IF N GREATER 0".into());
        }
        lines.push("    DISPLAY \"DEEP\".".into());
        lines.push("    STOP RUN.".into());
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        let err = run_cobol(&program(&refs)).unwrap_err();
        // The safety property: the depth cap turns runaway nesting into a clean
        // `RuntimeError::Parse` instead of a native stack overflow. (Since
        // `condition = relation | condition_name`, when the cap trips inside the
        // `relation` alternative the PEG falls back to `condition_name`, so the
        // surfaced message is a generic parse error rather than a depth-specific
        // one — but it is still a bounded, catchable parse failure, which is the
        // guarantee that matters.)
        assert!(matches!(err, RuntimeError::Parse(_)), "got {err:?}");
    }

    // ----------------------------------------------------------------------
    // COMPUTE — expression evaluation, ROUNDED, ON SIZE ERROR
    // ----------------------------------------------------------------------

    /// Build a program with three numeric inputs and one receiver `R`, run the
    /// given procedure body, and return what it DISPLAYed. `A`, `B`, `C` are
    /// `9(3)`; `R` is whatever `r_pic` says.
    fn run_compute(a: &str, b: &str, c: &str, r_pic: &str, body: &[&str]) -> Result<String, RuntimeError> {
        let mut lines = vec![
            "IDENTIFICATION DIVISION.".to_string(),
            "PROGRAM-ID. P.".to_string(),
            "DATA DIVISION.".to_string(),
            "WORKING-STORAGE SECTION.".to_string(),
            format!("01  A  PIC 9(3) VALUE {a}."),
            format!("01  B  PIC 9(3) VALUE {b}."),
            format!("01  C  PIC 9(3) VALUE {c}."),
            format!("01  R  PIC {r_pic} VALUE 0."),
            "PROCEDURE DIVISION.".to_string(),
            "MAIN.".to_string(),
        ];
        lines.extend(body.iter().map(|s| format!("    {s}")));
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        run_cobol(&program(&refs))
    }

    #[test]
    fn compute_respects_operator_precedence() {
        // A + B * C = 10 + 3*2 = 16 → stored in 9(4)V99 → "001600".
        let out = run_compute("10", "3", "2", "9(4)V99",
            &["COMPUTE R = A + B * C.", "DISPLAY R.", "STOP RUN."]).unwrap();
        assert_eq!(out, "001600\n");
    }

    #[test]
    fn compute_parentheses_override_precedence() {
        // (A + B) * C = (10 + 3) * 2 = 26 → "002600".
        let out = run_compute("10", "3", "2", "9(4)V99",
            &["COMPUTE R = (A + B) * C.", "DISPLAY R.", "STOP RUN."]).unwrap();
        assert_eq!(out, "002600\n");
    }

    #[test]
    fn compute_exponentiation_and_unary_minus() {
        // C ** B = 2 ** 3 = 8 → "000800".
        let out = run_compute("10", "3", "2", "9(4)V99",
            &["COMPUTE R = C ** B.", "DISPLAY R.", "STOP RUN."]).unwrap();
        assert_eq!(out, "000800\n");
        // Unary minus then stored into an unsigned receiver keeps the magnitude:
        // A - (A + B) = 10 - 13 = -3 → magnitude 3 → "000300".
        let out = run_compute("10", "3", "2", "9(4)V99",
            &["COMPUTE R = A - (A + B).", "DISPLAY R.", "STOP RUN."]).unwrap();
        assert_eq!(out, "000300\n");
    }

    #[test]
    fn compute_division_truncates_then_rounds() {
        // A / B = 10 / 3 = 3.333… → truncated into V99 → "0003.33" = "000333".
        let out = run_compute("10", "3", "2", "9(4)V99",
            &["COMPUTE R = A / B.", "DISPLAY R.", "STOP RUN."]).unwrap();
        assert_eq!(out, "000333\n");
        // With ROUNDED to two places, 3.333… → 3.33 (the third place is < 5).
        let out = run_compute("20", "3", "2", "9(4)V99",
            &["COMPUTE R ROUNDED = A / B.", "DISPLAY R.", "STOP RUN."]).unwrap();
        // 20 / 3 = 6.666… → rounds to 6.67 → "000667".
        assert_eq!(out, "000667\n");
    }

    #[test]
    fn compute_on_size_error_fires_on_overflow() {
        // A * A * A = 10^3 = 1000, but R is only 9(2) (max 99): integer overflow
        // → the ON SIZE ERROR handler runs and R is left unchanged (still 0).
        let out = run_compute("10", "3", "2", "9(2)", &[
            "COMPUTE R = A * A * A",
            "    ON SIZE ERROR DISPLAY \"TOO BIG\".",
            "DISPLAY R.",
            "STOP RUN.",
        ]).unwrap();
        assert_eq!(out, "TOO BIG\n00\n");
    }

    #[test]
    fn compute_overflow_without_handler_truncates() {
        // Same overflow but no handler: COBOL truncates high-order digits
        // silently (1000 into 9(2) keeps the low "00").
        let out = run_compute("10", "3", "2", "9(2)",
            &["COMPUTE R = A * A * A.", "DISPLAY R.", "STOP RUN."]).unwrap();
        assert_eq!(out, "00\n");
    }

    #[test]
    fn compute_on_size_error_catches_divide_by_zero() {
        // Dividing by (C - C) = 0 is a size-error condition; the handler runs.
        let out = run_compute("10", "3", "2", "9(4)V99", &[
            "COMPUTE R = A / (C - C)",
            "    ON SIZE ERROR DISPLAY \"DIV ZERO\".",
            "DISPLAY R.",
            "STOP RUN.",
        ]).unwrap();
        assert_eq!(out, "DIV ZERO\n000000\n");
    }

    #[test]
    fn compute_divide_by_zero_without_handler_is_an_error() {
        let err = run_compute("10", "3", "2", "9(4)V99",
            &["COMPUTE R = A / (C - C).", "DISPLAY R.", "STOP RUN."]).unwrap_err();
        assert!(matches!(err, RuntimeError::DivideByZero), "got {err:?}");
    }

    #[test]
    fn compute_huge_flat_operator_chain_errors_not_overflows() {
        // A flat `A + A + A + …` chain uses grammar repetition, so the parser's
        // recursion-depth cap does not bound its *width*. Folding it builds a
        // tree that deep, which would overflow the native stack in eval (and in
        // the recursive Drop). The operand budget turns it into a clean error
        // instead. 5000 terms is far past MAX_EXPR_OPERANDS (1024). Split across
        // cards so the 80-column format doesn't truncate the expression.
        let mut lines = vec![
            "IDENTIFICATION DIVISION.".to_string(),
            "PROGRAM-ID. P.".to_string(),
            "DATA DIVISION.".to_string(),
            "WORKING-STORAGE SECTION.".to_string(),
            "01  A  PIC 9(3) VALUE 1.".to_string(),
            "01  R  PIC 9(9) VALUE 0.".to_string(),
            "PROCEDURE DIVISION.".to_string(),
            "MAIN.".to_string(),
            "    COMPUTE R = A".to_string(),
        ];
        for _ in 0..5000 {
            lines.push("        + A".to_string());
        }
        lines.push("        .".to_string());
        lines.push("    STOP RUN.".to_string());
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        let err = run_cobol(&program(&refs)).unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
        assert!(err.to_string().contains("too large"), "message should explain: {err}");
    }

    // ----------------------------------------------------------------------
    // Honest failure: unmodelled features error, they do not run wrong.
    // ----------------------------------------------------------------------

    #[test]
    fn unsupported_verb_is_a_clear_error() {
        // ACCEPT is parsed but not yet executed (console input is a later PR).
        let err = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  N  PIC 9(3).",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    ACCEPT N.",
            "    STOP RUN.",
        ]))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
        assert!(err.to_string().contains("ACCEPT"), "message should name the verb: {err}");
    }

    // ----------------------------------------------------------------------
    // GO TO — unconditional paragraph transfer, and GO TO loops
    // ----------------------------------------------------------------------

    #[test]
    fn go_to_transfers_control_and_skips_fallthrough() {
        // GO TO SKIP jumps past MIDDLE; "MIDDLE" is never displayed.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    DISPLAY \"START\".",
            "    GO TO SKIP.",
            "MIDDLE.",
            "    DISPLAY \"MIDDLE\".",
            "SKIP.",
            "    DISPLAY \"END\".",
            "    STOP RUN.",
        ]))
        .unwrap();
        assert_eq!(out, "START\nEND\n");
    }

    #[test]
    fn go_to_forms_a_loop_that_terminates() {
        // A back-edge GO TO drives a counting loop (iterative — no stack growth).
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  I  PIC 9 VALUE 0.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    MOVE 0 TO I.",
            "LOOP.",
            "    ADD 1 TO I.",
            "    DISPLAY I.",
            "    IF I LESS 3 GO TO LOOP.",
            "    STOP RUN.",
        ]))
        .unwrap();
        assert_eq!(out, "1\n2\n3\n");
    }

    #[test]
    fn go_to_out_of_a_performed_paragraph_transfers_at_top_level() {
        // A GO TO inside a performed paragraph transfers control at the top
        // level, abandoning the PERFORM's return: MAIN's DISPLAY after the
        // PERFORM never runs.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    PERFORM SUB.",
            "    DISPLAY \"AFTER MAIN\".",
            "    STOP RUN.",
            "SUB.",
            "    GO TO ELSEWHERE.",
            "ELSEWHERE.",
            "    DISPLAY \"ELSEWHERE\".",
            "    STOP RUN.",
        ]))
        .unwrap();
        assert_eq!(out, "ELSEWHERE\n");
    }

    #[test]
    fn go_to_unknown_paragraph_is_an_error() {
        let err = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    GO TO NOWHERE.",
        ]))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::UndefinedName(_)), "got {err:?}");
    }

    // ----------------------------------------------------------------------
    // PERFORM — out-of-line paragraph invocation
    // ----------------------------------------------------------------------

    #[test]
    fn perform_runs_a_paragraph_then_returns() {
        // MAIN performs GREET, then continues to its own DISPLAY.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    PERFORM GREET.",
            "    DISPLAY \"BACK\".",
            "    STOP RUN.",
            "GREET.",
            "    DISPLAY \"HI\".",
        ]))
        .unwrap();
        // HI (from the perform), BACK (after it returns), then STOP RUN — the
        // GREET paragraph does NOT run a second time by fall-through.
        assert_eq!(out, "HI\nBACK\n");
    }

    #[test]
    fn perform_n_times_repeats() {
        // PERFORM TICK 3 TIMES accumulates COUNT 1,2,3.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  COUNT  PIC 9 VALUE 0.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    PERFORM TICK 3 TIMES.",
            "    STOP RUN.",
            "TICK.",
            "    ADD 1 TO COUNT.",
            "    DISPLAY COUNT.",
        ]))
        .unwrap();
        assert_eq!(out, "1\n2\n3\n");
    }

    #[test]
    fn perform_zero_or_negative_times_runs_never() {
        // A zero count performs the paragraph zero times.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  N  PIC 9 VALUE 0.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    PERFORM NOISE N TIMES.",
            "    DISPLAY \"DONE\".",
            "    STOP RUN.",
            "NOISE.",
            "    DISPLAY \"X\".",
        ]))
        .unwrap();
        assert_eq!(out, "DONE\n");
    }

    #[test]
    fn perform_of_unknown_paragraph_is_an_error() {
        let err = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    PERFORM NOWHERE.",
            "    STOP RUN.",
        ]))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::UndefinedName(_)), "got {err:?}");
    }

    #[test]
    fn self_performing_paragraph_errors_not_overflows() {
        // LOOP performs itself: without the depth cap this recurses until the
        // native stack overflows. The cap turns it into a clean error.
        let err = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    PERFORM LOOP.",
            "    STOP RUN.",
            "LOOP.",
            "    PERFORM LOOP.",
        ]))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
        assert!(err.to_string().to_lowercase().contains("deep"), "message should explain: {err}");
    }

    #[test]
    fn stop_run_inside_a_performed_paragraph_ends_the_program() {
        // The STOP RUN inside DONE ends everything; MAIN's trailing DISPLAY
        // never runs.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    PERFORM DONE.",
            "    DISPLAY \"AFTER\".",
            "    STOP RUN.",
            "DONE.",
            "    DISPLAY \"IN\".",
            "    STOP RUN.",
        ]))
        .unwrap();
        assert_eq!(out, "IN\n");
    }

    #[test]
    fn perform_until_loops_while_condition_is_false() {
        // Count I from 1 to 3: PERFORM STEP UNTIL I GREATER 2 runs STEP while
        // I is not > 2. STEP adds 1 and displays, so I goes 1, 2, 3 and stops
        // once I = 3 (which is > 2, tested before the would-be 4th run).
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  I  PIC 9 VALUE 0.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    PERFORM STEP UNTIL I GREATER 2.",
            "    DISPLAY \"DONE\".",
            "    STOP RUN.",
            "STEP.",
            "    ADD 1 TO I.",
            "    DISPLAY I.",
        ]))
        .unwrap();
        // STEP runs at I=0→1, I=1→2, I=2→3; next test I=3>2 true → stop.
        assert_eq!(out, "1\n2\n3\nDONE\n");
    }

    #[test]
    fn perform_until_tests_before_so_can_run_zero_times() {
        // The condition is already true, so the paragraph never runs.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  I  PIC 9 VALUE 5.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    PERFORM NOISE UNTIL I GREATER 2.",
            "    DISPLAY \"DONE\".",
            "    STOP RUN.",
            "NOISE.",
            "    DISPLAY \"X\".",
        ]))
        .unwrap();
        assert_eq!(out, "DONE\n");
    }

    #[test]
    fn perform_until_propagates_stop_run() {
        // A STOP RUN inside the UNTIL body ends the program immediately.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  I  PIC 9 VALUE 0.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    PERFORM STEP UNTIL I GREATER 9.",
            "    DISPLAY \"NEVER\".",
            "    STOP RUN.",
            "STEP.",
            "    DISPLAY \"ONCE\".",
            "    STOP RUN.",
        ]))
        .unwrap();
        assert_eq!(out, "ONCE\n");
    }

    #[test]
    fn perform_thru_runs_a_paragraph_range() {
        // PERFORM A THRU C runs A, B, C in order, then returns; MAIN's own
        // DISPLAY runs after. The paragraphs do NOT run again by fall-through
        // because MAIN stops.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    PERFORM A THRU C.",
            "    DISPLAY \"BACK\".",
            "    STOP RUN.",
            "A.",
            "    DISPLAY \"A\".",
            "B.",
            "    DISPLAY \"B\".",
            "C.",
            "    DISPLAY \"C\".",
        ]))
        .unwrap();
        assert_eq!(out, "A\nB\nC\nBACK\n");
    }

    #[test]
    fn perform_thru_with_times_repeats_the_whole_range() {
        // PERFORM A THRU B 2 TIMES runs (A, B) twice → A B A B.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    PERFORM A THRU B 2 TIMES.",
            "    STOP RUN.",
            "A.",
            "    DISPLAY \"A\".",
            "B.",
            "    DISPLAY \"B\".",
        ]))
        .unwrap();
        assert_eq!(out, "A\nB\nA\nB\n");
    }

    #[test]
    fn perform_thru_backwards_range_is_an_error() {
        // THRU target must not precede the start paragraph.
        let err = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    PERFORM C THRU A.",
            "    STOP RUN.",
            "A.",
            "    DISPLAY \"A\".",
            "C.",
            "    DISPLAY \"C\".",
        ]))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
        assert!(err.to_string().contains("backwards"), "message should explain: {err}");
    }

    #[test]
    fn perform_varying_counts_with_an_induction_variable() {
        // VARYING I FROM 1 BY 1 UNTIL I GREATER 3 runs the body for I = 1, 2, 3.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  I  PIC 9 VALUE 0.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    PERFORM SHOW VARYING I FROM 1 BY 1 UNTIL I GREATER 3.",
            "    DISPLAY \"DONE\".",
            "    STOP RUN.",
            "SHOW.",
            "    DISPLAY I.",
        ]))
        .unwrap();
        assert_eq!(out, "1\n2\n3\nDONE\n");
    }

    #[test]
    fn perform_varying_can_step_by_more_than_one() {
        // FROM 0 BY 2 UNTIL I GREATER 6 → I = 0, 2, 4, 6.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  I  PIC 9 VALUE 0.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    PERFORM SHOW VARYING I FROM 0 BY 2 UNTIL I GREATER 6.",
            "    STOP RUN.",
            "SHOW.",
            "    DISPLAY I.",
        ]))
        .unwrap();
        assert_eq!(out, "0\n2\n4\n6\n");
    }

    #[test]
    fn perform_varying_tests_before_so_can_run_zero_times() {
        // The condition is already true at the FROM value, so the body never runs.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  I  PIC 9 VALUE 0.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    PERFORM SHOW VARYING I FROM 9 BY 1 UNTIL I GREATER 3.",
            "    DISPLAY \"DONE\".",
            "    STOP RUN.",
            "SHOW.",
            "    DISPLAY I.",
        ]))
        .unwrap();
        assert_eq!(out, "DONE\n");
    }

    #[test]
    fn hostile_picture_size_errors_rather_than_exhausting_memory() {
        // A ~30-byte program that would allocate gigabytes without the bound.
        let err = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  N  PIC X(4000000000).",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    STOP RUN.",
        ]))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::UnsupportedPicture(_)), "got {err:?}");
    }

    #[test]
    fn invalid_level_number_is_rejected() {
        // Level 88 (condition-names) is a deferred feature; level 50 is invalid.
        for lvl in ["88", "50"] {
            let err = run_cobol(&program(&[
                "IDENTIFICATION DIVISION.",
                "PROGRAM-ID. P.",
                "DATA DIVISION.",
                "WORKING-STORAGE SECTION.",
                "01  REC  PIC X(3).",
                &format!("{lvl}  SUB  PIC X(2)."),
                "PROCEDURE DIVISION.",
                "MAIN.",
                "    STOP RUN.",
            ]))
            .unwrap_err();
            assert!(matches!(err, RuntimeError::Unsupported(_)), "level {lvl}: got {err:?}");
        }
    }

    // ----------------------------------------------------------------------
    // Signed numerics (PIC S9…) — sign carried through, overpunch on DISPLAY
    // ----------------------------------------------------------------------

    /// Run a program with a signed receiver `N PIC <pic>` initialised to
    /// `value`, then the body, returning what it DISPLAYed.
    fn run_signed(pic: &str, value: &str, body: &[&str]) -> String {
        let mut lines = vec![
            "IDENTIFICATION DIVISION.".to_string(),
            "PROGRAM-ID. P.".to_string(),
            "DATA DIVISION.".to_string(),
            "WORKING-STORAGE SECTION.".to_string(),
            format!("01  N  PIC {pic} VALUE {value}."),
            "PROCEDURE DIVISION.".to_string(),
            "MAIN.".to_string(),
        ];
        lines.extend(body.iter().map(|s| format!("    {s}")));
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        run_cobol(&program(&refs)).unwrap()
    }

    #[test]
    fn signed_value_displays_with_trailing_overpunch() {
        // -123 in S9(3): magnitude "123", units 3 → 'L' (negative). → "12L".
        assert_eq!(run_signed("S9(3)", "-123", &["DISPLAY N.", "STOP RUN."]), "12L\n");
        // +123 → units 3 → 'C' (positive). → "12C".
        assert_eq!(run_signed("S9(3)", "123", &["DISPLAY N.", "STOP RUN."]), "12C\n");
        // Zero is unsigned: units 0 → '{' (positive). → "00{".
        assert_eq!(run_signed("S9(3)", "0", &["DISPLAY N.", "STOP RUN."]), "00{\n");
    }

    #[test]
    fn signed_field_keeps_sign_through_arithmetic() {
        // 3 - 5 = -2 into a signed receiver → magnitude 2, negative → "0K"
        // (units 2 → 'K'). An unsigned receiver would show "02".
        assert_eq!(
            run_signed("S9(2)", "3", &["SUBTRACT 5 FROM N.", "DISPLAY N.", "STOP RUN."]),
            "0K\n"
        );
    }

    #[test]
    fn signed_value_used_in_arithmetic_carries_its_sign() {
        // N = -10; ADD 4 → -6 → "0O" (units 6 → 'O', negative).
        assert_eq!(
            run_signed("S9(2)", "-10", &["ADD 4 TO N.", "DISPLAY N.", "STOP RUN."]),
            "0O\n"
        );
    }

    #[test]
    fn moving_signed_into_unsigned_drops_the_sign() {
        // A signed source moved into an unsigned receiver keeps only magnitude.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  S  PIC S9(3) VALUE -45.",
            "01  U  PIC 9(3) VALUE 0.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    MOVE S TO U.",
            "    DISPLAY U.",
            "    STOP RUN.",
        ]))
        .unwrap();
        assert_eq!(out, "045\n");
    }

    #[test]
    fn compute_into_signed_receiver_shows_negative_overpunch() {
        // COMPUTE N = 2 - 9 = -7 into S9(2) → "0P" (units 7 → 'P', negative).
        assert_eq!(
            run_signed("S9(2)", "0", &["COMPUTE N = 2 - 9.", "DISPLAY N.", "STOP RUN."]),
            "0P\n"
        );
    }

    /// Wrap DATA and PROCEDURE lines into a minimal well-formed program.
    fn wrap(data: &[&str], proc: &[&str]) -> String {
        let mut lines = vec![
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
        ];
        lines.extend_from_slice(data);
        lines.push("PROCEDURE DIVISION.");
        lines.push("MAIN.");
        lines.extend_from_slice(proc);
        program(&lines)
    }

    #[test]
    fn string_concatenates_delimited_by_size() {
        // "ABC" ++ "DE" = "ABCDE", left-justified into a 10-wide field; the
        // untouched tail stays as its original spaces.
        let out = run_cobol(&wrap(
            &[
                "01  A  PIC X(3) VALUE \"ABC\".",
                "01  B  PIC X(2) VALUE \"DE\".",
                "01  T  PIC X(10) VALUE SPACES.",
            ],
            &["STRING A B DELIMITED BY SIZE INTO T.", "DISPLAY T.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(out, "ABCDE     \n");
    }

    #[test]
    fn string_truncates_and_preserves_untouched_tail() {
        // Wider-than-receiver concatenation truncates …
        let trunc = run_cobol(&wrap(
            &[
                "01  A  PIC X(3) VALUE \"ABC\".",
                "01  B  PIC X(2) VALUE \"DE\".",
                "01  T  PIC X(4) VALUE SPACES.",
            ],
            &["STRING A B DELIMITED BY SIZE INTO T.", "DISPLAY T.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(trunc, "ABCD\n");
        // … and a short write leaves the receiver's prior (non-space) tail intact.
        let nofill = run_cobol(&wrap(
            &["01  A  PIC X(2) VALUE \"AB\".", "01  T  PIC X(6) VALUE \"ZZZZZZ\"."],
            &["STRING A DELIMITED BY SIZE INTO T.", "DISPLAY T.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(nofill, "ABZZZZ\n");
    }

    #[test]
    fn string_later_rung_options_are_clean_errors() {
        // A real delimiter needs a scan (later rung) …
        let delim = run_cobol(&wrap(
            &["01  A  PIC X(3) VALUE \"ABC\".", "01  T  PIC X(6) VALUE SPACES."],
            &["STRING A DELIMITED BY \"-\" INTO T.", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(delim, RuntimeError::Unsupported(_)), "got {delim:?}");
        // … and so is WITH POINTER.
        let ptr = run_cobol(&wrap(
            &[
                "01  A  PIC X(3) VALUE \"ABC\".",
                "01  T  PIC X(6) VALUE SPACES.",
                "01  P  PIC 9(2) VALUE 1.",
            ],
            &["STRING A DELIMITED BY SIZE INTO T WITH POINTER P.", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(ptr, RuntimeError::Unsupported(_)), "got {ptr:?}");
    }
}
