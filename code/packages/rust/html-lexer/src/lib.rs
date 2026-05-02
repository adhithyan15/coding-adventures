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
    Rawtext,
    Plaintext,
    CdataSection,
    ScriptData,
    ScriptDataEscaped,
    ScriptDataEscapedDash,
    ScriptDataEscapedDashDash,
    ScriptDataEscapedLessThanSign,
    ScriptDataDoubleEscaped,
    ScriptDataDoubleEscapedDash,
    ScriptDataDoubleEscapedDashDash,
    ScriptDataDoubleEscapedLessThanSign,
}

impl HtmlTokenizerState {
    /// Machine-state identifier used by the generated static lexer.
    pub fn as_machine_state(self) -> &'static str {
        match self {
            Self::Data => "data",
            Self::Rcdata => "rcdata",
            Self::Rawtext => "rawtext",
            Self::Plaintext => "plaintext",
            Self::CdataSection => "cdata_section",
            Self::ScriptData => "script_data",
            Self::ScriptDataEscaped => "script_data_escaped",
            Self::ScriptDataEscapedDash => "script_data_escaped_dash",
            Self::ScriptDataEscapedDashDash => "script_data_escaped_dash_dash",
            Self::ScriptDataEscapedLessThanSign => "script_data_escaped_less_than_sign",
            Self::ScriptDataDoubleEscaped => "script_data_double_escaped",
            Self::ScriptDataDoubleEscapedDash => "script_data_double_escaped_dash",
            Self::ScriptDataDoubleEscapedDashDash => "script_data_double_escaped_dash_dash",
            Self::ScriptDataDoubleEscapedLessThanSign => {
                "script_data_double_escaped_less_than_sign"
            }
        }
    }
}

/// Initial tokenizer context for fragment or parser-controlled lexing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlLexContext {
    pub initial_state: HtmlTokenizerState,
    pub last_start_tag: Option<String>,
}

impl HtmlLexContext {
    pub fn new(initial_state: HtmlTokenizerState) -> Self {
        Self {
            initial_state,
            last_start_tag: None,
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

    pub fn with_last_start_tag(mut self, tag: impl Into<String>) -> Self {
        self.last_start_tag = Some(tag.into());
        self
    }

    pub fn is_data(&self) -> bool {
        self.initial_state == HtmlTokenizerState::Data && self.last_start_tag.is_none()
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
