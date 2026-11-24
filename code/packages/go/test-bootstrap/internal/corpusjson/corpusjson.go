package corpusjson

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"math/big"
	"strings"
)

// The caps below exist because a conformance fixture is DATA: a hostile or
// corrupted value (count = 10^9, a million-deep list, a 1 GiB document) must be
// a reported problem, never a hang or a stack overflow (CWE-400).
const (
	// MaxSize bounds every fixture-controlled size: counts, lengths, list and
	// bytes elements.
	MaxSize = 10_000
	// MaxIntDigits bounds the decimal digits of any integer in a case.
	MaxIntDigits = 1_000
	// MaxDepth bounds JSON nesting so decoding recursion is finite.
	MaxDepth = 64
	// MaxBytes bounds the JSON document Parse will read.
	MaxBytes = 8 << 20
	// MaxProblemLen bounds one reported problem.  Fixture-derived fragments in a
	// problem string are attacker-controlled, so every fragment goes through
	// Show/Clip: a report is never larger than the problem count times this.
	MaxProblemLen = 400
	// MaxFragmentLen bounds one fixture-derived fragment inside a problem.
	MaxFragmentLen = 200
)

// Parse strictly decodes a corpus document into its case list.  The document
// must be exactly {"schema_version": 1, "cases": [...]}.  Duplicate object
// keys, trailing data, nesting deeper than MaxDepth and documents over
// MaxBytes are refused.  Objects become map[string]any, arrays []any, numbers
// json.Number; cases are returned unvalidated (the runner judges them).  label
// names the corpus in error messages ("pbt corpus", "fuzz corpus").
func Parse(data []byte, label string) ([]any, error) {
	if len(data) > MaxBytes {
		return nil, fmt.Errorf("%s: %d bytes exceeds %d", label, len(data), MaxBytes)
	}
	dec := json.NewDecoder(bytes.NewReader(data))
	dec.UseNumber()
	root, err := readValue(dec, 0)
	if err != nil {
		return nil, fmt.Errorf("%s: %w", label, err)
	}
	if _, err := dec.Token(); !errors.Is(err, io.EOF) {
		return nil, fmt.Errorf("%s: trailing data after the document", label)
	}
	doc, isObj := root.(map[string]any)
	if !isObj || len(doc) != 2 {
		return nil, fmt.Errorf(`%s: want exactly {"schema_version": 1, "cases": [...]}`, label)
	}
	if v, isNum := doc["schema_version"].(json.Number); !isNum || v.String() != "1" {
		return nil, fmt.Errorf("%s: schema_version must be 1", label)
	}
	cases, isList := doc["cases"].([]any)
	if !isList {
		return nil, fmt.Errorf(`%s: "cases" must be an array`, label)
	}
	return cases, nil
}

func readValue(dec *json.Decoder, depth int) (any, error) {
	if depth > MaxDepth {
		return nil, fmt.Errorf("nesting deeper than %d", MaxDepth)
	}
	tok, err := dec.Token()
	if err != nil {
		return nil, err
	}
	delim, isDelim := tok.(json.Delim)
	if !isDelim {
		return tok, nil // string, json.Number, bool or nil
	}
	switch delim {
	case '[':
		out := []any{}
		for dec.More() {
			v, err := readValue(dec, depth+1)
			if err != nil {
				return nil, err
			}
			out = append(out, v)
		}
		if _, err := dec.Token(); err != nil {
			return nil, err
		}
		return out, nil
	case '{':
		out := map[string]any{}
		for dec.More() {
			keyTok, err := dec.Token()
			if err != nil {
				return nil, err
			}
			key, isStr := keyTok.(string)
			if !isStr {
				return nil, errors.New("object key is not a string")
			}
			if _, dup := out[key]; dup {
				return nil, fmt.Errorf("duplicate JSON key %q", Clip(key, 64))
			}
			v, err := readValue(dec, depth+1)
			if err != nil {
				return nil, err
			}
			out[key] = v
		}
		if _, err := dec.Token(); err != nil {
			return nil, err
		}
		return out, nil
	}
	return nil, errors.New("unexpected delimiter")
}

// CaseError is a problem with one case's encoding; its text is already clipped.
type CaseError string

func (e CaseError) Error() string { return string(e) }

// CaseErr formats a CaseError, clipped to MaxProblemLen.
func CaseErr(format string, args ...any) error {
	return CaseError(Clip(fmt.Sprintf(format, args...), MaxProblemLen))
}

// Clip truncates s to limit bytes, marking the cut with "...".
func Clip(s string, limit int) string {
	if len(s) <= limit {
		return s
	}
	return s[:limit] + "..."
}

// Show renders a fixture-derived value for a message: bounded Canon, clipped.
func Show(v any) string { return Clip(Canon(v), MaxFragmentLen) }

// IsCanonicalInt: decimal digits only, a single leading "-" at most, no leading
// zeros, no "-0", nothing a lenient parser would also swallow (underscores,
// spaces, "+").
func IsCanonicalInt(text string) bool {
	digits := strings.TrimPrefix(text, "-")
	if digits == "" {
		return false
	}
	for i := 0; i < len(digits); i++ {
		if digits[i] < '0' || digits[i] > '9' {
			return false
		}
	}
	if digits[0] == '0' {
		return text == "0"
	}
	return true
}

// IsLowercaseHex: even length, digits and a-f only.
func IsLowercaseHex(text string) bool {
	if len(text)%2 != 0 {
		return false
	}
	for i := 0; i < len(text); i++ {
		c := text[i]
		if !(c >= '0' && c <= '9' || c >= 'a' && c <= 'f') {
			return false
		}
	}
	return true
}

func hexToBytes(text string) []byte {
	out := make([]byte, len(text)/2)
	for i := range out {
		out[i] = hexVal(text[2*i])<<4 | hexVal(text[2*i+1])
	}
	return out
}

// hexVal assumes IsLowercaseHex already vetted the byte.
func hexVal(c byte) byte {
	if c >= 'a' {
		return c - 'a' + 10
	}
	return c - '0'
}

// Decode turns a JSON node into bool, string, *big.Int, []byte or []any,
// strictly.
func Decode(node any) (any, error) { return decodeAt(node, 0) }

// decodeAt is Decode with the nesting depth so far.  Runners are exported and
// may be handed a hand-built tree that never went through Parse, so the depth
// cap lives here too: a deeper input is a problem, not a stack overflow.
func decodeAt(node any, depth int) (any, error) {
	if depth > MaxDepth {
		return nil, CaseErr("value nesting deeper than %d", MaxDepth)
	}
	switch n := node.(type) {
	case bool, string:
		return n, nil
	case []any:
		if len(n) > MaxSize {
			return nil, CaseErr("list exceeds %d elements", MaxSize)
		}
		out := make([]any, len(n))
		for i, item := range n {
			v, err := decodeAt(item, depth+1)
			if err != nil {
				return nil, err
			}
			out[i] = v
		}
		return out, nil
	case map[string]any:
		kind, _ := n["kind"].(string)
		switch {
		case kind == "int" && len(n) == 2:
			text, isStr := n["value"].(string)
			if !isStr || !IsCanonicalInt(text) {
				return nil, CaseErr("non-canonical int %s", Show(n["value"]))
			}
			if len(strings.TrimPrefix(text, "-")) > MaxIntDigits {
				return nil, CaseErr("int exceeds %d digits", MaxIntDigits)
			}
			v, _ := new(big.Int).SetString(text, 10)
			return v, nil
		case kind == "bytes" && len(n) == 2:
			text, isStr := n["hex"].(string)
			if !isStr || !IsLowercaseHex(text) {
				return nil, CaseErr("bytes hex must be even-length lowercase: %s", Show(n["hex"]))
			}
			if len(text) > 2*MaxSize {
				return nil, CaseErr("bytes exceed %d bytes", MaxSize)
			}
			return hexToBytes(text), nil
		}
		return nil, CaseErr("unrecognised value node %s", Show(n))
	}
	return nil, CaseErr("unsupported JSON value %s", Show(node))
}

// Same is equality that does not conflate true with 1, lists with bytes, or an
// int with its decimal text.
func Same(a, b any) bool { return sameAt(a, b, 0) }

func sameAt(a, b any, depth int) bool {
	if depth > MaxDepth {
		return false // Decode caps real inputs; deeper is never "the same"
	}
	switch x := a.(type) {
	case bool:
		y, isBool := b.(bool)
		return isBool && x == y
	case string:
		y, isStr := b.(string)
		return isStr && x == y
	case *big.Int:
		y, isBig := b.(*big.Int)
		return isBig && x != nil && y != nil && x.Cmp(y) == 0
	case []byte:
		y, isBytes := b.([]byte)
		return isBytes && bytes.Equal(x, y)
	case []any:
		y, isList := b.([]any)
		if !isList || len(x) != len(y) {
			return false
		}
		for i := range x {
			if !sameAt(x[i], y[i], depth+1) {
				return false
			}
		}
		return true
	}
	return false
}
