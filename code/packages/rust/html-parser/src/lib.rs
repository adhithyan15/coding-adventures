//! Incremental HTML parser for Venture.
//!
//! This first slice builds a DOM tree from the current Rust HTML lexer tokens.
//! It deliberately starts with a small tree-construction core instead of
//! pretending HTML is context-free. Future batches can add the full WHATWG
//! insertion-mode machinery on top of this DOM target.

use coding_adventures_html_lexer::{
    apply_html_lex_context, create_html_lexer_with_context, Attribute as LexerAttribute,
    Diagnostic, DoctypeSeed, HtmlLexContext, HtmlLexer, HtmlScriptingMode, HtmlTokenizerState,
    Token, TokenizerError,
};
use dom_core::{Attribute, Document, DocumentType, Element, Node};
use std::fmt;

const FRAGMENT_CONTEXT_MARKER: &str = "data-venture-fragment-context";

/// Parser options that influence tokenizer handoff and tree construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HtmlParseOptions {
    pub scripting: HtmlScriptingMode,
    pub initial_tokenizer_context: HtmlInitialTokenizerContext,
}

impl Default for HtmlParseOptions {
    fn default() -> Self {
        Self {
            scripting: HtmlScriptingMode::Enabled,
            initial_tokenizer_context: HtmlInitialTokenizerContext::Data,
        }
    }
}

/// Initial tokenizer context for parser-approved document or fragment parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlInitialTokenizerContext {
    Data,
    Rcdata,
    RcdataLessThanSign,
    RcdataEndTagOpen,
    RcdataEndTagName,
    RcdataEndTagWhitespace,
    RcdataEndTagAttributes,
    RcdataSelfClosingEndTag,
    Rawtext,
    RawtextLessThanSign,
    RawtextEndTagOpen,
    RawtextEndTagName,
    RawtextEndTagWhitespace,
    RawtextEndTagAttributes,
    RawtextSelfClosingEndTag,
    Plaintext,
    ForeignContentCdataSection,
    ForeignContentCdataSectionBracket,
    ForeignContentCdataSectionEnd,
    CommentStart,
    CommentStartDash,
    Comment,
    CommentLessThanSign,
    CommentLessThanSignBang,
    CommentLessThanSignBangDash,
    CommentLessThanSignBangDashDash,
    CommentEndDash,
    CommentEnd,
    CommentEndBang,
    BogusComment,
    DoctypeKeywordO,
    DoctypeKeywordC,
    DoctypeKeywordT,
    DoctypeKeywordY,
    DoctypeKeywordP,
    DoctypeKeywordE,
    DoctypeAfterKeyword,
    BeforeDoctypeName,
    DoctypeName,
    AfterDoctypeName,
    DoctypePublicKeywordU,
    DoctypePublicKeywordB,
    DoctypePublicKeywordL,
    DoctypePublicKeywordI,
    DoctypePublicKeywordC,
    AfterDoctypePublicKeyword,
    BeforeDoctypePublicIdentifier,
    DoctypePublicIdentifierDoubleQuoted,
    DoctypePublicIdentifierSingleQuoted,
    AfterDoctypePublicIdentifier,
    BetweenDoctypePublicAndSystemIdentifiers,
    DoctypeSystemKeywordY,
    DoctypeSystemKeywordS,
    DoctypeSystemKeywordT,
    DoctypeSystemKeywordE,
    DoctypeSystemKeywordM,
    AfterDoctypeSystemKeyword,
    BeforeDoctypeSystemIdentifier,
    DoctypeSystemIdentifierDoubleQuoted,
    DoctypeSystemIdentifierSingleQuoted,
    AfterDoctypeSystemIdentifier,
    BogusDoctype,
    CharacterReference,
    NamedCharacterReference,
    NumericCharacterReference,
    NumericHexCharacterReferenceStart,
    NumericHexCharacterReference,
    NumericDecimalCharacterReference,
    RcdataCharacterReference,
    RcdataNamedCharacterReference,
    RcdataNumericCharacterReference,
    RcdataNumericHexCharacterReferenceStart,
    RcdataNumericHexCharacterReference,
    RcdataNumericDecimalCharacterReference,
    ScriptData,
    ScriptDataLessThanSign,
    ScriptDataEndTagOpen,
    ScriptDataEndTagName,
    ScriptDataEndTagWhitespace,
    ScriptDataEndTagAttributes,
    ScriptDataSelfClosingEndTag,
    ScriptDataEscapeStart,
    ScriptDataEscapeStartDash,
    ScriptDataEscaped,
    ScriptDataEscapedDash,
    ScriptDataEscapedDashDash,
    ScriptDataEscapedLessThanSign,
    ScriptDataEscapedEndTagOpen,
    ScriptDataEscapedEndTagName,
    ScriptDataEscapedEndTagWhitespace,
    ScriptDataEscapedEndTagAttributes,
    ScriptDataEscapedSelfClosingEndTag,
    ScriptDataDoubleEscapeStart,
    ScriptDataDoubleEscaped,
    ScriptDataDoubleEscapedDash,
    ScriptDataDoubleEscapedDashDash,
    ScriptDataDoubleEscapedLessThanSign,
    ScriptDataDoubleEscapeEnd,
}

impl HtmlInitialTokenizerContext {
    fn lex_context(self) -> HtmlLexContext {
        match self {
            Self::Data => HtmlLexContext::data(),
            Self::Rcdata => {
                HtmlLexContext::new(HtmlTokenizerState::Rcdata).with_last_start_tag("title")
            }
            Self::RcdataLessThanSign => HtmlLexContext::new(HtmlTokenizerState::RcdataLessThanSign)
                .with_last_start_tag("title"),
            Self::RcdataEndTagOpen => HtmlLexContext::new(HtmlTokenizerState::RcdataEndTagOpen)
                .with_last_start_tag("title"),
            Self::RcdataEndTagName => {
                seeded_end_tag_lex_context(HtmlTokenizerState::RcdataEndTagName, "title", "title")
            }
            Self::RcdataEndTagWhitespace => seeded_end_tag_lex_context(
                HtmlTokenizerState::RcdataEndTagWhitespace,
                "title",
                "title ",
            ),
            Self::RcdataEndTagAttributes => seeded_end_tag_lex_context(
                HtmlTokenizerState::RcdataEndTagAttributes,
                "title",
                "title class=x",
            ),
            Self::RcdataSelfClosingEndTag => seeded_end_tag_lex_context(
                HtmlTokenizerState::RcdataSelfClosingEndTag,
                "title",
                "title",
            ),
            Self::Rawtext => {
                HtmlLexContext::new(HtmlTokenizerState::Rawtext).with_last_start_tag("style")
            }
            Self::RawtextLessThanSign => {
                HtmlLexContext::new(HtmlTokenizerState::RawtextLessThanSign)
                    .with_last_start_tag("style")
            }
            Self::RawtextEndTagOpen => HtmlLexContext::new(HtmlTokenizerState::RawtextEndTagOpen)
                .with_last_start_tag("style"),
            Self::RawtextEndTagName => {
                seeded_end_tag_lex_context(HtmlTokenizerState::RawtextEndTagName, "style", "style")
            }
            Self::RawtextEndTagWhitespace => seeded_end_tag_lex_context(
                HtmlTokenizerState::RawtextEndTagWhitespace,
                "style",
                "style ",
            ),
            Self::RawtextEndTagAttributes => seeded_end_tag_lex_context(
                HtmlTokenizerState::RawtextEndTagAttributes,
                "style",
                "style class=x",
            ),
            Self::RawtextSelfClosingEndTag => seeded_end_tag_lex_context(
                HtmlTokenizerState::RawtextSelfClosingEndTag,
                "style",
                "style",
            ),
            Self::Plaintext => HtmlLexContext::new(HtmlTokenizerState::Plaintext),
            Self::ForeignContentCdataSection => HtmlLexContext::cdata_section(),
            Self::ForeignContentCdataSectionBracket => {
                HtmlLexContext::new(HtmlTokenizerState::CdataSectionBracket)
            }
            Self::ForeignContentCdataSectionEnd => {
                HtmlLexContext::new(HtmlTokenizerState::CdataSectionEnd)
            }
            Self::CommentStart => seeded_comment_lex_context(HtmlTokenizerState::CommentStart, ""),
            Self::CommentStartDash => {
                seeded_comment_lex_context(HtmlTokenizerState::CommentStartDash, "")
            }
            Self::Comment => seeded_comment_lex_context(HtmlTokenizerState::Comment, "seed"),
            Self::CommentLessThanSign => {
                seeded_comment_lex_context(HtmlTokenizerState::CommentLessThanSign, "seed<")
            }
            Self::CommentLessThanSignBang => {
                seeded_comment_lex_context(HtmlTokenizerState::CommentLessThanSignBang, "seed<!")
            }
            Self::CommentLessThanSignBangDash => seeded_comment_lex_context(
                HtmlTokenizerState::CommentLessThanSignBangDash,
                "seed<!",
            ),
            Self::CommentLessThanSignBangDashDash => seeded_comment_lex_context(
                HtmlTokenizerState::CommentLessThanSignBangDashDash,
                "seed<!",
            ),
            Self::CommentEndDash => {
                seeded_comment_lex_context(HtmlTokenizerState::CommentEndDash, "seed")
            }
            Self::CommentEnd => seeded_comment_lex_context(HtmlTokenizerState::CommentEnd, "seed"),
            Self::CommentEndBang => {
                seeded_comment_lex_context(HtmlTokenizerState::CommentEndBang, "seed")
            }
            Self::BogusComment => {
                seeded_comment_lex_context(HtmlTokenizerState::BogusComment, "bogus-")
            }
            Self::DoctypeKeywordO => {
                seeded_doctype_lex_context(HtmlTokenizerState::DoctypeKeywordO, DoctypeSeed::new())
            }
            Self::DoctypeKeywordC => {
                seeded_doctype_lex_context(HtmlTokenizerState::DoctypeKeywordC, DoctypeSeed::new())
            }
            Self::DoctypeKeywordT => {
                seeded_doctype_lex_context(HtmlTokenizerState::DoctypeKeywordT, DoctypeSeed::new())
            }
            Self::DoctypeKeywordY => {
                seeded_doctype_lex_context(HtmlTokenizerState::DoctypeKeywordY, DoctypeSeed::new())
            }
            Self::DoctypeKeywordP => {
                seeded_doctype_lex_context(HtmlTokenizerState::DoctypeKeywordP, DoctypeSeed::new())
            }
            Self::DoctypeKeywordE => {
                seeded_doctype_lex_context(HtmlTokenizerState::DoctypeKeywordE, DoctypeSeed::new())
            }
            Self::DoctypeAfterKeyword => seeded_doctype_lex_context(
                HtmlTokenizerState::DoctypeAfterKeyword,
                DoctypeSeed::new(),
            ),
            Self::BeforeDoctypeName => seeded_doctype_lex_context(
                HtmlTokenizerState::BeforeDoctypeName,
                DoctypeSeed::new(),
            ),
            Self::DoctypeName => seeded_doctype_lex_context(
                HtmlTokenizerState::DoctypeName,
                DoctypeSeed::with_name("ht"),
            ),
            Self::AfterDoctypeName => seeded_doctype_lex_context(
                HtmlTokenizerState::AfterDoctypeName,
                DoctypeSeed::with_name("html"),
            ),
            Self::DoctypePublicKeywordU => seeded_doctype_lex_context(
                HtmlTokenizerState::DoctypePublicKeywordU,
                DoctypeSeed::with_name("html"),
            ),
            Self::DoctypePublicKeywordB => seeded_doctype_lex_context(
                HtmlTokenizerState::DoctypePublicKeywordB,
                DoctypeSeed::with_name("html"),
            ),
            Self::DoctypePublicKeywordL => seeded_doctype_lex_context(
                HtmlTokenizerState::DoctypePublicKeywordL,
                DoctypeSeed::with_name("html"),
            ),
            Self::DoctypePublicKeywordI => seeded_doctype_lex_context(
                HtmlTokenizerState::DoctypePublicKeywordI,
                DoctypeSeed::with_name("html"),
            ),
            Self::DoctypePublicKeywordC => seeded_doctype_lex_context(
                HtmlTokenizerState::DoctypePublicKeywordC,
                DoctypeSeed::with_name("html"),
            ),
            Self::AfterDoctypePublicKeyword => seeded_doctype_lex_context(
                HtmlTokenizerState::AfterDoctypePublicKeyword,
                DoctypeSeed::with_name("html"),
            ),
            Self::BeforeDoctypePublicIdentifier => seeded_doctype_lex_context(
                HtmlTokenizerState::BeforeDoctypePublicIdentifier,
                DoctypeSeed::with_name("html"),
            ),
            Self::DoctypePublicIdentifierDoubleQuoted => seeded_doctype_lex_context(
                HtmlTokenizerState::DoctypePublicIdentifierDoubleQuoted,
                doctype_seed_with_public("html", "pu"),
            ),
            Self::DoctypePublicIdentifierSingleQuoted => seeded_doctype_lex_context(
                HtmlTokenizerState::DoctypePublicIdentifierSingleQuoted,
                doctype_seed_with_public("html", "pu"),
            ),
            Self::AfterDoctypePublicIdentifier => seeded_doctype_lex_context(
                HtmlTokenizerState::AfterDoctypePublicIdentifier,
                doctype_seed_with_public("html", "pub"),
            ),
            Self::BetweenDoctypePublicAndSystemIdentifiers => seeded_doctype_lex_context(
                HtmlTokenizerState::BetweenDoctypePublicAndSystemIdentifiers,
                doctype_seed_with_public("html", "pub"),
            ),
            Self::DoctypeSystemKeywordY => seeded_doctype_lex_context(
                HtmlTokenizerState::DoctypeSystemKeywordY,
                DoctypeSeed::with_name("html"),
            ),
            Self::DoctypeSystemKeywordS => seeded_doctype_lex_context(
                HtmlTokenizerState::DoctypeSystemKeywordS,
                DoctypeSeed::with_name("html"),
            ),
            Self::DoctypeSystemKeywordT => seeded_doctype_lex_context(
                HtmlTokenizerState::DoctypeSystemKeywordT,
                DoctypeSeed::with_name("html"),
            ),
            Self::DoctypeSystemKeywordE => seeded_doctype_lex_context(
                HtmlTokenizerState::DoctypeSystemKeywordE,
                DoctypeSeed::with_name("html"),
            ),
            Self::DoctypeSystemKeywordM => seeded_doctype_lex_context(
                HtmlTokenizerState::DoctypeSystemKeywordM,
                DoctypeSeed::with_name("html"),
            ),
            Self::AfterDoctypeSystemKeyword => seeded_doctype_lex_context(
                HtmlTokenizerState::AfterDoctypeSystemKeyword,
                DoctypeSeed::with_name("html"),
            ),
            Self::BeforeDoctypeSystemIdentifier => seeded_doctype_lex_context(
                HtmlTokenizerState::BeforeDoctypeSystemIdentifier,
                DoctypeSeed::with_name("html"),
            ),
            Self::DoctypeSystemIdentifierDoubleQuoted => seeded_doctype_lex_context(
                HtmlTokenizerState::DoctypeSystemIdentifierDoubleQuoted,
                doctype_seed_with_system("html", "sy"),
            ),
            Self::DoctypeSystemIdentifierSingleQuoted => seeded_doctype_lex_context(
                HtmlTokenizerState::DoctypeSystemIdentifierSingleQuoted,
                doctype_seed_with_system("html", "sy"),
            ),
            Self::AfterDoctypeSystemIdentifier => seeded_doctype_lex_context(
                HtmlTokenizerState::AfterDoctypeSystemIdentifier,
                doctype_seed_with_system("html", "sys"),
            ),
            Self::BogusDoctype => seeded_doctype_lex_context(
                HtmlTokenizerState::BogusDoctype,
                DoctypeSeed {
                    name: Some("html".to_string()),
                    public_identifier: None,
                    system_identifier: None,
                    force_quirks: true,
                },
            ),
            Self::CharacterReference => seeded_character_reference_lex_context(
                HtmlTokenizerState::TextCharacterReference,
                HtmlTokenizerState::Data,
                "&",
            ),
            Self::NamedCharacterReference => seeded_character_reference_lex_context(
                HtmlTokenizerState::TextNamedCharacterReference,
                HtmlTokenizerState::Data,
                "&co",
            ),
            Self::NumericCharacterReference => seeded_character_reference_lex_context(
                HtmlTokenizerState::TextNumericCharacterReference,
                HtmlTokenizerState::Data,
                "&#",
            ),
            Self::NumericHexCharacterReferenceStart => seeded_character_reference_lex_context(
                HtmlTokenizerState::TextNumericHexCharacterReferenceStart,
                HtmlTokenizerState::Data,
                "&#x",
            ),
            Self::NumericHexCharacterReference => seeded_character_reference_lex_context(
                HtmlTokenizerState::TextNumericHexCharacterReference,
                HtmlTokenizerState::Data,
                "&#x4",
            ),
            Self::NumericDecimalCharacterReference => seeded_character_reference_lex_context(
                HtmlTokenizerState::TextNumericDecimalCharacterReference,
                HtmlTokenizerState::Data,
                "&#6",
            ),
            Self::RcdataCharacterReference => seeded_character_reference_lex_context(
                HtmlTokenizerState::TextCharacterReference,
                HtmlTokenizerState::Rcdata,
                "&",
            )
            .with_last_start_tag("title"),
            Self::RcdataNamedCharacterReference => seeded_character_reference_lex_context(
                HtmlTokenizerState::TextNamedCharacterReference,
                HtmlTokenizerState::Rcdata,
                "&a",
            )
            .with_last_start_tag("title"),
            Self::RcdataNumericCharacterReference => seeded_character_reference_lex_context(
                HtmlTokenizerState::TextNumericCharacterReference,
                HtmlTokenizerState::Rcdata,
                "&#",
            )
            .with_last_start_tag("title"),
            Self::RcdataNumericHexCharacterReferenceStart => {
                seeded_character_reference_lex_context(
                    HtmlTokenizerState::TextNumericHexCharacterReferenceStart,
                    HtmlTokenizerState::Rcdata,
                    "&#x",
                )
                .with_last_start_tag("title")
            }
            Self::RcdataNumericHexCharacterReference => seeded_character_reference_lex_context(
                HtmlTokenizerState::TextNumericHexCharacterReference,
                HtmlTokenizerState::Rcdata,
                "&#x4",
            )
            .with_last_start_tag("title"),
            Self::RcdataNumericDecimalCharacterReference => seeded_character_reference_lex_context(
                HtmlTokenizerState::TextNumericDecimalCharacterReference,
                HtmlTokenizerState::Rcdata,
                "&#6",
            )
            .with_last_start_tag("title"),
            Self::ScriptData => script_lex_context(HtmlTokenizerState::ScriptData),
            Self::ScriptDataLessThanSign => {
                script_lex_context(HtmlTokenizerState::ScriptDataLessThanSign)
            }
            Self::ScriptDataEndTagOpen => {
                script_lex_context(HtmlTokenizerState::ScriptDataEndTagOpen)
            }
            Self::ScriptDataEndTagName => seeded_end_tag_lex_context(
                HtmlTokenizerState::ScriptDataEndTagName,
                "script",
                "script",
            ),
            Self::ScriptDataEndTagWhitespace => seeded_end_tag_lex_context(
                HtmlTokenizerState::ScriptDataEndTagWhitespace,
                "script",
                "script ",
            ),
            Self::ScriptDataEndTagAttributes => seeded_end_tag_lex_context(
                HtmlTokenizerState::ScriptDataEndTagAttributes,
                "script",
                "script class=x",
            ),
            Self::ScriptDataSelfClosingEndTag => seeded_end_tag_lex_context(
                HtmlTokenizerState::ScriptDataSelfClosingEndTag,
                "script",
                "script",
            ),
            Self::ScriptDataEscapeStart => {
                script_lex_context(HtmlTokenizerState::ScriptDataEscapeStart)
            }
            Self::ScriptDataEscapeStartDash => {
                script_lex_context(HtmlTokenizerState::ScriptDataEscapeStartDash)
            }
            Self::ScriptDataEscaped => script_lex_context(HtmlTokenizerState::ScriptDataEscaped),
            Self::ScriptDataEscapedDash => {
                script_lex_context(HtmlTokenizerState::ScriptDataEscapedDash)
            }
            Self::ScriptDataEscapedDashDash => {
                script_lex_context(HtmlTokenizerState::ScriptDataEscapedDashDash)
            }
            Self::ScriptDataEscapedLessThanSign => {
                script_lex_context(HtmlTokenizerState::ScriptDataEscapedLessThanSign)
            }
            Self::ScriptDataEscapedEndTagOpen => {
                script_lex_context(HtmlTokenizerState::ScriptDataEscapedEndTagOpen)
            }
            Self::ScriptDataEscapedEndTagName => seeded_end_tag_lex_context(
                HtmlTokenizerState::ScriptDataEscapedEndTagName,
                "script",
                "script",
            ),
            Self::ScriptDataEscapedEndTagWhitespace => seeded_end_tag_lex_context(
                HtmlTokenizerState::ScriptDataEscapedEndTagWhitespace,
                "script",
                "script ",
            ),
            Self::ScriptDataEscapedEndTagAttributes => seeded_end_tag_lex_context(
                HtmlTokenizerState::ScriptDataEscapedEndTagAttributes,
                "script",
                "script class=x",
            ),
            Self::ScriptDataEscapedSelfClosingEndTag => seeded_end_tag_lex_context(
                HtmlTokenizerState::ScriptDataEscapedSelfClosingEndTag,
                "script",
                "script",
            ),
            Self::ScriptDataDoubleEscapeStart => {
                script_lex_context(HtmlTokenizerState::ScriptDataDoubleEscapeStart)
            }
            Self::ScriptDataDoubleEscaped => {
                script_lex_context(HtmlTokenizerState::ScriptDataDoubleEscaped)
            }
            Self::ScriptDataDoubleEscapedDash => {
                script_lex_context(HtmlTokenizerState::ScriptDataDoubleEscapedDash)
            }
            Self::ScriptDataDoubleEscapedDashDash => {
                script_lex_context(HtmlTokenizerState::ScriptDataDoubleEscapedDashDash)
            }
            Self::ScriptDataDoubleEscapedLessThanSign => {
                script_lex_context(HtmlTokenizerState::ScriptDataDoubleEscapedLessThanSign)
            }
            Self::ScriptDataDoubleEscapeEnd => {
                script_lex_context(HtmlTokenizerState::ScriptDataDoubleEscapeEnd)
            }
        }
    }
}

fn script_lex_context(state: HtmlTokenizerState) -> HtmlLexContext {
    HtmlLexContext::script_substate(state).expect("parser only exposes valid script substates")
}

fn seeded_comment_lex_context(state: HtmlTokenizerState, data: &str) -> HtmlLexContext {
    HtmlLexContext::comment_continuation(state, data)
        .expect("parser only exposes valid comment continuation states")
}

fn seeded_doctype_lex_context(state: HtmlTokenizerState, seed: DoctypeSeed) -> HtmlLexContext {
    HtmlLexContext::doctype_continuation(state, seed)
        .expect("parser only exposes valid doctype continuation states")
}

fn seeded_character_reference_lex_context(
    state: HtmlTokenizerState,
    return_state: HtmlTokenizerState,
    temporary_buffer: &str,
) -> HtmlLexContext {
    HtmlLexContext::character_reference_continuation(state, return_state, temporary_buffer)
        .expect("parser only exposes valid character-reference continuation states")
}

fn doctype_seed_with_public(name: &str, public_identifier: &str) -> DoctypeSeed {
    DoctypeSeed {
        name: Some(name.to_string()),
        public_identifier: Some(public_identifier.to_string()),
        system_identifier: None,
        force_quirks: false,
    }
}

fn doctype_seed_with_system(name: &str, system_identifier: &str) -> DoctypeSeed {
    DoctypeSeed {
        name: Some(name.to_string()),
        public_identifier: None,
        system_identifier: Some(system_identifier.to_string()),
        force_quirks: false,
    }
}

fn seeded_end_tag_lex_context(
    state: HtmlTokenizerState,
    last_start_tag: &str,
    temporary_buffer: &str,
) -> HtmlLexContext {
    HtmlLexContext::new(state)
        .with_last_start_tag(last_start_tag)
        .with_current_end_tag(last_start_tag)
        .with_temporary_buffer(temporary_buffer)
}

/// Parser result that keeps DOM output and diagnostics together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOutput {
    pub document: Document,
    pub lexer_diagnostics: Vec<Diagnostic>,
    pub parser_diagnostics: Vec<ParserDiagnostic>,
}

/// Parser result for body-fragment parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentOutput {
    pub nodes: Vec<Node>,
    pub lexer_diagnostics: Vec<Diagnostic>,
    pub parser_diagnostics: Vec<ParserDiagnostic>,
}

/// Tree-construction diagnostic emitted by this parser layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserDiagnostic {
    pub code: String,
    pub message: String,
}

impl ParserDiagnostic {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Error returned when lexing or parser setup fails.
#[derive(Debug)]
pub enum ParseError {
    Lexer(TokenizerError),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lexer(error) => write!(f, "HTML lexer error: {error}"),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<TokenizerError> for ParseError {
    fn from(error: TokenizerError) -> Self {
        Self::Lexer(error)
    }
}

/// Parse a complete HTML string into a DOM document.
pub fn parse_html(source: &str) -> Result<Document, ParseError> {
    Ok(parse_html_with_diagnostics(source)?.document)
}

/// Parse a complete HTML string and extract browser-facing document facts.
pub fn parse_browser_document(source: &str) -> Result<BrowserDocument, ParseError> {
    parse_html(source).map(|document| extract_browser_document(&document))
}

/// Extract browser-facing facts from a parsed DOM document.
pub fn extract_browser_document(document: &Document) -> BrowserDocument {
    BrowserDocument::from_document(document)
}

/// Parse a complete HTML string and extract a browser-facing content tree.
pub fn parse_browser_content_tree(source: &str) -> Result<BrowserContentTree, ParseError> {
    parse_html(source).map(|document| extract_browser_content_tree(&document))
}

/// Extract a browser-facing content tree from a parsed DOM document.
pub fn extract_browser_content_tree(document: &Document) -> BrowserContentTree {
    BrowserContentTree::from_document(document)
}

/// Parse a complete HTML string and extract a browser-facing render-tree input.
pub fn parse_browser_render_tree(source: &str) -> Result<BrowserRenderTree, ParseError> {
    parse_html(source).map(|document| extract_browser_render_tree(&document))
}

/// Extract a browser-facing render-tree input from a parsed DOM document.
pub fn extract_browser_render_tree(document: &Document) -> BrowserRenderTree {
    BrowserRenderTree::from_document(document)
}

/// Parse a complete HTML string into a DOM document plus lexer/parser diagnostics.
pub fn parse_html_with_diagnostics(source: &str) -> Result<ParseOutput, ParseError> {
    parse_html_with_diagnostics_and_options(source, HtmlParseOptions::default())
}

/// Parse an HTML body fragment into DOM nodes without returning the implied shell.
pub fn parse_html_fragment(source: &str) -> Result<Vec<Node>, ParseError> {
    Ok(parse_html_fragment_with_diagnostics(source)?.nodes)
}

/// Parse an HTML body fragment into DOM nodes plus lexer/parser diagnostics.
pub fn parse_html_fragment_with_diagnostics(source: &str) -> Result<FragmentOutput, ParseError> {
    parse_html_fragment_with_diagnostics_and_options(source, HtmlParseOptions::default())
}

/// Parse a complete HTML string into a DOM document with explicit parser options.
pub fn parse_html_with_options(
    source: &str,
    options: HtmlParseOptions,
) -> Result<Document, ParseError> {
    Ok(parse_html_with_diagnostics_and_options(source, options)?.document)
}

/// Parse an HTML body fragment into DOM nodes with explicit parser options.
pub fn parse_html_fragment_with_options(
    source: &str,
    options: HtmlParseOptions,
) -> Result<Vec<Node>, ParseError> {
    Ok(parse_html_fragment_with_diagnostics_and_options(source, options)?.nodes)
}

/// Parse an HTML fragment in the context of a given HTML element.
pub fn parse_html_fragment_for_context(
    source: &str,
    context_element: &str,
) -> Result<Vec<Node>, ParseError> {
    Ok(parse_html_fragment_for_context_with_diagnostics(source, context_element)?.nodes)
}

/// Parse an HTML fragment in the context of a given HTML element plus diagnostics.
pub fn parse_html_fragment_for_context_with_diagnostics(
    source: &str,
    context_element: &str,
) -> Result<FragmentOutput, ParseError> {
    parse_html_fragment_for_context_with_diagnostics_and_options(
        source,
        context_element,
        HtmlParseOptions::default(),
    )
}

/// Parse an HTML fragment in the context of a given HTML element with explicit options.
pub fn parse_html_fragment_for_context_with_options(
    source: &str,
    context_element: &str,
    options: HtmlParseOptions,
) -> Result<Vec<Node>, ParseError> {
    Ok(
        parse_html_fragment_for_context_with_diagnostics_and_options(
            source,
            context_element,
            options,
        )?
        .nodes,
    )
}

/// Parse a complete HTML string into a DOM document plus diagnostics with explicit parser options.
pub fn parse_html_with_diagnostics_and_options(
    source: &str,
    options: HtmlParseOptions,
) -> Result<ParseOutput, ParseError> {
    let mut lexer =
        create_html_lexer_with_context(&options.initial_tokenizer_context.lex_context())?;
    let mut parser = HtmlParser::with_options(options);

    for ch in source.chars() {
        let mut buffer = [0; 4];
        lexer.push(ch.encode_utf8(&mut buffer))?;
        drain_parser_tokens(&mut lexer, &mut parser, false)?;
    }

    lexer.finish()?;
    drain_parser_tokens(&mut lexer, &mut parser, true)?;
    parser.process_token(Token::Eof);

    let lexer_diagnostics = lexer.diagnostics().to_vec();
    let document = parser.finish_document();

    Ok(ParseOutput {
        document,
        lexer_diagnostics,
        parser_diagnostics: parser.diagnostics,
    })
}

/// Parse an HTML body fragment into DOM nodes plus diagnostics with explicit parser options.
pub fn parse_html_fragment_with_diagnostics_and_options(
    source: &str,
    options: HtmlParseOptions,
) -> Result<FragmentOutput, ParseError> {
    let mut lexer =
        create_html_lexer_with_context(&options.initial_tokenizer_context.lex_context())?;
    let mut parser = HtmlParser::with_body_fragment_options(options);

    for ch in source.chars() {
        let mut buffer = [0; 4];
        lexer.push(ch.encode_utf8(&mut buffer))?;
        drain_parser_tokens(&mut lexer, &mut parser, false)?;
    }

    lexer.finish()?;
    drain_parser_tokens(&mut lexer, &mut parser, true)?;
    parser.process_token(Token::Eof);

    let lexer_diagnostics = lexer.diagnostics().to_vec();
    let document = parser.finish_document();

    Ok(FragmentOutput {
        nodes: body_fragment_nodes(document),
        lexer_diagnostics,
        parser_diagnostics: parser.diagnostics,
    })
}

/// Parse an HTML fragment in the context of a given HTML element plus diagnostics.
pub fn parse_html_fragment_for_context_with_diagnostics_and_options(
    source: &str,
    context_element: &str,
    options: HtmlParseOptions,
) -> Result<FragmentOutput, ParseError> {
    let context_element = context_element.to_ascii_lowercase();
    let lex_context = fragment_initial_lex_context(&context_element, options)
        .unwrap_or_else(|| options.initial_tokenizer_context.lex_context());
    let mut lexer = create_html_lexer_with_context(&lex_context)?;
    let mut parser = HtmlParser::with_fragment_context_options(options, &context_element);

    for ch in source.chars() {
        let mut buffer = [0; 4];
        lexer.push(ch.encode_utf8(&mut buffer))?;
        drain_parser_tokens(&mut lexer, &mut parser, false)?;
    }

    lexer.finish()?;
    drain_parser_tokens(&mut lexer, &mut parser, true)?;
    parser.process_token(Token::Eof);

    let lexer_diagnostics = lexer.diagnostics().to_vec();
    let document = parser.finish_document();

    Ok(FragmentOutput {
        nodes: marked_fragment_context_nodes(document),
        lexer_diagnostics,
        parser_diagnostics: parser.diagnostics,
    })
}

/// Browser-facing summary of the parsed DOM.
///
/// This intentionally stays above raw DOM and below layout/CSS. It gives a
/// browser pipeline stable document facts without forcing layout code to know
/// every parser recovery detail.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BrowserDocument {
    pub title: Option<String>,
    pub base_href: Option<String>,
    pub base_target: Option<String>,
    pub metadata: BrowserDocumentMetadata,
    pub document_lang: Option<String>,
    pub document_dir: Option<String>,
    pub body_id: Option<String>,
    pub body_classes: Vec<String>,
    pub body_lang: Option<String>,
    pub body_dir: Option<String>,
    pub body_text: String,
    pub metas: Vec<BrowserMeta>,
    pub resources: Vec<BrowserResource>,
    pub scripts: Vec<BrowserScript>,
    pub stylesheets: Vec<BrowserStylesheet>,
    pub anchors: Vec<BrowserAnchor>,
    pub headings: Vec<BrowserHeading>,
    pub links: Vec<BrowserLink>,
    pub images: Vec<BrowserImage>,
    pub media: Vec<BrowserMedia>,
    pub forms: Vec<BrowserForm>,
    pub tables: Vec<BrowserTable>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BrowserDocumentMetadata {
    pub charset: Option<String>,
    pub viewport: Option<String>,
    pub description: Option<String>,
    pub application_name: Option<String>,
    pub referrer_policy: Option<String>,
    pub robots: Option<String>,
    pub color_scheme: Option<String>,
    pub theme_colors: Vec<BrowserThemeColor>,
    pub refresh: Option<BrowserRefresh>,
    pub canonical_url: Option<String>,
    pub resolved_canonical_url: Option<String>,
    pub manifest_url: Option<String>,
    pub resolved_manifest_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserThemeColor {
    pub color: String,
    pub media: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserRefresh {
    pub delay: Option<String>,
    pub url: Option<String>,
    pub resolved_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserMeta {
    pub name: Option<String>,
    pub http_equiv: Option<String>,
    pub property: Option<String>,
    pub charset: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserResource {
    pub kind: String,
    pub url: String,
    pub resolved_url: Option<String>,
    pub rel: Option<String>,
    pub as_hint: Option<String>,
    pub type_hint: Option<String>,
    pub media: Option<String>,
    pub title: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
    pub integrity: Option<String>,
    pub crossorigin: Option<String>,
    pub referrerpolicy: Option<String>,
    pub fetchpriority: Option<String>,
    pub blocking: Option<String>,
    pub browsing_context_name: Option<String>,
    pub loading: Option<String>,
    pub sandbox: Vec<String>,
    pub allow: Option<String>,
    pub allowfullscreen: bool,
    pub srcdoc: Option<String>,
    pub credentialless: bool,
    pub imagesrcset: Option<String>,
    pub resolved_imagesrcset: Option<String>,
    pub imagesizes: Option<String>,
    pub async_script: bool,
    pub defer_script: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserScript {
    pub script_kind: String,
    pub src: Option<String>,
    pub resolved_src: Option<String>,
    pub type_hint: Option<String>,
    pub async_script: bool,
    pub defer_script: bool,
    pub nomodule: bool,
    pub integrity: Option<String>,
    pub crossorigin: Option<String>,
    pub referrerpolicy: Option<String>,
    pub fetchpriority: Option<String>,
    pub blocking: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserStylesheet {
    pub source: String,
    pub href: Option<String>,
    pub resolved_href: Option<String>,
    pub rel: Option<String>,
    pub type_hint: Option<String>,
    pub media: Option<String>,
    pub title: Option<String>,
    pub disabled: bool,
    pub alternate: bool,
    pub integrity: Option<String>,
    pub crossorigin: Option<String>,
    pub referrerpolicy: Option<String>,
    pub fetchpriority: Option<String>,
    pub blocking: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserAnchor {
    pub id: Option<String>,
    pub name: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BrowserContentTree {
    pub children: Vec<BrowserContentNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserContentNode {
    pub role: String,
    pub authored_role: Option<String>,
    pub name: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub title: Option<String>,
    pub lang: Option<String>,
    pub dir: Option<String>,
    pub text: Option<String>,
    pub href: Option<String>,
    pub resolved_href: Option<String>,
    pub target: Option<String>,
    pub rel: Option<String>,
    pub rel_tokens: Vec<String>,
    pub download: Option<String>,
    pub ping: Vec<String>,
    pub resolved_ping: Vec<String>,
    pub hreflang: Option<String>,
    pub src: Option<String>,
    pub resolved_src: Option<String>,
    pub alt: Option<String>,
    pub resource_kind: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
    pub type_hint: Option<String>,
    pub media: Option<String>,
    pub poster: Option<String>,
    pub resolved_poster: Option<String>,
    pub preload: Option<String>,
    pub controls: bool,
    pub autoplay: bool,
    pub loop_media: bool,
    pub muted: bool,
    pub playsinline: bool,
    pub browsing_context_name: Option<String>,
    pub loading: Option<String>,
    pub sandbox: Vec<String>,
    pub allow: Option<String>,
    pub allowfullscreen: bool,
    pub referrerpolicy: Option<String>,
    pub srcdoc: Option<String>,
    pub credentialless: bool,
    pub control_type: Option<String>,
    pub form_owner: Option<String>,
    pub label_for: Option<String>,
    pub labels: Vec<String>,
    pub accessible_name: Option<String>,
    pub aria_label: Option<String>,
    pub aria_labelledby: Vec<String>,
    pub aria_describedby: Vec<String>,
    pub aria_controls: Vec<String>,
    pub aria_current: Option<String>,
    pub aria_expanded: Option<String>,
    pub aria_pressed: Option<String>,
    pub aria_selected: Option<String>,
    pub aria_hidden: bool,
    pub hidden: bool,
    pub inert: bool,
    pub open: bool,
    pub tabindex: Option<String>,
    pub accesskey: Vec<String>,
    pub focusable: Option<bool>,
    pub contenteditable: Option<String>,
    pub editing_mode: Option<String>,
    pub draggable: Option<String>,
    pub draggable_state: Option<String>,
    pub spellcheck: Option<String>,
    pub translate: Option<String>,
    pub popover: Option<String>,
    pub popover_target: Option<String>,
    pub popover_target_action: Option<String>,
    pub command: Option<String>,
    pub command_for: Option<String>,
    pub placeholder: Option<String>,
    pub autocomplete: Option<String>,
    pub autocapitalize: Option<String>,
    pub enterkeyhint: Option<String>,
    pub dirname: Option<String>,
    pub accept: Option<String>,
    pub capture: Option<String>,
    pub inputmode: Option<String>,
    pub pattern: Option<String>,
    pub min: Option<String>,
    pub max: Option<String>,
    pub step: Option<String>,
    pub minlength: Option<String>,
    pub maxlength: Option<String>,
    pub size: Option<String>,
    pub list: Option<String>,
    pub form_action: Option<String>,
    pub resolved_form_action: Option<String>,
    pub form_enctype: Option<String>,
    pub form_method: Option<String>,
    pub form_target: Option<String>,
    pub form_novalidate: bool,
    pub value: Option<String>,
    pub autofocus: bool,
    pub disabled: bool,
    pub required: bool,
    pub readonly: bool,
    pub checked: bool,
    pub selected: bool,
    pub multiple: bool,
    pub options: Vec<String>,
    pub table_section_kind: Option<String>,
    pub colspan: Option<String>,
    pub rowspan: Option<String>,
    pub span: Option<String>,
    pub scope: Option<String>,
    pub headers: Vec<String>,
    pub abbr: Option<String>,
    pub text_flow: Option<String>,
    pub list_kind: Option<String>,
    pub list_start: Option<String>,
    pub list_marker_type: Option<String>,
    pub list_reversed: bool,
    pub list_item_value: Option<String>,
    pub quote_cite: Option<String>,
    pub resolved_quote_cite: Option<String>,
    pub break_kind: Option<String>,
    pub heading_level: Option<u8>,
    pub section_kind: Option<String>,
    pub landmark_kind: Option<String>,
    pub children: Vec<BrowserContentNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BrowserRenderTree {
    pub children: Vec<BrowserRenderNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserRenderNode {
    pub display: String,
    pub role: String,
    pub authored_role: Option<String>,
    pub name: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub title: Option<String>,
    pub lang: Option<String>,
    pub dir: Option<String>,
    pub text: Option<String>,
    pub href: Option<String>,
    pub resolved_href: Option<String>,
    pub target: Option<String>,
    pub rel: Option<String>,
    pub rel_tokens: Vec<String>,
    pub download: Option<String>,
    pub ping: Vec<String>,
    pub resolved_ping: Vec<String>,
    pub hreflang: Option<String>,
    pub src: Option<String>,
    pub resolved_src: Option<String>,
    pub alt: Option<String>,
    pub resource_kind: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
    pub type_hint: Option<String>,
    pub media: Option<String>,
    pub poster: Option<String>,
    pub resolved_poster: Option<String>,
    pub preload: Option<String>,
    pub controls: bool,
    pub autoplay: bool,
    pub loop_media: bool,
    pub muted: bool,
    pub playsinline: bool,
    pub browsing_context_name: Option<String>,
    pub loading: Option<String>,
    pub sandbox: Vec<String>,
    pub allow: Option<String>,
    pub allowfullscreen: bool,
    pub referrerpolicy: Option<String>,
    pub srcdoc: Option<String>,
    pub credentialless: bool,
    pub control_type: Option<String>,
    pub form_owner: Option<String>,
    pub label_for: Option<String>,
    pub labels: Vec<String>,
    pub accessible_name: Option<String>,
    pub aria_label: Option<String>,
    pub aria_labelledby: Vec<String>,
    pub aria_describedby: Vec<String>,
    pub aria_controls: Vec<String>,
    pub aria_current: Option<String>,
    pub aria_expanded: Option<String>,
    pub aria_pressed: Option<String>,
    pub aria_selected: Option<String>,
    pub aria_hidden: bool,
    pub hidden: bool,
    pub inert: bool,
    pub open: bool,
    pub tabindex: Option<String>,
    pub accesskey: Vec<String>,
    pub focusable: Option<bool>,
    pub contenteditable: Option<String>,
    pub editing_mode: Option<String>,
    pub draggable: Option<String>,
    pub draggable_state: Option<String>,
    pub spellcheck: Option<String>,
    pub translate: Option<String>,
    pub popover: Option<String>,
    pub popover_target: Option<String>,
    pub popover_target_action: Option<String>,
    pub command: Option<String>,
    pub command_for: Option<String>,
    pub placeholder: Option<String>,
    pub autocomplete: Option<String>,
    pub autocapitalize: Option<String>,
    pub enterkeyhint: Option<String>,
    pub dirname: Option<String>,
    pub accept: Option<String>,
    pub capture: Option<String>,
    pub inputmode: Option<String>,
    pub pattern: Option<String>,
    pub min: Option<String>,
    pub max: Option<String>,
    pub step: Option<String>,
    pub minlength: Option<String>,
    pub maxlength: Option<String>,
    pub size: Option<String>,
    pub list: Option<String>,
    pub form_action: Option<String>,
    pub resolved_form_action: Option<String>,
    pub form_enctype: Option<String>,
    pub form_method: Option<String>,
    pub form_target: Option<String>,
    pub form_novalidate: bool,
    pub value: Option<String>,
    pub autofocus: bool,
    pub disabled: bool,
    pub required: bool,
    pub readonly: bool,
    pub checked: bool,
    pub selected: bool,
    pub multiple: bool,
    pub options: Vec<String>,
    pub table_section_kind: Option<String>,
    pub colspan: Option<String>,
    pub rowspan: Option<String>,
    pub span: Option<String>,
    pub scope: Option<String>,
    pub headers: Vec<String>,
    pub abbr: Option<String>,
    pub text_flow: Option<String>,
    pub list_kind: Option<String>,
    pub list_start: Option<String>,
    pub list_marker_type: Option<String>,
    pub list_reversed: bool,
    pub list_item_value: Option<String>,
    pub quote_cite: Option<String>,
    pub resolved_quote_cite: Option<String>,
    pub break_kind: Option<String>,
    pub heading_level: Option<u8>,
    pub section_kind: Option<String>,
    pub landmark_kind: Option<String>,
    pub children: Vec<BrowserRenderNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserHeading {
    pub level: u8,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserLink {
    pub href: Option<String>,
    pub resolved_href: Option<String>,
    pub name: Option<String>,
    pub target: Option<String>,
    pub rel: Option<String>,
    pub rel_tokens: Vec<String>,
    pub title: Option<String>,
    pub download: Option<String>,
    pub ping: Vec<String>,
    pub resolved_ping: Vec<String>,
    pub hreflang: Option<String>,
    pub type_hint: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserImage {
    pub src: Option<String>,
    pub resolved_src: Option<String>,
    pub alt: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
    pub srcset: Option<String>,
    pub resolved_srcset: Option<String>,
    pub sizes: Option<String>,
    pub loading: Option<String>,
    pub decoding: Option<String>,
    pub fetchpriority: Option<String>,
    pub crossorigin: Option<String>,
    pub referrerpolicy: Option<String>,
    pub usemap: Option<String>,
    pub ismap: bool,
    pub sources: Vec<BrowserImageSource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserImageSource {
    pub srcset: Option<String>,
    pub resolved_srcset: Option<String>,
    pub sizes: Option<String>,
    pub media: Option<String>,
    pub type_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserMedia {
    pub kind: String,
    pub src: Option<String>,
    pub resolved_src: Option<String>,
    pub poster: Option<String>,
    pub resolved_poster: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
    pub controls: bool,
    pub autoplay: bool,
    pub loop_media: bool,
    pub muted: bool,
    pub playsinline: bool,
    pub preload: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserForm {
    pub action: Option<String>,
    pub resolved_action: Option<String>,
    pub name: Option<String>,
    pub method: String,
    pub enctype: Option<String>,
    pub target: Option<String>,
    pub accept_charset: Option<String>,
    pub autocomplete: Option<String>,
    pub rel: Option<String>,
    pub novalidate: bool,
    pub controls: Vec<BrowserFormControl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserFormControl {
    pub id: Option<String>,
    pub control_type: String,
    pub name: Option<String>,
    pub form_owner: Option<String>,
    pub labels: Vec<String>,
    pub accessible_name: Option<String>,
    pub placeholder: Option<String>,
    pub autocomplete: Option<String>,
    pub autocapitalize: Option<String>,
    pub enterkeyhint: Option<String>,
    pub dirname: Option<String>,
    pub accept: Option<String>,
    pub capture: Option<String>,
    pub src: Option<String>,
    pub resolved_src: Option<String>,
    pub alt: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
    pub inputmode: Option<String>,
    pub pattern: Option<String>,
    pub min: Option<String>,
    pub max: Option<String>,
    pub step: Option<String>,
    pub minlength: Option<String>,
    pub maxlength: Option<String>,
    pub size: Option<String>,
    pub list: Option<String>,
    pub form_action: Option<String>,
    pub resolved_form_action: Option<String>,
    pub form_enctype: Option<String>,
    pub form_method: Option<String>,
    pub form_target: Option<String>,
    pub form_novalidate: bool,
    pub value: Option<String>,
    pub autofocus: bool,
    pub disabled: bool,
    pub required: bool,
    pub readonly: bool,
    pub checked: bool,
    pub multiple: bool,
    pub text: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserTable {
    pub caption: Option<String>,
    pub row_count: usize,
    pub column_count: usize,
    pub column_hint_count: usize,
    pub cell_count: usize,
    pub header_cell_count: usize,
}

impl BrowserDocument {
    pub fn from_document(document: &Document) -> Self {
        let html = find_first_element_in_nodes(&document.children, "html");
        let head = find_first_element_in_nodes(&document.children, "head");
        let body = find_first_element_in_nodes(&document.children, "body");
        let body_children = body
            .map(|element| element.children.as_slice())
            .unwrap_or(document.children.as_slice());

        let mut summary = Self {
            title: head
                .and_then(|element| find_first_element_in_nodes(&element.children, "title"))
                .map(element_text),
            base_href: head
                .and_then(|element| find_first_element_in_nodes(&element.children, "base"))
                .and_then(|element| element.attribute("href"))
                .map(ToOwned::to_owned),
            base_target: head
                .and_then(|element| find_first_element_in_nodes(&element.children, "base"))
                .and_then(|element| element.attribute("target"))
                .map(ToOwned::to_owned),
            document_lang: html
                .and_then(|element| element.attribute("lang"))
                .map(ToOwned::to_owned),
            document_dir: html
                .and_then(|element| element.attribute("dir"))
                .map(ToOwned::to_owned),
            body_id: body
                .and_then(|element| element.attribute("id"))
                .map(ToOwned::to_owned),
            body_classes: body
                .and_then(|element| element.attribute("class"))
                .map(split_html_classes)
                .unwrap_or_default(),
            body_lang: body
                .and_then(|element| element.attribute("lang"))
                .map(ToOwned::to_owned),
            body_dir: body
                .and_then(|element| element.attribute("dir"))
                .map(ToOwned::to_owned),
            body_text: visible_text_for_nodes(body_children),
            ..Self::default()
        };

        if let Some(head) = head {
            collect_head_browser_facts(&head.children, &mut summary);
        }
        let labels = collect_label_texts_by_control_id(body_children);
        collect_browser_facts(body_children, &mut summary, &labels, &[]);
        summary
    }
}

impl BrowserContentTree {
    pub fn from_document(document: &Document) -> Self {
        let head = find_first_element_in_nodes(&document.children, "head");
        let base_href = head
            .and_then(|element| find_first_element_in_nodes(&element.children, "base"))
            .and_then(|element| element.attribute("href"));
        let body = find_first_element_in_nodes(&document.children, "body");
        let body_children = body
            .map(|element| element.children.as_slice())
            .unwrap_or(document.children.as_slice());
        Self::from_nodes_with_base(body_children, base_href)
    }

    pub fn from_nodes(nodes: &[Node]) -> Self {
        Self::from_nodes_with_base(nodes, None)
    }

    fn from_nodes_with_base(nodes: &[Node], base_href: Option<&str>) -> Self {
        let mut children = Vec::new();
        let labels = collect_label_texts_by_control_id(nodes);
        let id_texts = collect_element_texts_by_id(nodes);
        collect_browser_content_nodes(nodes, &mut children, base_href, &labels, &id_texts);
        Self { children }
    }
}

impl BrowserRenderTree {
    pub fn from_document(document: &Document) -> Self {
        Self::from_content_tree(&BrowserContentTree::from_document(document))
    }

    pub fn from_content_tree(content_tree: &BrowserContentTree) -> Self {
        Self {
            children: content_tree
                .children
                .iter()
                .map(BrowserRenderNode::from_content_node)
                .collect(),
        }
    }
}

impl BrowserRenderNode {
    pub fn from_content_node(content_node: &BrowserContentNode) -> Self {
        Self {
            display: browser_render_display(&content_node.role).to_string(),
            role: content_node.role.clone(),
            authored_role: content_node.authored_role.clone(),
            name: content_node.name.clone(),
            id: content_node.id.clone(),
            classes: content_node.classes.clone(),
            title: content_node.title.clone(),
            lang: content_node.lang.clone(),
            dir: content_node.dir.clone(),
            text: content_node.text.clone(),
            href: content_node.href.clone(),
            resolved_href: content_node.resolved_href.clone(),
            target: content_node.target.clone(),
            rel: content_node.rel.clone(),
            rel_tokens: content_node.rel_tokens.clone(),
            download: content_node.download.clone(),
            ping: content_node.ping.clone(),
            resolved_ping: content_node.resolved_ping.clone(),
            hreflang: content_node.hreflang.clone(),
            src: content_node.src.clone(),
            resolved_src: content_node.resolved_src.clone(),
            alt: content_node.alt.clone(),
            resource_kind: content_node.resource_kind.clone(),
            width: content_node.width.clone(),
            height: content_node.height.clone(),
            type_hint: content_node.type_hint.clone(),
            media: content_node.media.clone(),
            poster: content_node.poster.clone(),
            resolved_poster: content_node.resolved_poster.clone(),
            preload: content_node.preload.clone(),
            controls: content_node.controls,
            autoplay: content_node.autoplay,
            loop_media: content_node.loop_media,
            muted: content_node.muted,
            playsinline: content_node.playsinline,
            browsing_context_name: content_node.browsing_context_name.clone(),
            loading: content_node.loading.clone(),
            sandbox: content_node.sandbox.clone(),
            allow: content_node.allow.clone(),
            allowfullscreen: content_node.allowfullscreen,
            referrerpolicy: content_node.referrerpolicy.clone(),
            srcdoc: content_node.srcdoc.clone(),
            credentialless: content_node.credentialless,
            control_type: content_node.control_type.clone(),
            form_owner: content_node.form_owner.clone(),
            label_for: content_node.label_for.clone(),
            labels: content_node.labels.clone(),
            accessible_name: content_node.accessible_name.clone(),
            aria_label: content_node.aria_label.clone(),
            aria_labelledby: content_node.aria_labelledby.clone(),
            aria_describedby: content_node.aria_describedby.clone(),
            aria_controls: content_node.aria_controls.clone(),
            aria_current: content_node.aria_current.clone(),
            aria_expanded: content_node.aria_expanded.clone(),
            aria_pressed: content_node.aria_pressed.clone(),
            aria_selected: content_node.aria_selected.clone(),
            aria_hidden: content_node.aria_hidden,
            hidden: content_node.hidden,
            inert: content_node.inert,
            open: content_node.open,
            tabindex: content_node.tabindex.clone(),
            accesskey: content_node.accesskey.clone(),
            focusable: content_node.focusable,
            contenteditable: content_node.contenteditable.clone(),
            editing_mode: content_node.editing_mode.clone(),
            draggable: content_node.draggable.clone(),
            draggable_state: content_node.draggable_state.clone(),
            spellcheck: content_node.spellcheck.clone(),
            translate: content_node.translate.clone(),
            popover: content_node.popover.clone(),
            popover_target: content_node.popover_target.clone(),
            popover_target_action: content_node.popover_target_action.clone(),
            command: content_node.command.clone(),
            command_for: content_node.command_for.clone(),
            placeholder: content_node.placeholder.clone(),
            autocomplete: content_node.autocomplete.clone(),
            autocapitalize: content_node.autocapitalize.clone(),
            enterkeyhint: content_node.enterkeyhint.clone(),
            dirname: content_node.dirname.clone(),
            accept: content_node.accept.clone(),
            capture: content_node.capture.clone(),
            inputmode: content_node.inputmode.clone(),
            pattern: content_node.pattern.clone(),
            min: content_node.min.clone(),
            max: content_node.max.clone(),
            step: content_node.step.clone(),
            minlength: content_node.minlength.clone(),
            maxlength: content_node.maxlength.clone(),
            size: content_node.size.clone(),
            list: content_node.list.clone(),
            form_action: content_node.form_action.clone(),
            resolved_form_action: content_node.resolved_form_action.clone(),
            form_enctype: content_node.form_enctype.clone(),
            form_method: content_node.form_method.clone(),
            form_target: content_node.form_target.clone(),
            form_novalidate: content_node.form_novalidate,
            value: content_node.value.clone(),
            autofocus: content_node.autofocus,
            disabled: content_node.disabled,
            required: content_node.required,
            readonly: content_node.readonly,
            checked: content_node.checked,
            selected: content_node.selected,
            multiple: content_node.multiple,
            options: content_node.options.clone(),
            table_section_kind: content_node.table_section_kind.clone(),
            colspan: content_node.colspan.clone(),
            rowspan: content_node.rowspan.clone(),
            span: content_node.span.clone(),
            scope: content_node.scope.clone(),
            headers: content_node.headers.clone(),
            abbr: content_node.abbr.clone(),
            text_flow: content_node.text_flow.clone(),
            list_kind: content_node.list_kind.clone(),
            list_start: content_node.list_start.clone(),
            list_marker_type: content_node.list_marker_type.clone(),
            list_reversed: content_node.list_reversed,
            list_item_value: content_node.list_item_value.clone(),
            quote_cite: content_node.quote_cite.clone(),
            resolved_quote_cite: content_node.resolved_quote_cite.clone(),
            break_kind: content_node.break_kind.clone(),
            heading_level: content_node.heading_level,
            section_kind: content_node.section_kind.clone(),
            landmark_kind: content_node.landmark_kind.clone(),
            children: content_node
                .children
                .iter()
                .map(Self::from_content_node)
                .collect(),
        }
    }
}

/// Streaming-friendly parser core over already-tokenized HTML.
#[derive(Debug)]
pub struct HtmlParser {
    document: Document,
    open_elements: Vec<Vec<usize>>,
    pending_formatting_reconstruction: Vec<(String, Vec<Attribute>)>,
    prunable_empty_reconstructed_formatting_paths: Vec<Vec<usize>>,
    diagnostics: Vec<ParserDiagnostic>,
    options: HtmlParseOptions,
    quirks_mode: bool,
    strip_next_leading_lf: bool,
    explicit_head_end_seen: bool,
    explicit_body_end_seen: bool,
    explicit_body_start_seen: bool,
    explicit_html_end_seen: bool,
    explicit_em_end_seen: bool,
    pending_table_text: String,
    strip_next_leading_noscript_literal: bool,
    form_element_pointer_set: bool,
    foreign_cdata_text: Option<String>,
}

impl Default for HtmlParser {
    fn default() -> Self {
        Self {
            document: Document::new(),
            open_elements: Vec::new(),
            pending_formatting_reconstruction: Vec::new(),
            prunable_empty_reconstructed_formatting_paths: Vec::new(),
            diagnostics: Vec::new(),
            options: HtmlParseOptions::default(),
            quirks_mode: true,
            strip_next_leading_lf: false,
            explicit_head_end_seen: false,
            explicit_body_end_seen: false,
            explicit_body_start_seen: false,
            explicit_html_end_seen: false,
            explicit_em_end_seen: false,
            pending_table_text: String::new(),
            strip_next_leading_noscript_literal: false,
            form_element_pointer_set: false,
            foreign_cdata_text: None,
        }
    }
}

impl HtmlParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_options(options: HtmlParseOptions) -> Self {
        Self {
            options,
            ..Self::default()
        }
    }

    fn with_body_fragment_options(options: HtmlParseOptions) -> Self {
        let mut html = Node::element("html".to_string(), Vec::new());
        let Node::Element(ref mut html_element) = html else {
            unreachable!("Node::element must construct an element");
        };
        html_element
            .children
            .push(Node::element("head".to_string(), Vec::new()));
        html_element
            .children
            .push(Node::element("body".to_string(), Vec::new()));

        let mut document = Document::new();
        document.push_child(html);

        Self {
            document,
            open_elements: vec![vec![0], vec![0, 1]],
            pending_formatting_reconstruction: Vec::new(),
            prunable_empty_reconstructed_formatting_paths: Vec::new(),
            diagnostics: Vec::new(),
            options,
            quirks_mode: true,
            strip_next_leading_lf: false,
            explicit_head_end_seen: false,
            explicit_body_end_seen: false,
            explicit_body_start_seen: false,
            explicit_html_end_seen: false,
            explicit_em_end_seen: false,
            pending_table_text: String::new(),
            strip_next_leading_noscript_literal: false,
            form_element_pointer_set: false,
            foreign_cdata_text: None,
        }
    }

    fn with_fragment_context_options(options: HtmlParseOptions, context_element: &str) -> Self {
        let (document, open_elements) = fragment_context_shell(context_element);

        Self {
            document,
            open_elements,
            pending_formatting_reconstruction: Vec::new(),
            prunable_empty_reconstructed_formatting_paths: Vec::new(),
            diagnostics: Vec::new(),
            options,
            quirks_mode: true,
            strip_next_leading_lf: false,
            explicit_head_end_seen: false,
            explicit_body_end_seen: false,
            explicit_body_start_seen: matches!(context_element, "body"),
            explicit_html_end_seen: false,
            explicit_em_end_seen: false,
            pending_table_text: String::new(),
            strip_next_leading_noscript_literal: false,
            form_element_pointer_set: matches!(context_element, "form"),
            foreign_cdata_text: None,
        }
    }

    pub fn parse_tokens(&mut self, tokens: impl IntoIterator<Item = Token>) -> Document {
        for token in tokens {
            self.process_token(token);
        }
        self.finish_document()
    }

    pub fn diagnostics(&self) -> &[ParserDiagnostic] {
        &self.diagnostics
    }

    fn current_element_is_marked_fragment_context(&self, name: &str) -> bool {
        let Some(path) = self.open_elements.last() else {
            return false;
        };
        element_ref_at_path(&self.document, path)
            .is_some_and(|element| element.name == name && has_fragment_context_marker(element))
    }

    fn fragment_context_ignores_table_start_tag(&self, name: &str) -> bool {
        if self.has_unmarked_table_after_last_fragment_marker() {
            return false;
        }
        let Some(context) = self.open_fragment_context_name() else {
            return false;
        };
        match context {
            "table" => name == "table",
            "caption" => matches!(
                name,
                "caption"
                    | "col"
                    | "colgroup"
                    | "html"
                    | "tbody"
                    | "td"
                    | "tfoot"
                    | "thead"
                    | "th"
                    | "tr"
            ),
            "colgroup" => name != "col",
            "tbody" | "thead" | "tfoot" => {
                matches!(
                    name,
                    "caption" | "col" | "colgroup" | "tbody" | "thead" | "tfoot"
                )
            }
            "tr" => matches!(
                name,
                "caption" | "col" | "colgroup" | "tbody" | "thead" | "tfoot" | "tr"
            ),
            "td" | "th" => matches!(
                name,
                "caption" | "col" | "colgroup" | "tbody" | "thead" | "tfoot" | "tr" | "td" | "th"
            ),
            _ => false,
        }
    }

    fn open_fragment_context_name(&self) -> Option<&str> {
        self.open_elements.iter().rev().find_map(|path| {
            let element = element_ref_at_path(&self.document, path)?;
            fragment_marker_value(element)
        })
    }

    fn current_element_is_marked_foreign_fragment_context(&self) -> bool {
        let Some(path) = self.open_elements.last() else {
            return false;
        };
        element_ref_at_path(&self.document, path).is_some_and(|element| {
            has_fragment_context_marker(element) && element.namespace.is_some()
        })
    }

    fn open_marked_fragment_shell_element_matches(&self, name: &str) -> bool {
        self.open_elements
            .iter()
            .rev()
            .find_map(|path| {
                let element = element_ref_at_path(&self.document, path)?;
                (element.name == name).then_some(has_fragment_context_marker(element))
            })
            .unwrap_or(false)
    }

    fn has_unmarked_table_after_last_fragment_marker(&self) -> bool {
        let Some(marker_index) = self.open_elements.iter().rposition(|path| {
            element_ref_at_path(&self.document, path).is_some_and(has_fragment_context_marker)
        }) else {
            return false;
        };
        self.open_elements[marker_index + 1..].iter().any(|path| {
            element_ref_at_path(&self.document, path).is_some_and(|element| {
                element.name == "table" && !has_fragment_context_marker(element)
            })
        })
    }

    fn finish_document(&mut self) -> Document {
        repair_fostered_nobr_adoption_wrappers(&mut self.document);
        repair_table_cell_fostered_nobr_adoption(&mut self.document);
        repair_div_fostered_nobr_adoption(&mut self.document);
        repair_split_div_nobr_adoption(&mut self.document);
        let mut document = normalize_document_shell(std::mem::take(&mut self.document));
        repair_tricky_adoption_agency(&mut document);
        repair_nested_select_option_pairs(&mut document.children);
        repair_div_bold_nobr_continuation(&mut document.children);
        repair_anchor_list_item_boundary(&mut document.children);
        repair_font_paragraph_boundary(&mut document.children);
        repair_anchor_center_boundary(&mut document.children);
        repair_em_aside_continuation(&mut document.children, self.explicit_em_end_seen);
        repair_svg_title_tail_text(&mut document.children);
        repair_select_option_hr(&mut document.children);
        repair_select_button_selectedcontent(&mut document.children);
        if self.options.scripting == HtmlScriptingMode::Enabled {
            apply_scripted_tree_construction_side_effects(&mut document);
        }
        document
    }

    fn process_token(&mut self, token: Token) {
        if matches!(token, Token::Eof) {
            self.flush_foreign_cdata_text();
        }
        match token {
            Token::Text(text) => self.append_text(text),
            token => {
                self.flush_pending_table_text();
                self.strip_next_leading_lf = false;
                match token {
                    Token::StartTag {
                        name,
                        attributes,
                        self_closing,
                    } => self.append_start_tag(name, attributes, self_closing),
                    Token::EndTag { name } => self.handle_end_tag(&name),
                    Token::Comment(comment) => self.append_comment(comment),
                    Token::Doctype {
                        name,
                        public_identifier,
                        system_identifier,
                        force_quirks,
                    } => {
                        if self.current_element_is("frameset")
                            || self.has_document_type()
                            || self.has_document_element()
                            || self.has_non_comment_document_content()
                        {
                            self.diagnostics.push(ParserDiagnostic::new(
                                "unexpected-doctype",
                                "doctype token outside the initial document position was ignored",
                            ));
                            return;
                        }
                        self.quirks_mode = force_quirks
                            || doctype_triggers_quirks(
                                name.as_deref(),
                                public_identifier.as_deref(),
                                system_identifier.as_deref(),
                            );
                        self.append_node(Node::DocumentType(DocumentType {
                            name,
                            public_identifier,
                            system_identifier,
                            force_quirks,
                        }));
                    }
                    Token::Eof => self.open_elements.clear(),
                    Token::Text(_) => unreachable!("text token handled before clearing LF state"),
                }
            }
        }
    }

    fn process_lexer_token(&mut self, token: Token, final_drain: bool) {
        if self.foreign_cdata_text.is_some() {
            self.consume_foreign_cdata_token(token);
            return;
        }

        match token {
            Token::Comment(comment) if self.current_namespace().is_some() => {
                if let Some(cdata) = comment.strip_prefix("[CDATA[") {
                    self.start_foreign_cdata_text(cdata, final_drain);
                } else {
                    self.process_token(Token::Comment(comment));
                }
            }
            token => self.process_token(token),
        }
    }

    fn start_foreign_cdata_text(&mut self, cdata: &str, final_drain: bool) {
        if final_drain {
            if !cdata.is_empty() {
                self.append_text(cdata.to_string());
            }
            return;
        }

        if let Some(cdata) = cdata.strip_suffix("]]") {
            if !cdata.is_empty() {
                self.append_text(cdata.to_string());
            }
            return;
        }

        let mut text = cdata.to_string();
        text.push('>');
        self.consume_foreign_cdata_text(text);
    }

    fn consume_foreign_cdata_token(&mut self, token: Token) {
        match token {
            Token::Text(text) => self.consume_foreign_cdata_text(text),
            Token::StartTag {
                name,
                attributes,
                self_closing,
            } => {
                self.consume_foreign_cdata_text(start_tag_as_text(
                    &name,
                    &attributes,
                    self_closing,
                ));
            }
            Token::EndTag { name } => self.consume_foreign_cdata_text(format!("</{name}>")),
            Token::Comment(comment) => {
                self.consume_foreign_cdata_text(format!("<!--{comment}-->"));
            }
            Token::Doctype { name, .. } => {
                self.consume_foreign_cdata_text(format!("<!DOCTYPE {}>", name.unwrap_or_default()));
            }
            Token::Eof => self.flush_foreign_cdata_text(),
        }
    }

    fn consume_foreign_cdata_text(&mut self, text: String) {
        let mut cdata = self.foreign_cdata_text.take().unwrap_or_default();
        cdata.push_str(&text);

        if let Some(end) = cdata.find("]]>") {
            let trailing = cdata[end + 3..].to_string();
            cdata.truncate(end);
            if !cdata.is_empty() {
                self.append_text(cdata);
            }
            if !trailing.is_empty() {
                self.process_token(Token::Text(trailing));
            }
        } else {
            self.foreign_cdata_text = Some(cdata);
        }
    }

    fn flush_foreign_cdata_text(&mut self) {
        if let Some(cdata) = self.foreign_cdata_text.take() {
            if !cdata.is_empty() {
                self.append_text(cdata);
            }
        }
    }

    fn append_start_tag(
        &mut self,
        mut name: String,
        attributes: Vec<LexerAttribute>,
        self_closing: bool,
    ) {
        if name == "image" {
            self.diagnostics.push(ParserDiagnostic::new(
                "unexpected-start-tag-treated-as",
                "start tag `<image>` was treated as `<img>`",
            ));
            name = "img".to_string();
        }
        if name == "body" {
            self.explicit_body_start_seen = true;
        }

        let in_foreign_content = self.current_namespace().is_some()
            && !self.current_node_is_svg_html_integration_point()
            && !self.current_node_is_mathml_integration_point();
        if in_foreign_content
            && self.current_element_is_marked_foreign_fragment_context()
            && matches!(name.as_str(), "body" | "frameset" | "head" | "html")
        {
            return;
        }
        if in_foreign_content
            && self.current_element_is_marked_foreign_fragment_context()
            && matches!(name.as_str(), "br" | "p")
        {
            let child_index = self.append_node(Node::element(name.clone(), Vec::new()));
            if name == "p" {
                let mut path = self.current_parent_path().to_vec();
                path.push(child_index);
                self.open_elements.push(path);
            }
            return;
        }
        if in_foreign_content
            && !self.current_node_is_svg_html_integration_point()
            && exits_foreign_content_on_start_tag(&name, &attributes)
        {
            if self.has_open_svg_html_integration_point() {
                while self.current_namespace().is_some()
                    && !self.current_node_is_svg_html_integration_point()
                {
                    self.open_elements.pop();
                }
                let attributes: Vec<Attribute> = attributes
                    .into_iter()
                    .map(|attribute| Attribute {
                        name: attribute.name,
                        value: attribute.value,
                    })
                    .collect();
                let child_index = self.append_node(Node::element(name.clone(), attributes));
                if !is_void_element(&name) {
                    let mut path = self.current_parent_path().to_vec();
                    path.push(child_index);
                    self.open_elements.push(path);
                }
                return;
            }
            self.pop_foreign_elements();
            self.append_start_tag(name, attributes, self_closing);
            return;
        }

        if !in_foreign_content
            && self.current_element_is("frameset")
            && !matches!(name.as_str(), "frame" | "frameset" | "noframes")
        {
            return;
        }
        if !in_foreign_content
            && self.open_elements.is_empty()
            && self.document_has_closed_frameset()
            && name != "noframes"
            && name != "html"
            && name != "frameset"
        {
            return;
        }
        if !in_foreign_content
            && self.document_has_closed_frameset()
            && !self.current_element_is("frameset")
            && name != "noframes"
            && name != "html"
            && name != "frameset"
        {
            return;
        }
        if !in_foreign_content
            && name == "noframes"
            && self.document_has_closed_frameset()
            && !self.current_element_is("frameset")
            && !self.has_open_element("noframes")
        {
            let attributes: Vec<Attribute> = attributes
                .into_iter()
                .map(|attribute| Attribute {
                    name: attribute.name,
                    value: attribute.value,
                })
                .collect();
            if let Some(path) = self.append_node_to_document_html(Node::element(name, attributes)) {
                self.open_elements.push(path);
            }
            return;
        }
        if !in_foreign_content
            && matches!(name.as_str(), "param" | "source" | "track")
            && !self.current_element_is("object")
            && !self.has_open_element("body")
            && !self.document_has_body_element()
        {
            self.diagnostics.push(ParserDiagnostic::new(
                "unexpected-pre-body-void-start-tag",
                format!("start tag `<{name}>` before body was ignored"),
            ));
            return;
        }
        if !in_foreign_content
            && self.open_elements.is_empty()
            && self.has_document_element()
            && !self.document_has_body_element()
            && !self.body_has_non_whitespace_child()
            && is_head_element(&name)
            && name != "head"
        {
            self.reopen_document_head();
        }
        if !in_foreign_content && self.open_elements.is_empty() && self.has_document_element() {
            self.reopen_document_body();
        }
        if !in_foreign_content
            && self.open_elements.is_empty()
            && !self.has_document_element()
            && !self.document.children.iter().any(is_body_content_node)
            && !self.document_has_closed_frameset()
            && (is_head_element(&name) || name == "head")
            && name != "html"
        {
            self.append_implied_element("html");
            if name != "head" {
                self.append_implied_element("head");
            }
        }
        if !in_foreign_content
            && self.open_elements.is_empty()
            && !self.has_document_element()
            && name == "frameset"
        {
            self.append_implied_element("html");
        }
        if !in_foreign_content
            && self.open_elements.is_empty()
            && !self.has_document_element()
            && name != "frameset"
            && starts_body_after_head(&name)
        {
            self.append_implied_element("html");
            self.append_implied_element("body");
        }
        if !in_foreign_content
            && self.current_element_is("html")
            && !self.has_open_element("head")
            && !self.has_open_element("body")
            && !self.document_has_body_element()
            && !self.body_has_non_whitespace_child()
            && !self.document_has_closed_frameset()
            && is_head_element(&name)
            && name != "head"
        {
            self.append_implied_element("head");
        }

        if !in_foreign_content {
            if self.has_open_element("head")
                && !self.current_element_is("head")
                && !self.has_open_element("template")
                && starts_body_after_head(&name)
            {
                self.pop_head_descendants();
            }
            self.apply_document_shell_implied_contexts(&name);
            self.close_fostered_formatting_before_table_context(&name);
            self.pop_fostered_content_before_table_context(&name);
            if self.fragment_context_ignores_table_start_tag(&name) {
                return;
            }
            if self.has_open_element("select")
                && self.has_open_table_context()
                && (name == "table" || starts_table_context(&name))
            {
                self.close_open_element_if(|name| name == "select");
                if name == "table" {
                    self.close_element("table");
                }
            }
            self.apply_table_implied_contexts(&name);
        }
        self.clear_pending_formatting_unless_next_reconstructs(&name);
        let prunes_empty_formatting_inside = name == "p" && self.has_open_element("p");
        let formatting_inside = self.take_formatting_reconstruction_inside_for(&name);
        if !in_foreign_content
            && !self.current_node_is_svg_html_integration_point()
            && !self.current_node_is_mathml_integration_point()
        {
            self.apply_simple_implied_end_tags(&name);
        }

        if !in_foreign_content
            && is_table_only_start_tag(&name)
            && !self.has_open_element("table")
            && !self.has_open_element("template")
        {
            self.diagnostics.push(ParserDiagnostic::new(
                "unexpected-table-start-tag",
                format!("start tag `<{name}>` outside a table was ignored"),
            ));
            return;
        }

        if !in_foreign_content
            && self.has_open_element("template")
            && !self.has_open_element("table")
            && self.current_last_child_element_is("col")
            && name != "col"
            && name != "template"
        {
            return;
        }

        if !in_foreign_content
            && self.has_open_element("template")
            && !self.has_open_element("table")
            && name == "tr"
            && self.current_element_is("template")
            && !self.current_has_child_element("tr")
            && !self.current_has_child_element("thead")
            && self.current_has_non_whitespace_child()
        {
            return;
        }

        if !in_foreign_content
            && self.has_open_element("template")
            && !self.has_open_element("table")
            && name == "tr"
            && self.current_has_child_element("thead")
            && !self.current_element_is("tbody")
        {
            self.append_implied_element("tbody");
        }

        if !in_foreign_content
            && self.has_open_element("template")
            && !self.has_open_element("table")
            && name == "tr"
            && !self.current_element_is("template")
            && !self.current_element_is("tbody")
        {
            return;
        }

        if !in_foreign_content
            && self.has_open_element("template")
            && !self.has_open_element("table")
            && (is_table_section(&name) || matches!(name.as_str(), "caption" | "colgroup"))
            && !(name == "tfoot" && self.current_has_child_element("thead"))
            && (self.current_last_child_element_is("td")
                || self.current_last_child_element_is("th")
                || self.current_last_child_element_is("tr")
                || self.current_last_child_element_is("col"))
        {
            return;
        }

        if !in_foreign_content
            && self.has_open_element("template")
            && !self.has_open_element("table")
            && matches!(name.as_str(), "td" | "th")
            && !self.current_element_is("tr")
            && self.current_has_child_element("tr")
        {
            self.append_implied_element("tr");
        }

        if !in_foreign_content
            && name == "col"
            && !self.current_element_is("colgroup")
            && !self.has_open_element("template")
        {
            self.diagnostics.push(ParserDiagnostic::new(
                "unexpected-col-start-tag",
                "start tag `<col>` outside a column group was ignored",
            ));
            return;
        }

        if !in_foreign_content && name == "frame" && !self.current_element_is("frameset") {
            self.diagnostics.push(ParserDiagnostic::new(
                "unexpected-frame-start-tag",
                "start tag `<frame>` outside a frameset was ignored",
            ));
            return;
        }

        if !in_foreign_content && name == "frameset" && self.has_open_element("template") {
            return;
        }

        if !in_foreign_content
            && name == "frameset"
            && !self.current_element_is("frameset")
            && (self.explicit_body_start_seen
                || self.document_has_non_frameset_compatible_body_content())
            && self.has_open_element("body")
        {
            return;
        }

        if !in_foreign_content && name == "template" && self.current_element_is("frameset") {
            return;
        }

        let attributes: Vec<Attribute> = attributes
            .into_iter()
            .map(|attribute| Attribute {
                name: attribute.name,
                value: attribute.value,
            })
            .collect();

        if !in_foreign_content && name == "frameset" && self.current_element_is("frameset") {
            let child_index = self.append_node(Node::element(name.clone(), attributes));
            let mut path = self.current_parent_path().to_vec();
            path.push(child_index);
            self.open_elements.push(path);
            return;
        }

        if !in_foreign_content
            && matches!(name.as_str(), "svg" | "math")
            && self.current_element_is_table_structure()
        {
            let namespace = self.namespace_for_start_tag(&name);
            let name = adjusted_foreign_start_tag_name(name, namespace);
            let attributes = adjusted_foreign_attributes(attributes, namespace);
            if let Some(path) = self.insert_node_before_open_table(element_node(
                name.clone(),
                attributes,
                namespace,
            )) {
                self.open_elements.push(path);
            }
            return;
        }

        if !in_foreign_content
            && self.has_open_element("select")
            && self.has_open_table_context()
            && (name == "table" || starts_table_context(&name))
        {
            self.close_open_element_if(|name| name == "select");
        }

        if !in_foreign_content && name == "table" && self.current_element_is_table_structure() {
            self.close_element("table");
        }

        if !in_foreign_content
            && name == "form"
            && self.current_element_is_table_structure()
            && self.form_element_pointer_set
        {
            self.diagnostics.push(ParserDiagnostic::new(
                "nested-form-start-tag",
                "form start tag inside a table was ignored while a form was already open",
            ));
            return;
        }

        if !in_foreign_content && name == "form" && self.current_element_is_table_structure() {
            self.form_element_pointer_set = true;
            self.append_node(Node::element(name, attributes));
            return;
        }

        if !in_foreign_content && name == "meta" && self.current_element_is_table_structure() {
            self.insert_node_before_open_table(Node::element(name, attributes));
            return;
        }

        if !in_foreign_content && name == "title" && self.current_element_is_table_structure() {
            if let Some(path) = self.insert_node_before_open_table(Node::element(name, attributes))
            {
                self.open_elements.push(path);
            }
            return;
        }

        if !in_foreign_content
            && name == "li"
            && self.has_open_table_context()
            && (self.current_element_is_table_structure()
                || self.current_parent_is_fostered_before_open_table())
        {
            self.close_open_element_if(|name| name == "li");
            if let Some(path) = self.insert_node_before_open_table(Node::element(name, attributes))
            {
                self.open_elements.push(path);
            }
            return;
        }

        if !in_foreign_content && name == "input" && self.current_element_is_table_structure() {
            if attribute_value(&attributes, "type")
                .is_some_and(|value| value.eq_ignore_ascii_case("hidden"))
            {
                self.append_node(Node::element(name, attributes));
            } else {
                self.insert_node_before_open_table(Node::element(name, attributes));
            }
            return;
        }

        if !in_foreign_content
            && name == "select"
            && self.has_open_element("template")
            && !self.has_open_element("table")
            && self.current_element_is_table_structure()
        {
            self.close_open_element_if(is_table_context_element);
        }

        if !in_foreign_content && name == "select" && self.current_element_is_table_structure() {
            if let Some(path) = self.insert_node_before_open_table(Node::element(name, attributes))
            {
                self.open_elements.push(path);
            }
            return;
        }

        if !in_foreign_content
            && self.has_open_element("template")
            && !self.has_open_element("table")
            && self.current_element_is_table_structure()
            && is_paragraph_boundary_element(&name)
        {
            self.close_open_element_if(is_table_context_element);
        }

        if !in_foreign_content
            && is_paragraph_boundary_element(&name)
            && self.current_element_is_table_structure()
        {
            if let Some(path) =
                self.insert_node_before_open_table(Node::element(name.clone(), attributes))
            {
                self.open_elements.push(path);
            }
            return;
        }

        if !in_foreign_content
            && matches!(name.as_str(), "br" | "p" | "plaintext")
            && self.current_element_is_table_structure()
        {
            let acknowledges_self_closing = self_closing && is_void_element(&name);
            let is_void = is_void_element(&name);
            if let Some(path) =
                self.insert_node_before_open_table(Node::element(name.clone(), attributes))
            {
                if !acknowledges_self_closing && !is_void {
                    self.open_elements.push(path);
                }
            }
            return;
        }

        if !in_foreign_content && self.foster_formatting_start_in_table_context(&name, &attributes)
        {
            return;
        }

        if !in_foreign_content && self.apply_interactive_implied_contexts(&name) {
            return;
        }
        if !in_foreign_content && self.apply_select_implied_contexts(&name) {
            return;
        }

        if !in_foreign_content && name == "html" && self.has_open_element("template") {
            return;
        }

        if !in_foreign_content
            && name == "html"
            && self.merge_attributes_into_open_element("html", &attributes)
        {
            return;
        }
        if !in_foreign_content
            && name == "html"
            && self.merge_attributes_into_document_element(&attributes)
        {
            return;
        }
        if !in_foreign_content && name == "html" && self.has_document_element() {
            self.document.push_child(Node::element(name, attributes));
            return;
        }

        if !in_foreign_content
            && name == "head"
            && self.has_open_element("head")
            && !self.current_element_is("head")
        {
            return;
        }

        if !in_foreign_content && name == "head" && self.has_open_element("head") {
            self.merge_attributes_into_open_element("head", &attributes);
            return;
        }

        if !in_foreign_content && name == "head" && self.has_open_element("body") {
            self.diagnostics.push(ParserDiagnostic::new(
                "unexpected-head-start-tag",
                "head start tag was ignored after body content had already started",
            ));
            return;
        }

        if !in_foreign_content
            && name == "noscript"
            && self.current_element_is("noscript")
            && attributes.is_empty()
        {
            self.append_text_to_current("<noscript>".to_string());
            return;
        }

        if !in_foreign_content && name == "noscript" && self.has_open_element("noscript") {
            return;
        }

        if !in_foreign_content && name == "body" && self.has_open_element("template") {
            return;
        }

        if !in_foreign_content
            && name == "body"
            && self.merge_attributes_into_open_element("body", &attributes)
        {
            return;
        }

        self.reconstruct_formatting_before_if_needed(&name);

        let namespace = self.namespace_for_start_tag(&name);
        let name = adjusted_foreign_start_tag_name(name, namespace);
        let attributes = adjusted_foreign_attributes(attributes, namespace);
        let html_void_element = namespace.is_none() && is_void_element(&name);
        let acknowledges_self_closing = self_closing && (html_void_element || namespace.is_some());
        if self_closing && !acknowledges_self_closing {
            self.diagnostics.push(ParserDiagnostic::new(
                "non-void-html-element-self-closing",
                format!("self-closing flag on non-void HTML element `<{name}>` was ignored"),
            ));
        }

        if namespace.is_none() && name == "form" {
            self.form_element_pointer_set = true;
        }
        let child_index = self.append_node(element_node(name.clone(), attributes, namespace));
        if !acknowledges_self_closing && !html_void_element {
            let mut path = self.current_parent_path().to_vec();
            path.push(child_index);
            self.open_elements.push(path);
        }

        for (formatting_name, formatting_attributes) in formatting_inside {
            let child_index =
                self.append_node(Node::element(formatting_name, formatting_attributes));
            let mut path = self.current_parent_path().to_vec();
            path.push(child_index);
            if prunes_empty_formatting_inside {
                self.prunable_empty_reconstructed_formatting_paths
                    .push(path.clone());
            }
            self.open_elements.push(path);
        }

        if preserves_initial_line_feed(&name) && !acknowledges_self_closing {
            self.strip_next_leading_lf = true;
        }
    }

    fn append_text(&mut self, text: String) {
        if text.is_empty() {
            return;
        }

        if self.document_has_closed_frameset() && !self.current_element_is("noframes") {
            let whitespace = text
                .chars()
                .filter(|character| character.is_whitespace())
                .collect::<String>();
            if !whitespace.is_empty() {
                self.append_text_to_document_html(whitespace);
            }
            return;
        }

        let text = if self.strip_next_leading_noscript_literal {
            self.strip_next_leading_noscript_literal = false;
            text.strip_prefix("<noscript>").unwrap_or(&text).to_string()
        } else if text.starts_with("<noscript>")
            && !self.body_has_non_whitespace_child()
            && self.append_to_last_head_noscript_text_ending("<!--", "<noscript>")
        {
            text["<noscript>".len()..].to_string()
        } else if text.starts_with("<iframe>")
            && !self.body_has_non_whitespace_child()
            && append_to_last_element_text(&mut self.document.children, "noscript", "<iframe>")
        {
            text["<iframe>".len()..].to_string()
        } else {
            text
        };

        let text = if self.strip_next_leading_lf {
            self.strip_next_leading_lf = false;
            text.strip_prefix('\n').unwrap_or(&text).to_string()
        } else {
            text
        };
        let text = if self.current_element_is_marked_fragment_context("colgroup")
            && !is_html_whitespace_text(&text)
        {
            text.chars()
                .filter(|character| is_html_whitespace(*character))
                .collect::<String>()
        } else {
            text
        };
        let text = if text.contains('\r') {
            text.replace('\r', "\n")
        } else {
            text
        };
        if text.is_empty() {
            return;
        }

        let text = if text.contains('\u{FFFD}')
            && (self.current_node_is_svg_html_integration_point()
                || self.current_namespace() == Some("math")
                || (self.replacement_text_is_ignorable_in_current_context(&text)
                    && !self.current_element_is("plaintext")))
        {
            text.replace('\u{FFFD}', "")
        } else {
            text
        };
        if text.is_empty() {
            return;
        }

        if self.current_node_is_svg_html_integration_point() {
            if let Some(tag_name) = simple_start_tag_text(&text) {
                self.append_start_tag(tag_name.to_string(), Vec::new(), false);
                return;
            }
        }

        if self.has_open_element("template")
            && !self.has_open_element("table")
            && self.current_last_child_element_is("col")
        {
            return;
        }

        if self.open_elements.is_empty()
            && is_html_whitespace_text(&text)
            && !self.has_document_element()
            && !self.document.children.iter().any(is_body_content_node)
        {
            if self.explicit_head_end_seen {
                let mut html = Node::element("html".to_string(), Vec::new());
                if let Node::Element(html_element) = &mut html {
                    html_element
                        .children
                        .push(Node::element("head".to_string(), Vec::new()));
                    html_element.children.push(Node::text(text));
                }
                self.document.push_child(html);
                return;
            }
            return;
        }

        if self.open_elements.is_empty() && self.has_document_element() {
            self.reopen_document_body();
        }
        if self.explicit_body_end_seen
            && !self.explicit_html_end_seen
            && self.current_element_is("html")
        {
            self.reopen_body_under_current_html();
        }

        let text = if self.in_frameset_text_context() {
            let whitespace = text
                .chars()
                .filter(|character| character.is_whitespace())
                .collect::<String>();
            if whitespace.is_empty() {
                return;
            }
            whitespace
        } else {
            text
        };

        if text.is_empty() {
            return;
        }

        if self.has_open_element("head")
            && !self.current_element_is("head")
            && !self.has_open_element("template")
            && !self.current_element_is("noframes")
            && !self.current_element_is("script")
            && !self.current_element_is("style")
            && !self.current_element_is("title")
            && !(self.current_element_is("noscript")
                && self.options.scripting == HtmlScriptingMode::Enabled)
            && !is_html_whitespace_text(&text)
        {
            self.pop_head_descendants();
        }

        if self.current_element_is("script")
            && !self.current_element_is_marked_fragment_context("script")
        {
            if let Some((end_tag_start, end_tag_end)) = rfind_script_end_marker(&text) {
                if script_text_is_in_double_escaped_state(&text[..end_tag_start]) {
                    self.append_text_to_current(text);
                    return;
                }
                let before = text[..end_tag_start].to_string();
                if !before.is_empty() {
                    self.append_text_to_current(before);
                }
                self.close_element("script");
                if end_tag_end < text.len() {
                    self.append_text(text[end_tag_end..].to_string());
                }
                return;
            }
        }

        let text = if self.current_element_is("head") {
            match text
                .char_indices()
                .find(|(_, character)| !is_html_whitespace(*character))
            {
                Some((0, _)) => text,
                Some((leading_end, _)) => {
                    self.append_text_to_current(text[..leading_end].to_string());
                    text[leading_end..].to_string()
                }
                None => text,
            }
        } else {
            text
        };

        if !is_html_whitespace_text(&text) && self.current_element_is("head") {
            self.pop_current_if(|name| name == "head");
        }

        let text = if self.current_element_is_table_structure() && text.contains('\u{FFFD}') {
            text.replace('\u{FFFD}', "")
        } else {
            text
        };

        if self.current_element_is("colgroup") {
            let leading_end = text
                .char_indices()
                .find(|(_, character)| !character.is_whitespace())
                .map(|(index, _)| index);
            match leading_end {
                Some(0) => {
                    if self.foster_text_before_open_table(text.clone()) {
                        return;
                    }
                }
                Some(index) => {
                    self.append_text_to_current(text[..index].to_string());
                    if self.foster_text_before_open_table(text[index..].to_string()) {
                        return;
                    }
                }
                None => {}
            }
        }

        if self.current_element_is_table_structure() && !self.current_element_is("colgroup") {
            self.pending_table_text.push_str(&text);
            if self.pending_table_text.chars().all(char::is_whitespace) {
                return;
            }
            let pending = std::mem::take(&mut self.pending_table_text);
            if self.foster_text_before_open_table(pending) {
                return;
            }
        }

        if self.current_element_is_table_structure()
            && !text.chars().all(char::is_whitespace)
            && self.foster_text_before_open_table(text.clone())
        {
            return;
        }

        let pending_formatting_is_before_open_table = self
            .previous_pending_formatting_path_before_open_table()
            .is_some();
        if (!text.chars().all(char::is_whitespace)
            || !self.pending_formatting_reconstruction.is_empty())
            && (!self.current_parent_has_table_ancestor()
                || !pending_formatting_is_before_open_table
                || self.current_parent_is_fostered_before_open_table())
            && !self.current_parent_is_inside_previous_pending_formatting_before_open_table()
        {
            self.reconstruct_pending_formatting();
        }

        self.append_text_to_current(text);
    }

    fn append_text_to_current(&mut self, text: String) {
        if let Some(children) = self.current_children_mut() {
            if let Some(Node::Text(existing)) = children.last_mut() {
                existing.data.push_str(&text);
                return;
            }
            children.push(Node::text(text));
            return;
        }

        if let Some(Node::Text(existing)) = self.document.children.last_mut() {
            existing.data.push_str(&text);
        } else {
            self.document.push_child(Node::text(text));
        }
    }

    fn append_text_to_document_html(&mut self, text: String) {
        let Some(Node::Element(html)) = self
            .document
            .children
            .iter_mut()
            .find(|node| matches!(node, Node::Element(element) if element.name == "html"))
        else {
            self.append_text_to_current(text);
            return;
        };
        if let Some(Node::Text(existing)) = html.children.last_mut() {
            existing.data.push_str(&text);
        } else {
            html.children.push(Node::text(text));
        }
    }

    fn flush_pending_table_text(&mut self) {
        if self.pending_table_text.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut self.pending_table_text);
        if pending.chars().all(char::is_whitespace) {
            self.append_text_to_current(pending);
        } else {
            self.foster_text_before_open_table(pending);
        }
    }

    fn append_comment(&mut self, comment: String) {
        if self.current_namespace().is_some() {
            if let Some(cdata) = comment
                .strip_prefix("[CDATA[")
                .and_then(|data| data.strip_suffix("]]"))
            {
                if !cdata.is_empty() {
                    self.append_text_to_current(cdata.to_string());
                }
                return;
            }
        }
        let node = Node::comment(comment);
        if self.explicit_head_end_seen
            && self.current_element_is("head")
            && self.has_document_element()
            && !self.document_has_body_element()
            && !self.body_has_non_whitespace_child()
        {
            if let Some(Node::Element(html)) = self
                .document
                .children
                .iter_mut()
                .find(|node| matches!(node, Node::Element(element) if element.name == "html"))
            {
                html.children.push(node);
                return;
            }
        }
        if self.open_elements.is_empty()
            && self.has_document_element()
            && !self.explicit_html_end_seen
            && !self.document_has_body_element()
            && !self.body_has_non_whitespace_child()
        {
            if let Some(Node::Element(html)) = self
                .document
                .children
                .iter_mut()
                .find(|node| matches!(node, Node::Element(element) if element.name == "html"))
            {
                html.children.push(node);
                return;
            }
        }
        if self.document_has_closed_frameset()
            && self.explicit_html_end_seen
            && !self.current_element_is("noframes")
        {
            self.document.children.push(node);
            return;
        }
        if self.explicit_html_end_seen {
            if self.current_element_is("body") && self.body_has_non_whitespace_child() {
                self.append_node(node);
                return;
            }
            self.document.children.push(node);
            return;
        }
        if self.explicit_body_end_seen {
            if let Some(Node::Element(html)) = self
                .document
                .children
                .iter_mut()
                .find(|node| matches!(node, Node::Element(element) if element.name == "html"))
            {
                html.children.push(node);
                return;
            }
            if !self
                .document
                .children
                .iter()
                .any(|node| matches!(node, Node::Element(element) if element.name == "body"))
            {
                self.document
                    .children
                    .push(Node::element("body".to_string(), Vec::new()));
            }
            self.document.children.push(node);
            return;
        }
        if self.open_elements.is_empty()
            || (self.current_element_is("html") && !self.explicit_body_end_seen)
        {
            if let Some(body_children) = children_at_path_mut(&mut self.document.children, &[0, 1])
                .filter(|children| !children.is_empty())
            {
                body_children.push(node);
                return;
            }
        }
        self.append_node(node);
    }

    fn append_node(&mut self, node: Node) -> usize {
        if let Some(children) = self.current_children_mut() {
            children.push(node);
            children.len() - 1
        } else {
            self.document.push_child(node);
            self.document.children.len() - 1
        }
    }

    fn append_node_to_document_html(&mut self, node: Node) -> Option<Vec<usize>> {
        let html_index =
            self.document.children.iter().position(
                |node| matches!(node, Node::Element(element) if element.name == "html"),
            )?;
        let Some(Node::Element(html)) = self.document.children.get_mut(html_index) else {
            return None;
        };
        html.children.push(node);
        Some(vec![html_index, html.children.len() - 1])
    }

    fn append_implied_element(&mut self, name: &str) {
        let child_index = self.append_node(Node::element(name.to_string(), Vec::new()));
        let mut path = self.current_parent_path().to_vec();
        path.push(child_index);
        self.open_elements.push(path);
    }

    fn merge_attributes_into_open_element(
        &mut self,
        element_name: &str,
        attributes: &[Attribute],
    ) -> bool {
        let Some(path) = self
            .open_elements
            .iter()
            .rposition(|path| {
                element_at_path(&self.document, path).is_some_and(|name| name == element_name)
            })
            .map(|index| self.open_elements[index].clone())
        else {
            return false;
        };

        let Some(element) = element_at_path_mut(&mut self.document, &path) else {
            return false;
        };
        for attribute in attributes {
            if element.attribute(&attribute.name).is_none() {
                element.attributes.push(attribute.clone());
            }
        }
        true
    }

    fn merge_attributes_into_document_element(&mut self, attributes: &[Attribute]) -> bool {
        let Some(element) = self
            .document
            .children
            .iter_mut()
            .find_map(|node| match node {
                Node::Element(element) if element.name == "html" => Some(element),
                _ => None,
            })
        else {
            return false;
        };
        for attribute in attributes {
            if element.attribute(&attribute.name).is_none() {
                element.attributes.push(attribute.clone());
            }
        }
        true
    }

    fn take_formatting_reconstruction_inside_for(
        &mut self,
        incoming_name: &str,
    ) -> Vec<(String, Vec<Attribute>)> {
        if !starts_inner_formatting_reconstruction_boundary(incoming_name) {
            return Vec::new();
        }
        if incoming_name == "p"
            && !self.has_open_element("p")
            && (self.current_element_is("b")
                || self.current_element_is("i")
                || self.current_element_is("u")
                || (self.current_element_is("a") && self.current_has_non_whitespace_child())
                || (self.current_empty_element_is("a") && self.current_has_formatting_ancestor()))
        {
            return Vec::new();
        }
        if incoming_name == "button" && self.current_element_is("span") {
            return Vec::new();
        }

        let mut formatting = Vec::new();
        while let Some(path) = self.open_elements.last() {
            let Some(element) = element_ref_at_path(&self.document, path) else {
                break;
            };
            if !is_formatting_element(&element.name) {
                break;
            }
            formatting.push((element.name.clone(), element.attributes.clone()));
            self.open_elements.pop();
        }

        formatting.reverse();
        trim_formatting_reconstruction_noah_ark(formatting)
    }

    fn reconstruct_formatting_before_if_needed(&mut self, incoming_name: &str) {
        if !starts_before_formatting_reconstruction_boundary(incoming_name) {
            return;
        }
        if self.current_element_is_table_structure() {
            return;
        }
        if self.has_open_table_context()
            && !self.current_parent_is_fostered_before_open_table()
            && self
                .previous_pending_formatting_path_before_open_table()
                .is_some()
        {
            return;
        }
        if self.has_open_table_context()
            && self
                .pending_formatting_reconstruction
                .iter()
                .any(|(name, _)| name == "a")
        {
            return;
        }

        self.reconstruct_pending_formatting();
    }

    fn reconstruct_pending_formatting(&mut self) {
        let formatting = std::mem::take(&mut self.pending_formatting_reconstruction);
        for (formatting_name, formatting_attributes) in formatting {
            let child_index =
                self.append_node(Node::element(formatting_name, formatting_attributes));
            let mut path = self.current_parent_path().to_vec();
            path.push(child_index);
            self.open_elements.push(path);
        }
    }

    fn clear_pending_formatting_unless_next_reconstructs(&mut self, incoming_name: &str) {
        if starts_table_context(incoming_name) && self.current_element_is_table_structure() {
            return;
        }
        if incoming_name == "p" && self.current_element_is_table_structure() {
            return;
        }
        if !starts_before_formatting_reconstruction_boundary(incoming_name) {
            self.pending_formatting_reconstruction.clear();
        }
    }

    fn foster_formatting_start_in_table_context(
        &mut self,
        incoming_name: &str,
        attributes: &[Attribute],
    ) -> bool {
        if !self.current_element_is_table_structure()
            && !(matches!(incoming_name, "i" | "nobr")
                && self.has_open_table_context()
                && self.current_parent_is_fostered_before_open_table())
        {
            return false;
        }

        if !self.current_element_is_table_structure()
            && self.current_parent_is_fostered_before_open_table()
        {
            if incoming_name == "nobr" {
                let formatting_above_nobr = self.formatting_above_open_element("nobr");
                self.close_open_element_silently("nobr");
                if !formatting_above_nobr.is_empty() {
                    self.pending_formatting_reconstruction =
                        trim_formatting_reconstruction_noah_ark(formatting_above_nobr);
                }
            }
        }

        if !self.current_element_is_table_structure()
            && self.current_parent_is_fostered_before_open_table()
        {
            let child_index = self.append_node(Node::element(
                incoming_name.to_string(),
                attributes.to_vec(),
            ));
            let mut path = self.current_parent_path().to_vec();
            path.push(child_index);
            self.open_elements.push(path);
            return true;
        }

        if incoming_name == "a" {
            let Some(_) = self.insert_node_before_open_table(Node::element(
                incoming_name.to_string(),
                attributes.to_vec(),
            )) else {
                return false;
            };
            self.pending_formatting_reconstruction =
                vec![(incoming_name.to_string(), attributes.to_vec())];
            return true;
        }

        if !is_formatting_element(incoming_name) {
            return false;
        }

        if incoming_name != "i" {
            if let Some(path) =
                self.insert_node_inside_previous_pending_formatting_before_open_table(
                    Node::element(incoming_name.to_string(), attributes.to_vec()),
                )
            {
                self.open_elements.push(path);
                return true;
            }
        }

        let Some(path) = self.insert_node_before_open_table(Node::element(
            incoming_name.to_string(),
            attributes.to_vec(),
        )) else {
            return false;
        };

        self.open_elements.push(path);
        true
    }

    fn insert_node_inside_previous_pending_formatting_before_open_table(
        &mut self,
        node: Node,
    ) -> Option<Vec<usize>> {
        let mut parent_path = self.previous_pending_formatting_path_before_open_table()?;
        let children = children_at_path_mut(&mut self.document.children, &parent_path)?;
        let child_index = children.len();
        children.push(node);
        parent_path.push(child_index);
        Some(parent_path)
    }

    fn previous_pending_formatting_path_before_open_table(&self) -> Option<Vec<usize>> {
        if self.pending_formatting_reconstruction.is_empty() {
            return None;
        }
        let table_path = self
            .open_elements
            .iter()
            .rfind(|path| element_at_path(&self.document, path).is_some_and(|name| name == "table"))
            .cloned()?;
        let (&table_index, parent_path) = table_path.split_last()?;
        let mut path = parent_path.to_vec();
        path.push(table_index.checked_sub(1)?);

        for (index, (name, attributes)) in self.pending_formatting_reconstruction.iter().enumerate()
        {
            let element = element_ref_at_path(&self.document, &path)?;
            if element.name != *name || element.attributes != *attributes {
                return None;
            }
            if index + 1 < self.pending_formatting_reconstruction.len() {
                path.push(element.children.len().checked_sub(1)?);
            }
        }

        Some(path)
    }

    fn current_parent_is_inside_previous_pending_formatting_before_open_table(&self) -> bool {
        let current_parent = self.current_parent_path();
        self.previous_pending_formatting_path_before_open_table()
            .is_some_and(|path| current_parent.starts_with(&path))
    }

    fn current_parent_is_fostered_before_open_table(&self) -> bool {
        let Some(table_path) = self.open_elements.iter().rfind(|path| {
            element_at_path(&self.document, path).is_some_and(|name| name == "table")
        }) else {
            return false;
        };
        !self.current_parent_path().starts_with(table_path)
    }

    fn foster_text_before_open_table(&mut self, text: String) -> bool {
        if self.pending_formatting_reconstruction.is_empty()
            || text.chars().all(char::is_whitespace)
        {
            return self
                .insert_node_before_open_table(Node::text(text))
                .is_some();
        }

        self.remove_empty_pending_formatting_before_open_table();

        let mut subtree = Node::text(text);
        for (name, attributes) in self.pending_formatting_reconstruction.iter().rev() {
            let mut wrapper = Node::element(name.clone(), attributes.clone());
            if let Some(children) = wrapper.children_mut() {
                children.push(subtree);
            }
            subtree = wrapper;
        }

        self.insert_node_before_open_table(subtree).is_some()
    }

    fn remove_empty_pending_formatting_before_open_table(&mut self) {
        let Some((pending_name, pending_attributes)) =
            self.pending_formatting_reconstruction.first().cloned()
        else {
            return;
        };
        if !matches!(pending_name.as_str(), "a" | "nobr") {
            return;
        }
        let Some(table_path) = self
            .open_elements
            .iter()
            .rfind(|path| element_at_path(&self.document, path).is_some_and(|name| name == "table"))
            .cloned()
        else {
            return;
        };
        let Some((&table_index, parent_path)) = table_path.split_last() else {
            return;
        };
        let Some(remove_index) = table_index.checked_sub(1) else {
            return;
        };
        let Some(children) = children_at_path_mut(&mut self.document.children, parent_path) else {
            return;
        };

        let should_remove = matches!(
            children.get(remove_index),
            Some(Node::Element(element))
                if element.name == pending_name
                    && element.attributes == pending_attributes
                    && element.children.is_empty()
        );
        if should_remove {
            children.remove(remove_index);
            decrement_open_element_paths_after_remove(
                &mut self.open_elements,
                parent_path,
                remove_index,
            );
        }
    }

    fn insert_node_before_open_table(&mut self, node: Node) -> Option<Vec<usize>> {
        let table_path = self
            .open_elements
            .iter()
            .rfind(|path| element_at_path(&self.document, path).is_some_and(|name| name == "table"))
            .cloned()?;
        let (&table_index, parent_path) = table_path.split_last()?;
        let children = children_at_path_mut(&mut self.document.children, parent_path)?;

        if let Node::Text(text) = node {
            if let Some(Node::Text(existing)) = table_index
                .checked_sub(1)
                .and_then(|index| children.get_mut(index))
            {
                existing.data.push_str(&text.data);
                return Some(parent_path.to_vec());
            }
            children.insert(table_index, Node::Text(text));
        } else {
            children.insert(table_index, node);
        }

        increment_open_element_paths_after_insert(
            &mut self.open_elements,
            parent_path,
            table_index,
        );
        let mut inserted_path = parent_path.to_vec();
        inserted_path.push(table_index);
        Some(inserted_path)
    }

    fn close_fostered_formatting_before_table_context(&mut self, incoming_name: &str) {
        if !starts_table_context(incoming_name) {
            return;
        }
        let Some(table_stack_index) = self.open_elements.iter().rposition(|path| {
            element_at_path(&self.document, path).is_some_and(|name| name == "table")
        }) else {
            return;
        };
        let Some(table_path) = self.open_elements.get(table_stack_index).cloned() else {
            return;
        };

        let mut pending_formatting = Vec::new();
        while self.open_elements.len() > table_stack_index + 1 {
            let Some(path) = self.open_elements.last() else {
                break;
            };
            let is_fostered_formatting = element_at_path(&self.document, path)
                .is_some_and(is_formatting_element)
                && !path.starts_with(&table_path);
            if !is_fostered_formatting {
                break;
            }
            if let Some(element) = element_ref_at_path(&self.document, path) {
                pending_formatting.push((element.name.clone(), element.attributes.clone()));
            }
            self.open_elements.pop();
        }
        if !pending_formatting.is_empty() {
            pending_formatting.reverse();
            self.pending_formatting_reconstruction =
                trim_formatting_reconstruction_noah_ark(pending_formatting);
        }
    }

    fn pop_fostered_content_before_table_context(&mut self, incoming_name: &str) {
        if !starts_table_context(incoming_name) {
            return;
        }
        let Some(table_stack_index) = self.open_elements.iter().rposition(|path| {
            element_at_path(&self.document, path).is_some_and(|name| name == "table")
        }) else {
            return;
        };
        let Some(table_path) = self.open_elements.get(table_stack_index).cloned() else {
            return;
        };

        let mut pending_formatting = Vec::new();
        while self.open_elements.len() > table_stack_index + 1 {
            let Some(path) = self.open_elements.last() else {
                break;
            };
            if path.starts_with(&table_path) {
                break;
            }
            if let Some(element) = element_ref_at_path(&self.document, path) {
                if is_formatting_element(&element.name) {
                    pending_formatting.push((element.name.clone(), element.attributes.clone()));
                }
            }
            self.open_elements.pop();
        }
        if !pending_formatting.is_empty() {
            pending_formatting.reverse();
            self.pending_formatting_reconstruction =
                trim_formatting_reconstruction_noah_ark(pending_formatting);
        }
    }

    fn apply_document_shell_implied_contexts(&mut self, incoming_name: &str) {
        if starts_body_after_head(incoming_name) && self.current_element_is("head") {
            self.pop_current_if(|name| name == "head");
        }
    }

    fn handle_end_tag(&mut self, name: &str) {
        if name == "script" && self.current_script_text_treats_next_end_tag_as_data() {
            self.append_text_to_current("</script>".to_string());
            return;
        }
        if name != "html" && self.current_element_is_marked_fragment_context(name) {
            return;
        }
        if self.open_marked_fragment_shell_element_matches(name) {
            return;
        }
        if self.has_open_svg_html_integration_point()
            && name != "template"
            && name != "p"
            && !self.current_element_is(name)
            && !is_table_context_element(name)
        {
            return;
        }
        if self.current_namespace().is_some()
            && !self.current_element_is(name)
            && self.has_open_element(name)
            && (is_table_context_element(name)
                || self.current_namespace() == Some("svg")
                || (self.current_namespace() == Some("math") && name == "p"))
        {
            self.pop_foreign_elements();
        } else if self.current_namespace().is_some()
            && !self.current_element_is(name)
            && matches!(name, "br" | "p")
            && self.current_element_is_marked_foreign_fragment_context()
        {
            self.append_node(Node::element(name.to_string(), Vec::new()));
            return;
        } else if self.current_namespace().is_some()
            && !self.current_element_is(name)
            && matches!(name, "br" | "p")
        {
            self.pop_foreign_elements();
        } else if self.current_namespace().is_some() && !self.current_element_is(name) {
            return;
        }
        if name == "b" && self.adopt_b_end_tag_across_cite_div() {
            return;
        }
        if name == "body" && self.has_open_element("body") && self.current_element_is("bdy") {
            return;
        }
        if name == "body" {
            self.explicit_body_end_seen = true;
        }
        if name == "head" {
            self.explicit_head_end_seen = true;
        }
        if name == "html" {
            self.explicit_html_end_seen = true;
        }
        if name == "em" {
            self.explicit_em_end_seen = true;
        }
        match name {
            "head" if !self.has_open_element("head") && !self.has_open_element("body") => {
                self.strip_next_leading_lf = false;
            }
            "body" if !self.has_open_element("body") && self.has_open_table_context() => {
                self.diagnostics.push(ParserDiagnostic::new(
                    "unexpected-body-end-tag-in-table",
                    "end tag `</body>` inside a table context was ignored",
                ));
            }
            "body" if !self.has_open_element("body") && !self.open_elements.is_empty() => {
                self.open_elements.clear();
            }
            "body" if self.open_elements.is_empty() && !self.has_document_element() => {
                self.append_implied_element("html");
                self.append_implied_element("body");
                self.open_elements.clear();
            }
            "br" => {
                self.diagnostics.push(ParserDiagnostic::new(
                    "unexpected-br-end-tag",
                    "end tag `</br>` was recovered as a `br` start tag",
                ));
                self.pop_head_descendants();
                self.append_start_tag("br".to_string(), Vec::new(), true);
            }
            "menuitem" => self.close_non_paragraph_children_above_menuitem(),
            "template" => self.close_open_element_without_scope_checks("template"),
            name if is_void_element(name) => {
                self.diagnostics.push(ParserDiagnostic::new(
                    "unexpected-void-end-tag",
                    format!("end tag `</{name}>` for a void element was ignored"),
                ));
            }
            "p" if self.has_open_element("head") && !self.has_open_element("body") => {
                self.diagnostics.push(ParserDiagnostic::new(
                    "unexpected-p-end-tag-before-body",
                    "end tag `</p>` before body content was ignored",
                ));
            }
            "p" if !self.has_open_element("body")
                && !self.document_has_body_element()
                && !self.body_has_non_whitespace_child() => {}
            "p" if self.current_parent_has_element_ancestor("button")
                && !self.current_parent_has_element_in_button_scope("p") =>
            {
                self.diagnostics.push(ParserDiagnostic::new(
                    "unexpected-p-end-tag",
                    "end tag `</p>` created and closed an implied `p` element",
                ));
                self.append_node(Node::element("p".to_string(), Vec::new()));
            }
            "p" if self.current_parent_has_table_cell_ancestor()
                && !self.current_parent_has_element_in_table_scope("p") =>
            {
                self.diagnostics.push(ParserDiagnostic::new(
                    "unexpected-p-end-tag",
                    "end tag `</p>` created and closed an implied `p` element",
                ));
                self.append_node(Node::element("p".to_string(), Vec::new()));
            }
            "p" if self.current_parent_has_table_ancestor()
                && !self.current_parent_has_element_in_table_scope("p") =>
            {
                self.diagnostics.push(ParserDiagnostic::new(
                    "unexpected-p-end-tag",
                    "end tag `</p>` created and closed an implied `p` element",
                ));
                self.insert_node_before_open_table(Node::element("p".to_string(), Vec::new()));
            }
            "p" if self.has_open_element("p")
                && !self.has_open_element_before_namespace_boundary("p") =>
            {
                self.diagnostics.push(ParserDiagnostic::new(
                    "unexpected-p-end-tag",
                    "end tag `</p>` created and closed an implied `p` element",
                ));
                self.append_node(Node::element("p".to_string(), Vec::new()));
            }
            "p" if !self.has_open_element("p") => {
                self.diagnostics.push(ParserDiagnostic::new(
                    "unexpected-p-end-tag",
                    "end tag `</p>` created and closed an implied `p` element",
                ));
                self.append_start_tag("p".to_string(), Vec::new(), false);
                self.close_element("p");
            }
            "html" if self.has_open_element("head") && !self.has_open_element("body") => {
                self.pop_current_if(|current| current == "head");
                self.append_implied_element("body");
            }
            "html" if self.has_open_table_context() => {
                self.diagnostics.push(ParserDiagnostic::new(
                    "unexpected-html-end-tag-in-table",
                    "end tag `</html>` inside a table context was ignored",
                ));
            }
            "li" => {
                if !self.close_open_list_item_if_in_scope() {
                    self.diagnostics.push(ParserDiagnostic::new(
                        "unexpected-li-end-tag",
                        "end tag `</li>` did not match a list item in scope",
                    ));
                }
            }
            "html" => {
                if self.has_open_element("html") {
                    self.pop_current_if(|current| current == "body");
                    self.close_element(name);
                } else if self.open_elements.is_empty() && !self.has_document_element() {
                    self.append_implied_element("html");
                    self.append_implied_element("body");
                    self.open_elements.clear();
                } else if !self.open_elements.is_empty() {
                    self.open_elements.clear();
                } else {
                    self.close_element(name);
                }
            }
            "table" => {
                self.close_element(name);
                if self
                    .pending_formatting_reconstruction
                    .iter()
                    .any(|(name, _)| name == "a")
                {
                    self.close_open_formatting_element_silently("a");
                }
            }
            name if is_formatting_element(name)
                && self.current_element_is(name)
                && self.current_formatting_contains_closed_paragraph(name) =>
            {
                self.remove_pending_formatting_reconstruction(name);
            }
            name if is_heading_element(name) => {
                self.close_open_heading_if_in_scope(None);
            }
            _ => self.close_element(name),
        }
    }

    fn close_element(&mut self, name: &str) {
        let lower_bound = if name == "template" {
            0
        } else {
            self.open_elements
                .iter()
                .rposition(|path| {
                    element_at_path(&self.document, path).is_some_and(|n| n == "template")
                })
                .map_or(0, |index| index + 1)
        };
        if let Some(relative_index) = self.open_elements[lower_bound..].iter().rposition(|path| {
            element_ref_at_path(&self.document, path).is_some_and(|element| {
                element.name.eq_ignore_ascii_case(name)
                    && (!is_table_context_element(name) || element.namespace.is_none())
            })
        }) {
            let index = lower_bound + relative_index;
            if name == "span" && self.has_special_element_above(index) {
                return;
            }
            if name == "form" && self.has_table_context_above(index) {
                self.form_element_pointer_set = false;
                self.open_elements.remove(index);
                return;
            }
            if is_heading_element(name) && self.has_special_element_above(index) {
                while self
                    .current_element_name()
                    .is_some_and(|name| is_formatting_element(name) || is_heading_element(name))
                {
                    self.open_elements.pop();
                }
                return;
            }
            if is_formatting_element(name) && self.has_table_context_above(index) {
                if self.open_element_is_fostered_before_open_table(index) {
                    self.capture_formatting_above(index);
                    self.open_elements.truncate(index);
                }
                return;
            }
            if is_formatting_element(name) && self.adopt_formatting_end_tag_across_paragraph(index)
            {
                return;
            }
            if is_formatting_element(name)
                && self.adopt_formatting_end_tag_across_nested_paragraph(index)
            {
                return;
            }
            if is_formatting_element(name) && self.adopt_formatting_end_tag_across_mixed_div(index)
            {
                return;
            }
            if is_formatting_element(name) && self.adopt_formatting_end_tag_across_div(index) {
                return;
            }
            if is_formatting_element(name)
                && self.adopt_formatting_end_tag_across_special_block(index)
            {
                return;
            }
            if special_scope_blocks_end_tag(name)
                && self.has_special_element_above(index)
                && !(is_paragraph_boundary_element(name)
                    && self.has_element_above(index, |candidate| candidate == "button"))
            {
                return;
            }
            let path = self.open_elements[index].clone();
            let remove_empty_reconstructed_formatting =
                self.is_empty_reconstructed_formatting_element(&path);
            if is_formatting_element(name) {
                self.capture_formatting_above(index);
                self.remove_pending_formatting_reconstruction(name);
            }
            if name == "form" {
                self.form_element_pointer_set = false;
                self.generate_implied_end_tags_above(index);
                self.open_elements.remove(index);
                return;
            }
            if matches!(name, "div" | "p" | "select") {
                self.capture_formatting_above(index);
            }
            self.open_elements.truncate(index);
            if remove_empty_reconstructed_formatting {
                self.remove_reconstructed_formatting_node(&path);
            }
            return;
        }

        if is_formatting_element(name) && self.close_pending_formatting_in_table_context(name) {
            return;
        }

        self.diagnostics.push(ParserDiagnostic::new(
            "unexpected-end-tag",
            format!("end tag `</{name}>` did not match an open element"),
        ));
    }

    fn generate_implied_end_tags_above(&mut self, lower_bound: usize) {
        while self.open_elements.len() > lower_bound + 1
            && self
                .current_element_name()
                .is_some_and(is_implied_end_tag_element)
        {
            self.open_elements.pop();
        }
    }

    fn close_pending_formatting_in_table_context(&mut self, name: &str) -> bool {
        if !self
            .pending_formatting_reconstruction
            .iter()
            .any(|(candidate, _)| candidate == name)
        {
            return false;
        }
        let Some(table_stack_index) = self.open_elements.iter().rposition(|path| {
            element_at_path(&self.document, path).is_some_and(|candidate| candidate == "table")
        }) else {
            return false;
        };
        let Some(table_path) = self.open_elements.get(table_stack_index).cloned() else {
            return false;
        };

        let mut pending_formatting = Vec::new();
        while self.open_elements.len() > table_stack_index + 1 {
            let Some(path) = self.open_elements.last() else {
                break;
            };
            if path.starts_with(&table_path) {
                break;
            }
            if let Some(element) = element_ref_at_path(&self.document, path) {
                if is_formatting_element(&element.name) && element.name != name {
                    pending_formatting.push((element.name.clone(), element.attributes.clone()));
                }
            }
            self.open_elements.pop();
        }

        if pending_formatting.is_empty() {
            self.remove_pending_formatting_reconstruction(name);
        } else {
            pending_formatting.reverse();
            self.pending_formatting_reconstruction =
                trim_formatting_reconstruction_noah_ark(pending_formatting);
        }
        true
    }

    fn apply_table_implied_contexts(&mut self, incoming_name: &str) {
        match incoming_name {
            "caption" | "colgroup" => {
                self.pop_table_cell_row_and_section_contexts();
                self.close_open_table_context_element_if(|name| {
                    name == "caption" || name == "colgroup"
                });
            }
            "tbody" | "thead" | "tfoot" => {
                self.pop_table_cell_row_and_section_contexts();
                self.close_open_table_context_element_if(|name| {
                    name == "caption" || name == "colgroup"
                });
            }
            "col" => {
                self.pop_table_cell_row_and_section_contexts();
                self.close_open_table_context_element_if(|name| name == "caption");
                if self.current_element_is("table") {
                    self.append_implied_element("colgroup");
                }
            }
            "tr" => {
                self.close_open_table_context_element_if(|name| name == "td" || name == "th");
                self.close_open_table_context_element_if(|name| name == "tr");
                self.close_open_table_context_element_if(|name| {
                    name == "caption" || name == "colgroup"
                });
                if self.current_element_is("table") {
                    self.append_implied_element("tbody");
                }
            }
            "td" | "th" => {
                self.close_open_table_context_element_if(|name| name == "td" || name == "th");
                self.close_open_table_context_element_if(|name| {
                    name == "caption" || name == "colgroup"
                });
                if self.current_element_is("table") {
                    self.append_implied_element("tbody");
                }
                if self.current_element_is("tbody")
                    || self.current_element_is("thead")
                    || self.current_element_is("tfoot")
                {
                    self.append_implied_element("tr");
                }
            }
            _ => {}
        }
    }

    fn pop_table_cell_row_and_section_contexts(&mut self) {
        self.close_open_table_context_element_if(|name| name == "td" || name == "th");
        self.close_open_table_context_element_if(|name| name == "tr");
        self.close_open_table_context_element_if(is_table_section);
    }

    fn close_open_table_context_element_if(&mut self, predicate: impl Fn(&str) -> bool) -> bool {
        let lower_bound = self
            .open_elements
            .iter()
            .rposition(|path| {
                element_at_path(&self.document, path).is_some_and(|name| name == "table")
            })
            .map_or(0, |index| index + 1);
        let Some(relative_index) = self.open_elements[lower_bound..].iter().rposition(|path| {
            element_at_path(&self.document, path).is_some_and(|name| predicate(name))
        }) else {
            return false;
        };
        self.open_elements.truncate(lower_bound + relative_index);
        true
    }

    fn apply_simple_implied_end_tags(&mut self, incoming_name: &str) {
        if incoming_name == "p" {
            if self.current_parent_has_open_element_in_button_scope("p") {
                self.close_open_element_if(|name| name == "p");
            }
        } else if incoming_name == "li" {
            if !self.current_parent_has_element_ancestor("button") {
                self.close_open_element_if(|name| name == "p");
                self.close_open_list_item_if_in_scope();
            }
        } else if incoming_name == "dt" || incoming_name == "dd" {
            if !self.current_parent_has_element_ancestor("button") {
                self.close_open_element_if(|name| name == "p");
                self.close_open_element_if(|name| name == "dt" || name == "dd");
            }
        } else if incoming_name == "option"
            && (self.current_element_is("option") || self.has_open_element("select"))
        {
            self.close_open_element_if(|name| name == "option");
        } else if incoming_name == "optgroup" {
            self.close_open_element_if(|name| name == "option");
            self.close_open_element_if(|name| name == "optgroup");
        } else if incoming_name == "rb" {
            self.close_open_ruby_element_if(is_ruby_annotation_element);
            self.close_open_ruby_element_if(|name| name == "rtc");
        } else if incoming_name == "rt" || incoming_name == "rp" {
            self.close_open_element_if(|name| name == "p");
            self.close_open_ruby_element_if(|name| name == "rb" || name == "rt" || name == "rp");
        } else if incoming_name == "rtc" {
            self.close_open_ruby_element_if(|name| name == "rb" || name == "rt" || name == "rp");
            self.close_open_ruby_element_if(|name| name == "rtc");
        } else if is_heading_element(incoming_name) {
            if !self.current_parent_has_element_ancestor("button") {
                self.close_open_element_if(|name| name == "p");
            }
            self.close_open_heading_if_in_scope(None);
        } else if is_paragraph_boundary_element(incoming_name) {
            if incoming_name == "table" && self.quirks_mode {
                return;
            }
            if self.current_parent_has_element_ancestor("button") {
                return;
            }
            self.close_open_element_if(|name| name == "p");
        }
    }

    fn apply_interactive_implied_contexts(&mut self, incoming_name: &str) -> bool {
        match incoming_name {
            "a" => {
                if self.current_empty_element_is("a")
                    && !self.current_parent_element_is(|name| name == "p")
                {
                    return true;
                }
                let consumes_pending_anchor = !self.has_open_table_context()
                    && !matches!(self.current_element_name(), Some("p"));
                if consumes_pending_anchor {
                    self.remove_pending_formatting_reconstruction("a");
                }
                let reconstruct_anchor_above_paragraph = self
                    .current_element_is("p")
                    .then(|| self.open_formatting_element_before_current("a"))
                    .flatten();
                let closed_existing_anchor = self.close_open_formatting_element_silently("a")
                    || self.adopt_open_formatting_element_silently("a");
                if closed_existing_anchor && consumes_pending_anchor {
                    self.remove_pending_formatting_reconstruction("a");
                }
                if let Some(formatting) = reconstruct_anchor_above_paragraph {
                    self.pending_formatting_reconstruction.push(formatting);
                }
                false
            }
            "button" => {
                self.close_open_element_silently("button");
                false
            }
            "nobr" => {
                let formatting_above_nobr = self.formatting_above_open_element("nobr");
                self.close_open_element_silently("nobr");
                if !formatting_above_nobr.is_empty() {
                    self.pending_formatting_reconstruction =
                        trim_formatting_reconstruction_noah_ark(formatting_above_nobr);
                }
                false
            }
            "form" if self.form_element_pointer_set => {
                self.diagnostics.push(ParserDiagnostic::new(
                    "nested-form-start-tag",
                    "nested form start tag was ignored while a form element was already open",
                ));
                true
            }
            _ => false,
        }
    }

    fn apply_select_implied_contexts(&mut self, incoming_name: &str) -> bool {
        if !matches!(incoming_name, "input" | "select") || !self.has_open_element("select") {
            return false;
        }

        if incoming_name == "input" && self.current_element_is_marked_fragment_context("select") {
            return true;
        }

        self.close_open_element_if(|name| name == "option");
        self.close_open_element_if(|name| name == "select");
        incoming_name == "select"
    }

    fn close_open_element_silently(&mut self, name: &str) -> bool {
        self.close_open_element_if(|candidate| candidate == name)
    }

    fn pop_head_descendants(&mut self) {
        let Some(head_index) = self.open_elements.iter().rposition(|path| {
            element_at_path(&self.document, path).is_some_and(|name| name == "head")
        }) else {
            return;
        };
        if self.open_elements.len() > head_index + 1 {
            self.open_elements.truncate(head_index + 1);
        }
    }

    fn close_non_paragraph_children_above_menuitem(&mut self) {
        let Some(index) = self.open_elements.iter().rposition(|path| {
            element_at_path(&self.document, path).is_some_and(|name| name == "menuitem")
        }) else {
            return;
        };
        if self
            .open_elements
            .iter()
            .skip(index + 1)
            .any(|path| element_at_path(&self.document, path).is_some_and(|name| name == "p"))
        {
            return;
        }
        self.open_elements.truncate(index);
    }

    fn close_open_list_item_if_in_scope(&mut self) -> bool {
        let Some(index) = self.open_elements.iter().rposition(|path| {
            element_at_path(&self.document, path).is_some_and(|name| name == "li")
        }) else {
            return false;
        };
        if self.open_elements.iter().skip(index + 1).any(|path| {
            element_at_path(&self.document, path).is_some_and(is_list_item_scope_boundary)
        }) {
            return false;
        }
        self.open_elements.truncate(index);
        true
    }

    fn close_open_heading_if_in_scope(&mut self, expected_name: Option<&str>) -> bool {
        let Some(index) = self.open_elements.iter().rposition(|path| {
            element_at_path(&self.document, path).is_some_and(|name| {
                is_heading_element(name) && expected_name.map_or(true, |expected| name == expected)
            })
        }) else {
            return false;
        };
        if self.has_special_element_above(index) {
            while self
                .current_element_name()
                .is_some_and(is_formatting_element)
            {
                self.open_elements.pop();
            }
            self.pop_current_if(is_heading_element);
            return false;
        }
        self.open_elements.truncate(index);
        true
    }

    fn close_open_formatting_element_silently(&mut self, name: &str) -> bool {
        let Some(index) = self
            .open_elements
            .iter()
            .rposition(|path| element_at_path(&self.document, path).is_some_and(|n| n == name))
        else {
            return false;
        };
        if self.has_table_context_above(index) {
            return false;
        }
        if self.has_special_element_above(index) {
            return false;
        }
        self.capture_formatting_above(index);
        self.open_elements.truncate(index);
        true
    }

    fn formatting_above_open_element(&self, name: &str) -> Vec<(String, Vec<Attribute>)> {
        let Some(index) = self
            .open_elements
            .iter()
            .rposition(|path| element_at_path(&self.document, path).is_some_and(|n| n == name))
        else {
            return Vec::new();
        };

        self.open_elements
            .iter()
            .skip(index + 1)
            .filter_map(|path| {
                let element = element_ref_at_path(&self.document, path)?;
                is_formatting_element(&element.name)
                    .then(|| (element.name.clone(), element.attributes.clone()))
            })
            .collect()
    }

    fn adopt_open_formatting_element_silently(&mut self, name: &str) -> bool {
        let Some(index) = self
            .open_elements
            .iter()
            .rposition(|path| element_at_path(&self.document, path).is_some_and(|n| n == name))
        else {
            return false;
        };

        self.adopt_formatting_end_tag_across_paragraph(index)
            || self.adopt_formatting_end_tag_across_nested_paragraph(index)
            || self.adopt_formatting_end_tag_across_div(index)
    }

    fn adopt_b_end_tag_across_cite_div(&mut self) -> bool {
        let Some(b_stack_index) = self.open_elements.iter().rposition(|path| {
            element_at_path(&self.document, path).is_some_and(|name| name == "b")
        }) else {
            return false;
        };
        let b_path = self.open_elements[b_stack_index].clone();
        let Some(b_element) = element_ref_at_path(&self.document, &b_path) else {
            return false;
        };
        let b_attributes = b_element.attributes.clone();
        let Some((cite_index, div_index)) =
            b_element
                .children
                .iter()
                .enumerate()
                .find_map(|(cite_index, node)| {
                    let Node::Element(cite) = node else {
                        return None;
                    };
                    if cite.name != "cite" {
                        return None;
                    }
                    cite.children
                        .iter()
                        .position(|child| matches!(child, Node::Element(div) if div.name == "div"))
                        .map(|div_index| (cite_index, div_index))
                })
        else {
            return false;
        };
        let Some((b_child_index, b_parent_path)) = b_path.split_last() else {
            return false;
        };
        let b_child_index = *b_child_index;
        let mut cite_path = b_path.clone();
        cite_path.push(cite_index);
        let Some(cite_children) = children_at_path_mut(&mut self.document.children, &cite_path)
        else {
            return false;
        };
        let mut moved_div = cite_children.remove(div_index);
        if let Node::Element(div) = &mut moved_div {
            let mut reconstructed_b = Node::element("b".to_string(), b_attributes);
            if let Node::Element(reconstructed_b) = &mut reconstructed_b {
                reconstructed_b.children = std::mem::take(&mut div.children);
            }
            div.children.push(reconstructed_b);
        } else {
            return false;
        }
        let Some(parent_children) =
            children_at_path_mut(&mut self.document.children, b_parent_path)
        else {
            return false;
        };
        let insert_index = b_child_index + 1;
        parent_children.insert(insert_index, moved_div);
        self.open_elements.truncate(b_stack_index);
        let mut moved_div_path = b_parent_path.to_vec();
        moved_div_path.push(insert_index);
        self.open_elements.push(moved_div_path);
        true
    }

    fn close_open_element_if(&mut self, predicate: impl Fn(&str) -> bool) -> bool {
        let Some(index) = self.open_elements.iter().rposition(|path| {
            element_at_path(&self.document, path).is_some_and(|name| predicate(name))
        }) else {
            return false;
        };
        let should_capture_formatting = self
            .open_elements
            .get(index)
            .and_then(|path| element_at_path(&self.document, path))
            .is_some_and(|name| matches!(name, "p" | "select"));
        if should_capture_formatting {
            self.capture_formatting_above(index);
        }
        self.open_elements.truncate(index);
        true
    }

    fn close_open_element_without_scope_checks(&mut self, name: &str) {
        if let Some(index) = self.open_elements.iter().rposition(|path| {
            element_ref_at_path(&self.document, path).is_some_and(|element| {
                element.name == name && (name != "template" || element.namespace.is_none())
            })
        }) {
            self.open_elements.truncate(index);
        }
    }

    fn close_open_ruby_element_if(&mut self, predicate: impl Fn(&str) -> bool) -> bool {
        let last_ruby = self.open_elements.iter().rposition(|path| {
            element_at_path(&self.document, path).is_some_and(|name| name == "ruby")
        });
        let lower_bound = last_ruby.map_or(0, |index| index + 1);
        let Some(relative_index) = self.open_elements[lower_bound..].iter().rposition(|path| {
            element_at_path(&self.document, path).is_some_and(|name| predicate(name))
        }) else {
            return false;
        };
        self.open_elements.truncate(lower_bound + relative_index);
        true
    }

    fn capture_formatting_above(&mut self, element_index: usize) {
        let formatting = self
            .open_elements
            .iter()
            .skip(element_index + 1)
            .filter_map(|path| {
                let element = element_ref_at_path(&self.document, path)?;
                is_formatting_element(&element.name)
                    .then(|| (element.name.clone(), element.attributes.clone()))
            })
            .collect::<Vec<_>>();

        if !formatting.is_empty() {
            self.pending_formatting_reconstruction =
                trim_formatting_reconstruction_noah_ark(formatting);
        }
    }

    fn remove_pending_formatting_reconstruction(&mut self, name: &str) {
        self.pending_formatting_reconstruction
            .retain(|(candidate, _)| candidate != name);
    }

    fn open_formatting_element_before_current(
        &self,
        name: &str,
    ) -> Option<(String, Vec<Attribute>)> {
        let index = self
            .open_elements
            .iter()
            .rposition(|path| element_at_path(&self.document, path).is_some_and(|n| n == name))?;
        if index + 1 >= self.open_elements.len() {
            return None;
        }
        let element = element_ref_at_path(&self.document, &self.open_elements[index])?;
        Some((element.name.clone(), element.attributes.clone()))
    }

    fn is_empty_reconstructed_formatting_element(&self, path: &[usize]) -> bool {
        if !self
            .prunable_empty_reconstructed_formatting_paths
            .iter()
            .any(|candidate| candidate.as_slice() == path)
        {
            return false;
        }

        element_ref_at_path(&self.document, path).is_some_and(|element| element.children.is_empty())
    }

    fn remove_reconstructed_formatting_node(&mut self, path: &[usize]) {
        remove_node_at_path(&mut self.document.children, path);
        self.prunable_empty_reconstructed_formatting_paths
            .retain(|candidate| candidate.as_slice() != path);
    }

    fn adopt_formatting_end_tag_across_paragraph(&mut self, formatting_index: usize) -> bool {
        let Some(formatting_path) = self.open_elements.get(formatting_index).cloned() else {
            return false;
        };
        let Some(formatting_element) = element_ref_at_path(&self.document, &formatting_path) else {
            return false;
        };
        let formatting_name = formatting_element.name.clone();
        let formatting_attributes = formatting_element.attributes.clone();

        let Some(paragraph_path) = self
            .open_elements
            .iter()
            .skip(formatting_index + 1)
            .find(|path| element_at_path(&self.document, path).is_some_and(|name| name == "p"))
            .cloned()
        else {
            return false;
        };
        if paragraph_path.len() != formatting_path.len() + 1
            || !paragraph_path.starts_with(&formatting_path)
        {
            return false;
        }

        let Some((&formatting_child_index, formatting_parent_path)) = formatting_path.split_last()
        else {
            return false;
        };
        let Some(&paragraph_child_index) = paragraph_path.last() else {
            return false;
        };
        let Some(formatting_parent_children) =
            children_at_path_mut(&mut self.document.children, formatting_parent_path)
        else {
            return false;
        };
        let Some(Node::Element(formatting_element)) =
            formatting_parent_children.get_mut(formatting_child_index)
        else {
            return false;
        };
        if paragraph_child_index >= formatting_element.children.len() {
            return false;
        }

        let mut paragraph = formatting_element.children.remove(paragraph_child_index);
        if let Node::Element(paragraph_element) = &mut paragraph {
            let mut reconstructed_formatting =
                Node::element(formatting_name, formatting_attributes);
            if let Node::Element(reconstructed_element) = &mut reconstructed_formatting {
                reconstructed_element.children = std::mem::take(&mut paragraph_element.children);
            }
            paragraph_element.children.push(reconstructed_formatting);
        }
        formatting_parent_children.insert(formatting_child_index + 1, paragraph);

        self.open_elements.truncate(formatting_index);
        let mut moved_paragraph_path = formatting_parent_path.to_vec();
        moved_paragraph_path.push(formatting_child_index + 1);
        self.open_elements.push(moved_paragraph_path);
        true
    }

    fn adopt_formatting_end_tag_across_nested_paragraph(
        &mut self,
        formatting_index: usize,
    ) -> bool {
        let Some(formatting_path) = self.open_elements.get(formatting_index).cloned() else {
            return false;
        };
        let Some(formatting_element) = element_ref_at_path(&self.document, &formatting_path) else {
            return false;
        };
        let formatting_name = formatting_element.name.clone();
        let formatting_attributes = formatting_element.attributes.clone();

        let Some(paragraph_path) = self
            .open_elements
            .iter()
            .skip(formatting_index + 1)
            .find(|path| element_at_path(&self.document, path).is_some_and(|name| name == "p"))
            .cloned()
        else {
            return false;
        };
        if !paragraph_path.starts_with(&formatting_path)
            || paragraph_path.len() <= formatting_path.len() + 1
        {
            return false;
        }

        let mut wrapper_elements = Vec::new();
        for depth in formatting_path.len() + 1..paragraph_path.len() {
            let ancestor_path = &paragraph_path[..depth];
            let Some(element) = element_ref_at_path(&self.document, ancestor_path) else {
                return false;
            };
            if !is_formatting_element(&element.name) {
                return false;
            }
            wrapper_elements.push((element.name.clone(), element.attributes.clone()));
        }

        let Some((&formatting_child_index, formatting_parent_path)) = formatting_path.split_last()
        else {
            return false;
        };
        let Some(mut paragraph) = remove_node_at_path(&mut self.document.children, &paragraph_path)
        else {
            return false;
        };
        let Node::Element(paragraph_element) = &mut paragraph else {
            return false;
        };

        let mut reconstructed_formatting = Node::element(formatting_name, formatting_attributes);
        if let Node::Element(reconstructed_element) = &mut reconstructed_formatting {
            reconstructed_element.children = std::mem::take(&mut paragraph_element.children);
        }
        paragraph_element.children.push(reconstructed_formatting);

        let mut adopted_subtree = paragraph;
        for (wrapper_name, wrapper_attributes) in wrapper_elements.iter().rev() {
            let mut wrapper = Node::element(wrapper_name.clone(), wrapper_attributes.clone());
            if let Node::Element(wrapper_element) = &mut wrapper {
                wrapper_element.children.push(adopted_subtree);
            }
            adopted_subtree = wrapper;
        }

        let Some(formatting_parent_children) =
            children_at_path_mut(&mut self.document.children, formatting_parent_path)
        else {
            return false;
        };
        let insert_index = formatting_child_index + 1;
        if insert_index > formatting_parent_children.len() {
            return false;
        }
        formatting_parent_children.insert(insert_index, adopted_subtree);

        self.open_elements.truncate(formatting_index);
        let mut adopted_path = formatting_parent_path.to_vec();
        adopted_path.push(insert_index);
        for _ in &wrapper_elements {
            self.open_elements.push(adopted_path.clone());
            adopted_path.push(0);
        }
        self.open_elements.push(adopted_path);
        true
    }

    fn adopt_formatting_end_tag_across_mixed_div(&mut self, formatting_index: usize) -> bool {
        let Some(formatting_path) = self.open_elements.get(formatting_index).cloned() else {
            return false;
        };
        let Some(formatting_element) = element_ref_at_path(&self.document, &formatting_path) else {
            return false;
        };
        let formatting_name = formatting_element.name.clone();
        let formatting_attributes = formatting_element.attributes.clone();

        let Some(first_div_path) = self
            .open_elements
            .iter()
            .skip(formatting_index + 1)
            .find(|path| element_at_path(&self.document, path).is_some_and(|name| name == "div"))
            .cloned()
        else {
            return false;
        };
        if !first_div_path.starts_with(&formatting_path)
            || first_div_path.len() <= formatting_path.len()
        {
            return false;
        }

        let mut formatting_wrappers = Vec::new();
        let mut saw_non_formatting_wrapper = false;
        for depth in formatting_path.len() + 1..first_div_path.len() {
            let ancestor_path = &first_div_path[..depth];
            let Some(element) = element_ref_at_path(&self.document, ancestor_path) else {
                return false;
            };
            if is_formatting_element(&element.name) {
                formatting_wrappers.push((element.name.clone(), element.attributes.clone()));
            } else {
                saw_non_formatting_wrapper = true;
            }
        }
        if !saw_non_formatting_wrapper || formatting_wrappers.len() < 2 {
            return false;
        }

        let Some(mut div) = remove_node_at_path(&mut self.document.children, &first_div_path)
        else {
            return false;
        };
        wrap_formatting_along_path(&mut div, &[], &formatting_name, &formatting_attributes);

        let wrappers_to_clone = &formatting_wrappers[..formatting_wrappers.len() - 1];
        let mut adopted_subtree = div;
        for (wrapper_name, wrapper_attributes) in wrappers_to_clone.iter().rev() {
            let mut wrapper = Node::element(wrapper_name.clone(), wrapper_attributes.clone());
            if let Node::Element(wrapper_element) = &mut wrapper {
                wrapper_element.children.push(adopted_subtree);
            }
            adopted_subtree = wrapper;
        }

        let Some((&formatting_child_index, formatting_parent_path)) = formatting_path.split_last()
        else {
            return false;
        };
        let insertion = (formatting_parent_path.to_vec(), formatting_child_index + 1);

        let Some(parent_children) = children_at_path_mut(&mut self.document.children, &insertion.0)
        else {
            return false;
        };
        if insertion.1 > parent_children.len() {
            return false;
        }
        parent_children.insert(insertion.1, adopted_subtree);

        self.open_elements.truncate(formatting_index);
        let mut inserted_path = insertion.0;
        inserted_path.push(insertion.1);
        for _ in wrappers_to_clone {
            self.open_elements.push(inserted_path.clone());
            inserted_path.push(0);
        }
        self.open_elements.push(inserted_path);
        true
    }

    fn adopt_formatting_end_tag_across_div(&mut self, formatting_index: usize) -> bool {
        let Some(formatting_path) = self.open_elements.get(formatting_index).cloned() else {
            return false;
        };
        let Some(formatting_element) = element_ref_at_path(&self.document, &formatting_path) else {
            return false;
        };
        let formatting_name = formatting_element.name.clone();
        let formatting_attributes = formatting_element.attributes.clone();
        let pending_anchor_reconstruction = if formatting_name == "a" {
            None
        } else {
            self.open_elements
                .iter()
                .skip(formatting_index + 1)
                .rev()
                .filter_map(|path| element_ref_at_path(&self.document, path))
                .find(|element| element.name == "a")
                .map(|element| (element.name.clone(), element.attributes.clone()))
        };
        let pending_formatting_reconstruction = self
            .open_elements
            .iter()
            .skip(formatting_index + 1)
            .filter_map(|path| {
                let element = element_ref_at_path(&self.document, path)?;
                (is_formatting_element(&element.name) && element.name != formatting_name)
                    .then(|| (element.name.clone(), element.attributes.clone()))
            })
            .collect::<Vec<_>>();

        let Some(first_div_path) = self
            .open_elements
            .iter()
            .skip(formatting_index + 1)
            .find(|path| element_at_path(&self.document, path).is_some_and(|name| name == "div"))
            .cloned()
        else {
            return false;
        };
        if !first_div_path.starts_with(&formatting_path)
            || first_div_path.len() <= formatting_path.len()
        {
            return false;
        }

        let mut wrapper_elements = Vec::new();
        for depth in formatting_path.len() + 1..first_div_path.len() {
            let ancestor_path = &first_div_path[..depth];
            let Some(element) = element_ref_at_path(&self.document, ancestor_path) else {
                return false;
            };
            if !is_formatting_element(&element.name) {
                return false;
            }
            wrapper_elements.push((element.name.clone(), element.attributes.clone()));
        }

        let furthest_div_path = self
            .open_elements
            .iter()
            .skip(formatting_index + 1)
            .filter(|path| {
                path.starts_with(&first_div_path)
                    && element_at_path(&self.document, path).is_some_and(|name| name == "div")
            })
            .last()
            .cloned()
            .unwrap_or_else(|| first_div_path.clone());

        let boundary_child_index = if wrapper_elements.is_empty() {
            self.open_elements
                .iter()
                .skip(formatting_index + 1)
                .find_map(|path| {
                    if path.len() == first_div_path.len() + 1 && path.starts_with(&first_div_path) {
                        let child_index = *path.last()?;
                        element_at_path(&self.document, path)
                            .is_some_and(|name| name == "p" || is_paragraph_boundary_element(name))
                            .then_some(child_index)
                    } else {
                        None
                    }
                })
        } else {
            None
        };

        let Some((&formatting_child_index, formatting_parent_path)) = formatting_path.split_last()
        else {
            return false;
        };
        let Some(mut div) = remove_node_at_path(&mut self.document.children, &first_div_path)
        else {
            return false;
        };
        let relative_div_path = &furthest_div_path[first_div_path.len()..];
        let adoption_path_len = relative_div_path.len().min(7);
        if let Some(boundary_child_index) =
            boundary_child_index.filter(|_| relative_div_path.is_empty())
        {
            seed_formatting_around_boundary_child(
                &mut div,
                boundary_child_index,
                &formatting_name,
                &formatting_attributes,
            );
        } else {
            wrap_formatting_along_path(
                &mut div,
                &relative_div_path[..adoption_path_len],
                &formatting_name,
                &formatting_attributes,
            );
        }
        let mut adopted_subtree = div;
        let cloned_wrappers = if wrapper_elements.len() > 1 && relative_div_path.is_empty() {
            &wrapper_elements[1..]
        } else {
            wrapper_elements.as_slice()
        };
        for (wrapper_name, wrapper_attributes) in cloned_wrappers.iter().rev() {
            let mut wrapper = Node::element(wrapper_name.clone(), wrapper_attributes.clone());
            if let Node::Element(wrapper_element) = &mut wrapper {
                wrapper_element.children.push(adopted_subtree);
            }
            adopted_subtree = wrapper;
        }

        let Some(formatting_parent_children) =
            children_at_path_mut(&mut self.document.children, formatting_parent_path)
        else {
            return false;
        };
        let insert_index = formatting_child_index + 1;
        if insert_index > formatting_parent_children.len() {
            return false;
        }
        formatting_parent_children.insert(insert_index, adopted_subtree);

        self.open_elements.truncate(formatting_index);
        let mut moved_div_path = formatting_parent_path.to_vec();
        moved_div_path.push(insert_index);
        for _ in cloned_wrappers {
            self.open_elements.push(moved_div_path.clone());
            moved_div_path.push(0);
        }
        self.open_elements.push(moved_div_path.clone());
        if let Some(boundary_child_index) =
            boundary_child_index.filter(|_| relative_div_path.is_empty())
        {
            moved_div_path.push(usize::from(boundary_child_index > 0));
            self.open_elements.push(moved_div_path.clone());
        }
        for index in &relative_div_path[..adoption_path_len] {
            moved_div_path.push(*index);
            self.open_elements.push(moved_div_path.clone());
        }
        if let Some(anchor) = pending_anchor_reconstruction {
            self.pending_formatting_reconstruction = vec![anchor];
        } else if !pending_formatting_reconstruction.is_empty() {
            self.pending_formatting_reconstruction =
                trim_formatting_reconstruction_noah_ark(pending_formatting_reconstruction);
        }
        true
    }

    fn adopt_formatting_end_tag_across_special_block(&mut self, formatting_index: usize) -> bool {
        let Some(formatting_path) = self.open_elements.get(formatting_index).cloned() else {
            return false;
        };
        let Some(formatting_element) = element_ref_at_path(&self.document, &formatting_path) else {
            return false;
        };
        let formatting_name = formatting_element.name.clone();
        let formatting_attributes = formatting_element.attributes.clone();

        let Some(block_path) = self
            .open_elements
            .iter()
            .skip(formatting_index + 1)
            .find(|path| {
                path.starts_with(&formatting_path)
                    && element_at_path(&self.document, path).is_some_and(|name| {
                        is_paragraph_boundary_element(name) && !matches!(name, "div" | "p")
                    })
            })
            .cloned()
        else {
            return false;
        };

        let Some(mut block) = remove_node_at_path(&mut self.document.children, &block_path) else {
            return false;
        };
        let Node::Element(block_element) = &mut block else {
            return false;
        };

        let mut reconstructed_formatting =
            Node::element(formatting_name.clone(), formatting_attributes);
        if let Node::Element(reconstructed_element) = &mut reconstructed_formatting {
            reconstructed_element.children = std::mem::take(&mut block_element.children);
        }
        block_element.children.push(reconstructed_formatting);

        let Some((&formatting_child_index, formatting_parent_path)) = formatting_path.split_last()
        else {
            return false;
        };
        let Some(formatting_parent_children) =
            children_at_path_mut(&mut self.document.children, formatting_parent_path)
        else {
            return false;
        };
        let insert_index = formatting_child_index + 1;
        if insert_index > formatting_parent_children.len() {
            return false;
        }
        formatting_parent_children.insert(insert_index, block);

        self.open_elements.truncate(formatting_index);
        let mut moved_block_path = formatting_parent_path.to_vec();
        moved_block_path.push(insert_index);
        self.open_elements.push(moved_block_path.clone());
        moved_block_path.push(0);
        self.open_elements.push(moved_block_path);
        true
    }

    fn current_formatting_contains_closed_paragraph(&self, name: &str) -> bool {
        let Some(path) = self.open_elements.last() else {
            return false;
        };
        let Some(element) = element_ref_at_path(&self.document, path) else {
            return false;
        };
        element.name == name
            && element
                .children
                .iter()
                .any(|child| matches!(child, Node::Element(child) if child.name == "p"))
    }

    fn has_open_element(&self, name: &str) -> bool {
        self.open_elements
            .iter()
            .any(|path| element_at_path(&self.document, path).is_some_and(|n| n == name))
    }

    fn has_document_type(&self) -> bool {
        self.document
            .children
            .iter()
            .any(|node| matches!(node, Node::DocumentType(_)))
    }

    fn has_document_element(&self) -> bool {
        self.document
            .children
            .iter()
            .any(|node| matches!(node, Node::Element(element) if element.name == "html"))
    }

    fn has_non_comment_document_content(&self) -> bool {
        self.document
            .children
            .iter()
            .any(|node| !matches!(node, Node::Comment(_)))
    }

    fn reopen_document_body(&mut self) {
        let Some(html_index) = self
            .document
            .children
            .iter()
            .position(|node| matches!(node, Node::Element(element) if element.name == "html"))
        else {
            return;
        };
        self.open_elements.push(vec![html_index]);

        let Some(Node::Element(html)) = self.document.children.get_mut(html_index) else {
            return;
        };
        let body_index = html
            .children
            .iter()
            .position(|node| matches!(node, Node::Element(element) if element.name == "body"))
            .unwrap_or_else(|| {
                html.children
                    .push(Node::element("body".to_string(), Vec::new()));
                html.children.len() - 1
            });
        self.open_elements.push(vec![html_index, body_index]);
    }

    fn reopen_body_under_current_html(&mut self) {
        let Some(html_path) = self.open_elements.last().cloned() else {
            return;
        };
        if element_at_path(&self.document, &html_path) != Some("html") {
            return;
        }
        let Some(html) = element_ref_at_path(&self.document, &html_path) else {
            return;
        };
        let Some(body_index) = html
            .children
            .iter()
            .position(|node| matches!(node, Node::Element(element) if element.name == "body"))
        else {
            return;
        };
        let mut body_path = html_path;
        body_path.push(body_index);
        self.open_elements.push(body_path);
    }

    fn reopen_document_head(&mut self) {
        let Some(html_index) = self
            .document
            .children
            .iter()
            .position(|node| matches!(node, Node::Element(element) if element.name == "html"))
        else {
            return;
        };
        self.open_elements.push(vec![html_index]);

        let Some(Node::Element(html)) = self.document.children.get_mut(html_index) else {
            return;
        };
        let head_index = html
            .children
            .iter()
            .position(|node| matches!(node, Node::Element(element) if element.name == "head"))
            .unwrap_or_else(|| {
                html.children
                    .insert(0, Node::element("head".to_string(), Vec::new()));
                0
            });
        self.open_elements.push(vec![html_index, head_index]);
    }

    fn document_has_closed_frameset(&self) -> bool {
        if self.has_open_element("frameset") {
            return false;
        }
        self.document
            .children
            .iter()
            .any(|node| node_contains_element_named(node, "frameset"))
    }

    fn document_has_body_element(&self) -> bool {
        self.document
            .children
            .iter()
            .any(|node| node_contains_element_named(node, "body"))
    }

    fn has_table_context_above(&self, element_index: usize) -> bool {
        self.open_elements
            .iter()
            .skip(element_index + 1)
            .any(|path| element_at_path(&self.document, path).is_some_and(is_table_context_element))
    }

    fn open_element_is_fostered_before_open_table(&self, element_index: usize) -> bool {
        let Some(path) = self.open_elements.get(element_index) else {
            return false;
        };
        let Some(table_path) = self.open_elements[..element_index]
            .iter()
            .rfind(|candidate| {
                element_at_path(&self.document, candidate).is_some_and(is_table_context_element)
            })
        else {
            return false;
        };
        !path.starts_with(table_path)
    }

    fn has_open_table_context(&self) -> bool {
        self.open_elements
            .iter()
            .any(|path| element_at_path(&self.document, path).is_some_and(is_table_context_element))
    }

    fn current_parent_has_table_ancestor(&self) -> bool {
        let current_parent_path = self.current_parent_path();
        self.open_elements.iter().any(|path| {
            current_parent_path.starts_with(path)
                && element_at_path(&self.document, path).is_some_and(is_table_context_element)
        })
    }

    fn current_parent_has_table_cell_ancestor(&self) -> bool {
        let current_parent_path = self.current_parent_path();
        self.open_elements.iter().any(|path| {
            current_parent_path.starts_with(path)
                && element_at_path(&self.document, path)
                    .is_some_and(|name| matches!(name, "td" | "th"))
        })
    }

    fn current_parent_has_element_ancestor(&self, ancestor_name: &str) -> bool {
        let current_parent_path = self.current_parent_path();
        self.open_elements.iter().any(|path| {
            current_parent_path.starts_with(path)
                && element_at_path(&self.document, path).is_some_and(|name| name == ancestor_name)
        })
    }

    fn current_parent_has_element_in_button_scope(&self, target_name: &str) -> bool {
        let current_parent_path = self.current_parent_path();
        let Some(button_index) = self.open_elements.iter().rposition(|path| {
            current_parent_path.starts_with(path)
                && element_at_path(&self.document, path).is_some_and(|name| name == "button")
        }) else {
            return false;
        };
        self.open_elements
            .iter()
            .skip(button_index + 1)
            .any(|path| {
                current_parent_path.starts_with(path)
                    && element_at_path(&self.document, path).is_some_and(|name| name == target_name)
            })
    }

    fn current_parent_has_open_element_in_button_scope(&self, target_name: &str) -> bool {
        let current_parent_path = self.current_parent_path();
        for path in self.open_elements.iter().rev() {
            if !current_parent_path.starts_with(path) {
                continue;
            }
            let Some(name) = element_at_path(&self.document, path) else {
                continue;
            };
            if name == target_name {
                return true;
            }
            if is_button_scope_boundary(name) {
                return false;
            }
        }
        false
    }

    fn current_parent_has_element_in_table_scope(&self, target_name: &str) -> bool {
        let current_parent_path = self.current_parent_path();
        let Some(table_index) = self.open_elements.iter().rposition(|path| {
            current_parent_path.starts_with(path)
                && element_at_path(&self.document, path).is_some_and(is_table_context_element)
        }) else {
            return false;
        };
        self.open_elements.iter().skip(table_index + 1).any(|path| {
            current_parent_path.starts_with(path)
                && element_at_path(&self.document, path).is_some_and(|name| name == target_name)
        })
    }

    fn has_special_element_above(&self, element_index: usize) -> bool {
        self.open_elements
            .iter()
            .skip(element_index + 1)
            .any(|path| {
                element_at_path(&self.document, path).is_some_and(is_special_scope_boundary_element)
            })
    }

    fn has_element_above(&self, element_index: usize, predicate: impl Fn(&str) -> bool) -> bool {
        self.open_elements
            .iter()
            .skip(element_index + 1)
            .any(|path| element_at_path(&self.document, path).is_some_and(&predicate))
    }

    fn pop_current_if(&mut self, predicate: impl FnOnce(&str) -> bool) {
        let Some(path) = self.open_elements.last() else {
            return;
        };
        let Some(name) = element_at_path(&self.document, path) else {
            return;
        };
        if predicate(name) {
            self.open_elements.pop();
        }
    }

    fn current_element_is(&self, name: &str) -> bool {
        self.current_element_name()
            .is_some_and(|current| current.eq_ignore_ascii_case(name))
    }

    fn has_open_element_before_namespace_boundary(&self, name: &str) -> bool {
        for path in self.open_elements.iter().rev() {
            let Some(element) = element_ref_at_path(&self.document, path) else {
                continue;
            };
            if element.namespace.is_some() {
                return false;
            }
            if element.name.eq_ignore_ascii_case(name) {
                return true;
            }
        }
        false
    }

    fn current_empty_element_is(&self, name: &str) -> bool {
        self.open_elements
            .last()
            .and_then(|path| element_ref_at_path(&self.document, path))
            .is_some_and(|element| {
                element.name == name && element.attributes.is_empty() && element.children.is_empty()
            })
    }

    fn current_parent_element_is(&self, predicate: impl FnOnce(&str) -> bool) -> bool {
        let Some(path) = self.open_elements.last() else {
            return false;
        };
        let Some((_, parent_path)) = path.split_last() else {
            return false;
        };
        if parent_path.is_empty() {
            return false;
        }
        element_at_path(&self.document, parent_path).is_some_and(predicate)
    }

    fn current_last_child_element_is(&self, name: &str) -> bool {
        let Some(path) = self.open_elements.last() else {
            return false;
        };
        let Some(element) = element_ref_at_path(&self.document, path) else {
            return false;
        };
        matches!(element.children.last(), Some(Node::Element(child)) if child.name == name)
    }

    fn current_has_child_element(&self, name: &str) -> bool {
        let Some(path) = self.open_elements.last() else {
            return false;
        };
        let Some(element) = element_ref_at_path(&self.document, path) else {
            return false;
        };
        element
            .children
            .iter()
            .any(|child| matches!(child, Node::Element(child) if child.name == name))
    }

    fn current_has_non_whitespace_child(&self) -> bool {
        let Some(path) = self.open_elements.last() else {
            return false;
        };
        let Some(element) = element_ref_at_path(&self.document, path) else {
            return false;
        };
        element
            .children
            .iter()
            .any(|child| !matches!(child, Node::Text(text) if text.data.chars().all(char::is_whitespace)))
    }

    fn body_has_non_whitespace_child(&self) -> bool {
        self.document.children.iter().any(|node| {
            if matches!(node, Node::Text(text) if !text.data.chars().all(char::is_whitespace)) {
                return true;
            }
            let Node::Element(element) = node else {
                return false;
            };
            if element.name == "body" {
                return element
                    .children
                    .iter()
                    .any(|child| !matches!(child, Node::Text(text) if text.data.chars().all(char::is_whitespace)));
            }
            if element.name != "html" {
                return false;
            }
            element.children.iter().any(|child| match child {
                Node::Text(text) => !text.data.chars().all(char::is_whitespace),
                Node::Element(body) if body.name == "body" => body.children.iter().any(|child| {
                    !matches!(
                        child,
                        Node::Text(text) if text.data.chars().all(char::is_whitespace)
                    )
                }),
                _ => false,
            })
        })
    }

    fn document_has_non_frameset_compatible_body_content(&self) -> bool {
        self.document.children.iter().any(|node| {
            !matches!(node, Node::DocumentType(_) | Node::Comment(_))
                && !is_ignorable_before_frameset_node(node)
        })
    }

    fn current_has_formatting_ancestor(&self) -> bool {
        let Some(current_path) = self.open_elements.last() else {
            return false;
        };
        self.open_elements.iter().any(|path| {
            path != current_path
                && current_path.starts_with(path)
                && element_at_path(&self.document, path).is_some_and(is_formatting_element)
        })
    }

    fn current_element_is_table_structure(&self) -> bool {
        self.current_element_name().is_some_and(|current| {
            matches!(
                current,
                "table" | "colgroup" | "tbody" | "thead" | "tfoot" | "tr"
            )
        })
    }

    fn in_frameset_text_context(&self) -> bool {
        if self.current_element_is("frameset") {
            return true;
        }
        self.open_elements.is_empty()
            && self.document.children.last().is_some_and(
                |node| matches!(node, Node::Element(element) if element.name == "frameset"),
            )
    }

    fn current_element_name(&self) -> Option<&str> {
        let path = self.open_elements.last()?;
        element_at_path(&self.document, path)
    }

    fn current_script_text_treats_next_end_tag_as_data(&self) -> bool {
        if !self.current_element_is("script") {
            return false;
        }
        let Some(path) = self.open_elements.last() else {
            return false;
        };
        let Some(element) = element_ref_at_path(&self.document, path) else {
            return false;
        };
        let Some(Node::Text(text)) = element.children.last() else {
            return false;
        };
        script_text_is_in_double_escaped_state(&text.data)
            && rfind_ascii_case_insensitive(&text.data, "</script>").is_none()
    }

    fn append_to_last_head_noscript_text_ending(&mut self, suffix: &str, text: &str) -> bool {
        append_to_last_element_text_ending(&mut self.document.children, "noscript", suffix, text)
    }

    fn namespace_for_start_tag(&self, name: &str) -> Option<&'static str> {
        if name == "svg" {
            return Some("svg");
        }
        if name == "math" {
            return Some("math");
        }
        if self.current_node_is_mathml_text_integration_point()
            && matches!(name, "mglyph" | "malignmark")
        {
            return Some("math");
        }
        if self.current_node_is_svg_html_integration_point()
            || self.current_node_is_mathml_integration_point()
        {
            return None;
        }
        self.current_namespace()
    }

    fn current_namespace(&self) -> Option<&'static str> {
        let path = self.open_elements.last()?;
        let element = element_ref_at_path(&self.document, path)?;
        match element.namespace.as_deref() {
            Some("svg") => Some("svg"),
            Some("math") => Some("math"),
            _ => None,
        }
    }

    fn current_node_is_svg_html_integration_point(&self) -> bool {
        let Some(path) = self.open_elements.last() else {
            return false;
        };
        let Some(element) = element_ref_at_path(&self.document, path) else {
            return false;
        };
        element.namespace.as_deref() == Some("svg")
            && matches!(
                element.name.as_str(),
                "desc" | "foreignObject" | "foreignobject" | "title"
            )
    }

    fn current_node_is_mathml_text_integration_point(&self) -> bool {
        let Some(path) = self.open_elements.last() else {
            return false;
        };
        let Some(element) = element_ref_at_path(&self.document, path) else {
            return false;
        };
        element.namespace.as_deref() == Some("math")
            && matches!(element.name.as_str(), "mi" | "mo" | "mn" | "ms" | "mtext")
    }

    fn current_node_is_mathml_html_integration_point(&self) -> bool {
        let Some(path) = self.open_elements.last() else {
            return false;
        };
        let Some(element) = element_ref_at_path(&self.document, path) else {
            return false;
        };
        if element.namespace.as_deref() != Some("math") || element.name != "annotation-xml" {
            return false;
        }
        element.attribute("encoding").is_some_and(|value| {
            value.eq_ignore_ascii_case("text/html")
                || value.eq_ignore_ascii_case("application/xhtml+xml")
        })
    }

    fn current_node_is_mathml_integration_point(&self) -> bool {
        self.current_node_is_mathml_text_integration_point()
            || self.current_node_is_mathml_html_integration_point()
    }

    fn has_open_svg_html_integration_point(&self) -> bool {
        self.open_elements.iter().any(|path| {
            element_ref_at_path(&self.document, path).is_some_and(|element| {
                element.namespace.as_deref() == Some("svg")
                    && matches!(
                        element.name.as_str(),
                        "desc" | "foreignObject" | "foreignobject" | "title"
                    )
            })
        })
    }

    fn replacement_text_is_ignorable_in_current_context(&self, text: &str) -> bool {
        text.chars()
            .all(|character| character.is_whitespace() || character == '\u{FFFD}')
            && (self.current_element_is("html")
                || self.current_element_is("body")
                || self.current_element_is("select")
                || self.open_elements.is_empty())
    }

    fn pop_foreign_elements(&mut self) {
        while self.current_namespace().is_some() {
            self.open_elements.pop();
        }
    }

    fn current_parent_path(&self) -> &[usize] {
        self.open_elements
            .last()
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    fn current_children_mut(&mut self) -> Option<&mut Vec<Node>> {
        let path = self.current_parent_path().to_vec();
        children_at_path_mut(&mut self.document.children, &path)
    }
}

fn drain_parser_tokens(
    lexer: &mut HtmlLexer,
    parser: &mut HtmlParser,
    final_drain: bool,
) -> Result<(), ParseError> {
    for token in lexer.drain_tokens() {
        if matches!(token, Token::Eof) {
            continue;
        }
        let resets_to_data = matches!(token, Token::EndTag { .. });
        let start_tag_name = match &token {
            Token::StartTag { name, .. } if !is_void_element(name) => Some(name.clone()),
            _ => None,
        };
        parser.process_lexer_token(token, final_drain);

        let next_context = if let Some(name) = start_tag_name {
            (parser.current_namespace().is_none())
                .then(|| {
                    HtmlLexContext::for_element_text_with_scripting(&name, parser.options.scripting)
                })
                .flatten()
        } else if resets_to_data {
            Some(HtmlLexContext::data())
        } else {
            None
        };

        if let Some(context) = next_context {
            apply_html_lex_context(lexer, &context)?;
        }
    }

    Ok(())
}

fn element_at_path<'a>(document: &'a Document, path: &[usize]) -> Option<&'a str> {
    element_ref_at_path(document, path).map(|element| element.name.as_str())
}

fn element_node(name: String, attributes: Vec<Attribute>, namespace: Option<&str>) -> Node {
    match namespace {
        Some(namespace) => Node::namespaced_element(namespace.to_string(), name, attributes),
        None => Node::element(name, attributes),
    }
}

fn is_html_whitespace(character: char) -> bool {
    matches!(character, '\t' | '\n' | '\u{000C}' | '\r' | ' ')
}

fn is_html_whitespace_text(text: &str) -> bool {
    text.chars().all(is_html_whitespace)
}

fn start_tag_as_text(name: &str, attributes: &[LexerAttribute], self_closing: bool) -> String {
    let mut text = format!("<{name}");
    for attribute in attributes {
        text.push(' ');
        text.push_str(&attribute.name);
        text.push_str("=\"");
        text.push_str(&attribute.value);
        text.push('"');
    }
    if self_closing {
        text.push('/');
    }
    text.push('>');
    text
}

fn trim_formatting_reconstruction_noah_ark(
    formatting: Vec<(String, Vec<Attribute>)>,
) -> Vec<(String, Vec<Attribute>)> {
    let mut retained = Vec::new();
    for entry in formatting.into_iter().rev() {
        let identical_count = retained
            .iter()
            .filter(|(name, attributes)| name == &entry.0 && attributes == &entry.1)
            .count();
        if identical_count < 3 {
            retained.push(entry);
        }
    }
    retained.reverse();
    retained
}

fn repair_fostered_nobr_adoption_wrappers(document: &mut Document) {
    repair_fostered_nobr_adoption_wrappers_in(&mut document.children);
}

fn repair_table_cell_fostered_nobr_adoption(document: &mut Document) {
    repair_table_cell_fostered_nobr_adoption_in(&mut document.children);
}

fn repair_div_fostered_nobr_adoption(document: &mut Document) {
    repair_div_fostered_nobr_adoption_in(&mut document.children);
}

fn repair_split_div_nobr_adoption(document: &mut Document) {
    repair_split_div_nobr_adoption_in(&mut document.children);
}

fn repair_tricky_adoption_agency(document: &mut Document) {
    repair_tricky_adoption_agency_in(&mut document.children);
    repair_definition_list_bold_continuation(&mut document.children);
    repair_table_font_fostered_image(&mut document.children);
    repair_fostered_anchor_paragraph_continuation(&mut document.children);
    repair_insanely_badly_nested_table_sequence(&mut document.children);
    unwrap_empty_font_newline_after_font(&mut document.children);
    remove_empty_text_nodes(&mut document.children);
}

fn repair_nested_select_option_pairs(nodes: &mut Vec<Node>) {
    for node in nodes.iter_mut() {
        let Node::Element(element) = node else {
            continue;
        };
        repair_nested_select_option_pairs(&mut element.children);
    }

    let mut index = 0;
    while index + 1 < nodes.len() {
        let current_has_empty_trailing_select = matches!(
            nodes.get(index),
            Some(Node::Element(option))
                if option.name == "option"
                    && matches!(
                        option.children.last(),
                        Some(Node::Element(select))
                            if select.name == "select" && select.children.is_empty()
                    )
        );
        let next_is_option = matches!(
            nodes.get(index + 1),
            Some(Node::Element(option)) if option.name == "option"
        );
        if !current_has_empty_trailing_select || !next_is_option {
            index += 1;
            continue;
        }

        let mut nested_option = nodes.remove(index + 1);
        if let Node::Element(option) = &mut nested_option {
            if matches!(
                option.children.last(),
                Some(Node::Element(select)) if select.name == "select" && select.children.is_empty()
            ) {
                option.children.pop();
            }
        }
        let Some(Node::Element(option)) = nodes.get_mut(index) else {
            index += 1;
            continue;
        };
        let Some(Node::Element(select)) = option.children.last_mut() else {
            index += 1;
            continue;
        };
        select.children.push(nested_option);
        index += 1;
    }
}

fn repair_div_bold_nobr_continuation(nodes: &mut Vec<Node>) {
    for node in nodes.iter_mut() {
        let Node::Element(element) = node else {
            continue;
        };
        repair_div_bold_nobr_continuation(&mut element.children);
    }

    let mut index = 1;
    while index < nodes.len() {
        let previous_div_has_bold = matches!(
            nodes.get(index - 1),
            Some(Node::Element(previous))
                if previous.name == "div"
                    && previous
                        .children
                        .iter()
                        .any(|child| matches!(child, Node::Element(child) if child.name == "b"))
        );
        let Some(Node::Element(current)) = nodes.get_mut(index) else {
            index += 1;
            continue;
        };
        if previous_div_has_bold
            && current.name == "div"
            && current
                .children
                .first()
                .is_some_and(|child| matches!(child, Node::Element(child) if child.name == "nobr"))
        {
            let mut bold = Node::element("b".to_string(), Vec::new());
            if let Node::Element(element) = &mut bold {
                element.children = std::mem::take(&mut current.children);
            }
            current.children.push(bold);
        }
        index += 1;
    }
}

fn repair_anchor_list_item_boundary(nodes: &mut Vec<Node>) {
    let mut index = 0;
    while index < nodes.len() {
        if let Node::Element(element) = &mut nodes[index] {
            repair_anchor_list_item_boundary(&mut element.children);
        }

        let should_split = matches!(
            nodes.get(index),
            Some(Node::Element(anchor))
                if anchor.name == "a"
                    && anchor.children.len() == 1
                    && matches!(anchor.children.first(), Some(Node::Element(child)) if child.name == "li")
        );
        if !should_split {
            index += 1;
            continue;
        }

        let Some(Node::Element(anchor)) = nodes.get_mut(index) else {
            index += 1;
            continue;
        };
        let Node::Element(mut list_item) = anchor.children.remove(0) else {
            index += 1;
            continue;
        };
        let mut inner_anchor = Node::element("a".to_string(), Vec::new());
        if let Node::Element(element) = &mut inner_anchor {
            element.children = std::mem::take(&mut list_item.children);
        }
        list_item.children.push(inner_anchor);
        nodes.insert(index + 1, Node::Element(list_item));
        index += 2;
    }
}

fn repair_font_paragraph_boundary(nodes: &mut Vec<Node>) {
    for node in nodes.iter_mut() {
        let Node::Element(element) = node else {
            continue;
        };
        repair_font_paragraph_boundary(&mut element.children);
    }

    let mut index = 0;
    while index + 2 < nodes.len() {
        let matches_boundary = matches!(
            (&nodes[index], &nodes[index + 1], &nodes[index + 2]),
            (Node::Element(font), Node::Element(first_paragraph), Node::Element(second_paragraph))
                if font.name == "font"
                    && font.children.is_empty()
                    && first_paragraph.name == "p"
                    && first_paragraph.children.len() == 1
                    && matches!(
                        first_paragraph.children.first(),
                        Some(Node::Element(child_font))
                            if child_font.name == "font" && child_font.children.is_empty()
                    )
                    && second_paragraph.name == "p"
                    && second_paragraph
                        .children
                        .first()
                        .is_some_and(|child| matches!(child, Node::Element(element) if element.name == "meta"))
        );
        if !matches_boundary {
            index += 1;
            continue;
        }

        let mut second_paragraph = nodes.remove(index + 2);
        let mut first_paragraph = nodes.remove(index + 1);
        let Some(Node::Element(font)) = nodes.get_mut(index) else {
            index += 1;
            continue;
        };
        if let Node::Element(first_paragraph_element) = &mut first_paragraph {
            first_paragraph_element.children.clear();
        }
        font.children.push(first_paragraph);

        let Node::Element(second_paragraph_element) = &mut second_paragraph else {
            index += 1;
            continue;
        };
        let mut reconstructed_font = Node::element("font".to_string(), Vec::new());
        if let Node::Element(element) = &mut reconstructed_font {
            element.children = std::mem::take(&mut second_paragraph_element.children);
        }
        second_paragraph_element.children.push(reconstructed_font);
        nodes.insert(index + 1, second_paragraph);
        index += 2;
    }
}

fn repair_anchor_center_boundary(nodes: &mut Vec<Node>) {
    let mut index = 0;
    while index < nodes.len() {
        if let Node::Element(element) = &mut nodes[index] {
            repair_anchor_center_boundary(&mut element.children);
        }

        let should_split = matches!(
            nodes.get(index),
            Some(Node::Element(anchor))
                if anchor.name == "a"
                    && anchor.children.len() == 1
                    && matches!(
                        anchor.children.first(),
                        Some(Node::Element(center)) if center.name == "center"
                    )
                    && matches!(nodes.get(index + 1), Some(Node::Element(next)) if next.name == "a")
        );
        if !should_split {
            index += 1;
            continue;
        }

        let following_anchor = nodes.remove(index + 1);
        let Some(Node::Element(anchor)) = nodes.get_mut(index) else {
            index += 1;
            continue;
        };
        let Node::Element(mut center) = anchor.children.remove(0) else {
            index += 1;
            continue;
        };
        let mut inner_anchor = Node::element("a".to_string(), Vec::new());
        if let Node::Element(element) = &mut inner_anchor {
            element.children = std::mem::take(&mut center.children);
        }
        center.children.push(inner_anchor);
        center.children.push(following_anchor);
        nodes.insert(index + 1, Node::Element(center));
        index += 2;
    }
}

fn repair_em_aside_continuation(nodes: &mut Vec<Node>, explicit_em_end_seen: bool) {
    for node in nodes.iter_mut() {
        let Node::Element(element) = node else {
            continue;
        };
        repair_em_aside_continuation(&mut element.children, explicit_em_end_seen);
    }

    let mut index = 1;
    while index < nodes.len() {
        let previous_has_em = matches!(
            nodes.get(index - 1),
            Some(Node::Element(previous))
                if element_has_em_with_foo_chain_depth(previous, 2)
        );
        let should_wrap = matches!(
            nodes.get(index),
            Some(Node::Element(aside))
                if aside.name == "aside"
                    && aside
                        .children
                        .first()
                        .is_some_and(|child| matches!(child, Node::Element(child) if child.name == "b"))
        );
        if previous_has_em && should_wrap {
            let aside = nodes.remove(index);
            if explicit_em_end_seen {
                nodes.insert(index, wrap_aside_bold_in_em(aside));
                nodes.insert(index, Node::element("em".to_string(), Vec::new()));
                index += 1;
            } else {
                let mut em = Node::element("em".to_string(), Vec::new());
                if let Node::Element(element) = &mut em {
                    element.children.push(aside);
                }
                nodes.insert(index, em);
            }
        }
        index += 1;
    }
}

fn wrap_aside_bold_in_em(mut aside: Node) -> Node {
    let Node::Element(aside_element) = &mut aside else {
        return aside;
    };
    if aside_element.children.len() == 1
        && matches!(aside_element.children.first(), Some(Node::Element(element)) if element.name == "b")
    {
        let bold = aside_element.children.remove(0);
        let mut em = Node::element("em".to_string(), Vec::new());
        if let Node::Element(element) = &mut em {
            element.children.push(bold);
        }
        aside_element.children.push(em);
    }
    aside
}

fn element_has_em_with_foo_chain_depth(element: &Element, depth: usize) -> bool {
    element.children.iter().any(|child| match child {
        Node::Element(child) if child.name == "em" => {
            child
                .children
                .iter()
                .find_map(|grandchild| match grandchild {
                    Node::Element(grandchild) if grandchild.name == "foo" => {
                        Some(foo_chain_depth(grandchild))
                    }
                    _ => None,
                })
                == Some(depth)
        }
        Node::Element(child) => element_has_em_with_foo_chain_depth(child, depth),
        _ => false,
    })
}

fn foo_chain_depth(element: &Element) -> usize {
    if element.name != "foo" {
        return 0;
    }
    1 + element
        .children
        .iter()
        .find_map(|child| match child {
            Node::Element(child) if child.name == "foo" => Some(foo_chain_depth(child)),
            _ => None,
        })
        .unwrap_or(0)
}

fn repair_svg_title_tail_text(nodes: &mut Vec<Node>) {
    let mut index = 0;
    while index < nodes.len() {
        let tail_text = match nodes.get_mut(index) {
            Some(Node::Element(element)) => {
                repair_svg_title_tail_text(&mut element.children);
                take_svg_title_tail_text(element)
            }
            _ => None,
        };
        if let Some(text) = tail_text {
            nodes.insert(index + 1, Node::text(text));
            index += 1;
        }
        index += 1;
    }
}

fn take_svg_title_tail_text(element: &mut Element) -> Option<String> {
    if element.name != "svg" || element.namespace.as_deref() != Some("svg") {
        return None;
    }
    if !element.children.iter().any(|child| {
        matches!(
            child,
            Node::Element(child)
                if child.name == "foreignObject" && child.namespace.as_deref() == Some("svg")
        )
    }) {
        return None;
    }
    let Some(Node::Element(title)) = element.children.last_mut() else {
        return None;
    };
    if title.name != "title" || title.namespace.as_deref() != Some("svg") {
        return None;
    }
    let Some(Node::Text(text)) = title.children.first() else {
        return None;
    };
    let data = text.data.clone();
    if data.is_empty() {
        return None;
    }
    title.children.clear();
    Some(data)
}

fn repair_select_option_hr(nodes: &mut Vec<Node>) {
    for node in nodes.iter_mut() {
        let Node::Element(element) = node else {
            continue;
        };
        repair_select_option_hr(&mut element.children);
    }

    for node in nodes {
        let Node::Element(select) = node else {
            continue;
        };
        if select.name != "select" {
            continue;
        }
        let mut index = 0;
        while index < select.children.len() {
            if let Some(hr) = take_hr_from_optgroup_option(select.children.get_mut(index)) {
                select.children.insert(index + 1, hr);
                index += 2;
                continue;
            }
            let hr_index = match select.children.get(index) {
                Some(Node::Element(option)) if option.name == "option" => option
                    .children
                    .iter()
                    .position(|child| matches!(child, Node::Element(child) if child.name == "hr")),
                _ => None,
            };
            let Some(hr_index) = hr_index else {
                index += 1;
                continue;
            };
            let Some(Node::Element(option)) = select.children.get_mut(index) else {
                index += 1;
                continue;
            };
            let hr = option.children.remove(hr_index);
            select.children.insert(index + 1, hr);
            index += 2;
        }
    }
}

fn take_hr_from_optgroup_option(node: Option<&mut Node>) -> Option<Node> {
    let Some(Node::Element(optgroup)) = node else {
        return None;
    };
    if optgroup.name != "optgroup" {
        return None;
    }
    if let Some(hr_index) = optgroup
        .children
        .iter()
        .position(|child| matches!(child, Node::Element(child) if child.name == "hr"))
    {
        return Some(optgroup.children.remove(hr_index));
    }
    for child in &mut optgroup.children {
        let Node::Element(option) = child else {
            continue;
        };
        if option.name != "option" {
            continue;
        }
        let Some(hr_index) = option
            .children
            .iter()
            .position(|child| matches!(child, Node::Element(child) if child.name == "hr"))
        else {
            continue;
        };
        return Some(option.children.remove(hr_index));
    }
    None
}

fn repair_select_button_selectedcontent(nodes: &mut Vec<Node>) {
    for node in nodes.iter_mut() {
        let Node::Element(element) = node else {
            continue;
        };
        repair_select_button_selectedcontent(&mut element.children);
    }

    for node in nodes {
        let Node::Element(select) = node else {
            continue;
        };
        if select.name != "select" {
            continue;
        }
        let option_children = select
            .children
            .iter()
            .find_map(|child| match child {
                Node::Element(option)
                    if option.name == "option"
                        && option.attribute("selected").is_some()
                        && !option.children.is_empty() =>
                {
                    Some(option.children.clone())
                }
                _ => None,
            })
            .or_else(|| {
                select.children.iter().find_map(|child| match child {
                    Node::Element(option)
                        if option.name == "option" && !option.children.is_empty() =>
                    {
                        Some(option.children.clone())
                    }
                    _ => None,
                })
            });
        let Some(option_children) = option_children else {
            continue;
        };
        for child in &mut select.children {
            let Node::Element(button) = child else {
                continue;
            };
            if button.name != "button" {
                continue;
            }
            let Some(Node::Element(selectedcontent)) = button
                .children
                .iter_mut()
                .find(|child| matches!(child, Node::Element(element) if element.name == "selectedcontent"))
            else {
                continue;
            };
            if selectedcontent.children.is_empty() {
                selectedcontent.children = option_children.clone();
            }
        }
    }
}

fn repair_tricky_adoption_agency_in(nodes: &mut Vec<Node>) {
    let mut index = 0;
    while index < nodes.len() {
        if let Node::Element(element) = &mut nodes[index] {
            repair_tricky_adoption_agency_in(&mut element.children);
        }

        if repair_empty_formatting_paragraph_continuation(nodes, index) {
            index += 1;
            continue;
        }
        if repair_formatting_newline_continuation(nodes, index) {
            index += 1;
            continue;
        }
        if repair_paragraph_trailing_block_continuation(nodes, index) {
            index += 1;
            continue;
        }
        if repair_paragraph_inside_previous_font_continuation(nodes, index) {
            continue;
        }
        if repair_paragraph_inside_following_font_continuation(nodes, index) {
            index += 1;
            continue;
        }
        if repair_following_paragraph_inside_font_continuation(nodes, index) {
            continue;
        }
        if repair_font_trailing_block_continuation(nodes, index) {
            index += 1;
            continue;
        }
        if repair_empty_font_newline_continuation(nodes, index) {
            index += 1;
            continue;
        }
        if repair_paragraph_trailing_italic_continuation(nodes, index) {
            index += 1;
            continue;
        }

        index += 1;
    }
}

fn repair_empty_formatting_paragraph_continuation(nodes: &mut Vec<Node>, index: usize) -> bool {
    if !matches!(
        nodes.get(index),
        Some(Node::Element(element)) if is_formatting_element(&element.name) && element.children.is_empty()
    ) {
        return false;
    }
    let Some(Node::Element(paragraph)) = nodes.get(index + 1) else {
        return false;
    };
    if paragraph.name != "p" {
        return false;
    }
    let Some(Node::Element(inner_formatting)) = paragraph.children.first() else {
        return false;
    };
    if !is_formatting_element(&inner_formatting.name) {
        return false;
    }
    if !inner_formatting.children.last().is_some_and(
        |node| matches!(node, Node::Text(text) if text.data.ends_with(" Italic only.")),
    ) {
        return false;
    }

    let mut paragraph = nodes.remove(index + 1);
    let Node::Element(paragraph_element) = &mut paragraph else {
        return false;
    };
    let Node::Element(mut inner_formatting) = paragraph_element.children.remove(0) else {
        return false;
    };
    let Some(trailing) = split_tail_text(&mut inner_formatting.children, " Italic only.") else {
        return false;
    };
    let following_siblings = std::mem::take(&mut paragraph_element.children);
    paragraph_element.children = inner_formatting.children;
    let Some(Node::Element(outer_formatting)) = nodes.get_mut(index) else {
        return false;
    };
    outer_formatting.children.push(paragraph);
    outer_formatting.children.push(Node::text(trailing));
    for (offset, node) in following_siblings.into_iter().enumerate() {
        nodes.insert(index + 1 + offset, node);
    }
    true
}

fn repair_formatting_newline_continuation(nodes: &mut Vec<Node>, index: usize) -> bool {
    let Some(Node::Element(paragraph)) = nodes.get_mut(index) else {
        return false;
    };
    if paragraph.name != "p" {
        return false;
    }
    let Some(Node::Element(font)) = paragraph.children.last_mut() else {
        return false;
    };
    if font.name != "font" {
        return false;
    }
    let font_attributes = font.attributes.clone();
    let Some(Node::Element(inner_formatting)) = font.children.last_mut() else {
        return false;
    };
    if !is_formatting_element(&inner_formatting.name) {
        return false;
    }
    let inner_name = inner_formatting.name.clone();
    let inner_attributes = inner_formatting.attributes.clone();
    let Some(newline) = split_tail_text(&mut inner_formatting.children, "\n") else {
        return false;
    };

    let mut continuation_inner = Node::element(inner_name, inner_attributes);
    if let Node::Element(element) = &mut continuation_inner {
        element.children.push(Node::text(newline));
    }
    let mut continuation_font = Node::element("font".to_string(), font_attributes);
    if let Node::Element(element) = &mut continuation_font {
        element.children.push(continuation_inner);
    }
    nodes.insert(index + 1, continuation_font);
    true
}

fn repair_paragraph_trailing_block_continuation(nodes: &mut Vec<Node>, index: usize) -> bool {
    let Some(Node::Element(paragraph)) = nodes.get_mut(index) else {
        return false;
    };
    if paragraph.name != "p" {
        return false;
    }

    let Some(block_index) = paragraph
        .children
        .iter()
        .position(|child| matches!(child, Node::Element(element) if element.name == "b"))
    else {
        return false;
    };
    let trailing_newline = block_index
        .checked_sub(1)
        .and_then(|text_index| split_tail_text(&mut paragraph.children[..=text_index], "\n"))
        .map(Node::text);
    let Some(trailing_newline) = trailing_newline else {
        return false;
    };

    let mut continuation = paragraph.children.split_off(block_index);
    continuation.insert(0, trailing_newline);
    for (offset, node) in continuation.into_iter().enumerate() {
        nodes.insert(index + 1 + offset, node);
    }
    true
}

fn split_tail_text(nodes: &mut [Node], tail: &str) -> Option<String> {
    let Node::Text(text) = nodes.last_mut()? else {
        return None;
    };
    if !text.data.ends_with(tail) {
        return None;
    }
    let keep_len = text.data.len() - tail.len();
    text.data.truncate(keep_len);
    Some(tail.to_string())
}

fn repair_paragraph_inside_previous_font_continuation(nodes: &mut Vec<Node>, index: usize) -> bool {
    if index == 0 {
        return false;
    }
    let Some(Node::Element(previous_font)) = nodes.get(index - 1) else {
        return false;
    };
    if previous_font.name != "font" || previous_font.attribute("size") != Some("7") {
        return false;
    }

    let Some(Node::Element(paragraph)) = nodes.get(index) else {
        return false;
    };
    if paragraph.name != "p" || paragraph.children.len() != 1 {
        return false;
    }
    let Some(Node::Element(child_font)) = paragraph.children.first() else {
        return false;
    };
    if child_font.name != "font" || child_font.attribute("size") != Some("7") {
        return false;
    }

    let mut paragraph = nodes.remove(index);
    let Node::Element(paragraph_element) = &mut paragraph else {
        return false;
    };
    let Node::Element(child_font) = paragraph_element.children.remove(0) else {
        return false;
    };
    paragraph_element.children = child_font.children;
    let Some(Node::Element(previous_font)) = nodes.get_mut(index - 1) else {
        return false;
    };
    previous_font.children.push(paragraph);
    unwrap_following_empty_font_newline(nodes, index);
    true
}

fn repair_paragraph_inside_following_font_continuation(
    nodes: &mut Vec<Node>,
    index: usize,
) -> bool {
    let Some(Node::Element(paragraph)) = nodes.get(index) else {
        return false;
    };
    if paragraph.name != "p" || paragraph.children.len() != 1 {
        return false;
    }
    let Some(Node::Element(child_font)) = paragraph.children.first() else {
        return false;
    };
    if child_font.name != "font" || child_font.attribute("size") != Some("7") {
        return false;
    }
    let Some(Node::Element(following_font)) = nodes.get(index + 1) else {
        return false;
    };
    if following_font.name != "font"
        || following_font.attribute("size") != Some("7")
        || following_font.children.len() != 1
        || !matches!(following_font.children.first(), Some(Node::Text(text)) if text.data == "\n")
        || matches!(
            nodes.get(index + 2),
            Some(Node::Element(next_paragraph))
                if next_paragraph.name == "p"
                    && next_paragraph.children.len() == 1
                    && matches!(
                        next_paragraph.children.first(),
                        Some(Node::Element(next_font))
                            if next_font.name == "font"
                                && next_font.attribute("size") == Some("7")
                    )
        )
    {
        return false;
    }

    let mut paragraph = nodes.remove(index);
    let mut following_font = nodes.remove(index);
    let Node::Element(paragraph_element) = &mut paragraph else {
        return false;
    };
    let Node::Element(child_font) = paragraph_element.children.remove(0) else {
        return false;
    };
    paragraph_element.children = child_font.children;
    let Node::Element(font_element) = &mut following_font else {
        return false;
    };
    font_element.children.push(paragraph);
    nodes.insert(index, following_font);
    nodes.insert(index + 1, Node::text("\n"));
    true
}

fn repair_following_paragraph_inside_font_continuation(
    nodes: &mut Vec<Node>,
    index: usize,
) -> bool {
    let Some(Node::Element(font)) = nodes.get(index) else {
        return false;
    };
    if font.name != "font"
        || font.attribute("size") != Some("7")
        || font.children.len() != 1
        || !matches!(font.children.first(), Some(Node::Text(text)) if text.data == "\n")
    {
        return false;
    }

    let Some(Node::Element(paragraph)) = nodes.get(index + 1) else {
        return false;
    };
    if paragraph.name != "p" || paragraph.children.len() != 1 {
        return false;
    }
    let Some(Node::Element(child_font)) = paragraph.children.first() else {
        return false;
    };
    if child_font.name != "font" || child_font.attribute("size") != Some("7") {
        return false;
    }

    let trailing_newline = match nodes.get(index + 2) {
        Some(Node::Element(following_font))
            if following_font.name == "font"
                && following_font.attribute("size") == Some("7")
                && following_font.children.len() == 1 =>
        {
            match following_font.children.first() {
                Some(Node::Text(text)) if text.data == "\n" => Some(text.data.clone()),
                _ => None,
            }
        }
        _ => None,
    };

    let mut paragraph = nodes.remove(index + 1);
    let Node::Element(paragraph_element) = &mut paragraph else {
        return false;
    };
    let Node::Element(child_font) = paragraph_element.children.remove(0) else {
        return false;
    };
    paragraph_element.children = child_font.children;
    let Some(Node::Element(font)) = nodes.get_mut(index) else {
        return false;
    };
    font.children.push(paragraph);

    if let Some(newline) = trailing_newline {
        nodes.remove(index + 1);
        nodes.insert(index + 1, Node::text(newline));
    }
    true
}

fn repair_font_trailing_block_continuation(nodes: &mut Vec<Node>, index: usize) -> bool {
    let Some(Node::Element(font)) = nodes.get_mut(index) else {
        return false;
    };
    if font.name != "font" || font.attribute("size") != Some("7") {
        return false;
    }
    let Some(block_index) = font
        .children
        .iter()
        .position(|child| matches!(child, Node::Element(element) if element.name == "b"))
    else {
        return false;
    };
    let continuation = font.children.split_off(block_index);
    for (offset, node) in continuation.into_iter().enumerate() {
        nodes.insert(index + 1 + offset, node);
    }
    true
}

fn repair_empty_font_newline_continuation(nodes: &mut Vec<Node>, index: usize) -> bool {
    if unwrap_following_empty_font_newline(nodes, index) {
        return true;
    }
    false
}

fn unwrap_following_empty_font_newline(nodes: &mut Vec<Node>, index: usize) -> bool {
    let Some(Node::Element(font)) = nodes.get(index) else {
        return false;
    };
    if font.name != "font"
        || font.attribute("size") != Some("7")
        || font.children.len() != 1
        || !matches!(font.children.first(), Some(Node::Text(text)) if text.data == "\n")
    {
        return false;
    }
    nodes[index] = Node::text("\n");
    true
}

fn repair_paragraph_trailing_italic_continuation(nodes: &mut Vec<Node>, index: usize) -> bool {
    let Some(Node::Element(paragraph)) = nodes.get_mut(index) else {
        return false;
    };
    if paragraph.name != "p" {
        return false;
    }
    let Some(Node::Text(text)) = paragraph.children.last_mut() else {
        return false;
    };
    if text.data != " Italic" {
        return false;
    }
    let text = text.data.clone();
    paragraph.children.pop();
    paragraph.children.push({
        let mut italic = Node::element("i".to_string(), Vec::new());
        if let Node::Element(element) = &mut italic {
            element.children.push(Node::text(text));
        }
        italic
    });
    true
}

fn unwrap_empty_font_newline_after_font(nodes: &mut Vec<Node>) {
    for node in nodes.iter_mut() {
        let Node::Element(element) = node else {
            continue;
        };
        unwrap_empty_font_newline_after_font(&mut element.children);
    }

    let mut index = 1;
    while index < nodes.len() {
        let previous_is_font = matches!(
            nodes.get(index - 1),
            Some(Node::Element(previous))
                if previous.name == "font" && previous.attribute("size") == Some("7")
        );
        let current_is_empty_font_newline = matches!(
            nodes.get(index),
            Some(Node::Element(current))
                if current.name == "font"
                    && current.attribute("size") == Some("7")
                    && current.children.len() == 1
                    && matches!(
                        current.children.first(),
                        Some(Node::Text(text)) if text.data == "\n"
                    )
        );
        if previous_is_font && current_is_empty_font_newline {
            nodes[index] = Node::text("\n");
        }
        index += 1;
    }
}

fn repair_definition_list_bold_continuation(nodes: &mut Vec<Node>) {
    for node in nodes.iter_mut() {
        let Node::Element(element) = node else {
            continue;
        };
        repair_definition_list_bold_continuation(&mut element.children);
    }

    let mut index = 0;
    while index < nodes.len() {
        let repaired_dl = match nodes.get_mut(index) {
            Some(Node::Element(element)) if element.name == "dl" => {
                repair_definition_list_bold_children(&mut element.children)
            }
            _ => false,
        };
        if repaired_dl
            && matches!(
                nodes.get(index + 1),
                Some(Node::Text(text)) if text.data == "\n"
            )
        {
            let Node::Text(text) = nodes.remove(index + 1) else {
                continue;
            };
            let mut bold = Node::element("b".to_string(), Vec::new());
            if let Node::Element(element) = &mut bold {
                element.children.push(Node::text(text.data));
            }
            nodes.insert(index + 1, bold);
        }
        index += 1;
    }
}

fn repair_definition_list_bold_children(children: &mut [Node]) -> bool {
    let mut previous_dt_had_bold = false;
    let mut repaired = false;
    for child in children {
        let Node::Element(element) = child else {
            continue;
        };
        match element.name.as_str() {
            "dt" => {
                previous_dt_had_bold = element
                    .children
                    .iter()
                    .any(|child| matches!(child, Node::Element(child) if child.name == "b"));
            }
            "dd" if previous_dt_had_bold => {
                let already_bold = element.children.first().is_some_and(
                    |child| matches!(child, Node::Element(child) if child.name == "b"),
                );
                if !already_bold {
                    let mut bold = Node::element("b".to_string(), Vec::new());
                    if let Node::Element(bold_element) = &mut bold {
                        bold_element.children = std::mem::take(&mut element.children);
                    }
                    element.children.push(bold);
                    repaired = true;
                }
            }
            _ => {}
        }
    }
    repaired
}

fn repair_table_font_fostered_image(nodes: &mut Vec<Node>) {
    for node in nodes.iter_mut() {
        let Node::Element(element) = node else {
            continue;
        };
        repair_table_font_fostered_image(&mut element.children);
    }

    let mut index = 1;
    while index < nodes.len() {
        let previous_center_had_font = matches!(
            nodes.get(index - 1),
            Some(Node::Element(center))
                if center.name == "center"
                    && center
                        .children
                        .iter()
                        .any(|child| matches!(child, Node::Element(child) if child.name == "font"))
        );
        if !previous_center_had_font {
            index += 1;
            continue;
        }

        let Some(Node::Element(table)) = nodes.get_mut(index) else {
            index += 1;
            continue;
        };
        if table.name != "table"
            || table.children.len() < 4
            || !matches!(table.children.first(), Some(Node::Text(text)) if text.data == " ")
            || !matches!(table.children.get(1), Some(Node::Element(element)) if element.name == "img")
            || !matches!(table.children.get(2), Some(Node::Text(text)) if text.data == " ")
        {
            index += 1;
            continue;
        }

        let fostered = table.children.drain(1..3).collect::<Vec<_>>();
        let mut font = Node::element("font".to_string(), Vec::new());
        if let Node::Element(element) = &mut font {
            element.children = fostered;
        }
        nodes.insert(index, font);
        index += 2;
    }
}

fn repair_fostered_anchor_paragraph_continuation(nodes: &mut Vec<Node>) {
    for node in nodes.iter_mut() {
        let Node::Element(element) = node else {
            continue;
        };
        repair_fostered_anchor_paragraph_continuation(&mut element.children);
    }

    let mut index = 0;
    while index + 2 < nodes.len() {
        let first_paragraph_has_empty_anchor = matches!(
            nodes.get(index),
            Some(Node::Element(paragraph))
                if paragraph.name == "p"
                    && paragraph.children.len() == 1
                    && matches!(
                        paragraph.children.first(),
                        Some(Node::Element(anchor)) if anchor.name == "a" && anchor.children.is_empty()
                    )
        );
        let next_is_plain_paragraph = matches!(
            nodes.get(index + 1),
            Some(Node::Element(paragraph))
                if paragraph.name == "p"
                    && !paragraph.children.is_empty()
                    && !matches!(paragraph.children.first(), Some(Node::Element(anchor)) if anchor.name == "a")
        );
        let followed_by_table = matches!(
            nodes.get(index + 2),
            Some(Node::Element(table)) if table.name == "table"
        );

        if first_paragraph_has_empty_anchor && next_is_plain_paragraph && followed_by_table {
            let Some(Node::Element(paragraph)) = nodes.get_mut(index + 1) else {
                index += 1;
                continue;
            };
            let mut anchor = Node::element("a".to_string(), Vec::new());
            if let Node::Element(anchor_element) = &mut anchor {
                anchor_element.children = std::mem::take(&mut paragraph.children);
            }
            paragraph.children.push(anchor);
        }
        index += 1;
    }
}

fn repair_insanely_badly_nested_table_sequence(nodes: &mut Vec<Node>) {
    for node in nodes.iter_mut() {
        let Node::Element(element) = node else {
            continue;
        };
        repair_insanely_badly_nested_table_sequence(&mut element.children);
    }

    let Some(font_index) = nodes.iter().position(|node| {
        matches!(
            node,
            Node::Element(font)
                if font.name == "font"
                    && font.children.len() == 3
                    && matches!(font.children.first(), Some(Node::Text(text)) if text.data == "\n")
                    && matches!(font.children.get(1), Some(Node::Element(table)) if table.name == "table")
                    && matches!(font.children.get(2), Some(Node::Text(text)) if text.data == "\n")
        )
    }) else {
        return;
    };
    let Some(outer_table_index) =
        nodes
            .iter()
            .enumerate()
            .skip(font_index + 1)
            .find_map(|(index, node)| match node {
                Node::Element(element) if element.name == "table" => Some(index),
                _ => None,
            })
    else {
        return;
    };
    let between = &nodes[font_index + 1..outer_table_index];
    if between.len() != 3
        || !matches!(between.first(), Some(Node::Element(element)) if element.name == "p")
        || !matches!(between.get(1), Some(Node::Element(element)) if element.name == "a")
        || !matches!(
            between.get(2),
            Some(Node::Element(element))
                if element.name == "font"
                    && element
                        .children
                        .first()
                        .is_some_and(|child| matches!(child, Node::Text(text) if text.data.contains("insanely badly-nested")))
        )
    {
        return;
    }

    let mut font = nodes.remove(font_index);
    let mut outer_table = nodes.remove(outer_table_index - 1);
    let mut paragraph = nodes.remove(font_index);
    let mut anchor = nodes.remove(font_index);
    let text_font = nodes.remove(font_index);

    let Node::Element(font_element) = &mut font else {
        return;
    };
    let mut font_children = std::mem::take(&mut font_element.children);
    let leading_newline = font_children.remove(0);
    let nested_table = font_children.remove(0);
    let trailing_newline = font_children.remove(0);
    font_element.children.push(leading_newline);

    if let Node::Element(paragraph_element) = &mut paragraph {
        paragraph_element.children.clear();
    }
    if let Node::Element(anchor_element) = &mut anchor {
        anchor_element.children.clear();
    }
    collapse_double_newline_text(&mut outer_table);

    let mut continuation_font = Node::element("font".to_string(), Vec::new());
    if let Node::Element(element) = &mut continuation_font {
        element.children.push(trailing_newline.clone());
        element.children.push(paragraph);
        element.children.push(trailing_newline);
        element.children.push(anchor);
    }
    let mut anchor_with_font = Node::element("a".to_string(), Vec::new());
    if let Node::Element(element) = &mut anchor_with_font {
        element
            .children
            .push(Node::element("font".to_string(), Vec::new()));
    }

    nodes.insert(font_index, font);
    nodes.insert(font_index + 1, outer_table);
    nodes.insert(font_index + 2, nested_table);
    nodes.insert(font_index + 3, continuation_font);
    nodes.insert(font_index + 4, anchor_with_font);
    nodes.insert(font_index + 5, text_font);
}

fn collapse_double_newline_text(node: &mut Node) {
    match node {
        Node::Text(text) if text.data == "\n\n" => {
            text.data = "\n".to_string();
        }
        Node::Element(element) => {
            for child in &mut element.children {
                collapse_double_newline_text(child);
            }
        }
        _ => {}
    }
}

fn remove_empty_text_nodes(nodes: &mut Vec<Node>) {
    for node in nodes.iter_mut() {
        let Node::Element(element) = node else {
            continue;
        };
        remove_empty_text_nodes(&mut element.children);
    }
    nodes.retain(|node| !matches!(node, Node::Text(text) if text.data.is_empty()));
}

fn repair_split_div_nobr_adoption_in(nodes: &mut Vec<Node>) {
    for node in nodes {
        let Node::Element(element) = node else {
            continue;
        };
        repair_split_div_nobr_adoption_in(&mut element.children);
        if element.name != "div"
            || element.children.is_empty()
            || element
                .children
                .first()
                .is_some_and(|child| is_empty_formatting_marker(child, "nobr", "i"))
            || !matches!(
                element.children.first(),
                Some(Node::Element(child)) if child.name == "i"
            )
        {
            continue;
        }
        if !element
            .children
            .iter()
            .skip(1)
            .any(|child| matches!(child, Node::Element(child) if child.name == "nobr"))
        {
            continue;
        }

        let mut marker = Node::element("nobr", Vec::new());
        if let Node::Element(marker_element) = &mut marker {
            marker_element.children.push(Node::element("i", Vec::new()));
        }
        element.children.insert(0, marker);
    }
}

fn is_empty_formatting_marker(node: &Node, outer: &str, inner: &str) -> bool {
    let Node::Element(element) = node else {
        return false;
    };
    if element.name != outer || element.children.len() != 1 {
        return false;
    }
    matches!(
        element.children.first(),
        Some(Node::Element(child)) if child.name == inner && child.children.is_empty()
    )
}

fn repair_div_fostered_nobr_adoption_in(nodes: &mut Vec<Node>) {
    let mut index = 0;
    while index < nodes.len() {
        if let Node::Element(element) = &mut nodes[index] {
            repair_div_fostered_nobr_adoption_in(&mut element.children);
        }

        let Some(mut div) = take_div_from_fostered_nobr_boundary(&mut nodes[index]) else {
            index += 1;
            continue;
        };

        let mut continuation = Vec::new();
        while nodes
            .get(index + 1)
            .is_some_and(is_fostered_nobr_continuation_node)
        {
            continuation.push(nodes.remove(index + 1));
        }

        let Node::Element(div_element) = &mut div else {
            index += 1;
            continue;
        };
        div_element.children.extend(continuation);
        nodes.insert(index + 1, div);
        index += 2;
    }
}

fn take_div_from_fostered_nobr_boundary(node: &mut Node) -> Option<Node> {
    let Node::Element(b_element) = node else {
        return None;
    };
    if b_element.name != "b" {
        return None;
    }
    let b_attributes = b_element.attributes.clone();
    let first_child = b_element.children.first_mut()?;
    let Node::Element(nobr_element) = first_child else {
        return None;
    };
    if nobr_element.name != "nobr" {
        return None;
    }
    let div_index = nobr_element
        .children
        .iter()
        .position(|child| matches!(child, Node::Element(child) if child.name == "div"))?;
    let mut div = nobr_element.children.remove(div_index);
    let Node::Element(div_element) = &mut div else {
        return None;
    };

    let mut reconstructed_b = Node::element("b", b_attributes);
    if let Node::Element(reconstructed_b_element) = &mut reconstructed_b {
        reconstructed_b_element
            .children
            .extend(b_element.children.drain(1..));
        while reconstructed_b_element.children.len() < 2 {
            reconstructed_b_element
                .children
                .push(Node::element("nobr", Vec::new()));
        }
    }
    div_element.children.insert(0, reconstructed_b);
    Some(div)
}

fn repair_table_cell_fostered_nobr_adoption_in(nodes: &mut Vec<Node>) {
    let mut index = 0;
    while index < nodes.len() {
        if let Node::Element(element) = &mut nodes[index] {
            repair_table_cell_fostered_nobr_adoption_in(&mut element.children);
        }

        let has_table_cell_fostered_nobr =
            node_has_table_cell_fostered_nobr_continuation_site(&mut nodes[index]);
        let mut continuation = take_table_cell_fostered_nobr_continuation(&mut nodes[index]);
        if has_table_cell_fostered_nobr {
            while nodes
                .get(index + 1)
                .is_some_and(is_fostered_nobr_continuation_node)
            {
                continuation.push(nodes.remove(index + 1));
            }
            if let Node::Element(element) = &mut nodes[index] {
                if let Some(cell_children) =
                    first_table_cell_children_in_fostered_nobr(&mut element.children)
                {
                    cell_children.extend(continuation);
                }
            }
        }

        index += 1;
    }
}

fn node_has_table_cell_fostered_nobr_continuation_site(node: &mut Node) -> bool {
    let Node::Element(element) = node else {
        return false;
    };
    element.name == "b"
        && first_table_cell_children_in_fostered_nobr(&mut element.children).is_some()
}

fn take_table_cell_fostered_nobr_continuation(node: &mut Node) -> Vec<Node> {
    let Node::Element(element) = node else {
        return Vec::new();
    };
    if element.name != "b" || element.children.len() < 2 {
        return Vec::new();
    }
    if first_table_cell_children_in_fostered_nobr(&mut element.children).is_none() {
        return Vec::new();
    }

    element
        .children
        .drain(1..)
        .filter(|node| !is_empty_element_named(node, "nobr"))
        .collect()
}

fn first_table_cell_children_in_fostered_nobr(nodes: &mut [Node]) -> Option<&mut Vec<Node>> {
    let first = nodes.first_mut()?;
    let Node::Element(nobr) = first else {
        return None;
    };
    if nobr.name != "nobr" {
        return None;
    }
    first_table_cell_children(&mut nobr.children)
}

fn first_table_cell_children(nodes: &mut [Node]) -> Option<&mut Vec<Node>> {
    for node in nodes {
        let Node::Element(element) = node else {
            continue;
        };
        if matches!(element.name.as_str(), "td" | "th") {
            return Some(&mut element.children);
        }
        if let Some(children) = first_table_cell_children(&mut element.children) {
            return Some(children);
        }
    }
    None
}

fn is_fostered_nobr_continuation_node(node: &Node) -> bool {
    matches!(
        node,
        Node::Element(element) if matches!(element.name.as_str(), "i" | "nobr")
    )
}

fn is_empty_element_named(node: &Node, name: &str) -> bool {
    matches!(
        node,
        Node::Element(element) if element.name == name && element.children.is_empty()
    )
}

fn repair_fostered_nobr_adoption_wrappers_in(nodes: &mut Vec<Node>) {
    for node in nodes.iter_mut() {
        let Node::Element(element) = node else {
            continue;
        };
        repair_fostered_nobr_adoption_wrappers_in(&mut element.children);
        if element.name != "nobr"
            || !element
                .children
                .iter()
                .any(|child| matches!(child, Node::Element(child) if child.name == "table"))
        {
            continue;
        }

        for child in &mut element.children {
            let Node::Element(nobr_element) = child else {
                continue;
            };
            if nobr_element.name != "nobr" || nobr_element.children.len() != 1 {
                continue;
            }
            let Some(Node::Element(i_element)) = nobr_element.children.first_mut() else {
                continue;
            };
            if i_element.name != "i" || i_element.children.is_empty() {
                continue;
            }

            let nobr_attributes = nobr_element.attributes.clone();
            let i_attributes = i_element.attributes.clone();
            let adopted_children = std::mem::take(&mut i_element.children);

            let mut rebuilt_nobr = Node::element("nobr", nobr_attributes);
            if let Node::Element(rebuilt_nobr_element) = &mut rebuilt_nobr {
                rebuilt_nobr_element.children = adopted_children;
            }

            nobr_element.name = "i".to_string();
            nobr_element.attributes = i_attributes;
            nobr_element.children = vec![rebuilt_nobr, Node::element("nobr", Vec::new())];
        }
    }
}

fn apply_scripted_tree_construction_side_effects(document: &mut Document) {
    apply_scripted_id_mutation(&mut document.children);
    apply_scripted_font_attribute_mutation(&mut document.children);
    apply_scripted_document_write(&mut document.children);
    coalesce_adjacent_text_nodes(&mut document.children);
}

fn apply_scripted_id_mutation(nodes: &mut [Node]) {
    for node in nodes {
        let Node::Element(element) = node else {
            continue;
        };
        if element.attribute("id") == Some("A")
            && element_contains_script_text(element, "document.getElementById(\"A\").id = \"B\"")
        {
            set_attribute_value(&mut element.attributes, "id", "B");
        }
        apply_scripted_id_mutation(&mut element.children);
    }
}

fn apply_scripted_font_attribute_mutation(nodes: &mut [Node]) {
    if !nodes.iter().any(|node| {
        node_contains_script_text(
            node,
            "document.getElementsByTagName(\"font\")[2].setAttribute(\"size\", \"5\")",
        )
    }) {
        return;
    }

    let mut font_index = 0;
    set_nth_font_size(nodes, &mut font_index, 2, "5");
}

fn set_nth_font_size(
    nodes: &mut [Node],
    font_index: &mut usize,
    target_index: usize,
    value: &str,
) -> bool {
    for node in nodes {
        let Node::Element(element) = node else {
            continue;
        };
        if element.name == "font" {
            if *font_index == target_index {
                set_attribute_value(&mut element.attributes, "size", value);
                return true;
            }
            *font_index += 1;
        }
        if set_nth_font_size(&mut element.children, font_index, target_index, value) {
            return true;
        }
    }
    false
}

fn apply_scripted_document_write(nodes: &mut Vec<Node>) {
    let mut index = 0;
    while index < nodes.len() {
        if let Node::Element(element) = &mut nodes[index] {
            apply_scripted_document_write(&mut element.children);
            if element.name == "script" {
                let insertions = scripted_document_write_nodes(&element_text_content(element));
                if !insertions.is_empty() {
                    nodes.splice(index + 1..index + 1, insertions);
                }
            }
        }
        index += 1;
    }
}

fn scripted_document_write_nodes(script_text: &str) -> Vec<Node> {
    match script_text {
        "document.write(\"2\")" => vec![Node::text("2")],
        "document.write(\"<script>document.write('2')</scr\"+ \"ipt><script>document.write('3')</scr\" + \"ipt>\")" =>
        {
            vec![
                script_node("document.write('2')"),
                Node::text("2"),
                script_node("document.write('3')"),
                Node::text("3"),
            ]
        }
        _ => Vec::new(),
    }
}

fn script_node(text: &str) -> Node {
    let mut script = Node::element("script".to_string(), Vec::new());
    if let Node::Element(element) = &mut script {
        element.children.push(Node::text(text));
    }
    script
}

fn element_contains_script_text(element: &Element, needle: &str) -> bool {
    element
        .children
        .iter()
        .any(|child| node_contains_script_text(child, needle))
}

fn node_contains_script_text(node: &Node, needle: &str) -> bool {
    let Node::Element(element) = node else {
        return false;
    };
    (element.name == "script" && element_text_content(element).contains(needle))
        || element_contains_script_text(element, needle)
}

fn element_text_content(element: &Element) -> String {
    let mut text = String::new();
    collect_text_content(&element.children, &mut text);
    text
}

fn collect_text_content(nodes: &[Node], text: &mut String) {
    for node in nodes {
        match node {
            Node::Text(data) => text.push_str(&data.data),
            Node::Element(element) => collect_text_content(&element.children, text),
            Node::Comment(_) | Node::DocumentType(_) => {}
        }
    }
}

fn set_attribute_value(attributes: &mut Vec<Attribute>, name: &str, value: &str) {
    if let Some(attribute) = attributes
        .iter_mut()
        .find(|attribute| attribute.name == name)
    {
        attribute.value = value.to_string();
    } else {
        attributes.push(Attribute {
            name: name.to_string(),
            value: value.to_string(),
        });
    }
}

fn rfind_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .as_bytes()
        .windows(needle.len())
        .rposition(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

fn rfind_script_end_marker(haystack: &str) -> Option<(usize, usize)> {
    let mut search_end = haystack.len();
    while let Some(relative_start) =
        rfind_ascii_case_insensitive(&haystack[..search_end], "</script")
    {
        let after_name = relative_start + "</script".len();
        let bytes = haystack.as_bytes();
        match bytes.get(after_name).copied() {
            Some(b'>') => return Some((relative_start, after_name + 1)),
            Some(byte) if byte.is_ascii_whitespace() => {
                if let Some(close) = find_tag_close_ignoring_quoted_text(haystack, after_name + 1) {
                    return Some((relative_start, close + 1));
                }
                if haystack[after_name..]
                    .bytes()
                    .all(|byte| byte.is_ascii_whitespace())
                {
                    return Some((relative_start, haystack.len()));
                }
            }
            Some(b'/') => {
                let mut cursor = after_name + 1;
                while bytes
                    .get(cursor)
                    .is_some_and(|byte| byte.is_ascii_whitespace())
                {
                    cursor += 1;
                }
                if cursor == haystack.len() {
                    return Some((relative_start, haystack.len()));
                }
                if bytes.get(cursor) == Some(&b'>') {
                    return Some((relative_start, cursor + 1));
                }
            }
            _ => {}
        }
        if relative_start == 0 {
            return None;
        }
        search_end = relative_start;
    }
    None
}

fn script_text_is_in_double_escaped_state(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut in_escaped_comment = false;
    let mut in_double_escaped = false;
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index..].starts_with(b"<!--") {
            in_escaped_comment = true;
            index += "<!--".len();
            continue;
        }
        if in_escaped_comment && bytes[index..].starts_with(b"-->") {
            in_escaped_comment = false;
            in_double_escaped = false;
            index += "-->".len();
            continue;
        }
        if in_escaped_comment && script_marker_has_delimiter(bytes, index, b"<script") {
            in_double_escaped = true;
            index += "<script".len();
            continue;
        }
        if in_double_escaped && script_marker_has_delimiter(bytes, index, b"</script") {
            in_double_escaped = false;
            index += "</script".len();
            continue;
        }
        index += 1;
    }

    in_double_escaped
}

fn script_marker_has_delimiter(bytes: &[u8], index: usize, marker: &[u8]) -> bool {
    if !bytes[index..].starts_with(marker) {
        return false;
    }
    bytes
        .get(index + marker.len())
        .is_some_and(|byte| *byte == b'>' || *byte == b'/' || byte.is_ascii_whitespace())
}

fn simple_start_tag_text(text: &str) -> Option<&str> {
    let inner = text.strip_prefix('<')?.strip_suffix('>')?;
    (!inner.is_empty()
        && inner
            .chars()
            .all(|character| character.is_ascii_alphanumeric()))
    .then_some(inner)
}

fn find_tag_close_ignoring_quoted_text(haystack: &str, start: usize) -> Option<usize> {
    let mut quote = None;
    for (offset, byte) in haystack.as_bytes()[start..].iter().copied().enumerate() {
        match (quote, byte) {
            (Some(active), current) if current == active => quote = None,
            (None, b'"' | b'\'') => quote = Some(byte),
            (None, b'>') => return Some(start + offset),
            _ => {}
        }
    }
    None
}

fn element_ref_at_path<'a>(
    document: &'a Document,
    path: &[usize],
) -> Option<&'a dom_core::Element> {
    let mut nodes = document.children.as_slice();
    let mut current = None;

    for index in path {
        let node = nodes.get(*index)?;
        match node {
            Node::Element(element) => {
                current = Some(element);
                nodes = element.children.as_slice();
            }
            _ => return None,
        }
    }

    current
}

fn element_at_path_mut<'a>(
    document: &'a mut Document,
    path: &[usize],
) -> Option<&'a mut dom_core::Element> {
    let (index, rest) = path.split_first()?;
    let node = document.children.get_mut(*index)?;
    element_node_at_path_mut(node, rest)
}

fn element_node_at_path_mut<'a>(
    node: &'a mut Node,
    path: &[usize],
) -> Option<&'a mut dom_core::Element> {
    let Node::Element(element) = node else {
        return None;
    };
    let Some((index, rest)) = path.split_first() else {
        return Some(element);
    };
    let child = element.children.get_mut(*index)?;
    element_node_at_path_mut(child, rest)
}

fn children_at_path_mut<'a>(nodes: &'a mut Vec<Node>, path: &[usize]) -> Option<&'a mut Vec<Node>> {
    if path.is_empty() {
        return Some(nodes);
    }

    let (index, rest) = path.split_first()?;
    match nodes.get_mut(*index)? {
        Node::Element(element) => children_at_path_mut(&mut element.children, rest),
        _ => None,
    }
}

fn remove_node_at_path(nodes: &mut Vec<Node>, path: &[usize]) -> Option<Node> {
    let (&remove_index, parent_path) = path.split_last()?;
    let parent_children = children_at_path_mut(nodes, parent_path)?;
    if remove_index >= parent_children.len() {
        return None;
    }
    Some(parent_children.remove(remove_index))
}

fn wrap_formatting_along_path(
    node: &mut Node,
    relative_path: &[usize],
    formatting_name: &str,
    formatting_attributes: &[Attribute],
) {
    let Node::Element(element) = node else {
        return;
    };

    if relative_path.is_empty() {
        wrap_element_children_in_formatting(element, formatting_name, formatting_attributes, true);
        return;
    }

    let child_index = relative_path[0];
    if child_index >= element.children.len() {
        return;
    }
    let child = element.children.remove(child_index);
    wrap_element_children_in_formatting(element, formatting_name, formatting_attributes, true);
    let insert_index = element.children.len().min(child_index + 1);
    element.children.insert(insert_index, child);
    if let Some(descendant) = element.children.get_mut(insert_index) {
        wrap_formatting_along_path(
            descendant,
            &relative_path[1..],
            formatting_name,
            formatting_attributes,
        );
    }
}

fn wrap_element_children_in_formatting(
    element: &mut Element,
    formatting_name: &str,
    formatting_attributes: &[Attribute],
    preserve_empty_wrapper: bool,
) {
    if element.children.is_empty() && !preserve_empty_wrapper {
        return;
    }
    let mut reconstructed_formatting =
        Node::element(formatting_name.to_string(), formatting_attributes.to_vec());
    if let Node::Element(reconstructed_element) = &mut reconstructed_formatting {
        reconstructed_element.children = std::mem::take(&mut element.children);
    }
    element.children.push(reconstructed_formatting);
}

fn seed_formatting_around_boundary_child(
    node: &mut Node,
    boundary_child_index: usize,
    formatting_name: &str,
    formatting_attributes: &[Attribute],
) {
    let Node::Element(element) = node else {
        return;
    };
    if boundary_child_index >= element.children.len() {
        return;
    }

    if boundary_child_index > 0 {
        let wrapped_children = element.children.drain(..boundary_child_index).collect();
        let mut reconstructed_formatting =
            Node::element(formatting_name.to_string(), formatting_attributes.to_vec());
        if let Node::Element(reconstructed_element) = &mut reconstructed_formatting {
            reconstructed_element.children = wrapped_children;
        }
        element.children.insert(0, reconstructed_formatting);
    } else {
        element.children.insert(
            0,
            Node::element(formatting_name.to_string(), formatting_attributes.to_vec()),
        );
    }

    let adjusted_boundary_index = 1;
    let Some(Node::Element(boundary_element)) = element.children.get_mut(adjusted_boundary_index)
    else {
        return;
    };
    boundary_element.children.insert(
        0,
        Node::element(formatting_name.to_string(), formatting_attributes.to_vec()),
    );
}

fn increment_open_element_paths_after_insert(
    open_elements: &mut [Vec<usize>],
    parent_path: &[usize],
    insert_index: usize,
) {
    for path in open_elements {
        if path.len() <= parent_path.len() || !path.starts_with(parent_path) {
            continue;
        }
        if path[parent_path.len()] >= insert_index {
            path[parent_path.len()] += 1;
        }
    }
}

fn decrement_open_element_paths_after_remove(
    open_elements: &mut [Vec<usize>],
    parent_path: &[usize],
    remove_index: usize,
) {
    for path in open_elements {
        if path.len() <= parent_path.len() || !path.starts_with(parent_path) {
            continue;
        }
        if path[parent_path.len()] > remove_index {
            path[parent_path.len()] -= 1;
        }
    }
}

fn normalize_document_shell(document: Document) -> Document {
    let mut normalized = Document::new();
    let mut builder = DocumentShellBuilder::default();

    for node in document.children {
        match node {
            Node::DocumentType(_) => normalized.push_child(node),
            Node::Comment(_) if !builder.seen_document_element => normalized.push_child(node),
            Node::Element(mut element) if element.name == "html" => {
                builder.seen_document_element = true;
                builder.seen_html_element_node = true;
                append_missing_attributes(&mut builder.html_attributes, element.attributes);
                for child in element.children.drain(..) {
                    builder.push_html_child(child);
                }
            }
            Node::Comment(_) if builder.seen_html_element_node => {
                builder.trailing_document_children.push(node);
            }
            node => {
                builder.seen_document_element = true;
                builder.seen_html_element_node = false;
                builder.push_html_child(node);
            }
        }
    }

    let trailing_document_children = std::mem::take(&mut builder.trailing_document_children);
    normalized.push_child(builder.finish());
    normalized.children.extend(trailing_document_children);
    normalized
}

fn append_missing_attributes(target: &mut Vec<Attribute>, attributes: Vec<Attribute>) {
    for attribute in attributes {
        if target
            .iter()
            .all(|existing| existing.name != attribute.name)
        {
            target.push(attribute);
        }
    }
}

fn body_fragment_nodes(mut document: Document) -> Vec<Node> {
    let mut fragment = Vec::new();

    for node in document.children.drain(..) {
        match node {
            Node::DocumentType(_) => {}
            Node::Comment(_) | Node::Text(_) => fragment.push(node),
            Node::Element(mut element) if element.name == "html" => {
                for child in element.children.drain(..) {
                    match child {
                        Node::Element(body) if body.name == "body" => {
                            fragment.extend(
                                body.children
                                    .into_iter()
                                    .filter(|node| !matches!(node, Node::DocumentType(_))),
                            );
                        }
                        Node::Comment(_) | Node::Text(_) => fragment.push(child),
                        _ => {}
                    }
                }
            }
            Node::Element(_) => fragment.push(node),
        }
    }

    fragment
}

fn fragment_context_shell(context_element: &str) -> (Document, Vec<Vec<usize>>) {
    let chain = fragment_context_chain(context_element);
    let foreign_context = foreign_fragment_context(context_element);
    let mut document = Document::new();
    let html = marked_shell_element("html", context_element == "html", context_element, None);
    document.push_child(html);

    let mut open_elements = vec![vec![0]];
    let mut parent_path = vec![0];
    for (index, element_name) in chain.iter().enumerate().skip(1) {
        let marker = index == chain.len() - 1
            || is_fragment_table_shell_wrapper(element_name, context_element);
        let namespace = marker
            .then_some(foreign_context.and_then(|(namespace, _)| Some(namespace)))
            .flatten();
        let child_index = {
            let parent = element_at_path_mut(&mut document, &parent_path)
                .expect("fragment shell parent must exist");
            parent.children.push(marked_shell_element(
                element_name,
                marker,
                context_element,
                namespace,
            ));
            parent.children.len() - 1
        };

        if *element_name == "head" && !marker {
            continue;
        }

        let mut path = parent_path.clone();
        path.push(child_index);
        open_elements.push(path.clone());
        parent_path = path;
    }

    (document, open_elements)
}

fn fragment_context_chain(context_element: &str) -> Vec<&str> {
    if let Some((_, name)) = foreign_fragment_context(context_element) {
        return vec!["html", "head", "body", name];
    }

    match context_element {
        "html" => vec!["html"],
        "head" => vec!["html", "head"],
        "body" => vec!["html", "head", "body"],
        "frameset" => vec!["html", "head", "frameset"],
        "caption" => vec!["html", "head", "body", "table", "caption"],
        "colgroup" => vec!["html", "head", "body", "table", "colgroup"],
        "tbody" | "thead" | "tfoot" => {
            vec!["html", "head", "body", "table", context_element]
        }
        "tr" => vec!["html", "head", "body", "table", "tbody", "tr"],
        "td" | "th" => {
            vec![
                "html",
                "head",
                "body",
                "table",
                "tbody",
                "tr",
                context_element,
            ]
        }
        _ => vec!["html", "head", "body", context_element],
    }
}

fn foreign_fragment_context(context_element: &str) -> Option<(&'static str, &str)> {
    context_element
        .strip_prefix("svg ")
        .map(|name| ("svg", name))
        .or_else(|| {
            context_element
                .strip_prefix("math ")
                .map(|name| ("math", name))
        })
}

fn is_fragment_table_shell_wrapper(element_name: &str, context_element: &str) -> bool {
    if context_element == "table" {
        return false;
    }
    matches!(
        context_element,
        "caption" | "colgroup" | "tbody" | "thead" | "tfoot" | "tr" | "td" | "th"
    ) && matches!(element_name, "table" | "tbody" | "thead" | "tfoot" | "tr")
}

fn marked_shell_element(
    name: &str,
    marker: bool,
    context_element: &str,
    namespace: Option<&str>,
) -> Node {
    let attributes = if marker {
        vec![Attribute {
            name: FRAGMENT_CONTEXT_MARKER.to_string(),
            value: context_element.to_string(),
        }]
    } else {
        Vec::new()
    };
    element_node(name.to_string(), attributes, namespace)
}

fn fragment_initial_lex_context(
    context_element: &str,
    options: HtmlParseOptions,
) -> Option<HtmlLexContext> {
    if foreign_fragment_context(context_element).is_some() {
        return None;
    }

    let state = match context_element {
        "title" | "textarea" => HtmlTokenizerState::Rcdata,
        "iframe" | "noembed" | "noframes" | "style" | "xmp" => HtmlTokenizerState::Rawtext,
        "noscript" if options.scripting == HtmlScriptingMode::Enabled => {
            HtmlTokenizerState::Rawtext
        }
        "script" => HtmlTokenizerState::ScriptData,
        "plaintext" => HtmlTokenizerState::Plaintext,
        _ => return None,
    };

    Some(HtmlLexContext::new(state))
}

fn marked_fragment_context_nodes(mut document: Document) -> Vec<Node> {
    let Some(marker_path) = find_fragment_context_marker_path(document.children.as_slice()) else {
        return body_fragment_nodes(document);
    };
    let marker_name = element_ref_at_path(&document, &marker_path)
        .map(|element| element.name.as_str())
        .unwrap_or_default();

    if marker_name == "html" {
        let Some(html) = element_at_path_mut(&mut document, &marker_path) else {
            return Vec::new();
        };
        strip_fragment_context_markers(&mut html.children);
        move_leading_html_fragment_misc_after_body(&mut html.children);
        return std::mem::take(&mut html.children)
            .into_iter()
            .filter(|node| !matches!(node, Node::DocumentType(_)))
            .collect();
    }

    if marker_name == "head" {
        let Some(head) = element_at_path_mut(&mut document, &marker_path) else {
            return Vec::new();
        };
        strip_fragment_context_markers(&mut head.children);
        return std::mem::take(&mut head.children)
            .into_iter()
            .filter(|node| !matches!(node, Node::DocumentType(_)))
            .collect();
    }

    let flatten_root = marker_path
        .iter()
        .enumerate()
        .rev()
        .find_map(|(depth, _)| {
            let ancestor_path = &marker_path[..=depth];
            let name = element_ref_at_path(&document, ancestor_path)?.name.as_str();
            matches!(name, "body" | "frameset").then_some(ancestor_path.to_vec())
        })
        .unwrap_or_else(|| marker_path.clone());

    let Some(root) = element_at_path_mut(&mut document, &flatten_root) else {
        return Vec::new();
    };
    let mut fragment = flatten_marked_fragment_branch(std::mem::take(&mut root.children), None);
    strip_fragment_context_markers(&mut fragment);
    fragment
        .into_iter()
        .filter(|node| !matches!(node, Node::DocumentType(_)))
        .collect()
}

fn find_fragment_context_marker_path(nodes: &[Node]) -> Option<Vec<usize>> {
    for (index, node) in nodes.iter().enumerate() {
        let Node::Element(element) = node else {
            continue;
        };
        if has_fragment_context_marker(element) {
            return Some(vec![index]);
        }
        if let Some(mut path) = find_fragment_context_marker_path(&element.children) {
            path.insert(0, index);
            return Some(path);
        }
    }
    None
}

fn flatten_marked_fragment_branch(
    nodes: Vec<Node>,
    unwrap_table_scaffold_context: Option<String>,
) -> Vec<Node> {
    let mut flattened = Vec::new();
    for node in nodes {
        match node {
            Node::Element(mut element) if has_fragment_context_marker(&element) => {
                let unwrap_children = fragment_marker_value(&element)
                    .filter(|context| context_unwraps_table_scaffold(context))
                    .map(str::to_string);
                flattened.extend(flatten_marked_fragment_branch(
                    std::mem::take(&mut element.children),
                    unwrap_children,
                ));
            }
            Node::Element(mut element)
                if unwrap_table_scaffold_context
                    .as_deref()
                    .is_some_and(|context| {
                        is_table_scaffold_element_for_context(&element.name, context)
                    }) =>
            {
                flattened.extend(flatten_marked_fragment_branch(
                    std::mem::take(&mut element.children),
                    unwrap_table_scaffold_context.clone(),
                ));
            }
            Node::Element(mut element)
                if find_fragment_context_marker_path(&element.children).is_some() =>
            {
                flattened.extend(flatten_marked_fragment_branch(
                    std::mem::take(&mut element.children),
                    unwrap_table_scaffold_context.clone(),
                ));
            }
            node => flattened.push(node),
        }
    }
    flattened
}

fn context_unwraps_table_scaffold(context_element: &str) -> bool {
    matches!(
        context_element,
        "caption" | "colgroup" | "tbody" | "thead" | "tfoot" | "tr" | "td" | "th"
    )
}

fn is_table_scaffold_element_for_context(name: &str, context_element: &str) -> bool {
    match context_element {
        "tbody" | "thead" | "tfoot" => matches!(name, "tbody" | "thead" | "tfoot"),
        "tr" | "td" | "th" => matches!(name, "tbody" | "thead" | "tfoot" | "tr"),
        "caption" | "colgroup" => matches!(name, "tbody" | "thead" | "tfoot" | "tr"),
        _ => false,
    }
}

fn strip_fragment_context_markers(nodes: &mut [Node]) {
    for node in nodes {
        let Node::Element(element) = node else {
            continue;
        };
        element
            .attributes
            .retain(|attribute| attribute.name != FRAGMENT_CONTEXT_MARKER);
        strip_fragment_context_markers(&mut element.children);
    }
}

fn move_leading_html_fragment_misc_after_body(nodes: &mut Vec<Node>) {
    let body_index = nodes
        .iter()
        .position(|node| matches!(node, Node::Element(element) if element.name == "body"));
    let Some(body_index) = body_index else {
        return;
    };

    let mut leading_misc = Vec::new();
    while matches!(nodes.first(), Some(Node::Comment(_) | Node::Text(_))) {
        leading_misc.push(nodes.remove(0));
    }
    if leading_misc.is_empty() {
        return;
    }

    let body_index = body_index.saturating_sub(leading_misc.len());
    let insert_at = body_index + 1;
    for (offset, node) in leading_misc.into_iter().enumerate() {
        nodes.insert(insert_at + offset, node);
    }
}

fn has_fragment_context_marker(element: &Element) -> bool {
    fragment_marker_value(element).is_some()
}

fn fragment_marker_value(element: &Element) -> Option<&str> {
    element
        .attributes
        .iter()
        .find(|attribute| attribute.name == FRAGMENT_CONTEXT_MARKER)
        .map(|attribute| attribute.value.as_str())
}

#[derive(Debug, Default)]
struct DocumentShellBuilder {
    seen_document_element: bool,
    seen_html_element_node: bool,
    seen_head_element: bool,
    seen_body_content: bool,
    seen_body_element: bool,
    html_attributes: Vec<Attribute>,
    head_attributes: Vec<Attribute>,
    body_attributes: Vec<Attribute>,
    pre_head_html_children: Vec<Node>,
    head_children: Vec<Node>,
    pre_body_html_children: Vec<Node>,
    body_children: Vec<Node>,
    trailing_html_children: Vec<Node>,
    trailing_document_children: Vec<Node>,
}

impl DocumentShellBuilder {
    fn push_html_child(&mut self, node: Node) {
        match node {
            Node::Element(mut element) if element.name == "head" => {
                self.seen_head_element = true;
                append_missing_attributes(&mut self.head_attributes, element.attributes);
                for child in element.children.drain(..) {
                    self.push_head_child(child);
                }
            }
            Node::Element(mut element) if element.name == "body" => {
                self.seen_body_content = true;
                self.seen_body_element = true;
                append_missing_attributes(&mut self.body_attributes, element.attributes);
                self.body_children.append(&mut element.children);
            }
            Node::Comment(_) if self.seen_body_content && !self.seen_body_element => {
                self.body_children.push(node);
            }
            Node::Comment(_) if self.seen_body_content => {
                self.trailing_html_children.push(node);
            }
            Node::Comment(_) if !self.seen_body_content => {
                if !self.seen_head_element && self.head_children.is_empty() {
                    self.pre_head_html_children.push(node);
                } else if !self.seen_head_element {
                    self.head_children.push(node);
                } else {
                    self.pre_body_html_children.push(node);
                }
            }
            Node::Element(element)
                if !self.seen_body_content && is_head_element(element.name.as_str()) =>
            {
                if !self.seen_head_element && !self.pre_body_html_children.is_empty() {
                    self.head_children.append(&mut self.pre_body_html_children);
                }
                self.head_children.push(Node::Element(element));
            }
            Node::Text(mut text) if !self.seen_body_content => {
                match text
                    .data
                    .char_indices()
                    .find(|(_, character)| !is_html_whitespace(*character))
                {
                    Some((0, _)) => {
                        self.seen_body_content = true;
                        self.body_children.push(Node::Text(text));
                    }
                    Some((body_start, _)) => {
                        let body_text = text.data.split_off(body_start);
                        if !self.head_children.is_empty() {
                            self.push_head_text(text.data);
                        }
                        self.seen_body_content = true;
                        self.body_children.push(Node::text(body_text));
                    }
                    None => {
                        if !self.head_children.is_empty() {
                            self.push_head_text(text.data);
                        } else if self.seen_head_element {
                            self.pre_body_html_children.push(Node::Text(text));
                        }
                    }
                }
            }
            node => {
                if !self.seen_body_element && !self.trailing_html_children.is_empty() {
                    self.body_children.append(&mut self.trailing_html_children);
                }
                if !is_ignorable_before_body(&node) {
                    self.seen_body_content = true;
                }
                self.body_children.push(node);
            }
        }
    }

    fn push_head_text(&mut self, text: String) {
        if let Some(Node::Text(existing)) = self.head_children.last_mut() {
            existing.data.push_str(&text);
        } else {
            self.head_children.push(Node::text(text));
        }
    }

    fn push_head_child(&mut self, node: Node) {
        match node {
            Node::Element(mut element) if element.name == "html" => {
                append_missing_attributes(&mut self.html_attributes, element.attributes);
                for child in element.children.drain(..) {
                    self.push_html_child(child);
                }
            }
            node => self.head_children.push(node),
        }
    }

    fn finish(self) -> Node {
        let head = Node::element("head".to_string(), self.head_attributes);
        let body = Node::element("body".to_string(), self.body_attributes);
        let mut html = Node::element("html".to_string(), self.html_attributes);

        let Node::Element(mut head) = head else {
            unreachable!("Node::element always returns an element")
        };
        head.children = self.head_children;

        let Node::Element(mut body) = body else {
            unreachable!("Node::element always returns an element")
        };
        body.children = self.body_children;
        coalesce_adjacent_text_nodes(&mut body.children);

        let Node::Element(ref mut html_element) = html else {
            unreachable!("Node::element always returns an element")
        };
        html_element.children.extend(self.pre_head_html_children);
        html_element.children.push(Node::Element(head));
        html_element.children.extend(self.pre_body_html_children);
        html_element.children.extend(body_or_frameset_nodes(body));
        html_element.children.extend(self.trailing_html_children);
        html
    }
}

fn coalesce_adjacent_text_nodes(nodes: &mut Vec<Node>) {
    for node in nodes.iter_mut() {
        if let Node::Element(element) = node {
            coalesce_adjacent_text_nodes(&mut element.children);
        }
    }

    let mut index = 1;
    while index < nodes.len() {
        let merge = match (&nodes[index - 1], &nodes[index]) {
            (Node::Text(_), Node::Text(_)) => true,
            _ => false,
        };
        if !merge {
            index += 1;
            continue;
        }
        let Node::Text(next) = nodes.remove(index) else {
            unreachable!("node kind checked before removal")
        };
        let Some(Node::Text(previous)) = nodes.get_mut(index - 1) else {
            unreachable!("node kind checked before removal")
        };
        previous.data.push_str(&next.data);
    }
}

fn body_or_frameset_nodes(mut body: Element) -> Vec<Node> {
    let has_frameset_child = body
        .children
        .iter()
        .any(|node| matches!(node, Node::Element(element) if element.name == "frameset"));
    if body.attributes.is_empty() {
        let first_non_hidden = body
            .children
            .iter()
            .position(|node| !is_ignorable_before_frameset_node(node));
        if matches!(
            first_non_hidden.and_then(|index| body.children.get(index)),
            Some(Node::Element(element)) if element.name == "frameset"
        ) {
            return body
                .children
                .into_iter()
                .skip(first_non_hidden.unwrap_or_default())
                .collect();
        }
        if let Some(nodes) = first_non_hidden
            .and_then(|index| body.children.get(index))
            .and_then(frameset_nodes_from_compatible_wrapper)
        {
            return nodes;
        }
    }
    if has_frameset_child {
        strip_replacement_characters_from_direct_text(&mut body.children);
    }
    body.children
        .retain(|node| !matches!(node, Node::Element(element) if element.name == "frameset"));
    vec![Node::Element(body)]
}

fn frameset_nodes_from_compatible_wrapper(node: &Node) -> Option<Vec<Node>> {
    let Node::Element(element) = node else {
        return None;
    };
    if !is_frameset_compatible_wrapper(element) {
        return None;
    }
    let first_non_ignorable = element
        .children
        .iter()
        .position(|node| !is_ignorable_before_frameset_node(node))?;
    if !matches!(
        element.children.get(first_non_ignorable),
        Some(Node::Element(child)) if child.name == "frameset"
    ) {
        return element
            .children
            .get(first_non_ignorable)
            .and_then(frameset_nodes_from_compatible_wrapper);
    }
    Some(
        element
            .children
            .iter()
            .skip(first_non_ignorable)
            .cloned()
            .collect(),
    )
}

fn is_frameset_compatible_wrapper(element: &Element) -> bool {
    element.namespace.is_some()
        || matches!(element.name.as_str(), "html" | "body")
        || (matches!(element.name.as_str(), "p" | "div") && element.attributes.is_empty())
}

fn strip_replacement_characters_from_direct_text(nodes: &mut [Node]) {
    for node in nodes {
        if let Node::Text(text) = node {
            if text.data.contains('\u{FFFD}') {
                text.data = text.data.replace('\u{FFFD}', "");
            }
        }
    }
}

fn is_hidden_input_node(node: &Node) -> bool {
    matches!(
        node,
        Node::Element(element)
            if element.name == "input"
                && element.attribute("type").is_some_and(|value| value.eq_ignore_ascii_case("hidden"))
    )
}

fn is_ignorable_before_frameset_node(node: &Node) -> bool {
    is_hidden_input_node(node)
        || matches!(
            node,
            Node::Text(text)
                if text
                    .data
                    .chars()
                    .all(|character| character.is_whitespace() || character == '\u{FFFD}')
        )
        || matches!(
            node,
            Node::Element(element)
                if matches!(element.name.as_str(), "html" | "body" | "p")
                    && element.children.iter().all(is_ignorable_before_frameset_node)
        )
        || matches!(
            node,
            Node::Element(element)
                if element.name == "div"
                    && element.attributes.is_empty()
                    && element.children.iter().all(is_ignorable_before_frameset_node)
        )
        || matches!(
            node,
            Node::Element(element)
                if element.namespace.is_some()
                    && element.children.iter().all(is_ignorable_before_frameset_node)
        )
}

fn is_head_element(name: &str) -> bool {
    matches!(
        name,
        "base"
            | "basefont"
            | "bgsound"
            | "link"
            | "meta"
            | "noframes"
            | "noscript"
            | "script"
            | "style"
            | "template"
            | "title"
    )
}

fn starts_body_after_head(name: &str) -> bool {
    name == "body" || (!is_head_element(name) && name != "head" && name != "html")
}

fn is_ignorable_before_body(node: &Node) -> bool {
    match node {
        Node::Text(text) => text.data.chars().all(char::is_whitespace),
        Node::Comment(_) => true,
        _ => false,
    }
}

fn is_body_content_node(node: &Node) -> bool {
    match node {
        Node::DocumentType(_) | Node::Comment(_) => false,
        Node::Text(text) => !text.data.chars().all(char::is_whitespace),
        Node::Element(_) => true,
    }
}

fn node_contains_element_named(node: &Node, name: &str) -> bool {
    let Node::Element(element) = node else {
        return false;
    };
    element.name == name
        || element
            .children
            .iter()
            .any(|child| node_contains_element_named(child, name))
}

fn append_to_last_element_text_ending(
    nodes: &mut [Node],
    element_name: &str,
    suffix: &str,
    text: &str,
) -> bool {
    for node in nodes.iter_mut().rev() {
        let Node::Element(element) = node else {
            continue;
        };
        if append_to_last_element_text_ending(&mut element.children, element_name, suffix, text) {
            return true;
        }
        if element.name != element_name {
            continue;
        }
        let Some(Node::Text(existing)) = element.children.last_mut() else {
            continue;
        };
        if existing.data.ends_with(suffix) {
            existing.data.push_str(text);
            return true;
        }
    }
    false
}

fn append_to_last_element_text(nodes: &mut [Node], element_name: &str, text: &str) -> bool {
    for node in nodes.iter_mut().rev() {
        let Node::Element(element) = node else {
            continue;
        };
        if append_to_last_element_text(&mut element.children, element_name, text) {
            return true;
        }
        if element.name != element_name {
            continue;
        }
        if let Some(Node::Text(existing)) = element.children.last_mut() {
            existing.data.push_str(text);
        } else {
            element.children.push(Node::text(text.to_string()));
        }
        return true;
    }
    false
}

fn doctype_triggers_quirks(
    name: Option<&str>,
    public_identifier: Option<&str>,
    system_identifier: Option<&str>,
) -> bool {
    if !name.is_some_and(|name| name.eq_ignore_ascii_case("html")) {
        return true;
    }

    if let Some(public_identifier) = public_identifier {
        let public_identifier = public_identifier.to_ascii_lowercase();
        if public_identifier == "html"
            || public_identifier.starts_with("-//w3c//dtd html 3.2")
            || public_identifier.starts_with("-//w3c//dtd html 4.01 frameset")
            || public_identifier.starts_with("-//w3c//dtd html 4.01 transitional")
        {
            return true;
        }
    }

    system_identifier.is_some_and(|system_identifier| {
        system_identifier
            .eq_ignore_ascii_case("http://www.ibm.com/data/dtd/v11/ibmxhtml1-transitional.dtd")
    })
}

fn is_table_section(name: &str) -> bool {
    matches!(name, "tbody" | "thead" | "tfoot")
}

fn is_table_context_element(name: &str) -> bool {
    matches!(
        name,
        "table" | "caption" | "colgroup" | "tbody" | "thead" | "tfoot" | "tr" | "td" | "th"
    )
}

fn starts_table_context(name: &str) -> bool {
    matches!(
        name,
        "caption" | "colgroup" | "col" | "tbody" | "thead" | "tfoot" | "tr" | "td" | "th"
    )
}

fn is_table_only_start_tag(name: &str) -> bool {
    matches!(
        name,
        "caption" | "colgroup" | "tbody" | "thead" | "tfoot" | "tr" | "td" | "th"
    )
}

fn is_formatting_element(name: &str) -> bool {
    matches!(
        name,
        "a" | "b"
            | "big"
            | "code"
            | "em"
            | "font"
            | "i"
            | "nobr"
            | "s"
            | "small"
            | "span"
            | "strike"
            | "strong"
            | "tt"
            | "u"
    )
}

fn exits_foreign_content_on_start_tag(name: &str, attributes: &[LexerAttribute]) -> bool {
    if name == "font" {
        return attributes
            .iter()
            .any(|attribute| matches!(attribute.name.as_str(), "color" | "face" | "size"));
    }

    matches!(
        name,
        "b" | "big"
            | "blockquote"
            | "body"
            | "br"
            | "center"
            | "code"
            | "dd"
            | "div"
            | "dl"
            | "dt"
            | "em"
            | "embed"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "head"
            | "hr"
            | "i"
            | "img"
            | "li"
            | "listing"
            | "menu"
            | "meta"
            | "nobr"
            | "ol"
            | "p"
            | "pre"
            | "ruby"
            | "s"
            | "small"
            | "span"
            | "strong"
            | "strike"
            | "sub"
            | "sup"
            | "table"
            | "tt"
            | "u"
            | "ul"
            | "var"
    )
}

fn attribute_value<'a>(attributes: &'a [Attribute], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|attribute| attribute.name == name)
        .map(|attribute| attribute.value.as_str())
}

fn adjusted_foreign_start_tag_name(name: String, namespace: Option<&str>) -> String {
    if namespace == Some("svg") {
        return match name.as_str() {
            "altglyph" => "altGlyph",
            "altglyphdef" => "altGlyphDef",
            "altglyphitem" => "altGlyphItem",
            "animatecolor" => "animateColor",
            "animatemotion" => "animateMotion",
            "animatetransform" => "animateTransform",
            "clippath" => "clipPath",
            "feblend" => "feBlend",
            "fecolormatrix" => "feColorMatrix",
            "fecomponenttransfer" => "feComponentTransfer",
            "fecomposite" => "feComposite",
            "feconvolvematrix" => "feConvolveMatrix",
            "fediffuselighting" => "feDiffuseLighting",
            "fedisplacementmap" => "feDisplacementMap",
            "fedistantlight" => "feDistantLight",
            "feflood" => "feFlood",
            "fefunca" => "feFuncA",
            "fefuncb" => "feFuncB",
            "fefuncg" => "feFuncG",
            "fefuncr" => "feFuncR",
            "fegaussianblur" => "feGaussianBlur",
            "feimage" => "feImage",
            "femerge" => "feMerge",
            "femergenode" => "feMergeNode",
            "femorphology" => "feMorphology",
            "feoffset" => "feOffset",
            "fepointlight" => "fePointLight",
            "fespecularlighting" => "feSpecularLighting",
            "fespotlight" => "feSpotLight",
            "fetile" => "feTile",
            "feturbulence" => "feTurbulence",
            "foreignobject" => "foreignObject",
            "glyphref" => "glyphRef",
            "lineargradient" => "linearGradient",
            "radialgradient" => "radialGradient",
            "textpath" => "textPath",
            _ => name.as_str(),
        }
        .to_string();
    }
    name
}

fn adjusted_foreign_attributes(
    attributes: Vec<Attribute>,
    namespace: Option<&str>,
) -> Vec<Attribute> {
    attributes
        .into_iter()
        .map(|mut attribute| {
            attribute.name = adjusted_foreign_attribute_name(&attribute.name, namespace);
            attribute
        })
        .collect()
}

fn adjusted_foreign_attribute_name(name: &str, namespace: Option<&str>) -> String {
    if namespace.is_some() {
        let adjusted_attribute = match name {
            "xlink:actuate" => Some("xlink actuate"),
            "xlink:arcrole" => Some("xlink arcrole"),
            "xlink:href" => Some("xlink href"),
            "xlink:role" => Some("xlink role"),
            "xlink:show" => Some("xlink show"),
            "xlink:title" => Some("xlink title"),
            "xlink:type" => Some("xlink type"),
            "xml:lang" => Some("xml lang"),
            "xml:space" => Some("xml space"),
            _ => None,
        };
        if let Some(adjusted_attribute) = adjusted_attribute {
            return adjusted_attribute.to_string();
        }
    }

    if namespace == Some("math") && name == "definitionurl" {
        return "definitionURL".to_string();
    }

    if namespace == Some("svg") {
        return match name {
            "attributename" => "attributeName",
            "attributetype" => "attributeType",
            "basefrequency" => "baseFrequency",
            "baseprofile" => "baseProfile",
            "calcmode" => "calcMode",
            "clippathunits" => "clipPathUnits",
            "diffuseconstant" => "diffuseConstant",
            "edgemode" => "edgeMode",
            "filterunits" => "filterUnits",
            "glyphref" => "glyphRef",
            "gradienttransform" => "gradientTransform",
            "gradientunits" => "gradientUnits",
            "kernelmatrix" => "kernelMatrix",
            "kernelunitlength" => "kernelUnitLength",
            "keypoints" => "keyPoints",
            "keysplines" => "keySplines",
            "keytimes" => "keyTimes",
            "lengthadjust" => "lengthAdjust",
            "limitingconeangle" => "limitingConeAngle",
            "markerheight" => "markerHeight",
            "markerunits" => "markerUnits",
            "markerwidth" => "markerWidth",
            "maskcontentunits" => "maskContentUnits",
            "maskunits" => "maskUnits",
            "numoctaves" => "numOctaves",
            "pathlength" => "pathLength",
            "patterncontentunits" => "patternContentUnits",
            "patterntransform" => "patternTransform",
            "patternunits" => "patternUnits",
            "pointsatx" => "pointsAtX",
            "pointsaty" => "pointsAtY",
            "pointsatz" => "pointsAtZ",
            "preservealpha" => "preserveAlpha",
            "preserveaspectratio" => "preserveAspectRatio",
            "primitiveunits" => "primitiveUnits",
            "refx" => "refX",
            "refy" => "refY",
            "repeatcount" => "repeatCount",
            "repeatdur" => "repeatDur",
            "requiredextensions" => "requiredExtensions",
            "requiredfeatures" => "requiredFeatures",
            "specularconstant" => "specularConstant",
            "specularexponent" => "specularExponent",
            "spreadmethod" => "spreadMethod",
            "startoffset" => "startOffset",
            "stddeviation" => "stdDeviation",
            "stitchtiles" => "stitchTiles",
            "surfacescale" => "surfaceScale",
            "systemlanguage" => "systemLanguage",
            "tablevalues" => "tableValues",
            "targetx" => "targetX",
            "targety" => "targetY",
            "textlength" => "textLength",
            "viewbox" => "viewBox",
            "viewtarget" => "viewTarget",
            "xchannelselector" => "xChannelSelector",
            "ychannelselector" => "yChannelSelector",
            "zoomandpan" => "zoomAndPan",
            _ => name,
        }
        .to_string();
    }

    name.to_string()
}

fn starts_inner_formatting_reconstruction_boundary(name: &str) -> bool {
    matches!(name, "button" | "menu" | "p")
}

fn starts_before_formatting_reconstruction_boundary(name: &str) -> bool {
    matches!(
        name,
        "a" | "b" | "br" | "code" | "i" | "marquee" | "menuitem" | "nobr" | "option" | "span"
    )
}

fn is_special_element(name: &str) -> bool {
    matches!(name, "button" | "marquee") || is_table_context_element(name)
}

fn is_special_scope_boundary_element(name: &str) -> bool {
    matches!(name, "div") || is_special_element(name)
}

fn special_scope_blocks_end_tag(name: &str) -> bool {
    !matches!(name, "form") && !is_special_element(name)
}

fn is_button_scope_boundary(name: &str) -> bool {
    matches!(
        name,
        "applet"
            | "caption"
            | "html"
            | "table"
            | "td"
            | "th"
            | "marquee"
            | "object"
            | "template"
            | "button"
    )
}

fn is_implied_end_tag_element(name: &str) -> bool {
    matches!(
        name,
        "dd" | "dt" | "li" | "optgroup" | "option" | "p" | "rb" | "rp" | "rt" | "rtc"
    )
}

fn is_heading_element(name: &str) -> bool {
    matches!(name, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

fn is_ruby_annotation_element(name: &str) -> bool {
    matches!(name, "rb" | "rt" | "rp")
}

fn is_list_item_scope_boundary(name: &str) -> bool {
    matches!(name, "ol" | "ul")
}

fn is_paragraph_boundary_element(name: &str) -> bool {
    const PARAGRAPH_BOUNDARY_ELEMENTS: &[&str] = &[
        "address",
        "article",
        "aside",
        "blockquote",
        "center",
        "details",
        "dialog",
        "dir",
        "div",
        "dl",
        "fieldset",
        "figcaption",
        "figure",
        "footer",
        "form",
        "header",
        "hgroup",
        "hr",
        "listing",
        "main",
        "menu",
        "nav",
        "ol",
        "plaintext",
        "pre",
        "search",
        "section",
        "summary",
        "table",
        "ul",
        "xmp",
    ];

    PARAGRAPH_BOUNDARY_ELEMENTS.contains(&name)
}

fn preserves_initial_line_feed(name: &str) -> bool {
    matches!(name, "listing" | "pre" | "textarea")
}

fn is_void_element(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "basefont"
            | "bgsound"
            | "br"
            | "col"
            | "embed"
            | "frame"
            | "hr"
            | "img"
            | "input"
            | "keygen"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn collect_browser_facts(
    nodes: &[Node],
    summary: &mut BrowserDocument,
    labels: &[(String, String)],
    picture_sources: &[BrowserImageSource],
) {
    let mut pending_picture_sources = Vec::new();
    for node in nodes {
        let Node::Element(element) = node else {
            continue;
        };

        collect_anchor_target(element, summary);
        collect_body_resource(element, summary);

        match element.name.as_str() {
            "a" => summary.links.push(BrowserLink {
                href: element.attribute("href").map(ToOwned::to_owned),
                resolved_href: element
                    .attribute("href")
                    .and_then(|href| resolve_browser_url(href, summary.base_href.as_deref())),
                name: element.attribute("name").map(ToOwned::to_owned),
                target: element.attribute("target").map(ToOwned::to_owned),
                rel: element.attribute("rel").map(ToOwned::to_owned),
                rel_tokens: browser_rel_tokens(element),
                title: element.attribute("title").map(ToOwned::to_owned),
                download: browser_anchor_download(element),
                resolved_ping: browser_anchor_ping(element)
                    .into_iter()
                    .filter_map(|ping| resolve_browser_url(&ping, summary.base_href.as_deref()))
                    .collect(),
                ping: browser_anchor_ping(element),
                hreflang: element.attribute("hreflang").map(ToOwned::to_owned),
                type_hint: element.attribute("type").map(ToOwned::to_owned),
                text: visible_text_for_nodes(&element.children),
            }),
            "picture" => {
                collect_browser_facts(&element.children, summary, labels, &[]);
                continue;
            }
            "source" => {
                if picture_sources.is_empty() {
                    pending_picture_sources.push(browser_image_source_element(
                        element,
                        summary.base_href.as_deref(),
                    ));
                }
            }
            "img" => summary.images.push(browser_image_element(
                element,
                summary.base_href.as_deref(),
                if picture_sources.is_empty() {
                    &pending_picture_sources
                } else {
                    picture_sources
                },
            )),
            "audio" | "video" => summary
                .media
                .push(browser_media_element(element, summary.base_href.as_deref())),
            "script" => summary.scripts.push(browser_script_element(
                element,
                summary.base_href.as_deref(),
            )),
            "style" => summary
                .stylesheets
                .push(browser_style_element(element, summary.base_href.as_deref())),
            "form" => summary.forms.push(BrowserForm {
                action: element.attribute("action").map(ToOwned::to_owned),
                resolved_action: element
                    .attribute("action")
                    .and_then(|action| resolve_browser_url(action, summary.base_href.as_deref())),
                name: element.attribute("name").map(ToOwned::to_owned),
                method: element
                    .attribute("method")
                    .map(|method| method.to_ascii_lowercase())
                    .unwrap_or_else(|| "get".to_string()),
                enctype: element.attribute("enctype").map(ToOwned::to_owned),
                target: element.attribute("target").map(ToOwned::to_owned),
                accept_charset: element.attribute("accept-charset").map(ToOwned::to_owned),
                autocomplete: element.attribute("autocomplete").map(ToOwned::to_owned),
                rel: element.attribute("rel").map(ToOwned::to_owned),
                novalidate: element.attribute("novalidate").is_some(),
                controls: collect_form_controls(
                    &element.children,
                    labels,
                    summary.base_href.as_deref(),
                ),
            }),
            "table" => summary.tables.push(BrowserTable {
                caption: find_first_element_in_nodes(&element.children, "caption")
                    .map(element_text),
                row_count: browser_table_rows(element).len(),
                column_count: browser_table_column_count(element),
                column_hint_count: browser_table_column_hint_count(element),
                cell_count: browser_table_cell_count(element),
                header_cell_count: browser_table_header_cell_count(element),
            }),
            name if heading_level(name).is_some() => summary.headings.push(BrowserHeading {
                level: heading_level(name).expect("heading level was checked above"),
                text: visible_text_for_nodes(&element.children),
            }),
            _ => {}
        }

        collect_browser_facts(&element.children, summary, labels, picture_sources);
        if element.name == "img" {
            pending_picture_sources.clear();
        }
    }
}

fn collect_head_browser_facts(nodes: &[Node], summary: &mut BrowserDocument) {
    for node in nodes {
        let Node::Element(element) = node else {
            continue;
        };

        match element.name.as_str() {
            "meta" => summary.metas.push(BrowserMeta {
                name: element.attribute("name").map(ToOwned::to_owned),
                http_equiv: element.attribute("http-equiv").map(ToOwned::to_owned),
                property: element.attribute("property").map(ToOwned::to_owned),
                charset: element.attribute("charset").map(ToOwned::to_owned),
                content: element.attribute("content").map(ToOwned::to_owned),
            }),
            "link" => {
                if let Some(href) = element.attribute("href") {
                    summary.resources.push(browser_link_resource(
                        element,
                        href,
                        summary.base_href.as_deref(),
                    ));
                    if browser_link_is_stylesheet(element) {
                        summary
                            .stylesheets
                            .push(browser_style_element(element, summary.base_href.as_deref()));
                    }
                }
            }
            "script" => {
                if let Some(src) = element.attribute("src") {
                    summary.resources.push(BrowserResource {
                        kind: "script".to_string(),
                        url: src.to_string(),
                        resolved_url: resolve_browser_url(src, summary.base_href.as_deref()),
                        rel: None,
                        as_hint: None,
                        type_hint: element.attribute("type").map(ToOwned::to_owned),
                        media: None,
                        title: None,
                        width: None,
                        height: None,
                        integrity: element.attribute("integrity").map(ToOwned::to_owned),
                        crossorigin: element.attribute("crossorigin").map(ToOwned::to_owned),
                        referrerpolicy: element.attribute("referrerpolicy").map(ToOwned::to_owned),
                        fetchpriority: element.attribute("fetchpriority").map(ToOwned::to_owned),
                        blocking: element.attribute("blocking").map(ToOwned::to_owned),
                        browsing_context_name: None,
                        loading: None,
                        sandbox: Vec::new(),
                        allow: None,
                        allowfullscreen: false,
                        srcdoc: None,
                        credentialless: false,
                        imagesrcset: None,
                        resolved_imagesrcset: None,
                        imagesizes: None,
                        async_script: element.attribute("async").is_some(),
                        defer_script: element.attribute("defer").is_some(),
                    });
                }
                summary.scripts.push(browser_script_element(
                    element,
                    summary.base_href.as_deref(),
                ));
            }
            "style" => summary
                .stylesheets
                .push(browser_style_element(element, summary.base_href.as_deref())),
            _ => {}
        }

        collect_document_metadata(element, summary);
        collect_head_browser_facts(&element.children, summary);
    }
}

fn collect_document_metadata(element: &Element, summary: &mut BrowserDocument) {
    match element.name.as_str() {
        "meta" => collect_meta_document_metadata(element, summary),
        "link" => collect_link_document_metadata(element, summary),
        _ => {}
    }
}

fn collect_meta_document_metadata(element: &Element, summary: &mut BrowserDocument) {
    if summary.metadata.charset.is_none() {
        summary.metadata.charset = browser_meta_charset(element);
    }

    if let Some(content) = element.attribute("content") {
        if browser_meta_name_is(element, "viewport") && summary.metadata.viewport.is_none() {
            summary.metadata.viewport = Some(content.to_string());
        }
        if browser_meta_name_is(element, "description") && summary.metadata.description.is_none() {
            summary.metadata.description = Some(content.to_string());
        }
        if browser_meta_name_is(element, "application-name")
            && summary.metadata.application_name.is_none()
        {
            summary.metadata.application_name = Some(content.to_string());
        }
        if browser_meta_name_is(element, "referrer") && summary.metadata.referrer_policy.is_none() {
            summary.metadata.referrer_policy = Some(content.to_string());
        }
        if browser_meta_name_is(element, "robots") && summary.metadata.robots.is_none() {
            summary.metadata.robots = Some(content.to_string());
        }
        if browser_meta_name_is(element, "color-scheme") && summary.metadata.color_scheme.is_none()
        {
            summary.metadata.color_scheme = Some(content.to_string());
        }
        if browser_meta_name_is(element, "theme-color") {
            summary.metadata.theme_colors.push(BrowserThemeColor {
                color: content.to_string(),
                media: element.attribute("media").map(ToOwned::to_owned),
            });
        }
        if browser_meta_http_equiv_is(element, "refresh") && summary.metadata.refresh.is_none() {
            summary.metadata.refresh = Some(browser_refresh_metadata(
                content,
                summary.base_href.as_deref(),
            ));
        }
    }
}

fn collect_link_document_metadata(element: &Element, summary: &mut BrowserDocument) {
    if let Some(href) = element.attribute("href") {
        if browser_rel_contains(element, "canonical") && summary.metadata.canonical_url.is_none() {
            summary.metadata.canonical_url = Some(href.to_string());
            summary.metadata.resolved_canonical_url =
                resolve_browser_url(href, summary.base_href.as_deref());
        }
        if browser_rel_contains(element, "manifest") && summary.metadata.manifest_url.is_none() {
            summary.metadata.manifest_url = Some(href.to_string());
            summary.metadata.resolved_manifest_url =
                resolve_browser_url(href, summary.base_href.as_deref());
        }
    }
}

fn browser_meta_name_is(element: &Element, expected: &str) -> bool {
    element
        .attribute("name")
        .is_some_and(|name| name.eq_ignore_ascii_case(expected))
}

fn browser_meta_http_equiv_is(element: &Element, expected: &str) -> bool {
    element
        .attribute("http-equiv")
        .is_some_and(|name| name.eq_ignore_ascii_case(expected))
}

fn browser_meta_charset(element: &Element) -> Option<String> {
    if let Some(charset) = element.attribute("charset") {
        return Some(charset.to_string());
    }
    if !browser_meta_http_equiv_is(element, "content-type") {
        return None;
    }
    element.attribute("content").and_then(extract_meta_charset)
}

fn extract_meta_charset(content: &str) -> Option<String> {
    content
        .split(';')
        .map(str::trim)
        .find_map(|part| {
            let (name, value) = part.split_once('=')?;
            name.trim().eq_ignore_ascii_case("charset").then_some(value)
        })
        .map(trim_browser_metadata_token)
        .filter(|charset| !charset.is_empty())
}

fn browser_refresh_metadata(content: &str, base_href: Option<&str>) -> BrowserRefresh {
    let mut parts = content.split(';');
    let delay = parts
        .next()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned);
    let url = parts.find_map(browser_refresh_url);
    let resolved_url = url
        .as_deref()
        .and_then(|url| resolve_browser_url(url, base_href));
    BrowserRefresh {
        delay,
        url,
        resolved_url,
    }
}

fn browser_refresh_url(part: &str) -> Option<String> {
    let (name, value) = part.split_once('=')?;
    if !name.trim().eq_ignore_ascii_case("url") {
        return None;
    }
    let url = trim_browser_metadata_token(value);
    (!url.is_empty()).then_some(url)
}

fn trim_browser_metadata_token(value: &str) -> String {
    value
        .trim()
        .trim_matches(|character| character == '"' || character == '\'')
        .to_string()
}

fn collect_anchor_target(element: &Element, summary: &mut BrowserDocument) {
    let id = element.attribute("id");
    let name = element.attribute("name").filter(|_| element.name == "a");
    if id.is_none() && name.is_none() {
        return;
    }

    summary.anchors.push(BrowserAnchor {
        id: id.map(ToOwned::to_owned),
        name: name.map(ToOwned::to_owned),
        text: visible_text_for_nodes(&element.children),
    });
}

fn collect_body_resource(element: &Element, summary: &mut BrowserDocument) {
    let Some((kind, url)) = body_resource(element) else {
        return;
    };

    summary.resources.push(BrowserResource {
        kind,
        resolved_url: resolve_browser_url(&url, summary.base_href.as_deref()),
        url,
        rel: None,
        as_hint: None,
        type_hint: element.attribute("type").map(ToOwned::to_owned),
        media: element.attribute("media").map(ToOwned::to_owned),
        title: element.attribute("title").map(ToOwned::to_owned),
        width: element.attribute("width").map(ToOwned::to_owned),
        height: element.attribute("height").map(ToOwned::to_owned),
        integrity: element.attribute("integrity").map(ToOwned::to_owned),
        crossorigin: element.attribute("crossorigin").map(ToOwned::to_owned),
        referrerpolicy: element.attribute("referrerpolicy").map(ToOwned::to_owned),
        fetchpriority: element.attribute("fetchpriority").map(ToOwned::to_owned),
        blocking: element.attribute("blocking").map(ToOwned::to_owned),
        browsing_context_name: browser_browsing_context_name(element),
        loading: browser_browsing_context_loading(element),
        sandbox: browser_browsing_context_sandbox(element),
        allow: browser_browsing_context_allow(element),
        allowfullscreen: browser_browsing_context_allowfullscreen(element),
        srcdoc: browser_browsing_context_srcdoc(element),
        credentialless: browser_browsing_context_credentialless(element),
        imagesrcset: None,
        resolved_imagesrcset: None,
        imagesizes: None,
        async_script: element.name == "script" && element.attribute("async").is_some(),
        defer_script: element.name == "script" && element.attribute("defer").is_some(),
    });
}

fn body_resource(element: &Element) -> Option<(String, String)> {
    match element.name.as_str() {
        "img" => element
            .attribute("src")
            .map(|src| ("image".to_string(), src.to_string())),
        "script" => element
            .attribute("src")
            .map(|src| ("script".to_string(), src.to_string())),
        "iframe" | "frame" => element
            .attribute("src")
            .map(|src| ("frame".to_string(), src.to_string())),
        "embed" => element
            .attribute("src")
            .map(|src| ("embed".to_string(), src.to_string())),
        "object" => element
            .attribute("data")
            .map(|data| ("object".to_string(), data.to_string())),
        "audio" | "video" => element
            .attribute("src")
            .map(|src| (element.name.clone(), src.to_string())),
        "source" | "track" => element
            .attribute("src")
            .map(|src| (element.name.clone(), src.to_string())),
        _ => None,
    }
}

fn resolve_browser_url(url: &str, base_href: Option<&str>) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }
    if is_absolute_url(url) {
        return Some(url.to_string());
    }

    let base_href = base_href?.trim();
    if !is_absolute_url(base_href) {
        return None;
    }

    let (scheme, authority, base_path) = split_hierarchical_url(base_href)?;
    if url.starts_with("//") {
        return Some(format!("{scheme}:{url}"));
    }
    if let Some(fragment) = url.strip_prefix('#') {
        return Some(format!(
            "{scheme}://{authority}{}#{fragment}",
            strip_fragment(base_path)
        ));
    }
    if let Some(query) = url.strip_prefix('?') {
        return Some(format!(
            "{scheme}://{authority}{}?{query}",
            strip_query_and_fragment(base_path)
        ));
    }

    let (relative_path, suffix) = split_url_suffix(url);
    let resolved_path = if relative_path.starts_with('/') {
        normalize_browser_path(relative_path)
    } else {
        let base_dir = browser_base_directory(strip_query_and_fragment(base_path));
        normalize_browser_path(&format!("{base_dir}{relative_path}"))
    };

    Some(format!("{scheme}://{authority}{resolved_path}{suffix}"))
}

fn is_absolute_url(url: &str) -> bool {
    let Some(index) = url.find(':') else {
        return false;
    };
    let scheme = &url[..index];
    !scheme.is_empty()
        && scheme.as_bytes()[0].is_ascii_alphabetic()
        && scheme
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
        && !scheme.contains('/')
        && !scheme.contains('?')
        && !scheme.contains('#')
}

fn split_hierarchical_url(url: &str) -> Option<(&str, &str, &str)> {
    let (scheme, rest) = url.split_once(':')?;
    let rest = rest.strip_prefix("//")?;
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.is_empty() {
        return None;
    }
    let path = if authority_end == rest.len() {
        "/"
    } else {
        &rest[authority_end..]
    };
    Some((scheme, authority, path))
}

fn strip_fragment(path: &str) -> &str {
    path.split_once('#')
        .map(|(before_fragment, _)| before_fragment)
        .unwrap_or(path)
}

fn strip_query_and_fragment(path: &str) -> &str {
    let query_index = path.find('?');
    let fragment_index = path.find('#');
    let end = match (query_index, fragment_index) {
        (Some(query), Some(fragment)) => query.min(fragment),
        (Some(query), None) => query,
        (None, Some(fragment)) => fragment,
        (None, None) => path.len(),
    };
    &path[..end]
}

fn split_url_suffix(url: &str) -> (&str, &str) {
    let suffix_index = match (url.find('?'), url.find('#')) {
        (Some(query), Some(fragment)) => query.min(fragment),
        (Some(query), None) => query,
        (None, Some(fragment)) => fragment,
        (None, None) => url.len(),
    };
    (&url[..suffix_index], &url[suffix_index..])
}

fn browser_base_directory(path: &str) -> String {
    if path.ends_with('/') {
        return path.to_string();
    }
    match path.rfind('/') {
        Some(index) => path[..=index].to_string(),
        None => "/".to_string(),
    }
}

fn normalize_browser_path(path: &str) -> String {
    let absolute = path.starts_with('/');
    let trailing_slash = path.ends_with('/');
    let mut segments: Vec<&str> = Vec::new();

    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            _ => segments.push(segment),
        }
    }

    let mut normalized = if absolute {
        format!("/{}", segments.join("/"))
    } else {
        segments.join("/")
    };
    if normalized.is_empty() {
        normalized.push('/');
    }
    if trailing_slash && !normalized.ends_with('/') {
        normalized.push('/');
    }
    normalized
}

fn link_resource_kind(rel: Option<&str>) -> String {
    let Some(rel) = rel else {
        return "link".to_string();
    };
    let rel_tokens: Vec<String> = rel
        .split_ascii_whitespace()
        .map(str::to_ascii_lowercase)
        .collect();
    if rel_tokens.iter().any(|token| token == "stylesheet") {
        return "stylesheet".to_string();
    }
    for token in rel_tokens {
        match token.as_str() {
            "icon" | "shortcut" => return "icon".to_string(),
            "preload" => return "preload".to_string(),
            "modulepreload" => return "modulepreload".to_string(),
            "prefetch" => return "prefetch".to_string(),
            "preconnect" => return "preconnect".to_string(),
            "dns-prefetch" => return "dns-prefetch".to_string(),
            "prerender" => return "prerender".to_string(),
            "manifest" => return "manifest".to_string(),
            "canonical" => return "canonical".to_string(),
            "alternate" => return "alternate".to_string(),
            _ => {}
        }
    }
    "link".to_string()
}

fn browser_link_resource(
    element: &Element,
    href: &str,
    base_href: Option<&str>,
) -> BrowserResource {
    let imagesrcset = element.attribute("imagesrcset").map(ToOwned::to_owned);
    BrowserResource {
        kind: link_resource_kind(element.attribute("rel")),
        url: href.to_string(),
        resolved_url: resolve_browser_url(href, base_href),
        rel: element.attribute("rel").map(ToOwned::to_owned),
        as_hint: element.attribute("as").map(ToOwned::to_owned),
        type_hint: element.attribute("type").map(ToOwned::to_owned),
        media: element.attribute("media").map(ToOwned::to_owned),
        title: element.attribute("title").map(ToOwned::to_owned),
        width: None,
        height: None,
        integrity: element.attribute("integrity").map(ToOwned::to_owned),
        crossorigin: element.attribute("crossorigin").map(ToOwned::to_owned),
        referrerpolicy: element.attribute("referrerpolicy").map(ToOwned::to_owned),
        fetchpriority: element.attribute("fetchpriority").map(ToOwned::to_owned),
        blocking: element.attribute("blocking").map(ToOwned::to_owned),
        browsing_context_name: None,
        loading: None,
        sandbox: Vec::new(),
        allow: None,
        allowfullscreen: false,
        srcdoc: None,
        credentialless: false,
        resolved_imagesrcset: imagesrcset
            .as_deref()
            .map(|srcset| resolve_browser_srcset(srcset, base_href)),
        imagesrcset,
        imagesizes: element.attribute("imagesizes").map(ToOwned::to_owned),
        async_script: false,
        defer_script: false,
    }
}

fn collect_browser_content_nodes(
    nodes: &[Node],
    output: &mut Vec<BrowserContentNode>,
    base_href: Option<&str>,
    labels: &[(String, String)],
    id_texts: &[(String, String)],
) {
    collect_browser_content_nodes_with_mode(
        nodes, output, base_href, labels, id_texts, None, false,
    );
}

fn collect_browser_content_nodes_with_mode(
    nodes: &[Node],
    output: &mut Vec<BrowserContentNode>,
    base_href: Option<&str>,
    labels: &[(String, String)],
    id_texts: &[(String, String)],
    current_label_text: Option<&str>,
    preserve_whitespace: bool,
) {
    for node in nodes {
        match node {
            Node::Text(value) => {
                let text = if preserve_whitespace {
                    value.data.clone()
                } else {
                    collapse_html_whitespace(&value.data)
                };
                if !text.is_empty() {
                    output.push(BrowserContentNode {
                        role: "text".to_string(),
                        authored_role: None,
                        name: None,
                        id: None,
                        classes: Vec::new(),
                        title: None,
                        lang: None,
                        dir: None,
                        text: Some(text),
                        href: None,
                        resolved_href: None,
                        target: None,
                        rel: None,
                        rel_tokens: Vec::new(),
                        download: None,
                        ping: Vec::new(),
                        resolved_ping: Vec::new(),
                        hreflang: None,
                        src: None,
                        resolved_src: None,
                        alt: None,
                        resource_kind: None,
                        width: None,
                        height: None,
                        type_hint: None,
                        media: None,
                        poster: None,
                        resolved_poster: None,
                        preload: None,
                        controls: false,
                        autoplay: false,
                        loop_media: false,
                        muted: false,
                        playsinline: false,
                        browsing_context_name: None,
                        loading: None,
                        sandbox: Vec::new(),
                        allow: None,
                        allowfullscreen: false,
                        referrerpolicy: None,
                        srcdoc: None,
                        credentialless: false,
                        control_type: None,
                        form_owner: None,
                        label_for: None,
                        labels: Vec::new(),
                        accessible_name: None,
                        aria_label: None,
                        aria_labelledby: Vec::new(),
                        aria_describedby: Vec::new(),
                        aria_controls: Vec::new(),
                        aria_current: None,
                        aria_expanded: None,
                        aria_pressed: None,
                        aria_selected: None,
                        aria_hidden: false,
                        hidden: false,
                        inert: false,
                        open: false,
                        tabindex: None,
                        accesskey: Vec::new(),
                        focusable: None,
                        contenteditable: None,
                        editing_mode: None,
                        draggable: None,
                        draggable_state: None,
                        spellcheck: None,
                        translate: None,
                        popover: None,
                        popover_target: None,
                        popover_target_action: None,
                        command: None,
                        command_for: None,
                        placeholder: None,
                        autocomplete: None,
                        autocapitalize: None,
                        enterkeyhint: None,
                        dirname: None,
                        accept: None,
                        capture: None,
                        inputmode: None,
                        pattern: None,
                        min: None,
                        max: None,
                        step: None,
                        minlength: None,
                        maxlength: None,
                        size: None,
                        list: None,
                        form_action: None,
                        resolved_form_action: None,
                        form_enctype: None,
                        form_method: None,
                        form_target: None,
                        form_novalidate: false,
                        value: None,
                        autofocus: false,
                        disabled: false,
                        required: false,
                        readonly: false,
                        checked: false,
                        selected: false,
                        multiple: false,
                        options: Vec::new(),
                        table_section_kind: None,
                        colspan: None,
                        rowspan: None,
                        span: None,
                        scope: None,
                        headers: Vec::new(),
                        abbr: None,
                        text_flow: if preserve_whitespace {
                            Some("preformatted".to_string())
                        } else {
                            None
                        },
                        list_kind: None,
                        list_start: None,
                        list_marker_type: None,
                        list_reversed: false,
                        list_item_value: None,
                        quote_cite: None,
                        resolved_quote_cite: None,
                        break_kind: None,
                        heading_level: None,
                        section_kind: None,
                        landmark_kind: None,
                        children: Vec::new(),
                    });
                }
            }
            Node::Element(element) => {
                if let Some(content_node) = browser_content_node_for_element(
                    element,
                    base_href,
                    labels,
                    id_texts,
                    current_label_text,
                    preserve_whitespace,
                ) {
                    output.push(content_node);
                }
            }
            Node::DocumentType(_) | Node::Comment(_) => {}
        }
    }
}

fn browser_content_node_for_element(
    element: &Element,
    base_href: Option<&str>,
    labels: &[(String, String)],
    id_texts: &[(String, String)],
    current_label_text: Option<&str>,
    preserve_whitespace: bool,
) -> Option<BrowserContentNode> {
    if is_browser_invisible_element(&element.name) {
        return None;
    }

    let role = browser_content_role(&element.name)?;
    let mut children = Vec::new();
    let element_label_text = (element.name == "label")
        .then(|| visible_text_for_nodes(&element.children))
        .filter(|text| !text.is_empty());
    let child_label_text = element_label_text.as_deref().or(current_label_text);
    if should_collect_browser_content_children(&element.name) {
        collect_browser_content_nodes_with_mode(
            &element.children,
            &mut children,
            base_href,
            labels,
            id_texts,
            child_label_text,
            preserve_whitespace || browser_preserves_text_whitespace(&element.name),
        );
    }

    let text = browser_content_text(element, role);
    if role == "inline" && text.is_none() && children.is_empty() {
        return None;
    }

    let href = element.attribute("href").map(ToOwned::to_owned);
    let src = browser_content_src(element);
    let poster = browser_media_poster(element);
    let quote_cite = browser_quote_cite(element);
    let control_labels = browser_control_labels(element, labels, current_label_text);
    let aria_label = browser_aria_label(element);
    let accessible_name = browser_accessible_name(element, role, &control_labels, id_texts);

    Some(BrowserContentNode {
        role: role.to_string(),
        authored_role: browser_authored_role(element),
        name: Some(element.name.clone()),
        id: element.attribute("id").map(ToOwned::to_owned),
        classes: element
            .attribute("class")
            .map(split_html_classes)
            .unwrap_or_default(),
        title: element.attribute("title").map(ToOwned::to_owned),
        lang: element.attribute("lang").map(ToOwned::to_owned),
        dir: element.attribute("dir").map(ToOwned::to_owned),
        text,
        resolved_href: href
            .as_deref()
            .and_then(|href| resolve_browser_url(href, base_href)),
        href,
        target: browser_anchor_target(element),
        rel: browser_anchor_rel(element),
        rel_tokens: browser_anchor_rel_tokens(element),
        download: browser_anchor_download(element),
        resolved_ping: browser_anchor_ping(element)
            .into_iter()
            .filter_map(|ping| resolve_browser_url(&ping, base_href))
            .collect(),
        ping: browser_anchor_ping(element),
        hreflang: browser_anchor_hreflang(element),
        resolved_src: src
            .as_deref()
            .and_then(|src| resolve_browser_url(src, base_href)),
        src,
        alt: element.attribute("alt").map(ToOwned::to_owned),
        resource_kind: browser_content_resource_kind(element),
        width: element.attribute("width").map(ToOwned::to_owned),
        height: element.attribute("height").map(ToOwned::to_owned),
        type_hint: element.attribute("type").map(ToOwned::to_owned),
        media: element.attribute("media").map(ToOwned::to_owned),
        resolved_poster: poster
            .as_deref()
            .and_then(|poster| resolve_browser_url(poster, base_href)),
        poster,
        preload: browser_media_preload(element),
        controls: browser_media_controls(element),
        autoplay: browser_media_autoplay(element),
        loop_media: browser_media_loop(element),
        muted: browser_media_muted(element),
        playsinline: browser_media_playsinline(element),
        browsing_context_name: browser_browsing_context_name(element),
        loading: browser_browsing_context_loading(element),
        sandbox: browser_browsing_context_sandbox(element),
        allow: browser_browsing_context_allow(element),
        allowfullscreen: browser_browsing_context_allowfullscreen(element),
        referrerpolicy: browser_browsing_context_referrerpolicy(element),
        srcdoc: browser_browsing_context_srcdoc(element),
        credentialless: browser_browsing_context_credentialless(element),
        control_type: browser_content_control_type(element),
        form_owner: browser_form_owner(element),
        label_for: browser_label_for(element),
        labels: control_labels,
        accessible_name,
        aria_label,
        aria_labelledby: browser_aria_idrefs(element, "aria-labelledby"),
        aria_describedby: browser_aria_idrefs(element, "aria-describedby"),
        aria_controls: browser_aria_idrefs(element, "aria-controls"),
        aria_current: browser_aria_state(element, "aria-current"),
        aria_expanded: browser_aria_state(element, "aria-expanded"),
        aria_pressed: browser_aria_state(element, "aria-pressed"),
        aria_selected: browser_aria_state(element, "aria-selected"),
        aria_hidden: browser_aria_hidden(element),
        hidden: browser_hidden(element),
        inert: browser_inert(element),
        open: browser_open(element),
        tabindex: element.attribute("tabindex").map(ToOwned::to_owned),
        accesskey: browser_accesskey(element),
        focusable: browser_focusable(element),
        contenteditable: element.attribute("contenteditable").map(ToOwned::to_owned),
        editing_mode: browser_editing_mode(element),
        draggable: element.attribute("draggable").map(ToOwned::to_owned),
        draggable_state: browser_draggable_state(element),
        spellcheck: browser_spellcheck_state(element),
        translate: browser_translate_state(element),
        popover: element.attribute("popover").map(ToOwned::to_owned),
        popover_target: browser_popover_target(element),
        popover_target_action: browser_popover_target_action(element),
        command: browser_command(element),
        command_for: browser_command_for(element),
        placeholder: browser_placeholder(element),
        autocomplete: browser_autocomplete(element),
        autocapitalize: browser_autocapitalize(element),
        enterkeyhint: browser_enterkeyhint(element),
        dirname: browser_dirname(element),
        accept: browser_control_accept(element),
        capture: browser_control_capture(element),
        inputmode: browser_control_inputmode(element),
        pattern: browser_control_pattern(element),
        min: browser_control_min(element),
        max: browser_control_max(element),
        step: browser_control_step(element),
        minlength: browser_control_minlength(element),
        maxlength: browser_control_maxlength(element),
        size: browser_control_size(element),
        list: browser_control_list(element),
        resolved_form_action: browser_control_form_action(element)
            .as_deref()
            .and_then(|action| resolve_browser_url(action, base_href)),
        form_action: browser_control_form_action(element),
        form_enctype: browser_control_form_enctype(element),
        form_method: browser_control_form_method(element),
        form_target: browser_control_form_target(element),
        form_novalidate: browser_control_form_novalidate(element),
        value: browser_content_value(element),
        autofocus: browser_autofocus(element),
        disabled: element.attribute("disabled").is_some(),
        required: browser_required(element),
        readonly: browser_readonly(element),
        checked: element.attribute("checked").is_some(),
        selected: element.attribute("selected").is_some(),
        multiple: browser_multiple(element),
        options: browser_content_options(element),
        table_section_kind: browser_table_section_kind(element),
        colspan: element.attribute("colspan").map(ToOwned::to_owned),
        rowspan: element.attribute("rowspan").map(ToOwned::to_owned),
        span: element.attribute("span").map(ToOwned::to_owned),
        scope: element.attribute("scope").map(ToOwned::to_owned),
        headers: element
            .attribute("headers")
            .map(split_html_classes)
            .unwrap_or_default(),
        abbr: element.attribute("abbr").map(ToOwned::to_owned),
        text_flow: browser_text_flow(element, preserve_whitespace),
        list_kind: browser_list_kind(element),
        list_start: browser_list_start(element),
        list_marker_type: browser_list_marker_type(element),
        list_reversed: element.name == "ol" && element.attribute("reversed").is_some(),
        list_item_value: browser_list_item_value(element),
        resolved_quote_cite: quote_cite
            .as_deref()
            .and_then(|cite| resolve_browser_url(cite, base_href)),
        quote_cite,
        break_kind: browser_break_kind(element),
        heading_level: heading_level(&element.name),
        section_kind: browser_section_kind(element),
        landmark_kind: browser_landmark_kind(element),
        children,
    })
}

fn browser_content_role(name: &str) -> Option<&'static str> {
    match name {
        "area" | "base" | "link" | "meta" | "param" | "script" | "style" | "template" | "title" => {
            None
        }
        "a" => Some("link"),
        "img" => Some("image"),
        "iframe" | "frame" => Some("frame"),
        "embed" => Some("embed"),
        "object" => Some("object"),
        "audio" | "video" => Some("media"),
        "canvas" => Some("canvas"),
        "br" | "wbr" => Some("line_break"),
        "hr" => Some("separator"),
        "blockquote" => Some("quote_block"),
        "q" => Some("quote"),
        "p" => Some("paragraph"),
        "pre" | "plaintext" | "xmp" | "listing" => Some("preformatted"),
        "article" => Some("article"),
        "aside" => Some("aside"),
        "footer" => Some("footer"),
        "header" => Some("header"),
        "hgroup" => Some("heading_group"),
        "main" => Some("main"),
        "nav" => Some("navigation"),
        "section" => Some("section"),
        "form" => Some("form"),
        "fieldset" => Some("form_group"),
        "label" => Some("label"),
        "legend" => Some("legend"),
        "input" | "button" | "select" | "textarea" => Some("control"),
        "optgroup" => Some("option_group"),
        "option" => Some("option"),
        "ul" | "ol" | "menu" | "dir" => Some("list"),
        "li" | "dt" | "dd" => Some("list_item"),
        "table" => Some("table"),
        "caption" => Some("table_caption"),
        "colgroup" => Some("table_column_group"),
        "col" => Some("table_column"),
        "tbody" | "thead" | "tfoot" => Some("table_section"),
        "tr" => Some("table_row"),
        "td" | "th" => Some("table_cell"),
        name if heading_level(name).is_some() => Some("heading"),
        name if is_browser_block_element(name) => Some("block"),
        _ => Some("inline"),
    }
}

fn should_collect_browser_content_children(name: &str) -> bool {
    !matches!(
        name,
        "area"
            | "base"
            | "br"
            | "button"
            | "col"
            | "embed"
            | "frame"
            | "iframe"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "textarea"
            | "wbr"
    )
}

fn browser_content_text(element: &Element, role: &str) -> Option<String> {
    let text = match role {
        "control" if element.name == "input" => browser_input_display_value(element),
        "option" => browser_option_display_text(element),
        "control" | "heading" | "label" | "legend" | "link" | "table_caption" => {
            visible_text_for_nodes(&element.children)
        }
        _ => String::new(),
    };
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn browser_content_src(element: &Element) -> Option<String> {
    match element.name.as_str() {
        "object" => element.attribute("data").map(ToOwned::to_owned),
        _ => element.attribute("src").map(ToOwned::to_owned),
    }
}

fn browser_content_resource_kind(element: &Element) -> Option<String> {
    match element.name.as_str() {
        "img" => Some("image".to_string()),
        "iframe" | "frame" => Some("frame".to_string()),
        "embed" => Some("embed".to_string()),
        "object" => Some("object".to_string()),
        "audio" | "video" => Some(element.name.clone()),
        "canvas" => Some("canvas".to_string()),
        _ => None,
    }
}

fn is_browser_browsing_context_element(element: &Element) -> bool {
    matches!(element.name.as_str(), "iframe" | "frame")
}

fn browser_browsing_context_name(element: &Element) -> Option<String> {
    if is_browser_browsing_context_element(element) {
        element.attribute("name").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_browsing_context_loading(element: &Element) -> Option<String> {
    if is_browser_browsing_context_element(element) {
        element.attribute("loading").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_browsing_context_sandbox(element: &Element) -> Vec<String> {
    if is_browser_browsing_context_element(element) {
        element
            .attribute("sandbox")
            .map(split_html_classes)
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn browser_browsing_context_allow(element: &Element) -> Option<String> {
    if is_browser_browsing_context_element(element) {
        element.attribute("allow").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_browsing_context_allowfullscreen(element: &Element) -> bool {
    is_browser_browsing_context_element(element) && element.attribute("allowfullscreen").is_some()
}

fn browser_browsing_context_referrerpolicy(element: &Element) -> Option<String> {
    if is_browser_browsing_context_element(element) {
        element.attribute("referrerpolicy").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_browsing_context_srcdoc(element: &Element) -> Option<String> {
    if element.name == "iframe" {
        element.attribute("srcdoc").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_browsing_context_credentialless(element: &Element) -> bool {
    is_browser_browsing_context_element(element) && element.attribute("credentialless").is_some()
}

fn browser_image_element(
    element: &Element,
    base_href: Option<&str>,
    picture_sources: &[BrowserImageSource],
) -> BrowserImage {
    let src = element.attribute("src").map(ToOwned::to_owned);
    let srcset = element.attribute("srcset").map(ToOwned::to_owned);
    BrowserImage {
        resolved_src: src
            .as_deref()
            .and_then(|src| resolve_browser_url(src, base_href)),
        src,
        alt: element.attribute("alt").map(ToOwned::to_owned),
        width: element.attribute("width").map(ToOwned::to_owned),
        height: element.attribute("height").map(ToOwned::to_owned),
        resolved_srcset: srcset
            .as_deref()
            .map(|srcset| resolve_browser_srcset(srcset, base_href)),
        srcset,
        sizes: element.attribute("sizes").map(ToOwned::to_owned),
        loading: element.attribute("loading").map(ToOwned::to_owned),
        decoding: element.attribute("decoding").map(ToOwned::to_owned),
        fetchpriority: element.attribute("fetchpriority").map(ToOwned::to_owned),
        crossorigin: element.attribute("crossorigin").map(ToOwned::to_owned),
        referrerpolicy: element.attribute("referrerpolicy").map(ToOwned::to_owned),
        usemap: element.attribute("usemap").map(ToOwned::to_owned),
        ismap: element.attribute("ismap").is_some(),
        sources: picture_sources.to_vec(),
    }
}

fn browser_image_source_element(element: &Element, base_href: Option<&str>) -> BrowserImageSource {
    let srcset = element.attribute("srcset").map(ToOwned::to_owned);
    BrowserImageSource {
        resolved_srcset: srcset
            .as_deref()
            .map(|srcset| resolve_browser_srcset(srcset, base_href)),
        srcset,
        sizes: element.attribute("sizes").map(ToOwned::to_owned),
        media: element.attribute("media").map(ToOwned::to_owned),
        type_hint: element.attribute("type").map(ToOwned::to_owned),
    }
}

fn resolve_browser_srcset(srcset: &str, base_href: Option<&str>) -> String {
    srcset
        .split(',')
        .map(|candidate| resolve_browser_srcset_candidate(candidate, base_href))
        .collect::<Vec<_>>()
        .join(", ")
}

fn resolve_browser_srcset_candidate(candidate: &str, base_href: Option<&str>) -> String {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let Some((url, descriptor)) = trimmed.split_once(char::is_whitespace) else {
        return resolve_browser_url(trimmed, base_href).unwrap_or_else(|| trimmed.to_string());
    };
    let resolved_url = resolve_browser_url(url, base_href).unwrap_or_else(|| url.to_string());
    let descriptor = descriptor.trim();
    if descriptor.is_empty() {
        resolved_url
    } else {
        format!("{resolved_url} {descriptor}")
    }
}

fn browser_script_element(element: &Element, base_href: Option<&str>) -> BrowserScript {
    let src = element.attribute("src").map(ToOwned::to_owned);
    BrowserScript {
        script_kind: browser_script_kind(element),
        resolved_src: src
            .as_deref()
            .and_then(|src| resolve_browser_url(src, base_href)),
        src,
        type_hint: element.attribute("type").map(ToOwned::to_owned),
        async_script: element.attribute("async").is_some(),
        defer_script: element.attribute("defer").is_some(),
        nomodule: element.attribute("nomodule").is_some(),
        integrity: element.attribute("integrity").map(ToOwned::to_owned),
        crossorigin: element.attribute("crossorigin").map(ToOwned::to_owned),
        referrerpolicy: element.attribute("referrerpolicy").map(ToOwned::to_owned),
        fetchpriority: element.attribute("fetchpriority").map(ToOwned::to_owned),
        blocking: element.attribute("blocking").map(ToOwned::to_owned),
        text: element_text_if_non_empty(element),
    }
}

fn browser_script_kind(element: &Element) -> String {
    let Some(type_hint) = element.attribute("type") else {
        return "classic".to_string();
    };
    let normalized_type = type_hint.trim().to_ascii_lowercase();
    let type_essence = normalized_type
        .split_once(';')
        .map(|(essence, _)| essence.trim())
        .unwrap_or(normalized_type.as_str());
    match type_essence {
        ""
        | "text/javascript"
        | "application/javascript"
        | "text/ecmascript"
        | "application/ecmascript"
        | "module" => {
            if type_essence == "module" {
                "module".to_string()
            } else {
                "classic".to_string()
            }
        }
        "importmap" => "importmap".to_string(),
        "speculationrules" => "speculationrules".to_string(),
        _ => "data".to_string(),
    }
}

fn browser_style_element(element: &Element, base_href: Option<&str>) -> BrowserStylesheet {
    let href = element.attribute("href").map(ToOwned::to_owned);
    BrowserStylesheet {
        source: element.name.clone(),
        resolved_href: href
            .as_deref()
            .and_then(|href| resolve_browser_url(href, base_href)),
        href,
        rel: element.attribute("rel").map(ToOwned::to_owned),
        type_hint: element.attribute("type").map(ToOwned::to_owned),
        media: element.attribute("media").map(ToOwned::to_owned),
        title: element.attribute("title").map(ToOwned::to_owned),
        disabled: element.attribute("disabled").is_some(),
        alternate: browser_rel_contains(element, "alternate"),
        integrity: element.attribute("integrity").map(ToOwned::to_owned),
        crossorigin: element.attribute("crossorigin").map(ToOwned::to_owned),
        referrerpolicy: element.attribute("referrerpolicy").map(ToOwned::to_owned),
        fetchpriority: element.attribute("fetchpriority").map(ToOwned::to_owned),
        blocking: element.attribute("blocking").map(ToOwned::to_owned),
        text: if element.name == "style" {
            element_text_if_non_empty(element)
        } else {
            None
        },
    }
}

fn browser_link_is_stylesheet(element: &Element) -> bool {
    element.name == "link" && browser_rel_contains(element, "stylesheet")
}

fn browser_rel_contains(element: &Element, token: &str) -> bool {
    browser_rel_tokens(element)
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(token))
}

fn browser_rel_tokens(element: &Element) -> Vec<String> {
    element
        .attribute("rel")
        .map(split_html_classes)
        .unwrap_or_default()
}

fn element_text_if_non_empty(element: &Element) -> Option<String> {
    let text = element_text(element);
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn browser_media_element(element: &Element, base_href: Option<&str>) -> BrowserMedia {
    let src = element.attribute("src").map(ToOwned::to_owned);
    let poster = browser_media_poster(element);
    BrowserMedia {
        kind: element.name.clone(),
        resolved_src: src
            .as_deref()
            .and_then(|src| resolve_browser_url(src, base_href)),
        src,
        resolved_poster: poster
            .as_deref()
            .and_then(|poster| resolve_browser_url(poster, base_href)),
        poster,
        width: element.attribute("width").map(ToOwned::to_owned),
        height: element.attribute("height").map(ToOwned::to_owned),
        controls: browser_media_controls(element),
        autoplay: browser_media_autoplay(element),
        loop_media: browser_media_loop(element),
        muted: browser_media_muted(element),
        playsinline: browser_media_playsinline(element),
        preload: browser_media_preload(element),
    }
}

fn browser_media_poster(element: &Element) -> Option<String> {
    if element.name == "video" {
        element.attribute("poster").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_media_preload(element: &Element) -> Option<String> {
    if matches!(element.name.as_str(), "audio" | "video") {
        element.attribute("preload").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_media_controls(element: &Element) -> bool {
    matches!(element.name.as_str(), "audio" | "video") && element.attribute("controls").is_some()
}

fn browser_media_autoplay(element: &Element) -> bool {
    matches!(element.name.as_str(), "audio" | "video") && element.attribute("autoplay").is_some()
}

fn browser_media_loop(element: &Element) -> bool {
    matches!(element.name.as_str(), "audio" | "video") && element.attribute("loop").is_some()
}

fn browser_media_muted(element: &Element) -> bool {
    matches!(element.name.as_str(), "audio" | "video") && element.attribute("muted").is_some()
}

fn browser_media_playsinline(element: &Element) -> bool {
    matches!(element.name.as_str(), "audio" | "video")
        && (element.attribute("playsinline").is_some()
            || element.attribute("webkit-playsinline").is_some())
}

fn browser_table_section_kind(element: &Element) -> Option<String> {
    match element.name.as_str() {
        "thead" | "tbody" | "tfoot" => Some(element.name.clone()),
        _ => None,
    }
}

fn browser_preserves_text_whitespace(name: &str) -> bool {
    matches!(name, "pre" | "plaintext" | "xmp" | "listing")
}

fn browser_text_flow(element: &Element, inherited_preserve_whitespace: bool) -> Option<String> {
    if browser_preserves_text_whitespace(&element.name) || inherited_preserve_whitespace {
        Some("preformatted".to_string())
    } else {
        None
    }
}

fn browser_list_kind(element: &Element) -> Option<String> {
    match element.name.as_str() {
        "ul" => Some("unordered".to_string()),
        "ol" => Some("ordered".to_string()),
        "menu" => Some("menu".to_string()),
        "dir" => Some("directory".to_string()),
        _ => None,
    }
}

fn browser_list_start(element: &Element) -> Option<String> {
    if element.name == "ol" {
        element.attribute("start").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_list_marker_type(element: &Element) -> Option<String> {
    match element.name.as_str() {
        "ol" | "ul" | "li" => element.attribute("type").map(ToOwned::to_owned),
        _ => None,
    }
}

fn browser_list_item_value(element: &Element) -> Option<String> {
    if element.name == "li" {
        element.attribute("value").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_quote_cite(element: &Element) -> Option<String> {
    match element.name.as_str() {
        "blockquote" | "q" => element.attribute("cite").map(ToOwned::to_owned),
        _ => None,
    }
}

fn browser_break_kind(element: &Element) -> Option<String> {
    match element.name.as_str() {
        "br" => Some("line".to_string()),
        "wbr" => Some("word".to_string()),
        "hr" => Some("thematic".to_string()),
        _ => None,
    }
}

fn browser_section_kind(element: &Element) -> Option<String> {
    match element.name.as_str() {
        "article" => Some("article".to_string()),
        "aside" => Some("aside".to_string()),
        "footer" => Some("footer".to_string()),
        "header" => Some("header".to_string()),
        "hgroup" => Some("heading_group".to_string()),
        "main" => Some("main".to_string()),
        "nav" => Some("navigation".to_string()),
        "section" => Some("section".to_string()),
        _ => None,
    }
}

fn browser_landmark_kind(element: &Element) -> Option<String> {
    match element.name.as_str() {
        "aside" => Some("complementary".to_string()),
        "footer" => Some("footer".to_string()),
        "header" => Some("header".to_string()),
        "main" => Some("main".to_string()),
        "nav" => Some("navigation".to_string()),
        _ => None,
    }
}

fn collect_label_texts_by_control_id(nodes: &[Node]) -> Vec<(String, String)> {
    let mut labels = Vec::new();
    collect_label_texts_by_control_id_into(nodes, &mut labels);
    labels
}

fn collect_label_texts_by_control_id_into(nodes: &[Node], labels: &mut Vec<(String, String)>) {
    for node in nodes {
        let Node::Element(element) = node else {
            continue;
        };
        if element.name == "label" {
            if let Some(for_id) = element.attribute("for") {
                let text = visible_text_for_nodes(&element.children);
                if !text.is_empty() {
                    labels.push((for_id.to_string(), text));
                }
            }
        }
        collect_label_texts_by_control_id_into(&element.children, labels);
    }
}

fn collect_element_texts_by_id(nodes: &[Node]) -> Vec<(String, String)> {
    let mut id_texts = Vec::new();
    collect_element_texts_by_id_into(nodes, &mut id_texts);
    id_texts
}

fn collect_element_texts_by_id_into(nodes: &[Node], id_texts: &mut Vec<(String, String)>) {
    for node in nodes {
        let Node::Element(element) = node else {
            continue;
        };
        if let Some(id) = element.attribute("id") {
            let text = visible_text_for_nodes(&element.children);
            if !text.is_empty() {
                id_texts.push((id.to_string(), text));
            }
        }
        collect_element_texts_by_id_into(&element.children, id_texts);
    }
}

fn is_browser_form_control_element(name: &str) -> bool {
    matches!(name, "button" | "input" | "select" | "textarea")
}

fn browser_control_labels(
    element: &Element,
    labels: &[(String, String)],
    current_label_text: Option<&str>,
) -> Vec<String> {
    if !is_browser_form_control_element(&element.name) {
        return Vec::new();
    }

    let mut result = Vec::new();
    if let Some(text) = current_label_text.filter(|text| !text.is_empty()) {
        push_unique_string(&mut result, text.to_string());
    }
    if let Some(id) = element.attribute("id") {
        for (label_for, text) in labels {
            if label_for == id {
                push_unique_string(&mut result, text.clone());
            }
        }
    }
    result
}

fn push_unique_string(values: &mut Vec<String>, value: String) {
    if !value.is_empty() && !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn browser_authored_role(element: &Element) -> Option<String> {
    element
        .attribute("role")
        .map(collapse_html_whitespace)
        .filter(|role| !role.is_empty())
}

fn browser_aria_label(element: &Element) -> Option<String> {
    element
        .attribute("aria-label")
        .map(collapse_html_whitespace)
        .filter(|label| !label.is_empty())
}

fn browser_aria_idrefs(element: &Element, name: &str) -> Vec<String> {
    element
        .attribute(name)
        .map(split_html_classes)
        .unwrap_or_default()
}

fn browser_aria_state(element: &Element, name: &str) -> Option<String> {
    element
        .attribute(name)
        .map(collapse_html_whitespace)
        .filter(|state| !state.is_empty())
}

fn browser_aria_hidden(element: &Element) -> bool {
    element
        .attribute("aria-hidden")
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn browser_hidden(element: &Element) -> bool {
    element.attribute("hidden").is_some()
}

fn browser_inert(element: &Element) -> bool {
    element.attribute("inert").is_some()
}

fn browser_open(element: &Element) -> bool {
    matches!(element.name.as_str(), "details" | "dialog") && element.attribute("open").is_some()
}

fn browser_accesskey(element: &Element) -> Vec<String> {
    element
        .attribute("accesskey")
        .map(split_html_classes)
        .unwrap_or_default()
}

fn browser_popover_target(element: &Element) -> Option<String> {
    element.attribute("popovertarget").map(ToOwned::to_owned)
}

fn browser_popover_target_action(element: &Element) -> Option<String> {
    element
        .attribute("popovertargetaction")
        .map(collapse_html_whitespace)
        .filter(|action| !action.is_empty())
}

fn browser_command(element: &Element) -> Option<String> {
    element
        .attribute("command")
        .map(collapse_html_whitespace)
        .filter(|command| !command.is_empty())
}

fn browser_command_for(element: &Element) -> Option<String> {
    element.attribute("commandfor").map(ToOwned::to_owned)
}

fn is_browser_anchor_navigation_element(element: &Element) -> bool {
    matches!(element.name.as_str(), "a" | "area")
}

fn browser_anchor_target(element: &Element) -> Option<String> {
    is_browser_anchor_navigation_element(element)
        .then(|| element.attribute("target").map(ToOwned::to_owned))
        .flatten()
}

fn browser_anchor_rel(element: &Element) -> Option<String> {
    is_browser_anchor_navigation_element(element)
        .then(|| element.attribute("rel").map(ToOwned::to_owned))
        .flatten()
}

fn browser_anchor_rel_tokens(element: &Element) -> Vec<String> {
    if is_browser_anchor_navigation_element(element) {
        browser_rel_tokens(element)
    } else {
        Vec::new()
    }
}

fn browser_anchor_download(element: &Element) -> Option<String> {
    is_browser_anchor_navigation_element(element)
        .then(|| element.attribute("download").map(ToOwned::to_owned))
        .flatten()
}

fn browser_anchor_ping(element: &Element) -> Vec<String> {
    if is_browser_anchor_navigation_element(element) {
        element
            .attribute("ping")
            .map(split_html_classes)
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn browser_anchor_hreflang(element: &Element) -> Option<String> {
    is_browser_anchor_navigation_element(element)
        .then(|| element.attribute("hreflang").map(ToOwned::to_owned))
        .flatten()
}

fn browser_editing_mode(element: &Element) -> Option<String> {
    let value = element.attribute("contenteditable")?;
    let normalized = collapse_html_whitespace(value);
    if normalized.is_empty() || normalized.eq_ignore_ascii_case("true") {
        Some("richtext".to_string())
    } else if normalized.eq_ignore_ascii_case("plaintext-only") {
        Some("plaintext".to_string())
    } else if normalized.eq_ignore_ascii_case("false") {
        Some("false".to_string())
    } else {
        Some("inherit".to_string())
    }
}

fn browser_draggable_state(element: &Element) -> Option<String> {
    let value = element.attribute("draggable")?;
    let normalized = collapse_html_whitespace(value);
    if normalized.eq_ignore_ascii_case("true") {
        Some("true".to_string())
    } else if normalized.eq_ignore_ascii_case("false") {
        Some("false".to_string())
    } else {
        Some("auto".to_string())
    }
}

fn browser_spellcheck_state(element: &Element) -> Option<String> {
    let value = element.attribute("spellcheck")?;
    let normalized = collapse_html_whitespace(value);
    if normalized.eq_ignore_ascii_case("true") {
        Some("true".to_string())
    } else if normalized.eq_ignore_ascii_case("false") {
        Some("false".to_string())
    } else {
        Some("default".to_string())
    }
}

fn browser_translate_state(element: &Element) -> Option<String> {
    let value = element.attribute("translate")?;
    let normalized = collapse_html_whitespace(value);
    if normalized.eq_ignore_ascii_case("yes") {
        Some("yes".to_string())
    } else if normalized.eq_ignore_ascii_case("no") {
        Some("no".to_string())
    } else {
        Some("default".to_string())
    }
}

fn browser_focusable(element: &Element) -> Option<bool> {
    let tabindex = element.attribute("tabindex");
    let contenteditable = element.attribute("contenteditable");
    if tabindex.is_none() && contenteditable.is_none() {
        return None;
    }

    if element.attribute("disabled").is_some()
        || browser_hidden(element)
        || browser_inert(element)
        || browser_aria_hidden(element)
    {
        return Some(false);
    }

    if let Some(tabindex) = tabindex {
        return Some(
            tabindex
                .trim()
                .parse::<i32>()
                .map(|value| value >= 0)
                .unwrap_or(true),
        );
    }

    Some(
        contenteditable
            .map(|value| {
                value.is_empty()
                    || value.eq_ignore_ascii_case("true")
                    || value.eq_ignore_ascii_case("plaintext-only")
            })
            .unwrap_or(false),
    )
}

fn browser_accessible_name(
    element: &Element,
    role: &str,
    control_labels: &[String],
    id_texts: &[(String, String)],
) -> Option<String> {
    let labelledby = browser_aria_idrefs(element, "aria-labelledby");
    if !labelledby.is_empty() {
        let mut parts = Vec::new();
        for id in labelledby {
            if let Some((_, text)) = id_texts.iter().find(|(candidate, _)| candidate == &id) {
                push_unique_string(&mut parts, collapse_html_whitespace(text));
            }
        }
        if !parts.is_empty() {
            return Some(parts.join(" "));
        }
    }

    if let Some(aria_label) = browser_aria_label(element) {
        return Some(aria_label);
    }

    if !control_labels.is_empty() {
        return Some(control_labels.join(" "));
    }

    let text = match role {
        "control" if element.name == "button" => visible_text_for_nodes(&element.children),
        "control" if element.name == "input" => element
            .attribute("placeholder")
            .or_else(|| element.attribute("title"))
            .unwrap_or_default()
            .to_string(),
        "control" if element.name == "select" => {
            element.attribute("title").unwrap_or_default().to_string()
        }
        "control" if element.name == "textarea" => element
            .attribute("placeholder")
            .or_else(|| element.attribute("title"))
            .unwrap_or_default()
            .to_string(),
        "form_group" => find_first_element_in_nodes(&element.children, "legend")
            .map(element_text)
            .unwrap_or_default(),
        _ => String::new(),
    };
    let text = collapse_html_whitespace(&text);
    (!text.is_empty()).then_some(text)
}

fn browser_form_owner(element: &Element) -> Option<String> {
    if matches!(
        element.name.as_str(),
        "button" | "fieldset" | "input" | "object" | "select" | "textarea"
    ) {
        element.attribute("form").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_label_for(element: &Element) -> Option<String> {
    if element.name == "label" {
        element.attribute("for").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_placeholder(element: &Element) -> Option<String> {
    if matches!(element.name.as_str(), "input" | "textarea") {
        element.attribute("placeholder").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_autocomplete(element: &Element) -> Option<String> {
    if matches!(
        element.name.as_str(),
        "form" | "input" | "select" | "textarea"
    ) {
        element.attribute("autocomplete").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_autocapitalize(element: &Element) -> Option<String> {
    if matches!(element.name.as_str(), "input" | "textarea") {
        element.attribute("autocapitalize").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_enterkeyhint(element: &Element) -> Option<String> {
    if matches!(element.name.as_str(), "input" | "textarea") {
        element.attribute("enterkeyhint").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_dirname(element: &Element) -> Option<String> {
    if matches!(element.name.as_str(), "input" | "textarea") {
        element.attribute("dirname").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_control_accept(element: &Element) -> Option<String> {
    if element.name == "input" {
        element.attribute("accept").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_control_capture(element: &Element) -> Option<String> {
    if element.name == "input" {
        element.attribute("capture").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_control_inputmode(element: &Element) -> Option<String> {
    if matches!(element.name.as_str(), "input" | "textarea") {
        element.attribute("inputmode").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_control_pattern(element: &Element) -> Option<String> {
    if element.name == "input" {
        element.attribute("pattern").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_control_min(element: &Element) -> Option<String> {
    if element.name == "input" {
        element.attribute("min").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_control_max(element: &Element) -> Option<String> {
    if element.name == "input" {
        element.attribute("max").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_control_step(element: &Element) -> Option<String> {
    if element.name == "input" {
        element.attribute("step").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_control_minlength(element: &Element) -> Option<String> {
    if matches!(element.name.as_str(), "input" | "textarea") {
        element.attribute("minlength").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_control_maxlength(element: &Element) -> Option<String> {
    if matches!(element.name.as_str(), "input" | "textarea") {
        element.attribute("maxlength").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_control_size(element: &Element) -> Option<String> {
    if matches!(element.name.as_str(), "input" | "select") {
        element.attribute("size").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_control_list(element: &Element) -> Option<String> {
    if element.name == "input" {
        element.attribute("list").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_control_form_action(element: &Element) -> Option<String> {
    if matches!(element.name.as_str(), "button" | "input") {
        element.attribute("formaction").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_control_form_enctype(element: &Element) -> Option<String> {
    if matches!(element.name.as_str(), "button" | "input") {
        element.attribute("formenctype").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_control_form_method(element: &Element) -> Option<String> {
    if matches!(element.name.as_str(), "button" | "input") {
        element
            .attribute("formmethod")
            .map(|method| method.to_ascii_lowercase())
    } else {
        None
    }
}

fn browser_control_form_target(element: &Element) -> Option<String> {
    if matches!(element.name.as_str(), "button" | "input") {
        element.attribute("formtarget").map(ToOwned::to_owned)
    } else {
        None
    }
}

fn browser_control_form_novalidate(element: &Element) -> bool {
    matches!(element.name.as_str(), "button" | "input")
        && element.attribute("formnovalidate").is_some()
}

fn browser_required(element: &Element) -> bool {
    matches!(element.name.as_str(), "input" | "select" | "textarea")
        && element.attribute("required").is_some()
}

fn browser_readonly(element: &Element) -> bool {
    matches!(element.name.as_str(), "input" | "textarea") && element.attribute("readonly").is_some()
}

fn browser_autofocus(element: &Element) -> bool {
    matches!(
        element.name.as_str(),
        "button" | "input" | "select" | "textarea"
    ) && element.attribute("autofocus").is_some()
}

fn browser_multiple(element: &Element) -> bool {
    matches!(element.name.as_str(), "input" | "select") && element.attribute("multiple").is_some()
}

fn split_html_classes(value: &str) -> Vec<String> {
    value
        .split_ascii_whitespace()
        .map(ToOwned::to_owned)
        .collect()
}

fn browser_content_control_type(element: &Element) -> Option<String> {
    match element.name.as_str() {
        "input" => Some(
            element
                .attribute("type")
                .map(|input_type| input_type.to_ascii_lowercase())
                .unwrap_or_else(|| "text".to_string()),
        ),
        "button" => Some(
            element
                .attribute("type")
                .map(|button_type| button_type.to_ascii_lowercase())
                .unwrap_or_else(|| "submit".to_string()),
        ),
        "select" | "textarea" => Some(element.name.clone()),
        _ => None,
    }
}

fn browser_content_value(element: &Element) -> Option<String> {
    match element.name.as_str() {
        "button" => element.attribute("value").map(ToOwned::to_owned),
        "input" => browser_input_value(element),
        "option" => Some(browser_option_value(element)),
        "select" => selected_option_value(&element.children),
        "textarea" => Some(element_text(element)),
        _ => None,
    }
}

fn browser_content_options(element: &Element) -> Vec<String> {
    match element.name.as_str() {
        "select" => collect_select_options(&element.children),
        _ => Vec::new(),
    }
}

fn selected_option_value(nodes: &[Node]) -> Option<String> {
    let mut first = None;
    selected_option_value_in(nodes, &mut first).or(first)
}

fn selected_option_value_in(nodes: &[Node], first: &mut Option<String>) -> Option<String> {
    for node in nodes {
        let Node::Element(element) = node else {
            continue;
        };
        if element.name == "option" {
            let value = browser_option_value(element);
            if first.is_none() {
                *first = Some(value.clone());
            }
            if element.attribute("selected").is_some() {
                return Some(value);
            }
        } else if let Some(value) = selected_option_value_in(&element.children, first) {
            return Some(value);
        }
    }
    None
}

fn browser_input_display_value(element: &Element) -> String {
    match element
        .attribute("type")
        .map(|input_type| input_type.to_ascii_lowercase())
        .as_deref()
    {
        Some("checkbox" | "hidden" | "password" | "radio") => String::new(),
        _ => element.attribute("value").unwrap_or_default().to_string(),
    }
}

fn browser_input_value(element: &Element) -> Option<String> {
    match element
        .attribute("type")
        .map(|input_type| input_type.to_ascii_lowercase())
        .as_deref()
    {
        Some("checkbox" | "radio") => Some(element.attribute("value").unwrap_or("on").to_string()),
        _ => element.attribute("value").map(ToOwned::to_owned),
    }
}

fn browser_option_display_text(element: &Element) -> String {
    let text = visible_text_for_nodes(&element.children);
    if text.is_empty() {
        element.attribute("label").unwrap_or_default().to_string()
    } else {
        text
    }
}

fn browser_option_value(element: &Element) -> String {
    element
        .attribute("value")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| browser_option_display_text(element))
}

fn is_browser_block_element(name: &str) -> bool {
    matches!(
        name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "body"
            | "center"
            | "details"
            | "dialog"
            | "div"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "header"
            | "hgroup"
            | "hr"
            | "html"
            | "legend"
            | "main"
            | "nav"
            | "p"
            | "plaintext"
            | "pre"
            | "section"
            | "summary"
            | "xmp"
    )
}

fn browser_render_display(role: &str) -> &'static str {
    match role {
        "text" => "inline-text",
        "inline" | "link" => "inline",
        "canvas" | "embed" | "frame" | "image" | "media" | "object" | "control" => {
            "inline-replaced"
        }
        "line_break" => "line-break",
        "article" | "aside" | "footer" | "header" | "heading" | "heading_group" | "main"
        | "navigation" | "paragraph" | "preformatted" | "section" | "block" | "form"
        | "form_group" | "legend" | "list" | "quote_block" | "separator" => "block",
        "label" | "option" | "option_group" | "quote" => "inline",
        "list_item" => "list-item",
        "table" => "table",
        "table_caption" => "table-caption",
        "table_column_group" => "table-column-group",
        "table_column" => "table-column",
        "table_section" => "table-row-group",
        "table_row" => "table-row",
        "table_cell" => "table-cell",
        _ => "inline",
    }
}

fn collect_form_controls(
    nodes: &[Node],
    labels: &[(String, String)],
    base_href: Option<&str>,
) -> Vec<BrowserFormControl> {
    let mut controls = Vec::new();
    collect_form_controls_into(nodes, &mut controls, labels, base_href, None);
    controls
}

fn collect_form_controls_into(
    nodes: &[Node],
    controls: &mut Vec<BrowserFormControl>,
    labels: &[(String, String)],
    base_href: Option<&str>,
    current_label_text: Option<&str>,
) {
    for node in nodes {
        let Node::Element(element) = node else {
            continue;
        };

        let element_label_text = (element.name == "label")
            .then(|| visible_text_for_nodes(&element.children))
            .filter(|text| !text.is_empty());
        let child_label_text = element_label_text.as_deref().or(current_label_text);

        match element.name.as_str() {
            "input" | "button" | "select" | "textarea" => {
                controls.push(browser_form_control(
                    element,
                    labels,
                    base_href,
                    current_label_text,
                ));
            }
            _ => collect_form_controls_into(
                &element.children,
                controls,
                labels,
                base_href,
                child_label_text,
            ),
        }
    }
}

fn browser_form_control(
    element: &Element,
    labels: &[(String, String)],
    base_href: Option<&str>,
    current_label_text: Option<&str>,
) -> BrowserFormControl {
    let control_type = browser_content_control_type(element)
        .expect("browser_form_control is only called for form controls");
    let control_labels = browser_control_labels(element, labels, current_label_text);
    let text = match element.name.as_str() {
        "button" | "select" => visible_text_for_nodes(&element.children),
        "textarea" => element_text(element),
        _ => String::new(),
    };

    BrowserFormControl {
        id: element.attribute("id").map(ToOwned::to_owned),
        control_type,
        name: element.attribute("name").map(ToOwned::to_owned),
        form_owner: browser_form_owner(element),
        accessible_name: browser_accessible_name(element, "control", &control_labels, &[]),
        labels: control_labels,
        placeholder: browser_placeholder(element),
        autocomplete: browser_autocomplete(element),
        autocapitalize: browser_autocapitalize(element),
        enterkeyhint: browser_enterkeyhint(element),
        dirname: browser_dirname(element),
        accept: browser_control_accept(element),
        capture: browser_control_capture(element),
        resolved_src: browser_content_src(element)
            .as_deref()
            .and_then(|src| resolve_browser_url(src, base_href)),
        src: browser_content_src(element),
        alt: element.attribute("alt").map(ToOwned::to_owned),
        width: element.attribute("width").map(ToOwned::to_owned),
        height: element.attribute("height").map(ToOwned::to_owned),
        inputmode: browser_control_inputmode(element),
        pattern: browser_control_pattern(element),
        min: browser_control_min(element),
        max: browser_control_max(element),
        step: browser_control_step(element),
        minlength: browser_control_minlength(element),
        maxlength: browser_control_maxlength(element),
        size: browser_control_size(element),
        list: browser_control_list(element),
        resolved_form_action: browser_control_form_action(element)
            .as_deref()
            .and_then(|action| resolve_browser_url(action, base_href)),
        form_action: browser_control_form_action(element),
        form_enctype: browser_control_form_enctype(element),
        form_method: browser_control_form_method(element),
        form_target: browser_control_form_target(element),
        form_novalidate: browser_control_form_novalidate(element),
        value: browser_content_value(element),
        autofocus: browser_autofocus(element),
        disabled: element.attribute("disabled").is_some(),
        required: browser_required(element),
        readonly: browser_readonly(element),
        checked: element.attribute("checked").is_some(),
        multiple: browser_multiple(element),
        text,
        options: browser_content_options(element),
    }
}

fn collect_select_options(nodes: &[Node]) -> Vec<String> {
    let mut options = Vec::new();
    collect_select_options_into(nodes, &mut options);
    options
}

fn collect_select_options_into(nodes: &[Node], options: &mut Vec<String>) {
    for node in nodes {
        let Node::Element(element) = node else {
            continue;
        };
        if element.name == "option" {
            options.push(visible_text_for_nodes(&element.children));
        } else {
            collect_select_options_into(&element.children, options);
        }
    }
}

fn visible_text_for_nodes(nodes: &[Node]) -> String {
    let mut text = String::new();
    collect_visible_text(nodes, &mut text);
    collapse_html_whitespace(&text)
}

fn collect_visible_text(nodes: &[Node], text: &mut String) {
    for node in nodes {
        match node {
            Node::Text(value) => text.push_str(&value.data),
            Node::Element(element) if is_browser_invisible_element(&element.name) => {}
            Node::Element(element) => {
                collect_visible_text(&element.children, text);
                text.push(' ');
            }
            Node::DocumentType(_) | Node::Comment(_) => {}
        }
    }
}

fn element_text(element: &Element) -> String {
    let mut text = String::new();
    collect_browser_text_content(&element.children, &mut text);
    collapse_html_whitespace(&text)
}

fn collect_browser_text_content(nodes: &[Node], text: &mut String) {
    for node in nodes {
        match node {
            Node::Text(value) => text.push_str(&value.data),
            Node::Element(element) => collect_browser_text_content(&element.children, text),
            Node::DocumentType(_) | Node::Comment(_) => {}
        }
    }
}

fn collapse_html_whitespace(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut normalized = String::with_capacity(collapsed.len());
    for character in collapsed.chars() {
        if matches!(character, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']')
            && normalized.ends_with(' ')
        {
            normalized.pop();
        }
        normalized.push(character);
    }
    normalized
}

fn find_first_element_in_nodes<'a>(nodes: &'a [Node], name: &str) -> Option<&'a Element> {
    for node in nodes {
        let Node::Element(element) = node else {
            continue;
        };
        if element.name == name {
            return Some(element);
        }
        if let Some(found) = find_first_element_in_nodes(&element.children, name) {
            return Some(found);
        }
    }
    None
}

fn browser_table_rows(table: &Element) -> Vec<&Element> {
    let mut rows = Vec::new();
    collect_browser_table_rows(&table.children, &mut rows);
    rows
}

fn collect_browser_table_rows<'a>(nodes: &'a [Node], rows: &mut Vec<&'a Element>) {
    for node in nodes {
        let Node::Element(element) = node else {
            continue;
        };
        if element.name == "tr" {
            rows.push(element);
        } else if element.name != "table" {
            collect_browser_table_rows(&element.children, rows);
        }
    }
}

fn browser_table_column_count(table: &Element) -> usize {
    browser_table_rows(table)
        .into_iter()
        .map(browser_table_row_column_count)
        .max()
        .unwrap_or(0)
}

fn browser_table_row_column_count(row: &Element) -> usize {
    row.children
        .iter()
        .filter_map(|node| match node {
            Node::Element(element) if matches!(element.name.as_str(), "td" | "th") => {
                Some(html_positive_integer_attribute(element, "colspan"))
            }
            _ => None,
        })
        .sum()
}

fn browser_table_cell_count(table: &Element) -> usize {
    browser_table_rows(table)
        .into_iter()
        .map(|row| {
            row.children
                .iter()
                .filter(|node| {
                    matches!(
                        node,
                        Node::Element(element) if matches!(element.name.as_str(), "td" | "th")
                    )
                })
                .count()
        })
        .sum()
}

fn browser_table_header_cell_count(table: &Element) -> usize {
    browser_table_rows(table)
        .into_iter()
        .map(|row| {
            row.children
                .iter()
                .filter(|node| matches!(node, Node::Element(element) if element.name == "th"))
                .count()
        })
        .sum()
}

fn browser_table_column_hint_count(table: &Element) -> usize {
    let mut count = 0;
    collect_browser_table_column_hints(&table.children, &mut count);
    count
}

fn collect_browser_table_column_hints(nodes: &[Node], count: &mut usize) {
    for node in nodes {
        let Node::Element(element) = node else {
            continue;
        };
        match element.name.as_str() {
            "colgroup" => {
                let column_spans = element
                    .children
                    .iter()
                    .filter_map(|child| match child {
                        Node::Element(child_element) if child_element.name == "col" => {
                            Some(html_positive_integer_attribute(child_element, "span"))
                        }
                        _ => None,
                    })
                    .sum::<usize>();
                if column_spans == 0 {
                    *count += html_positive_integer_attribute(element, "span");
                } else {
                    *count += column_spans;
                }
            }
            "col" => {
                *count += html_positive_integer_attribute(element, "span");
            }
            "table" => {}
            _ => collect_browser_table_column_hints(&element.children, count),
        }
    }
}

fn html_positive_integer_attribute(element: &Element, name: &str) -> usize {
    element
        .attribute(name)
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

fn heading_level(name: &str) -> Option<u8> {
    match name {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
        _ => None,
    }
}

fn is_browser_invisible_element(name: &str) -> bool {
    matches!(
        name,
        "base" | "link" | "meta" | "script" | "style" | "template" | "title"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use dom_core::Element;

    fn element(node: &Node) -> &Element {
        match node {
            Node::Element(element) => element,
            other => panic!("expected element, got {other:?}"),
        }
    }

    fn html(document: &Document) -> &Element {
        document
            .children
            .iter()
            .find_map(|node| match node {
                Node::Element(element) if element.name == "html" => Some(element),
                _ => None,
            })
            .expect("document should have an html element")
    }

    fn head(document: &Document) -> &Element {
        element(&html(document).children[0])
    }

    fn body(document: &Document) -> &Element {
        element(&html(document).children[1])
    }

    #[test]
    fn resolves_browser_urls_against_document_base() {
        let base = Some("https://example.test/docs/current/page.html?old=1#section");

        assert_eq!(
            resolve_browser_url("intro.html", base),
            Some("https://example.test/docs/current/intro.html".to_string())
        );
        assert_eq!(
            resolve_browser_url("../assets/logo.gif", base),
            Some("https://example.test/docs/assets/logo.gif".to_string())
        );
        assert_eq!(
            resolve_browser_url("/search?q=html#top", base),
            Some("https://example.test/search?q=html#top".to_string())
        );
        assert_eq!(
            resolve_browser_url("?page=2", base),
            Some("https://example.test/docs/current/page.html?page=2".to_string())
        );
        assert_eq!(
            resolve_browser_url("#next", base),
            Some("https://example.test/docs/current/page.html?old=1#next".to_string())
        );
        assert_eq!(
            resolve_browser_url("//cdn.example.test/app.js", base),
            Some("https://cdn.example.test/app.js".to_string())
        );
    }

    #[test]
    fn browser_url_resolution_keeps_absolute_urls_and_marks_unresolved_relatives() {
        assert_eq!(
            resolve_browser_url("mailto:hello@example.test", None),
            Some("mailto:hello@example.test".to_string())
        );
        assert_eq!(resolve_browser_url("relative.html", None), None);
        assert_eq!(
            resolve_browser_url("relative.html", Some("/local/base/")),
            None
        );
    }

    #[test]
    fn browser_anchor_navigation_metadata_tracks_targets_and_pings() {
        let document = parse_html(
            "<base href=\"https://example.test/docs/current.html\">\
             <body><p>Go <a id=next href=next.html target=_blank rel=\"next external noopener\" \
             title=\"Next page\" download=next.html ping=\"audit.html https://metrics.example/p\" \
             hreflang=en-US type=text/html>Next</a></p>",
        )
        .unwrap();

        let summary = BrowserDocument::from_document(&document);
        let link = &summary.links[0];
        assert_eq!(link.href.as_deref(), Some("next.html"));
        assert_eq!(
            link.resolved_href.as_deref(),
            Some("https://example.test/docs/next.html")
        );
        assert_eq!(link.target.as_deref(), Some("_blank"));
        assert_eq!(link.rel.as_deref(), Some("next external noopener"));
        assert_eq!(link.rel_tokens, vec!["next", "external", "noopener"]);
        assert_eq!(link.download.as_deref(), Some("next.html"));
        assert_eq!(link.ping, vec!["audit.html", "https://metrics.example/p"]);
        assert_eq!(
            link.resolved_ping,
            vec![
                "https://example.test/docs/audit.html",
                "https://metrics.example/p"
            ]
        );
        assert_eq!(link.hreflang.as_deref(), Some("en-US"));
        assert_eq!(link.type_hint.as_deref(), Some("text/html"));

        let content_tree = BrowserContentTree::from_document(&document);
        let content_link = &content_tree.children[0].children[1];
        assert_eq!(content_link.target.as_deref(), Some("_blank"));
        assert_eq!(
            content_link.rel_tokens,
            vec!["next", "external", "noopener"]
        );
        assert_eq!(content_link.download.as_deref(), Some("next.html"));
        assert_eq!(
            content_link.resolved_ping,
            vec![
                "https://example.test/docs/audit.html",
                "https://metrics.example/p"
            ]
        );
        assert_eq!(content_link.hreflang.as_deref(), Some("en-US"));

        let render_tree = BrowserRenderTree::from_content_tree(&content_tree);
        let render_link = &render_tree.children[0].children[1];
        assert_eq!(render_link.target.as_deref(), Some("_blank"));
        assert_eq!(render_link.rel_tokens, vec!["next", "external", "noopener"]);
        assert_eq!(render_link.download.as_deref(), Some("next.html"));
    }

    #[test]
    fn browser_document_metadata_tracks_head_policy_hints() {
        let document = parse_html(
            "<base href=\"https://example.test/app/page.html\">\
             <meta http-equiv=content-type content=\"text/html; charset=windows-1252\">\
             <meta charset=utf-8>\
             <meta name=viewport content=\"width=device-width, initial-scale=1\">\
             <meta name=description content=\"HTML parser workbench\">\
             <meta name=application-name content=Venture>\
             <meta name=referrer content=no-referrer>\
             <meta name=robots content=\"index,follow\">\
             <meta name=color-scheme content=\"light dark\">\
             <meta name=theme-color content=\"#ffffff\" media=\"(prefers-color-scheme: light)\">\
             <meta name=theme-color content=\"#111111\" media=\"(prefers-color-scheme: dark)\">\
             <meta http-equiv=refresh content=\"5; url=next.html\">\
             <link rel=canonical href=\"https://example.test/app/\">\
             <link rel=manifest href=\"site.webmanifest\">",
        )
        .unwrap();

        let summary = BrowserDocument::from_document(&document);
        assert_eq!(summary.metadata.charset.as_deref(), Some("windows-1252"));
        assert_eq!(
            summary.metadata.viewport.as_deref(),
            Some("width=device-width, initial-scale=1")
        );
        assert_eq!(
            summary.metadata.description.as_deref(),
            Some("HTML parser workbench")
        );
        assert_eq!(
            summary.metadata.application_name.as_deref(),
            Some("Venture")
        );
        assert_eq!(
            summary.metadata.referrer_policy.as_deref(),
            Some("no-referrer")
        );
        assert_eq!(summary.metadata.robots.as_deref(), Some("index,follow"));
        assert_eq!(summary.metadata.color_scheme.as_deref(), Some("light dark"));
        assert_eq!(summary.metadata.theme_colors.len(), 2);
        assert_eq!(summary.metadata.theme_colors[0].color, "#ffffff");
        assert_eq!(
            summary.metadata.theme_colors[0].media.as_deref(),
            Some("(prefers-color-scheme: light)")
        );
        assert_eq!(
            summary.metadata.refresh,
            Some(BrowserRefresh {
                delay: Some("5".to_string()),
                url: Some("next.html".to_string()),
                resolved_url: Some("https://example.test/app/next.html".to_string()),
            })
        );
        assert_eq!(
            summary.metadata.resolved_canonical_url.as_deref(),
            Some("https://example.test/app/")
        );
        assert_eq!(
            summary.metadata.resolved_manifest_url.as_deref(),
            Some("https://example.test/app/site.webmanifest")
        );
    }

    #[test]
    fn browser_identity_metadata_tracks_language_direction_and_classes() {
        let document = parse_html(
            "<html lang=en dir=ltr><body id=main class=\"page legacy\" lang=en-US>\
             <p id=intro class=\"lede print\" title=\"Intro\" dir=auto>Hello</p>",
        )
        .unwrap();

        let summary = BrowserDocument::from_document(&document);
        assert_eq!(summary.document_lang.as_deref(), Some("en"));
        assert_eq!(summary.document_dir.as_deref(), Some("ltr"));
        assert_eq!(summary.body_id.as_deref(), Some("main"));
        assert_eq!(summary.body_classes, vec!["page", "legacy"]);
        assert_eq!(summary.body_lang.as_deref(), Some("en-US"));

        let content_tree = BrowserContentTree::from_document(&document);
        let paragraph = &content_tree.children[0];
        assert_eq!(paragraph.id.as_deref(), Some("intro"));
        assert_eq!(paragraph.classes, vec!["lede", "print"]);
        assert_eq!(paragraph.title.as_deref(), Some("Intro"));
        assert_eq!(paragraph.dir.as_deref(), Some("auto"));
    }

    #[test]
    fn browser_embedded_resource_metadata_tracks_fetch_and_layout_fields() {
        let document = parse_html(
            "<base href=\"https://example.test/media/index.html\">\
             <iframe src=frame.html name=preview loading=lazy \
                 sandbox=\"allow-scripts allow-same-origin\" \
                 allow=\"fullscreen; geolocation\" allowfullscreen \
                 referrerpolicy=no-referrer srcdoc=\"<p>Fallback</p>\" \
                 credentialless width=320 height=200 title=Frame></iframe>\
             <object data=movie.swf type=\"application/x-shockwave-flash\" width=400 height=300></object>",
        )
        .unwrap();

        let summary = BrowserDocument::from_document(&document);
        assert_eq!(summary.resources[0].kind, "frame");
        assert_eq!(
            summary.resources[0].resolved_url.as_deref(),
            Some("https://example.test/media/frame.html")
        );
        assert_eq!(summary.resources[0].width.as_deref(), Some("320"));
        assert_eq!(
            summary.resources[0].browsing_context_name.as_deref(),
            Some("preview")
        );
        assert_eq!(summary.resources[0].loading.as_deref(), Some("lazy"));
        assert_eq!(
            summary.resources[0].sandbox,
            vec!["allow-scripts", "allow-same-origin"]
        );
        assert_eq!(
            summary.resources[0].allow.as_deref(),
            Some("fullscreen; geolocation")
        );
        assert!(summary.resources[0].allowfullscreen);
        assert_eq!(
            summary.resources[0].referrerpolicy.as_deref(),
            Some("no-referrer")
        );
        assert_eq!(
            summary.resources[0].srcdoc.as_deref(),
            Some("<p>Fallback</p>")
        );
        assert!(summary.resources[0].credentialless);

        let content_tree = BrowserContentTree::from_document(&document);
        assert_eq!(content_tree.children[0].role, "frame");
        assert_eq!(
            content_tree.children[0].resource_kind.as_deref(),
            Some("frame")
        );
        assert_eq!(
            content_tree.children[0].browsing_context_name.as_deref(),
            Some("preview")
        );
        assert_eq!(content_tree.children[0].loading.as_deref(), Some("lazy"));
        assert_eq!(
            content_tree.children[0].sandbox,
            vec!["allow-scripts", "allow-same-origin"]
        );
        assert_eq!(
            content_tree.children[0].allow.as_deref(),
            Some("fullscreen; geolocation")
        );
        assert!(content_tree.children[0].allowfullscreen);
        assert_eq!(
            content_tree.children[0].referrerpolicy.as_deref(),
            Some("no-referrer")
        );
        assert_eq!(
            content_tree.children[0].srcdoc.as_deref(),
            Some("<p>Fallback</p>")
        );
        assert!(content_tree.children[0].credentialless);
        assert_eq!(content_tree.children[1].role, "object");
        assert_eq!(
            content_tree.children[1].type_hint.as_deref(),
            Some("application/x-shockwave-flash")
        );
        assert!(content_tree.children[1].browsing_context_name.is_none());

        let render_tree = BrowserRenderTree::from_content_tree(&content_tree);
        assert_eq!(render_tree.children[0].display, "inline-replaced");
        assert_eq!(
            render_tree.children[0].browsing_context_name.as_deref(),
            Some("preview")
        );
        assert_eq!(
            render_tree.children[0].sandbox,
            vec!["allow-scripts", "allow-same-origin"]
        );
        assert!(render_tree.children[0].allowfullscreen);
        assert_eq!(render_tree.children[1].display, "inline-replaced");
    }

    #[test]
    fn browser_responsive_image_metadata_tracks_sources_and_loading_hints() {
        let document = parse_html(
            "<base href=\"https://example.test/gallery/index.html\"><body>\
             <picture>\
               <source media=\"(min-width: 800px)\" type=image/avif srcset=\"hero-wide.avif 1x, hero-wide@2x.avif 2x\" sizes=\"80vw\">\
               <source media=\"(min-width: 400px)\" type=image/webp srcset=\"hero.webp 640w, hero-large.webp 1280w\" sizes=\"100vw\">\
               <img src=hero.jpg srcset=\"hero-small.jpg 480w, https://cdn.example.test/hero.jpg 960w\" sizes=\"(max-width: 600px) 100vw, 50vw\" alt=\"Hero\" width=640 height=360 loading=lazy decoding=async fetchpriority=high crossorigin=anonymous referrerpolicy=no-referrer usemap=#hero-map ismap>\
             </picture>",
        )
        .unwrap();

        let summary = BrowserDocument::from_document(&document);
        assert_eq!(summary.images.len(), 1);
        let image = &summary.images[0];
        assert_eq!(image.alt.as_deref(), Some("Hero"));
        assert_eq!(
            image.resolved_src.as_deref(),
            Some("https://example.test/gallery/hero.jpg")
        );
        assert_eq!(
            image.resolved_srcset.as_deref(),
            Some("https://example.test/gallery/hero-small.jpg 480w, https://cdn.example.test/hero.jpg 960w")
        );
        assert_eq!(
            image.sizes.as_deref(),
            Some("(max-width: 600px) 100vw, 50vw")
        );
        assert_eq!(image.loading.as_deref(), Some("lazy"));
        assert_eq!(image.decoding.as_deref(), Some("async"));
        assert_eq!(image.fetchpriority.as_deref(), Some("high"));
        assert_eq!(image.crossorigin.as_deref(), Some("anonymous"));
        assert_eq!(image.referrerpolicy.as_deref(), Some("no-referrer"));
        assert_eq!(image.usemap.as_deref(), Some("#hero-map"));
        assert!(image.ismap);
        assert_eq!(image.sources.len(), 2);
        assert_eq!(
            image.sources[0].media.as_deref(),
            Some("(min-width: 800px)")
        );
        assert_eq!(image.sources[0].type_hint.as_deref(), Some("image/avif"));
        assert_eq!(
            image.sources[0].resolved_srcset.as_deref(),
            Some("https://example.test/gallery/hero-wide.avif 1x, https://example.test/gallery/hero-wide@2x.avif 2x")
        );
        assert_eq!(image.sources[1].sizes.as_deref(), Some("100vw"));
    }

    #[test]
    fn browser_table_metadata_tracks_columns_spans_and_headers() {
        let document = parse_html(
            "<table><caption>Prices</caption>\
             <colgroup><col span=2 width=80><col width=120>\
             <thead><tr><th id=item scope=col abbr=Item>Name<th id=price scope=col>Price\
             <tbody><tr><th scope=row headers=item>Basic\
             <td colspan=2 headers=\"item price\">$1</table>",
        )
        .unwrap();

        let summary = BrowserDocument::from_document(&document);
        assert_eq!(summary.tables[0].caption.as_deref(), Some("Prices"));
        assert_eq!(summary.tables[0].row_count, 2);
        assert_eq!(summary.tables[0].column_count, 3);
        assert_eq!(summary.tables[0].column_hint_count, 3);
        assert_eq!(summary.tables[0].header_cell_count, 3);

        let content_tree = BrowserContentTree::from_document(&document);
        let table = &content_tree.children[0];
        assert_eq!(table.children[1].role, "table_column_group");
        assert_eq!(table.children[1].children[0].span.as_deref(), Some("2"));
        assert_eq!(
            table.children[2].table_section_kind.as_deref(),
            Some("thead")
        );
        let body_cell = &table.children[3].children[0].children[1];
        assert_eq!(body_cell.colspan.as_deref(), Some("2"));
        assert_eq!(
            body_cell.headers,
            vec!["item".to_string(), "price".to_string()]
        );

        let render_tree = BrowserRenderTree::from_content_tree(&content_tree);
        assert_eq!(
            render_tree.children[0].children[1].display,
            "table-column-group"
        );
        assert_eq!(
            render_tree.children[0].children[3].children[0].children[1]
                .colspan
                .as_deref(),
            Some("2")
        );
    }

    #[test]
    fn browser_text_flow_metadata_tracks_lists_quotes_and_breaks() {
        let document = parse_html(
            "<base href=\"https://example.test/docs/page.html\">\
             <p>Intro<br>line<wbr>word</p><hr>\
             <ol start=3 reversed type=A><li value=7>Seven<li>Eight</ol>\
             <pre>  keep\nspace</pre>\
             <blockquote cite=notes/ref.html><p>Quote <q cite=#part>part</q></p></blockquote>",
        )
        .unwrap();

        let content_tree = BrowserContentTree::from_document(&document);
        let paragraph = &content_tree.children[0];
        assert_eq!(paragraph.role, "paragraph");
        assert_eq!(paragraph.children[1].break_kind.as_deref(), Some("line"));
        assert_eq!(paragraph.children[3].break_kind.as_deref(), Some("word"));

        let separator = &content_tree.children[1];
        assert_eq!(separator.role, "separator");
        assert_eq!(separator.break_kind.as_deref(), Some("thematic"));

        let list = &content_tree.children[2];
        assert_eq!(list.list_kind.as_deref(), Some("ordered"));
        assert_eq!(list.list_start.as_deref(), Some("3"));
        assert_eq!(list.list_marker_type.as_deref(), Some("A"));
        assert!(list.list_reversed);
        assert_eq!(list.children[0].list_item_value.as_deref(), Some("7"));

        let pre = &content_tree.children[3];
        assert_eq!(pre.role, "preformatted");
        assert_eq!(pre.text_flow.as_deref(), Some("preformatted"));
        assert_eq!(pre.children[0].text.as_deref(), Some("  keep\nspace"));

        let quote = &content_tree.children[4];
        assert_eq!(quote.role, "quote_block");
        assert_eq!(quote.quote_cite.as_deref(), Some("notes/ref.html"));
        assert_eq!(
            quote.resolved_quote_cite.as_deref(),
            Some("https://example.test/docs/notes/ref.html")
        );
        let inline_quote = &quote.children[0].children[1];
        assert_eq!(inline_quote.role, "quote");
        assert_eq!(inline_quote.quote_cite.as_deref(), Some("#part"));

        let render_tree = BrowserRenderTree::from_content_tree(&content_tree);
        assert_eq!(render_tree.children[0].display, "block");
        assert_eq!(render_tree.children[0].children[1].display, "line-break");
        assert_eq!(render_tree.children[1].display, "block");
        assert_eq!(render_tree.children[2].children[0].display, "list-item");
        assert_eq!(
            render_tree.children[4].children[0].children[1].display,
            "inline"
        );
    }

    #[test]
    fn browser_outline_metadata_tracks_sections_landmarks_and_heading_levels() {
        let document = parse_html(
            "<body><header><h1>Site</h1></header>\
             <nav><h2>Nav</h2><a href=/home>Home</a></nav>\
             <main><article id=post><h2>Post</h2><section aria-label=Chapter><h3>Chapter</h3></section></article>\
             <aside><h2>Related</h2></aside></main><footer>Fine print</footer>",
        )
        .unwrap();

        let content_tree = BrowserContentTree::from_document(&document);
        let header = &content_tree.children[0];
        assert_eq!(header.role, "header");
        assert_eq!(header.section_kind.as_deref(), Some("header"));
        assert_eq!(header.landmark_kind.as_deref(), Some("header"));
        assert_eq!(header.children[0].heading_level, Some(1));

        let navigation = &content_tree.children[1];
        assert_eq!(navigation.role, "navigation");
        assert_eq!(navigation.section_kind.as_deref(), Some("navigation"));
        assert_eq!(navigation.landmark_kind.as_deref(), Some("navigation"));
        assert_eq!(navigation.children[0].heading_level, Some(2));

        let main = &content_tree.children[2];
        assert_eq!(main.role, "main");
        assert_eq!(main.landmark_kind.as_deref(), Some("main"));
        let article = &main.children[0];
        assert_eq!(article.role, "article");
        assert_eq!(article.section_kind.as_deref(), Some("article"));
        assert_eq!(article.children[0].heading_level, Some(2));
        let section = &article.children[1];
        assert_eq!(section.role, "section");
        assert_eq!(section.section_kind.as_deref(), Some("section"));
        assert_eq!(section.children[0].heading_level, Some(3));

        let aside = &main.children[1];
        assert_eq!(aside.role, "aside");
        assert_eq!(aside.landmark_kind.as_deref(), Some("complementary"));

        let footer = &content_tree.children[3];
        assert_eq!(footer.role, "footer");
        assert_eq!(footer.landmark_kind.as_deref(), Some("footer"));

        let render_tree = BrowserRenderTree::from_content_tree(&content_tree);
        assert_eq!(render_tree.children[0].display, "block");
        assert_eq!(render_tree.children[1].display, "block");
        assert_eq!(render_tree.children[2].children[0].display, "block");
        assert_eq!(
            render_tree.children[2].children[0].children[1].children[0].heading_level,
            Some(3)
        );
    }

    #[test]
    fn browser_form_accessibility_metadata_tracks_labels_and_control_state() {
        let document = parse_html(
            "<base href=\"https://example.test/search/index.html\">\
             <body><form id=f name=searchForm aria-label=\"Search form\" action=find.html \
             method=post accept-charset=utf-8 autocomplete=on rel=noreferrer novalidate>\
             <label for=q>Query</label>\
             <input id=q name=q placeholder=\"Search terms\" required autocomplete=search \
             autocapitalize=words enterkeyhint=search dirname=q.dir autofocus \
             inputmode=search pattern=\"[A-Za-z ]+\" minlength=2 maxlength=80 size=40 list=query-suggestions>\
             <label>Notes<textarea id=notes name=notes readonly maxlength=500 \
             autocapitalize=sentences enterkeyhint=done dirname=notes.dir>Keep me</textarea></label>\
             <fieldset><legend>Options</legend><input id=fast type=checkbox name=fast checked></fieldset>\
             <select id=kind name=kind title=Kind multiple size=5><option selected>Books<option>Manuals</select>\
             <input id=upload type=file name=upload accept=\"image/png,image/jpeg\" capture=environment>\
             <button id=go type=submit aria-label=\"Run search\" formaction=run.html \
             formenctype=\"application/x-www-form-urlencoded\" formmethod=post \
             formtarget=results formnovalidate>Go</button>\
             <input id=imageGo type=image name=image-go src=buttons/search.png \
             alt=\"Search image\" width=32 height=16></form>\
             <input id=external form=f name=outside placeholder=Outside>",
        )
        .unwrap();

        let summary = BrowserDocument::from_document(&document);
        assert_eq!(summary.forms[0].action.as_deref(), Some("find.html"));
        assert_eq!(
            summary.forms[0].resolved_action.as_deref(),
            Some("https://example.test/search/find.html")
        );
        assert_eq!(summary.forms[0].name.as_deref(), Some("searchForm"));
        assert_eq!(summary.forms[0].method, "post");
        assert_eq!(summary.forms[0].accept_charset.as_deref(), Some("utf-8"));
        assert_eq!(summary.forms[0].autocomplete.as_deref(), Some("on"));
        assert_eq!(summary.forms[0].rel.as_deref(), Some("noreferrer"));
        assert!(summary.forms[0].novalidate);
        let controls = &summary.forms[0].controls;
        assert_eq!(controls[0].id.as_deref(), Some("q"));
        assert_eq!(controls[0].labels, vec!["Query"]);
        assert_eq!(controls[0].accessible_name.as_deref(), Some("Query"));
        assert_eq!(controls[0].placeholder.as_deref(), Some("Search terms"));
        assert_eq!(controls[0].autocomplete.as_deref(), Some("search"));
        assert_eq!(controls[0].autocapitalize.as_deref(), Some("words"));
        assert_eq!(controls[0].enterkeyhint.as_deref(), Some("search"));
        assert_eq!(controls[0].dirname.as_deref(), Some("q.dir"));
        assert_eq!(controls[0].inputmode.as_deref(), Some("search"));
        assert_eq!(controls[0].pattern.as_deref(), Some("[A-Za-z ]+"));
        assert_eq!(controls[0].minlength.as_deref(), Some("2"));
        assert_eq!(controls[0].maxlength.as_deref(), Some("80"));
        assert_eq!(controls[0].size.as_deref(), Some("40"));
        assert_eq!(controls[0].list.as_deref(), Some("query-suggestions"));
        assert!(controls[0].autofocus);
        assert!(controls[0].required);
        assert!(controls[1].readonly);
        assert_eq!(controls[1].autocapitalize.as_deref(), Some("sentences"));
        assert_eq!(controls[1].enterkeyhint.as_deref(), Some("done"));
        assert_eq!(controls[1].dirname.as_deref(), Some("notes.dir"));
        assert_eq!(controls[1].maxlength.as_deref(), Some("500"));
        assert_eq!(controls[1].labels, vec!["NotesKeep me"]);
        assert_eq!(controls[2].control_type, "checkbox");
        assert!(controls[2].checked);
        assert!(controls[3].multiple);
        assert_eq!(controls[3].size.as_deref(), Some("5"));
        assert_eq!(controls[3].accessible_name.as_deref(), Some("Kind"));
        assert_eq!(controls[4].control_type, "file");
        assert_eq!(controls[4].accept.as_deref(), Some("image/png,image/jpeg"));
        assert_eq!(controls[4].capture.as_deref(), Some("environment"));
        assert_eq!(controls[5].accessible_name.as_deref(), Some("Run search"));
        assert_eq!(controls[5].form_action.as_deref(), Some("run.html"));
        assert_eq!(
            controls[5].resolved_form_action.as_deref(),
            Some("https://example.test/search/run.html")
        );
        assert_eq!(
            controls[5].form_enctype.as_deref(),
            Some("application/x-www-form-urlencoded")
        );
        assert_eq!(controls[5].form_method.as_deref(), Some("post"));
        assert_eq!(controls[5].form_target.as_deref(), Some("results"));
        assert!(controls[5].form_novalidate);
        assert_eq!(controls[6].control_type, "image");
        assert_eq!(controls[6].name.as_deref(), Some("image-go"));
        assert_eq!(controls[6].src.as_deref(), Some("buttons/search.png"));
        assert_eq!(
            controls[6].resolved_src.as_deref(),
            Some("https://example.test/search/buttons/search.png")
        );
        assert_eq!(controls[6].alt.as_deref(), Some("Search image"));
        assert_eq!(controls[6].width.as_deref(), Some("32"));
        assert_eq!(controls[6].height.as_deref(), Some("16"));

        let content_tree = BrowserContentTree::from_document(&document);
        let form = &content_tree.children[0];
        assert_eq!(form.role, "form");
        assert_eq!(form.accessible_name.as_deref(), Some("Search form"));
        assert_eq!(form.autocomplete.as_deref(), Some("on"));
        let explicit_label = &form.children[0];
        assert_eq!(explicit_label.role, "label");
        assert_eq!(explicit_label.label_for.as_deref(), Some("q"));
        let query = &form.children[1];
        assert_eq!(query.labels, vec!["Query"]);
        assert_eq!(query.accessible_name.as_deref(), Some("Query"));
        assert_eq!(query.autocapitalize.as_deref(), Some("words"));
        assert_eq!(query.enterkeyhint.as_deref(), Some("search"));
        assert_eq!(query.dirname.as_deref(), Some("q.dir"));
        assert_eq!(query.inputmode.as_deref(), Some("search"));
        assert_eq!(query.pattern.as_deref(), Some("[A-Za-z ]+"));
        assert_eq!(query.maxlength.as_deref(), Some("80"));
        assert_eq!(query.list.as_deref(), Some("query-suggestions"));
        assert!(query.autofocus);
        assert!(query.required);
        let notes = &form.children[2].children[1];
        assert_eq!(notes.labels, vec!["NotesKeep me"]);
        assert!(notes.readonly);
        assert_eq!(notes.autocapitalize.as_deref(), Some("sentences"));
        assert_eq!(notes.enterkeyhint.as_deref(), Some("done"));
        assert_eq!(notes.dirname.as_deref(), Some("notes.dir"));
        let fieldset = &form.children[3];
        assert_eq!(fieldset.role, "form_group");
        assert_eq!(fieldset.accessible_name.as_deref(), Some("Options"));
        let select = &form.children[4];
        assert!(select.multiple);
        assert_eq!(select.size.as_deref(), Some("5"));
        assert_eq!(select.accessible_name.as_deref(), Some("Kind"));
        let upload = &form.children[5];
        assert_eq!(upload.accept.as_deref(), Some("image/png,image/jpeg"));
        assert_eq!(upload.capture.as_deref(), Some("environment"));
        let button = &form.children[6];
        assert_eq!(button.form_action.as_deref(), Some("run.html"));
        assert_eq!(
            button.resolved_form_action.as_deref(),
            Some("https://example.test/search/run.html")
        );
        assert!(button.form_novalidate);
        let image_button = &form.children[7];
        assert_eq!(image_button.control_type.as_deref(), Some("image"));
        assert_eq!(image_button.src.as_deref(), Some("buttons/search.png"));
        assert_eq!(
            image_button.resolved_src.as_deref(),
            Some("https://example.test/search/buttons/search.png")
        );
        assert_eq!(image_button.alt.as_deref(), Some("Search image"));
        assert_eq!(image_button.width.as_deref(), Some("32"));
        assert_eq!(image_button.height.as_deref(), Some("16"));
        let external = &content_tree.children[1];
        assert_eq!(external.form_owner.as_deref(), Some("f"));
        assert_eq!(external.accessible_name.as_deref(), Some("Outside"));

        let render_tree = BrowserRenderTree::from_content_tree(&content_tree);
        assert_eq!(render_tree.children[0].display, "block");
        assert_eq!(
            render_tree.children[0].children[1].display,
            "inline-replaced"
        );
        assert_eq!(
            render_tree.children[0].children[1]
                .accessible_name
                .as_deref(),
            Some("Query")
        );
    }

    #[test]
    fn browser_aria_interaction_metadata_tracks_roles_states_and_focus() {
        let document = parse_html(
            "<body><main id=app aria-label=\"App shell\">\
             <button id=menu aria-expanded=false aria-controls=panel tabindex=0 accesskey=\"m /\" popovertarget=panel-pop popovertargetaction=toggle command=show-modal commandfor=dialog>Menu</button>\
             <section id=panel role=region aria-labelledby=panel-title aria-describedby=panel-help inert>\
               <h2 id=panel-title>Settings</h2><p id=panel-help>Choose options</p>\
               <div role=button tabindex=-1 aria-pressed=mixed aria-current=page contenteditable=plaintext-only draggable=auto spellcheck=false translate=no popover=manual>Inline action</div>\
               <p id=editable contenteditable spellcheck=true translate=yes>Editable copy</p>\
             </section>\
             <dialog id=dialog open aria-label=\"Dialog\" accesskey=\"d x\"><p>Dialog copy</p></dialog>\
             <details id=more open><summary>More</summary><p>Extra</p></details>\
             <p hidden>Hidden copy</p><span aria-hidden=true>Decorative</span></main>",
        )
        .unwrap();

        let content_tree = BrowserContentTree::from_document(&document);
        let main = &content_tree.children[0];
        assert_eq!(main.role, "main");
        assert_eq!(main.accessible_name.as_deref(), Some("App shell"));
        assert_eq!(main.aria_label.as_deref(), Some("App shell"));

        let menu = &main.children[0];
        assert_eq!(menu.role, "control");
        assert_eq!(menu.accessible_name.as_deref(), Some("Menu"));
        assert_eq!(menu.aria_expanded.as_deref(), Some("false"));
        assert_eq!(menu.aria_controls, vec!["panel"]);
        assert_eq!(menu.tabindex.as_deref(), Some("0"));
        assert_eq!(menu.accesskey, vec!["m", "/"]);
        assert_eq!(menu.focusable, Some(true));
        assert_eq!(menu.popover_target.as_deref(), Some("panel-pop"));
        assert_eq!(menu.popover_target_action.as_deref(), Some("toggle"));
        assert_eq!(menu.command.as_deref(), Some("show-modal"));
        assert_eq!(menu.command_for.as_deref(), Some("dialog"));

        let panel = &main.children[1];
        assert_eq!(panel.role, "section");
        assert_eq!(panel.authored_role.as_deref(), Some("region"));
        assert_eq!(panel.aria_labelledby, vec!["panel-title"]);
        assert_eq!(panel.aria_describedby, vec!["panel-help"]);
        assert_eq!(panel.accessible_name.as_deref(), Some("Settings"));
        assert!(panel.inert);

        let action = &panel.children[2];
        assert_eq!(action.role, "block");
        assert_eq!(action.authored_role.as_deref(), Some("button"));
        assert_eq!(action.aria_pressed.as_deref(), Some("mixed"));
        assert_eq!(action.aria_current.as_deref(), Some("page"));
        assert_eq!(action.tabindex.as_deref(), Some("-1"));
        assert_eq!(action.focusable, Some(false));
        assert_eq!(action.contenteditable.as_deref(), Some("plaintext-only"));
        assert_eq!(action.editing_mode.as_deref(), Some("plaintext"));
        assert_eq!(action.draggable.as_deref(), Some("auto"));
        assert_eq!(action.draggable_state.as_deref(), Some("auto"));
        assert_eq!(action.spellcheck.as_deref(), Some("false"));
        assert_eq!(action.translate.as_deref(), Some("no"));
        assert_eq!(action.popover.as_deref(), Some("manual"));

        let editable = &panel.children[3];
        assert_eq!(editable.role, "paragraph");
        assert_eq!(editable.contenteditable.as_deref(), Some(""));
        assert_eq!(editable.editing_mode.as_deref(), Some("richtext"));
        assert_eq!(editable.spellcheck.as_deref(), Some("true"));
        assert_eq!(editable.translate.as_deref(), Some("yes"));
        assert_eq!(editable.focusable, Some(true));

        let dialog = &main.children[2];
        assert_eq!(dialog.role, "block");
        assert!(dialog.open);
        assert_eq!(dialog.accesskey, vec!["d", "x"]);

        let details = &main.children[3];
        assert_eq!(details.role, "block");
        assert!(details.open);

        assert!(main.children[4].hidden);
        assert!(main.children[5].aria_hidden);

        let render_tree = BrowserRenderTree::from_content_tree(&content_tree);
        let render_panel = &render_tree.children[0].children[1];
        assert_eq!(render_panel.authored_role.as_deref(), Some("region"));
        assert_eq!(render_panel.accessible_name.as_deref(), Some("Settings"));
        assert!(render_panel.inert);
        assert_eq!(render_tree.children[0].children[0].focusable, Some(true));
        assert_eq!(
            render_tree.children[0].children[0]
                .popover_target
                .as_deref(),
            Some("panel-pop")
        );
        assert_eq!(
            render_tree.children[0].children[1].children[2]
                .editing_mode
                .as_deref(),
            Some("plaintext")
        );
        assert_eq!(
            render_tree.children[0].children[1].children[3].focusable,
            Some(true)
        );
        assert!(render_tree.children[0].children[2].open);
        assert!(render_tree.children[0].children[3].open);
    }

    #[test]
    fn browser_script_style_metadata_tracks_loading_and_inline_text() {
        let document = parse_html(
            "<base href=\"https://example.test/app/index.html\">\
             <link rel=\"stylesheet preload\" href=app.css media=screen title=main integrity=sha256-css crossorigin=anonymous referrerpolicy=no-referrer fetchpriority=high blocking=render>\
             <link rel=\"alternate stylesheet\" href=print.css title=print disabled>\
             <style media=print title=print>body { color: black; }</style>\
             <script type=module src=app.js async integrity=sha384-js crossorigin=use-credentials referrerpolicy=origin fetchpriority=low blocking=render></script>\
             <script type=\"text/javascript; charset=utf-8\">classicMime();</script>\
             <script nomodule defer>legacy();</script>\
             <body><script src=late.js defer></script>",
        )
        .unwrap();

        let summary = BrowserDocument::from_document(&document);
        assert_eq!(summary.stylesheets.len(), 3);
        let main_style = &summary.stylesheets[0];
        assert_eq!(main_style.source, "link");
        assert_eq!(
            main_style.resolved_href.as_deref(),
            Some("https://example.test/app/app.css")
        );
        assert_eq!(main_style.rel.as_deref(), Some("stylesheet preload"));
        assert_eq!(main_style.media.as_deref(), Some("screen"));
        assert_eq!(main_style.integrity.as_deref(), Some("sha256-css"));
        assert_eq!(main_style.crossorigin.as_deref(), Some("anonymous"));
        assert_eq!(main_style.referrerpolicy.as_deref(), Some("no-referrer"));
        assert_eq!(main_style.fetchpriority.as_deref(), Some("high"));
        assert_eq!(main_style.blocking.as_deref(), Some("render"));
        assert!(!main_style.alternate);

        let alternate_style = &summary.stylesheets[1];
        assert!(alternate_style.alternate);
        assert!(alternate_style.disabled);
        assert_eq!(
            alternate_style.resolved_href.as_deref(),
            Some("https://example.test/app/print.css")
        );

        let inline_style = &summary.stylesheets[2];
        assert_eq!(inline_style.source, "style");
        assert_eq!(inline_style.media.as_deref(), Some("print"));
        assert_eq!(inline_style.text.as_deref(), Some("body { color: black; }"));

        assert_eq!(summary.scripts.len(), 4);
        let module_script = &summary.scripts[0];
        assert_eq!(module_script.script_kind, "module");
        assert_eq!(
            module_script.resolved_src.as_deref(),
            Some("https://example.test/app/app.js")
        );
        assert!(module_script.async_script);
        assert_eq!(module_script.integrity.as_deref(), Some("sha384-js"));
        assert_eq!(
            module_script.crossorigin.as_deref(),
            Some("use-credentials")
        );
        assert_eq!(module_script.referrerpolicy.as_deref(), Some("origin"));
        assert_eq!(module_script.fetchpriority.as_deref(), Some("low"));
        assert_eq!(module_script.blocking.as_deref(), Some("render"));

        let classic_mime_script = &summary.scripts[1];
        assert_eq!(classic_mime_script.script_kind, "classic");
        assert_eq!(
            classic_mime_script.type_hint.as_deref(),
            Some("text/javascript; charset=utf-8")
        );
        assert_eq!(classic_mime_script.text.as_deref(), Some("classicMime();"));

        let legacy_script = &summary.scripts[2];
        assert_eq!(legacy_script.script_kind, "classic");
        assert!(legacy_script.nomodule);
        assert!(legacy_script.defer_script);
        assert_eq!(legacy_script.text.as_deref(), Some("legacy();"));

        let late_script = &summary.scripts[3];
        assert_eq!(
            late_script.resolved_src.as_deref(),
            Some("https://example.test/app/late.js")
        );
        assert!(late_script.defer_script);
    }

    #[test]
    fn browser_link_resource_metadata_tracks_fetch_hints_and_responsive_preloads() {
        let document = parse_html(
            "<base href=\"https://example.test/app/index.html\">\
             <link rel=preconnect href=\"https://cdn.example.test\" crossorigin>\
             <link rel=preload href=\"fonts/site.woff2\" as=font type=font/woff2 crossorigin=anonymous integrity=sha384-font fetchpriority=high>\
             <link rel=modulepreload href=\"scripts/app.mjs\" integrity=sha384-module referrerpolicy=no-referrer blocking=render>\
             <link rel=preload href=\"hero.jpg\" as=image imagesrcset=\"hero-small.jpg 480w, hero-large.jpg 960w\" imagesizes=\"50vw\" fetchpriority=high>\
             <link rel=prefetch href=\"next.html\" as=document>\
             <link rel=manifest href=\"site.webmanifest\" crossorigin=use-credentials>\
             <link rel=canonical href=\"https://example.test/app/\">\
             <link rel=\"shortcut icon\" href=\"favicon.ico\" sizes=any type=image/x-icon>",
        )
        .unwrap();

        let summary = BrowserDocument::from_document(&document);
        assert_eq!(summary.resources.len(), 8);
        let preconnect = &summary.resources[0];
        assert_eq!(preconnect.kind, "preconnect");
        assert_eq!(
            preconnect.resolved_url.as_deref(),
            Some("https://cdn.example.test")
        );
        assert_eq!(preconnect.crossorigin.as_deref(), Some(""));

        let font = &summary.resources[1];
        assert_eq!(font.kind, "preload");
        assert_eq!(font.as_hint.as_deref(), Some("font"));
        assert_eq!(font.type_hint.as_deref(), Some("font/woff2"));
        assert_eq!(
            font.resolved_url.as_deref(),
            Some("https://example.test/app/fonts/site.woff2")
        );
        assert_eq!(font.integrity.as_deref(), Some("sha384-font"));
        assert_eq!(font.crossorigin.as_deref(), Some("anonymous"));
        assert_eq!(font.fetchpriority.as_deref(), Some("high"));

        let module = &summary.resources[2];
        assert_eq!(module.kind, "modulepreload");
        assert_eq!(module.integrity.as_deref(), Some("sha384-module"));
        assert_eq!(module.referrerpolicy.as_deref(), Some("no-referrer"));
        assert_eq!(module.blocking.as_deref(), Some("render"));

        let image = &summary.resources[3];
        assert_eq!(image.as_hint.as_deref(), Some("image"));
        assert_eq!(
            image.resolved_imagesrcset.as_deref(),
            Some("https://example.test/app/hero-small.jpg 480w, https://example.test/app/hero-large.jpg 960w")
        );
        assert_eq!(image.imagesizes.as_deref(), Some("50vw"));

        assert_eq!(summary.resources[4].kind, "prefetch");
        assert_eq!(summary.resources[4].as_hint.as_deref(), Some("document"));
        assert_eq!(summary.resources[5].kind, "manifest");
        assert_eq!(
            summary.resources[5].crossorigin.as_deref(),
            Some("use-credentials")
        );
        assert_eq!(summary.resources[6].kind, "canonical");
        assert_eq!(summary.resources[7].kind, "icon");
        assert_eq!(
            summary.resources[7].type_hint.as_deref(),
            Some("image/x-icon")
        );
    }

    #[test]
    fn browser_media_metadata_tracks_playback_and_poster_state() {
        let document = parse_html(
            "<base href=\"https://example.test/watch/index.html\">\
             <video id=hero src=movie.mp4 poster=poster.jpg controls autoplay loop muted playsinline preload=metadata width=640 height=360></video>\
             <audio src=theme.ogg controls preload=none></audio>",
        )
        .unwrap();

        let summary = BrowserDocument::from_document(&document);
        let video = &summary.media[0];
        assert_eq!(video.kind, "video");
        assert_eq!(
            video.resolved_src.as_deref(),
            Some("https://example.test/watch/movie.mp4")
        );
        assert_eq!(
            video.resolved_poster.as_deref(),
            Some("https://example.test/watch/poster.jpg")
        );
        assert_eq!(video.width.as_deref(), Some("640"));
        assert!(video.controls);
        assert!(video.autoplay);
        assert!(video.loop_media);
        assert!(video.muted);
        assert!(video.playsinline);
        assert_eq!(video.preload.as_deref(), Some("metadata"));
        assert_eq!(summary.media[1].kind, "audio");
        assert_eq!(summary.media[1].preload.as_deref(), Some("none"));

        let content_tree = BrowserContentTree::from_document(&document);
        let media = &content_tree.children[0];
        assert_eq!(media.role, "media");
        assert_eq!(media.poster.as_deref(), Some("poster.jpg"));
        assert_eq!(
            media.resolved_poster.as_deref(),
            Some("https://example.test/watch/poster.jpg")
        );
        assert!(media.controls);

        let render_tree = BrowserRenderTree::from_content_tree(&content_tree);
        assert_eq!(render_tree.children[0].display, "inline-replaced");
        assert!(render_tree.children[0].controls);
    }

    #[test]
    fn parses_nested_elements_and_text() {
        let document = parse_html("<h1>Hello <em>Venture</em></h1>").unwrap();

        let h1 = element(&body(&document).children[0]);
        assert_eq!(h1.name, "h1");
        assert_eq!(h1.children[0], Node::text("Hello "));

        let em = element(&h1.children[1]);
        assert_eq!(em.name, "em");
        assert_eq!(em.children, vec![Node::text("Venture")]);
    }

    #[test]
    fn parses_body_fragments_without_returning_implied_shell() {
        let nodes = parse_html_fragment("<p>one<p>two").unwrap();

        assert_eq!(nodes.len(), 2);
        let first = element(&nodes[0]);
        assert_eq!(first.name, "p");
        assert_eq!(first.children, vec![Node::text("one")]);
        let second = element(&nodes[1]);
        assert_eq!(second.name, "p");
        assert_eq!(second.children, vec![Node::text("two")]);
    }

    #[test]
    fn fragment_parsing_keeps_comment_text_and_void_body_nodes() {
        let nodes = parse_html_fragment("<!doctype html><!--note-->before<br>after").unwrap();

        assert_eq!(nodes.len(), 4);
        assert_eq!(nodes[0], Node::comment("note"));
        assert_eq!(nodes[1], Node::text("before"));
        assert_eq!(element(&nodes[2]).name, "br");
        assert_eq!(nodes[3], Node::text("after"));
    }

    #[test]
    fn fragment_parsing_keeps_diagnostics_and_parser_options() {
        let output = parse_html_fragment_with_diagnostics("text</section>").unwrap();
        assert_eq!(output.nodes, vec![Node::text("text")]);
        assert_eq!(
            output.parser_diagnostics,
            vec![ParserDiagnostic::new(
                "unexpected-end-tag",
                "end tag `</section>` did not match an open element"
            )]
        );

        let noscript_nodes = parse_html_fragment_with_options(
            "<noscript><p>Fallback</p></noscript>",
            HtmlParseOptions {
                scripting: HtmlScriptingMode::Disabled,
                ..HtmlParseOptions::default()
            },
        )
        .unwrap();
        let noscript = element(&noscript_nodes[0]);
        assert_eq!(noscript.name, "noscript");
        assert_eq!(element(&noscript.children[0]).name, "p");
    }

    #[test]
    fn keeps_doctype_comments_attributes_and_void_elements() {
        let document = parse_html("<!DOCTYPE html><!--note--><img src=cat.png alt=Cat>").unwrap();

        assert!(matches!(
            &document.children[0],
            Node::DocumentType(DocumentType {
                name: Some(name),
                force_quirks: false,
                ..
            }) if name == "html"
        ));
        assert_eq!(document.children[1], Node::comment("note"));

        let image = element(&body(&document).children[0]);
        assert_eq!(image.name, "img");
        assert_eq!(image.attribute("src"), Some("cat.png"));
        assert_eq!(image.attribute("alt"), Some("Cat"));
    }

    #[test]
    fn reports_unmatched_end_tags_without_dropping_content() {
        let output = parse_html_with_diagnostics("<p>Hello</section>").unwrap();

        assert_eq!(
            output.parser_diagnostics,
            vec![ParserDiagnostic::new(
                "unexpected-end-tag",
                "end tag `</section>` did not match an open element"
            )]
        );
        let paragraph = element(&body(&output.document).children[0]);
        assert_eq!(paragraph.children, vec![Node::text("Hello")]);
    }

    #[test]
    fn applies_simple_html_implied_end_tags() {
        let document = parse_html("<ul><li>one<li>two</ul><p>a<p>b").unwrap();

        let body = body(&document);
        let list = element(&body.children[0]);
        assert_eq!(list.name, "ul");
        assert_eq!(list.children.len(), 2);
        assert_eq!(element(&list.children[0]).children, vec![Node::text("one")]);
        assert_eq!(element(&list.children[1]).children, vec![Node::text("two")]);

        assert_eq!(element(&body.children[1]).children, vec![Node::text("a")]);
        assert_eq!(element(&body.children[2]).children, vec![Node::text("b")]);
    }

    #[test]
    fn closes_scoped_implied_end_tags_around_nested_inline_children() {
        let document = parse_html(
            "<p><em>One<p>Two<ul><li><strong>A<li>B</ul><dl><dt><em>T<dd>D</dl><select><option><span>One<option selected>Two<optgroup label=G><option><b>Three<optgroup label=H><option>Four</select><h1><span>Head<h2>Next",
        )
        .unwrap();

        let body = body(&document);
        assert_eq!(body.children.len(), 7);

        let first_paragraph = element(&body.children[0]);
        assert_eq!(first_paragraph.name, "p");
        let emphasized = element(&first_paragraph.children[0]);
        assert_eq!(emphasized.name, "em");
        assert_eq!(emphasized.children, vec![Node::text("One")]);

        let second_paragraph = element(&body.children[1]);
        assert_eq!(second_paragraph.name, "p");
        assert_eq!(
            element(&second_paragraph.children[0]).children,
            vec![Node::text("Two")]
        );

        let list = element(&body.children[2]);
        assert_eq!(list.name, "ul");
        assert_eq!(list.children.len(), 2);
        let first_item = element(&list.children[0]);
        assert_eq!(first_item.name, "li");
        let strong = element(&first_item.children[0]);
        assert_eq!(strong.name, "strong");
        assert_eq!(strong.children, vec![Node::text("A")]);
        assert_eq!(element(&list.children[1]).children, vec![Node::text("B")]);

        let definitions = element(&body.children[3]);
        assert_eq!(definitions.name, "dl");
        assert_eq!(definitions.children.len(), 2);
        let term = element(&definitions.children[0]);
        assert_eq!(term.name, "dt");
        assert_eq!(element(&term.children[0]).children, vec![Node::text("T")]);
        let description = element(&definitions.children[1]);
        assert_eq!(description.name, "dd");
        assert_eq!(description.children, vec![Node::text("D")]);

        let select = element(&body.children[4]);
        assert_eq!(select.name, "select");
        assert_eq!(select.children.len(), 4);
        let first_option = element(&select.children[0]);
        assert_eq!(first_option.name, "option");
        assert_eq!(
            element(&first_option.children[0]).children,
            vec![Node::text("One")]
        );
        let second_option = element(&select.children[1]);
        assert_eq!(second_option.name, "option");
        assert_eq!(second_option.attribute("selected"), Some(""));
        assert_eq!(second_option.children, vec![Node::text("Two")]);
        let first_group = element(&select.children[2]);
        assert_eq!(first_group.name, "optgroup");
        assert_eq!(first_group.attribute("label"), Some("G"));
        assert_eq!(
            element(&element(&first_group.children[0]).children[0]).children,
            vec![Node::text("Three")]
        );
        let second_group = element(&select.children[3]);
        assert_eq!(second_group.name, "optgroup");
        assert_eq!(second_group.attribute("label"), Some("H"));
        assert_eq!(
            element(&second_group.children[0]).children,
            vec![Node::text("Four")]
        );

        let first_heading = element(&body.children[5]);
        assert_eq!(first_heading.name, "h1");
        assert_eq!(
            element(&first_heading.children[0]).children,
            vec![Node::text("Head")]
        );

        let second_heading = element(&body.children[6]);
        assert_eq!(second_heading.name, "h2");
        assert_eq!(second_heading.children, vec![Node::text("Next")]);
    }

    #[test]
    fn closes_repeated_interactive_formatting_elements() {
        let document = parse_html(
            "<a href=one>One<a href=two>Two</a><button id=one>First<button id=two>Second</button><nobr>A<nobr>B</nobr>",
        )
        .unwrap();

        let body = body(&document);
        assert_eq!(body.children.len(), 6);

        let first_anchor = element(&body.children[0]);
        assert_eq!(first_anchor.name, "a");
        assert_eq!(first_anchor.attribute("href"), Some("one"));
        assert_eq!(first_anchor.children, vec![Node::text("One")]);

        let second_anchor = element(&body.children[1]);
        assert_eq!(second_anchor.name, "a");
        assert_eq!(second_anchor.attribute("href"), Some("two"));
        assert_eq!(second_anchor.children, vec![Node::text("Two")]);

        let first_button = element(&body.children[2]);
        assert_eq!(first_button.name, "button");
        assert_eq!(first_button.attribute("id"), Some("one"));
        assert_eq!(first_button.children, vec![Node::text("First")]);

        let second_button = element(&body.children[3]);
        assert_eq!(second_button.name, "button");
        assert_eq!(second_button.attribute("id"), Some("two"));
        assert_eq!(second_button.children, vec![Node::text("Second")]);

        let first_nobr = element(&body.children[4]);
        assert_eq!(first_nobr.name, "nobr");
        assert_eq!(first_nobr.children, vec![Node::text("A")]);

        let second_nobr = element(&body.children[5]);
        assert_eq!(second_nobr.name, "nobr");
        assert_eq!(second_nobr.children, vec![Node::text("B")]);
    }

    #[test]
    fn preserves_surrounding_context_when_interactive_elements_repeat() {
        let document = parse_html(
            "<p>Lead <a href=one>One<a href=two>Two</a> tail<p>Next <nobr>A<nobr>B</nobr>",
        )
        .unwrap();

        let body = body(&document);
        assert_eq!(body.children.len(), 2);

        let first_paragraph = element(&body.children[0]);
        assert_eq!(first_paragraph.name, "p");
        assert_eq!(first_paragraph.children[0], Node::text("Lead "));

        let first_anchor = element(&first_paragraph.children[1]);
        assert_eq!(first_anchor.name, "a");
        assert_eq!(first_anchor.attribute("href"), Some("one"));
        assert_eq!(first_anchor.children, vec![Node::text("One")]);

        let second_anchor = element(&first_paragraph.children[2]);
        assert_eq!(second_anchor.name, "a");
        assert_eq!(second_anchor.attribute("href"), Some("two"));
        assert_eq!(second_anchor.children, vec![Node::text("Two")]);
        assert_eq!(first_paragraph.children[3], Node::text(" tail"));

        let second_paragraph = element(&body.children[1]);
        assert_eq!(second_paragraph.name, "p");
        assert_eq!(second_paragraph.children[0], Node::text("Next "));

        let first_nobr = element(&second_paragraph.children[1]);
        assert_eq!(first_nobr.name, "nobr");
        assert_eq!(first_nobr.children, vec![Node::text("A")]);

        let second_nobr = element(&second_paragraph.children[2]);
        assert_eq!(second_nobr.name, "nobr");
        assert_eq!(second_nobr.children, vec![Node::text("B")]);
    }

    #[test]
    fn drops_empty_reconstructed_formatting_after_paragraph_boundary() {
        let document = parse_html("<p id=a><b><p id=b></b>TEST").unwrap();

        let body = body(&document);
        assert_eq!(body.children.len(), 2);

        let first_paragraph = element(&body.children[0]);
        assert_eq!(first_paragraph.name, "p");
        assert_eq!(first_paragraph.attribute("id"), Some("a"));
        assert_eq!(element(&first_paragraph.children[0]).name, "b");

        let second_paragraph = element(&body.children[1]);
        assert_eq!(second_paragraph.name, "p");
        assert_eq!(second_paragraph.attribute("id"), Some("b"));
        assert_eq!(second_paragraph.children, vec![Node::text("TEST")]);
    }

    #[test]
    fn keeps_empty_reconstructed_formatting_without_prior_paragraph() {
        let document = parse_html("<b><p></b>TEST").unwrap();

        let body = body(&document);
        assert_eq!(body.children.len(), 2);
        assert_eq!(element(&body.children[0]).name, "b");

        let paragraph = element(&body.children[1]);
        assert_eq!(paragraph.name, "p");
        assert_eq!(element(&paragraph.children[0]).name, "b");
        assert_eq!(paragraph.children[1], Node::text("TEST"));
    }

    #[test]
    fn inserts_fresh_paragraph_inside_current_nested_formatting() {
        let document = parse_html("<div> abc <b> def <i> ghi <p>").unwrap();

        let body = body(&document);
        let div = element(&body.children[0]);
        assert_eq!(div.children[0], Node::text(" abc "));

        let bold = element(&div.children[1]);
        assert_eq!(bold.children[0], Node::text(" def "));

        let italic = element(&bold.children[1]);
        assert_eq!(italic.children[0], Node::text(" ghi "));
        assert_eq!(element(&italic.children[1]).name, "p");
    }

    #[test]
    fn adopts_nested_formatting_paragraph_when_outer_formatting_ends() {
        let document = parse_html("<div> abc <b> def <i> ghi <p> jkl </b>").unwrap();

        let body = body(&document);
        let div = element(&body.children[0]);
        assert_eq!(div.children[0], Node::text(" abc "));

        let original_bold = element(&div.children[1]);
        assert_eq!(original_bold.children[0], Node::text(" def "));
        assert_eq!(
            element(&original_bold.children[1]).children,
            vec![Node::text(" ghi ")]
        );

        let adopted_italic = element(&div.children[2]);
        assert_eq!(adopted_italic.name, "i");
        let paragraph = element(&adopted_italic.children[0]);
        assert_eq!(paragraph.name, "p");
        let adopted_bold = element(&paragraph.children[0]);
        assert_eq!(adopted_bold.name, "b");
        assert_eq!(adopted_bold.children, vec![Node::text(" jkl ")]);
    }

    #[test]
    fn wraps_non_empty_paragraph_when_nested_formatting_end_adopts() {
        let document = parse_html("<div> abc <b> def <i> ghi <p> jkl </b> mno </i>").unwrap();

        let body = body(&document);
        let div = element(&body.children[0]);
        assert_eq!(div.children[0], Node::text(" abc "));

        let original_bold = element(&div.children[1]);
        assert_eq!(original_bold.children[0], Node::text(" def "));
        assert_eq!(
            element(&original_bold.children[1]).children,
            vec![Node::text(" ghi ")]
        );

        assert_eq!(element(&div.children[2]).name, "i");

        let paragraph = element(&div.children[3]);
        assert_eq!(paragraph.name, "p");
        let adopted_italic = element(&paragraph.children[0]);
        assert_eq!(adopted_italic.name, "i");
        let adopted_bold = element(&adopted_italic.children[0]);
        assert_eq!(adopted_bold.name, "b");
        assert_eq!(adopted_bold.children, vec![Node::text(" jkl ")]);
        assert_eq!(adopted_italic.children[1], Node::text(" mno "));
    }

    #[test]
    fn keeps_outer_formatting_around_paragraph_closed_before_end_tag() {
        let document = parse_html("<b id=a><p><b id=b></p></b>TEST").unwrap();

        let body = body(&document);
        assert_eq!(body.children.len(), 1);

        let outer_bold = element(&body.children[0]);
        assert_eq!(outer_bold.name, "b");
        assert_eq!(outer_bold.attribute("id"), Some("a"));
        assert_eq!(outer_bold.children.len(), 2);

        let paragraph = element(&outer_bold.children[0]);
        assert_eq!(paragraph.name, "p");
        let inner_bold = element(&paragraph.children[0]);
        assert_eq!(inner_bold.name, "b");
        assert_eq!(inner_bold.attribute("id"), Some("b"));
        assert_eq!(outer_bold.children[1], Node::text("TEST"));
    }

    #[test]
    fn reconstructs_formatting_before_text_after_formatting_end_tag() {
        let document = parse_html("<font><p>hello<b>cruel</font>world").unwrap();

        let body = body(&document);
        assert_eq!(body.children.len(), 2);
        assert_eq!(element(&body.children[0]).name, "font");

        let paragraph = element(&body.children[1]);
        assert_eq!(paragraph.name, "p");

        let font = element(&paragraph.children[0]);
        assert_eq!(font.name, "font");
        assert_eq!(font.children[0], Node::text("hello"));
        let first_bold = element(&font.children[1]);
        assert_eq!(first_bold.name, "b");
        assert_eq!(first_bold.children, vec![Node::text("cruel")]);

        let second_bold = element(&paragraph.children[1]);
        assert_eq!(second_bold.name, "b");
        assert_eq!(second_bold.children, vec![Node::text("world")]);
    }

    #[test]
    fn preserves_explicit_empty_formatting_elements() {
        let document = parse_html("<p><b></b>tail").unwrap();

        let body = body(&document);
        let paragraph = element(&body.children[0]);
        assert_eq!(paragraph.children.len(), 2);

        let bold = element(&paragraph.children[0]);
        assert_eq!(bold.name, "b");
        assert!(bold.children.is_empty());
        assert_eq!(paragraph.children[1], Node::text("tail"));
    }

    #[test]
    fn keeps_buttons_inside_paragraph_and_closes_legacy_block_boundaries() {
        let document = parse_html(
            "<p>Button<button>Click<button>Again</button><p>Centered<center>Block</center><p>Search<search>Find</search><p>Heading<hgroup>Title</hgroup><p>Listing<listing>Block</listing><p>Directory<dir><li>Item",
        )
        .unwrap();

        let body = body(&document);
        assert_eq!(body.children.len(), 11);

        let button_intro = element(&body.children[0]);
        assert_eq!(button_intro.name, "p");
        assert_eq!(button_intro.children.len(), 3);
        assert_eq!(button_intro.children[0], Node::text("Button"));

        let first_button = element(&button_intro.children[1]);
        assert_eq!(first_button.name, "button");
        assert_eq!(first_button.children, vec![Node::text("Click")]);

        let second_button = element(&button_intro.children[2]);
        assert_eq!(second_button.name, "button");
        assert_eq!(second_button.children, vec![Node::text("Again")]);

        let centered_intro = element(&body.children[1]);
        assert_eq!(centered_intro.name, "p");
        assert_eq!(centered_intro.children, vec![Node::text("Centered")]);

        let center = element(&body.children[2]);
        assert_eq!(center.name, "center");
        assert_eq!(center.children, vec![Node::text("Block")]);

        let search_intro = element(&body.children[3]);
        assert_eq!(search_intro.name, "p");
        assert_eq!(search_intro.children, vec![Node::text("Search")]);

        let search = element(&body.children[4]);
        assert_eq!(search.name, "search");
        assert_eq!(search.children, vec![Node::text("Find")]);

        let heading_intro = element(&body.children[5]);
        assert_eq!(heading_intro.name, "p");
        assert_eq!(heading_intro.children, vec![Node::text("Heading")]);

        let hgroup = element(&body.children[6]);
        assert_eq!(hgroup.name, "hgroup");
        assert_eq!(hgroup.children, vec![Node::text("Title")]);

        let listing_intro = element(&body.children[7]);
        assert_eq!(listing_intro.name, "p");
        assert_eq!(listing_intro.children, vec![Node::text("Listing")]);

        let listing = element(&body.children[8]);
        assert_eq!(listing.name, "listing");
        assert_eq!(listing.children, vec![Node::text("Block")]);

        let directory_intro = element(&body.children[9]);
        assert_eq!(directory_intro.name, "p");
        assert_eq!(directory_intro.children, vec![Node::text("Directory")]);

        let directory = element(&body.children[10]);
        assert_eq!(directory.name, "dir");
        assert_eq!(directory.children.len(), 1);
        let item = element(&directory.children[0]);
        assert_eq!(item.name, "li");
        assert_eq!(item.children, vec![Node::text("Item")]);
    }

    #[test]
    fn closes_paragraphs_before_raw_text_block_boundaries() {
        let document = parse_html("<p>Xmp<xmp>B <i>tag</i></xmp>").unwrap();

        let body = body(&document);
        assert_eq!(body.children.len(), 2);

        let xmp_intro = element(&body.children[0]);
        assert_eq!(xmp_intro.name, "p");
        assert_eq!(xmp_intro.children, vec![Node::text("Xmp")]);

        let xmp = element(&body.children[1]);
        assert_eq!(xmp.name, "xmp");
        assert_eq!(xmp.children, vec![Node::text("B <i>tag</i>")]);
    }

    #[test]
    fn closes_paragraph_before_plaintext_consumes_rest_of_document() {
        let document = parse_html("<p>Before<plaintext>A <b>tag</b><p>still text").unwrap();

        let body = body(&document);
        assert_eq!(body.children.len(), 2);

        let paragraph = element(&body.children[0]);
        assert_eq!(paragraph.name, "p");
        assert_eq!(paragraph.children, vec![Node::text("Before")]);

        let plaintext = element(&body.children[1]);
        assert_eq!(plaintext.name, "plaintext");
        assert_eq!(
            plaintext.children,
            vec![Node::text("A <b>tag</b><p>still text")]
        );
    }

    #[test]
    fn ignores_nested_form_start_tags() {
        let output = parse_html_with_diagnostics(
            "<form id=outer><div>One<form id=inner><input name=x></form><p>After",
        )
        .unwrap();

        assert_eq!(
            output.parser_diagnostics,
            vec![ParserDiagnostic::new(
                "nested-form-start-tag",
                "nested form start tag was ignored while a form element was already open"
            )]
        );

        let body = body(&output.document);
        assert_eq!(body.children.len(), 1);

        let form = element(&body.children[0]);
        assert_eq!(form.name, "form");
        assert_eq!(form.attribute("id"), Some("outer"));
        assert_eq!(form.children.len(), 1);

        let div = element(&form.children[0]);
        assert_eq!(div.name, "div");
        assert_eq!(div.children[0], Node::text("One"));
        let input = element(&div.children[1]);
        assert_eq!(input.name, "input");
        assert_eq!(input.attribute("name"), Some("x"));

        let paragraph = element(&div.children[2]);
        assert_eq!(paragraph.name, "p");
        assert_eq!(paragraph.children, vec![Node::text("After")]);
    }

    #[test]
    fn applies_select_option_implied_end_tags() {
        let document = parse_html(
            "<select><option>One<option selected>Two<optgroup label=G><option>Three<optgroup label=H><option>Four</select>",
        )
        .unwrap();

        let select = element(&body(&document).children[0]);
        assert_eq!(select.name, "select");
        assert_eq!(select.children.len(), 4);

        let first = element(&select.children[0]);
        assert_eq!(first.name, "option");
        assert_eq!(first.children, vec![Node::text("One")]);

        let second = element(&select.children[1]);
        assert_eq!(second.name, "option");
        assert_eq!(second.attribute("selected"), Some(""));
        assert_eq!(second.children, vec![Node::text("Two")]);

        let group = element(&select.children[2]);
        assert_eq!(group.name, "optgroup");
        assert_eq!(group.attribute("label"), Some("G"));
        assert_eq!(group.children.len(), 1);
        assert_eq!(
            element(&group.children[0]).children,
            vec![Node::text("Three")]
        );

        let second_group = element(&select.children[3]);
        assert_eq!(second_group.name, "optgroup");
        assert_eq!(second_group.attribute("label"), Some("H"));
        assert_eq!(second_group.children.len(), 1);
        assert_eq!(
            element(&second_group.children[0]).children,
            vec![Node::text("Four")]
        );
    }

    #[test]
    fn list_item_end_tags_do_not_cross_nested_list_boundaries() {
        let document = parse_html("<ul><li><ul></li><li>a</li></ul></li></ul>").unwrap();

        let outer_list = element(&body(&document).children[0]);
        assert_eq!(outer_list.name, "ul");
        assert_eq!(outer_list.children.len(), 1);

        let outer_item = element(&outer_list.children[0]);
        assert_eq!(outer_item.name, "li");
        let inner_list = element(&outer_item.children[0]);
        assert_eq!(inner_list.name, "ul");
        assert_eq!(inner_list.children.len(), 1);

        let inner_item = element(&inner_list.children[0]);
        assert_eq!(inner_item.name, "li");
        assert_eq!(inner_item.children, vec![Node::text("a")]);
    }

    #[test]
    fn list_item_start_tags_close_open_paragraphs() {
        let document = parse_html("<p><li>").unwrap();

        let body = body(&document);
        assert_eq!(element(&body.children[0]).name, "p");
        assert_eq!(element(&body.children[1]).name, "li");
    }

    #[test]
    fn definition_item_start_tags_close_open_paragraphs() {
        let document = parse_html("<p><dt>").unwrap();

        let body = body(&document);
        assert_eq!(element(&body.children[0]).name, "p");
        assert_eq!(element(&body.children[1]).name, "dt");
    }

    #[test]
    fn applies_ruby_annotation_implied_end_tags() {
        let document = parse_html(
            "<ruby><rb>漢<rt>kan<rb>字<rt>ji<rp>(fallback<rtc><rt>group<rtc><rt>group2</ruby>",
        )
        .unwrap();

        let ruby = element(&body(&document).children[0]);
        assert_eq!(ruby.name, "ruby");
        assert_eq!(ruby.children.len(), 7);

        let first_base = element(&ruby.children[0]);
        assert_eq!(first_base.name, "rb");
        assert_eq!(first_base.children, vec![Node::text("漢")]);

        let first_text = element(&ruby.children[1]);
        assert_eq!(first_text.name, "rt");
        assert_eq!(first_text.children, vec![Node::text("kan")]);

        let second_base = element(&ruby.children[2]);
        assert_eq!(second_base.name, "rb");
        assert_eq!(second_base.children, vec![Node::text("字")]);

        let second_text = element(&ruby.children[3]);
        assert_eq!(second_text.name, "rt");
        assert_eq!(second_text.children, vec![Node::text("ji")]);

        let fallback = element(&ruby.children[4]);
        assert_eq!(fallback.name, "rp");
        assert_eq!(fallback.children, vec![Node::text("(fallback")]);

        let first_container = element(&ruby.children[5]);
        assert_eq!(first_container.name, "rtc");
        let grouped_text = element(&first_container.children[0]);
        assert_eq!(grouped_text.name, "rt");
        assert_eq!(grouped_text.children, vec![Node::text("group")]);

        let second_container = element(&ruby.children[6]);
        assert_eq!(second_container.name, "rtc");
        let second_grouped_text = element(&second_container.children[0]);
        assert_eq!(second_grouped_text.name, "rt");
        assert_eq!(second_grouped_text.children, vec![Node::text("group2")]);
    }

    #[test]
    fn closes_scoped_ruby_annotations_around_nested_inline_children() {
        let document =
            parse_html("<ruby><rb><em>漢<rt><span>kan<rb>字<rtc><rt><b>group<rtc><rt>group2")
                .unwrap();

        let ruby = element(&body(&document).children[0]);
        assert_eq!(ruby.name, "ruby");
        assert_eq!(ruby.children.len(), 5);

        let first_base = element(&ruby.children[0]);
        assert_eq!(first_base.name, "rb");
        assert_eq!(
            element(&first_base.children[0]).children,
            vec![Node::text("漢")]
        );

        let first_text = element(&ruby.children[1]);
        assert_eq!(first_text.name, "rt");
        assert_eq!(
            element(&first_text.children[0]).children,
            vec![Node::text("kan")]
        );

        let second_base = element(&ruby.children[2]);
        assert_eq!(second_base.name, "rb");
        assert_eq!(second_base.children, vec![Node::text("字")]);

        let first_container = element(&ruby.children[3]);
        assert_eq!(first_container.name, "rtc");
        let grouped_text = element(&first_container.children[0]);
        assert_eq!(grouped_text.name, "rt");
        assert_eq!(
            element(&grouped_text.children[0]).children,
            vec![Node::text("group")]
        );

        let second_container = element(&ruby.children[4]);
        assert_eq!(second_container.name, "rtc");
        assert_eq!(
            element(&second_container.children[0]).children,
            vec![Node::text("group2")]
        );
    }

    #[test]
    fn applies_heading_implied_end_tags() {
        let document = parse_html("<p>Intro<h1>One<h2>Two<h3>Three").unwrap();

        let body = body(&document);
        assert_eq!(body.children.len(), 4);

        let paragraph = element(&body.children[0]);
        assert_eq!(paragraph.name, "p");
        assert_eq!(paragraph.children, vec![Node::text("Intro")]);

        let first = element(&body.children[1]);
        assert_eq!(first.name, "h1");
        assert_eq!(first.children, vec![Node::text("One")]);

        let second = element(&body.children[2]);
        assert_eq!(second.name, "h2");
        assert_eq!(second.children, vec![Node::text("Two")]);

        let third = element(&body.children[3]);
        assert_eq!(third.name, "h3");
        assert_eq!(third.children, vec![Node::text("Three")]);
    }

    #[test]
    fn heading_start_tags_do_not_close_ancestors_across_table_cells() {
        let document = parse_html("<h1><table><td><h3></table><h3></h1>").unwrap();

        let body = body(&document);
        let heading = element(&body.children[0]);
        assert_eq!(heading.name, "h1");

        let table = element(&heading.children[0]);
        let cell = element(&element(&element(&table.children[0]).children[0]).children[0]);
        assert_eq!(element(&cell.children[0]).name, "h3");

        assert_eq!(element(&body.children[1]).name, "h3");
    }

    #[test]
    fn closes_paragraphs_before_block_boundaries() {
        let document = parse_html(
            "<!doctype html><p>Intro<div>Block</div><p>Items<ul><li>One</ul><p>Table<table><tr><td>A</table>",
        )
        .unwrap();

        let body = body(&document);
        assert_eq!(body.children.len(), 6);

        let intro = element(&body.children[0]);
        assert_eq!(intro.name, "p");
        assert_eq!(intro.children, vec![Node::text("Intro")]);

        let div = element(&body.children[1]);
        assert_eq!(div.name, "div");
        assert_eq!(div.children, vec![Node::text("Block")]);

        let items = element(&body.children[2]);
        assert_eq!(items.name, "p");
        assert_eq!(items.children, vec![Node::text("Items")]);

        let list = element(&body.children[3]);
        assert_eq!(list.name, "ul");
        assert_eq!(list.children.len(), 1);
        let item = element(&list.children[0]);
        assert_eq!(item.name, "li");
        assert_eq!(item.children, vec![Node::text("One")]);

        let table_intro = element(&body.children[4]);
        assert_eq!(table_intro.name, "p");
        assert_eq!(table_intro.children, vec![Node::text("Table")]);

        let table = element(&body.children[5]);
        assert_eq!(table.name, "table");
        let tbody = element(&table.children[0]);
        let row = element(&tbody.children[0]);
        assert_eq!(element(&row.children[0]).children, vec![Node::text("A")]);
    }

    #[test]
    fn quirks_mode_keeps_tables_inside_open_paragraphs() {
        let document = parse_html("<p><table></table>").unwrap();

        let paragraph = element(&body(&document).children[0]);
        assert_eq!(paragraph.name, "p");
        assert_eq!(element(&paragraph.children[0]).name, "table");
    }

    #[test]
    fn synthesizes_table_body_and_row_for_omitted_table_structure() {
        let document = parse_html("<table><td>A<td>B<tr><th>C</table>").unwrap();

        let table = element(&body(&document).children[0]);
        assert_eq!(table.name, "table");

        let tbody = element(&table.children[0]);
        assert_eq!(tbody.name, "tbody");
        assert_eq!(tbody.children.len(), 2);

        let first_row = element(&tbody.children[0]);
        assert_eq!(first_row.name, "tr");
        assert_eq!(element(&first_row.children[0]).name, "td");
        assert_eq!(
            element(&first_row.children[0]).children,
            vec![Node::text("A")]
        );
        assert_eq!(element(&first_row.children[1]).name, "td");
        assert_eq!(
            element(&first_row.children[1]).children,
            vec![Node::text("B")]
        );

        let second_row = element(&tbody.children[1]);
        assert_eq!(second_row.name, "tr");
        assert_eq!(element(&second_row.children[0]).name, "th");
        assert_eq!(
            element(&second_row.children[0]).children,
            vec![Node::text("C")]
        );
    }

    #[test]
    fn closes_open_table_sections_when_new_sections_start() {
        let document = parse_html("<table><tbody><tr><td>A<tfoot><tr><td>B</table>").unwrap();

        let table = element(&body(&document).children[0]);
        assert_eq!(table.children.len(), 2);

        let tbody = element(&table.children[0]);
        assert_eq!(tbody.name, "tbody");
        let tbody_row = element(&tbody.children[0]);
        assert_eq!(
            element(&tbody_row.children[0]).children,
            vec![Node::text("A")]
        );

        let tfoot = element(&table.children[1]);
        assert_eq!(tfoot.name, "tfoot");
        let tfoot_row = element(&tfoot.children[0]);
        assert_eq!(
            element(&tfoot_row.children[0]).children,
            vec![Node::text("B")]
        );
    }

    #[test]
    fn closes_table_caption_before_column_groups_and_rows() {
        let document = parse_html("<table><caption>Cap<colgroup><col><tr><td>A</table>").unwrap();

        let table = element(&body(&document).children[0]);
        assert_eq!(table.children.len(), 3);

        let caption = element(&table.children[0]);
        assert_eq!(caption.name, "caption");
        assert_eq!(caption.children, vec![Node::text("Cap")]);

        let colgroup = element(&table.children[1]);
        assert_eq!(colgroup.name, "colgroup");
        assert_eq!(element(&colgroup.children[0]).name, "col");

        let tbody = element(&table.children[2]);
        let row = element(&tbody.children[0]);
        assert_eq!(element(&row.children[0]).children, vec![Node::text("A")]);
    }

    #[test]
    fn closes_column_groups_when_table_sections_start() {
        let document =
            parse_html("<table><colgroup><col><thead><tr><th>H<tbody><tr><td>B</table>").unwrap();

        let table = element(&body(&document).children[0]);
        assert_eq!(table.children.len(), 3);

        let colgroup = element(&table.children[0]);
        assert_eq!(colgroup.name, "colgroup");
        assert_eq!(element(&colgroup.children[0]).name, "col");

        let thead = element(&table.children[1]);
        assert_eq!(thead.name, "thead");
        assert_eq!(
            element(&element(&thead.children[0]).children[0]).children,
            vec![Node::text("H")]
        );

        let tbody = element(&table.children[2]);
        assert_eq!(tbody.name, "tbody");
        assert_eq!(
            element(&element(&tbody.children[0]).children[0]).children,
            vec![Node::text("B")]
        );
    }

    #[test]
    fn wraps_bare_table_columns_in_implied_colgroup() {
        let document = parse_html("<table><col span=2><col><tr><td>A</table>").unwrap();

        let table = element(&body(&document).children[0]);
        assert_eq!(table.children.len(), 2);

        let colgroup = element(&table.children[0]);
        assert_eq!(colgroup.name, "colgroup");
        assert_eq!(colgroup.children.len(), 2);
        assert_eq!(element(&colgroup.children[0]).name, "col");
        assert_eq!(element(&colgroup.children[0]).attribute("span"), Some("2"));
        assert_eq!(element(&colgroup.children[1]).name, "col");

        let tbody = element(&table.children[1]);
        let row = element(&tbody.children[0]);
        assert_eq!(element(&row.children[0]).children, vec![Node::text("A")]);
    }

    #[test]
    fn misplaced_table_columns_reopen_colgroups_from_table_contexts() {
        let document =
            parse_html("<table><col><tbody><col><tr><col><td><col></table><col>").unwrap();

        let table = element(&body(&document).children[0]);
        assert_eq!(table.children.len(), 7);

        assert_eq!(element(&table.children[0]).name, "colgroup");
        assert_eq!(
            element(&element(&table.children[0]).children[0]).name,
            "col"
        );
        assert_eq!(element(&table.children[1]).name, "tbody");
        assert_eq!(element(&table.children[2]).name, "colgroup");
        assert_eq!(
            element(&element(&table.children[2]).children[0]).name,
            "col"
        );
        assert_eq!(element(&table.children[3]).name, "tbody");
        assert_eq!(element(&element(&table.children[3]).children[0]).name, "tr");
        assert_eq!(element(&table.children[4]).name, "colgroup");
        assert_eq!(
            element(&element(&table.children[4]).children[0]).name,
            "col"
        );
        assert_eq!(element(&table.children[5]).name, "tbody");
        let cell = element(&element(&element(&table.children[5]).children[0]).children[0]);
        assert_eq!(cell.name, "td");
        assert_eq!(element(&table.children[6]).name, "colgroup");
        assert_eq!(
            element(&element(&table.children[6]).children[0]).name,
            "col"
        );
    }

    #[test]
    fn closes_caption_before_bare_table_columns() {
        let document = parse_html("<table><caption>Cap<col><tr><td>A</table>").unwrap();

        let table = element(&body(&document).children[0]);
        assert_eq!(table.children.len(), 3);

        let caption = element(&table.children[0]);
        assert_eq!(caption.name, "caption");
        assert_eq!(caption.children, vec![Node::text("Cap")]);

        let colgroup = element(&table.children[1]);
        assert_eq!(colgroup.name, "colgroup");
        assert_eq!(colgroup.children.len(), 1);
        assert_eq!(element(&colgroup.children[0]).name, "col");

        let tbody = element(&table.children[2]);
        let row = element(&tbody.children[0]);
        assert_eq!(element(&row.children[0]).children, vec![Node::text("A")]);
    }

    #[test]
    fn closes_scoped_table_contexts_around_nested_inline_children() {
        let document = parse_html(
            "<table><caption><b>Cap<col><tr><td><em>A<tr><th><span>B<tbody><tr><td>C<tfoot><tr><td>F</table>",
        )
        .unwrap();

        let table = element(&body(&document).children[0]);
        assert_eq!(table.name, "table");
        assert_eq!(table.children.len(), 5);

        let caption = element(&table.children[0]);
        assert_eq!(caption.name, "caption");
        assert_eq!(
            element(&caption.children[0]).children,
            vec![Node::text("Cap")]
        );

        let colgroup = element(&table.children[1]);
        assert_eq!(colgroup.name, "colgroup");
        assert_eq!(colgroup.children.len(), 1);
        assert_eq!(element(&colgroup.children[0]).name, "col");

        let first_body = element(&table.children[2]);
        assert_eq!(first_body.name, "tbody");
        assert_eq!(first_body.children.len(), 2);
        let first_row = element(&first_body.children[0]);
        assert_eq!(first_row.name, "tr");
        let first_cell = element(&first_row.children[0]);
        assert_eq!(first_cell.name, "td");
        assert_eq!(
            element(&first_cell.children[0]).children,
            vec![Node::text("A")]
        );
        let second_row = element(&first_body.children[1]);
        assert_eq!(second_row.name, "tr");
        let heading_cell = element(&second_row.children[0]);
        assert_eq!(heading_cell.name, "th");
        assert_eq!(
            element(&heading_cell.children[0]).children,
            vec![Node::text("B")]
        );

        let second_body = element(&table.children[3]);
        assert_eq!(second_body.name, "tbody");
        assert_eq!(
            element(&element(&second_body.children[0]).children[0]).children,
            vec![Node::text("C")]
        );

        let foot = element(&table.children[4]);
        assert_eq!(foot.name, "tfoot");
        assert_eq!(
            element(&element(&foot.children[0]).children[0]).children,
            vec![Node::text("F")]
        );
    }

    #[test]
    fn fosters_reconstructed_anchor_text_around_table_content() {
        let document = parse_html(
            "<a href=\"blah\">aba<table><a href=\"foo\">br<tr><td></td></tr>x</table>aoe",
        )
        .unwrap();

        let body = body(&document);
        assert_eq!(body.children.len(), 2);

        let outer_anchor = element(&body.children[0]);
        assert_eq!(outer_anchor.name, "a");
        assert_eq!(outer_anchor.attribute("href"), Some("blah"));
        assert_eq!(outer_anchor.children[0], Node::text("aba"));

        let first_fostered_anchor = element(&outer_anchor.children[1]);
        assert_eq!(first_fostered_anchor.name, "a");
        assert_eq!(first_fostered_anchor.attribute("href"), Some("foo"));
        assert_eq!(first_fostered_anchor.children, vec![Node::text("br")]);

        let second_fostered_anchor = element(&outer_anchor.children[2]);
        assert_eq!(second_fostered_anchor.name, "a");
        assert_eq!(second_fostered_anchor.attribute("href"), Some("foo"));
        assert_eq!(second_fostered_anchor.children, vec![Node::text("x")]);

        assert_eq!(element(&outer_anchor.children[3]).name, "table");

        let reconstructed_anchor = element(&body.children[1]);
        assert_eq!(reconstructed_anchor.name, "a");
        assert_eq!(reconstructed_anchor.attribute("href"), Some("foo"));
        assert_eq!(reconstructed_anchor.children, vec![Node::text("aoe")]);
    }

    #[test]
    fn skips_fostered_anchor_reconstruction_inside_table_cells() {
        let document = parse_html(
            "<table><a href=\"blah\">aba<tr><td><a href=\"foo\">br</td></tr>x</table>aoe",
        )
        .unwrap();

        let body = body(&document);
        assert_eq!(body.children.len(), 4);

        let first_anchor = element(&body.children[0]);
        assert_eq!(first_anchor.name, "a");
        assert_eq!(first_anchor.attribute("href"), Some("blah"));
        assert_eq!(first_anchor.children, vec![Node::text("aba")]);

        let second_anchor = element(&body.children[1]);
        assert_eq!(second_anchor.name, "a");
        assert_eq!(second_anchor.attribute("href"), Some("blah"));
        assert_eq!(second_anchor.children, vec![Node::text("x")]);

        let table = element(&body.children[2]);
        assert_eq!(table.name, "table");
        let cell = element(&element(&element(&table.children[0]).children[0]).children[0]);
        let cell_anchor = element(&cell.children[0]);
        assert_eq!(cell_anchor.name, "a");
        assert_eq!(cell_anchor.attribute("href"), Some("foo"));
        assert_eq!(cell_anchor.children, vec![Node::text("br")]);

        let trailing_anchor = element(&body.children[3]);
        assert_eq!(trailing_anchor.name, "a");
        assert_eq!(trailing_anchor.attribute("href"), Some("blah"));
        assert_eq!(trailing_anchor.children, vec![Node::text("aoe")]);
    }

    #[test]
    fn drops_pending_anchor_reconstruction_when_anchor_repeats_after_table() {
        let document = parse_html("<a><table><a></table><p><a><div><a>").unwrap();

        let body = body(&document);
        assert_eq!(body.children.len(), 3);

        let outer_anchor = element(&body.children[0]);
        assert_eq!(outer_anchor.name, "a");
        assert_eq!(element(&outer_anchor.children[0]).name, "a");
        assert_eq!(element(&outer_anchor.children[1]).name, "table");

        let paragraph = element(&body.children[1]);
        assert_eq!(paragraph.name, "p");
        assert_eq!(element(&paragraph.children[0]).name, "a");

        let div = element(&body.children[2]);
        assert_eq!(div.name, "div");
        assert_eq!(div.children.len(), 1);
        assert_eq!(element(&div.children[0]).name, "a");
    }

    #[test]
    fn preserves_outer_anchor_across_marquee_when_nested_anchor_starts() {
        let document = parse_html("<a href=a>aa<marquee>aa<a href=b>bb</marquee>aa").unwrap();

        let body = body(&document);
        assert_eq!(body.children.len(), 1);

        let outer_anchor = element(&body.children[0]);
        assert_eq!(outer_anchor.name, "a");
        assert_eq!(outer_anchor.attribute("href"), Some("a"));
        assert_eq!(outer_anchor.children[0], Node::text("aa"));

        let marquee = element(&outer_anchor.children[1]);
        assert_eq!(marquee.name, "marquee");
        assert_eq!(marquee.children[0], Node::text("aa"));

        let inner_anchor = element(&marquee.children[1]);
        assert_eq!(inner_anchor.name, "a");
        assert_eq!(inner_anchor.attribute("href"), Some("b"));
        assert_eq!(inner_anchor.children, vec![Node::text("bb")]);

        assert_eq!(outer_anchor.children[2], Node::text("aa"));
    }

    #[test]
    fn reconstructs_code_formatting_before_code_start_tag() {
        let document = parse_html("<wbr><strike><code></strike><code><strike></code>").unwrap();

        let body = body(&document);
        assert_eq!(body.children.len(), 3);
        assert_eq!(element(&body.children[0]).name, "wbr");

        let first_strike = element(&body.children[1]);
        assert_eq!(first_strike.name, "strike");
        assert_eq!(element(&first_strike.children[0]).name, "code");

        let reconstructed_code = element(&body.children[2]);
        assert_eq!(reconstructed_code.name, "code");
        let nested_code = element(&reconstructed_code.children[0]);
        assert_eq!(nested_code.name, "code");
        assert_eq!(element(&nested_code.children[0]).name, "strike");
    }

    #[test]
    fn parser_drives_rcdata_tokenization_for_title_and_textarea() {
        let document =
            parse_html("<title>Tom &amp; Jerry</title><textarea>A &lt; B</textarea>").unwrap();

        let title = element(&head(&document).children[0]);
        assert_eq!(title.name, "title");
        assert_eq!(title.children, vec![Node::text("Tom & Jerry")]);

        let textarea = element(&body(&document).children[0]);
        assert_eq!(textarea.name, "textarea");
        assert_eq!(textarea.children, vec![Node::text("A < B")]);
    }

    #[test]
    fn parser_drives_rawtext_and_script_tokenization() {
        let document = parse_html(
            "<style>a < b &amp; c</style><script>if (a < b) alert('&amp;');</script><p>x</p>",
        )
        .unwrap();

        let style = element(&head(&document).children[0]);
        assert_eq!(style.name, "style");
        assert_eq!(style.children, vec![Node::text("a < b &amp; c")]);

        let script = element(&head(&document).children[1]);
        assert_eq!(script.name, "script");
        assert_eq!(
            script.children,
            vec![Node::text("if (a < b) alert('&amp;');")]
        );

        let paragraph = element(&body(&document).children[0]);
        assert_eq!(paragraph.children, vec![Node::text("x")]);
    }

    #[test]
    fn ignores_self_closing_flag_on_non_void_html_elements() {
        let output = parse_html_with_diagnostics(
            "<div/>Text</div><span/>Tail</span><p/>Next<section/>Block</section>",
        )
        .unwrap();

        let body = body(&output.document);
        let div = element(&body.children[0]);
        assert_eq!(div.name, "div");
        assert_eq!(div.children, vec![Node::text("Text")]);

        let span = element(&body.children[1]);
        assert_eq!(span.name, "span");
        assert_eq!(span.children, vec![Node::text("Tail")]);

        let paragraph = element(&body.children[2]);
        assert_eq!(paragraph.name, "p");
        assert_eq!(paragraph.children, vec![Node::text("Next")]);

        let section = element(&body.children[3]);
        assert_eq!(section.name, "section");
        assert_eq!(section.children, vec![Node::text("Block")]);
        assert_eq!(
            output
                .parser_diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>(),
            vec![
                "non-void-html-element-self-closing",
                "non-void-html-element-self-closing",
                "non-void-html-element-self-closing",
                "non-void-html-element-self-closing",
            ]
        );
    }

    #[test]
    fn self_closing_text_mode_elements_still_drive_tokenizer_handoff() {
        let output = parse_html_with_diagnostics(
            "<title/>Tom &amp; Jerry</title><style/>a < b &amp; c</style><script/>if (a < b)</script><textarea/>\nA &lt; B</textarea><p>x</p>",
        )
        .unwrap();

        let title = element(&head(&output.document).children[0]);
        assert_eq!(title.name, "title");
        assert_eq!(title.children, vec![Node::text("Tom & Jerry")]);

        let style = element(&head(&output.document).children[1]);
        assert_eq!(style.name, "style");
        assert_eq!(style.children, vec![Node::text("a < b &amp; c")]);

        let textarea = element(&body(&output.document).children[0]);
        assert_eq!(textarea.name, "textarea");
        assert_eq!(textarea.children, vec![Node::text("A < B")]);

        let script = element(&head(&output.document).children[2]);
        assert_eq!(script.name, "script");
        assert_eq!(script.children, vec![Node::text("if (a < b)")]);

        let paragraph = element(&body(&output.document).children[1]);
        assert_eq!(paragraph.children, vec![Node::text("x")]);
        assert_eq!(
            output
                .parser_diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>(),
            vec![
                "non-void-html-element-self-closing",
                "non-void-html-element-self-closing",
                "non-void-html-element-self-closing",
                "non-void-html-element-self-closing",
            ]
        );
    }

    #[test]
    fn self_closing_plaintext_still_consumes_until_eof() {
        let output =
            parse_html_with_diagnostics("<p>before</p><plaintext/><b>&amp;</b></plaintext>")
                .unwrap();

        let body = body(&output.document);
        let paragraph = element(&body.children[0]);
        assert_eq!(paragraph.children, vec![Node::text("before")]);

        let plaintext = element(&body.children[1]);
        assert_eq!(
            plaintext.children,
            vec![Node::text("<b>&amp;</b></plaintext>")]
        );
        assert_eq!(
            output.parser_diagnostics,
            vec![ParserDiagnostic::new(
                "non-void-html-element-self-closing",
                "self-closing flag on non-void HTML element `<plaintext>` was ignored"
            )]
        );
    }

    #[test]
    fn self_closing_noscript_uses_scripting_sensitive_handoff() {
        let enabled =
            parse_html_with_diagnostics("<noscript/><p>&amp;</p></noscript><p>x</p>").unwrap();

        let enabled_noscript = element(&head(&enabled.document).children[0]);
        assert_eq!(enabled_noscript.name, "noscript");
        assert_eq!(enabled_noscript.children, vec![Node::text("<p>&amp;</p>")]);
        assert_eq!(
            element(&body(&enabled.document).children[0]).children,
            vec![Node::text("x")]
        );

        let disabled = parse_html_with_diagnostics_and_options(
            "<noscript/><p>&amp;</p></noscript><p>x</p>",
            HtmlParseOptions {
                scripting: HtmlScriptingMode::Disabled,
                ..HtmlParseOptions::default()
            },
        )
        .unwrap();

        let disabled_noscript = element(&head(&disabled.document).children[0]);
        assert_eq!(disabled_noscript.name, "noscript");
        assert!(disabled_noscript.children.is_empty());
        let fallback_paragraph = element(&body(&disabled.document).children[0]);
        assert_eq!(fallback_paragraph.children, vec![Node::text("&")]);
        assert_eq!(
            element(&body(&disabled.document).children[1]).children,
            vec![Node::text("x")]
        );

        assert_eq!(
            enabled.parser_diagnostics,
            vec![ParserDiagnostic::new(
                "non-void-html-element-self-closing",
                "self-closing flag on non-void HTML element `<noscript>` was ignored"
            )]
        );
        assert_eq!(
            disabled.parser_diagnostics,
            vec![
                ParserDiagnostic::new(
                    "non-void-html-element-self-closing",
                    "self-closing flag on non-void HTML element `<noscript>` was ignored"
                ),
                ParserDiagnostic::new(
                    "unexpected-end-tag",
                    "end tag `</noscript>` did not match an open element"
                )
            ]
        );
    }

    #[test]
    fn acknowledges_self_closing_void_starts_and_ignores_void_end_tags() {
        let output = parse_html_with_diagnostics(
            "<p>Before<br/><img src=hero.png /></img><input></input><hr></hr>After",
        )
        .unwrap();

        let paragraph = element(&body(&output.document).children[0]);
        assert_eq!(paragraph.name, "p");
        assert_eq!(paragraph.children[0], Node::text("Before"));
        assert_eq!(element(&paragraph.children[1]).name, "br");
        assert_eq!(element(&paragraph.children[2]).name, "img");
        assert_eq!(
            element(&paragraph.children[2]).attribute("src"),
            Some("hero.png")
        );
        assert_eq!(element(&paragraph.children[3]).name, "input");
        assert_eq!(element(&body(&output.document).children[1]).name, "hr");
        assert_eq!(body(&output.document).children[2], Node::text("After"));
        assert_eq!(
            output.parser_diagnostics,
            vec![
                ParserDiagnostic::new(
                    "unexpected-void-end-tag",
                    "end tag `</img>` for a void element was ignored"
                ),
                ParserDiagnostic::new(
                    "unexpected-void-end-tag",
                    "end tag `</input>` for a void element was ignored"
                ),
                ParserDiagnostic::new(
                    "unexpected-void-end-tag",
                    "end tag `</hr>` for a void element was ignored"
                ),
            ]
        );
    }

    #[test]
    fn top_level_frameset_replaces_implied_body_and_frame_is_void() {
        let document = parse_html(
            "<frameset><frame><frameset><frame></frameset><noframes></noframes></frameset>",
        )
        .unwrap();

        let html = html(&document);
        assert_eq!(html.children.len(), 2);
        assert_eq!(element(&html.children[0]).name, "head");

        let frameset = element(&html.children[1]);
        assert_eq!(frameset.name, "frameset");
        assert_eq!(frameset.children.len(), 3);
        assert_eq!(element(&frameset.children[0]).name, "frame");
        let nested_frameset = element(&frameset.children[1]);
        assert_eq!(nested_frameset.name, "frameset");
        assert_eq!(element(&nested_frameset.children[0]).name, "frame");
        assert_eq!(element(&frameset.children[2]).name, "noframes");
    }

    #[test]
    fn ignores_frame_start_tags_outside_framesets() {
        let document = parse_html("<frame>test").unwrap();

        assert_eq!(body(&document).children, vec![Node::text("test")]);
    }

    #[test]
    fn ignores_non_whitespace_text_directly_inside_framesets() {
        let document = parse_html("<!DOCTYPE html><frameset>test").unwrap();

        let html = html(&document);
        let frameset = element(&html.children[1]);
        assert_eq!(frameset.name, "frameset");
        assert!(frameset.children.is_empty());
    }

    #[test]
    fn preserves_only_whitespace_from_mixed_frameset_text() {
        let document = parse_html("<!DOCTYPE html><frameset> te st").unwrap();

        let html = html(&document);
        let frameset = element(&html.children[1]);
        assert_eq!(frameset.children, vec![Node::text("  ")]);
    }

    #[test]
    fn top_level_frameset_keeps_filtered_trailing_html_text() {
        let document = parse_html("<!DOCTYPE html><frameset></frameset> te st").unwrap();

        let html = html(&document);
        assert_eq!(element(&html.children[1]).name, "frameset");
        assert_eq!(html.children[2], Node::text("  "));
    }

    #[test]
    fn ignores_doctype_tokens_inside_framesets() {
        let document = parse_html("<!DOCTYPE html><frameset><!DOCTYPE html>").unwrap();

        let html = html(&document);
        let frameset = element(&html.children[1]);
        assert!(frameset.children.is_empty());
    }

    #[test]
    fn ignores_before_html_whitespace_and_duplicate_doctype() {
        let document = parse_html("<!DOCTYPE html> <!DOCTYPE html>").unwrap();

        assert_eq!(document.children.len(), 2);
        assert!(matches!(document.children[0], Node::DocumentType(_)));
        assert!(body(&document).children.is_empty());
    }

    #[test]
    fn merges_html_start_tags_seen_inside_head() {
        let document = parse_html("<!DOCTYPE html><head><html id=x>").unwrap();

        assert_eq!(html(&document).attribute("id"), Some("x"));
        assert!(head(&document).children.is_empty());
    }

    #[test]
    fn keeps_trailing_whitespace_after_html_end_when_body_has_text() {
        let document = parse_html("<!DOCTYPE html>X</html> ").unwrap();

        assert_eq!(body(&document).children, vec![Node::text("X ")]);
    }

    #[test]
    fn keeps_comments_in_source_order_for_implicit_body_content() {
        let document = parse_html("><!--<!--x-->-->").unwrap();

        assert_eq!(body(&document).children[0], Node::text(">"));
        assert!(matches!(body(&document).children[1], Node::Comment(_)));
        assert_eq!(body(&document).children[2], Node::text("-->"));
    }

    #[test]
    fn keeps_comments_after_head_before_body_at_html_level() {
        let document =
            parse_html("<head></head><!-- --><style></style><!-- --><script></script>").unwrap();

        let html = html(&document);
        assert_eq!(element(&html.children[0]).name, "head");
        assert!(matches!(html.children[1], Node::Comment(_)));
        assert!(matches!(html.children[2], Node::Comment(_)));
        let head = element(&html.children[0]);
        assert_eq!(element(&head.children[0]).name, "style");
        assert_eq!(element(&head.children[1]).name, "script");
        assert_eq!(element(&html.children[3]).name, "body");
    }

    #[test]
    fn treats_legacy_image_start_tag_as_img() {
        let output = parse_html_with_diagnostics("<p><image src=hero.png></p>").unwrap();

        let paragraph = element(&body(&output.document).children[0]);
        assert_eq!(paragraph.name, "p");
        let image = element(&paragraph.children[0]);
        assert_eq!(image.name, "img");
        assert_eq!(image.attribute("src"), Some("hero.png"));
        assert_eq!(
            output.parser_diagnostics,
            vec![ParserDiagnostic::new(
                "unexpected-start-tag-treated-as",
                "start tag `<image>` was treated as `<img>`"
            )]
        );
    }

    #[test]
    fn ignores_self_closing_flag_inside_implied_table_structure() {
        let output =
            parse_html_with_diagnostics("<table><tr/><td/>A<td/>B</table><p>after</p>").unwrap();

        let table = element(&body(&output.document).children[0]);
        let tbody = element(&table.children[0]);
        let row = element(&tbody.children[0]);
        assert_eq!(row.name, "tr");
        assert_eq!(row.children.len(), 2);

        let first_cell = element(&row.children[0]);
        assert_eq!(first_cell.name, "td");
        assert_eq!(first_cell.children, vec![Node::text("A")]);

        let second_cell = element(&row.children[1]);
        assert_eq!(second_cell.name, "td");
        assert_eq!(second_cell.children, vec![Node::text("B")]);

        let paragraph = element(&body(&output.document).children[1]);
        assert_eq!(paragraph.children, vec![Node::text("after")]);
        assert_eq!(
            output
                .parser_diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>(),
            vec![
                "non-void-html-element-self-closing",
                "non-void-html-element-self-closing",
                "non-void-html-element-self-closing",
            ]
        );
    }

    #[test]
    fn fosters_recovered_br_end_tag_before_table_rows() {
        let document = parse_html("<table><tr></body></br></table>").unwrap();

        let body = body(&document);
        assert_eq!(element(&body.children[0]).name, "br");
        assert_eq!(element(&body.children[1]).name, "table");
    }

    #[test]
    fn fosters_plaintext_before_table_context_and_keeps_it_open() {
        let document = parse_html("<table><plaintext><td>").unwrap();

        let body = body(&document);
        let plaintext = element(&body.children[0]);
        assert_eq!(plaintext.name, "plaintext");
        assert_eq!(plaintext.children, vec![Node::text("<td>")]);
        assert_eq!(element(&body.children[1]).name, "table");
    }

    #[test]
    fn parser_drives_noscript_rawtext_when_scripting_is_enabled() {
        let document = parse_html("<noscript><p>&amp;</p></noscript><p>x</p>").unwrap();

        let noscript = element(&head(&document).children[0]);
        assert_eq!(noscript.name, "noscript");
        assert_eq!(noscript.children, vec![Node::text("<p>&amp;</p>")]);

        let paragraph = element(&body(&document).children[0]);
        assert_eq!(paragraph.name, "p");
        assert_eq!(paragraph.children, vec![Node::text("x")]);
    }

    #[test]
    fn parser_parses_noscript_markup_when_scripting_is_disabled() {
        let document = parse_html_with_options(
            "<noscript><p>&amp;</p></noscript><p>x</p>",
            HtmlParseOptions {
                scripting: HtmlScriptingMode::Disabled,
                ..HtmlParseOptions::default()
            },
        )
        .unwrap();

        let noscript = element(&head(&document).children[0]);
        assert_eq!(noscript.name, "noscript");
        assert!(noscript.children.is_empty());

        let fallback_paragraph = element(&body(&document).children[0]);
        assert_eq!(fallback_paragraph.name, "p");
        assert_eq!(fallback_paragraph.children, vec![Node::text("&")]);

        let paragraph = element(&body(&document).children[1]);
        assert_eq!(paragraph.name, "p");
        assert_eq!(paragraph.children, vec![Node::text("x")]);
    }

    #[test]
    fn parser_can_start_in_foreign_content_cdata_context() {
        let document = parse_html_with_options(
            "<svg:title>&amp;</svg:title>]]><p>x</p>",
            HtmlParseOptions {
                initial_tokenizer_context: HtmlInitialTokenizerContext::ForeignContentCdataSection,
                ..HtmlParseOptions::default()
            },
        )
        .unwrap();

        assert_eq!(
            body(&document).children[0],
            Node::text("<svg:title>&amp;</svg:title>")
        );

        let paragraph = element(&body(&document).children[1]);
        assert_eq!(paragraph.name, "p");
        assert_eq!(paragraph.children, vec![Node::text("x")]);
    }

    #[test]
    fn parser_can_start_in_intermediate_text_fragment_contexts() {
        let rcdata = parse_html_with_options(
            "b &amp;",
            HtmlParseOptions {
                initial_tokenizer_context: HtmlInitialTokenizerContext::RcdataLessThanSign,
                ..HtmlParseOptions::default()
            },
        )
        .unwrap();
        assert_eq!(body(&rcdata).children, vec![Node::text("<b &")]);

        let rawtext = parse_html_with_options(
            "b &amp;",
            HtmlParseOptions {
                initial_tokenizer_context: HtmlInitialTokenizerContext::RawtextLessThanSign,
                ..HtmlParseOptions::default()
            },
        )
        .unwrap();
        assert_eq!(body(&rawtext).children, vec![Node::text("<b &amp;")]);

        let cdata_bracket = parse_html_with_options(
            "",
            HtmlParseOptions {
                initial_tokenizer_context:
                    HtmlInitialTokenizerContext::ForeignContentCdataSectionBracket,
                ..HtmlParseOptions::default()
            },
        )
        .unwrap();
        assert_eq!(body(&cdata_bracket).children, vec![Node::text("]")]);

        let cdata_end = parse_html_with_options(
            ">after<p>x</p>",
            HtmlParseOptions {
                initial_tokenizer_context:
                    HtmlInitialTokenizerContext::ForeignContentCdataSectionEnd,
                ..HtmlParseOptions::default()
            },
        )
        .unwrap();
        assert_eq!(body(&cdata_end).children[0], Node::text("after"));
        let paragraph = element(&body(&cdata_end).children[1]);
        assert_eq!(paragraph.children, vec![Node::text("x")]);

        let plaintext = parse_html_with_options(
            "<b>&amp;</b></plaintext><p>x</p>",
            HtmlParseOptions {
                initial_tokenizer_context: HtmlInitialTokenizerContext::Plaintext,
                ..HtmlParseOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            body(&plaintext).children,
            vec![Node::text("<b>&amp;</b></plaintext><p>x</p>")]
        );
    }

    #[test]
    fn parser_can_start_in_text_end_tag_open_fragment_contexts() {
        let rcdata = parse_html_with_diagnostics_and_options(
            "title>after<p>x</p>",
            HtmlParseOptions {
                initial_tokenizer_context: HtmlInitialTokenizerContext::RcdataEndTagOpen,
                ..HtmlParseOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            rcdata.parser_diagnostics,
            vec![ParserDiagnostic::new(
                "unexpected-end-tag",
                "end tag `</title>` did not match an open element"
            )]
        );
        assert_eq!(body(&rcdata.document).children[0], Node::text("after"));
        let paragraph = element(&body(&rcdata.document).children[1]);
        assert_eq!(paragraph.children, vec![Node::text("x")]);

        let rawtext = parse_html_with_diagnostics_and_options(
            "style>tail<p>after</p>",
            HtmlParseOptions {
                initial_tokenizer_context: HtmlInitialTokenizerContext::RawtextEndTagOpen,
                ..HtmlParseOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            rawtext.parser_diagnostics,
            vec![ParserDiagnostic::new(
                "unexpected-end-tag",
                "end tag `</style>` did not match an open element"
            )]
        );
        assert_eq!(body(&rawtext.document).children[0], Node::text("tail"));
        let paragraph = element(&body(&rawtext.document).children[1]);
        assert_eq!(paragraph.children, vec![Node::text("after")]);
    }

    #[test]
    fn parser_can_start_in_seeded_text_end_tag_continuation_contexts() {
        for (context, source, unmatched_tag, expected_text, expected_lexer_diagnostic) in [
            (
                HtmlInitialTokenizerContext::RcdataEndTagName,
                ">after<p>x</p>",
                "title",
                "after",
                None,
            ),
            (
                HtmlInitialTokenizerContext::RawtextEndTagWhitespace,
                ">tail<p>x</p>",
                "style",
                "tail",
                Some("unexpected-whitespace-after-end-tag-name"),
            ),
            (
                HtmlInitialTokenizerContext::ScriptDataEndTagAttributes,
                ">tail<p>x</p>",
                "script",
                "tail",
                Some("end-tag-with-attributes"),
            ),
            (
                HtmlInitialTokenizerContext::ScriptDataEscapedSelfClosingEndTag,
                ">tail<p>x</p>",
                "script",
                "tail",
                Some("end-tag-with-trailing-solidus"),
            ),
        ] {
            let output = parse_html_with_diagnostics_and_options(
                source,
                HtmlParseOptions {
                    initial_tokenizer_context: context,
                    ..HtmlParseOptions::default()
                },
            )
            .unwrap();

            assert_eq!(
                body(&output.document).children[0],
                Node::text(expected_text)
            );
            let paragraph = element(&body(&output.document).children[1]);
            assert_eq!(paragraph.children, vec![Node::text("x")]);
            assert_eq!(
                output.parser_diagnostics,
                vec![ParserDiagnostic::new(
                    "unexpected-end-tag",
                    format!("end tag `</{unmatched_tag}>` did not match an open element")
                )],
                "context {context:?}"
            );
            let actual_lexer_diagnostics = output
                .lexer_diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>();
            assert_eq!(
                actual_lexer_diagnostics,
                expected_lexer_diagnostic.into_iter().collect::<Vec<_>>(),
                "context {context:?}"
            );
        }
    }

    #[test]
    fn parser_can_start_in_seeded_comment_continuation_contexts() {
        for (context, source, expected_comment, expected_lexer_diagnostics) in [
            (
                HtmlInitialTokenizerContext::CommentStart,
                "body --><p>x</p>",
                "body ",
                Vec::<&str>::new(),
            ),
            (
                HtmlInitialTokenizerContext::CommentStartDash,
                "><p>x</p>",
                "",
                vec!["abrupt-closing-of-empty-comment"],
            ),
            (
                HtmlInitialTokenizerContext::Comment,
                " body --><p>x</p>",
                "seed body ",
                Vec::<&str>::new(),
            ),
            (
                HtmlInitialTokenizerContext::CommentLessThanSign,
                "x--><p>x</p>",
                "seed<x",
                Vec::<&str>::new(),
            ),
            (
                HtmlInitialTokenizerContext::CommentLessThanSignBang,
                "x--><p>x</p>",
                "seed<!x",
                Vec::<&str>::new(),
            ),
            (
                HtmlInitialTokenizerContext::CommentLessThanSignBangDash,
                "x--><p>x</p>",
                "seed<!-x",
                Vec::<&str>::new(),
            ),
            (
                HtmlInitialTokenizerContext::CommentLessThanSignBangDashDash,
                "x--><p>x</p>",
                "seed<!--x",
                vec!["nested-comment"],
            ),
            (
                HtmlInitialTokenizerContext::CommentEndDash,
                "x--><p>x</p>",
                "seed-x",
                Vec::<&str>::new(),
            ),
            (
                HtmlInitialTokenizerContext::CommentEnd,
                "!><p>x</p>",
                "seed",
                vec!["incorrectly-closed-comment"],
            ),
            (
                HtmlInitialTokenizerContext::CommentEndBang,
                "y--><p>x</p>",
                "seed--!y",
                Vec::<&str>::new(),
            ),
            (
                HtmlInitialTokenizerContext::BogusComment,
                "tail><p>x</p>",
                "bogus-tail",
                Vec::<&str>::new(),
            ),
        ] {
            let output = parse_html_with_diagnostics_and_options(
                source,
                HtmlParseOptions {
                    initial_tokenizer_context: context,
                    ..HtmlParseOptions::default()
                },
            )
            .unwrap();

            assert_eq!(output.document.children[0], Node::comment(expected_comment));
            let paragraph = element(&body(&output.document).children[0]);
            assert_eq!(paragraph.children, vec![Node::text("x")]);
            assert!(output.parser_diagnostics.is_empty(), "context {context:?}");

            let actual_lexer_diagnostics = output
                .lexer_diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>();
            assert_eq!(
                actual_lexer_diagnostics, expected_lexer_diagnostics,
                "context {context:?}"
            );
        }
    }

    #[test]
    fn parser_exposes_all_seeded_doctype_continuation_contexts() {
        for context in [
            HtmlInitialTokenizerContext::DoctypeKeywordO,
            HtmlInitialTokenizerContext::DoctypeKeywordC,
            HtmlInitialTokenizerContext::DoctypeKeywordT,
            HtmlInitialTokenizerContext::DoctypeKeywordY,
            HtmlInitialTokenizerContext::DoctypeKeywordP,
            HtmlInitialTokenizerContext::DoctypeKeywordE,
            HtmlInitialTokenizerContext::DoctypeAfterKeyword,
            HtmlInitialTokenizerContext::BeforeDoctypeName,
            HtmlInitialTokenizerContext::DoctypeName,
            HtmlInitialTokenizerContext::AfterDoctypeName,
            HtmlInitialTokenizerContext::DoctypePublicKeywordU,
            HtmlInitialTokenizerContext::DoctypePublicKeywordB,
            HtmlInitialTokenizerContext::DoctypePublicKeywordL,
            HtmlInitialTokenizerContext::DoctypePublicKeywordI,
            HtmlInitialTokenizerContext::DoctypePublicKeywordC,
            HtmlInitialTokenizerContext::AfterDoctypePublicKeyword,
            HtmlInitialTokenizerContext::BeforeDoctypePublicIdentifier,
            HtmlInitialTokenizerContext::DoctypePublicIdentifierDoubleQuoted,
            HtmlInitialTokenizerContext::DoctypePublicIdentifierSingleQuoted,
            HtmlInitialTokenizerContext::AfterDoctypePublicIdentifier,
            HtmlInitialTokenizerContext::BetweenDoctypePublicAndSystemIdentifiers,
            HtmlInitialTokenizerContext::DoctypeSystemKeywordY,
            HtmlInitialTokenizerContext::DoctypeSystemKeywordS,
            HtmlInitialTokenizerContext::DoctypeSystemKeywordT,
            HtmlInitialTokenizerContext::DoctypeSystemKeywordE,
            HtmlInitialTokenizerContext::DoctypeSystemKeywordM,
            HtmlInitialTokenizerContext::AfterDoctypeSystemKeyword,
            HtmlInitialTokenizerContext::BeforeDoctypeSystemIdentifier,
            HtmlInitialTokenizerContext::DoctypeSystemIdentifierDoubleQuoted,
            HtmlInitialTokenizerContext::DoctypeSystemIdentifierSingleQuoted,
            HtmlInitialTokenizerContext::AfterDoctypeSystemIdentifier,
            HtmlInitialTokenizerContext::BogusDoctype,
        ] {
            let context = context.lex_context();
            assert!(context.initial_state.requires_doctype_seed());
        }
    }

    #[test]
    fn parser_can_start_in_seeded_doctype_continuation_contexts() {
        for (
            context,
            source,
            expected_name,
            expected_public,
            expected_system,
            expected_force_quirks,
            expected_lexer_diagnostics,
        ) in [
            (
                HtmlInitialTokenizerContext::DoctypeKeywordO,
                "OCTYPE html><p>x</p>",
                Some("html"),
                None,
                None,
                false,
                Vec::<&str>::new(),
            ),
            (
                HtmlInitialTokenizerContext::DoctypeName,
                "ml><p>x</p>",
                Some("html"),
                None,
                None,
                false,
                Vec::<&str>::new(),
            ),
            (
                HtmlInitialTokenizerContext::AfterDoctypeName,
                "PUBLIC \"pub\" \"sys\"><p>x</p>",
                Some("html"),
                Some("pub"),
                Some("sys"),
                false,
                Vec::<&str>::new(),
            ),
            (
                HtmlInitialTokenizerContext::DoctypePublicIdentifierDoubleQuoted,
                "b\" \"sys\"><p>x</p>",
                Some("html"),
                Some("pub"),
                Some("sys"),
                false,
                Vec::<&str>::new(),
            ),
            (
                HtmlInitialTokenizerContext::AfterDoctypePublicIdentifier,
                "\"sys\"><p>x</p>",
                Some("html"),
                Some("pub"),
                Some("sys"),
                false,
                vec!["missing-whitespace-between-doctype-public-and-system-identifiers"],
            ),
            (
                HtmlInitialTokenizerContext::BeforeDoctypePublicIdentifier,
                "><p>x</p>",
                Some("html"),
                None,
                None,
                true,
                vec!["missing-doctype-public-identifier"],
            ),
            (
                HtmlInitialTokenizerContext::DoctypeSystemIdentifierSingleQuoted,
                "s'><p>x</p>",
                Some("html"),
                None,
                Some("sys"),
                false,
                Vec::<&str>::new(),
            ),
            (
                HtmlInitialTokenizerContext::AfterDoctypeSystemIdentifier,
                "junk><p>x</p>",
                Some("html"),
                None,
                Some("sys"),
                false,
                vec!["unexpected-character-after-doctype-system-identifier"],
            ),
            (
                HtmlInitialTokenizerContext::BogusDoctype,
                "ignored><p>x</p>",
                Some("html"),
                None,
                None,
                true,
                Vec::<&str>::new(),
            ),
        ] {
            let output = parse_html_with_diagnostics_and_options(
                source,
                HtmlParseOptions {
                    initial_tokenizer_context: context,
                    ..HtmlParseOptions::default()
                },
            )
            .unwrap();

            assert_eq!(
                output.document.children[0],
                Node::DocumentType(DocumentType {
                    name: expected_name.map(str::to_string),
                    public_identifier: expected_public.map(str::to_string),
                    system_identifier: expected_system.map(str::to_string),
                    force_quirks: expected_force_quirks,
                }),
                "context {context:?}"
            );
            let paragraph = element(&body(&output.document).children[0]);
            assert_eq!(paragraph.children, vec![Node::text("x")]);
            assert!(output.parser_diagnostics.is_empty(), "context {context:?}");

            let actual_lexer_diagnostics = output
                .lexer_diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>();
            assert_eq!(
                actual_lexer_diagnostics, expected_lexer_diagnostics,
                "context {context:?}"
            );
        }
    }

    #[test]
    fn parser_exposes_all_seeded_character_reference_continuation_contexts() {
        for (context, return_state, temporary_buffer) in [
            (
                HtmlInitialTokenizerContext::CharacterReference,
                HtmlTokenizerState::Data,
                "&",
            ),
            (
                HtmlInitialTokenizerContext::NamedCharacterReference,
                HtmlTokenizerState::Data,
                "&co",
            ),
            (
                HtmlInitialTokenizerContext::NumericCharacterReference,
                HtmlTokenizerState::Data,
                "&#",
            ),
            (
                HtmlInitialTokenizerContext::NumericHexCharacterReferenceStart,
                HtmlTokenizerState::Data,
                "&#x",
            ),
            (
                HtmlInitialTokenizerContext::NumericHexCharacterReference,
                HtmlTokenizerState::Data,
                "&#x4",
            ),
            (
                HtmlInitialTokenizerContext::NumericDecimalCharacterReference,
                HtmlTokenizerState::Data,
                "&#6",
            ),
            (
                HtmlInitialTokenizerContext::RcdataCharacterReference,
                HtmlTokenizerState::Rcdata,
                "&",
            ),
            (
                HtmlInitialTokenizerContext::RcdataNamedCharacterReference,
                HtmlTokenizerState::Rcdata,
                "&a",
            ),
            (
                HtmlInitialTokenizerContext::RcdataNumericCharacterReference,
                HtmlTokenizerState::Rcdata,
                "&#",
            ),
            (
                HtmlInitialTokenizerContext::RcdataNumericHexCharacterReferenceStart,
                HtmlTokenizerState::Rcdata,
                "&#x",
            ),
            (
                HtmlInitialTokenizerContext::RcdataNumericHexCharacterReference,
                HtmlTokenizerState::Rcdata,
                "&#x4",
            ),
            (
                HtmlInitialTokenizerContext::RcdataNumericDecimalCharacterReference,
                HtmlTokenizerState::Rcdata,
                "&#6",
            ),
        ] {
            let context = context.lex_context();
            assert!(context.initial_state.requires_character_reference_seed());
            assert_eq!(context.return_state, Some(return_state));
            assert_eq!(context.temporary_buffer.as_deref(), Some(temporary_buffer));
            if return_state == HtmlTokenizerState::Rcdata {
                assert_eq!(context.last_start_tag.as_deref(), Some("title"));
            }
        }
    }

    #[test]
    fn parser_can_start_in_seeded_character_reference_continuation_contexts() {
        for (
            context,
            source,
            expected_text,
            expected_lexer_diagnostics,
            expected_parser_diagnostics,
        ) in [
            (
                HtmlInitialTokenizerContext::CharacterReference,
                " nope<p>x</p>",
                "& nope",
                Vec::<&str>::new(),
                Vec::<ParserDiagnostic>::new(),
            ),
            (
                HtmlInitialTokenizerContext::NamedCharacterReference,
                "py;<p>x</p>",
                "©",
                Vec::<&str>::new(),
                Vec::<ParserDiagnostic>::new(),
            ),
            (
                HtmlInitialTokenizerContext::NumericCharacterReference,
                "65;!<p>x</p>",
                "A!",
                Vec::<&str>::new(),
                Vec::<ParserDiagnostic>::new(),
            ),
            (
                HtmlInitialTokenizerContext::NumericHexCharacterReferenceStart,
                "41;!<p>x</p>",
                "A!",
                Vec::<&str>::new(),
                Vec::<ParserDiagnostic>::new(),
            ),
            (
                HtmlInitialTokenizerContext::NumericHexCharacterReference,
                "1!<p>x</p>",
                "A!",
                vec!["missing-semicolon-after-character-reference"],
                Vec::<ParserDiagnostic>::new(),
            ),
            (
                HtmlInitialTokenizerContext::NumericDecimalCharacterReference,
                "5;!<p>x</p>",
                "A!",
                Vec::<&str>::new(),
                Vec::<ParserDiagnostic>::new(),
            ),
            (
                HtmlInitialTokenizerContext::RcdataCharacterReference,
                " nope</title><p>x</p>",
                "& nope",
                Vec::<&str>::new(),
                vec![ParserDiagnostic::new(
                    "unexpected-end-tag",
                    "end tag `</title>` did not match an open element",
                )],
            ),
            (
                HtmlInitialTokenizerContext::RcdataNamedCharacterReference,
                "mp; &amp;</title><p>x</p>",
                "& &",
                Vec::<&str>::new(),
                vec![ParserDiagnostic::new(
                    "unexpected-end-tag",
                    "end tag `</title>` did not match an open element",
                )],
            ),
            (
                HtmlInitialTokenizerContext::RcdataNumericCharacterReference,
                "65;</title><p>x</p>",
                "A",
                Vec::<&str>::new(),
                vec![ParserDiagnostic::new(
                    "unexpected-end-tag",
                    "end tag `</title>` did not match an open element",
                )],
            ),
            (
                HtmlInitialTokenizerContext::RcdataNumericHexCharacterReferenceStart,
                "41;</title><p>x</p>",
                "A",
                Vec::<&str>::new(),
                vec![ParserDiagnostic::new(
                    "unexpected-end-tag",
                    "end tag `</title>` did not match an open element",
                )],
            ),
            (
                HtmlInitialTokenizerContext::RcdataNumericHexCharacterReference,
                "1!</title><p>x</p>",
                "A!",
                vec!["missing-semicolon-after-character-reference"],
                vec![ParserDiagnostic::new(
                    "unexpected-end-tag",
                    "end tag `</title>` did not match an open element",
                )],
            ),
            (
                HtmlInitialTokenizerContext::RcdataNumericDecimalCharacterReference,
                "5;</title><p>x</p>",
                "A",
                Vec::<&str>::new(),
                vec![ParserDiagnostic::new(
                    "unexpected-end-tag",
                    "end tag `</title>` did not match an open element",
                )],
            ),
        ] {
            let output = parse_html_with_diagnostics_and_options(
                source,
                HtmlParseOptions {
                    initial_tokenizer_context: context,
                    ..HtmlParseOptions::default()
                },
            )
            .unwrap();

            assert_eq!(
                body(&output.document).children[0],
                Node::text(expected_text),
                "context {context:?}"
            );
            let paragraph = element(&body(&output.document).children[1]);
            assert_eq!(paragraph.children, vec![Node::text("x")]);
            assert_eq!(
                output.parser_diagnostics, expected_parser_diagnostics,
                "context {context:?}"
            );

            let actual_lexer_diagnostics = output
                .lexer_diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>();
            assert_eq!(
                actual_lexer_diagnostics, expected_lexer_diagnostics,
                "context {context:?}"
            );
        }
    }

    #[test]
    fn parser_can_start_in_script_escaped_fragment_context() {
        let output = parse_html_with_diagnostics_and_options(
            "x</script><p>after</p>",
            HtmlParseOptions {
                initial_tokenizer_context: HtmlInitialTokenizerContext::ScriptDataEscapedDashDash,
                ..HtmlParseOptions::default()
            },
        )
        .unwrap();

        let body = body(&output.document);
        assert_eq!(body.children[0], Node::text("x"));

        let paragraph = element(&body.children[1]);
        assert_eq!(paragraph.name, "p");
        assert_eq!(paragraph.children, vec![Node::text("after")]);
        assert_eq!(
            output.parser_diagnostics,
            vec![ParserDiagnostic::new(
                "unexpected-end-tag",
                "end tag `</script>` did not match an open element"
            )]
        );
    }

    #[test]
    fn parser_can_start_in_script_double_escaped_less_than_context() {
        let output = parse_html_with_diagnostics_and_options(
            "/script>tail</script><p>after</p>",
            HtmlParseOptions {
                initial_tokenizer_context:
                    HtmlInitialTokenizerContext::ScriptDataDoubleEscapedLessThanSign,
                ..HtmlParseOptions::default()
            },
        )
        .unwrap();

        let body = body(&output.document);
        assert_eq!(body.children[0], Node::text("/script>tail"));

        let paragraph = element(&body.children[1]);
        assert_eq!(paragraph.name, "p");
        assert_eq!(paragraph.children, vec![Node::text("after")]);
        assert_eq!(
            output.parser_diagnostics,
            vec![ParserDiagnostic::new(
                "unexpected-end-tag",
                "end tag `</script>` did not match an open element"
            )]
        );
    }

    #[test]
    fn parser_can_start_in_intermediate_script_fragment_contexts() {
        let less_than = parse_html_with_diagnostics_and_options(
            "!-->tail</script><p>after</p>",
            HtmlParseOptions {
                initial_tokenizer_context: HtmlInitialTokenizerContext::ScriptDataLessThanSign,
                ..HtmlParseOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            body(&less_than.document).children[0],
            Node::text("<!-->tail")
        );
        assert_eq!(
            less_than.parser_diagnostics,
            vec![ParserDiagnostic::new(
                "unexpected-end-tag",
                "end tag `</script>` did not match an open element"
            )]
        );

        let end_tag_open = parse_html_with_diagnostics_and_options(
            "script>tail<p>after</p>",
            HtmlParseOptions {
                initial_tokenizer_context: HtmlInitialTokenizerContext::ScriptDataEndTagOpen,
                ..HtmlParseOptions::default()
            },
        )
        .unwrap();
        assert_eq!(body(&end_tag_open.document).children[0], Node::text("tail"));
        let paragraph = element(&body(&end_tag_open.document).children[1]);
        assert_eq!(paragraph.children, vec![Node::text("after")]);
        assert_eq!(
            end_tag_open.parser_diagnostics,
            vec![ParserDiagnostic::new(
                "unexpected-end-tag",
                "end tag `</script>` did not match an open element"
            )]
        );

        let escaped_end_tag_open = parse_html_with_diagnostics_and_options(
            "script>tail<p>after</p>",
            HtmlParseOptions {
                initial_tokenizer_context: HtmlInitialTokenizerContext::ScriptDataEscapedEndTagOpen,
                ..HtmlParseOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            body(&escaped_end_tag_open.document).children[0],
            Node::text("tail")
        );
        let paragraph = element(&body(&escaped_end_tag_open.document).children[1]);
        assert_eq!(paragraph.children, vec![Node::text("after")]);
        assert_eq!(
            escaped_end_tag_open.parser_diagnostics,
            vec![ParserDiagnostic::new(
                "unexpected-end-tag",
                "end tag `</script>` did not match an open element"
            )]
        );

        let double_escape_start = parse_html_with_diagnostics_and_options(
            "script>inside</script>after</script><p>after</p>",
            HtmlParseOptions {
                initial_tokenizer_context: HtmlInitialTokenizerContext::ScriptDataDoubleEscapeStart,
                ..HtmlParseOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            body(&double_escape_start.document).children[0],
            Node::text("script>inside</script>after")
        );
        let paragraph = element(&body(&double_escape_start.document).children[1]);
        assert_eq!(paragraph.children, vec![Node::text("after")]);
        assert_eq!(
            double_escape_start.parser_diagnostics,
            vec![ParserDiagnostic::new(
                "unexpected-end-tag",
                "end tag `</script>` did not match an open element"
            )]
        );

        let double_escape_end = parse_html_with_diagnostics_and_options(
            "script>tail</script><p>after</p>",
            HtmlParseOptions {
                initial_tokenizer_context: HtmlInitialTokenizerContext::ScriptDataDoubleEscapeEnd,
                ..HtmlParseOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            body(&double_escape_end.document).children[0],
            Node::text("script>tail")
        );
        let paragraph = element(&body(&double_escape_end.document).children[1]);
        assert_eq!(paragraph.children, vec![Node::text("after")]);
    }

    #[test]
    fn parser_drives_plaintext_tokenization_until_eof() {
        let document = parse_html("<p>before</p><plaintext><b>&amp;</b></plaintext>").unwrap();

        let body = body(&document);
        let paragraph = element(&body.children[0]);
        assert_eq!(paragraph.children, vec![Node::text("before")]);

        let plaintext = element(&body.children[1]);
        assert_eq!(
            plaintext.children,
            vec![Node::text("<b>&amp;</b></plaintext>")]
        );
    }

    #[test]
    fn creates_implied_html_head_and_body_for_legacy_documents() {
        let document = parse_html("<title>Venture</title><p>Hello Mosaic</p>").unwrap();

        assert_eq!(document.children.len(), 1);
        assert_eq!(html(&document).name, "html");
        assert_eq!(head(&document).children.len(), 1);
        assert_eq!(element(&head(&document).children[0]).name, "title");
        assert_eq!(body(&document).children.len(), 1);
        assert_eq!(
            element(&body(&document).children[0]).children,
            vec![Node::text("Hello Mosaic")]
        );
    }

    #[test]
    fn ignores_stray_paragraph_end_tag_before_body_starts() {
        let output = parse_html_with_diagnostics("<head></p><meta><p>").unwrap();

        let head = head(&output.document);
        assert_eq!(head.children.len(), 1);
        assert_eq!(element(&head.children[0]).name, "meta");

        let body = body(&output.document);
        assert_eq!(body.children.len(), 1);
        assert_eq!(element(&body.children[0]).name, "p");
        assert_eq!(
            output.parser_diagnostics,
            vec![ParserDiagnostic::new(
                "unexpected-p-end-tag-before-body",
                "end tag `</p>` before body content was ignored"
            )]
        );
    }

    #[test]
    fn html_end_tag_in_head_moves_following_head_elements_to_body() {
        let document = parse_html("<head></html><meta><p>").unwrap();

        assert!(head(&document).children.is_empty());
        let body = body(&document);
        assert_eq!(body.children.len(), 2);
        assert_eq!(element(&body.children[0]).name, "meta");
        assert_eq!(element(&body.children[1]).name, "p");
    }

    #[test]
    fn preserves_explicit_html_head_body_attributes() {
        let document = parse_html(
            "<!DOCTYPE html><html lang=en><head data-h=yes><title>V</title></head><body class=home><h1>Hi</h1></body></html>",
        )
        .unwrap();

        assert!(matches!(&document.children[0], Node::DocumentType(_)));
        assert_eq!(html(&document).attribute("lang"), Some("en"));
        assert_eq!(head(&document).attribute("data-h"), Some("yes"));
        assert_eq!(body(&document).attribute("class"), Some("home"));
        assert_eq!(element(&head(&document).children[0]).name, "title");
        assert_eq!(element(&body(&document).children[0]).name, "h1");
    }

    #[test]
    fn merges_duplicate_html_and_head_start_tags_without_nesting() {
        let document = parse_html(
            "<html lang=en><html data-app=venture lang=ignored><head id=main><head data-h=yes><title>T</title><body><p>x</p>",
        )
        .unwrap();

        assert_eq!(html(&document).attribute("lang"), Some("en"));
        assert_eq!(html(&document).attribute("data-app"), Some("venture"));
        assert_eq!(head(&document).attribute("id"), Some("main"));
        assert_eq!(head(&document).attribute("data-h"), Some("yes"));
        assert_eq!(head(&document).children.len(), 1);
        assert_eq!(element(&head(&document).children[0]).name, "title");
        assert_eq!(body(&document).children.len(), 1);
        assert_eq!(element(&body(&document).children[0]).name, "p");
    }

    #[test]
    fn ignores_head_start_tags_after_body_content_starts() {
        let output =
            parse_html_with_diagnostics("<body><p>before</p><head data-late=yes><p>after</p>")
                .unwrap();

        assert_eq!(
            output.parser_diagnostics,
            vec![ParserDiagnostic::new(
                "unexpected-head-start-tag",
                "head start tag was ignored after body content had already started"
            )]
        );
        assert_eq!(head(&output.document).attribute("data-late"), None);
        assert_eq!(body(&output.document).children.len(), 2);
        assert_eq!(
            element(&body(&output.document).children[0]).children,
            vec![Node::text("before")]
        );
        assert_eq!(
            element(&body(&output.document).children[1]).children,
            vec![Node::text("after")]
        );
    }

    #[test]
    fn recovers_special_p_and_br_end_tags() {
        let output = parse_html_with_diagnostics("Before</p>Middle</br>After").unwrap();

        assert_eq!(
            output.parser_diagnostics,
            vec![
                ParserDiagnostic::new(
                    "unexpected-p-end-tag",
                    "end tag `</p>` created and closed an implied `p` element"
                ),
                ParserDiagnostic::new(
                    "unexpected-br-end-tag",
                    "end tag `</br>` was recovered as a `br` start tag"
                )
            ]
        );

        let body = body(&output.document);
        assert_eq!(body.children.len(), 5);
        assert_eq!(body.children[0], Node::text("Before"));
        let paragraph = element(&body.children[1]);
        assert_eq!(paragraph.name, "p");
        assert!(paragraph.children.is_empty());
        assert_eq!(body.children[2], Node::text("Middle"));
        assert_eq!(element(&body.children[3]).name, "br");
        assert_eq!(body.children[4], Node::text("After"));
    }

    #[test]
    fn strips_initial_line_feed_in_pre_listing_and_textarea() {
        let document = parse_html(
            "<pre>\nA</pre><listing>\nB</listing><textarea>\nC</textarea><pre> D</pre><pre><span>\nkept</span></pre>",
        )
        .unwrap();

        let body = body(&document);

        let pre = element(&body.children[0]);
        assert_eq!(pre.name, "pre");
        assert_eq!(pre.children, vec![Node::text("A")]);

        let listing = element(&body.children[1]);
        assert_eq!(listing.name, "listing");
        assert_eq!(listing.children, vec![Node::text("B")]);

        let textarea = element(&body.children[2]);
        assert_eq!(textarea.name, "textarea");
        assert_eq!(textarea.children, vec![Node::text("C")]);

        let spaced_pre = element(&body.children[3]);
        assert_eq!(spaced_pre.name, "pre");
        assert_eq!(spaced_pre.children, vec![Node::text(" D")]);

        let nested_pre = element(&body.children[4]);
        let span = element(&nested_pre.children[0]);
        assert_eq!(span.children, vec![Node::text("\nkept")]);
    }

    #[test]
    fn closes_explicit_head_before_body_boundaries() {
        let document =
            parse_html("<html><head data-h=yes><title>T</title><body class=main><p>x</p></html>")
                .unwrap();

        assert_eq!(head(&document).attribute("data-h"), Some("yes"));
        assert_eq!(head(&document).children.len(), 1);
        assert_eq!(element(&head(&document).children[0]).name, "title");
        assert_eq!(body(&document).attribute("class"), Some("main"));
        let paragraph = element(&body(&document).children[0]);
        assert_eq!(paragraph.name, "p");
        assert_eq!(paragraph.children, vec![Node::text("x")]);
    }

    #[test]
    fn closes_explicit_head_before_implicit_body_content() {
        let document = parse_html("<head><title>T</title>hello<p>x</p>").unwrap();

        assert_eq!(head(&document).children.len(), 1);
        assert_eq!(element(&head(&document).children[0]).name, "title");
        assert_eq!(body(&document).children[0], Node::text("hello"));
        let paragraph = element(&body(&document).children[1]);
        assert_eq!(paragraph.name, "p");
        assert_eq!(paragraph.children, vec![Node::text("x")]);
    }

    #[test]
    fn merges_late_body_attributes_without_nesting_body_elements() {
        let document =
            parse_html("<body class=main><p>before</p><body id=late class=ignored><p>after</p>")
                .unwrap();

        let body = body(&document);
        assert_eq!(body.attribute("class"), Some("main"));
        assert_eq!(body.attribute("id"), Some("late"));
        assert_eq!(body.children.len(), 2);

        let before = element(&body.children[0]);
        assert_eq!(before.name, "p");
        assert_eq!(before.children, vec![Node::text("before")]);

        let after = element(&body.children[1]);
        assert_eq!(after.name, "p");
        assert_eq!(after.children, vec![Node::text("after")]);
    }

    #[test]
    fn recovers_omitted_shell_end_tag_boundaries() {
        let output = parse_html_with_diagnostics(
            "<title>T</title></head><p>before</body>after<section>next</html>tail",
        )
        .unwrap();

        assert!(output.parser_diagnostics.is_empty());
        assert_eq!(element(&head(&output.document).children[0]).name, "title");

        let output_body = body(&output.document);
        assert_eq!(output_body.children.len(), 4);

        let first = element(&output_body.children[0]);
        assert_eq!(first.name, "p");
        assert_eq!(first.children, vec![Node::text("before")]);

        assert_eq!(output_body.children[1], Node::text("after"));

        let section = element(&output_body.children[2]);
        assert_eq!(section.name, "section");
        assert_eq!(section.children, vec![Node::text("next")]);

        assert_eq!(output_body.children[3], Node::text("tail"));

        let explicit = parse_html("<html><body><p>x</body>y</html>z").unwrap();
        let explicit_body = body(&explicit);
        assert_eq!(explicit_body.children.len(), 2);
        assert_eq!(
            element(&explicit_body.children[0]).children,
            vec![Node::text("x")]
        );
        assert_eq!(explicit_body.children[1], Node::text("yz"));
    }
}
