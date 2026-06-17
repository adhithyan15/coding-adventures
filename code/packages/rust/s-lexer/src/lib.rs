//! # S Lexer — tokenizing the historical Bell Labs S language.
//!
//! [S](https://en.wikipedia.org/wiki/S_(programming_language)) was created at
//! Bell Laboratories beginning in 1976 by John Chambers, Rick Becker, and
//! Allan Wilks as an interactive environment for data analysis. It is the
//! direct ancestor of R. This crate tokenizes S source text; see
//! `code/specs/S00-s-language.md` for the language design.
//!
//! ## The iconic historical detail: `_` is assignment
//!
//! In historical S the underscore is the assignment operator, identical to
//! `<-`: `x _ c(1, 2, 3)` assigns to `x`. This is why an underscore was never
//! allowed inside an S identifier — and why R programmers are still taught to
//! avoid `_` in names. We honour that faithfully: `_` lexes as the
//! `UNDERSCORE` token and the `NAME` pattern excludes it.
//!
//! ## Architecture
//!
//! Like every language frontend in this repo, this crate is a thin wrapper
//! around the generic [`GrammarLexer`]. It never hand-writes tokenization:
//!
//! ```text
//! s.tokens            (grammar file — declares every token pattern)
//!     |  (compiled ahead of time into src/_grammar.rs)
//!     v
//! lexer::GrammarLexer (tokenizes source using the embedded TokenGrammar)
//!     |
//!     v
//! s-lexer             (this crate — adds one post-tokenize hook)
//! ```
//!
//! ## The one post-tokenize hook: bracket-interior newlines
//!
//! S, like Macsyma, treats a newline as a statement terminator — *except*
//! inside an open `(` or `[`, where a newline is insignificant so that a call
//! or index may span lines. Inside `{ }`, newlines stay significant (they
//! separate a block's statements, exactly as in R). A pure token grammar
//! cannot express this context sensitivity, so [`drop_bracketed_newlines`]
//! walks the stream, tracks parenthesis/bracket depth (not brace depth), and
//! drops the `NEWLINE` tokens that sit at depth > 0.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::{Token, TokenType};
mod _grammar;

// ===========================================================================
// Post-tokenize hook
// ===========================================================================

/// Drop `NEWLINE` tokens that occur inside open `(` or `[` (but not `{`).
///
/// # Why this is necessary
///
/// In S a newline ends a statement. But the arguments of a call, or the
/// contents of an index, may legally span several physical lines:
///
/// ```text
/// total <- sum(1,
///              2,
///              3)
/// ```
///
/// Those interior newlines must NOT terminate the statement. R draws the line
/// precisely: newlines are insignificant inside `( )` and `[ ]`, but *do*
/// separate statements inside `{ }`. We therefore track only parenthesis and
/// bracket depth — braces are deliberately ignored — and drop a `NEWLINE`
/// whenever that depth is greater than zero.
///
/// ```text
/// Before:  NAME("sum") LPAREN NUMBER("1") COMMA NEWLINE NUMBER("2") RPAREN
/// After:   NAME("sum") LPAREN NUMBER("1") COMMA        NUMBER("2") RPAREN
/// ```
fn drop_bracketed_newlines(tokens: Vec<Token>) -> Vec<Token> {
    let mut result = Vec::with_capacity(tokens.len());

    // Combined depth of open parentheses and square brackets. Braces are
    // intentionally NOT counted: newlines inside `{ }` remain significant.
    let mut depth: i32 = 0;

    for tok in tokens {
        // A newline while we are inside a paren/bracket group is whitespace.
        if tok.type_ == TokenType::Newline && depth > 0 {
            continue;
        }

        match tok.type_ {
            TokenType::LParen | TokenType::LBracket => depth += 1,
            // saturating_sub keeps a stray closer (in malformed input) from
            // driving the counter negative and corrupting later tracking.
            TokenType::RParen | TokenType::RBracket => depth = depth.saturating_sub(1),
            _ => {}
        }

        result.push(tok);
    }

    result
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a [`GrammarLexer`] configured for S source text.
///
/// The returned lexer embeds the compiled `s.tokens` grammar and has the
/// [`drop_bracketed_newlines`] hook registered. It is ready to `.tokenize()`.
pub fn create_s_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer.add_post_tokenize(Box::new(drop_bracketed_newlines));
    lexer
}

/// Tokenize S source text into a vector of tokens (ending in an `EOF` token).
///
/// # Panics
///
/// Panics if the lexer hits an unrecognized character. Use [`try_tokenize_s`]
/// to handle lexical errors as a `Result` instead.
///
/// # Example
///
/// ```
/// use coding_adventures_s_lexer::tokenize_s;
/// let tokens = tokenize_s("x <- c(1, 2, 3)\n");
/// assert_eq!(tokens[0].value, "x");
/// ```
pub fn tokenize_s(source: &str) -> Vec<Token> {
    create_s_lexer(source)
        .tokenize()
        .unwrap_or_else(|e| panic!("S tokenization failed: {e}"))
}

/// Tokenize S source text, returning a `Result` instead of panicking.
pub fn try_tokenize_s(source: &str) -> Result<Vec<Token>, String> {
    create_s_lexer(source).tokenize().map_err(|e| e.to_string())
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Tokenize `source` and return owned `(effective_type_name, value)` pairs
    /// for every non-EOF token. Returning owned `String`s (rather than borrows
    /// into a temporary token vector) keeps each test a one-liner.
    fn lex(source: &str) -> Vec<(String, String)> {
        tokenize_s(source)
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.effective_type_name().to_string(), t.value.clone()))
            .collect()
    }

    /// Like [`lex`] but also drops NEWLINE tokens.
    fn lex_nl(source: &str) -> Vec<(String, String)> {
        tokenize_s(source)
            .iter()
            .filter(|t| t.type_ != TokenType::Eof && t.type_ != TokenType::Newline)
            .map(|t| (t.effective_type_name().to_string(), t.value.clone()))
            .collect()
    }

    /// Assert that pair `i` of `p` has the given type name and value.
    fn assert_pair(p: &[(String, String)], i: usize, ty: &str, val: &str) {
        assert_eq!(
            (p[i].0.as_str(), p[i].1.as_str()),
            (ty, val),
            "token {i} mismatch in {p:?}"
        );
    }

    /// Count NEWLINE tokens in the source.
    fn newline_count(source: &str) -> usize {
        tokenize_s(source)
            .iter()
            .filter(|t| t.type_ == TokenType::Newline)
            .count()
    }

    /// Does the token stream contain a KEYWORD with this value?
    fn has_keyword(source: &str, value: &str) -> bool {
        tokenize_s(source)
            .iter()
            .any(|t| t.effective_type_name() == "KEYWORD" && t.value == value)
    }

    // --- Assignment operators -------------------------------------------

    #[test]
    fn arrow_assignment() {
        let p = lex_nl("x <- 1\n");
        assert_pair(&p, 0, "NAME", "x");
        assert_pair(&p, 1, "ASSIGN", "<-");
        assert_pair(&p, 2, "NUMBER", "1");
    }

    #[test]
    fn historical_underscore_assignment() {
        // The defining feature of S: `_` is assignment, not part of a name.
        let p = lex_nl("x _ 1\n");
        assert_pair(&p, 0, "NAME", "x");
        assert_pair(&p, 1, "UNDERSCORE", "_");
        assert_pair(&p, 2, "NUMBER", "1");
    }

    #[test]
    fn right_assignment_arrow() {
        let p = lex_nl("1 -> x\n");
        assert_pair(&p, 1, "RIGHT_ASSIGN", "->");
    }

    #[test]
    fn super_assignment_lexes_distinctly() {
        let p = lex_nl("x <<- 1\n");
        assert_pair(&p, 1, "SUPER_ASSIGN", "<<-");
    }

    // --- Numbers and strings --------------------------------------------

    #[test]
    fn number_forms() {
        for n in ["42", "3.14", ".5", "1e3", "1.5e-3"] {
            let p = lex_nl(&format!("x <- {n}\n"));
            assert_pair(&p, 2, "NUMBER", n);
        }
    }

    #[test]
    fn both_string_quote_styles() {
        assert_eq!(lex_nl("\"hello\"\n")[0].0, "STRING");
        assert_eq!(lex_nl("'world'\n")[0].0, "STRING");
    }

    // --- Identifiers: dots allowed, underscores are not ------------------

    #[test]
    fn dotted_names_are_one_token() {
        let p = lex_nl("data.frame\n");
        assert_pair(&p, 0, "NAME", "data.frame");
    }

    #[test]
    fn underscore_splits_a_name() {
        // `a_b` is NOT one identifier in S — it is `a`, assign, `b`.
        let p = lex_nl("a_b\n");
        assert_pair(&p, 0, "NAME", "a");
        assert_pair(&p, 1, "UNDERSCORE", "_");
        assert_pair(&p, 2, "NAME", "b");
    }

    // --- Keywords and reserved constants --------------------------------

    #[test]
    fn control_keywords_promoted() {
        for kw in [
            "if", "else", "for", "while", "repeat", "function", "break", "next", "in",
        ] {
            assert!(
                has_keyword(&format!("{kw}\n"), kw),
                "expected KEYWORD({kw})"
            );
        }
    }

    #[test]
    fn constant_keywords_promoted() {
        for kw in ["TRUE", "FALSE", "T", "F", "NULL", "NA", "Inf", "NaN"] {
            assert!(
                has_keyword(&format!("{kw}\n"), kw),
                "expected KEYWORD({kw})"
            );
        }
    }

    #[test]
    fn case_sensitive_true_vs_name() {
        // `TRUE` is a keyword; `True` and `true` are ordinary names in S.
        let p = lex_nl("True\n");
        assert_pair(&p, 0, "NAME", "True");
    }

    // --- Operators -------------------------------------------------------

    #[test]
    fn arithmetic_and_sequence_operators() {
        let p = lex_nl("a + b - c * d / e ^ f : g\n");
        let names: Vec<&str> = p.iter().map(|t| t.0.as_str()).collect();
        for op in ["PLUS", "MINUS", "STAR", "SLASH", "CARET", "COLON"] {
            assert!(names.contains(&op), "missing {op} in {names:?}");
        }
    }

    #[test]
    fn multichar_comparisons_not_split() {
        let p = lex_nl("a <= b >= c == d != e\n");
        assert_pair(&p, 1, "LE", "<=");
        assert_pair(&p, 3, "GE", ">=");
        assert_pair(&p, 5, "EQEQ", "==");
        assert_pair(&p, 7, "NE", "!=");
    }

    #[test]
    fn single_char_relops_and_equals() {
        let p = lex_nl("a < b > c\n");
        assert_pair(&p, 1, "LT", "<");
        assert_pair(&p, 3, "GT", ">");
        // `=` is its own token (named-argument binding), distinct from `==`.
        let q = lex_nl("f(x = 1)\n");
        assert!(q.iter().any(|t| t.0 == "EQ" && t.1 == "="));
    }

    #[test]
    fn brackets_braces_parens_and_separators() {
        let p = lex_nl("f(x)[1]{}\n");
        let names: Vec<&str> = p.iter().map(|t| t.0.as_str()).collect();
        for b in [
            "LPAREN", "RPAREN", "LBRACKET", "RBRACKET", "LBRACE", "RBRACE",
        ] {
            assert!(names.contains(&b), "missing {b} in {names:?}");
        }
    }

    // --- Comments and whitespace ----------------------------------------

    #[test]
    fn line_comments_are_stripped() {
        let p = lex("x <- 1 # assign one\n");
        assert!(
            !p.iter().any(|t| t.1.contains("assign")),
            "comment leaked: {p:?}"
        );
        assert_pair(&p, 0, "NAME", "x");
    }

    #[test]
    fn whitespace_is_insignificant() {
        assert_eq!(lex("x<-c(1,2)\n"), lex("x  <-  c( 1 , 2 )\n"));
    }

    // --- Newline handling -----------------------------------------------

    #[test]
    fn top_level_newlines_are_kept() {
        assert_eq!(newline_count("x <- 1\ny <- 2\n"), 2);
    }

    #[test]
    fn newlines_inside_parens_are_dropped() {
        // A call spanning lines must not be terminated by the interior newline.
        // Exactly one trailing newline survives (after the closing paren).
        assert_eq!(newline_count("sum(1,\n2,\n3)\n"), 1);
        let p = lex_nl("sum(1,\n2,\n3)\n");
        assert_eq!(p.last().map(|t| t.0.as_str()), Some("RPAREN"));
    }

    #[test]
    fn newlines_inside_braces_are_kept() {
        // Inside { } newlines separate statements and must survive.
        assert!(
            newline_count("{\nx <- 1\ny <- 2\n}\n") >= 3,
            "brace-interior newlines should be kept"
        );
    }

    #[test]
    fn newlines_inside_brackets_are_dropped() {
        assert_eq!(newline_count("x[1,\n2]\n"), 1);
    }

    // --- End-to-end and error handling ----------------------------------

    #[test]
    fn canonical_session_line() {
        let p = lex_nl("x <- c(1, 2, 3)\n");
        let got: Vec<(&str, &str)> = p.iter().map(|t| (t.0.as_str(), t.1.as_str())).collect();
        assert_eq!(
            got,
            vec![
                ("NAME", "x"),
                ("ASSIGN", "<-"),
                ("NAME", "c"),
                ("LPAREN", "("),
                ("NUMBER", "1"),
                ("COMMA", ","),
                ("NUMBER", "2"),
                ("COMMA", ","),
                ("NUMBER", "3"),
                ("RPAREN", ")"),
            ]
        );
    }

    #[test]
    fn always_ends_with_eof() {
        let toks = tokenize_s("1\n");
        assert_eq!(toks.last().unwrap().type_, TokenType::Eof);
    }

    #[test]
    fn unknown_character_is_an_error() {
        assert!(try_tokenize_s("x <- @\n").is_err());
    }
}
