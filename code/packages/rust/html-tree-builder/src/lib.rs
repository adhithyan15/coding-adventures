//! # html-tree-builder
//!
//! HTML tree construction written the way the WHATWG HTML specification
//! describes it (§13.2.4 to §13.2.6), one named type per specification
//! concept. This is BR03 (BR02 phase P2): it grows beside `html-parser`,
//! measured against the same corpus, until it is at least as correct and
//! `html-parser` delegates tree construction to it.
//!
//! ```text
//!   source text ──▶ html-lexer (tokenizer) ──tokens──▶ TreeBuilder ──▶ Arena ──▶ dom_core::Document
//!                          ▲                               │
//!                          └──── tokenizer state switch ───┘
//!                              (RCDATA after <title>, …)
//! ```
//!
//! | specification | here |
//! |---|---|
//! | §13.2.4.1 insertion mode | [`InsertionMode`] |
//! | §13.2.4.2 stack of open elements | [`open_elements::OpenElements`] |
//! | §13.2.4.3 list of active formatting elements | [`active_formatting::ActiveFormatting`] |
//! | §13.2.6 tree construction | [`tree_builder::TreeBuilder`], one method per mode |
//! | §13.2.6.4.7 adoption agency algorithm | `TreeBuilder::adoption_agency` |
//!
//! **Not written yet** (BR03 §5): the table insertion modes, foreign
//! content, fragment parsing, and parse errors by specification code. A token that reaches an unwritten mode is handled by the `InBody`
//! rules and reported as `tree-builder-mode-not-implemented`; the corpus cases
//! that need them are listed in `tests/fixtures/expected-failures.txt`.
//!
//! ```
//! use coding_adventures_html_tree_builder::{parse_document, html5lib, TreeBuilderOptions};
//!
//! let output = parse_document("<p>One<p>Two", TreeBuilderOptions::default()).unwrap();
//! assert_eq!(
//!     html5lib::document_lines(&output.document),
//!     ["| <html>", "|   <head>", "|   <body>", "|     <p>", "|       \"One\"", "|     <p>", "|       \"Two\""]
//! );
//! ```

pub mod active_formatting;
pub mod arena;
pub mod elements;
pub mod html5lib;
pub mod insertion_mode;
pub mod open_elements;
pub mod tree_builder;

pub use insertion_mode::InsertionMode;
pub use tree_builder::TreeDiagnostic;

use coding_adventures_html_lexer::{
    apply_html_lex_context, create_html_lexer_with_context, Diagnostic, HtmlLexContext, HtmlLexer,
    HtmlScriptingMode, Token, TokenizerError,
};
use dom_core::Document;
use elements::DocumentMode;
use tree_builder::TreeBuilder;

/// Options for one parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeBuilderOptions {
    /// The scripting flag (§13.2.4.5). Venture runs no scripts yet, so the
    /// default is `Disabled`, matching `html-parser` (BR02 P1).
    pub scripting: HtmlScriptingMode,
}

impl Default for TreeBuilderOptions {
    fn default() -> Self {
        Self {
            scripting: HtmlScriptingMode::Disabled,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOutput {
    pub document: Document,
    pub document_mode: DocumentMode,
    pub lexer_diagnostics: Vec<Diagnostic>,
    pub tree_diagnostics: Vec<TreeDiagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// The tokenizer's state machine failed. Malformed HTML never does this;
    /// it would be a defect in the tokenizer definition.
    Lexer(TokenizerError),
}

impl From<TokenizerError> for ParseError {
    fn from(error: TokenizerError) -> Self {
        ParseError::Lexer(error)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Lexer(error) => write!(formatter, "HTML tokenizer failed: {error:?}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse a complete document (§13.2).
///
/// The source is fed to the tokenizer one character at a time, and the
/// tokens it has produced are handed to the tree builder after each one, so
/// that a tokenizer state switch the tree builder asks for (after `<title>`,
/// `<script>`, `<plaintext>`…) takes effect before the next character is
/// read, as the specification requires.
pub fn parse_document(
    source: &str,
    options: TreeBuilderOptions,
) -> Result<ParseOutput, ParseError> {
    let mut lexer = create_html_lexer_with_context(&HtmlLexContext::data())?;
    let mut builder = TreeBuilder::new(options.scripting);

    let mut buffer = [0; 4];
    for character in source.chars() {
        lexer.push(character.encode_utf8(&mut buffer))?;
        drain(&mut lexer, &mut builder)?;
    }
    lexer.finish()?;
    drain(&mut lexer, &mut builder)?;
    builder.process(Token::Eof, lexer.position());

    let lexer_diagnostics = lexer.diagnostics().to_vec();
    let document_mode = builder.document_mode();
    let (arena, tree_diagnostics) = builder.finish();
    Ok(ParseOutput {
        document: arena.to_document(),
        document_mode,
        lexer_diagnostics,
        tree_diagnostics,
    })
}

fn drain(lexer: &mut HtmlLexer, builder: &mut TreeBuilder) -> Result<(), ParseError> {
    for positioned in lexer.drain_positioned_tokens() {
        // The end of input is the driver's to report, once, after `finish`.
        if matches!(positioned.token, Token::Eof) {
            continue;
        }
        builder.process(positioned.token, positioned.position);
        if let Some(context) = builder.take_tokenizer_request() {
            apply_html_lex_context(lexer, &context)?;
        }
    }
    Ok(())
}
