//! # MATLAB Lexer — tokenizing the MATLAB language.
//!
//! MATLAB (Cleve Moler, ~1979 — "MATrix LABoratory") is the matrix-laboratory
//! language whose only data type is the array. This crate is the lexical layer
//! of the MATLAB frontend on `array-runtime`; see
//! `code/specs/MA01-matlab-language.md`. Like every language frontend in this
//! repo it does not hand-write the bulk of tokenization — it loads the compiled
//! `matlab.tokens` grammar and feeds it to the generic [`GrammarLexer`]. What it
//! *does* hand-write are the two context-sensitive rules a regex grammar cannot
//! express.
//!
//! ## The hard one: `'` is transpose *and* a string delimiter
//!
//! A single quote is the (conjugate) transpose operator after a value, but the
//! char-array delimiter otherwise (`A'` transposes `A`; `'abc'` is a string;
//! `A' * B'` is two transposes, *not* the string `' * B'`). MATLAB's rule: `'`
//! is **transpose when a value-terminator immediately precedes it** — an
//! identifier, number, string, or a closing `)`/`]`/`}` or postfix `'` with **no
//! intervening whitespace** — and **starts a string otherwise**. Whitespace
//! resets the context, which is why `[1 'a']` reads `'a'` as a string while
//! `A'` is a transpose.
//!
//! A regex token grammar can't see "the previous token", so a
//! [pre-tokenize hook](GrammarLexer::add_pre_tokenize) resolves it *before* the
//! grammar runs ([`protect_quotes`]): transpose quotes are left as bare `'`
//! (lexed as `TRANSPOSE`), and each char-array literal is rewritten to a
//! `` `N` `` backtick placeholder (lexed as `CHARARRAY` → `STRING`). Backtick is
//! not a MATLAB token, so it only ever appears as that placeholder. A
//! post-tokenize hook ([`restore_quotes`]) swaps the placeholder back for the
//! original source text. The same pre-pass splices `...` line continuations and
//! a sibling pre-pass ([`strip_block_comments`]) removes `%{ %}` blocks.
//!
//! ## The inverted newline rule
//!
//! Like S/R, a newline ends a statement and is insignificant inside `( )`. But
//! **unlike** S/R, a newline inside `[ ]`/`{ }` is significant — it separates
//! matrix/cell rows. So [`drop_paren_newlines`] tracks only **parenthesis**
//! depth and keeps bracket/brace-interior newlines.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::{Token, TokenType};
use std::cell::RefCell;
use std::rc::Rc;
mod _grammar;

/// Shared store of the original char-array lexemes, indexed by the integer in
/// the `` `N` `` placeholder. The pre-hook fills it; the post-hook reads it.
type LiteralTable = Rc<RefCell<Vec<String>>>;

/// Strip `%{ … %}` block comments. MATLAB requires the block markers to sit
/// **alone on their lines**, so this is a line-wise pass: a line whose trimmed
/// content is exactly `%{` opens a block and `%}` closes it; lines in between
/// (and the marker lines) are blanked, preserving newlines so line numbers and
/// the char offsets the next pass relies on stay aligned.
fn strip_block_comments(source: String) -> String {
    let mut out = String::with_capacity(source.len());
    let mut in_block = false;
    for line in source.split_inclusive('\n') {
        // Separate the line body from its trailing newline (if any).
        let (body, nl) = match line.strip_suffix('\n') {
            Some(b) => (b, "\n"),
            None => (line, ""),
        };
        let trimmed = body.trim();
        if !in_block && trimmed == "%{" {
            in_block = true;
            out.push_str(nl);
        } else if in_block && trimmed == "%}" {
            in_block = false;
            out.push_str(nl);
        } else if in_block {
            out.push_str(nl);
        } else {
            out.push_str(body);
            out.push_str(nl);
        }
    }
    out
}

/// Resolve the transpose/char-array ambiguity and splice `...` continuations.
///
/// Walks the source once, tracking whether the immediately preceding character
/// was a value-terminator (`prev_value`) — reset by any whitespace, newline, or
/// operator. A `'` is left in place (a transpose) when `prev_value` holds, and
/// otherwise consumes a char-array literal (honouring the `''` escape) which is
/// recorded in `table` and replaced by a `` `N` `` placeholder. Double-quoted
/// strings and `%` line comments are passed through verbatim (their interior
/// quotes must not be reinterpreted). `...` followed by the rest of the line and
/// its newline is dropped, joining the two physical lines.
fn protect_quotes(source: String, table: &LiteralTable) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    let n = chars.len();
    let mut prev_value = false;

    while i < n {
        let c = chars[i];
        match c {
            // `...` line continuation: drop it and the rest of the line + newline.
            '.' if i + 2 < n && chars[i + 1] == '.' && chars[i + 2] == '.' => {
                i += 3;
                while i < n && chars[i] != '\n' {
                    i += 1;
                }
                if i < n {
                    i += 1; // consume the newline too — the lines join
                }
                // prev_value is unchanged: the expression continues.
            }
            // `%` line comment: copy verbatim to end of line (grammar skips it).
            '%' => {
                while i < n && chars[i] != '\n' {
                    out.push(chars[i]);
                    i += 1;
                }
            }
            // Double-quoted string: copy whole, respecting the `""` escape.
            '"' => {
                out.push('"');
                i += 1;
                while i < n {
                    if chars[i] == '"' {
                        if i + 1 < n && chars[i + 1] == '"' {
                            out.push_str("\"\"");
                            i += 2;
                        } else {
                            out.push('"');
                            i += 1;
                            break;
                        }
                    } else {
                        out.push(chars[i]);
                        i += 1;
                    }
                }
                prev_value = true; // a string is a value
            }
            '\'' if prev_value => {
                // Transpose: leave the quote for the grammar to lex as TRANSPOSE.
                out.push('\'');
                i += 1;
                prev_value = true; // the transposed result is itself a value
            }
            '\'' => {
                // Char-array literal: consume to the matching `'` (with `''`
                // escape), record it, and emit a placeholder.
                let mut lit = String::from("'");
                i += 1;
                loop {
                    if i >= n {
                        break; // unterminated — emit what we have, grammar/parser will error
                    }
                    if chars[i] == '\'' {
                        if i + 1 < n && chars[i + 1] == '\'' {
                            lit.push_str("''");
                            i += 2;
                        } else {
                            lit.push('\'');
                            i += 1;
                            break;
                        }
                    } else {
                        lit.push(chars[i]);
                        i += 1;
                    }
                }
                let idx = {
                    let mut t = table.borrow_mut();
                    t.push(lit);
                    t.len() - 1
                };
                out.push('`');
                out.push_str(&idx.to_string());
                out.push('`');
                prev_value = true; // a string is a value
            }
            ' ' | '\t' => {
                out.push(c);
                i += 1;
                prev_value = false; // whitespace breaks the transpose context
            }
            '\n' | '\r' => {
                out.push(c);
                i += 1;
                prev_value = false;
            }
            // Value-terminators with no preceding whitespace: a following `'` is
            // a transpose. `.` is included so `A.'` reads `.'` as transpose.
            _ => {
                out.push(c);
                i += 1;
                prev_value =
                    c.is_alphanumeric() || c == '_' || c == ')' || c == ']' || c == '}' || c == '.';
            }
        }
    }
    out
}

/// Decode a char-array lexeme to its content: drop the surrounding quotes and
/// collapse each doubled `''` to a single `'`. So `'it''s'` → `it's`. This
/// matches how the grammar lexer already resolves double-quoted strings
/// (`"hello"` → `hello`), so every `STRING` token carries resolved content.
fn decode_char_array(raw: &str) -> String {
    let inner = raw
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(raw);
    inner.replace("''", "'")
}

/// Swap each `` `N` `` placeholder STRING token back for the decoded content of
/// the original char-array lexeme recorded in `table`.
fn restore_quotes(mut tokens: Vec<Token>, table: &LiteralTable) -> Vec<Token> {
    let lits = table.borrow();
    for tok in &mut tokens {
        if tok.effective_type_name() == "STRING" {
            let v = &tok.value;
            if let Some(inner) = v.strip_prefix('`').and_then(|s| s.strip_suffix('`')) {
                if let Ok(idx) = inner.parse::<usize>() {
                    if let Some(orig) = lits.get(idx) {
                        tok.value = decode_char_array(orig);
                    }
                }
            }
        }
    }
    tokens
}

/// Drop `NEWLINE` tokens inside open `( )` only. Inside `[ ]`/`{ }` a newline
/// separates matrix/cell rows and is kept — the inverse of the S/R rule, so we
/// track parenthesis depth alone.
fn drop_paren_newlines(tokens: Vec<Token>) -> Vec<Token> {
    let mut result = Vec::with_capacity(tokens.len());
    let mut paren_depth: i32 = 0;
    for tok in tokens {
        if tok.type_ == TokenType::Newline && paren_depth > 0 {
            continue;
        }
        match tok.type_ {
            TokenType::LParen => paren_depth += 1,
            TokenType::RParen => paren_depth = paren_depth.saturating_sub(1),
            _ => {}
        }
        result.push(tok);
    }
    result
}

/// Create a [`GrammarLexer`] configured for MATLAB source, with the block-comment
/// strip, quote-disambiguation, placeholder-restore, and paren-newline hooks.
pub fn create_matlab_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    let mut lexer = GrammarLexer::new(source, &grammar);
    let table: LiteralTable = Rc::new(RefCell::new(Vec::new()));

    lexer.add_pre_tokenize(Box::new(strip_block_comments));
    let pre_table = Rc::clone(&table);
    lexer.add_pre_tokenize(Box::new(move |src| protect_quotes(src, &pre_table)));

    let post_table = Rc::clone(&table);
    lexer.add_post_tokenize(Box::new(move |toks| restore_quotes(toks, &post_table)));
    lexer.add_post_tokenize(Box::new(drop_paren_newlines));
    lexer
}

/// Tokenize MATLAB source text into a vector of tokens (ending in `EOF`).
///
/// # Panics
///
/// Panics on an unrecognized character. Use [`try_tokenize_matlab`] for a
/// `Result`.
///
/// # Example
///
/// ```
/// use coding_adventures_matlab_lexer::tokenize_matlab;
/// let tokens = tokenize_matlab("A'\n");
/// assert_eq!(tokens[1].effective_type_name(), "TRANSPOSE"); // A then transpose
/// ```
pub fn tokenize_matlab(source: &str) -> Vec<Token> {
    create_matlab_lexer(source)
        .tokenize()
        .unwrap_or_else(|e| panic!("MATLAB tokenization failed: {e}"))
}

/// Tokenize MATLAB source text, returning a `Result` instead of panicking.
pub fn try_tokenize_matlab(source: &str) -> Result<Vec<Token>, String> {
    create_matlab_lexer(source)
        .tokenize()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tokenize, dropping EOF and NEWLINE, into (effective-type, value) pairs.
    fn lex(source: &str) -> Vec<(String, String)> {
        tokenize_matlab(source)
            .iter()
            .filter(|t| t.type_ != TokenType::Eof && t.type_ != TokenType::Newline)
            .map(|t| (t.effective_type_name().to_string(), t.value.clone()))
            .collect()
    }

    fn types(source: &str) -> Vec<String> {
        lex(source).into_iter().map(|(t, _)| t).collect()
    }

    fn pair(p: &[(String, String)], i: usize, ty: &str, val: &str) {
        assert_eq!(
            (p[i].0.as_str(), p[i].1.as_str()),
            (ty, val),
            "tok {i} in {p:?}"
        );
    }

    // --- The defining problem: `'` transpose vs char array --------------

    #[test]
    fn transpose_after_a_value() {
        pair(&lex("A'\n"), 1, "TRANSPOSE", "'");
        // No space: `(A)'`, `x(1)'`, and `A''` (double transpose) are transposes.
        assert_eq!(types("(A)'\n"), ["LPAREN", "NAME", "RPAREN", "TRANSPOSE"]);
        assert_eq!(types("A''\n"), ["NAME", "TRANSPOSE", "TRANSPOSE"]);
    }

    #[test]
    fn char_array_when_not_after_a_value() {
        // At the start, after `=`, after an operator → string. STRING values are
        // the decoded content (quotes stripped), matching double-quoted strings.
        pair(&lex("'abc'\n"), 0, "STRING", "abc");
        pair(&lex("x = 'hi'\n"), 2, "STRING", "hi");
        // The classic trap: `A' * B'` is two transposes, NOT a string.
        assert_eq!(
            types("A' * B'\n"),
            ["NAME", "TRANSPOSE", "STAR", "NAME", "TRANSPOSE"]
        );
    }

    #[test]
    fn whitespace_resets_to_string_inside_brackets() {
        // `[1 'a']`: the space before `'` means it starts a string, not a
        // transpose of `1`.
        assert_eq!(
            types("[1 'a']\n"),
            ["LBRACKET", "NUMBER", "STRING", "RBRACKET"]
        );
    }

    #[test]
    fn doubled_quote_is_an_escaped_apostrophe() {
        // `'it''s'` is the single char array  it's  — `''` decodes to one `'`.
        pair(&lex("x = 'it''s'\n"), 2, "STRING", "it's");
    }

    #[test]
    fn double_quoted_strings() {
        // The grammar lexer resolves double-quoted strings to their content.
        pair(&lex("\"hello\"\n"), 0, "STRING", "hello");
        // A `'` inside a double-quoted string is just text, not a transpose.
        pair(&lex("\"it's\"\n"), 0, "STRING", "it's");
    }

    // --- Element-wise vs matrix operators -------------------------------

    #[test]
    fn elementwise_operators() {
        assert_eq!(
            types("A .* B ./ C .^ D .\\ E\n"),
            [
                "NAME",
                "ELEM_MUL",
                "NAME",
                "ELEM_RDIV",
                "NAME",
                "ELEM_POW",
                "NAME",
                "ELEM_LDIV",
                "NAME"
            ]
        );
        // `.'` is the non-conjugate transpose.
        assert_eq!(types("A.'\n"), ["NAME", "ELEM_TRANSPOSE"]);
    }

    #[test]
    fn number_dot_star_is_elementwise_not_a_trailing_dot() {
        // `3.*4` must be 3 .* 4, never `3.` `*` `4`.
        assert_eq!(types("3.*4\n"), ["NUMBER", "ELEM_MUL", "NUMBER"]);
    }

    #[test]
    fn matrix_operators_and_backslash() {
        assert_eq!(
            types("A * B / C \\ D ^ 2\n"),
            [
                "NAME",
                "STAR",
                "NAME",
                "SLASH",
                "NAME",
                "BACKSLASH",
                "NAME",
                "CARET",
                "NUMBER"
            ]
        );
    }

    #[test]
    fn double_quoted_escape_and_no_trailing_newline() {
        // The `""` escape inside a double-quoted string is preserved through the
        // pre-pass (the grammar lexer resolves it). And a source with no trailing
        // newline tokenizes fine.
        assert_eq!(types("\"a\"\"b\"\n"), ["STRING"]);
        assert_eq!(types("x"), ["NAME"]);
        // A `...` continuation at end-of-input is harmless.
        assert_eq!(types("1 + ..."), ["NUMBER", "PLUS"]);
    }

    // --- Numbers --------------------------------------------------------

    #[test]
    fn numbers() {
        pair(&lex("3.5\n"), 0, "NUMBER", "3.5");
        pair(&lex(".5\n"), 0, "NUMBER", ".5");
        pair(&lex("1e3\n"), 0, "NUMBER", "1e3");
        pair(&lex("2.5e-3\n"), 0, "NUMBER", "2.5e-3");
    }

    // --- Comparison / logical -------------------------------------------

    #[test]
    fn comparison_and_logical_operators() {
        assert_eq!(
            types("a == b ~= c <= d >= e && f || g\n"),
            [
                "NAME", "EQ_EQ", "NAME", "NE", "NAME", "LE", "NAME", "GE", "NAME", "AND_AND",
                "NAME", "OR_OR", "NAME"
            ]
        );
        assert_eq!(types("~x\n"), ["TILDE", "NAME"]);
    }

    // --- Matrix literals, ranges, indexing ------------------------------

    #[test]
    fn matrix_literal_and_range() {
        assert_eq!(
            types("[1 2; 3 4]\n"),
            [
                "LBRACKET",
                "NUMBER",
                "NUMBER",
                "SEMICOLON",
                "NUMBER",
                "NUMBER",
                "RBRACKET"
            ]
        );
        assert_eq!(
            types("1:2:10\n"),
            ["NUMBER", "COLON", "NUMBER", "COLON", "NUMBER"]
        );
    }

    #[test]
    fn keywords_and_end() {
        for kw in [
            "if", "elseif", "else", "end", "for", "while", "function", "switch", "return",
        ] {
            assert!(
                tokenize_matlab(&format!("{kw}\n"))
                    .iter()
                    .any(|t| t.effective_type_name() == "KEYWORD" && t.value == *kw),
                "expected KEYWORD({kw})"
            );
        }
    }

    // --- Newlines: dropped in ( ), kept in [ ] --------------------------

    #[test]
    fn newlines_dropped_in_parens_kept_in_brackets() {
        let in_parens = tokenize_matlab("f(1,\n2)\n")
            .iter()
            .filter(|t| t.type_ == TokenType::Newline)
            .count();
        assert_eq!(in_parens, 1, "paren-interior newline should be dropped");
        // Inside [ ], a newline IS a row separator — it must be kept.
        let in_brackets = tokenize_matlab("[1 2\n3 4]\n")
            .iter()
            .filter(|t| t.type_ == TokenType::Newline)
            .count();
        assert_eq!(
            in_brackets, 2,
            "bracket-interior newline (row sep) should be kept"
        );
    }

    // --- Comments and continuation --------------------------------------

    #[test]
    fn line_comment_is_skipped() {
        assert_eq!(types("x = 1 % a comment\n"), ["NAME", "EQ", "NUMBER"]);
    }

    #[test]
    fn block_comment_is_stripped() {
        let src = "x = 1\n%{\nthis is ignored\n'and so is this'\n%}\ny = 2\n";
        assert_eq!(types(src), ["NAME", "EQ", "NUMBER", "NAME", "EQ", "NUMBER"]);
    }

    #[test]
    fn line_continuation_joins_lines() {
        // `1 + ...` then `2` is the single expression `1 + 2` — no newline between.
        assert_eq!(types("1 + ...\n2\n"), ["NUMBER", "PLUS", "NUMBER"]);
    }

    // --- Errors ---------------------------------------------------------

    #[test]
    fn unknown_character_is_an_error() {
        assert!(try_tokenize_matlab("x = #\n").is_err());
    }

    #[test]
    fn handles_eof_mid_string_without_panic() {
        // An unterminated char array must not panic the scanner.
        let _ = try_tokenize_matlab("x = 'oops\n");
    }
}
