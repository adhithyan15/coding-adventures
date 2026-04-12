package ls00

// document_manager.go — DocumentManager and UTF-16 offset handling
//
// # The Document Manager's Job
//
// When the user opens a file in VS Code, the editor sends a textDocument/didOpen
// notification with the full file content. From that point on, the editor does
// NOT re-send the entire file on every keystroke. Instead, it sends incremental
// changes: what changed, and where. The DocumentManager applies these changes to
// maintain the current text of each open file.
//
//   Editor opens file:   didOpen   → DocumentManager stores text at version 1
//   User types "X":      didChange → DocumentManager applies delta → version 2
//   User saves:          didSave   → (optional: trigger format)
//   User closes:         didClose  → DocumentManager removes entry
//
// # Why Version Numbers?
//
// The editor increments the version number with every change. The ParseCache
// uses (uri, version) as its cache key — if the version matches, the cached
// parse result is still valid. This avoids re-parsing the file on every
// keystroke when the user is just moving the cursor.
//
// # UTF-16: The Tricky Part
//
// LSP specifies that character offsets are measured in UTF-16 CODE UNITS.
// This is a historical accident: VS Code is built on TypeScript, which uses
// UTF-16 strings internally (like Java and C#). Since LSP was designed for VS Code,
// it inherited this convention.
//
// Go strings are UTF-8. A single Unicode codepoint can occupy:
//   - 1 byte in UTF-8 (ASCII, e.g. 'A')
//   - 2 bytes in UTF-8 (e.g. 'é', U+00E9)
//   - 3 bytes in UTF-8 (e.g. '中', U+4E2D)
//   - 4 bytes in UTF-8 (e.g. '🎸', U+1F3B8, guitar emoji)
//
// In UTF-16:
//   - Codepoints in the Basic Multilingual Plane (U+0000–U+FFFF) → 1 code unit (2 bytes)
//   - Codepoints above U+FFFF (emojis, rare CJK) → 2 code units (a "surrogate pair")
//
// The guitar emoji 🎸 (U+1F3B8) is above U+FFFF:
//   UTF-8:  4 bytes  (0xF0 0x9F 0x8E 0xB8)
//   UTF-16: 2 code units (surrogate pair: 0xD83C 0xDFB8)
//
// So if the LSP client says character=8 (UTF-16), we cannot simply slice 8 bytes
// into the UTF-8 Go string. We must walk the UTF-8 bytes, converting each
// codepoint to its UTF-16 length, accumulating until we reach code unit 8.
//
// The function convertUTF16OffsetToByteOffset below performs this conversion.

import (
	"fmt"
	"unicode/utf16"
	"unicode/utf8"
)

// Document represents an open file tracked by the DocumentManager.
type Document struct {
	URI     string
	Text    string // current content, UTF-8 encoded
	Version int    // monotonically increasing; matches LSP's document version
}

// DocumentManager tracks all files currently open in the editor.
//
// The editor sends open/change/close notifications; this manager keeps the
// authoritative current text of each file. The ParseCache and all feature
// handlers read from this manager to get the source text to work on.
type DocumentManager struct {
	docs map[string]*Document // uri → document
}

// NewDocumentManager creates an empty DocumentManager.
func NewDocumentManager() *DocumentManager {
	return &DocumentManager{docs: make(map[string]*Document)}
}

// Open records a newly opened file.
//
// Called when the editor sends textDocument/didOpen. Stores the initial text
// and version number (typically 1 for a freshly opened file).
func (dm *DocumentManager) Open(uri, text string, version int) {
	dm.docs[uri] = &Document{URI: uri, Text: text, Version: version}
}

// TextChange describes one incremental change to a document.
//
// If Range is nil, NewText replaces the ENTIRE document content (full sync).
// If Range is non-nil, NewText replaces just the specified range (incremental sync).
//
// The LSP textDocumentSync capability controls which mode the editor uses:
//   - textDocumentSync=1 → full sync (range is always nil)
//   - textDocumentSync=2 → incremental sync (range specifies what changed)
//
// We advertise textDocumentSync=2 (incremental) in our capabilities, but
// we handle both modes for robustness.
type TextChange struct {
	Range   *Range // nil = full replacement
	NewText string
}

// ApplyChanges applies a list of incremental changes to an open document.
//
// Changes are applied in order. If a range is nil, the change replaces the
// entire document. After all changes, the document's version is updated.
//
// Returns an error if the document is not open, or if a range is invalid.
func (dm *DocumentManager) ApplyChanges(uri string, changes []TextChange, version int) error {
	doc, ok := dm.docs[uri]
	if !ok {
		return fmt.Errorf("document not open: %s", uri)
	}

	for _, change := range changes {
		if change.Range == nil {
			// Full document replacement — simplest case.
			doc.Text = change.NewText
		} else {
			// Incremental update: splice new text at the specified range.
			newText, err := applyRangeChange(doc.Text, *change.Range, change.NewText)
			if err != nil {
				return fmt.Errorf("applying change to %s: %w", uri, err)
			}
			doc.Text = newText
		}
	}

	doc.Version = version
	return nil
}

// Get returns the document for a URI, or false if the document is not open.
func (dm *DocumentManager) Get(uri string) (*Document, bool) {
	doc, ok := dm.docs[uri]
	return doc, ok
}

// Close removes a document from the manager.
//
// Called when the editor sends textDocument/didClose. After this, the document's
// text is no longer tracked. Further feature requests for this URI will fail.
func (dm *DocumentManager) Close(uri string) {
	delete(dm.docs, uri)
}

// ─── Range application ────────────────────────────────────────────────────────

// applyRangeChange splices newText into text at the given LSP range.
//
// It converts LSP's (line, UTF-16-character) coordinates to byte offsets in the
// UTF-8 Go string, then performs the splice.
func applyRangeChange(text string, r Range, newText string) (string, error) {
	startByte, err := convertPositionToByteOffset(text, r.Start)
	if err != nil {
		return "", fmt.Errorf("start position: %w", err)
	}
	endByte, err := convertPositionToByteOffset(text, r.End)
	if err != nil {
		return "", fmt.Errorf("end position: %w", err)
	}

	if startByte > endByte {
		return "", fmt.Errorf("start offset %d > end offset %d", startByte, endByte)
	}
	if endByte > len(text) {
		endByte = len(text)
	}

	return text[:startByte] + newText + text[endByte:], nil
}

// convertPositionToByteOffset converts an LSP Position (0-based line, UTF-16 char)
// to a byte offset in the UTF-8 Go string.
//
// Algorithm:
//  1. Walk line-by-line to find the byte offset of the start of the target line.
//  2. From that offset, walk UTF-8 codepoints, converting each to its UTF-16
//     length, until we reach the target UTF-16 character offset.
func convertPositionToByteOffset(text string, pos Position) (int, error) {
	lineStart := 0
	currentLine := 0

	// Phase 1: find the byte offset of the start of pos.Line.
	// We walk the string looking for newline characters.
	for currentLine < pos.Line {
		idx := indexByte(text, lineStart, '\n')
		if idx == -1 {
			// Line number exceeds the number of lines in the file.
			// Clamp to end of file.
			return len(text), nil
		}
		lineStart = idx + 1 // byte AFTER the newline
		currentLine++
	}

	// Phase 2: from lineStart, advance pos.Character UTF-16 code units.
	byteOffset := lineStart
	utf16Units := 0

	for utf16Units < pos.Character && byteOffset < len(text) {
		// Decode one Unicode codepoint from the UTF-8 stream.
		r, size := utf8.DecodeRuneInString(text[byteOffset:])
		if r == '\n' {
			// Don't advance past the newline — the position is beyond the line end.
			break
		}

		// How many UTF-16 code units does this codepoint occupy?
		// - Codepoints in the BMP (U+0000–U+FFFF) → 1 unit
		// - Codepoints above U+FFFF (emoji, etc.) → 2 units (surrogate pair)
		utf16Len := utf16UnitLength(r)

		if utf16Units+utf16Len > pos.Character {
			// This codepoint would overshoot the target character. Stop here.
			// This can happen in the middle of a surrogate pair.
			break
		}

		byteOffset += size
		utf16Units += utf16Len
	}

	return byteOffset, nil
}

// convertUTF16OffsetToByteOffset converts a 0-based (line, UTF-16 char) position
// to a byte offset in a UTF-8 Go string.
//
// This is the exported version for use in tests and external packages.
//
// # Why UTF-16?
//
// LSP character offsets are UTF-16 code units because VS Code's internal
// string representation is UTF-16 (as is JavaScript's String type).
// This function bridges the gap to Go's UTF-8 strings.
//
// # Example
//
//	text := "hello 🎸 world"
//	// 🎸 (U+1F3B8) is 4 UTF-8 bytes but 2 UTF-16 code units.
//	// After the guitar emoji, LSP says character=8 (6 for "hello ", 2 for 🎸).
//	// But in UTF-8, "world" starts at byte 11 (6 + 4 + 1 for the space).
//	byteOff := ConvertUTF16OffsetToByteOffset(text, 0, 8)
//	// byteOff = 11
func ConvertUTF16OffsetToByteOffset(text string, line, char int) int {
	offset, _ := convertPositionToByteOffset(text, Position{Line: line, Character: char})
	return offset
}

// utf16UnitLength returns the number of UTF-16 code units required to encode r.
//
// The UTF-16 encoding works as follows:
//   - BMP codepoints (U+0000–U+FFFF): 1 code unit (2 bytes in UTF-16)
//   - Non-BMP codepoints (U+10000–U+10FFFF): 2 code units (4 bytes, a surrogate pair)
//
// The cutoff is utf16.MaxRune for surrogates (U+10000), above which IsSurrogate
// becomes false and we need 2 code units.
func utf16UnitLength(r rune) int {
	// utf16.IsSurrogate reports whether r requires a surrogate pair in UTF-16.
	// For runes above U+FFFF, it's not technically a surrogate but requires a pair.
	// The standard library encodes this with EncodeRune.
	r1, _ := utf16.EncodeRune(r)
	if r1 == utf8.RuneError {
		// BMP codepoint: fits in one UTF-16 code unit.
		return 1
	}
	// Non-BMP: requires a surrogate pair = 2 UTF-16 code units.
	return 2
}

// indexByte returns the index of the first occurrence of b in text[from:],
// or -1 if not found. This is a simple linear scan — for short lines it's fast.
func indexByte(text string, from int, b byte) int {
	for i := from; i < len(text); i++ {
		if text[i] == b {
			return i
		}
	}
	return -1
}
