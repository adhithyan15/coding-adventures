//! # xml-parser — a namespace-aware XML parser
//!
//! This crate parses XML source text into a typed, namespace-resolved AST
//! ([`XmlDocument`]). It is milestone **M1** of the OOXML effort: the AST it
//! produces is designed to be consumed by an OPC (Open Packaging Conventions)
//! package reader, which walks `[Content_Types].xml` and `.rels` files to find
//! the parts of a `.docx` / `.xlsx` / `.pptx` package.
//!
//! ## The pipeline
//!
//! ```text
//! Source text  ("<w:p xmlns:w=\"...\">hi</w:p>")
//!       |
//!       v
//! xml-lexer            → Vec<Token>            (grammar-driven tokenizer)
//!       |
//!       v
//! xml.grammar          → ParserGrammar         (structural nesting rules)
//!       |
//!       v
//! parser::GrammarParser → GrammarASTNode tree  (generic rule/token tree)
//!       |
//!       v
//! parser.rs (this crate) → XmlDocument          (namespaces + entities + typing)
//! ```
//!
//! The lexer and grammar do the *structural* work. Everything that a
//! context-free grammar cannot express — matching end-tag names to start
//! tags, binding namespace prefixes to URIs, and decoding entity references —
//! lives in this crate's [`parser`] and [`namespace`] modules.
//!
//! ## Quick start
//!
//! ```
//! use coding_adventures_xml_parser::parse_xml;
//!
//! let doc = parse_xml(r#"<note lang="en"><to>Alice</to></note>"#).unwrap();
//! assert_eq!(doc.root.local_name, "note");
//! assert_eq!(doc.root.get_attr(None, "lang"), Some("en"));
//! let to = doc.root.get_child(None, "to").unwrap();
//! assert_eq!(to.text_content(), "Alice");
//! ```
//!
//! ## What this parser does *not* do
//!
//! It has no DTD support, so the only named entities it understands are the
//! five predefined ones (`&amp;`, `&lt;`, `&gt;`, `&apos;`, `&quot;`). That is
//! exactly the subset OOXML uses, which keeps M1 focused.

use coding_adventures_xml_lexer::{create_xml_lexer, tokenize_xml};
use parser::grammar_parser::{GrammarASTNode, GrammarParser, DEFAULT_MAX_RULE_DEPTH};

mod _grammar;
pub mod ast;
mod builder;
pub mod namespace;

// Re-export the AST vocabulary so callers use one import.
pub use ast::{ParseError, XmlAttribute, XmlDocument, XmlElement, XmlNode};
pub use namespace::{XMLNS_NAMESPACE, XML_NAMESPACE};

/// Create a [`GrammarParser`] configured for XML.
///
/// This mirrors `create_json_parser`: it tokenizes the source with the XML
/// lexer and pairs the tokens with the compiled `xml.grammar`. Most callers
/// want [`parse_xml`] instead, which also builds the typed [`XmlDocument`];
/// this factory is exposed for callers that want the raw generic tree.
pub fn create_xml_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_xml(source);
    let grammar = _grammar::parser_grammar();
    // Enable the recursion-depth cap. XML nests without bound (`element →
    // content → element`), so a deeply-nested (possibly hostile) document would
    // otherwise recurse the parser past the native stack and abort the whole
    // process — an uncatchable DoS that no `Result` can report. The cap turns
    // over-deep input into a normal parse error well below the overflow point.
    GrammarParser::new(tokens, grammar).with_max_depth(DEFAULT_MAX_RULE_DEPTH)
}

/// Parse XML source text into a namespace-aware [`XmlDocument`].
///
/// Returns a [`ParseError`] if the input is not well-formed, if a namespace
/// prefix is unbound, or if an entity reference is malformed.
pub fn parse_xml(source: &str) -> Result<XmlDocument, ParseError> {
    // Tokenize via the fallible lexer path so a lexer-level problem (e.g. a
    // stray `&` that starts no valid reference) surfaces as a `ParseError`
    // rather than a panic. This is why `parse_xml` does not reuse
    // `create_xml_parser` (which follows the panicking json-parser glue).
    let tokens = create_xml_lexer(source).tokenize().map_err(|e| ParseError {
        message: e.message,
        line: Some(e.line),
        column: Some(e.column),
    })?;
    // Cap recursion depth (see `create_xml_parser`) so deeply-nested input
    // returns `Err` instead of overflowing the stack and aborting the process.
    let mut grammar_parser =
        GrammarParser::new(tokens, _grammar::parser_grammar()).with_max_depth(DEFAULT_MAX_RULE_DEPTH);
    let root: GrammarASTNode = grammar_parser.parse().map_err(|e| ParseError {
        message: e.message,
        line: Some(e.token.line),
        column: Some(e.token.column),
    })?;
    builder::build_document(&root)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The OPC content-types namespace, used by [Content_Types].xml.
    const CT_NS: &str = "http://schemas.openxmlformats.org/package/2006/content-types";

    // -----------------------------------------------------------------------
    // Simple structure
    // -----------------------------------------------------------------------

    #[test]
    fn test_simple_element() {
        let doc = parse_xml("<p>text</p>").unwrap();
        assert_eq!(doc.root.local_name, "p");
        assert_eq!(doc.root.namespace_uri, None);
        assert_eq!(doc.root.text_content(), "text");
        assert!(doc.root.attributes.is_empty());
    }

    #[test]
    fn test_self_closing_element() {
        let doc = parse_xml("<br/>").unwrap();
        assert_eq!(doc.root.local_name, "br");
        assert!(doc.root.children.is_empty());
    }

    #[test]
    fn test_self_closing_with_space_and_attr() {
        let doc = parse_xml(r#"<img src="a.png" />"#).unwrap();
        assert_eq!(doc.root.local_name, "img");
        assert_eq!(doc.root.get_attr(None, "src"), Some("a.png"));
        assert!(doc.root.children.is_empty());
    }

    #[test]
    fn test_explicit_empty_element() {
        let doc = parse_xml("<div></div>").unwrap();
        assert_eq!(doc.root.local_name, "div");
        assert!(doc.root.children.is_empty());
        assert_eq!(doc.root.text_content(), "");
    }

    // -----------------------------------------------------------------------
    // Attributes — both quote styles
    // -----------------------------------------------------------------------

    #[test]
    fn test_double_quoted_attribute() {
        let doc = parse_xml(r#"<a href="url">x</a>"#).unwrap();
        assert_eq!(doc.root.get_attr(None, "href"), Some("url"));
    }

    #[test]
    fn test_single_quoted_attribute() {
        let doc = parse_xml("<a href='url'>x</a>").unwrap();
        assert_eq!(doc.root.get_attr(None, "href"), Some("url"));
    }

    #[test]
    fn test_multiple_attributes_order() {
        let doc = parse_xml(r#"<a href="u" target="_blank" rel="x"/>"#).unwrap();
        let names: Vec<&str> = doc
            .root
            .attributes
            .iter()
            .map(|a| a.local_name.as_str())
            .collect();
        assert_eq!(names, vec!["href", "target", "rel"]);
        assert_eq!(doc.root.get_attr(None, "target"), Some("_blank"));
    }

    #[test]
    fn test_missing_attribute_is_none() {
        let doc = parse_xml("<a/>").unwrap();
        assert_eq!(doc.root.get_attr(None, "nope"), None);
    }

    // -----------------------------------------------------------------------
    // Nested and mixed content
    // -----------------------------------------------------------------------

    #[test]
    fn test_nested_elements() {
        let doc = parse_xml("<a><b><c>deep</c></b></a>").unwrap();
        let b = doc.root.get_child(None, "b").unwrap();
        let c = b.get_child(None, "c").unwrap();
        assert_eq!(c.text_content(), "deep");
        // text_content aggregates descendants:
        assert_eq!(doc.root.text_content(), "deep");
    }

    #[test]
    fn test_mixed_content() {
        let doc = parse_xml("<p>Hello <b>world</b>!</p>").unwrap();
        // Children: Text("Hello "), Element(b), Text("!")
        assert_eq!(doc.root.children.len(), 3);
        assert!(matches!(&doc.root.children[0], XmlNode::Text(t) if t == "Hello "));
        assert!(matches!(&doc.root.children[1], XmlNode::Element(_)));
        assert!(matches!(&doc.root.children[2], XmlNode::Text(t) if t == "!"));
        assert_eq!(doc.root.text_content(), "Hello world!");
        // get_child must skip the Text children and find the element `b`.
        assert!(doc.root.get_child(None, "b").is_some());
        assert_eq!(doc.root.get_child(None, "missing"), None);
    }

    #[test]
    fn test_get_children_repeated() {
        let doc = parse_xml("<list><item>a</item><item>b</item><other/></list>").unwrap();
        let items = doc.root.get_children(None, "item");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].text_content(), "a");
        assert_eq!(items[1].text_content(), "b");
        assert_eq!(doc.root.get_children(None, "item").len(), 2);
    }

    // -----------------------------------------------------------------------
    // Namespaces
    // -----------------------------------------------------------------------

    #[test]
    fn test_default_namespace_applies_to_element() {
        let doc = parse_xml(r#"<root xmlns="http://example.com/ns"><child/></root>"#).unwrap();
        assert_eq!(doc.root.namespace_uri.as_deref(), Some("http://example.com/ns"));
        // The default namespace is inherited by unprefixed child elements.
        let child = doc.root.children.iter().find_map(|c| match c {
            XmlNode::Element(e) => Some(e),
            _ => None,
        }).unwrap();
        assert_eq!(child.namespace_uri.as_deref(), Some("http://example.com/ns"));
    }

    #[test]
    fn test_prefixed_element_and_attribute_resolve() {
        let src = r#"<w:document xmlns:w="http://word/ns" w:id="7" plain="p"><w:body/></w:document>"#;
        let doc = parse_xml(src).unwrap();
        assert_eq!(doc.root.namespace_uri.as_deref(), Some("http://word/ns"));
        assert_eq!(doc.root.local_name, "document");
        // Prefixed attribute resolves to the URI.
        assert_eq!(doc.root.get_attr(Some("http://word/ns"), "id"), Some("7"));
        // Unprefixed attribute is in NO namespace, even though the element is
        // in a namespace.
        assert_eq!(doc.root.get_attr(None, "plain"), Some("p"));
        assert_eq!(doc.root.get_attr(Some("http://word/ns"), "plain"), None);
        // Prefixed child resolves too.
        let body = doc.root.get_child(Some("http://word/ns"), "body").unwrap();
        assert_eq!(body.local_name, "body");
    }

    #[test]
    fn test_unprefixed_attr_not_in_default_namespace() {
        let doc = parse_xml(r#"<r xmlns="http://d/ns" a="1"/>"#).unwrap();
        assert_eq!(doc.root.get_attr(None, "a"), Some("1"));
        assert_eq!(doc.root.get_attr(Some("http://d/ns"), "a"), None);
    }

    #[test]
    fn test_nested_namespace_scoping_and_shadowing() {
        let src = r#"<a xmlns:p="uri1"><p:b><c xmlns:p="uri2"><p:d/></c></p:b></a>"#;
        let doc = parse_xml(src).unwrap();
        let b = doc.root.get_child(Some("uri1"), "b").unwrap();
        let c = b.get_child(None, "c").unwrap();
        // Inner redefinition shadows the outer binding for prefix p.
        let d = c.get_child(Some("uri2"), "d").unwrap();
        assert_eq!(d.local_name, "d");
        assert!(c.get_child(Some("uri1"), "d").is_none());
    }

    #[test]
    fn test_namespace_uri_is_case_sensitive() {
        let doc = parse_xml(r#"<r xmlns="http://Example.com/NS"/>"#).unwrap();
        assert_eq!(doc.root.namespace_uri.as_deref(), Some("http://Example.com/NS"));
        assert_ne!(doc.root.namespace_uri.as_deref(), Some("http://example.com/ns"));
    }

    #[test]
    fn test_unbound_prefix_is_error() {
        let err = parse_xml("<x:y/>").unwrap_err();
        assert!(err.message.contains("unbound namespace prefix"));
    }

    #[test]
    fn test_reserved_xml_prefix() {
        let doc = parse_xml(r#"<r xml:lang="en"/>"#).unwrap();
        assert_eq!(doc.root.get_attr(Some(XML_NAMESPACE), "lang"), Some("en"));
    }

    #[test]
    fn test_default_namespace_undeclared_on_child() {
        // xmlns="" on a child removes the default namespace for that subtree.
        let src = r#"<a xmlns="U"><b xmlns=""><c/></b></a>"#;
        let doc = parse_xml(src).unwrap();
        assert_eq!(doc.root.namespace_uri.as_deref(), Some("U"));
        let b = doc.root.get_child(None, "b").unwrap();
        assert_eq!(b.namespace_uri, None);
        assert!(b.get_child(None, "c").is_some());
    }

    #[test]
    fn test_malformed_xmlns_declaration_is_error() {
        let err = parse_xml(r#"<r xmlns:="u"/>"#).unwrap_err();
        assert!(err.message.contains("malformed namespace declaration"));
    }

    #[test]
    fn test_xmlns_declaration_not_stored_as_attribute() {
        // The xmlns declaration is consumed, not surfaced as an attribute.
        let doc = parse_xml(r#"<r xmlns:w="u" w:a="1"/>"#).unwrap();
        assert_eq!(doc.root.attributes.len(), 1);
        assert_eq!(doc.root.attributes[0].local_name, "a");
    }

    // -----------------------------------------------------------------------
    // Entity and character references
    // -----------------------------------------------------------------------

    #[test]
    fn test_entities_in_text() {
        // Note: the xml-lexer skips whitespace that sits *between* tokens in
        // the default group, so the space immediately following an entity
        // reference is consumed by the lexer (a known lexer-layer limitation,
        // documented in the README). The parser faithfully decodes and
        // concatenates whatever TEXT / ENTITY_REF tokens it is handed.
        let doc = parse_xml("<p>a &amp; b &lt; c &gt; d</p>").unwrap();
        assert_eq!(doc.root.text_content(), "a &b <c >d");
    }

    #[test]
    fn test_entity_adjacent_to_text_without_spaces() {
        // With no surrounding whitespace the lexer keeps every character, so
        // decoding is exact.
        let doc = parse_xml("<p>x&amp;y&lt;z</p>").unwrap();
        assert_eq!(doc.root.text_content(), "x&y<z");
    }

    #[test]
    fn test_apos_and_quot_entities() {
        let doc = parse_xml("<p>&apos;q&quot;</p>").unwrap();
        assert_eq!(doc.root.text_content(), "'q\"");
    }

    #[test]
    fn test_entities_in_attribute() {
        let doc = parse_xml(r#"<a t="x &amp; y &lt; z"/>"#).unwrap();
        assert_eq!(doc.root.get_attr(None, "t"), Some("x & y < z"));
    }

    #[test]
    fn test_decimal_char_ref() {
        let doc = parse_xml("<p>&#65;&#66;</p>").unwrap();
        assert_eq!(doc.root.text_content(), "AB");
    }

    #[test]
    fn test_hex_char_ref() {
        let doc = parse_xml("<p>&#x41;&#x2764;</p>").unwrap();
        assert_eq!(doc.root.text_content(), "A\u{2764}");
    }

    #[test]
    fn test_char_ref_in_attribute() {
        let doc = parse_xml(r#"<a t="&#72;i"/>"#).unwrap();
        assert_eq!(doc.root.get_attr(None, "t"), Some("Hi"));
    }

    #[test]
    fn test_unknown_entity_is_error() {
        let err = parse_xml("<p>&bogus;</p>").unwrap_err();
        assert!(err.message.contains("unknown entity"));
    }

    #[test]
    fn test_bare_ampersand_is_error() {
        // A stray `&` that starts no valid reference is rejected by the lexer.
        // `parse_xml` surfaces that as a `ParseError` (with position) rather
        // than panicking.
        let err = parse_xml("<p>a &amp b</p>").unwrap_err();
        assert!(!err.message.is_empty());
        assert!(err.line.is_some());
    }

    // -----------------------------------------------------------------------
    // CDATA — verbatim, not decoded
    // -----------------------------------------------------------------------

    #[test]
    fn test_cdata_verbatim() {
        let doc = parse_xml("<p><![CDATA[x < y & z]]></p>").unwrap();
        assert_eq!(doc.root.children.len(), 1);
        assert!(matches!(&doc.root.children[0], XmlNode::CData(t) if t == "x < y & z"));
        // text_content includes CDATA.
        assert_eq!(doc.root.text_content(), "x < y & z");
    }

    #[test]
    fn test_empty_cdata() {
        let doc = parse_xml("<p><![CDATA[]]></p>").unwrap();
        assert!(matches!(&doc.root.children[0], XmlNode::CData(t) if t.is_empty()));
    }

    // -----------------------------------------------------------------------
    // Comments
    // -----------------------------------------------------------------------

    #[test]
    fn test_comment_node() {
        let doc = parse_xml("<p><!-- hi --></p>").unwrap();
        assert!(matches!(&doc.root.children[0], XmlNode::Comment(t) if t == " hi "));
        // Comments contribute nothing to text_content.
        assert_eq!(doc.root.text_content(), "");
    }

    #[test]
    fn test_empty_comment() {
        let doc = parse_xml("<p><!----></p>").unwrap();
        assert!(matches!(&doc.root.children[0], XmlNode::Comment(t) if t.is_empty()));
    }

    #[test]
    fn test_leading_comment_before_root() {
        let doc = parse_xml("<!-- header --><root/>").unwrap();
        assert_eq!(doc.root.local_name, "root");
    }

    // -----------------------------------------------------------------------
    // Processing instructions and the XML declaration
    // -----------------------------------------------------------------------

    #[test]
    fn test_xml_declaration_version_and_encoding() {
        let doc = parse_xml(r#"<?xml version="1.0" encoding="UTF-8"?><root/>"#).unwrap();
        assert_eq!(doc.version.as_deref(), Some("1.0"));
        assert_eq!(doc.encoding.as_deref(), Some("UTF-8"));
        assert_eq!(doc.root.local_name, "root");
    }

    #[test]
    fn test_xml_declaration_version_only() {
        let doc = parse_xml(r#"<?xml version="1.0"?><root/>"#).unwrap();
        assert_eq!(doc.version.as_deref(), Some("1.0"));
        assert_eq!(doc.encoding, None);
    }

    #[test]
    fn test_processing_instruction_node() {
        let doc =
            parse_xml(r#"<root><?xml-stylesheet type="text/xsl" href="a.xsl"?></root>"#).unwrap();
        match &doc.root.children[0] {
            XmlNode::ProcessingInstruction { target, text } => {
                assert_eq!(target, "xml-stylesheet");
                assert!(text.contains("text/xsl"));
            }
            other => panic!("expected PI, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Whitespace handling
    // -----------------------------------------------------------------------

    #[test]
    fn test_whitespace_between_tags_is_skipped() {
        let doc = parse_xml("<a>  <b/>  <c/>  </a>").unwrap();
        // Whitespace-only text between elements is consumed by the lexer's
        // skip pattern, so only the two child elements remain.
        let elems: Vec<_> = doc
            .root
            .children
            .iter()
            .filter(|c| matches!(c, XmlNode::Element(_)))
            .collect();
        assert_eq!(elems.len(), 2);
    }

    #[test]
    fn test_significant_whitespace_in_text_preserved() {
        let doc = parse_xml("<p>a  b</p>").unwrap();
        assert_eq!(doc.root.text_content(), "a  b");
    }

    #[test]
    fn test_pretty_printed_document() {
        let src = "<root>\n  <child>hi</child>\n</root>";
        let doc = parse_xml(src).unwrap();
        let child = doc.root.get_child(None, "child").unwrap();
        assert_eq!(child.text_content(), "hi");
    }

    // -----------------------------------------------------------------------
    // Well-formedness errors
    // -----------------------------------------------------------------------

    #[test]
    fn test_mismatched_tags_is_error() {
        let err = parse_xml("<a></b>").unwrap_err();
        // Either the grammar fails to consume, or the AST builder reports the
        // mismatch; both are acceptable failures. The AST-level message is
        // more precise, so prefer it when reachable.
        assert!(!err.message.is_empty());
    }

    #[test]
    fn test_prefixed_mismatch_detected_by_builder() {
        // Structurally valid (both are TAG_NAME tokens) but names differ.
        let err = parse_xml("<a>x</bb>").unwrap_err();
        assert!(err.message.contains("mismatched tags"));
    }

    #[test]
    fn test_unclosed_element_is_error() {
        let err = parse_xml("<a>text").unwrap_err();
        assert!(!err.message.is_empty());
    }

    #[test]
    fn test_empty_input_is_error() {
        let err = parse_xml("").unwrap_err();
        assert!(!err.message.is_empty());
    }

    #[test]
    fn test_parse_error_display() {
        let err = ParseError {
            message: "boom".to_string(),
            line: Some(3),
            column: Some(5),
        };
        assert_eq!(format!("{err}"), "boom (line 3, column 5)");
        let err2 = ParseError { message: "x".into(), line: None, column: None };
        assert_eq!(format!("{err2}"), "x");
        let err3 = ParseError { message: "y".into(), line: Some(2), column: None };
        assert_eq!(format!("{err3}"), "y (line 2)");
    }

    // -----------------------------------------------------------------------
    // Factory function (raw generic tree)
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_xml_parser_factory() {
        let mut p = create_xml_parser("<r/>");
        let ast = p.parse().expect("should parse");
        assert_eq!(ast.rule_name, "document");
    }

    // -----------------------------------------------------------------------
    // OOXML-flavoured integration test: a small [Content_Types].xml
    // -----------------------------------------------------------------------

    #[test]
    fn test_content_types_document() {
        let src = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="{ns}">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
            ns = CT_NS
        );
        let doc = parse_xml(&src).unwrap();

        assert_eq!(doc.version.as_deref(), Some("1.0"));
        assert_eq!(doc.encoding.as_deref(), Some("UTF-8"));

        // Root <Types> is in the content-types namespace.
        assert_eq!(doc.root.local_name, "Types");
        assert_eq!(doc.root.namespace_uri.as_deref(), Some(CT_NS));

        // Two <Default> children (in the default namespace).
        let defaults = doc.root.get_children(Some(CT_NS), "Default");
        assert_eq!(defaults.len(), 2);
        assert_eq!(defaults[0].get_attr(None, "Extension"), Some("rels"));

        // The <Override> — attributes are unprefixed, so live in NO namespace.
        let override_el = doc.root.get_child(Some(CT_NS), "Override").unwrap();
        assert_eq!(
            override_el.get_attr(None, "PartName"),
            Some("/word/document.xml")
        );
        assert!(override_el
            .get_attr(None, "ContentType")
            .unwrap()
            .contains("wordprocessingml.document.main+xml"));
    }

    #[test]
    fn test_rels_document() {
        // A simplified .rels part in the OPC relationships namespace.
        let rel_ns = "http://schemas.openxmlformats.org/package/2006/relationships";
        let src = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="{ns}">
  <Relationship Id="rId1" Type="http://.../officeDocument" Target="word/document.xml"/>
</Relationships>"#,
            ns = rel_ns
        );
        let doc = parse_xml(&src).unwrap();
        assert_eq!(doc.root.local_name, "Relationships");
        let rel = doc.root.get_child(Some(rel_ns), "Relationship").unwrap();
        assert_eq!(rel.get_attr(None, "Id"), Some("rId1"));
        assert_eq!(rel.get_attr(None, "Target"), Some("word/document.xml"));
    }

    #[test]
    fn test_deeply_nested_input_errors_instead_of_overflowing() {
        // A hostile document can nest elements without bound. Without a depth
        // cap the parser recurses past the native stack and *aborts the whole
        // process* (an uncatchable stack overflow) — a trivial DoS for any
        // reader of untrusted OOXML. With the cap enabled at the entry points,
        // over-deep input must instead return a normal `Err`. 20000 levels is
        // far past both the 128-level cap and any real document, and (crucially)
        // this test would SIGABRT rather than fail if the cap regressed.
        let depth = 20_000;
        let src: String = "<a>".repeat(depth) + &"</a>".repeat(depth);
        let result = parse_xml(&src);
        assert!(
            result.is_err(),
            "deeply-nested input must error, not overflow the stack"
        );
    }
}
