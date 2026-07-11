//! # COBOL runtime — running COBOL, where the quirks live.
//!
//! A tree-walking interpreter for COBOL-60, built on the `cobol-parser` CST. It
//! turns WORKING-STORAGE into a **PICTURE-typed data model** and executes the
//! PROCEDURE DIVISION, capturing everything `DISPLAY`ed. See
//! [PL08](../../../specs/PL08-cobol-runtime.md).
//!
//! It implements a *small but fully correct* slice — `MOVE` / `DISPLAY` /
//! `STOP RUN`, fixed-point decimal `ADD` / `SUBTRACT` / `MULTIPLY` / `DIVIDE`,
//! and `IF … ELSE` (numeric and alphanumeric comparison) over unsigned
//! numeric-display and character pictures — and returns a descriptive error for
//! anything not yet modelled, rather than producing wrong output. The roadmap
//! toward full COBOL (signed numerics, `COMPUTE`, `ROUNDED`/`ON SIZE ERROR`,
//! editing pictures, `PERFORM`, tables, files, later standards) is in PL08.
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
    fn if_then_branch_runs_multiple_statements() {
        // THEN branch has two statements; both run when the condition holds.
        assert_eq!(
            run_if("5", &["IF N GREATER 3 MOVE 8 TO N DISPLAY N.", "STOP RUN."]),
            "008\n"
        );
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

    #[test]
    fn stop_run_inside_a_branch_ends_the_program() {
        // The STOP RUN is inside the THEN branch; the trailing DISPLAY never runs.
        assert_eq!(
            run_if("5", &["IF N GREATER 3 DISPLAY \"IN\" STOP RUN.", "DISPLAY \"AFTER\".", "STOP RUN."]),
            "IN\n"
        );
    }

    // ----------------------------------------------------------------------
    // Honest failure: unmodelled features error, they do not run wrong.
    // ----------------------------------------------------------------------

    #[test]
    fn unsupported_verb_is_a_clear_error() {
        // PERFORM is not yet executed (control flow is a later PR).
        let err = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    PERFORM SUB.",
            "    STOP RUN.",
            "SUB.",
            "    STOP RUN.",
        ]))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
        assert!(err.to_string().contains("PERFORM"), "message should name the verb: {err}");
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

    #[test]
    fn signed_picture_is_unsupported_not_wrong() {
        let err = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  N  PIC S9(3).",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    STOP RUN.",
        ]))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::UnsupportedPicture(_)), "got {err:?}");
    }
}
