//! The AST builder: turn a raw `GrammarASTNode` tree into an [`XmlDocument`].
//!
//! `xml.grammar` only tells the [`GrammarParser`] how tokens may nest; it
//! produces a generic tree keyed by rule names (`"document"`, `"element"`,
//! `"attribute"`, …). This module walks that generic tree once and builds the
//! typed, namespace-resolved [`XmlDocument`], doing the three jobs the grammar
//! cannot:
//!
//! 1. **Well-formedness beyond nesting** — checking that each end tag names
//!    the same element as its start tag (a context-sensitive constraint).
//! 2. **Namespace resolution** — binding prefixes to URIs via a scope stack.
//! 3. **Entity decoding** — turning `&amp;`, `&#65;`, etc. into real text.
//!
//! # How to read the generic tree
//!
//! The `parser` crate flattens token matches and repetitions into a node's
//! `children` list, but wraps each *rule* reference in its own `Node`. So a
//! `container_element` node's children are, in order:
//!
//! ```text
//! Token(OPEN_TAG_START) Token(TAG_NAME) Node(attribute)* Token(TAG_CLOSE)
//! Node(content)* Token(CLOSE_TAG_START) Token(TAG_NAME) Token(TAG_CLOSE)
//! ```
//!
//! We navigate by walking those children and dispatching on token type name
//! or child rule name.

use std::collections::HashMap;

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

use crate::ast::{ParseError, XmlAttribute, XmlDocument, XmlElement, XmlNode};
use crate::namespace::{decode_entities, split_qname, NamespaceStack};

// ===========================================================================
// Small helpers over the generic tree
// ===========================================================================

/// Return the raw source text of a leaf token child, or `None` if this child
/// is a sub-node rather than a token.
fn token_value(child: &ASTNodeOrToken) -> Option<&str> {
    match child {
        ASTNodeOrToken::Token(t) => Some(t.value.as_str()),
        ASTNodeOrToken::Node(_) => None,
    }
}

/// Return the grammar token-type name of a leaf token child (e.g. "TAG_NAME"),
/// or `None` if this child is a sub-node.
fn token_type(child: &ASTNodeOrToken) -> Option<&str> {
    match child {
        ASTNodeOrToken::Token(t) => Some(t.effective_type_name()),
        ASTNodeOrToken::Node(_) => None,
    }
}

/// Position (line, column) of the first token under a node, for error
/// reporting. Falls back to the node's recorded start position.
fn node_position(node: &GrammarASTNode) -> (Option<usize>, Option<usize>) {
    (node.start_line, node.start_column)
}

// ===========================================================================
// Entry point
// ===========================================================================

/// Build an [`XmlDocument`] from the root `document` grammar node.
pub fn build_document(root: &GrammarASTNode) -> Result<XmlDocument, ParseError> {
    // The `document` rule is: { misc } element { misc }. Its children are a
    // flat list of `Node(misc)` and one `Node(element)`. We scan for the XML
    // declaration among the leading misc items and for the single element.
    let mut version = None;
    let mut encoding = None;
    let mut root_element: Option<XmlElement> = None;

    for child in &root.children {
        let node = match child {
            ASTNodeOrToken::Node(n) => n,
            ASTNodeOrToken::Token(_) => continue,
        };
        match node.rule_name.as_str() {
            "misc" => {
                // misc wraps a single pi or comment. Only a `<?xml ...?>` PI
                // carries the version/encoding we care about at this level.
                if let Some(inner) = first_child_node(node) {
                    if inner.rule_name == "pi" {
                        if let Some((target, text)) = read_pi(inner) {
                            if target == "xml" {
                                let (v, e) = parse_xml_declaration(&text);
                                version = v;
                                encoding = e;
                            }
                        }
                    }
                }
            }
            "element" => {
                let mut ns = NamespaceStack::new();
                root_element = Some(build_element(node, &mut ns)?);
            }
            _ => {}
        }
    }

    let root = root_element.ok_or_else(|| ParseError {
        message: "document has no root element".to_string(),
        line: None,
        column: None,
    })?;

    Ok(XmlDocument {
        root,
        version,
        encoding,
    })
}

/// Return the first child that is a sub-node (skipping leaf tokens).
fn first_child_node(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) => Some(n),
        ASTNodeOrToken::Token(_) => None,
    })
}

// ===========================================================================
// Elements
// ===========================================================================

/// Build an [`XmlElement`] from an `element` node (which wraps either an
/// `empty_element` or a `container_element`).
fn build_element(
    element_node: &GrammarASTNode,
    ns: &mut NamespaceStack,
) -> Result<XmlElement, ParseError> {
    let inner = first_child_node(element_node).ok_or_else(|| ParseError {
        message: "malformed element node".to_string(),
        line: element_node.start_line,
        column: element_node.start_column,
    })?;

    match inner.rule_name.as_str() {
        "empty_element" => build_start_tag(inner, &[], ns),
        "container_element" => build_container(inner, ns),
        other => Err(ParseError {
            message: format!("unexpected element kind '{other}'"),
            line: inner.start_line,
            column: inner.start_column,
        }),
    }
}

/// Build a container element: start tag, content children, and the matching
/// end tag (whose name we verify against the start tag).
fn build_container(
    node: &GrammarASTNode,
    ns: &mut NamespaceStack,
) -> Result<XmlElement, ParseError> {
    // Split the children at TAG_CLOSE: everything before is the start tag
    // (name + attributes); everything after, up to CLOSE_TAG_START, is
    // content; then CLOSE_TAG_START TAG_NAME TAG_CLOSE.
    let children = &node.children;

    // Find the index of the first TAG_CLOSE (end of the start tag).
    let start_tag_close = children
        .iter()
        .position(|c| token_type(c) == Some("TAG_CLOSE"))
        .ok_or_else(|| ParseError {
            message: "start tag missing '>'".to_string(),
            line: node.start_line,
            column: node.start_column,
        })?;

    // Find CLOSE_TAG_START (start of the end tag).
    let close_start = children
        .iter()
        .position(|c| token_type(c) == Some("CLOSE_TAG_START"))
        .ok_or_else(|| ParseError {
            message: "element missing closing tag".to_string(),
            line: node.start_line,
            column: node.start_column,
        })?;

    // --- Start tag: children[0..=start_tag_close] ---
    let start_slice = &children[..start_tag_close];
    let content_slice = &children[start_tag_close + 1..close_start];

    // The end tag name is the TAG_NAME between CLOSE_TAG_START and its
    // TAG_CLOSE.
    let end_name = children[close_start + 1..]
        .iter()
        .find_map(|c| {
            if token_type(c) == Some("TAG_NAME") {
                token_value(c)
            } else {
                None
            }
        })
        .ok_or_else(|| ParseError {
            message: "closing tag missing name".to_string(),
            line: node.start_line,
            column: node.start_column,
        })?;

    // Build the element from its start tag (this pushes a namespace scope).
    let start_name = start_slice
        .iter()
        .find_map(|c| {
            if token_type(c) == Some("TAG_NAME") {
                token_value(c)
            } else {
                None
            }
        })
        .ok_or_else(|| ParseError {
            message: "start tag missing name".to_string(),
            line: node.start_line,
            column: node.start_column,
        })?
        .to_string();

    // Well-formedness: start and end names must be the *raw* qualified names
    // matched literally (XML matches on the qname text, prefix included).
    if start_name != end_name {
        let (l, c) = node_position(node);
        return Err(ParseError {
            message: format!(
                "mismatched tags: <{start_name}> closed by </{end_name}>"
            ),
            line: l,
            column: c,
        });
    }

    build_start_tag(node, content_slice, ns)
}

/// Build an element from a node whose children begin with the start tag
/// (`OPEN_TAG_START TAG_NAME attribute*`), with `content_children` supplying
/// the already-sliced content nodes (empty for a self-closing element).
///
/// This function owns the namespace scope lifecycle: it collects this tag's
/// `xmlns` declarations, pushes a scope, resolves names and builds children,
/// then pops.
fn build_start_tag(
    node: &GrammarASTNode,
    content_children: &[ASTNodeOrToken],
    ns: &mut NamespaceStack,
) -> Result<XmlElement, ParseError> {
    // Collect the raw (qname, raw_value) attribute pairs and the element's
    // own qualified name.
    let mut raw_attributes: Vec<(String, String)> = Vec::new();
    let mut element_qname: Option<String> = None;
    let mut seen_tag_name = false;

    for child in &node.children {
        // Stop scanning the start tag once we reach its closing token: for a
        // container the first TAG_CLOSE ends the start tag; for an empty
        // element SELF_CLOSE ends it.
        match token_type(child) {
            Some("TAG_CLOSE") | Some("SELF_CLOSE") => break,
            Some("TAG_NAME") if !seen_tag_name => {
                element_qname = token_value(child).map(|s| s.to_string());
                seen_tag_name = true;
                continue;
            }
            _ => {}
        }
        if let ASTNodeOrToken::Node(attr) = child {
            if attr.rule_name == "attribute" {
                let (name, value) = read_attribute(attr)?;
                raw_attributes.push((name, value));
            }
        }
    }

    let element_qname = element_qname.ok_or_else(|| ParseError {
        message: "element has no name".to_string(),
        line: node.start_line,
        column: node.start_column,
    })?;

    // --- Namespace declarations from this tag's attributes ---
    let mut declarations: HashMap<String, String> = HashMap::new();
    let mut normal_attrs: Vec<(String, String)> = Vec::new();
    for (qname, raw_value) in raw_attributes {
        let decoded = decode_entities(strip_quotes(&raw_value))?;
        if qname == "xmlns" {
            // Default-namespace declaration; key "" in the scope map.
            declarations.insert(String::new(), decoded);
        } else if let Some(prefix) = qname.strip_prefix("xmlns:") {
            if prefix.is_empty() {
                return Err(ParseError {
                    message: "malformed namespace declaration 'xmlns:'".to_string(),
                    line: node.start_line,
                    column: node.start_column,
                });
            }
            declarations.insert(prefix.to_string(), decoded);
        } else {
            normal_attrs.push((qname, decoded));
        }
    }

    ns.push(declarations);

    // --- Resolve the element name ---
    let (prefix, local) = split_qname(&element_qname)?;
    let namespace_uri = ns.resolve_prefix(prefix)?;

    // --- Resolve attribute names ---
    // Per the XML Namespaces spec, an *unprefixed* attribute is in no
    // namespace even when a default namespace is in scope. Only prefixed
    // attributes get a namespace URI.
    let mut attributes = Vec::with_capacity(normal_attrs.len());
    for (qname, value) in normal_attrs {
        let (aprefix, alocal) = split_qname(&qname)?;
        let auri = if aprefix.is_empty() {
            None
        } else {
            ns.resolve_prefix(aprefix)?
        };
        attributes.push(XmlAttribute {
            namespace_uri: auri,
            local_name: alocal.to_string(),
            value,
        });
    }

    // --- Build children ---
    // Build children, then pop the scope *whether or not* child building
    // failed, and only then propagate any error. This keeps the scope stack
    // balanced even on the error path.
    let result = build_children(content_children, ns);
    ns.pop();
    let children = result?;

    Ok(XmlElement {
        namespace_uri,
        local_name: local.to_string(),
        attributes,
        children,
    })
}

/// Build the list of child nodes for an element from its `content` slice.
fn build_children(
    content_children: &[ASTNodeOrToken],
    ns: &mut NamespaceStack,
) -> Result<Vec<XmlNode>, ParseError> {
    let mut out = Vec::new();
    for child in content_children {
        // Each content item is a `Node(content)` wrapping one thing.
        let content = match child {
            ASTNodeOrToken::Node(n) if n.rule_name == "content" => n,
            _ => continue,
        };
        // The content node has exactly one child: a sub-node (element,
        // comment, cdata, pi) or a leaf token (TEXT, CHAR_REF, ENTITY_REF).
        let inner = match content.children.first() {
            Some(c) => c,
            None => continue,
        };
        match inner {
            ASTNodeOrToken::Node(n) => match n.rule_name.as_str() {
                "element" => {
                    let el = build_element(n, ns)?;
                    out.push(XmlNode::Element(Box::new(el)));
                }
                "comment" => out.push(XmlNode::Comment(read_comment(n))),
                "cdata" => out.push(XmlNode::CData(read_cdata(n))),
                "pi" => {
                    if let Some((target, text)) = read_pi(n) {
                        out.push(XmlNode::ProcessingInstruction { target, text });
                    }
                }
                _ => {}
            },
            ASTNodeOrToken::Token(t) => match t.effective_type_name() {
                "TEXT" | "ENTITY_REF" | "CHAR_REF" => {
                    let decoded = decode_entities(&t.value)?;
                    // Merge with a preceding text node so `a&amp;b` yields one
                    // "a&b" node rather than three fragments.
                    if let Some(XmlNode::Text(prev)) = out.last_mut() {
                        prev.push_str(&decoded);
                    } else {
                        out.push(XmlNode::Text(decoded));
                    }
                }
                _ => {}
            },
        }
    }
    Ok(out)
}

// ===========================================================================
// Leaf readers
// ===========================================================================

/// Read an `attribute` node into a `(qname, raw_value_with_quotes)` pair.
fn read_attribute(node: &GrammarASTNode) -> Result<(String, String), ParseError> {
    let mut name = None;
    let mut value = None;
    for child in &node.children {
        match token_type(child) {
            Some("TAG_NAME") => name = token_value(child).map(str::to_string),
            Some("ATTR_VALUE") => value = token_value(child).map(str::to_string),
            _ => {}
        }
    }
    match (name, value) {
        (Some(n), Some(v)) => Ok((n, v)),
        _ => Err(ParseError {
            message: "malformed attribute".to_string(),
            line: node.start_line,
            column: node.start_column,
        }),
    }
}

/// Read a `comment` node's text (empty for `<!---->`).
fn read_comment(node: &GrammarASTNode) -> String {
    node.children
        .iter()
        .find_map(|c| {
            if token_type(c) == Some("COMMENT_TEXT") {
                token_value(c).map(str::to_string)
            } else {
                None
            }
        })
        .unwrap_or_default()
}

/// Read a `cdata` node's verbatim text (empty for `<![CDATA[]]>`).
fn read_cdata(node: &GrammarASTNode) -> String {
    node.children
        .iter()
        .find_map(|c| {
            if token_type(c) == Some("CDATA_TEXT") {
                token_value(c).map(str::to_string)
            } else {
                None
            }
        })
        .unwrap_or_default()
}

/// Read a `pi` node into `(target, text)`. The text is verbatim, with any
/// single leading space (which the lexer includes) trimmed for convenience.
fn read_pi(node: &GrammarASTNode) -> Option<(String, String)> {
    let mut target = None;
    let mut text = String::new();
    for child in &node.children {
        match token_type(child) {
            Some("PI_TARGET") => target = token_value(child).map(str::to_string),
            Some("PI_TEXT") => {
                text = token_value(child).unwrap_or("").to_string();
            }
            _ => {}
        }
    }
    target.map(|t| (t, text.trim_start().to_string()))
}

// ===========================================================================
// Small string utilities
// ===========================================================================

/// Strip one matching pair of surrounding quotes from a raw ATTR_VALUE.
/// The lexer guarantees the value starts and ends with the same quote char,
/// but we defensively handle the degenerate case.
fn strip_quotes(raw: &str) -> &str {
    let bytes = raw.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' || first == b'\'') && first == last {
            return &raw[1..raw.len() - 1];
        }
    }
    raw
}

/// Parse the pseudo-attributes of an XML declaration's text
/// (`version="1.0" encoding="UTF-8"`) into `(version, encoding)`.
///
/// This is a tiny purpose-built scanner: the XML declaration is *not* an
/// element and its "attributes" are only these fixed pseudo-attributes, so a
/// full attribute parse is unnecessary.
fn parse_xml_declaration(text: &str) -> (Option<String>, Option<String>) {
    (
        pseudo_attr(text, "version"),
        pseudo_attr(text, "encoding"),
    )
}

/// Extract `name="value"` (or single-quoted) from a declaration fragment.
fn pseudo_attr(text: &str, name: &str) -> Option<String> {
    let idx = text.find(name)?;
    let after = &text[idx + name.len()..];
    let after = after.trim_start();
    let after = after.strip_prefix('=')?.trim_start();
    let quote = after.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &after[1..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_quotes_double() {
        assert_eq!(strip_quotes("\"hi\""), "hi");
    }

    #[test]
    fn strip_quotes_single() {
        assert_eq!(strip_quotes("'hi'"), "hi");
    }

    #[test]
    fn strip_quotes_degenerate_passthrough() {
        // Too short, or mismatched quotes — returned unchanged.
        assert_eq!(strip_quotes(""), "");
        assert_eq!(strip_quotes("x"), "x");
        assert_eq!(strip_quotes("\"mismatch'"), "\"mismatch'");
    }

    #[test]
    fn xml_declaration_both_pseudo_attrs() {
        let (v, e) = parse_xml_declaration(r#" version="1.0" encoding="UTF-8""#);
        assert_eq!(v.as_deref(), Some("1.0"));
        assert_eq!(e.as_deref(), Some("UTF-8"));
    }

    #[test]
    fn xml_declaration_single_quotes() {
        let (v, e) = parse_xml_declaration("version='1.1' encoding='us-ascii'");
        assert_eq!(v.as_deref(), Some("1.1"));
        assert_eq!(e.as_deref(), Some("us-ascii"));
    }

    #[test]
    fn xml_declaration_missing_attr_is_none() {
        let (v, e) = parse_xml_declaration("version=\"1.0\"");
        assert_eq!(v.as_deref(), Some("1.0"));
        assert_eq!(e, None);
    }

    #[test]
    fn pseudo_attr_rejects_unquoted_value() {
        // A value not wrapped in quotes yields None.
        assert_eq!(pseudo_attr("version=1.0", "version"), None);
    }

    #[test]
    fn pseudo_attr_absent_returns_none() {
        assert_eq!(pseudo_attr("encoding=\"UTF-8\"", "version"), None);
    }
}
