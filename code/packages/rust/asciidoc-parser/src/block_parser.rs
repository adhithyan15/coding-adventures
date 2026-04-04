//! Block-Level Parser
//!
//! Phase 1 of AsciiDoc parsing: split the input into block-level tokens
//! and build the structural skeleton of the document.
//!
//! # State Machine
//!
//! The parser maintains a `State` that determines how each line is processed:
//!
//!   `Normal`       — between blocks; dispatch each line to a block type
//!   `Paragraph`    — accumulating lines of a paragraph
//!   `CodeBlock`    — inside a `----` fenced code block
//!   `LiteralBlock` — inside a `....` fenced literal block
//!   `Passthrough`  — inside a `++++` passthrough block (raw HTML)
//!   `QuoteBlock`   — inside a `____` blockquote (content re-parsed)
//!
//! # List Handling
//!
//! List items are accumulated in a flat buffer and converted to a nested
//! `ListNode` tree when a blank line or non-list line is encountered.
//! Nesting level is determined by counting leading `*` or `.` characters.

use document_ast::{
    BlockNode, CodeBlockNode, BlockquoteNode, HeadingNode, ListNode,
    ListItemNode, ListChildNode, ParagraphNode, RawBlockNode, ThematicBreakNode,
};
use crate::inline_parser::parse_inlines;

// ─── State ────────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum State {
    Normal,
    Paragraph,
    CodeBlock,
    LiteralBlock,
    Passthrough,
    QuoteBlock,
}

// ─── List item accumulator ────────────────────────────────────────────────────

#[derive(Debug)]
struct ListItemAccum {
    level: usize,
    ordered: bool,
    raw: String,
}

// ─── parse_blocks ─────────────────────────────────────────────────────────────

/// Convert AsciiDoc text into a `Vec<BlockNode>`.
///
/// This is Phase 1 of the two-phase parsing pipeline. It identifies block
/// structure and leaves inline content as raw strings for Phase 2.
pub fn parse_blocks(text: &str) -> Vec<BlockNode> {
    let lines = split_lines(text);
    let mut result: Vec<BlockNode> = Vec::new();
    let mut state = State::Normal;
    let mut accum: Vec<String> = Vec::new();
    let mut pending_language: Option<String> = None;
    let mut list_items: Vec<ListItemAccum> = Vec::new();

    let flush_paragraph = |accum: &mut Vec<String>, result: &mut Vec<BlockNode>| {
        if accum.is_empty() {
            return;
        }
        let raw = accum.join("\n");
        let children = parse_inlines(&raw);
        result.push(BlockNode::Paragraph(ParagraphNode { children }));
        accum.clear();
    };

    let flush_list = |list_items: &mut Vec<ListItemAccum>, result: &mut Vec<BlockNode>| {
        if list_items.is_empty() {
            return;
        }
        let nodes = build_list_nodes(list_items);
        result.extend(nodes);
        list_items.clear();
    };

    for line in &lines {
        match state {
            State::CodeBlock => {
                if is_delim(line, b'-', 4) {
                    let val = if accum.is_empty() {
                        String::new()
                    } else {
                        let mut v = accum.join("\n");
                        if !v.ends_with('\n') { v.push('\n'); }
                        v
                    };
                    let lang = pending_language.take();
                    result.push(BlockNode::CodeBlock(CodeBlockNode { language: lang, value: val }));
                    accum.clear();
                    state = State::Normal;
                } else {
                    accum.push(line.clone());
                }
            }

            State::LiteralBlock => {
                if is_delim(line, b'.', 4) {
                    let val = if accum.is_empty() {
                        String::new()
                    } else {
                        let mut v = accum.join("\n");
                        if !v.ends_with('\n') { v.push('\n'); }
                        v
                    };
                    result.push(BlockNode::CodeBlock(CodeBlockNode { language: None, value: val }));
                    accum.clear();
                    state = State::Normal;
                } else {
                    accum.push(line.clone());
                }
            }

            State::Passthrough => {
                if is_delim(line, b'+', 4) {
                    let val = accum.join("\n");
                    result.push(BlockNode::RawBlock(RawBlockNode { format: "html".to_string(), value: val }));
                    accum.clear();
                    state = State::Normal;
                } else {
                    accum.push(line.clone());
                }
            }

            State::QuoteBlock => {
                if is_delim(line, b'_', 4) {
                    let inner = accum.join("\n");
                    let inner_blocks = parse_blocks(&inner);
                    result.push(BlockNode::Blockquote(BlockquoteNode { children: inner_blocks }));
                    accum.clear();
                    state = State::Normal;
                } else {
                    accum.push(line.clone());
                }
            }

            State::Paragraph => {
                if line.trim().is_empty() {
                    flush_paragraph(&mut accum, &mut result);
                    state = State::Normal;
                } else if let Some((level, content)) = parse_heading_line(line) {
                    flush_paragraph(&mut accum, &mut result);
                    let children = parse_inlines(&content);
                    result.push(BlockNode::Heading(HeadingNode { level, children }));
                    state = State::Normal;
                } else if let Some((item_level, item_ordered, item_content)) = parse_list_line(line) {
                    flush_paragraph(&mut accum, &mut result);
                    list_items.push(ListItemAccum { level: item_level, ordered: item_ordered, raw: item_content });
                    state = State::Normal;
                } else {
                    accum.push(line.clone());
                }
            }

            State::Normal => {
                let trimmed = line.trim();

                // Blank line
                if trimmed.is_empty() {
                    flush_list(&mut list_items, &mut result);
                    continue;
                }

                // Single-line comment: // (but not ///)
                if line.starts_with("//") && !line.starts_with("///") {
                    continue;
                }

                // Block attribute: [source,lang] etc.
                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    flush_list(&mut list_items, &mut result);
                    let attr = &trimmed[1..trimmed.len() - 1];
                    if attr.to_lowercase().starts_with("source") {
                        let parts: Vec<&str> = attr.splitn(2, ',').collect();
                        if parts.len() == 2 {
                            pending_language = Some(parts[1].trim().to_string());
                        } else {
                            pending_language = None;
                        }
                    }
                    continue;
                }

                // Heading
                if let Some((level, content)) = parse_heading_line(line) {
                    flush_list(&mut list_items, &mut result);
                    let children = parse_inlines(&content);
                    result.push(BlockNode::Heading(HeadingNode { level, children }));
                    continue;
                }

                // Thematic break: '''
                if is_thematic_break(line) {
                    flush_list(&mut list_items, &mut result);
                    result.push(BlockNode::ThematicBreak(ThematicBreakNode));
                    continue;
                }

                // Code block delimiter: ----
                if is_delim(line, b'-', 4) {
                    flush_list(&mut list_items, &mut result);
                    accum.clear();
                    state = State::CodeBlock;
                    continue;
                }

                // Literal block delimiter: ....
                if is_delim(line, b'.', 4) {
                    flush_list(&mut list_items, &mut result);
                    accum.clear();
                    state = State::LiteralBlock;
                    continue;
                }

                // Passthrough block delimiter: ++++
                if is_delim(line, b'+', 4) {
                    flush_list(&mut list_items, &mut result);
                    accum.clear();
                    state = State::Passthrough;
                    continue;
                }

                // Quote block delimiter: ____
                if is_delim(line, b'_', 4) {
                    flush_list(&mut list_items, &mut result);
                    accum.clear();
                    state = State::QuoteBlock;
                    continue;
                }

                // List item
                if let Some((item_level, item_ordered, item_content)) = parse_list_line(line) {
                    // If list type changes, flush current list first
                    if !list_items.is_empty() && list_items[0].ordered != item_ordered {
                        flush_list(&mut list_items, &mut result);
                    }
                    list_items.push(ListItemAccum { level: item_level, ordered: item_ordered, raw: item_content });
                    continue;
                }

                // List continuation marker
                if line == "+" && !list_items.is_empty() {
                    continue;
                }

                // Regular text → paragraph
                flush_list(&mut list_items, &mut result);
                accum.push(line.clone());
                state = State::Paragraph;
            }
        }
    }

    // Flush remaining state
    match state {
        State::Paragraph => {
            flush_paragraph(&mut accum, &mut result);
        }
        State::CodeBlock | State::LiteralBlock => {
            let val = if accum.is_empty() {
                String::new()
            } else {
                let mut v = accum.join("\n");
                if !v.ends_with('\n') { v.push('\n'); }
                v
            };
            let lang = if matches!(state, State::CodeBlock) { pending_language.take() } else { None };
            result.push(BlockNode::CodeBlock(CodeBlockNode { language: lang, value: val }));
        }
        State::Passthrough => {
            let val = accum.join("\n");
            result.push(BlockNode::RawBlock(RawBlockNode { format: "html".to_string(), value: val }));
        }
        State::QuoteBlock => {
            let inner = accum.join("\n");
            let inner_blocks = parse_blocks(&inner);
            result.push(BlockNode::Blockquote(BlockquoteNode { children: inner_blocks }));
        }
        State::Normal => {}
    }
    flush_list(&mut list_items, &mut result);

    result
}

// ─── Helper functions ─────────────────────────────────────────────────────────

/// Split text on `\n`. A trailing newline does not produce an extra empty line.
fn split_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<String> = text.split('\n').map(String::from).collect();
    // Remove trailing empty element from trailing newline
    if lines.last().map(|s| s.is_empty()).unwrap_or(false) {
        lines.pop();
    }
    lines
}

/// Returns `true` if `line` consists entirely of `ch` repeated at least `min_len` times.
fn is_delim(line: &str, ch: u8, min_len: usize) -> bool {
    let trimmed = line.trim_end();
    if trimmed.len() < min_len {
        return false;
    }
    trimmed.bytes().all(|b| b == ch)
}

/// Returns `true` if `line` is `'''` (three or more single-quotes).
fn is_thematic_break(line: &str) -> bool {
    let trimmed = line.trim_end();
    if trimmed.len() < 3 {
        return false;
    }
    trimmed.bytes().all(|b| b == b'\'')
}

/// Try to parse `line` as a heading. Returns `Some((level, content))` or `None`.
///
/// Format: one or more `=` characters followed by exactly one space, then content.
fn parse_heading_line(line: &str) -> Option<(u8, String)> {
    let mut count = 0u8;
    let bytes = line.as_bytes();
    while count < 6 && (count as usize) < bytes.len() && bytes[count as usize] == b'=' {
        count += 1;
    }
    if count == 0 {
        return None;
    }
    let idx = count as usize;
    if idx < bytes.len() && bytes[idx] == b' ' {
        Some((count, line[idx + 1..].trim().to_string()))
    } else {
        None
    }
}

/// Try to parse `line` as a list item.
/// Returns `Some((level, ordered, content))` or `None`.
///
/// Unordered: `* text` (level=1), `** text` (level=2), etc.
/// Ordered:   `. text` (level=1), `.. text` (level=2), etc.
fn parse_list_line(line: &str) -> Option<(usize, bool, String)> {
    let bytes = line.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let marker = bytes[0];
    if marker != b'*' && marker != b'.' {
        return None;
    }
    let mut count = 0usize;
    while count < bytes.len() && bytes[count] == marker {
        count += 1;
    }
    if count < bytes.len() && bytes[count] == b' ' {
        let content = line[count + 1..].trim().to_string();
        let ordered = marker == b'.';
        Some((count, ordered, content))
    } else {
        None
    }
}

// ─── List building ────────────────────────────────────────────────────────────

/// Convert a flat list of `ListItemAccum` into a nested `Vec<BlockNode>`.
fn build_list_nodes(items: &mut Vec<ListItemAccum>) -> Vec<BlockNode> {
    if items.is_empty() {
        return Vec::new();
    }
    let ordered = items[0].ordered;
    let list = build_nested_list(items, ordered, 1);
    match list {
        Some(l) => vec![BlockNode::List(l)],
        None => Vec::new(),
    }
}

/// Recursively build a `ListNode` from items at `level`.
fn build_nested_list(items: &[ListItemAccum], ordered: bool, level: usize) -> Option<ListNode> {
    let mut children: Vec<ListChildNode> = Vec::new();
    let mut i = 0;
    while i < items.len() {
        let item = &items[i];
        if item.level < level {
            break;
        }
        if item.level == level {
            // Create a list item
            let inlines = parse_inlines(&item.raw);
            let mut li_children: Vec<BlockNode> = vec![
                BlockNode::Paragraph(ParagraphNode { children: inlines }),
            ];
            // Look ahead for nested items
            let j_start = i + 1;
            let mut j = j_start;
            while j < items.len() && items[j].level > level {
                j += 1;
            }
            if j > j_start {
                let nested_items = &items[j_start..j];
                if !nested_items.is_empty() {
                    let nested_ordered = nested_items[0].ordered;
                    if let Some(nested) = build_nested_list(nested_items, nested_ordered, level + 1) {
                        li_children.push(BlockNode::List(nested));
                    }
                }
            }
            children.push(ListChildNode::ListItem(ListItemNode { children: li_children }));
            i = j;
        } else {
            // item.level > level: already consumed by nested call
            i += 1;
        }
    }
    if children.is_empty() {
        None
    } else {
        Some(ListNode {
            ordered,
            start: Some(1),
            tight: true,
            children,
        })
    }
}
