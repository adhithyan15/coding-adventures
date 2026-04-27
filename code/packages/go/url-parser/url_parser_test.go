package urlparser

import (
	"testing"
)

// ──────────────────────────────────────────────────────────────────────────────
// Helper functions for tests
// ──────────────────────────────────────────────────────────────────────────────

// requireNoError fails the test immediately if err is non-nil.
func requireNoError(t *testing.T, err error) {
	t.Helper()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
}

// assertEqual fails the test if got != want.
func assertEqual(t *testing.T, label, got, want string) {
	t.Helper()
	if got != want {
		t.Errorf("%s: got %q, want %q", label, got, want)
	}
}

// assertNilStr fails if ptr is non-nil.
func assertNilStr(t *testing.T, label string, ptr *string) {
	t.Helper()
	if ptr != nil {
		t.Errorf("%s: expected nil, got %q", label, *ptr)
	}
}

// assertStrPtr fails if ptr is nil or *ptr != want.
func assertStrPtr(t *testing.T, label string, ptr *string, want string) {
	t.Helper()
	if ptr == nil {
		t.Fatalf("%s: expected %q, got nil", label, want)
	}
	if *ptr != want {
		t.Errorf("%s: got %q, want %q", label, *ptr, want)
	}
}

// assertNilPort fails if ptr is non-nil.
func assertNilPort(t *testing.T, label string, ptr *uint16) {
	t.Helper()
	if ptr != nil {
		t.Errorf("%s: expected nil, got %d", label, *ptr)
	}
}

// assertPort fails if ptr is nil or *ptr != want.
func assertPort(t *testing.T, label string, ptr *uint16, want uint16) {
	t.Helper()
	if ptr == nil {
		t.Fatalf("%s: expected %d, got nil", label, want)
	}
	if *ptr != want {
		t.Errorf("%s: got %d, want %d", label, *ptr, want)
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Parsing tests
// ──────────────────────────────────────────────────────────────────────────────

func TestParseSimpleHTTP(t *testing.T) {
	u, err := Parse("http://example.com")
	requireNoError(t, err)
	assertEqual(t, "scheme", u.Scheme, "http")
	assertStrPtr(t, "host", u.Host, "example.com")
	assertNilPort(t, "port", u.Port)
	assertEqual(t, "path", u.Path, "/")
	assertNilStr(t, "query", u.Query)
	assertNilStr(t, "fragment", u.Fragment)
	assertNilStr(t, "userinfo", u.Userinfo)
}

func TestParseWithPath(t *testing.T) {
	u, err := Parse("http://example.com/foo/bar")
	requireNoError(t, err)
	assertEqual(t, "scheme", u.Scheme, "http")
	assertStrPtr(t, "host", u.Host, "example.com")
	assertEqual(t, "path", u.Path, "/foo/bar")
}

func TestParseAllComponents(t *testing.T) {
	u, err := Parse("http://user:pass@example.com:8080/path?query=1#frag")
	requireNoError(t, err)
	assertEqual(t, "scheme", u.Scheme, "http")
	assertStrPtr(t, "userinfo", u.Userinfo, "user:pass")
	assertStrPtr(t, "host", u.Host, "example.com")
	assertPort(t, "port", u.Port, 8080)
	assertEqual(t, "path", u.Path, "/path")
	assertStrPtr(t, "query", u.Query, "query=1")
	assertStrPtr(t, "fragment", u.Fragment, "frag")
}

func TestParseHTTPS(t *testing.T) {
	u, err := Parse("https://secure.example.com/login")
	requireNoError(t, err)
	assertEqual(t, "scheme", u.Scheme, "https")
	assertStrPtr(t, "host", u.Host, "secure.example.com")
	assertEqual(t, "path", u.Path, "/login")
}

func TestParseFTP(t *testing.T) {
	u, err := Parse("ftp://files.example.com/pub/readme.txt")
	requireNoError(t, err)
	assertEqual(t, "scheme", u.Scheme, "ftp")
	assertStrPtr(t, "host", u.Host, "files.example.com")
	assertEqual(t, "path", u.Path, "/pub/readme.txt")
}

func TestParseMailto(t *testing.T) {
	// Mailto is an opaque URI — no authority, just scheme:path.
	u, err := Parse("mailto:user@example.com")
	requireNoError(t, err)
	assertEqual(t, "scheme", u.Scheme, "mailto")
	assertNilStr(t, "host", u.Host) // no authority in mailto
	assertEqual(t, "path", u.Path, "user@example.com")
}

func TestCaseNormalization(t *testing.T) {
	// Scheme and host should be lowercased.
	u, err := Parse("HTTP://EXAMPLE.COM/Path")
	requireNoError(t, err)
	assertEqual(t, "scheme", u.Scheme, "http")
	assertStrPtr(t, "host", u.Host, "example.com")
	// Path case is preserved.
	assertEqual(t, "path", u.Path, "/Path")
}

// ──────────────────────────────────────────────────────────────────────────────
// Effective port tests
// ──────────────────────────────────────────────────────────────────────────────

func TestEffectivePortDefault(t *testing.T) {
	u, err := Parse("http://example.com")
	requireNoError(t, err)
	assertPort(t, "effective port", u.EffectivePort(), 80)

	u2, err := Parse("https://example.com")
	requireNoError(t, err)
	assertPort(t, "effective port https", u2.EffectivePort(), 443)

	u3, err := Parse("ftp://example.com")
	requireNoError(t, err)
	assertPort(t, "effective port ftp", u3.EffectivePort(), 21)
}

func TestEffectivePortExplicit(t *testing.T) {
	u, err := Parse("http://example.com:9090")
	requireNoError(t, err)
	assertPort(t, "explicit port", u.EffectivePort(), 9090)
}

// ──────────────────────────────────────────────────────────────────────────────
// Authority tests
// ──────────────────────────────────────────────────────────────────────────────

func TestAuthorityAll(t *testing.T) {
	u, err := Parse("http://user:pass@example.com:8080/path")
	requireNoError(t, err)
	assertEqual(t, "authority", u.Authority(), "user:pass@example.com:8080")
}

func TestAuthorityHostOnly(t *testing.T) {
	u, err := Parse("http://example.com/path")
	requireNoError(t, err)
	assertEqual(t, "authority", u.Authority(), "example.com")
}

// ──────────────────────────────────────────────────────────────────────────────
// Error tests
// ──────────────────────────────────────────────────────────────────────────────

func TestMissingScheme(t *testing.T) {
	_, err := Parse("example.com/path")
	if err == nil {
		t.Fatal("expected error for missing scheme")
	}
	urlErr, ok := err.(*UrlError)
	if !ok {
		t.Fatalf("expected *UrlError, got %T", err)
	}
	if urlErr.Kind != "MissingScheme" {
		t.Errorf("expected MissingScheme, got %s", urlErr.Kind)
	}
}

func TestInvalidScheme(t *testing.T) {
	_, err := Parse("123://example.com")
	if err == nil {
		t.Fatal("expected error for invalid scheme")
	}
	urlErr, ok := err.(*UrlError)
	if !ok {
		t.Fatalf("expected *UrlError, got %T", err)
	}
	// Could be MissingScheme or InvalidScheme depending on parse path.
	if urlErr.Kind != "MissingScheme" && urlErr.Kind != "InvalidScheme" {
		t.Errorf("expected MissingScheme or InvalidScheme, got %s", urlErr.Kind)
	}
}

func TestInvalidPort(t *testing.T) {
	_, err := Parse("http://example.com:99999")
	if err == nil {
		t.Fatal("expected error for invalid port")
	}
	urlErr, ok := err.(*UrlError)
	if !ok {
		t.Fatalf("expected *UrlError, got %T", err)
	}
	if urlErr.Kind != "InvalidPort" {
		t.Errorf("expected InvalidPort, got %s", urlErr.Kind)
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Percent-encoding tests
// ──────────────────────────────────────────────────────────────────────────────

func TestEncodeSpace(t *testing.T) {
	assertEqual(t, "encode space", PercentEncode("hello world"), "hello%20world")
}

func TestEncodeUnreserved(t *testing.T) {
	// Unreserved characters should pass through unchanged.
	input := "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.~/"
	assertEqual(t, "encode unreserved", PercentEncode(input), input)
}

func TestEncodeSlashes(t *testing.T) {
	// Slashes are in our unreserved set (path separators).
	assertEqual(t, "encode slashes", PercentEncode("/foo/bar"), "/foo/bar")
}

func TestDecodeSpace(t *testing.T) {
	result, err := PercentDecode("hello%20world")
	requireNoError(t, err)
	assertEqual(t, "decode space", result, "hello world")
}

func TestDecodeUTF8(t *testing.T) {
	// "café" encoded as "caf%C3%A9"
	result, err := PercentDecode("caf%C3%A9")
	requireNoError(t, err)
	assertEqual(t, "decode utf8", result, "café")
}

func TestDecodeRoundtrip(t *testing.T) {
	original := "hello world/café"
	encoded := PercentEncode(original)
	decoded, err := PercentDecode(encoded)
	requireNoError(t, err)
	assertEqual(t, "roundtrip", decoded, original)
}

func TestDecodeMalformed(t *testing.T) {
	// Incomplete percent encoding.
	_, err := PercentDecode("hello%2")
	if err == nil {
		t.Fatal("expected error for malformed percent encoding")
	}

	// Invalid hex characters.
	_, err2 := PercentDecode("hello%ZZ")
	if err2 == nil {
		t.Fatal("expected error for invalid hex in percent encoding")
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Resolve tests (RFC 1808 relative resolution)
// ──────────────────────────────────────────────────────────────────────────────

func TestResolveSameDir(t *testing.T) {
	base, err := Parse("http://example.com/a/b/c")
	requireNoError(t, err)
	result, err := base.Resolve("d")
	requireNoError(t, err)
	assertEqual(t, "resolve same dir", result.Path, "/a/b/d")
	assertEqual(t, "scheme preserved", result.Scheme, "http")
}

func TestResolveParent(t *testing.T) {
	base, err := Parse("http://example.com/a/b/c")
	requireNoError(t, err)
	result, err := base.Resolve("../d")
	requireNoError(t, err)
	assertEqual(t, "resolve parent", result.Path, "/a/d")
}

func TestResolveGrandparent(t *testing.T) {
	base, err := Parse("http://example.com/a/b/c")
	requireNoError(t, err)
	result, err := base.Resolve("../../d")
	requireNoError(t, err)
	assertEqual(t, "resolve grandparent", result.Path, "/d")
}

func TestResolveAbsolutePath(t *testing.T) {
	base, err := Parse("http://example.com/a/b/c")
	requireNoError(t, err)
	result, err := base.Resolve("/x/y/z")
	requireNoError(t, err)
	assertEqual(t, "resolve absolute path", result.Path, "/x/y/z")
	assertStrPtr(t, "host preserved", result.Host, "example.com")
}

func TestResolveSchemeRelative(t *testing.T) {
	base, err := Parse("http://example.com/a/b")
	requireNoError(t, err)
	result, err := base.Resolve("//other.com/path")
	requireNoError(t, err)
	assertEqual(t, "scheme", result.Scheme, "http")
	assertStrPtr(t, "host", result.Host, "other.com")
	assertEqual(t, "path", result.Path, "/path")
}

func TestResolveAlreadyAbsolute(t *testing.T) {
	base, err := Parse("http://example.com/a/b")
	requireNoError(t, err)
	result, err := base.Resolve("https://other.com/new")
	requireNoError(t, err)
	assertEqual(t, "scheme", result.Scheme, "https")
	assertStrPtr(t, "host", result.Host, "other.com")
	assertEqual(t, "path", result.Path, "/new")
}

func TestResolveDotSegments(t *testing.T) {
	base, err := Parse("http://example.com/a/b/c")
	requireNoError(t, err)
	result, err := base.Resolve("./d/../e")
	requireNoError(t, err)
	// ./d/../e relative to /a/b/ → /a/b/d/../e → /a/b/e
	assertEqual(t, "resolve dot segments", result.Path, "/a/b/e")
}

func TestResolveEmpty(t *testing.T) {
	base, err := Parse("http://example.com/a/b?q=1#frag")
	requireNoError(t, err)
	result, err := base.Resolve("")
	requireNoError(t, err)
	assertEqual(t, "path preserved", result.Path, "/a/b")
	assertStrPtr(t, "query preserved", result.Query, "q=1")
	assertNilStr(t, "fragment removed", result.Fragment)
}

func TestResolveFragmentOnly(t *testing.T) {
	base, err := Parse("http://example.com/a/b")
	requireNoError(t, err)
	result, err := base.Resolve("#newfrag")
	requireNoError(t, err)
	assertEqual(t, "path preserved", result.Path, "/a/b")
	assertStrPtr(t, "fragment updated", result.Fragment, "newfrag")
}

func TestResolveWithQuery(t *testing.T) {
	base, err := Parse("http://example.com/a/b")
	requireNoError(t, err)
	result, err := base.Resolve("c?key=val")
	requireNoError(t, err)
	assertEqual(t, "path", result.Path, "/a/c")
	assertStrPtr(t, "query", result.Query, "key=val")
}

// ──────────────────────────────────────────────────────────────────────────────
// Dot segment removal (standalone)
// ──────────────────────────────────────────────────────────────────────────────

func TestDotSegmentRemoval(t *testing.T) {
	tests := []struct {
		input string
		want  string
	}{
		{"/a/./b", "/a/b"},
		{"/a/b/../c", "/a/c"},
		{"/a/b/../../c", "/c"},
		{"/a/b/c/./../../d", "/a/d"},
		{"/../a", "/a"},
		{"/a/b/./c/./d", "/a/b/c/d"},
	}
	for _, tt := range tests {
		got := removeDotSegments(tt.input)
		if got != tt.want {
			t.Errorf("removeDotSegments(%q) = %q, want %q", tt.input, got, tt.want)
		}
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Roundtrip test
// ──────────────────────────────────────────────────────────────────────────────

func TestRoundtrip(t *testing.T) {
	urls := []string{
		"http://example.com/path?query=1#frag",
		"https://user:pass@example.com:8080/path",
		"ftp://files.example.com/pub/readme.txt",
		"mailto:user@example.com",
	}
	for _, raw := range urls {
		u, err := Parse(raw)
		requireNoError(t, err)
		reconstructed := u.ToUrlString()
		// Re-parse and compare fields — the reconstructed URL should parse
		// to the same components.
		u2, err := Parse(reconstructed)
		requireNoError(t, err)
		if u.Scheme != u2.Scheme {
			t.Errorf("roundtrip scheme mismatch for %q: %q vs %q", raw, u.Scheme, u2.Scheme)
		}
		if u.Path != u2.Path {
			t.Errorf("roundtrip path mismatch for %q: %q vs %q", raw, u.Path, u2.Path)
		}
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Historical URLs
// ──────────────────────────────────────────────────────────────────────────────

func TestHistoricalURLs(t *testing.T) {
	// A variety of real-world URLs to ensure broad compatibility.
	tests := []struct {
		input  string
		scheme string
		host   string
		path   string
	}{
		{"http://www.w3.org/Protocols/rfc2616/rfc2616.html", "http", "www.w3.org", "/Protocols/rfc2616/rfc2616.html"},
		{"https://en.wikipedia.org/wiki/URL", "https", "en.wikipedia.org", "/wiki/URL"},
		{"ftp://ftp.gnu.org/gnu/gcc/", "ftp", "ftp.gnu.org", "/gnu/gcc/"},
		{"http://localhost:3000/api/v1", "http", "localhost", "/api/v1"},
	}
	for _, tt := range tests {
		u, err := Parse(tt.input)
		requireNoError(t, err)
		assertEqual(t, tt.input+" scheme", u.Scheme, tt.scheme)
		assertStrPtr(t, tt.input+" host", u.Host, tt.host)
		assertEqual(t, tt.input+" path", u.Path, tt.path)
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// IPv6 tests
// ──────────────────────────────────────────────────────────────────────────────

func TestIPv6(t *testing.T) {
	u, err := Parse("http://[::1]:8080/path")
	requireNoError(t, err)
	assertEqual(t, "scheme", u.Scheme, "http")
	assertStrPtr(t, "host", u.Host, "[::1]")
	assertPort(t, "port", u.Port, 8080)
	assertEqual(t, "path", u.Path, "/path")

	// IPv6 without port.
	u2, err := Parse("http://[2001:db8::1]/path")
	requireNoError(t, err)
	assertStrPtr(t, "host", u2.Host, "[2001:db8::1]")
	assertNilPort(t, "port", u2.Port)
}

// ──────────────────────────────────────────────────────────────────────────────
// Edge cases
// ──────────────────────────────────────────────────────────────────────────────

func TestEdgeCases(t *testing.T) {
	// URL with empty path.
	u, err := Parse("http://example.com")
	requireNoError(t, err)
	assertEqual(t, "empty path defaults to /", u.Path, "/")

	// URL with only query.
	u2, err := Parse("http://example.com?key=val")
	requireNoError(t, err)
	assertStrPtr(t, "query", u2.Query, "key=val")
	assertEqual(t, "path", u2.Path, "/")

	// URL with only fragment.
	u3, err := Parse("http://example.com#section")
	requireNoError(t, err)
	assertStrPtr(t, "fragment", u3.Fragment, "section")

	// URL with empty query and fragment.
	u4, err := Parse("http://example.com/path?#")
	requireNoError(t, err)
	assertStrPtr(t, "empty query", u4.Query, "")
	assertStrPtr(t, "empty fragment", u4.Fragment, "")

	// Deeply nested path.
	u5, err := Parse("http://example.com/a/b/c/d/e/f/g")
	requireNoError(t, err)
	assertEqual(t, "deep path", u5.Path, "/a/b/c/d/e/f/g")

	// Port 0 is valid.
	u6, err := Parse("http://example.com:0/path")
	requireNoError(t, err)
	assertPort(t, "port 0", u6.Port, 0)

	// URL with userinfo but no port.
	u7, err := Parse("http://admin@example.com/path")
	requireNoError(t, err)
	assertStrPtr(t, "userinfo", u7.Userinfo, "admin")
	assertNilPort(t, "no port", u7.Port)

	// Authority with no host (just userinfo@ and empty host).
	u8, err := Parse("http://user@/path")
	requireNoError(t, err)
	assertStrPtr(t, "userinfo", u8.Userinfo, "user")
	assertNilStr(t, "nil host", u8.Host)
}

// ──────────────────────────────────────────────────────────────────────────────
// String / ToUrlString tests
// ──────────────────────────────────────────────────────────────────────────────

func TestToUrlStringWithHost(t *testing.T) {
	u, err := Parse("http://example.com/path?q=1#f")
	requireNoError(t, err)
	assertEqual(t, "to_url_string", u.ToUrlString(), "http://example.com/path?q=1#f")
}

func TestToUrlStringNoHost(t *testing.T) {
	u, err := Parse("mailto:user@example.com")
	requireNoError(t, err)
	assertEqual(t, "to_url_string mailto", u.ToUrlString(), "mailto:user@example.com")
}

func TestStringMethod(t *testing.T) {
	u, err := Parse("http://example.com/path")
	requireNoError(t, err)
	// String() should delegate to ToUrlString().
	assertEqual(t, "String()", u.String(), u.ToUrlString())
}

// ──────────────────────────────────────────────────────────────────────────────
// UrlError formatting
// ──────────────────────────────────────────────────────────────────────────────

func TestUrlErrorFormat(t *testing.T) {
	err := &UrlError{Kind: "TestKind", Message: "test message"}
	assertEqual(t, "error format", err.Error(), "TestKind: test message")
}

// ──────────────────────────────────────────────────────────────────────────────
// Additional encoding edge cases
// ──────────────────────────────────────────────────────────────────────────────

func TestEncodeMultibyteUTF8(t *testing.T) {
	// Each byte of the multi-byte UTF-8 encoding should be percent-encoded.
	encoded := PercentEncode("é")
	assertEqual(t, "encode é", encoded, "%C3%A9")
}

func TestEncodeSpecialChars(t *testing.T) {
	assertEqual(t, "encode @", PercentEncode("@"), "%40")
	assertEqual(t, "encode #", PercentEncode("#"), "%23")
	assertEqual(t, "encode ?", PercentEncode("?"), "%3F")
	assertEqual(t, "encode =", PercentEncode("="), "%3D")
	assertEqual(t, "encode &", PercentEncode("&"), "%26")
}

func TestDecodeUpperAndLowerHex(t *testing.T) {
	// Both uppercase and lowercase hex should decode correctly.
	r1, err := PercentDecode("%2f")
	requireNoError(t, err)
	assertEqual(t, "lower hex", r1, "/")

	r2, err := PercentDecode("%2F")
	requireNoError(t, err)
	assertEqual(t, "upper hex", r2, "/")
}

// ──────────────────────────────────────────────────────────────────────────────
// Authority for no-host URL
// ──────────────────────────────────────────────────────────────────────────────

func TestAuthorityNoHost(t *testing.T) {
	u, err := Parse("mailto:user@example.com")
	requireNoError(t, err)
	assertEqual(t, "authority empty", u.Authority(), "")
}

// ──────────────────────────────────────────────────────────────────────────────
// Effective port for unknown scheme
// ──────────────────────────────────────────────────────────────────────────────

func TestEffectivePortUnknownScheme(t *testing.T) {
	u, err := Parse("mailto:user@example.com")
	requireNoError(t, err)
	assertNilPort(t, "no default port for mailto", u.EffectivePort())
}
