"""DocumentManager and UTF-16 offset handling.

The Document Manager's Job
----------------------------

When the user opens a file in VS Code, the editor sends a
``textDocument/didOpen`` notification with the full file content. From that
point on, the editor does NOT re-send the entire file on every keystroke.
Instead, it sends incremental changes: what changed, and where. The
DocumentManager applies these changes to maintain the current text of each
open file.

::

    Editor opens file:   didOpen   -> DocumentManager stores text at version 1
    User types "X":      didChange -> DocumentManager applies delta -> version 2
    User saves:          didSave   -> (optional: trigger format)
    User closes:         didClose  -> DocumentManager removes entry

Why Version Numbers?
---------------------

The editor increments the version number with every change. The ParseCache
uses ``(uri, version)`` as its cache key -- if the version matches, the
cached parse result is still valid.

UTF-16: The Tricky Part
--------------------------

LSP specifies that character offsets are measured in UTF-16 CODE UNITS.
This is a historical accident: VS Code is built on TypeScript, which uses
UTF-16 strings internally (like Java and C#).

Python strings are sequences of Unicode codepoints (internally UCS-2 or
UCS-4 depending on build). A single Unicode codepoint can occupy:

- 1 byte in UTF-8 (ASCII, e.g. 'A')
- 2 bytes in UTF-8 (e.g. 'e-accent', U+00E9)
- 3 bytes in UTF-8 (e.g. 'zhong', U+4E2D)
- 4 bytes in UTF-8 (e.g. guitar emoji, U+1F3B8)

In UTF-16:
- Codepoints in the Basic Multilingual Plane (U+0000-U+FFFF) -> 1 code unit
- Codepoints above U+FFFF (emojis, rare CJK) -> 2 code units (a "surrogate pair")

The guitar emoji (U+1F3B8) is above U+FFFF:
  UTF-8:  4 bytes  (0xF0 0x9F 0x8E 0xB8)
  UTF-16: 2 code units (surrogate pair: 0xD83C 0xDFB8)

So if the LSP client says character=8 (UTF-16), we cannot simply index 8
characters into the Python string. We must walk codepoints, converting each
to its UTF-16 length, accumulating until we reach code unit 8.
"""

from __future__ import annotations

from dataclasses import dataclass

from ls00.types import Position, Range


# ---------------------------------------------------------------------------
# Document — a single open file
# ---------------------------------------------------------------------------


@dataclass
class Document:
    """Represents an open file tracked by the DocumentManager.

    Attributes:
        uri: The file URI (e.g. ``"file:///home/user/main.py"``).
        text: Current content, as a Python string.
        version: Monotonically increasing; matches LSP's document version.
    """

    uri: str
    text: str
    version: int


# ---------------------------------------------------------------------------
# TextChange — one incremental change to a document
# ---------------------------------------------------------------------------


@dataclass
class TextChange:
    """Describes one incremental change to a document.

    If ``range`` is ``None``, ``new_text`` replaces the ENTIRE document
    content (full sync). If ``range`` is set, ``new_text`` replaces just
    the specified range (incremental sync).

    The LSP ``textDocumentSync`` capability controls which mode the editor uses:
    - ``textDocumentSync=1`` -> full sync (range is always None)
    - ``textDocumentSync=2`` -> incremental sync (range specifies what changed)

    We advertise ``textDocumentSync=2`` (incremental) but handle both modes.
    """

    range: Range | None
    new_text: str


# ---------------------------------------------------------------------------
# DocumentManager — tracks all open files
# ---------------------------------------------------------------------------


class DocumentManager:
    """Tracks all files currently open in the editor.

    The editor sends open/change/close notifications; this manager keeps the
    authoritative current text of each file. The ParseCache and all feature
    handlers read from this manager to get the source text.
    """

    def __init__(self) -> None:
        self._docs: dict[str, Document] = {}

    def open(self, uri: str, text: str, version: int) -> None:
        """Record a newly opened file.

        Called when the editor sends ``textDocument/didOpen``. Stores the
        initial text and version number (typically 1 for a freshly opened file).
        """
        self._docs[uri] = Document(uri=uri, text=text, version=version)

    def apply_changes(
        self, uri: str, changes: list[TextChange], version: int
    ) -> None:
        """Apply a list of incremental changes to an open document.

        Changes are applied in order. If a range is ``None``, the change
        replaces the entire document. After all changes, the document's
        version is updated.

        Raises:
            KeyError: If the document is not open.
        """
        doc = self._docs.get(uri)
        if doc is None:
            raise KeyError(f"document not open: {uri}")

        for change in changes:
            if change.range is None:
                # Full document replacement -- simplest case.
                doc.text = change.new_text
            else:
                # Incremental update: splice new text at the specified range.
                doc.text = _apply_range_change(
                    doc.text, change.range, change.new_text
                )

        doc.version = version

    def get(self, uri: str) -> Document | None:
        """Return the document for a URI, or None if not open."""
        return self._docs.get(uri)

    def close(self, uri: str) -> None:
        """Remove a document from the manager.

        Called when the editor sends ``textDocument/didClose``. After this,
        the document's text is no longer tracked.
        """
        self._docs.pop(uri, None)


# ---------------------------------------------------------------------------
# Range application — splicing text at an LSP range
# ---------------------------------------------------------------------------


def _apply_range_change(text: str, r: Range, new_text: str) -> str:
    """Splice ``new_text`` into ``text`` at the given LSP range.

    Converts LSP's (line, UTF-16-character) coordinates to Python string
    indices, then performs the splice.
    """
    start_idx = _convert_position_to_index(text, r.start)
    end_idx = _convert_position_to_index(text, r.end)

    if start_idx > end_idx:
        start_idx, end_idx = end_idx, start_idx
    if end_idx > len(text):
        end_idx = len(text)

    return text[:start_idx] + new_text + text[end_idx:]


def _convert_position_to_index(text: str, pos: Position) -> int:
    """Convert an LSP Position (0-based line, UTF-16 char) to a Python string index.

    Algorithm:
    1. Walk line-by-line to find the start of the target line.
    2. From that offset, walk codepoints, converting each to its UTF-16
       length, until we reach the target UTF-16 character offset.
    """
    # Phase 1: find the string index of the start of pos.line.
    line_start = 0
    current_line = 0

    while current_line < pos.line:
        idx = text.find("\n", line_start)
        if idx == -1:
            # Line number exceeds the number of lines. Clamp to end.
            return len(text)
        line_start = idx + 1
        current_line += 1

    # Phase 2: from line_start, advance pos.character UTF-16 code units.
    char_idx = line_start
    utf16_units = 0

    while utf16_units < pos.character and char_idx < len(text):
        ch = text[char_idx]
        if ch == "\n":
            # Don't advance past the newline.
            break

        # How many UTF-16 code units does this codepoint occupy?
        cp = ord(ch)
        utf16_len = 2 if cp > 0xFFFF else 1

        if utf16_units + utf16_len > pos.character:
            # This codepoint would overshoot the target character.
            break

        char_idx += 1
        utf16_units += utf16_len

    return char_idx


def convert_utf16_offset_to_byte_offset(text: str, line: int, char: int) -> int:
    """Convert a 0-based (line, UTF-16 char) position to a byte offset in UTF-8.

    This is the exported version for use in tests and external packages.

    Why UTF-16?
    ~~~~~~~~~~~~

    LSP character offsets are UTF-16 code units because VS Code's internal
    string representation is UTF-16 (as is JavaScript's String type). This
    function bridges the gap to Python's strings and UTF-8 encoded bytes.

    Example::

        text = "hello guitar_emoji world"
        # guitar_emoji (U+1F3B8) is 4 UTF-8 bytes but 2 UTF-16 code units.
        # After the guitar emoji, LSP says character=8 (6 for "hello ", 2 for emoji).
        # But in UTF-8, "world" starts at byte 11 (6 + 4 + 1 for the space).
        byte_off = convert_utf16_offset_to_byte_offset(text, 0, 8)
        # byte_off = 11
    """
    # First convert to a Python string index, then compute the byte offset
    # of that index in the UTF-8 encoding of the text.
    idx = _convert_position_to_index(text, Position(line=line, character=char))
    # The byte offset is the length of text[:idx] encoded as UTF-8.
    return len(text[:idx].encode("utf-8"))
