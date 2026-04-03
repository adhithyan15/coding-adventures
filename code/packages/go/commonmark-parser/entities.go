package commonmarkparser

import (
	"regexp"
	"strings"
	"unicode/utf8"
)

// entityPattern matches all three forms of HTML character references:
//
//	&name;     — named entity (1–32 alphanumeric chars)
//	&#NNN;     — decimal numeric reference
//	&#xHHH;   — hex numeric reference
var entityPattern = regexp.MustCompile(`&(?:#[xX][0-9a-fA-F]{1,6}|#[0-9]{1,7}|[a-zA-Z][a-zA-Z0-9]{0,31});`)

// DecodeEntity decodes a single HTML character reference string like "&amp;",
// "&#65;", or "&#x41;" into its Unicode character equivalent.
//
// If the reference is not recognised (invalid name, out-of-range code point),
// the original reference string is returned as-is for named entities, or
// U+FFFD (replacement character) for invalid numeric entities.
//
// # Why this matters
//
// CommonMark spec §2.5: "An entity reference consists of & + any valid HTML5
// named entity + ;". The decoded character should appear in the output, not
// the raw reference. So &amp; in Markdown source becomes & in the rendered text.
func DecodeEntity(ref string) string {
	result, _ := StartNew[string]("commonmark-parser.DecodeEntity", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("ref", ref)
			if !strings.HasPrefix(ref, "&") || !strings.HasSuffix(ref, ";") {
				return rf.Generate(true, false, ref)
			}

			inner := ref[1 : len(ref)-1] // strip & and ;

			// Numeric reference: &#NNN; or &#xHHH;
			if strings.HasPrefix(inner, "#") {
				rest := inner[1:]
				var codePoint rune

				if strings.HasPrefix(rest, "x") || strings.HasPrefix(rest, "X") {
					// Hex reference: &#xHHH;
					var n int64
					_, err := scanHex(rest[1:], &n)
					if err != nil {
						return rf.Generate(true, false, "\uFFFD")
					}
					codePoint = rune(n)
				} else {
					// Decimal reference: &#NNN;
					var n int64
					_, err := scanDecimal(rest, &n)
					if err != nil {
						return rf.Generate(true, false, "\uFFFD")
					}
					codePoint = rune(n)
				}

				// Invalid or null code point — per CommonMark spec, replace with U+FFFD
				if codePoint == 0 || codePoint > 0x10FFFF || !utf8.ValidRune(codePoint) {
					return rf.Generate(true, false, "\uFFFD")
				}

				return rf.Generate(true, false, string(codePoint))
			}

			// Named reference: &name;
			if decoded, ok := namedEntities[inner]; ok {
				return rf.Generate(true, false, decoded)
			}
			return rf.Generate(true, false, ref) // unrecognised — leave as-is
		}).GetResult()
	return result
}

// scanHex parses a hex string and stores the result in *n.
// Returns the number of chars parsed and nil error on success.
func scanHex(s string, n *int64) (int, error) {
	var result int64
	if len(s) == 0 {
		return 0, errNoDigits
	}
	for i, ch := range s {
		var d int64
		switch {
		case ch >= '0' && ch <= '9':
			d = int64(ch - '0')
		case ch >= 'a' && ch <= 'f':
			d = int64(ch-'a') + 10
		case ch >= 'A' && ch <= 'F':
			d = int64(ch-'A') + 10
		default:
			return i, errBadDigit
		}
		result = result*16 + d
		_ = i
	}
	*n = result
	return len(s), nil
}

// scanDecimal parses a decimal string and stores the result in *n.
func scanDecimal(s string, n *int64) (int, error) {
	var result int64
	if len(s) == 0 {
		return 0, errNoDigits
	}
	for i, ch := range s {
		if ch < '0' || ch > '9' {
			return i, errBadDigit
		}
		result = result*10 + int64(ch-'0')
		_ = i
	}
	*n = result
	return len(s), nil
}

type parseError string

func (e parseError) Error() string { return string(e) }

const errNoDigits parseError = "no digits"
const errBadDigit parseError = "bad digit"

// DecodeEntities decodes all HTML character references in a string.
//
// Scans for &...; patterns and replaces each recognised reference with
// its decoded character. Unrecognised references are left as-is.
//
//	DecodeEntities("Tom &amp; Jerry")           // "Tom & Jerry"
//	DecodeEntities("&#x1F600; smile")           // "😀 smile"
//	DecodeEntities("&lt;p&gt;hello&lt;/p&gt;") // "<p>hello</p>"
func DecodeEntities(text string) string {
	result, _ := StartNew[string]("commonmark-parser.DecodeEntities", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("text_len", len(text))
			// Fast path: no & means no entities
			if !strings.Contains(text, "&") {
				return rf.Generate(true, false, text)
			}
			return rf.Generate(true, false, entityPattern.ReplaceAllStringFunc(text, DecodeEntity))
		}).GetResult()
	return result
}

// EscapeHTML encodes characters that must be escaped in HTML attribute
// values and text content: & < > "
//
// The four characters escaped are the ones with HTML significance in text:
//
//	&  → &amp;
//	<  → &lt;
//	>  → &gt;
//	"  → &quot;
//
// Apostrophes are NOT escaped because CommonMark's reference implementation
// uses double-quoted attributes.
func EscapeHTML(text string) string {
	result, _ := StartNew[string]("commonmark-parser.EscapeHTML", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("text_len", len(text))
			// Fast path: scan for any of the special characters
			needsEscape := false
			for _, ch := range text {
				if ch == '&' || ch == '<' || ch == '>' || ch == '"' {
					needsEscape = true
					break
				}
			}
			if !needsEscape {
				return rf.Generate(true, false, text)
			}

			var b strings.Builder
			b.Grow(len(text) + 16)
			for _, ch := range text {
				switch ch {
				case '&':
					b.WriteString("&amp;")
				case '<':
					b.WriteString("&lt;")
				case '>':
					b.WriteString("&gt;")
				case '"':
					b.WriteString("&quot;")
				default:
					b.WriteRune(ch)
				}
			}
			return rf.Generate(true, false, b.String())
		}).GetResult()
	return result
}
