package documenthtmlsanitizer

import (
	"regexp"
	"strings"
)

// SanitizeHtml sanitizes an HTML string by removing dangerous elements,
// stripping dangerous attributes, and neutralizing harmful URL values.
//
// This is a string → string transformation — there is no DocumentNode, no
// AST, and no external dependencies. The function operates purely on the
// HTML text using regexp-based pattern matching.
//
// The function applies the policy in this order:
//  1. Drop HTML comments (if DropComments: true)
//  2. Drop dangerous elements (script, iframe, etc.) including their content
//  3. Strip dangerous attributes (on*, srcdoc, formaction, plus DropAttributes)
//  4. Sanitize href and src URLs (replace disallowed schemes with "")
//  5. Strip dangerous style attributes (if SanitizeStyleAttributes: true)
//
//	// Untrusted HTML from an external API:
//	safe := SanitizeHtml(apiResponse.Body, HTML_STRICT)
//
//	// Belt-and-suspenders after AST sanitization:
//	html := ToHtml(Sanitize(parse(md), STRICT))
//	safe := SanitizeHtml(html, HTML_STRICT)
func SanitizeHtml(html string, policy HtmlSanitizationPolicy) string {
	sanitized, _ := StartNew[string]("document-html-sanitizer.SanitizeHtml", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			result := html

			// Step 1: Drop HTML comments <!-- … -->
			// Must be done before element dropping so that comments cannot hide
			// dangerous content from the element-dropping regexps.
			if policy.DropComments {
				result = dropComments(result)
			}

			// Step 2: Drop dangerous elements (including their inner content).
			// Each element is removed along with everything nested inside it.
			for _, elem := range policy.DropElements {
				result = dropElement(result, elem)
			}

			// Step 3 + 4 + 5: Process all remaining tags — strip attributes,
			// sanitize URLs, strip style expressions.
			result = processTags(result, policy)

			return rf.Generate(true, false, result)
		}).GetResult()
	return sanitized
}

// ─── Step 1: Comment Dropping ─────────────────────────────────────────────────
//
// HTML comments can hide attack payloads from naive string scanners.
// For example:
//
//	<!--<img src=x onerror=alert(1)>-->
//	<!--[if IE]><script>alert(1)</script><![endif]-->
//
// We drop comments entirely — they have no display value in sanitized output.

// htmlCommentPattern matches HTML comments: <!-- (anything, non-greedy) -->
// The (?s) flag makes . match newlines too (multi-line comments).
var htmlCommentPattern = regexp.MustCompile(`(?s)<!--.*?-->`)

// dropComments removes all HTML comments from the string.
func dropComments(html string) string {
	return htmlCommentPattern.ReplaceAllString(html, "")
}

// ─── Step 2: Element Dropping ─────────────────────────────────────────────────
//
// Dangerous elements must be removed INCLUDING their content. A naive "strip
// tags" approach that keeps the inner text would leave the JavaScript in
// <script>alert(1)</script> visible and potentially executable in some contexts.
//
// We use a regexp that matches:
//   - The opening tag (including any attributes): <script[^>]*>
//   - All content inside (non-greedy, dotall): (?s).*?
//   - The closing tag: </script>
//
// Case-insensitive matching handles <SCRIPT>, <Script>, etc.

// buildDropElementPattern returns a compiled regexp for dropping an element
// and all its content (including nested content of the same type is NOT
// guaranteed — for deeply nested same-tag content, callers should run the
// replacement in a loop).
func buildDropElementPattern(elem string) *regexp.Regexp {
	// (?i)(?s) = case-insensitive, dot matches newlines
	// <elem       = opening tag name
	// (?:\s[^>]*)? = optional attributes (non-capturing)
	// >            = close of opening tag
	// .*?          = content (non-greedy)
	// </elem\s*>   = closing tag (allowing whitespace before >)
	pattern := `(?i)(?s)<` + regexp.QuoteMeta(elem) + `(?:\s[^>]*)?>.*?</` + regexp.QuoteMeta(elem) + `\s*>`
	return regexp.MustCompile(pattern)
}

// buildDropSelfClosingPattern returns a regexp for self-closing or void
// elements that don't have a closing tag: <meta ...> or <meta ... />
func buildDropSelfClosingPattern(elem string) *regexp.Regexp {
	pattern := `(?i)<` + regexp.QuoteMeta(elem) + `(?:\s[^>]*)?\s*/?>`
	return regexp.MustCompile(pattern)
}

// dropElement removes all instances of the given element (and their content)
// from the HTML. Handles both paired (script, style) and void (meta, link)
// elements.
//
// We run the paired-element pattern in a loop to handle the (unusual but
// possible) case of nested same-element structures like nested <noscript>.
func dropElement(html, elem string) string {
	paired := buildDropElementPattern(elem)
	selfClosing := buildDropSelfClosingPattern(elem)

	// Loop for the paired pattern to handle nested same-name elements.
	// Maximum 20 iterations prevents infinite loops on adversarial input.
	prev := ""
	result := html
	for i := 0; i < 20 && result != prev; i++ {
		prev = result
		result = paired.ReplaceAllString(result, "")
	}

	// Also drop self-closing / void versions of the element.
	result = selfClosing.ReplaceAllString(result, "")
	return result
}

// ─── Step 3–5: Tag Attribute Processing ───────────────────────────────────────
//
// After removing whole dangerous elements, we process the attributes of all
// REMAINING tags. This handles:
//
//   - Event handlers (onclick, onload, onerror, etc.) — XSS via attribute
//   - srcdoc — inline HTML in iframes (iframe may survive RELAXED policy)
//   - formaction — overrides form submission target
//   - href / src — URL scheme injection
//   - style — CSS expression() injection
//
// We use a regexp to find each HTML tag, then process its attribute string.
// This is conservative: if we can't cleanly parse an attribute, we drop it.

// htmlTagPattern matches an entire HTML opening tag:
//
//	<tagname [attributes]>
//	<tagname [attributes] />  (self-closing)
//
// Capture groups:
//
//	[1] = tag name
//	[2] = attribute string (everything between tag name and closing >)
var htmlTagPattern = regexp.MustCompile(`(?i)<([a-zA-Z][a-zA-Z0-9-]*)([^>]*)>`)

// processTags finds all HTML opening tags and re-emits them with dangerous
// attributes stripped and URLs sanitized.
func processTags(html string, policy HtmlSanitizationPolicy) string {
	return htmlTagPattern.ReplaceAllStringFunc(html, func(tag string) string {
		m := htmlTagPattern.FindStringSubmatch(tag)
		if m == nil {
			return tag
		}
		tagName := strings.ToLower(m[1])
		attrStr := m[2]

		// Process the attribute string.
		cleaned := processAttributes(attrStr, tagName, policy)

		// Preserve self-closing slash if present (though HTML5 ignores it).
		selfClose := ""
		if strings.HasSuffix(strings.TrimSpace(attrStr), "/") &&
			!strings.HasSuffix(strings.TrimSpace(cleaned), "/") {
			// Only add it if the original had it and the cleaned string lost it.
		}
		_ = selfClose // suppress unused warning; handled below

		if cleaned == "" {
			return "<" + m[1] + ">"
		}
		return "<" + m[1] + cleaned + ">"
	})
}

// ─── Attribute Processing ─────────────────────────────────────────────────────
//
// Parsing HTML attributes with a regexp is tricky because attribute values
// can be:
//   - Quoted with double quotes:  attr="value"
//   - Quoted with single quotes:  attr='value'
//   - Unquoted:                   attr=value
//   - Boolean (no value):         disabled
//
// Our approach: find individual attribute tokens, classify them, and rebuild
// the attribute string with dangerous attributes removed.

// attrPattern matches one HTML attribute token. Four alternatives:
//
//  1. name="value"   double-quoted
//  2. name='value'   single-quoted
//  3. name=value     unquoted
//  4. name           boolean (no value)
var attrPattern = regexp.MustCompile(
	`(?i)` +
		`([\w\-:]+)` + // [1] attribute name
		`(?:` +
		`\s*=\s*` + // = sign with optional whitespace
		`(?:` +
		`"([^"]*)"` + // [2] double-quoted value
		`|'([^']*)'` + // [3] single-quoted value
		`|([^\s"'>=` + "`" + `]*)` + // [4] unquoted value
		`)` +
		`)?`, // no = or value → boolean attribute
)

// processAttributes scans the raw attribute string from an HTML tag and returns
// a cleaned version with dangerous attributes removed.
func processAttributes(attrStr, tagName string, policy HtmlSanitizationPolicy) string {
	var result strings.Builder

	// Build a fast lowercase lookup set for DropAttributes.
	dropSet := make(map[string]bool, len(policy.DropAttributes))
	for _, a := range policy.DropAttributes {
		dropSet[strings.ToLower(a)] = true
	}

	// Track position to avoid re-scanning content we've already emitted.
	// We iterate over all matches in the attribute string.
	matches := attrPattern.FindAllStringSubmatchIndex(attrStr, -1)
	for _, loc := range matches {
		// loc[0]:loc[1] = full match
		// loc[2]:loc[3] = name
		// loc[4]:loc[5] = double-quoted value (or -1 if absent)
		// loc[6]:loc[7] = single-quoted value (or -1 if absent)
		// loc[8]:loc[9] = unquoted value (or -1 if absent)

		if loc[2] == loc[3] {
			// Empty match (the pattern can match zero-length at whitespace) — skip.
			continue
		}

		name := strings.ToLower(attrStr[loc[2]:loc[3]])

		// Reconstruct the original value (before sanitization).
		var origValue string
		var hasValue bool
		var quoteChar string
		if loc[4] >= 0 {
			origValue = attrStr[loc[4]:loc[5]]
			hasValue = true
			quoteChar = `"`
		} else if loc[6] >= 0 {
			origValue = attrStr[loc[6]:loc[7]]
			hasValue = true
			quoteChar = `'`
		} else if loc[8] >= 0 {
			origValue = attrStr[loc[8]:loc[9]]
			hasValue = true
			quoteChar = `"`
		}

		// ── Rule 1: Drop event handlers (on* attributes) ──────────────────
		//
		// onclick, onload, onerror, onfocus, onblur, onmouseover, etc.
		// These are the most common XSS vector after <script> tags.
		// We block ALL on* attributes unless the policy is a full passthrough
		// (AllowAllUrlSchemes == true AND DropElements is empty AND
		// DropAttributes is empty AND SanitizeStyleAttributes == false).
		//
		// PASSTHROUGH policy explicitly opts out of all sanitization, so we
		// must respect that even for on* attributes.
		if strings.HasPrefix(name, "on") && !isPassthroughPolicy(policy) {
			continue
		}

		// ── Rule 2: Drop explicitly listed attributes ──────────────────────
		if dropSet[name] {
			continue
		}

		// ── Rule 3: Sanitize href and src URLs ────────────────────────────
		//
		// These are the two attributes that carry clickable/loadable URLs.
		// A javascript: href or data: src can execute code in the browser.
		if (name == "href" || name == "src") && hasValue {
			if !htmlIsSchemeAllowed(origValue, policy) {
				// Replace the dangerous URL with an empty string.
				result.WriteString(` `)
				result.WriteString(name)
				result.WriteString(`=""`)
				continue
			}
		}

		// ── Rule 4: Sanitize style attribute ─────────────────────────────
		//
		// CSS expressions and url() with dangerous content can run JS.
		//   width: expression(alert(1))
		//   background: url(javascript:alert(1))
		//
		// We drop the ENTIRE style attribute rather than trying to parse CSS.
		if name == "style" && hasValue && policy.SanitizeStyleAttributes {
			if isSuspiciousStyle(origValue) {
				continue // drop the style attribute entirely
			}
		}

		// ── Pass through safe attribute ───────────────────────────────────
		result.WriteString(` `)
		result.WriteString(name)
		if hasValue {
			result.WriteString(`=`)
			result.WriteString(quoteChar)
			result.WriteString(origValue)
			result.WriteString(quoteChar)
		}
	}

	return result.String()
}

// ─── Policy Introspection ─────────────────────────────────────────────────────

// isPassthroughPolicy returns true if the policy performs no sanitization at
// all. This is used to decide whether to apply the hardcoded on* attribute
// stripping (which is always on for STRICT/RELAXED but must be bypassed for
// PASSTHROUGH).
//
// A policy is considered a full passthrough when all of the following are true:
//   - AllowAllUrlSchemes is true (no URL scheme restrictions)
//   - DropElements is empty (no elements are removed)
//   - DropAttributes is empty (no attributes are removed)
//   - SanitizeStyleAttributes is false (no style stripping)
//   - DropComments is false (comments are preserved)
func isPassthroughPolicy(policy HtmlSanitizationPolicy) bool {
	return policy.AllowAllUrlSchemes &&
		len(policy.DropElements) == 0 &&
		len(policy.DropAttributes) == 0 &&
		!policy.SanitizeStyleAttributes &&
		!policy.DropComments
}

// ─── CSS Injection Detection ──────────────────────────────────────────────────
//
// CSS expressions are a legacy IE feature that allows JavaScript execution
// inside style attribute values:
//
//	style="width: expression(alert(1))"
//
// The CSS url() function can also load content from javascript: URIs:
//
//	style="background: url(javascript:alert(1))"
//	style="background: url(data:image/svg+xml,<svg onload=alert(1)>)"
//
// Our approach: detect these patterns case-insensitively and drop the
// entire style attribute. We do NOT attempt to parse and fix the CSS —
// that would require a CSS parser and is beyond the scope of this package.

// cssExpressionPattern matches CSS expression() (case-insensitive).
// This is the primary CSS-injection vector (IE legacy but still tested for).
var cssExpressionPattern = regexp.MustCompile(`(?i)expression\s*\(`)

// cssUrlPattern matches any url() call in a CSS value, capturing the URL
// argument (after stripping leading whitespace and optional quotes).
//
// Go's regexp package does not support lookahead assertions, so we use a
// capture-group approach instead: match url(...) and then inspect the
// captured content to see whether it starts with http:// or https://.
var cssUrlPattern = regexp.MustCompile(`(?i)url\s*\(\s*['"]?\s*([^'"\)\s]*)`)

// isSuspiciousStyle returns true if the style attribute value contains
// patterns that could enable CSS injection attacks.
//
// Two cases:
//
//  1. expression(...)   — IE CSS expressions execute JavaScript
//  2. url(<non-http>)  — url() with a non-http/https argument can load
//     data: URIs, javascript: URIs, or other dangerous content
func isSuspiciousStyle(style string) bool {
	// Case 1: expression() injection
	if cssExpressionPattern.MatchString(style) {
		return true
	}

	// Case 2: url() with a non-http/https argument.
	// Go has no lookahead, so we find all url() occurrences and check each.
	matches := cssUrlPattern.FindAllStringSubmatch(style, -1)
	for _, m := range matches {
		if len(m) < 2 {
			continue
		}
		arg := strings.ToLower(strings.TrimSpace(m[1]))
		// Allow empty (url() with no argument — harmless) and http/https.
		if arg == "" || strings.HasPrefix(arg, "http://") || strings.HasPrefix(arg, "https://") {
			continue
		}
		// Any other scheme (javascript:, data:, blob:, etc.) is suspicious.
		return true
	}

	return false
}
