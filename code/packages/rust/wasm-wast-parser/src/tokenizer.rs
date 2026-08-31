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
            i = skip_line_comment(bytes, i);
            continue;
        }

        // Nestable block comment: `(; ... ;)`.
        if c == '(' && bytes.get(i + 1) == Some(&b';') {
            i = skip_block_comment(bytes, i)?;
            continue;
        }

        // Annotation: `(@id ...)`. No real WAT keyword ever starts a list
        // with an atom beginning `@`, so this is unambiguous -- and it
        // needs its OWN scan, not the ordinary atom tokenizer below,
        // because an annotation's BODY (everything after its id) allows
        // characters the ordinary `idchar`-based atom scanner rejects
        // outright (`,`, `[`, `]`, `{`, `}`, ...) -- see
        // `scan_annotation_body`'s own doc comment. Tokenized as ordinary
        // `LParen`, an id `Atom` (and, for a `(@"id" ...)` form, a `Str`
        // right after it), then `RParen` -- everything in between is
        // scanned and discarded here, emitting no tokens for it at all, so
        // `sexpr::strip_annotations` sees exactly `(head [id-str])` and
        // nothing else to worry about.
        if c == '(' && bytes.get(i + 1) == Some(&b'@') {
            let paren_pos = i;
            out.push(SpannedToken { token: Token::LParen, pos: paren_pos });
            i += 1; // now at '@'
            let atom_start = i;
            let mut atom_end = i;
            while atom_end < bytes.len() && is_idchar(bytes[atom_end] as char) {
                atom_end += 1;
            }
            // `atom_end > atom_start` always holds: '@' itself is an
            // idchar (see `is_idchar`), so the run is at least `"@"`.
            let atom_text = std::str::from_utf8(&bytes[atom_start..atom_end])
                .map_err(|_| WastParseError::InvalidUtf8 { pos: atom_start })?
                .to_string();
            out.push(SpannedToken { token: Token::Atom(atom_text.clone()), pos: atom_start });
            i = atom_end;
            // A bare `@` immediately (no whitespace) followed by a quoted
            // string: that string IS the id (`(@"a" ...)`), per the real
            // annotation grammar -- consumed as an ordinary `Str` token
            // before the permissive body scan takes over.
            if atom_text == "@" && bytes.get(i) == Some(&b'"') {
                let str_start = i;
                let (str_bytes, next) = scan_string(bytes, i)?;
                out.push(SpannedToken { token: Token::Str(str_bytes), pos: str_start });
                i = next;
            }
            i = scan_annotation_body(bytes, i)?;
            out.push(SpannedToken { token: Token::RParen, pos: i });
            i += 1;
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

/// Skip a `;;`-to-end-of-line comment starting at `bytes[i] == b';'` (with
/// `bytes[i + 1] == b';'` already confirmed by the caller). Returns the
/// index of the line terminator (LF or CR), not consumed -- the caller's
/// own whitespace handling picks it up on the next iteration. Shared by
/// the main [`tokenize`] loop and [`scan_annotation_body`] (an annotation
/// body's own comments must be skipped the same way, including a stray
/// `)` inside one not affecting paren-depth tracking).
fn skip_line_comment(bytes: &[u8], i: usize) -> usize {
    let mut i = i + 2; // past `;;`
    while i < bytes.len() && bytes[i] != b'\n' && bytes[i] != b'\r' {
        i += 1;
    }
    i
}

/// Skip a nestable `(; ... ;)` block comment starting at `bytes[start] ==
/// b'('` (with `bytes[start + 1] == b';'` already confirmed by the
/// caller). Returns the index just past the closing `;)`. Shared by
/// [`tokenize`] and [`scan_annotation_body`] -- see [`skip_line_comment`]'s
/// doc comment for why both need the exact same skipping behavior.
fn skip_block_comment(bytes: &[u8], start: usize) -> Result<usize, WastParseError> {
    let mut i = start + 2; // past `(;`
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
    Ok(i)
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
            // A raw (unescaped) control character or DEL is malformed --
            // WAT's `stringchar` grammar requires these to be written as an
            // escape (`\t`/`\n`/the raw-byte `\XX` form, all handled in the
            // `b'\\'` arm above, never reaching here). Every other raw byte,
            // including any multi-byte UTF-8 sequence's lead/continuation
            // bytes (0x80+), is unrestricted -- WAT strings may contain
            // arbitrary raw Unicode text, just not raw control bytes.
            b if b < 0x20 || b == 0x7f => {
                return Err(WastParseError::IllegalStringCharacter { pos: i, byte: b });
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
}

/// Permissively scans an annotation's **body** — everything between its
/// id and its own matching close paren — without trying to form ordinary
/// `idchar` atoms from it. WAT's annotation grammar allows almost any
/// character here: `annotations.wast`'s own `(@a , ; ] [ }} }x{ ({) ,{{};}]
/// ;)` is content the ordinary atom scanner would reject outright (`,`
/// isn't an idchar) but which is perfectly legal annotation content,
/// since none of it is ever semantically interpreted — the whole
/// annotation is discarded once parsed (see `sexpr::strip_annotations`).
/// So this scan only needs to track the three things that still matter
/// structurally:
///
/// - **Paren balance**: a nested `(...)` is just more body content, but
///   the FIRST unmatched `)` (depth back to zero) is this annotation's own
///   close paren — where the scan stops.
/// - **Comments still nest/terminate normally**: both comment forms (`;;
///   ...`, nestable `(; ... ;)`) are recognized and skipped whole, so a
///   `)` or `(` *inside* a comment never affects paren-depth tracking —
///   `annotations.wast`'s own `(@a (;bla;) (; ) ;) ;; bla) ;; bla (@x )`
///   depends on exactly this (two stray `)` characters, one inside a
///   block comment and one inside a line comment, that must NOT close the
///   annotation early).
/// - **Strings still nest via `scan_string`**: a `"..."` inside the body
///   is scanned (and validated) the normal way, so a paren or semicolon
///   inside a string doesn't affect depth/comment tracking either, and an
///   unterminated string still produces a clean error.
///
/// Every other raw byte is allowed EXCEPT: a control character other than
/// tab/LF/CR, DEL (`0x7F`), or any byte `>= 0x80`. The real corpus asserts
/// both non-ASCII text (`"Heiße Würstchen"`, "illegal character") and raw
/// invalid-UTF-8 bytes (`\80`..`\ff`, "malformed UTF-8 encoding") are
/// rejected here — since both cases are simply "a byte with the top bit
/// set outside a string," one uniform check covers both without needing
/// a real UTF-8-validity pass over the body.
///
/// Returns the index of the matching close paren (NOT consumed — the
/// caller pushes the `RParen` token for it, mirroring how it pushed the
/// `LParen` for the annotation's own open paren).
fn scan_annotation_body(bytes: &[u8], mut i: usize) -> Result<usize, WastParseError> {
    let mut depth: usize = 0;
    loop {
        if i >= bytes.len() {
            return Err(WastParseError::UnexpectedEof);
        }
        let b = bytes[i];
        if b == b';' && bytes.get(i + 1) == Some(&b';') {
            i = skip_line_comment(bytes, i);
            continue;
        }
        if b == b'(' && bytes.get(i + 1) == Some(&b';') {
            i = skip_block_comment(bytes, i)?;
            continue;
        }
        if b == b'"' {
            let (_decoded, next) = scan_string(bytes, i)?;
            i = next;
            continue;
        }
        if b == b'(' {
            depth += 1;
            i += 1;
            continue;
        }
        if b == b')' {
            if depth == 0 {
                return Ok(i);
            }
            depth -= 1;
            i += 1;
            continue;
        }
        if (b < 0x20 && b != b'\t' && b != b'\n' && b != b'\r') || b == 0x7f || b >= 0x80 {
            return Err(WastParseError::UnexpectedByte { pos: i, byte: b });
        }
        i += 1;
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

    #[test]
    fn a_raw_unescaped_newline_inside_a_string_is_illegal() {
        // WAT's `stringchar` grammar requires control characters to be
        // written as an escape (`\n`, or the raw-byte `\XX` form) -- a
        // literal, unescaped newline byte inside a string is malformed
        // (annotations.wast's own `(@"\n")` case, once the outer `module
        // quote` string's own `\n` escape decodes to a real newline byte
        // sitting inside an INNER string literal).
        let err = tokenize("\"a\nb\"").unwrap_err();
        assert!(matches!(err, WastParseError::IllegalStringCharacter { byte: b'\n', .. }));
    }

    #[test]
    fn a_raw_del_byte_inside_a_string_is_illegal() {
        let src = format!("\"a{}b\"", '\u{7f}');
        assert!(matches!(tokenize(&src), Err(WastParseError::IllegalStringCharacter { byte: 0x7f, .. })));
    }

    #[test]
    fn a_raw_tab_inside_a_string_is_illegal_but_the_escape_works() {
        // Per WAT's `stringchar` grammar, EVERY control character --
        // including tab -- is < U+20 and therefore must be written as an
        // escape (`\t`) inside a string; only OUTSIDE of strings (e.g. an
        // annotation's body, see `scan_annotation_body`) is a raw tab
        // treated as ordinary whitespace.
        assert!(matches!(tokenize("\"a\tb\""), Err(WastParseError::IllegalStringCharacter { byte: 0x09, .. })));
        let toks = tokenize(r#""a\tb""#).unwrap();
        match &toks[0].token {
            Token::Str(bytes) => assert_eq!(bytes, b"a\tb"),
            other => panic!("expected Str, got {other:?}"),
        }
    }

    // ══════════════════════════════════════════════════════════════════════
    // Annotations -- `(@id ...)`. See `scan_annotation_body`'s own doc
    // comment for the design; these tests exercise the tokenizer directly.
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn a_simple_annotation_tokenizes_as_a_normal_list() {
        let toks = tokenize("(@a)").unwrap();
        assert_eq!(
            toks.iter().map(|t| t.token.clone()).collect::<Vec<_>>(),
            vec![Token::LParen, Token::Atom("@a".into()), Token::RParen]
        );
    }

    #[test]
    fn an_annotation_body_with_punctuation_the_ordinary_tokenizer_would_reject_is_fine() {
        // `,`/`;`/`]`/`[`/`{`/`}` are all illegal in an ordinary WAT atom,
        // but perfectly legal inside an annotation's body.
        let toks = tokenize(r#"(@a , ; ] [ }} }x{ ({) ,{{};}] ;)"#).unwrap();
        assert_eq!(
            toks.iter().map(|t| t.token.clone()).collect::<Vec<_>>(),
            vec![Token::LParen, Token::Atom("@a".into()), Token::RParen]
        );
    }

    #[test]
    fn an_annotation_body_can_contain_a_nested_block_comment_with_a_stray_close_paren() {
        // The block comment's own internal `)` must not end the annotation
        // early -- mirrors annotations.wast's own `(@a (;bla;) (; ) ;) ...)`.
        let toks = tokenize("(@a (;bla;) (; ) ;) )").unwrap();
        assert_eq!(
            toks.iter().map(|t| t.token.clone()).collect::<Vec<_>>(),
            vec![Token::LParen, Token::Atom("@a".into()), Token::RParen]
        );
    }

    #[test]
    fn an_annotation_body_can_contain_a_line_comment_with_a_stray_close_paren() {
        let toks = tokenize("(@a ;; bla)\n)").unwrap();
        assert_eq!(
            toks.iter().map(|t| t.token.clone()).collect::<Vec<_>>(),
            vec![Token::LParen, Token::Atom("@a".into()), Token::RParen]
        );
    }

    #[test]
    fn an_annotation_with_an_adjacent_quoted_id_tokenizes_id_and_body_separately() {
        let toks = tokenize(r#"(@"a")"#).unwrap();
        assert_eq!(
            toks.iter().map(|t| t.token.clone()).collect::<Vec<_>>(),
            vec![Token::LParen, Token::Atom("@".into()), Token::Str(b"a".to_vec()), Token::RParen]
        );
    }

    #[test]
    fn a_raw_control_byte_inside_an_annotation_body_is_illegal() {
        let src = "(@a \x00)";
        assert!(matches!(tokenize(src), Err(WastParseError::UnexpectedByte { byte: 0x00, .. })));
    }

    #[test]
    fn a_raw_non_ascii_byte_inside_an_annotation_body_is_illegal() {
        // Both non-ASCII TEXT ("Heiße Würstchen") and a raw invalid-UTF-8
        // byte are rejected the same way: any byte with the top bit set,
        // outside of a string, is simply not allowed in an annotation body.
        let toks = tokenize("(@a Hei\u{df}e)");
        assert!(toks.is_err());
    }

    #[test]
    fn an_unclosed_annotation_is_an_error() {
        assert!(tokenize("(@a").is_err());
        assert!(tokenize("(@a (y (z))").is_err());
    }

    #[test]
    fn nested_annotation_like_forms_inside_an_annotation_body_are_just_more_body() {
        let toks = tokenize("(@a @ @x (@x) (@x y) (@) (@ x))").unwrap();
        assert_eq!(
            toks.iter().map(|t| t.token.clone()).collect::<Vec<_>>(),
            vec![Token::LParen, Token::Atom("@a".into()), Token::RParen]
        );
    }
}
