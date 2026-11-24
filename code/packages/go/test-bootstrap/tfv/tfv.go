// Package tfv validates TFV v1 text, the line format that carries test corpora
// between languages (TF01 section 2).
//
// This is a validator, not yet a full reader: its one job in this module is to
// let the property engine check the pbt-v1 "corpus_reject" cases with a real
// parser instead of skipping them.  The TF01 runner that will decode values
// into a tree is a later sub-package; the grammar below is complete, so that
// runner can grow from this file.
//
// # The format in one screen
//
//	tfv 1
//	# comment
//	<id> R<n> K<field> <value> K<field> <value> ...
//
// A value is one of
//
//	N  T  F                 null, true, false
//	I<int>                  canonical decimal integer, at most 1000 digits
//	D<16 hex>               IEEE-754 bit pattern, lowercase
//	X<hex>  S<hex>          bytes / UTF-8 text as even-length lowercase hex
//	L<n> v1 ... vn          list
//	M<n> k1 v1 ... kn vn    map; no two keys deep-equal
//
// Tokens are separated by exactly one ASCII space.  Anything else is rejected:
// uppercase or odd-length hex, "I-0", "I007", a count that disagrees with the
// tokens, trailing tokens, doubled spaces, a duplicate field or record id,
// invalid UTF-8 inside S, a carriage return in a record line, nesting deeper
// than 64 containers, and integers of more than 1000 digits.
//
// Worked example.  "tfv 1\nr R1 Kop S706274 Kdata XFF\n" fails: XFF is
// uppercase hex (and odd length).  "bad header\n" fails: the first line must be
// exactly "tfv 1".
package tfv

import (
	"errors"
	"fmt"
	"sort"
	"strconv"
	"strings"
	"unicode/utf8"
)

const (
	// Header is the exact first line of every TFV file.
	Header = "tfv 1"
	// MaxDepth is the deepest container nesting accepted (outermost is 1).
	MaxDepth = 64
	// MaxIntDigits bounds the digits of an integer token (sign excluded).
	MaxIntDigits = 1000
	// MaxInputBytes bounds the size of a document.
	MaxInputBytes = 1 << 20
)

// ErrInvalid wraps every rejection, so callers can use errors.Is.
var ErrInvalid = errors.New("invalid TFV")

// q quotes a fragment of the input for an error message, clipped so a hostile
// megabyte-long token cannot become a megabyte-long error.
func q(s string) string {
	const limit = 64
	if len(s) > limit {
		return strconv.Quote(s[:limit]) + "..."
	}
	return strconv.Quote(s)
}

func reject(format string, args ...any) error {
	return fmt.Errorf("%w: %s", ErrInvalid, fmt.Sprintf(format, args...))
}

// Validate returns nil when text is a well-formed TFV v1 document and an error
// wrapping ErrInvalid otherwise.  It never panics and reserves no memory from a
// count in the input.
func Validate(text string) error {
	if len(text) > MaxInputBytes {
		return reject("input of %d bytes exceeds %d", len(text), MaxInputBytes)
	}
	if !utf8.ValidString(text) {
		return reject("document is not valid UTF-8")
	}
	lines := strings.Split(text, "\n")
	if lines[len(lines)-1] == "" {
		lines = lines[:len(lines)-1] // the final newline is optional
	}
	if len(lines) == 0 || lines[0] != Header {
		return reject("the first line must be exactly %q", Header)
	}
	seenIDs := map[string]bool{}
	for n, line := range lines[1:] {
		if line == "" || line[0] == '#' {
			continue // blank and comment lines are skipped whole
		}
		id, err := validateRecord(line)
		if err != nil {
			return reject("line %d: %v", n+2, err)
		}
		if seenIDs[id] {
			return reject("line %d: duplicate record id %s", n+2, q(id))
		}
		seenIDs[id] = true
	}
	return nil
}

func isIDByte(c byte) bool {
	return c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z' || c >= '0' && c <= '9' ||
		c == '.' || c == '_' || c == '/' || c == '-'
}

func validID(s string) bool {
	if s == "" {
		return false
	}
	for i := 0; i < len(s); i++ {
		if !isIDByte(s[i]) {
			return false
		}
	}
	return true
}

func validFieldName(s string) bool {
	if s == "" || s[0] < 'a' || s[0] > 'z' {
		return false
	}
	for i := 1; i < len(s); i++ {
		c := s[i]
		if !(c >= 'a' && c <= 'z' || c >= '0' && c <= '9' || c == '_') {
			return false
		}
	}
	return true
}

type reader struct {
	tokens []string
	pos    int
}

func (r *reader) next() (string, error) {
	if r.pos >= len(r.tokens) {
		return "", errors.New("record ends early")
	}
	tok := r.tokens[r.pos]
	r.pos++
	return tok, nil
}

func validateRecord(line string) (string, error) {
	if strings.ContainsRune(line, '\r') {
		return "", errors.New("carriage return in a record line")
	}
	tokens := strings.Split(line, " ")
	for _, t := range tokens {
		if t == "" {
			return "", errors.New("leading, trailing or doubled space")
		}
	}
	if len(tokens) < 2 {
		return "", errors.New("a record needs an id and R<n>")
	}
	id := tokens[0]
	if !validID(id) {
		return "", fmt.Errorf("bad record id %s", q(id))
	}
	if tokens[1][0] != 'R' {
		return "", fmt.Errorf("expected R<n>, got %s", q(tokens[1]))
	}
	count, err := canonicalNat(tokens[1][1:])
	if err != nil {
		return "", fmt.Errorf("R count: %w", err)
	}
	r := &reader{tokens: tokens[2:]}
	seen := map[string]bool{}
	for i := 0; i < count; i++ {
		keyTok, err := r.next()
		if err != nil {
			return "", err
		}
		if keyTok[0] != 'K' || !validFieldName(keyTok[1:]) {
			return "", fmt.Errorf("bad field key %s", q(keyTok))
		}
		if seen[keyTok] {
			return "", fmt.Errorf("duplicate field %s", q(keyTok[1:]))
		}
		seen[keyTok] = true
		if _, err := r.value(0); err != nil {
			return "", err
		}
	}
	if r.pos != len(r.tokens) {
		return "", errors.New("trailing tokens after the last field")
	}
	return id, nil
}

// canonicalNat parses 0 or digits without a leading zero.  The result is
// clamped: a count too large for the tokens present is rejected later as
// "record ends early", and no allocation depends on it.
func canonicalNat(s string) (int, error) {
	if s == "" {
		return 0, errors.New("empty count")
	}
	for i := 0; i < len(s); i++ {
		if s[i] < '0' || s[i] > '9' {
			return 0, fmt.Errorf("non-digit in count %s", q(s))
		}
	}
	if len(s) > 1 && s[0] == '0' {
		return 0, fmt.Errorf("leading zero in count %s", q(s))
	}
	if len(s) > 9 {
		return 0, fmt.Errorf("count %s too large", q(s))
	}
	n := 0
	for i := 0; i < len(s); i++ {
		n = n*10 + int(s[i]-'0')
	}
	return n, nil
}

func lowerHexBytes(s string) ([]byte, bool) {
	if len(s)%2 != 0 {
		return nil, false
	}
	out := make([]byte, len(s)/2)
	for i := 0; i < len(s); i += 2 {
		hi, ok1 := hexVal(s[i])
		lo, ok2 := hexVal(s[i+1])
		if !ok1 || !ok2 {
			return nil, false
		}
		out[i/2] = hi<<4 | lo
	}
	return out, true
}

func hexVal(c byte) (byte, bool) {
	switch {
	case c >= '0' && c <= '9':
		return c - '0', true
	case c >= 'a' && c <= 'f':
		return c - 'a' + 10, true
	}
	return 0, false
}

func validInt(body string) error {
	digits := strings.TrimPrefix(body, "-")
	neg := len(digits) != len(body)
	if digits == "" {
		return errors.New("empty integer")
	}
	for i := 0; i < len(digits); i++ {
		if digits[i] < '0' || digits[i] > '9' {
			return fmt.Errorf("non-digit in integer %s", q(body))
		}
	}
	if digits[0] == '0' && (len(digits) > 1 || neg) {
		return fmt.Errorf("non-canonical integer %s", q(body))
	}
	if len(digits) > MaxIntDigits {
		return fmt.Errorf("integer of %d digits exceeds %d", len(digits), MaxIntDigits)
	}
	return nil
}

// value validates one value and returns its deep-equality key, a string that
// is equal for two values exactly when TF01 section 3.1 calls them deep-equal.
// depth is the number of enclosing containers.
func (r *reader) value(depth int) (string, error) {
	tok, err := r.next()
	if err != nil {
		return "", err
	}
	tag, body := tok[0], tok[1:]
	switch tag {
	case 'N', 'T', 'F':
		if body != "" {
			return "", fmt.Errorf("unexpected body in %s", q(tok))
		}
		return string(tag), nil
	case 'I':
		if err := validInt(body); err != nil {
			return "", err
		}
		return tok, nil
	case 'D':
		raw, ok := lowerHexBytes(body)
		if len(body) != 16 || !ok {
			return "", fmt.Errorf("float needs exactly 16 lowercase hex digits: %s", q(tok))
		}
		// Every NaN equals every NaN; -0 and +0 stay distinct.
		exp := uint16(raw[0]&0x7f)<<4 | uint16(raw[1]>>4)
		mantissaZero := raw[1]&0x0f == 0
		for _, b := range raw[2:] {
			mantissaZero = mantissaZero && b == 0
		}
		if exp == 0x7ff && !mantissaZero {
			return "Dnan", nil
		}
		return tok, nil
	case 'X':
		if _, ok := lowerHexBytes(body); !ok {
			return "", fmt.Errorf("bytes need even-length lowercase hex: %s", q(tok))
		}
		return tok, nil
	case 'S':
		raw, ok := lowerHexBytes(body)
		if !ok {
			return "", fmt.Errorf("text needs even-length lowercase hex: %s", q(tok))
		}
		if !utf8.Valid(raw) {
			return "", fmt.Errorf("text is not strict UTF-8: %s", q(tok))
		}
		return tok, nil
	case 'L', 'M':
		if depth+1 > MaxDepth {
			return "", fmt.Errorf("nesting deeper than %d", MaxDepth)
		}
		count, err := canonicalNat(body)
		if err != nil {
			return "", fmt.Errorf("%c count: %w", tag, err)
		}
		if tag == 'L' {
			var b strings.Builder
			b.WriteString("L[")
			for i := 0; i < count; i++ {
				k, err := r.value(depth + 1)
				if err != nil {
					return "", err
				}
				b.WriteString(k)
				b.WriteByte(',')
			}
			b.WriteByte(']')
			return b.String(), nil
		}
		keys := map[string]bool{}
		entries := make([]string, 0)
		for i := 0; i < count; i++ {
			k, err := r.value(depth + 1)
			if err != nil {
				return "", err
			}
			v, err := r.value(depth + 1)
			if err != nil {
				return "", err
			}
			if keys[k] {
				return "", errors.New("duplicate map key")
			}
			keys[k] = true
			entries = append(entries, "("+k+"=>"+v+")")
		}
		sort.Strings(entries)
		return "M{" + strings.Join(entries, "") + "}", nil
	}
	return "", fmt.Errorf("unknown tag in %s", q(tok))
}
