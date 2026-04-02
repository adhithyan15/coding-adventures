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
	result, _ := StartNew[*Scanner]("commonmark-parser.NewScanner", nil,
		func(op *Operation[*Scanner], rf *ResultFactory[*Scanner]) *OperationResult[*Scanner] {
			op.AddProperty("source_len", len(source))
			return rf.Generate(true, false, &Scanner{Source: source, Pos: 0})
		}).GetResult()
	return result
}

// Done returns true if the scanner has consumed all input.
func (s *Scanner) Done() bool {
	result, _ := StartNew[bool]("commonmark-parser.Scanner.Done", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, s.Pos >= len(s.Source))
		}).GetResult()
	return result
}

// Remaining returns the number of bytes remaining.
func (s *Scanner) Remaining() int {
	result, _ := StartNew[int]("commonmark-parser.Scanner.Remaining", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(s.Source)-s.Pos)
		}).GetResult()
	return result
}

// Peek returns the rune at the current position without advancing.
// Returns 0 (NUL) if the scanner is at the end of input.
func (s *Scanner) Peek() rune {
	result, _ := StartNew[rune]("commonmark-parser.Scanner.Peek", 0,
		func(op *Operation[rune], rf *ResultFactory[rune]) *OperationResult[rune] {
			if s.Pos >= len(s.Source) {
				return rf.Generate(true, false, 0)
			}
			r, _ := utf8.DecodeRuneInString(s.Source[s.Pos:])
			return rf.Generate(true, false, r)
		}).GetResult()
	return result
}

// PeekByte returns the byte at the current position without advancing.
// Returns 0 if at end. For ASCII-only scanning this is faster than Peek.
func (s *Scanner) PeekByte() byte {
	result, _ := StartNew[byte]("commonmark-parser.Scanner.PeekByte", 0,
		func(op *Operation[byte], rf *ResultFactory[byte]) *OperationResult[byte] {
			if s.Pos >= len(s.Source) {
				return rf.Generate(true, false, 0)
			}
			return rf.Generate(true, false, s.Source[s.Pos])
		}).GetResult()
	return result
}

// PeekAt returns the rune at position Pos+offset (in runes, not bytes).
// This is used for single-character lookahead. Returns 0 at end.
// Note: for efficiency, offset should be 0 or 1 in practice.
func (s *Scanner) PeekAt(offset int) rune {
	result, _ := StartNew[rune]("commonmark-parser.Scanner.PeekAt", 0,
		func(op *Operation[rune], rf *ResultFactory[rune]) *OperationResult[rune] {
			op.AddProperty("offset", offset)
			pos := s.Pos
			for i := 0; i < offset; i++ {
				if pos >= len(s.Source) {
					return rf.Generate(true, false, 0)
				}
				_, size := utf8.DecodeRuneInString(s.Source[pos:])
				pos += size
			}
			if pos >= len(s.Source) {
				return rf.Generate(true, false, 0)
			}
			r, _ := utf8.DecodeRuneInString(s.Source[pos:])
			return rf.Generate(true, false, r)
		}).GetResult()
	return result
}

// PeekByteAt returns the byte at Pos+offset (in bytes).
// For ASCII lookahead without needing rune decoding.
func (s *Scanner) PeekByteAt(offset int) byte {
	result, _ := StartNew[byte]("commonmark-parser.Scanner.PeekByteAt", 0,
		func(op *Operation[byte], rf *ResultFactory[byte]) *OperationResult[byte] {
			op.AddProperty("offset", offset)
			pos := s.Pos + offset
			if pos < 0 || pos >= len(s.Source) {
				return rf.Generate(true, false, 0)
			}
			return rf.Generate(true, false, s.Source[pos])
		}).GetResult()
	return result
}

// PeekSlice returns the next n bytes from the current position without advancing.
func (s *Scanner) PeekSlice(n int) string {
	result, _ := StartNew[string]("commonmark-parser.Scanner.PeekSlice", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("n", n)
			end := s.Pos + n
			if end > len(s.Source) {
				end = len(s.Source)
			}
			return rf.Generate(true, false, s.Source[s.Pos:end])
		}).GetResult()
	return result
}

// Advance advances Pos by one rune and returns it.
// Returns 0 if at end of input.
func (s *Scanner) Advance() rune {
	result, _ := StartNew[rune]("commonmark-parser.Scanner.Advance", 0,
		func(op *Operation[rune], rf *ResultFactory[rune]) *OperationResult[rune] {
			if s.Pos >= len(s.Source) {
				return rf.Generate(true, false, 0)
			}
			r, size := utf8.DecodeRuneInString(s.Source[s.Pos:])
			s.Pos += size
			return rf.Generate(true, false, r)
		}).GetResult()
	return result
}

// AdvanceByte advances Pos by one byte and returns it.
// Use only for ASCII characters.
func (s *Scanner) AdvanceByte() byte {
	result, _ := StartNew[byte]("commonmark-parser.Scanner.AdvanceByte", 0,
		func(op *Operation[byte], rf *ResultFactory[byte]) *OperationResult[byte] {
			if s.Pos >= len(s.Source) {
				return rf.Generate(true, false, 0)
			}
			b := s.Source[s.Pos]
			s.Pos++
			return rf.Generate(true, false, b)
		}).GetResult()
	return result
}

// Skip advances Pos by n bytes.
func (s *Scanner) Skip(n int) {
	_, _ = StartNew[struct{}]("commonmark-parser.Scanner.Skip", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("n", n)
			s.Pos += n
			if s.Pos > len(s.Source) {
				s.Pos = len(s.Source)
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Match checks if the next bytes exactly match str. If so, advances past
// them and returns true. Otherwise leaves Pos unchanged and returns false.
func (s *Scanner) Match(str string) bool {
	result, _ := StartNew[bool]("commonmark-parser.Scanner.Match", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("str", str)
			if strings.HasPrefix(s.Source[s.Pos:], str) {
				s.Pos += len(str)
				return rf.Generate(true, false, true)
			}
			return rf.Generate(true, false, false)
		}).GetResult()
	return result
}

// MatchRegex tries to match a regex anchored at the current position.
// On success, advances Pos and returns the matched string.
// On failure, returns "" and leaves Pos unchanged.
func (s *Scanner) MatchRegex(re *regexp.Regexp) string {
	result, _ := StartNew[string]("commonmark-parser.Scanner.MatchRegex", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			rest := s.Source[s.Pos:]
			loc := re.FindStringIndex(rest)
			if loc == nil || loc[0] != 0 {
				return rf.Generate(true, false, "")
			}
			matched := rest[loc[0]:loc[1]]
			s.Pos += len(matched)
			return rf.Generate(true, false, matched)
		}).GetResult()
	return result
}

// ConsumeWhile advances while the predicate returns true for the current byte.
// Returns the consumed string.
func (s *Scanner) ConsumeWhile(pred func(byte) bool) string {
	result, _ := StartNew[string]("commonmark-parser.Scanner.ConsumeWhile", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			start := s.Pos
			for s.Pos < len(s.Source) && pred(s.Source[s.Pos]) {
				s.Pos++
			}
			return rf.Generate(true, false, s.Source[start:s.Pos])
		}).GetResult()
	return result
}

// ConsumeWhileRune is like ConsumeWhile but uses rune-level predicate.
func (s *Scanner) ConsumeWhileRune(pred func(rune) bool) string {
	result, _ := StartNew[string]("commonmark-parser.Scanner.ConsumeWhileRune", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			start := s.Pos
			for s.Pos < len(s.Source) {
				r, size := utf8.DecodeRuneInString(s.Source[s.Pos:])
				if !pred(r) {
					break
				}
				s.Pos += size
			}
			return rf.Generate(true, false, s.Source[start:s.Pos])
		}).GetResult()
	return result
}

// ConsumeLine consumes the rest of the line up to but not including the newline.
func (s *Scanner) ConsumeLine() string {
	result, _ := StartNew[string]("commonmark-parser.Scanner.ConsumeLine", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			start := s.Pos
			for s.Pos < len(s.Source) && s.Source[s.Pos] != '\n' {
				s.Pos++
			}
			return rf.Generate(true, false, s.Source[start:s.Pos])
		}).GetResult()
	return result
}

// Rest returns the rest of the input from the current position without advancing.
func (s *Scanner) Rest() string {
	result, _ := StartNew[string]("commonmark-parser.Scanner.Rest", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, s.Source[s.Pos:])
		}).GetResult()
	return result
}

// SliceFrom returns the source from start to the current position.
func (s *Scanner) SliceFrom(start int) string {
	result, _ := StartNew[string]("commonmark-parser.Scanner.SliceFrom", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("start", start)
			return rf.Generate(true, false, s.Source[start:s.Pos])
		}).GetResult()
	return result
}

// SkipSpaces skips ASCII spaces and tabs. Returns the number of bytes skipped.
func (s *Scanner) SkipSpaces() int {
	result, _ := StartNew[int]("commonmark-parser.Scanner.SkipSpaces", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			start := s.Pos
			for s.Pos < len(s.Source) && (s.Source[s.Pos] == ' ' || s.Source[s.Pos] == '\t') {
				s.Pos++
			}
			return rf.Generate(true, false, s.Pos-start)
		}).GetResult()
	return result
}

// CountIndent counts leading spaces/tabs without advancing, returning the
// virtual column count (tabs expand to the next 4-column tab stop).
func (s *Scanner) CountIndent() int {
	result, _ := StartNew[int]("commonmark-parser.Scanner.CountIndent", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
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
			return rf.Generate(true, false, indent)
		}).GetResult()
	return result
}

// SkipIndent advances past exactly n virtual spaces of indentation,
// expanding tabs to the next 4-space tab stop.
func (s *Scanner) SkipIndent(n int) {
	_, _ = StartNew[struct{}]("commonmark-parser.Scanner.SkipIndent", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("n", n)
			remaining := n
			for remaining > 0 && s.Pos < len(s.Source) {
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
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
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
	result, _ := StartNew[bool]("commonmark-parser.IsAsciiPunctuation", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, ch < 128 && strings.ContainsRune(asciiPunctuationChars, ch))
		}).GetResult()
	return result
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
	result, _ := StartNew[bool]("commonmark-parser.IsUnicodePunctuation", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			if ch == 0 {
				return rf.Generate(true, false, false)
			}
			if ch < 128 && strings.ContainsRune(asciiPunctuationChars, ch) {
				return rf.Generate(true, false, true)
			}
			// Unicode punctuation categories (P*) and symbol categories (S*)
			return rf.Generate(true, false, unicode.IsPunct(ch) || unicode.IsSymbol(ch))
		}).GetResult()
	return result
}

// IsAsciiWhitespace returns true if ch is ASCII whitespace:
// space (U+0020), tab (U+0009), newline (U+000A), form feed (U+000C),
// carriage return (U+000D).
func IsAsciiWhitespace(ch rune) bool {
	result, _ := StartNew[bool]("commonmark-parser.IsAsciiWhitespace", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' || ch == '\f')
		}).GetResult()
	return result
}

// IsUnicodeWhitespace returns true if ch is Unicode whitespace
// (any code point with Unicode property White_Space=yes).
func IsUnicodeWhitespace(ch rune) bool {
	result, _ := StartNew[bool]("commonmark-parser.IsUnicodeWhitespace", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			if ch == 0 {
				return rf.Generate(true, false, false)
			}
			return rf.Generate(true, false, unicode.IsSpace(ch) || ch == '\u00A0' || ch == '\u1680' ||
				(ch >= '\u2000' && ch <= '\u200A') || ch == '\u202F' ||
				ch == '\u205F' || ch == '\u3000')
		}).GetResult()
	return result
}

// IsDigit returns true if ch is an ASCII digit (0-9).
func IsDigit(ch rune) bool {
	result, _ := StartNew[bool]("commonmark-parser.IsDigit", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, ch >= '0' && ch <= '9')
		}).GetResult()
	return result
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
	result, _ := StartNew[string]("commonmark-parser.NormalizeLinkLabel", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("label", label)
			r := strings.TrimSpace(label)
			// Collapse internal whitespace
			r = whitespaceRun.ReplaceAllString(r, " ")
			r = strings.ToLower(r)
			// Unicode full case fold for ß (U+00DF) → "ss"
			r = strings.ReplaceAll(r, "ß", "ss")
			return rf.Generate(true, false, r)
		}).GetResult()
	return result
}

var whitespaceRun = regexp.MustCompile(`\s+`)

// NormalizeURL percent-encodes spaces and characters that should not appear
// unencoded in HTML href/src attributes.
func NormalizeURL(url string) string {
	result, _ := StartNew[string]("commonmark-parser.NormalizeURL", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("url", url)
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
			return rf.Generate(true, false, b.String())
		}).GetResult()
	return result
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
