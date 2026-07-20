//! # Scilab Lexer — tokenizing the Scilab numerical/array language.
//!
//! Scilab (INRIA/ENPC "Meta2" project, 1990) looks, on the surface, like a
//! close cousin of MATLAB — matrix literals, ranges, the operator-precedence
//! cascade, and indexing all carry over — but MA10 §1 documents that it
//! genuinely is not a thin MATLAB respelling the way Octave is: at least one
//! shared piece of surface syntax (`+` on strings) means something different
//! at *runtime* in the two languages, which is why Scilab gets its own
//! grammar/lexer/parser/runtime rather than a text-rewrite shim onto
//! `matlab-runtime` (see `code/specs/MA10-scilab-language.md`). This crate is
//! the lexical layer, forked from `matlab-lexer` at the **grammar-source**
//! level (`code/grammars/scilab/scilab.tokens` is a copy-then-diverge of
//! `matlab.tokens`) — not a Rust-crate dependency; this crate does not import
//! `matlab-lexer` at all (MA10 §5).
//!
//! Like every language frontend in this repo, it does not hand-write the
//! bulk of tokenization — it loads the compiled `scilab.tokens` grammar and
//! feeds it to the generic [`GrammarLexer`]. What it *does* hand-write is the
//! one context-sensitive rule a regex grammar cannot express (the same one
//! MATLAB needs), plus two mechanical hooks matlab-lexer also needs for
//! newline significance.
//!
//! ## The hard one: `'` is transpose *and* a string delimiter
//!
//! Exactly as in MATLAB (MA01 §3), a single quote is the (conjugate)
//! transpose operator after a value, but the string delimiter otherwise (`A'`
//! transposes `A`; `'abc'` is a string; `A' * B'` is two transposes, *not*
//! the string `' * B'`). The rule: `'` is **transpose when a
//! value-terminator immediately precedes it** — an identifier, number,
//! string, `$` (the last-index token — itself a value, MA10 §1 finding 5),
//! or a closing `)`/`]`/`}`/postfix `'`, with **no intervening whitespace**
//! — and **starts a string otherwise**.
//!
//! MA10 §3 explicitly calls out that this is the same *strategy* as MA01 §3,
//! reimplemented independently here (not shared code): a
//! [pre-tokenize hook](GrammarLexer::add_pre_tokenize) resolves it before the
//! grammar runs ([`protect_quotes`]): transpose quotes are left as bare `'`
//! (lexed as `TRANSPOSE`), and each string literal is rewritten to a `` `N` ``
//! backtick placeholder (lexed as the distinct token `STRING_PLACEHOLDER`).
//! A post-tokenize hook ([`restore_placeholders`]) replaces each
//! `STRING_PLACEHOLDER` token with the decoded literal content and re-labels
//! it `STRING` — keying off the *type*, not the *value*, so a double-quoted
//! string whose content looks like a placeholder is never mistaken for one.
//! `"` has no such ambiguity (there is no double-quote transpose in either
//! language), so it needs no *pre*-tokenize hook — `scilab.tokens`'s own
//! `DQ_STRING` pattern matches it directly. It still needs a small
//! *post*-tokenize step ([`collapse_dq_string_escapes`]): the shared
//! `GrammarLexer` engine strips `DQ_STRING`'s outer quotes automatically but
//! does not know Scilab's doubled-`""`-means-one-literal-`"` convention, so
//! this hook collapses `""` to `"` and re-labels the token `STRING`. Unlike
//! MATLAB, both spellings collapse onto the SAME `STRING` token type by
//! construction (MA10 §3): Scilab has no
//! char-array-vs-string-scalar value-model split for a second type to exist.
//!
//! `//` line comments and `/* ... */` block comments are passed through
//! verbatim by this same pre-hook (so a `'` inside one is never
//! reinterpreted as opening a string) — but note MA10 §3's own point that
//! Scilab's block comments need no *separate* stripping pass the way
//! MATLAB's `%{`/`%}` do: there is no "alone on its line" restriction to
//! enforce, so `/* ... */` doubles as an ordinary `skip:` pattern in
//! `scilab.tokens` itself, and passing it through unmolested here is only
//! about protecting its *interior* from the quote-disambiguation scan, not
//! about deciding whether it is a comment at all.
//!
//! ## The newline rule — identical to MATLAB, unlike S/R
//!
//! A newline ends a statement, and is insignificant inside `( )`. But inside
//! `[ ]`/`{ }` a newline separates matrix/cell rows and stays significant
//! (MA10 §1's closing paragraph: matrix literals are inherited verbatim from
//! MATLAB's own rule). So [`drop_paren_newlines`] tracks only **parenthesis**
//! depth and keeps bracket/brace-interior newlines — the same shape as
//! `matlab-lexer`'s own hook of the same name, reimplemented independently.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::{Token, TokenType};
use std::cell::RefCell;
use std::rc::Rc;
mod _grammar;

/// Shared store of the original string lexemes, indexed by the integer in
/// the `` `N` `` placeholder. The pre-hook fills it; the post-hook reads it.
type LiteralTable = Rc<RefCell<Vec<String>>>;

/// Resolve the transpose/string ambiguity for `'`, passing `//`/`/* */`
/// comments and `"..."` strings through untouched.
///
/// Walks the source once, tracking whether the immediately preceding
/// character was a value-terminator (`prev_value`) — reset by whitespace, a
/// newline, or an operator. A `'` is left in place (a transpose) when
/// `prev_value` holds, and otherwise consumes a string literal (honouring
/// the `''` doubled-quote escape) which is recorded in `table` and replaced
/// by a `` `N` `` placeholder.
///
/// Comments and double-quoted strings are copied through character-for-
/// character without being re-scanned for `'` — mirroring matlab-lexer's own
/// `protect_quotes`, a comment/string-interior `'` never touches
/// `prev_value` at all (it is simply whatever it was immediately before the
/// comment/string began), the same "a comment or string is invisible to the
/// value-context tracking" behaviour MATLAB's own hook already has for `%`
/// comments.
fn protect_quotes(source: String, table: &LiteralTable) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    let n = chars.len();
    let mut prev_value = false;

    while i < n {
        let c = chars[i];
        match c {
            // `//` line comment: copy verbatim to end of line (grammar skips it).
            '/' if i + 1 < n && chars[i + 1] == '/' => {
                while i < n && chars[i] != '\n' {
                    out.push(chars[i]);
                    i += 1;
                }
            }
            // `/* ... */` block comment: copy verbatim through the closing
            // `*/` (or to EOF if unterminated -- the parser will honestly
            // reject that later). May span multiple lines and may sit
            // inline with code (MA10 §3) -- unlike MATLAB's `%{`/`%}`, there
            // is no "must be alone on its line" restriction to enforce here.
            '/' if i + 1 < n && chars[i + 1] == '*' => {
                out.push('/');
                out.push('*');
                i += 2;
                while i < n {
                    if chars[i] == '*' && i + 1 < n && chars[i + 1] == '/' {
                        out.push('*');
                        out.push('/');
                        i += 2;
                        break;
                    }
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
                // String literal: consume to the matching `'` (with `''`
                // escape), record it, and emit a placeholder.
                let mut lit = String::from("'");
                i += 1;
                loop {
                    if i >= n {
                        break; // unterminated -- emit what we have, parser will error
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
            // Value-terminators with no preceding whitespace: a following `'`
            // is a transpose. `.` is included so `A.'` reads `.'` as
            // transpose. `$` (the last-index token, MA10 §1 finding 5) is
            // included too -- it denotes a VALUE, the same as a closing
            // bracket, so `A($)'` and a bare `$'` both read as transposes.
            _ => {
                out.push(c);
                i += 1;
                prev_value = c.is_alphanumeric()
                    || c == '_'
                    || c == ')'
                    || c == ']'
                    || c == '}'
                    || c == '.'
                    || c == '$';
            }
        }
    }
    out
}

/// Decode a string lexeme to its content: drop the surrounding quotes and
/// collapse each doubled `''` to a single `'`. So `'it''s'` -> `it's`. This
/// matches how the grammar lexer already resolves double-quoted strings
/// (`"hello"` -> `hello`), so every `STRING` token carries resolved content.
fn decode_string_literal(raw: &str) -> String {
    let inner = raw
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(raw);
    inner.replace("''", "'")
}

/// Replace each `STRING_PLACEHOLDER` token with the decoded content of the
/// original string lexeme recorded in `table`, then re-label it `STRING` so
/// downstream sees single- and double-quoted strings uniformly -- Scilab has
/// only one string type (MA10 §3), unlike MATLAB's CHARARRAY-vs-STRING
/// split. Restoration keys off the distinct `STRING_PLACEHOLDER` *type*,
/// never the value -- so a crafted double-quoted string whose content
/// happens to look like `` `N` `` is never mistaken for a placeholder.
fn restore_placeholders(mut tokens: Vec<Token>, table: &LiteralTable) -> Vec<Token> {
    let lits = table.borrow();
    for tok in &mut tokens {
        if tok.effective_type_name() == "STRING_PLACEHOLDER" {
            if let Some(inner) = tok
                .value
                .strip_prefix('`')
                .and_then(|s| s.strip_suffix('`'))
            {
                if let Ok(idx) = inner.parse::<usize>() {
                    if let Some(orig) = lits.get(idx) {
                        tok.value = decode_string_literal(orig);
                    }
                }
            }
            tok.type_ = TokenType::String;
            tok.type_name = Some("STRING".to_string());
        }
    }
    tokens
}

/// Collapse the `""` doubled-quote escape in `DQ_STRING` tokens and re-label
/// them `STRING`.
///
/// The shared [`GrammarLexer`] engine strips a `DQ_STRING` token's *outer*
/// quotes automatically (any token name ending in `"STRING"` gets that
/// treatment) but does not know about the doubled-`""`-means-one-literal-`"`
/// convention -- that is specific to this grammar, not something the generic
/// engine can infer. So by the time this hook runs, `"a""b"` has already had
/// its outer quotes stripped to `a""b`, and this hook does the one remaining
/// step: collapsing each `""` pair to a single `"`, giving `a"b"`. This
/// mirrors how [`decode_string_literal`] collapses the SAME `''` convention
/// for `STRING_PLACEHOLDER`-derived strings -- kept as a separate pass
/// (rather than folded into [`restore_placeholders`]) specifically so it
/// only ever touches tokens still named `DQ_STRING`, never a
/// `STRING_PLACEHOLDER`-derived value that has already been fully decoded
/// (which might legitimately contain its own `""` substring as ordinary
/// content).
fn collapse_dq_string_escapes(mut tokens: Vec<Token>) -> Vec<Token> {
    for tok in &mut tokens {
        if tok.effective_type_name() == "DQ_STRING" {
            tok.value = tok.value.replace("\"\"", "\"");
            tok.type_ = TokenType::String;
            tok.type_name = Some("STRING".to_string());
        }
    }
    tokens
}

/// Drop `NEWLINE` tokens inside open `( )` only. Inside `[ ]`/`{ }` a newline
/// separates matrix/cell rows and is kept -- identical rule to MATLAB's own
/// matrix-literal grammar (MA10 §1's closing paragraph), so we track
/// parenthesis depth alone, exactly mirroring `matlab-lexer`'s
/// `drop_paren_newlines`.
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

/// Create a [`GrammarLexer`] configured for Scilab source, with the
/// quote-disambiguation, placeholder-restore, and paren-newline hooks.
pub fn create_scilab_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    let mut lexer = GrammarLexer::new(source, &grammar);
    let table: LiteralTable = Rc::new(RefCell::new(Vec::new()));

    let pre_table = Rc::clone(&table);
    lexer.add_pre_tokenize(Box::new(move |src| protect_quotes(src, &pre_table)));

    let post_table = Rc::clone(&table);
    lexer.add_post_tokenize(Box::new(move |toks| restore_placeholders(toks, &post_table)));
    lexer.add_post_tokenize(Box::new(collapse_dq_string_escapes));
    lexer.add_post_tokenize(Box::new(drop_paren_newlines));
    lexer
}

/// Tokenize Scilab source text into a vector of tokens (ending in `EOF`).
///
/// # Panics
///
/// Panics on an unrecognized character. Use [`try_tokenize_scilab`] for a
/// `Result`.
///
/// # Example
///
/// ```
/// use coding_adventures_scilab_lexer::tokenize_scilab;
/// let tokens = tokenize_scilab("A'\n");
/// assert_eq!(tokens[1].effective_type_name(), "TRANSPOSE"); // A then transpose
/// ```
pub fn tokenize_scilab(source: &str) -> Vec<Token> {
    create_scilab_lexer(source)
        .tokenize()
        .unwrap_or_else(|e| panic!("Scilab tokenization failed: {e}"))
}

/// Tokenize Scilab source text, returning a `Result` instead of panicking.
pub fn try_tokenize_scilab(source: &str) -> Result<Vec<Token>, String> {
    create_scilab_lexer(source)
        .tokenize()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tokenize, dropping EOF and NEWLINE, into (effective-type, value) pairs.
    fn lex(source: &str) -> Vec<(String, String)> {
        tokenize_scilab(source)
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

    // --- The defining problem: `'` transpose vs string ------------------

    #[test]
    fn transpose_after_a_value() {
        pair(&lex("A'\n"), 1, "TRANSPOSE", "'");
        assert_eq!(types("(A)'\n"), ["LPAREN", "NAME", "RPAREN", "TRANSPOSE"]);
        assert_eq!(types("A''\n"), ["NAME", "TRANSPOSE", "TRANSPOSE"]);
    }

    #[test]
    fn string_when_not_after_a_value() {
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
        assert_eq!(
            types("[1 'a']\n"),
            ["LBRACKET", "NUMBER", "STRING", "RBRACKET"]
        );
    }

    #[test]
    fn doubled_quote_is_an_escaped_apostrophe() {
        pair(&lex("x = 'it''s'\n"), 2, "STRING", "it's");
    }

    #[test]
    fn last_index_dollar_is_a_value_before_transpose() {
        // `$` denotes the last index (MA10 §1 finding 5) -- a value, so a
        // following `'` with no intervening whitespace is a transpose.
        assert_eq!(types("A($)'\n"), ["NAME", "LPAREN", "DOLLAR", "RPAREN", "TRANSPOSE"]);
    }

    // --- Single- and double-quoted strings are the SAME token type ------

    #[test]
    fn single_and_double_quoted_strings_are_both_string_type() {
        pair(&lex("\"hello\"\n"), 0, "STRING", "hello");
        pair(&lex("'hello'\n"), 0, "STRING", "hello");
        // A `'` inside a double-quoted string is just text, not a transpose.
        pair(&lex("\"it's\"\n"), 0, "STRING", "it's");
    }

    #[test]
    fn double_quoted_string_escape() {
        // The `""` escape inside a double-quoted string.
        assert_eq!(types("\"a\"\"b\"\n"), ["STRING"]);
        pair(&lex("\"a\"\"b\"\n"), 0, "STRING", "a\"b");
    }

    #[test]
    fn double_quoted_string_resembling_a_placeholder_is_not_restored() {
        // Regression: a crafted double-quoted string whose content looks like
        // the internal `` `N` `` placeholder must survive verbatim --
        // restoration keys off the distinct STRING_PLACEHOLDER type, not the
        // value. (An earlier real string is present so index 0 exists.)
        let p = lex("a = 'x'; b = \"`0`\"\n");
        let last_string = p.iter().rev().find(|(t, _)| t == "STRING").unwrap();
        assert_eq!(last_string.1, "`0`");
    }

    // --- Comments: `//` line, `/* */` block, INLINE mid-line-with-code ---

    #[test]
    fn line_comment_is_skipped() {
        assert_eq!(types("x = 1 // a comment\n"), ["NAME", "EQ", "NUMBER"]);
    }

    #[test]
    fn block_comment_is_skipped() {
        assert_eq!(
            types("x = /* ignored */ 1\n"),
            ["NAME", "EQ", "NUMBER"]
        );
    }

    #[test]
    fn block_comment_may_span_multiple_lines() {
        let src = "x = 1\n/*\nthis is ignored\n'and so is this'\n*/\ny = 2\n";
        assert_eq!(types(src), ["NAME", "EQ", "NUMBER", "NAME", "EQ", "NUMBER"]);
    }

    #[test]
    fn block_comment_is_not_required_to_be_alone_on_its_line() {
        // MA10 §3's key divergence from MATLAB's `%{`/`%}`: a block comment
        // may sit INLINE, sharing a line with real code both before and
        // after it -- no "alone on its line" restriction exists here.
        assert_eq!(
            types("x = 1 /* inline */ + 2 /* also inline */ * 3\n"),
            ["NAME", "EQ", "NUMBER", "PLUS", "NUMBER", "STAR", "NUMBER"]
        );
    }

    // --- `%`-prefixed special constants: the closed 8-word vocabulary ----

    #[test]
    fn all_eight_percent_constants_lex_as_one_token_each() {
        for name in ["%pi", "%e", "%i", "%inf", "%nan", "%eps", "%t", "%f"] {
            let p = lex(&format!("{name}\n"));
            assert_eq!(p.len(), 1, "expected exactly one token for {name}: {p:?}");
            pair(&p, 0, "PERCENT_CONST", name);
        }
    }

    #[test]
    fn percent_constants_do_not_swallow_a_trailing_identifier() {
        // `%inf` must win whole -- not split into `%i` + `nf` -- because the
        // regex alternation is ordered longest-spelling-first.
        assert_eq!(types("%inf\n"), ["PERCENT_CONST"]);
        assert_eq!(types("%eps\n"), ["PERCENT_CONST"]);
    }

    #[test]
    fn percent_constant_used_in_an_expression() {
        assert_eq!(
            types("r = %pi * 2\n"),
            ["NAME", "EQ", "PERCENT_CONST", "STAR", "NUMBER"]
        );
    }

    // --- `$` last-index token --------------------------------------------

    #[test]
    fn dollar_last_index() {
        assert_eq!(types("A($)\n"), ["NAME", "LPAREN", "DOLLAR", "RPAREN"]);
        assert_eq!(
            types("A($-1)\n"),
            ["NAME", "LPAREN", "DOLLAR", "MINUS", "NUMBER", "RPAREN"]
        );
    }

    // --- `<>` alongside bare `<`/`>`, and alongside `~=` -----------------

    #[test]
    fn not_equal_digraph_wins_over_bare_lt_gt() {
        assert_eq!(types("a <> b\n"), ["NAME", "NE_ALT", "NAME"]);
        assert_eq!(types("a < b\n"), ["NAME", "LT", "NAME"]);
        assert_eq!(types("a > b\n"), ["NAME", "GT", "NAME"]);
    }

    #[test]
    fn both_not_equal_spellings_are_valid_and_distinct() {
        assert_eq!(types("a ~= b\n"), ["NAME", "NE", "NAME"]);
        assert_eq!(types("a <> b\n"), ["NAME", "NE_ALT", "NAME"]);
    }

    #[test]
    fn le_and_ge_still_win_over_bare_lt_gt() {
        assert_eq!(types("a <= b\n"), ["NAME", "LE", "NAME"]);
        assert_eq!(types("a >= b\n"), ["NAME", "GE", "NAME"]);
    }

    // --- Element-wise vs matrix operators (inherited from MATLAB) -------

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
        assert_eq!(types("A.'\n"), ["NAME", "ELEM_TRANSPOSE"]);
    }

    #[test]
    fn number_dot_star_is_elementwise_not_a_trailing_dot() {
        assert_eq!(types("3.*4\n"), ["NUMBER", "ELEM_MUL", "NUMBER"]);
    }

    #[test]
    fn legacy_double_star_is_not_a_power_operator() {
        // MA10 §4: the deprecated `**` spelling for `^` is deferred by
        // simple omission -- `a ** b` lexes as two bare STAR tokens.
        assert_eq!(types("a ** b\n"), ["NAME", "STAR", "STAR", "NAME"]);
    }

    // --- Matrix literals, ranges (inherited from MATLAB, unchanged) -----

    #[test]
    fn matrix_literal_and_range() {
        assert_eq!(
            types("[1 2; 3 4]\n"),
            [
                "LBRACKET", "NUMBER", "NUMBER", "SEMICOLON", "NUMBER", "NUMBER", "RBRACKET"
            ]
        );
        assert_eq!(
            types("1:2:10\n"),
            ["NUMBER", "COLON", "NUMBER", "COLON", "NUMBER"]
        );
    }

    #[test]
    fn newlines_dropped_in_parens_kept_in_brackets() {
        let in_parens = tokenize_scilab("f(1,\n2)\n")
            .iter()
            .filter(|t| t.type_ == TokenType::Newline)
            .count();
        assert_eq!(in_parens, 1, "paren-interior newline should be dropped");
        let in_brackets = tokenize_scilab("[1 2\n3 4]\n")
            .iter()
            .filter(|t| t.type_ == TokenType::Newline)
            .count();
        assert_eq!(
            in_brackets, 2,
            "bracket-interior newline (row sep) should be kept"
        );
    }

    // --- Keywords: if/select/while/for/break/continue/function/endfunction

    #[test]
    fn keywords_promote_from_name() {
        for kw in [
            "if", "elseif", "else", "end", "select", "case", "then", "while", "do", "for",
            "break", "continue", "function", "endfunction",
        ] {
            assert!(
                tokenize_scilab(&format!("{kw}\n"))
                    .iter()
                    .any(|t| t.effective_type_name() == "KEYWORD" && t.value == *kw),
                "expected KEYWORD({kw})"
            );
        }
    }

    #[test]
    fn endfunction_is_distinct_from_generic_end() {
        // MA10 §1 finding 7 / §3: `endfunction` is Scilab's own historically
        // mandated function terminator -- lexed as its OWN keyword value,
        // never conflated with the generic block-closer `end`.
        let p = lex("function y=f(x)\n y = x\nendfunction\n");
        let last_kw = p.iter().rev().find(|(t, _)| t == "KEYWORD").unwrap();
        assert_eq!(last_kw.1, "endfunction");
        // `end` and `endfunction` are different KEYWORD values, not aliases.
        assert_ne!(
            tokenize_scilab("end\n")[0].value,
            tokenize_scilab("endfunction\n")[0].value
        );
    }

    #[test]
    fn switch_and_otherwise_are_not_keywords_in_scilab() {
        // Scilab has no `switch`/`otherwise` at all (its own construct is
        // `select`/`case`/`else`, MA10 §1 finding 4) -- these MUST remain
        // ordinary NAMEs, unlike matlab.tokens which reserves them.
        assert_eq!(types("switch\n"), ["NAME"]);
        assert_eq!(types("otherwise\n"), ["NAME"]);
    }

    #[test]
    fn select_case_then_else_end_snippet() {
        let p = lex("select x\n case 1 then y = 1\n else y = 0\n end\n");
        assert_eq!(
            types_from(&p),
            vec![
                "KEYWORD", "NAME", "KEYWORD", "NUMBER", "KEYWORD", "NAME", "EQ", "NUMBER",
                "KEYWORD", "NAME", "EQ", "NUMBER", "KEYWORD",
            ]
        );
    }

    fn types_from(p: &[(String, String)]) -> Vec<&str> {
        p.iter().map(|(t, _)| t.as_str()).collect()
    }

    // --- No AT ("@") token: deliberately omitted -------------------------

    #[test]
    fn at_sign_is_an_unrecognized_character() {
        // Neither MATLAB's function-handle meaning nor Scilab's own
        // deprecated legacy `~`-spelling is in scope this cut (MA10 §4) --
        // `@` is simply absent from the grammar.
        assert!(try_tokenize_scilab("a = @b\n").is_err());
    }

    // --- Errors -----------------------------------------------------------

    #[test]
    fn unknown_character_is_an_error() {
        assert!(try_tokenize_scilab("x = #\n").is_err());
    }

    #[test]
    fn unrecognized_percent_word_falls_through_honestly() {
        // `%xyz` starts with a letter that is not a prefix of any of the
        // closed eight-word vocabulary (MA10 §4), so PERCENT_CONST does not
        // match it at all -- the bare `%` is an unrecognized character (an
        // honest failure, not a silent guess at what `%xyz` might mean).
        assert!(try_tokenize_scilab("x = %xyz\n").is_err());
    }

    #[test]
    fn percent_word_sharing_a_prefix_with_a_real_constant_still_splits() {
        // `%foo` is NOT one of the eight (only `%f` is real) -- because `f`
        // is a legitimate one-letter constant, `%foo` lexes as
        // PERCENT_CONST("%f") followed by NAME("oo") rather than failing
        // outright. This is a known, accepted edge case (MA10 §4's closed
        // set is enforced by the pattern, not by extra lookahead the
        // `regex` crate cannot express) -- a construct like this is not
        // valid Scilab either way, and would be rejected at parse time
        // (MA-10c) since two adjacent value tokens with no operator between
        // them is a syntax error in any sane grammar.
        assert_eq!(types("%foo\n"), ["PERCENT_CONST", "NAME"]);
    }

    #[test]
    fn handles_eof_mid_string_without_panic() {
        let _ = try_tokenize_scilab("x = 'oops\n");
    }
}
