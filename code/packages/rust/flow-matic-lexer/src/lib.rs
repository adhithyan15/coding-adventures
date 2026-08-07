//! # FLOW-MATIC lexer — tokenizing the first English-like programming language.
//!
//! [FLOW-MATIC](https://en.wikipedia.org/wiki/FLOW-MATIC) — originally **B-0**
//! ("Business Language version 0") — was designed by Grace Hopper and her team
//! at Remington Rand between roughly 1955 and 1959, and ran on the UNIVAC I, the
//! first commercial computer sold in the United States. It was the first
//! programming language to let people write data-processing logic in English
//! verbs rather than numeric machine code, and it is the direct ancestor of
//! COBOL: its hyphenated data names, its English keywords, and its separation of
//! file/data description from procedure all flowed into the 1959 COBOL design.
//!
//! A FLOW-MATIC program is a list of numbered *operations*. Each operation is a
//! parenthesised operation number, then one or more clauses separated by `;`,
//! terminated by a period `.`:
//!
//! ```text
//! (0)  INPUT INVENTORY FILE-A PRICE FILE-B ; OUTPUT PRICED-INV FILE-C
//!        UNPRICED-INV FILE-D ; HSP D .
//! (1)  COMPARE PRODUCT-NO (A) WITH PRODUCT-NO (B) ;
//!        IF GREATER GO TO OPERATION 10 ;
//!        IF EQUAL GO TO OPERATION 5 ;
//!        OTHERWISE GO TO OPERATION 2 .
//! ```
//!
//! # Architecture
//!
//! Like every language frontend in this repo, this crate is a **thin wrapper**
//! around the generic [`GrammarLexer`]. It does not hand-write tokenization: it
//! loads the compiled `flow_matic.tokens` grammar and hands it to the engine.
//!
//! ```text
//! flow_matic.tokens   (grammar file — one source of truth for every language)
//!        │  grammar-tools  (parses .tokens → TokenGrammar, embedded in _grammar.rs)
//!        ▼
//! lexer::GrammarLexer (tokenizes the source using the TokenGrammar)
//!        ▼
//! flow-matic-lexer    (this crate — just picks token vs error out of the result)
//! ```
//!
//! # No hooks required
//!
//! FLOW-MATIC needs **no** pre/post-tokenize hooks — it is the simplest kind of
//! frontend. Two properties make that possible:
//!
//! - **Free-form layout.** Unlike COBOL's fixed 80-column punched-card format
//!   (which needs a `pre_tokenize` column-strip hook), FLOW-MATIC listings
//!   separate tokens with whitespace and end each operation with a period, so
//!   newlines carry no meaning and are simply skipped.
//! - **Grammar-owned disambiguation.** The operation label `(0)` and a field
//!   qualifier `(A)` are both just `LPAREN … RPAREN` here; the parser — not the
//!   lexer — decides which is which by whether a `NUMBER` or a `NAME` sits
//!   inside. So no custom `OP_NUMBER` token and no relabelling hook are needed.
//!
//! # Public API
//!
//! - [`create_flow_matic_lexer`] — a configured [`GrammarLexer`] for fine control.
//! - [`tokenize_flow_matic`] — convenience `&str` → `Vec<Token>` (panics on error).
//! - [`try_tokenize_flow_matic`] — the fallible form returning `Result`.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

/// Create a [`GrammarLexer`] configured for FLOW-MATIC source text.
///
/// The returned lexer is ready to call `.tokenize()` on. Use this when you want
/// the `GrammarLexer` object itself; most callers want [`tokenize_flow_matic`].
pub fn create_flow_matic_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

/// Tokenize FLOW-MATIC `source` into a `Vec<Token>` (ending in EOF).
///
/// Panics on a lexical error; use [`try_tokenize_flow_matic`] for the fallible
/// form.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_flow_matic_lexer::tokenize_flow_matic;
///
/// let tokens = tokenize_flow_matic("(2) TRANSFER A TO D .");
/// for t in &tokens {
///     println!("{} {:?}", t.effective_type_name(), t.value);
/// }
/// ```
pub fn tokenize_flow_matic(source: &str) -> Vec<Token> {
    try_tokenize_flow_matic(source)
        .unwrap_or_else(|e| panic!("FLOW-MATIC tokenization failed: {e}"))
}

/// Tokenize FLOW-MATIC `source`, returning a human-readable error string on
/// failure instead of panicking.
pub fn try_tokenize_flow_matic(source: &str) -> Result<Vec<Token>, String> {
    create_flow_matic_lexer(source)
        .tokenize()
        .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    /// (effective_type_name, value) for every non-EOF token — compact asserts.
    fn pairs(src: &str) -> Vec<(String, String)> {
        tokenize_flow_matic(src)
            .into_iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.effective_type_name().to_string(), t.value.clone()))
            .collect()
    }

    /// Just the effective type-names, for structure-only assertions.
    fn kinds(src: &str) -> Vec<String> {
        pairs(src).into_iter().map(|(k, _)| k).collect()
    }

    // ----------------------------------------------------------------------
    // Data names: the COBOL-family hyphenated identifier
    // ----------------------------------------------------------------------

    /// `PRODUCT-NO` is ONE name, not `PRODUCT` minus `NO`. Case is insignificant
    /// for matching, but a NAME's value preserves the source's original case —
    /// so canonical uppercase FLOW-MATIC keeps `PRODUCT-NO` intact.
    #[test]
    fn hyphenated_name_is_one_token() {
        assert_eq!(pairs("PRODUCT-NO"), vec![("NAME".into(), "PRODUCT-NO".into())]);
        assert_eq!(pairs("UNPRICED-INV"), vec![("NAME".into(), "UNPRICED-INV".into())]);
    }

    /// A single letter (a file handle like `A`) and a file name like `FILE-A`
    /// are both NAMEs, value preserved as typed.
    #[test]
    fn single_letter_and_file_names_are_names() {
        assert_eq!(kinds("A B FILE-A FILE-B"), vec!["NAME"; 4]);
        assert_eq!(pairs("FILE-A")[0].1, "FILE-A");
    }

    /// Case really is insignificant for matching: a lowercased data name still
    /// lexes as a NAME (its value just preserves the lowercase it was typed in).
    #[test]
    fn lowercase_name_still_matches() {
        assert_eq!(pairs("product-no"), vec![("NAME".into(), "product-no".into())]);
    }

    // ----------------------------------------------------------------------
    // Keyword promotion, including the hyphenated verbs
    // ----------------------------------------------------------------------

    /// English verbs surface as uppercase KEYWORD tokens regardless of the case
    /// they were typed in (keyword promotion normalizes their value).
    #[test]
    fn verbs_are_uppercase_keywords() {
        assert_eq!(pairs("compare"), vec![("KEYWORD".into(), "COMPARE".into())]);
        assert_eq!(pairs("Transfer"), vec![("KEYWORD".into(), "TRANSFER".into())]);
        assert_eq!(pairs("COMPARE"), vec![("KEYWORD".into(), "COMPARE".into())]);
    }

    /// The hyphenated verbs are keywords (via keyword promotion), not plain
    /// names — this is the subtle case, since they match the NAME pattern first.
    #[test]
    fn hyphenated_verbs_promote_to_keywords() {
        assert_eq!(pairs("WRITE-ITEM"), vec![("KEYWORD".into(), "WRITE-ITEM".into())]);
        assert_eq!(pairs("READ-ITEM"), vec![("KEYWORD".into(), "READ-ITEM".into())]);
        assert_eq!(pairs("CLOSE-OUT"), vec![("KEYWORD".into(), "CLOSE-OUT".into())]);
    }

    /// A word that merely *starts* like a keyword stays a NAME — no accidental
    /// splitting of `INVENTORY` into `IN` + `VENTORY`.
    #[test]
    fn keyword_prefix_word_stays_a_name() {
        assert_eq!(pairs("INVENTORY"), vec![("NAME".into(), "INVENTORY".into())]);
    }

    // ----------------------------------------------------------------------
    // Punctuation and the label-vs-qualifier parens
    // ----------------------------------------------------------------------

    /// An operation label `(0)` lexes as `( NUMBER )` — the parser, not the
    /// lexer, knows it is a label.
    #[test]
    fn operation_label_is_paren_number_paren() {
        assert_eq!(
            pairs("(0)"),
            vec![
                ("LPAREN".into(), "(".into()),
                ("NUMBER".into(), "0".into()),
                ("RPAREN".into(), ")".into()),
            ]
        );
    }

    /// A field qualifier `(A)` lexes as `( NAME )` — same shape, NAME inside.
    /// This is exactly how the two are told apart downstream.
    #[test]
    fn field_qualifier_is_paren_name_paren() {
        assert_eq!(
            kinds("(A)"),
            vec!["LPAREN", "NAME", "RPAREN"]
        );
    }

    /// Period ends an operation; semicolon separates clauses.
    #[test]
    fn period_and_semicolon() {
        assert_eq!(kinds(". ;"), vec!["PERIOD", "SEMICOLON"]);
    }

    // ----------------------------------------------------------------------
    // Whole clauses from the canonical program
    // ----------------------------------------------------------------------

    /// `COMPARE PRODUCT-NO (A) WITH PRODUCT-NO (B)` — operation (1)'s first
    /// clause, the heart of the inventory-pricing example.
    #[test]
    fn compare_clause() {
        assert_eq!(
            pairs("COMPARE PRODUCT-NO (A) WITH PRODUCT-NO (B)"),
            vec![
                ("KEYWORD".into(), "COMPARE".into()),
                ("NAME".into(), "PRODUCT-NO".into()),
                ("LPAREN".into(), "(".into()),
                ("NAME".into(), "A".into()),
                ("RPAREN".into(), ")".into()),
                ("KEYWORD".into(), "WITH".into()),
                ("NAME".into(), "PRODUCT-NO".into()),
                ("LPAREN".into(), "(".into()),
                ("NAME".into(), "B".into()),
                ("RPAREN".into(), ")".into()),
            ]
        );
    }

    /// `IF GREATER GO TO OPERATION 10` — the three-way branch word plus the
    /// two-word `GO TO` and a numeric target.
    #[test]
    fn if_branch_clause() {
        assert_eq!(
            pairs("IF GREATER GO TO OPERATION 10"),
            vec![
                ("KEYWORD".into(), "IF".into()),
                ("KEYWORD".into(), "GREATER".into()),
                ("KEYWORD".into(), "GO".into()),
                ("KEYWORD".into(), "TO".into()),
                ("KEYWORD".into(), "OPERATION".into()),
                ("NUMBER".into(), "10".into()),
            ]
        );
    }

    /// `READ-ITEM A ; IF END OF DATA GO TO OPERATION 14` — the end-of-file
    /// test, exercising the multi-word `END OF DATA`.
    #[test]
    fn read_item_end_of_data_clause() {
        assert_eq!(
            kinds("READ-ITEM A ; IF END OF DATA GO TO OPERATION 14"),
            vec![
                "KEYWORD",   // READ-ITEM
                "NAME",      // a
                "SEMICOLON",
                "KEYWORD",   // IF
                "KEYWORD",   // END
                "KEYWORD",   // OF
                "KEYWORD",   // DATA
                "KEYWORD",   // GO
                "KEYWORD",   // TO
                "KEYWORD",   // OPERATION
                "NUMBER",    // 14
            ]
        );
    }

    // ----------------------------------------------------------------------
    // Free-form layout: newlines are insignificant
    // ----------------------------------------------------------------------

    /// An operation may wrap across physical lines; only the period ends it.
    /// The token stream is identical whether or not we break the line.
    #[test]
    fn newlines_are_whitespace() {
        let one_line = kinds("INPUT INVENTORY FILE-A ; HSP D .");
        let wrapped = kinds("INPUT INVENTORY FILE-A ;\n   HSP D .");
        assert_eq!(one_line, wrapped);
    }

    // ----------------------------------------------------------------------
    // Whole-program smoke test + EOF invariant
    // ----------------------------------------------------------------------

    /// The first two operations of the canonical program tokenize without error
    /// and the stream ends in exactly one EOF sentinel.
    #[test]
    fn canonical_program_head_tokenizes() {
        let src = "\
(0)  INPUT INVENTORY FILE-A PRICE FILE-B ; OUTPUT PRICED-INV FILE-C
       UNPRICED-INV FILE-D ; HSP D .
(1)  COMPARE PRODUCT-NO (A) WITH PRODUCT-NO (B) ;
       IF GREATER GO TO OPERATION 10 ;
       IF EQUAL GO TO OPERATION 5 ;
       OTHERWISE GO TO OPERATION 2 .";
        let toks = tokenize_flow_matic(src);
        // Ends in EOF, and there is exactly one.
        assert_eq!(toks.last().unwrap().type_, TokenType::Eof);
        assert_eq!(toks.iter().filter(|t| t.type_ == TokenType::Eof).count(), 1);
        // Two operation-terminating periods appear (one per operation).
        let periods = toks.iter().filter(|t| t.effective_type_name() == "PERIOD").count();
        assert_eq!(periods, 2, "expected two operation terminators");
        // The three branch words of operation (1) are all present as keywords.
        for word in ["GREATER", "EQUAL", "OTHERWISE"] {
            assert!(
                toks.iter().any(|t| t.effective_type_name() == "KEYWORD" && t.value == word),
                "missing branch keyword {word}"
            );
        }
    }

    /// An unrecognised character (FLOW-MATIC has no `@`) is a lexical error.
    #[test]
    fn unknown_character_is_an_error() {
        assert!(try_tokenize_flow_matic("(0) @ .").is_err());
    }
}
