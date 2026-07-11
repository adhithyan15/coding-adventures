//! # COBOL-60 parser — the syntactic layer of the four-division program.
//!
//! COBOL-60 (CODASYL, the 1960 report) is FLOW-MATIC's direct descendant. This
//! crate is the **parser** half of its frontend: it tokenizes with
//! [`coding_adventures_cobol_lexer`] — which strips the 80-column card format
//! and lexes PICTURE strings — and feeds the tokens to the generic
//! [`GrammarParser`] driving the compiled `cobol.grammar`.
//!
//! ```text
//! COBOL-60 source (carded)
//!    │  cobol_lexer::tokenize_cobol   (column-strip hook + tokenize)
//!    ▼
//! Vec<Token>
//!    │  parser::GrammarParser (cobol.grammar → CST)
//!    ▼
//! GrammarASTNode  { rule_name, children }   (root rule_name == "program")
//! ```
//!
//! The tree is the generic, uniform [`GrammarASTNode`]; a consumer walks it by
//! `rule_name`. Nothing is hand-written — the grammar file is the single source
//! of truth.
//!
//! ## What it parses
//!
//! The demonstrated language of [PL07](../../../specs/PL07-cobol-60.md): a
//! four-division program (IDENTIFICATION and PROCEDURE required; ENVIRONMENT and
//! DATA optional), `WORKING-STORAGE` data entries (level numbers, `PICTURE`,
//! `VALUE`), and PROCEDURE paragraphs of sentences over the core verbs.
//!
//! ## Public API
//!
//! - [`create_cobol_parser`] — a configured [`GrammarParser`], ready to `.parse()`.
//! - [`parse_cobol`] — convenience `&str` → [`GrammarASTNode`] (panics on error).
//! - [`try_parse_cobol`] — the fully fallible form (lexical *and* parse errors → `Err`).

use coding_adventures_cobol_lexer::{tokenize_cobol, try_tokenize_cobol};
use parser::grammar_parser::{GrammarASTNode, GrammarParser, DEFAULT_MAX_RULE_DEPTH};

mod _grammar;

/// Create a [`GrammarParser`] wired to the COBOL grammar and tokens (with the
/// lexer's column-strip hook), ready to call `.parse()`. Uses the panicking
/// tokenizer; for the fully fallible path use [`try_parse_cobol`].
///
/// The parser opts into the shared recursion-depth cap
/// ([`DEFAULT_MAX_RULE_DEPTH`]). Deeply-nested syntax — e.g. hundreds of nested
/// `IF … IF … IF …` — recurses once per level through `parse_rule`; without a
/// cap that overflows the *native* stack, an uncatchable process abort. With
/// the cap it surfaces as a recoverable [`GrammarParseError`] instead.
pub fn create_cobol_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_cobol(source);
    GrammarParser::new(tokens, _grammar::parser_grammar()).with_max_depth(DEFAULT_MAX_RULE_DEPTH)
}

/// Parse COBOL-60 `source` into a [`GrammarASTNode`] CST rooted at `"program"`.
/// Panics on a parse error; use [`try_parse_cobol`] for the fallible form.
pub fn parse_cobol(source: &str) -> GrammarASTNode {
    try_parse_cobol(source).unwrap_or_else(|e| panic!("COBOL parse failed: {e}"))
}

/// Parse COBOL-60 `source`, returning a human-readable error string on failure.
/// The truly fallible path — a *lexical* error becomes an `Err` too (it routes
/// through [`try_tokenize_cobol`], not the panicking tokenizer that
/// [`create_cobol_parser`] uses).
pub fn try_parse_cobol(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = try_tokenize_cobol(source)?;
    GrammarParser::new(tokens, _grammar::parser_grammar())
        .with_max_depth(DEFAULT_MAX_RULE_DEPTH)
        .parse()
        .map_err(|e| format!("{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    fn root(src: &str) -> GrammarASTNode {
        parse_cobol(src)
    }

    fn has_rule(node: &GrammarASTNode, target: &str) -> bool {
        if node.rule_name == target {
            return true;
        }
        node.children.iter().any(|c| match c {
            ASTNodeOrToken::Node(n) => has_rule(n, target),
            ASTNodeOrToken::Token(_) => false,
        })
    }

    fn count_rule(node: &GrammarASTNode, target: &str) -> usize {
        let here = usize::from(node.rule_name == target);
        here + node
            .children
            .iter()
            .map(|c| match c {
                ASTNodeOrToken::Node(n) => count_rule(n, target),
                ASTNodeOrToken::Token(_) => 0,
            })
            .sum::<usize>()
    }

    /// A one-line card (6 sequence cols + 1 space indicator, then code).
    fn card(code: &str) -> String {
        format!("000000 {code}")
    }

    /// Assemble a program from code lines, carding each one.
    fn program(lines: &[&str]) -> String {
        lines.iter().map(|l| card(l)).collect::<Vec<_>>().join("\n")
    }

    // ----------------------------------------------------------------------
    // Smallest legal program: IDENTIFICATION + PROCEDURE
    // ----------------------------------------------------------------------

    #[test]
    fn minimal_program() {
        let ast = root(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. HELLO.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    STOP RUN.",
        ]));
        assert_eq!(ast.rule_name, "program");
        assert!(has_rule(&ast, "identification_division"));
        assert!(has_rule(&ast, "procedure_division"));
        assert!(has_rule(&ast, "stop_stmt"));
        // No optional divisions were invented.
        assert!(!has_rule(&ast, "environment_division"));
        assert!(!has_rule(&ast, "data_division"));
    }

    // ----------------------------------------------------------------------
    // DATA DIVISION: level numbers, PICTURE, VALUE
    // ----------------------------------------------------------------------

    #[test]
    fn working_storage_entries() {
        let ast = root(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. PAY.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  EMPLOYEE-RECORD.",
            "    02  EMP-NAME  PICTURE X(20).",
            "    02  EMP-RATE  PIC 9(3)V99.",
            "77  GROSS-PAY  PIC 9(6)V99 VALUE ZERO.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    STOP RUN.",
        ]));
        assert!(has_rule(&ast, "working_storage_section"));
        // Four data entries: the 01 record, two 02 items, and the 77 item.
        assert_eq!(count_rule(&ast, "data_entry"), 4);
        // Two PICTURE clauses carry PIC_STRINGs; the 01 group item has none.
        assert_eq!(count_rule(&ast, "picture_clause"), 3);
        assert!(has_rule(&ast, "value_clause"));
    }

    #[test]
    fn filler_and_value_is() {
        let ast = root(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "DATA DIVISION.",
            "WORKING-STORAGE SECTION.",
            "01  FILLER  PIC X VALUE IS SPACE.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    STOP RUN.",
        ]));
        assert_eq!(count_rule(&ast, "data_entry"), 1);
        assert!(has_rule(&ast, "value_clause"));
    }

    // ----------------------------------------------------------------------
    // PROCEDURE DIVISION: paragraphs, sentences, verbs
    // ----------------------------------------------------------------------

    #[test]
    fn procedure_paragraphs_and_verbs() {
        let ast = root(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "PROCEDURE DIVISION.",
            "MAIN-PARAGRAPH.",
            "    MOVE ZERO TO GROSS-PAY.",
            "    MULTIPLY EMP-RATE BY EMP-HOURS GIVING GROSS-PAY.",
            "    DISPLAY EMP-NAME GROSS-PAY.",
            "    PERFORM SUB-ROUTINE.",
            "    GO TO MAIN-PARAGRAPH.",
            "SUB-ROUTINE.",
            "    STOP RUN.",
        ]));
        assert_eq!(count_rule(&ast, "paragraph"), 2);
        assert!(has_rule(&ast, "move_stmt"));
        assert!(has_rule(&ast, "multiply_stmt"));
        assert!(has_rule(&ast, "display_stmt"));
        assert!(has_rule(&ast, "perform_stmt"));
        assert!(has_rule(&ast, "goto_stmt"));
        // DISPLAY takes multiple operands.
        assert!(has_rule(&ast, "display_stmt"));
    }

    #[test]
    fn if_else_statement() {
        let ast = root(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    IF GROSS-PAY IS GREATER THAN ZERO",
            "        DISPLAY GROSS-PAY",
            "    ELSE",
            "        DISPLAY ZERO.",
            "    STOP RUN.",
        ]));
        assert!(has_rule(&ast, "if_stmt"));
        assert!(has_rule(&ast, "condition"));
        // Two DISPLAYs: one in the then-branch, one in the else-branch.
        assert_eq!(count_rule(&ast, "display_stmt"), 2);
    }

    // ----------------------------------------------------------------------
    // COMPUTE and arithmetic expressions
    // ----------------------------------------------------------------------

    #[test]
    fn compute_with_operator_precedence() {
        // `A + B * C ** D` must nest so that ** binds tightest, then *, then +.
        let ast = root(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    COMPUTE RESULT = A + B * C ** D.",
            "    STOP RUN.",
        ]));
        assert!(has_rule(&ast, "compute_stmt"));
        // One additive expr at the top; the multiplicative and exponent layers
        // each appear once, proving the precedence cascade was built.
        assert_eq!(count_rule(&ast, "arith_expr"), 1);
        assert_eq!(count_rule(&ast, "arith_term"), 2); // A, and (B * C ** D)
        assert_eq!(count_rule(&ast, "arith_factor"), 3); // A, B, (C ** D)
    }

    #[test]
    fn compute_parentheses_regroup_precedence() {
        // Parentheses force the addition to evaluate first: a nested arith_expr.
        let ast = root(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    COMPUTE X = (A + B) * C.",
            "    STOP RUN.",
        ]));
        assert!(has_rule(&ast, "compute_stmt"));
        // The parenthesised `A + B` is a second, nested arith_expr.
        assert_eq!(count_rule(&ast, "arith_expr"), 2);
    }

    #[test]
    fn compute_rounded_and_on_size_error() {
        // ROUNDED and the ON SIZE ERROR clause both parse; the clause carries a
        // statement to run on overflow.
        let ast = root(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            // Split across cards so no line exceeds the 72-column code area
            // (a COBOL statement flows freely from one card to the next).
            "    COMPUTE TOTAL ROUNDED = PRICE * QTY",
            "        ON SIZE ERROR DISPLAY \"OVR\".",
            "    STOP RUN.",
        ]));
        assert!(has_rule(&ast, "compute_stmt"));
        assert!(has_rule(&ast, "size_error"));
        // The overflow handler is an ordinary statement (a DISPLAY here).
        assert!(has_rule(&ast, "display_stmt"));
    }

    #[test]
    fn compute_negative_literal_vs_subtraction() {
        // `A - 3` (spaced) is subtraction; `-3` (unspaced) would be a negative
        // literal. Here we exercise the binary minus.
        let ast = root(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    COMPUTE D = A - 3.",
            "    STOP RUN.",
        ]));
        assert!(has_rule(&ast, "compute_stmt"));
        assert_eq!(count_rule(&ast, "arith_term"), 2); // A and 3, split by MINUS
    }

    // ----------------------------------------------------------------------
    // ENVIRONMENT DIVISION (optional, minimal)
    // ----------------------------------------------------------------------

    #[test]
    fn environment_configuration_section() {
        let ast = root(&program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "ENVIRONMENT DIVISION.",
            "CONFIGURATION SECTION.",
            "SOURCE-COMPUTER. UNIVAC-II.",
            "OBJECT-COMPUTER. UNIVAC-II.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    STOP RUN.",
        ]));
        assert!(has_rule(&ast, "environment_division"));
        assert!(has_rule(&ast, "configuration_section"));
        assert_eq!(count_rule(&ast, "config_paragraph"), 2);
    }

    // ----------------------------------------------------------------------
    // The whole demonstrated program, all together
    // ----------------------------------------------------------------------

    #[test]
    fn full_four_division_program() {
        let src = "\
000100 IDENTIFICATION DIVISION.
000200 PROGRAM-ID. PAYROLL.
000300 AUTHOR. GRACE HOPPER.
000400 ENVIRONMENT DIVISION.
000500 CONFIGURATION SECTION.
000600 SOURCE-COMPUTER. UNIVAC-II.
000700 OBJECT-COMPUTER. UNIVAC-II.
000800 DATA DIVISION.
000900 WORKING-STORAGE SECTION.
001000 01  EMP-NAME     PICTURE X(20).
001100 77  GROSS-PAY    PIC 9(6)V99 VALUE ZERO.
001200 PROCEDURE DIVISION.
001300 MAIN-PARAGRAPH.
001400     MOVE ZERO TO GROSS-PAY.
001500     DISPLAY EMP-NAME GROSS-PAY.
001600     STOP RUN.";
        let ast = root(src);
        assert_eq!(ast.rule_name, "program");
        // All four divisions present.
        for rule in [
            "identification_division", "environment_division",
            "data_division", "procedure_division",
        ] {
            assert!(has_rule(&ast, rule), "missing {rule}");
        }
        // The commentary AUTHOR paragraph parsed.
        assert!(has_rule(&ast, "id_paragraph"));
    }

    // ----------------------------------------------------------------------
    // Error paths
    // ----------------------------------------------------------------------

    /// A program with no PROCEDURE DIVISION is incomplete → parse error.
    #[test]
    fn missing_procedure_division_is_error() {
        let src = program(&["IDENTIFICATION DIVISION.", "PROGRAM-ID. P."]);
        assert!(try_parse_cobol(&src).is_err());
    }

    /// Deeply-nested `IF … IF … IF …` recurses once per level through the
    /// generic `parse_rule`. Without the depth cap this overflows the native
    /// stack — an uncatchable process abort. With [`DEFAULT_MAX_RULE_DEPTH`]
    /// (opted into by [`create_cobol_parser`] / [`try_parse_cobol`]) it must
    /// come back as a recoverable `Err`. One `IF` per card so the fixed
    /// 80-column format doesn't truncate them; a statement flows freely across
    /// cards, so the nest is genuinely deep. 4096 levels is far past the cap.
    #[test]
    fn deeply_nested_if_is_a_clean_error_not_a_stack_overflow() {
        let mut lines: Vec<String> = vec![
            "IDENTIFICATION DIVISION.".into(),
            "PROGRAM-ID. P.".into(),
            "PROCEDURE DIVISION.".into(),
            "MAIN.".into(),
        ];
        for _ in 0..4096 {
            lines.push("    IF GROSS-PAY IS GREATER THAN ZERO".into());
        }
        lines.push("        DISPLAY ZERO.".into());
        lines.push("    STOP RUN.".into());
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        let err = try_parse_cobol(&program(&refs)).unwrap_err();
        assert!(err.to_lowercase().contains("nest") || err.to_lowercase().contains("depth"),
            "depth refusal should be self-explanatory, got: {err}");
    }

    /// A lexical error (stray `@` in the code area) surfaces as an `Err`.
    #[test]
    fn lexical_error_is_reported() {
        let src = program(&[
            "IDENTIFICATION DIVISION.",
            "PROGRAM-ID. P.",
            "PROCEDURE DIVISION.",
            "MAIN.",
            "    MOVE @ TO X.",
        ]);
        assert!(try_parse_cobol(&src).is_err());
    }
}
