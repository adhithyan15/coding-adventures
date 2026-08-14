//! # Tokenizer — turns `.wast`/`.wat` source text into a flat token stream.
//!
//! WAT's lexical grammar is small: parenthesized S-expressions, whitespace,
//! two comment forms, and two token shapes (bare atoms, and quoted strings).
//! Everything downstream (module forms, folded instructions, script
//! directives) is built on top of this flat stream — the tokenizer itself
//! knows nothing about WASM semantics.
//!
//! ```text
//! (module
//!   (func $add (param i32 i32) (result i32)
//!     local.get 0
//!     local.get 1
//!     i32.add))
//!
//! tokens: LParen "module" LParen "func" "$add" LParen "param" "i32" "i32"
//!         RParen LParen "result" "i32" RParen "local.get" "0" "local.get"
//!         "1" "i32.add" RParen RParen
//! ```
//!
//! ## The two comment forms
//!
//! - `;; ...` runs to end of line.
//! - `(; ... ;)` is a **nestable** block comment — `(; a (; b ;) c ;)` is one
//!   comment, not `(; a (; b ;)` followed by stray text `c ;)`. A tokenizer
//!   that doesn't track nesting depth breaks on the first real-world file
//!   that comments out a block containing another comment.
//!
//! ## Why strings are their own token kind, not atoms
//!
//! A string literal can contain any byte via escapes (including bytes that
//! aren't valid UTF-8 — used by `assert_malformed` cases to embed
//! intentionally-broken module bytes), and can contain the delimiter
//! characters (spaces, parens, `;`) that would otherwise end an atom. So a
//! [`Token::Str`] carries fully escape-decoded raw bytes, distinct from
//! [`Token::Atom`], which is always valid UTF-8 by construction (WAT's own
//! `idchar` grammar excludes bytes above 0x7E and excludes `"`, `(`, `)`,
//! `;`, and whitespace).

use crate::WastParseError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    LParen,
    RParen,
    /// A bare, unquoted token — a keyword (`module`, `func`, `i32.add`), a
    /// numeric literal (`42`, `-0x1.8p3`, `nan:0x1`), or an identifier
    /// (`$foo`). Always valid UTF-8; WAT's own grammar restricts atom
    /// characters to a fixed ASCII set.
    Atom(String),
    /// A quoted string literal, fully escape-decoded to raw bytes. May
    /// contain arbitrary bytes (including invalid UTF-8, for
    /// `assert_malformed` cases) or valid UTF-8 text (import/export names,
    /// `(module quote "...")` bodies).
    Str(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpannedToken {
    pub token: Token,
    /// Byte offset into the source where this token starts — carried
    /// through to error messages, not used for anything semantic.
    pub pos: usize,
}

/// An `idchar` per the WAT grammar: the characters a bare atom may contain.
/// Letters, digits, and this fixed punctuation set — notably excludes `"`,
/// `(`, `)`, `;`, and whitespace, which is what lets the tokenizer find atom
/// boundaries without a symbol table.
fn is_idchar(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '!' | '#'
                | '$'
                | '%'
                | '&'
                | '\''
                | '*'
                | '+'
                | '-'
                | '.'
                | '/'
                | ':'
                | '<'
                | '='
                | '>'
                | '?'
                | '@'
                | '\\'
                | '^'
                | '_'
                | '`'
                | '|'
                | '~'
        )
}

pub fn tokenize(src: &str) -> Result<Vec<SpannedToken>, WastParseError> {
    let bytes = src.as_bytes();
    let mut i = 0usize;
    let mut out = Vec::new();

    while i < bytes.len() {
        let c = bytes[i] as char;

        // Whitespace (space, tab, newline, CR).
        if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
            i += 1;
            continue;
        }

        // Line comment: `;;` to end of line. A line ends at LF, at CR, or at
        // CRLF (the CR is the terminator; the following LF is then consumed
        // as ordinary whitespace on the next iteration) -- the WASM spec
        // testsuite's comments.wast exercises all three explicitly.
        if c == ';' && bytes.get(i + 1) == Some(&b';') {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' && bytes[i] != b'\r' {
                i += 1;
            }
            continue;
        }

        // Nestable block comment: `(; ... ;)`.
        if c == '(' && bytes.get(i + 1) == Some(&b';') {
            let start = i;
            i += 2;
            let mut depth = 1usize;
            while depth > 0 {
                if i + 1 >= bytes.len() {
                    return Err(WastParseError::UnterminatedBlockComment { pos: start });
                }
                if bytes[i] == b'(' && bytes[i + 1] == b';' {
                    depth += 1;
                    i += 2;
                } else if bytes[i] == b';' && bytes[i + 1] == b')' {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            continue;
        }

        if c == '(' {
            out.push(SpannedToken { token: Token::LParen, pos: i });
            i += 1;
            continue;
        }
        if c == ')' {
            out.push(SpannedToken { token: Token::RParen, pos: i });
            i += 1;
            continue;
        }

        if c == '"' {
            let start = i;
            let (bytes_out, next) = scan_string(bytes, i)?;
            out.push(SpannedToken { token: Token::Str(bytes_out), pos: start });
            i = next;
            continue;
        }

        // Bare atom: a maximal run of idchars.
        let start = i;
        let mut end = i;
        while end < bytes.len() && is_idchar(bytes[end] as char) {
            end += 1;
        }
        if end == start {
            return Err(WastParseError::UnexpectedByte { pos: i, byte: bytes[i] });
        }
        let atom = std::str::from_utf8(&bytes[start..end])
            .map_err(|_| WastParseError::InvalidUtf8 { pos: start })?
            .to_string();
        out.push(SpannedToken { token: Token::Atom(atom), pos: start });
        i = end;
    }

    Ok(out)
}

/// Scan a `"..."` string literal starting at `bytes[start] == b'"'`, decoding
/// escapes as it goes. Returns the decoded bytes and the index just past the
/// closing quote.
///
/// Escapes: `\n \t \\ \' \"`, `\u{XXXX}` (a Unicode scalar, UTF-8-encoded),
/// and raw `\XX` (two hex digits — an arbitrary byte, used by
/// `assert_malformed` fixtures to embed invalid UTF-8 or invalid module
/// bytes directly).
fn scan_string(bytes: &[u8], start: usize) -> Result<(Vec<u8>, usize), WastParseError> {
    let mut i = start + 1; // skip opening quote
    let mut out = Vec::new();
    loop {
        if i >= bytes.len() {
            return Err(WastParseError::UnterminatedString { pos: start });
        }
        match bytes[i] {
            b'"' => {
                i += 1;
                return Ok((out, i));
            }
            b'\\' => {
                i += 1;
                if i >= bytes.len() {
                    return Err(WastParseError::UnterminatedString { pos: start });
                }
                match bytes[i] {
                    b'n' => {
                        out.push(b'\n');
                        i += 1;
                    }
                    b't' => {
                        out.push(b'\t');
                        i += 1;
                    }
                    b'\\' => {
                        out.push(b'\\');
                        i += 1;
                    }
                    b'\'' => {
                        out.push(b'\'');
                        i += 1;
                    }
                    b'"' => {
                        out.push(b'"');
                        i += 1;
                    }
                    b'u' if bytes.get(i + 1) == Some(&b'{') => {
                        let hex_start = i + 2;
                        let mut j = hex_start;
                        while j < bytes.len() && bytes[j] != b'}' {
                            j += 1;
                        }
                        if j >= bytes.len() {
                            return Err(WastParseError::InvalidEscape { pos: i - 1 });
                        }
                        let hex = std::str::from_utf8(&bytes[hex_start..j])
                            .map_err(|_| WastParseError::InvalidEscape { pos: i - 1 })?;
                        let code = u32::from_str_radix(hex, 16)
                            .map_err(|_| WastParseError::InvalidEscape { pos: i - 1 })?;
                        let ch = char::from_u32(code)
                            .ok_or(WastParseError::InvalidEscape { pos: i - 1 })?;
                        let mut buf = [0u8; 4];
                        out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        i = j + 1;
                    }
                    h1 if h1.is_ascii_hexdigit() => {
                        // Raw \XX hex-byte escape — exactly two hex digits.
                        let h2 = *bytes.get(i + 1).ok_or(WastParseError::InvalidEscape { pos: i - 1 })?;
                        if !h2.is_ascii_hexdigit() {
                            return Err(WastParseError::InvalidEscape { pos: i - 1 });
                        }
                        let hex = std::str::from_utf8(&bytes[i..i + 2]).unwrap();
                        let byte = u8::from_str_radix(hex, 16).unwrap();
                        out.push(byte);
                        i += 2;
                    }
                    _ => return Err(WastParseError::InvalidEscape { pos: i - 1 }),
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atoms(toks: &[SpannedToken]) -> Vec<String> {
        toks.iter()
            .filter_map(|t| match &t.token {
                Token::Atom(s) => Some(s.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn tokenizes_parens_and_atoms() {
        let toks = tokenize("(module (func $f))").unwrap();
        assert_eq!(
            toks.iter().map(|t| t.token.clone()).collect::<Vec<_>>(),
            vec![
                Token::LParen,
                Token::Atom("module".into()),
                Token::LParen,
                Token::Atom("func".into()),
                Token::Atom("$f".into()),
                Token::RParen,
                Token::RParen,
            ]
        );
    }

    #[test]
    fn line_comment_runs_to_newline_only() {
        let toks = tokenize("(a ;; comment (b)\n  c)").unwrap();
        assert_eq!(atoms(&toks), vec!["a", "c"]);
    }

    #[test]
    fn line_comment_terminates_at_a_bare_carriage_return() {
        // Old-Mac-style line endings (`\r` alone, no `\n`) must also end a
        // `;;` comment -- the WASM spec testsuite's comments.wast exercises
        // this exact case (its "f2" function).
        let toks = tokenize("(a ;; comment (b)\r  c)").unwrap();
        assert_eq!(atoms(&toks), vec!["a", "c"]);
    }

    #[test]
    fn line_comment_terminates_at_crlf() {
        let toks = tokenize("(a ;; comment (b)\r\n  c)").unwrap();
        assert_eq!(atoms(&toks), vec!["a", "c"]);
    }

    #[test]
    fn block_comment_is_nestable() {
        // A comment containing another comment must be consumed as ONE
        // comment, not close at the first `;)`.
        let toks = tokenize("(a (; outer (; inner ;) still-outer ;) b)").unwrap();
        assert_eq!(atoms(&toks), vec!["a", "b"]);
    }

    #[test]
    fn unterminated_block_comment_is_an_error() {
        let err = tokenize("(a (; never closes").unwrap_err();
        assert!(matches!(err, WastParseError::UnterminatedBlockComment { .. }));
    }

    #[test]
    fn string_decodes_standard_escapes() {
        let toks = tokenize(r#" "a\nb\tc\\d\"e\'f" "#).unwrap();
        assert_eq!(toks.len(), 1);
        match &toks[0].token {
            Token::Str(bytes) => assert_eq!(bytes, b"a\nb\tc\\d\"e'f"),
            other => panic!("expected Str, got {other:?}"),
        }
    }

    #[test]
    fn string_decodes_raw_hex_byte_escape() {
        // \FF is not valid UTF-8 on its own -- this is exactly the mechanism
        // assert_malformed fixtures use to embed invalid bytes.
        let toks = tokenize(r#" "\ff\00\41" "#).unwrap();
        match &toks[0].token {
            Token::Str(bytes) => assert_eq!(bytes, &[0xff, 0x00, 0x41]),
            other => panic!("expected Str, got {other:?}"),
        }
    }

    #[test]
    fn string_decodes_unicode_escape() {
        let toks = tokenize(r#" "\u{48}\u{49}" "#).unwrap();
        match &toks[0].token {
            Token::Str(bytes) => assert_eq!(bytes, b"HI"),
            other => panic!("expected Str, got {other:?}"),
        }
    }

    #[test]
    fn unterminated_string_is_an_error() {
        let err = tokenize(r#" "no closing quote "#).unwrap_err();
        assert!(matches!(err, WastParseError::UnterminatedString { .. }));
    }

    #[test]
    fn string_can_contain_parens_and_semicolons() {
        let toks = tokenize(r#" "(a;;b)" "#).unwrap();
        match &toks[0].token {
            Token::Str(bytes) => assert_eq!(bytes, b"(a;;b)"),
            other => panic!("expected Str, got {other:?}"),
        }
    }

    #[test]
    fn numeric_and_identifier_atoms_round_trip() {
        let toks = tokenize("-0x1.8p3 nan:0x1 $my-id 1_000_000").unwrap();
        assert_eq!(atoms(&toks), vec!["-0x1.8p3", "nan:0x1", "$my-id", "1_000_000"]);
    }
}
