import XCTest
@testable import UrlParser

// ============================================================================
// UrlParserTests — comprehensive unit tests for the UrlParser module
// ============================================================================
//
// These tests mirror the Rust url-parser test suite, covering:
//
// 1. Basic parsing (simple URLs, all components)
// 2. Case normalization (scheme and host lowercased)
// 3. Effective port (defaults and explicit)
// 4. Authority reconstruction
// 5. Invalid URLs (missing scheme, bad scheme, bad port)
// 6. Percent-encoding and decoding
// 7. Relative resolution (same dir, parent, absolute, scheme-relative, etc.)
// 8. Dot segment removal
// 9. Round-trip serialization
// 10. Historical URLs and edge cases
//
// Total: 44+ tests

final class UrlParserTests: XCTestCase {

    // ═══════════════════════════════════════════════════════════════════════
    // Basic parsing
    // ═══════════════════════════════════════════════════════════════════════

    /// The simplest possible HTTP URL — just scheme + host.
    func testParseSimpleHttpUrl() throws {
        let url = try Url.parse("http://www.example.com")
        XCTAssertEqual(url.scheme, "http")
        XCTAssertEqual(url.host, "www.example.com")
        XCTAssertNil(url.port)
        XCTAssertEqual(url.path, "/")
        XCTAssertNil(url.query)
        XCTAssertNil(url.fragment)
    }

    /// HTTP URL with a path component.
    func testParseHttpWithPath() throws {
        let url = try Url.parse("http://www.example.com/docs/page.html")
        XCTAssertEqual(url.scheme, "http")
        XCTAssertEqual(url.host, "www.example.com")
        XCTAssertEqual(url.path, "/docs/page.html")
    }

    /// URL with every component: scheme, userinfo, host, port, path, query, fragment.
    func testParseAllComponents() throws {
        let url = try Url.parse(
            "http://alice:secret@www.example.com:8080/docs/page.html?q=hello#section2"
        )
        XCTAssertEqual(url.scheme, "http")
        XCTAssertEqual(url.userinfo, "alice:secret")
        XCTAssertEqual(url.host, "www.example.com")
        XCTAssertEqual(url.port, 8080)
        XCTAssertEqual(url.path, "/docs/page.html")
        XCTAssertEqual(url.query, "q=hello")
        XCTAssertEqual(url.fragment, "section2")
    }

    /// HTTPS URL — scheme should be "https", default port 443.
    func testParseHttpsUrl() throws {
        let url = try Url.parse("https://secure.example.com/login")
        XCTAssertEqual(url.scheme, "https")
        XCTAssertEqual(url.host, "secure.example.com")
        XCTAssertEqual(url.effectivePort(), 443)
    }

    /// FTP URL — scheme should be "ftp", default port 21.
    func testParseFtpUrl() throws {
        let url = try Url.parse("ftp://files.example.com/pub/readme.txt")
        XCTAssertEqual(url.scheme, "ftp")
        XCTAssertEqual(url.effectivePort(), 21)
    }

    /// Mailto URL — no authority, path is the email address.
    func testParseMailtoUrl() throws {
        let url = try Url.parse("mailto:alice@example.com")
        XCTAssertEqual(url.scheme, "mailto")
        XCTAssertNil(url.host)
        XCTAssertEqual(url.path, "alice@example.com")
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Case normalization
    // ═══════════════════════════════════════════════════════════════════════

    /// Scheme and host are lowercased; path case is preserved.
    func testSchemeIsLowercased() throws {
        let url = try Url.parse("HTTP://WWW.EXAMPLE.COM/PATH")
        XCTAssertEqual(url.scheme, "http")
        XCTAssertEqual(url.host, "www.example.com")
        // Path case must be preserved — paths are case-sensitive
        XCTAssertEqual(url.path, "/PATH")
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Effective port
    // ═══════════════════════════════════════════════════════════════════════

    /// HTTP without explicit port → effective port is 80.
    func testEffectivePortHttpDefault() throws {
        let url = try Url.parse("http://example.com")
        XCTAssertNil(url.port)
        XCTAssertEqual(url.effectivePort(), 80)
    }

    /// HTTP with explicit port → effective port is the explicit one.
    func testEffectivePortExplicit() throws {
        let url = try Url.parse("http://example.com:9090")
        XCTAssertEqual(url.port, 9090)
        XCTAssertEqual(url.effectivePort(), 9090)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Authority
    // ═══════════════════════════════════════════════════════════════════════

    /// Authority with userinfo, host, and port.
    func testAuthorityWithAllParts() throws {
        let url = try Url.parse("http://user:pass@host.com:8080/path")
        XCTAssertEqual(url.authority(), "user:pass@host.com:8080")
    }

    /// Authority with host only.
    func testAuthorityHostOnly() throws {
        let url = try Url.parse("http://host.com/path")
        XCTAssertEqual(url.authority(), "host.com")
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Invalid URLs
    // ═══════════════════════════════════════════════════════════════════════

    /// No scheme at all → missingScheme.
    func testMissingScheme() {
        XCTAssertThrowsError(try Url.parse("www.example.com")) { error in
            XCTAssertEqual(error as? UrlError, UrlError.missingScheme)
        }
    }

    /// Scheme starting with a digit → invalidScheme.
    func testInvalidSchemeStartsWithDigit() {
        XCTAssertThrowsError(try Url.parse("1http://x.com")) { error in
            XCTAssertEqual(error as? UrlError, UrlError.invalidScheme)
        }
    }

    /// Port larger than UInt16 max (65535) → invalidPort.
    func testInvalidPortTooLarge() {
        XCTAssertThrowsError(try Url.parse("http://host:99999")) { error in
            XCTAssertEqual(error as? UrlError, UrlError.invalidPort)
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Percent-encoding
    // ═══════════════════════════════════════════════════════════════════════

    /// Space (0x20) is encoded as %20.
    func testEncodeSpace() {
        XCTAssertEqual(percentEncode("hello world"), "hello%20world")
    }

    /// Unreserved characters pass through unchanged.
    func testEncodePreservesUnreserved() {
        XCTAssertEqual(percentEncode("abc-def_ghi.jkl~mno"), "abc-def_ghi.jkl~mno")
    }

    /// Forward slashes are unreserved in our encoding.
    func testEncodePreservesSlashes() {
        XCTAssertEqual(percentEncode("/path/to/file"), "/path/to/file")
    }

    /// %20 decodes back to a space.
    func testDecodeSpace() throws {
        XCTAssertEqual(try percentDecode("hello%20world"), "hello world")
    }

    /// Multi-byte UTF-8: 日 = U+65E5 = E6 97 A5 in UTF-8.
    func testDecodeUtf8() throws {
        XCTAssertEqual(try percentDecode("%E6%97%A5"), "日")
    }

    /// Encode then decode returns the original string.
    func testDecodeRoundtrip() throws {
        let original = "hello world/日本語"
        let encoded = percentEncode(original)
        let decoded = try percentDecode(encoded)
        XCTAssertEqual(decoded, original)
    }

    /// Truncated percent encoding ("%2" with no second digit) → error.
    func testDecodeMalformedTruncated() {
        XCTAssertThrowsError(try percentDecode("%2")) { error in
            XCTAssertEqual(error as? UrlError, UrlError.invalidPercentEncoding)
        }
    }

    /// Invalid hex digits → error.
    func testDecodeMalformedBadHex() {
        XCTAssertThrowsError(try percentDecode("%GG")) { error in
            XCTAssertEqual(error as? UrlError, UrlError.invalidPercentEncoding)
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Relative resolution
    // ═══════════════════════════════════════════════════════════════════════

    /// Relative path in the same directory: "d.html" from "/a/b/c.html".
    func testResolveSameDirectory() throws {
        let base = try Url.parse("http://host/a/b/c.html")
        let resolved = try base.resolve("d.html")
        XCTAssertEqual(resolved.scheme, "http")
        XCTAssertEqual(resolved.host, "host")
        XCTAssertEqual(resolved.path, "/a/b/d.html")
    }

    /// Go up one level with "..": "../d.html" from "/a/b/c.html" → "/a/d.html".
    func testResolveParentDirectory() throws {
        let base = try Url.parse("http://host/a/b/c.html")
        let resolved = try base.resolve("../d.html")
        XCTAssertEqual(resolved.path, "/a/d.html")
    }

    /// Go up two levels: "../../d.html" from "/a/b/c.html" → "/d.html".
    func testResolveGrandparentDirectory() throws {
        let base = try Url.parse("http://host/a/b/c.html")
        let resolved = try base.resolve("../../d.html")
        XCTAssertEqual(resolved.path, "/d.html")
    }

    /// Absolute path replaces entire base path but keeps authority.
    func testResolveAbsolutePath() throws {
        let base = try Url.parse("http://host/a/b/c.html")
        let resolved = try base.resolve("/x/y.html")
        XCTAssertEqual(resolved.path, "/x/y.html")
        XCTAssertEqual(resolved.host, "host")
    }

    /// Scheme-relative "//other.com/path" inherits base scheme.
    func testResolveSchemeRelative() throws {
        let base = try Url.parse("http://host/a/b")
        let resolved = try base.resolve("//other.com/path")
        XCTAssertEqual(resolved.scheme, "http")
        XCTAssertEqual(resolved.host, "other.com")
        XCTAssertEqual(resolved.path, "/path")
    }

    /// Already-absolute URL is returned as-is.
    func testResolveAlreadyAbsolute() throws {
        let base = try Url.parse("http://host/a/b")
        let resolved = try base.resolve("https://other.com/x")
        XCTAssertEqual(resolved.scheme, "https")
        XCTAssertEqual(resolved.host, "other.com")
        XCTAssertEqual(resolved.path, "/x")
    }

    /// Single dot segment: "./d" from "/a/b/c" → "/a/b/d".
    func testResolveDotSegments() throws {
        let base = try Url.parse("http://host/a/b/c")
        let resolved = try base.resolve("./d")
        XCTAssertEqual(resolved.path, "/a/b/d")
    }

    /// Empty relative string returns base without fragment.
    func testResolveEmptyReturnsBase() throws {
        let base = try Url.parse("http://host/a/b?q=1#frag")
        let resolved = try base.resolve("")
        XCTAssertEqual(resolved.path, "/a/b")
        XCTAssertEqual(resolved.query, "q=1")
        XCTAssertNil(resolved.fragment) // fragment stripped
    }

    /// Fragment-only relative: "#sec" keeps base path, sets fragment.
    func testResolveFragmentOnly() throws {
        let base = try Url.parse("http://host/a/b")
        let resolved = try base.resolve("#sec")
        XCTAssertEqual(resolved.path, "/a/b")
        XCTAssertEqual(resolved.fragment, "sec")
    }

    /// Relative path with query: "c?key=val" from "/a/b".
    func testResolveWithQuery() throws {
        let base = try Url.parse("http://host/a/b")
        let resolved = try base.resolve("c?key=val")
        XCTAssertEqual(resolved.path, "/a/c")
        XCTAssertEqual(resolved.query, "key=val")
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Dot segment removal
    // ═══════════════════════════════════════════════════════════════════════

    /// Single "." segment is removed.
    func testRemoveSingleDot() throws {
        // We test indirectly through resolve with an absolute path
        let base = try Url.parse("http://host/")
        let resolved = try base.resolve("/a/./b")
        XCTAssertEqual(resolved.path, "/a/b")
    }

    /// ".." goes up one level.
    func testRemoveDoubleDot() throws {
        let base = try Url.parse("http://host/")
        let resolved = try base.resolve("/a/b/../c")
        XCTAssertEqual(resolved.path, "/a/c")
    }

    /// Multiple ".." segments.
    func testRemoveMultipleDoubleDots() throws {
        let base = try Url.parse("http://host/")
        let resolved = try base.resolve("/a/b/../../c")
        XCTAssertEqual(resolved.path, "/c")
    }

    /// ".." can't go above root — stops at "/".
    func testDoubleDotAboveRoot() throws {
        let base = try Url.parse("http://host/")
        let resolved = try base.resolve("/a/../../../c")
        XCTAssertEqual(resolved.path, "/c")
    }

    // ═══════════════════════════════════════════════════════════════════════
    // to_url_string / Display / round-trip
    // ═══════════════════════════════════════════════════════════════════════

    /// Full URL with all components round-trips correctly.
    func testRoundtripFullUrl() throws {
        let input = "http://user:pass@host.com:8080/path?q=1#frag"
        let url = try Url.parse(input)
        XCTAssertEqual(url.toUrlString(), input)
    }

    /// Simple URL round-trips correctly.
    func testRoundtripSimpleUrl() throws {
        let input = "http://example.com/path"
        let url = try Url.parse(input)
        XCTAssertEqual(url.toUrlString(), input)
    }

    /// CustomStringConvertible (description) matches toUrlString().
    func testDescriptionMatchesToUrlString() throws {
        let url = try Url.parse("http://example.com/path?q=1#f")
        XCTAssertEqual(url.description, url.toUrlString())
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Historical Mosaic-era URLs
    // ═══════════════════════════════════════════════════════════════════════

    /// The very first web page URL from CERN.
    func testParseCernOriginalUrl() throws {
        let url = try Url.parse("http://info.cern.ch/hypertext/WWW/TheProject.html")
        XCTAssertEqual(url.scheme, "http")
        XCTAssertEqual(url.host, "info.cern.ch")
        XCTAssertEqual(url.path, "/hypertext/WWW/TheProject.html")
        XCTAssertEqual(url.effectivePort(), 80)
    }

    /// NCSA Mosaic homepage URL.
    func testParseNcsaMosaicUrl() throws {
        let url = try Url.parse("http://www.ncsa.uiuc.edu/SDG/Software/Mosaic/")
        XCTAssertEqual(url.host, "www.ncsa.uiuc.edu")
        XCTAssertEqual(url.path, "/SDG/Software/Mosaic/")
    }

    // ═══════════════════════════════════════════════════════════════════════
    // IPv6
    // ═══════════════════════════════════════════════════════════════════════

    /// IPv6 localhost address with port.
    func testParseIpv6Localhost() throws {
        let url = try Url.parse("http://[::1]:8080/path")
        XCTAssertEqual(url.host, "[::1]")
        XCTAssertEqual(url.port, 8080)
        XCTAssertEqual(url.path, "/path")
    }

    /// IPv6 address without port.
    func testParseIpv6WithoutPort() throws {
        let url = try Url.parse("http://[::1]/path")
        XCTAssertEqual(url.host, "[::1]")
        XCTAssertNil(url.port)
        XCTAssertEqual(url.path, "/path")
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Edge cases
    // ═══════════════════════════════════════════════════════════════════════

    /// Trailing slash produces path "/".
    func testParseTrailingSlash() throws {
        let url = try Url.parse("http://host/")
        XCTAssertEqual(url.path, "/")
    }

    /// Query without explicit path → path defaults to "/".
    func testParseQueryWithoutPath() throws {
        let url = try Url.parse("http://host?q=1")
        XCTAssertEqual(url.host, "host")
        XCTAssertEqual(url.path, "/")
        XCTAssertEqual(url.query, "q=1")
    }

    /// Fragment without explicit path → path defaults to "/".
    func testParseFragmentWithoutPath() throws {
        let url = try Url.parse("http://host#frag")
        XCTAssertEqual(url.host, "host")
        XCTAssertEqual(url.path, "/")
        XCTAssertEqual(url.fragment, "frag")
    }

    /// Equatable conformance: two identical parses are equal.
    func testEquatable() throws {
        let a = try Url.parse("http://example.com/path?q=1#frag")
        let b = try Url.parse("http://example.com/path?q=1#frag")
        XCTAssertEqual(a, b)
    }

    /// UrlError equatable conformance.
    func testUrlErrorEquatable() {
        XCTAssertEqual(UrlError.missingScheme, UrlError.missingScheme)
        XCTAssertNotEqual(UrlError.missingScheme, UrlError.invalidScheme)
    }

    /// Scheme with plus, dash, dot: "svn+ssh", "coap+tcp".
    func testSchemeWithSpecialChars() throws {
        let url = try Url.parse("svn+ssh://host/repo")
        XCTAssertEqual(url.scheme, "svn+ssh")
        XCTAssertEqual(url.host, "host")
    }

    /// Userinfo with special characters.
    func testUserinfoWithSpecialChars() throws {
        let url = try Url.parse("http://user%40name:p%40ss@host/path")
        XCTAssertEqual(url.userinfo, "user%40name:p%40ss")
        XCTAssertEqual(url.host, "host")
    }

    /// Empty path after authority.
    func testEmptyPathDefaultsToSlash() throws {
        let url = try Url.parse("http://host")
        XCTAssertEqual(url.path, "/")
    }

    /// Multiple query parameters.
    func testMultipleQueryParams() throws {
        let url = try Url.parse("http://host/path?a=1&b=2&c=3")
        XCTAssertEqual(url.query, "a=1&b=2&c=3")
    }

    /// Port zero is valid.
    func testPortZero() throws {
        let url = try Url.parse("http://host:0/path")
        XCTAssertEqual(url.port, 0)
    }

    /// Port 65535 (max UInt16) is valid.
    func testPortMax() throws {
        let url = try Url.parse("http://host:65535/path")
        XCTAssertEqual(url.port, 65535)
    }

    /// Encode special characters: @, :, ?, #.
    func testEncodeSpecialChars() {
        let encoded = percentEncode("@:?#")
        XCTAssertEqual(encoded, "%40%3A%3F%23")
    }

    /// Decode lowercase hex digits.
    func testDecodeLowercaseHex() throws {
        XCTAssertEqual(try percentDecode("%2f"), "/")
    }

    /// Mailto with query (e.g., subject).
    func testMailtoWithQuery() throws {
        let url = try Url.parse("mailto:alice@example.com?subject=Hello")
        XCTAssertEqual(url.scheme, "mailto")
        XCTAssertEqual(url.path, "alice@example.com")
        XCTAssertEqual(url.query, "subject=Hello")
    }
}
