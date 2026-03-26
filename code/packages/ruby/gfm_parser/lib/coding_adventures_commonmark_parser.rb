# frozen_string_literal: true

# coding_adventures_gfm_parser — GFM 0.31.2 Markdown Parser
#
# Parses Markdown source text into a Document AST — the format-agnostic IR
# defined in coding_adventures_document_ast. The result is a DocumentNode
# ready for any back-end renderer (HTML, PDF, plain text, ...).
#
# The parse is two-phase:
#   Phase 1 — Block structure: headings, lists, code blocks, blockquotes, ...
#   Phase 2 — Inline content: emphasis, links, images, code spans, ...
#
# === Quick Start ===
#
#   require "coding_adventures_gfm_parser"
#
#   doc = CodingAdventures::CommonmarkParser.parse("# Hello\n\nWorld *with* emphasis.\n")
#   doc.type               # => "document"
#   doc.children[0].type   # => "heading"
#   doc.children[1].type   # => "paragraph"
#
# Spec: TE01 — GFM Parser

require "coding_adventures_document_ast"
require "coding_adventures_state_machine"

require_relative "coding_adventures/commonmark_parser/version"
require_relative "coding_adventures/commonmark_parser/entities"
require_relative "coding_adventures/commonmark_parser/scanner"
require_relative "coding_adventures/commonmark_parser/block_parser"
require_relative "coding_adventures/commonmark_parser/inline_parser"

module CodingAdventures
  module CommonmarkParser
    # Parse a GitHub Flavored Markdown string into a DocumentNode AST.
    #
    # The result conforms to the Document AST spec (TE00) — a format-agnostic
    # IR with all link references resolved and all inline markup parsed.
    #
    # @param markdown [String] The Markdown source string.
    # @return [DocumentAst::DocumentNode] The root document node.
    #
    # @example
    #   doc = CodingAdventures::CommonmarkParser.parse("## Heading\n\n- item 1\n- item 2\n")
    #   doc.children[0].type   # => "heading"
    #   doc.children[1].type   # => "list"
    def self.parse(markdown)
      # Phase 1: Block parsing — builds the structural skeleton
      mutable_doc, link_refs = BlockParser.parse_blocks(markdown)

      # Phase 2: AST conversion with inline parsing — converts mutable blocks to
      # immutable DocumentAst nodes, parsing inline content (emphasis, links,
      # code spans, etc.) eagerly as each node is constructed.
      BlockParser.convert_to_ast(mutable_doc, link_refs)
    end

    # Expose BlockParser and InlineParser as namespaced submodules
    module BlockParser
      def self.parse_blocks(markdown)
        CommonmarkParser.parse_blocks(markdown)
      end

      def self.convert_to_ast(mutable_doc, link_refs)
        CommonmarkParser.convert_to_ast(mutable_doc, link_refs)
      end
    end

    module InlineParser
      def self.resolve_inline_content(document, raw_inline_content, link_refs)
        CommonmarkParser.resolve_inline_content(document, raw_inline_content, link_refs)
      end
    end
  end
end
