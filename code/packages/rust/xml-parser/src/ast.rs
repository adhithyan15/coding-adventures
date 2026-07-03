//! The XML Abstract Syntax Tree (AST).
//!
//! This module defines the *shape* of a parsed XML document — the data
//! structures that the rest of the crate builds and that downstream consumers
//! (an OPC package reader, in the OOXML effort) walk.
//!
//! # A quick tour of XML for a newcomer
//!
//! An XML document is a tree of **elements**. An element has a *name*, a bag
//! of *attributes*, and a list of *children*. The children can be more
//! elements, plain text, comments, and a few other things. Here is a tiny
//! document:
//!
//! ```xml
//! <note lang="en">
//!   <to>Alice</to>
//!   <body>Hi &amp; welcome</body>
//! </note>
//! ```
//!
//! - `note` is the **root element**. It has one attribute, `lang="en"`.
//! - `to` and `body` are **child elements** of `note`.
//! - `Alice` and `Hi & welcome` are **text**. (`&amp;` is an *entity
//!   reference* that stands for the literal `&`; the parser decodes it.)
//!
//! # Namespaces — why a name is really two things
//!
//! XML lets documents from different vocabularies mix without their names
//! colliding. A **namespace** is a URI (just an opaque identifier — it need
//! not resolve to anything on the web) that scopes a set of names. In OOXML a
//! Word document element looks like `<w:p>`, where `w` is a short *prefix*
//! bound to the long namespace URI
//! `http://schemas.openxmlformats.org/wordprocessingml/2006/main`.
//!
//! Because prefixes are just local shorthands (the same document could bind
//! `w` to something else, or use a different prefix for the same URI), a
//! *resolved* name is the pair `(namespace_uri, local_name)`. That is exactly
//! what [`XmlElement`] and [`XmlAttribute`] store: the prefix is thrown away
//! once resolved, and code compares against the stable URI. URIs are
//! **case-sensitive**.

// ===========================================================================
// XmlDocument
// ===========================================================================

/// A whole parsed XML document.
///
/// The XML declaration `<?xml version="1.0" encoding="UTF-8"?>` is not stored
/// as a node in the tree — instead its `version` and `encoding` pseudo-
/// attributes are lifted onto this struct. Everything else in a well-formed
/// document hangs off the single [`root`](XmlDocument::root) element.
#[derive(Debug, Clone, PartialEq)]
pub struct XmlDocument {
    /// The single root element (XML documents have exactly one).
    pub root: XmlElement,
    /// The `version` from the XML declaration, if one was present.
    pub version: Option<String>,
    /// The `encoding` from the XML declaration, if one was present.
    pub encoding: Option<String>,
}

// ===========================================================================
// XmlElement
// ===========================================================================

/// A single XML element: a resolved name, its attributes, and its children.
#[derive(Debug, Clone, PartialEq)]
pub struct XmlElement {
    /// The resolved namespace URI, or `None` if the element is in no
    /// namespace. Resolution follows the prefix / default-namespace rules
    /// described on this module.
    pub namespace_uri: Option<String>,
    /// The local part of the element's name (the part after the `prefix:`).
    /// For `<w:p>` this is `"p"`; for `<note>` it is `"note"`.
    pub local_name: String,
    /// The element's attributes, in document order.
    pub attributes: Vec<XmlAttribute>,
    /// The element's children, in document order.
    pub children: Vec<XmlNode>,
}

impl XmlElement {
    /// Find the first *direct child element* matching a resolved name.
    ///
    /// `uri` is the namespace to match: pass `Some("...")` to require a
    /// specific namespace, or `None` to require that the child is in *no*
    /// namespace. `local` is the local name.
    ///
    /// Only direct children are searched (not grandchildren), which is what a
    /// package reader wants when walking a known document shape.
    pub fn get_child(&self, uri: Option<&str>, local: &str) -> Option<&XmlElement> {
        self.child_elements()
            .find(|e| e.namespace_uri.as_deref() == uri && e.local_name == local)
    }

    /// Like [`get_child`](XmlElement::get_child) but returns *every* matching
    /// direct child element, in document order. Handy for repeated elements
    /// such as `<Default>` / `<Override>` entries in `[Content_Types].xml`.
    pub fn get_children(&self, uri: Option<&str>, local: &str) -> Vec<&XmlElement> {
        self.child_elements()
            .filter(|e| e.namespace_uri.as_deref() == uri && e.local_name == local)
            .collect()
    }

    /// Look up an attribute value by resolved name, returning the (already
    /// entity-decoded) value if present.
    ///
    /// Remember the asymmetry: an unprefixed attribute is in *no* namespace
    /// even when the element sits inside a default namespace, so most OPC
    /// attributes are looked up with `uri = None`.
    pub fn get_attr(&self, uri: Option<&str>, local: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| a.namespace_uri.as_deref() == uri && a.local_name == local)
            .map(|a| a.value.as_str())
    }

    /// Concatenate all descendant text — the element's text nodes and CDATA
    /// sections, plus those of every nested element, in document order.
    ///
    /// This is the XML "string value" of an element. Comments and processing
    /// instructions contribute nothing.
    pub fn text_content(&self) -> String {
        let mut out = String::new();
        self.collect_text(&mut out);
        out
    }

    fn collect_text(&self, out: &mut String) {
        for child in &self.children {
            match child {
                XmlNode::Text(t) | XmlNode::CData(t) => out.push_str(t),
                XmlNode::Element(e) => e.collect_text(out),
                XmlNode::Comment(_) | XmlNode::ProcessingInstruction { .. } => {}
            }
        }
    }

    /// Iterator over just the child *elements* (skipping text, comments, etc.).
    fn child_elements(&self) -> impl Iterator<Item = &XmlElement> {
        self.children.iter().filter_map(|c| match c {
            XmlNode::Element(e) => Some(e.as_ref()),
            _ => None,
        })
    }
}

// ===========================================================================
// XmlAttribute
// ===========================================================================

/// A resolved attribute: name pair plus its decoded value.
#[derive(Debug, Clone, PartialEq)]
pub struct XmlAttribute {
    /// The resolved namespace URI, or `None`. Note that `xmlns` and
    /// `xmlns:prefix` declarations are *consumed* during parsing and never
    /// appear here as attributes.
    pub namespace_uri: Option<String>,
    /// The local part of the attribute name.
    pub local_name: String,
    /// The attribute value, with surrounding quotes stripped and entity /
    /// character references decoded.
    pub value: String,
}

// ===========================================================================
// XmlNode
// ===========================================================================

/// One child in an element's content list.
///
/// An element's children are heterogeneous — text and elements can interleave
/// ("mixed content"), and comments / CDATA / processing instructions can
/// appear too — so a single enum captures all the possibilities.
#[derive(Debug, Clone, PartialEq)]
pub enum XmlNode {
    /// A nested element. Boxed because [`XmlElement`] contains a `Vec` of
    /// these, so without the indirection the type would be infinitely sized.
    Element(Box<XmlElement>),
    /// A run of character data, with entity / character references already
    /// decoded.
    Text(String),
    /// The verbatim contents of a `<![CDATA[ ... ]]>` section. CDATA is *not*
    /// entity-decoded — its whole point is to hold text like `<` and `&`
    /// literally.
    CData(String),
    /// A `<!-- ... -->` comment's text (without the delimiters).
    Comment(String),
    /// A processing instruction `<?target text?>` (other than the XML
    /// declaration, which is lifted onto [`XmlDocument`]).
    ProcessingInstruction {
        /// The PI target (the name right after `<?`).
        target: String,
        /// The rest of the PI, verbatim (may be empty).
        text: String,
    },
}

// ===========================================================================
// ParseError
// ===========================================================================

/// A failure to parse XML source into an [`XmlDocument`].
///
/// Carries an optional 1-based line / column so callers can point at the
/// offending spot. Some errors (e.g. a lexer-level failure) may not have a
/// precise position, hence the `Option`s.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// A human-readable description of what went wrong.
    pub message: String,
    /// The 1-based line where the problem was detected, if known.
    pub line: Option<usize>,
    /// The 1-based column where the problem was detected, if known.
    pub column: Option<usize>,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.line, self.column) {
            (Some(l), Some(c)) => write!(f, "{} (line {}, column {})", self.message, l, c),
            (Some(l), None) => write!(f, "{} (line {})", self.message, l),
            _ => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for ParseError {}
