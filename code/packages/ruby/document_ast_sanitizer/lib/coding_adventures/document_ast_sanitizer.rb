# frozen_string_literal: true

# coding_adventures-document_ast_sanitizer
#
# Policy-driven AST sanitizer for the Document AST pipeline.
#
# == Usage
#
#   require "coding_adventures/document_ast_sanitizer"
#
#   include CodingAdventures::DocumentAstSanitizer
#
#   # Strict mode for user-generated content
#   safe_doc = CodingAdventures::DocumentAstSanitizer.sanitize(doc, STRICT)
#   html = CodingAdventures::DocumentAstToHtml.to_html(safe_doc)
#
#   # Custom policy — reserve h1 for page title, allow HTML raw blocks
#   policy = RELAXED.with(min_heading_level: 2)
#   safe_doc = CodingAdventures::DocumentAstSanitizer.sanitize(doc, policy)

require "coding_adventures_document_ast"

require_relative "document_ast_sanitizer/version"
require_relative "document_ast_sanitizer/policy"
require_relative "document_ast_sanitizer/url_utils"
require_relative "document_ast_sanitizer/sanitizer"
