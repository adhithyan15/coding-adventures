//! # Maple Lexer — tokenizing the Maple symbolic-CAS subset (MP-2).
//!
//! Maple (Keith Geddes & Gaston Gonnet, University of Waterloo, 1980-82;
//! still actively developed and sold by Maplesoft) looks, on the surface,
//! like a close cousin of REDUCE — the same `:=` assignment spelling, the
//! same `and`/`or`/`not` keywords — but MA09 §1 confirms it genuinely is
//! not: Maple has THREE aggregate literal types (an expression sequence,
//! a list `[a, b, c]`, and a set `{a, b, c}` — this subset covers the
//! latter two, MA09 §2/§3) where REDUCE/Derive each have one, and its own
//! `f(x) := expr` spelling means something narrower (a remember-table
//! patch onto an *existing* procedure, MA09 §1) than REDUCE's/Derive's own
//! general-definition idiom of the same shape. Real Maple's general
//! function definition is the arrow/functional operator — `f := (x, y)
//! -> e` (Help page `operators/functional`) — which is why this crate's
//! grammar has an `ARROW` token neither `reduce-lexer` nor `derive-lexer`
//! needs. This crate is a thin wrapper over the generic [`GrammarLexer`]
//! (a sibling of `reduce-lexer`/`derive-lexer`/`wolfram-lexer`/
//! `macsyma-lexer`). See `code/specs/MA09-maple-language.md`.
//!
//! ## Architecture
//!
//! ```text
//! maple.tokens         (grammar file — declares every token pattern)
//!     |  (compiled ahead of time into src/_grammar.rs)
//!     v
//! lexer::GrammarLexer  (tokenizes source using the embedded TokenGrammar)
//!     |
//!     v
//! maple-lexer          (this crate)
//! ```
//!
//! ## No post-tokenize hook needed
//!
//! Unlike `derive-lexer`/`wolfram-lexer` (whose worksheet-style grammars
//! have a significant top-level `NEWLINE` and so need a bracket-interior
//! newline-dropping hook), Maple statements are separated by `;` or `:`
//! (Programming Guide §5.3 "Statement Separators" — `;` displays the
//! result, `:` suppresses it) — a newline is never significant, and real
//! Maple's own interactive session has no `#n:`/`In[n]:=` numbered-
//! worksheet-prompt convention either (MA09 §5). This exactly mirrors
//! `reduce-lexer`'s/`macsyma-lexer`'s identical `;`/`$`(or `:`)-terminated
//! statement model, so `maple.tokens` emits no `NEWLINE` token at all
//! (every newline is ordinary skipped whitespace), and this crate needs
//! nothing beyond a bare [`GrammarLexer::new`] — the same shape as
//! `reduce-lexer`/`macsyma-lexer`/`j-lexer`.
//!
//! ## `end`/`fi` — two distinct tokens, not one
//!
//! Real Maple closes an `if` statement with either `end if` (two
//! keywords in a row) or the standalone `fi` keyword ("if" reversed — the
//! `if` Help page confirms `fi` is short for `end if`; MA09 §3). This
//! lexer treats `end` and `if` as two independent `KEYWORD` tokens (`if`
//! is already a keyword from the conditional's own opening); it does not
//! special-case the `end if` sequence into one token. Composing `end` +
//! `if` into one production (vs. accepting bare `fi`) is a parser
//! concern — MP-3, not this lexer.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

/// Create a [`GrammarLexer`] configured for Maple source text.
pub fn create_maple_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

/// Tokenize Maple source text into a vector of tokens (ending in an `EOF`).
///
/// # Panics
///
/// Panics on an unrecognized character. Use [`try_tokenize_maple`] for a
/// `Result`.
///
/// # Example
///
/// ```
/// use coding_adventures_maple_lexer::tokenize_maple;
/// let tokens = tokenize_maple("f(x, y)");
/// assert_eq!(tokens[0].value, "f");
/// assert_eq!(tokens[1].effective_type_name(), "LPAREN");
/// ```
pub fn tokenize_maple(source: &str) -> Vec<Token> {
    create_maple_lexer(source)
        .tokenize()
        .unwrap_or_else(|e| panic!("Maple tokenization failed: {e}"))
}

/// Tokenize Maple source text, returning a `Result` instead of panicking.
pub fn try_tokenize_maple(source: &str) -> Result<Vec<Token>, String> {
    create_maple_lexer(source)
        .tokenize()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    /// Lex, dropping the trailing EOF, into `(effective_type_name, value)` pairs.
    fn lex(source: &str) -> Vec<(String, String)> {
        tokenize_maple(source)
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

    // --- Function application uses ordinary parens -------------------------

    #[test]
    fn function_application_uses_ordinary_parens() {
        let p = lex("f(x, y);");
        pair(&p, 0, "NAME", "f");
        pair(&p, 1, "LPAREN", "(");
        pair(&p, 2, "NAME", "x");
        pair(&p, 3, "COMMA", ",");
        pair(&p, 4, "NAME", "y");
        pair(&p, 5, "RPAREN", ")");
        pair(&p, 6, "SEMI", ";");
    }

    // --- `:=` is assignment, `=` is equation, never confused ---------------

    #[test]
    fn assign_and_eq_are_distinct_tokens() {
        pair(&lex("x := 5;"), 1, "ASSIGN", ":=");
        let p = lex("x = 4;");
        pair(&p, 1, "EQ", "=");
        assert!(!types(&p).contains(&"ASSIGN"));
    }

    #[test]
    fn assign_wins_over_bare_colon_then_eq() {
        let p = lex("x:=5;");
        assert!(types(&p).contains(&"ASSIGN"));
        assert!(!types(&p).contains(&"COLON"));
        assert!(!types(&p).contains(&"EQ"));
    }

    // --- `->` is the arrow/functional operator, never split as `-` `>` -----

    #[test]
    fn arrow_wins_over_minus_then_greater() {
        let p = lex("f := (x, y) -> x + y;");
        assert!(types(&p).contains(&"ARROW"));
        assert!(!types(&p).contains(&"GREATER"));
    }

    #[test]
    fn arrow_operator_definition_snippet_lexes_end_to_end() {
        // MA09 §3's own worked example of the general-purpose
        // function-definition idiom.
        let p = lex("f := (x, y) -> x + y;");
        assert_eq!(
            types(&p),
            vec![
                "NAME", "ASSIGN", "LPAREN", "NAME", "COMMA", "NAME", "RPAREN", "ARROW", "NAME",
                "PLUS", "NAME", "SEMI",
            ]
        );
    }

    // --- `<>` is not-equal, distinct from a split `<` `>` -------------------

    #[test]
    fn not_equal_wins_over_less_then_greater() {
        let p = lex("a <> b;");
        assert!(types(&p).contains(&"NEQ"));
        assert!(!types(&p).contains(&"LESS"));
        assert!(!types(&p).contains(&"GREATER"));
    }

    // --- `<=`/`>=` win over bare `<`/`>` -------------------------------------

    #[test]
    fn le_and_ge_win_over_bare_less_and_greater() {
        let p1 = lex("a <= b;");
        assert!(types(&p1).contains(&"LE"));
        assert!(!types(&p1).contains(&"LESS"));

        let p2 = lex("a >= b;");
        assert!(types(&p2).contains(&"GE"));
        assert!(!types(&p2).contains(&"GREATER"));
    }

    #[test]
    fn bare_less_and_greater_still_lex_alone() {
        let p1 = lex("a < b;");
        assert!(types(&p1).contains(&"LESS"));
        let p2 = lex("a > b;");
        assert!(types(&p2).contains(&"GREATER"));
    }

    // --- List literals use SQUARE brackets, set literals use CURLY braces --

    #[test]
    fn list_literal_lexes_with_square_brackets_and_commas() {
        let p = lex("[a, b, c];");
        assert_eq!(
            types(&p),
            vec![
                "LBRACKET", "NAME", "COMMA", "NAME", "COMMA", "NAME", "RBRACKET", "SEMI",
            ]
        );
    }

    #[test]
    fn set_literal_lexes_with_curly_braces_and_commas() {
        let p = lex("{a, b, c};");
        assert_eq!(
            types(&p),
            vec![
                "LBRACE", "NAME", "COMMA", "NAME", "COMMA", "NAME", "RBRACE", "SEMI",
            ]
        );
    }

    #[test]
    fn list_and_set_brackets_are_distinct_token_types() {
        // Same element content, different bracket -- must not collapse to
        // the same token type (MA09 §1's own "two brackets, two meanings"
        // point).
        let list_lexed = lex("[x];");
        let set_lexed = lex("{x};");
        let list_types = types(&list_lexed);
        let set_types = types(&set_lexed);
        assert_eq!(list_types[0], "LBRACKET");
        assert_eq!(set_types[0], "LBRACE");
        assert_ne!(list_types[0], set_types[0]);
    }

    // --- Single-char arithmetic operators, `^` only (no `**` synonym) ------

    #[test]
    fn arithmetic_and_power_operators() {
        for (src, ty) in [
            ("a + b;", "PLUS"),
            ("a - b;", "MINUS"),
            ("a * b;", "TIMES"),
            ("a / b;", "SLASH"),
            ("a ^ b;", "CARET"),
        ] {
            assert!(
                types(&lex(src)).contains(&ty),
                "expected {ty} in {src:?}: {:?}",
                types(&lex(src))
            );
        }
    }

    #[test]
    fn double_star_is_not_a_single_pow_token() {
        // Real Maple documents no `**` synonym for `^` (MA09 §3) -- `**`
        // must lex as two separate TIMES tokens, never a POW token (there
        // is no POW token in this grammar at all).
        let p = lex("a ** b;");
        assert_eq!(
            types(&p).iter().filter(|t| **t == "TIMES").count(),
            2,
            "{:?}",
            types(&p)
        );
        assert!(!types(&p).contains(&"POW"));
        assert!(!types(&p).contains(&"CARET"));
    }

    // --- Statement separators `;`/`:` are distinct terminator tokens -------

    #[test]
    fn semi_and_colon_are_distinct_terminator_tokens() {
        assert_eq!(lex("a := 1;").last().unwrap().0, "SEMI");
        assert_eq!(lex("a := 1:").last().unwrap().0, "COLON");
    }

    #[test]
    fn colon_never_collides_with_assign() {
        // A bare `:` (not immediately followed by `=`) must lex as COLON,
        // never accidentally consumed as half of an ASSIGN.
        let p = lex("a := 1: b := 2;");
        assert_eq!(
            types(&p),
            vec![
                "NAME", "ASSIGN", "NUMBER", "COLON", "NAME", "ASSIGN", "NUMBER", "SEMI",
            ]
        );
    }

    // --- NUMBER / NAME literals, no-leading-dot convention ------------------

    #[test]
    fn numbers_and_names() {
        pair(&lex("42;"), 0, "NUMBER", "42");
        pair(&lex("1.5;"), 0, "NUMBER", "1.5");
        pair(&lex("x;"), 0, "NAME", "x");
    }

    #[test]
    fn leading_dot_is_not_a_number() {
        // `.5` is not a valid Maple NUMBER in this grammar -- there is no
        // DOT token here at all (unlike reduce.tokens's cons operator), so
        // a bare `.` is simply an unrecognized character.
        assert!(try_tokenize_maple(".5;").is_err());
    }

    // --- Keywords lex as KEYWORD, case-sensitively (lowercase only) --------

    #[test]
    fn logical_keywords_promote_to_keyword_token_type() {
        let p = lex("a and b or not c;");
        assert_eq!(
            p,
            vec![
                ("NAME".to_string(), "a".to_string()),
                ("KEYWORD".to_string(), "and".to_string()),
                ("NAME".to_string(), "b".to_string()),
                ("KEYWORD".to_string(), "or".to_string()),
                ("KEYWORD".to_string(), "not".to_string()),
                ("NAME".to_string(), "c".to_string()),
                ("SEMI".to_string(), ";".to_string()),
            ]
        );
    }

    #[test]
    fn conditional_keywords_promote_to_keyword_token_type() {
        // Positions: if(0) a(1) then(2) b(3) elif(4) c(5) then(6) d(7)
        // else(8) e(9) end(10) ;(11) -- `if`/`then`/`elif`/`then`/`else`/
        // `end` are all KEYWORD, `a`/`b`/`c`/`d`/`e` remain ordinary NAMEs.
        let p = lex("if a then b elif c then d else e end;");
        assert_eq!(
            types(&p),
            vec![
                "KEYWORD", "NAME", "KEYWORD", "NAME", "KEYWORD", "NAME", "KEYWORD", "NAME",
                "KEYWORD", "NAME", "KEYWORD", "SEMI",
            ]
        );
    }

    #[test]
    fn fi_keyword_promotes_to_keyword_token_type() {
        let p = lex("if x > 0 then 1 else -1 fi");
        assert!(types(&p).contains(&"KEYWORD"));
        assert_eq!(p.last().unwrap(), &("KEYWORD".to_string(), "fi".to_string()));
    }

    #[test]
    fn end_and_if_are_two_separate_keyword_tokens_not_one() {
        let p = lex("if true then 1 else 0 end if");
        assert_eq!(
            (
                p[p.len() - 2].0.as_str(),
                p[p.len() - 2].1.as_str(),
                p[p.len() - 1].0.as_str(),
                p[p.len() - 1].1.as_str(),
            ),
            ("KEYWORD", "end", "KEYWORD", "if")
        );
    }

    #[test]
    fn boolean_literals_promote_to_keyword_token_type() {
        let p = lex("true; false;");
        pair(&p, 0, "KEYWORD", "true");
        pair(&p, 2, "KEYWORD", "false");
    }

    #[test]
    fn uppercase_keyword_spellings_are_ordinary_names_not_keywords() {
        // Maple's keywords are lowercase, like REDUCE's -- the mirror
        // image of Derive's uppercase AND/OR/NOT. Uppercase spellings of
        // the same words are NOT the reserved words here; only the exact
        // lowercase spelling is.
        let p = lex("AND OR NOT IF THEN ELIF ELSE END FI TRUE FALSE;");
        assert_eq!(
            types(&p),
            vec![
                "NAME", "NAME", "NAME", "NAME", "NAME", "NAME", "NAME", "NAME", "NAME", "NAME",
                "NAME", "SEMI",
            ]
        );
    }

    // --- Case sensitivity of ordinary symbols --------------------------------

    #[test]
    fn names_are_case_sensitive() {
        pair(&lex("Sin;"), 0, "NAME", "Sin");
        pair(&lex("sin;"), 0, "NAME", "sin");
    }

    // --- Newlines are ordinary whitespace, never a distinct token ----------

    #[test]
    fn newlines_are_plain_whitespace_not_a_token() {
        let p = lex("a := 1;\nb := 2:\n");
        assert!(!types(&p).contains(&"NEWLINE"));
    }

    // --- End-to-end realistic snippets from MA09 §3 -------------------------

    #[test]
    fn if_elif_else_end_if_snippet_lexes_end_to_end() {
        let p = lex("if x > 0 then 1 elif x < 0 then -1 else 0 end if;");
        assert_eq!(
            types(&p),
            vec![
                "KEYWORD", "NAME", "GREATER", "NUMBER", "KEYWORD", "NUMBER", "KEYWORD", "NAME",
                "LESS", "NUMBER", "KEYWORD", "MINUS", "NUMBER", "KEYWORD", "NUMBER", "KEYWORD",
                "KEYWORD", "SEMI",
            ]
        );
    }

    #[test]
    fn list_and_set_literal_in_a_call_snippet() {
        let p = lex("g([1, 2, 3], {1, 2, 2});");
        assert_eq!(
            types(&p),
            vec![
                "NAME", "LPAREN", "LBRACKET", "NUMBER", "COMMA", "NUMBER", "COMMA", "NUMBER",
                "RBRACKET", "COMMA", "LBRACE", "NUMBER", "COMMA", "NUMBER", "COMMA", "NUMBER",
                "RBRACE", "RPAREN", "SEMI",
            ]
        );
    }

    // --- Errors ---------------------------------------------------------------

    #[test]
    fn unknown_char_is_an_error() {
        assert!(try_tokenize_maple("x ~ y;").is_err());
    }
}
