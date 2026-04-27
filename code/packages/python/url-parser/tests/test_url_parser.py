"""Tests for url-parser — ported from the Rust implementation.

This test suite covers every aspect of URL parsing:
  - Basic parsing of HTTP, HTTPS, FTP, and mailto URLs
  - Case normalization (scheme and host lowercased)
  - Default and explicit port resolution
  - Authority reconstruction
  - Error handling (missing scheme, invalid scheme, invalid port)
  - Percent-encoding and decoding (including UTF-8 multi-byte)
  - Relative URL resolution (RFC 1808 / RFC 3986 §5)
  - Dot segment removal
  - Roundtrip serialization (parse → to_url_string → parse)
  - Historical URLs from the early web (CERN, NCSA Mosaic)
  - IPv6 address parsing
  - Edge cases (trailing slashes, query without path, fragment without path)
"""

from __future__ import annotations

import pytest

from url_parser import (
    InvalidPercentEncoding,
    InvalidPort,
    InvalidScheme,
    MissingScheme,
    Url,
    __version__,
    percent_decode,
    percent_encode,
)


# ──────────────────────────────────────────────────────────────────────
# Version
# ──────────────────────────────────────────────────────────────────────


class TestVersion:
    """Verify the package is importable and has a version."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


# ──────────────────────────────────────────────────────────────────────
# Basic Parsing
# ──────────────────────────────────────────────────────────────────────


class TestBasicParsing:
    """Test parsing of straightforward, well-formed URLs."""

    def test_parse_simple_http_url(self) -> None:
        """The simplest possible HTTP URL: just scheme + host."""
        url = Url.parse("http://example.com")
        assert url.scheme == "http"
        assert url.host == "example.com"
        assert url.path == "/"
        assert url.port is None
        assert url.query is None
        assert url.fragment is None
        assert url.userinfo is None

    def test_parse_http_with_path(self) -> None:
        """HTTP URL with a multi-segment path."""
        url = Url.parse("http://example.com/path/to/resource")
        assert url.scheme == "http"
        assert url.host == "example.com"
        assert url.path == "/path/to/resource"

    def test_parse_all_components(self) -> None:
        """A URL with every possible component populated."""
        url = Url.parse("http://user:pass@example.com:8080/path?query=1#frag")
        assert url.scheme == "http"
        assert url.userinfo == "user:pass"
        assert url.host == "example.com"
        assert url.port == 8080
        assert url.path == "/path"
        assert url.query == "query=1"
        assert url.fragment == "frag"

    def test_parse_https_url(self) -> None:
        """HTTPS — the encrypted variant of HTTP."""
        url = Url.parse("https://secure.example.com/login")
        assert url.scheme == "https"
        assert url.host == "secure.example.com"
        assert url.path == "/login"

    def test_parse_ftp_url(self) -> None:
        """FTP — the original file transfer protocol."""
        url = Url.parse("ftp://files.example.com/pub/readme.txt")
        assert url.scheme == "ftp"
        assert url.host == "files.example.com"
        assert url.path == "/pub/readme.txt"

    def test_parse_mailto_url(self) -> None:
        """mailto: is an opaque URI — no host, the path is the email address."""
        url = Url.parse("mailto:alice@example.com")
        assert url.scheme == "mailto"
        assert url.host is None
        assert url.path == "alice@example.com"


# ──────────────────────────────────────────────────────────────────────
# Case Normalization
# ──────────────────────────────────────────────────────────────────────


class TestCaseNormalization:
    """Scheme and host are case-insensitive per RFC 3986 §3.1 and §3.2.2."""

    def test_scheme_is_lowercased(self) -> None:
        """HTTP → http, HOST → host, but Path stays as-is."""
        url = Url.parse("HTTP://EXAMPLE.COM/Path")
        assert url.scheme == "http"
        assert url.host == "example.com"
        # Path is case-sensitive! /Path and /path are different resources.
        assert url.path == "/Path"


# ──────────────────────────────────────────────────────────────────────
# Port Handling
# ──────────────────────────────────────────────────────────────────────


class TestPortHandling:
    """Test default port lookup and explicit port parsing."""

    def test_effective_port_http_default(self) -> None:
        """HTTP's default port is 80 — the well-known port from 1991."""
        url = Url.parse("http://example.com")
        assert url.effective_port() == 80

    def test_effective_port_explicit(self) -> None:
        """An explicit port overrides the default."""
        url = Url.parse("http://example.com:9090")
        assert url.effective_port() == 9090
        assert url.port == 9090


# ──────────────────────────────────────────────────────────────────────
# Authority Reconstruction
# ──────────────────────────────────────────────────────────────────────


class TestAuthority:
    """Test the authority() method that reassembles userinfo@host:port."""

    def test_authority_with_all_parts(self) -> None:
        url = Url.parse("http://user:pass@example.com:8080/path")
        assert url.authority() == "user:pass@example.com:8080"

    def test_authority_host_only(self) -> None:
        url = Url.parse("http://example.com/path")
        assert url.authority() == "example.com"


# ──────────────────────────────────────────────────────────────────────
# Error Cases
# ──────────────────────────────────────────────────────────────────────


class TestErrors:
    """Verify that malformed URLs produce the correct error types."""

    def test_missing_scheme(self) -> None:
        """A URL with no scheme at all should fail."""
        with pytest.raises(MissingScheme):
            Url.parse("example.com/path")

    def test_invalid_scheme_starts_with_digit(self) -> None:
        """Schemes must start with a letter, not a digit."""
        with pytest.raises(InvalidScheme):
            Url.parse("1http://example.com")

    def test_invalid_port_too_large(self) -> None:
        """Port 99999 exceeds the 16-bit range (max 65535)."""
        with pytest.raises(InvalidPort):
            Url.parse("http://example.com:99999")


# ──────────────────────────────────────────────────────────────────────
# Percent-Encoding
# ──────────────────────────────────────────────────────────────────────


class TestPercentEncoding:
    """Test the percent_encode() and percent_decode() functions."""

    def test_encode_space(self) -> None:
        """Space (0x20) → %20 — the most common encoding."""
        assert percent_encode("hello world") == "hello%20world"

    def test_encode_preserves_unreserved(self) -> None:
        """Unreserved characters pass through unchanged."""
        assert percent_encode("hello") == "hello"

    def test_encode_preserves_slashes(self) -> None:
        """Slashes are in our unreserved set for path compatibility."""
        assert percent_encode("a/b") == "a/b"

    def test_decode_space(self) -> None:
        """%20 → space."""
        assert percent_decode("hello%20world") == "hello world"

    def test_decode_utf8(self) -> None:
        """Multi-byte UTF-8: %E6%97%A5 → 日 (U+65E5, 'sun/day')."""
        assert percent_decode("%E6%97%A5") == "日"

    def test_decode_roundtrip(self) -> None:
        """Encoding then decoding should return the original string."""
        original = "hello world/path"
        assert percent_decode(percent_encode(original)) == original

    def test_decode_malformed_truncated(self) -> None:
        """A '%' with only one hex digit after it is malformed."""
        with pytest.raises(InvalidPercentEncoding):
            percent_decode("%2")

    def test_decode_malformed_bad_hex(self) -> None:
        """'%GG' is not valid hex."""
        with pytest.raises(InvalidPercentEncoding):
            percent_decode("%GG")


# ──────────────────────────────────────────────────────────────────────
# Relative Resolution
# ──────────────────────────────────────────────────────────────────────


class TestRelativeResolution:
    """Test RFC 1808 / RFC 3986 §5 relative URL resolution."""

    BASE = "http://example.com/a/b/c"

    @pytest.fixture()
    def base(self) -> Url:
        return Url.parse(self.BASE)

    def test_resolve_same_directory(self, base: Url) -> None:
        """'d' relative to /a/b/c → /a/b/d (same directory)."""
        resolved = base.resolve("d")
        assert resolved.path == "/a/b/d"

    def test_resolve_parent_directory(self, base: Url) -> None:
        """'../d' → go up one level → /a/d."""
        resolved = base.resolve("../d")
        assert resolved.path == "/a/d"

    def test_resolve_grandparent_directory(self, base: Url) -> None:
        """'../../d' → go up two levels → /d."""
        resolved = base.resolve("../../d")
        assert resolved.path == "/d"

    def test_resolve_absolute_path(self, base: Url) -> None:
        """'/d' is an absolute path — replaces the entire path."""
        resolved = base.resolve("/d")
        assert resolved.path == "/d"
        assert resolved.host == "example.com"

    def test_resolve_scheme_relative(self, base: Url) -> None:
        """'//other.com/d' inherits only the scheme."""
        resolved = base.resolve("//other.com/d")
        assert resolved.scheme == "http"
        assert resolved.host == "other.com"
        assert resolved.path == "/d"

    def test_resolve_already_absolute(self, base: Url) -> None:
        """A reference with its own scheme is already absolute."""
        resolved = base.resolve("https://new.com/page")
        assert resolved.scheme == "https"
        assert resolved.host == "new.com"
        assert resolved.path == "/page"

    def test_resolve_dot_segments(self, base: Url) -> None:
        """'./d' (explicit current directory) → /a/b/d."""
        resolved = base.resolve("./d")
        assert resolved.path == "/a/b/d"

    def test_resolve_empty_returns_base(self, base: Url) -> None:
        """Empty reference returns the base URL without its fragment."""
        base_with_frag = Url.parse("http://example.com/a/b/c#section")
        resolved = base_with_frag.resolve("")
        assert resolved.path == "/a/b/c"
        assert resolved.fragment is None

    def test_resolve_fragment_only(self, base: Url) -> None:
        """'#frag' updates only the fragment."""
        resolved = base.resolve("#frag")
        assert resolved.path == "/a/b/c"
        assert resolved.fragment == "frag"
        assert resolved.host == "example.com"

    def test_resolve_with_query(self, base: Url) -> None:
        """A relative reference can include a query string."""
        resolved = base.resolve("d?key=val")
        assert resolved.path == "/a/b/d"
        assert resolved.query == "key=val"


# ──────────────────────────────────────────────────────────────────────
# Dot Segment Removal
# ──────────────────────────────────────────────────────────────────────


class TestDotSegmentRemoval:
    """Test the internal _remove_dot_segments function via resolve()."""

    def test_remove_single_dot(self) -> None:
        """Single dot (current dir) is removed: /a/./b → /a/b."""
        base = Url.parse("http://example.com/")
        resolved = base.resolve("/a/./b")
        assert resolved.path == "/a/b"

    def test_remove_double_dot(self) -> None:
        """Double dot (parent dir): /a/b/../c → /a/c."""
        base = Url.parse("http://example.com/")
        resolved = base.resolve("/a/b/../c")
        assert resolved.path == "/a/c"

    def test_remove_multiple_double_dots(self) -> None:
        """Multiple parent traversals: /a/b/c/../../d → /a/d."""
        base = Url.parse("http://example.com/")
        resolved = base.resolve("/a/b/c/../../d")
        assert resolved.path == "/a/d"

    def test_double_dot_above_root(self) -> None:
        """Can't go above root: /a/../../../c → /c."""
        base = Url.parse("http://example.com/")
        resolved = base.resolve("/a/../../../c")
        assert resolved.path == "/c"


# ──────────────────────────────────────────────────────────────────────
# Roundtrip Serialization
# ──────────────────────────────────────────────────────────────────────


class TestRoundtrip:
    """Verify that parse → to_url_string produces a valid URL."""

    def test_roundtrip_full_url(self) -> None:
        """A URL with all components should survive a roundtrip."""
        original = "http://user:pass@example.com:8080/path?query=1#frag"
        url = Url.parse(original)
        assert url.to_url_string() == original

    def test_roundtrip_simple_url(self) -> None:
        """A minimal URL should also roundtrip."""
        original = "http://example.com/"
        url = Url.parse(original)
        assert url.to_url_string() == original


# ──────────────────────────────────────────────────────────────────────
# Historical URLs
# ──────────────────────────────────────────────────────────────────────


class TestHistoricalUrls:
    """URLs from the early days of the web — parsing these correctly
    means our implementation handles real-world formats."""

    def test_parse_cern_original_url(self) -> None:
        """The first web server at CERN, 1991."""
        url = Url.parse("http://info.cern.ch/hypertext/WWW/TheProject.html")
        assert url.scheme == "http"
        assert url.host == "info.cern.ch"
        assert url.path == "/hypertext/WWW/TheProject.html"

    def test_parse_ncsa_mosaic_url(self) -> None:
        """NCSA Mosaic — the browser that popularized the web, 1993."""
        url = Url.parse("http://www.ncsa.uiuc.edu/SDG/Software/Mosaic/")
        assert url.scheme == "http"
        assert url.host == "www.ncsa.uiuc.edu"
        assert url.path == "/SDG/Software/Mosaic/"


# ──────────────────────────────────────────────────────────────────────
# IPv6
# ──────────────────────────────────────────────────────────────────────


class TestIPv6:
    """IPv6 addresses in URLs are wrapped in brackets: [::1]:8080."""

    def test_parse_ipv6_localhost(self) -> None:
        """[::1] is the IPv6 loopback address (equivalent to 127.0.0.1)."""
        url = Url.parse("http://[::1]:8080/path")
        assert url.host == "[::1]"
        assert url.port == 8080
        assert url.path == "/path"


# ──────────────────────────────────────────────────────────────────────
# Edge Cases
# ──────────────────────────────────────────────────────────────────────


class TestEdgeCases:
    """Unusual but valid URL forms that parsers commonly mishandle."""

    def test_parse_trailing_slash(self) -> None:
        """A trailing slash is significant — /path/ and /path are different."""
        url = Url.parse("http://example.com/path/")
        assert url.path == "/path/"

    def test_parse_query_without_path(self) -> None:
        """A URL can have a query but no explicit path (defaults to '/')."""
        url = Url.parse("http://example.com?query=1")
        assert url.path == "/"
        assert url.query == "query=1"

    def test_parse_fragment_without_path(self) -> None:
        """A URL can have a fragment but no explicit path."""
        url = Url.parse("http://example.com#section")
        assert url.path == "/"
        assert url.fragment == "section"
