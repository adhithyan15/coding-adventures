# frozen_string_literal: true

# URL Utilities for the AST sanitizer
#
# URL sanitization happens in three steps:
#
#   1. Strip "invisible" characters that browsers silently ignore when
#      parsing URL schemes — this closes control-character bypass attacks.
#
#   2. Extract the scheme (everything before the first ":").
#
#   3. Check the scheme against the policy's allowed list.
#
# == Why strip control characters first?
#
# WHATWG URL parsing normalises C0 controls and certain Unicode zero-width
# characters away before scheme detection. An attacker can exploit this to
# sneak past a naive regex check:
#
#   "java\x00script:alert(1)"  →  browser sees "javascript:alert(1)"
#   "\u200Bjavascript:alert(1)" →  browser strips the zero-width space
#
# We replicate this normalisation so our check matches what browsers do.
#
# == What counts as a relative URL?
#
# A URL is relative if it has no scheme at all, or if the ":" appears
# after a "/" or "?" (meaning the ":" is part of the path, not the scheme).
# Relative URLs always pass through regardless of the policy.
#
#   "/images/cat.png"          → relative, no colon in scheme position
#   "page.html?k=v:w"          → relative, colon is in query string
#   "https://example.com"      → absolute, scheme "https"
#   "javascript:alert(1)"      → absolute, scheme "javascript"

module CodingAdventures
  module DocumentAstSanitizer
    module UrlUtils
      # Characters stripped before scheme detection.
      #
      # C0 controls (U+0000–U+001F): TAB, LF, CR, NUL, ESC, etc.
      # Zero-width Unicode: U+200B ZERO WIDTH SPACE
      #                     U+200C ZERO WIDTH NON-JOINER
      #                     U+200D ZERO WIDTH JOINER
      #                     U+2060 WORD JOINER
      #                     U+FEFF BOM / ZERO WIDTH NO-BREAK SPACE
      CONTROL_CHARS = /[\u0000-\u001F\u200B-\u200D\u2060\uFEFF]/u

      # Strip C0 controls and zero-width characters from the URL.
      # This mirrors what WHATWG URL parsing does implicitly.
      #
      # @param url [String]
      # @return [String]
      def self.strip_control_chars(url)
        url.gsub(CONTROL_CHARS, "")
      end

      # Extract the URL scheme (everything before the first ":").
      #
      # Returns nil if the URL has no scheme (i.e., it is relative).
      #
      # A URL is treated as relative (no scheme) when:
      #   - There is no ":" in the string at all.
      #   - The ":" appears after "/" or "?" — meaning it is inside a path
      #     or query string, not a scheme delimiter.
      #
      # Examples:
      #   "https://example.com"  → "https"
      #   "javascript:alert(1)"  → "javascript"
      #   "/path/to/page"        → nil  (no colon)
      #   "page?k=v:w"           → nil  (colon after "?")
      #   "path/segment:colon"   → nil  (colon after "/")
      #
      # @param url [String]
      # @return [String, nil] The lowercase scheme, or nil if relative.
      def self.extract_scheme(url)
        colon_pos = url.index(":")
        return nil if colon_pos.nil?

        # If a "/" or "?" appears before the first ":", the colon is part of
        # the path/query — not a scheme separator.
        slash_pos = url.index("/")
        query_pos = url.index("?")

        if slash_pos && slash_pos < colon_pos
          return nil
        end
        if query_pos && query_pos < colon_pos
          return nil
        end

        url[0, colon_pos].downcase
      end

      # Check whether the URL is allowed by the given scheme allowlist.
      #
      # Logic table:
      #
      #   allowed_schemes | URL type  | Result
      #   ────────────────┼───────────┼────────
      #   nil             | any       | true   (nil = allow everything)
      #   [...]           | relative  | true   (relative URLs always pass)
      #   [...]           | absolute  | scheme in list?
      #
      # @param url [String] The URL after control-char stripping.
      # @param allowed_schemes [Array<String>, nil] Lowercase scheme names,
      #   or nil to allow any scheme.
      # @return [Boolean]
      def self.scheme_allowed?(url, allowed_schemes)
        return true if allowed_schemes.nil?

        scheme = extract_scheme(url)
        return true if scheme.nil? # relative URL

        allowed_schemes.include?(scheme)
      end

      # Apply URL sanitization: strip control chars then check the scheme.
      #
      # Returns the cleaned URL if allowed, or "" if the scheme is blocked.
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
