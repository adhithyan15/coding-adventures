# frozen_string_literal: true

# coding_adventures_document_ast_to_html — Document AST → HTML Renderer
#
# Converts a Document AST (produced by any front-end parser that implements
# the TE00 Document AST spec) into an HTML string. The result is a valid HTML
# fragment ready to embed in a page body.
#
# === Quick Start ===
#
#   require "coding_adventures_document_ast_to_html"
#   require "coding_adventures_commonmark_parser"
#
#   doc  = CodingAdventures::CommonmarkParser.parse("# Hello\n\nWorld\n")
#   html = CodingAdventures::DocumentAstToHtml.to_html(doc)
#   # => "<h1>Hello</h1>\n<p>World</p>\n"
#
#   # Strip raw HTML for untrusted input:
#   html = CodingAdventures::DocumentAstToHtml.to_html(doc, sanitize: true)
#
# Spec: TE00 — Document AST, TE02 — Document AST to HTML

require "coding_adventures_document_ast"

require_relative "coding_adventures/document_ast_to_html/version"
require_relative "coding_adventures/document_ast_to_html/renderer"

module CodingAdventures
  module DocumentAstToHtml
    # Convenience alias so callers can use the short form:
    #   CodingAdventures::DocumentAstToHtml.to_html(doc)
    #
    # The full implementation lives in renderer.rb.
  end
end
