# frozen_string_literal: true

# = URL Parser Test Suite
#
# 44+ tests covering all aspects of URL parsing, ported from the Rust
# implementation. Organized into logical groups:
#
#   1. Basic parsing (simple URLs, paths, all components)
#   2. Scheme variants (https, ftp, mailto)
#   3. Case normalization (scheme and host lowercased)
#   4. Effective port (defaults and explicit)
#   5. Authority reconstruction
#   6. Invalid URLs (missing scheme, bad scheme, bad port)
#   7. Percent-encoding (encode, decode, roundtrip, errors)
#   8. Relative resolution (RFC 1808 algorithm)
#   9. Dot segment removal
#  10. Roundtrip serialization
#  11. Historical URLs (CERN, NCSA Mosaic)
#  12. IPv6 addresses
#  13. Edge cases (trailing slash, query/fragment without path)

require "minitest/autorun"
require "coding_adventures_url_parser"

class TestUrlParser < Minitest::Test
  # Convenience aliases to reduce typing in test methods
  Url = CodingAdventures::UrlParser::Url
  UP = CodingAdventures::UrlParser

  # ==========================================================================
  # Version
  # ==========================================================================

  def test_version_exists
    refute_nil CodingAdventures::UrlParser::VERSION
  end

  # ==========================================================================
  # Basic parsing
  # ==========================================================================

  # The simplest case: just a scheme and host, no path, no port.
  # Path defaults to "/" for authority-based URLs.
  def test_parse_simple_http_url
    url = Url.parse("http://www.example.com")
    assert_equal "http", url.scheme
    assert_equal "www.example.com", url.host
    assert_nil url.port
    assert_equal "/", url.path
    assert_nil url.query
    assert_nil url.fragment
  end

  # A URL with a path but no query or fragment.
  def test_parse_http_with_path
    url = Url.parse("http://www.example.com/docs/page.html")
    assert_equal "http", url.scheme
    assert_equal "www.example.com", url.host
    assert_equal "/docs/page.html", url.path
  end

  # The "kitchen sink" test: every component is present.
  #
  #   http://alice:secret@www.example.com:8080/docs/page.html?q=hello#section2
  #   └─┬─┘ └────┬─────┘└──────┬───────┘└─┬─┘└─────┬───────┘└──┬───┘└───┬───┘
  #  scheme  userinfo        host       port     path         query   fragment
  def test_parse_all_components
    url = Url.parse("http://alice:secret@www.example.com:8080/docs/page.html?q=hello#section2")
    assert_equal "http", url.scheme
    assert_equal "alice:secret", url.userinfo
    assert_equal "www.example.com", url.host
    assert_equal 8080, url.port
    assert_equal "/docs/page.html", url.path
    assert_equal "q=hello", url.query
    assert_equal "section2", url.fragment
  end

  # ==========================================================================
  # Scheme variants
  # ==========================================================================

  def test_parse_https_url
    url = Url.parse("https://secure.example.com/login")
    assert_equal "https", url.scheme
    assert_equal "secure.example.com", url.host
    assert_equal 443, url.effective_port
  end

  def test_parse_ftp_url
    url = Url.parse("ftp://files.example.com/pub/readme.txt")
    assert_equal "ftp", url.scheme
    assert_equal 21, url.effective_port
  end

  # mailto: is a "scheme:path" URL with no authority (no "://").
  # The entire part after "mailto:" is the path.
  def test_parse_mailto_url
    url = Url.parse("mailto:alice@example.com")
    assert_equal "mailto", url.scheme
    assert_nil url.host
    assert_equal "alice@example.com", url.path
  end

  # ==========================================================================
  # Case normalization
  # ==========================================================================

  # Scheme and host are case-insensitive per the RFCs, so we lowercase them.
  # Path case is preserved because paths ARE case-sensitive on most servers.
  def test_scheme_is_lowercased
    url = Url.parse("HTTP://WWW.EXAMPLE.COM/PATH")
    assert_equal "http", url.scheme
    assert_equal "www.example.com", url.host
    # Path case is preserved
    assert_equal "/PATH", url.path
  end

  # ==========================================================================
  # Effective port
  # ==========================================================================

  # When no port is specified, effective_port returns the scheme default.
  def test_effective_port_http_default
    url = Url.parse("http://example.com")
    assert_nil url.port
    assert_equal 80, url.effective_port
  end

  # When an explicit port is given, it overrides the scheme default.
  def test_effective_port_explicit
    url = Url.parse("http://example.com:9090")
    assert_equal 9090, url.port
    assert_equal 9090, url.effective_port
  end

  # Schemes without a known default return nil for effective_port.
  def test_effective_port_unknown_scheme
    url = Url.parse("custom://host/path")
    assert_nil url.port
    assert_nil url.effective_port
  end

  # ==========================================================================
  # Authority
  # ==========================================================================

  # Full authority includes userinfo, host, and port.
  def test_authority_with_all_parts
    url = Url.parse("http://user:pass@host.com:8080/path")
    assert_equal "user:pass@host.com:8080", url.authority
  end

  # Minimal authority is just the host.
  def test_authority_host_only
    url = Url.parse("http://host.com/path")
    assert_equal "host.com", url.authority
  end

  # Authority with userinfo but no port.
  def test_authority_with_userinfo_no_port
    url = Url.parse("http://admin@host.com/path")
    assert_equal "admin@host.com", url.authority
  end

  # ==========================================================================
  # Invalid URLs
  # ==========================================================================

  # A bare hostname with no scheme triggers MissingScheme.
  def test_missing_scheme
    assert_raises(CodingAdventures::UrlParser::MissingScheme) do
      Url.parse("www.example.com")
    end
  end

  # Schemes must start with a letter; "1http" is invalid.
  def test_invalid_scheme_starts_with_digit
    assert_raises(CodingAdventures::UrlParser::InvalidScheme) do
      Url.parse("1http://x.com")
    end
  end

  # Port must be 0-65535; 99999 is out of range.
  def test_invalid_port_too_large
    assert_raises(CodingAdventures::UrlParser::InvalidPort) do
      Url.parse("http://host:99999")
    end
  end

  # Completely empty input has no scheme.
  def test_empty_input
    assert_raises(CodingAdventures::UrlParser::MissingScheme) do
      Url.parse("")
    end
  end

  # Scheme with special characters like spaces is invalid.
  def test_invalid_scheme_with_space
    assert_raises(CodingAdventures::UrlParser::InvalidScheme) do
      Url.parse("ht tp://example.com")
    end
  end

  # ==========================================================================
  # Percent-encoding
  # ==========================================================================

  # Spaces become %20 in percent-encoding.
  def test_encode_space
    assert_equal "hello%20world", UP.percent_encode("hello world")
  end

  # Unreserved characters (A-Z, a-z, 0-9, -, _, ., ~) pass through unchanged.
  def test_encode_preserves_unreserved
    assert_equal "abc-def_ghi.jkl~mno", UP.percent_encode("abc-def_ghi.jkl~mno")
  end

  # Forward slashes are also unreserved in our encoding (path-safe).
  def test_encode_preserves_slashes
    assert_equal "/path/to/file", UP.percent_encode("/path/to/file")
  end

  # Decoding %20 produces a space.
  def test_decode_space
    assert_equal "hello world", UP.percent_decode("hello%20world")
  end

  # Multi-byte UTF-8 decoding: 日 = U+65E5 = E6 97 A5 in UTF-8.
  def test_decode_utf8
    assert_equal "日", UP.percent_decode("%E6%97%A5")
  end

  # Roundtrip: encode then decode should produce the original string.
  # This tests both ASCII and multi-byte characters.
  def test_decode_roundtrip
    original = "hello world/日本語"
    encoded = UP.percent_encode(original)
    decoded = UP.percent_decode(encoded)
    assert_equal original, decoded
  end

  # Truncated percent-encoding ("%2" with only one hex digit) is invalid.
  def test_decode_malformed_truncated
    assert_raises(CodingAdventures::UrlParser::InvalidPercentEncoding) do
      UP.percent_decode("%2")
    end
  end

  # Non-hex digits after "%" are invalid.
  def test_decode_malformed_bad_hex
    assert_raises(CodingAdventures::UrlParser::InvalidPercentEncoding) do
      UP.percent_decode("%GG")
    end
  end

  # Encoding special characters like @ and #.
  def test_encode_special_characters
    assert_equal "user%40host%23frag", UP.percent_encode("user@host#frag")
  end

  # ==========================================================================
  # Relative resolution (RFC 1808)
  # ==========================================================================

  # Relative path in the same directory: "d.html" relative to "/a/b/c.html"
  # merges to "/a/b/d.html" (replaces the last segment).
  def test_resolve_same_directory
    base = Url.parse("http://host/a/b/c.html")
    resolved = base.resolve("d.html")
    assert_equal "http", resolved.scheme
    assert_equal "host", resolved.host
    assert_equal "/a/b/d.html", resolved.path
  end

  # ".." goes up one directory level.
  def test_resolve_parent_directory
    base = Url.parse("http://host/a/b/c.html")
    resolved = base.resolve("../d.html")
    assert_equal "/a/d.html", resolved.path
  end

  # "../.." goes up two directory levels.
  def test_resolve_grandparent_directory
    base = Url.parse("http://host/a/b/c.html")
    resolved = base.resolve("../../d.html")
    assert_equal "/d.html", resolved.path
  end

  # Absolute path replaces the base path entirely but keeps scheme + authority.
  def test_resolve_absolute_path
    base = Url.parse("http://host/a/b/c.html")
    resolved = base.resolve("/x/y.html")
    assert_equal "/x/y.html", resolved.path
    assert_equal "host", resolved.host
  end

  # Scheme-relative "//other.com/path" inherits only the scheme.
  def test_resolve_scheme_relative
    base = Url.parse("http://host/a/b")
    resolved = base.resolve("//other.com/path")
    assert_equal "http", resolved.scheme
    assert_equal "other.com", resolved.host
    assert_equal "/path", resolved.path
  end

  # Already-absolute URLs are returned as-is (parsed independently).
  def test_resolve_already_absolute
    base = Url.parse("http://host/a/b")
    resolved = base.resolve("https://other.com/x")
    assert_equal "https", resolved.scheme
    assert_equal "other.com", resolved.host
    assert_equal "/x", resolved.path
  end

  # "./d" is current directory + "d" -> replaces last segment.
  def test_resolve_dot_segments
    base = Url.parse("http://host/a/b/c")
    resolved = base.resolve("./d")
    assert_equal "/a/b/d", resolved.path
  end

  # Empty relative returns base without fragment.
  def test_resolve_empty_returns_base
    base = Url.parse("http://host/a/b?q=1#frag")
    resolved = base.resolve("")
    assert_equal "/a/b", resolved.path
    assert_equal "q=1", resolved.query
    assert_nil resolved.fragment  # fragment stripped
  end

  # Fragment-only relative changes just the fragment.
  def test_resolve_fragment_only
    base = Url.parse("http://host/a/b")
    resolved = base.resolve("#sec")
    assert_equal "/a/b", resolved.path
    assert_equal "sec", resolved.fragment
  end

  # Relative with query string.
  def test_resolve_with_query
    base = Url.parse("http://host/a/b")
    resolved = base.resolve("c?key=val")
    assert_equal "/a/c", resolved.path
    assert_equal "key=val", resolved.query
  end

  # ==========================================================================
  # Dot segment removal
  # ==========================================================================

  # Single "." is removed (current directory is a no-op).
  def test_remove_single_dot
    assert_equal "/a/b", UP.send(:remove_dot_segments, "/a/./b")
  end

  # ".." goes up one level: /a/b/../c -> /a/c
  def test_remove_double_dot
    assert_equal "/a/c", UP.send(:remove_dot_segments, "/a/b/../c")
  end

  # Multiple ".." segments: /a/b/../../c -> /c
  def test_remove_multiple_double_dots
    assert_equal "/c", UP.send(:remove_dot_segments, "/a/b/../../c")
  end

  # Can't go above root: /a/../../../c -> /c
  def test_double_dot_above_root
    assert_equal "/c", UP.send(:remove_dot_segments, "/a/../../../c")
  end

  # ==========================================================================
  # to_url_string / Display (roundtrip)
  # ==========================================================================

  # Full URL with all components should roundtrip through parse -> to_url_string.
  def test_roundtrip_full_url
    input = "http://user:pass@host.com:8080/path?q=1#frag"
    url = Url.parse(input)
    assert_equal input, url.to_url_string
  end

  # Simple URL roundtrip.
  def test_roundtrip_simple_url
    input = "http://example.com/path"
    url = Url.parse(input)
    assert_equal input, url.to_url_string
  end

  # to_s delegates to to_url_string.
  def test_to_s_matches_to_url_string
    url = Url.parse("http://example.com/path")
    assert_equal url.to_url_string, url.to_s
  end

  # mailto: roundtrip uses "scheme:path" form (no "://").
  def test_roundtrip_mailto
    input = "mailto:alice@example.com"
    url = Url.parse(input)
    assert_equal input, url.to_url_string
  end

  # ==========================================================================
  # Historical URLs (CERN, NCSA Mosaic era)
  # ==========================================================================

  # Tim Berners-Lee's original web page URL (1991).
  def test_parse_cern_original_url
    url = Url.parse("http://info.cern.ch/hypertext/WWW/TheProject.html")
    assert_equal "http", url.scheme
    assert_equal "info.cern.ch", url.host
    assert_equal "/hypertext/WWW/TheProject.html", url.path
    assert_equal 80, url.effective_port
  end

  # NCSA Mosaic download page (1993).
  def test_parse_ncsa_mosaic_url
    url = Url.parse("http://www.ncsa.uiuc.edu/SDG/Software/Mosaic/")
    assert_equal "www.ncsa.uiuc.edu", url.host
    assert_equal "/SDG/Software/Mosaic/", url.path
  end

  # ==========================================================================
  # IPv6
  # ==========================================================================

  # IPv6 localhost with port: the host includes the brackets.
  def test_parse_ipv6_localhost
    url = Url.parse("http://[::1]:8080/path")
    assert_equal "[::1]", url.host
    assert_equal 8080, url.port
    assert_equal "/path", url.path
  end

  # IPv6 without port.
  def test_parse_ipv6_without_port
    url = Url.parse("http://[::1]/path")
    assert_equal "[::1]", url.host
    assert_nil url.port
    assert_equal "/path", url.path
  end

  # ==========================================================================
  # Edge cases
  # ==========================================================================

  # Trailing slash produces path "/".
  def test_parse_trailing_slash
    url = Url.parse("http://host/")
    assert_equal "/", url.path
  end

  # Query without an explicit path: path defaults to "/".
  def test_parse_query_without_path
    url = Url.parse("http://host?q=1")
    assert_equal "host", url.host
    assert_equal "/", url.path
    assert_equal "q=1", url.query
  end

  # Fragment without an explicit path: path defaults to "/".
  def test_parse_fragment_without_path
    url = Url.parse("http://host#frag")
    assert_equal "host", url.host
    assert_equal "/", url.path
    assert_equal "frag", url.fragment
  end

  # Whitespace around the input is trimmed.
  def test_parse_strips_whitespace
    url = Url.parse("  http://example.com/path  ")
    assert_equal "http", url.scheme
    assert_equal "example.com", url.host
    assert_equal "/path", url.path
  end

  # URL with only scheme and authority, no trailing slash.
  def test_parse_no_path_defaults_to_slash
    url = Url.parse("http://example.com")
    assert_equal "/", url.path
  end

  # Port at the boundary: 0 is valid.
  def test_port_zero_is_valid
    url = Url.parse("http://host:0/path")
    assert_equal 0, url.port
  end

  # Port at the boundary: 65535 is valid.
  def test_port_65535_is_valid
    url = Url.parse("http://host:65535/path")
    assert_equal 65_535, url.port
  end

  # Scheme with valid special characters: svn+ssh
  def test_scheme_with_plus
    url = Url.parse("svn+ssh://host/repo")
    assert_equal "svn+ssh", url.scheme
    assert_equal "host", url.host
  end

  # Multiple query parameters.
  def test_parse_multiple_query_params
    url = Url.parse("http://host/path?a=1&b=2&c=3")
    assert_equal "a=1&b=2&c=3", url.query
  end

  # Fragment with special characters.
  def test_parse_fragment_with_special_chars
    url = Url.parse("http://host/path#sec/2?not-a-query")
    assert_equal "sec/2?not-a-query", url.fragment
  end
end
