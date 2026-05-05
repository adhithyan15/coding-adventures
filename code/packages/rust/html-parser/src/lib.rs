//! Incremental HTML parser for Venture.
//!
//! This first slice builds a DOM tree from the current Rust HTML lexer tokens.
//! It deliberately starts with a small tree-construction core instead of
//! pretending HTML is context-free. Future batches can add the full WHATWG
//! insertion-mode machinery on top of this DOM target.

use coding_adventures_html_lexer::{
    apply_html_lex_context, create_html_lexer, Attribute as LexerAttribute, Diagnostic,
    HtmlLexContext, HtmlLexer, HtmlScriptingMode, HtmlTokenizerState, Token, TokenizerError,
};
use dom_core::{Attribute, Document, DocumentType, Node};
use std::fmt;

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
    ForeignContentCdataSection,
    ForeignContentCdataSectionBracket,
    ForeignContentCdataSectionEnd,
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
            Self::ForeignContentCdataSection => HtmlLexContext::cdata_section(),
            Self::ForeignContentCdataSectionBracket => {
                HtmlLexContext::new(HtmlTokenizerState::CdataSectionBracket)
            }
            Self::ForeignContentCdataSectionEnd => {
                HtmlLexContext::new(HtmlTokenizerState::CdataSectionEnd)
            }
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

/// Parse a complete HTML string into a DOM document plus lexer/parser diagnostics.
pub fn parse_html_with_diagnostics(source: &str) -> Result<ParseOutput, ParseError> {
    parse_html_with_diagnostics_and_options(source, HtmlParseOptions::default())
}

/// Parse a complete HTML string into a DOM document with explicit parser options.
pub fn parse_html_with_options(
    source: &str,
    options: HtmlParseOptions,
) -> Result<Document, ParseError> {
    Ok(parse_html_with_diagnostics_and_options(source, options)?.document)
}

/// Parse a complete HTML string into a DOM document plus diagnostics with explicit parser options.
pub fn parse_html_with_diagnostics_and_options(
    source: &str,
    options: HtmlParseOptions,
) -> Result<ParseOutput, ParseError> {
    let mut lexer = create_html_lexer()?;
    apply_html_lex_context(&mut lexer, &options.initial_tokenizer_context.lex_context())?;
    let mut parser = HtmlParser::with_options(options);

    for ch in source.chars() {
        let mut buffer = [0; 4];
        lexer.push(ch.encode_utf8(&mut buffer))?;
        drain_parser_tokens(&mut lexer, &mut parser)?;
    }

    lexer.finish()?;
    drain_parser_tokens(&mut lexer, &mut parser)?;

    let lexer_diagnostics = lexer.diagnostics().to_vec();
    let document = parser.finish_document();

    Ok(ParseOutput {
        document,
        lexer_diagnostics,
        parser_diagnostics: parser.diagnostics,
    })
}

/// Streaming-friendly parser core over already-tokenized HTML.
#[derive(Debug, Default)]
pub struct HtmlParser {
    document: Document,
    open_elements: Vec<Vec<usize>>,
    diagnostics: Vec<ParserDiagnostic>,
    options: HtmlParseOptions,
    strip_next_leading_lf: bool,
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

    pub fn parse_tokens(&mut self, tokens: impl IntoIterator<Item = Token>) -> Document {
        for token in tokens {
            self.process_token(token);
        }
        self.finish_document()
    }

    pub fn diagnostics(&self) -> &[ParserDiagnostic] {
        &self.diagnostics
    }

    fn finish_document(&mut self) -> Document {
        normalize_document_shell(std::mem::take(&mut self.document))
    }

    fn process_token(&mut self, token: Token) {
        match token {
            Token::Text(text) => self.append_text(text),
            token => {
                self.strip_next_leading_lf = false;
                match token {
                    Token::StartTag {
                        name,
                        attributes,
                        self_closing,
                    } => self.append_start_tag(name, attributes, self_closing),
                    Token::EndTag { name } => self.handle_end_tag(&name),
                    Token::Comment(comment) => {
                        self.append_node(Node::comment(comment));
                    }
                    Token::Doctype {
                        name,
                        public_identifier,
                        system_identifier,
                        force_quirks,
                    } => {
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

    fn append_start_tag(
        &mut self,
        name: String,
        attributes: Vec<LexerAttribute>,
        self_closing: bool,
    ) {
        self.apply_document_shell_implied_contexts(&name);
        self.apply_table_implied_contexts(&name);
        self.apply_simple_implied_end_tags(&name);
        if self.apply_interactive_implied_contexts(&name) {
            return;
        }

        let attributes: Vec<Attribute> = attributes
            .into_iter()
            .map(|attribute| Attribute {
                name: attribute.name,
                value: attribute.value,
            })
            .collect();

        if name == "html" && self.merge_attributes_into_open_element("html", &attributes) {
            return;
        }

        if name == "head" && self.has_open_element("head") {
            self.merge_attributes_into_open_element("head", &attributes);
            return;
        }

        if name == "head" && self.has_open_element("body") {
            self.diagnostics.push(ParserDiagnostic::new(
                "unexpected-head-start-tag",
                "head start tag was ignored after body content had already started",
            ));
            return;
        }

        if name == "body" && self.merge_attributes_into_open_element("body", &attributes) {
            return;
        }

        let acknowledges_self_closing = self_closing && is_void_element(&name);
        if self_closing && !acknowledges_self_closing {
            self.diagnostics.push(ParserDiagnostic::new(
                "non-void-html-element-self-closing",
                format!("self-closing flag on non-void HTML element `<{name}>` was ignored"),
            ));
        }

        let child_index = self.append_node(Node::element(name.clone(), attributes));

        if !acknowledges_self_closing && !is_void_element(&name) {
            let mut path = self.current_parent_path().to_vec();
            path.push(child_index);
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

        let text = if self.strip_next_leading_lf {
            self.strip_next_leading_lf = false;
            text.strip_prefix('\n').unwrap_or(&text).to_string()
        } else {
            text
        };
        if text.is_empty() {
            return;
        }

        if !text.chars().all(char::is_whitespace) {
            self.pop_current_if(|name| name == "head");
        }

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

    fn append_node(&mut self, node: Node) -> usize {
        if let Some(children) = self.current_children_mut() {
            children.push(node);
            children.len() - 1
        } else {
            self.document.push_child(node);
            self.document.children.len() - 1
        }
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

    fn apply_document_shell_implied_contexts(&mut self, incoming_name: &str) {
        if starts_body_after_head(incoming_name) {
            self.pop_current_if(|name| name == "head");
        }
    }

    fn handle_end_tag(&mut self, name: &str) {
        match name {
            "head" if !self.has_open_element("head") && !self.has_open_element("body") => {
                self.strip_next_leading_lf = false;
            }
            "body" if !self.has_open_element("body") && !self.open_elements.is_empty() => {
                self.open_elements.clear();
            }
            "br" => {
                self.diagnostics.push(ParserDiagnostic::new(
                    "unexpected-br-end-tag",
                    "end tag `</br>` was recovered as a `br` start tag",
                ));
                self.append_start_tag("br".to_string(), Vec::new(), true);
            }
            name if is_void_element(name) => {
                self.diagnostics.push(ParserDiagnostic::new(
                    "unexpected-void-end-tag",
                    format!("end tag `</{name}>` for a void element was ignored"),
                ));
            }
            "p" if !self.has_open_element("p") => {
                self.diagnostics.push(ParserDiagnostic::new(
                    "unexpected-p-end-tag",
                    "end tag `</p>` created and closed an implied `p` element",
                ));
                self.append_start_tag("p".to_string(), Vec::new(), false);
                self.close_element("p");
            }
            "html" => {
                if self.has_open_element("html") {
                    self.pop_current_if(|current| current == "body");
                    self.close_element(name);
                } else if !self.open_elements.is_empty() {
                    self.open_elements.clear();
                } else {
                    self.close_element(name);
                }
            }
            _ => self.close_element(name),
        }
    }

    fn close_element(&mut self, name: &str) {
        if let Some(index) = self
            .open_elements
            .iter()
            .rposition(|path| element_at_path(&self.document, path).is_some_and(|n| n == name))
        {
            self.open_elements.truncate(index);
            return;
        }

        self.diagnostics.push(ParserDiagnostic::new(
            "unexpected-end-tag",
            format!("end tag `</{name}>` did not match an open element"),
        ));
    }

    fn apply_table_implied_contexts(&mut self, incoming_name: &str) {
        match incoming_name {
            "caption" | "colgroup" => {
                self.pop_table_cell_row_and_section_contexts();
                self.close_open_element_if(|name| name == "caption" || name == "colgroup");
            }
            "tbody" | "thead" | "tfoot" => {
                self.pop_table_cell_row_and_section_contexts();
                self.close_open_element_if(|name| name == "caption" || name == "colgroup");
            }
            "col" => {
                self.close_open_element_if(|name| name == "caption");
                if self.current_element_is("table") {
                    self.append_implied_element("colgroup");
                }
            }
            "tr" => {
                self.close_open_element_if(|name| name == "td" || name == "th");
                self.close_open_element_if(|name| name == "tr");
                self.close_open_element_if(|name| name == "caption" || name == "colgroup");
                if self.current_element_is("table") {
                    self.append_implied_element("tbody");
                }
            }
            "td" | "th" => {
                self.close_open_element_if(|name| name == "td" || name == "th");
                self.close_open_element_if(|name| name == "caption" || name == "colgroup");
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
        self.close_open_element_if(|name| name == "td" || name == "th");
        self.close_open_element_if(|name| name == "tr");
        self.close_open_element_if(is_table_section);
    }

    fn apply_simple_implied_end_tags(&mut self, incoming_name: &str) {
        if incoming_name == "p" {
            self.close_open_element_if(|name| name == "p");
        } else if incoming_name == "li" {
            self.close_open_element_if(|name| name == "li");
        } else if incoming_name == "dt" || incoming_name == "dd" {
            self.close_open_element_if(|name| name == "dt" || name == "dd");
        } else if incoming_name == "option" {
            self.close_open_element_if(|name| name == "option");
        } else if incoming_name == "optgroup" {
            self.close_open_element_if(|name| name == "option");
            self.close_open_element_if(|name| name == "optgroup");
        } else if incoming_name == "rb" {
            self.close_open_element_if(is_ruby_annotation_element);
            self.close_open_element_if(|name| name == "rtc");
        } else if incoming_name == "rt" || incoming_name == "rp" {
            self.close_open_element_if(|name| name == "rb" || name == "rt" || name == "rp");
        } else if incoming_name == "rtc" {
            self.close_open_element_if(|name| name == "rb" || name == "rt" || name == "rp");
            self.close_open_element_if(|name| name == "rtc");
        } else if is_heading_element(incoming_name) {
            self.close_open_element_if(|name| name == "p");
            self.close_open_element_if(is_heading_element);
        } else if is_paragraph_boundary_element(incoming_name) {
            self.close_open_element_if(|name| name == "p");
        }
    }

    fn apply_interactive_implied_contexts(&mut self, incoming_name: &str) -> bool {
        match incoming_name {
            "a" => {
                self.close_open_element_silently("a");
                false
            }
            "button" => {
                self.close_open_element_silently("button");
                false
            }
            "nobr" => {
                self.close_open_element_silently("nobr");
                false
            }
            "form" if self.has_open_element("form") => {
                self.diagnostics.push(ParserDiagnostic::new(
                    "nested-form-start-tag",
                    "nested form start tag was ignored while a form element was already open",
                ));
                true
            }
            _ => false,
        }
    }

    fn close_open_element_silently(&mut self, name: &str) -> bool {
        self.close_open_element_if(|candidate| candidate == name)
    }

    fn close_open_element_if(&mut self, predicate: impl Fn(&str) -> bool) -> bool {
        let Some(index) = self.open_elements.iter().rposition(|path| {
            element_at_path(&self.document, path).is_some_and(|name| predicate(name))
        }) else {
            return false;
        };
        self.open_elements.truncate(index);
        true
    }

    fn has_open_element(&self, name: &str) -> bool {
        self.open_elements
            .iter()
            .any(|path| element_at_path(&self.document, path).is_some_and(|n| n == name))
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
            .is_some_and(|current| current == name)
    }

    fn current_element_name(&self) -> Option<&str> {
        let path = self.open_elements.last()?;
        element_at_path(&self.document, path)
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

    fn text_context_for_token(&self, token: &Token) -> Option<HtmlLexContext> {
        match token {
            Token::StartTag { name, .. } if !is_void_element(name) => {
                HtmlLexContext::for_element_text_with_scripting(name, self.options.scripting)
            }
            Token::EndTag { .. } => Some(HtmlLexContext::data()),
            _ => None,
        }
    }
}

fn drain_parser_tokens(lexer: &mut HtmlLexer, parser: &mut HtmlParser) -> Result<(), ParseError> {
    for token in lexer.drain_tokens() {
        let next_context = parser.text_context_for_token(&token);
        parser.process_token(token);

        if let Some(context) = next_context {
            apply_html_lex_context(lexer, &context)?;
        }
    }

    Ok(())
}

fn element_at_path<'a>(document: &'a Document, path: &[usize]) -> Option<&'a str> {
    let mut nodes = document.children.as_slice();
    let mut current = None;

    for index in path {
        let node = nodes.get(*index)?;
        match node {
            Node::Element(element) => {
                current = Some(element.name.as_str());
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

fn normalize_document_shell(document: Document) -> Document {
    let mut normalized = Document::new();
    let mut builder = DocumentShellBuilder::default();

    for node in document.children {
        match node {
            Node::DocumentType(_) => normalized.push_child(node),
            Node::Comment(_) if !builder.seen_document_element => normalized.push_child(node),
            Node::Element(mut element) if element.name == "html" => {
                builder.seen_document_element = true;
                builder.html_attributes.extend(element.attributes);
                for child in element.children.drain(..) {
                    builder.push_html_child(child);
                }
            }
            node => {
                builder.seen_document_element = true;
                builder.push_html_child(node);
            }
        }
    }

    normalized.push_child(builder.finish());
    normalized
}

#[derive(Debug, Default)]
struct DocumentShellBuilder {
    seen_document_element: bool,
    seen_body_content: bool,
    html_attributes: Vec<Attribute>,
    head_attributes: Vec<Attribute>,
    body_attributes: Vec<Attribute>,
    head_children: Vec<Node>,
    body_children: Vec<Node>,
}

impl DocumentShellBuilder {
    fn push_html_child(&mut self, node: Node) {
        match node {
            Node::Element(mut element) if element.name == "head" => {
                self.head_attributes.extend(element.attributes);
                self.head_children.append(&mut element.children);
            }
            Node::Element(mut element) if element.name == "body" => {
                self.seen_body_content = true;
                self.body_attributes.extend(element.attributes);
                self.body_children.append(&mut element.children);
            }
            Node::Element(element)
                if !self.seen_body_content && is_head_element(element.name.as_str()) =>
            {
                self.head_children.push(Node::Element(element));
            }
            node => {
                if !is_ignorable_before_body(&node) {
                    self.seen_body_content = true;
                }
                self.body_children.push(node);
            }
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

        let Node::Element(ref mut html_element) = html else {
            unreachable!("Node::element always returns an element")
        };
        html_element.children.push(Node::Element(head));
        html_element.children.push(Node::Element(body));
        html
    }
}

fn is_head_element(name: &str) -> bool {
    matches!(
        name,
        "base"
            | "basefont"
            | "bgsound"
            | "link"
            | "meta"
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

fn is_table_section(name: &str) -> bool {
    matches!(name, "tbody" | "thead" | "tfoot")
}

fn is_heading_element(name: &str) -> bool {
    matches!(name, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

fn is_ruby_annotation_element(name: &str) -> bool {
    matches!(name, "rb" | "rt" | "rp")
}

fn is_paragraph_boundary_element(name: &str) -> bool {
    const PARAGRAPH_BOUNDARY_ELEMENTS: &[&str] = &[
        "address",
        "article",
        "aside",
        "blockquote",
        "button",
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
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
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
        assert_eq!(second_paragraph.children, vec![Node::text("Two")]);

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
    fn closes_paragraph_before_button_and_legacy_block_boundaries() {
        let document = parse_html(
            "<p>Button<button>Click<button>Again</button><p>Centered<center>Block</center><p>Search<search>Find</search><p>Heading<hgroup>Title</hgroup><p>Listing<listing>Block</listing><p>Directory<dir><li>Item",
        )
        .unwrap();

        let body = body(&document);
        assert_eq!(body.children.len(), 13);

        let button_intro = element(&body.children[0]);
        assert_eq!(button_intro.name, "p");
        assert_eq!(button_intro.children, vec![Node::text("Button")]);

        let first_button = element(&body.children[1]);
        assert_eq!(first_button.name, "button");
        assert_eq!(first_button.children, vec![Node::text("Click")]);

        let second_button = element(&body.children[2]);
        assert_eq!(second_button.name, "button");
        assert_eq!(second_button.children, vec![Node::text("Again")]);

        let centered_intro = element(&body.children[3]);
        assert_eq!(centered_intro.name, "p");
        assert_eq!(centered_intro.children, vec![Node::text("Centered")]);

        let center = element(&body.children[4]);
        assert_eq!(center.name, "center");
        assert_eq!(center.children, vec![Node::text("Block")]);

        let search_intro = element(&body.children[5]);
        assert_eq!(search_intro.name, "p");
        assert_eq!(search_intro.children, vec![Node::text("Search")]);

        let search = element(&body.children[6]);
        assert_eq!(search.name, "search");
        assert_eq!(search.children, vec![Node::text("Find")]);

        let heading_intro = element(&body.children[7]);
        assert_eq!(heading_intro.name, "p");
        assert_eq!(heading_intro.children, vec![Node::text("Heading")]);

        let hgroup = element(&body.children[8]);
        assert_eq!(hgroup.name, "hgroup");
        assert_eq!(hgroup.children, vec![Node::text("Title")]);

        let listing_intro = element(&body.children[9]);
        assert_eq!(listing_intro.name, "p");
        assert_eq!(listing_intro.children, vec![Node::text("Listing")]);

        let listing = element(&body.children[10]);
        assert_eq!(listing.name, "listing");
        assert_eq!(listing.children, vec![Node::text("Block")]);

        let directory_intro = element(&body.children[11]);
        assert_eq!(directory_intro.name, "p");
        assert_eq!(directory_intro.children, vec![Node::text("Directory")]);

        let directory = element(&body.children[12]);
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
        assert_eq!(body.children.len(), 2);

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

        let paragraph = element(&body.children[1]);
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
    fn closes_paragraphs_before_block_boundaries() {
        let document = parse_html(
            "<p>Intro<div>Block</div><p>Items<ul><li>One</ul><p>Table<table><tr><td>A</table>",
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
        let fallback_paragraph = element(&disabled_noscript.children[0]);
        assert_eq!(fallback_paragraph.children, vec![Node::text("&")]);
        assert_eq!(
            element(&body(&disabled.document).children[0]).children,
            vec![Node::text("x")]
        );

        for output in [enabled, disabled] {
            assert_eq!(
                output.parser_diagnostics,
                vec![ParserDiagnostic::new(
                    "non-void-html-element-self-closing",
                    "self-closing flag on non-void HTML element `<noscript>` was ignored"
                )]
            );
        }
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

        let fallback_paragraph = element(&noscript.children[0]);
        assert_eq!(fallback_paragraph.name, "p");
        assert_eq!(fallback_paragraph.children, vec![Node::text("&")]);

        let paragraph = element(&body(&document).children[0]);
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
        assert_eq!(explicit_body.children.len(), 3);
        assert_eq!(
            element(&explicit_body.children[0]).children,
            vec![Node::text("x")]
        );
        assert_eq!(explicit_body.children[1], Node::text("y"));
        assert_eq!(explicit_body.children[2], Node::text("z"));
    }
}
