# frozen_string_literal: true

# coding_adventures_asciidoc_parser — AsciiDoc Parser
#
# Parses AsciiDoc source text into a Document AST — the format-agnostic IR
# defined in coding_adventures_document_ast. The result is a DocumentNode
# ready for any back-end renderer (HTML, PDF, plain text, ...).
#
# The parse is two-phase:
#   Phase 1 — Block structure: headings, lists, code blocks, blockquotes, ...
#   Phase 2 — Inline content: strong, emphasis, links, images, code spans, ...
#
# === Quick Start ===
#
#   require "coding_adventures_asciidoc_parser"
#
#   doc = CodingAdventures::AsciidocParser.parse("= Hello\n\nWorld *bold*.\n")
#   doc.type               # => "document"
#   doc.children[0].type   # => "heading"
#   doc.children[1].type   # => "paragraph"
#
# === AsciiDoc conventions ===
#
#   *text*   → StrongNode (bold)    — NOT EmphasisNode!
#   _text_   → EmphasisNode (italic)
#   **text** → StrongNode (unconstrained)
#   __text__ → EmphasisNode (unconstrained)
#   `code`   → CodeSpanNode (verbatim — no nested parsing)
#
# Spec: TE03 — AsciiDoc Parser

require "coding_adventures_document_ast"

require_relative "coding_adventures/asciidoc_parser/version"
require_relative "coding_adventures/asciidoc_parser/inline_parser"
require_relative "coding_adventures/asciidoc_parser/block_parser"

module CodingAdventures
  # AsciiDoc parser — converts AsciiDoc source text into a DocumentNode AST.
  module AsciidocParser
    # Parse an AsciiDoc source string into a DocumentNode AST.
    #
    # The result conforms to the Document AST spec (TE00) — a format-agnostic
    # IR with all inline markup parsed and all cross-references resolved.
    #
    # @param text [String] The AsciiDoc source string.
    # @return [DocumentAst::DocumentNode] The root document node.
    #
    # @example
    #   doc = CodingAdventures::AsciidocParser.parse("= Title\n\n- item 1\n- item 2\n")
    #   doc.children[0].type   # => "heading"
    def self.parse(text)
      BlockParser.parse(text)
    end
  end
end
