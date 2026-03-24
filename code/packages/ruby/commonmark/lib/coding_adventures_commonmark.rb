# frozen_string_literal: true

# coding_adventures_commonmark — CommonMark pipeline convenience package
#
# This is the public-facing convenience package that combines the parser and
# renderer into a minimal two-method API:
#
#   - CodingAdventures::Commonmark.parse(markdown) → DocumentNode
#   - CodingAdventures::Commonmark.to_html(doc)    → String
#   - CodingAdventures::Commonmark.parse_to_html(markdown) → String
#
# === Dependency diagram ===
#
#   coding_adventures_document_ast          ← format-agnostic types
#          ↓ types                                ↓ types
#   coding_adventures_commonmark_parser     coding_adventures_document_ast_to_html
#     parse(markdown) → DocumentNode          to_html(doc) → String
#          ↓ depends on both
#   coding_adventures_commonmark            ← you are here
#     html = Commonmark.parse_to_html(markdown)
#
# === Quick Start ===
#
#   require "coding_adventures_commonmark"
#
#   html = CodingAdventures::Commonmark.parse_to_html("# Hello\n\nWorld\n")
#   # => "<h1>Hello</h1>\n<p>World</p>\n"
#
#   # Two-step (to work with the AST):
#   doc  = CodingAdventures::Commonmark.parse("# Hello\n\nWorld\n")
#   html = CodingAdventures::Commonmark.to_html(doc)
#
# Users who need to work with the AST directly or plug in a custom renderer
# should use the constituent packages directly:
#
#   require "coding_adventures_commonmark_parser"
#   require "coding_adventures_document_ast_to_html"
#   doc  = CodingAdventures::CommonmarkParser.parse(markdown)
#   html = CodingAdventures::DocumentAstToHtml.to_html(doc)
#
# Spec: TE00 (Document AST), TE01 (CommonMark Parser), TE02 (Document AST to HTML)

require "coding_adventures_document_ast"
require "coding_adventures_commonmark_parser"
require "coding_adventures_document_ast_to_html"

require_relative "coding_adventures/commonmark/version"

module CodingAdventures
  # CommonMark pipeline — convenience wrapper over the parser and HTML renderer.
  #
  # Provides a simple two-step (or one-step) API for the most common use case:
  # converting a CommonMark Markdown string to an HTML string.
  module Commonmark
    # Parse a CommonMark Markdown string into a DocumentNode AST.
    #
    # Delegates to CodingAdventures::CommonmarkParser.parse.
    #
    # @param markdown [String] The Markdown source string.
    # @return [DocumentAst::DocumentNode] The root document node.
    #
    # @example
    #   doc = CodingAdventures::Commonmark.parse("# Hello\n")
    #   doc.type               # => "document"
    #   doc.children[0].type   # => "heading"
    def self.parse(markdown)
      CodingAdventures::CommonmarkParser.parse(markdown)
    end

    # Render a DocumentNode AST to an HTML string.
    #
    # Delegates to CodingAdventures::DocumentAstToHtml.to_html.
    #
    # @param document [DocumentAst::DocumentNode] The root document node.
    # @param sanitize [Boolean] Strip raw HTML (for untrusted input).
    # @return [String] An HTML fragment string.
    #
    # @example
    #   html = CodingAdventures::Commonmark.to_html(doc)
    #   html = CodingAdventures::Commonmark.to_html(doc, sanitize: true)
    def self.to_html(document, sanitize: false)
      CodingAdventures::DocumentAstToHtml.to_html(document, sanitize: sanitize)
    end

    # Convert a CommonMark Markdown string directly to an HTML string.
    #
    # This is a convenience wrapper for the common case where you just need
    # the HTML output and do not care about the intermediate AST.
    #
    # @param markdown [String] The Markdown source string.
    # @param sanitize [Boolean] Strip raw HTML (for untrusted input).
    # @return [String] An HTML fragment string.
    #
    # @example
    #   html = CodingAdventures::Commonmark.parse_to_html("# Hello\n\nWorld\n")
    #   # => "<h1>Hello</h1>\n<p>World</p>\n"
    #
    #   # Strip raw HTML for untrusted user-supplied Markdown:
    #   html = CodingAdventures::Commonmark.parse_to_html(user_markdown, sanitize: true)
    def self.parse_to_html(markdown, sanitize: false)
      document = parse(markdown)
      to_html(document, sanitize: sanitize)
    end
  end
end
