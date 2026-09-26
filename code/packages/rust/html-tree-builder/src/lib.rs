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
//! **Not written yet** (BR03 §5): parse errors by specification code.
//! [`parse_fragment`] is §13.4, what `innerHTML` does. The corpus cases that need them are
//! listed in `tests/fixtures/expected-failures.txt`.
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
    feed(source, &mut lexer, &mut builder)?;

    let lexer_diagnostics = lexer.diagnostics().to_vec();
    let document_mode = builder.document_mode();
    let (arena, mut tree_diagnostics) = builder.finish();
    let (document, flattened) = arena.to_document_capped();
    if flattened {
        tree_diagnostics.push(TreeDiagnostic {
            code: "tree-builder-depth-limit",
            position: lexer.position(),
        });
    }
    Ok(ParseOutput {
        document,
        document_mode,
        lexer_diagnostics,
        tree_diagnostics,
    })
}

/// The element a fragment is parsed inside (§13.4), e.g. `<td>` for a
/// table cell's `innerHTML`, or SVG `<desc>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentContext {
    pub namespace: arena::Namespace,
    /// The local name, lowercase for HTML.
    pub name: String,
    pub attributes: Vec<dom_core::Attribute>,
}

impl FragmentContext {
    /// An HTML context element with no attributes.
    pub fn html(name: &str) -> Self {
        Self {
            namespace: arena::Namespace::Html,
            name: name.to_ascii_lowercase(),
            attributes: Vec::new(),
        }
    }
}

/// The nodes of a parsed fragment plus its diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentOutput {
    pub nodes: Vec<dom_core::Node>,
    pub lexer_diagnostics: Vec<Diagnostic>,
    pub tree_diagnostics: Vec<TreeDiagnostic>,
}

/// Parse `source` as the children of `context` (§13.4, "parsing HTML
/// fragments"): what `innerHTML` and `insertAdjacentHTML` do.
pub fn parse_fragment(
    source: &str,
    context: &FragmentContext,
    options: TreeBuilderOptions,
) -> Result<FragmentOutput, ParseError> {
    let mut lexer = create_html_lexer_with_context(&fragment_lex_context(context, options.scripting))?;
    let mut builder = TreeBuilder::for_fragment(
        options.scripting,
        context.namespace,
        &context.name,
        context.attributes.clone(),
    );
    feed(source, &mut lexer, &mut builder)?;
    let lexer_diagnostics = lexer.diagnostics().to_vec();
    let root = builder.fragment_root();
    let (arena, tree_diagnostics) = builder.finish();
    Ok(FragmentOutput {
        nodes: root.map(|root| arena.to_nodes(root)).unwrap_or_default(),
        lexer_diagnostics,
        tree_diagnostics,
    })
}

/// The tokenizer state a fragment starts in, by its context (§13.4). There
/// is no "appropriate end tag" in the fragment case: `</textarea>` inside a
/// `textarea` fragment is text, so no last start tag is set.
fn fragment_lex_context(context: &FragmentContext, scripting: HtmlScriptingMode) -> HtmlLexContext {
    use coding_adventures_html_lexer::HtmlTokenizerState as State;
    if context.namespace != arena::Namespace::Html {
        return HtmlLexContext::data();
    }
    let state = match context.name.as_str() {
        "title" | "textarea" => State::Rcdata,
        "style" | "xmp" | "iframe" | "noembed" | "noframes" => State::Rawtext,
        "script" => State::ScriptData,
        "noscript" if scripting == HtmlScriptingMode::Enabled => State::Rawtext,
        "plaintext" => State::Plaintext,
        _ => State::Data,
    };
    HtmlLexContext::new(state)
}

/// Run the tokenizer over `source`, handing each token to the builder, then
/// end of file.
fn feed(source: &str, lexer: &mut HtmlLexer, builder: &mut TreeBuilder) -> Result<(), ParseError> {
    let mut cdata = CdataTally::default();
    let mut buffer = [0; 4];
    for character in source.chars() {
        lexer.push(character.encode_utf8(&mut buffer))?;
        drain(lexer, builder, &mut cdata)?;
    }
    lexer.finish()?;
    builder.input_finished();
    drain(lexer, builder, &mut cdata)?;
    builder.process(Token::Eof, lexer.position());
    Ok(())
}

/// How many `<![CDATA[` the lexer has turned into bogus comments so far, and
/// how many of those comments the builder has been handed.
/// Diagnostics are scanned once each, so the tally stays linear however many
/// comments look like CDATA.
#[derive(Default)]
struct CdataTally {
    scanned: usize,
    seen: usize,
    handed: usize,
}

fn drain(
    lexer: &mut HtmlLexer,
    builder: &mut TreeBuilder,
    cdata: &mut CdataTally,
) -> Result<(), ParseError> {
    for positioned in lexer.drain_positioned_tokens() {
        // The end of input is the driver's to report, once, after `finish`.
        if matches!(positioned.token, Token::Eof) {
            continue;
        }
        // `html-lexer` raises `cdata-in-html-content` on reading `<![CDATA[`
        // and nowhere else, then emits that section as one bogus comment before
        // any other token. So a comment is a CDATA section exactly when more
        // of those diagnostics exist than such comments were handed on.
        if let Token::Comment(data) = &positioned.token {
            if data.starts_with("[CDATA[") {
                let diagnostics = lexer.diagnostics();
                cdata.seen += diagnostics[cdata.scanned..]
                    .iter()
                    .filter(|diagnostic| diagnostic.code == "cdata-in-html-content")
                    .count();
                cdata.scanned = diagnostics.len();
                if cdata.seen > cdata.handed {
                    cdata.handed += 1;
                    builder.next_comment_is_cdata();
                }
            }
        }
        builder.process(positioned.token, positioned.position);
        if let Some(context) = builder.take_tokenizer_request() {
            apply_html_lex_context(lexer, &context)?;
        }
    }
    Ok(())
}
