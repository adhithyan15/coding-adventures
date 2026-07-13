//! # Derive Lexer — tokenizing the Derive symbolic-CAS subset (D-2).
//!
//! Derive (Soft Warehouse, 1988; successor to muMATH, later Texas
//! Instruments, discontinued 2007 at v6.1) is expression-oriented — a
//! worksheet is a flat sequence of expressions and `:=` definitions, much
//! closer in spirit to Macsyma/Wolfram than to the array languages
//! (APL/J) — but its surface syntax is genuinely its own: ordinary
//! parentheses for function application (`DIF(u, x)`, not `f[x]`), `:=` for
//! both variable and function definition, and `[...]`/`[...;...]` for
//! vector/matrix literals. This crate is a thin wrapper over the generic
//! [`GrammarLexer`] (a sibling of `wolfram-lexer`/`macsyma-lexer`), adding
//! only the bracket-interior newline hook. See
//! `code/specs/MA07-derive-language.md`.
//!
//! ## Architecture
//!
//! ```text
//! derive.tokens       (grammar file — declares every token pattern)
//!     |  (compiled ahead of time into src/_grammar.rs)
//!     v
//! lexer::GrammarLexer (tokenizes source using the embedded TokenGrammar)
//!     |
//!     v
//! derive-lexer        (this crate — adds the bracket-interior newline hook)
//! ```
//!
//! ## The post-tokenize hook: bracket-interior newlines
//!
//! A worksheet is a flat sequence of expressions, each its own line at
//! Derive's own numbered `#n:` prompt, so a NEWLINE at bracket-depth 0 ends
//! a top-level expression — but the contents of a `( … )` grouping/call or a
//! `[ … ]` vector/matrix literal may legally span several physical lines, so
//! those interior newlines must not terminate the expression.
//! [`drop_bracketed_newlines`] tracks the combined `(`/`[` depth (Derive has
//! no `{ }` and no `[[ ]]` part-sugar, unlike Wolfram) and drops `NEWLINE`
//! tokens whenever depth is positive.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::{Token, TokenType};
mod _grammar;

/// Drop `NEWLINE` tokens that occur inside an open `(` or `[`.
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

/// Create a [`GrammarLexer`] configured for Derive source text, with the
/// bracket-interior newline hook registered.
pub fn create_derive_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer.add_post_tokenize(Box::new(drop_bracketed_newlines));
    lexer
}

/// Tokenize Derive source text into a vector of tokens (ending in an `EOF`).
///
/// # Panics
///
/// Panics on an unrecognized character. Use [`try_tokenize_derive`] for a
/// `Result`.
///
/// # Example
///
/// ```
/// use coding_adventures_derive_lexer::tokenize_derive;
/// let tokens = tokenize_derive("DIF(SIN(x), x)\n");
/// assert_eq!(tokens[0].value, "DIF");
/// assert_eq!(tokens[1].effective_type_name(), "LPAREN");
/// ```
pub fn tokenize_derive(source: &str) -> Vec<Token> {
    create_derive_lexer(source)
        .tokenize()
        .unwrap_or_else(|e| panic!("Derive tokenization failed: {e}"))
}

/// Tokenize Derive source text, returning a `Result` instead of panicking.
pub fn try_tokenize_derive(source: &str) -> Result<Vec<Token>, String> {
    create_derive_lexer(source)
        .tokenize()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lex, dropping the trailing EOF and any NEWLINE tokens, into
    /// `(effective_type_name, value)` pairs.
    fn lex(source: &str) -> Vec<(String, String)> {
        tokenize_derive(source)
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

    fn types(p: &[(String, String)]) -> Vec<&str> {
        p.iter().map(|t| t.0.as_str()).collect()
    }

    // --- Function application uses ordinary parens, unlike Wolfram -------

    #[test]
    fn function_application_uses_ordinary_parens() {
        let p = lex("DIF(u, x)\n");
        pair(&p, 0, "NAME", "DIF");
        pair(&p, 1, "LPAREN", "(");
        pair(&p, 2, "NAME", "u");
        pair(&p, 3, "COMMA", ",");
        pair(&p, 4, "NAME", "x");
        pair(&p, 5, "RPAREN", ")");
    }

    // --- `:=` is the ONE assign/define operator ---------------------------

    #[test]
    fn assign_token_is_shared_by_variable_and_function_definition() {
        pair(&lex("x := 5\n"), 1, "ASSIGN", ":=");
        pair(&lex("F(x) := x^2 + 1\n"), 4, "ASSIGN", ":=");
    }

    // --- `=` is equation, NOT assignment (Derive/Macsyma convention) -----

    #[test]
    fn eq_is_equation_distinct_from_assign() {
        let p = lex("x = 4\n");
        pair(&p, 1, "EQ", "=");
        assert!(!types(&p).contains(&"ASSIGN"));
    }

    // --- Vector/matrix literals: `[...]` / `[...;...]` --------------------

    #[test]
    fn vector_literal_lexes_with_brackets_and_commas() {
        let p = lex("[a, b, c]\n");
        assert_eq!(
            types(&p),
            vec!["LBRACKET", "NAME", "COMMA", "NAME", "COMMA", "NAME", "RBRACKET"]
        );
    }

    #[test]
    fn matrix_literal_row_separator_is_semi() {
        let p = lex("[a, b; c, d]\n");
        assert_eq!(
            types(&p),
            vec![
                "LBRACKET", "NAME", "COMMA", "NAME", "SEMI", "NAME", "COMMA", "NAME", "RBRACKET"
            ]
        );
    }

    // --- Boolean-algebra keywords AND/OR/NOT, case-sensitive --------------

    #[test]
    fn boolean_keywords_promote_to_keyword_token_type() {
        let p = lex("a AND b OR NOT c\n");
        assert_eq!(
            p,
            vec![
                ("NAME".to_string(), "a".to_string()),
                ("KEYWORD".to_string(), "AND".to_string()),
                ("NAME".to_string(), "b".to_string()),
                ("KEYWORD".to_string(), "OR".to_string()),
                ("KEYWORD".to_string(), "NOT".to_string()),
                ("NAME".to_string(), "c".to_string()),
            ]
        );
    }

    #[test]
    fn lowercase_boolean_words_are_ordinary_names_not_keywords() {
        // Derive is case-sensitive: `and`/`or`/`not` in lowercase are NOT the
        // reserved boolean keywords — only the exact-case AND/OR/NOT are.
        let p = lex("and or not\n");
        assert_eq!(types(&p), vec!["NAME", "NAME", "NAME"]);
    }

    // --- Comparison and arithmetic operators ------------------------------

    #[test]
    fn comparison_and_arithmetic_operators() {
        for (src, ty) in [
            ("a <= b\n", "LE"),
            ("a >= b\n", "GE"),
            ("a < b\n", "LESS"),
            ("a > b\n", "GREATER"),
            ("a ^ b\n", "POWER"),
            ("a * b\n", "TIMES"),
            ("a / b\n", "SLASH"),
        ] {
            assert!(
                types(&lex(src)).contains(&ty),
                "expected {ty} in {src:?}: {:?}",
                types(&lex(src))
            );
        }
    }

    // --- Longest-match: `:=` never lexes as two tokens --------------------

    #[test]
    fn assign_wins_over_any_shorter_prefix() {
        assert!(types(&lex("x:=5\n")).contains(&"ASSIGN"));
        assert!(!types(&lex("x:=5\n")).contains(&"EQ"));
    }

    // --- Case sensitivity of ordinary symbols (SIN vs sin) ----------------

    #[test]
    fn names_are_case_sensitive() {
        pair(&lex("SIN\n"), 0, "NAME", "SIN");
        pair(&lex("sin\n"), 0, "NAME", "sin");
    }

    // --- Significant newlines, dropped inside `(` / `[` -------------------

    #[test]
    fn newline_inside_parens_or_brackets_is_dropped_top_level_kept() {
        let in_paren = tokenize_derive("F(\n a,\n b\n)\n")
            .iter()
            .filter(|t| t.type_ == TokenType::Newline)
            .count();
        assert_eq!(in_paren, 1, "only the trailing top-level newline remains");

        let in_bracket = tokenize_derive("[\n1,\n2\n]\n")
            .iter()
            .filter(|t| t.type_ == TokenType::Newline)
            .count();
        assert_eq!(in_bracket, 1);

        let top = tokenize_derive("a\nb\n")
            .iter()
            .filter(|t| t.type_ == TokenType::Newline)
            .count();
        assert_eq!(top, 2, "two top-level statements keep their separating newline");
    }

    // --- Literals and errors ----------------------------------------------

    #[test]
    fn numbers_and_symbols() {
        pair(&lex("3.14\n"), 0, "NUMBER", "3.14");
        pair(&lex("42\n"), 0, "NUMBER", "42");
        pair(&lex("x\n"), 0, "NAME", "x");
    }

    #[test]
    fn unknown_char_is_an_error() {
        assert!(try_tokenize_derive("x ~ y\n").is_err());
    }
}
