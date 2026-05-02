//! Incremental HTML parser for Venture.
//!
//! This first slice builds a DOM tree from the current Rust HTML lexer tokens.
//! It deliberately starts with a small tree-construction core instead of
//! pretending HTML is context-free. Future batches can add the full WHATWG
//! insertion-mode machinery on top of this DOM target.

use coding_adventures_html_lexer::{
    apply_html_lex_context, create_html_lexer, Attribute as LexerAttribute, Diagnostic,
    HtmlLexContext, HtmlLexer, Token, TokenizerError,
};
use dom_core::{Attribute, Document, DocumentType, Node};
use std::fmt;

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
    let mut lexer = create_html_lexer()?;
    let mut parser = HtmlParser::new();

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
}

impl HtmlParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse_tokens(&mut self, tokens: impl IntoIterator<Item = Token>) -> Document {
        for token in tokens {
            self.process_token(token);
        }
        std::mem::take(&mut self.document)
    }

    pub fn diagnostics(&self) -> &[ParserDiagnostic] {
        &self.diagnostics
    }

    fn finish_document(&mut self) -> Document {
        std::mem::take(&mut self.document)
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

    fn apply_simple_implied_end_tags(&mut self, incoming_name: &str) {
        if incoming_name == "p" {
            self.pop_current_if(|name| name == "p");
        } else if incoming_name == "li" {
            self.pop_current_if(|name| name == "li");
        } else if incoming_name == "dt" || incoming_name == "dd" {
            self.pop_current_if(|name| name == "dt" || name == "dd");
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

fn drain_parser_tokens(lexer: &mut HtmlLexer, parser: &mut HtmlParser) -> Result<(), ParseError> {
    for token in lexer.drain_tokens() {
        let next_context = text_context_for_token(&token);
        parser.process_token(token);

        if let Some(context) = next_context {
            apply_html_lex_context(lexer, &context)?;
        }
    }

    Ok(())
}

fn text_context_for_token(token: &Token) -> Option<HtmlLexContext> {
    match token {
        Token::StartTag {
            name, self_closing, ..
        } if !self_closing && !is_void_element(name) => HtmlLexContext::for_element_text(name),
        Token::EndTag { .. } => Some(HtmlLexContext::data()),
        _ => None,
    }
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

    #[test]
    fn parses_nested_elements_and_text() {
        let document = parse_html("<h1>Hello <em>Venture</em></h1>").unwrap();

        let h1 = element(&document.children[0]);
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

        let image = element(&document.children[2]);
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
        let paragraph = element(&output.document.children[0]);
        assert_eq!(paragraph.children, vec![Node::text("Hello")]);
    }

    #[test]
    fn applies_simple_html_implied_end_tags() {
        let document = parse_html("<ul><li>one<li>two</ul><p>a<p>b").unwrap();

        let list = element(&document.children[0]);
        assert_eq!(list.name, "ul");
        assert_eq!(list.children.len(), 2);
        assert_eq!(element(&list.children[0]).children, vec![Node::text("one")]);
        assert_eq!(element(&list.children[1]).children, vec![Node::text("two")]);

        assert_eq!(
            element(&document.children[1]).children,
            vec![Node::text("a")]
        );
        assert_eq!(
            element(&document.children[2]).children,
            vec![Node::text("b")]
        );
    }

    #[test]
    fn parser_drives_rcdata_tokenization_for_title_and_textarea() {
        let document =
            parse_html("<title>Tom &amp; Jerry</title><textarea>A &lt; B</textarea>").unwrap();

        let title = element(&document.children[0]);
        assert_eq!(title.name, "title");
        assert_eq!(title.children, vec![Node::text("Tom & Jerry")]);

        let textarea = element(&document.children[1]);
        assert_eq!(textarea.name, "textarea");
        assert_eq!(textarea.children, vec![Node::text("A < B")]);
    }

    #[test]
    fn parser_drives_rawtext_and_script_tokenization() {
        let document = parse_html(
            "<style>a < b &amp; c</style><script>if (a < b) alert('&amp;');</script><p>x</p>",
        )
        .unwrap();

        let style = element(&document.children[0]);
        assert_eq!(style.name, "style");
        assert_eq!(style.children, vec![Node::text("a < b &amp; c")]);

        let script = element(&document.children[1]);
        assert_eq!(script.name, "script");
        assert_eq!(
            script.children,
            vec![Node::text("if (a < b) alert('&amp;');")]
        );

        let paragraph = element(&document.children[2]);
        assert_eq!(paragraph.children, vec![Node::text("x")]);
    }

    #[test]
    fn parser_drives_plaintext_tokenization_until_eof() {
        let document = parse_html("<p>before</p><plaintext><b>&amp;</b></plaintext>").unwrap();

        let paragraph = element(&document.children[0]);
        assert_eq!(paragraph.children, vec![Node::text("before")]);

        let plaintext = element(&document.children[1]);
        assert_eq!(
            plaintext.children,
            vec![Node::text("<b>&amp;</b></plaintext>")]
        );
    }
}
