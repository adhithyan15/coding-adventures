//! Error types for the LaTeX front-end.
//!
//! Every error carries a half-open byte [`span`](LexError::span) `(start, end)` into the
//! source so a caller can underline the exact offending slice. The tokenizer reports
//! [`LexError`]; later layers (the structural and math parsers) report a `ParseError`.

/// A lexical error: the input could not be split into LaTeX tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    /// Half-open byte span `[start, end)` into the source.
    pub span: (usize, usize),
}

impl LexError {
    pub fn new(message: impl Into<String>, start: usize, end: usize) -> Self {
        LexError {
            message: message.into(),
            span: (start, end),
        }
    }
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "lex error at bytes {}..{}: {}", self.span.0, self.span.1, self.message)
    }
}

impl std::error::Error for LexError {}
