//! Rust HTML lexer built on the generic state-machine lexer runtime.
//!
//! The HTML standard calls its lexical stage a tokenizer. This package uses the
//! repo's lexer naming while preserving standard state names like `data` and
//! `tag_open`.

use state_machine::{EffectfulStateMachine, StateMachineDefinition};

pub use state_machine_tokenizer::{
    Attribute, Diagnostic, Result, SourcePosition, Token, Tokenizer as HtmlLexer, TokenizerError,
    TokenizerTraceEntry,
};

mod generated_html1;
mod generated_html_skeleton;

/// Parser-facing scripting flag for tokenizer text-mode decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlScriptingMode {
    Enabled,
    Disabled,
}

/// Parser-facing HTML tokenizer entry state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlTokenizerState {
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
    CdataSection,
    CdataSectionBracket,
    CdataSectionEnd,
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

/// Tokenizer states that are valid parser-facing entry points.
pub const HTML_TOKENIZER_STATES: [HtmlTokenizerState; 54] = [
    HtmlTokenizerState::Data,
    HtmlTokenizerState::Rcdata,
    HtmlTokenizerState::RcdataLessThanSign,
    HtmlTokenizerState::RcdataEndTagOpen,
    HtmlTokenizerState::RcdataEndTagName,
    HtmlTokenizerState::RcdataEndTagWhitespace,
    HtmlTokenizerState::RcdataEndTagAttributes,
    HtmlTokenizerState::RcdataSelfClosingEndTag,
    HtmlTokenizerState::Rawtext,
    HtmlTokenizerState::RawtextLessThanSign,
    HtmlTokenizerState::RawtextEndTagOpen,
    HtmlTokenizerState::RawtextEndTagName,
    HtmlTokenizerState::RawtextEndTagWhitespace,
    HtmlTokenizerState::RawtextEndTagAttributes,
    HtmlTokenizerState::RawtextSelfClosingEndTag,
    HtmlTokenizerState::Plaintext,
    HtmlTokenizerState::CdataSection,
    HtmlTokenizerState::CdataSectionBracket,
    HtmlTokenizerState::CdataSectionEnd,
    HtmlTokenizerState::CommentStart,
    HtmlTokenizerState::CommentStartDash,
    HtmlTokenizerState::Comment,
    HtmlTokenizerState::CommentLessThanSign,
    HtmlTokenizerState::CommentLessThanSignBang,
    HtmlTokenizerState::CommentLessThanSignBangDash,
    HtmlTokenizerState::CommentLessThanSignBangDashDash,
    HtmlTokenizerState::CommentEndDash,
    HtmlTokenizerState::CommentEnd,
    HtmlTokenizerState::CommentEndBang,
    HtmlTokenizerState::BogusComment,
    HtmlTokenizerState::ScriptData,
    HtmlTokenizerState::ScriptDataLessThanSign,
    HtmlTokenizerState::ScriptDataEndTagOpen,
    HtmlTokenizerState::ScriptDataEndTagName,
    HtmlTokenizerState::ScriptDataEndTagWhitespace,
    HtmlTokenizerState::ScriptDataEndTagAttributes,
    HtmlTokenizerState::ScriptDataSelfClosingEndTag,
    HtmlTokenizerState::ScriptDataEscapeStart,
    HtmlTokenizerState::ScriptDataEscapeStartDash,
    HtmlTokenizerState::ScriptDataEscaped,
    HtmlTokenizerState::ScriptDataEscapedDash,
    HtmlTokenizerState::ScriptDataEscapedDashDash,
    HtmlTokenizerState::ScriptDataEscapedLessThanSign,
    HtmlTokenizerState::ScriptDataEscapedEndTagOpen,
    HtmlTokenizerState::ScriptDataEscapedEndTagName,
    HtmlTokenizerState::ScriptDataEscapedEndTagWhitespace,
    HtmlTokenizerState::ScriptDataEscapedEndTagAttributes,
    HtmlTokenizerState::ScriptDataEscapedSelfClosingEndTag,
    HtmlTokenizerState::ScriptDataDoubleEscapeStart,
    HtmlTokenizerState::ScriptDataDoubleEscaped,
    HtmlTokenizerState::ScriptDataDoubleEscapedDash,
    HtmlTokenizerState::ScriptDataDoubleEscapedDashDash,
    HtmlTokenizerState::ScriptDataDoubleEscapedLessThanSign,
    HtmlTokenizerState::ScriptDataDoubleEscapeEnd,
];

/// Tokenizer states used for parser-controlled text or foreign-content fragments.
pub const HTML_FRAGMENT_TOKENIZER_STATES: [HtmlTokenizerState; 53] = [
    HtmlTokenizerState::Rcdata,
    HtmlTokenizerState::RcdataLessThanSign,
    HtmlTokenizerState::RcdataEndTagOpen,
    HtmlTokenizerState::RcdataEndTagName,
    HtmlTokenizerState::RcdataEndTagWhitespace,
    HtmlTokenizerState::RcdataEndTagAttributes,
    HtmlTokenizerState::RcdataSelfClosingEndTag,
    HtmlTokenizerState::Rawtext,
    HtmlTokenizerState::RawtextLessThanSign,
    HtmlTokenizerState::RawtextEndTagOpen,
    HtmlTokenizerState::RawtextEndTagName,
    HtmlTokenizerState::RawtextEndTagWhitespace,
    HtmlTokenizerState::RawtextEndTagAttributes,
    HtmlTokenizerState::RawtextSelfClosingEndTag,
    HtmlTokenizerState::Plaintext,
    HtmlTokenizerState::CdataSection,
    HtmlTokenizerState::CdataSectionBracket,
    HtmlTokenizerState::CdataSectionEnd,
    HtmlTokenizerState::CommentStart,
    HtmlTokenizerState::CommentStartDash,
    HtmlTokenizerState::Comment,
    HtmlTokenizerState::CommentLessThanSign,
    HtmlTokenizerState::CommentLessThanSignBang,
    HtmlTokenizerState::CommentLessThanSignBangDash,
    HtmlTokenizerState::CommentLessThanSignBangDashDash,
    HtmlTokenizerState::CommentEndDash,
    HtmlTokenizerState::CommentEnd,
    HtmlTokenizerState::CommentEndBang,
    HtmlTokenizerState::BogusComment,
    HtmlTokenizerState::ScriptData,
    HtmlTokenizerState::ScriptDataLessThanSign,
    HtmlTokenizerState::ScriptDataEndTagOpen,
    HtmlTokenizerState::ScriptDataEndTagName,
    HtmlTokenizerState::ScriptDataEndTagWhitespace,
    HtmlTokenizerState::ScriptDataEndTagAttributes,
    HtmlTokenizerState::ScriptDataSelfClosingEndTag,
    HtmlTokenizerState::ScriptDataEscapeStart,
    HtmlTokenizerState::ScriptDataEscapeStartDash,
    HtmlTokenizerState::ScriptDataEscaped,
    HtmlTokenizerState::ScriptDataEscapedDash,
    HtmlTokenizerState::ScriptDataEscapedDashDash,
    HtmlTokenizerState::ScriptDataEscapedLessThanSign,
    HtmlTokenizerState::ScriptDataEscapedEndTagOpen,
    HtmlTokenizerState::ScriptDataEscapedEndTagName,
    HtmlTokenizerState::ScriptDataEscapedEndTagWhitespace,
    HtmlTokenizerState::ScriptDataEscapedEndTagAttributes,
    HtmlTokenizerState::ScriptDataEscapedSelfClosingEndTag,
    HtmlTokenizerState::ScriptDataDoubleEscapeStart,
    HtmlTokenizerState::ScriptDataDoubleEscaped,
    HtmlTokenizerState::ScriptDataDoubleEscapedDash,
    HtmlTokenizerState::ScriptDataDoubleEscapedDashDash,
    HtmlTokenizerState::ScriptDataDoubleEscapedLessThanSign,
    HtmlTokenizerState::ScriptDataDoubleEscapeEnd,
];

/// Tokenizer states that are valid script-substate entry points.
pub const HTML_SCRIPT_TOKENIZER_STATES: [HtmlTokenizerState; 24] = [
    HtmlTokenizerState::ScriptData,
    HtmlTokenizerState::ScriptDataLessThanSign,
    HtmlTokenizerState::ScriptDataEndTagOpen,
    HtmlTokenizerState::ScriptDataEndTagName,
    HtmlTokenizerState::ScriptDataEndTagWhitespace,
    HtmlTokenizerState::ScriptDataEndTagAttributes,
    HtmlTokenizerState::ScriptDataSelfClosingEndTag,
    HtmlTokenizerState::ScriptDataEscapeStart,
    HtmlTokenizerState::ScriptDataEscapeStartDash,
    HtmlTokenizerState::ScriptDataEscaped,
    HtmlTokenizerState::ScriptDataEscapedDash,
    HtmlTokenizerState::ScriptDataEscapedDashDash,
    HtmlTokenizerState::ScriptDataEscapedLessThanSign,
    HtmlTokenizerState::ScriptDataEscapedEndTagOpen,
    HtmlTokenizerState::ScriptDataEscapedEndTagName,
    HtmlTokenizerState::ScriptDataEscapedEndTagWhitespace,
    HtmlTokenizerState::ScriptDataEscapedEndTagAttributes,
    HtmlTokenizerState::ScriptDataEscapedSelfClosingEndTag,
    HtmlTokenizerState::ScriptDataDoubleEscapeStart,
    HtmlTokenizerState::ScriptDataDoubleEscaped,
    HtmlTokenizerState::ScriptDataDoubleEscapedDash,
    HtmlTokenizerState::ScriptDataDoubleEscapedDashDash,
    HtmlTokenizerState::ScriptDataDoubleEscapedLessThanSign,
    HtmlTokenizerState::ScriptDataDoubleEscapeEnd,
];

impl HtmlTokenizerState {
    /// Machine-state identifier used by the generated static lexer.
    pub fn as_machine_state(self) -> &'static str {
        match self {
            Self::Data => "data",
            Self::Rcdata => "rcdata",
            Self::RcdataLessThanSign => "rcdata_less_than_sign",
            Self::RcdataEndTagOpen => "rcdata_end_tag_open",
            Self::RcdataEndTagName => "rcdata_end_tag_name",
            Self::RcdataEndTagWhitespace => "rcdata_end_tag_whitespace",
            Self::RcdataEndTagAttributes => "rcdata_end_tag_attributes",
            Self::RcdataSelfClosingEndTag => "rcdata_self_closing_end_tag",
            Self::Rawtext => "rawtext",
            Self::RawtextLessThanSign => "rawtext_less_than_sign",
            Self::RawtextEndTagOpen => "rawtext_end_tag_open",
            Self::RawtextEndTagName => "rawtext_end_tag_name",
            Self::RawtextEndTagWhitespace => "rawtext_end_tag_whitespace",
            Self::RawtextEndTagAttributes => "rawtext_end_tag_attributes",
            Self::RawtextSelfClosingEndTag => "rawtext_self_closing_end_tag",
            Self::Plaintext => "plaintext",
            Self::CdataSection => "cdata_section",
            Self::CdataSectionBracket => "cdata_section_bracket",
            Self::CdataSectionEnd => "cdata_section_end",
            Self::CommentStart => "comment_start",
            Self::CommentStartDash => "comment_start_dash",
            Self::Comment => "comment",
            Self::CommentLessThanSign => "comment_less_than_sign",
            Self::CommentLessThanSignBang => "comment_less_than_sign_bang",
            Self::CommentLessThanSignBangDash => "comment_less_than_sign_bang_dash",
            Self::CommentLessThanSignBangDashDash => "comment_less_than_sign_bang_dash_dash",
            Self::CommentEndDash => "comment_end_dash",
            Self::CommentEnd => "comment_end",
            Self::CommentEndBang => "comment_end_bang",
            Self::BogusComment => "bogus_comment",
            Self::ScriptData => "script_data",
            Self::ScriptDataLessThanSign => "script_data_less_than_sign",
            Self::ScriptDataEndTagOpen => "script_data_end_tag_open",
            Self::ScriptDataEndTagName => "script_data_end_tag_name",
            Self::ScriptDataEndTagWhitespace => "script_data_end_tag_whitespace",
            Self::ScriptDataEndTagAttributes => "script_data_end_tag_attributes",
            Self::ScriptDataSelfClosingEndTag => "script_data_self_closing_end_tag",
            Self::ScriptDataEscapeStart => "script_data_escape_start",
            Self::ScriptDataEscapeStartDash => "script_data_escape_start_dash",
            Self::ScriptDataEscaped => "script_data_escaped",
            Self::ScriptDataEscapedDash => "script_data_escaped_dash",
            Self::ScriptDataEscapedDashDash => "script_data_escaped_dash_dash",
            Self::ScriptDataEscapedLessThanSign => "script_data_escaped_less_than_sign",
            Self::ScriptDataEscapedEndTagOpen => "script_data_escaped_end_tag_open",
            Self::ScriptDataEscapedEndTagName => "script_data_escaped_end_tag_name",
            Self::ScriptDataEscapedEndTagWhitespace => "script_data_escaped_end_tag_whitespace",
            Self::ScriptDataEscapedEndTagAttributes => "script_data_escaped_end_tag_attributes",
            Self::ScriptDataEscapedSelfClosingEndTag => "script_data_escaped_self_closing_end_tag",
            Self::ScriptDataDoubleEscapeStart => "script_data_double_escape_start",
            Self::ScriptDataDoubleEscaped => "script_data_double_escaped",
            Self::ScriptDataDoubleEscapedDash => "script_data_double_escaped_dash",
            Self::ScriptDataDoubleEscapedDashDash => "script_data_double_escaped_dash_dash",
            Self::ScriptDataDoubleEscapedLessThanSign => {
                "script_data_double_escaped_less_than_sign"
            }
            Self::ScriptDataDoubleEscapeEnd => "script_data_double_escape_end",
        }
    }

    /// html5lib tokenizer fixture state label for this parser-facing state.
    pub fn as_html5lib_state(self) -> &'static str {
        match self {
            Self::Data => "Data state",
            Self::Rcdata => "RCDATA state",
            Self::RcdataLessThanSign => "RCDATA less-than sign state",
            Self::RcdataEndTagOpen => "RCDATA end tag open state",
            Self::RcdataEndTagName => "RCDATA end tag name state",
            Self::RcdataEndTagWhitespace => "RCDATA end tag whitespace state",
            Self::RcdataEndTagAttributes => "RCDATA end tag attributes state",
            Self::RcdataSelfClosingEndTag => "RCDATA self-closing end tag state",
            Self::Rawtext => "RAWTEXT state",
            Self::RawtextLessThanSign => "RAWTEXT less-than sign state",
            Self::RawtextEndTagOpen => "RAWTEXT end tag open state",
            Self::RawtextEndTagName => "RAWTEXT end tag name state",
            Self::RawtextEndTagWhitespace => "RAWTEXT end tag whitespace state",
            Self::RawtextEndTagAttributes => "RAWTEXT end tag attributes state",
            Self::RawtextSelfClosingEndTag => "RAWTEXT self-closing end tag state",
            Self::Plaintext => "PLAINTEXT state",
            Self::CdataSection => "CDATA section state",
            Self::CdataSectionBracket => "CDATA section bracket state",
            Self::CdataSectionEnd => "CDATA section end state",
            Self::CommentStart => "Comment start state",
            Self::CommentStartDash => "Comment start dash state",
            Self::Comment => "Comment state",
            Self::CommentLessThanSign => "Comment less-than sign state",
            Self::CommentLessThanSignBang => "Comment less-than sign bang state",
            Self::CommentLessThanSignBangDash => "Comment less-than sign bang dash state",
            Self::CommentLessThanSignBangDashDash => "Comment less-than sign bang dash dash state",
            Self::CommentEndDash => "Comment end dash state",
            Self::CommentEnd => "Comment end state",
            Self::CommentEndBang => "Comment end bang state",
            Self::BogusComment => "Bogus comment state",
            Self::ScriptData => "Script data state",
            Self::ScriptDataLessThanSign => "Script data less-than sign state",
            Self::ScriptDataEndTagOpen => "Script data end tag open state",
            Self::ScriptDataEndTagName => "Script data end tag name state",
            Self::ScriptDataEndTagWhitespace => "Script data end tag whitespace state",
            Self::ScriptDataEndTagAttributes => "Script data end tag attributes state",
            Self::ScriptDataSelfClosingEndTag => "Script data self-closing end tag state",
            Self::ScriptDataEscapeStart => "Script data escape start state",
            Self::ScriptDataEscapeStartDash => "Script data escape start dash state",
            Self::ScriptDataEscaped => "Script data escaped state",
            Self::ScriptDataEscapedDash => "Script data escaped dash state",
            Self::ScriptDataEscapedDashDash => "Script data escaped dash dash state",
            Self::ScriptDataEscapedLessThanSign => "Script data escaped less-than sign state",
            Self::ScriptDataEscapedEndTagOpen => "Script data escaped end tag open state",
            Self::ScriptDataEscapedEndTagName => "Script data escaped end tag name state",
            Self::ScriptDataEscapedEndTagWhitespace => {
                "Script data escaped end tag whitespace state"
            }
            Self::ScriptDataEscapedEndTagAttributes => {
                "Script data escaped end tag attributes state"
            }
            Self::ScriptDataEscapedSelfClosingEndTag => {
                "Script data escaped self-closing end tag state"
            }
            Self::ScriptDataDoubleEscapeStart => "Script data double escape start state",
            Self::ScriptDataDoubleEscaped => "Script data double escaped state",
            Self::ScriptDataDoubleEscapedDash => "Script data double escaped dash state",
            Self::ScriptDataDoubleEscapedDashDash => "Script data double escaped dash dash state",
            Self::ScriptDataDoubleEscapedLessThanSign => {
                "Script data double escaped less-than sign state"
            }
            Self::ScriptDataDoubleEscapeEnd => "Script data double escape end state",
        }
    }

    /// Return the typed tokenizer state for a generated machine-state identifier.
    pub fn from_machine_state(machine_state: &str) -> Option<Self> {
        HTML_TOKENIZER_STATES
            .iter()
            .copied()
            .find(|state| state.as_machine_state() == machine_state)
    }

    /// Return the typed tokenizer state for a standard html5lib fixture label.
    pub fn from_html5lib_state(html5lib_state: &str) -> Option<Self> {
        HTML_TOKENIZER_STATES
            .iter()
            .copied()
            .find(|state| state.as_html5lib_state() == html5lib_state)
    }

    /// Return the typed fragment state for a generated machine-state identifier.
    pub fn from_fragment_machine_state(machine_state: &str) -> Option<Self> {
        Self::from_machine_state(machine_state).filter(|state| state.is_fragment_state())
    }

    /// Return whether this state is a parser-approved script tokenizer substate.
    pub fn is_script_substate(self) -> bool {
        HTML_SCRIPT_TOKENIZER_STATES.contains(&self)
    }

    /// Return whether this state is a parser-approved fragment entry point.
    pub fn is_fragment_state(self) -> bool {
        HTML_FRAGMENT_TOKENIZER_STATES.contains(&self)
    }

    /// Return whether this state resumes an already-started end tag.
    pub fn requires_end_tag_seed(self) -> bool {
        matches!(
            self,
            Self::RcdataEndTagName
                | Self::RcdataEndTagWhitespace
                | Self::RcdataEndTagAttributes
                | Self::RcdataSelfClosingEndTag
                | Self::RawtextEndTagName
                | Self::RawtextEndTagWhitespace
                | Self::RawtextEndTagAttributes
                | Self::RawtextSelfClosingEndTag
                | Self::ScriptDataEndTagName
                | Self::ScriptDataEndTagWhitespace
                | Self::ScriptDataEndTagAttributes
                | Self::ScriptDataSelfClosingEndTag
                | Self::ScriptDataEscapedEndTagName
                | Self::ScriptDataEscapedEndTagWhitespace
                | Self::ScriptDataEscapedEndTagAttributes
                | Self::ScriptDataEscapedSelfClosingEndTag
        )
    }

    /// Return whether this state resumes an already-started comment token.
    pub fn requires_comment_seed(self) -> bool {
        matches!(
            self,
            Self::CommentStart
                | Self::CommentStartDash
                | Self::Comment
                | Self::CommentLessThanSign
                | Self::CommentLessThanSignBang
                | Self::CommentLessThanSignBangDash
                | Self::CommentLessThanSignBangDashDash
                | Self::CommentEndDash
                | Self::CommentEnd
                | Self::CommentEndBang
                | Self::BogusComment
        )
    }

    /// Return whether a seeded state needs the parser's last-start-tag context.
    pub fn requires_last_start_tag(self) -> bool {
        matches!(
            self,
            Self::Rcdata
                | Self::RcdataLessThanSign
                | Self::RcdataEndTagOpen
                | Self::RcdataEndTagName
                | Self::RcdataEndTagWhitespace
                | Self::RcdataEndTagAttributes
                | Self::RcdataSelfClosingEndTag
                | Self::Rawtext
                | Self::RawtextLessThanSign
                | Self::RawtextEndTagOpen
                | Self::RawtextEndTagName
                | Self::RawtextEndTagWhitespace
                | Self::RawtextEndTagAttributes
                | Self::RawtextSelfClosingEndTag
        ) || self.is_script_substate()
    }
}

/// Initial tokenizer context for fragment or parser-controlled lexing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlLexContext {
    pub initial_state: HtmlTokenizerState,
    pub last_start_tag: Option<String>,
    pub current_end_tag: Option<String>,
    pub current_comment: Option<String>,
    pub temporary_buffer: Option<String>,
}

impl HtmlLexContext {
    pub fn new(initial_state: HtmlTokenizerState) -> Self {
        Self {
            initial_state,
            last_start_tag: None,
            current_end_tag: None,
            current_comment: None,
            temporary_buffer: None,
        }
    }

    pub fn data() -> Self {
        Self::new(HtmlTokenizerState::Data)
    }

    /// Return the tokenizer context for parser-approved foreign-content CDATA.
    ///
    /// HTML data-state markup only treats `<![CDATA[` specially when the parser
    /// has entered a foreign-content integration point such as SVG or MathML.
    /// Keeping this as an explicit context prevents element-name mapping from
    /// pretending CDATA is valid in ordinary HTML content.
    pub fn cdata_section() -> Self {
        Self::new(HtmlTokenizerState::CdataSection)
    }

    /// Return a script tokenizer substate context for parser-approved fragments.
    ///
    /// These substates are useful for html5lib/WPT-style fixtures and for
    /// future parser flows that need to resume script tokenization after
    /// already recognizing an escaped or double-escaped script section.
    pub fn script_substate(initial_state: HtmlTokenizerState) -> Option<Self> {
        if initial_state.is_script_substate() {
            Some(Self::new(initial_state).with_last_start_tag("script"))
        } else {
            None
        }
    }

    pub fn with_last_start_tag(mut self, tag: impl Into<String>) -> Self {
        self.last_start_tag = Some(tag.into());
        self
    }

    pub fn with_current_end_tag(mut self, tag: impl Into<String>) -> Self {
        self.current_end_tag = Some(tag.into());
        self
    }

    pub fn with_current_comment(mut self, data: impl Into<String>) -> Self {
        self.current_comment = Some(data.into());
        self
    }

    pub fn with_temporary_buffer(mut self, value: impl Into<String>) -> Self {
        self.temporary_buffer = Some(value.into());
        self
    }

    pub fn is_data(&self) -> bool {
        self.initial_state == HtmlTokenizerState::Data
            && self.last_start_tag.is_none()
            && self.current_end_tag.is_none()
            && self.current_comment.is_none()
            && self.temporary_buffer.is_none()
    }

    /// Return a comment tokenizer continuation context for importer/parser tests.
    ///
    /// These states resume with an already-created comment token. States such as
    /// `comment_end_dash` and `comment_end` encode pending dash delimiters in
    /// the tokenizer state itself, so the seed only contains already-committed
    /// comment data.
    pub fn comment_continuation(
        initial_state: HtmlTokenizerState,
        data: impl Into<String>,
    ) -> Option<Self> {
        if initial_state.requires_comment_seed() {
            Some(Self::new(initial_state).with_current_comment(data))
        } else {
            None
        }
    }

    /// Return the tokenizer context used for text following a start tag.
    ///
    /// This is the parser-facing map from element names to HTML tokenizer
    /// submodes. It deliberately keeps foreign-content CDATA decisions out of
    /// the element map because those depend on tree-construction context.
    pub fn for_element_text(element_name: &str) -> Option<Self> {
        Self::for_element_text_with_scripting(element_name, HtmlScriptingMode::Enabled)
    }

    /// Return the tokenizer context used for text after a start tag, including
    /// scripting-sensitive `noscript` handling.
    pub fn for_element_text_with_scripting(
        element_name: &str,
        scripting: HtmlScriptingMode,
    ) -> Option<Self> {
        let name = element_name.to_ascii_lowercase();
        let state = match name.as_str() {
            "title" | "textarea" => HtmlTokenizerState::Rcdata,
            "iframe" | "noembed" | "noframes" | "style" | "xmp" => HtmlTokenizerState::Rawtext,
            "noscript" if scripting == HtmlScriptingMode::Enabled => HtmlTokenizerState::Rawtext,
            "script" => HtmlTokenizerState::ScriptData,
            "plaintext" => HtmlTokenizerState::Plaintext,
            _ => return None,
        };

        let context = Self::new(state);
        if state == HtmlTokenizerState::Plaintext {
            Some(context)
        } else {
            Some(context.with_last_start_tag(name))
        }
    }
}

/// Return the generated typed definition for the HTML 1.x compatibility-floor lexer.
pub fn html1_definition() -> StateMachineDefinition {
    generated_html1::html1_lexer_definition()
}

/// Build the statically linked HTML 1.x compatibility-floor lexer machine.
pub fn html1_machine() -> std::result::Result<EffectfulStateMachine, String> {
    generated_html1::html1_lexer_transducer()
}

/// Return the generated typed definition for the current HTML lexer skeleton.
pub fn html_skeleton_definition() -> StateMachineDefinition {
    generated_html_skeleton::html_skeleton_lexer_definition()
}

/// Build the first statically linked HTML lexer skeleton.
pub fn html_skeleton_machine() -> std::result::Result<EffectfulStateMachine, String> {
    generated_html_skeleton::html_skeleton_lexer_transducer()
}

/// Build a Rust HTML lexer over the statically linked HTML 1.x compatibility floor.
pub fn create_html_lexer() -> Result<HtmlLexer> {
    html1_machine()
        .map(|machine| HtmlLexer::new(machine).with_normalized_carriage_returns())
        .map_err(TokenizerError::Machine)
}

/// Build a lexer seeded with a parser-controlled HTML tokenizer context.
pub fn create_html_lexer_with_context(context: &HtmlLexContext) -> Result<HtmlLexer> {
    let mut lexer = create_html_lexer()?;
    apply_html_lex_context(&mut lexer, context)?;
    Ok(lexer)
}

/// Move an existing lexer into a parser-controlled HTML tokenizer context.
pub fn apply_html_lex_context(lexer: &mut HtmlLexer, context: &HtmlLexContext) -> Result<()> {
    lexer.set_initial_state(context.initial_state.as_machine_state())?;
    if let Some(last_start_tag) = context.last_start_tag.as_deref() {
        lexer.set_last_start_tag(last_start_tag);
    } else {
        lexer.clear_last_start_tag();
    }
    if let Some(current_end_tag) = context.current_end_tag.as_deref() {
        lexer.set_current_end_tag(current_end_tag);
    } else if let Some(current_comment) = context.current_comment.as_deref() {
        lexer.set_current_comment(current_comment);
    } else {
        lexer.clear_current_token();
    }
    if let Some(temporary_buffer) = context.temporary_buffer.as_deref() {
        lexer.set_temporary_buffer(temporary_buffer);
    } else {
        lexer.clear_temporary_buffer();
    }
    Ok(())
}

/// Lex one complete HTML string with the current compatibility-floor machine.
pub fn lex_html(source: &str) -> Result<Vec<Token>> {
    let mut lexer = create_html_lexer()?;
    lexer.push(source)?;
    lexer.finish()?;
    Ok(lexer.drain_tokens())
}

/// Lex a parser-controlled HTML fragment with an explicit tokenizer context.
pub fn lex_html_fragment(source: &str, context: &HtmlLexContext) -> Result<Vec<Token>> {
    let mut lexer = create_html_lexer_with_context(context)?;
    lexer.push(source)?;
    lexer.finish()?;
    Ok(lexer.drain_tokens())
}
