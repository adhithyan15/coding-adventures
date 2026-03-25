// Package documenthtmlsanitizer sanitizes an HTML string by stripping dangerous
// elements and attributes using regexp-based pattern matching.
//
// # No DOM dependency
//
// This package has NO dependencies — not even on document-ast. It is a
// pure string → string transformation using only the Go standard library.
// This design choice follows Decision 5 from the TE02 spec:
//
//	"The HTML sanitizer uses regex/string operations for portability across
//	Go, Python, Rust, Elixir, Lua, and edge JS runtimes — none of which have
//	a native DOM."
//
// # When to use this package
//
// Use the HTML sanitizer when:
//   - You receive HTML from an external source (CMS, API, user paste)
//   - The AST is no longer available (the pipeline has already rendered)
//   - You need a belt-and-suspenders second pass after document-ast-sanitizer
//
// For best results, sanitize BEFORE rendering with document-ast-sanitizer
// (Stage 1), and then apply this package (Stage 2) as a safety net.
//
// # Limitations
//
// Regexp-based HTML sanitization cannot handle all HTML edge cases with the
// same fidelity as a real DOM parser. In particular:
//
//   - Malformed HTML (unclosed tags, mismatched quotes) may not be handled
//     as a browser would.
//   - Complex CSS values are stripped as a whole attribute rather than parsed.
//   - For the highest fidelity, use a real HTML parser (e.g. golang.org/x/net/html)
//     as a pre-processing step before calling SanitizeHtml.
//
// Spec: TE02 — Document Sanitization (Stage 2)
package documenthtmlsanitizer

// ─── Policy Type ──────────────────────────────────────────────────────────────

// HtmlSanitizationPolicy controls what the HTML sanitizer removes or keeps.
//
// All fields are optional — a zero-value policy keeps everything (equivalent
// to HTML_PASSTHROUGH). Use the named presets as starting points.
type HtmlSanitizationPolicy struct {

	// DropElements is a list of element names (lowercase) that should be
	// removed entirely, including all content nested inside them.
	//
	// Example: "script" → <script>alert(1)</script> is entirely removed.
	//
	// Default (HTML_STRICT): ["script","style","iframe","object","embed",
	// "applet","form","input","button","select","textarea","noscript","meta",
	// "link","base"]
	DropElements []string

	// DropAttributes is a list of attribute names (lowercase) that should be
	// stripped from ALL elements. In addition to this list, all "on*"
	// event handler attributes are ALWAYS stripped (hardcoded safety measure
	// in the sanitizer, regardless of policy).
	//
	// Default (HTML_STRICT): ["srcdoc", "formaction"]
	DropAttributes []string

	// AllowedUrlSchemes is an allowlist of URL schemes permitted in href and
	// src attributes. URLs whose scheme is not in the list have the attribute
	// value replaced with "".
	//
	// A nil slice means "allow any scheme" (PASSTHROUGH behaviour).
	// Default (HTML_STRICT): ["http", "https", "mailto"]
	AllowedUrlSchemes []string

	// AllowAllUrlSchemes bypasses URL scheme checks when true.
	// Used by HTML_PASSTHROUGH.
	AllowAllUrlSchemes bool

	// DropComments controls whether HTML comments (<!-- … -->) are removed.
	// Default (HTML_STRICT): true
	DropComments bool

	// SanitizeStyleAttributes controls whether style attributes containing
	// CSS expression() or url() with non-http/https arguments are stripped.
	// When true, the ENTIRE style attribute is removed if it is suspicious.
	// Default (HTML_STRICT): true
	SanitizeStyleAttributes bool
}

// ─── Named Presets ────────────────────────────────────────────────────────────

// HTML_STRICT is the recommended policy for untrusted HTML from external sources:
// user-pasted HTML, CMS output, third-party APIs.
//
// What HTML_STRICT does:
//   - Removes script, style, iframe, object, embed, applet, form, input,
//     button, select, textarea, noscript, meta, link, base elements
//   - Strips all on* event handler attributes
//   - Strips srcdoc and formaction attributes
//   - Allows only http, https, and mailto URL schemes in href/src
//   - Strips HTML comments
//   - Strips style attributes containing CSS expression() or unsafe url()
var HTML_STRICT = HtmlSanitizationPolicy{
	DropElements: []string{
		"script", "style", "iframe", "object", "embed", "applet",
		"form", "input", "button", "select", "textarea",
		"noscript", "meta", "link", "base",
	},
	DropAttributes:          []string{"srcdoc", "formaction"},
	AllowedUrlSchemes:       []string{"http", "https", "mailto"},
	DropComments:            true,
	SanitizeStyleAttributes: true,
}

// HTML_RELAXED is the recommended policy for authenticated users and internal tools.
//
// What HTML_RELAXED does:
//   - Removes script, iframe, object, embed, applet (the most dangerous elements)
//   - Strips all on* event handler attributes
//   - Allows http, https, mailto, and ftp URL schemes
//   - Keeps HTML comments (useful for template markers in internal tools)
//   - Still strips style attributes containing CSS expressions
var HTML_RELAXED = HtmlSanitizationPolicy{
	DropElements:            []string{"script", "iframe", "object", "embed", "applet"},
	DropAttributes:          []string{},
	AllowedUrlSchemes:       []string{"http", "https", "mailto", "ftp"},
	DropComments:            false,
	SanitizeStyleAttributes: true,
}

// HTML_PASSTHROUGH performs no sanitization. Everything passes through
// unchanged. Use only for fully trusted HTML content.
var HTML_PASSTHROUGH = HtmlSanitizationPolicy{
	DropElements:            []string{},
	DropAttributes:          []string{},
	AllowAllUrlSchemes:      true,
	DropComments:            false,
	SanitizeStyleAttributes: false,
}
