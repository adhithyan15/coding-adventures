//! Tokens — the alphabet the catcode [`crate::lexer`] emits.
//!
//! A LaTeX token is the smallest meaningful unit: a control sequence, a group brace, a
//! math-mode shift, an ordinary character, a run of whitespace, a paragraph break, a
//! comment. Unlike a programming-language lexer, the LaTeX tokenizer does **not** coalesce
//! ordinary characters into words — in TeX each ordinary character is its own token, and
//! grouping them into runs of text is the *parser's* job (the next layer). This keeps the
//! tokenizer a faithful image of TeX's mouth.
//!
//! Every token records its half-open byte [`Span`].

/// A half-open byte range `[start, end)` into the source. `&src[start..end]` is the slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }
}

/// One lexical token plus its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, start: usize, end: usize) -> Self {
        Token { kind, span: Span::new(start, end) }
    }
}

/// The kinds of LaTeX token.
///
/// Math-mode shifts are emitted as paired [`TokenKind::MathOn`] / [`TokenKind::MathOff`]
/// rather than a raw `$`, because the tokenizer tracks the math/text mode stack and so
/// already knows whether a given `$` opens or closes — handing the parser an unambiguous
/// on/off (with the inline-vs-display flag) instead of a toggle it would have to track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// A control word: `\` followed by one or more letters — `\frac`, `\begin`, `\alpha`.
    ControlWord(String),
    /// A control symbol: `\` followed by a single non-letter — `\,`, `\{`, `\%`, and `\\`
    /// (stored as the char `\\`, the line break).
    ControlSymbol(char),
    /// `{` — begin a group.
    BeginGroup,
    /// `}` — end a group.
    EndGroup,
    /// Enter math mode (`$`, `$$`, `\(`, `\[`). `display` distinguishes `$$`/`\[` (true)
    /// from `$`/`\(` (false).
    MathOn { display: bool },
    /// Leave math mode (`$`, `$$`, `\)`, `\]`).
    MathOff { display: bool },
    /// `&` — alignment tab (table/matrix column separator).
    AlignTab,
    /// `#` — a macro parameter marker (the following digit, if any, is a separate `Char`;
    /// the macro layer interprets the pairing).
    Parameter,
    /// `^` — superscript marker.
    Superscript,
    /// `_` — subscript marker.
    Subscript,
    /// An active character that behaves like a macro — `~` (a non-breaking space).
    Active(char),
    /// A run of inter-word whitespace, collapsed to one token (text mode only; in math
    /// mode whitespace is insignificant and emits nothing).
    Space,
    /// A paragraph break — a blank line (two or more newlines) in text mode (`\par`).
    Par,
    /// An ordinary character (a letter or "other": digit, punctuation, …).
    Char(char),
    /// A comment from `%` to end of line (text kept, without the `%` or the newline).
    Comment(String),
    /// Inline verbatim: `\verb<delim>…<delim>` (or `\verb*…` for the visible-space variant).
    /// The lexer reads the body **raw** — catcodes are suspended inside it, so `{ $ # \` etc.
    /// are literal characters. `delim` is the chosen delimiter (e.g. `|`), `star` is the `*`
    /// variant, and `content` is the raw inner text.
    Verb { star: bool, delim: char, content: String },
    /// A verbatim *environment* body: `\begin{verbatim}…\end{verbatim}` (or `verbatim*`). The
    /// lexer reads everything between the opening `}` and the matching `\end{<env>}` **raw**
    /// (catcodes suspended, newlines included). `env` is the environment name verbatim
    /// (`verbatim` / `verbatim*`), `content` the raw body.
    VerbatimEnv { env: String, content: String },
    /// End of input — always the final token.
    Eof,
}
