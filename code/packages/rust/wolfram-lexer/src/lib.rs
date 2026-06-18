//! # Wolfram Lexer — tokenizing the Wolfram Language (Mathematica).
//!
//! The Wolfram Language is built around M-expressions: *everything* is
//! `head[arg, …]`, function application uses **square** brackets, lists use
//! **braces** (`{a, b}`), and the language centres on the replacement operators
//! `/.`, `->`, `:>` and the pattern blanks `_`/`x_`. This crate is a thin
//! wrapper over the generic [`GrammarLexer`] (a sibling of `r-lexer` /
//! `macsyma-lexer`), adding only the bracket-interior newline hook. See
//! `code/specs/MA04-wolfram-language.md`.
//!
//! ## Architecture
//!
//! ```text
//! wolfram.tokens      (grammar file — declares every token pattern)
//!     |  (compiled ahead of time into src/_grammar.rs)
//!     v
//! lexer::GrammarLexer (tokenizes source using the embedded TokenGrammar)
//!     |
//!     v
//! wolfram-lexer       (this crate — adds the bracket-interior newline hook)
//! ```
//!
//! ## The post-tokenize hook: bracket-interior newlines
//!
//! A newline ends a top-level statement, but inside any open bracket — `( )`,
//! `[ ]`, **or** `{ }` — a newline is insignificant, so a grouping, a function
//! application, or a list may span several physical lines. This differs from R,
//! where `{ }` is a statement *block* whose interior newlines stay significant;
//! in Wolfram `{a, b}` is a *list*, so its interior newlines are dropped too.
//! [`drop_bracketed_newlines`] tracks the combined `(`/`[`/`{` depth and drops
//! `NEWLINE` tokens whenever depth is positive.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::{Token, TokenType};
mod _grammar;

/// Drop `NEWLINE` tokens that occur inside an open `(`, `[`, `{`, or `[[`.
///
/// Wolfram has no required statement terminator — a newline at depth 0 ends a
/// statement — but the contents of a group, an `f[…]` application, a `{…}`
/// list, or a `[[ … ]]` part expression may legally span lines, so those
/// interior newlines must not terminate the statement.
///
/// The single-character brackets (`(`/`[`/`{`) carry dedicated `TokenType`
/// enum variants, so they match by `type_`. The W-6 part-sugar opener `[[`
/// (LDBRACKET) is a *custom* multi-char token with no enum variant — it falls
/// back to a generic `type_`, so we recognise it by `effective_type_name()`.
/// Crucially, `[[` is one *token* but two *levels* of bracket nesting (it is
/// closed by two single `]` RBRACKETs — there is no `]]` token, see the grammar
/// and `wolfram.tokens`), so it adds `2` to the depth and each closing `]` (an
/// ordinary RBracket) subtracts `1`. That keeps the count balanced, so a `\n`
/// inside `x[[\n i\n]]` is dropped just like one inside `f[\n a\n]`.
fn drop_bracketed_newlines(tokens: Vec<Token>) -> Vec<Token> {
    let mut result = Vec::with_capacity(tokens.len());
    let mut depth: i32 = 0;

    for tok in tokens {
        if tok.type_ == TokenType::Newline && depth > 0 {
            continue;
        }
        if tok.effective_type_name() == "LDBRACKET" {
            depth += 2; // `[[` opens two bracket levels (closes with `]` `]`).
        } else {
            match tok.type_ {
                TokenType::LParen | TokenType::LBracket | TokenType::LBrace => depth += 1,
                TokenType::RParen | TokenType::RBracket | TokenType::RBrace => {
                    depth = depth.saturating_sub(1)
                }
                _ => {}
            }
        }
        result.push(tok);
    }

    result
}

/// Create a [`GrammarLexer`] configured for Wolfram source text, with the
/// bracket-interior newline hook registered.
pub fn create_wolfram_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer.add_post_tokenize(Box::new(drop_bracketed_newlines));
    lexer
}

/// Tokenize Wolfram source text into a vector of tokens (ending in an `EOF`).
///
/// # Panics
///
/// Panics on an unrecognized character. Use [`try_tokenize_wolfram`] for a
/// `Result`.
///
/// # Example
///
/// ```
/// use coding_adventures_wolfram_lexer::tokenize_wolfram;
/// let tokens = tokenize_wolfram("Sin[x]\n");
/// assert_eq!(tokens[0].value, "Sin");
/// assert_eq!(tokens[1].effective_type_name(), "LBRACKET"); // square-bracket apply
/// ```
pub fn tokenize_wolfram(source: &str) -> Vec<Token> {
    create_wolfram_lexer(source)
        .tokenize()
        .unwrap_or_else(|e| panic!("Wolfram tokenization failed: {e}"))
}

/// Tokenize Wolfram source text, returning a `Result` instead of panicking.
pub fn try_tokenize_wolfram(source: &str) -> Result<Vec<Token>, String> {
    create_wolfram_lexer(source)
        .tokenize()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lex, dropping the trailing EOF and any NEWLINE tokens, into
    /// `(effective_type_name, value)` pairs.
    fn lex(source: &str) -> Vec<(String, String)> {
        tokenize_wolfram(source)
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

    // --- The defining Wolfram surface: f[x] and {a, b} ------------------

    #[test]
    fn function_application_uses_square_brackets() {
        let p = lex("Sin[x]\n");
        pair(&p, 0, "NAME", "Sin");
        pair(&p, 1, "LBRACKET", "[");
        pair(&p, 2, "NAME", "x");
        pair(&p, 3, "RBRACKET", "]");
    }

    #[test]
    fn list_uses_braces() {
        let p = lex("{a, b, c}\n");
        pair(&p, 0, "LBRACE", "{");
        pair(&p, 1, "NAME", "a");
        pair(&p, 2, "COMMA", ",");
        pair(&p, 3, "NAME", "b");
        assert_eq!(p.last().unwrap().0, "RBRACE");
    }

    // --- Pattern blanks: `_`, `x_`, `_h`, `x_h` -------------------------

    #[test]
    fn named_blank_lexes_as_name_then_blank() {
        // `x_` is NAME `x` followed by BLANK (`_` is not a name character).
        let p = lex("x_\n");
        pair(&p, 0, "NAME", "x");
        pair(&p, 1, "BLANK", "_");
    }

    #[test]
    fn typed_blank_lexes_as_blank_then_name() {
        // `_h` is BLANK followed by the head NAME `h`.
        let p = lex("_h\n");
        pair(&p, 0, "BLANK", "_");
        pair(&p, 1, "NAME", "h");
        // `x_h` is NAME BLANK NAME.
        let q = lex("x_Integer\n");
        assert_eq!(types(&q), vec!["NAME", "BLANK", "NAME"]);
    }

    // --- The replacement operators are single tokens --------------------

    #[test]
    fn replacement_operators_are_single_tokens() {
        pair(&lex("x /. r\n"), 1, "REPLACEALL", "/.");
        pair(&lex("a -> b\n"), 1, "RULE", "->");
        pair(&lex("a :> b\n"), 1, "RULEDELAYED", ":>");
        pair(&lex("x := y\n"), 1, "SETDELAYED", ":=");
        // `/.` wins over a bare `/`, and `->` over `-`.
        assert!(types(&lex("a/.b\n")).contains(&"REPLACEALL"));
        assert!(!types(&lex("a/.b\n")).contains(&"SLASH"));
    }

    // --- W-6 operator sugar: /@, @@, [[ ]] are single (multi-char) tokens ---

    #[test]
    fn map_and_apply_sugar_are_single_tokens() {
        // `/@` is MAP, distinct from `/.` (REPLACEALL) and a bare `/` (SLASH).
        pair(&lex("f /@ x\n"), 1, "MAP", "/@");
        // `@@` is APPLY.
        pair(&lex("f @@ x\n"), 1, "APPLY", "@@");
        // Longest-match: `/@` wins over `/`, and `@@` is not two unknown chars.
        assert!(types(&lex("f/@x\n")).contains(&"MAP"));
        assert!(!types(&lex("f/@x\n")).contains(&"SLASH"));
        assert!(types(&lex("f@@x\n")).contains(&"APPLY"));
    }

    #[test]
    fn double_bracket_opener_wins_over_single_but_closer_is_two_singles() {
        // `x[[i]]` lexes the opener as one LDBRACKET (winning over `[`) but the
        // closer as TWO ordinary RBRACKETs — there is no `]]` token, on purpose,
        // so the tail of nested `f[g[x]]` cannot be mis-lexed (see grammar).
        let p = lex("x[[2]]\n");
        pair(&p, 0, "NAME", "x");
        pair(&p, 1, "LDBRACKET", "[[");
        pair(&p, 2, "NUMBER", "2");
        pair(&p, 3, "RBRACKET", "]");
        pair(&p, 4, "RBRACKET", "]");
        assert!(!types(&lex("x[[2]]\n")).contains(&"LBRACKET"));
        // A single `[`/`]` still lexes as LBRACKET/RBRACKET (f[x]).
        assert!(types(&lex("f[x]\n")).contains(&"LBRACKET"));
        // The regression guard: nested ordinary application keeps two singles.
        assert_eq!(
            types(&lex("f[g[x]]\n")),
            vec!["NAME", "LBRACKET", "NAME", "LBRACKET", "NAME", "RBRACKET", "RBRACKET"]
        );
    }

    #[test]
    fn newline_inside_double_brackets_is_dropped() {
        // The W-6 hook tracks `[[`/`]]` depth, so an interior newline is dropped.
        let n = tokenize_wolfram("x[[\n 1\n]]\n")
            .iter()
            .filter(|t| t.type_ == TokenType::Newline)
            .count();
        assert_eq!(n, 1, "only the trailing top-level newline remains");
    }

    #[test]
    fn comparison_logic_and_arithmetic_operators() {
        for (src, ty) in [
            ("a == b\n", "EQUAL"),
            ("a != b\n", "UNEQUAL"),
            ("a <= b\n", "LE"),
            ("a >= b\n", "GE"),
            ("a && b\n", "AND"),
            ("a || b\n", "OR"),
            ("a ^ b\n", "POWER"),
            ("a * b\n", "TIMES"),
        ] {
            assert!(
                types(&lex(src)).contains(&ty),
                "expected {ty} in {src:?}: {:?}",
                types(&lex(src))
            );
        }
    }

    // --- Comments are `(* ... *)` and are skipped -----------------------

    #[test]
    fn block_comment_is_skipped() {
        let p = lex("1 (* a comment *) + 2\n");
        assert_eq!(types(&p), vec!["NUMBER", "PLUS", "NUMBER"]);
    }

    // --- Significant newlines, dropped inside brackets ------------------

    #[test]
    fn newline_inside_any_bracket_is_dropped_top_level_kept() {
        // Inside `[ ]` — dropped.
        let in_sq = tokenize_wolfram("f[\n a\n]\n")
            .iter()
            .filter(|t| t.type_ == TokenType::Newline)
            .count();
        assert_eq!(in_sq, 1, "only the trailing top-level newline remains");
        // Inside `{ }` — also dropped (a list, not a block).
        let in_br = tokenize_wolfram("{\n1,\n2\n}\n")
            .iter()
            .filter(|t| t.type_ == TokenType::Newline)
            .count();
        assert_eq!(in_br, 1);
        // Two top-level statements keep their separating newline.
        let top = tokenize_wolfram("a\nb\n")
            .iter()
            .filter(|t| t.type_ == TokenType::Newline)
            .count();
        assert_eq!(top, 2);
    }

    // --- Literals and errors --------------------------------------------

    #[test]
    fn numbers_strings_and_symbols() {
        pair(&lex("3.14\n"), 0, "NUMBER", "3.14");
        // The lexer strips the surrounding quotes; the value is the contents.
        pair(&lex("\"hi\"\n"), 0, "STRING", "hi");
        // Case-sensitive: `Plus` and `plus` are distinct symbols (both NAME).
        pair(&lex("Plus\n"), 0, "NAME", "Plus");
        pair(&lex("plus\n"), 0, "NAME", "plus");
    }

    #[test]
    fn unknown_char_is_an_error() {
        assert!(try_tokenize_wolfram("x ~ y\n").is_err());
    }
}
