package documenthtmlsanitizer

import (
	"regexp"
	"strings"
)

// ─── URL Sanitization for HTML Attributes ────────────────────────────────────
//
// This is an independent copy of the URL sanitization logic — it does NOT share
// code with document-ast-sanitizer. The design decision (Decision 1 in the spec)
// is that the HTML sanitizer has NO external dependencies. Sharing a common
// library would introduce coupling.
//
// The logic is identical to the AST sanitizer's url_utils.go by design.

// htmlUrlControlChars strips the same invisible characters that browsers strip
// before scheme detection. See the AST sanitizer's url_utils.go for the full
// rationale.
var htmlUrlControlChars = regexp.MustCompile("[\u0000-\u001F\u200B-\u200D\u2060\uFEFF]")

// htmlStripControlChars removes invisible bypass characters from a URL.
func htmlStripControlChars(url string) string {
	return htmlUrlControlChars.ReplaceAllString(url, "")
}

// htmlExtractScheme returns the lowercased scheme from a URL (everything before
// the first ":"), or "" for relative URLs.
//
// A colon that appears after a "/" or "?" is not a scheme delimiter.
func htmlExtractScheme(url string) string {
	colonIdx := strings.IndexByte(url, ':')
	if colonIdx < 0 {
		return ""
	}
	// A colon after "/", "?", or "#" is not a scheme delimiter.
	// "#javascript:alert(1)" is a fragment URL, not a javascript: URL.
	for i := 0; i < colonIdx; i++ {
		if url[i] == '/' || url[i] == '?' || url[i] == '#' {
			return ""
		}
	}
	return strings.ToLower(url[:colonIdx])
}

// htmlIsSchemeAllowed reports whether the URL should pass scheme validation.
// Returns true for relative URLs (no scheme) and for URLs whose scheme is in
// the allowlist.
func htmlIsSchemeAllowed(url string, policy HtmlSanitizationPolicy) bool {
	if policy.AllowAllUrlSchemes {
		return true
	}
	clean := htmlStripControlChars(url)
	scheme := htmlExtractScheme(clean)
	if scheme == "" {
		return true // relative URL
	}
	for _, allowed := range policy.AllowedUrlSchemes {
		if strings.ToLower(allowed) == scheme {
			return true
		}
	}
	return false
}
