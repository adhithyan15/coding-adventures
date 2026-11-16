//! # The lexer
//!
//! A shape check is only as honest as its tokens.  Reading source with
//! `contains("unsafe")` would flag the word inside a comment, a string or the
//! attribute `#![forbid(unsafe_code)]`, and would miss `r#unsafe`-style tricks
//! in the other direction.  So we tokenise first, the way `rustc` does, and
//! every rule looks only at tokens.
//!
//! ```text
//!   source text          tokens
//!   ------------------   --------------------------------------------
//!   let x = 1u8 << n;    Ident(let) Ident(x) Punct(=) Int(1u8)
//!                        Punct(<) Punct(<) Ident(n) Punct(;)
//!   // unsafe            (nothing: a comment is not code)
//!   "unsafe { }"         Str(unsafe { })      <- data, not code
//!   'a  vs  'a'          Lifetime('a)  vs  Char(a)
//! ```
//!
//! ## What is, and is not, a token here
//!
//! * Comments (`//`, `/* */` with nesting, doc comments) are dropped.
//! * Strings of every flavour (`"..."`, `b".."`, `c".."`, raw `r#".."#`) become
//!   one `Str` token whose text is the characters between the quotes.
//! * An identifier is `Ident`.  A *raw* identifier `r#unsafe` keeps its `r#`
//!   in the text, so it can never be mistaken for the keyword `unsafe`.
//! * Numbers become `Int` or `Float`.  After a `.` token a number is read as
//!   digits only, so the tuple field chain `t.0.1` is two integers and not the
//!   float `0.1`.
//! * Punctuation is one character per token, with one exception: `::`, `->`,
//!   `=>` and `..` are single tokens, because generic-bracket counting must
//!   not mistake the `>` of `->` for a closing angle bracket.  Every other
//!   punctuation token records `joint`: is the very next character also
//!   punctuation?  That is how `<<`, `>>`, `/=` and `!=` are recognised
//!   without lexing `Vec<Vec<u8>>` as a shift.
//!
//! ## Fail closed
//!
//! Input the lexer cannot classify (an unterminated string or block comment,
//! a stray non-ASCII symbol, more than [`MAX_TOKENS`] tokens, unbalanced
//! brackets) is an error, never a silent skip: a skipped region is exactly
//! where a violation would hide (VOS03 section 3).

/// Cap on tokens per file.  A source file is capped at 1 MiB, so this is
/// about four tokens per three bytes; real code is far below it.
pub const MAX_TOKENS: usize = 262_144;

/// The kind of a token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Identifier or keyword (raw identifiers keep their `r#` prefix).
    Ident,
    /// A lifetime or loop label such as `'a` or `'static`.
    Lifetime,
    /// An integer literal, suffix included (`1u8`, `0xff`).
    Int,
    /// A float literal (`1.0`, `1e3`, `2f32`).
    Float,
    /// A string literal of any flavour; the text is the inside of the quotes.
    Str,
    /// A character or byte literal; the text is the inside of the quotes.
    Char,
    /// Punctuation.
    Punct,
}

/// One token with its 1-based source line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    /// What it is.
    pub kind: Kind,
    /// The text (see [`Kind`] for what that means per kind).
    pub text: String,
    /// 1-based line on which the token starts.
    pub line: u32,
    /// For punctuation: the next character is punctuation with no gap.
    pub joint: bool,
}

impl Token {
    /// True if this is the identifier `s` (a raw identifier never matches).
    pub fn is_ident(&self, s: &str) -> bool {
        self.kind == Kind::Ident && self.text == s
    }

    /// True if this is the punctuation `s`.
    pub fn is_punct(&self, s: &str) -> bool {
        self.kind == Kind::Punct && self.text == s
    }
}

/// A lexing failure: where and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexError {
    /// 1-based line.
    pub line: u32,
    /// Human-readable reason.
    pub message: String,
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

/// ASCII punctuation as the lexer treats it (everything that is not a
/// letter, digit, quote, whitespace or underscore).
fn is_punct_char(c: char) -> bool {
    c.is_ascii_punctuation() && c != '_' && c != '"' && c != '\''
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: u32,
    out: Vec<Token>,
}

impl Lexer {
    fn peek(&self, off: usize) -> Option<char> {
        self.chars.get(self.pos + off).copied()
    }

    fn err<T>(&self, line: u32, message: &str) -> Result<T, LexError> {
        Err(LexError {
            line,
            message: message.to_string(),
        })
    }

    fn push(&mut self, kind: Kind, text: String, line: u32) -> Result<(), LexError> {
        if self.out.len() >= MAX_TOKENS {
            return self.err(line, "token cap exceeded (262144 tokens in one file)");
        }
        let joint = kind == Kind::Punct && self.peek(0).is_some_and(is_punct_char);
        self.out.push(Token {
            kind,
            text,
            line,
            joint,
        });
        Ok(())
    }

    /// Skip a `/* ... */` comment; Rust block comments nest.
    fn block_comment(&mut self) -> Result<(), LexError> {
        let start = self.line;
        self.pos += 2;
        let mut depth = 1usize;
        while depth > 0 {
            match (self.peek(0), self.peek(1)) {
                (Some('/'), Some('*')) => {
                    depth += 1;
                    self.pos += 2;
                }
                (Some('*'), Some('/')) => {
                    depth -= 1;
                    self.pos += 2;
                }
                (Some(c), _) => {
                    if c == '\n' {
                        self.line += 1;
                    }
                    self.pos += 1;
                }
                (None, _) => return self.err(start, "unterminated block comment"),
            }
        }
        Ok(())
    }

    /// Read a cooked string body; `self.pos` is just after the opening quote.
    fn cooked(&mut self, quote: char, line: u32) -> Result<String, LexError> {
        let mut text = String::new();
        loop {
            match self.peek(0) {
                None => return self.err(line, "unterminated string or character literal"),
                Some('\\') => {
                    text.push('\\');
                    self.pos += 1;
                    if let Some(c) = self.peek(0) {
                        if c == '\n' {
                            self.line += 1;
                        }
                        text.push(c);
                        self.pos += 1;
                    }
                }
                Some(c) if c == quote => {
                    self.pos += 1;
                    return Ok(text);
                }
                Some(c) => {
                    if c == '\n' {
                        self.line += 1;
                    }
                    text.push(c);
                    self.pos += 1;
                }
            }
        }
    }

    /// Read a raw string; `self.pos` is at the first `#` or the quote.
    fn raw(&mut self, line: u32) -> Result<String, LexError> {
        let mut hashes = 0usize;
        while self.peek(0) == Some('#') {
            hashes += 1;
            self.pos += 1;
        }
        if self.peek(0) != Some('"') {
            return self.err(line, "malformed raw string");
        }
        self.pos += 1;
        let mut text = String::new();
        loop {
            match self.peek(0) {
                None => return self.err(line, "unterminated raw string"),
                Some('"') => {
                    let mut n = 0usize;
                    while n < hashes && self.peek(1 + n) == Some('#') {
                        n += 1;
                    }
                    if n == hashes {
                        self.pos += 1 + hashes;
                        return Ok(text);
                    }
                    text.push('"');
                    self.pos += 1;
                }
                Some(c) => {
                    if c == '\n' {
                        self.line += 1;
                    }
                    text.push(c);
                    self.pos += 1;
                }
            }
        }
    }

    /// An identifier, or a prefixed literal (`b"..`, `r#".."#`, `c".."`,
    /// `b'x'`) or a raw identifier (`r#name`).
    fn word(&mut self) -> Result<(), LexError> {
        let line = self.line;
        let start = self.pos;
        while self.peek(0).is_some_and(is_ident_continue) {
            self.pos += 1;
        }
        let word: String = self.chars[start..self.pos].iter().collect();
        let next = self.peek(0);
        let is_raw_prefix = matches!(word.as_str(), "r" | "br" | "cr");
        let is_cooked_prefix = matches!(word.as_str(), "b" | "c");
        if is_cooked_prefix && next == Some('"') {
            self.pos += 1;
            let t = self.cooked('"', line)?;
            return self.push(Kind::Str, t, line);
        }
        if word == "b" && next == Some('\'') {
            self.pos += 1;
            let t = self.cooked('\'', line)?;
            return self.push(Kind::Char, t, line);
        }
        if is_raw_prefix && next == Some('"') {
            let t = self.raw(line)?;
            return self.push(Kind::Str, t, line);
        }
        if is_raw_prefix && next == Some('#') {
            // `r#"..."#` is a raw string; `r#name` is a raw identifier.
            let mut k = 0usize;
            while self.peek(k) == Some('#') {
                k += 1;
            }
            if self.peek(k) == Some('"') {
                let t = self.raw(line)?;
                return self.push(Kind::Str, t, line);
            }
            if word == "r" && k == 1 && self.peek(1).is_some_and(is_ident_start) {
                self.pos += 1;
                let s2 = self.pos;
                while self.peek(0).is_some_and(is_ident_continue) {
                    self.pos += 1;
                }
                let name: String = self.chars[s2..self.pos].iter().collect();
                return self.push(Kind::Ident, format!("r#{name}"), line);
            }
            return self.err(line, "malformed raw string or raw identifier");
        }
        self.push(Kind::Ident, word, line)
    }

    /// A character literal or a lifetime, both of which start with `'`.
    fn quote(&mut self) -> Result<(), LexError> {
        let line = self.line;
        match (self.peek(1), self.peek(2)) {
            (Some('\\'), _) => {
                self.pos += 1;
                let t = self.cooked('\'', line)?;
                self.push(Kind::Char, t, line)
            }
            (Some(c), Some('\'')) if c != '\'' => {
                self.pos += 3;
                self.push(Kind::Char, c.to_string(), line)
            }
            (Some(c), _) if is_ident_start(c) => {
                let start = self.pos;
                self.pos += 1;
                while self.peek(0).is_some_and(is_ident_continue) {
                    self.pos += 1;
                }
                let t: String = self.chars[start..self.pos].iter().collect();
                self.push(Kind::Lifetime, t, line)
            }
            _ => self.err(line, "unrecognised use of `'`"),
        }
    }

    /// A number.  `after_dot` means the previous token was a single `.`
    /// (a tuple index), so only digits are consumed.
    fn number(&mut self, after_dot: bool) -> Result<(), LexError> {
        let line = self.line;
        let start = self.pos;
        let mut float = false;
        if after_dot {
            while self.peek(0).is_some_and(|c| c.is_ascii_digit()) {
                self.pos += 1;
            }
        } else if self.peek(0) == Some('0') && matches!(self.peek(1), Some('x' | 'o' | 'b')) {
            self.pos += 2;
            while self
                .peek(0)
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                self.pos += 1;
            }
        } else {
            let digits = |l: &mut Self| {
                while l.peek(0).is_some_and(|c| c.is_ascii_digit() || c == '_') {
                    l.pos += 1;
                }
            };
            digits(self);
            if self.peek(0) == Some('.') && self.peek(1).is_some_and(|c| c.is_ascii_digit()) {
                float = true;
                self.pos += 1;
                digits(self);
            }
            if matches!(self.peek(0), Some('e' | 'E')) {
                let sign = matches!(self.peek(1), Some('+' | '-'));
                let digit_at = if sign { 2 } else { 1 };
                if self.peek(digit_at).is_some_and(|c| c.is_ascii_digit()) {
                    float = true;
                    self.pos += digit_at;
                    digits(self);
                }
            }
            while self
                .peek(0)
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                self.pos += 1;
            }
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        if text.ends_with("f32") || text.ends_with("f64") {
            float = true;
        }
        let kind = if float { Kind::Float } else { Kind::Int };
        self.push(kind, text, line)
    }

    fn punct(&mut self) -> Result<(), LexError> {
        let line = self.line;
        let c = self.chars[self.pos];
        let two: Option<&str> = match (c, self.peek(1)) {
            (':', Some(':')) => Some("::"),
            ('-', Some('>')) => Some("->"),
            ('=', Some('>')) => Some("=>"),
            ('.', Some('.')) => Some(".."),
            _ => None,
        };
        if let Some(t) = two {
            self.pos += 2;
            return self.push(Kind::Punct, t.to_string(), line);
        }
        self.pos += 1;
        self.push(Kind::Punct, c.to_string(), line)
    }
}

/// Tokenise `src`, or say exactly why that is impossible.
///
/// Brackets are checked to balance (`()`, `[]`, `{}`) by [`pair_brackets`],
/// which every rule needs anyway.
pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let text = src.strip_prefix('\u{feff}').unwrap_or(src);
    let mut lx = Lexer {
        chars: text.chars().collect(),
        pos: 0,
        line: 1,
        out: Vec::new(),
    };
    // A shebang line is not Rust.  `#![` is an attribute, not a shebang.
    if text.starts_with("#!") && !text.starts_with("#![") {
        while lx.peek(0).is_some_and(|c| c != '\n') {
            lx.pos += 1;
        }
    }
    while let Some(c) = lx.peek(0) {
        let line = lx.line;
        if c == '\n' {
            lx.line += 1;
            lx.pos += 1;
        } else if c.is_whitespace() {
            lx.pos += 1;
        } else if c == '/' && lx.peek(1) == Some('/') {
            while lx.peek(0).is_some_and(|c| c != '\n') {
                lx.pos += 1;
            }
        } else if c == '/' && lx.peek(1) == Some('*') {
            lx.block_comment()?;
        } else if c == '"' {
            lx.pos += 1;
            let t = lx.cooked('"', line)?;
            lx.push(Kind::Str, t, line)?;
        } else if c == '\'' {
            lx.quote()?;
        } else if c.is_ascii_digit() {
            let after_dot = lx.out.last().is_some_and(|t| t.is_punct("."));
            lx.number(after_dot)?;
        } else if is_ident_start(c) {
            lx.word()?;
        } else if is_punct_char(c) {
            lx.punct()?;
        } else {
            return lx.err(line, "character the lexer cannot classify");
        }
    }
    Ok(lx.out)
}

/// For every bracket token, the index of its partner; `usize::MAX` for all
/// other tokens.  Unbalanced or mismatched brackets are an error.
pub fn pair_brackets(toks: &[Token]) -> Result<Vec<usize>, LexError> {
    let mut pair = vec![usize::MAX; toks.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        if t.kind != Kind::Punct {
            continue;
        }
        match t.text.as_str() {
            "(" | "[" | "{" => stack.push(i),
            ")" | "]" | "}" => {
                let Some(open) = stack.pop() else {
                    return Err(LexError {
                        line: t.line,
                        message: format!("unmatched `{}`", t.text),
                    });
                };
                let want = match toks[open].text.as_str() {
                    "(" => ")",
                    "[" => "]",
                    _ => "}",
                };
                if t.text != want {
                    return Err(LexError {
                        line: t.line,
                        message: format!("`{}` closes `{}`", t.text, toks[open].text),
                    });
                }
                pair[open] = i;
                pair[i] = open;
            }
            _ => {}
        }
    }
    if let Some(open) = stack.pop() {
        return Err(LexError {
            line: toks[open].line,
            message: format!("unclosed `{}`", toks[open].text),
        });
    }
    Ok(pair)
}
