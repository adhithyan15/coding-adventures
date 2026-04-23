//! # dot-lexer
//!
//! Tokeniser for the DOT graph description language.
//!
//! DOT is Graphviz's text format for describing graphs:
//!
//! ```dot
//! digraph G {
//!     A -> B [label = "edge"]
//!     B -> C
//! }
//! ```
//!
//! The lexer transforms a source string into a flat `Vec<Token>`.
//! Whitespace, line comments (`// …`), and block comments (`/* … */`) are
//! consumed and discarded — they never appear in the output stream.
//!
//! ## Token stream example
//!
//! ```text
//! "digraph G { A -> B }"
//!
//! Digraph   value=""   line=1 col=1
//! Id        value="G"  line=1 col=9
//! LBrace    value=""   line=1 col=11
//! Id        value="A"  line=1 col=13
//! Arrow     value=""   line=1 col=15
//! Id        value="B"  line=1 col=18
//! RBrace    value=""   line=1 col=20
//! Eof       value=""   line=1 col=21
//! ```
//!
//! ## ID flavours
//!
//! DOT has four syntactic flavours for identifiers, all collapsed into
//! `TokenKind::Id`:
//!
//! | Flavour        | Example         |
//! |----------------|-----------------|
//! | Unquoted       | `foo`, `_bar`   |
//! | Numeral        | `3.14`, `-42`   |
//! | Double-quoted  | `"hello world"` |
//! | HTML           | `<b>text</b>`   |
//!
//! Quoted strings are stripped of their delimiters and `\"` / `\\` escapes
//! are unescaped before being stored in `Token::value`.

pub const VERSION: &str = "0.1.0";

// ============================================================================
// TokenKind
// ============================================================================

/// The category of a single lexed token.
///
/// Keywords are case-insensitive in DOT (`GRAPH`, `Graph`, `graph` are the same).
/// They are always stored lowercase here for easy comparison.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    // ── Keywords ──────────────────────────────────────────────────────────────
    /// `strict` — makes graph edge uniqueness strict (at most one edge per pair).
    Strict,
    /// `graph` — declares an undirected graph.
    Graph,
    /// `digraph` — declares a directed graph.
    Digraph,
    /// `node` — used in `node [shape=box]` attribute statements.
    Node,
    /// `edge` — used in `edge [color=red]` attribute statements.
    Edge,
    /// `subgraph` — names a cluster subgraph.
    Subgraph,

    // ── Punctuation ───────────────────────────────────────────────────────────
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `=`
    Equals,
    /// `;`
    Semicolon,
    /// `,`
    Comma,
    /// `:`
    Colon,

    // ── Edge operators ────────────────────────────────────────────────────────
    /// `->` (directed edge operator; only valid inside `digraph`)
    Arrow,
    /// `--` (undirected edge operator; only valid inside `graph`)
    DashDash,

    // ── Identifier ───────────────────────────────────────────────────────────
    /// Any DOT identifier: unquoted word, numeral, quoted string, or HTML string.
    /// The `value` field holds the resolved text (quotes stripped, escapes unescaped).
    Id,

    // ── Sentinel ─────────────────────────────────────────────────────────────
    /// End-of-input marker. Always the last token in the stream.
    Eof,
}

// ============================================================================
// Token
// ============================================================================

/// A single token produced by the lexer.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    /// Resolved text value.
    ///
    /// - For keywords and punctuation: empty string (the kind is enough).
    /// - For `Id`: the identifier text (quotes/angles stripped, escapes resolved).
    /// - For `Eof`: empty string.
    pub value: String,
    /// 1-based line number of the first character.
    pub line: u32,
    /// 1-based column number of the first character.
    pub col: u32,
}

// ============================================================================
// LexError
// ============================================================================

/// A non-fatal lexical error.
///
/// The lexer continues after an error by skipping the offending character.
/// This lets us report multiple errors in a single pass.
#[derive(Clone, Debug, PartialEq)]
pub struct LexError {
    pub message: String,
    pub line: u32,
    pub col: u32,
}

// ============================================================================
// LexResult
// ============================================================================

/// The complete result of tokenising a DOT source string.
pub struct LexResult {
    pub tokens: Vec<Token>,
    pub errors: Vec<LexError>,
}

// ============================================================================
// Lexer — internal state machine
// ============================================================================

struct Lexer<'a> {
    source: &'a [u8],
    pos: usize,
    line: u32,
    col: u32,
    tokens: Vec<Token>,
    errors: Vec<LexError>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Lexer {
            source: source.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
            tokens: Vec::new(),
            errors: Vec::new(),
        }
    }

    // ── Character navigation ──────────────────────────────────────────────────

    fn peek(&self) -> Option<u8> {
        self.source.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.source.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let ch = self.source.get(self.pos).copied()?;
        self.pos += 1;
        if ch == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    // ── Token emission ────────────────────────────────────────────────────────

    fn emit(&mut self, kind: TokenKind, value: String, line: u32, col: u32) {
        self.tokens.push(Token { kind, value, line, col });
    }

    fn error(&mut self, message: impl Into<String>) {
        self.errors.push(LexError {
            message: message.into(),
            line: self.line,
            col: self.col,
        });
    }

    // ── Skip whitespace and comments ──────────────────────────────────────────

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace characters.
            while let Some(ch) = self.peek() {
                if ch == b' ' || ch == b'\t' || ch == b'\r' || ch == b'\n' {
                    self.advance();
                } else {
                    break;
                }
            }

            // Skip line comments: // ...
            if self.peek() == Some(b'/') && self.peek2() == Some(b'/') {
                self.advance();
                self.advance();
                while let Some(ch) = self.advance() {
                    if ch == b'\n' {
                        break;
                    }
                }
                continue;
            }

            // Skip block comments: /* ... */
            if self.peek() == Some(b'/') && self.peek2() == Some(b'*') {
                self.advance();
                self.advance();
                loop {
                    if self.at_end() {
                        self.error("unterminated block comment");
                        break;
                    }
                    if self.peek() == Some(b'*') && self.peek2() == Some(b'/') {
                        self.advance();
                        self.advance();
                        break;
                    }
                    self.advance();
                }
                continue;
            }

            // No more whitespace or comments to skip.
            break;
        }
    }

    // ── Scan a quoted string: "..." ───────────────────────────────────────────

    fn scan_quoted_string(&mut self, start_line: u32, start_col: u32) -> String {
        // Opening `"` has already been consumed by the caller.
        let mut buf = String::new();
        loop {
            match self.advance() {
                None => {
                    self.errors.push(LexError {
                        message: "unterminated string literal".to_string(),
                        line: start_line,
                        col: start_col,
                    });
                    break;
                }
                Some(b'"') => break,
                Some(b'\\') => {
                    // Escape sequence: only \" and \\ are handled; others pass through.
                    match self.advance() {
                        Some(b'"') => buf.push('"'),
                        Some(b'\\') => buf.push('\\'),
                        Some(b'n') => buf.push('\n'),
                        Some(b't') => buf.push('\t'),
                        Some(ch) => {
                            buf.push('\\');
                            buf.push(ch as char);
                        }
                        None => {
                            self.errors.push(LexError {
                                message: "unexpected end of input in string escape".to_string(),
                                line: self.line,
                                col: self.col,
                            });
                            break;
                        }
                    }
                }
                Some(ch) => buf.push(ch as char),
            }
        }
        buf
    }

    // ── Scan an HTML string: <...> with balanced nesting ─────────────────────

    fn scan_html_string(&mut self, start_line: u32, start_col: u32) -> String {
        // Opening `<` has already been consumed.
        let mut buf = String::new();
        let mut depth: i32 = 1;
        loop {
            match self.advance() {
                None => {
                    self.errors.push(LexError {
                        message: "unterminated HTML string".to_string(),
                        line: start_line,
                        col: start_col,
                    });
                    break;
                }
                Some(b'<') => {
                    depth += 1;
                    buf.push('<');
                }
                Some(b'>') => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    buf.push('>');
                }
                Some(ch) => buf.push(ch as char),
            }
        }
        buf
    }

    // ── Scan an unquoted identifier ───────────────────────────────────────────

    fn scan_unquoted_id(&mut self, first: u8) -> String {
        // `first` is the character that opened the identifier (already consumed).
        let mut buf = String::new();
        buf.push(first as char);
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == b'_' || ch >= 0x80 {
                self.advance();
                buf.push(ch as char);
            } else {
                break;
            }
        }
        buf
    }

    // ── Scan a numeral: -?(\.[0-9]+|[0-9]+(\.[0-9]*)?) ──────────────────────

    fn scan_numeral(&mut self, first: u8) -> String {
        // `first` is already consumed.
        let mut buf = String::new();
        buf.push(first as char);

        // Consume digits before the decimal point.
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.advance();
                buf.push(ch as char);
            } else {
                break;
            }
        }

        // Consume optional decimal part.
        if self.peek() == Some(b'.') {
            self.advance();
            buf.push('.');
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    self.advance();
                    buf.push(ch as char);
                } else {
                    break;
                }
            }
        }

        buf
    }

    // ── Map an unquoted word to a keyword kind (or Id if not a keyword) ───────

    fn keyword_or_id(word: &str) -> TokenKind {
        match word.to_ascii_lowercase().as_str() {
            "strict"   => TokenKind::Strict,
            "graph"    => TokenKind::Graph,
            "digraph"  => TokenKind::Digraph,
            "node"     => TokenKind::Node,
            "edge"     => TokenKind::Edge,
            "subgraph" => TokenKind::Subgraph,
            _          => TokenKind::Id,
        }
    }

    // ── Main scan loop ────────────────────────────────────────────────────────

    fn scan_all(&mut self) {
        loop {
            self.skip_whitespace_and_comments();

            if self.at_end() {
                self.emit(TokenKind::Eof, String::new(), self.line, self.col);
                break;
            }

            let line = self.line;
            let col  = self.col;
            let ch   = self.advance().unwrap();

            match ch {
                b'{' => self.emit(TokenKind::LBrace,    String::new(), line, col),
                b'}' => self.emit(TokenKind::RBrace,    String::new(), line, col),
                b'[' => self.emit(TokenKind::LBracket,  String::new(), line, col),
                b']' => self.emit(TokenKind::RBracket,  String::new(), line, col),
                b'=' => self.emit(TokenKind::Equals,    String::new(), line, col),
                b';' => self.emit(TokenKind::Semicolon, String::new(), line, col),
                b',' => self.emit(TokenKind::Comma,     String::new(), line, col),
                b':' => self.emit(TokenKind::Colon,     String::new(), line, col),

                // `->` (directed edge)
                b'-' if self.peek() == Some(b'>') => {
                    self.advance();
                    self.emit(TokenKind::Arrow, String::new(), line, col);
                }
                // `--` (undirected edge)
                b'-' if self.peek() == Some(b'-') => {
                    self.advance();
                    self.emit(TokenKind::DashDash, String::new(), line, col);
                }
                // Numeral starting with `-` (must be followed by a digit or `.`)
                b'-' if self.peek().map_or(false, |c| c.is_ascii_digit() || c == b'.') => {
                    let s = self.scan_numeral(ch);
                    self.emit(TokenKind::Id, s, line, col);
                }

                // Numeral starting with `.`
                b'.' if self.peek().map_or(false, |c| c.is_ascii_digit()) => {
                    let s = self.scan_numeral(ch);
                    self.emit(TokenKind::Id, s, line, col);
                }

                // Numeral starting with a digit
                b'0'..=b'9' => {
                    let s = self.scan_numeral(ch);
                    self.emit(TokenKind::Id, s, line, col);
                }

                // Quoted string
                b'"' => {
                    let s = self.scan_quoted_string(line, col);
                    self.emit(TokenKind::Id, s, line, col);
                }

                // HTML string
                b'<' => {
                    let s = self.scan_html_string(line, col);
                    self.emit(TokenKind::Id, s, line, col);
                }

                // Unquoted identifier or keyword
                ch if ch.is_ascii_alphabetic() || ch == b'_' || ch >= 0x80 => {
                    let word = self.scan_unquoted_id(ch);
                    let kind = Self::keyword_or_id(&word);
                    // For keywords the value is empty; for Id it carries the text.
                    let value = if kind == TokenKind::Id { word } else { String::new() };
                    self.emit(kind, value, line, col);
                }

                other => {
                    self.error(format!(
                        "unexpected character '{}' (0x{:02x})",
                        other as char, other
                    ));
                }
            }
        }
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Tokenise a DOT source string.
///
/// Returns all tokens including the final `Eof` sentinel. Errors are
/// collected rather than aborting — a partial token stream is still returned
/// alongside any errors.
///
/// # Example
///
/// ```rust
/// use dot_lexer::{tokenise, TokenKind};
///
/// let result = tokenise("digraph G { A -> B }");
/// assert!(result.errors.is_empty());
/// assert_eq!(result.tokens[0].kind, TokenKind::Digraph);
/// ```
pub fn tokenise(source: &str) -> LexResult {
    let mut lexer = Lexer::new(source);
    lexer.scan_all();
    LexResult { tokens: lexer.tokens, errors: lexer.errors }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        tokenise(src).tokens.into_iter().map(|t| t.kind).collect()
    }

    fn values(src: &str) -> Vec<String> {
        tokenise(src).tokens.into_iter().map(|t| t.value).collect()
    }

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    // ── Empty input ───────────────────────────────────────────────────────────

    #[test]
    fn empty_input_produces_eof() {
        let r = tokenise("");
        assert_eq!(r.tokens.len(), 1);
        assert_eq!(r.tokens[0].kind, TokenKind::Eof);
        assert!(r.errors.is_empty());
    }

    // ── Keywords ─────────────────────────────────────────────────────────────

    #[test]
    fn keywords_are_recognised() {
        assert_eq!(
            kinds("strict graph digraph node edge subgraph"),
            vec![
                TokenKind::Strict,
                TokenKind::Graph,
                TokenKind::Digraph,
                TokenKind::Node,
                TokenKind::Edge,
                TokenKind::Subgraph,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn keywords_are_case_insensitive() {
        assert_eq!(kinds("DIGRAPH"), vec![TokenKind::Digraph, TokenKind::Eof]);
        assert_eq!(kinds("Graph"),   vec![TokenKind::Graph,   TokenKind::Eof]);
        assert_eq!(kinds("STRICT"),  vec![TokenKind::Strict,  TokenKind::Eof]);
    }

    // ── Punctuation ───────────────────────────────────────────────────────────

    #[test]
    fn punctuation_tokens() {
        assert_eq!(
            kinds("{}[]=;,:"),
            vec![
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::Equals,
                TokenKind::Semicolon,
                TokenKind::Comma,
                TokenKind::Colon,
                TokenKind::Eof,
            ]
        );
    }

    // ── Edge operators ────────────────────────────────────────────────────────

    #[test]
    fn directed_edge_arrow() {
        assert_eq!(kinds("->"), vec![TokenKind::Arrow, TokenKind::Eof]);
    }

    #[test]
    fn undirected_edge_dashdash() {
        assert_eq!(kinds("--"), vec![TokenKind::DashDash, TokenKind::Eof]);
    }

    // ── Identifiers ───────────────────────────────────────────────────────────

    #[test]
    fn unquoted_identifiers() {
        let r = tokenise("foo _bar A1");
        assert_eq!(r.tokens[0].kind, TokenKind::Id);
        assert_eq!(r.tokens[0].value, "foo");
        assert_eq!(r.tokens[1].value, "_bar");
        assert_eq!(r.tokens[2].value, "A1");
    }

    #[test]
    fn numerals() {
        let r = tokenise("3.14 -42 .5 0");
        assert_eq!(r.tokens[0].value, "3.14");
        assert_eq!(r.tokens[1].value, "-42");
        assert_eq!(r.tokens[2].value, ".5");
        assert_eq!(r.tokens[3].value, "0");
    }

    #[test]
    fn quoted_string() {
        let r = tokenise(r#""hello world""#);
        assert_eq!(r.tokens[0].kind, TokenKind::Id);
        assert_eq!(r.tokens[0].value, "hello world");
    }

    #[test]
    fn quoted_string_with_escapes() {
        let r = tokenise(r#""say \"hi\"""#);
        assert_eq!(r.tokens[0].value, r#"say "hi""#);
    }

    #[test]
    fn html_string() {
        // DOT HTML labels use double angle brackets: the outer <> delimit the
        // DOT string, and the inner <b>...</b> is the HTML content.
        // Balanced nesting: <<b>text</b>> has depth 1→2→1→2→1→0.
        let r = tokenise("<<b>text</b>>");
        assert_eq!(r.tokens[0].kind, TokenKind::Id);
        assert_eq!(r.tokens[0].value, "<b>text</b>");
    }

    // ── Comments ──────────────────────────────────────────────────────────────

    #[test]
    fn line_comment_is_skipped() {
        let r = tokenise("// this is a comment\ndigraph");
        assert_eq!(r.tokens[0].kind, TokenKind::Digraph);
    }

    #[test]
    fn block_comment_is_skipped() {
        let r = tokenise("/* open */ digraph /* close */");
        assert_eq!(r.tokens[0].kind, TokenKind::Digraph);
        assert_eq!(r.tokens[1].kind, TokenKind::Eof);
    }

    // ── Line and column tracking ──────────────────────────────────────────────

    #[test]
    fn line_numbers_increment_at_newlines() {
        let r = tokenise("a\nb\nc");
        assert_eq!(r.tokens[0].line, 1);
        assert_eq!(r.tokens[1].line, 2);
        assert_eq!(r.tokens[2].line, 3);
    }

    // ── Full small graph ──────────────────────────────────────────────────────

    #[test]
    fn small_digraph() {
        let r = tokenise("digraph G { A -> B }");
        assert!(r.errors.is_empty());
        let k: Vec<_> = r.tokens.iter().map(|t| &t.kind).collect();
        assert_eq!(
            k,
            &[
                &TokenKind::Digraph,
                &TokenKind::Id,    // G
                &TokenKind::LBrace,
                &TokenKind::Id,    // A
                &TokenKind::Arrow,
                &TokenKind::Id,    // B
                &TokenKind::RBrace,
                &TokenKind::Eof,
            ]
        );
        let v: Vec<_> = r.tokens.iter().map(|t| t.value.as_str()).collect();
        assert_eq!(v[1], "G");
        assert_eq!(v[3], "A");
        assert_eq!(v[5], "B");
    }

    #[test]
    fn attributes_parse() {
        let r = tokenise(r#"digraph { A [label="hello", shape=box] }"#);
        assert!(r.errors.is_empty());
        // Check the label value was unquoted
        let label_tok = r.tokens.iter().find(|t| t.value == "hello").unwrap();
        assert_eq!(label_tok.kind, TokenKind::Id);
    }

    // ── Error recovery ────────────────────────────────────────────────────────

    #[test]
    fn unknown_character_produces_error() {
        let r = tokenise("digraph { @ }");
        assert_eq!(r.errors.len(), 1);
        assert!(r.errors[0].message.contains("unexpected character"));
        // Tokens before and after the bad char are still present.
        assert!(r.tokens.iter().any(|t| t.kind == TokenKind::Digraph));
    }
}
