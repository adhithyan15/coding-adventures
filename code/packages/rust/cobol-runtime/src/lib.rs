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
    // Cross-category MOVE: unsigned-integer numeric → alphanumeric
    // ----------------------------------------------------------------------

    /// Run a program with the given WORKING-STORAGE and PROCEDURE bodies.
    fn run_ws(ws: &[&str], body: &[&str]) -> Result<String, RuntimeError> {
        let mut lines = vec![
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
        ];
        lines.extend_from_slice(ws);
        lines.push("PROCEDURE DIVISION.");
        lines.push("MAIN.");
        lines.extend_from_slice(body);
        run_cobol(&program(&lines))
    }

    #[test]
    fn numeric_to_alphanumeric_move_left_justifies_and_space_pads() {
        // PIC 9(3)=042 → PIC X(5): the digit image "042" left-justified, right-padded.
        let out = run_ws(
            &["01  N  PIC 9(3) VALUE 42.", "01  W  PIC X(5)."],
            &["    MOVE N TO W.", "    DISPLAY W \"|\".", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(out, "042  |\n");
    }

    #[test]
    fn numeric_to_alphanumeric_move_truncates_on_the_right() {
        // PIC 9(3)=042 → PIC X(2): keeps the leftmost two digits "04".
        let out = run_ws(
            &["01  N  PIC 9(3) VALUE 42.", "01  W  PIC X(2)."],
            &["    MOVE N TO W.", "    DISPLAY W.", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(out, "04\n");
    }

    #[test]
    fn numeric_to_alphanumeric_move_exact_fit() {
        // PIC 9(3)=042 → PIC X(3): the whole digit image "042".
        let out = run_ws(
            &["01  N  PIC 9(3) VALUE 42.", "01  W  PIC X(3)."],
            &["    MOVE N TO W.", "    DISPLAY W.", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(out, "042\n");
    }

    #[test]
    fn signed_numeric_to_alphanumeric_move_overpunches_units_digit() {
        // A SIGNED integer source: its magnitude image carries the sign as a trailing
        // overpunch on the units digit. +123 → units 3, positive → 'C' → "12C"; the
        // X(4) receiver left-justifies and space-pads → "12C ".
        let pos = run_ws(
            &["01  S  PIC S9(3) VALUE 123.", "01  W  PIC X(4)."],
            &["    MOVE S TO W.", "    DISPLAY W \"|\".", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(pos, "12C |\n");
        // −123 → units 3, negative → 'L' → "12L"; exact fit into X(3).
        let neg = run_ws(
            &["01  S  PIC S9(3) VALUE -123.", "01  W  PIC X(3)."],
            &["    MOVE S TO W.", "    DISPLAY W.", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(neg, "12L\n");
    }

    #[test]
    fn signed_numeric_to_alphanumeric_move_units_zero_and_truncation() {
        // Units digit 0 selects the '{' (positive) / '}' (negative) overpunch.
        let neg = run_ws(
            &["01  S  PIC S9(3) VALUE -120.", "01  W  PIC X(3)."],
            &["    MOVE S TO W.", "    DISPLAY W.", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(neg, "12}\n");
        // A NARROWER receiver right-truncates the image: S9(3)=-123 → "12L" → X(2) "12".
        let trunc = run_ws(
            &["01  S  PIC S9(3) VALUE -123.", "01  W  PIC X(2)."],
            &["    MOVE S TO W.", "    DISPLAY W.", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(trunc, "12\n");
    }

    #[test]
    fn signed_scaled_numeric_to_alphanumeric_move_overpunches_last_fraction_digit() {
        // A SIGNED SCALED source `PIC S9V9 = -4.2` → magnitude image "42", overpunch
        // the units (last fractional) digit 2, negative → 'K' → "4K" (exact fit X(2)).
        let out = run_ws(
            &["01  F  PIC S9V9 VALUE -4.2.", "01  W  PIC X(2)."],
            &["    MOVE F TO W.", "    DISPLAY W.", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(out, "4K\n");
    }

    #[test]
    fn signed_value_truncating_to_zero_magnitude_is_positive() {
        // COBOL has no negative zero: a nonzero negative value that high-order-
        // truncates to an all-zero slot (`-1000` into `PIC S9(3)` → `000`) stores
        // POSITIVE. Its alphanumeric image therefore takes the positive units-0
        // overpunch '{' → "00{", NOT the negative '}'. DISPLAY of the signed field
        // agrees. This matches the compiler, whose single-i64 slot collapses the
        // value to a plain 0 (regression for a cross-engine sign-of-zero divergence).
        let out = run_ws(
            &["01  A  PIC S9(4) VALUE -1000.", "01  S  PIC S9(3).", "01  W  PIC X(3)."],
            &[
                "    MOVE A TO S.",
                "    MOVE S TO W.",
                "    DISPLAY W.",
                "    DISPLAY S.",
                "    STOP RUN.",
            ],
        )
        .unwrap();
        assert_eq!(out, "00{\n00{\n");
    }

    #[test]
    fn signed_numeric_to_group_receiver_move_is_deferred() {
        // A GROUP receiver is still a later rung on both engines — the oracle rejects
        // it as "MOVE into a group item"; the compiler models no group items.
        let err = run_ws(
            &[
                "01  S  PIC S9(3) VALUE -12.",
                "01  G.",
                "    05  A  PIC X(2).",
                "    05  B  PIC X(1).",
            ],
            &["    MOVE S TO G.", "    STOP RUN."],
        )
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn scaled_numeric_to_alphanumeric_move_uses_digit_image() {
        // An UNSIGNED SCALED source `PIC 9(2)V9 = 4.2` moves its (int + frac) digit
        // image "042" — no decimal point — into the alphanumeric receiver, left-
        // justified and space-padded (X(4) → "042 ").
        let out = run_ws(
            &["01  F  PIC 9(2)V9 VALUE 4.2.", "01  W  PIC X(4)."],
            &["    MOVE F TO W.", "    DISPLAY W.", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(out, "042 \n");
    }

    #[test]
    fn scaled_numeric_to_alphanumeric_move_more_fraction_digits() {
        // `PIC 9(1)V99 = 3.14` → image "314"; exact fit into X(3).
        let out = run_ws(
            &["01  F  PIC 9(1)V99 VALUE 3.14.", "01  W  PIC X(3)."],
            &["    MOVE F TO W.", "    DISPLAY W.", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(out, "314\n");
    }

    // Cross-category alphanumeric → numeric MOVE (the reverse direction): an
    // alphanumeric source (`PIC X(m)`) read as an unsigned integer and de-scaled
    // into an UNSIGNED INTEGER receiver — RIGHT-justified, keeping the low-order
    // `n` digits (`receiver = (integer from the m source chars) mod 10^n`).

    #[test]
    fn alphanumeric_to_numeric_move_exact_fit() {
        // PIC X(3)="042" → PIC 9(3): fold → 42; DISPLAY shows "042".
        let out = run_ws(
            &["01  A  PIC X(3) VALUE \"042\".", "01  N  PIC 9(3)."],
            &["    MOVE A TO N.", "    DISPLAY N.", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(out, "042\n");
    }

    #[test]
    fn alphanumeric_to_numeric_move_shorter_source_zero_pads() {
        // PIC X(2)="05" → PIC 9(4): fold → 5, right-justified into 4 digits.
        let out = run_ws(
            &["01  A  PIC X(2) VALUE \"05\".", "01  N  PIC 9(4)."],
            &["    MOVE A TO N.", "    DISPLAY N.", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(out, "0005\n");
    }

    #[test]
    fn alphanumeric_to_numeric_move_longer_source_truncates_high_order() {
        // PIC X(5)="12345" → PIC 9(3): fold → 12345, keep the low-order 3 → 345.
        let out = run_ws(
            &["01  A  PIC X(5) VALUE \"12345\".", "01  N  PIC 9(3)."],
            &["    MOVE A TO N.", "    DISPLAY N.", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(out, "345\n");
    }

    #[test]
    fn alphanumeric_to_signed_numeric_move_is_supported_and_positive() {
        // A SIGNED receiver (`PIC S9`) is now supported: an alphanumeric source has
        // NO operational sign, so the fold's MAGNITUDE is stored POSITIVE. PIC
        // X(3)="042" → PIC S9(3): magnitude 042; DISPLAY overpunches the units digit
        // on the POSITIVE row (2 → 'B') → "04B".
        let out = run_ws(
            &["01  A  PIC X(3) VALUE \"042\".", "01  N  PIC S9(3)."],
            &["    MOVE A TO N.", "    DISPLAY N.", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(out, "04B\n");
    }

    // A SCALED receiver `PIC 9(i)V9(d)` is now supported: the fold IS the scaled
    // slot magnitude (`V mod 10^(i+d)`), the point sitting `d` places from the
    // right. DISPLAY shows the raw `(i + d)` digits (no point).

    #[test]
    fn alphanumeric_to_scaled_numeric_move_exact_fit() {
        // PIC X(3)="042" → PIC 9(2)V9: fold → 42, slot 042, reads 4.2 → DISPLAY "042".
        let out = run_ws(
            &["01  A  PIC X(3) VALUE \"042\".", "01  N  PIC 9(2)V9."],
            &["    MOVE A TO N.", "    DISPLAY N.", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(out, "042\n");
    }

    #[test]
    fn alphanumeric_to_scaled_numeric_move_shorter_source_zero_pads() {
        // PIC X(2)="42" → PIC 9(2)V9: fold → 42, slot 042 (left-zero-padded to the
        // 3 positions), reads 4.2 → DISPLAY "042".
        let out = run_ws(
            &["01  A  PIC X(2) VALUE \"42\".", "01  N  PIC 9(2)V9."],
            &["    MOVE A TO N.", "    DISPLAY N.", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(out, "042\n");
    }

    #[test]
    fn alphanumeric_to_scaled_numeric_move_longer_source_truncates_high_order() {
        // PIC X(5)="12345" → PIC 9(2)V9: fold → 12345, keep the low-order (i+d)=3
        // digits → slot 345, reads 34.5 → DISPLAY "345".
        let out = run_ws(
            &["01  A  PIC X(5) VALUE \"12345\".", "01  N  PIC 9(2)V9."],
            &["    MOVE A TO N.", "    DISPLAY N.", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(out, "345\n");
    }

    #[test]
    fn alphanumeric_to_scaled_numeric_move_more_fraction_than_source_digits() {
        // PIC X(1)="5" → PIC 9(1)V99: fold → 5, slot 005 (magnitude has fewer digits
        // than i+d=3), reads 0.05 → DISPLAY "005".
        let out = run_ws(
            &["01  A  PIC X(1) VALUE \"5\".", "01  N  PIC 9(1)V99."],
            &["    MOVE A TO N.", "    DISPLAY N.", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(out, "005\n");
    }

    #[test]
    fn alphanumeric_to_signed_scaled_numeric_move_is_supported_and_positive() {
        // A SIGNED SCALED receiver (`PIC S9V9`) is now supported too: the fold's
        // magnitude IS the scaled slot at scale `d`, stored POSITIVE (source has no
        // sign). PIC X(3)="042" → PIC S9V9 (i=1,d=1): fold 42, slot 42; DISPLAY shows
        // the raw 2 digits with the units overpunched positive (2 → 'B') → "4B".
        let out = run_ws(
            &["01  A  PIC X(3) VALUE \"042\".", "01  N  PIC S9V9."],
            &["    MOVE A TO N.", "    DISPLAY N.", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(out, "4B\n");
    }

    #[test]
    fn group_to_numeric_move_is_deferred() {
        // A GROUP source into a numeric receiver is a later rung.
        let err = run_ws(
            &[
                "01  G.",
                "    05  A  PIC X(2) VALUE \"04\".",
                "    05  B  PIC X(1) VALUE \"2\".",
                "01  N  PIC 9(3).",
            ],
            &["    MOVE G TO N.", "    STOP RUN."],
        )
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
    }

    // ----------------------------------------------------------------------
    // Mixed numeric ↔ alphanumeric comparison.
    //
    // A relation comparing an UNSIGNED numeric operand (integer OR scaled,
    // `PIC 9(i)V9(d)`) with an ALPHANUMERIC one treats the numeric operand as
    // though moved to an alphanumeric field — its digit image (`Decimal::digits()`
    // yields the item's fixed-width `(int + frac)` zero-padded storage, no point) —
    // then compares by the alphanumeric byte rule (space-pad the shorter side,
    // byte-by-byte). A SIGNED numeric operand is also supported: its image is that
    // same magnitude with the operational sign folded into a TRAILING OVERPUNCH on
    // the units digit (`overpunch_trailing`), so `PIC S9(3) = -123` compares equal
    // to "12L" and `= +123` equal to "12C". Only a group item in a mixed comparison
    // is still a clean later rung, rejected to match the compiler.
    // ----------------------------------------------------------------------

    #[test]
    fn mixed_numeric_equals_matching_alphanumeric_literal() {
        // NUM PIC 9(3)=42 → image "042"; "042" = "042" → equal.
        let out = run_ws(
            &["01  NUM  PIC 9(3) VALUE 42."],
            &["    IF NUM = \"042\" DISPLAY \"MATCH\" ELSE DISPLAY \"NO\".", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(out, "MATCH\n");
    }

    #[test]
    fn mixed_numeric_space_pad_mismatch() {
        // "042" vs "42" (space-padded to "42 ") differ — the byte rule, not a
        // value comparison.
        let out = run_ws(
            &["01  NUM  PIC 9(3) VALUE 42."],
            &["    IF NUM = \"42\" DISPLAY \"MATCH\" ELSE DISPLAY \"NO\".", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(out, "NO\n");
    }

    #[test]
    fn mixed_numeric_ordering_and_right_operand() {
        // Ordering ("042" > "040") and the numeric operand on the RIGHT.
        let out = run_ws(
            &["01  NUM  PIC 9(3) VALUE 42."],
            &[
                "    IF NUM > \"040\" DISPLAY \"GT\" ELSE DISPLAY \"LE\".",
                "    IF \"042\" = NUM DISPLAY \"MATCH\" ELSE DISPLAY \"NO\".",
                "    STOP RUN.",
            ],
        )
        .unwrap();
        assert_eq!(out, "GT\nMATCH\n");
    }

    #[test]
    fn mixed_numeric_against_a_pic_x_item() {
        // The alphanumeric side is a `PIC X` item, not a literal.
        let out = run_ws(
            &["01  NUM  PIC 9(3) VALUE 42.", "01  W  PIC X(3) VALUE \"042\"."],
            &["    IF NUM = W DISPLAY \"MATCH\" ELSE DISPLAY \"NO\".", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(out, "MATCH\n");
    }

    #[test]
    fn mixed_signed_numeric_vs_alphanumeric_uses_overpunched_image() {
        // A SIGNED numeric operand compares by its magnitude image with a trailing
        // sign overpunch on the units digit (the same bytes the signed→alphanumeric
        // MOVE builds): `PIC S9(3) = -123` → "12L", `= +123` → "12C". Equality and an
        // ORDERING relation both follow the byte comparison of those images.
        let neg = run_ws(
            &["01  S  PIC S9(3) VALUE -123."],
            &[
                "    IF S = \"12L\" DISPLAY \"EQ\" ELSE DISPLAY \"NE\".",
                "    IF S = \"12C\" DISPLAY \"EQ\" ELSE DISPLAY \"NE\".",
                "    IF S < \"12M\" DISPLAY \"LT\" ELSE DISPLAY \"GE\".",
                "    STOP RUN.",
            ],
        )
        .unwrap();
        assert_eq!(neg, "EQ\nNE\nLT\n");
        // A signed item with an unsigned VALUE is POSITIVE (neg=false), so its image
        // takes the positive `{…I` overpunch row: 123 → "12C".
        let pos = run_ws(
            &["01  S  PIC S9(3) VALUE 123."],
            &["    IF S = \"12C\" DISPLAY \"EQ\" ELSE DISPLAY \"NE\".", "    STOP RUN."],
        )
        .unwrap();
        assert_eq!(pos, "EQ\n");
    }

    #[test]
    fn mixed_signed_units_zero_and_scaled_overpunch() {
        // Units digit 0 selects '}' (negative) / '{' (positive); a scaled
        // `PIC S9V9 = -4.2` overpunches the last fraction digit → "4K".
        let out = run_ws(
            &["01  S  PIC S9(3) VALUE -120.", "01  P  PIC S9(3) VALUE 120.", "01  F  PIC S9V9 VALUE -4.2."],
            &[
                "    IF S = \"12}\" DISPLAY \"EQ\" ELSE DISPLAY \"NE\".",
                "    IF P = \"12{\" DISPLAY \"EQ\" ELSE DISPLAY \"NE\".",
                "    IF F = \"4K\" DISPLAY \"EQ\" ELSE DISPLAY \"NE\".",
                "    STOP RUN.",
            ],
        )
        .unwrap();
        assert_eq!(out, "EQ\nEQ\nEQ\n");
    }

    #[test]
    fn mixed_signed_zero_magnitude_compares_positive() {
        // COBOL has no negative zero: -1000 truncated into PIC S9(3) stores an
        // all-zero POSITIVE slot whose overpunched image is "00{" (units 0, positive),
        // so `IF S = "00{"` is TRUE — no reintroduced sign-of-zero divergence.
        let out = run_ws(
            &["01  A  PIC S9(4) VALUE -1000.", "01  S  PIC S9(3)."],
            &[
                "    MOVE A TO S.",
                "    IF S = \"00{\" DISPLAY \"EQ\" ELSE DISPLAY \"NE\".",
                "    STOP RUN.",
            ],
        )
        .unwrap();
        assert_eq!(out, "EQ\n");
    }

    #[test]
    fn mixed_numeric_literal_vs_alphanumeric_is_deferred() {
        // A numeric LITERAL against an alphanumeric operand is a different pairing,
        // still out of scope on both engines.
        let err = run_ws(
            &["01  W  PIC X(3) VALUE \"042\"."],
            &["    IF 42 = W DISPLAY \"Y\".", "    STOP RUN."],
        )
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn figurative_vs_figurative_comparison() {
        // Two figurative constants compare by filling each to a single character
        // (ZERO → "0", SPACE → " "): ZERO = ZERO and SPACE = SPACE are true; ZERO ≠
        // SPACE and, by byte ('0'=0x30 > ' '=0x20), ZERO > SPACE. Matches the compiler.
        let out = run_ws(
            &["01  D  PIC X(1)."],
            &[
                "    IF ZERO = ZERO DISPLAY \"ZZ\" ELSE DISPLAY \"zz\".",
                "    IF SPACE = SPACE DISPLAY \"SS\" ELSE DISPLAY \"ss\".",
                "    IF ZERO = SPACE DISPLAY \"EQ\" ELSE DISPLAY \"NE\".",
                "    IF ZERO > SPACE DISPLAY \"GT\" ELSE DISPLAY \"le\".",
                "    STOP RUN.",
            ],
        )
        .unwrap();
        assert_eq!(out, "ZZ\nSS\nNE\nGT\n");
    }

    #[test]
    fn mixed_scaled_numeric_vs_alphanumeric_uses_digit_image() {
        // An UNSIGNED SCALED operand `PIC 9(2)V9 = 4.2` compares by its (int + frac)
        // digit image "042" — no point — so `IF F = "042"` is TRUE and `IF F > "040"`
        // is TRUE, matching the compiler.
        let out = run_ws(
            &["01  F  PIC 9(2)V9 VALUE 4.2."],
            &[
                "    IF F = \"042\" DISPLAY \"EQ\" ELSE DISPLAY \"NE\".",
                "    IF F > \"040\" DISPLAY \"GT\" ELSE DISPLAY \"LE\".",
                "    STOP RUN.",
            ],
        )
        .unwrap();
        assert_eq!(out, "EQ\nGT\n");
    }

    #[test]
    fn mixed_group_item_vs_numeric_is_deferred() {
        // A GROUP item in a mixed numeric comparison is a later rung.
        let err = run_ws(
            &[
                "01  G.",
                "    05  A  PIC X(2) VALUE \"04\".",
                "    05  B  PIC X(1) VALUE \"2\".",
                "01  NUM  PIC 9(3) VALUE 42.",
            ],
            &["    IF G = NUM DISPLAY \"Y\".", "    STOP RUN."],
        )
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
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
    fn evaluate_numeric_subject_vs_alphanumeric_when() {
        // A numeric subject vs an alphanumeric WHEN value routes through
        // `compare_operands` exactly like `IF N = "042"`: N PIC 9(3)=42 → digit
        // image "042" matches WHEN "042"; WHEN "42" space-pads to "42 " ('0' < '4')
        // → no match. This pins the oracle's mixed EVALUATE that the compiler now
        // matches byte-for-byte.
        let run = |v: &str| {
            let body = [
                "EVALUATE N".to_string(),
                format!("WHEN {v} DISPLAY \"HIT\""),
                "WHEN OTHER DISPLAY \"MISS\"".to_string(),
                "END-EVALUATE.".to_string(),
                "STOP RUN.".to_string(),
            ];
            let refs: Vec<&str> = body.iter().map(|s| s.as_str()).collect();
            run_if("42", &refs)
        };
        assert_eq!(run("\"042\""), "HIT\n");
        assert_eq!(run("\"42\""), "MISS\n");
    }

    #[test]
    fn evaluate_alpha_subject_vs_numeric_literal_when_is_a_later_rung() {
        // An alphanumeric subject vs a numeric-LITERAL WHEN value is a *different*
        // pairing (numeric literal vs alphanumeric) — `compare_operands` rejects it,
        // exactly as the compiler's `num_digit_str_operand` does. Both engines
        // defer this identically.
        let body = &[
            "EVALUATE GRADE",
            "WHEN 42 DISPLAY \"HIT\"",
            "WHEN OTHER DISPLAY \"MISS\"",
            "END-EVALUATE.",
            "STOP RUN.",
        ];
        assert!(run_alpha_evaluate("A", body).is_err());
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
    fn level_88_on_an_alphanumeric_item_with_a_discrete_string_value_holds() {
        // A condition-name over an alphanumeric variable with a discrete string
        // VALUE now reads: FLAG holds "Y", so IS-YES (VALUE "Y") is true.
        let out = run_cobol(&program(&[
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
        .unwrap();
        assert_eq!(out, "YES\n");
    }

    #[test]
    fn level_88_alphanumeric_thru_range_reads_inclusively() {
        // A `THRU` range with STRING bounds on an alphanumeric conditional variable
        // now reads: GRADE holds "C", and PASSING (VALUE "A" THRU "D") is an
        // inclusive range that contains it → true.
        let out = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  GRADE  PIC X VALUE \"C\".",
            "88  PASSING  VALUE \"A\" THRU \"D\".",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    IF PASSING DISPLAY \"pass\" ELSE DISPLAY \"fail\".",
            "    STOP RUN.",
        ]))
        .unwrap();
        assert_eq!(out, "pass\n");
    }

    #[test]
    fn level_88_alphanumeric_thru_range_with_a_numeric_bound_is_still_a_later_rung() {
        // A `THRU` range with a NON-string (numeric) bound on an alphanumeric
        // conditional variable stays deferred — a clean error, never a wrong answer.
        let err = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  GRADE  PIC X VALUE \"C\".",
            "88  IN-RANGE  VALUE \"A\" THRU 5.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    IF IN-RANGE DISPLAY \"YES\".",
            "    STOP RUN.",
        ]))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn level_88_alphanumeric_numeric_value_is_still_a_later_rung() {
        // A numeric VALUE on an alphanumeric conditional variable stays deferred.
        let err = run_cobol(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  FLAG  PIC X VALUE \"5\".",
            "88  IS-FIVE  VALUE 5.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    IF IS-FIVE DISPLAY \"YES\".",
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
    fn string_delimited_by_a_single_char_delimiter() {
        // Each field contributes only its prefix up to its first delimiter char:
        // "ab,cd" → "ab", "ef" (no comma) → "ef", "gh,ij" → "gh"; concat "abefgh".
        let out = run_cobol(&wrap(
            &[
                "01  A  PIC X(5) VALUE \"ab,cd\".",
                "01  B  PIC X(2) VALUE \"ef\".",
                "01  C  PIC X(5) VALUE \"gh,ij\".",
                "01  T  PIC X(20) VALUE SPACES.",
            ],
            &[
                "STRING A B C DELIMITED BY \",\"",
                "    INTO T.",
                "DISPLAY T.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "abefgh              \n");
    }

    #[test]
    fn string_later_rung_options_are_clean_errors() {
        // A NON-ASCII delimiter is a later rung (byte-vs-char) …
        let delim = run_cobol(&wrap(
            &["01  A  PIC X(3) VALUE \"ABC\".", "01  T  PIC X(6) VALUE SPACES."],
            &["STRING A DELIMITED BY \"é\" INTO T.", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(delim, RuntimeError::Unsupported(_)), "got {delim:?}");
        // … a multi-character delimiter is a later rung …
        let multi = run_cobol(&wrap(
            &["01  A  PIC X(3) VALUE \"ABC\".", "01  T  PIC X(6) VALUE SPACES."],
            &["STRING A DELIMITED BY \"ab\" INTO T.", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(multi, RuntimeError::Unsupported(_)), "got {multi:?}");
        // (`WITH POINTER` and `ON OVERFLOW` / `NOT ON OVERFLOW` are now modelled — see
        // the `string_with_pointer_*` and `string_on_overflow_*` tests below.)
    }

    #[test]
    fn string_with_pointer_overlays_at_offset_and_writes_resume_back() {
        // `WITH POINTER p` overlays the concatenation starting at 0-based `p-1` and
        // writes the pointer back to `p + chars_placed`. "XY" (2 chars) at p = 3 into
        // a 6-wide receiver lands at positions 2–3, preserving head and tail dots, and
        // p becomes 3 + 2 = 5.
        let out = run_cobol(&wrap(
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
        ))
        .unwrap();
        assert_eq!(out, "..XY..\n05\n");
    }

    #[test]
    fn string_with_pointer_out_of_range_leaves_everything_unchanged() {
        // An out-of-range initial pointer (p = 0 or p > size) is ISO overflow: no
        // character is transferred and the pointer is left unchanged. p = 0 here → the
        // receiver keeps its sentinel and p stays 0.
        let out = run_cobol(&wrap(
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
        ))
        .unwrap();
        assert_eq!(out, "....\n00\n");
    }

    #[test]
    fn string_with_pointer_bad_picture_is_a_later_rung() {
        // The pointer must be an unsigned integer `PIC 9(n)`. A signed pointer is a
        // clean later rung, rejected at exec time (co-total with the compiler).
        let err = run_cobol(&wrap(
            &[
                "01  A  PIC X(3) VALUE \"ABC\".",
                "01  T  PIC X(6) VALUE \"......\".",
                "01  P  PIC S9(2) VALUE 1.",
            ],
            &["STRING A DELIMITED BY SIZE INTO T WITH POINTER P.", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn string_on_overflow_runs_when_content_is_dropped() {
        // Overflow: "abcd"+"efgh" (8 chars) into a 5-wide receiver drops 3 chars, so
        // the ON OVERFLOW imperative runs (sets F = "YES") and the NOT clause does not.
        let out = run_cobol(&wrap(
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
        ))
        .unwrap();
        assert_eq!(out, "abcde\nYES\n");
    }

    #[test]
    fn string_not_on_overflow_runs_when_content_fits() {
        // No overflow: "ab"+"cd" (4 chars) fits a 5-wide receiver, so the NOT ON
        // OVERFLOW imperative runs (F = "NON") and the ON clause does not.
        let out = run_cobol(&wrap(
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
        ))
        .unwrap();
        assert_eq!(out, "abcd.\nNON\n");
    }

    #[test]
    fn string_pointer_out_of_range_runs_on_overflow() {
        // An out-of-range initial pointer (p = 0) is overflow: NO data movement and
        // the pointer is left unchanged, but the ON OVERFLOW imperative now runs.
        let out = run_cobol(&wrap(
            &[
                "01  A  PIC X(3) VALUE \"abc\".",
                "01  T  PIC X(6) VALUE \"......\".",
                "01  P  PIC 9(2) VALUE 0.",
                "01  F  PIC X(3) VALUE \"no \".",
            ],
            &[
                "STRING A DELIMITED BY SIZE INTO T WITH POINTER P",
                "    ON OVERFLOW MOVE \"YES\" TO F.",
                "DISPLAY T.",
                "DISPLAY P.",
                "DISPLAY F.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "......\n00\nYES\n");
    }

    #[test]
    fn unstring_splits_into_receivers() {
        // "A,B,C" → three fields into three PIC X(3) receivers, each left-
        // justified and space-padded.
        let out = run_cobol(&wrap(
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
        ))
        .unwrap();
        assert_eq!(out, "A  \nB  \nC  \n");
    }

    #[test]
    fn unstring_empty_fields_and_unchanged_trailing_receiver() {
        // "A,,C" bounds an empty middle field (R2 → spaces); a shorter source
        // leaves the trailing receiver's prior VALUE intact.
        let empties = run_cobol(&wrap(
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
        ))
        .unwrap();
        assert_eq!(empties, "A  \n   \nC  \n");

        let short = run_cobol(&wrap(
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
        ))
        .unwrap();
        assert_eq!(short, "A  \nB  \nZZZ\n");
    }

    #[test]
    fn unstring_later_rung_options_are_clean_errors() {
        // ON OVERFLOW is now MODELLED (see `unstring_on_overflow_*`); here "A,B" into
        // ONE receiver fills R1="A" and leaves the source unexhausted (field "B"
        // remains), so overflow fires and the imperative runs.
        let overflow = run_cobol(&wrap(
            &["01  S  PIC X(3) VALUE \"A,B\".", "01  R1 PIC X(3) VALUE SPACES."],
            &[
                "UNSTRING S DELIMITED BY \",\" INTO R1",
                "    ON OVERFLOW DISPLAY \"OVF\".",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(overflow, "OVF\n");

        // … a multi-character delimiter needs a multi-char scan …
        let multi = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"A::B\".", "01  R1 PIC X(3) VALUE SPACES."],
            &["UNSTRING S DELIMITED BY \"::\" INTO R1.", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(multi, RuntimeError::Unsupported(_)), "got {multi:?}");

        // … and a numeric receiver needs numeric editing on receipt.
        let numeric = run_cobol(&wrap(
            &["01  S  PIC X(3) VALUE \"1,2\".", "01  N  PIC 9(3) VALUE 0."],
            &["UNSTRING S DELIMITED BY \",\" INTO N.", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(numeric, RuntimeError::Unsupported(_)), "got {numeric:?}");
    }

    #[test]
    fn unstring_with_pointer_starts_offset_and_writes_resume_back() {
        // p = 3 over "a,b,c" starts at 0-based index 2 ("b,c"): R1="b", R2="c",
        // and the pointer is updated to one past the last examined char — final
        // cursor 6, clamped to len 5, +1 → 6 ("06").
        let out = run_cobol(&wrap(
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
        ))
        .unwrap();
        assert_eq!(out, "b  \nc  \n06\n");
    }

    #[test]
    fn unstring_with_pointer_one_matches_no_pointer_receivers() {
        // The anchor: p = 1 fills the SAME receivers as the no-pointer statement.
        let data: &[&str] = &[
            "01  S  PIC X(5) VALUE \"a,b,c\".",
            "01  R1 PIC X(3) VALUE SPACES.",
            "01  R2 PIC X(3) VALUE SPACES.",
            "01  R3 PIC X(3) VALUE SPACES.",
        ];
        let no_ptr = run_cobol(&wrap(
            data,
            &[
                "UNSTRING S DELIMITED BY \",\" INTO R1 R2 R3.",
                "DISPLAY R1.",
                "DISPLAY R2.",
                "DISPLAY R3.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        let mut data_ptr = data.to_vec();
        data_ptr.push("01  P  PIC 9(2) VALUE 1.");
        let with_ptr = run_cobol(&wrap(
            &data_ptr,
            &[
                "UNSTRING S DELIMITED BY \",\" INTO R1 R2 R3 WITH POINTER P.",
                "DISPLAY R1.",
                "DISPLAY R2.",
                "DISPLAY R3.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(no_ptr, "a  \nb  \nc  \n");
        assert_eq!(with_ptr, no_ptr, "p = 1 must fill the same receivers");
    }

    #[test]
    fn unstring_with_pointer_out_of_range_leaves_everything_unchanged() {
        // p = 0 and p > len are ISO overflow: no receiver modified, pointer
        // unchanged. R1 keeps "ZZZ"; P keeps its initial value.
        for pval in ["0", "6", "9"] {
            let out = run_cobol(&wrap(
                &[
                    "01  S  PIC X(5) VALUE \"a,b,c\".",
                    "01  R1 PIC X(3) VALUE \"ZZZ\".",
                    &format!("01  P  PIC 9(2) VALUE {pval}."),
                ],
                &[
                    "UNSTRING S DELIMITED BY \",\" INTO R1 WITH POINTER P.",
                    "DISPLAY R1.",
                    "DISPLAY P.",
                    "STOP RUN.",
                ],
            ))
            .unwrap();
            // Two-digit zero-padded echo of the UNCHANGED pointer value.
            let expected = format!("ZZZ\n{:02}\n", pval.parse::<u32>().unwrap());
            assert_eq!(out, expected, "pval={pval}");
        }
    }

    #[test]
    fn unstring_with_pointer_bad_picture_is_a_later_rung() {
        // Signed, fractional, and non-numeric pointers are clean later rungs.
        for pic in ["S9(2)", "9V9", "X(2)"] {
            let err = run_cobol(&wrap(
                &[
                    "01  S  PIC X(5) VALUE \"a,b,c\".",
                    "01  R1 PIC X(3) VALUE SPACES.",
                    &format!("01  P  PIC {pic}."),
                ],
                &["UNSTRING S DELIMITED BY \",\" INTO R1 WITH POINTER P.", "STOP RUN."],
            ))
            .unwrap_err();
            assert!(matches!(err, RuntimeError::Unsupported(_)), "pic={pic}: got {err:?}");
        }
    }

    #[test]
    fn inspect_tallying_counts_and_adds_to_the_counter() {
        // "BANANA" has three A's, added to C (starting 0) → 3.
        let out = run_cobol(&wrap(
            &["01  S  PIC X(6) VALUE \"BANANA\".", "01  C  PIC 9(3) VALUE 0."],
            &["INSPECT S TALLYING C FOR ALL \"A\".", "DISPLAY C.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(out, "003\n");
    }

    #[test]
    fn inspect_tallying_adds_to_a_nonzero_counter() {
        // C starts at 5; four S's in "MISSISSIPPI" → 5 + 4 = 9 (ADD, not replace).
        // A PIC X(1) delimiter item and a zero-count case are covered too.
        let out = run_cobol(&wrap(
            &["01  S  PIC X(11) VALUE \"MISSISSIPPI\".", "01  C  PIC 9(3) VALUE 5."],
            &["INSPECT S TALLYING C FOR ALL \"S\".", "DISPLAY C.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(out, "009\n");

        let none = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"HELLO\".", "01  C  PIC 9(2) VALUE 0."],
            &["INSPECT S TALLYING C FOR ALL \"Z\".", "DISPLAY C.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(none, "00\n");
    }

    #[test]
    fn inspect_tallying_for_leading_counts_only_a_leading_run() {
        // FOR LEADING counts the run of consecutive delimiters at the START, then
        // stops at the first non-match. "000123" → three leading "0"s → 3.
        let out = run_cobol(&wrap(
            &["01  S  PIC X(6) VALUE \"000123\".", "01  C  PIC 9(3) VALUE 0."],
            &["INSPECT S TALLYING C FOR LEADING \"0\".", "DISPLAY C.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(out, "003\n");

        // A non-delimiter first character stops the run immediately: "120003" has
        // three "0"s but NONE are leading → 0 (whereas FOR ALL would give 3).
        let gap = run_cobol(&wrap(
            &["01  S  PIC X(6) VALUE \"120003\".", "01  C  PIC 9(3) VALUE 0."],
            &["INSPECT S TALLYING C FOR LEADING \"0\".", "DISPLAY C.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(gap, "000\n");

        // An all-delimiter source counts every character: "0000" → 4.
        let all = run_cobol(&wrap(
            &["01  S  PIC X(4) VALUE \"0000\".", "01  C  PIC 9(3) VALUE 0."],
            &["INSPECT S TALLYING C FOR LEADING \"0\".", "DISPLAY C.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(all, "004\n");

        // FOR LEADING adds to the counter (does not clear): C starts at 5, two
        // leading "0"s in "00X", and the delimiter is a PIC X(1) item → 5 + 2 = 7.
        let adds = run_cobol(&wrap(
            &[
                "01  S  PIC X(3) VALUE \"00X\".",
                "01  D  PIC X(1) VALUE \"0\".",
                "01  C  PIC 9(3) VALUE 5.",
            ],
            &["INSPECT S TALLYING C FOR LEADING D.", "DISPLAY C.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(adds, "007\n");
    }

    #[test]
    fn inspect_tallying_for_all_with_a_before_after_region() {
        // BEFORE "C" restricts the count to "AB0" → one "0"; AFTER "C" restricts it
        // to "D0" → one "0". Same source, complementary windows.
        let before = run_cobol(&wrap(
            &["01  S  PIC X(6) VALUE \"AB0CD0\".", "01  C  PIC 9(3) VALUE 0."],
            &["INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"C\".", "DISPLAY C.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(before, "001\n");

        let after = run_cobol(&wrap(
            &["01  S  PIC X(6) VALUE \"AB0CD0\".", "01  C  PIC 9(3) VALUE 0."],
            &["INSPECT S TALLYING C FOR ALL \"0\" AFTER \"C\".", "DISPLAY C.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(after, "001\n");

        // The not-found ASYMMETRY: BEFORE with the delimiter absent counts the WHOLE
        // source (both "0"s → 2); AFTER with it absent counts NOTHING (0).
        let before_absent = run_cobol(&wrap(
            &["01  S  PIC X(6) VALUE \"AB0CD0\".", "01  C  PIC 9(3) VALUE 0."],
            &["INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"Z\".", "DISPLAY C.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(before_absent, "002\n");

        let after_absent = run_cobol(&wrap(
            &["01  S  PIC X(6) VALUE \"AB0CD0\".", "01  C  PIC 9(3) VALUE 0."],
            &["INSPECT S TALLYING C FOR ALL \"0\" AFTER \"Z\".", "DISPLAY C.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(after_absent, "000\n");

        // The region delimiter equal to the tally delimiter: AFTER "A" in "ABABA" —
        // the first "A" (index 0) bounds the region to "BABA", where two "A"s remain.
        let same = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"ABABA\".", "01  C  PIC 9(3) VALUE 0."],
            &["INSPECT S TALLYING C FOR ALL \"A\" AFTER \"A\".", "DISPLAY C.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(same, "002\n");
    }

    #[test]
    fn inspect_tallying_for_leading_with_a_before_after_region() {
        // The STANDALONE `FOR LEADING … {BEFORE|AFTER}` form is now supported, with the
        // leading run ANCHORED at the window start. "aaXaab" AFTER "X" narrows to "aab"
        // — the leading "a" run there is 2, the "aa" before the X ignored.
        let after = run_cobol(&wrap(
            &["01  S  PIC X(6) VALUE \"aaXaab\".", "01  C  PIC 9(3) VALUE 0."],
            &["INSPECT S TALLYING C FOR LEADING \"a\" AFTER \"X\".", "DISPLAY C.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(after, "002\n");

        // The window's first char is a mismatch ⇒ leading run 0, even though a's
        // precede the X: "aaXbb" AFTER "X" → window "bb" → 0.
        let mismatch = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"aaXbb\".", "01  C  PIC 9(3) VALUE 0."],
            &["INSPECT S TALLYING C FOR LEADING \"a\" AFTER \"X\".", "DISPLAY C.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(mismatch, "000\n");

        // BEFORE counts the prefix run: "aaXaa" BEFORE "X" → window "aa" → 2.
        let before = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"aaXaa\".", "01  C  PIC 9(3) VALUE 0."],
            &["INSPECT S TALLYING C FOR LEADING \"a\" BEFORE \"X\".", "DISPLAY C.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(before, "002\n");

        // Not-found asymmetry: AFTER "Z" absent ⇒ EMPTY window ⇒ 0; BEFORE "Z" absent ⇒
        // WHOLE source ⇒ the leading run from position 0 (2 in "aaXaa").
        let after_absent = run_cobol(&wrap(
            &["01  S  PIC X(6) VALUE \"aaXaab\".", "01  C  PIC 9(3) VALUE 0."],
            &["INSPECT S TALLYING C FOR LEADING \"a\" AFTER \"Z\".", "DISPLAY C.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(after_absent, "000\n");

        let before_absent = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"aaXaa\".", "01  C  PIC 9(3) VALUE 0."],
            &["INSPECT S TALLYING C FOR LEADING \"a\" BEFORE \"Z\".", "DISPLAY C.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(before_absent, "002\n");
    }

    #[test]
    fn inspect_tallying_region_later_rung_forms_are_clean_errors() {
        // The STANDALONE `FOR LEADING … BEFORE/AFTER` form is now supported (see
        // `inspect_tallying_for_leading_with_a_before_after_region`). What remains
        // deferred is a MULTI-character region delimiter — rejected at exec, exactly
        // like a multi-character tally delimiter. (A LEADING half PLUS a region on the
        // COMBINED TALLYING … REPLACING form is still deferred — see
        // `inspect_tally_replace_combined_leading_with_region_is_a_later_rung`.)
        let multi = run_cobol(&wrap(
            &["01  S  PIC X(6) VALUE \"AB0CD0\".", "01  C  PIC 9(3) VALUE 0."],
            &["INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"CD\".", "STOP RUN."],
        ));
        assert!(multi.is_err(), "multi-char region delimiter must reject");
    }

    #[test]
    fn inspect_replacing_maps_every_occurrence_in_place() {
        // "ABABA" with A→X → "XBXBX" (same width, per-position map).
        let out = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"ABABA\"."],
            &["INSPECT S REPLACING ALL \"A\" BY \"X\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(out, "XBXBX\n");

        // A character that never occurs leaves the source unchanged.
        let miss = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"HELLO\"."],
            &["INSPECT S REPLACING ALL \"Z\" BY \"Q\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(miss, "HELLO\n");

        // The search and replacement can be PIC X(1) items.
        let via_items = run_cobol(&wrap(
            &[
                "01  S  PIC X(4) VALUE \"MOON\".",
                "01  X  PIC X(1) VALUE \"O\".",
                "01  Y  PIC X(1) VALUE \"0\".",
            ],
            &["INSPECT S REPLACING ALL X BY Y.", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(via_items, "M00N\n");
    }

    #[test]
    fn inspect_replacing_characters_fills_the_whole_field() {
        // REPLACING CHARACTERS BY overwrites EVERY position unconditionally — even
        // embedded spaces. "A B C" (5 chars) → "-----".
        let out = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"A B C\"."],
            &["INSPECT S REPLACING CHARACTERS BY \"-\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(out, "-----\n");
    }

    #[test]
    fn inspect_replacing_characters_replacement_can_be_a_pic_x1_item() {
        // The replacement `x` may be a PIC X(1) DATA ITEM, not just a literal.
        let out = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"hello\".", "01  R  PIC X(1) VALUE \"*\"."],
            &["INSPECT S REPLACING CHARACTERS BY R.", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(out, "*****\n");
    }

    #[test]
    fn inspect_replacing_characters_non_ascii_source_caps_to_picture_width() {
        // `PIC X(5) VALUE "café"` stores "café " (5 CHARS / 6 BYTES). The byte-basis
        // fill builds n = 6 copies of "Z", then `move_into` caps to the picture's
        // 5 chars → "ZZZZZ" (FIVE, not six). This is the co-total answer the
        // byte-based compiler also produces (width = 5 copies).
        let out = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"café\"."],
            &["INSPECT S REPLACING CHARACTERS BY \"Z\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(out, "ZZZZZ\n");
    }

    #[test]
    fn inspect_replacing_multi_items_each_with_a_region() {
        // Per-item regions on a multi-item REPLACING (this rung). Source "a0b0a":
        // item 1 `ALL "a" BY "x"` (no region → whole source) turns both "a"s to "x";
        // item 2 `ALL "0" BY "*" BEFORE "b"` (window [0, index_of_b) = [0,2)) turns
        // only the "0" at index 1 to "*"; the "0" at index 3 is outside the window and
        // stays. Result "x*b0x".
        let out = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"a0b0a\"."],
            &[
                "INSPECT S REPLACING ALL \"a\" BY \"x\" ALL \"0\" BY \"*\" BEFORE \"b\".",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "x*b0x\n");
    }

    #[test]
    fn inspect_replacing_multi_before_and_after_windows_first_match_wins() {
        // Two items with differing windows over "aXaXa" (X at index 1, 3): item 1
        // `ALL "a" BY "b" BEFORE "X"` (window [0,1)) claims only index 0; item 2
        // `ALL "a" BY "c" AFTER "X"` (window (1,5]) claims indices 2 and 4. The "a" at
        // index 0 is claimed by the earlier item (first-match-wins). Result "bXcXc".
        let out = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"aXaXa\"."],
            &[
                "INSPECT S REPLACING ALL \"a\" BY \"b\" BEFORE \"X\"",
                "    ALL \"a\" BY \"c\" AFTER \"X\".",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "bXcXc\n");
    }

    #[test]
    fn inspect_replacing_multi_after_absent_delimiter_is_an_empty_window() {
        // `AFTER x` with x absent → an EMPTY window: that item NEVER fires; the other
        // (region-less) item still applies. Source "abab", item 1 `ALL "a" BY "*"
        // AFTER "Z"` (no "Z" → empty window), item 2 `ALL "b" BY "y"` rewrites both
        // "b"s. The "a"s stay untouched. Result "ayay".
        let out = run_cobol(&wrap(
            &["01  S  PIC X(4) VALUE \"abab\"."],
            &[
                "INSPECT S REPLACING ALL \"a\" BY \"*\" AFTER \"Z\" ALL \"b\" BY \"y\".",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "ayay\n");
    }

    #[test]
    fn inspect_tally_multi_items_each_with_a_region() {
        // Per-item regions on a multi-item TALLYING (this rung, the count-side analogue
        // of the REPLACING form above). Source "0a0a0" into one counter:
        // item 1 `ALL "0" AFTER "a"` (first "a" at index 1 → window [2,5)) counts the
        // "0"s at indices 2 and 4; item 2 `ALL "a"` (no region → whole source) counts
        // both "a"s at indices 1 and 3. The "0" at index 0 is outside item 1's window
        // and is not an "a", so it is not counted → 4.
        let out = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"0a0a0\".", "01  C  PIC 9(2) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"0\" AFTER \"a\" ALL \"a\".",
                "DISPLAY C.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "04\n");
    }

    #[test]
    fn inspect_tally_multi_before_and_after_windows_first_match_per_position() {
        // Two items with differing windows over "aXaXa" (X at indices 1, 3): item 1
        // `ALL "a" BEFORE "X"` (window [0,1)) counts only index 0; item 2 `ALL "a"
        // AFTER "X"` (window [2,5)) counts indices 2 and 4. Each position is counted by
        // the FIRST item whose window contains it and whose delimiter matches → 3.
        let out = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"aXaXa\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"a\" BEFORE \"X\"",
                "    ALL \"a\" AFTER \"X\".",
                "DISPLAY C.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "003\n");
    }

    #[test]
    fn inspect_tally_multi_after_absent_delimiter_is_an_empty_window() {
        // `AFTER x` with x absent → an EMPTY window: that item contributes 0; the other
        // (region-less) item still counts. Source "abab": item 1 `ALL "a" AFTER "Z"`
        // (no "Z" → empty window, contributes 0), item 2 `ALL "b"` counts both "b"s → 2.
        let out = run_cobol(&wrap(
            &["01  S  PIC X(4) VALUE \"abab\".", "01  C  PIC 9(2) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"a\" AFTER \"Z\" ALL \"b\".",
                "DISPLAY C.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "02\n");
    }

    #[test]
    fn inspect_tally_multi_duplicate_windows_first_match_counts_once() {
        // FIRST-MATCH-PER-POSITION with a DUPLICATE delimiter over overlapping windows:
        // a position matched by BOTH items is counted ONCE. Source "aabaa" (b at index
        // 2): item 1 `ALL "a" BEFORE "b"` (window [0,2) → indices 0,1) and item 2
        // `ALL "a"` (whole source → 0,1,3,4). Indices 0,1 are counted once (by item 1),
        // and item 2 adds indices 3,4 → 4, NOT 6 (a naive per-item sum would double the
        // shared positions).
        let out = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"aabaa\".", "01  C  PIC 9(2) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"a\" BEFORE \"b\" ALL \"a\".",
                "DISPLAY C.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "04\n");
    }

    #[test]
    fn inspect_tally_multi_non_ascii_source_counts_correctly() {
        // POSITIVE non-ASCII parity: TALLYING only COUNTS (it never reconstructs the
        // source), so the char-based oracle counts ASCII delimiters correctly even on a
        // non-ASCII source — the "é" matches no ASCII delimiter. Source "aé0b0":
        // item 1 `ALL "0" BEFORE "b"` counts the "0" before "b"; item 2 `ALL "0" AFTER
        // "b"` counts the "0" after "b" → 2. (The compiler's byte-based scan agrees; the
        // e2e test `inspect_tally_multi_non_ascii_source_positive_parity` pins that.)
        let out = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"aé0b0\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"b\" ALL \"0\" AFTER \"b\".",
                "DISPLAY C.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "002\n");
    }

    #[test]
    fn inspect_tally_multi_leading_and_all_items() {
        // A MULTI-item list mixing a LEADING and an ALL item (this rung lifts the
        // multi-item LEADING reject). `FOR LEADING "a" ALL "b"` over "aabab": the leading
        // run of "a" is indices 0,1 (breaks at the "b" at 2), then ALL "b" counts the b's
        // at 2 and 4; the "a" at index 3 is NOT counted (the leading run is dead). 2+2=4.
        let out = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"aabab\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR LEADING \"a\" ALL \"b\".",
                "DISPLAY C.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "004\n");
    }

    #[test]
    fn inspect_tally_multi_all_then_leading_same_delim_run_survives_claim() {
        // The run-stays-alive subtlety: `FOR ALL "a" LEADING "a"` over "aab". ALL "a"
        // claims indices 0,1 (count 2); the LEADING item never tallies (ALL wins every
        // "a"), but the per-item run flag KEEPS the leading run alive at 0,1 (each char
        // equals "a") — a matching char claimed by a higher-priority item does NOT break
        // the run — and the run decays only at the "b". Count = 2.
        let out = run_cobol(&wrap(
            &["01  S  PIC X(3) VALUE \"aab\".", "01  C  PIC 9(2) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"a\" LEADING \"a\".",
                "DISPLAY C.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "02\n");
    }

    #[test]
    fn inspect_tally_multi_leading_with_region_anchored_at_window_start() {
        // A LEADING item WITH a region + an ALL item — the leading run is anchored at the
        // WINDOW START. `FOR LEADING "a" AFTER "X" ALL "b"` over "aaXaab" (X at index 2):
        // the leading window is "aab" (indices 3..6), so the two "a"s before the X are
        // IGNORED; the run counts the "a"s at 3,4 (breaks at the "b" at 5), and ALL "b"
        // counts the "b" at 5. 2 + 1 = 3.
        let out = run_cobol(&wrap(
            &["01  S  PIC X(6) VALUE \"aaXaab\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR LEADING \"a\" AFTER \"X\" ALL \"b\".",
                "DISPLAY C.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "003\n");
    }

    #[test]
    fn inspect_tally_multi_two_leading_items_disjoint_windows() {
        // Two LEADING items with different delimiters AND disjoint windows, so BOTH runs
        // count — each anchored at its OWN window start. `FOR LEADING "a" BEFORE "X"
        // LEADING "b" AFTER "X"` over "aaXbb" (X at index 2): item 1's window "aa" → 2
        // leading a's; item 2's window "bb" → 2 leading b's. Total 4.
        let out = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"aaXbb\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR LEADING \"a\" BEFORE \"X\"",
                "    LEADING \"b\" AFTER \"X\".",
                "DISPLAY C.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "004\n");
    }

    #[test]
    fn inspect_tally_multi_leading_non_ascii_source_counts_correctly() {
        // POSITIVE non-ASCII parity WITH a LEADING item: the multi-byte "é" equals no
        // ASCII delimiter and BREAKS the leading run at char index 2 (the compiler's byte
        // scan breaks at 0xC3, byte index 2). Source "aaébb": LEADING "a" run "aa" → 2,
        // ALL "b" → 2. Total 4. (The compiler's byte scan agrees; the e2e test
        // `inspect_tally_multi_leading_non_ascii_source_positive_parity` pins that.)
        let out = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"aaébb\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR LEADING \"a\" ALL \"b\".",
                "DISPLAY C.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "004\n");
    }

    #[test]
    fn inspect_tally_counters_items_each_with_a_region() {
        // Per-item regions in the SEVERAL-COUNTERS form (this rung). Source "aXaXa"
        // (X at char indices 1 and 3), two counter groups:
        //   C1 `ALL "a" BEFORE "X"`: window [0,1) → the "a" at index 0 → C1=1;
        //   C2 `ALL "a" AFTER "X"`:  window [2,5) → the "a"s at indices 2, 4 → C2=2.
        // The combined pass counts each position for the first in-window matching entry.
        let out = run_cobol(&wrap(
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
        ))
        .unwrap();
        assert_eq!(out, "001\n002\n");
    }

    #[test]
    fn inspect_tally_counters_earlier_window_starves_later_group() {
        // CROSS-COUNTER first-match with a WINDOW: an earlier group's IN-WINDOW delimiter
        // claims a position, starving a later group of it. Source "aZa" (Z at index 1),
        // both groups matching "a": C1 `ALL "a" BEFORE "Z"` (window [0,1) → only index 0)
        // claims index 0; C2 `ALL "a"` (whole source) then only sees index 2 → C1=1, C2=1
        // (NOT C2=2 — index 0 was starved by C1's in-window delimiter).
        let out = run_cobol(&wrap(
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
        ))
        .unwrap();
        assert_eq!(out, "001\n001\n");
    }

    #[test]
    fn inspect_tally_counters_same_counter_two_groups_with_regions() {
        // The SAME counter name in two groups, each with its OWN region — both regions'
        // hits ADD to that one item. Source "0b0" (b at index 1): group0 `C FOR ALL "0"
        // BEFORE "b"` (window [0,1) → index 0) and group1 `C FOR ALL "0" AFTER "b"`
        // (window [2,3) → index 2) each contribute 1 → C = 0 + 1 + 1 = 2.
        let out = run_cobol(&wrap(
            &["01  S  PIC X(3) VALUE \"0b0\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"b\"",
                "    C FOR ALL \"0\" AFTER \"b\".",
                "DISPLAY C.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "002\n");
    }

    #[test]
    fn inspect_tally_counters_leading_item_is_a_later_rung() {
        // A LEADING item in ANY group of the several-counters path stays a later rung
        // even now that per-item regions are supported — the multi-counter path is
        // ALL-only.
        let err = run_cobol(&wrap(
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
        ));
        assert!(err.is_err(), "a multi-counter LEADING item must be a later rung");
    }

    #[test]
    fn inspect_replacing_multi_leading_and_all_items() {
        // THIS rung: a LEADING item inside a multi-item REPLACING list is now SUPPORTED
        // (this test was the "later rung" reject, converted to a positive). `REPLACING
        // LEADING "a" BY "X" ALL "b" BY "Y"` over "aabaa": the leading run of "a" (indices
        // 0,1) → X; the run breaks at the "b" (index 2 → Y); the "a"s after the break
        // (indices 3,4) are NOT replaced. → "XXYaa". The replace-side twin of #65's
        // tally-multi-LEADING machine (decision loop EMITS instead of counting).
        let out = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"aabaa\"."],
            &[
                "INSPECT S REPLACING LEADING \"a\" BY \"X\" ALL \"b\" BY \"Y\".",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "XXYaa\n");
    }

    #[test]
    fn inspect_replacing_multi_leading_run_breaks_on_higher_priority_claim() {
        // The run-update-independent-of-winner subtlety: a higher-priority ALL item claims
        // position 0, whose char is NOT the LEADING item's search — the LEADING run STILL
        // breaks there (the run-update is a SEPARATE pass, not folded into the first-match
        // decision). `ALL "X" BY "Q" LEADING "a" BY "Z"` over "Xaa": index 0 "X"→"Q" stops
        // the decision, the run-update kills the LEADING "a" run (0's "X" != "a"), so the
        // "a"s at 1,2 are NOT replaced → "Qaa" (a fold-into-decision bug would give "QZZ").
        let out = run_cobol(&wrap(
            &["01  S  PIC X(3) VALUE \"Xaa\"."],
            &[
                "INSPECT S REPLACING ALL \"X\" BY \"Q\" LEADING \"a\" BY \"Z\".",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "Qaa\n");
    }

    #[test]
    fn inspect_replacing_multi_two_leading_items_disjoint_windows() {
        // Two LEADING items with different delimiters AND disjoint windows, so BOTH runs
        // fire — each anchored at its OWN window start. `LEADING "a" BY "X" BEFORE "Z"
        // LEADING "b" BY "Y" AFTER "Z"` over "aaZbb" (Z at index 2): item 1's window "aa"
        // → X,X; item 2's window "bb" → Y,Y; the "Z" stays. → "XXZYY".
        let out = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"aaZbb\"."],
            &[
                "INSPECT S REPLACING LEADING \"a\" BY \"X\" BEFORE \"Z\"",
                "    LEADING \"b\" BY \"Y\" AFTER \"Z\".",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "XXZYY\n");
    }

    #[test]
    fn inspect_replacing_multi_characters_item_is_a_later_rung() {
        // A CHARACTERS item inside a multi-item list stays a later rung.
        let err = run_cobol(&wrap(
            &["01  S  PIC X(4) VALUE \"aabb\"."],
            &[
                "INSPECT S REPLACING ALL \"a\" BY \"x\" CHARACTERS BY \"y\".",
                "STOP RUN.",
            ],
        ))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn inspect_replacing_characters_non_ascii_literal_is_a_later_rung() {
        // A single but NON-ASCII replacement LITERAL ("é") is deferred so the oracle
        // stays co-total with the byte-based compiler (guard 2).
        let err = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"ABABA\"."],
            &["INSPECT S REPLACING CHARACTERS BY \"é\".", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn inspect_replacing_characters_with_a_region_is_a_later_rung() {
        // A `{BEFORE|AFTER}` region on the CHARACTERS item is deferred (guard 3).
        let err = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"ABQBA\"."],
            &["INSPECT S REPLACING CHARACTERS BY \"X\" BEFORE \"Q\".", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn inspect_replacing_later_rung_forms_are_clean_errors() {
        // REPLACING CHARACTERS BY replaces every position unconditionally — now
        // SUPPORTED (fills the whole field with the replacement char): "ABABA" → "XXXXX".
        let chars = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"ABABA\"."],
            &["INSPECT S REPLACING CHARACTERS BY \"X\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(chars, "XXXXX\n");

        // … a multi-character search needs a multi-char scan …
        let multi = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"AB::B\"."],
            &["INSPECT S REPLACING ALL \"::\" BY \"XY\".", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(multi, RuntimeError::Unsupported(_)), "got {multi:?}");

        // … several replace items are now SUPPORTED (the multi-item first-match-wins
        // path): "ABABA" with A→X, B→Y in one pass → "XYXYX".
        let many = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"ABABA\"."],
            &[
                "INSPECT S REPLACING ALL \"A\" BY \"X\" ALL \"B\" BY \"Y\".",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(many, "XYXYX\n");

        // … but the combined TALLYING … REPLACING in one INSPECT is now SUPPORTED:
        // it counts "A" (3 in "ABABA") into C, THEN replaces "B" with "X". The two
        // phrases touch different characters, so ordering is not yet observable
        // here (that is exercised by `inspect_tally_replace_*` below).
        let combined = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"ABABA\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"A\" REPLACING ALL \"B\" BY \"X\".",
                "DISPLAY C.",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(combined, "003\nAXAXA\n");

        // A combined statement whose TALLYING half is FOR LEADING is now SUPPORTED:
        // it counts only the leading run of "A" (2 in "AABBB") into C, THEN replaces
        // ALL "B" with "X" → "AAXXX". (The dedicated ordering/edge cases live in
        // `inspect_tally_replace_for_leading_*`.)
        let combined_lead = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"AABBB\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR LEADING \"A\" REPLACING ALL \"B\" BY \"X\".",
                "DISPLAY C.",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(combined_lead, "002\nAAXXX\n");

        // … and a combined statement whose REPLACING half is REPLACING LEADING is
        // now SUPPORTED too: it counts ALL "B" (3 in "AABBB") into C FIRST, THEN
        // replaces only the LEADING run of "A" → "XXBBB". (The dedicated ordering
        // and both-halves-leading cases live in `inspect_tally_replace_leading_*`.)
        let combined_repl_lead = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"AABBB\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"B\" REPLACING LEADING \"A\" BY \"X\".",
                "DISPLAY C.",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(combined_repl_lead, "003\nXXBBB\n");
    }

    /// A LONE `INSPECT … REPLACING LEADING search BY replace` replaces only the run
    /// of consecutive `search` characters at the START of the source, stopping at
    /// the first character that is not `search`. Positions after that first gap are
    /// left unchanged even if they equal `search` — the key contrast with
    /// `REPLACING ALL`.
    #[test]
    fn inspect_replacing_leading_replaces_only_the_leading_run() {
        // "000123": the three leading "0"s become "*"; the digits after are kept.
        let lead = run_cobol(&wrap(
            &["01  S  PIC X(6) VALUE \"000123\"."],
            &["INSPECT S REPLACING LEADING \"0\" BY \"*\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(lead, "***123\n");

        // "00X00": stops at "X"; the trailing "00" is NOT replaced (the contrast
        // with REPLACING ALL, which would give "**X**").
        let stop = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"00X00\"."],
            &["INSPECT S REPLACING LEADING \"0\" BY \"*\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(stop, "**X00\n");

        // The same source under REPLACING ALL replaces BOTH runs — proving the two
        // forms diverge exactly where the leading run ends.
        let all = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"00X00\"."],
            &["INSPECT S REPLACING ALL \"0\" BY \"*\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(all, "**X**\n");

        // No leading run at all (first char is not the search) — unchanged.
        let none = run_cobol(&wrap(
            &["01  S  PIC X(6) VALUE \"120003\"."],
            &["INSPECT S REPLACING LEADING \"0\" BY \"*\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(none, "120003\n");

        // Every character is the search — the whole field is replaced.
        let all_match = run_cobol(&wrap(
            &["01  S  PIC X(4) VALUE \"0000\"."],
            &["INSPECT S REPLACING LEADING \"0\" BY \"*\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(all_match, "****\n");

        // A blank source has no leading run — unchanged.
        let blank = run_cobol(&wrap(
            &["01  S  PIC X(3)."],
            &["INSPECT S REPLACING LEADING \"0\" BY \"*\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(blank, "   \n");

        // The search and replacement can be PIC X(1) items, not just literals.
        let via_items = run_cobol(&wrap(
            &[
                "01  S  PIC X(6) VALUE \"000123\".",
                "01  X  PIC X(1) VALUE \"0\".",
                "01  Y  PIC X(1) VALUE \"*\".",
            ],
            &["INSPECT S REPLACING LEADING X BY Y.", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(via_items, "***123\n");
    }

    /// The STANDALONE `INSPECT … REPLACING LEADING search BY replace {BEFORE|AFTER} x`
    /// anchors the leading substitution run at the WINDOW START: characters before the
    /// window are untouched and neither begin nor break the run, the run begins at the
    /// window start, and it stops at the first non-`search` INSIDE the window.
    #[test]
    fn inspect_replacing_leading_with_a_before_after_region() {
        // AFTER "X" over "aaXaab" narrows to "aab" — only the two leading a's after the
        // X are rewritten; the "aa" before the X is untouched.
        let after = run_cobol(&wrap(
            &["01  S  PIC X(6) VALUE \"aaXaab\"."],
            &["INSPECT S REPLACING LEADING \"a\" BY \"*\" AFTER \"X\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(after, "aaX**b\n");

        // The window's first char is not `search` ⇒ nothing replaced: "aaXbb" AFTER
        // "X" → window "bb" → unchanged.
        let mismatch = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"aaXbb\"."],
            &["INSPECT S REPLACING LEADING \"a\" BY \"*\" AFTER \"X\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(mismatch, "aaXbb\n");

        // BEFORE rewrites the prefix run: "aaXaa" BEFORE "X" → window "aa" → "**Xaa".
        let before = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"aaXaa\"."],
            &["INSPECT S REPLACING LEADING \"a\" BY \"*\" BEFORE \"X\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(before, "**Xaa\n");

        // Not-found asymmetry: AFTER "Z" absent ⇒ EMPTY window ⇒ nothing replaced;
        // BEFORE "Z" absent ⇒ WHOLE source ⇒ the leading run from position 0.
        let after_absent = run_cobol(&wrap(
            &["01  S  PIC X(6) VALUE \"aaXaab\"."],
            &["INSPECT S REPLACING LEADING \"a\" BY \"*\" AFTER \"Z\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(after_absent, "aaXaab\n");

        let before_absent = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"aaXaa\"."],
            &["INSPECT S REPLACING LEADING \"a\" BY \"*\" BEFORE \"Z\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(before_absent, "**Xaa\n");
    }

    /// The COMBINED `INSPECT … TALLYING … REPLACING` runs tally-then-replace: the
    /// count sees the ORIGINAL bytes, then the replace overwrites the source. When
    /// the tallied delimiter and the replaced search character are the SAME, the
    /// tally must still count every original occurrence.
    #[test]
    fn inspect_tally_replace_shared_char_counts_before_replacing() {
        // "MISSISSIPPI": TALLYING counts "S" (4), THEN REPLACING S→Z. If the tally
        // ran after the replace it would see zero "S" left — the 4 proves ordering.
        let out = run_cobol(&wrap(
            &["01  S  PIC X(11) VALUE \"MISSISSIPPI\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"S\" REPLACING ALL \"S\" BY \"Z\".",
                "DISPLAY C.",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(out, "004\nMIZZIZZIPPI\n");
    }

    /// The COMBINED form's TALLYING half may be `FOR LEADING`: count only the run of
    /// consecutive delimiters at the START (on the ORIGINAL bytes), then run the
    /// `REPLACING ALL` rebuild. The REPLACING half stays `ALL`, so it still touches
    /// every occurrence regardless of where the leading run ended.
    #[test]
    fn inspect_tally_replace_for_leading_counts_only_the_leading_run() {
        // "000X0": leading run of "0" = 3 (stops at 'X'), so C := 0 + 3 = 3. THEN
        // REPLACING ALL "0" BY "*" hits every "0" → "***X*". delim == search proves
        // the tally saw the original bytes before the replace overwrote them.
        let shared = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"000X0\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR LEADING \"0\" REPLACING ALL \"0\" BY \"*\".",
                "DISPLAY C.",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(shared, "003\n***X*\n");

        // No leading run: the first character is not the delimiter, so the count
        // stops immediately at 0 (whereas FOR ALL would count the two later "0"s).
        // The REPLACING ALL still rewrites every "0" → "X**X".
        let no_run = run_cobol(&wrap(
            &["01  S  PIC X(4) VALUE \"X00X\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR LEADING \"0\" REPLACING ALL \"0\" BY \"*\".",
                "DISPLAY C.",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(no_run, "000\nX**X\n");

        // An all-delimiter source: the leading run spans the whole field (4), and
        // the replace rewrites all of it → "****".
        let all = run_cobol(&wrap(
            &["01  S  PIC X(4) VALUE \"0000\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR LEADING \"0\" REPLACING ALL \"0\" BY \"*\".",
                "DISPLAY C.",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(all, "004\n****\n");
    }

    /// The COMBINED form's REPLACING half may be `LEADING`: after the tally counts
    /// (on the ORIGINAL bytes), the rebuild rewrites only the consecutive run of
    /// `search` at the START of the source, stopping at the first non-match. The
    /// TALLYING half's own leading flag is independent — this test drives both
    /// `FOR ALL` and `FOR LEADING` tally halves against a `REPLACING LEADING` half.
    #[test]
    fn inspect_tally_replace_leading_replaces_only_the_leading_run() {
        // "00X00" — TALLYING FOR ALL "0" counts every "0" (4 — two leading, two
        // trailing) into C FIRST, THEN REPLACING LEADING "0" rewrites only the
        // leading run (2, stops at 'X') → "**X00". delim == search proves the tally
        // saw the original bytes: the count is 4 (all zeros) even though only the
        // two leading zeros are ultimately replaced.
        let all_tally = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"00X00\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"0\" REPLACING LEADING \"0\" BY \"*\".",
                "DISPLAY C.",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(all_tally, "004\n**X00\n");

        // Both halves LEADING: TALLYING FOR LEADING "0" counts only the leading run
        // (2) into C, THEN REPLACING LEADING "0" rewrites only that same leading run
        // → "**X00". The two leading flags are threaded independently.
        let both_leading = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"00X00\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR LEADING \"0\" REPLACING LEADING \"0\" BY \"*\".",
                "DISPLAY C.",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(both_leading, "002\n**X00\n");

        // No leading run: the first character is not `search`, so REPLACING LEADING
        // changes nothing, even though later "0"s exist. The FOR ALL tally still
        // counts every "0" (2) → source unchanged "X00X".
        let no_run = run_cobol(&wrap(
            &["01  S  PIC X(4) VALUE \"X00X\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"0\" REPLACING LEADING \"0\" BY \"*\".",
                "DISPLAY C.",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(no_run, "002\nX00X\n");
    }

    /// The COMBINED form's TWO halves each carry an INDEPENDENT `{BEFORE|AFTER}`
    /// region. Because the tally does not mutate the source, BOTH windows are
    /// computed over the SAME original bytes — the count's window and the
    /// replacement's window each see the pre-replacement source.
    #[test]
    fn inspect_tally_replace_with_before_after_regions() {
        // Tally region only: BEFORE "C" over "AB0CD0" counts one "0" (region "AB0");
        // the region-less REPLACING ALL then maps BOTH "0"s → "AB*CD*".
        let tally_only = run_cobol(&wrap(
            &["01  S  PIC X(6) VALUE \"AB0CD0\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"C\"",
                "    REPLACING ALL \"0\" BY \"*\".",
                "DISPLAY C.",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(tally_only, "001\nAB*CD*\n");

        // Replace region only: the region-less TALLYING counts all three "0"s (3),
        // then REPLACING ALL "0" BEFORE "B" restricts to "0A0" → "*A*B0".
        let replace_only = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"0A0B0\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"0\"",
                "    REPLACING ALL \"0\" BY \"*\" BEFORE \"B\".",
                "DISPLAY C.",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(replace_only, "003\n*A*B0\n");

        // BOTH halves, DIFFERENT kinds and delimiters: tally BEFORE "B" over "0A0B0"
        // counts "0A0" → 2; replace AFTER "B" restricts to "0" → "0A0B*".
        let both = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"0A0B0\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"0\" BEFORE \"B\"",
                "    REPLACING ALL \"0\" BY \"*\" AFTER \"B\".",
                "DISPLAY C.",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(both, "002\n0A0B*\n");

        // The not-found asymmetry per half: tally AFTER "Z" (absent) counts NOTHING
        // (empty window) → 0; replace BEFORE "Z" (absent) rewrites the WHOLE source →
        // both "0"s replaced → "*A*".
        let not_found = run_cobol(&wrap(
            &["01  S  PIC X(3) VALUE \"0A0\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"0\" AFTER \"Z\"",
                "    REPLACING ALL \"0\" BY \"*\" BEFORE \"Z\".",
                "DISPLAY C.",
                "DISPLAY S.",
                "STOP RUN.",
            ],
        ))
        .unwrap();
        assert_eq!(not_found, "000\n*A*\n");
    }

    #[test]
    fn inspect_tally_replace_combined_leading_with_region_is_a_later_rung() {
        // The STANDALONE `FOR LEADING`/`REPLACING LEADING … BEFORE/AFTER` forms are
        // supported, but a LEADING half carrying a region on the COMBINED form is still
        // deferred — the combined reader re-imposes the rejection. A LEADING tally half
        // with a region …
        let tally_leading = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"00A0B\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR LEADING \"0\" BEFORE \"A\"",
                "    REPLACING ALL \"0\" BY \"*\".",
                "STOP RUN.",
            ],
        ));
        assert!(tally_leading.is_err(), "combined FOR LEADING + region must reject");

        // … and a LEADING replace half with a region are BOTH deferred.
        let replace_leading = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"00A0B\".", "01  C  PIC 9(3) VALUE 0."],
            &[
                "INSPECT S TALLYING C FOR ALL \"0\"",
                "    REPLACING LEADING \"0\" BY \"*\" BEFORE \"A\".",
                "STOP RUN.",
            ],
        ));
        assert!(replace_leading.is_err(), "combined REPLACING LEADING + region must reject");
    }

    #[test]
    fn inspect_tallying_later_rung_forms_are_clean_errors() {
        // A multi-character delimiter needs a multi-char scan …
        let multi = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"AB::B\".", "01  C  PIC 9(3) VALUE 0."],
            &["INSPECT S TALLYING C FOR ALL \"::\".", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(multi, RuntimeError::Unsupported(_)), "got {multi:?}");

        // … and a non-integer (fractional) counter needs numeric editing.
        let frac = run_cobol(&wrap(
            &["01  S  PIC X(3) VALUE \"A.A\".", "01  C  PIC 9V9 VALUE 0."],
            &["INSPECT S TALLYING C FOR ALL \"A\".", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(frac, RuntimeError::Unsupported(_)), "got {frac:?}");
    }

    #[test]
    fn inspect_converting_translates_through_the_table() {
        // A→X, B→Y, C→Z applied to "CAB" → "ZXY".
        let out = run_cobol(&wrap(
            &["01  S  PIC X(3) VALUE \"CAB\"."],
            &["INSPECT S CONVERTING \"ABC\" TO \"XYZ\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(out, "ZXY\n");

        // A multi-character vowel table: "AEIOU"→"12345" on "BEAN" → "B21N"
        // (B and N are in no entry → unchanged; E→2, A→1).
        let vowels = run_cobol(&wrap(
            &["01  S  PIC X(4) VALUE \"BEAN\"."],
            &["INSPECT S CONVERTING \"AEIOU\" TO \"12345\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(vowels, "B21N\n");

        // A duplicated `from` character: the LEFTMOST entry wins — "AAB"→"XYZ" maps
        // A→X (not the later A→Y), B→Z → "XXZ".
        let dup = run_cobol(&wrap(
            &["01  S  PIC X(3) VALUE \"AAB\"."],
            &["INSPECT S CONVERTING \"AAB\" TO \"XYZ\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(dup, "XXZ\n");
    }

    #[test]
    fn inspect_converting_with_a_before_after_region() {
        // BEFORE "Y" restricts the A→0 translate to "AXA" (indices 0..3) → the two
        // "A"s there become "0"; the trailing "A" (right of "Y") is UNTOUCHED.
        let before = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"AXAYA\"."],
            &["INSPECT S CONVERTING \"A\" TO \"0\" BEFORE \"Y\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(before, "0X0YA\n");

        // AFTER "Y" restricts it to "A" (index 4) → only that trailing "A" → "0".
        let after = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"AXAYA\"."],
            &["INSPECT S CONVERTING \"A\" TO \"0\" AFTER \"Y\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(after, "AXAY0\n");

        // The not-found ASYMMETRY: BEFORE with the delimiter absent translates the
        // WHOLE source (all three "A"s → "0"); AFTER with it absent translates
        // NOTHING (the source is unchanged).
        let before_absent = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"AXAYA\"."],
            &["INSPECT S CONVERTING \"A\" TO \"0\" BEFORE \"Z\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(before_absent, "0X0Y0\n");
        let after_absent = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"AXAYA\"."],
            &["INSPECT S CONVERTING \"A\" TO \"0\" AFTER \"Z\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(after_absent, "AXAYA\n");

        // The region delimiter equal to a `from` character: AFTER "A" in "AXAYA" —
        // the FIRST "A" (index 0) bounds the region to "XAYA"; the translate runs over
        // the ORIGINAL bytes, so the "A"s at indices 2 and 4 become "0", while the
        // delimiter "A" at index 0 (left of the region) is KEPT → "AX0Y0".
        let delim_in_from = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"AXAYA\"."],
            &["INSPECT S CONVERTING \"A\" TO \"0\" AFTER \"A\".", "DISPLAY S.", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(delim_in_from, "AX0Y0\n");
    }

    #[test]
    fn inspect_converting_later_rung_forms_are_clean_errors() {
        // Unequal-length FROM/TO have no well-defined table — a later rung.
        let unequal = run_cobol(&wrap(
            &["01  S  PIC X(3) VALUE \"ABC\"."],
            &["INSPECT S CONVERTING \"AB\" TO \"XYZ\".", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(unequal, RuntimeError::Unsupported(_)), "got {unequal:?}");

        // A PIC X item as the `from` operand (not a string literal) is a later rung.
        let item = run_cobol(&wrap(
            &["01  S  PIC X(3) VALUE \"ABC\".", "01  F  PIC X(3) VALUE \"ABC\"."],
            &["INSPECT S CONVERTING F TO \"XYZ\".", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(item, RuntimeError::Unsupported(_)), "got {item:?}");

        // A MULTI-character region delimiter restricting the conversion is a later
        // rung (a single-character region delimiter is now supported — see
        // `inspect_converting_with_a_before_after_region`).
        let multi_region = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"ABABA\"."],
            &["INSPECT S CONVERTING \"A\" TO \"X\" BEFORE \"BC\".", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(multi_region, RuntimeError::Unsupported(_)), "got {multi_region:?}");

        // CONVERTING is a STANDALONE alternative — combining it with a REPLACING
        // clause in one statement does not parse (the two are mutually exclusive
        // grammar alternatives), so it is a clean parse-time rejection, never a
        // mis-run.
        let combined = run_cobol(&wrap(
            &["01  S  PIC X(5) VALUE \"ABABA\"."],
            &["INSPECT S CONVERTING \"A\" TO \"X\" REPLACING ALL \"B\" BY \"Y\".", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(combined, RuntimeError::Parse(_)), "got {combined:?}");
    }

    // ----------------------------------------------------------------------
    // Reference modification — computed (data-name) indices (the oracle side)
    // ----------------------------------------------------------------------

    #[test]
    fn refmod_computed_mid_substring() {
        // WS(J:K) with J=2, K=3 over "ABCDE" → positions 2..4 → "BCD".
        let out = run_cobol(&wrap(
            &[
                "01  WS  PIC X(5) VALUE \"ABCDE\".",
                "01  J   PIC 9 VALUE 2.",
                "01  K   PIC 9 VALUE 3.",
            ],
            &["DISPLAY WS(J:K).", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(out, "BCD\n");
    }

    #[test]
    fn refmod_computed_omitted_length_runs_to_end() {
        // WS(J:) with J=3 over "ABCDE" runs to the end → "CDE".
        let out = run_cobol(&wrap(
            &["01  WS  PIC X(5) VALUE \"ABCDE\".", "01  J   PIC 9 VALUE 3."],
            &["DISPLAY WS(J:).", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(out, "CDE\n");
    }

    #[test]
    fn refmod_computed_out_of_range_traps() {
        // WS(J:K) with J=4, K=5 over a 5-char item runs to position 8 > 5 → a
        // well-defined RefModOutOfRange trap (never a wrong slice or a panic).
        let err = run_cobol(&wrap(
            &[
                "01  WS  PIC X(5) VALUE \"ABCDE\".",
                "01  J   PIC 9 VALUE 4.",
                "01  K   PIC 9 VALUE 5.",
            ],
            &["DISPLAY WS(J:K).", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::RefModOutOfRange(_)), "got {err:?}");
    }

    #[test]
    fn refmod_computed_zero_start_traps() {
        // A start of 0 makes start0 = -1 < 0 → an out-of-range trap, matching the
        // compiled str_slice's `start < 0` bounds check.
        let err = run_cobol(&wrap(
            &["01  WS  PIC X(5) VALUE \"ABCDE\".", "01  J   PIC 9 VALUE 0."],
            &["DISPLAY WS(J:2).", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::RefModOutOfRange(_)), "got {err:?}");
    }

    #[test]
    fn refmod_of_numeric_item_is_a_later_rung() {
        // Reference modification is defined on alphanumeric items; a numeric base
        // is a later rung — the same reject the compiler makes.
        let err = run_cobol(&wrap(
            &["01  N  PIC 9(5) VALUE 12345.", "01  J  PIC 9 VALUE 2."],
            &["DISPLAY N(J:2).", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn refmod_signed_index_item_is_a_later_rung() {
        // A signed index item is a later rung (the index model is unsigned integer).
        let err = run_cobol(&wrap(
            &["01  WS  PIC X(5) VALUE \"ABCDE\".", "01  J  PIC S9 VALUE 2."],
            &["DISPLAY WS(J:2).", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn refmod_move_source_into_alnum_receiver() {
        // A reference-modification MOVE source into an ALPHANUMERIC receiver is now
        // supported (this rung), including a computed (data-name) index: the slice
        // is char-fit to the receiver width. WS(J:2) with J=2 → "BC" into DST X(3)
        // → left-justified, one trailing space.
        let out = run_cobol(&wrap(
            &[
                "01  WS  PIC X(5) VALUE \"ABCDE\".",
                "01  J   PIC 9 VALUE 2.",
                "01  DST PIC X(3).",
            ],
            &["MOVE WS(J:2) TO DST.", "DISPLAY DST \"|\".", "STOP RUN."],
        ))
        .unwrap();
        assert_eq!(out, "BC |\n");
    }

    #[test]
    fn refmod_move_source_into_numeric_receiver_is_a_later_rung() {
        // The remaining boundary: a refmod MOVE source into a NUMERIC receiver stays
        // a later rung (de-editing a slice into a numeric field is not lowered here).
        let err = run_cobol(&wrap(
            &["01  WS  PIC X(5) VALUE \"12345\".", "01  NUM PIC 9(3)."],
            &["MOVE WS(1:3) TO NUM.", "STOP RUN."],
        ))
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Unsupported(_)), "got {err:?}");
    }
}
