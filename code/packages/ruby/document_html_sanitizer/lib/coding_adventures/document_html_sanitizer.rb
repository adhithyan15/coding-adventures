# frozen_string_literal: true

# coding_adventures-document_html_sanitizer
#
# Regex-based HTML string sanitizer with no dependency on document-ast.
# String in, string out.
#
# == Usage
#
#   require "coding_adventures/document_html_sanitizer"
#
#   include CodingAdventures::DocumentHtmlSanitizer
#
#   # Strict mode — untrusted external HTML
#   safe = CodingAdventures::DocumentHtmlSanitizer.sanitize_html(html, HTML_STRICT)
#
#   # Custom policy — keep comments, allow ftp
#   policy = HTML_STRICT.with(drop_comments: false,
#                              allowed_url_schemes: %w[http https mailto ftp])
#   safe = CodingAdventures::DocumentHtmlSanitizer.sanitize_html(html, policy)

require_relative "document_html_sanitizer/version"
require_relative "document_html_sanitizer/policy"
require_relative "document_html_sanitizer/url_utils"
require_relative "document_html_sanitizer/html_sanitizer"
