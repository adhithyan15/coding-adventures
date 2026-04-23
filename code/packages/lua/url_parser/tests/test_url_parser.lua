-- Tests for url-parser
--
-- This test suite covers every aspect of the URL parser: basic parsing,
-- component extraction, case normalization, port handling, authority
-- reconstruction, invalid inputs, percent-encoding/decoding, relative
-- resolution, dot segment removal, roundtrip fidelity, historical URLs,
-- IPv6, and edge cases.
--
-- We use the busted test framework with describe/it blocks.
-- Each test group focuses on one aspect of the parser, making it easy
-- to locate failures and understand what each test verifies.

local m = require("coding_adventures.url_parser")

-- ============================================================================
-- Version
-- ============================================================================

describe("url-parser", function()
    it("has a VERSION", function()
        assert.is_not_nil(m.VERSION)
        assert.equals("0.1.0", m.VERSION)
    end)

    -- ========================================================================
    -- Basic Parsing
    -- ========================================================================
    -- These tests verify that common URLs are parsed into the expected
    -- components. They cover the most frequent URL patterns.

    describe("basic parsing", function()
        it("parses a simple HTTP URL", function()
            local url, err = m.parse("http://example.com")
            assert.is_nil(err)
            assert.equals("http", url.scheme)
            assert.equals("example.com", url.host)
            assert.equals("/", url.path)
            assert.is_nil(url.query)
            assert.is_nil(url.fragment)
            assert.is_nil(url.userinfo)
            assert.is_nil(url.port)
        end)

        it("parses a URL with all components", function()
            local url, err = m.parse("https://user:pass@example.com:8080/path/to/page?q=1&lang=en#section2")
            assert.is_nil(err)
            assert.equals("https", url.scheme)
            assert.equals("user:pass", url.userinfo)
            assert.equals("example.com", url.host)
            assert.equals(8080, url.port)
            assert.equals("/path/to/page", url.path)
            assert.equals("q=1&lang=en", url.query)
            assert.equals("section2", url.fragment)
        end)

        it("parses a URL with path only", function()
            local url, err = m.parse("http://example.com/path/to/resource")
            assert.is_nil(err)
            assert.equals("http", url.scheme)
            assert.equals("example.com", url.host)
            assert.equals("/path/to/resource", url.path)
        end)

        it("parses a URL with query only", function()
            local url, err = m.parse("http://example.com?search=hello")
            assert.is_nil(err)
            assert.equals("http", url.scheme)
            assert.equals("example.com", url.host)
            assert.equals("search=hello", url.query)
        end)

        it("parses a URL with fragment only", function()
            local url, err = m.parse("http://example.com#top")
            assert.is_nil(err)
            assert.equals("http", url.scheme)
            assert.equals("example.com", url.host)
            assert.equals("top", url.fragment)
        end)

        it("preserves the raw input", function()
            local raw = "HTTP://Example.COM/Path"
            local url, err = m.parse(raw)
            assert.is_nil(err)
            assert.equals(raw, url.raw)
        end)
    end)

    -- ========================================================================
    -- Component Extraction
    -- ========================================================================

    describe("component extraction", function()
        it("extracts userinfo with special characters", function()
            local url, err = m.parse("http://admin%40host:p%40ss@example.com/")
            assert.is_nil(err)
            assert.equals("admin%40host:p%40ss", url.userinfo)
            assert.equals("example.com", url.host)
        end)

        it("handles URL with empty path after host", function()
            local url, err = m.parse("http://example.com")
            assert.is_nil(err)
            assert.equals("/", url.path)
        end)

        it("handles multiple query parameters", function()
            local url, err = m.parse("http://example.com/search?q=test&page=2&sort=asc")
            assert.is_nil(err)
            assert.equals("q=test&page=2&sort=asc", url.query)
        end)

        it("handles fragment with special characters", function()
            local url, err = m.parse("http://example.com/page#section/2/part-a")
            assert.is_nil(err)
            assert.equals("section/2/part-a", url.fragment)
        end)

        it("handles query and fragment together", function()
            local url, err = m.parse("http://example.com/path?q=1#frag")
            assert.is_nil(err)
            assert.equals("q=1", url.query)
            assert.equals("frag", url.fragment)
        end)
    end)

    -- ========================================================================
    -- Case Normalization
    -- ========================================================================
    -- Schemes and hosts are case-insensitive per the RFCs, so we normalize
    -- them to lowercase. Paths, queries, and fragments are case-sensitive.

    describe("case normalization", function()
        it("lowercases the scheme", function()
            local url, err = m.parse("HTTP://example.com")
            assert.is_nil(err)
            assert.equals("http", url.scheme)
        end)

        it("lowercases the host", function()
            local url, err = m.parse("http://EXAMPLE.COM")
            assert.is_nil(err)
            assert.equals("example.com", url.host)
        end)

        it("preserves path case", function()
            local url, err = m.parse("http://example.com/Path/To/Page")
            assert.is_nil(err)
            assert.equals("/Path/To/Page", url.path)
        end)

        it("preserves query case", function()
            local url, err = m.parse("http://example.com?Key=Value")
            assert.is_nil(err)
            assert.equals("Key=Value", url.query)
        end)

        it("preserves fragment case", function()
            local url, err = m.parse("http://example.com#Section")
            assert.is_nil(err)
            assert.equals("Section", url.fragment)
        end)
    end)

    -- ========================================================================
    -- Port Handling
    -- ========================================================================

    describe("port handling", function()
        it("parses explicit port", function()
            local url, err = m.parse("http://example.com:9090/path")
            assert.is_nil(err)
            assert.equals(9090, url.port)
        end)

        it("effective_port returns explicit port when present", function()
            local url = m.parse("http://example.com:9090")
            assert.equals(9090, m.effective_port(url))
        end)

        it("effective_port returns default port for http", function()
            local url = m.parse("http://example.com")
            assert.equals(80, m.effective_port(url))
        end)

        it("effective_port returns default port for https", function()
            local url = m.parse("https://example.com")
            assert.equals(443, m.effective_port(url))
        end)

        it("effective_port returns default port for ftp", function()
            local url = m.parse("ftp://files.example.com")
            assert.equals(21, m.effective_port(url))
        end)

        it("effective_port returns nil for unknown scheme", function()
            local url = m.parse("custom://example.com")
            assert.is_nil(m.effective_port(url))
        end)
    end)

    -- ========================================================================
    -- Authority Reconstruction
    -- ========================================================================

    describe("authority", function()
        it("reconstructs simple authority", function()
            local url = m.parse("http://example.com")
            assert.equals("example.com", m.authority(url))
        end)

        it("reconstructs authority with port", function()
            local url = m.parse("http://example.com:8080")
            assert.equals("example.com:8080", m.authority(url))
        end)

        it("reconstructs authority with userinfo", function()
            local url = m.parse("http://user@example.com")
            assert.equals("user@example.com", m.authority(url))
        end)

        it("reconstructs full authority", function()
            local url = m.parse("http://user:pass@example.com:8080")
            assert.equals("user:pass@example.com:8080", m.authority(url))
        end)

        it("returns empty string for no-authority URL", function()
            local url = m.parse("mailto:user@example.com")
            assert.equals("", m.authority(url))
        end)
    end)

    -- ========================================================================
    -- Invalid Inputs
    -- ========================================================================

    describe("invalid inputs", function()
        it("rejects empty string", function()
            local url, err = m.parse("")
            assert.is_nil(url)
            assert.equals("missing_scheme", err)
        end)

        it("rejects nil input", function()
            local url, err = m.parse(nil)
            assert.is_nil(url)
            assert.equals("missing_scheme", err)
        end)

        it("rejects string without scheme", function()
            local url, err = m.parse("://example.com")
            assert.is_nil(url)
            -- No valid scheme before ://
        end)

        it("rejects scheme starting with digit", function()
            local url, err = m.parse("1http://example.com")
            assert.is_nil(url)
        end)
    end)

    -- ========================================================================
    -- Percent-Encoding
    -- ========================================================================
    -- Percent-encoding converts unsafe characters to %XX hex sequences.
    -- The unreserved set (A-Za-z0-9-_.~/) is NOT encoded.

    describe("percent_encode", function()
        it("does not encode unreserved characters", function()
            assert.equals("abcABC012", m.percent_encode("abcABC012"))
        end)

        it("does not encode hyphens, underscores, dots, tildes", function()
            assert.equals("-_.~", m.percent_encode("-_.~"))
        end)

        it("does not encode slashes", function()
            assert.equals("/path/to/file", m.percent_encode("/path/to/file"))
        end)

        it("encodes spaces", function()
            assert.equals("hello%20world", m.percent_encode("hello world"))
        end)

        it("encodes special characters", function()
            assert.equals("%40%23%24%25", m.percent_encode("@#$%"))
        end)

        it("encodes non-ASCII bytes", function()
            -- The euro sign in UTF-8 is 0xE2 0x82 0xAC
            local encoded = m.percent_encode("\xC3\xA9")  -- e with acute accent (UTF-8)
            assert.equals("%C3%A9", encoded)
        end)

        it("handles empty string", function()
            assert.equals("", m.percent_encode(""))
        end)
    end)

    -- ========================================================================
    -- Percent-Decoding
    -- ========================================================================

    describe("percent_decode", function()
        it("decodes %20 to space", function()
            local result, err = m.percent_decode("hello%20world")
            assert.is_nil(err)
            assert.equals("hello world", result)
        end)

        it("decodes uppercase hex", function()
            local result, err = m.percent_decode("%41%42%43")
            assert.is_nil(err)
            assert.equals("ABC", result)
        end)

        it("decodes lowercase hex", function()
            local result, err = m.percent_decode("%61%62%63")
            assert.is_nil(err)
            assert.equals("abc", result)
        end)

        it("passes through unreserved characters", function()
            local result, err = m.percent_decode("hello")
            assert.is_nil(err)
            assert.equals("hello", result)
        end)

        it("rejects truncated percent encoding", function()
            local result, err = m.percent_decode("hello%2")
            assert.is_nil(result)
            assert.equals("invalid_percent_encoding", err)
        end)

        it("rejects invalid hex digits", function()
            local result, err = m.percent_decode("hello%GG")
            assert.is_nil(result)
            assert.equals("invalid_percent_encoding", err)
        end)

        it("handles empty string", function()
            local result, err = m.percent_decode("")
            assert.is_nil(err)
            assert.equals("", result)
        end)

        it("decodes mixed encoded and plain text", function()
            local result, err = m.percent_decode("path%2Fto%2Ffile")
            assert.is_nil(err)
            assert.equals("path/to/file", result)
        end)
    end)

    -- ========================================================================
    -- Relative URL Resolution
    -- ========================================================================
    -- Relative URLs are resolved against a base URL. This is how browsers
    -- interpret relative links in HTML pages.

    describe("resolve", function()
        local base = "http://a.com/b/c/d?q=1#f"

        it("resolves empty reference (returns base without fragment)", function()
            local url, err = m.resolve(base, "")
            assert.is_nil(err)
            assert.equals("http", url.scheme)
            assert.equals("a.com", url.host)
            assert.equals("/b/c/d", url.path)
            assert.equals("q=1", url.query)
            assert.is_nil(url.fragment)
        end)

        it("resolves fragment-only reference", function()
            local url, err = m.resolve(base, "#new")
            assert.is_nil(err)
            assert.equals("http", url.scheme)
            assert.equals("a.com", url.host)
            assert.equals("/b/c/d", url.path)
            assert.equals("q=1", url.query)
            assert.equals("new", url.fragment)
        end)

        it("resolves absolute URL (has scheme)", function()
            local url, err = m.resolve(base, "https://other.com/page")
            assert.is_nil(err)
            assert.equals("https", url.scheme)
            assert.equals("other.com", url.host)
            assert.equals("/page", url.path)
        end)

        it("resolves scheme-relative reference", function()
            local url, err = m.resolve(base, "//other.com/page")
            assert.is_nil(err)
            assert.equals("http", url.scheme)
            assert.equals("other.com", url.host)
            assert.equals("/page", url.path)
        end)

        it("resolves absolute path reference", function()
            local url, err = m.resolve(base, "/new/path")
            assert.is_nil(err)
            assert.equals("http", url.scheme)
            assert.equals("a.com", url.host)
            assert.equals("/new/path", url.path)
            assert.is_nil(url.query)
        end)

        it("resolves relative path reference", function()
            local url, err = m.resolve(base, "g")
            assert.is_nil(err)
            assert.equals("http", url.scheme)
            assert.equals("a.com", url.host)
            assert.equals("/b/c/g", url.path)
        end)

        it("resolves relative path with query", function()
            local url, err = m.resolve(base, "g?y=2")
            assert.is_nil(err)
            assert.equals("/b/c/g", url.path)
            assert.equals("y=2", url.query)
        end)

        it("resolves query-only reference", function()
            local url, err = m.resolve(base, "?newquery")
            assert.is_nil(err)
            assert.equals("/b/c/d", url.path)
            assert.equals("newquery", url.query)
        end)

        it("resolves relative path with fragment", function()
            local url, err = m.resolve(base, "g#s")
            assert.is_nil(err)
            assert.equals("/b/c/g", url.path)
            assert.equals("s", url.fragment)
        end)
    end)

    -- ========================================================================
    -- Dot Segment Removal
    -- ========================================================================
    -- Dot segments (. and ..) in paths must be resolved. These tests verify
    -- that our remove_dot_segments function works correctly through resolve.

    describe("dot segments", function()
        local base = "http://a.com/b/c/d"

        it("resolves ./g to /b/c/g", function()
            local url = m.resolve(base, "./g")
            assert.equals("/b/c/g", url.path)
        end)

        it("resolves ../g to /b/g", function()
            local url = m.resolve(base, "../g")
            assert.equals("/b/g", url.path)
        end)

        it("resolves ../../g to /g", function()
            local url = m.resolve(base, "../../g")
            assert.equals("/g", url.path)
        end)

        it("resolves ../../../g (above root) to /g", function()
            local url = m.resolve(base, "../../../g")
            assert.equals("/g", url.path)
        end)

        it("resolves /a/b/c/./../../g to /a/g", function()
            local url = m.resolve(base, "/a/b/c/./../../g")
            assert.equals("/a/g", url.path)
        end)

        it("resolves . to /b/c/", function()
            local url = m.resolve(base, ".")
            assert.equals("/b/c/", url.path)
        end)

        it("resolves .. to /b/", function()
            local url = m.resolve(base, "..")
            assert.equals("/b/", url.path)
        end)
    end)

    -- ========================================================================
    -- Roundtrip (parse -> to_url_string)
    -- ========================================================================
    -- A good parser should be able to reconstruct a URL string from its
    -- parsed components that is functionally equivalent to the original.

    describe("roundtrip", function()
        it("roundtrips a simple URL", function()
            local url = m.parse("http://example.com/path")
            assert.equals("http://example.com/path", m.to_url_string(url))
        end)

        it("roundtrips a URL with all components", function()
            local url = m.parse("https://user:pass@example.com:8080/path?q=1#frag")
            assert.equals("https://user:pass@example.com:8080/path?q=1#frag", m.to_url_string(url))
        end)

        it("roundtrips a URL with query only", function()
            local url = m.parse("http://example.com?key=value")
            assert.equals("http://example.com/?key=value", m.to_url_string(url))
        end)

        it("roundtrips a mailto URL", function()
            local url = m.parse("mailto:user@example.com")
            assert.equals("mailto:user@example.com", m.to_url_string(url))
        end)

        it("roundtrips ftp URL", function()
            local url = m.parse("ftp://files.example.com/pub/readme.txt")
            assert.equals("ftp://files.example.com/pub/readme.txt", m.to_url_string(url))
        end)
    end)

    -- ========================================================================
    -- Historical / Scheme-Path URLs
    -- ========================================================================
    -- Some URL schemes don't use authority (no "//"). For example:
    --   mailto:user@example.com
    --   tel:+1-555-0100
    --   urn:isbn:0451450523

    describe("historical/scheme-path URLs", function()
        it("parses mailto URL", function()
            local url, err = m.parse("mailto:user@example.com")
            assert.is_nil(err)
            assert.equals("mailto", url.scheme)
            assert.equals("user@example.com", url.path)
            assert.is_nil(url.host)
        end)

        it("parses tel URL", function()
            local url, err = m.parse("tel:+1-555-0100")
            assert.is_nil(err)
            assert.equals("tel", url.scheme)
            assert.equals("+1-555-0100", url.path)
        end)

        it("parses urn URL", function()
            local url, err = m.parse("urn:isbn:0451450523")
            assert.is_nil(err)
            assert.equals("urn", url.scheme)
            assert.equals("isbn:0451450523", url.path)
        end)

        it("parses data URL", function()
            local url, err = m.parse("data:text/plain;base64,SGVsbG8=")
            assert.is_nil(err)
            assert.equals("data", url.scheme)
            assert.equals("text/plain;base64,SGVsbG8=", url.path)
        end)

        it("parses mailto with query (subject)", function()
            local url, err = m.parse("mailto:user@example.com?subject=Hello")
            assert.is_nil(err)
            assert.equals("mailto", url.scheme)
            assert.equals("user@example.com", url.path)
            assert.equals("subject=Hello", url.query)
        end)
    end)

    -- ========================================================================
    -- IPv6
    -- ========================================================================
    -- IPv6 addresses in URLs are enclosed in square brackets: [::1]

    describe("IPv6", function()
        it("parses IPv6 localhost", function()
            local url, err = m.parse("http://[::1]/path")
            assert.is_nil(err)
            assert.equals("[::1]", url.host)
            assert.equals("/path", url.path)
        end)

        it("parses IPv6 with port", function()
            local url, err = m.parse("http://[::1]:8080/path")
            assert.is_nil(err)
            assert.equals("[::1]", url.host)
            assert.equals(8080, url.port)
            assert.equals("/path", url.path)
        end)

        it("parses full IPv6 address", function()
            local url, err = m.parse("http://[2001:db8::1]/")
            assert.is_nil(err)
            assert.equals("[2001:db8::1]", url.host)
        end)

        it("parses IPv6 with userinfo", function()
            local url, err = m.parse("http://user@[::1]:3000/")
            assert.is_nil(err)
            assert.equals("user", url.userinfo)
            assert.equals("[::1]", url.host)
            assert.equals(3000, url.port)
        end)
    end)

    -- ========================================================================
    -- Edge Cases
    -- ========================================================================

    describe("edge cases", function()
        it("handles URL with empty query", function()
            local url, err = m.parse("http://example.com/path?")
            assert.is_nil(err)
            assert.equals("", url.query)
        end)

        it("handles URL with empty fragment", function()
            local url, err = m.parse("http://example.com/path#")
            assert.is_nil(err)
            assert.equals("", url.fragment)
        end)

        it("handles URL with port but no path", function()
            local url, err = m.parse("http://example.com:3000")
            assert.is_nil(err)
            assert.equals("example.com", url.host)
            assert.equals(3000, url.port)
            assert.equals("/", url.path)
        end)

        it("handles deeply nested path", function()
            local url, err = m.parse("http://example.com/a/b/c/d/e/f/g")
            assert.is_nil(err)
            assert.equals("/a/b/c/d/e/f/g", url.path)
        end)

        it("handles scheme with plus and dot", function()
            local url, err = m.parse("svn+ssh://example.com/repo")
            assert.is_nil(err)
            assert.equals("svn+ssh", url.scheme)
        end)

        it("handles scheme with hyphen", function()
            local url, err = m.parse("coap-tcp://example.com/path")
            assert.is_nil(err)
            assert.equals("coap-tcp", url.scheme)
        end)

        it("handles URL with only scheme and host", function()
            local url, err = m.parse("http://localhost")
            assert.is_nil(err)
            assert.equals("localhost", url.host)
            assert.equals("/", url.path)
        end)

        it("handles URL with trailing slash", function()
            local url, err = m.parse("http://example.com/")
            assert.is_nil(err)
            assert.equals("/", url.path)
        end)

        it("handles URL with numeric host (IPv4)", function()
            local url, err = m.parse("http://127.0.0.1:8080/path")
            assert.is_nil(err)
            assert.equals("127.0.0.1", url.host)
            assert.equals(8080, url.port)
        end)

        it("handles URL with query containing hash-like characters", function()
            -- The first # always starts the fragment
            local url, err = m.parse("http://example.com/path?q=a%23b#frag")
            assert.is_nil(err)
            assert.equals("q=a%23b", url.query)
            assert.equals("frag", url.fragment)
        end)

        it("handles to_url_string with no host (scheme:path)", function()
            local url = m.parse("mailto:test@example.com")
            local s = m.to_url_string(url)
            assert.equals("mailto:test@example.com", s)
        end)

        it("percent_encode then percent_decode roundtrips", function()
            local original = "hello world@test#value"
            local encoded = m.percent_encode(original)
            local decoded, err = m.percent_decode(encoded)
            assert.is_nil(err)
            assert.equals(original, decoded)
        end)
    end)
end)
