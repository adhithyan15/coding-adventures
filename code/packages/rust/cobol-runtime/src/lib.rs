//! # COBOL runtime — running COBOL, where the quirks live.
//!
//! A tree-walking interpreter for COBOL-60, built on the `cobol-parser` CST. It
//! turns WORKING-STORAGE into a **PICTURE-typed data model** and executes the
//! PROCEDURE DIVISION, capturing everything `DISPLAY`ed. See
//! [PL08](../../../specs/PL08-cobol-runtime.md).
//!
//! This is v0.1 — the execution spine. It implements a *small but fully
//! correct* slice (`MOVE` / `DISPLAY` / `STOP RUN` over unsigned numeric-display
//! and character pictures) and returns a descriptive error for anything not yet
//! modelled, rather than producing wrong output. The roadmap toward full COBOL
//! (arithmetic, editing pictures, `PERFORM`, tables, files, later standards) is
//! in PL08.
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
    // Honest failure: unmodelled features error, they do not run wrong.
    // ----------------------------------------------------------------------

    #[test]
    fn unsupported_verb_is_a_clear_error() {
        let err = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  N  PIC 9(3).",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    ADD 1 TO N.",
            "    STOP RUN.",
        ]))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
        assert!(err.to_string().contains("ADD"), "message should name the verb: {err}");
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
