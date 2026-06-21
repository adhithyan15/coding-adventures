//! Automatic Semicolon Insertion (ASI) — Phase 1: the `}` / end-of-input rule.
//!
//! # Why this module exists
//!
//! ECMAScript lets you omit many statement-ending semicolons; the language
//! inserts them automatically (§12.10). closurec's grammar, however, spells
//! `SEMICOLON` out as a *required* terminal in every statement production, so
//! ordinary semicolon-light source fails to parse:
//!
//! ```text
//!   function f(x) { if (x) { return 1 } else { g() } }
//!                                     ^^^ no `;` before `}`  →  parse error
//! ```
//!
//! When that happens, closurec degrades the *whole program* to
//! `WHITESPACE_ONLY` — every optimization pass is skipped. ASI closes that gap.
//!
//! # The approach: retry-on-parse-error (byte-identical by construction)
//!
//! Phase 1 implements only ASI **Rule 2** — a `;` is inserted before a `}` (or
//! at end of input) that would otherwise be a syntax error. This rule does *not*
//! depend on line terminators, which is convenient because the lexer discards
//! newlines as trivia (the line-terminator rule is Phase 2, using the
//! `TOKEN_PRECEDED_BY_NEWLINE` flag the lexer already records).
//!
//! Rather than guess insertion points from a lookahead table (the approach the
//! design spec first sketched), we drive insertion from the parser itself:
//!
//! 1. Parse the token stream.
//! 2. If it parses, we are done — **nothing is inserted**.
//! 3. If it fails *specifically because a `SEMICOLON` was expected before a `}`
//!    or end-of-input*, synthesize a `;` at that position and re-parse.
//! 4. Any other failure is returned unchanged (closurec then degrades as before).
//!
//! The decisive property: a semicolon is inserted **only when parsing genuinely
//! failed for lack of one**. Therefore ASI is a *no-op on any input that already
//! parses* — it can never change a valid program's parse. This is what makes the
//! transform safe to drop into the parse path: every existing fixture (which all
//! use explicit semicolons) is byte-for-byte unaffected, and the only programs
//! whose behaviour changes are ones that previously failed outright.
//!
//! The cost is re-parsing once per inserted semicolon (O(insertions × n)). For a
//! build-time minifier this is acceptable; a batched single-pass insertion is a
//! possible future optimization, but correctness-by-construction comes first.

use coding_adventures_javascript_tokens::EsVersion;
use lexer::token::{Token, TokenType};
use parser::grammar_parser::{GrammarASTNode, GrammarParseError, GrammarParser};

/// Parse `tokens` with Phase-1 ASI applied.
///
/// Returns the parsed tree on success, or the final [`GrammarParseError`] if the
/// program cannot be parsed even with `}`/EOF semicolons supplied (so the caller
/// still degrades exactly as it did before ASI existed).
pub fn parse_with_asi(
    tokens: Vec<Token>,
    version: EsVersion,
) -> Result<GrammarASTNode, GrammarParseError> {
    let grammar = crate::_grammar::parser_grammar(version.as_str())
        .expect("compiled JavaScript parser grammar missing supported version");

    let mut tokens = tokens;
    // Each insertion adds exactly one token and (when it helps) lets the parser
    // advance past one more `}`/EOF, so the number of useful insertions is
    // bounded by the token count. The `+ 1` covers the final EOF position.
    let max_insertions = tokens.len() + 1;
    // Positions we have already tried inserting a `;` before. If a failure
    // recurs at a position we have already addressed — even non-consecutively
    // (an A→B→A oscillation) — the inserted `;` is not making progress, so we
    // stop rather than churn up to the hard cap. With correct insertion the
    // failure position strictly advances, so this set only guards pathological
    // input; the `max_insertions` cap is the hard backstop.
    let mut tried: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();

    for _ in 0..max_insertions {
        let mut parser = GrammarParser::new(tokens.clone(), grammar.clone());
        match parser.parse() {
            Ok(ast) => return Ok(ast),
            Err(e) => {
                if !is_asi_recoverable(&e) {
                    return Err(e);
                }
                let fail_pos = (e.token.line, e.token.column);
                if !tried.insert(fail_pos) {
                    // Already inserted a `;` for this position and parsing still
                    // fails here — give up with the real error.
                    return Err(e);
                }
                match find_token_index(&tokens, &e.token) {
                    Some(idx) => tokens.insert(idx, synthetic_semicolon(&e.token)),
                    // The offending token isn't locatable in our stream (should
                    // not happen — full-identity match is unique); bail safely.
                    None => return Err(e),
                }
            }
        }
    }

    // Budget exhausted (pathological input). Return the final outcome so the
    // caller sees a real error rather than a silent wrong parse.
    GrammarParser::new(tokens, grammar).parse()
}

/// Is this parse failure one that inserting a `;` before a `}` / end-of-input
/// could plausibly fix? We require BOTH that a `SEMICOLON` was among the
/// expected tokens AND that the offending token is a closing brace or EOF —
/// the only positions ASI Rule 2 applies. Anything else is a genuine syntax
/// error we must not paper over.
fn is_asi_recoverable(e: &GrammarParseError) -> bool {
    let expects_semicolon = e.message.contains("SEMICOLON");
    let off = &e.token;
    let is_close_brace = off.type_ == TokenType::RBrace
        || off.type_name.as_deref() == Some("RBRACE")
        || off.value == "}";
    let is_eof = off.type_ == TokenType::Eof;
    expects_semicolon && (is_close_brace || is_eof)
}

/// A synthesized `;` token positioned at the offending token. The parser matches
/// tokens by `type_name` first, so `type_name = "SEMICOLON"` is what makes this
/// satisfy a `SEMICOLON` grammar reference; `type_` and `value` are filled in to
/// match a real semicolon for any consumer that inspects them.
fn synthetic_semicolon(offending: &Token) -> Token {
    Token {
        type_: TokenType::Semicolon,
        value: ";".to_string(),
        line: offending.line,
        column: offending.column,
        type_name: Some("SEMICOLON".to_string()),
        flags: None,
    }
}

/// Locate the offending token in the stream. We match on **full token
/// identity** — kind (`type_`), text (`value`), and position (`line`,
/// `column`) — not position alone. This matters because the synthetic `;` we
/// insert is deliberately stamped with the offending token's own `(line,
/// column)` (good for diagnostics/source maps), so a position-only match could
/// later find that injected `;` instead of a real `}`/EOF. A `;` differs from a
/// `}`/EOF in `type_` and `value`, so including those fields keeps the match
/// unambiguous even when positions collide. Returns the index to insert the
/// synthetic `;` *before*.
fn find_token_index(tokens: &[Token], offending: &Token) -> Option<usize> {
    tokens.iter().position(|t| {
        t.line == offending.line
            && t.column == offending.column
            && t.type_ == offending.type_
            && t.value == offending.value
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(src: &str) -> Vec<Token> {
        coding_adventures_javascript_lexer::tokenize_javascript_typed(src, EsVersion::Es2025)
            .expect("tokenize")
    }

    /// Count how many SEMICOLON tokens ASI would add by diffing the stream the
    /// parser ultimately accepts against the original. We re-run the insertion
    /// loop indirectly via `parse_with_asi`; here we just assert parse success.
    fn parses_with_asi(src: &str) -> bool {
        parse_with_asi(tokenize(src), EsVersion::Es2025).is_ok()
    }

    fn parses_without_asi(src: &str) -> bool {
        let grammar = crate::_grammar::parser_grammar("es2025").unwrap();
        GrammarParser::new(tokenize(src), grammar).parse().is_ok()
    }

    #[test]
    fn return_before_close_brace_is_recovered() {
        // No `;` before the `}` — fails without ASI, parses with it.
        let src = "function f(){return 1}";
        assert!(!parses_without_asi(src), "precondition: should fail raw");
        assert!(parses_with_asi(src), "ASI should insert a semicolon before the close brace");
    }

    #[test]
    fn call_before_close_brace_is_recovered() {
        let src = "function f(){g()}";
        assert!(!parses_without_asi(src));
        assert!(parses_with_asi(src));
    }

    #[test]
    fn expression_statement_at_eof_is_recovered() {
        // Top-level expression statement with no trailing `;` at end of input.
        let src = "a()";
        assert!(!parses_without_asi(src));
        assert!(parses_with_asi(src));
    }

    #[test]
    fn already_valid_input_is_a_noop_passthrough() {
        // With explicit semicolons the program already parses; ASI must not
        // change anything — it parses, and the FIRST parse (no insertion) wins.
        let src = "function f(){return 1;}";
        assert!(parses_without_asi(src), "precondition: valid as-is");
        assert!(parses_with_asi(src));
    }

    #[test]
    fn idempotent_on_recovered_input() {
        // Running ASI is stable: a stream that needed one insertion still parses
        // (and the inserted `;` doesn't trigger further spurious insertions).
        let src = "function f(){if(x){return 1}else{g()}}";
        assert!(parses_with_asi(src));
    }

    #[test]
    fn genuine_syntax_error_is_not_papered_over() {
        // A real error (mismatched paren) is NOT a missing-semicolon case, so
        // ASI must return the error rather than loop or wrongly "recover".
        let src = "function f({)}";
        assert!(!parses_with_asi(src), "non-ASI syntax error must still fail");
    }

    #[test]
    fn empty_block_is_not_given_a_semicolon() {
        // `function f(){}` already parses; ASI must not insert into `{}`.
        let src = "function f(){}";
        assert!(parses_without_asi(src));
        assert!(parses_with_asi(src));
    }
}
