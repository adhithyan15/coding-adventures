//! # Reduce Lexer — tokenizing the REDUCE symbolic-CAS subset (R-2).
//!
//! REDUCE (Anthony C. Hearn, Stanford/Rand Corporation, 1968;
//! open-sourced 2008; still actively maintained) is one of the two oldest
//! CAS ever built (alongside Macsyma, both 1968). Its "algebraic mode"
//! reads like an ordinary Algol/Pascal-family imperative language at the
//! *statement* level (`:=` assignment, `if`/`then`/`else`, `<< ... >>`
//! statement grouping) but every *expression* is an ordinary infix/
//! `f(args)`-call expression, exactly the same shape Macsyma/Wolfram/
//! Derive already lower to. This crate is a thin wrapper over the generic
//! [`GrammarLexer`] (a sibling of `derive-lexer`/`wolfram-lexer`/
//! `macsyma-lexer`). See `code/specs/MA08-reduce-language.md`.
//!
//! ## Architecture
//!
//! ```text
//! reduce.tokens       (grammar file — declares every token pattern)
//!     |  (compiled ahead of time into src/_grammar.rs)
//!     v
//! lexer::GrammarLexer (tokenizes source using the embedded TokenGrammar)
//!     |
//!     v
//! reduce-lexer        (this crate)
//! ```
//!
//! ## No post-tokenize hook needed
//!
//! Unlike `derive-lexer`/`wolfram-lexer` (whose worksheet-style grammars
//! have a significant top-level `NEWLINE` and so need a bracket-interior
//! newline-dropping hook), REDUCE statements are separated by `;` or `$`
//! (manual §5.1) — a newline is never significant, exactly mirroring
//! `macsyma-lexer`'s identical `;`/`$`-terminated statement model. So
//! `reduce.tokens` emits no `NEWLINE` token at all (every newline is
//! ordinary skipped whitespace), and this crate needs nothing beyond a
//! bare [`GrammarLexer::new`] — the same shape as `macsyma-lexer`/
//! `j-lexer`.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

/// Create a [`GrammarLexer`] configured for REDUCE source text.
pub fn create_reduce_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

/// Tokenize REDUCE source text into a vector of tokens (ending in an `EOF`).
///
/// # Panics
///
/// Panics on an unrecognized character. Use [`try_tokenize_reduce`] for a
/// `Result`.
///
/// # Example
///
/// ```
/// use coding_adventures_reduce_lexer::tokenize_reduce;
/// let tokens = tokenize_reduce("df(x, y)$");
/// assert_eq!(tokens[0].value, "df");
/// assert_eq!(tokens[1].effective_type_name(), "LPAREN");
/// ```
pub fn tokenize_reduce(source: &str) -> Vec<Token> {
    create_reduce_lexer(source)
        .tokenize()
        .unwrap_or_else(|e| panic!("Reduce tokenization failed: {e}"))
}

/// Tokenize REDUCE source text, returning a `Result` instead of panicking.
pub fn try_tokenize_reduce(source: &str) -> Result<Vec<Token>, String> {
    create_reduce_lexer(source)
        .tokenize()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    /// Lex, dropping the trailing EOF, into `(effective_type_name, value)` pairs.
    fn lex(source: &str) -> Vec<(String, String)> {
        tokenize_reduce(source)
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.effective_type_name().to_string(), t.value.clone()))
            .collect()
    }

    fn pair(p: &[(String, String)], i: usize, ty: &str, val: &str) {
        assert_eq!(
            (p[i].0.as_str(), p[i].1.as_str()),
            (ty, val),
            "tok {i} in {p:?}"
        );
    }

    fn types(p: &[(String, String)]) -> Vec<&str> {
        p.iter().map(|t| t.0.as_str()).collect()
    }

    // --- Function/procedure application uses ordinary parens -------------

    #[test]
    fn function_application_uses_ordinary_parens() {
        let p = lex("df(x, y)$");
        pair(&p, 0, "NAME", "df");
        pair(&p, 1, "LPAREN", "(");
        pair(&p, 2, "NAME", "x");
        pair(&p, 3, "COMMA", ",");
        pair(&p, 4, "NAME", "y");
        pair(&p, 5, "RPAREN", ")");
    }

    // --- `:=` is assignment, `=` is equation, never confused --------------

    #[test]
    fn assign_and_eq_are_distinct_tokens() {
        pair(&lex("x := 5"), 1, "ASSIGN", ":=");
        pair(&lex("h(l, m) := l + m"), 6, "ASSIGN", ":=");
        let p = lex("x = 4");
        pair(&p, 1, "EQ", "=");
        assert!(!types(&p).contains(&"ASSIGN"));
    }

    // --- List literals use CURLY braces, not square brackets --------------

    #[test]
    fn list_literal_lexes_with_braces_and_commas() {
        let p = lex("{a, b, c}");
        assert_eq!(
            types(&p),
            vec!["LBRACE", "NAME", "COMMA", "NAME", "COMMA", "NAME", "RBRACE"]
        );
    }

    // --- Cons operator `.` never confused with a decimal point ------------

    #[test]
    fn cons_dot_is_distinct_from_a_decimal_number() {
        let p = lex("a . {b, c}");
        assert_eq!(
            types(&p),
            vec!["NAME", "DOT", "LBRACE", "NAME", "COMMA", "NAME", "RBRACE"]
        );
        pair(&lex("3.14$"), 0, "NUMBER", "3.14");
    }

    // --- Word-spelled logical/relational keywords, lowercase only ---------

    #[test]
    fn logical_and_relational_keywords_promote_to_keyword_token_type() {
        let p = lex("a and b or not c neq d");
        assert_eq!(
            p,
            vec![
                ("NAME".to_string(), "a".to_string()),
                ("KEYWORD".to_string(), "and".to_string()),
                ("NAME".to_string(), "b".to_string()),
                ("KEYWORD".to_string(), "or".to_string()),
                ("KEYWORD".to_string(), "not".to_string()),
                ("NAME".to_string(), "c".to_string()),
                ("KEYWORD".to_string(), "neq".to_string()),
                ("NAME".to_string(), "d".to_string()),
            ]
        );
    }

    #[test]
    fn uppercase_keyword_spellings_are_ordinary_names_not_keywords() {
        // REDUCE's keywords are lowercase — the mirror image of Derive's
        // uppercase AND/OR/NOT. AND/OR/NOT/NEQ in uppercase are NOT the
        // reserved words here; only the exact lowercase spellings are.
        let p = lex("AND OR NOT NEQ");
        assert_eq!(types(&p), vec!["NAME", "NAME", "NAME", "NAME"]);
    }

    // --- `if`/`then`/`else` conditional keywords ---------------------------

    #[test]
    fn if_then_else_promote_to_keyword_token_type() {
        let p = lex("if a > b then a else b");
        pair(&p, 0, "KEYWORD", "if");
        pair(&p, 4, "KEYWORD", "then");
        pair(&p, 6, "KEYWORD", "else");
    }

    // --- Comparison and arithmetic operators -------------------------------

    #[test]
    fn comparison_and_arithmetic_operators() {
        for (src, ty) in [
            ("a <= b$", "LE"),
            ("a >= b$", "GE"),
            ("a < b$", "LESS"),
            ("a > b$", "GREATER"),
            ("a ^ b$", "CARET"),
            ("a ** b$", "POW"),
            ("a * b$", "TIMES"),
            ("a / b$", "SLASH"),
        ] {
            assert!(
                types(&lex(src)).contains(&ty),
                "expected {ty} in {src:?}: {:?}",
                types(&lex(src))
            );
        }
    }

    // --- Group statement `<< ... >>` ---------------------------------------

    #[test]
    fn group_statement_delimiters() {
        let p = lex("<< a := 1; b := 2 >>$");
        assert_eq!(p[0].0, "GROUP_OPEN");
        assert_eq!(p.last().unwrap().0, "DOLLAR");
        assert!(types(&p).contains(&"GROUP_CLOSE"));
        assert!(types(&p).contains(&"SEMI"));
    }

    // --- `;` and `$` are both statement terminators, kept distinct --------

    #[test]
    fn semi_and_dollar_are_distinct_terminator_tokens() {
        assert_eq!(lex("a := 1;").last().unwrap().0, "SEMI");
        assert_eq!(lex("a := 1$").last().unwrap().0, "DOLLAR");
    }

    // --- Longest-match: multi-char operators never split ------------------

    #[test]
    fn assign_wins_over_any_shorter_prefix() {
        assert!(types(&lex("x:=5$")).contains(&"ASSIGN"));
        assert!(!types(&lex("x:=5$")).contains(&"EQ"));
    }

    #[test]
    fn double_star_wins_over_two_single_stars() {
        let p = lex("a**b$");
        assert_eq!(types(&p).iter().filter(|t| **t == "TIMES").count(), 0);
        assert!(types(&p).contains(&"POW"));
    }

    #[test]
    fn double_angle_wins_over_comparison_operators() {
        let p = lex("<<a>>$");
        assert!(types(&p).contains(&"GROUP_OPEN"));
        assert!(types(&p).contains(&"GROUP_CLOSE"));
        assert!(!types(&p).contains(&"LESS"));
        assert!(!types(&p).contains(&"GREATER"));
        assert!(!types(&p).contains(&"LE"));
        assert!(!types(&p).contains(&"GE"));
    }

    // --- Newlines are ordinary whitespace, never a distinct token ---------

    #[test]
    fn newlines_are_plain_whitespace_not_a_token() {
        let p = lex("a := 1;\nb := 2$\n");
        assert!(!types(&p).contains(&"NEWLINE"));
    }

    // --- Case sensitivity of ordinary symbols ------------------------------

    #[test]
    fn names_are_case_sensitive() {
        pair(&lex("Df$"), 0, "NAME", "Df");
        pair(&lex("df$"), 0, "NAME", "df");
    }

    // --- Literals and errors -------------------------------------------------

    #[test]
    fn numbers_and_symbols() {
        pair(&lex("3.14$"), 0, "NUMBER", "3.14");
        pair(&lex("42$"), 0, "NUMBER", "42");
        pair(&lex("x$"), 0, "NAME", "x");
    }

    #[test]
    fn unknown_char_is_an_error() {
        assert!(try_tokenize_reduce("x ~ y$").is_err());
    }
}
