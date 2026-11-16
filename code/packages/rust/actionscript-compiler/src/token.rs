//! Token definitions and source spans for ActionScript lexing.

#![deny(unsafe_code)]

/// Source span indicating position in source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Span {
    /// Create a new source span.
    pub fn span_new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self {
            start,
            end,
            line,
            column,
        }
    }

    /// Default empty span.
    pub fn span_empty() -> Self {
        Self {
            start: 0,
            end: 0,
            line: 1,
            column: 1,
        }
    }
}

/// Token kinds across ActionScript 1.0, 2.0, and 3.0.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Number(f64),
    StringLiteral(String),
    BoolLiteral(bool),
    Null,
    Undefined,

    // Identifiers
    Ident(String),

    // Common ECMAScript Keywords
    Var,
    Function,
    Return,
    If,
    Else,
    While,
    Do,
    For,
    In,
    Break,
    Continue,
    This,
    New,
    Delete,
    Typeof,
    Instanceof,

    // Flash / AS1 Timeline Verbs & Paths
    Stop,
    Play,
    GotoAndPlay,
    GotoAndStop,
    TellTarget,
    Call,
    Eval,
    OnClipEvent,
    Trace,
    Root,
    Parent,
    Global,

    // AS2 Keywords
    Class,
    Extends,
    Implements,
    Interface,
    Public,
    Private,
    Static,
    Import,

    // AS3 Keywords
    Package,
    Namespace,
    Const,
    Try,
    Catch,
    Finally,
    Throw,
    Is,
    As,
    Override,
    Internal,
    Final,
    Dynamic,
    Native,
    Use,
    Vector,

    // Operators & Punctuation
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    EqEq,
    NotEq,
    StrictEq,
    StrictNotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    AmpAmp,
    PipePipe,
    Bang,
    Amp,
    Pipe,
    Caret,
    Tilde,
    Shl,
    Shr,
    Ushr,
    PlusPlus,
    MinusMinus,
    Question,
    Colon,
    Dot,
    DotLt,
    Comma,
    Semicolon,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    // End of file
    Eof,
}

/// A scanned token with span and raw text.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub text: String,
}

impl Token {
    /// Construct a new token.
    pub fn token_new(kind: TokenKind, span: Span, text: String) -> Self {
        Self { kind, span, text }
    }
}
