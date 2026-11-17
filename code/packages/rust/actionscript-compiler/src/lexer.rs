//! ActionScript lexer supporting AS1, AS2, and AS3 dialects.

#![deny(unsafe_code)]

use crate::dialect::AsDialect;
use crate::error::{AsCompilerError, MAX_TOKENS};
use crate::token::{Span, Token, TokenKind};

/// ActionScript lexical analyzer.
pub struct AsLexer<'a> {
    source: &'a str,
    chars: Vec<(usize, char)>,
    cursor: usize,
    line: usize,
    column: usize,
    dialect: AsDialect,
    token_count: usize,
}

impl<'a> AsLexer<'a> {
    /// Create a new lexer for the given source and dialect.
    pub fn lexer_new(source: &'a str, dialect: AsDialect) -> Self {
        let chars: Vec<(usize, char)> = source.char_indices().collect();
        Self {
            source,
            chars,
            cursor: 0,
            line: 1,
            column: 1,
            dialect,
            token_count: 0,
        }
    }

    /// Tokenize the entire source into a vector of tokens ending with EOF.
    pub fn lexer_tokenize(&mut self) -> Result<Vec<Token>, AsCompilerError> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.lexer_next_token()?;
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    /// Read the next token.
    pub fn lexer_next_token(&mut self) -> Result<Token, AsCompilerError> {
        self.lexer_skip_whitespace_and_comments()?;

        if self.token_count >= MAX_TOKENS {
            return Err(AsCompilerError::TokenLimitExceeded);
        }

        if self.cursor >= self.chars.len() {
            let start = self.source.len();
            let span = Span::span_new(start, start, self.line, self.column);
            self.token_count += 1;
            return Ok(Token::token_new(TokenKind::Eof, span, String::new()));
        }

        let (start_byte, ch) = self.chars[self.cursor];
        let start_line = self.line;
        let start_col = self.column;

        // String literals
        if ch == '"' || ch == '\'' {
            return self.lexer_read_string(ch, start_byte, start_line, start_col);
        }

        // Numbers: digits or '.' followed by digit
        if ch.is_ascii_digit()
            || (ch == '.'
                && self.cursor + 1 < self.chars.len()
                && self.chars[self.cursor + 1].1.is_ascii_digit())
        {
            return self.lexer_read_number(start_byte, start_line, start_col);
        }

        // Identifiers and keywords
        if ch.is_alphabetic() || ch == '_' || ch == '$' {
            return self.lexer_read_ident(start_byte, start_line, start_col);
        }

        // Punctuation and Operators
        self.lexer_advance(); // Consume first char

        let (kind, end_byte) = match ch {
            '+' => {
                if self.lexer_match('+') {
                    (TokenKind::PlusPlus, self.lexer_prev_end_byte())
                } else if self.lexer_match('=') {
                    (TokenKind::PlusEq, self.lexer_prev_end_byte())
                } else {
                    (TokenKind::Plus, start_byte + 1)
                }
            }
            '-' => {
                if self.lexer_match('-') {
                    (TokenKind::MinusMinus, self.lexer_prev_end_byte())
                } else if self.lexer_match('=') {
                    (TokenKind::MinusEq, self.lexer_prev_end_byte())
                } else {
                    (TokenKind::Minus, start_byte + 1)
                }
            }
            '*' => {
                if self.lexer_match('=') {
                    (TokenKind::StarEq, self.lexer_prev_end_byte())
                } else {
                    (TokenKind::Star, start_byte + 1)
                }
            }
            '/' => {
                if self.lexer_match('=') {
                    (TokenKind::SlashEq, self.lexer_prev_end_byte())
                } else {
                    (TokenKind::Slash, start_byte + 1)
                }
            }
            '%' => {
                if self.lexer_match('=') {
                    (TokenKind::PercentEq, self.lexer_prev_end_byte())
                } else {
                    (TokenKind::Percent, start_byte + 1)
                }
            }
            '=' => {
                if self.lexer_match('=') {
                    if self.lexer_match('=') {
                        (TokenKind::StrictEq, self.lexer_prev_end_byte())
                    } else {
                        (TokenKind::EqEq, self.lexer_prev_end_byte())
                    }
                } else {
                    (TokenKind::Eq, start_byte + 1)
                }
            }
            '!' => {
                if self.lexer_match('=') {
                    if self.lexer_match('=') {
                        (TokenKind::StrictNotEq, self.lexer_prev_end_byte())
                    } else {
                        (TokenKind::NotEq, self.lexer_prev_end_byte())
                    }
                } else {
                    (TokenKind::Bang, start_byte + 1)
                }
            }
            '<' => {
                if self.lexer_match('<') {
                    (TokenKind::Shl, self.lexer_prev_end_byte())
                } else if self.lexer_match('=') {
                    (TokenKind::LtEq, self.lexer_prev_end_byte())
                } else {
                    (TokenKind::Lt, start_byte + 1)
                }
            }
            '>' => {
                if self.lexer_match('>') {
                    if self.lexer_match('>') {
                        (TokenKind::Ushr, self.lexer_prev_end_byte())
                    } else {
                        (TokenKind::Shr, self.lexer_prev_end_byte())
                    }
                } else if self.lexer_match('=') {
                    (TokenKind::GtEq, self.lexer_prev_end_byte())
                } else {
                    (TokenKind::Gt, start_byte + 1)
                }
            }
            '&' => {
                if self.lexer_match('&') {
                    (TokenKind::AmpAmp, self.lexer_prev_end_byte())
                } else {
                    (TokenKind::Amp, start_byte + 1)
                }
            }
            '|' => {
                if self.lexer_match('|') {
                    (TokenKind::PipePipe, self.lexer_prev_end_byte())
                } else {
                    (TokenKind::Pipe, start_byte + 1)
                }
            }
            '^' => (TokenKind::Caret, start_byte + 1),
            '~' => (TokenKind::Tilde, start_byte + 1),
            '?' => (TokenKind::Question, start_byte + 1),
            ':' => (TokenKind::Colon, start_byte + 1),
            '.' => {
                if self.dialect == AsDialect::As3 && self.lexer_match('<') {
                    (TokenKind::DotLt, self.lexer_prev_end_byte())
                } else {
                    (TokenKind::Dot, start_byte + 1)
                }
            }
            ',' => (TokenKind::Comma, start_byte + 1),
            ';' => (TokenKind::Semicolon, start_byte + 1),
            '(' => (TokenKind::LParen, start_byte + 1),
            ')' => (TokenKind::RParen, start_byte + 1),
            '{' => (TokenKind::LBrace, start_byte + 1),
            '}' => (TokenKind::RBrace, start_byte + 1),
            '[' => (TokenKind::LBracket, start_byte + 1),
            ']' => (TokenKind::RBracket, start_byte + 1),
            other => {
                return Err(AsCompilerError::SyntaxError(format!(
                    "unexpected character '{}' at line {}, column {}",
                    other, start_line, start_col
                )));
            }
        };

        let raw_text = self.source[start_byte..end_byte].to_string();
        let span = Span::span_new(start_byte, end_byte, start_line, start_col);
        self.token_count += 1;
        Ok(Token::token_new(kind, span, raw_text))
    }

    fn lexer_read_string(
        &mut self,
        quote: char,
        start_byte: usize,
        start_line: usize,
        start_col: usize,
    ) -> Result<Token, AsCompilerError> {
        self.lexer_advance(); // Consume opening quote
        let mut buf = String::new();

        while self.cursor < self.chars.len() {
            let (_, ch) = self.chars[self.cursor];
            if ch == quote {
                self.lexer_advance(); // Consume closing quote
                let end_byte = self.lexer_prev_end_byte();
                let span = Span::span_new(start_byte, end_byte, start_line, start_col);
                self.token_count += 1;
                return Ok(Token::token_new(
                    TokenKind::StringLiteral(buf),
                    span,
                    self.source[start_byte..end_byte].to_string(),
                ));
            } else if ch == '\\' {
                self.lexer_advance();
                if self.cursor >= self.chars.len() {
                    return Err(AsCompilerError::SyntaxError(
                        "unterminated escape sequence in string literal".into(),
                    ));
                }
                let (_, esc) = self.chars[self.cursor];
                self.lexer_advance();
                match esc {
                    'n' => buf.push('\n'),
                    'r' => buf.push('\r'),
                    't' => buf.push('\t'),
                    '\\' => buf.push('\\'),
                    '\'' => buf.push('\''),
                    '"' => buf.push('"'),
                    '0' => buf.push('\0'),
                    other => buf.push(other),
                }
            } else {
                buf.push(ch);
                self.lexer_advance();
            }
        }

        Err(AsCompilerError::SyntaxError(
            "unterminated string literal".into(),
        ))
    }

    fn lexer_read_number(
        &mut self,
        start_byte: usize,
        start_line: usize,
        start_col: usize,
    ) -> Result<Token, AsCompilerError> {
        // Check for hex: 0x or 0X
        if self.cursor + 1 < self.chars.len()
            && self.chars[self.cursor].1 == '0'
            && (self.chars[self.cursor + 1].1 == 'x' || self.chars[self.cursor + 1].1 == 'X')
        {
            self.lexer_advance(); // '0'
            self.lexer_advance(); // 'x'
            let hex_start = self.cursor;
            while self.cursor < self.chars.len() && self.chars[self.cursor].1.is_ascii_hexdigit() {
                self.lexer_advance();
            }
            if self.cursor == hex_start {
                return Err(AsCompilerError::SyntaxError("malformed hex literal".into()));
            }
            let end_byte = self.lexer_prev_end_byte();
            let raw = &self.source[start_byte..end_byte];
            let val = i64::from_str_radix(&raw[2..], 16)
                .map_err(|e| AsCompilerError::SyntaxError(format!("invalid hex literal: {}", e)))?
                as f64;

            let span = Span::span_new(start_byte, end_byte, start_line, start_col);
            self.token_count += 1;
            return Ok(Token::token_new(
                TokenKind::Number(val),
                span,
                raw.to_string(),
            ));
        }

        // Decimal / Float
        let mut has_dot = false;
        while self.cursor < self.chars.len() {
            let (_, ch) = self.chars[self.cursor];
            if ch.is_ascii_digit() {
                self.lexer_advance();
            } else if ch == '.' && !has_dot {
                // Peek next: if next is '<' in AS3, do not consume '.' (it is .<)
                if self.dialect == AsDialect::As3
                    && self.cursor + 1 < self.chars.len()
                    && self.chars[self.cursor + 1].1 == '<'
                {
                    break;
                }
                has_dot = true;
                self.lexer_advance();
            } else {
                break;
            }
        }

        let end_byte = self.lexer_prev_end_byte();
        let raw = &self.source[start_byte..end_byte];
        let val: f64 = raw.parse().map_err(|e| {
            AsCompilerError::SyntaxError(format!("invalid numeric literal '{}': {}", raw, e))
        })?;

        let span = Span::span_new(start_byte, end_byte, start_line, start_col);
        self.token_count += 1;
        Ok(Token::token_new(
            TokenKind::Number(val),
            span,
            raw.to_string(),
        ))
    }

    fn lexer_read_ident(
        &mut self,
        start_byte: usize,
        start_line: usize,
        start_col: usize,
    ) -> Result<Token, AsCompilerError> {
        while self.cursor < self.chars.len() {
            let (_, ch) = self.chars[self.cursor];
            if ch.is_alphanumeric() || ch == '_' || ch == '$' {
                self.lexer_advance();
            } else {
                break;
            }
        }

        let end_byte = self.lexer_prev_end_byte();
        let raw = &self.source[start_byte..end_byte];

        let kind = match raw {
            // Literals
            "true" => TokenKind::BoolLiteral(true),
            "false" => TokenKind::BoolLiteral(false),
            "null" => TokenKind::Null,
            "undefined" => TokenKind::Undefined,

            // Common keywords
            "var" => TokenKind::Var,
            "function" => TokenKind::Function,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "do" => TokenKind::Do,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "this" => TokenKind::This,
            "new" => TokenKind::New,
            "delete" => TokenKind::Delete,
            "typeof" => TokenKind::Typeof,
            "instanceof" => TokenKind::Instanceof,

            // Flash AS1 verbs and paths
            "stop" => TokenKind::Stop,
            "play" => TokenKind::Play,
            "gotoAndPlay" => TokenKind::GotoAndPlay,
            "gotoAndStop" => TokenKind::GotoAndStop,
            "tellTarget" => TokenKind::TellTarget,
            "call" => TokenKind::Call,
            "eval" => TokenKind::Eval,
            "onClipEvent" => TokenKind::OnClipEvent,
            "trace" => TokenKind::Trace,
            "_root" => TokenKind::Root,
            "_parent" => TokenKind::Parent,
            "_global" => TokenKind::Global,

            // AS2 keywords
            "class" if self.dialect != AsDialect::As1 => TokenKind::Class,
            "extends" if self.dialect != AsDialect::As1 => TokenKind::Extends,
            "implements" if self.dialect != AsDialect::As1 => TokenKind::Implements,
            "interface" if self.dialect != AsDialect::As1 => TokenKind::Interface,
            "public" if self.dialect != AsDialect::As1 => TokenKind::Public,
            "private" if self.dialect != AsDialect::As1 => TokenKind::Private,
            "static" if self.dialect != AsDialect::As1 => TokenKind::Static,
            "import" if self.dialect != AsDialect::As1 => TokenKind::Import,

            // AS3 keywords
            "package" if self.dialect == AsDialect::As3 => TokenKind::Package,
            "namespace" if self.dialect == AsDialect::As3 => TokenKind::Namespace,
            "const" if self.dialect == AsDialect::As3 => TokenKind::Const,
            "try" if self.dialect == AsDialect::As3 => TokenKind::Try,
            "catch" if self.dialect == AsDialect::As3 => TokenKind::Catch,
            "finally" if self.dialect == AsDialect::As3 => TokenKind::Finally,
            "throw" if self.dialect == AsDialect::As3 => TokenKind::Throw,
            "is" if self.dialect == AsDialect::As3 => TokenKind::Is,
            "as" if self.dialect == AsDialect::As3 => TokenKind::As,
            "override" if self.dialect == AsDialect::As3 => TokenKind::Override,
            "internal" if self.dialect == AsDialect::As3 => TokenKind::Internal,
            "final" if self.dialect == AsDialect::As3 => TokenKind::Final,
            "dynamic" if self.dialect == AsDialect::As3 => TokenKind::Dynamic,
            "native" if self.dialect == AsDialect::As3 => TokenKind::Native,
            "use" if self.dialect == AsDialect::As3 => TokenKind::Use,
            "Vector" if self.dialect == AsDialect::As3 => TokenKind::Vector,

            _ => TokenKind::Ident(raw.to_string()),
        };

        let span = Span::span_new(start_byte, end_byte, start_line, start_col);
        self.token_count += 1;
        Ok(Token::token_new(kind, span, raw.to_string()))
    }

    fn lexer_advance(&mut self) {
        if self.cursor < self.chars.len() {
            let (_, ch) = self.chars[self.cursor];
            self.cursor += 1;
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
    }

    fn lexer_match(&mut self, expected: char) -> bool {
        if self.cursor < self.chars.len() && self.chars[self.cursor].1 == expected {
            self.lexer_advance();
            true
        } else {
            false
        }
    }

    fn lexer_prev_end_byte(&self) -> usize {
        if self.cursor < self.chars.len() {
            self.chars[self.cursor].0
        } else {
            self.source.len()
        }
    }

    fn lexer_skip_whitespace_and_comments(&mut self) -> Result<(), AsCompilerError> {
        while self.cursor < self.chars.len() {
            let (_, ch) = self.chars[self.cursor];
            if ch.is_whitespace() {
                self.lexer_advance();
            } else if ch == '/' && self.cursor + 1 < self.chars.len() {
                let next = self.chars[self.cursor + 1].1;
                if next == '/' {
                    // Line comment
                    self.lexer_advance(); // '/'
                    self.lexer_advance(); // '/'
                    while self.cursor < self.chars.len() && self.chars[self.cursor].1 != '\n' {
                        self.lexer_advance();
                    }
                } else if next == '*' {
                    // Block comment
                    self.lexer_advance(); // '/'
                    self.lexer_advance(); // '*'
                    let mut closed = false;
                    while self.cursor + 1 < self.chars.len() {
                        if self.chars[self.cursor].1 == '*' && self.chars[self.cursor + 1].1 == '/'
                        {
                            self.lexer_advance(); // '*'
                            self.lexer_advance(); // '/'
                            closed = true;
                            break;
                        }
                        self.lexer_advance();
                    }
                    if !closed {
                        return Err(AsCompilerError::SyntaxError(
                            "unterminated block comment".into(),
                        ));
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        Ok(())
    }
}
