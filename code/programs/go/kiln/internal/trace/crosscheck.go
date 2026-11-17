package trace

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"reflect"
	"strconv"
	"strings"
	"sync"
	"unicode"

	stracelexer "github.com/adhithyan15/coding-adventures/code/packages/go/strace-lexer"
	straceparser "github.com/adhithyan15/coding-adventures/code/packages/go/strace-parser"
)

// This file is the differential check of D24 (KILN04 section 4.9).
//
// While the hand-written scanner is the default and the grammar path is on
// probation, a hook in ParseOptions can run BOTH on the same input and refuse to
// go on when they disagree.  Two things turn it on:
//
//   - this package's tests (a _test.go init sets crossCheck), which makes every
//     existing Parse test a differential with no edit to any of them;
//   - KILN_TRACE_DIFF=1 in the environment, which the kiln-sandbox CI job sets,
//     so every trace the real strace produced is cross-checked.
//
// # The rule
//
// The old scanner is the oracle, and the policy is spec section 4.9:
//
//  1. the old parser refused and the new one accepted: ALWAYS a failure (this is
//     the false cache hit this whole check exists to catch);
//  2. both accepted: the two Traces must be deeply equal;
//  3. both refused: the Reason and the Line must be equal, unless the pair is a
//     reason-only divergence on the spec's list (see stricterRows);
//  4. the old parser accepted and the new one refused: allowed only for a line
//     shape on the spec's list of stricter-only divergences.
//
// A mismatch PANICS with a *CrossCheckMismatch: loud, attributed to the test
// that was running, and impossible to mistake for a legitimate trace-incomplete
// refusal.  With KILN_TRACE_DIFF a bounded, sanitised window of the lines around
// the mismatch is printed to standard error first.

// crossCheck, when non-nil, replaces the normal body of ParseOptions.  It is
// unexported on purpose: only this package's tests and the environment switch
// below set it.
var crossCheck func(r io.Reader, initialCwd string, opts Options) (*Trace, error)

func init() {
	if os.Getenv("KILN_TRACE_DIFF") == "1" {
		crossCheck = parseCrossChecked
	}
}

// CrossCheckMismatch is what a failed cross-check panics with.
type CrossCheckMismatch struct {
	Detail string // what differs, without any path text
	Window string // the sanitised lines around the first difference ("" when none)
}

func (m *CrossCheckMismatch) Error() string {
	s := "kiln trace cross-check: the grammar path and the old scanner disagree: " + m.Detail
	if m.Window != "" {
		s += "\n" + m.Window
	}
	return s
}

// AllowedDivergences is how many stricter-only divergences the cross-check has
// allowed in this process.  On real strace output it must stay 0: a stricter-only
// hit there means the default flip would void that recording.
func AllowedDivergences() int {
	allowedMu.Lock()
	defer allowedMu.Unlock()
	n := 0
	for _, c := range allowedStats {
		n += c
	}
	return n
}

// allowedStats counts, per row of the spec's table, the divergences that were
// allowed.  Tests read it to report what a differential run found.
var (
	allowedMu    sync.Mutex
	allowedStats = map[string]int{}
)

func noteAllowed(row string) {
	allowedMu.Lock()
	allowedStats[row]++
	allowedMu.Unlock()
}

// parseCrossChecked runs the old scanner and the grammar path on one input and
// returns the old scanner's answer, or panics when the two disagree in a way
// the spec does not allow.
func parseCrossChecked(r io.Reader, initialCwd string, opts Options) (*Trace, error) {
	src := &replaySource{src: r}
	trOld, errOld := parseWith(src.cursor(), initialCwd, opts, false)
	before := grammarPanics.Load()
	trNew, errNew := parseWith(src.cursor(), initialCwd, opts, true)
	panicked := grammarPanics.Load() != before
	if detail, line := compareParses(src.buf, opts, panicked, trOld, errOld, trNew, errNew); detail != "" {
		m := &CrossCheckMismatch{Detail: detail, Window: sanitisedWindow(src.buf, line)}
		if os.Getenv("KILN_TRACE_DIFF") == "1" {
			fmt.Fprintln(os.Stderr, m.Error())
		}
		panic(m)
	}
	return trOld, errOld
}

// replaySource lets two parses read the same stream: it records what the first
// reader consumed, and gives it back to the second before reading on.  (The old
// scanner stops reading at its first refusal, so a plain copy of what it read
// would be a prefix the other parser could mistake for the whole input.)
type replaySource struct {
	src io.Reader
	buf []byte
	err error // sticky: the source's terminal error, replayed to every cursor
}

func (s *replaySource) cursor() io.Reader { return &replayCursor{s: s} }

type replayCursor struct {
	s   *replaySource
	off int
}

func (c *replayCursor) Read(p []byte) (int, error) {
	s := c.s
	if c.off < len(s.buf) {
		n := copy(p, s.buf[c.off:])
		c.off += n
		return n, nil
	}
	if s.err != nil {
		return 0, s.err
	}
	n, err := s.src.Read(p)
	s.buf = append(s.buf, p[:n]...)
	c.off += n
	if err != nil {
		s.err = err
	}
	if n > 0 {
		return n, nil
	}
	return 0, err
}

// errorOf extracts the *Error of a parse result (nil when it succeeded).
func errorOf(err error) *Error {
	if err == nil {
		return nil
	}
	if e, ok := err.(*Error); ok {
		return e
	}
	return &Error{Reason: "non-trace-error:" + err.Error()}
}

// compareParses applies the rule above.  It returns "" when the results agree
// or differ only as the spec allows, else what differs and the line to show.
//
// Where a refusal happened is not taken from the reason's name.  The line reader
// (pass 1) is run on its own with each implementation, and readRecordsAt says
// exactly how far each got.  If the old pass 1 refused at line L and the grammar
// path read past L, that is FAIL-OPEN whatever the reasons say.
func compareParses(data []byte, opts Options, panicked bool, trOld *Trace, errOld error, trNew *Trace, errNew error) (string, int) {
	eo, en := errorOf(errOld), errorOf(errNew)
	lim := opts.Limits.withDefaults()
	maxLine := lim.MaxLineBytes
	if panicked {
		// A recovered panic in the grammar path is a bug, never a divergence the
		// spec lists, whatever line shape it happened on.
		return "the grammar path panicked (recovered)", 0
	}
	switch {
	case eo == nil && en == nil:
		if !reflect.DeepEqual(trOld, trNew) {
			return "both accepted, but the Traces differ" + traceDiff(trOld, trNew), firstDiffLine(trOld, trNew)
		}
		return "", 0
	case eo != nil && en == nil:
		return "FAIL-OPEN: the old scanner refused (" + eo.Reason + ", line " + strconv.Itoa(eo.Line) + ") and the grammar path accepted", eo.Line
	}
	// At least the grammar path refused.  Find how far each pass 1 got.
	_, _, e1, a1 := readRecordsAt(bytes.NewReader(data), lim, opts.HelperRoot, false)
	_, _, e2, a2 := readRecordsAt(bytes.NewReader(data), lim, opts.HelperRoot, true)
	if eo != nil && eo.Reason == en.Reason && eo.Line == en.Line {
		return "", 0
	}
	switch {
	case e1 != nil && (e2 == nil || a2 > a1):
		return "FAIL-OPEN: the old scanner refused at line " + strconv.Itoa(a1) + " (" + e1.Reason + "); the grammar path read past it (" + en.Reason + " at line " + strconv.Itoa(en.Line) + ")", a1
	case e1 != nil && e2 != nil:
		old := e1.Reason
		if a2 < a1 {
			old = "" // the old scanner accepted the line the grammar path refused
		}
		if row := stricterRow(lineAt(data, a2), old, e2.Reason, data, a2, maxLine); row != "" {
			noteAllowed(row)
			return "", 0
		}
	case e1 == nil && e2 != nil:
		// The old scanner read every line; the grammar path refused one (and the
		// old scanner refused later, in the model, or not at all).
		if row := stricterRow(lineAt(data, a2), "", e2.Reason, data, a2, maxLine); row != "" {
			noteAllowed(row)
			return "", 0
		}
	}
	old := "accepted"
	if eo != nil {
		old = eo.Reason + " at line " + strconv.Itoa(eo.Line)
	}
	return "the grammar path refused (" + en.Reason + " at line " + strconv.Itoa(en.Line) + ") where the old scanner " + old + ", outside the spec's stricter-only list", en.Line
}

// traceDiff names the top-level fields of two Traces that differ.
func traceDiff(a, b *Trace) string {
	if a == nil || b == nil {
		return " (one is nil)"
	}
	var names []string
	va, vb := reflect.ValueOf(*a), reflect.ValueOf(*b)
	for i := 0; i < va.NumField(); i++ {
		if !reflect.DeepEqual(va.Field(i).Interface(), vb.Field(i).Interface()) {
			names = append(names, va.Type().Field(i).Name)
		}
	}
	return ": fields " + strings.Join(names, ",")
}

// firstDiffLine is the line of the first event that differs, or 0.
func firstDiffLine(a, b *Trace) int {
	if a == nil || b == nil {
		return 0
	}
	for i := 0; i < len(a.Events) && i < len(b.Events); i++ {
		if !reflect.DeepEqual(a.Events[i], b.Events[i]) {
			return a.Events[i].Line
		}
	}
	return 0
}

// lineAt returns the 1-based line n of data, without its newline ("" when
// there is none).
func lineAt(data []byte, n int) string {
	if n <= 0 {
		return ""
	}
	for i := 1; len(data) > 0; i++ {
		k := bytes.IndexByte(data, '\n')
		var l []byte
		if k < 0 {
			l, data = data, nil
		} else {
			l, data = data[:k], data[k+1:]
		}
		if i == n {
			return string(l)
		}
	}
	return ""
}

// sanitisedWindow returns up to two lines either side of line n, each cut to 160
// bytes and printed as Go-quoted text (so no raw control character, no
// workflow-command line start, no unbounded output).  Real traces hold paths a
// pull request chose, so this is only ever printed on an explicit KILN_TRACE_DIFF
// run, for the lines around a mismatch.
func sanitisedWindow(data []byte, n int) string {
	if n <= 0 {
		return ""
	}
	var b strings.Builder
	for i := max(1, n-2); i <= n+2; i++ {
		l := lineAt(data, i)
		if l == "" && i > n {
			break
		}
		if len(l) > 160 {
			l = l[:160] + "..."
		}
		mark := "  "
		if i == n {
			mark = "> "
		}
		fmt.Fprintf(&b, "  %s%d: %s\n", mark, i, strconv.QuoteToASCII(l))
	}
	return strings.TrimRight(b.String(), "\n")
}

// ---------------------------------------------------------------------------
// The spec's list of stricter-only divergences (KILN04 section 4.9)
// ---------------------------------------------------------------------------

// lineFacts are the properties of one line the rows below are written in terms
// of.  They are computed with the grammar's own lexer (so they are exact about
// strings and annotations) plus a few byte tests.
type lineFacts struct {
	tokens           int    // tokens including the EOF
	depth            int    // deepest bracket nesting (a nested call counts)
	errTok           string // the first ERR_* token's type, or ""
	errAmbig         bool   // ... and it is an ambiguous-path kind
	badAnnot         bool   // an annotation whose escapes do not decode
	badQuoted        bool   // a quoted string whose escapes do not decode
	marker           bool   // the text "<unfinished ...>" occurs
	resumed          bool   // the body is a "<... " line
	signedPid        bool   // "[pid +N]"
	signedEx         bool   // "+++ exited with +N +++"
	unicodeSpaceExit bool   // an exit line holding a non-ASCII space (strings.TrimSpace removed it)
	trailSp          bool   // ends with a space
	overArity        bool   // a call with more top-level items than its arity cap, counted before the first error token
}

func factsOf(line string) lineFacts {
	var f lineFacts
	f.trailSp = strings.HasSuffix(line, " ")
	f.marker = strings.Contains(line, "<unfinished ...>")
	f.signedPid = strings.HasPrefix(line, "[pid +")
	if _, body, ok := parsePid(line); ok {
		f.resumed = strings.HasPrefix(body, "<... ")
		if strings.HasPrefix(body, "+++ ") {
			for _, r := range body {
				if r > 0x7f && unicode.IsSpace(r) {
					f.unicodeSpaceExit = true
				}
			}
		}
		f.signedEx = strings.HasPrefix(body, "+++ ") && (strings.Contains(body, "exited with +") || strings.Contains(body, "exited with -"))
	}
	toks, err := stracelexer.TokenizeLineMax(line, 1<<30)
	if err != nil {
		return f
	}
	f.tokens = len(toks)
	depth := 0
	items, countItems, callName := 0, true, ""
	for ti, t := range toks {
		if ti == 1 && t.TypeName == "SYSCALL" {
			callName = strings.TrimSuffix(t.Value, "(")
			items = 1
		}
		if stracelexer.IsErrToken(t) {
			countItems = false
		}
		if countItems && callName != "" && t.TypeName == "COMMA" && depth == 1 {
			items++
		}
		switch t.TypeName {
		case "LPAREN", "LBRACKET", "LBRACE", "SYSCALL", "FUNC":
			depth++
			f.depth = max(f.depth, depth)
		case "RPAREN", "RBRACKET", "RBRACE":
			depth--
		case "ANNOTATED":
			if strings.IndexByte(t.Value, '\\') >= 0 {
				_, body := stracelexer.AnnotationBody(t)
				if _, e := stracelexer.DecodeAnnotation(body); e != nil {
					f.badAnnot = true
				}
			}
		case "QUOTED":
			if strings.IndexByte(t.Value, '\\') >= 0 {
				body, _ := stracelexer.QuotedBody(t)
				if _, e := stracelexer.DecodeString(body); e != nil {
					f.badQuoted = true
				}
			}
		}
		if f.errTok == "" && stracelexer.IsErrToken(t) {
			f.errTok, f.errAmbig = t.TypeName, stracelexer.Ambiguous(t)
		}
	}
	if callName != "" {
		limit := maxUnknownItems
		if ent := lookup(callName); ent.class == classHandled {
			limit = ent.max
		}
		f.overArity = items > limit
	}
	return f
}

// stricterRow says whether (oldReason, newReason) on this line is one of the
// divergences the spec lists, and names its row.  oldReason "" means the old
// scanner accepted the line.  It is deliberately narrow: each row names the
// shape of the line, not just the pair of reasons.  "New accepts, old refuses"
// never reaches here.
func stricterRow(line, oldReason, newReason string, data []byte, lineNo, maxLine int) string {
	f := factsOf(line)
	// A refusal by name (unknown-syscall, refused) comes AFTER the old scanner
	// accepted the line's syntax, so for the syntax rules it counts as accepted.
	oldOK := oldReason == "" || strings.HasPrefix(oldReason, "unknown-syscall:") || strings.HasPrefix(oldReason, "refused:") ||
		// State refusals: the old scanner checks a line's syntax BEFORE it looks
		// at the pending calls (interleaved-call, and the unresumed-call an exit
		// line raises), so such a refusal means it accepted the syntax.
		oldReason == reasonInterleaved || oldReason == reasonUnresumed
	switch {
	case newReason == reasonSize && (f.tokens > maxLineTokens || f.depth > straceparser.MaxBracketNesting || gluedTooBig(data, lineNo, maxLine, f)):
		return "caps: depth 64, 65,536 tokens, or a glued call over the byte limit"
	case newReason == reasonUnparsable && oldOK && f.signedPid:
		return "signed pid"
	case newReason == reasonUnparsable && oldOK && f.unicodeSpaceExit:
		return "Unicode space inside an exit line"
	case newReason == reasonUnparsable && oldOK && f.signedEx:
		return "signed exit code"
	case newReason == reasonUnparsable && f.trailSp:
		return "trailing space"
	case newReason == reasonUnparsable && f.resumed && (f.errTok == "ERR_LT" || f.errTok == "ERR_QUOTE") && (oldOK || oldReason == reasonUnparsable):
		return "resumed half spans the seam"
	case newReason == reasonUnparsable && oldOK && (f.errTok == "ERR_LT" || f.marker) && strings.Contains(line, "PID NS */"):
		// The old result parser took an annotation after the pid-namespace note
		// (`= 5 /* 9 in strace's PID NS */<x>`), a bare "<" no word precedes.
		// strace prints the annotation first; the grammar sees an ERR_LT.
		return "annotation after the pid-namespace note"
	case newReason == reasonAmbiguous && oldOK && f.badQuoted:
		return "strict quoted string"
	case newReason == reasonUnparsable && (oldReason == reasonResult || oldReason == reasonAmbiguous) && f.marker:
		return "bare <unfinished ...> after the close"
	case (newReason == reasonUnparsable || newReason == reasonAmbiguous) && (oldReason == reasonResult || oldReason == reasonAmbiguous) && (f.errTok != "" || f.badAnnot || f.badQuoted):
		return "ERR token in the text after the close"
	case newReason == reasonUnparsable && oldReason == reasonUnmatched && f.resumed && f.errTok == "ERR_RESUME":
		return "malformed resume marker"
	case oldReason == reasonUnmatched && f.resumed && (newReason == reasonUnparsable || newReason == reasonAmbiguous):
		// The grammar path reads the line before it looks at the pending call,
		// so a resumed line that is itself malformed AND has no matching pending
		// call is a syntax refusal where the old scanner said unmatched-resume.
		return "malformed resumed half with no matching pending call"
	case newReason == reasonAmbiguous && oldReason == reasonUnparsable && (f.badAnnot || f.badQuoted || (f.errTok != "" && f.errAmbig)):
		return "structural error and a later bad annotation or string"
	case (newReason == reasonUnparsable || newReason == reasonAmbiguous) && oldReason == reasonArity && (f.overArity || glueOverArity(data, lineNo, f)):
		return "too many arguments and a later error"
	}
	return ""
}

const reasonAmbiguous = reasonAmbiguousPath

// gluedTooBig reports whether the line is a resumed half whose glue with its
// pending prefix exceeds a cap the grammar path enforces.  It rebuilds the
// prefix from the earlier lines of data, the way the pending map did.
func gluedTooBig(data []byte, lineNo, maxLine int, f lineFacts) bool {
	cur := gluedText(data, lineNo, f)
	if cur == "" {
		return false
	}
	if len(cur) > maxLine {
		return true
	}
	g := factsOf("1  " + cur)
	return g.tokens > maxLineTokens || g.depth > straceparser.MaxBracketNesting
}

// gluedText rebuilds, for a resumed line, the call the old scanner glued: the
// pending prefix of the same pid plus the text after "resumed>".  It is "" for
// any other line.
func gluedText(data []byte, lineNo int, f lineFacts) string {
	if !f.resumed || lineNo <= 0 {
		return ""
	}
	last := map[int]string{} // pid -> pending prefix
	var cur string
	for i, rest := 1, data; len(rest) > 0 && i <= lineNo; i++ {
		k := bytes.IndexByte(rest, '\n')
		var l string
		if k < 0 {
			l, rest = string(rest), nil
		} else {
			l, rest = string(rest[:k]), rest[k+1:]
		}
		pid, body, ok := parsePid(l)
		if !ok {
			continue
		}
		if i == lineNo {
			if k := strings.Index(body, " resumed>"); k >= 0 {
				cur = last[pid] + body[k+len(" resumed>"):]
			}
			break
		}
		if strings.HasSuffix(body, " <unfinished ...>") || strings.HasSuffix(body, "<unfinished ...>") {
			j := strings.LastIndex(body, "<unfinished ...>")
			last[pid] = strings.TrimSuffix(body[:j], " ")
		} else {
			delete(last, pid)
		}
	}
	return cur
}

// glueOverArity: a resumed line has no call name of its own, so the arity of the
// call it completes is counted on the glued text.
func glueOverArity(data []byte, lineNo int, f lineFacts) bool {
	g := gluedText(data, lineNo, f)
	return g != "" && factsOf("1  "+g).overArity
}
