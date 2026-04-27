// ============================================================================
// DocumentManager.swift — DocumentManager and UTF-16 offset handling
// ============================================================================
//
// # The Document Manager's Job
//
// When the user opens a file in VS Code, the editor sends a textDocument/didOpen
// notification with the full file content. From that point on, the editor sends
// incremental changes: what changed, and where. The DocumentManager applies these
// changes to maintain the current text of each open file.
//
//   Editor opens file:   didOpen   -> DocumentManager stores text at version 1
//   User types "X":      didChange -> DocumentManager applies delta -> version 2
//   User saves:          didSave   -> (optional: trigger format)
//   User closes:         didClose  -> DocumentManager removes entry
//
// # Why Version Numbers?
//
// The editor increments the version number with every change. The ParseCache
// uses (uri, version) as its cache key -- if the version matches, the cached
// parse result is still valid.
//
// # UTF-16: The Tricky Part
//
// LSP specifies that character offsets are measured in UTF-16 CODE UNITS.
// Swift strings are UTF-8 internally (via String). A single Unicode codepoint
// can occupy different numbers of units in each encoding:
//
//   Codepoint   UTF-8 bytes   UTF-16 code units
//   ---------   -----------   -----------------
//   'A'         1             1
//   'e'         2             1
//   'zhong'     3             1
//   'guitar'    4             2 (surrogate pair)
//
// So when the LSP client says character=8 (UTF-16), we cannot simply slice
// 8 bytes into the UTF-8 string. We must walk the string converting each
// codepoint to its UTF-16 length until we reach code unit 8.
//
// ============================================================================

import Foundation

// ============================================================================
// Document — an open file tracked by the DocumentManager
// ============================================================================

/// Represents an open file tracked by the DocumentManager.
public class Document {
    /// The file URI (e.g. "file:///home/user/main.swift").
    public let uri: String

    /// Current content, UTF-8 encoded.
    public var text: String

    /// Monotonically increasing; matches LSP's document version.
    public var version: Int

    public init(uri: String, text: String, version: Int) {
        self.uri = uri
        self.text = text
        self.version = version
    }
}

// ============================================================================
// TextChange — one incremental change to a document
// ============================================================================
//
// If range is nil, newText replaces the ENTIRE document content (full sync).
// If range is non-nil, newText replaces just the specified range (incremental sync).
//

/// One incremental change to a document's text.
///
/// With `range` nil, `newText` replaces the entire document (full sync).
/// With `range` set, `newText` replaces just that range (incremental sync).
public struct TextChange: Sendable {
    /// The range to replace. Nil means full document replacement.
    public let range: Range?

    /// The new text to insert at the range (or the full document).
    public let newText: String

    public init(range: Range? = nil, newText: String) {
        self.range = range
        self.newText = newText
    }
}

// ============================================================================
// DocumentManager
// ============================================================================

/// Tracks all files currently open in the editor.
///
/// The editor sends open/change/close notifications; this manager keeps the
/// authoritative current text of each file. The ParseCache and all feature
/// handlers read from this manager to get the source text.
public class DocumentManager {
    private var docs: [String: Document] = [:]

    public init() {}

    /// Record a newly opened file.
    ///
    /// Called when the editor sends textDocument/didOpen.
    public func open(uri: String, text: String, version: Int) {
        docs[uri] = Document(uri: uri, text: text, version: version)
    }

    /// Apply incremental changes to an open document.
    ///
    /// Changes are applied in order. If a change's range is nil, it replaces
    /// the entire document. After all changes, the document's version is updated.
    ///
    /// - Returns: An error string if the document is not open, nil on success.
    public func applyChanges(uri: String, changes: [TextChange], version: Int) -> String? {
        guard let doc = docs[uri] else {
            return "document not open: \(uri)"
        }

        for change in changes {
            if let range = change.range {
                // Incremental update: splice new text at the specified range.
                if let newText = applyRangeChange(doc.text, range, change.newText) {
                    doc.text = newText
                }
            } else {
                // Full document replacement.
                doc.text = change.newText
            }
        }

        doc.version = version
        return nil
    }

    /// Get the document for a URI.
    ///
    /// - Returns: The document, or nil if not open.
    public func get(uri: String) -> Document? {
        return docs[uri]
    }

    /// Remove a document from tracking.
    ///
    /// Called when the editor sends textDocument/didClose.
    public func close(uri: String) {
        docs.removeValue(forKey: uri)
    }
}

// ============================================================================
// Range application
// ============================================================================

/// Splice newText into text at the given LSP range.
///
/// Converts LSP's (line, UTF-16-character) coordinates to byte offsets
/// in the UTF-8 Swift string, then performs the splice.
func applyRangeChange(_ text: String, _ r: Range, _ newText: String) -> String? {
    guard let startByte = convertPositionToByteOffset(text, r.start),
          let endByte = convertPositionToByteOffset(text, r.end) else {
        return nil
    }

    let startIdx = text.utf8.index(text.utf8.startIndex, offsetBy: min(startByte, text.utf8.count))
    let endIdx = text.utf8.index(text.utf8.startIndex, offsetBy: min(endByte, text.utf8.count))

    return String(text[text.startIndex ..< String.Index(startIdx, within: text)!])
         + newText
         + String(text[String.Index(endIdx, within: text)! ..< text.endIndex])
}

// ============================================================================
// UTF-16 offset conversion
// ============================================================================

/// Convert an LSP Position (0-based line, UTF-16 char) to a byte offset
/// in the UTF-8 string.
///
/// Algorithm:
/// 1. Walk line-by-line to find the byte offset of the start of the target line.
/// 2. From that offset, walk UTF-8 codepoints, converting each to its UTF-16
///    length, until we reach the target UTF-16 character offset.
func convertPositionToByteOffset(_ text: String, _ pos: Position) -> Int? {
    let utf8 = Array(text.utf8)
    var lineStart = 0
    var currentLine = 0

    // Phase 1: find the byte offset of the start of pos.line.
    while currentLine < pos.line {
        guard let idx = utf8[lineStart...].firstIndex(of: UInt8(ascii: "\n")) else {
            // Line number exceeds the number of lines. Clamp to end.
            return utf8.count
        }
        lineStart = idx + 1
        currentLine += 1
    }

    // Phase 2: from lineStart, advance pos.character UTF-16 code units.
    var byteOffset = lineStart
    var utf16Units = 0

    while utf16Units < pos.character && byteOffset < utf8.count {
        // Don't advance past newline
        if utf8[byteOffset] == UInt8(ascii: "\n") {
            break
        }

        // Decode one codepoint from UTF-8
        let (codepoint, size) = decodeUTF8(utf8, byteOffset)

        // How many UTF-16 code units does this codepoint need?
        let utf16Len = codepoint > 0xFFFF ? 2 : 1

        if utf16Units + utf16Len > pos.character {
            // Would overshoot. Stop here (e.g. middle of surrogate pair).
            break
        }

        byteOffset += size
        utf16Units += utf16Len
    }

    return byteOffset
}

/// Convert a 0-based (line, UTF-16 char) position to a byte offset.
///
/// This is the public version for use in tests and external packages.
///
/// # Why UTF-16?
///
/// LSP character offsets are UTF-16 code units because VS Code's internal
/// string representation is UTF-16. This function bridges the gap to
/// Swift's UTF-8 strings.
///
/// # Example
///
///   let text = "hello guitar-emoji world"
///   // guitar-emoji (U+1F3B8) is 4 UTF-8 bytes but 2 UTF-16 code units.
///   let byteOff = convertUTF16OffsetToByteOffset(text, line: 0, char: 8)
///   // byteOff = 11
public func convertUTF16OffsetToByteOffset(_ text: String, line: Int, char: Int) -> Int {
    return convertPositionToByteOffset(text, Position(line: line, character: char)) ?? text.utf8.count
}

/// Decode one UTF-8 codepoint starting at the given byte offset.
///
/// Returns (codepoint, byteCount). The byteCount is how many bytes
/// this codepoint consumed (1-4).
private func decodeUTF8(_ bytes: [UInt8], _ offset: Int) -> (UInt32, Int) {
    let b0 = bytes[offset]

    if b0 & 0x80 == 0 {
        // 1-byte: 0xxxxxxx (ASCII)
        return (UInt32(b0), 1)
    } else if b0 & 0xE0 == 0xC0 {
        // 2-byte: 110xxxxx 10xxxxxx
        guard offset + 1 < bytes.count else { return (0xFFFD, 1) }
        let cp = (UInt32(b0 & 0x1F) << 6) | UInt32(bytes[offset + 1] & 0x3F)
        return (cp, 2)
    } else if b0 & 0xF0 == 0xE0 {
        // 3-byte: 1110xxxx 10xxxxxx 10xxxxxx
        guard offset + 2 < bytes.count else { return (0xFFFD, 1) }
        let cp = (UInt32(b0 & 0x0F) << 12)
               | (UInt32(bytes[offset + 1] & 0x3F) << 6)
               | UInt32(bytes[offset + 2] & 0x3F)
        return (cp, 3)
    } else if b0 & 0xF8 == 0xF0 {
        // 4-byte: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
        guard offset + 3 < bytes.count else { return (0xFFFD, 1) }
        let cp = (UInt32(b0 & 0x07) << 18)
               | (UInt32(bytes[offset + 1] & 0x3F) << 12)
               | (UInt32(bytes[offset + 2] & 0x3F) << 6)
               | UInt32(bytes[offset + 3] & 0x3F)
        return (cp, 4)
    }

    return (0xFFFD, 1) // invalid UTF-8 start byte
}
