/**
 * Tests for @coding-adventures/url-parser
 *
 * These tests are a faithful port of the Rust url-parser test suite,
 * covering parsing, encoding, decoding, relative resolution, dot-segment
 * removal, and round-tripping. Each test group corresponds to a specific
 * area of URL handling.
 */

import { describe, it, expect } from "vitest";
import {
  VERSION,
  Url,
  UrlError,
  MissingScheme,
  InvalidScheme,
  InvalidPort,
  InvalidPercentEncoding,
  EmptyHost,
  RelativeWithoutBase,
  percentEncode,
  percentDecode,
} from "../src/index.js";

// ============================================================================
// Version
// ============================================================================

describe("version", () => {
  it("has a version", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

// ============================================================================
// Basic parsing
// ============================================================================
//
// These tests verify that the single-pass parser correctly extracts each
// component from a URL string. The simplest case is just scheme + host;
// more complex cases add port, path, query, fragment, and userinfo.

describe("basic parsing", () => {
  it("parse_simple_http_url", () => {
    const url = Url.parse("http://www.example.com");
    expect(url.scheme).toBe("http");
    expect(url.host).toBe("www.example.com");
    expect(url.port).toBeNull();
    expect(url.path).toBe("/");
    expect(url.query).toBeNull();
    expect(url.fragment).toBeNull();
  });

  it("parse_http_with_path", () => {
    const url = Url.parse("http://www.example.com/docs/page.html");
    expect(url.scheme).toBe("http");
    expect(url.host).toBe("www.example.com");
    expect(url.path).toBe("/docs/page.html");
  });

  it("parse_all_components", () => {
    const url = Url.parse(
      "http://alice:secret@www.example.com:8080/docs/page.html?q=hello#section2",
    );
    expect(url.scheme).toBe("http");
    expect(url.userinfo).toBe("alice:secret");
    expect(url.host).toBe("www.example.com");
    expect(url.port).toBe(8080);
    expect(url.path).toBe("/docs/page.html");
    expect(url.query).toBe("q=hello");
    expect(url.fragment).toBe("section2");
  });

  it("parse_https_url", () => {
    const url = Url.parse("https://secure.example.com/login");
    expect(url.scheme).toBe("https");
    expect(url.host).toBe("secure.example.com");
    expect(url.effectivePort()).toBe(443);
  });

  it("parse_ftp_url", () => {
    const url = Url.parse("ftp://files.example.com/pub/readme.txt");
    expect(url.scheme).toBe("ftp");
    expect(url.effectivePort()).toBe(21);
  });

  it("parse_mailto_url", () => {
    // mailto: uses the "scheme:path" form -- no "//" authority separator.
    // The email address becomes the path.
    const url = Url.parse("mailto:alice@example.com");
    expect(url.scheme).toBe("mailto");
    expect(url.host).toBeNull();
    expect(url.path).toBe("alice@example.com");
  });
});

// ============================================================================
// Case normalization
// ============================================================================
//
// Schemes and hosts are case-insensitive per the RFCs. Our parser lowercases
// them during parsing to enable reliable comparisons. Paths, however, are
// case-sensitive (a web server might serve different content for /Page vs /page).

describe("case normalization", () => {
  it("scheme_is_lowercased", () => {
    const url = Url.parse("HTTP://WWW.EXAMPLE.COM/PATH");
    expect(url.scheme).toBe("http");
    expect(url.host).toBe("www.example.com");
    // Path case is preserved -- only scheme and host are lowercased
    expect(url.path).toBe("/PATH");
  });
});

// ============================================================================
// Effective port
// ============================================================================
//
// effectivePort() returns the explicit port if one was specified, otherwise
// the well-known default for the scheme. This is what a TCP client would
// actually connect to.

describe("effective port", () => {
  it("effective_port_http_default", () => {
    const url = Url.parse("http://example.com");
    expect(url.port).toBeNull();
    expect(url.effectivePort()).toBe(80);
  });

  it("effective_port_explicit", () => {
    const url = Url.parse("http://example.com:9090");
    expect(url.port).toBe(9090);
    expect(url.effectivePort()).toBe(9090);
  });
});

// ============================================================================
// Authority
// ============================================================================
//
// The authority is the [userinfo@]host[:port] part of the URL. It identifies
// the server (and optionally credentials) to connect to.

describe("authority", () => {
  it("authority_with_all_parts", () => {
    const url = Url.parse("http://user:pass@host.com:8080/path");
    expect(url.authority()).toBe("user:pass@host.com:8080");
  });

  it("authority_host_only", () => {
    const url = Url.parse("http://host.com/path");
    expect(url.authority()).toBe("host.com");
  });
});

// ============================================================================
// Invalid URLs
// ============================================================================
//
// These tests verify that the parser throws specific, descriptive errors
// for various malformed inputs.

describe("invalid URLs", () => {
  it("missing_scheme", () => {
    expect(() => Url.parse("www.example.com")).toThrow(MissingScheme);
  });

  it("invalid_scheme_starts_with_digit", () => {
    expect(() => Url.parse("1http://x.com")).toThrow(InvalidScheme);
  });

  it("invalid_port_too_large", () => {
    // Port 99999 exceeds the 16-bit maximum (65535)
    expect(() => Url.parse("http://host:99999")).toThrow(InvalidPort);
  });
});

// ============================================================================
// Percent-encoding
// ============================================================================
//
// URLs can only contain ASCII characters. To include spaces, Unicode, or
// special characters, we use percent-encoding: each byte is written as
// %XX where XX is the hex value. For example, a space (byte 0x20) becomes
// %20, and the Japanese character 日 (3 UTF-8 bytes: E6 97 A5) becomes
// %E6%97%A5.

describe("percent-encoding", () => {
  it("encode_space", () => {
    expect(percentEncode("hello world")).toBe("hello%20world");
  });

  it("encode_preserves_unreserved", () => {
    // These characters are safe in URLs and should NOT be encoded
    expect(percentEncode("abc-def_ghi.jkl~mno")).toBe("abc-def_ghi.jkl~mno");
  });

  it("encode_preserves_slashes", () => {
    // Slashes are path separators and must be preserved
    expect(percentEncode("/path/to/file")).toBe("/path/to/file");
  });

  it("decode_space", () => {
    expect(percentDecode("hello%20world")).toBe("hello world");
  });

  it("decode_utf8", () => {
    // 日 = U+65E5 = E6 97 A5 in UTF-8
    // This is the kanji for "sun" or "day"
    expect(percentDecode("%E6%97%A5")).toBe("日");
  });

  it("decode_roundtrip", () => {
    // Encode then decode should return the original string
    const original = "hello world/日本語";
    const encoded = percentEncode(original);
    const decoded = percentDecode(encoded);
    expect(decoded).toBe(original);
  });

  it("decode_malformed_truncated", () => {
    // "%2" is missing the second hex digit
    expect(() => percentDecode("%2")).toThrow(InvalidPercentEncoding);
  });

  it("decode_malformed_bad_hex", () => {
    // "G" is not a valid hex digit
    expect(() => percentDecode("%GG")).toThrow(InvalidPercentEncoding);
  });
});

// ============================================================================
// Relative resolution
// ============================================================================
//
// Relative URLs are like giving directions relative to your current location.
// "d.html" means "same directory, different file". "../d.html" means "go up
// one directory". These tests verify the RFC 1808 resolution algorithm.

describe("relative resolution", () => {
  it("resolve_same_directory", () => {
    // From /a/b/c.html, "d.html" -> /a/b/d.html
    // Like clicking a link on a page to another page in the same folder
    const base = Url.parse("http://host/a/b/c.html");
    const resolved = base.resolve("d.html");
    expect(resolved.scheme).toBe("http");
    expect(resolved.host).toBe("host");
    expect(resolved.path).toBe("/a/b/d.html");
  });

  it("resolve_parent_directory", () => {
    // From /a/b/c.html, "../d.html" -> /a/d.html
    // ".." means go up one level, like "cd .." in a terminal
    const base = Url.parse("http://host/a/b/c.html");
    const resolved = base.resolve("../d.html");
    expect(resolved.path).toBe("/a/d.html");
  });

  it("resolve_grandparent_directory", () => {
    // From /a/b/c.html, "../../d.html" -> /d.html
    // Two levels up from /a/b/ takes us to the root
    const base = Url.parse("http://host/a/b/c.html");
    const resolved = base.resolve("../../d.html");
    expect(resolved.path).toBe("/d.html");
  });

  it("resolve_absolute_path", () => {
    // "/x/y.html" replaces the entire path -- like navigating to a
    // completely different section of the same website
    const base = Url.parse("http://host/a/b/c.html");
    const resolved = base.resolve("/x/y.html");
    expect(resolved.path).toBe("/x/y.html");
    expect(resolved.host).toBe("host");
  });

  it("resolve_scheme_relative", () => {
    // "//other.com/path" inherits only the scheme (http or https)
    // This pattern is used on pages that work on both HTTP and HTTPS
    const base = Url.parse("http://host/a/b");
    const resolved = base.resolve("//other.com/path");
    expect(resolved.scheme).toBe("http");
    expect(resolved.host).toBe("other.com");
    expect(resolved.path).toBe("/path");
  });

  it("resolve_already_absolute", () => {
    // A full URL with its own scheme is already absolute -- base is ignored
    const base = Url.parse("http://host/a/b");
    const resolved = base.resolve("https://other.com/x");
    expect(resolved.scheme).toBe("https");
    expect(resolved.host).toBe("other.com");
    expect(resolved.path).toBe("/x");
  });

  it("resolve_dot_segments", () => {
    // "./d" means "current directory, file d" -- the "." is a no-op
    const base = Url.parse("http://host/a/b/c");
    const resolved = base.resolve("./d");
    expect(resolved.path).toBe("/a/b/d");
  });

  it("resolve_empty_returns_base", () => {
    // Empty relative URL means "the same resource" -- fragment is stripped
    const base = Url.parse("http://host/a/b?q=1#frag");
    const resolved = base.resolve("");
    expect(resolved.path).toBe("/a/b");
    expect(resolved.query).toBe("q=1");
    expect(resolved.fragment).toBeNull(); // fragment stripped
  });

  it("resolve_fragment_only", () => {
    // "#sec" only updates the fragment -- used for in-page navigation
    const base = Url.parse("http://host/a/b");
    const resolved = base.resolve("#sec");
    expect(resolved.path).toBe("/a/b");
    expect(resolved.fragment).toBe("sec");
  });

  it("resolve_with_query", () => {
    // "c?key=val" resolves the path AND sets a new query string
    const base = Url.parse("http://host/a/b");
    const resolved = base.resolve("c?key=val");
    expect(resolved.path).toBe("/a/c");
    expect(resolved.query).toBe("key=val");
  });
});

// ============================================================================
// Dot segment removal
// ============================================================================
//
// These tests verify the internal remove_dot_segments algorithm directly
// through round-trip parsing. The algorithm normalizes paths by resolving
// "." (current directory) and ".." (parent directory) references.

describe("dot segment removal", () => {
  it("remove_single_dot", () => {
    // /a/./b -> /a/b  (the "." is a no-op)
    const base = Url.parse("http://host/");
    const resolved = base.resolve("/a/./b");
    expect(resolved.path).toBe("/a/b");
  });

  it("remove_double_dot", () => {
    // /a/b/../c -> /a/c  (.. cancels out "b")
    const base = Url.parse("http://host/");
    const resolved = base.resolve("/a/b/../c");
    expect(resolved.path).toBe("/a/c");
  });

  it("remove_multiple_double_dots", () => {
    // /a/b/../../c -> /c  (two .. cancel "a" and "b")
    const base = Url.parse("http://host/");
    const resolved = base.resolve("/a/b/../../c");
    expect(resolved.path).toBe("/c");
  });

  it("double_dot_above_root", () => {
    // Can't go above root -- extra ".." are silently ignored
    const base = Url.parse("http://host/");
    const resolved = base.resolve("/a/../../../c");
    expect(resolved.path).toBe("/c");
  });
});

// ============================================================================
// to_url_string / roundtrip
// ============================================================================
//
// A good parser can reconstruct the original URL from its parts. These tests
// verify that parse -> toUrlString produces the same string.

describe("roundtrip", () => {
  it("roundtrip_full_url", () => {
    const input = "http://user:pass@host.com:8080/path?q=1#frag";
    const url = Url.parse(input);
    expect(url.toUrlString()).toBe(input);
  });

  it("roundtrip_simple_url", () => {
    const input = "http://example.com/path";
    const url = Url.parse(input);
    expect(url.toUrlString()).toBe(input);
  });
});

// ============================================================================
// Historical Mosaic-era URLs
// ============================================================================
//
// These are real URLs from the early days of the World Wide Web (1991-1993).
// The first URL ever created by Tim Berners-Lee pointed to info.cern.ch.
// NCSA Mosaic was the first popular graphical web browser.

describe("historical URLs", () => {
  it("parse_cern_original_url", () => {
    // The first web page, created by Tim Berners-Lee at CERN in 1991
    const url = Url.parse(
      "http://info.cern.ch/hypertext/WWW/TheProject.html",
    );
    expect(url.scheme).toBe("http");
    expect(url.host).toBe("info.cern.ch");
    expect(url.path).toBe("/hypertext/WWW/TheProject.html");
    expect(url.effectivePort()).toBe(80);
  });

  it("parse_ncsa_mosaic_url", () => {
    // NCSA Mosaic -- the browser that brought the web to the masses
    const url = Url.parse(
      "http://www.ncsa.uiuc.edu/SDG/Software/Mosaic/",
    );
    expect(url.host).toBe("www.ncsa.uiuc.edu");
    expect(url.path).toBe("/SDG/Software/Mosaic/");
  });
});

// ============================================================================
// IPv6
// ============================================================================
//
// IPv6 addresses contain colons, so they must be enclosed in brackets to
// distinguish them from the port delimiter. Example: [::1]:8080

describe("IPv6", () => {
  it("parse_ipv6_localhost", () => {
    const url = Url.parse("http://[::1]:8080/path");
    expect(url.host).toBe("[::1]");
    expect(url.port).toBe(8080);
    expect(url.path).toBe("/path");
  });
});

// ============================================================================
// Edge cases
// ============================================================================
//
// These tests cover corner cases in URL parsing: trailing slashes, missing
// paths, and queries/fragments without paths.

describe("edge cases", () => {
  it("parse_trailing_slash", () => {
    const url = Url.parse("http://host/");
    expect(url.path).toBe("/");
  });

  it("parse_query_without_path", () => {
    // "http://host?q=1" has no explicit path -- defaults to "/"
    const url = Url.parse("http://host?q=1");
    expect(url.host).toBe("host");
    expect(url.path).toBe("/");
    expect(url.query).toBe("q=1");
  });

  it("parse_fragment_without_path", () => {
    // "http://host#frag" has no explicit path -- defaults to "/"
    const url = Url.parse("http://host#frag");
    expect(url.host).toBe("host");
    expect(url.path).toBe("/");
    expect(url.fragment).toBe("frag");
  });
});

// ============================================================================
// Error class hierarchy
// ============================================================================
//
// All error types inherit from UrlError, so callers can catch either
// specific errors or any URL error.

describe("error hierarchy", () => {
  it("all errors are instances of UrlError", () => {
    expect(new MissingScheme()).toBeInstanceOf(UrlError);
    expect(new InvalidScheme()).toBeInstanceOf(UrlError);
    expect(new InvalidPort()).toBeInstanceOf(UrlError);
    expect(new InvalidPercentEncoding()).toBeInstanceOf(UrlError);
    expect(new EmptyHost()).toBeInstanceOf(UrlError);
    expect(new RelativeWithoutBase()).toBeInstanceOf(UrlError);
  });

  it("all errors are instances of Error", () => {
    expect(new MissingScheme()).toBeInstanceOf(Error);
    expect(new InvalidScheme()).toBeInstanceOf(Error);
  });
});

// ============================================================================
// toString alias
// ============================================================================

describe("toString", () => {
  it("toString returns same as toUrlString", () => {
    const url = Url.parse("http://example.com/path?q=1#frag");
    expect(url.toString()).toBe(url.toUrlString());
  });

  it("mailto toString uses colon form", () => {
    const url = Url.parse("mailto:alice@example.com");
    expect(url.toUrlString()).toBe("mailto:alice@example.com");
  });
});
