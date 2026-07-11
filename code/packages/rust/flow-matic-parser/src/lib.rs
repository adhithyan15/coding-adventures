//! # FLOW-MATIC parser — the syntactic layer of the first English-like language.
//!
//! [FLOW-MATIC](https://en.wikipedia.org/wiki/FLOW-MATIC) (B-0, 1955–1959, Grace
//! Hopper / UNIVAC I) is the direct ancestor of COBOL. This crate is the
//! **parser** half of its frontend: it tokenizes with
//! [`coding_adventures_flow_matic_lexer`] and feeds the tokens to the generic
//! [`GrammarParser`] driving the compiled `flow_matic.grammar`.
//!
//! ```text
//! FLOW-MATIC source
//!    │  flow_matic_lexer::tokenize_flow_matic
//!    ▼
//! Vec<Token>
//!    │  parser::GrammarParser (flow_matic.grammar → CST)
//!    ▼
//! GrammarASTNode  { rule_name, children }   (root rule_name == "program")
//! ```
//!
//! The tree is the generic, uniform [`GrammarASTNode`]; a consumer walks it by
//! `rule_name`. Nothing is hand-written — the grammar file is the single source
//! of truth, shared across every language's parser wrapper.
//!
//! ## What it parses
//!
//! The **demonstrated language** of the canonical inventory-pricing program:
//! numbered operations of `;`-separated clauses ended by `.`, file description
//! (`INPUT`/`OUTPUT`/`HSP`), `COMPARE … WITH …` with the three-way
//! `IF`/`OTHERWISE` branch, data movement (`TRANSFER`/`MOVE`), control (`JUMP`),
//! record I/O (`READ-ITEM`/`WRITE-ITEM`), the sentinel `TEST … AGAINST …`,
//! `REWIND`, `CLOSE-OUT FILES`, `STOP`, and the trailing `(END)` marker.
//!
//! ## Public API
//!
//! - [`create_flow_matic_parser`] — a configured [`GrammarParser`], ready to `.parse()`.
//! - [`parse_flow_matic`] — convenience `&str` → [`GrammarASTNode`] (panics on error).
//! - [`try_parse_flow_matic`] — the fully fallible form (lexical *and* parse errors → `Err`).

use coding_adventures_flow_matic_lexer::{tokenize_flow_matic, try_tokenize_flow_matic};
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Create a [`GrammarParser`] wired to the FLOW-MATIC grammar and tokens, ready
/// to call `.parse()`. Uses the panicking tokenizer; for the fully fallible
/// path use [`try_parse_flow_matic`].
pub fn create_flow_matic_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_flow_matic(source);
    GrammarParser::new(tokens, _grammar::parser_grammar())
}

/// Parse FLOW-MATIC `source` into a [`GrammarASTNode`] CST rooted at
/// `"program"`. Panics on a parse error; use [`try_parse_flow_matic`] for the
/// fallible form.
pub fn parse_flow_matic(source: &str) -> GrammarASTNode {
    try_parse_flow_matic(source).unwrap_or_else(|e| panic!("FLOW-MATIC parse failed: {e}"))
}

/// Parse FLOW-MATIC `source`, returning a human-readable error string on
/// failure. The truly fallible path — a *lexical* error becomes an `Err` too
/// (it routes through [`try_tokenize_flow_matic`], not the panicking tokenizer
/// that [`create_flow_matic_parser`] uses).
pub fn try_parse_flow_matic(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = try_tokenize_flow_matic(source)?;
    GrammarParser::new(tokens, _grammar::parser_grammar())
        .parse()
        .map_err(|e| format!("{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    fn root(src: &str) -> GrammarASTNode {
        parse_flow_matic(src)
    }

    /// Does the tree contain a node with this `rule_name` anywhere?
    fn has_rule(node: &GrammarASTNode, target: &str) -> bool {
        if node.rule_name == target {
            return true;
        }
        node.children.iter().any(|c| match c {
            ASTNodeOrToken::Node(n) => has_rule(n, target),
            ASTNodeOrToken::Token(_) => false,
        })
    }

    /// How many nodes with this `rule_name` does the tree contain?
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

    // ----------------------------------------------------------------------
    // Basic shape
    // ----------------------------------------------------------------------

    /// A single operation parses to a `program` root containing one `statement`.
    #[test]
    fn single_statement_program() {
        let ast = root("(2) TRANSFER A TO D .");
        assert_eq!(ast.rule_name, "program");
        assert_eq!(count_rule(&ast, "statement"), 1);
        assert!(has_rule(&ast, "transfer_clause"));
    }

    // ----------------------------------------------------------------------
    // Operation (0): file description
    // ----------------------------------------------------------------------

    /// The full operation (0): INPUT/OUTPUT file pairs plus HSP, three clauses
    /// separated by `;` under one statement.
    #[test]
    fn file_description_operation() {
        let ast = root(
            "(0) INPUT INVENTORY FILE-A PRICE FILE-B ; \
                 OUTPUT PRICED-INV FILE-C UNPRICED-INV FILE-D ; HSP D .",
        );
        assert!(has_rule(&ast, "input_clause"));
        assert!(has_rule(&ast, "output_clause"));
        assert!(has_rule(&ast, "hsp_clause"));
        // Two file pairs on INPUT, two on OUTPUT.
        assert_eq!(count_rule(&ast, "file_pair"), 4);
    }

    // ----------------------------------------------------------------------
    // Operation (1): compare + the three-way branch in one statement
    // ----------------------------------------------------------------------

    /// COMPARE with qualified fields, then three branch clauses (`IF GREATER`,
    /// `IF EQUAL`, `OTHERWISE`) — all one statement, `;`-separated.
    #[test]
    fn compare_and_three_way_branch() {
        let ast = root(
            "(1) COMPARE PRODUCT-NO (A) WITH PRODUCT-NO (B) ; \
                 IF GREATER GO TO OPERATION 10 ; \
                 IF EQUAL GO TO OPERATION 5 ; \
                 OTHERWISE GO TO OPERATION 2 .",
        );
        assert_eq!(count_rule(&ast, "statement"), 1, "all one operation");
        assert!(has_rule(&ast, "compare_clause"));
        // Two qualified fields in the COMPARE.
        assert_eq!(count_rule(&ast, "field"), 2);
        // Two IF clauses + one OTHERWISE.
        assert_eq!(count_rule(&ast, "if_clause"), 2);
        assert_eq!(count_rule(&ast, "otherwise_clause"), 1);
    }

    /// The `END OF DATA` condition (operation 8's end-of-file test) parses as a
    /// sibling clause after `READ-ITEM A ;`.
    #[test]
    fn read_item_then_end_of_data_branch() {
        let ast = root("(8) READ-ITEM A ; IF END OF DATA GO TO OPERATION 14 .");
        assert!(has_rule(&ast, "read_item_clause"));
        assert!(has_rule(&ast, "if_clause"));
        assert!(has_rule(&ast, "condition"));
    }

    // ----------------------------------------------------------------------
    // Data movement, control, I/O
    // ----------------------------------------------------------------------

    /// MOVE operates on qualified fields; JUMP takes an OPERATION target.
    #[test]
    fn move_and_jump() {
        let ast = root("(6) MOVE UNIT-PRICE (B) TO UNIT-PRICE (C) . (4) JUMP TO OPERATION 8 .");
        assert!(has_rule(&ast, "move_clause"));
        assert!(has_rule(&ast, "jump_clause"));
        assert_eq!(count_rule(&ast, "target"), 1);
    }

    // ----------------------------------------------------------------------
    // The CLOSE-OUT ; overlap
    // ----------------------------------------------------------------------

    /// `CLOSE-OUT FILES C ; D` — the `;` separates file names *inside* the
    /// clause. PEG greediness keeps both names under one `closeout_clause`, so
    /// the statement has exactly one clause, not two.
    #[test]
    fn closeout_semicolon_separates_file_names() {
        let ast = root("(16) CLOSE-OUT FILES C ; D .");
        assert_eq!(count_rule(&ast, "statement"), 1);
        assert_eq!(count_rule(&ast, "closeout_clause"), 1);
        // Crucially, the inner `; D` did NOT spawn a second clause of some other
        // kind — no stray if/transfer/etc. Only the one closeout clause exists.
        assert!(!has_rule(&ast, "if_clause"));
    }

    // ----------------------------------------------------------------------
    // Sentinel test + the whole canonical program
    // ----------------------------------------------------------------------

    /// `TEST … AGAINST ZZZ…` against the all-Z high-values sentinel (a NAME).
    #[test]
    fn test_against_sentinel() {
        let ast = root("(14) TEST PRODUCT-NO (B) AGAINST ZZZZZZZZZZZZ .");
        assert!(has_rule(&ast, "test_clause"));
    }

    /// The complete canonical inventory-pricing program — every operation
    /// `(0)`–`(17)` plus the trailing `(END)` marker — parses end to end.
    #[test]
    fn full_canonical_program() {
        let src = "\
(0)  INPUT INVENTORY FILE-A PRICE FILE-B ; OUTPUT PRICED-INV FILE-C
       UNPRICED-INV FILE-D ; HSP D .
(1)  COMPARE PRODUCT-NO (A) WITH PRODUCT-NO (B) ;
       IF GREATER GO TO OPERATION 10 ;
       IF EQUAL GO TO OPERATION 5 ;
       OTHERWISE GO TO OPERATION 2 .
(2)  TRANSFER A TO D .
(3)  WRITE-ITEM D .
(4)  JUMP TO OPERATION 8 .
(5)  TRANSFER A TO C .
(6)  MOVE UNIT-PRICE (B) TO UNIT-PRICE (C) .
(7)  WRITE-ITEM C .
(8)  READ-ITEM A ; IF END OF DATA GO TO OPERATION 14 .
(9)  JUMP TO OPERATION 1 .
(10) TRANSFER B TO D .
(11) WRITE-ITEM D .
(12) READ-ITEM B ; IF END OF DATA GO TO OPERATION 12 .
(13) JUMP TO OPERATION 1 .
(14) TEST PRODUCT-NO (B) AGAINST ZZZZZZZZZZZZ ;
       IF EQUAL GO TO OPERATION 16 ; OTHERWISE GO TO OPERATION 15 .
(15) REWIND B .
(16) CLOSE-OUT FILES C ; D .
(17) STOP . (END)";
        let ast = root(src);
        assert_eq!(ast.rule_name, "program");
        // 18 operations, (0) through (17).
        assert_eq!(count_rule(&ast, "statement"), 18);
        // The trailing (END) marker parsed.
        assert!(has_rule(&ast, "program_end"));
        // Spot-check a representative clause of each major kind is present.
        for rule in [
            "input_clause", "output_clause", "hsp_clause", "compare_clause",
            "if_clause", "otherwise_clause", "transfer_clause", "move_clause",
            "jump_clause", "read_item_clause", "write_item_clause",
            "test_clause", "rewind_clause", "closeout_clause", "stop_clause",
        ] {
            assert!(has_rule(&ast, rule), "missing {rule} in full program");
        }
    }

    // ----------------------------------------------------------------------
    // Error paths
    // ----------------------------------------------------------------------

    /// A missing terminating period is a parse error (not a panic via `try_`).
    #[test]
    fn missing_period_is_parse_error() {
        assert!(try_parse_flow_matic("(2) TRANSFER A TO D").is_err());
    }

    /// A lexical error (stray `@`) surfaces as an `Err` through the parser's
    /// fully-fallible path, not a panic.
    #[test]
    fn lexical_error_is_reported() {
        assert!(try_parse_flow_matic("(0) @ .").is_err());
    }
}
