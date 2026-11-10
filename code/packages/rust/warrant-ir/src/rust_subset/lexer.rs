//! Lexer for the Confined Rust Subset (VER25a, VER-P022).

use crate::rust_subset::ast::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // Keywords
    Fn,
    Pub,
    Mod,
    Use,
    Let,
    Mut,
    If,
    Else,
    For,
    In,
    Return,
    Unsafe,
    As,
    Const,
    True,
    False,
    Super,
    SelfValue,

    // Identifiers & Literals
    Ident(String),
    Int(u128, Option<String>),
    ByteString(Vec<u8>),
    StringLit(String),

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    // Operators & Punctuation
    ColonColon,
    Colon,
    Semi,
    Comma,
    Arrow, // ->
    DotDotEq,
    DotDot,
    Dot,
    EqEq,
    Eq,
    Ne,
    Not,
    LtEq,
    Lt,
    GtEq,
    Gt,
    PlusEq,
    Plus,
    MinusEq,
    Minus,
    StarEq,
    Star,
    SlashEq,
    Slash,
    PercentEq,
    Percent,
    AndAnd,
    AndEq,
    And,
    OrOr,
    OrEq,
    Or,
    CaretEq,
    Caret,
    ShlEq,
    Shl,
    ShrEq,
    Shr,
    Hash,
    Bang, // !

    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

const MAX_TOKENS: usize = 65_536;

pub struct Lexer<'a> {
    chars: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            chars: source.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<u8> {
        if self.pos < self.chars.len() {
            Some(self.chars[self.pos])
        } else {
            None
        }
    }

    fn peek_ahead(&self, n: usize) -> Option<u8> {
        if self.pos + n < self.chars.len() {
            Some(self.chars[self.pos + n])
        } else {
            None
        }
    }

    fn advance(&mut self) -> Option<u8> {
        if self.pos < self.chars.len() {
            let b = self.chars[self.pos];
            self.pos += 1;
            if b == b'\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            Some(b)
        } else {
            None
        }
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), LexError> {
        loop {
            match self.peek() {
                Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n') => {
                    self.advance();
                }
                Some(b'/') => {
                    if self.peek_ahead(1) == Some(b'/') {
                        // Line comment
                        while let Some(b) = self.peek() {
                            self.advance();
                            if b == b'\n' {
                                break;
                            }
                        }
                    } else if self.peek_ahead(1) == Some(b'*') {
                        // Block comment
                        let start = self.pos;
                        let line = self.line;
                        let col = self.col;
                        self.advance(); // '/'
                        self.advance(); // '*'
                        let mut depth = 1;
                        while depth > 0 {
                            match self.peek() {
                                Some(b'/') if self.peek_ahead(1) == Some(b'*') => {
                                    self.advance();
                                    self.advance();
                                    depth += 1;
                                }
                                Some(b'*') if self.peek_ahead(1) == Some(b'/') => {
                                    self.advance();
                                    self.advance();
                                    depth -= 1;
                                }
                                Some(_) => {
                                    self.advance();
                                }
                                None => {
                                    return Err(LexError {
                                        message: "Unterminated block comment".to_string(),
                                        span: Span::new(start, self.pos, line, col),
                                    });
                                }
                            }
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        Ok(())
    }

    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace_and_comments()?;

        let start_pos = self.pos;
        let start_line = self.line;
        let start_col = self.col;

        let b = match self.peek() {
            Some(b) => b,
            None => {
                return Ok(Token {
                    kind: TokenKind::Eof,
                    span: Span::new(self.pos, self.pos, self.line, self.col),
                });
            }
        };

        // Identifiers or keywords or byte strings
        if b == b'b' && (self.peek_ahead(1) == Some(b'"') || self.peek_ahead(1) == Some(b'\'')) {
            self.advance(); // 'b'
            if self.peek() == Some(b'"') {
                self.advance(); // '"'
                let mut bytes = Vec::new();
                while let Some(c) = self.peek() {
                    self.advance();
                    if c == b'"' {
                        return Ok(Token {
                            kind: TokenKind::ByteString(bytes),
                            span: Span::new(start_pos, self.pos, start_line, start_col),
                        });
                    }
                    if c == b'\\' {
                        match self.advance() {
                            Some(b'n') => bytes.push(b'\n'),
                            Some(b'r') => bytes.push(b'\r'),
                            Some(b't') => bytes.push(b'\t'),
                            Some(b'\\') => bytes.push(b'\\'),
                            Some(b'0') => bytes.push(b'\0'),
                            Some(b'"') => bytes.push(b'"'),
                            Some(b'x') => {
                                let h1 = self.advance().unwrap_or(0);
                                let h2 = self.advance().unwrap_or(0);
                                let hex_str = [h1, h2];
                                if let Ok(s) = std::str::from_utf8(&hex_str) {
                                    if let Ok(val) = u8::from_str_radix(s, 16) {
                                        bytes.push(val);
                                        continue;
                                    }
                                }
                                return Err(LexError {
                                    message: "Invalid hex escape in byte string".to_string(),
                                    span: Span::new(start_pos, self.pos, start_line, start_col),
                                });
                            }
                            _ => {
                                return Err(LexError {
                                    message: "Invalid escape in byte string".to_string(),
                                    span: Span::new(start_pos, self.pos, start_line, start_col),
                                });
                            }
                        }
                    } else {
                        bytes.push(c);
                    }
                }
                return Err(LexError {
                    message: "Unterminated byte string literal".to_string(),
                    span: Span::new(start_pos, self.pos, start_line, start_col),
                });
            }
        }

        if b == b'"' {
            self.advance(); // '"'
            let mut s = String::new();
            while let Some(c) = self.peek() {
                self.advance();
                if c == b'"' {
                    return Ok(Token {
                        kind: TokenKind::StringLit(s),
                        span: Span::new(start_pos, self.pos, start_line, start_col),
                    });
                }
                if c == b'\\' {
                    match self.advance() {
                        Some(b'n') => s.push('\n'),
                        Some(b'r') => s.push('\r'),
                        Some(b't') => s.push('\t'),
                        Some(b'\\') => s.push('\\'),
                        Some(b'"') => s.push('"'),
                        _ => s.push(c as char),
                    }
                } else {
                    s.push(c as char);
                }
            }
            return Err(LexError {
                message: "Unterminated string literal".to_string(),
                span: Span::new(start_pos, self.pos, start_line, start_col),
            });
        }

        if b.is_ascii_alphabetic() || b == b'_' {
            let mut ident = String::new();
            while let Some(c) = self.peek() {
                if c.is_ascii_alphanumeric() || c == b'_' {
                    ident.push(c as char);
                    self.advance();
                } else {
                    break;
                }
            }
            let kind = match ident.as_str() {
                "fn" => TokenKind::Fn,
                "pub" => TokenKind::Pub,
                "mod" => TokenKind::Mod,
                "use" => TokenKind::Use,
                "let" => TokenKind::Let,
                "mut" => TokenKind::Mut,
                "if" => TokenKind::If,
                "else" => TokenKind::Else,
                "for" => TokenKind::For,
                "in" => TokenKind::In,
                "return" => TokenKind::Return,
                "unsafe" => TokenKind::Unsafe,
                "as" => TokenKind::As,
                "const" => TokenKind::Const,
                "true" => TokenKind::True,
                "false" => TokenKind::False,
                "super" => TokenKind::Super,
                "self" => TokenKind::SelfValue,
                _ => TokenKind::Ident(ident),
            };
            return Ok(Token {
                kind,
                span: Span::new(start_pos, self.pos, start_line, start_col),
            });
        }

        // Numbers (0x..., 0b..., decimal)
        if b.is_ascii_digit() {
            let mut num_str = String::new();
            let mut radix = 10;
            if b == b'0' {
                match self.peek_ahead(1) {
                    Some(b'x') | Some(b'X') => {
                        self.advance();
                        self.advance();
                        radix = 16;
                    }
                    Some(b'b') | Some(b'B') => {
                        self.advance();
                        self.advance();
                        radix = 2;
                    }
                    _ => {}
                }
            }

            while let Some(c) = self.peek() {
                if (radix == 16 && c.is_ascii_hexdigit())
                    || (radix == 10 && c.is_ascii_digit())
                    || (radix == 2 && (c == b'0' || c == b'1'))
                    || c == b'_'
                {
                    if c != b'_' {
                        num_str.push(c as char);
                    }
                    self.advance();
                } else {
                    break;
                }
            }

            let val = u128::from_str_radix(&num_str, radix).map_err(|_| LexError {
                message: format!("Malformed integer literal: {}", num_str),
                span: Span::new(start_pos, self.pos, start_line, start_col),
            })?;

            // Check optional type suffix (u8, u16, u32, u64, usize, i8, etc.)
            let mut suffix = None;
            if let Some(c) = self.peek() {
                if c == b'u' || c == b'i' {
                    let mut s = String::new();
                    while let Some(sc) = self.peek() {
                        if sc.is_ascii_alphanumeric() {
                            s.push(sc as char);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    suffix = Some(s);
                }
            }

            return Ok(Token {
                kind: TokenKind::Int(val, suffix),
                span: Span::new(start_pos, self.pos, start_line, start_col),
            });
        }

        // Punctuation and multi-character operators
        self.advance();
        let kind = match b {
            b'(' => TokenKind::LParen,
            b')' => TokenKind::RParen,
            b'{' => TokenKind::LBrace,
            b'}' => TokenKind::RBrace,
            b'[' => TokenKind::LBracket,
            b']' => TokenKind::RBracket,
            b';' => TokenKind::Semi,
            b',' => TokenKind::Comma,
            b'#' => TokenKind::Hash,
            b':' => {
                if self.peek() == Some(b':') {
                    self.advance();
                    TokenKind::ColonColon
                } else {
                    TokenKind::Colon
                }
            }
            b'-' => {
                if self.peek() == Some(b'>') {
                    self.advance();
                    TokenKind::Arrow
                } else if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::MinusEq
                } else {
                    TokenKind::Minus
                }
            }
            b'.' => {
                if self.peek() == Some(b'.') {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        TokenKind::DotDotEq
                    } else {
                        TokenKind::DotDot
                    }
                } else {
                    TokenKind::Dot
                }
            }
            b'=' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::EqEq
                } else {
                    TokenKind::Eq
                }
            }
            b'!' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::Ne
                } else {
                    TokenKind::Bang
                }
            }
            b'<' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::LtEq
                } else if self.peek() == Some(b'<') {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        TokenKind::ShlEq
                    } else {
                        TokenKind::Shl
                    }
                } else {
                    TokenKind::Lt
                }
            }
            b'>' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::GtEq
                } else if self.peek() == Some(b'>') {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        TokenKind::ShrEq
                    } else {
                        TokenKind::Shr
                    }
                } else {
                    TokenKind::Gt
                }
            }
            b'+' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::PlusEq
                } else {
                    TokenKind::Plus
                }
            }
            b'*' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::StarEq
                } else {
                    TokenKind::Star
                }
            }
            b'/' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::SlashEq
                } else {
                    TokenKind::Slash
                }
            }
            b'%' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::PercentEq
                } else {
                    TokenKind::Percent
                }
            }
            b'&' => {
                if self.peek() == Some(b'&') {
                    self.advance();
                    TokenKind::AndAnd
                } else if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::AndEq
                } else {
                    TokenKind::And
                }
            }
            b'|' => {
                if self.peek() == Some(b'|') {
                    self.advance();
                    TokenKind::OrOr
                } else if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::OrEq
                } else {
                    TokenKind::Or
                }
            }
            b'^' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::CaretEq
                } else {
                    TokenKind::Caret
                }
            }
            _ => {
                return Err(LexError {
                    message: format!("Unexpected byte: '{}'", b as char),
                    span: Span::new(start_pos, self.pos, start_line, start_col),
                });
            }
        };

        Ok(Token {
            kind,
            span: Span::new(start_pos, self.pos, start_line, start_col),
        })
    }

    pub fn tokenize_all(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            if tokens.len() >= MAX_TOKENS {
                return Err(LexError {
                    message: format!("Maximum token limit of {} exceeded", MAX_TOKENS),
                    span: Span::new(self.pos, self.pos, self.line, self.col),
                });
            }
            let tok = self.next_token()?;
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }
}
