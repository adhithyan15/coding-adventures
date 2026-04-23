//! `DocumentManager` -- tracks open file contents and applies incremental edits.
//!
//! # The Document Manager's Job
//!
//! When the user opens a file in VS Code, the editor sends a `textDocument/didOpen`
//! notification with the full file content. From that point on, the editor does
//! NOT re-send the entire file on every keystroke. Instead, it sends incremental
//! changes: what changed, and where. The `DocumentManager` applies these changes
//! to maintain the current text of each open file.
//!
//! ```text
//! Editor opens file:   didOpen   -> DocumentManager stores text at version 1
//! User types "X":      didChange -> DocumentManager applies delta -> version 2
//! User saves:          didSave   -> (optional: trigger format)
//! User closes:         didClose  -> DocumentManager removes entry
//! ```
//!
//! # UTF-16: The Tricky Part
//!
//! LSP specifies that character offsets are measured in UTF-16 CODE UNITS.
//! This is a historical accident: VS Code is built on TypeScript, which uses
//! UTF-16 strings internally (like Java and C#).
//!
//! Rust strings are UTF-8. A single Unicode codepoint can occupy:
//! - 1 byte in UTF-8 (ASCII, e.g. 'A')
//! - 2 bytes in UTF-8 (e.g. 'e', U+00E9)
//! - 3 bytes in UTF-8 (e.g. Chinese chars, U+4E2D)
//! - 4 bytes in UTF-8 (e.g. guitar emoji, U+1F3B8)
//!
//! In UTF-16:
//! - Codepoints in the Basic Multilingual Plane (U+0000-U+FFFF) -> 1 code unit
//! - Codepoints above U+FFFF (emojis, rare CJK) -> 2 code units (a "surrogate pair")
//!
//! The function `convert_utf16_offset_to_byte_offset` bridges this gap.

use crate::types::{Position, Range};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Document
// ---------------------------------------------------------------------------

/// An open file tracked by the `DocumentManager`.
#[derive(Debug, Clone)]
pub struct Document {
    pub uri: String,
    /// Current content, UTF-8 encoded.
    pub text: String,
    /// Monotonically increasing; matches LSP's document version.
    pub version: i32,
}

// ---------------------------------------------------------------------------
// TextChange
// ---------------------------------------------------------------------------

/// One incremental change to a document.
///
/// If `range` is `None`, `new_text` replaces the ENTIRE document content (full sync).
/// If `range` is `Some`, `new_text` replaces just the specified range (incremental sync).
#[derive(Debug, Clone)]
pub struct TextChange {
    /// `None` = full replacement.
    pub range: Option<Range>,
    pub new_text: String,
}

// ---------------------------------------------------------------------------
// DocumentManager
// ---------------------------------------------------------------------------

/// Tracks all files currently open in the editor.
///
/// The editor sends open/change/close notifications; this manager keeps the
/// authoritative current text of each file.
pub struct DocumentManager {
    docs: HashMap<String, Document>,
}

impl DocumentManager {
    /// Create an empty `DocumentManager`.
    pub fn new() -> Self {
        Self {
            docs: HashMap::new(),
        }
    }

    /// Record a newly opened file.
    ///
    /// Called when the editor sends `textDocument/didOpen`.
    pub fn open(&mut self, uri: &str, text: &str, version: i32) {
        self.docs.insert(
            uri.to_string(),
            Document {
                uri: uri.to_string(),
                text: text.to_string(),
                version,
            },
        );
    }

    /// Get the document for a URI, or `None` if the document is not open.
    pub fn get(&self, uri: &str) -> Option<&Document> {
        self.docs.get(uri)
    }

    /// Remove a document from the manager.
    ///
    /// Called when the editor sends `textDocument/didClose`.
    pub fn close(&mut self, uri: &str) {
        self.docs.remove(uri);
    }

    /// Apply a list of incremental changes to an open document.
    ///
    /// Changes are applied in order. If a range is `None`, the change replaces
    /// the entire document. After all changes, the document's version is updated.
    pub fn apply_changes(
        &mut self,
        uri: &str,
        changes: &[TextChange],
        version: i32,
    ) -> Result<(), String> {
        let doc = self
            .docs
            .get_mut(uri)
            .ok_or_else(|| format!("document not open: {}", uri))?;

        for change in changes {
            match &change.range {
                None => {
                    // Full document replacement -- simplest case.
                    doc.text = change.new_text.clone();
                }
                Some(range) => {
                    // Incremental update: splice new text at the specified range.
                    let new_text =
                        apply_range_change(&doc.text, range, &change.new_text)?;
                    doc.text = new_text;
                }
            }
        }

        doc.version = version;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Range application
// ---------------------------------------------------------------------------

/// Splice `new_text` into `text` at the given LSP range.
///
/// Converts LSP's (line, UTF-16-character) coordinates to byte offsets in the
/// UTF-8 Rust string, then performs the splice.
fn apply_range_change(text: &str, r: &Range, new_text: &str) -> Result<String, String> {
    let start_byte = convert_position_to_byte_offset(text, &r.start);
    let end_byte = convert_position_to_byte_offset(text, &r.end);

    if start_byte > end_byte {
        return Err(format!(
            "start offset {} > end offset {}",
            start_byte, end_byte
        ));
    }

    let end_byte = end_byte.min(text.len());

    // Guard against slicing mid-codepoint. The UTF-16-to-byte conversion
    // normally lands on codepoint boundaries, but defensive checks prevent
    // panics if a client sends overlapping or malformed edits.
    if !text.is_char_boundary(start_byte) || !text.is_char_boundary(end_byte) {
        return Err(format!(
            "byte offsets not on char boundaries: {}, {}",
            start_byte, end_byte
        ));
    }

    let mut result = String::with_capacity(start_byte + new_text.len() + (text.len() - end_byte));
    result.push_str(&text[..start_byte]);
    result.push_str(new_text);
    result.push_str(&text[end_byte..]);
    Ok(result)
}

/// Convert an LSP `Position` (0-based line, UTF-16 char) to a byte offset
/// in the UTF-8 Rust string.
///
/// # Algorithm
///
/// 1. Walk line-by-line to find the byte offset of the start of the target line.
/// 2. From that offset, walk UTF-8 codepoints, converting each to its UTF-16
///    length, until we reach the target UTF-16 character offset.
fn convert_position_to_byte_offset(text: &str, pos: &Position) -> usize {
    let bytes = text.as_bytes();
    let mut line_start: usize = 0;
    let mut current_line: i32 = 0;

    // Phase 1: find the byte offset of the start of pos.line.
    while current_line < pos.line {
        match bytes[line_start..].iter().position(|&b| b == b'\n') {
            None => return text.len(), // line number exceeds file lines
            Some(idx) => {
                line_start += idx + 1;
                current_line += 1;
            }
        }
    }

    // Phase 2: from line_start, advance pos.character UTF-16 code units.
    let mut byte_offset = line_start;
    let mut utf16_units: i32 = 0;

    while utf16_units < pos.character && byte_offset < text.len() {
        // Check for newline -- don't advance past the end of the line.
        if bytes[byte_offset] == b'\n' {
            break;
        }

        // Decode one Unicode codepoint from the UTF-8 stream.
        // Use match instead of unwrap to avoid panicking on malformed input.
        let ch = match text[byte_offset..].chars().next() {
            Some(c) => c,
            None => break,
        };
        let char_len_utf8 = ch.len_utf8();

        // How many UTF-16 code units does this codepoint occupy?
        // Codepoints > U+FFFF require a surrogate pair (2 UTF-16 code units).
        let utf16_len = if ch > '\u{FFFF}' { 2 } else { 1 };

        if utf16_units + utf16_len > pos.character {
            // This codepoint would overshoot the target character. Stop here.
            break;
        }

        byte_offset += char_len_utf8;
        utf16_units += utf16_len;
    }

    byte_offset
}

/// Convert a 0-based (line, UTF-16 char) position to a byte offset in a
/// UTF-8 Rust string.
///
/// This is the public version for use in tests and external packages.
///
/// # Why UTF-16?
///
/// LSP character offsets are UTF-16 code units because VS Code's internal
/// string representation is UTF-16 (as is JavaScript's `String` type).
/// This function bridges the gap to Rust's UTF-8 strings.
///
/// # Example
///
/// ```rust
/// use coding_adventures_ls00::document_manager::convert_utf16_offset_to_byte_offset;
///
/// // "A🎸B" -- A(1 byte, 1 UTF-16), 🎸(4 bytes, 2 UTF-16), B(1 byte, 1 UTF-16)
/// // "B" is at UTF-16 character 3, byte offset 5.
/// let byte_off = convert_utf16_offset_to_byte_offset("A\u{1F3B8}B", 0, 3);
/// assert_eq!(byte_off, 5);
/// ```
pub fn convert_utf16_offset_to_byte_offset(text: &str, line: i32, char: i32) -> usize {
    convert_position_to_byte_offset(
        text,
        &Position {
            line,
            character: char,
        },
    )
}
