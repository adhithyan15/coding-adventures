package trace

import (
	"strings"
	"unicode/utf8"
)

// This file is the scanner: the part of the parser that understands the
// *characters* of a strace line.  There are no regular expressions here.  A
// regular expression for a line like
//
//	openat(AT_FDCWD, "/tmp/a\", O_RDONLY) = 3</x>", O_RDONLY) = 3</tmp/a">
//
// would have to guess where the quoted string ends; a scanner simply walks the
// bytes and keeps a little state: "am I inside a string?", "am I inside a -y
// annotation?", "how deep are my brackets?".
//
// # The three kinds of quoted text
//
//   - A STRING is "..." with C-style escapes.  strace prints a path argument
//     this way.  If it was cut off by -s the closing quote is followed by
//     three dots: "/very/long/pa"...
//   - An ANNOTATION is <...> glued to a file descriptor by -y, for example
//     3</etc/passwd> or AT_FDCWD</home/u>.  The kernel's own resolved path is
//     inside.  strace escapes the body like a string and additionally escapes
//     '<' and '>' (as \< and \>), so a path containing them round-trips.
//   - Everything else (flags, numbers, {structs}, [arrays], htons(80)) is
//     plain text that only needs its brackets balanced.
//
// # Fail closed
//
// A construct the scanner cannot place unambiguously is an error, never a
// guess.  Both callers rely on that: a wrong path is a wrong cache key.

// decodeEscape decodes the escape whose backslash is at s[i].  extra lists
// additional characters that may be escaped (annotations also escape < and >).
func decodeEscape(s string, i int, extra string) (byte, int, bool) {
	if i+1 >= len(s) {
		return 0, 0, false
	}
	c := s[i+1]
	switch c {
	case '"', '\\':
		return c, i + 2, true
	case 'n':
		return '\n', i + 2, true
	case 't':
		return '\t', i + 2, true
	case 'r':
		return '\r', i + 2, true
	case 'a':
		return 7, i + 2, true
	case 'b':
		return 8, i + 2, true
	case 'f':
		return 12, i + 2, true
	case 'v':
		return 11, i + 2, true
	case 'x':
		// \xHH: exactly two hex digits (strace -x style).
		if i+3 >= len(s) {
			return 0, 0, false
		}
		h, ok1 := hexVal(s[i+2])
		l, ok2 := hexVal(s[i+3])
		if !ok1 || !ok2 {
			return 0, 0, false
		}
		return h<<4 | l, i + 4, true
	}
	if c >= '0' && c <= '7' {
		// \ooo: one to three octal digits, greedy, value must fit a byte.
		v, j := 0, i+1
		for n := 0; n < 3 && j < len(s) && s[j] >= '0' && s[j] <= '7'; n++ {
			v = v*8 + int(s[j]-'0')
			j++
		}
		if v > 255 {
			return 0, 0, false
		}
		return byte(v), j, true
	}
	if extra != "" && strings.IndexByte(extra, c) >= 0 {
		return c, i + 2, true
	}
	return 0, 0, false
}

func hexVal(c byte) (byte, bool) {
	switch {
	case c >= '0' && c <= '9':
		return c - '0', true
	case c >= 'a' && c <= 'f':
		return c - 'a' + 10, true
	case c >= 'A' && c <= 'F':
		return c - 'A' + 10, true
	}
	return 0, false
}

// decodeString strictly decodes the quoted string opening at s[i] == '"'.
// It returns the bytes, whether the string was truncated (a "..." follows the
// closing quote), and the index just past everything it consumed.
//
//	"a\tb\"c"   ->  a<TAB>b"c
//	"\303\251"  ->  é            (octal bytes)
//	"/etc/x"... ->  /etc/x, truncated
func decodeString(s string, i int) (val []byte, trunc bool, end int, reason string) {
	var out []byte
	j := i + 1
	for j < len(s) {
		c := s[j]
		switch {
		case c == '"':
			j++
			if strings.HasPrefix(s[j:], "...") {
				return out, true, j + 3, ""
			}
			return out, false, j, ""
		case c == '\\':
			b, nj, ok := decodeEscape(s, j, "")
			if !ok {
				return nil, false, 0, reasonAmbiguousPath
			}
			out = append(out, b)
			j = nj
		case c < 0x20 || c == 0x7f:
			return nil, false, 0, reasonAmbiguousPath
		default:
			out = append(out, c)
			j++
		}
	}
	return nil, false, 0, reasonUnparsable
}

// skipString finds the end of the string opening at s[i] without decoding it.
// It is lenient about escapes (only "\x" pairs are skipped) because it is used
// for strings we do not interpret, such as read buffers.
func skipString(s string, i int) (int, bool) {
	j := i + 1
	for j < len(s) {
		switch s[j] {
		case '\\':
			j += 2
		case '"':
			j++
			if strings.HasPrefix(s[j:], "...") {
				j += 3
			}
			return j, true
		default:
			j++
		}
	}
	return 0, false
}

// scanAnnotation decodes the -y annotation opening at s[i] == '<'.  The body
// ends at the first unescaped '>'.  An unescaped '<' inside is ambiguous (a
// real strace escapes it), so it is refused.  It returns the body bytes and the
// index just past the closing '>'.
func scanAnnotation(s string, i int) (body []byte, end int, reason string) {
	j := i + 1
	for j < len(s) {
		c := s[j]
		switch {
		case c == '>':
			return body, j + 1, ""
		case c == '<':
			return nil, 0, reasonAmbiguousPath
		case c == '\\':
			b, nj, ok := decodeEscape(s, j, "<>")
			if !ok {
				return nil, 0, reasonAmbiguousPath
			}
			body = append(body, b)
			j = nj
		case c < 0x20 || c == 0x7f:
			return nil, 0, reasonAmbiguousPath
		default:
			body = append(body, c)
			j++
		}
	}
	return nil, 0, reasonUnparsable
}

func isTokenChar(c byte) bool {
	return c == '_' || c == '-' || (c >= '0' && c <= '9') || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')
}

const unfinishedMarker = "<unfinished ...>"

// scanResult is what scanItems found.
type scanResult struct {
	items      []string // top-level comma-separated items, trimmed
	end        int      // index of the closing ')' (closed), or of the marker (unfinished)
	closed     bool
	unfinished bool
}

// scanItems walks s from i splitting on top-level commas.  Brackets nest
// ( ) [ ] { }; strings and annotations are skipped as units so a comma, paren
// or '>' inside them never counts.  With stopAtParen it stops at the ')' that
// closes the call (the caller has consumed the opening one); it also stops at
// the "<unfinished ...>" marker, which strace prints when a call is
// interrupted by another process's output.
//
// maxItems bounds the number of top-level items: a line of a million commas
// must not become a million-element slice.  Exceeding it is an arity error.
//
// A '<' is an annotation only when it immediately follows a token character
// (3<, AT_FDCWD<).  The unfinished marker is always preceded by a space or a
// comma, so a path that merely contains the text "<unfinished ...>" can never
// be mistaken for it.
func scanItems(s string, i int, stopAtParen bool, maxItems int) (scanResult, string) {
	var res scanResult
	var stack []byte
	itemStart := i
	flush := func(end int, closing bool) string {
		item := strings.TrimSpace(s[itemStart:end])
		if item == "" {
			if closing && len(res.items) == 0 {
				return "" // f() has no arguments
			}
			return reasonUnparsable
		}
		if len(res.items) >= maxItems {
			return reasonArity // stop before an attacker-sized slice is built
		}
		res.items = append(res.items, item)
		return ""
	}
	j := i
	for j < len(s) {
		c := s[j]
		switch c {
		case '"':
			e, ok := skipString(s, j)
			if !ok {
				return res, reasonUnparsable
			}
			j = e
			continue
		case '<':
			if j > i && isTokenChar(s[j-1]) {
				_, e, r := scanAnnotation(s, j)
				if r != "" {
					return res, r
				}
				j = e
				continue
			}
			if len(stack) == 0 && stopAtParen && s[j:] == unfinishedMarker {
				if strings.TrimSpace(s[itemStart:j]) != "" {
					if r := flush(j, false); r != "" {
						return res, r
					}
				}
				res.unfinished = true
				res.end = j
				return res, ""
			}
			return res, reasonUnparsable
		case '(', '[', '{':
			stack = append(stack, c)
		case ')', ']', '}':
			if len(stack) == 0 {
				if c == ')' && stopAtParen {
					if r := flush(j, true); r != "" {
						return res, r
					}
					res.end, res.closed = j, true
					return res, ""
				}
				return res, reasonUnparsable
			}
			open := stack[len(stack)-1]
			if (c == ')' && open != '(') || (c == ']' && open != '[') || (c == '}' && open != '{') {
				return res, reasonUnparsable
			}
			stack = stack[:len(stack)-1]
		case ',':
			if len(stack) == 0 {
				if r := flush(j, false); r != "" {
					return res, r
				}
				itemStart = j + 1
			}
		}
		j++
	}
	if stopAtParen || len(stack) != 0 {
		return res, reasonUnparsable
	}
	if r := flush(len(s), true); r != "" {
		return res, r
	}
	return res, ""
}

// pathBytes validates decoded path bytes: no NUL, valid UTF-8.
func pathBytes(b []byte) (string, string) {
	for _, c := range b {
		if c == 0 {
			return "", reasonAmbiguousPath
		}
	}
	if !utf8.Valid(b) {
		return "", reasonNonUTF8
	}
	return string(b), ""
}

// pathArg interprets one raw argument as a path string.  The argument must be
// exactly one quoted string: a pointer such as 0x7ffd... (a bad address) is
// not a path we can use.
func pathArg(raw string) (string, string) {
	if len(raw) == 0 || raw[0] != '"' {
		return "", reasonPathArg
	}
	val, trunc, end, r := decodeString(raw, 0)
	if r != "" {
		return "", r
	}
	if trunc {
		return "", reasonTruncated
	}
	if end != len(raw) {
		return "", reasonPathArg
	}
	return pathBytes(val)
}

// fdRef is a parsed descriptor argument such as 3</etc> or AT_FDCWD.
type fdRef struct {
	cwd    bool
	num    int
	hasAnn bool
	ann    []byte
}

// parseFD interprets one raw argument as "AT_FDCWD" or a number, optionally
// followed by a -y annotation.  An annotation on AT_FDCWD is parsed and kept
// but callers never rely on it (KILN04 §5.1: the parser tracks cwd itself).
func parseFD(raw string) (fdRef, string) {
	var f fdRef
	j := 0
	if strings.HasPrefix(raw, "AT_FDCWD") {
		f.cwd = true
		j = len("AT_FDCWD")
	} else {
		k := j
		for k < len(raw) && raw[k] >= '0' && raw[k] <= '9' {
			k++
		}
		if k == j || k-j > 9 {
			return f, reasonDirfd
		}
		for _, c := range raw[j:k] {
			f.num = f.num*10 + int(c-'0')
		}
		j = k
	}
	if j == len(raw) {
		return f, ""
	}
	if raw[j] != '<' {
		return f, reasonDirfd
	}
	body, end, r := scanAnnotation(raw, j)
	if r != "" {
		return f, r
	}
	if end != len(raw) {
		// text after the closing '>': the path itself contained a raw '>'.
		return f, reasonAmbiguousPath
	}
	f.hasAnn, f.ann = true, body
	return f, ""
}

// fdPathText turns an annotation body into a usable path.  An annotation that
// is not an absolute filesystem path (a socket, a pipe) is an error where a
// directory is needed.
func fdPathText(ann []byte, requireAbs bool) (string, string) {
	s, r := pathBytes(ann)
	if r != "" {
		return "", r
	}
	if strings.HasSuffix(s, " (deleted)") {
		return "", reasonDeleted
	}
	if requireAbs && !strings.HasPrefix(s, "/") {
		return "", reasonFDPath
	}
	return s, ""
}
