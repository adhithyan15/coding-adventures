//! # R Lexer — tokenizing the R language.
//!
//! [R](https://en.wikipedia.org/wiki/R_(programming_language)) was created by
//! Ross Ihaka and Robert Gentleman (1993) as, in their words, "an
//! implementation of the S language." Its lexical structure is therefore ~98%
//! that of historical S — this crate is a deliberate sibling of the `s-lexer`,
//! built the same way (a thin wrapper over the generic [`GrammarLexer`]). See
//! `code/specs/R00-r-language.md` for the design and the S↔R differences.
//!
//! ## The key difference from S: `_` is a name character, not assignment
//!
//! In historical S the underscore *is* the assignment operator, so it can never
//! appear inside an identifier. R retired that in R 1.9 (2004): in R, `_` is an
//! ordinary identifier character, so `data_frame` is a single name. The R token
//! grammar (`code/grammars/r.tokens`) therefore has no `UNDERSCORE` token and
//! its `NAME` pattern includes `_`. R adds the right super-assignment `->>`.
//!
//! ## Architecture
//!
//! ```text
//! r.tokens            (grammar file — declares every token pattern)
//!     |  (compiled ahead of time into src/_grammar.rs)
//!     v
//! lexer::GrammarLexer (tokenizes source using the embedded TokenGrammar)
//!     |
//!     v
//! r-lexer             (this crate — adds the bracket-interior newline hook)
//! ```
//!
//! ## The post-tokenize hook: bracket-interior newlines (identical to S)
//!
//! R, like S, treats a newline as a statement terminator — except inside an
//! open `(` or `[`, where a newline is insignificant so a call or index may
//! span lines. Inside `{ }`, newlines stay significant (they separate a
//! block's statements). [`drop_bracketed_newlines`] tracks parenthesis/bracket
//! depth (not brace depth) and drops the `NEWLINE` tokens at depth > 0.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::{Token, TokenType};
mod _grammar;

/// Drop `NEWLINE` tokens that occur inside open `(` or `[` (but not `{`).
///
/// The arguments of a call, or the contents of an index, may legally span
/// several physical lines; those interior newlines must not terminate the
/// statement. Newlines inside `{ }` and at top level are kept as statement
/// terminators. (Identical to the S rule.)
fn drop_bracketed_newlines(tokens: Vec<Token>) -> Vec<Token> {
    let mut result = Vec::with_capacity(tokens.len());
    let mut depth: i32 = 0;

    for tok in tokens {
        if tok.type_ == TokenType::Newline && depth > 0 {
            continue;
        }
        match tok.type_ {
            TokenType::LParen | TokenType::LBracket => depth += 1,
            TokenType::RParen | TokenType::RBracket => depth = depth.saturating_sub(1),
            _ => {}
        }
        result.push(tok);
    }

    result
}

/// Create a [`GrammarLexer`] configured for R source text, with the
/// bracket-interior newline hook registered.
pub fn create_r_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer.add_post_tokenize(Box::new(drop_bracketed_newlines));
    lexer
}

/// Tokenize R source text into a vector of tokens (ending in an `EOF` token).
///
/// # Panics
///
/// Panics on an unrecognized character. Use [`try_tokenize_r`] for a `Result`.
///
/// # Example
///
/// ```
/// use coding_adventures_r_lexer::tokenize_r;
/// let tokens = tokenize_r("data_frame <- 1\n");
/// assert_eq!(tokens[0].value, "data_frame"); // `_` is part of the name in R
/// ```
pub fn tokenize_r(source: &str) -> Vec<Token> {
    create_r_lexer(source)
        .tokenize()
        .unwrap_or_else(|e| panic!("R tokenization failed: {e}"))
}

/// Tokenize R source text, returning a `Result` instead of panicking.
pub fn try_tokenize_r(source: &str) -> Result<Vec<Token>, String> {
    create_r_lexer(source).tokenize().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex_nl(source: &str) -> Vec<(String, String)> {
        tokenize_r(source)
            .iter()
            .filter(|t| t.type_ != TokenType::Eof && t.type_ != TokenType::Newline)
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

    fn has_keyword(source: &str, value: &str) -> bool {
        tokenize_r(source)
            .iter()
            .any(|t| t.effective_type_name() == "KEYWORD" && t.value == value)
    }

    // --- The defining R difference: `_` is a name character -------------

    #[test]
    fn underscore_is_part_of_a_name() {
        let p = lex_nl("data_frame <- 1\n");
        pair(&p, 0, "NAME", "data_frame");
        pair(&p, 1, "ASSIGN", "<-");
        pair(&p, 2, "NUMBER", "1");
    }

    #[test]
    fn no_underscore_assignment_token() {
        // `a_b` is ONE name in R (unlike S, where it is `a`, assign, `b`).
        let p = lex_nl("a_b\n");
        assert_eq!(p.len(), 1);
        pair(&p, 0, "NAME", "a_b");
    }

    #[test]
    fn typed_na_constants_are_keywords() {
        for kw in ["NA_integer_", "NA_real_", "NA_character_"] {
            assert!(
                has_keyword(&format!("{kw}\n"), kw),
                "expected KEYWORD({kw})"
            );
        }
    }

    // --- Assignment operators (R adds ->>) ------------------------------

    #[test]
    fn all_assignment_operators() {
        pair(&lex_nl("x <- 1\n"), 1, "ASSIGN", "<-");
        pair(&lex_nl("x <<- 1\n"), 1, "SUPER_ASSIGN", "<<-");
        pair(&lex_nl("1 -> x\n"), 1, "RIGHT_ASSIGN", "->");
        pair(&lex_nl("1 ->> x\n"), 1, "RIGHT_SUPER_ASSIGN", "->>");
        // `=` is its own token (assignment / named-arg binding).
        assert!(lex_nl("x = 1\n").iter().any(|t| t.0 == "EQ" && t.1 == "="));
    }

    // --- Shared-with-S surface still works ------------------------------

    #[test]
    fn operators_numbers_strings_keywords() {
        let p = lex_nl("a + b * c ^ d : e %in% f\n");
        let names: Vec<&str> = p.iter().map(|t| t.0.as_str()).collect();
        for op in ["PLUS", "STAR", "CARET", "COLON", "PERCENT_OP"] {
            assert!(names.contains(&op), "missing {op} in {names:?}");
        }
        assert!(has_keyword("if (x) 1 else 2\n", "if"));
        assert!(has_keyword("TRUE\n", "TRUE"));
        assert_eq!(lex_nl("\"hi\"\n")[0].0, "STRING");
        assert_eq!(lex_nl("'hi'\n")[0].0, "STRING");
        pair(&lex_nl("3.14\n"), 0, "NUMBER", "3.14");
    }

    #[test]
    fn dollar_and_brackets() {
        let p = lex_nl("df$col[1]\n");
        let names: Vec<&str> = p.iter().map(|t| t.0.as_str()).collect();
        for t in ["DOLLAR", "LBRACKET", "RBRACKET"] {
            assert!(names.contains(&t), "missing {t} in {names:?}");
        }
    }

    #[test]
    fn newlines_inside_parens_dropped_inside_braces_kept() {
        let in_parens = tokenize_r("sum(1,\n2)\n")
            .iter()
            .filter(|t| t.type_ == TokenType::Newline)
            .count();
        assert_eq!(in_parens, 1, "interior paren newline should be dropped");
        let in_braces = tokenize_r("{\nx <- 1\ny <- 2\n}\n")
            .iter()
            .filter(|t| t.type_ == TokenType::Newline)
            .count();
        assert!(in_braces >= 3, "brace-interior newlines should be kept");
    }

    #[test]
    fn dotted_names_and_dots() {
        pair(&lex_nl("data.frame\n"), 0, "NAME", "data.frame");
        pair(&lex_nl(".GlobalEnv\n"), 0, "NAME", ".GlobalEnv");
    }

    #[test]
    fn unknown_char_is_an_error() {
        assert!(try_tokenize_r("x <- @\n").is_err());
    }

    // --- R-4: typed numeric literals ------------------------------------

    #[test]
    fn integer_hex_and_complex_literals() {
        pair(&lex_nl("10L\n"), 0, "INT_LIT", "10L");
        pair(&lex_nl("0xFF\n"), 0, "HEX_LIT", "0xFF");
        pair(&lex_nl("0x1FL\n"), 0, "HEX_LIT", "0x1FL"); // hex with integer L
        pair(&lex_nl("2.5i\n"), 0, "COMPLEX_LIT", "2.5i");
        pair(&lex_nl("1e3L\n"), 0, "INT_LIT", "1e3L");
    }

    #[test]
    fn plain_numbers_and_names_unaffected() {
        // No suffix → plain NUMBER; suffix-looking names stay NAMEs.
        pair(&lex_nl("10\n"), 0, "NUMBER", "10");
        pair(&lex_nl("1e3\n"), 0, "NUMBER", "1e3");
        pair(&lex_nl("Length\n"), 0, "NAME", "Length"); // not INT_LIT
        pair(&lex_nl("i\n"), 0, "NAME", "i"); // bare i is a name
                                              // `0x1FL` is one token, not split.
        assert_eq!(lex_nl("0x1FL\n").len(), 1);
    }

    // --- R-9: the native pipe and the backslash lambda ------------------

    #[test]
    fn pipe_operator_and_backslash_lambda() {
        // `|>` is one token, not `|` + `>`.
        let p = lex_nl("x |> f()\n");
        pair(&p, 0, "NAME", "x");
        pair(&p, 1, "PIPE_OP", "|>");
        // `\` is the lambda introducer.
        let l = lex_nl("\\(x) x\n");
        pair(&l, 0, "BACKSLASH", "\\");
        pair(&l, 1, "LPAREN", "(");
    }
}
