/**
 * document-manager.ts -- DocumentManager and UTF-16 offset handling
 *
 * # The Document Manager's Job
 *
 * When the user opens a file in VS Code, the editor sends a textDocument/didOpen
 * notification with the full file content. From that point on, the editor does
 * NOT re-send the entire file on every keystroke. Instead, it sends incremental
 * changes: what changed, and where. The DocumentManager applies these changes to
 * maintain the current text of each open file.
 *
 *   Editor opens file:   didOpen   -> DocumentManager stores text at version 1
 *   User types "X":      didChange -> DocumentManager applies delta -> version 2
 *   User saves:          didSave   -> (optional: trigger format)
 *   User closes:         didClose  -> DocumentManager removes entry
 *
 * # Why Version Numbers?
 *
 * The editor increments the version number with every change. The ParseCache
 * uses (uri, version) as its cache key -- if the version matches, the cached
 * parse result is still valid.
 *
 * # UTF-16 in JavaScript
 *
 * JavaScript strings ARE UTF-16 internally. This means:
 *   - `string.length` gives UTF-16 code units (not Unicode codepoints)
 *   - `string.charCodeAt(i)` gives the UTF-16 code unit at index i
 *   - LSP character offsets map directly to JavaScript string indices
 *
 * For most operations, the LSP character offset IS the JavaScript string index.
 * However, when we need to convert to byte offsets (e.g., for Buffer operations),
 * we must account for the fact that UTF-8 byte counts differ from UTF-16 unit counts.
 *
 * The key insight: in JavaScript, LSP's `character` offset can be used directly
 * as a string index -- no conversion needed! A surrogate pair (emoji) is two
 * UTF-16 code units in both LSP and JavaScript, so string.slice(0, character)
 * gives the correct prefix.
 *
 * @module
 */

import type { Position, Range, TextChange } from "./types.js";

// ---------------------------------------------------------------------------
// Document -- one open file
// ---------------------------------------------------------------------------

/**
 * Document represents an open file tracked by the DocumentManager.
 */
export interface Document {
  uri: string;
  text: string;    // current content
  version: number; // monotonically increasing; matches LSP's document version
}

// ---------------------------------------------------------------------------
// DocumentManager -- tracks all open files
// ---------------------------------------------------------------------------

/**
 * DocumentManager tracks all files currently open in the editor.
 *
 * The editor sends open/change/close notifications; this manager keeps the
 * authoritative current text of each file. The ParseCache and all feature
 * handlers read from this manager to get the source text to work on.
 */
export class DocumentManager {
  /** Map from URI to Document. */
  private docs: Map<string, Document> = new Map();

  /**
   * Open records a newly opened file.
   *
   * Called when the editor sends textDocument/didOpen. Stores the initial text
   * and version number (typically 1 for a freshly opened file).
   */
  open(uri: string, text: string, version: number): void {
    this.docs.set(uri, { uri, text, version });
  }

  /**
   * ApplyChanges applies a list of incremental changes to an open document.
   *
   * Changes are applied in order. If a range is undefined, the change replaces
   * the entire document. After all changes, the document's version is updated.
   *
   * Throws an error if the document is not open, or if a range is invalid.
   */
  applyChanges(uri: string, changes: TextChange[], version: number): void {
    const doc = this.docs.get(uri);
    if (!doc) {
      throw new Error(`document not open: ${uri}`);
    }

    for (const change of changes) {
      if (change.range === undefined) {
        // Full document replacement -- simplest case.
        doc.text = change.newText;
      } else {
        // Incremental update: splice new text at the specified range.
        doc.text = applyRangeChange(doc.text, change.range, change.newText);
      }
    }

    doc.version = version;
  }

  /**
   * Get returns the document for a URI, or undefined if not open.
   */
  get(uri: string): Document | undefined {
    return this.docs.get(uri);
  }

  /**
   * Close removes a document from the manager.
   *
   * Called when the editor sends textDocument/didClose. After this, the document's
   * text is no longer tracked.
   */
  close(uri: string): void {
    this.docs.delete(uri);
  }
}

// ---------------------------------------------------------------------------
// Range application -- splicing text at LSP coordinates
// ---------------------------------------------------------------------------

/**
 * applyRangeChange splices newText into text at the given LSP range.
 *
 * It converts LSP's (line, UTF-16-character) coordinates to string indices
 * (which ARE UTF-16 in JavaScript), then performs the splice.
 */
function applyRangeChange(text: string, range: Range, newText: string): string {
  const startIdx = convertPositionToStringIndex(text, range.start);
  const endIdx = convertPositionToStringIndex(text, range.end);

  if (startIdx > endIdx) {
    throw new Error(`start offset ${startIdx} > end offset ${endIdx}`);
  }

  return text.slice(0, startIdx) + newText + text.slice(endIdx);
}

// ---------------------------------------------------------------------------
// Position -> string index conversion
// ---------------------------------------------------------------------------

/**
 * convertPositionToStringIndex converts an LSP Position (0-based line, UTF-16 char)
 * to a JavaScript string index.
 *
 * Since JavaScript strings ARE UTF-16, the `character` field from LSP maps
 * directly to the string index within a line. We just need to find the start
 * of the target line, then add the character offset.
 *
 * Algorithm:
 *  1. Walk line-by-line to find the string index of the start of the target line.
 *  2. From that index, advance `character` positions (clamped to line end).
 */
export function convertPositionToStringIndex(text: string, pos: Position): number {
  let lineStart = 0;
  let currentLine = 0;

  // Phase 1: find the string index of the start of pos.line.
  while (currentLine < pos.line) {
    const idx = text.indexOf("\n", lineStart);
    if (idx === -1) {
      // Line number exceeds the number of lines in the file. Clamp to end.
      return text.length;
    }
    lineStart = idx + 1; // character AFTER the newline
    currentLine++;
  }

  // Phase 2: from lineStart, advance pos.character UTF-16 code units.
  // In JavaScript, each string index IS one UTF-16 code unit, so we can
  // simply add pos.character -- but we must clamp to the line end.
  const lineEnd = text.indexOf("\n", lineStart);
  const lineLength = lineEnd === -1 ? text.length - lineStart : lineEnd - lineStart;

  const charOffset = Math.min(pos.character, lineLength);
  return lineStart + charOffset;
}

/**
 * convertUTF16OffsetToByteOffset converts a 0-based (line, UTF-16 char) position
 * to a byte offset in a UTF-8 encoding of the string.
 *
 * This is needed when interfacing with systems that count bytes (like Node.js
 * Buffers) rather than UTF-16 code units.
 *
 * # Why this function exists
 *
 * Even though JavaScript strings are UTF-16 (making LSP offsets easy to use
 * as string indices), sometimes we need byte offsets -- for example, when
 * computing Content-Length for JSON-RPC framing or when interfacing with
 * native code that uses UTF-8.
 *
 * # Example
 *
 *     const text = "hello \u{1F3B8} world";
 *     // \u{1F3B8} (guitar emoji) is 2 UTF-16 code units but 4 UTF-8 bytes.
 *     // After the emoji, LSP says character=8 (6 for "hello ", 2 for emoji).
 *     // But in UTF-8, " world" starts at byte 11 (6 + 4 + 1 for the space).
 *     const byteOff = convertUTF16OffsetToByteOffset(text, 0, 8);
 *     // byteOff = 11
 */
export function convertUTF16OffsetToByteOffset(text: string, line: number, char: number): number {
  const strIndex = convertPositionToStringIndex(text, { line, character: char });
  // Get the substring up to the target position and count its UTF-8 bytes.
  const prefix = text.slice(0, strIndex);
  return Buffer.byteLength(prefix, "utf8");
}
