//! Small, in-memory implementations of two archived early-Web profiles.
//!
//! The module deliberately does not use a modern HTTP or HTML library. The
//! wire codec follows the CERN 1991 HTTP/0.9 description; the markup scanner
//! follows W3C's November 1992 *HTML Tags* archive and the deterministic rules
//! collected in HWEB00. This is a historical teaching profile, not HTML5.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

/// A validated printable-ASCII document address for one HTTP/0.9 request.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Http09Address(String);

impl Http09Address {
    /// Validate and store a nonempty address with no spaces or control bytes.
    pub fn new(value: impl Into<String>) -> Result<Self, Http09RequestError> {
        let value = value.into();
        if value.is_empty() || !value.bytes().all(|byte| (0x21..=0x7e).contains(&byte)) {
            return Err(Http09RequestError::InvalidAddress);
        }
        Ok(Self(value))
    }

    /// Return the address without normalization.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Errors raised while constructing or parsing an HTTP/0.9 request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Http09RequestError {
    /// The address was empty, contained whitespace, or was not printable ASCII.
    InvalidAddress,
    /// The wire did not contain exactly one valid `GET` request line.
    InvalidRequest,
}

impl fmt::Display for Http09RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAddress => f.write_str("invalid HTTP/0.9 document address"),
            Self::InvalidRequest => f.write_str("invalid HTTP/0.9 request"),
        }
    }
}

impl Error for Http09RequestError {}

/// An error decoding a completed HTTP/0.9 response body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Http09ResponseError {
    /// The response contains at least one byte outside ASCII.
    NonAscii,
}

impl fmt::Display for Http09ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HTTP/0.9 response is not ASCII")
    }
}

impl Error for Http09ResponseError {}

/// Encode one CERN HTTP/0.9 request line as `GET address\r\n`.
pub fn encode_http09_request(address: &Http09Address) -> Vec<u8> {
    let mut wire = Vec::with_capacity(6 + address.0.len());
    wire.extend_from_slice(b"GET ");
    wire.extend_from_slice(address.0.as_bytes());
    wire.extend_from_slice(b"\r\n");
    wire
}

/// Parse exactly one HTTP/0.9 `GET` line, terminated by LF or CRLF.
pub fn parse_http09_request(wire: &[u8]) -> Result<Http09Address, Http09RequestError> {
    if wire.len() < 6 || !wire.starts_with(b"GET ") || wire.last() != Some(&b'\n') {
        return Err(Http09RequestError::InvalidRequest);
    }
    let mut end = wire.len() - 1;
    if end > 0 && wire[end - 1] == b'\r' {
        end -= 1;
    }
    let value =
        std::str::from_utf8(&wire[4..end]).map_err(|_| Http09RequestError::InvalidRequest)?;
    Http09Address::new(value.to_owned()).map_err(|_| Http09RequestError::InvalidRequest)
}

/// Decode the complete ASCII response body, preserving all line endings.
pub fn decode_http09_response(body: &[u8]) -> Result<String, Http09ResponseError> {
    if !body.is_ascii() {
        return Err(Http09ResponseError::NonAscii);
    }
    // ASCII is a strict subset of UTF-8.
    Ok(String::from_utf8(body.to_vec()).expect("ASCII bytes are valid UTF-8"))
}

/// The stable integer index of a node in an [`HtmlDocument`] arena.
pub type NodeId = usize;

/// A node in the 1992 HTML tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HtmlNode {
    /// Text or element data. Elements refer to children by arena index.
    pub kind: HtmlNodeKind,
}

/// The payload of one arena node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HtmlNodeKind {
    /// Character data after CRLF normalization and one archived entity pass.
    Text(String),
    /// A lowercase element name, its recognized attributes, and child indices.
    Element {
        /// Lowercase historical element name.
        tag: String,
        /// Attributes (only `name`, `href`, and `type` on anchors are retained).
        attributes: Vec<HtmlAttribute>,
        /// Ordered children in this same arena.
        children: Vec<NodeId>,
    },
}

/// One recognized anchor attribute. `None` means the attribute had no value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HtmlAttribute {
    /// Lowercase ASCII attribute name.
    pub name: String,
    /// Decoded value, or `None` for a valueless attribute.
    pub value: Option<String>,
}

/// A non-recursive ownership container for a parsed HTML document.
///
/// Every element stores child indices rather than owning child structs. This
/// keeps parser construction, traversal, and drop independent of document
/// nesting depth.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HtmlDocument {
    /// All nodes in append order. References in `roots` and element children
    /// index this vector.
    pub nodes: Vec<HtmlNode>,
    /// Ordered top-level node indices.
    pub roots: Vec<NodeId>,
}

impl HtmlDocument {
    /// Look up a node by its stable arena index.
    pub fn node(&self, id: NodeId) -> Option<&HtmlNode> {
        self.nodes.get(id)
    }
}

/// Reserved for impossible internal parser states; malformed markup recovers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HtmlParseError {
    /// An internal node index or stack invariant was violated.
    InvalidTreeState,
}

impl fmt::Display for HtmlParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid historical HTML tree state")
    }
}

impl Error for HtmlParseError {}

#[derive(Clone, Copy, Debug)]
struct Token<'a> {
    name: &'a str,
    attrs: &'a str,
    closing: bool,
    end: usize,
}

const TAGS: &[&str] = &[
    "title",
    "address",
    "a",
    "dl",
    "li",
    "xmp",
    "listing",
    "plaintext",
    "ul",
    "menu",
    "dir",
    "dt",
    "dd",
    "p",
    "isindex",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
];

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0c)
}

fn is_name_start(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

fn is_name_char(c: u8) -> bool {
    is_name_start(c) || c.is_ascii_digit() || c == b'-'
}

fn lower_ascii(value: &str) -> String {
    value
        .bytes()
        .map(|b| b.to_ascii_lowercase() as char)
        .collect()
}

fn tag_at(source: &str, start: usize) -> Option<Token<'_>> {
    let bytes = source.as_bytes();
    if bytes.get(start) != Some(&b'<') {
        return None;
    }
    let mut i = start + 1;
    let closing = bytes.get(i) == Some(&b'/');
    if closing {
        i += 1;
    }
    if !bytes.get(i).copied().is_some_and(is_name_start) {
        return None;
    }
    let name_start = i;
    while bytes.get(i).copied().is_some_and(is_name_char) {
        i += 1;
    }
    let name_end = i;
    if i < bytes.len() && bytes[i] != b'>' && !is_space(bytes[i]) {
        return None;
    }
    let mut quote = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'<' {
            return None;
        }
        if quote != 0 {
            if c == quote {
                quote = 0;
            }
        } else if matches!(c, b'\'' | b'"') {
            quote = c;
        } else if c == b'>' {
            return Some(Token {
                name: &source[name_start..name_end],
                attrs: &source[name_end..i],
                closing,
                end: i + 1,
            });
        }
        i += 1;
    }
    None
}

fn decode_entities(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut i = 0;
    while i < value.len() {
        let rest = &value[i..];
        let (replacement, length) = if rest.starts_with("&lt;") {
            (Some('<'), 4)
        } else if rest.starts_with("&gt;") {
            (Some('>'), 4)
        } else if rest.starts_with("&amp;") {
            (Some('&'), 5)
        } else {
            let ch = value[i..]
                .chars()
                .next()
                .expect("i is a character boundary");
            (None, ch.len_utf8())
        };
        if let Some(ch) = replacement {
            output.push(ch);
            i += length;
        } else {
            let ch = value[i..]
                .chars()
                .next()
                .expect("i is a character boundary");
            output.push(ch);
            i += length;
        }
    }
    output
}

fn anchor_attributes(source: &str) -> Vec<HtmlAttribute> {
    let bytes = source.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let space_start = i;
        while i < bytes.len() && is_space(bytes[i]) {
            i += 1;
        }
        if i == space_start || i >= bytes.len() || !is_name_start(bytes[i]) {
            break;
        }
        let start = i;
        while i < bytes.len() && is_name_char(bytes[i]) {
            i += 1;
        }
        let name_end = i;
        let name = lower_ascii(&source[start..i]);
        while i < bytes.len() && is_space(bytes[i]) {
            i += 1;
        }
        let value = if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && is_space(bytes[i]) {
                i += 1;
            }
            attr_value(source, &mut i)
        } else {
            i = name_end;
            None
        };
        if matches!(name.as_str(), "href" | "name" | "type")
            && !result
                .iter()
                .any(|attribute: &HtmlAttribute| attribute.name == name)
        {
            result.push(HtmlAttribute { name, value });
        }
    }
    result
}

fn attr_value(source: &str, i: &mut usize) -> Option<String> {
    let bytes = source.as_bytes();
    if *i >= bytes.len() {
        return None;
    }
    if matches!(bytes[*i], b'\'' | b'"') {
        let quote = bytes[*i];
        *i += 1;
        let start = *i;
        while *i < bytes.len() && bytes[*i] != quote {
            *i += 1;
        }
        if *i == bytes.len() {
            return None;
        }
        let value = decode_entities(&source[start..*i]);
        *i += 1;
        return Some(value);
    }
    let start = *i;
    while *i < bytes.len()
        && !is_space(bytes[*i])
        && !matches!(bytes[*i], b'"' | b'\'' | b'=' | b'<' | b'>')
    {
        *i += 1;
    }
    (*i > start).then(|| decode_entities(&source[start..*i]))
}

fn find_ascii_ignore_case(source: &str, target: &str, from: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let needle = target.as_bytes();
    let last = bytes.len().checked_sub(needle.len())?;
    if from > last {
        return None;
    }
    (from..=last).find(|index| bytes[*index..*index + needle.len()].eq_ignore_ascii_case(needle))
}

fn is_empty_tag(tag: &str) -> bool {
    matches!(tag, "p" | "isindex")
}

fn append_node(document: &mut HtmlDocument, stack: &[NodeId], node: HtmlNode) -> NodeId {
    let id = document.nodes.len();
    document.nodes.push(node);
    if let Some(parent) = stack.last().copied() {
        if let HtmlNodeKind::Element { children, .. } = &mut document.nodes[parent].kind {
            children.push(id);
        }
    } else {
        document.roots.push(id);
    }
    id
}

fn append_text(document: &mut HtmlDocument, stack: &[NodeId], value: &str, decode: bool) {
    if value.is_empty() {
        return;
    }
    let text = if decode {
        decode_entities(value)
    } else {
        value.to_owned()
    };
    let parent = stack.last().copied();
    let last = if let Some(parent) = parent {
        match &document.nodes[parent].kind {
            HtmlNodeKind::Element { children, .. } => children.last().copied(),
            HtmlNodeKind::Text(_) => return,
        }
    } else {
        document.roots.last().copied()
    };
    if let Some(last) = last
        && let HtmlNodeKind::Text(existing) = &mut document.nodes[last].kind
    {
        existing.push_str(&text);
        return;
    }
    let id = document.nodes.len();
    document.nodes.push(HtmlNode {
        kind: HtmlNodeKind::Text(text),
    });
    if let Some(parent) = parent {
        if let HtmlNodeKind::Element { children, .. } = &mut document.nodes[parent].kind {
            children.push(id);
        }
    } else {
        document.roots.push(id);
    }
}

fn matching_stack_position(
    positions: &HashMap<String, Vec<usize>>,
    closing: &str,
) -> Option<usize> {
    positions
        .get(closing)
        .and_then(|values| values.last().copied())
}

fn truncate_stack(
    stack: &mut Vec<NodeId>,
    positions: &mut HashMap<String, Vec<usize>>,
    at: usize,
    document: &HtmlDocument,
) {
    while stack.len() > at {
        let id = stack.pop().expect("length checked");
        if let HtmlNodeKind::Element { tag, .. } = &document.nodes[id].kind
            && let Some(values) = positions.get_mut(tag)
        {
            values.pop();
        }
    }
}

/// Parse the archived 1992 tag vocabulary into a flat, indexed arena.
pub fn parse_html_tags_1992(input: &str) -> Result<HtmlDocument, HtmlParseError> {
    // Normalize source-wide, including inside quoted attribute values.
    let source = input.replace("\r\n", "\n");
    let mut document = HtmlDocument::default();
    let mut stack: Vec<NodeId> = Vec::new();
    let mut positions: HashMap<String, Vec<usize>> = HashMap::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    let mut text_start = 0;
    while i < bytes.len() {
        let relative = match bytes[i..].iter().position(|byte| *byte == b'<') {
            Some(value) => value,
            None => {
                append_text(&mut document, &stack, &source[text_start..], true);
                break;
            }
        };
        let start = i + relative;
        if start > text_start {
            append_text(&mut document, &stack, &source[text_start..start], true);
        }
        if source[start..].starts_with("<!--") {
            if let Some(end) = source[start + 4..].find("-->") {
                i = start + 4 + end + 3;
                text_start = i;
                continue;
            }
            return Ok(document);
        }
        let Some(token) = tag_at(&source, start) else {
            append_text(&mut document, &stack, "<", false);
            i = start + 1;
            text_start = i;
            continue;
        };
        let name = lower_ascii(token.name);
        if !TAGS.contains(&name.as_str()) {
            i = token.end;
            text_start = i;
            continue;
        }
        if !token.closing && name == "plaintext" {
            let id = append_node(
                &mut document,
                &stack,
                HtmlNode {
                    kind: HtmlNodeKind::Element {
                        tag: name,
                        attributes: Vec::new(),
                        children: Vec::new(),
                    },
                },
            );
            append_text(&mut document, &[id], &source[token.end..], false);
            return Ok(document);
        }
        if !token.closing && matches!(name.as_str(), "xmp" | "listing") {
            let id = append_node(
                &mut document,
                &stack,
                HtmlNode {
                    kind: HtmlNodeKind::Element {
                        tag: name.clone(),
                        attributes: Vec::new(),
                        children: Vec::new(),
                    },
                },
            );
            let needle = format!("</{name}>");
            if let Some(end) = find_ascii_ignore_case(&source, &needle, token.end) {
                append_text(&mut document, &[id], &source[token.end..end], false);
                i = end + needle.len();
                text_start = i;
                continue;
            }
            append_text(&mut document, &[id], &source[token.end..], false);
            return Ok(document);
        }
        if token.closing {
            if !is_empty_tag(&name)
                && let Some(at) = matching_stack_position(&positions, &name)
            {
                truncate_stack(&mut stack, &mut positions, at, &document);
            }
            i = token.end;
            text_start = i;
            continue;
        }
        if is_empty_tag(&name) {
            append_node(
                &mut document,
                &stack,
                HtmlNode {
                    kind: HtmlNodeKind::Element {
                        tag: name,
                        attributes: Vec::new(),
                        children: Vec::new(),
                    },
                },
            );
            i = token.end;
            text_start = i;
            continue;
        }
        let mut close_candidates = Vec::new();
        let mut boundaries = Vec::new();
        if name == "li" {
            close_candidates.push("li");
            boundaries.extend(["ul", "menu", "dir"]);
        } else if matches!(name.as_str(), "dt" | "dd") {
            close_candidates.extend(["dt", "dd"]);
            boundaries.push("dl");
        } else if matches!(name.as_str(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
            close_candidates.extend(["h1", "h2", "h3", "h4", "h5", "h6"]);
        }
        let latest_match = close_candidates
            .iter()
            .filter_map(|tag| matching_stack_position(&positions, tag))
            .max();
        let latest_boundary = boundaries
            .iter()
            .filter_map(|tag| matching_stack_position(&positions, tag))
            .max();
        if let Some(at) =
            latest_match.filter(|at| latest_boundary.is_none_or(|boundary| *at > boundary))
        {
            truncate_stack(&mut stack, &mut positions, at, &document);
        }
        let attributes = if name == "a" {
            anchor_attributes(token.attrs)
        } else {
            Vec::new()
        };
        let id = append_node(
            &mut document,
            &stack,
            HtmlNode {
                kind: HtmlNodeKind::Element {
                    tag: name.clone(),
                    attributes,
                    children: Vec::new(),
                },
            },
        );
        positions.entry(name).or_default().push(stack.len());
        stack.push(id);
        i = token.end;
        text_start = i;
    }
    // Every open element is implicitly closed at EOF; the arena needs no
    // pointer rewrite because child relationships were recorded on insertion.
    stack.clear();
    if document.roots.iter().any(|id| *id >= document.nodes.len()) {
        return Err(HtmlParseError::InvalidTreeState);
    }
    Ok(document)
}
