# frozen_string_literal: true

# coding_adventures_asciidoc — AsciiDoc pipeline convenience package
#
# This is the public-facing convenience package that combines the AsciiDoc
# parser and the Document AST HTML renderer into a minimal one-method API:
#
#   CodingAdventures::Asciidoc.to_html(text) → String
#
# === Dependency diagram ===
#
#   coding_adventures_document_ast          ← format-agnostic types
#          ↓ types                                ↓ types
#   coding_adventures_asciidoc_parser       coding_adventures_document_ast_to_html
#     parse(text) → DocumentNode              to_html(doc) → String
#          ↓ depends on both
#   coding_adventures_asciidoc              ← you are here
#     html = Asciidoc.to_html(asciidoc_text)
#
# === Quick Start ===
#
#   require "coding_adventures_asciidoc"
#
#   html = CodingAdventures::Asciidoc.to_html("= Hello\n\nWorld *bold*.\n")
#   # => "<h1>Hello</h1>\n<p>World <strong>bold</strong>.</p>\n"
#
# Spec: TE03 — AsciiDoc Parser

require "coding_adventures_document_ast"
require "coding_adventures_asciidoc_parser"
require "coding_adventures_document_ast_to_html"

require_relative "coding_adventures/asciidoc/version"

module CodingAdventures
  # AsciiDoc pipeline convenience wrapper.
  #
  # Provides a simple one-call API for the most common use case:
  # converting an AsciiDoc string to an HTML fragment.
  module Asciidoc
    # Convert an AsciiDoc source string to an HTML fragment.
    #
    # This is equivalent to:
    #   doc  = CodingAdventures::AsciidocParser.parse(text)
    #   html = CodingAdventures::DocumentAstToHtml.to_html(doc)
    #
    # @param text [String] The AsciiDoc source string.
    # @return [String] An HTML fragment (no <html>/<body> wrapper).
    #
    # @example
    #   CodingAdventures::Asciidoc.to_html("= Title\n\nHello.\n")
    #   # => "<h1>Title</h1>\n<p>Hello.</p>\n"
    def self.to_html(text)
      doc = CodingAdventures::AsciidocParser.parse(text)
      CodingAdventures::DocumentAstToHtml.to_html(doc)
    end
  end
end
