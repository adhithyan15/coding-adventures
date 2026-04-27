# frozen_string_literal: true

# ================================================================
# CodingAdventures::Ls00::DocumentManager
# ================================================================
#
# # The Document Manager's Job
#
# When the user opens a file in VS Code, the editor sends a
# textDocument/didOpen notification with the full file content. From
# that point on, the editor does NOT re-send the entire file on every
# keystroke. Instead, it sends incremental changes: what changed, and
# where. The DocumentManager applies these changes to maintain the
# current text of each open file.
#
#   Editor opens file:   didOpen   -> DocumentManager stores text at version 1
#   User types "X":      didChange -> DocumentManager applies delta -> version 2
#   User saves:          didSave   -> (optional: trigger format)
#   User closes:         didClose  -> DocumentManager removes entry
#
# # Why Version Numbers?
#
# The editor increments the version number with every change. The
# ParseCache uses (uri, version) as its cache key -- if the version
# matches, the cached parse result is still valid.
#
# # UTF-16: The Tricky Part
#
# LSP specifies that character offsets are measured in UTF-16 CODE UNITS.
# This is a historical accident: VS Code is built on TypeScript, which
# uses UTF-16 strings internally.
#
# Ruby strings are UTF-8. A single Unicode codepoint can occupy:
#   - 1 byte in UTF-8 (ASCII, e.g. 'A')
#   - 2 bytes in UTF-8 (e.g. 'e-acute', U+00E9)
#   - 3 bytes in UTF-8 (e.g. CJK character, U+4E2D)
#   - 4 bytes in UTF-8 (e.g. guitar emoji, U+1F3B8)
#
# In UTF-16:
#   - Codepoints in the Basic Multilingual Plane (U+0000-U+FFFF) -> 1 code unit
#   - Codepoints above U+FFFF (emojis, rare CJK) -> 2 code units (surrogate pair)
#
# The guitar emoji (U+1F3B8) is above U+FFFF:
#   UTF-8:  4 bytes
#   UTF-16: 2 code units (surrogate pair)
#
# So if the LSP client says character=8 (UTF-16), we cannot simply slice
# 8 bytes into the UTF-8 Ruby string. We must walk the UTF-8 bytes,
# converting each codepoint to its UTF-16 length, accumulating until we
# reach code unit 8.
#
# ================================================================

module CodingAdventures
  module Ls00
    # Document represents an open file tracked by the DocumentManager.
    Document = Struct.new(:uri, :text, :version, keyword_init: true)

    # TextChange describes one incremental change to a document.
    #
    # If +range+ is nil, +new_text+ replaces the ENTIRE document content
    # (full sync). If +range+ is non-nil, +new_text+ replaces just the
    # specified range (incremental sync).
    TextChange = Struct.new(:range, :new_text, keyword_init: true)

    class DocumentManager
      def initialize
        @docs = {} # uri -> Document
      end

      # Open records a newly opened file.
      #
      # Called when the editor sends textDocument/didOpen. Stores the initial
      # text and version number (typically 1 for a freshly opened file).
      def open(uri, text, version)
        @docs[uri] = Document.new(uri: uri, text: text, version: version)
      end

      # apply_changes applies a list of incremental changes to an open document.
      #
      # Changes are applied in order. If a range is nil, the change replaces
      # the entire document. After all changes, the document's version is updated.
      #
      # Raises RuntimeError if the document is not open.
      def apply_changes(uri, changes, version)
        doc = @docs[uri]
        raise "document not open: #{uri}" unless doc

        changes.each do |change|
          if change.range.nil?
            # Full document replacement -- simplest case.
            doc.text = change.new_text
          else
            # Incremental update: splice new text at the specified range.
            doc.text = apply_range_change(doc.text, change.range, change.new_text)
          end
        end

        doc.version = version
      end

      # get returns the document for a URI, or nil if not open.
      def get(uri)
        @docs[uri]
      end

      # close removes a document from the manager.
      #
      # Called when the editor sends textDocument/didClose. After this, the
      # document's text is no longer tracked.
      def close(uri)
        @docs.delete(uri)
      end

      private

      # apply_range_change splices new_text into text at the given LSP range.
      #
      # It converts LSP's (line, UTF-16-character) coordinates to byte offsets
      # in the UTF-8 Ruby string, then performs the splice.
      def apply_range_change(text, range, new_text)
        start_byte = Ls00.convert_position_to_byte_offset(text, range.start)
        end_byte = Ls00.convert_position_to_byte_offset(text, range.end_pos)

        raise "start offset #{start_byte} > end offset #{end_byte}" if start_byte > end_byte

        end_byte = text.bytesize if end_byte > text.bytesize

        # Build new string by byte-slicing the original
        before = text.byteslice(0, start_byte) || ""
        after = text.byteslice(end_byte..) || ""
        before + new_text + after
      end
    end

    # convert_position_to_byte_offset converts an LSP Position (0-based line,
    # UTF-16 char) to a byte offset in the UTF-8 Ruby string.
    #
    # Algorithm:
    #  1. Walk line-by-line to find the byte offset of the start of the target line.
    #  2. From that offset, walk UTF-8 codepoints, converting each to its UTF-16
    #     length, until we reach the target UTF-16 character offset.
    #
    # This is a module-level function because it is used by both DocumentManager
    # and the test suite.
    def self.convert_position_to_byte_offset(text, pos)
      line_start = 0
      current_line = 0

      # Phase 1: find the byte offset of the start of pos.line.
      # We walk the raw bytes looking for newline (0x0A) characters.
      raw_bytes = text.b # binary string for byte-level scanning
      while current_line < pos.line
        idx = raw_bytes.index("\n".b, line_start)
        if idx.nil?
          # Line number exceeds the number of lines in the file.
          return text.bytesize
        end
        line_start = idx + 1
        current_line += 1
      end

      # Phase 2: from line_start, advance pos.character UTF-16 code units.
      byte_offset = line_start
      utf16_units = 0

      while utf16_units < pos.character && byte_offset < text.bytesize
        # Decode one Unicode codepoint from the UTF-8 stream.
        # Ruby's String#byteslice + force_encoding lets us decode one char.
        remaining = text.byteslice(byte_offset..)
        break if remaining.nil? || remaining.empty?

        remaining.force_encoding("UTF-8")
        char = remaining[0] # first character
        break if char.nil?

        # Don't advance past a newline -- the position is beyond the line end.
        break if char == "\n"

        # How many UTF-16 code units does this codepoint occupy?
        codepoint = char.ord
        utf16_len = codepoint > 0xFFFF ? 2 : 1

        # Would this codepoint overshoot the target character?
        break if utf16_units + utf16_len > pos.character

        byte_offset += char.bytesize
        utf16_units += utf16_len
      end

      byte_offset
    end

    # convert_utf16_offset_to_byte_offset is the public API for converting
    # a 0-based (line, UTF-16 char) position to a byte offset.
    #
    # # Why UTF-16?
    #
    # LSP character offsets are UTF-16 code units because VS Code's internal
    # string representation is UTF-16 (as is JavaScript's String type).
    # This function bridges the gap to Ruby's UTF-8 strings.
    #
    # # Example
    #
    #   text = "hello guitar-emoji world"
    #   # guitar-emoji (U+1F3B8) is 4 UTF-8 bytes but 2 UTF-16 code units.
    #   # After the emoji, LSP says character=8 (6 for "hello ", 2 for emoji).
    #   # But in UTF-8, "world" starts at byte 11 (6 + 4 + 1 for the space).
    #   byte_off = Ls00.convert_utf16_offset_to_byte_offset(text, 0, 8)
    #   # byte_off = 11
    #
    def self.convert_utf16_offset_to_byte_offset(text, line, char)
      convert_position_to_byte_offset(text, Position.new(line: line, character: char))
    end
  end
end
