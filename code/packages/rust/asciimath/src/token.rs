//! The AsciiMath tokenizer — a small, total, panic-free scanner.
//!
//! AsciiMath is (almost entirely) ASCII, so we scan byte-by-byte and record a half-open
//! byte `Span` for every token. The scanner recognises:
//!
//! * **numbers** — `42`, `3.14`, `6.022e23` (the exact text; [`Number`] validates it later);
//! * **identifiers** — a maximal run of ASCII letters (`x`, `pi`, `sin`, `sqrt`). Whether a
//!   run is a variable, a function, a constant, or a product of letters is the *parser's*
//!   job (it needs grammar context), so the tokenizer just hands over the run;
//! * **operators** — `+ - * / ^ _ = < >` and the multi-character forms `<= >= != ~~ -= -:`
//!   (and `**` for `*`), each one token;
//! * **brackets** — `( ) [ ] { }`;
//! * **text literals** — `"…"` (contents become a [`crate::Token`] of kind [`TokenKind::Text`]).
//!
//! Whitespace is skipped. Anything else is a *returned* error (never a panic), so the
//! tokenizer is total: every input yields `Ok(tokens)` or one `Err(FrontendError)`.
//!
//! Truth table for the `-` lead byte (the only genuinely ambiguous one):
//!
//! | next byte | token   | meaning            |
//! |-----------|---------|--------------------|
//! | `:`       | `Div`   | `-:` is ÷          |
//! | `=`       | `Equiv` | `-=` is ≡          |
//! | (other)   | `Minus` | binary/unary minus |

use math_frontend::FrontendError;

/// A half-open byte span `[start, end)` into the source.
pub type Span = (usize, usize);

/// The lexical categories AsciiMath PR-1 recognises.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// A numeric literal, exact source text (e.g. `"3.14"`).
    Num(String),
    /// A maximal run of ASCII letters (`"x"`, `"sin"`, `"pi"`).
    Ident(String),
    /// The contents of a `"…"` literal (the quotes are stripped).
    Text(String),
    Plus,
    Minus,
    /// `*` (and `**`) — multiplication.
    Star,
    /// `/` — built-up fraction.
    Slash,
    /// `^` — superscript / power.
    Caret,
    /// `_` — subscript.
    Underscore,
    /// `-:` (and the word `div`, handled in the parser) — division.
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// `~~` — approximately equal.
    Approx,
    /// `-=` — identically equal.
    Equiv,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    /// `,` — cell/row separator inside a matrix (`[[a,b],[c,d]]`). Outside a matrix the
    /// parser has no use for it and reports a clean error, never a panic.
    Comma,
    /// End of input (always the final token; zero-width span at the end).
    Eof,
}

/// A token plus where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

fn err(msg: impl Into<String>, span: Span) -> FrontendError {
    FrontendError::new("asciimath", msg, span)
}

/// Is `b` the start of a numeric literal given the following byte?
fn starts_number(b: u8, next: Option<u8>) -> bool {
    b.is_ascii_digit() || (b == b'.' && matches!(next, Some(d) if d.is_ascii_digit()))
}

/// Scan a numeric literal beginning at `start`, returning the end offset (exclusive).
/// Grammar: `digits? ('.' digits)? ([eE] [+-]? digits)?` with at least one mantissa digit.
/// We only *capture* the text here; [`Number::parse`] is the authority on validity.
fn scan_number(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
        i += 1; // the dot
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    // Optional exponent — only consumed if it is well-formed (digits follow e/E[+-]?),
    // otherwise the `e` belongs to a following identifier (`2e` ⇒ number `2`, ident `e`).
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
            j += 1;
        }
        if j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            i = j;
        }
    }
    i
}

/// Tokenize an AsciiMath source string. Total and panic-free: returns the token list
/// (terminated by [`TokenKind::Eof`]) or a single spanned [`FrontendError`].
pub fn tokenize(src: &str) -> Result<Vec<Token>, FrontendError> {
    let bytes = src.as_bytes();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        let next = bytes.get(i + 1).copied();
        let (kind, end) = match c {
            b'+' => (TokenKind::Plus, i + 1),
            b'-' => match next {
                Some(b':') => (TokenKind::Div, i + 2),
                Some(b'=') => (TokenKind::Equiv, i + 2),
                // Punctuation arrow `->`: emit the same identifier the word form `rarr` does, so it
                // flows through the existing symbol table (PR-3a) and lowers to `Symbol("rightarrow")`.
                // No new token kind, no parser change. (`a->b` ⇒ `a · → · b`; `lim_(x->0)` unaffected.)
                Some(b'>') => (TokenKind::Ident("rightarrow".to_string()), i + 2),
                _ => (TokenKind::Minus, i + 1),
            },
            b'*' => (TokenKind::Star, if next == Some(b'*') { i + 2 } else { i + 1 }),
            b'/' => (TokenKind::Slash, i + 1),
            b'^' => (TokenKind::Caret, i + 1),
            b'_' => (TokenKind::Underscore, i + 1),
            b'=' => match next {
                // Punctuation double-arrow `=>` → the `implies` identifier (PR-3a symbol table).
                Some(b'>') => (TokenKind::Ident("implies".to_string()), i + 2),
                _ => (TokenKind::Eq, i + 1),
            },
            b'!' => match next {
                Some(b'=') => (TokenKind::Ne, i + 2),
                _ => return Err(err("unexpected '!' (did you mean '!='?)", (start, start + 1))),
            },
            b'<' => match next {
                Some(b'=') => (TokenKind::Le, i + 2),
                _ => (TokenKind::Lt, i + 1),
            },
            b'>' => match next {
                Some(b'=') => (TokenKind::Ge, i + 2),
                _ => (TokenKind::Gt, i + 1),
            },
            b'~' => match next {
                Some(b'~') => (TokenKind::Approx, i + 2),
                _ => return Err(err("unexpected '~' (did you mean '~~'?)", (start, start + 1))),
            },
            b'(' => (TokenKind::LParen, i + 1),
            b')' => (TokenKind::RParen, i + 1),
            b'[' => (TokenKind::LBracket, i + 1),
            b']' => (TokenKind::RBracket, i + 1),
            b'{' => (TokenKind::LBrace, i + 1),
            b'}' => (TokenKind::RBrace, i + 1),
            b',' => (TokenKind::Comma, i + 1),
            b'"' => {
                // A text literal runs to the next double-quote.
                let mut j = i + 1;
                while j < bytes.len() && bytes[j] != b'"' {
                    j += 1;
                }
                if j >= bytes.len() {
                    return Err(err("unterminated string literal", (start, bytes.len())));
                }
                // The contents are ASCII-safe to slice on `"` boundaries; `"` is one byte.
                (TokenKind::Text(src[i + 1..j].to_string()), j + 1)
            }
            _ if starts_number(c, next) => {
                let end = scan_number(bytes, i);
                (TokenKind::Num(src[start..end].to_string()), end)
            }
            _ if c.is_ascii_alphabetic() => {
                let mut j = i;
                while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                    j += 1;
                }
                // PR-3c: the `text(…)` keyword form — the parenthesised twin of the `"…"`
                // literal. When the run is exactly `text` and an open paren *immediately*
                // follows, capture the raw bytes up to the matching close paren and emit the
                // same [`TokenKind::Text`] the `"…"` form does, so both spellings lower to an
                // identical `MathExpr::Text` (no parser or capability change). Parens nest, so
                // `text(f(x))` keeps its inner parens; a missing close paren is a clean error.
                // `text` NOT immediately followed by `(` stays an ordinary identifier (so a
                // variable named `text`, or `text` with a space before a group, is unchanged).
                if &src[start..j] == "text" && bytes.get(j) == Some(&b'(') {
                    let content_start = j + 1;
                    let mut depth = 1usize;
                    let mut k = content_start;
                    while k < bytes.len() {
                        match bytes[k] {
                            b'(' => depth += 1,
                            b')' => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        k += 1;
                    }
                    if depth != 0 {
                        return Err(err("unterminated text(...)", (start, bytes.len())));
                    }
                    // `(` and `)` are one-byte ASCII and never occur inside a multi-byte
                    // UTF-8 sequence, so `content_start` and `k` are char boundaries — the
                    // slice is panic-free even when the content is non-ASCII.
                    (TokenKind::Text(src[content_start..k].to_string()), k + 1)
                } else {
                    (TokenKind::Ident(src[start..j].to_string()), j)
                }
            }
            _ => {
                // Unknown byte (includes any non-ASCII lead byte). Report a one-byte span;
                // we never slice with it, so a mid-codepoint offset cannot cause a panic.
                return Err(err(format!("unexpected byte 0x{c:02x}"), (start, start + 1)));
            }
        };
        toks.push(Token { kind, span: (start, end) });
        i = end;
    }
    toks.push(Token { kind: TokenKind::Eof, span: (bytes.len(), bytes.len()) });
    Ok(toks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        tokenize(src).unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn numbers_idents_ops() {
        assert_eq!(
            kinds("3.14 + x"),
            vec![
                TokenKind::Num("3.14".into()),
                TokenKind::Plus,
                TokenKind::Ident("x".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn multi_char_operators() {
        assert_eq!(kinds("a <= b"), vec![
            TokenKind::Ident("a".into()), TokenKind::Le, TokenKind::Ident("b".into()), TokenKind::Eof,
        ]);
        assert_eq!(kinds("a != b")[1], TokenKind::Ne);
        assert_eq!(kinds("a -: b")[1], TokenKind::Div);
        assert_eq!(kinds("a ~~ b")[1], TokenKind::Approx);
        assert_eq!(kinds("a -= b")[1], TokenKind::Equiv);
        assert_eq!(kinds("a ** b")[1], TokenKind::Star);
        // PR-3c: punctuation arrows tokenize as the symbol-table identifiers (not `-`/`>`/`=`).
        assert_eq!(kinds("a -> b")[1], TokenKind::Ident("rightarrow".into()));
        assert_eq!(kinds("a => b")[1], TokenKind::Ident("implies".into()));
        // The single-char forms are unaffected.
        assert_eq!(kinds("a - b")[1], TokenKind::Minus);
        assert_eq!(kinds("a = b")[1], TokenKind::Eq);
    }

    #[test]
    fn exponent_only_consumed_when_well_formed() {
        assert_eq!(kinds("6.022e23")[0], TokenKind::Num("6.022e23".into()));
        // `2e` is the number 2 then the identifier e (no exponent digits).
        assert_eq!(kinds("2e"), vec![
            TokenKind::Num("2".into()), TokenKind::Ident("e".into()), TokenKind::Eof,
        ]);
    }

    #[test]
    fn text_literal() {
        assert_eq!(kinds(r#""kg" "#)[0], TokenKind::Text("kg".into()));
    }

    #[test]
    fn text_keyword_form() {
        // PR-3c: `text(…)` is the parenthesised twin of `"…"` — same Text token.
        assert_eq!(kinds("text(kg)")[0], TokenKind::Text("kg".into()));
        assert_eq!(kinds(r#""kg""#)[0], kinds("text(kg)")[0]);
        // Empty content is fine.
        assert_eq!(kinds("text()")[0], TokenKind::Text("".into()));
        // Inner parens nest and are preserved verbatim.
        assert_eq!(kinds("text(f(x))")[0], TokenKind::Text("f(x)".into()));
        // Spaces and arbitrary punctuation inside are raw.
        assert_eq!(kinds("text(a + b)")[0], TokenKind::Text("a + b".into()));
        // The closing paren is consumed (the token after is Eof here).
        assert_eq!(kinds("text(kg)"), vec![TokenKind::Text("kg".into()), TokenKind::Eof]);
    }

    #[test]
    fn text_without_immediate_paren_is_an_identifier() {
        // No paren → ordinary identifier run (parser makes it a product of letters).
        assert_eq!(kinds("text")[0], TokenKind::Ident("text".into()));
        // A space before `(` means `text` is an identifier and `(` opens a group.
        assert_eq!(kinds("text (x)"), vec![
            TokenKind::Ident("text".into()),
            TokenKind::LParen,
            TokenKind::Ident("x".into()),
            TokenKind::RParen,
            TokenKind::Eof,
        ]);
        // A longer word starting with `text` is untouched.
        assert_eq!(kinds("textual")[0], TokenKind::Ident("textual".into()));
    }

    #[test]
    fn unterminated_text_keyword_is_an_error() {
        let e = tokenize("text(oops").unwrap_err();
        assert_eq!(e.frontend, "asciimath");
        assert!(e.span.0 <= e.span.1);
    }

    #[test]
    fn comma_is_a_token() {
        assert_eq!(kinds("a,b"), vec![
            TokenKind::Ident("a".into()), TokenKind::Comma, TokenKind::Ident("b".into()), TokenKind::Eof,
        ]);
    }

    #[test]
    fn spans_are_in_range_and_ordered() {
        for t in tokenize("sqrt(x^2) + 1/2").unwrap() {
            assert!(t.span.0 <= t.span.1);
            assert!(t.span.1 <= "sqrt(x^2) + 1/2".len());
        }
    }

    #[test]
    fn errors_are_spanned_not_panics() {
        let e = tokenize("a ! b").unwrap_err();
        assert_eq!(e.frontend, "asciimath");
        assert!(e.span.0 <= e.span.1);
        // unterminated string
        assert!(tokenize("\"oops").is_err());
        // non-ascii byte
        assert!(tokenize("×").is_err());
    }
}
