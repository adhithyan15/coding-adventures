package commonmarkparser

import (
	"regexp"
	"strings"
	"unicode"
	"unicode/utf8"
)

// Scanner is a cursor-based scanner over a string. Used by both the block
// parser (to scan individual lines) and the inline parser (to scan inline
// content character by character).
//
// # Design
//
// The scanner maintains a position Pos into the string. All read operations
// advance Pos. The scanner never backtracks on its own — callers must save
// and restore Pos explicitly when lookahead fails.
//
// This is the same pattern used by hand-rolled recursive descent parsers:
// try to match; if it fails, restore the saved position.
//
//	saved := scanner.Pos
//	if !scanner.Match("```") {
//	    scanner.Pos = saved // backtrack
//	}
//
// # Character classification
//
// CommonMark cares about several Unicode character categories:
//   - ASCII punctuation: !"#$%&'()*+,-./:;<=>?@[\]^_`{|}~
//   - Unicode punctuation (for emphasis rules)
//   - ASCII whitespace: space, tab, CR, LF, FF
//   - Unicode whitespace
type Scanner struct {
	// Source is the original string being scanned.
	Source string
	// Pos is the current byte position in Source.
	Pos int
}

// NewScanner creates a new Scanner starting at position 0.
func NewScanner(source string) *Scanner {
	return &Scanner{Source: source, Pos: 0}
}

// Done returns true if the scanner has consumed all input.
func (s *Scanner) Done() bool {
	return s.Pos >= len(s.Source)
}

// Remaining returns the number of bytes remaining.
func (s *Scanner) Remaining() int {
	return len(s.Source) - s.Pos
}

// Peek returns the rune at the current position without advancing.
// Returns 0 (NUL) if the scanner is at the end of input.
func (s *Scanner) Peek() rune {
	if s.Pos >= len(s.Source) {
		return 0
	}
	r, _ := utf8.DecodeRuneInString(s.Source[s.Pos:])
	return r
}

// PeekByte returns the byte at the current position without advancing.
// Returns 0 if at end. For ASCII-only scanning this is faster than Peek.
func (s *Scanner) PeekByte() byte {
	if s.Pos >= len(s.Source) {
		return 0
	}
	return s.Source[s.Pos]
}

// PeekAt returns the rune at position Pos+offset (in runes, not bytes).
// This is used for single-character lookahead. Returns 0 at end.
// Note: for efficiency, offset should be 0 or 1 in practice.
func (s *Scanner) PeekAt(offset int) rune {
	pos := s.Pos
	for i := 0; i < offset; i++ {
		if pos >= len(s.Source) {
			return 0
		}
		_, size := utf8.DecodeRuneInString(s.Source[pos:])
		pos += size
	}
	if pos >= len(s.Source) {
		return 0
	}
	r, _ := utf8.DecodeRuneInString(s.Source[pos:])
	return r
}

// PeekByteAt returns the byte at Pos+offset (in bytes).
// For ASCII lookahead without needing rune decoding.
func (s *Scanner) PeekByteAt(offset int) byte {
	pos := s.Pos + offset
	if pos < 0 || pos >= len(s.Source) {
		return 0
	}
	return s.Source[pos]
}

// PeekSlice returns the next n bytes from the current position without advancing.
func (s *Scanner) PeekSlice(n int) string {
	end := s.Pos + n
	if end > len(s.Source) {
		end = len(s.Source)
	}
	return s.Source[s.Pos:end]
}

// Advance advances Pos by one rune and returns it.
// Returns 0 if at end of input.
func (s *Scanner) Advance() rune {
	if s.Pos >= len(s.Source) {
		return 0
	}
	r, size := utf8.DecodeRuneInString(s.Source[s.Pos:])
	s.Pos += size
	return r
}

// AdvanceByte advances Pos by one byte and returns it.
// Use only for ASCII characters.
func (s *Scanner) AdvanceByte() byte {
	if s.Pos >= len(s.Source) {
		return 0
	}
	b := s.Source[s.Pos]
	s.Pos++
	return b
}

// Skip advances Pos by n bytes.
func (s *Scanner) Skip(n int) {
	s.Pos += n
	if s.Pos > len(s.Source) {
		s.Pos = len(s.Source)
	}
}

// Match checks if the next bytes exactly match str. If so, advances past
// them and returns true. Otherwise leaves Pos unchanged and returns false.
func (s *Scanner) Match(str string) bool {
	if strings.HasPrefix(s.Source[s.Pos:], str) {
		s.Pos += len(str)
		return true
	}
	return false
}

// MatchRegex tries to match a regex anchored at the current position.
// On success, advances Pos and returns the matched string.
// On failure, returns "" and leaves Pos unchanged.
func (s *Scanner) MatchRegex(re *regexp.Regexp) string {
	rest := s.Source[s.Pos:]
	loc := re.FindStringIndex(rest)
	if loc == nil || loc[0] != 0 {
		return ""
	}
	matched := rest[loc[0]:loc[1]]
	s.Pos += len(matched)
	return matched
}

// ConsumeWhile advances while the predicate returns true for the current byte.
// Returns the consumed string.
func (s *Scanner) ConsumeWhile(pred func(byte) bool) string {
	start := s.Pos
	for s.Pos < len(s.Source) && pred(s.Source[s.Pos]) {
		s.Pos++
	}
	return s.Source[start:s.Pos]
}

// ConsumeWhileRune is like ConsumeWhile but uses rune-level predicate.
func (s *Scanner) ConsumeWhileRune(pred func(rune) bool) string {
	start := s.Pos
	for s.Pos < len(s.Source) {
		r, size := utf8.DecodeRuneInString(s.Source[s.Pos:])
		if !pred(r) {
			break
		}
		s.Pos += size
	}
	return s.Source[start:s.Pos]
}

// ConsumeLine consumes the rest of the line up to but not including the newline.
func (s *Scanner) ConsumeLine() string {
	start := s.Pos
	for s.Pos < len(s.Source) && s.Source[s.Pos] != '\n' {
		s.Pos++
	}
	return s.Source[start:s.Pos]
}

// Rest returns the rest of the input from the current position without advancing.
func (s *Scanner) Rest() string {
	return s.Source[s.Pos:]
}

// SliceFrom returns the source from start to the current position.
func (s *Scanner) SliceFrom(start int) string {
	return s.Source[start:s.Pos]
}

// SkipSpaces skips ASCII spaces and tabs. Returns the number of bytes skipped.
func (s *Scanner) SkipSpaces() int {
	start := s.Pos
	for s.Pos < len(s.Source) && (s.Source[s.Pos] == ' ' || s.Source[s.Pos] == '\t') {
		s.Pos++
	}
	return s.Pos - start
}

// CountIndent counts leading spaces/tabs without advancing, returning the
// virtual column count (tabs expand to the next 4-column tab stop).
func (s *Scanner) CountIndent() int {
	indent := 0
	i := s.Pos
	for i < len(s.Source) {
		ch := s.Source[i]
		if ch == ' ' {
			indent++
			i++
		} else if ch == '\t' {
			indent += 4 - (indent % 4)
			i++
		} else {
			break
		}
	}
	return indent
}

// SkipIndent advances past exactly n virtual spaces of indentation,
// expanding tabs to the next 4-space tab stop.
func (s *Scanner) SkipIndent(n int) {
	remaining := n
	for remaining > 0 && !s.Done() {
		ch := s.Source[s.Pos]
		if ch == ' ' {
			s.Pos++
			remaining--
		} else if ch == '\t' {
			tabWidth := 4 - (s.Pos % 4)
			if tabWidth <= remaining {
				s.Pos++
				remaining -= tabWidth
			} else {
				break // partial tab — don't consume
			}
		} else {
			break
		}
	}
}

// ─── Character Classification ─────────────────────────────────────────────────

// asciiPunctuation is the set of ASCII punctuation characters as defined by
// the CommonMark spec. Used in emphasis flanking rules.
//
// Exactly: ! " # $ % & ' ( ) * + , - . / : ; < = > ? @ [ \ ] ^ _ ` { | } ~
const asciiPunctuationChars = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"

// IsAsciiPunctuation returns true if ch is an ASCII punctuation character
// per the CommonMark definition.
func IsAsciiPunctuation(ch rune) bool {
	return ch < 128 && strings.ContainsRune(asciiPunctuationChars, ch)
}

// IsUnicodePunctuation returns true if ch is a Unicode punctuation character
// for CommonMark flanking rules.
//
// CommonMark defines this (per the cmark reference) as any ASCII punctuation
// character OR any character in Unicode categories:
//
//	Pc, Pd, Pe, Pf, Pi, Po, Ps (punctuation) or Sm, Sc, Sk, So (symbols).
//
// The symbol categories (S*) are included because cmark treats them as
// punctuation for delimiter flanking (e.g. £ U+00A3 Sc, € U+20AC Sc).
func IsUnicodePunctuation(ch rune) bool {
	if ch == 0 {
		return false
	}
	if IsAsciiPunctuation(ch) {
		return true
	}
	// Unicode punctuation categories (P*) and symbol categories (S*)
	return unicode.IsPunct(ch) || unicode.IsSymbol(ch)
}

// IsAsciiWhitespace returns true if ch is ASCII whitespace:
// space (U+0020), tab (U+0009), newline (U+000A), form feed (U+000C),
// carriage return (U+000D).
func IsAsciiWhitespace(ch rune) bool {
	return ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' || ch == '\f'
}

// IsUnicodeWhitespace returns true if ch is Unicode whitespace
// (any code point with Unicode property White_Space=yes).
func IsUnicodeWhitespace(ch rune) bool {
	if ch == 0 {
		return false
	}
	return unicode.IsSpace(ch) || ch == '\u00A0' || ch == '\u1680' ||
		(ch >= '\u2000' && ch <= '\u200A') || ch == '\u202F' ||
		ch == '\u205F' || ch == '\u3000'
}

// IsDigit returns true if ch is an ASCII digit (0-9).
func IsDigit(ch rune) bool {
	return ch >= '0' && ch <= '9'
}

// NormalizeLinkLabel normalizes a link label per CommonMark:
//   - Strip leading and trailing whitespace
//   - Collapse internal whitespace runs to a single space
//   - Fold to lowercase
//
// Two labels are equivalent if their normalized forms are equal.
//
// Per CommonMark §4.7: "ß" (U+00DF) and "ẞ" (U+1E9E) should both fold to
// "ss" in Unicode full case folding. Go's strings.ToLower doesn't do full
// case folding, so we handle these explicitly.
func NormalizeLinkLabel(label string) string {
	result := strings.TrimSpace(label)
	// Collapse internal whitespace
	result = whitespaceRun.ReplaceAllString(result, " ")
	result = strings.ToLower(result)
	// Unicode full case fold for ß (U+00DF) → "ss"
	result = strings.ReplaceAll(result, "ß", "ss")
	return result
}

var whitespaceRun = regexp.MustCompile(`\s+`)

// NormalizeURL percent-encodes spaces and characters that should not appear
// unencoded in HTML href/src attributes.
func NormalizeURL(url string) string {
	// Percent-encode characters that need encoding in HTML attributes
	// but are not already percent-encoded.
	// We encode the minimal set: control chars and space.
	var b strings.Builder
	b.Grow(len(url))
	for _, ch := range url {
		if shouldPercentEncode(ch) {
			// Percent-encode as UTF-8 bytes
			var buf [4]byte
			n := utf8.EncodeRune(buf[:], ch)
			for i := 0; i < n; i++ {
				b.WriteString(percentEncoded(buf[i]))
			}
		} else {
			b.WriteRune(ch)
		}
	}
	return b.String()
}

// shouldPercentEncode returns true for characters that need percent-encoding
// in URL attributes. We use the same character class as the TypeScript impl:
// encode everything NOT in [\w\-._~:/?#@!$&'()*+,;=%]
func shouldPercentEncode(ch rune) bool {
	if ch >= 'a' && ch <= 'z' {
		return false
	}
	if ch >= 'A' && ch <= 'Z' {
		return false
	}
	if ch >= '0' && ch <= '9' {
		return false
	}
	switch ch {
	case '-', '_', '.', '~', ':', '/', '?', '#', '@', '!', '$', '&',
		'\'', '(', ')', '*', '+', ',', ';', '=', '%':
		return false
	}
	return true
}

// percentEncoded returns the percent-encoded representation of a byte.
func percentEncoded(b byte) string {
	const hex = "0123456789ABCDEF"
	return "%" + string(hex[b>>4]) + string(hex[b&0xf])
}
