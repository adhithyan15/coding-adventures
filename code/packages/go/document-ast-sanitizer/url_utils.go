package documentastsanitizer

import (
	"regexp"
	"strings"
)

// ─── URL Sanitization Utilities ───────────────────────────────────────────────
//
// URLs in Markdown source can contain invisible characters that browsers strip
// before interpreting the scheme. An attacker can use these to bypass a simple
// string prefix check:
//
//	java\x00script:alert(1)      ← null byte stripped by browser
//	java\u200bscript:alert(1)    ← zero-width space stripped by browser
//	JAVASCRIPT:alert(1)          ← uppercase scheme still runs
//
// Our defence is two-step:
//  1. Strip the dangerous invisible characters before extracting the scheme.
//  2. Lowercase the scheme before comparing against the allowlist.
//
// This matches the approach used by the WHATWG URL parser and cmark-gfm.

// urlControlChars matches all Unicode characters that browsers silently strip
// before scheme detection. This covers:
//
//	U+0000–U+001F  C0 control characters (null, tab, LF, CR, etc.)
//	U+200B         ZERO WIDTH SPACE
//	U+200C         ZERO WIDTH NON-JOINER
//	U+200D         ZERO WIDTH JOINER
//	U+2060         WORD JOINER
//	U+FEFF         ZERO WIDTH NO-BREAK SPACE (BOM)
//
// Notably we do NOT strip U+007F–U+009F here because those characters are
// less commonly exploited in scheme position and stripping them could affect
// legitimate URL content. The C0 controls and zero-width characters are the
// primary bypass vectors.
var urlControlChars = regexp.MustCompile("[\u0000-\u001F\u200B-\u200D\u2060\uFEFF]")

// StripControlChars removes C0 control characters and zero-width Unicode
// characters from the URL string. This is the first step of scheme extraction.
//
//	StripControlChars("java\x00script:alert(1)")  → "javascript:alert(1)"
//	StripControlChars("\u200bjavascript:alert(1)") → "javascript:alert(1)"
func StripControlChars(url string) string {
	result, _ := StartNew[string]("document-ast-sanitizer.StripControlChars", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, urlControlChars.ReplaceAllString(url, ""))
		}).GetResult()
	return result
}

// ExtractScheme returns the scheme component of a URL — everything before the
// first ":" — lowercased. Returns "" if no scheme is found.
//
// A relative URL is one that contains no ":" before the first "/" or "?".
// Relative URLs do not have a scheme and always pass through the sanitizer.
//
// Examples:
//
//	ExtractScheme("https://example.com")   → "https"
//	ExtractScheme("JAVASCRIPT:alert(1)")   → "javascript"
//	ExtractScheme("/relative/path")        → ""   (relative URL)
//	ExtractScheme("../also/relative")      → ""   (relative URL)
//	ExtractScheme("?query=1")              → ""   (relative URL)
//	ExtractScheme("foo/bar:baz")           → ""   (colon after slash)
func ExtractScheme(url string) string {
	result, _ := StartNew[string]("document-ast-sanitizer.ExtractScheme", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			// Find the position of the first colon.
			colonIdx := strings.IndexByte(url, ':')
			if colonIdx < 0 {
				// No colon at all — definitely a relative URL.
				return rf.Generate(true, false, "")
			}

			// If the colon appears AFTER a "/", "?", or "#", this is a path, query,
			// or fragment component that happens to contain a colon, not a scheme.
			//
			// Example: "foo/bar:baz"  — the colon is inside a path segment.
			// Example: "?key=val:ue"  — the colon is inside a query value.
			// Example: "#id:value"    — the colon is inside a fragment (no scheme).
			//
			// The "#" check is important: "#javascript:alert(1)" would otherwise be
			// parsed as having scheme "javascript", but it is a pure fragment URL
			// that browsers treat as a safe same-page anchor navigation.
			for i := 0; i < colonIdx; i++ {
				if url[i] == '/' || url[i] == '?' || url[i] == '#' {
					return rf.Generate(true, false, "")
				}
			}

			// The colon is a genuine scheme delimiter.
			return rf.Generate(true, false, strings.ToLower(url[:colonIdx]))
		}).GetResult()
	return result
}

// IsSchemeAllowed reports whether the given URL is safe according to the
// policy. The logic:
//
//  1. Strip invisible control chars from the URL.
//  2. Extract the scheme.
//  3. If no scheme (relative URL) → always allowed.
//  4. If policy allows all schemes → always allowed.
//  5. Otherwise check if the lowercased scheme is in the allowlist.
//
// Returns true if the URL should be kept, false if it should be replaced with "".
func IsSchemeAllowed(url string, policy SanitizationPolicy) bool {
	result, _ := StartNew[bool]("document-ast-sanitizer.IsSchemeAllowed", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			clean := StripControlChars(url)
			scheme := ExtractScheme(clean)

			// Relative URLs (no scheme) are always safe.
			if scheme == "" {
				return rf.Generate(true, false, true)
			}

			// PASSTHROUGH: caller explicitly allows any scheme.
			if policy.AllowAllSchemes {
				return rf.Generate(true, false, true)
			}

			// Check against the allowlist.
			for _, allowed := range policy.AllowedUrlSchemes {
				if strings.ToLower(allowed) == scheme {
					return rf.Generate(true, false, true)
				}
			}

			return rf.Generate(true, false, false)
		}).GetResult()
	return result
}
