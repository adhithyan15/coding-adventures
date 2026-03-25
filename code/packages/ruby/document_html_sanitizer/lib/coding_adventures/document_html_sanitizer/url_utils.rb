# frozen_string_literal: true

# URL utilities for the HTML sanitizer (independent copy — no shared dep)
#
# This is intentionally a standalone copy rather than a shared library.
# The HTML sanitizer has NO dependency on document-ast, and the AST
# sanitizer has NO dependency on the HTML sanitizer. Each package is
# self-contained and deployable independently.
#
# The logic is identical to the AST sanitizer's url_utils.rb.

module CodingAdventures
  module DocumentHtmlSanitizer
    module UrlUtils
      # C0 control characters and Unicode zero-width characters that browsers
      # silently strip when parsing URL schemes (WHATWG URL spec).
      CONTROL_CHARS = /[\u0000-\u001F\u200B-\u200D\u2060\uFEFF]/u

      # Strip invisible characters before scheme detection.
      #
      # @param url [String]
      # @return [String]
      def self.strip_control_chars(url)
        url.gsub(CONTROL_CHARS, "")
      end

      # Extract the lowercase scheme from a URL (everything before the first ":").
      #
      # Returns nil for relative URLs (no scheme).
      #
      # @param url [String]
      # @return [String, nil]
      def self.extract_scheme(url)
        colon_pos = url.index(":")
        return nil if colon_pos.nil?

        slash_pos = url.index("/")
        query_pos = url.index("?")

        return nil if slash_pos && slash_pos < colon_pos
        return nil if query_pos && query_pos < colon_pos

        url[0, colon_pos].downcase
      end

      # Check whether the URL passes the given scheme allowlist.
      #
      # @param url [String] Already control-char-stripped.
      # @param allowed_schemes [Array<String>, nil]
      # @return [Boolean]
      def self.scheme_allowed?(url, allowed_schemes)
        return true if allowed_schemes.nil?

        scheme = extract_scheme(url)
        return true if scheme.nil? # relative URL

        allowed_schemes.include?(scheme)
      end

      # Strip controls and validate scheme. Return "" if blocked.
      #
      # @param url [String]
      # @param allowed_schemes [Array<String>, nil]
      # @return [String]
      def self.sanitize_url(url, allowed_schemes)
        cleaned = strip_control_chars(url)
        scheme_allowed?(cleaned, allowed_schemes) ? cleaned : ""
      end
    end
  end
end
