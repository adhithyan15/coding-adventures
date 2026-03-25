//! Block-Level Parser
//!
//! Phase 1 of CommonMark parsing: split the input into block-level tokens
//! and build the structural skeleton of the document.
//!
//! # Two-Phase Overview
//!
//! CommonMark parsing is inherently two-phase:
//!
//!   Phase 1 (this file): Block structure
//!     Input text → lines → block tree with raw inline content strings
//!
//!   Phase 2 (inline_parser.rs): Inline content
//!     Each block's raw content → inline nodes (emphasis, links, etc.)
//!
//! # Block Tree Construction
//!
//! Container blocks (document, blockquote, list items) form a stack.
//! When a new line arrives, we walk down the stack checking continuations,
//! then add the line's content to the appropriate block.
//!
//! # Internal representation
//!
//! We use a tree of `Node` values identified by `NodeId` (usize). Each node
//! stores its kind (document, blockquote, list, etc.) and its children as
//! a `Vec<NodeId>`. This avoids lifetime/borrow issues with mutable trees.

use std::collections::HashMap;
use crate::scanner::{normalize_link_label, normalize_url, is_ascii_punctuation};
use crate::entities::decode_entities;

// ─── Link Reference Map ────────────────────────────────────────────────────────

/// A resolved link reference definition.
#[derive(Debug, Clone)]
pub struct LinkReference {
    pub destination: String,
    pub title: Option<String>,
}

/// Map of normalized link labels → link reference data.
pub type LinkRefMap = HashMap<String, LinkReference>;

// ─── Final Block Types ────────────────────────────────────────────────────────
//
// These are the types consumed by the inline parser and HTML renderer.

/// A parsed block node from Phase 1.
///
/// Heading and Paragraph carry raw inline content strings that Phase 2
/// will parse into inline nodes. All other block types are fully resolved.
#[derive(Debug, Clone)]
pub enum FinalBlock {
    Heading { level: u8, raw_content: String },
    Paragraph { raw_content: String },
    CodeBlock { language: Option<String>, value: String },
    Blockquote { children: Vec<FinalBlock> },
    List { ordered: bool, start: Option<i64>, tight: bool, items: Vec<FinalBlock> },
    ListItem { children: Vec<FinalBlock> },
    HtmlBlock { value: String },
    ThematicBreak,
}

// ─── Internal Node Arena ──────────────────────────────────────────────────────

type NodeId = usize;

#[derive(Debug)]
enum NodeKind {
    Document,
    Blockquote,
    List {
        ordered: bool,
        marker: char,
        start: i64,
        tight: bool,
        had_blank_line: bool,
    },
    ListItem {
        content_indent: usize,
        had_blank_line: bool,
    },
    Paragraph { lines: Vec<String> },
    FencedCode {
        fence_char: char,
        fence_len: usize,
        base_indent: usize,
        info_string: String,
        lines: Vec<String>,
    },
    IndentedCode { lines: Vec<String> },
    HtmlBlock { html_type: u8, lines: Vec<String> },
    Heading { level: u8, content: String },
    ThematicBreak,
}

struct Node {
    kind: NodeKind,
    children: Vec<NodeId>,
    parent: Option<NodeId>,
}

struct Arena {
    nodes: Vec<Node>,
}

impl Arena {
    fn new() -> Self {
        Arena { nodes: Vec::new() }
    }

    fn alloc(&mut self, kind: NodeKind, parent: Option<NodeId>) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(Node { kind, children: Vec::new(), parent });
        id
    }

    fn get(&self, id: NodeId) -> &Node {
        &self.nodes[id]
    }

    fn get_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.nodes[id]
    }

    fn add_child(&mut self, parent: NodeId, child: NodeId) {
        self.nodes[parent].children.push(child);
    }

    fn last_child(&self, parent: NodeId) -> Option<NodeId> {
        self.nodes[parent].children.last().copied()
    }

    fn remove_last_child(&mut self, parent: NodeId) -> Option<NodeId> {
        self.nodes[parent].children.pop()
    }
}

// ─── Parser Mode ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum ParserMode {
    Normal,
    Fenced,
    Html,
}

// ─── HTML Block Detection ─────────────────────────────────────────────────────

const HTML_BLOCK_6_TAGS: &[&str] = &[
    "address", "article", "aside", "base", "basefont", "blockquote", "body",
    "caption", "center", "col", "colgroup", "dd", "details", "dialog", "dir",
    "div", "dl", "dt", "fieldset", "figcaption", "figure", "footer", "form",
    "frame", "frameset", "h1", "h2", "h3", "h4", "h5", "h6", "head", "header",
    "hr", "html", "iframe", "legend", "li", "link", "main", "menu", "menuitem",
    "meta", "nav", "noframes", "ol", "optgroup", "option", "p", "param",
    "search", "section", "summary", "table", "tbody", "td", "tfoot", "th",
    "thead", "title", "tr", "track", "ul",
];

fn detect_html_block_type(line: &str) -> Option<u8> {
    let s = line.trim_start();

    // Type 1: <script|pre|textarea|style followed by whitespace, >, or end
    if s.starts_with('<') {
        let rest = &s[1..];
        let tag_end = rest.find(|c: char| !c.is_alphanumeric()).unwrap_or(rest.len());
        let tag = rest[..tag_end].to_ascii_lowercase();
        if matches!(tag.as_str(), "script" | "pre" | "textarea" | "style") {
            let after = &rest[tag_end..];
            if after.is_empty() || after.starts_with(|c: char| c.is_ascii_whitespace() || c == '>') {
                return Some(1);
            }
        }
    }
    if s.starts_with("<!--") { return Some(2); }
    if s.starts_with("<?") { return Some(3); }
    if s.starts_with("<!") && s[2..].starts_with(|c: char| c.is_ascii_uppercase()) { return Some(4); }
    if s.starts_with("<![CDATA[") { return Some(5); }

    // Type 6: block-level open/close tag
    if let Some(tag) = html_tag_name(s) {
        if HTML_BLOCK_6_TAGS.binary_search(&tag.to_ascii_lowercase().as_str()).is_ok()
            || HTML_BLOCK_6_TAGS.contains(&tag.to_ascii_lowercase().as_str())
        {
            return Some(6);
        }
    }

    // Type 7: complete open tag or close tag (checked last)
    if is_type7_html(s) { return Some(7); }

    None
}

fn html_tag_name(s: &str) -> Option<String> {
    if !s.starts_with('<') { return None; }
    let inner = if s[1..].starts_with('/') { &s[2..] } else { &s[1..] };
    let end = inner.find(|c: char| !c.is_alphanumeric() && c != '-').unwrap_or(inner.len());
    if end == 0 { return None; }
    let tag = &inner[..end];
    let rest = &inner[end..];
    if rest.is_empty() || rest.starts_with(|c: char| c.is_ascii_whitespace() || c == '>' || c == '/') {
        Some(tag.to_string())
    } else {
        None
    }
}

/// Check if a trimmed line is a CommonMark type 7 HTML block opener.
///
/// Type 7 HTML blocks are complete open or close tags where:
///   - Tag name: `[A-Za-z][A-Za-z0-9-]*` (only letters, digits, hyphens)
///   - Open tag: `<tagname(attrs)*\s*/?>` — ends with `>` or `/>`, entire line
///   - Close tag: `</tagname\s*>` — entire line
///
/// This EXCLUDES autolinks like `<http://foo.bar.baz>` because `http` is a
/// valid tag name but the URL path `://foo...` is not valid attribute syntax.
///
/// The regex equivalent (from the TypeScript reference implementation):
///   open:  `^<[A-Za-z][A-Za-z0-9-]*(ATTR)*\s*/?>$`
///   close: `^</[A-Za-z][A-Za-z0-9-]*\s*>$`
fn is_type7_html(s: &str) -> bool {
    let s = s.trim_end();

    // Closing tag: `</tagname\s*>` — whole line
    if s.starts_with("</") {
        let inner = &s[2..];
        if inner.is_empty() || !inner.starts_with(|c: char| c.is_ascii_alphabetic()) {
            return false;
        }
        // Consume tag name: [A-Za-z][A-Za-z0-9-]*
        let name_end = inner
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '-')
            .unwrap_or(inner.len());
        let after_name = &inner[name_end..];
        // After tag name: optional whitespace then exactly `>`
        return after_name.trim_start() == ">";
    }

    // Open tag: `<tagname(attrs)*\s*/?>` — whole line
    if !s.starts_with('<') { return false; }
    let inner = &s[1..];
    // Tag name must start with letter
    if !inner.starts_with(|c: char| c.is_ascii_alphabetic()) { return false; }
    // Consume tag name: [A-Za-z][A-Za-z0-9-]*
    let name_end = inner
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .unwrap_or(inner.len());
    let tag_name = &inner[..name_end];

    // The tag name must be followed only by: attributes, optional whitespace, `>` or `/>`
    // For `<http://foo.bar.baz>`, after the tag name "http" we get "://foo.bar.baz>"
    // which does NOT start with whitespace or `>` or `/>`.
    let after_name = &inner[name_end..];

    // Parse optional attributes
    let rest = parse_type7_attrs(after_name);
    // After attributes: optional whitespace then `>` or `/>`
    let rest = rest.trim_start();
    rest == ">" || rest == "/>"
}

/// Consume zero or more type 7 HTML attributes from `s`.
/// Returns the remaining string after all attributes.
///
/// Each attribute: `\s+[a-zA-Z_:][a-zA-Z0-9_:.-]*(\s*=\s*VALUE)?`
/// VALUE: unquoted | single-quoted | double-quoted
fn parse_type7_attrs(s: &str) -> &str {
    let mut pos = s;
    loop {
        // Must start with at least one whitespace
        let trimmed = pos.trim_start_matches(|c: char| c == ' ' || c == '\t');
        if trimmed.len() == pos.len() {
            // No leading whitespace — no more attributes
            break;
        }
        // Attribute name: [a-zA-Z_:][a-zA-Z0-9_:.-]*
        if !trimmed.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == ':') {
            break;
        }
        let name_end = trimmed
            .find(|c: char| {
                !c.is_ascii_alphanumeric() && c != '_' && c != ':' && c != '.' && c != '-'
            })
            .unwrap_or(trimmed.len());
        if name_end == 0 { break; }
        let after_name = &trimmed[name_end..];

        // Optional: `\s*=\s*VALUE`
        let after_eq = after_name.trim_start();
        if after_eq.starts_with('=') {
            let after_eq = &after_eq[1..].trim_start();
            if after_eq.starts_with('"') {
                // Double-quoted: "[^"\n]*"
                let rest = &after_eq[1..];
                if let Some(end) = rest.find(|c| c == '"' || c == '\n') {
                    if rest.as_bytes()[end] == b'"' {
                        pos = &rest[end + 1..];
                        continue;
                    }
                }
                break; // unclosed or contains newline
            } else if after_eq.starts_with('\'') {
                // Single-quoted: '[^'\n]*'
                let rest = &after_eq[1..];
                if let Some(end) = rest.find(|c| c == '\'' || c == '\n') {
                    if rest.as_bytes()[end] == b'\'' {
                        pos = &rest[end + 1..];
                        continue;
                    }
                }
                break; // unclosed
            } else {
                // Unquoted: [^ \t"'=<>`]+
                let unq_end = after_eq
                    .find(|c: char| {
                        c == ' ' || c == '\t' || c == '"' || c == '\'' ||
                        c == '=' || c == '<' || c == '>' || c == '`'
                    })
                    .unwrap_or(after_eq.len());
                if unq_end == 0 { break; }
                pos = &after_eq[unq_end..];
                continue;
            }
        } else {
            pos = after_name;
        }
    }
    pos
}

fn html_block_ends(line: &str, html_type: u8) -> bool {
    match html_type {
        1 => {
            let l = line.to_ascii_lowercase();
            l.contains("</script>") || l.contains("</pre>") ||
            l.contains("</textarea>") || l.contains("</style>")
        }
        2 => line.contains("-->") || line.contains("--!>"),
        3 => line.contains("?>"),
        4 => line.contains('>'),
        5 => line.contains("]]>"),
        6 | 7 => line.trim().is_empty(),
        _ => false,
    }
}

// ─── Line Classification ──────────────────────────────────────────────────────

fn is_blank(line: &str) -> bool {
    line.bytes().all(|b| b == b' ' || b == b'\t' || b == b'\r')
}

fn indent_of(line: &str, base_col: usize) -> usize {
    let mut col = base_col;
    for ch in line.chars() {
        match ch {
            ' ' => col += 1,
            '\t' => col += 4 - (col % 4),
            _ => break,
        }
    }
    col - base_col
}

fn strip_indent(line: &str, n: usize, base_col: usize) -> (String, usize) {
    if n == 0 { return (line.to_string(), base_col); }
    let mut remaining = n;
    let mut col = base_col;
    let mut i = 0;
    let bytes = line.as_bytes();

    while remaining > 0 && i < bytes.len() {
        match bytes[i] {
            b' ' => { i += 1; remaining -= 1; col += 1; }
            b'\t' => {
                let w = 4 - (col % 4);
                if w <= remaining {
                    i += 1; remaining -= w; col += w;
                } else {
                    let leftover = w - remaining;
                    return (format!("{}{}", " ".repeat(leftover), &line[i + 1..]), col + remaining);
                }
            }
            _ => break,
        }
    }
    (line[i..].to_string(), col)
}

fn virtual_col_after(line: &str, char_count: usize, start_col: usize) -> usize {
    let mut col = start_col;
    for (i, ch) in line.chars().enumerate() {
        if i >= char_count { break; }
        col += if ch == '\t' { 4 - (col % 4) } else { 1 };
    }
    col
}

// ─── ATX Heading ─────────────────────────────────────────────────────────────

fn parse_atx_heading(line: &str) -> Option<(u8, String)> {
    let spaces = line.bytes().take_while(|&b| b == b' ').count();
    if spaces > 3 { return None; }
    let rest = &line[spaces..];

    let hashes = rest.bytes().take_while(|&b| b == b'#').count();
    if hashes == 0 || hashes > 6 { return None; }

    let after = &rest[hashes..];
    if after.is_empty() {
        return Some((hashes as u8, String::new()));
    }
    let fc = after.chars().next().unwrap();
    if fc != ' ' && fc != '\t' { return None; }

    let mut content = after.trim_end().to_string();

    // Remove trailing closing sequence: space/tab + hashes + optional spaces
    let content_trim = content.trim_end();
    let trailing_hashes = content_trim.bytes().rev().take_while(|&b| b == b'#').count();
    if trailing_hashes > 0 {
        let len = content_trim.len();
        let before_hashes = &content_trim[..len - trailing_hashes];
        if before_hashes.is_empty() || before_hashes.ends_with(|c: char| c == ' ' || c == '\t') {
            content = before_hashes.trim_end().to_string();
        }
    }

    Some((hashes as u8, content.trim().to_string()))
}

// ─── Thematic Break ───────────────────────────────────────────────────────────

fn is_thematic_break(line: &str) -> bool {
    let spaces = line.bytes().take_while(|&b| b == b' ').count();
    if spaces > 3 { return false; }
    let rest = &line[spaces..];
    if rest.is_empty() { return false; }

    let marker = rest.chars().next().unwrap();
    if !matches!(marker, '*' | '-' | '_') { return false; }

    let mut count = 0usize;
    for ch in rest.chars() {
        if ch == marker { count += 1; }
        else if ch != ' ' && ch != '\t' { return false; }
    }
    count >= 3
}

// ─── Setext Heading ───────────────────────────────────────────────────────────

fn is_setext_underline(line: &str) -> Option<u8> {
    let spaces = line.bytes().take_while(|&b| b == b' ').count();
    if spaces > 3 { return None; }
    let rest = line[spaces..].trim_end();
    if rest.is_empty() { return None; }
    if rest.chars().all(|c| c == '=') { return Some(1); }
    if rest.chars().all(|c| c == '-') { return Some(2); }
    None
}

// ─── List Marker ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ListMarker {
    pub ordered: bool,
    pub start: i64,
    pub marker: char,
    pub marker_len: usize, // bytes consumed by marker + spaces
    pub space_after: usize,
    pub indent: usize,
}

pub fn parse_list_marker(line: &str) -> Option<ListMarker> {
    let indent = line.bytes().take_while(|&b| b == b' ').count();
    if indent > 3 { return None; }
    let rest = &line[indent..];
    if rest.is_empty() { return None; }

    let first = rest.chars().next().unwrap();

    if matches!(first, '-' | '*' | '+') {
        let after = &rest[1..];
        if after.is_empty() {
            return Some(ListMarker { ordered: false, start: 1, marker: first,
                marker_len: indent + 1, space_after: 0, indent });
        }
        let nc = after.chars().next().unwrap();
        if nc == ' ' || nc == '\t' {
            let space_after = if nc == '\t' { 1 } else { after.bytes().take_while(|&b| b == b' ').count() };
            let space_bytes = if nc == '\t' { 1 } else { space_after };
            return Some(ListMarker { ordered: false, start: 1, marker: first,
                marker_len: indent + 1 + space_bytes, space_after, indent });
        }
        return None;
    }

    // Ordered
    let digs: Vec<u8> = rest.bytes().take_while(|b| b.is_ascii_digit()).collect();
    if digs.is_empty() || digs.len() > 9 { return None; }
    let after_digs = &rest[digs.len()..];
    if after_digs.is_empty() { return None; }
    let delim = after_digs.chars().next().unwrap();
    if !matches!(delim, '.' | ')') { return None; }
    let num: i64 = String::from_utf8_lossy(&digs).parse().ok()?;
    let after_delim = &after_digs[1..];

    if after_delim.is_empty() {
        return Some(ListMarker { ordered: true, start: num, marker: delim,
            marker_len: indent + digs.len() + 1, space_after: 0, indent });
    }
    let nc = after_delim.chars().next().unwrap();
    if nc == ' ' || nc == '\t' {
        let space_after = if nc == '\t' { 1 } else { after_delim.bytes().take_while(|&b| b == b' ').count() };
        let space_bytes = if nc == '\t' { 1 } else { space_after };
        return Some(ListMarker { ordered: true, start: num, marker: delim,
            marker_len: indent + digs.len() + 1 + space_bytes, space_after, indent });
    }
    None
}

// ─── Link Reference Definition ────────────────────────────────────────────────

struct ParsedLinkDef {
    label: String,
    destination: String,
    title: Option<String>,
    chars_consumed: usize,
}

fn apply_backslash_escapes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next) = chars.peek() {
                if is_ascii_punctuation(next) {
                    result.push(next);
                    chars.next();
                    continue;
                }
            }
            result.push('\\');
        } else {
            result.push(ch);
        }
    }
    result
}

fn extract_info_string(line: &str) -> String {
    let s = line.trim_start();
    let fc = s.chars().next().unwrap_or(' ');
    let fence_end = s.bytes().take_while(|&b| b == fc as u8).count();
    let after = s[fence_end..].trim();
    let first_word = after.split_whitespace().next().unwrap_or("");
    decode_entities(&apply_backslash_escapes(first_word))
}

fn parse_link_definition(text: &str) -> Option<ParsedLinkDef> {
    let bytes = text.as_bytes();
    let mut pos = 0usize;

    // Up to 3 leading spaces
    while pos < 3.min(bytes.len()) && bytes[pos] == b' ' { pos += 1; }
    if pos >= bytes.len() || bytes[pos] != b'[' { return None; }
    pos += 1;

    let label_start = pos;
    loop {
        if pos >= bytes.len() { return None; }
        match bytes[pos] {
            b']' => break,
            b'[' => return None,
            b'\\' => { pos += 2; if pos > bytes.len() { return None; } }
            _ => pos += 1,
        }
    }
    let raw_label = &text[label_start..pos];
    if raw_label.trim().is_empty() { return None; }
    let label = normalize_link_label(raw_label);
    pos += 1; // skip ]

    if pos >= bytes.len() || bytes[pos] != b':' { return None; }
    pos += 1; // skip :

    // Skip ws + optional newline + ws
    while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') { pos += 1; }
    if pos < bytes.len() && bytes[pos] == b'\n' {
        pos += 1;
        while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') { pos += 1; }
    }
    if pos >= bytes.len() { return None; }

    // Destination
    let destination;
    if bytes[pos] == b'<' {
        pos += 1;
        let ds = pos;
        loop {
            if pos >= bytes.len() { return None; }
            match bytes[pos] {
                b'>' => break,
                b'\n' | b'<' => return None,
                b'\\' => { pos += 2; }
                _ => pos += 1,
            }
        }
        destination = normalize_url(&decode_entities(&apply_backslash_escapes(&text[ds..pos])));
        pos += 1;
    } else {
        let ds = pos;
        let mut depth = 0i32;
        loop {
            if pos >= bytes.len() { break; }
            match bytes[pos] {
                b'(' => { depth += 1; pos += 1; }
                b')' => { if depth == 0 { break; } depth -= 1; pos += 1; }
                b' ' | b'\t' | b'\n' | b'\r' | 0x00..=0x1f => break,
                b'\\' => { pos += 2; }
                _ => pos += 1,
            }
        }
        if pos == ds { return None; }
        destination = normalize_url(&decode_entities(&apply_backslash_escapes(&text[ds..pos])));
    }

    let before_title = pos;
    let mut title: Option<String> = None;

    let mut tpos = pos;
    while tpos < bytes.len() && (bytes[tpos] == b' ' || bytes[tpos] == b'\t') { tpos += 1; }
    let had_space = tpos > pos;
    let mut had_newline = false;
    if tpos < bytes.len() && bytes[tpos] == b'\n' {
        tpos += 1; had_newline = true;
        while tpos < bytes.len() && (bytes[tpos] == b' ' || bytes[tpos] == b'\t') { tpos += 1; }
    }

    // A title can appear on the same line (after spaces) OR on the next line.
    // CommonMark spec §4.7: "The title may extend over multiple lines."
    if (had_space || had_newline) && tpos < bytes.len() {
        let title_ch = bytes[tpos];
        let close = match title_ch {
            b'"' => Some(b'"'), b'\'' => Some(b'\''), b'(' => Some(b')'), _ => None,
        };
        if let Some(close_ch) = close {
            tpos += 1;
            let ts = tpos;
            let mut esc = false;
            let mut ok = false;
            while tpos < bytes.len() {
                let b = bytes[tpos];
                if esc { esc = false; tpos += 1; continue; }
                if b == b'\\' { esc = true; tpos += 1; continue; }
                if b == close_ch { tpos += 1; ok = true; break; }
                if b == b'\n' && title_ch == b'(' { break; }
                tpos += 1;
            }
            if ok {
                title = Some(decode_entities(&apply_backslash_escapes(&text[ts..tpos - 1])));
                pos = tpos;
            }
        }
    }

    // EOL check
    let eol_start = pos;
    while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') { pos += 1; }
    if pos < bytes.len() && bytes[pos] == b'\n' {
        pos += 1;
    } else if pos < bytes.len() {
        // Try without title
        if title.is_some() {
            pos = before_title;
            title = None;
            while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') { pos += 1; }
            if pos < bytes.len() && bytes[pos] == b'\n' {
                pos += 1;
            } else if pos < bytes.len() {
                return None;
            }
        } else {
            return None;
        }
    }

    Some(ParsedLinkDef { label, destination, title, chars_consumed: pos })
}

// ─── Main Parser ──────────────────────────────────────────────────────────────

pub struct BlockParser {
    arena: Arena,
    root: NodeId,
    // Stack of open container nodes (root is always the bottom)
    open_containers: Vec<NodeId>,
    current_leaf: Option<NodeId>,
    link_refs: LinkRefMap,
    mode: ParserMode,
    last_line_was_blank: bool,
    last_blank_container: NodeId, // innermost container during last blank line
}

impl BlockParser {
    pub fn new() -> Self {
        let mut arena = Arena::new();
        let root = arena.alloc(NodeKind::Document, None);
        BlockParser {
            root,
            open_containers: vec![root],
            current_leaf: None,
            link_refs: HashMap::new(),
            mode: ParserMode::Normal,
            last_line_was_blank: false,
            last_blank_container: root,
            arena,
        }
    }

    fn inner_container(&self) -> NodeId {
        *self.open_containers.last().unwrap()
    }

    fn add_child_to_container(&mut self, container: NodeId, child: NodeId) {
        self.arena.add_child(container, child);
    }

    fn alloc_node(&mut self, kind: NodeKind, parent: NodeId) -> NodeId {
        let id = self.arena.alloc(kind, Some(parent));
        id
    }

    fn push_leaf(&mut self, container: NodeId, kind: NodeKind) -> NodeId {
        let id = self.alloc_node(kind, container);
        self.add_child_to_container(container, id);
        id
    }

    /// Finalize and close the current leaf block.
    fn close_current_leaf(&mut self) {
        if let Some(leaf_id) = self.current_leaf.take() {
            // If the closed leaf is a fenced code block or HTML block, reset the
            // parser mode to Normal so subsequent lines are not misinterpreted
            // as belonging to the now-closed block.
            match &self.arena.get(leaf_id).kind {
                NodeKind::FencedCode { .. } => { self.mode = ParserMode::Normal; }
                NodeKind::HtmlBlock { .. } => { self.mode = ParserMode::Normal; }
                _ => {}
            }
            self.finalize_leaf(leaf_id);
        }
    }

    fn close_paragraph_or_icb(&mut self) {
        if let Some(leaf_id) = self.current_leaf {
            match &self.arena.get(leaf_id).kind {
                NodeKind::Paragraph { .. } | NodeKind::IndentedCode { .. } => {
                    self.current_leaf = None;
                    self.finalize_leaf(leaf_id);
                }
                _ => {}
            }
        }
    }

    fn finalize_leaf(&mut self, leaf_id: NodeId) {
        match &self.arena.get(leaf_id).kind {
            NodeKind::Paragraph { .. } => {
                // Extract link reference definitions
                let lines = if let NodeKind::Paragraph { lines } = &self.arena.get(leaf_id).kind {
                    lines.clone()
                } else { vec![] };
                self.finalize_paragraph_lines(leaf_id, lines);
            }
            NodeKind::IndentedCode { .. } => {
                // Trim trailing blank lines
                if let NodeKind::IndentedCode { lines } = &mut self.arena.get_mut(leaf_id).kind {
                    while lines.last().map_or(false, |l: &String| l.trim().is_empty()) {
                        lines.pop();
                    }
                }
            }
            _ => {} // Other leaf types are already finalized
        }
    }

    fn finalize_paragraph_lines(&mut self, node_id: NodeId, lines: Vec<String>) {
        let mut text = lines.join("\n");
        loop {
            match parse_link_definition(&text) {
                Some(def) => {
                    if !self.link_refs.contains_key(&def.label) {
                        self.link_refs.insert(def.label.clone(), LinkReference {
                            destination: def.destination,
                            title: def.title,
                        });
                    }
                    text = text[def.chars_consumed..].to_string();
                }
                None => break,
            }
        }

        if text.trim().is_empty() {
            // Remove the paragraph node — all content was link defs
            let parent = self.arena.get(node_id).parent.unwrap_or(self.root);
            self.arena.remove_last_child(parent);
        } else {
            let remaining: Vec<String> = text.split('\n').map(str::to_string).collect();
            if let NodeKind::Paragraph { lines } = &mut self.arena.get_mut(node_id).kind {
                *lines = remaining;
                // Trim trailing whitespace from last line
                if let Some(last) = lines.last_mut() {
                    *last = last.trim_end().to_string();
                }
            }
        }
    }

    pub fn process_line(&mut self, raw_line: &str) {
        let orig_blank = is_blank(raw_line);
        let mut line_content = raw_line.to_string();
        let mut line_base_col = 0usize;

        // ── Container continuation pass ────────────────────────────────────
        let mut new_containers: Vec<NodeId> = vec![self.root];
        let mut lazy_paragraph_continuation = false;
        let prev_inner = self.inner_container();

        let mut container_idx = 1usize;
        while container_idx < self.open_containers.len() {
            let cont_id = self.open_containers[container_idx];
            let kind_tag = self.container_kind_tag(cont_id);

            match kind_tag {
                KindTag::Blockquote => {
                    let mut bq_i = 0usize;
                    let mut bq_col = line_base_col;
                    let lb = line_content.as_bytes();
                    while bq_i < 3 && bq_i < lb.len() && lb[bq_i] == b' ' { bq_i += 1; bq_col += 1; }
                    if bq_i < lb.len() && lb[bq_i] == b'>' {
                        bq_i += 1; bq_col += 1;
                        if bq_i < lb.len() {
                            if lb[bq_i] == b' ' { bq_i += 1; bq_col += 1; }
                            else if lb[bq_i] == b'\t' {
                                let w = 4 - (bq_col % 4);
                                bq_i += 1;
                                if w > 1 {
                                    line_content = format!("{}{}", " ".repeat(w - 1), &line_content[bq_i..]);
                                    line_base_col = bq_col + 1;
                                    new_containers.push(cont_id);
                                    container_idx += 1;
                                    continue;
                                }
                                bq_col += w;
                            }
                        }
                        line_content = line_content[bq_i..].to_string();
                        line_base_col = bq_col;
                        new_containers.push(cont_id);
                        container_idx += 1;
                    } else if !orig_blank
                        && matches!(&self.current_leaf.map(|id| self.leaf_kind_tag(id)),
                            Some(LeafTag::Paragraph))
                        && !is_thematic_break(&line_content)
                        && parse_atx_heading(&line_content).is_none()
                        && !(indent_of(&line_content, line_base_col) < 4
                            && line_content.trim_start().starts_with(|c| c == '`' || c == '~'))
                    {
                        let lm = parse_list_marker(&line_content);
                        let lm_blank = lm.as_ref().map_or(false, |m| is_blank(&line_content[m.marker_len..]));
                        if lm.is_none() || lm_blank {
                            new_containers.push(cont_id);
                            container_idx += 1;
                            lazy_paragraph_continuation = true;
                            break;
                        }
                        break;
                    } else {
                        break;
                    }
                }
                KindTag::List => {
                    new_containers.push(cont_id);
                    container_idx += 1;
                }
                KindTag::ListItem => {
                    let effective_blank = orig_blank || is_blank(&line_content);
                    let ci = self.list_item_content_indent(cont_id);
                    let ind = indent_of(&line_content, line_base_col);
                    if !effective_blank && ind >= ci {
                        let (s, nc) = strip_indent(&line_content, ci, line_base_col);
                        line_content = s; line_base_col = nc;
                        new_containers.push(cont_id);
                        container_idx += 1;
                    } else if effective_blank {
                        let has_content = !self.arena.get(cont_id).children.is_empty()
                            || self.current_leaf.map_or(false, |_| true);
                        if has_content {
                            new_containers.push(cont_id);
                            container_idx += 1;
                        } else {
                            break;
                        }
                    } else if !orig_blank
                        && matches!(&self.current_leaf.map(|id| self.leaf_kind_tag(id)),
                            Some(LeafTag::Paragraph))
                        && !is_thematic_break(&line_content)
                        && parse_list_marker(&line_content).is_none()
                        && !(indent_of(&line_content, line_base_col) < 4
                            && line_content.trim_start().starts_with(|c| c == '`' || c == '~'))
                        && parse_atx_heading(&line_content).is_none()
                    {
                        new_containers.push(cont_id);
                        container_idx += 1;
                        lazy_paragraph_continuation = true;
                        break;
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        // Pop containers that did not continue
        let new_depth = new_containers.len();
        let old_depth = self.open_containers.len();
        if new_depth < old_depth {
            if !lazy_paragraph_continuation {
                self.close_current_leaf();
            }
        }
        self.open_containers = new_containers;

        let mut blank = orig_blank;
        if !blank && is_blank(&line_content) { blank = true; }

        let current_inner = self.inner_container();

        // ── Multi-line block continuation ─────────────────────────────────
        if self.mode == ParserMode::Fenced {
            if let Some(leaf_id) = self.current_leaf {
                if current_inner != prev_inner {
                    // Container dropped — force close fence
                    self.mode = ParserMode::Normal;
                    self.current_leaf = None;
                    // Fall through to normal block detection
                } else {
                    let (fc, fl, bi) = self.fenced_code_params(leaf_id);
                    let stripped = line_content.trim_start();
                    let ind = indent_of(&line_content, line_base_col);
                    let cchr = if fc == '`' { '`' } else { '~' };
                    let cnt = stripped.bytes().take_while(|&b| b == cchr as u8).count();
                    let is_close = ind < 4 && cnt >= fl
                        && stripped[cnt..].trim().is_empty()
                        && !stripped.starts_with(if fc == '`' { '~' } else { '`' });

                    if is_close {
                        self.mode = ParserMode::Normal;
                        self.current_leaf = None;
                    } else {
                        let (fline, _) = strip_indent(&line_content, bi, line_base_col);
                        if let NodeKind::FencedCode { lines, .. } = &mut self.arena.get_mut(leaf_id).kind {
                            lines.push(fline);
                        }
                    }
                    self.last_line_was_blank = orig_blank;
                    return;
                }
            }
        }

        if self.mode == ParserMode::Html {
            if let Some(leaf_id) = self.current_leaf {
                if current_inner != prev_inner {
                    self.mode = ParserMode::Normal;
                    self.current_leaf = None;
                    // Fall through
                } else {
                    let ht = self.html_block_type(leaf_id);
                    if let NodeKind::HtmlBlock { lines, .. } = &mut self.arena.get_mut(leaf_id).kind {
                        lines.push(line_content.clone());
                    }
                    if html_block_ends(&line_content, ht) {
                        self.mode = ParserMode::Normal;
                        self.current_leaf = None;
                    }
                    self.last_line_was_blank = orig_blank;
                    return;
                }
            }
        }

        // Finalize current leaf if container changed
        if current_inner != prev_inner && self.current_leaf.is_some() && !lazy_paragraph_continuation {
            let leaf = self.current_leaf.take().unwrap();
            self.finalize_leaf(leaf);
        }

        // ── Lazy paragraph continuation ──────────────────────────────────
        if lazy_paragraph_continuation {
            if let Some(leaf_id) = self.current_leaf {
                if let NodeKind::Paragraph { lines } = &mut self.arena.get_mut(leaf_id).kind {
                    lines.push(line_content);
                    self.last_line_was_blank = false;
                    return;
                }
            }
        }

        // Trim list containers that won't continue
        loop {
            if self.open_containers.len() <= 1 { break; }
            let inner = self.inner_container();
            if self.container_kind_tag(inner) != KindTag::List { break; }
            if blank { break; }
            let (lo, lm) = self.list_params(inner);
            let can_cont = parse_list_marker(&line_content)
                .map_or(false, |m| m.ordered == lo && m.marker == lm && !is_thematic_break(&line_content));
            if can_cont { break; }
            self.open_containers.pop();
            // Finalize the list
            // (list node stays in its parent's children array — already added when created)
        }

        let inner = self.inner_container();

        // ── Blank line ────────────────────────────────────────────────────
        if blank {
            if let Some(leaf_id) = self.current_leaf {
                match &self.arena.get(leaf_id).kind {
                    NodeKind::Paragraph { .. } => {
                        self.current_leaf = None;
                        self.finalize_leaf(leaf_id);
                    }
                    NodeKind::IndentedCode { .. } => {
                        let (stripped, _) = strip_indent(raw_line, 4, 0);
                        if let NodeKind::IndentedCode { lines } = &mut self.arena.get_mut(leaf_id).kind {
                            lines.push(stripped);
                        }
                    }
                    _ => {}
                }
            }

            // Mark blank for tightness tracking
            self.mark_blank_in_containers(inner);
            self.last_line_was_blank = true;
            self.last_blank_container = inner;
            return;
        }

        // ── New block detection (loop allows blockquote re-dispatch) ──────
        let mut inner = inner;
        'block: loop {
            // Tightness: after blank line in list context
            if self.last_line_was_blank {
                let lbk = self.container_kind_tag(self.last_blank_container);
                if lbk == KindTag::List || lbk == KindTag::ListItem {
                    // Find nearest list and make it loose
                    for &cid in self.open_containers.iter().rev() {
                        if self.container_kind_tag(cid) == KindTag::List {
                            if let NodeKind::List { tight, .. } = &mut self.arena.get_mut(cid).kind {
                                *tight = false;
                            }
                            break;
                        }
                    }
                }
                // Mark list item as having had a blank
                for &cid in self.open_containers.iter().rev() {
                    if self.container_kind_tag(cid) == KindTag::ListItem {
                        if let NodeKind::ListItem { had_blank_line, .. } = &mut self.arena.get_mut(cid).kind {
                            *had_blank_line = true;
                        }
                        break;
                    }
                }
            }

            let ind = indent_of(&line_content, line_base_col);

            // 1. Fenced code block
            if ind < 4 {
                let stripped = line_content.trim_start();
                if stripped.starts_with('`') || stripped.starts_with('~') {
                    let fc = stripped.chars().next().unwrap();
                    let fl = stripped.bytes().take_while(|&b| b == fc as u8).count();
                    if fl >= 3 {
                        let info_after = &stripped[fl..];
                        // Backtick fences cannot have backtick in info string
                        if fc != '`' || !info_after.contains('`') {
                            let info = extract_info_string(&line_content);
                            self.close_paragraph_or_icb();
                            let node = self.alloc_node(NodeKind::FencedCode {
                                fence_char: fc, fence_len: fl, base_indent: ind,
                                info_string: info, lines: Vec::new(),
                            }, inner);
                            self.add_child_to_container(inner, node);
                            self.current_leaf = Some(node);
                            self.mode = ParserMode::Fenced;
                            self.last_line_was_blank = false;
                            break 'block;
                        }
                    }
                }
            }

            // 2. ATX heading
            if ind < 4 {
                if let Some((level, content)) = parse_atx_heading(&line_content) {
                    self.close_paragraph_or_icb();
                    let node = self.alloc_node(NodeKind::Heading { level, content }, inner);
                    self.add_child_to_container(inner, node);
                    self.current_leaf = None;
                    self.last_line_was_blank = false;
                    break 'block;
                }
            }

            // 3. Thematic break
            if ind < 4 && is_thematic_break(&line_content) {
                // Check for setext heading (only when paragraph is open)
                if let Some(leaf_id) = self.current_leaf {
                    if let NodeKind::Paragraph { .. } = &self.arena.get(leaf_id).kind {
                        if let Some(level) = is_setext_underline(&line_content) {
                            let lines = if let NodeKind::Paragraph { lines } = &self.arena.get(leaf_id).kind {
                                lines.clone()
                            } else { vec![] };
                            self.finalize_paragraph_as_setext(leaf_id, lines, level, inner);
                            self.current_leaf = None;
                            self.last_line_was_blank = false;
                            break 'block;
                        }
                    }
                }
                self.close_paragraph_or_icb();
                let node = self.alloc_node(NodeKind::ThematicBreak, inner);
                self.add_child_to_container(inner, node);
                self.last_line_was_blank = false;
                break 'block;
            }

            // 4. Setext heading (when no thematic break matched)
            //
            // CommonMark spec §4.3: A setext heading underline only creates a
            // heading if the preceding paragraph has some non-link-definition
            // content remaining after extracting all link definitions.
            // Example: `[foo]: /url\n===` → `===` is NOT a setext underline
            // because the "paragraph" was entirely a link definition.
            if ind < 4 {
                if let Some(level) = is_setext_underline(&line_content) {
                    if let Some(leaf_id) = self.current_leaf {
                        if let NodeKind::Paragraph { .. } = &self.arena.get(leaf_id).kind {
                            let lines = if let NodeKind::Paragraph { lines } = &self.arena.get(leaf_id).kind {
                                lines.clone()
                            } else { vec![] };
                            // Check if paragraph content is entirely link definitions
                            let mut text = lines.join("\n");
                            let mut all_defs = true;
                            loop {
                                match parse_link_definition(&text) {
                                    Some(def) => { text = text[def.chars_consumed..].to_string(); }
                                    None => { all_defs = text.trim().is_empty(); break; }
                                }
                            }
                            if !all_defs {
                                // Has real content — treat as setext heading
                                let lines = if let NodeKind::Paragraph { lines } = &self.arena.get(leaf_id).kind {
                                    lines.clone()
                                } else { vec![] };
                                self.finalize_paragraph_as_setext(leaf_id, lines, level, inner);
                                self.current_leaf = None;
                                self.last_line_was_blank = false;
                                break 'block;
                            }
                            // Paragraph would be empty after link defs — fall through
                            // and let `===` start a new paragraph
                        }
                    }
                }
            }

            // 5. HTML block
            if ind < 4 {
                let is_in_para = self.current_leaf.map_or(false, |id| {
                    matches!(self.arena.get(id).kind, NodeKind::Paragraph { .. })
                });
                if let Some(html_type) = detect_html_block_type(&line_content) {
                    if html_type != 7 || !is_in_para {
                        self.close_paragraph_or_icb();
                        let already_closed = html_block_ends(&line_content, html_type);
                        let node = self.alloc_node(NodeKind::HtmlBlock {
                            html_type,
                            lines: vec![line_content.clone()],
                        }, inner);
                        self.add_child_to_container(inner, node);
                        if !already_closed {
                            self.current_leaf = Some(node);
                            self.mode = ParserMode::Html;
                        }
                        self.last_line_was_blank = false;
                        break 'block;
                    }
                }
            }

            // 6. Blockquote
            if ind < 4 && line_content.trim_start().starts_with('>') {
                self.close_paragraph_or_icb();

                // Continue existing blockquote if last child is one and no blank line
                let bq_id = if !self.last_line_was_blank {
                    if let Some(last_child) = self.arena.last_child(inner) {
                        if matches!(self.arena.get(last_child).kind, NodeKind::Blockquote) {
                            Some(last_child)
                        } else { None }
                    } else { None }
                } else { None };

                let bq_id = bq_id.unwrap_or_else(|| {
                    let node = self.alloc_node(NodeKind::Blockquote, inner);
                    self.add_child_to_container(inner, node);
                    node
                });

                self.open_containers.push(bq_id);

                // Strip > marker
                let lb = line_content.as_bytes();
                let mut bq_i = 0usize; let mut bq_col = line_base_col;
                while bq_i < lb.len() && lb[bq_i] == b' ' && bq_i < 3 { bq_i += 1; bq_col += 1; }
                if bq_i < lb.len() && lb[bq_i] == b'>' {
                    bq_i += 1; bq_col += 1;
                    if bq_i < lb.len() {
                        if lb[bq_i] == b' ' { bq_i += 1; bq_col += 1; }
                        else if lb[bq_i] == b'\t' {
                            let w = 4 - (bq_col % 4); bq_i += 1;
                            if w > 1 {
                                line_content = format!("{}{}", " ".repeat(w - 1), &line_content[bq_i..]);
                                line_base_col = bq_col + 1;
                                inner = bq_id;
                                if is_blank(&line_content) { self.last_line_was_blank = false; break 'block; }
                                continue 'block;
                            }
                            bq_col += w;
                        }
                    }
                }
                line_content = line_content[bq_i..].to_string();
                line_base_col = bq_col;
                inner = bq_id;

                if is_blank(&line_content) { self.last_line_was_blank = false; break 'block; }
                continue 'block;
            }

            // 7. List item
            if ind < 4 {
                if let Some(marker) = parse_list_marker(&line_content) {
                    let blank_start = is_blank(&line_content[marker.marker_len..]);
                    let in_para = self.current_leaf.map_or(false, |id| {
                        matches!(self.arena.get(id).kind, NodeKind::Paragraph { .. })
                    });

                    let can_interrupt = if in_para {
                        // Ordered lists starting != 1 cannot interrupt a paragraph
                        let cont_list = self.find_matching_list(&marker);
                        if marker.ordered && marker.start != 1 && cont_list.is_none() {
                            false
                        } else {
                            true
                        }
                    } else { true };

                    let empty_item_interrupts = blank_start && in_para;

                    if can_interrupt && !empty_item_interrupts {
                        // Find or create the list
                        let list_id = self.ensure_list(&marker, inner);
                        inner = list_id; // Update inner to the list

                        // Compute content indent
                        let normal_indent = marker.marker_len;
                        let reduced_indent = marker.marker_len.saturating_sub(marker.space_after).saturating_add(1);
                        let content_indent = if blank_start || marker.space_after >= 5 {
                            reduced_indent
                        } else {
                            normal_indent
                        };

                        // Create list item
                        let item_id = self.alloc_node(NodeKind::ListItem {
                            content_indent,
                            had_blank_line: false,
                        }, list_id);
                        self.add_child_to_container(list_id, item_id);
                        self.open_containers.push(list_id);
                        self.open_containers.push(item_id);
                        inner = item_id;

                        if !blank_start {
                            let new_base_col;
                            let item_content;
                            if marker.space_after >= 5 {
                                new_base_col = virtual_col_after(&line_content, marker.marker_len - marker.space_after + 1, line_base_col);
                                item_content = format!("{}{}", " ".repeat(marker.space_after - 1), &line_content[marker.marker_len..]);
                            } else {
                                new_base_col = virtual_col_after(&line_content, marker.marker_len, line_base_col);
                                item_content = line_content[marker.marker_len..].to_string();
                            }

                            // Handle tab separator (CommonMark §2.1).
                            //
                            // When the separator after the list marker is a tab, that tab
                            // expands to `w` virtual spaces. Only 1 is the required separator;
                            // the remaining `w-1` spaces are prepended to the content.
                            //
                            // Example: `-\t\tfoo` — first `\t` at sepCol=1 expands 3 spaces;
                            //   1 used as separator, 2 prepended → `"  \tfoo"` at lineBaseCol=2.
                            //   (NOT at lineBaseCol=4, since only 1 virtual space was consumed.)
                            let mut final_content = item_content;
                            let mut final_base_col = new_base_col;
                            if marker.space_after == 1 && marker.marker_len > 0 {
                                let sep_byte_idx = marker.marker_len - 1;
                                if sep_byte_idx < line_content.len() && line_content.as_bytes()[sep_byte_idx] == b'\t' {
                                    let sep_col = virtual_col_after(&line_content, sep_byte_idx, line_base_col);
                                    let w = 4 - (sep_col % 4);
                                    if w > 1 {
                                        final_content = format!("{}{}", " ".repeat(w - 1), &line_content[marker.marker_len..]);
                                        // Only 1 virtual space was consumed from the tab, so the
                                        // base column is sepCol+1, not the full virtual_col_after
                                        // (which would be sepCol+w = the full tab stop).
                                        final_base_col = sep_col + 1;
                                    }
                                }
                            }

                            line_content = final_content;
                            line_base_col = final_base_col;
                            self.last_line_was_blank = false;
                            continue 'block;
                        }

                        self.last_line_was_blank = false;
                        break 'block;
                    }
                }
            }

            // 8. Indented code block
            if ind >= 4 && !self.current_leaf.map_or(false, |id| matches!(self.arena.get(id).kind, NodeKind::Paragraph { .. })) {
                let (stripped, _) = strip_indent(&line_content, 4, line_base_col);
                match self.current_leaf {
                    Some(leaf_id) if matches!(self.arena.get(leaf_id).kind, NodeKind::IndentedCode { .. }) => {
                        if let NodeKind::IndentedCode { lines } = &mut self.arena.get_mut(leaf_id).kind {
                            lines.push(stripped);
                        }
                    }
                    _ => {
                        self.close_paragraph_or_icb();
                        let node = self.alloc_node(NodeKind::IndentedCode { lines: vec![stripped] }, inner);
                        self.add_child_to_container(inner, node);
                        self.current_leaf = Some(node);
                    }
                }
                self.last_line_was_blank = false;
                break 'block;
            }

            // 9. Paragraph
            match self.current_leaf {
                Some(leaf_id) if matches!(self.arena.get(leaf_id).kind, NodeKind::Paragraph { .. }) => {
                    if let NodeKind::Paragraph { lines } = &mut self.arena.get_mut(leaf_id).kind {
                        lines.push(line_content.clone());
                    }
                }
                _ => {
                    self.close_paragraph_or_icb();
                    let node = self.alloc_node(NodeKind::Paragraph { lines: vec![line_content.clone()] }, inner);
                    self.add_child_to_container(inner, node);
                    self.current_leaf = Some(node);
                }
            }
            self.last_line_was_blank = false;
            break 'block;
        } // end 'block loop
    }

    fn finalize_paragraph_as_setext(&mut self, leaf_id: NodeId, lines: Vec<String>, level: u8, inner: NodeId) {
        let mut text = lines.join("\n");
        // Extract link defs
        loop {
            match parse_link_definition(&text) {
                Some(def) => {
                    if !self.link_refs.contains_key(&def.label) {
                        self.link_refs.insert(def.label.clone(), LinkReference {
                            destination: def.destination,
                            title: def.title,
                        });
                    }
                    text = text[def.chars_consumed..].to_string();
                }
                None => break,
            }
        }

        // Remove the paragraph
        self.arena.remove_last_child(inner);

        if !text.trim().is_empty() {
            let content = text.trim().to_string();
            let heading = self.alloc_node(NodeKind::Heading { level, content }, inner);
            self.add_child_to_container(inner, heading);
        }
    }

    fn mark_blank_in_containers(&mut self, inner: NodeId) {
        let tag = self.container_kind_tag(inner);
        if tag == KindTag::ListItem {
            if let NodeKind::ListItem { had_blank_line, .. } = &mut self.arena.get_mut(inner).kind {
                *had_blank_line = true;
            }
        } else if tag == KindTag::List {
            if let NodeKind::List { had_blank_line, .. } = &mut self.arena.get_mut(inner).kind {
                *had_blank_line = true;
            }
        }
    }

    fn find_matching_list(&self, marker: &ListMarker) -> Option<NodeId> {
        for &cid in self.open_containers.iter().rev() {
            if let NodeKind::List { ordered, marker: m, .. } = &self.arena.get(cid).kind {
                if *ordered == marker.ordered && *m == marker.marker {
                    return Some(cid);
                }
            }
        }
        None
    }

    fn ensure_list(&mut self, marker: &ListMarker, inner: NodeId) -> NodeId {
        // Check if inner IS already the matching list
        if let NodeKind::List { ordered, marker: m, .. } = &self.arena.get(inner).kind {
            if *ordered == marker.ordered && *m == marker.marker {
                // Update tightness
                let had_blank = self.last_line_was_blank
                    && (self.container_kind_tag(self.last_blank_container) == KindTag::List
                        || self.container_kind_tag(self.last_blank_container) == KindTag::ListItem);
                if had_blank {
                    if let NodeKind::List { tight, had_blank_line, .. } = &mut self.arena.get_mut(inner).kind {
                        *tight = false;
                        *had_blank_line = false;
                    }
                } else if let NodeKind::List { had_blank_line, .. } = &mut self.arena.get_mut(inner).kind {
                    *had_blank_line = false;
                }
                return inner;
            }
        }

        // Check last child of inner
        if let Some(last_id) = self.arena.last_child(inner) {
            if let NodeKind::List { ordered, marker: m, .. } = &self.arena.get(last_id).kind {
                if *ordered == marker.ordered && *m == marker.marker {
                    let had_blank = self.last_line_was_blank
                        && (self.container_kind_tag(self.last_blank_container) == KindTag::List
                            || self.container_kind_tag(self.last_blank_container) == KindTag::ListItem);
                    if had_blank {
                        if let NodeKind::List { tight, had_blank_line, .. } = &mut self.arena.get_mut(last_id).kind {
                            *tight = false;
                            *had_blank_line = false;
                        }
                    } else if let NodeKind::List { had_blank_line, .. } = &mut self.arena.get_mut(last_id).kind {
                        *had_blank_line = false;
                    }
                    return last_id;
                }
            }
        }

        // Close current leaf and create new list
        self.close_paragraph_or_icb();
        let list = self.alloc_node(NodeKind::List {
            ordered: marker.ordered,
            marker: marker.marker,
            start: marker.start,
            tight: true,
            had_blank_line: false,
        }, inner);
        self.add_child_to_container(inner, list);
        list
    }

    // ── Kind tag helpers ──────────────────────────────────────────────────

    fn container_kind_tag(&self, id: NodeId) -> KindTag {
        match &self.arena.get(id).kind {
            NodeKind::Document => KindTag::Document,
            NodeKind::Blockquote => KindTag::Blockquote,
            NodeKind::List { .. } => KindTag::List,
            NodeKind::ListItem { .. } => KindTag::ListItem,
            _ => KindTag::Other,
        }
    }

    fn leaf_kind_tag(&self, id: NodeId) -> LeafTag {
        match &self.arena.get(id).kind {
            NodeKind::Paragraph { .. } => LeafTag::Paragraph,
            NodeKind::FencedCode { .. } => LeafTag::FencedCode,
            NodeKind::IndentedCode { .. } => LeafTag::IndentedCode,
            NodeKind::HtmlBlock { .. } => LeafTag::HtmlBlock,
            _ => LeafTag::Other,
        }
    }

    fn list_item_content_indent(&self, id: NodeId) -> usize {
        if let NodeKind::ListItem { content_indent, .. } = &self.arena.get(id).kind {
            *content_indent
        } else { 0 }
    }

    fn list_params(&self, id: NodeId) -> (bool, char) {
        if let NodeKind::List { ordered, marker, .. } = &self.arena.get(id).kind {
            (*ordered, *marker)
        } else { (false, '-') }
    }

    fn fenced_code_params(&self, id: NodeId) -> (char, usize, usize) {
        if let NodeKind::FencedCode { fence_char, fence_len, base_indent, .. } = &self.arena.get(id).kind {
            (*fence_char, *fence_len, *base_indent)
        } else { ('`', 3, 0) }
    }

    fn html_block_type(&self, id: NodeId) -> u8 {
        if let NodeKind::HtmlBlock { html_type, .. } = &self.arena.get(id).kind {
            *html_type
        } else { 6 }
    }

    /// Convert the arena into final blocks.
    pub fn finalize(mut self) -> (Vec<FinalBlock>, LinkRefMap) {
        // Finalize any open leaf
        if let Some(leaf_id) = self.current_leaf.take() {
            self.finalize_leaf(leaf_id);
        }

        let root = self.root;
        let final_blocks = self.convert_node_children(root);
        (final_blocks, self.link_refs)
    }

    fn convert_node_children(&self, id: NodeId) -> Vec<FinalBlock> {
        let children: Vec<NodeId> = self.arena.get(id).children.clone();
        children.into_iter()
            .filter_map(|cid| self.convert_node(cid))
            .collect()
    }

    fn convert_node(&self, id: NodeId) -> Option<FinalBlock> {
        match &self.arena.get(id).kind {
            NodeKind::Document => {
                Some(FinalBlock::Blockquote { children: self.convert_node_children(id) })
            }
            NodeKind::Heading { level, content } => {
                Some(FinalBlock::Heading { level: *level, raw_content: content.clone() })
            }
            NodeKind::Paragraph { lines } => {
                if lines.is_empty() || lines.iter().all(|l| l.trim().is_empty()) {
                    return None;
                }
                // Strip leading whitespace from each line
                let content = lines.iter()
                    .map(|l| l.trim_start_matches(|c: char| c == ' ' || c == '\t').to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                Some(FinalBlock::Paragraph { raw_content: content })
            }
            NodeKind::FencedCode { info_string, lines, .. } => {
                let lang = if info_string.is_empty() { None } else { Some(info_string.clone()) };
                let value = if lines.is_empty() {
                    String::new()
                } else {
                    format!("{}\n", lines.join("\n"))
                };
                Some(FinalBlock::CodeBlock { language: lang, value })
            }
            NodeKind::IndentedCode { lines } => {
                if lines.is_empty() { return None; }
                let value = format!("{}\n", lines.join("\n"));
                Some(FinalBlock::CodeBlock { language: None, value })
            }
            NodeKind::HtmlBlock { lines, .. } => {
                let mut ls = lines.clone();
                while ls.last().map_or(false, |l: &String| l.trim().is_empty()) {
                    ls.pop();
                }
                let value = if ls.is_empty() { String::new() } else { format!("{}\n", ls.join("\n")) };
                Some(FinalBlock::HtmlBlock { value })
            }
            NodeKind::Blockquote => {
                let children = self.convert_node_children(id);
                Some(FinalBlock::Blockquote { children })
            }
            NodeKind::List { ordered, start, tight, had_blank_line, .. } => {
                let children: Vec<NodeId> = self.arena.get(id).children.clone();
                // Tightness: list is tight if tight flag is set AND no had_blank_line
                // AND no item with had_blank_line AND > 1 block child
                let is_tight = *tight && !*had_blank_line && {
                    let items_loose = children.iter().any(|&item_id| {
                        if let NodeKind::ListItem { had_blank_line: hbl, .. } = &self.arena.get(item_id).kind {
                            *hbl && self.arena.get(item_id).children.len() > 1
                        } else { false }
                    });
                    !items_loose
                };
                let items = children.into_iter()
                    .filter_map(|cid| self.convert_node(cid))
                    .collect();
                Some(FinalBlock::List {
                    ordered: *ordered,
                    start: if *ordered { Some(*start) } else { None },
                    tight: is_tight,
                    items,
                })
            }
            NodeKind::ListItem { .. } => {
                let children = self.convert_node_children(id);
                Some(FinalBlock::ListItem { children })
            }
            NodeKind::ThematicBreak => Some(FinalBlock::ThematicBreak),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum KindTag { Document, Blockquote, List, ListItem, Other }

#[derive(Debug, Clone, Copy, PartialEq)]
enum LeafTag { Paragraph, FencedCode, IndentedCode, HtmlBlock, Other }

// The Arena::get_mut in finalize_leaf needs to work with leaf finalization.
// Let's just make the convert_node method also work with a post-process step.

// ─── Public Entry Point ────────────────────────────────────────────────────────

/// Parse CommonMark Markdown input into a block tree.
///
/// Returns the list of top-level blocks and the link reference map.
/// Call the inline parser on headings and paragraphs to complete Phase 2.
pub fn parse(input: &str) -> (Vec<FinalBlock>, LinkRefMap) {
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines: Vec<&str> = normalized.split('\n').collect();
    if lines.last().map_or(false, |l| l.is_empty()) {
        lines.pop();
    }

    let mut parser = BlockParser::new();
    for line in &lines {
        parser.process_line(line);
    }
    parser.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_blank() {
        assert!(is_blank(""));
        assert!(is_blank("   "));
        assert!(!is_blank("x"));
    }

    #[test]
    fn test_indent_of() {
        assert_eq!(indent_of("  hello", 0), 2);
        assert_eq!(indent_of("\thello", 0), 4);
    }

    #[test]
    fn test_is_thematic_break() {
        assert!(is_thematic_break("---"));
        assert!(is_thematic_break("* * *"));
        assert!(!is_thematic_break("--"));
    }

    #[test]
    fn test_parse_atx_heading() {
        assert_eq!(parse_atx_heading("# Hello"), Some((1, "Hello".to_string())));
        assert_eq!(parse_atx_heading("## Foo ##"), Some((2, "Foo".to_string())));
        assert_eq!(parse_atx_heading("not"), None);
    }

    #[test]
    fn test_parse_list_marker_unordered() {
        let m = parse_list_marker("- item").unwrap();
        assert!(!m.ordered); assert_eq!(m.marker, '-');
    }

    #[test]
    fn test_parse_list_marker_ordered() {
        let m = parse_list_marker("1. item").unwrap();
        assert!(m.ordered); assert_eq!(m.start, 1);
    }

    #[test]
    fn test_parse_simple_document() {
        let (blocks, _refs) = parse("# Hello\n\nWorld\n");
        assert_eq!(blocks.len(), 2);
        matches!(&blocks[0], FinalBlock::Heading { level: 1, .. });
        matches!(&blocks[1], FinalBlock::Paragraph { .. });
    }

    #[test]
    fn test_parse_code_block() {
        let (blocks, _) = parse("```rust\nlet x = 1;\n```\n");
        assert_eq!(blocks.len(), 1);
        if let FinalBlock::CodeBlock { language, value } = &blocks[0] {
            assert_eq!(language.as_deref(), Some("rust"));
            assert!(value.contains("let x = 1;"));
        }
    }

    #[test]
    fn test_parse_thematic_break() {
        let (blocks, _) = parse("---\n");
        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0], FinalBlock::ThematicBreak));
    }
}
