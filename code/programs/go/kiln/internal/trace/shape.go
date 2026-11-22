package trace

import (
	"bufio"
	"io"
	"strconv"
	"strings"

	stracelexer "github.com/adhithyan15/coding-adventures/code/packages/go/strace-lexer"
)

// Shape: the REDACTED outline of one trace line (KILN04, census "line shape")
// ===========================================================================
//
// A census that says "the grammar refused line 8,113,402" tells nobody what to
// fix.  Diagnose (measure.go) would say, but it QUOTES the trace, and a real
// trace holds file names a pull request chose, so it must never run on a real
// recording (TestMeasureIsNotUsedByProductionCode).  Shape is the other way to
// answer "what did the refused line look like?": it prints the line's token
// sequence and nothing that was IN the tokens.
//
//	1234  openat(AT_FDCWD</w>, "/etc/passwd", O_RDONLY|O_CLOEXEC) = 3</etc/passwd>
//	  ->  PID SYSCALL:openat ANNOTATED COMMA QUOTED COMMA WORD:upper PUNCT[|] WORD:upper
//	      RPAREN PUNCT[=] WORD:digits ANNOTATED
//
// What may appear in the output, and why each is safe:
//
//	token type names   a closed list written in this file (the token names of
//	                   code/grammars/strace/strace.tokens); a name not on it is
//	                   printed as "TOKEN?", so even a grammar change cannot let
//	                   free text out through a type name
//	WORD / ASSIGN      a CHARACTER CLASS only: digits, hex, upper, lower, mixed.
//	                   Never the characters.
//	SYSCALL / RESUMED  the call's name, only when it is a member of the parser's
//	                   closed syscall table (SyscallClass); otherwise the type
//	                   alone.  The table is ours, so it cannot carry a secret.
//	WORD that is itself such a name: the same rule.
//	PUNCT              the ASCII punctuation, literally, at most 8 characters of
//	                   a run (the lexer's PUNCT also swallows every non-ASCII
//	                   byte; those print as "\x80" and no more)
//	QUOTED, ANNOTATED  the TYPE NAME ONLY.  These two token kinds are where
//	                   paths, file contents and environment values live.
//	ERR_*, PID, SIG_*  the type name only: they swallow whole tails of lines.
//	lineFacts          the booleans of crosscheck.go (is it resumed? does it end
//	                   with a space? ...) and three counts, none of which is text.
//
// What never appears: a byte of any token's text beyond the rules above.  The
// hostile-input tests put a secret-looking string in every token kind and
// assert that none of its bytes can be found in the result.
//
// Cost and bounds: the trace is read as a STREAM, never held; a line is read
// up to maxShapeLine bytes (a longer one is shaped from its prefix and marked
// truncated); the shape holds at most MaxShapeTokens tokens.  Two streaming
// passes at most: one to the line, one to find the pending half of its pid.

// MaxShapeTokens caps the tokens printed for one line.
const MaxShapeTokens = 96

// maxShapeLine is how much of one line is read.
const maxShapeLine = 1 << 20

// maxShapePunct is how many characters of one PUNCT run are printed.
const maxShapePunct = 8

// LineShape is the outline of one line.
type LineShape struct {
	// Line is the 1-based line number.
	Line int `json:"line"`
	// Role says why this line is shown: "stop" (where the grammar stopped) or
	// "pending" (the same pid's `<unfinished ...>` half, which a resumed line
	// completes).
	Role string `json:"role"`
	// Tokens is the token outline (see the file comment).
	Tokens string `json:"tokens"`
	// Facts are the lineFacts that are true, as fixed words.
	Facts string `json:"facts"`
	// Truncated says the line was longer than maxShapeLine, or the outline was
	// cut at MaxShapeTokens.
	Truncated bool `json:"truncated"`
}

// shapeTokenNames is the closed list of token type names of
// code/grammars/strace/strace.tokens (plus EOF, which is not printed).
var shapeTokenNames = map[string]bool{
	"UNFINISHED": true, "SYSCALL": true, "FUNC": true, "ANNOTATED": true, "ERR_ANNOT_BAD": true,
	"ERR_ANNOT_OPEN": true, "QUOTED": true, "ERR_QUOTE": true, "ASSIGN": true, "WORD": true,
	"COMMA": true, "LPAREN": true, "RPAREN": true, "LBRACKET": true, "RBRACKET": true,
	"LBRACE": true, "RBRACE": true, "PUNCT": true, "ERR_LT": true, "ERR_CTRL": true, "ERR_ANY": true,
	"PID_TAG": true, "PID": true, "ERR_PID": true, "DEAD_RESUMED": true, "RESUMED": true,
	"ERR_RESUME": true, "EXITED_LINE": true, "KILLED_LINE": true, "KILLED_CORE_LINE": true,
	"UNAVAILABLE": true, "SIG_OPEN": true, "DEAD_OPEN": true, "DEAD_WHOLE": true, "SIG_SP": true, "SIG_TEXT": true,
}

// Shape returns the outline of line n (1-based) of the trace open() yields,
// and, when line n belongs to a pid whose earlier line ended in
// `<unfinished ...>`, the outline of that earlier line first.  It returns nil
// when the line cannot be found or the trace cannot be read.  open is called at
// most twice, and its reader closed each time.
//
// ONLY `kiln record census` may call this (TestShapeOnlyInCensus): see the
// file comment for why it is safe there and Diagnose is not.
func Shape(open func() (io.ReadCloser, error), n int) []LineShape {
	if n <= 0 {
		return nil
	}
	text, trunc, ok := shapeLineAt(open, n)
	if !ok {
		return nil
	}
	var out []LineShape
	pid, _, hasPid := parsePid(text)
	if hasPid {
		if pt, pn, ptrunc := shapePending(open, n, pid); pn > 0 {
			out = append(out, shapeOf(pt, pn, "pending", ptrunc))
		}
	}
	return append(out, shapeOf(text, n, "stop", trunc))
}

// shapeLineAt reads to line n and returns its text (at most maxShapeLine
// bytes).  ok is false when the trace has fewer lines or cannot be opened.
func shapeLineAt(open func() (io.ReadCloser, error), n int) (text string, truncated, ok bool) {
	rc, err := open()
	if err != nil {
		return "", false, false
	}
	defer rc.Close()
	br := bufio.NewReaderSize(rc, 64<<10)
	for i := 1; ; i++ {
		l, tr, more := shapeReadLine(br)
		if !more {
			return "", false, false
		}
		if i == n {
			return l, tr, true
		}
	}
}

// shapePending finds, before line n, the last line of pid and returns it when
// it ends in `<unfinished ...>` (the half a resumed line completes).  pn is 0
// when there is none.
func shapePending(open func() (io.ReadCloser, error), n, pid int) (text string, pn int, truncated bool) {
	rc, err := open()
	if err != nil {
		return "", 0, false
	}
	defer rc.Close()
	br := bufio.NewReaderSize(rc, 64<<10)
	var last string
	var lastN int
	var lastTrunc bool
	for i := 1; i < n; i++ {
		l, tr, more := shapeReadLine(br)
		if !more {
			break
		}
		if p, _, found := parsePid(l); found && p == pid {
			last, lastN, lastTrunc = l, i, tr
		}
	}
	if lastN > 0 && strings.Contains(last, "<unfinished ...>") {
		return last, lastN, lastTrunc
	}
	return "", 0, false
}

// shapeReadLine reads one line, keeping at most maxShapeLine bytes of it and
// discarding the rest.  more is false at the end of the input (a final line
// without a newline is still returned, with more true, once).
func shapeReadLine(br *bufio.Reader) (line string, truncated, more bool) {
	var buf []byte
	got := false
	total := 0 // bytes of the line seen so far, newline excluded
	for {
		frag, err := br.ReadSlice('\n')
		if len(frag) > 0 {
			got = true
			body := strings.TrimSuffix(string(frag), "\n")
			total += len(body)
			if room := maxShapeLine - len(buf); room > 0 {
				take := body
				if len(take) > room {
					take = take[:room]
				}
				buf = append(buf, take...)
			}
			if total > maxShapeLine {
				truncated = true
			}
		}
		if err == bufio.ErrBufferFull {
			continue
		}
		if err != nil {
			if !got {
				return "", false, false
			}
			break
		}
		break
	}
	return string(buf), truncated, true
}

// shapeOf builds the outline of one line.
func shapeOf(line string, n int, role string, truncated bool) LineShape {
	sh := LineShape{Line: n, Role: role, Truncated: truncated}
	toks, err := stracelexer.TokenizeLineMax(line, 1<<30)
	if err != nil {
		// A raw newline cannot occur (the line was split on it); any other lexer
		// failure leaves nothing to outline.
		sh.Tokens = "(not lexable)"
		sh.Facts = shapeFacts(factsOf(line))
		return sh
	}
	var b strings.Builder
	printed := 0
	for i, t := range toks {
		if t.TypeName == "EOF" || (t.TypeName == "" && i == len(toks)-1) {
			continue
		}
		if printed >= MaxShapeTokens {
			b.WriteString(" +" + strconv.Itoa(len(toks)-i) + " more")
			sh.Truncated = true
			break
		}
		if printed > 0 {
			b.WriteByte(' ')
		}
		b.WriteString(shapeToken(t.TypeName, t.Value))
		printed++
	}
	sh.Tokens = b.String()
	sh.Facts = shapeFacts(factsOf(line))
	return sh
}

// shapeToken is the one place a token's text is looked at.  Everything it
// returns is built from the closed list, a class name, a table member or ASCII
// punctuation; none of it is a slice of value.
func shapeToken(typeName, value string) string {
	if !shapeTokenNames[typeName] {
		return "TOKEN?"
	}
	switch typeName {
	case "WORD":
		if tableName(value) {
			return "WORD:" + value
		}
		return "WORD:" + wordClass(value)
	case "ASSIGN":
		return "ASSIGN:" + wordClass(strings.TrimSuffix(value, "="))
	case "SYSCALL":
		if name := strings.TrimSuffix(value, "("); tableName(name) {
			return "SYSCALL:" + name
		}
		return "SYSCALL"
	case "RESUMED":
		// "<... NAME resumed>"
		name := strings.TrimSuffix(strings.TrimPrefix(value, "<... "), " resumed>")
		if tableName(name) {
			return "RESUMED:" + name
		}
		return "RESUMED"
	case "PUNCT":
		return "PUNCT[" + punctShape(value) + "]"
	}
	return typeName
}

// tableName says s is a member of the parser's closed syscall table.
func tableName(s string) bool { return s != "" && SyscallClass(s) != "" }

// wordClass summarises a run of word characters without quoting it.
//
//	digits  [-]0-9...          a number, a pid, an errno count
//	hex     [-]0x...           an address or a mask
//	upper   A-Z 0-9 _ -        a flag or a constant (O_RDONLY), at least one letter
//	lower   a-z 0-9 _ -        a name, at least one letter
//	mixed   anything else
func wordClass(s string) string {
	t := strings.TrimPrefix(s, "-")
	if t == "" {
		return "mixed"
	}
	if isAll(t, func(c byte) bool { return c >= '0' && c <= '9' }) {
		return "digits"
	}
	if h, ok := strings.CutPrefix(t, "0x"); ok && h != "" && isAll(h, func(c byte) bool {
		return c >= '0' && c <= '9' || c >= 'a' && c <= 'f' || c >= 'A' && c <= 'F'
	}) {
		return "hex"
	}
	letters, up, low := 0, 0, 0
	for i := 0; i < len(s); i++ {
		switch c := s[i]; {
		case c >= 'A' && c <= 'Z':
			letters++
			up++
		case c >= 'a' && c <= 'z':
			letters++
			low++
		case c >= '0' && c <= '9' || c == '_' || c == '-':
		default:
			return "mixed"
		}
	}
	switch {
	case letters > 0 && low == 0:
		return "upper"
	case letters > 0 && up == 0:
		return "lower"
	}
	return "mixed"
}

func isAll(s string, f func(byte) bool) bool {
	for i := 0; i < len(s); i++ {
		if !f(s[i]) {
			return false
		}
	}
	return true
}

// punctShape prints a PUNCT run: ASCII punctuation literally, any other byte as
// a single "\x80" marker per run of such bytes, at most maxShapePunct output
// characters in all.
func punctShape(s string) string {
	var b strings.Builder
	n := 0
	prevHigh := false
	for i := 0; i < len(s); i++ {
		if n >= maxShapePunct {
			b.WriteString("+")
			break
		}
		c := s[i]
		if c >= 0x21 && c <= 0x7e {
			b.WriteByte(c)
			n++
			prevHigh = false
			continue
		}
		if !prevHigh {
			b.WriteString(`\x80`)
			n++
		}
		prevHigh = true
	}
	return b.String()
}

// shapeFacts prints the true lineFacts as fixed words, and the counts.
func shapeFacts(f lineFacts) string {
	parts := []string{"tokens=" + strconv.Itoa(f.tokens), "depth=" + strconv.Itoa(f.depth)}
	if shapeTokenNames[f.errTok] {
		parts = append(parts, "err="+f.errTok)
	}
	for _, p := range []struct {
		on   bool
		name string
	}{
		{f.errAmbig, "err-ambiguous"}, {f.badAnnot, "bad-annotation"}, {f.badQuoted, "bad-quoted"},
		{f.marker, "unfinished-marker"}, {f.resumed, "resumed"}, {f.signedPid, "signed-pid"},
		{f.signedEx, "signed-exit"}, {f.unicodeSpaceExit, "unicode-space-exit"},
		{f.trailSp, "trailing-space"}, {f.overArity, "over-arity"},
	} {
		if p.on {
			parts = append(parts, p.name)
		}
	}
	return strings.Join(parts, " ")
}
