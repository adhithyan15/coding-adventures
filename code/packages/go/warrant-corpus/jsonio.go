package warrantcorpus

import (
	"crypto/sha256"
	"encoding/hex"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"unicode/utf8"
)

// # Bounded, strict JSON reading and canonical writing
//
// The corpus is untrusted input to the runner (VER04i section 6.2): a stale or
// hand-edited corpus must fail closed, not be interpreted charitably. Go's
// encoding/json is convenient and wrong for that job in four ways, so it is not
// used to read anything here:
//
//   - It keeps the LAST of two equal object keys without a word, which is how
//     two readers end up disagreeing about the same document.
//   - It replaces invalid UTF-8 and a lone surrogate escape ("\ud800") with
//     U+FFFD instead of refusing the input.
//   - Decoding into `any` turns every number into a float64, which rounds
//     integers above 2^53; decoding with UseNumber keeps the text but still
//     accepts fractions, exponents and "-0" where an integer count belongs.
//   - Duplicate detection would need a token-level pass anyway.
//
// So this file carries a small one-pass recursive-descent reader. What it
// enforces:
//
//   - files are size-capped BEFORE they are parsed, and only regular files are
//     opened (no symlink to somewhere surprising);
//   - the input must be valid UTF-8 (no replacement);
//   - duplicate object keys are an error;
//   - a number must be a safe integer (magnitude below 2^53): no fraction, no
//     exponent, no "-0", no integer large enough to have been rounded. `U64`
//     VALUES never travel as JSON numbers: they are tagged decimal text parsed
//     with strconv.ParseUint;
//   - an unpaired surrogate escape ("\ud800" with no low half, or a lone low
//     half) is an error, so corpus text is well-formed Unicode;
//   - nesting deeper than maxJSONDepth is an error;
//   - trailing data after the document is an error.
//
// A parsed value is one of: nil, bool, int64, string, []any, map[string]any.
// Go maps have no prototype, so a "__proto__" key is plain data.
//
// Output is canonical: sorted keys, two-space indent, LF, ASCII-only escapes
// (the same ones Python's ensure_ascii writes), and a trailing newline, so a
// regenerated file is byte-identical to the committed one in every lane.
const (
	maxCorpusBytes = 32 * 1024 * 1024
	maxTextBytes   = 8 * 1024 * 1024
	// maxJSONDepth: a 64-deep model IR is about 130 levels of JSON.
	maxJSONDepth = 256
	// maxSafeInt is 2^53 - 1, the largest integer a double holds exactly. Every
	// other lane refuses a bigger JSON number, so this one must too.
	maxSafeInt = 1<<53 - 1
)

// readBytes reads a regular file of at most limit bytes, or refuses.
//
// The size is checked from Lstat first (a huge file is refused without being
// read) and again on what was actually read (the file may have grown).
func readBytes(path string, limit int64) []byte {
	name := filepath.Base(path)
	info, err := os.Lstat(path)
	need(err == nil && info.Mode().IsRegular(), "io", "%s is not a regular file", name)
	file, err := os.Open(path)
	need(err == nil, "io", "cannot read %s", name)
	defer file.Close()
	stat, err := file.Stat()
	need(err == nil && stat.Mode().IsRegular(), "io", "cannot read %s", name)
	need(stat.Size() <= limit, "input-limit", "%s exceeds %d bytes", name, limit)
	data, err := io.ReadAll(io.LimitReader(file, limit+1))
	need(err == nil, "io", "cannot read %s", name)
	need(int64(len(data)) <= limit, "input-limit", "%s exceeds %d bytes", name, limit)
	return data
}

// sha256Hex is the lowercase hex SHA-256 of data.
func sha256Hex(data []byte) string {
	sum := sha256.Sum256(data)
	return hex.EncodeToString(sum[:])
}

// decodeUTF8 returns data as a string, refusing invalid UTF-8 (a BOM is kept as
// data, never stripped).
func decodeUTF8(data []byte, code string) string {
	need(utf8.Valid(data), code, "input is not valid UTF-8")
	return string(data)
}

// jsonReader is a one-pass recursive-descent JSON reader (see the file comment).
type jsonReader struct {
	text string
	at   int
}

func (r *jsonReader) fail(message string) { fail("malformed-json", "%s", message) }

func (r *jsonReader) peek() byte {
	if r.at < len(r.text) {
		return r.text[r.at]
	}
	return 0
}

func (r *jsonReader) space() {
	for r.at < len(r.text) {
		switch r.text[r.at] {
		case ' ', '\t', '\n', '\r':
			r.at++
		default:
			return
		}
	}
}

func (r *jsonReader) expect(literal string) {
	need(strings.HasPrefix(r.text[r.at:], literal), "malformed-json", "invalid or truncated JSON")
	r.at += len(literal)
}

func (r *jsonReader) value(depth int) any {
	need(depth <= maxJSONDepth, "malformed-json", "JSON nests too deeply")
	r.space()
	switch r.peek() {
	case '{':
		return r.object(depth)
	case '[':
		return r.array(depth)
	case '"':
		return r.str()
	case 't':
		r.expect("true")
		return true
	case 'f':
		r.expect("false")
		return false
	case 'n':
		r.expect("null")
		return nil
	}
	return r.number()
}

func isDigit(c byte) bool { return c >= '0' && c <= '9' }

// number reads `-?(0|[1-9][0-9]*)` and nothing more. A fraction or exponent is
// refused (every count in the corpus is an integer); so is -0 and any value
// whose magnitude is not a safe integer. "01" reads as 0 and leaves "1" behind,
// which the caller then rejects as an unexpected character.
func (r *jsonReader) number() int64 {
	negative := r.peek() == '-'
	if negative {
		r.at++
	}
	digitsAt := r.at
	switch {
	case r.peek() == '0':
		r.at++
	case isDigit(r.peek()):
		for isDigit(r.peek()) {
			r.at++
		}
	default:
		r.fail("invalid or truncated JSON")
	}
	switch r.peek() {
	case '.', 'e', 'E':
		r.fail("only integer numbers are allowed")
	}
	digits := r.text[digitsAt:r.at]
	need(len(digits) <= 16, "malformed-json", "number is not a safe integer")
	magnitude, err := strconv.ParseInt(digits, 10, 64)
	need(err == nil && magnitude <= maxSafeInt, "malformed-json", "number is not a safe integer")
	need(!(negative && magnitude == 0), "malformed-json", "number is not a safe integer")
	if negative {
		return -magnitude
	}
	return magnitude
}

func hexValue(c byte) (rune, bool) {
	switch {
	case c >= '0' && c <= '9':
		return rune(c - '0'), true
	case c >= 'a' && c <= 'f':
		return rune(c-'a') + 10, true
	case c >= 'A' && c <= 'F':
		return rune(c-'A') + 10, true
	}
	return 0, false
}

// unit reads the four hex digits of a \u escape.
func (r *jsonReader) unit() rune {
	need(r.at+4 <= len(r.text), "malformed-json", "bad \\u escape")
	var out rune
	for i := 0; i < 4; i++ {
		digit, ok := hexValue(r.text[r.at+i])
		need(ok, "malformed-json", "bad \\u escape")
		out = out<<4 | digit
	}
	r.at += 4
	return out
}

func (r *jsonReader) str() string {
	r.at++ // the opening quote
	var out strings.Builder
	for {
		need(r.at < len(r.text), "malformed-json", "unterminated string")
		c := r.text[r.at]
		r.at++
		switch {
		case c == '"':
			return out.String()
		case c < ' ':
			r.fail("control character in string")
		case c != '\\':
			out.WriteByte(c) // the whole input is valid UTF-8, so bytes copy through
		default:
			need(r.at < len(r.text), "malformed-json", "unterminated string")
			escape := r.text[r.at]
			r.at++
			switch escape {
			case '"', '\\', '/':
				out.WriteByte(escape)
			case 'b':
				out.WriteByte('\b')
			case 'f':
				out.WriteByte('\f')
			case 'n':
				out.WriteByte('\n')
			case 'r':
				out.WriteByte('\r')
			case 't':
				out.WriteByte('\t')
			case 'u':
				out.WriteRune(r.escapedRune())
			default:
				r.fail("bad escape")
			}
		}
	}
}

// escapedRune finishes a \u escape. A high surrogate must be followed by a \u
// low surrogate (together they are one astral character); a lone half of either
// kind is refused instead of being replaced with U+FFFD.
func (r *jsonReader) escapedRune() rune {
	first := r.unit()
	switch {
	case first >= 0xDC00 && first <= 0xDFFF:
		r.fail("string has an unpaired surrogate")
	case first >= 0xD800 && first <= 0xDBFF:
		need(strings.HasPrefix(r.text[r.at:], "\\u"), "malformed-json", "string has an unpaired surrogate")
		r.at += 2
		second := r.unit()
		need(second >= 0xDC00 && second <= 0xDFFF, "malformed-json", "string has an unpaired surrogate")
		return 0x10000 + (first-0xD800)<<10 + (second - 0xDC00)
	}
	return first
}

func (r *jsonReader) array(depth int) []any {
	r.at++
	items := []any{}
	r.space()
	if r.peek() == ']' {
		r.at++
		return items
	}
	for {
		items = append(items, r.value(depth+1))
		r.space()
		c := r.peek()
		r.at++
		if c == ']' {
			return items
		}
		need(c == ',', "malformed-json", "invalid or truncated JSON")
	}
}

func (r *jsonReader) object(depth int) map[string]any {
	r.at++
	result := map[string]any{}
	r.space()
	if r.peek() == '}' {
		r.at++
		return result
	}
	for {
		r.space()
		need(r.peek() == '"', "malformed-json", "object key must be a string")
		key := r.str()
		_, duplicate := result[key]
		need(!duplicate, "malformed-json", "duplicate object key")
		r.space()
		need(r.peek() == ':', "malformed-json", "invalid or truncated JSON")
		r.at++
		result[key] = r.value(depth + 1)
		r.space()
		c := r.peek()
		r.at++
		if c == '}' {
			return result
		}
		need(c == ',', "malformed-json", "invalid or truncated JSON")
	}
}

// parseJSON parses UTF-8 JSON strictly (see the file comment).
func parseJSON(data []byte) any {
	r := &jsonReader{text: decodeUTF8(data, "malformed-json")}
	value := r.value(0)
	r.space()
	need(r.at == len(r.text), "malformed-json", "trailing data after the document")
	return value
}

// writeJSONString appends s as a JSON string using only ASCII: the escapes
// Python's ensure_ascii writes (lowercase hex, surrogate pairs above U+FFFF).
func writeJSONString(out *strings.Builder, s string) {
	const digits = "0123456789abcdef"
	unit := func(u rune) {
		out.WriteString("\\u")
		out.WriteByte(digits[u>>12&0xF])
		out.WriteByte(digits[u>>8&0xF])
		out.WriteByte(digits[u>>4&0xF])
		out.WriteByte(digits[u&0xF])
	}
	out.WriteByte('"')
	for _, c := range s {
		switch {
		case c == '"':
			out.WriteString(`\"`)
		case c == '\\':
			out.WriteString(`\\`)
		case c == '\b':
			out.WriteString(`\b`)
		case c == '\f':
			out.WriteString(`\f`)
		case c == '\n':
			out.WriteString(`\n`)
		case c == '\r':
			out.WriteString(`\r`)
		case c == '\t':
			out.WriteString(`\t`)
		case c < ' ':
			unit(c)
		case c <= 0x7F:
			out.WriteRune(c)
		case c < 0x10000:
			unit(c)
		default:
			c -= 0x10000
			unit(0xD800 + c>>10)
			unit(0xDC00 + c&0x3FF)
		}
	}
	out.WriteByte('"')
}

func writeJSON(out *strings.Builder, value any, indent int) {
	pad := func(level int) { out.WriteString(strings.Repeat("  ", level)) }
	switch v := value.(type) {
	case nil:
		out.WriteString("null")
	case bool:
		out.WriteString(strconv.FormatBool(v))
	case int:
		out.WriteString(strconv.Itoa(v))
	case int64:
		out.WriteString(strconv.FormatInt(v, 10))
	case string:
		writeJSONString(out, v)
	case []string:
		items := make([]any, len(v))
		for i, item := range v {
			items[i] = item
		}
		writeJSON(out, items, indent)
	case []map[string]any:
		items := make([]any, len(v))
		for i, item := range v {
			items[i] = item
		}
		writeJSON(out, items, indent)
	case []any:
		if len(v) == 0 {
			out.WriteString("[]")
			return
		}
		out.WriteString("[\n")
		for i, item := range v {
			pad(indent + 1)
			writeJSON(out, item, indent+1)
			if i < len(v)-1 {
				out.WriteByte(',')
			}
			out.WriteByte('\n')
		}
		pad(indent)
		out.WriteByte(']')
	case map[string]any:
		if len(v) == 0 {
			out.WriteString("{}")
			return
		}
		keys := make([]string, 0, len(v))
		for key := range v {
			keys = append(keys, key)
		}
		// Byte order of valid UTF-8 is code point order, which is what Python's
		// sort_keys uses.
		sort.Strings(keys)
		out.WriteString("{\n")
		for i, key := range keys {
			pad(indent + 1)
			writeJSONString(out, key)
			out.WriteString(": ")
			writeJSON(out, v[key], indent+1)
			if i < len(keys)-1 {
				out.WriteByte(',')
			}
			out.WriteByte('\n')
		}
		pad(indent)
		out.WriteByte('}')
	default:
		panic("renderJSON: unsupported value type")
	}
}

// renderJSON is the canonical rendering: sorted keys, 2-space indent, LF, ASCII,
// final newline.
func renderJSON(document any) string {
	var out strings.Builder
	writeJSON(&out, document, 0)
	out.WriteByte('\n')
	return out.String()
}
