// Package urlparser provides an RFC 1738 URL parser with relative resolution
// and percent-encoding.
//
// This package is part of the coding-adventures monorepo, a ground-up
// implementation of the computing stack from transistors to operating systems.
//
// # Architecture
//
// A URL has this general form (RFC 1738 / RFC 3986):
//
//	scheme://userinfo@host:port/path?query#fragment
//
// Not all components are always present. For example, mailto:user@example.com
// has no authority section at all — just scheme:path.
//
// The parser works in a single left-to-right pass:
//  1. Extract scheme (everything before "://", or before ":" for opaque URIs)
//  2. Strip fragment (everything after "#")
//  3. Strip query (everything after "?")
//  4. Split authority from path (first "/" after authority)
//  5. Within authority: extract userinfo (@), detect IPv6 brackets, find port
//  6. Normalize: lowercase scheme and host
//
// Optional fields use Go pointer types — a nil pointer means "absent",
// distinguishing "no port specified" from "port 0".
package urlparser

import (
	"fmt"
	"strings"
	"unicode/utf8"
)

// ──────────────────────────────────────────────────────────────────────────────
// Error type
// ──────────────────────────────────────────────────────────────────────────────

// UrlError represents an error encountered while parsing or manipulating a URL.
// Kind is a short classifier like "InvalidScheme" or "InvalidPort", and Message
// provides human-readable detail.
type UrlError struct {
	Kind    string
	Message string
}

func (e *UrlError) Error() string {
	return fmt.Sprintf("%s: %s", e.Kind, e.Message)
}

// ──────────────────────────────────────────────────────────────────────────────
// URL struct
// ──────────────────────────────────────────────────────────────────────────────

// Url represents a parsed URL. Optional components are nil when absent.
//
//	┌──────────────────────────────────────────────────────────────┐
//	│  scheme :// userinfo @ host : port / path ? query # fragment│
//	│  └──┘      └──────┘   └──┘   └──┘  └──┘   └───┘   └──────┘│
//	│  required  optional   opt.   opt.   "/"    opt.     opt.    │
//	└──────────────────────────────────────────────────────────────┘
type Url struct {
	Scheme   string
	Userinfo *string // nil if absent
	Host     *string // nil if absent
	Port     *uint16 // nil if absent
	Path     string
	Query    *string // nil if absent
	Fragment *string // nil if absent
	raw      string  // the original input, kept for debugging / round-trips
}

// ──────────────────────────────────────────────────────────────────────────────
// Helpers: pointer constructors for optional fields
// ──────────────────────────────────────────────────────────────────────────────

// strPtr returns a pointer to s — a convenience for building optional fields.
func strPtr(s string) *string { return &s }

// portPtr returns a pointer to p.
func portPtr(p uint16) *uint16 { return &p }

// ──────────────────────────────────────────────────────────────────────────────
// Default port table
// ──────────────────────────────────────────────────────────────────────────────

// defaultPorts maps well-known schemes to their default port numbers.
// This is used by EffectivePort to return the implicit port when none is
// specified in the URL.
var defaultPorts = map[string]uint16{
	"http":  80,
	"https": 443,
	"ftp":   21,
}

// ──────────────────────────────────────────────────────────────────────────────
// Scheme validation
// ──────────────────────────────────────────────────────────────────────────────

// isValidScheme checks that a scheme matches the RFC grammar:
//
//	scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
//
// Examples of valid schemes: "http", "ftp", "svn+ssh", "coap+tcp".
// Invalid: "3com" (starts with digit), "" (empty).
func isValidScheme(s string) bool {
	if len(s) == 0 {
		return false
	}
	// First character must be a letter.
	first := s[0]
	if !((first >= 'a' && first <= 'z') || (first >= 'A' && first <= 'Z')) {
		return false
	}
	// Remaining characters: letter, digit, +, -, or .
	for i := 1; i < len(s); i++ {
		c := s[i]
		switch {
		case c >= 'a' && c <= 'z':
		case c >= 'A' && c <= 'Z':
		case c >= '0' && c <= '9':
		case c == '+' || c == '-' || c == '.':
		default:
			return false
		}
	}
	return true
}

// ──────────────────────────────────────────────────────────────────────────────
// Parse — the main entry point
// ──────────────────────────────────────────────────────────────────────────────

// Parse takes a raw URL string and returns a structured Url, or an error if
// the input is malformed.
//
// The algorithm proceeds left-to-right in one pass, peeling off components
// from the outside in:
//
//	input  →  scheme  →  fragment  →  query  →  authority vs path
//
// This mirrors how humans read URLs and avoids backtracking.
func Parse(input string) (*Url, error) {
	raw := input
	u := &Url{raw: raw}

	// ── Step 1: Extract scheme ────────────────────────────────────────────
	// Look for "://" first (authority-based URL). If not found, try ":" for
	// opaque URIs like "mailto:user@host".
	hasAuthority := false
	rest := ""

	if idx := strings.Index(input, "://"); idx >= 0 {
		u.Scheme = strings.ToLower(input[:idx])
		rest = input[idx+3:]
		hasAuthority = true
	} else if idx := strings.Index(input, ":"); idx >= 0 {
		candidate := input[:idx]
		if isValidScheme(candidate) {
			u.Scheme = strings.ToLower(candidate)
			rest = input[idx+1:]
		} else {
			return nil, &UrlError{Kind: "MissingScheme", Message: "no scheme found in URL"}
		}
	} else {
		return nil, &UrlError{Kind: "MissingScheme", Message: "no scheme found in URL"}
	}

	if !isValidScheme(u.Scheme) {
		return nil, &UrlError{Kind: "InvalidScheme", Message: fmt.Sprintf("invalid scheme: %s", u.Scheme)}
	}

	// ── Step 2: Extract fragment (everything after '#') ───────────────────
	// The fragment is the last thing peeled off because it's delimited by
	// the final '#' — it cannot contain another '#' per the RFC.
	if idx := strings.Index(rest, "#"); idx >= 0 {
		frag := rest[idx+1:]
		u.Fragment = &frag
		rest = rest[:idx]
	}

	// ── Step 3: Extract query (everything after '?') ──────────────────────
	if idx := strings.Index(rest, "?"); idx >= 0 {
		q := rest[idx+1:]
		u.Query = &q
		rest = rest[:idx]
	}

	// ── Step 4: Split authority from path ─────────────────────────────────
	if hasAuthority {
		// For authority-based URLs, find the first '/' that starts the path.
		authority := rest
		path := "/"
		if idx := strings.Index(rest, "/"); idx >= 0 {
			authority = rest[:idx]
			path = rest[idx:] // includes the leading '/'
		}
		u.Path = path

		// ── Step 5: Parse authority ───────────────────────────────────────
		if err := parseAuthority(u, authority); err != nil {
			return nil, err
		}
	} else {
		// Opaque URI (e.g. mailto:): everything remaining is the path.
		u.Path = rest
	}

	return u, nil
}

// parseAuthority dissects the authority component: userinfo@host:port.
//
//	authority = [ userinfo "@" ] host [ ":" port ]
//
// IPv6 addresses are enclosed in brackets: [::1]. The parser detects brackets
// to avoid mistaking the colons inside an IPv6 address for a port delimiter.
func parseAuthority(u *Url, authority string) error {
	rest := authority

	// ── Userinfo ──────────────────────────────────────────────────────────
	// Everything before the LAST '@' is userinfo. We use LastIndex because
	// userinfo itself may contain '@' in some edge cases, though rare.
	if idx := strings.LastIndex(rest, "@"); idx >= 0 {
		info := rest[:idx]
		u.Userinfo = &info
		rest = rest[idx+1:]
	}

	// ── IPv6 detection ────────────────────────────────────────────────────
	// If the host starts with '[', look for the closing ']'. The port, if
	// present, follows immediately after the ']'.
	//
	//	[::1]:8080   →  host="[::1]", port=8080
	//	[::1]        →  host="[::1]", no port
	if strings.HasPrefix(rest, "[") {
		closeBracket := strings.Index(rest, "]")
		if closeBracket < 0 {
			return &UrlError{Kind: "InvalidHost", Message: "unterminated IPv6 bracket"}
		}
		hostStr := strings.ToLower(rest[:closeBracket+1])
		u.Host = &hostStr
		after := rest[closeBracket+1:]
		if strings.HasPrefix(after, ":") {
			portStr := after[1:]
			port, err := parsePort(portStr)
			if err != nil {
				return err
			}
			u.Port = portPtr(port)
		}
	} else {
		// ── IPv4 / hostname ──────────────────────────────────────────────
		// The port separator is the LAST colon. This handles hostnames that
		// (unusually) contain colons, though in practice only IPv6 does.
		if idx := strings.LastIndex(rest, ":"); idx >= 0 {
			hostPart := rest[:idx]
			portStr := rest[idx+1:]
			// Only treat as port if all characters are digits.
			if isAllDigits(portStr) && portStr != "" {
				port, err := parsePort(portStr)
				if err != nil {
					return err
				}
				u.Port = portPtr(port)
				rest = hostPart
			}
			// If not all digits, treat the whole thing as the host (no port).
		}
		if rest != "" {
			h := strings.ToLower(rest)
			u.Host = &h
		}
	}

	return nil
}

// isAllDigits returns true if every byte in s is an ASCII digit.
func isAllDigits(s string) bool {
	for i := 0; i < len(s); i++ {
		if s[i] < '0' || s[i] > '9' {
			return false
		}
	}
	return true
}

// parsePort converts a decimal string to a uint16. Returns an error if the
// string is not a valid port number (0–65535).
func parsePort(s string) (uint16, error) {
	if len(s) == 0 {
		return 0, &UrlError{Kind: "InvalidPort", Message: "empty port"}
	}
	var n uint32
	for _, c := range s {
		if c < '0' || c > '9' {
			return 0, &UrlError{Kind: "InvalidPort", Message: fmt.Sprintf("non-digit in port: %s", s)}
		}
		n = n*10 + uint32(c-'0')
		if n > 65535 {
			return 0, &UrlError{Kind: "InvalidPort", Message: fmt.Sprintf("port out of range: %s", s)}
		}
	}
	return uint16(n), nil
}

// ──────────────────────────────────────────────────────────────────────────────
// Methods on Url
// ──────────────────────────────────────────────────────────────────────────────

// EffectivePort returns the port for this URL, considering both explicitly
// specified ports and well-known defaults. If the URL has an explicit port,
// that is returned. Otherwise, the default port for the scheme (if known) is
// returned. If neither applies, nil is returned.
//
//	"http://example.com"       → 80  (default)
//	"http://example.com:9090"  → 9090 (explicit)
//	"mailto:user@host"         → nil (no default for mailto)
func (u *Url) EffectivePort() *uint16 {
	if u.Port != nil {
		return u.Port
	}
	if dp, ok := defaultPorts[u.Scheme]; ok {
		return portPtr(dp)
	}
	return nil
}

// Authority reconstructs the authority component:
//
//	[ userinfo "@" ] host [ ":" port ]
//
// If there is no host, returns an empty string.
func (u *Url) Authority() string {
	if u.Host == nil {
		return ""
	}
	var b strings.Builder
	if u.Userinfo != nil {
		b.WriteString(*u.Userinfo)
		b.WriteByte('@')
	}
	b.WriteString(*u.Host)
	if u.Port != nil {
		b.WriteByte(':')
		b.WriteString(fmt.Sprintf("%d", *u.Port))
	}
	return b.String()
}

// ToUrlString reconstructs the full URL string from its parsed components.
//
// Two forms are produced depending on whether a host is present:
//
//	Host present:   scheme://authority/path?query#fragment
//	Host absent:    scheme:path?query#fragment
//
// The subtle difference: authority-based URLs use "://" and always include a
// "/" before the path, while opaque URIs use just ":" with no slashes.
func (u *Url) ToUrlString() string {
	var b strings.Builder
	b.WriteString(u.Scheme)

	if u.Host != nil {
		b.WriteString("://")
		b.WriteString(u.Authority())
		// Path always starts with "/" for authority-based URLs.
		if u.Path == "" || u.Path[0] != '/' {
			b.WriteByte('/')
		}
		b.WriteString(u.Path)
	} else {
		b.WriteByte(':')
		b.WriteString(u.Path)
	}

	if u.Query != nil {
		b.WriteByte('?')
		b.WriteString(*u.Query)
	}
	if u.Fragment != nil {
		b.WriteByte('#')
		b.WriteString(*u.Fragment)
	}
	return b.String()
}

// String implements fmt.Stringer and delegates to ToUrlString.
func (u *Url) String() string {
	return u.ToUrlString()
}

// ──────────────────────────────────────────────────────────────────────────────
// Relative resolution (RFC 1808 algorithm)
// ──────────────────────────────────────────────────────────────────────────────

// Resolve resolves a relative reference against this URL as a base, producing
// a new absolute URL. The algorithm follows RFC 1808 Section 4:
//
//  1. Empty reference → base URL without fragment
//  2. "#fragment" → base URL with new fragment
//  3. Has scheme → already absolute, return as-is
//  4. "//authority..." → scheme-relative
//  5. "/path" → absolute path (keeps scheme + authority)
//  6. Otherwise → merge relative path with base, then remove dot segments
//
// Dot segments ("." and "..") are resolved after merging:
//
//	/a/b/c + ../d  →  /a/d
//	/a/b/c + ./d   →  /a/b/d
func (u *Url) Resolve(relative string) (*Url, error) {
	// ── Case 1: empty reference ──────────────────────────────────────────
	// "An empty reference resolves to the base URI without its fragment."
	if relative == "" {
		result := u.clone()
		result.Fragment = nil
		result.raw = result.ToUrlString()
		return result, nil
	}

	// ── Case 2: fragment-only ────────────────────────────────────────────
	if strings.HasPrefix(relative, "#") {
		result := u.clone()
		frag := relative[1:]
		result.Fragment = &frag
		result.raw = result.ToUrlString()
		return result, nil
	}

	// ── Case 3: has scheme → already absolute ────────────────────────────
	if idx := strings.Index(relative, "://"); idx >= 0 {
		return Parse(relative)
	}
	// Also check for opaque scheme (e.g. "mailto:...")
	if idx := strings.Index(relative, ":"); idx >= 0 {
		candidate := relative[:idx]
		if isValidScheme(candidate) && !strings.Contains(candidate, "/") {
			return Parse(relative)
		}
	}

	// ── Case 4: scheme-relative (starts with "//") ──────────────────────
	if strings.HasPrefix(relative, "//") {
		return Parse(u.Scheme + ":" + relative)
	}

	// ── Case 5: absolute path (starts with "/") ─────────────────────────
	if strings.HasPrefix(relative, "/") {
		result := u.clone()
		// Split off query and fragment from the relative.
		path, query, fragment := splitPathQueryFragment(relative)
		result.Path = removeDotSegments(path)
		result.Query = query
		result.Fragment = fragment
		result.raw = result.ToUrlString()
		return result, nil
	}

	// ── Case 6: relative path — merge with base ─────────────────────────
	// Merge: take the base path up to the last '/', append the relative ref.
	//
	// Example: base="/a/b/c", relative="d"  →  "/a/b/d"
	//          base="/a/b/c", relative="../d" → "/a/d" (after dot removal)
	basePath := u.Path
	if idx := strings.LastIndex(basePath, "/"); idx >= 0 {
		basePath = basePath[:idx+1]
	} else {
		basePath = "/"
	}

	path, query, fragment := splitPathQueryFragment(relative)
	merged := basePath + path
	result := u.clone()
	result.Path = removeDotSegments(merged)
	result.Query = query
	result.Fragment = fragment
	result.raw = result.ToUrlString()
	return result, nil
}

// splitPathQueryFragment splits a reference into path, optional query, optional
// fragment — mirroring the parse order (fragment first, then query).
func splitPathQueryFragment(ref string) (path string, query *string, fragment *string) {
	path = ref
	if idx := strings.Index(path, "#"); idx >= 0 {
		frag := path[idx+1:]
		fragment = &frag
		path = path[:idx]
	}
	if idx := strings.Index(path, "?"); idx >= 0 {
		q := path[idx+1:]
		query = &q
		path = path[:idx]
	}
	return
}

// clone makes a shallow copy of a Url. Pointer fields point to new copies so
// mutations to the clone don't affect the original.
func (u *Url) clone() *Url {
	c := *u // struct copy
	if u.Userinfo != nil {
		s := *u.Userinfo
		c.Userinfo = &s
	}
	if u.Host != nil {
		s := *u.Host
		c.Host = &s
	}
	if u.Port != nil {
		p := *u.Port
		c.Port = &p
	}
	if u.Query != nil {
		s := *u.Query
		c.Query = &s
	}
	if u.Fragment != nil {
		s := *u.Fragment
		c.Fragment = &s
	}
	return &c
}

// ──────────────────────────────────────────────────────────────────────────────
// Dot-segment removal (RFC 3986 Section 5.2.4)
// ──────────────────────────────────────────────────────────────────────────────

// removeDotSegments processes a path to resolve "." (current directory) and
// ".." (parent directory) segments.
//
// Truth table of transformations:
//
//	Input              Output
//	─────────────────  ──────────────
//	/a/b/c/./d         /a/b/c/d
//	/a/b/c/../d        /a/b/d
//	/a/b/../../../c    /c
//	/../a              /a       (can't go above root)
//	/a/b/c/../../d     /a/d
//
// The algorithm uses a stack: push each segment, pop on "..", skip on ".".
func removeDotSegments(path string) string {
	// Split on "/" and process each segment.
	segments := strings.Split(path, "/")
	var stack []string

	for _, seg := range segments {
		switch seg {
		case ".":
			// Current directory — skip it.
			continue
		case "..":
			// Parent directory — pop the stack (if non-empty).
			if len(stack) > 0 {
				stack = stack[:len(stack)-1]
			}
		default:
			stack = append(stack, seg)
		}
	}

	result := strings.Join(stack, "/")
	// Ensure absolute paths stay absolute.
	if len(path) > 0 && path[0] == '/' && (len(result) == 0 || result[0] != '/') {
		result = "/" + result
	}
	return result
}

// ──────────────────────────────────────────────────────────────────────────────
// Percent-encoding and decoding
// ──────────────────────────────────────────────────────────────────────────────

// PercentEncode encodes a string using percent-encoding (RFC 3986).
//
// Characters that are "unreserved" are left as-is:
//
//	A–Z  a–z  0–9  -  _  .  ~  /
//
// All other bytes are encoded as %XX where XX is the uppercase hexadecimal
// representation of the byte value. For multi-byte UTF-8 characters, each
// byte is encoded separately:
//
//	"café" → "caf%C3%A9"      (é is U+00E9, encoded as two bytes C3 A9)
//	"hello world" → "hello%20world"
func PercentEncode(input string) string {
	var b strings.Builder
	b.Grow(len(input)) // at least this big

	for i := 0; i < len(input); i++ {
		c := input[i]
		if isUnreserved(c) {
			b.WriteByte(c)
		} else {
			// Encode as %XX with uppercase hex.
			b.WriteByte('%')
			b.WriteByte(hexDigit(c >> 4))
			b.WriteByte(hexDigit(c & 0x0F))
		}
	}
	return b.String()
}

// isUnreserved checks if a byte is in the RFC 3986 unreserved set, plus '/'.
// We include '/' because the spec says to preserve path separators.
func isUnreserved(c byte) bool {
	switch {
	case c >= 'A' && c <= 'Z':
		return true
	case c >= 'a' && c <= 'z':
		return true
	case c >= '0' && c <= '9':
		return true
	case c == '-' || c == '_' || c == '.' || c == '~' || c == '/':
		return true
	default:
		return false
	}
}

// hexDigit converts a nibble (0–15) to its uppercase hex character.
func hexDigit(n byte) byte {
	if n < 10 {
		return '0' + n
	}
	return 'A' + (n - 10)
}

// PercentDecode decodes a percent-encoded string back to its original form.
//
// Each %XX sequence is converted to the byte with that hex value. The result
// is interpreted as UTF-8. If a '%' is not followed by exactly two hex digits,
// an error is returned.
//
//	"hello%20world" → "hello world"
//	"caf%C3%A9"     → "café"
func PercentDecode(input string) (string, error) {
	var b strings.Builder
	b.Grow(len(input))

	i := 0
	for i < len(input) {
		if input[i] == '%' {
			// Need at least two more characters for the hex digits.
			if i+2 >= len(input) {
				return "", &UrlError{Kind: "InvalidEncoding", Message: "incomplete percent encoding"}
			}
			hi, okHi := fromHex(input[i+1])
			lo, okLo := fromHex(input[i+2])
			if !okHi || !okLo {
				return "", &UrlError{Kind: "InvalidEncoding", Message: fmt.Sprintf("invalid hex in percent encoding: %%%c%c", input[i+1], input[i+2])}
			}
			b.WriteByte(hi<<4 | lo)
			i += 3
		} else {
			b.WriteByte(input[i])
			i++
		}
	}

	// Validate that the result is valid UTF-8.
	result := b.String()
	if !utf8.ValidString(result) {
		return "", &UrlError{Kind: "InvalidEncoding", Message: "decoded bytes are not valid UTF-8"}
	}
	return result, nil
}

// fromHex converts a single hex character to its numeric value (0–15).
// Returns (value, true) on success, (0, false) for non-hex characters.
func fromHex(c byte) (byte, bool) {
	switch {
	case c >= '0' && c <= '9':
		return c - '0', true
	case c >= 'a' && c <= 'f':
		return c - 'a' + 10, true
	case c >= 'A' && c <= 'F':
		return c - 'A' + 10, true
	default:
		return 0, false
	}
}
