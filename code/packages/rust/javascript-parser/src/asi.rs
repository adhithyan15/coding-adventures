//! Automatic Semicolon Insertion (ASI) — the `}` / end-of-input rule (Rule 2)
//! and the line-terminator rule (Rule 1).
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
//! Two ASI rules are implemented (§12.10):
//!
//! * **Rule 2** (`}` / end-of-input) — a `;` is inserted before a `}` (or at end
//!   of input) that would otherwise be a syntax error. No line terminator needed.
//! * **Rule 1** (line terminator) — a `;` is inserted before an offending token
//!   that is **preceded by a line terminator**. The lexer discards newlines as
//!   trivia, but each token records the `line` it starts on, so a line
//!   terminator sits between two tokens exactly when the later one starts on a
//!   higher line (and its single-line predecessor did not itself span lines —
//!   see [`asi_applies_at`]). No shared-lexer change is required.
//!
//! Rather than guess insertion points from a lookahead table (the approach the
//! design spec first sketched), we drive insertion from the parser itself:
//!
//! 1. Parse the token stream.
//! 2. If it parses, we are done — **nothing is inserted**.
//! 3. If it fails *specifically because a `SEMICOLON` was expected* at a point
//!    where Rule 1 or Rule 2 applies, synthesize a `;` there and re-parse.
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
                // A `;` must be among what the parser expected here, or this is
                // not a missing-semicolon situation at all.
                if !e.message.contains("SEMICOLON") {
                    return Err(e);
                }
                // Locate the offending token so we can inspect its predecessor
                // (the line-terminator rule needs it).
                let idx = match find_token_index(&tokens, &e.token) {
                    Some(i) => i,
                    // Not locatable (should not happen — full-identity match is
                    // unique); bail safely.
                    None => return Err(e),
                };
                if !asi_applies_at(&tokens, idx) {
                    return Err(e);
                }
                let fail_pos = (e.token.line, e.token.column);
                if !tried.insert(fail_pos) {
                    // Already inserted a `;` for this position and parsing still
                    // fails here — give up with the real error.
                    return Err(e);
                }
                tokens.insert(idx, synthetic_semicolon(&e.token));
            }
        }
    }

    // Budget exhausted (pathological input). Return the final outcome so the
    // caller sees a real error rather than a silent wrong parse.
    GrammarParser::new(tokens, grammar).parse()
}

/// Does an ASI insertion apply *before the token at `idx`* (which the parser
/// rejected while expecting a `SEMICOLON`)? Two ECMAScript §12.10 rules:
///
/// * **Rule 2** — the offending token is a `}` or end-of-input. A statement may
///   always be terminated there; no line terminator is required.
/// * **Rule 1** — the offending token is **preceded by a line terminator**.
///   The lexer discards newlines as trivia, but every token still records the
///   `line` it starts on, so a line terminator sits between `tokens[idx-1]` and
///   `tokens[idx]` exactly when the offending token starts on a *later line*
///   than its predecessor. We only trust that comparison when the predecessor
///   is **single-line** (its own text contains no newline); a multi-line
///   predecessor — e.g. a template literal spanning lines — makes the
///   start-line comparison ambiguous, so we conservatively decline (a missed
///   optimization, never a miscompile).
///
/// Soundness: this is only ever consulted on a genuine parse *failure*, so a
/// program that already parses is untouched. Requiring an actual line
/// terminator for the non-`}`/EOF case is what keeps a true one-line syntax
/// error (`a=1 b=2`) from being silently "recovered".
fn asi_applies_at(tokens: &[Token], idx: usize) -> bool {
    let off = &tokens[idx];

    // Rule 2: before a `}` or EOF.
    if off.type_ == TokenType::RBrace
        || off.type_name.as_deref() == Some("RBRACE")
        || off.value == "}"
        || off.type_ == TokenType::Eof
    {
        return true;
    }

    // Rule 1: a line terminator between the predecessor and this token.
    if idx == 0 {
        return false;
    }
    let prev = &tokens[idx - 1];
    off.line > prev.line && !token_may_span_lines(prev)
}

/// Could this token's lexeme cross source lines *without that being visible in
/// its `value`* — making the start-line comparison in [`asi_applies_at`]
/// unreliable?
///
/// The lexer stores the **cooked** text in `value` (escapes resolved), so a
/// token can span multiple source lines while `value` contains no raw newline:
///
/// * a **string** can use a backslash line-continuation (`"a\<LF>b"` → `"ab"`),
/// * **template literals** and **regex** can embed or escape newlines.
///
/// For any such kind we cannot conclude from `off.line > prev.line` that a real
/// line terminator separates the two tokens (the higher line may just be the
/// predecessor's own continuation), so we treat it as line-spanning and decline
/// Rule 1 — a missed optimization, never a miscompile. (A token whose `value`
/// already contains a raw newline is obviously multi-line and likewise
/// declined.) Identifiers, numbers, punctuators, and keywords are always
/// single-line, so the start-line comparison is exact for them.
///
/// MAINTENANCE: this enumerates the lexer's only multi-line-capable token
/// kinds. If a future grammar adds another (e.g. a token-preserving multi-line
/// comment, or a heredoc/raw-string variant), it MUST be added here or Rule 1
/// could wrongly fire across it.
fn token_may_span_lines(t: &Token) -> bool {
    matches!(t.type_, TokenType::String)
        || matches!(t.type_name.as_deref(), Some("STRING") | Some("REGEX"))
        || t.type_name.as_deref().is_some_and(|n| n.starts_with("TEMPLATE"))
        || t.value.contains('\n')
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

    // --- Phase 2: the line-terminator rule (ECMAScript Rule 1) ------------

    #[test]
    fn statements_separated_by_a_newline_are_recovered() {
        // `a = 1` then `b = 2` on the next line: the offending token `b` is
        // preceded by a line terminator, so ASI inserts a `;` after `1`.
        let src = "a = 1\nb = 2";
        assert!(!parses_without_asi(src), "precondition: should fail raw");
        assert!(parses_with_asi(src), "ASI Rule 1 should split across the newline");
    }

    #[test]
    fn two_statements_on_one_line_are_not_recovered() {
        // No line terminator between `1` and `b`: this is a genuine syntax
        // error (ASI does NOT apply mid-line), so it must stay a failure — we
        // must not paper it over.
        let src = "a = 1 b = 2";
        assert!(!parses_with_asi(src), "one-line `a=1 b=2` is a real error, not ASI");
    }

    #[test]
    fn newline_after_assignment_keyword_call() {
        // A multi-line sequence of statements with no semicolons anywhere.
        let src = "var x = f()\nvar y = g()\nh(x, y)";
        assert!(!parses_without_asi(src));
        assert!(parses_with_asi(src));
    }

    #[test]
    fn valid_multiline_program_is_a_noop() {
        // Already valid (explicit semicolons), spanning lines: ASI must not
        // change the parse — it succeeds on the first try.
        let src = "var x = 1;\nvar y = 2;\nh(x, y);";
        assert!(parses_without_asi(src), "precondition: valid as-is");
        assert!(parses_with_asi(src));
    }

    #[test]
    fn continuation_on_next_line_is_not_split() {
        // A binary expression continued on the next line is ONE statement; the
        // `+` can continue it, so the parser does not fail at `b` and ASI never
        // fires. (`a + b` is a single expression statement.) This guards against
        // wrongly splitting a legal multi-line expression.
        let src = "var a = 1, b = 2;\nvar c = a\n+ b;";
        assert!(parses_with_asi(src));
    }

    #[test]
    fn statement_ending_in_a_string_before_a_newline_is_declined() {
        // Documented Phase-2 limitation: when the token before the newline is a
        // STRING (a kind that *could* span source lines while its cooked value
        // hides it), `asi_applies_at` conservatively declines Rule 1 rather than
        // trust the start-line comparison. So this does NOT recover — a missed
        // optimization, never a miscompile. (Recoverable later with token
        // end-position tracking.) `token_may_span_lines` is what makes the
        // line-terminator heuristic sound regardless of lexer escape handling.
        let src = "var s = \"x\"\nlog(s)";
        assert!(!parses_without_asi(src), "precondition: should fail raw");
        assert!(
            !parses_with_asi(src),
            "string predecessor is conservatively declined for Rule 1"
        );
    }
}
