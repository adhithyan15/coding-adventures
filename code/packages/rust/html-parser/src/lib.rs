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
    ForeignContentCdataSection,
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

impl HtmlInitialTokenizerContext {
    fn lex_context(self) -> HtmlLexContext {
        match self {
            Self::Data => HtmlLexContext::data(),
            Self::ForeignContentCdataSection => HtmlLexContext::cdata_section(),
            Self::ScriptData => script_lex_context(HtmlTokenizerState::ScriptData),
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
        }
    }
}

fn script_lex_context(state: HtmlTokenizerState) -> HtmlLexContext {
    HtmlLexContext::script_substate(state).expect("parser only exposes valid script substates")
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
            Token::StartTag {
                name,
                attributes,
                self_closing,
            } => self.append_start_tag(name, attributes, self_closing),
            Token::EndTag { name } => self.close_element(&name),
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
        }
    }

    fn append_start_tag(
        &mut self,
        name: String,
        attributes: Vec<LexerAttribute>,
        self_closing: bool,
    ) {
        self.apply_table_implied_contexts(&name);
        self.apply_simple_implied_end_tags(&name);

        let attributes = attributes
            .into_iter()
            .map(|attribute| Attribute {
                name: attribute.name,
                value: attribute.value,
            })
            .collect();
        let child_index = self.append_node(Node::element(name.clone(), attributes));

        if !self_closing && !is_void_element(&name) {
            let mut path = self.current_parent_path().to_vec();
            path.push(child_index);
            self.open_elements.push(path);
        }
    }

    fn append_text(&mut self, text: String) {
        if text.is_empty() {
            return;
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
                self.pop_current_if(|name| name == "caption" || name == "colgroup");
            }
            "tbody" | "thead" | "tfoot" => {
                self.pop_table_cell_row_and_section_contexts();
                self.pop_current_if(|name| name == "caption" || name == "colgroup");
            }
            "col" => {
                self.pop_current_if(|name| name == "caption");
                if self.current_element_is("table") {
                    self.append_implied_element("colgroup");
                }
            }
            "tr" => {
                self.pop_current_if(|name| name == "td" || name == "th");
                self.pop_current_if(|name| name == "tr");
                self.pop_current_if(|name| name == "caption" || name == "colgroup");
                if self.current_element_is("table") {
                    self.append_implied_element("tbody");
                }
            }
            "td" | "th" => {
                self.pop_current_if(|name| name == "td" || name == "th");
                self.pop_current_if(|name| name == "caption" || name == "colgroup");
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
        self.pop_current_if(|name| name == "td" || name == "th");
        self.pop_current_if(|name| name == "tr");
        self.pop_current_if(is_table_section);
    }

    fn apply_simple_implied_end_tags(&mut self, incoming_name: &str) {
        if incoming_name == "p" {
            self.pop_current_if(|name| name == "p");
        } else if incoming_name == "li" {
            self.pop_current_if(|name| name == "li");
        } else if incoming_name == "dt" || incoming_name == "dd" {
            self.pop_current_if(|name| name == "dt" || name == "dd");
        } else if incoming_name == "option" {
            self.pop_current_if(|name| name == "option");
        } else if incoming_name == "optgroup" {
            self.pop_current_if(|name| name == "option");
            self.pop_current_if(|name| name == "optgroup");
        } else if incoming_name == "rb" {
            self.pop_current_if(is_ruby_annotation_element);
            self.pop_current_if(|name| name == "rtc");
        } else if incoming_name == "rt" || incoming_name == "rp" {
            self.pop_current_if(|name| name == "rb" || name == "rt" || name == "rp");
        } else if incoming_name == "rtc" {
            self.pop_current_if(|name| name == "rb" || name == "rt" || name == "rp");
            self.pop_current_if(|name| name == "rtc");
        } else if is_heading_element(incoming_name) {
            self.pop_current_if(|name| name == "p");
            self.pop_current_if(is_heading_element);
        } else if is_paragraph_boundary_element(incoming_name) {
            self.pop_current_if(|name| name == "p");
        }
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
            Token::StartTag {
                name, self_closing, ..
            } if !self_closing && !is_void_element(name) => {
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
    matches!(
        name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "details"
            | "dialog"
            | "div"
            | "dl"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "header"
            | "hr"
            | "main"
            | "menu"
            | "nav"
            | "ol"
            | "pre"
            | "section"
            | "table"
            | "ul"
    )
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
}
