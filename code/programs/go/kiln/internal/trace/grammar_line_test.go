package trace

import (
	"strings"
	"testing"

	stracelexer "github.com/adhithyan15/coding-adventures/code/packages/go/strace-lexer"
)

// The grammar path's own tests: the lexer's properties kiln relies on, the
// strictness of the old interpreters over the grammar's raw spans, and the
// divergences of KILN04 section 4.9 pinned one by one.

// The G1 reviews' requirements: the strace grammar is case-sensitive (a
// case-insensitive lexer lowercases the source and re-reads values at the same
// offsets, which ToLower can shift) and every rule runs on the fast byte
// matchers (a regexp rule costs time that grows with the rest of the line, per
// token: a denial of service on a huge trace).
func TestStraceGrammarsAreCaseSensitiveAndFast(t *testing.T) {
	if !stracelexer.TokenGrammarData.CaseSensitive {
		t.Error("strace.tokens must be case sensitive")
	}
	if !stracelexer.StringTokenGrammarData.CaseSensitive {
		t.Error("strace-string.tokens must be case sensitive")
	}
	c, err := stracelexer.Compiled()
	if err != nil {
		t.Fatal(err)
	}
	if s := c.SlowRules(); len(s) != 0 {
		t.Errorf("strace.tokens has rules on the regexp engine: %v", s)
	}
	if s := stracelexer.SlowRules(); len(s) != 0 {
		t.Errorf("SlowRules: %v", s)
	}
}

// reasonOfLine runs one line through handleLineG with an empty state.
func reasonOfLine(t *testing.T, line string) string {
	t.Helper()
	_, e := handleLineG(line, 1, &lineState{pend: map[int]*pending{}})
	if e == nil {
		return ""
	}
	return e.Reason
}

// Spaces are invisible to the grammar, so these arrive as ONE raw span of
// several pieces.  The old interpreters must refuse each of them.  (Flags such
// as `O_RDONLY O_CLOEXEC` are accepted by the old scanner too, and the
// cross-check holds the two to the same Trace.)
func TestInterpretersStayStrictOverMultiPieceArguments(t *testing.T) {
	for _, c := range []struct{ name, line, want string }{
		{"two strings are not a path", `1  stat("a""b", 0x1) = 0`, "path-arg"},
		{"two strings with a space", `1  stat("a" "b", 0x1) = 0`, "path-arg"},
		{"two annotations on one fd", `1  openat(3</a>x</b>, "p", O_RDONLY) = 3</p>`, ""},
		{"embedded NUL in a path", `1  stat("a\0b", 0x1) = 0`, "ambiguous-path"},
	} {
		t.Run(c.name, func(t *testing.T) {
			// Both implementations through the whole parser (the model is where
			// the interpreters run); the cross-check hook compares them too.
			text := c.line + "\n1  +++ exited with 0 +++\n"
			_, err := Parse(strings.NewReader(text), "/w", Limits{})
			if err == nil {
				t.Fatalf("accepted: %q", c.line)
			}
			if c.want != "" {
				if e, ok := err.(*Error); !ok || e.Reason != c.want {
					t.Errorf("reason = %v, want %s", err, c.want)
				}
			}
		})
	}
}

// Depth 64 parses, depth 65 is refused with reason size (KILN04 section 4.9:
// the first row of the divergence table).
func TestDepthCap(t *testing.T) {
	nest := func(n int) string { return "1  f(" + strings.Repeat("[", n) + strings.Repeat("]", n) + ") = 0" }
	if r := reasonOfLine(t, nest(63)); r != "unknown-syscall:f" {
		t.Errorf("depth 63 (+ the call): %q", r)
	}
	if r := reasonOfLine(t, nest(70)); r != reasonSize {
		t.Errorf("depth 70: %q, want size", r)
	}
	// The old scanner takes any depth (its stack is on the heap).
	_, e := handleLine(nest(70), 1, &lineState{pend: map[int]*pending{}})
	if e == nil || e.Reason != "unknown-syscall:f" {
		t.Errorf("old scanner at depth 70: %v", e)
	}
}

// More than 65,536 tokens on a line is refused with reason size (second row).
func TestTokenCap(t *testing.T) {
	long := "1  f(" + strings.Repeat("a,", 40000) + "a) = 0" // 80,001 tokens
	if r := reasonOfLine(t, long); r != reasonSize {
		t.Errorf("80k tokens: %q, want size", r)
	}
	if _, e := handleLine(long, 1, &lineState{pend: map[int]*pending{}}); e == nil || e.Reason != "arity" && !strings.HasPrefix(e.Reason, "unknown-syscall") {
		t.Errorf("old scanner: %v", e)
	}
	// A signal line counts too: every run of spaces in its text is a token.
	sig := "1  --- " + strings.Repeat("a ", 40000) + "---"
	if r := reasonOfLine(t, sig); r != reasonSize {
		t.Errorf("signal line with 80k tokens: %q, want size", r)
	}
	if _, e := handleLine(sig, 1, &lineState{pend: map[int]*pending{}}); e != nil {
		t.Errorf("old scanner refused the signal line: %v", e)
	}
}

// A split call whose glued text is over the line limit is refused (size), though
// each half is under it: the grammar path reads the glued call as one line.
func TestGluedCallOverTheByteLimit(t *testing.T) {
	st := &lineState{pend: map[int]*pending{}, maxLine: 1 << 12}
	half := strings.Repeat("a", 3000)
	if _, e := handleLineG("1  stat("+half+" <unfinished ...>", 1, st); e != nil {
		t.Fatal(e)
	}
	_, e := handleLineG("1  <... stat resumed>"+half+") = 0", 2, st)
	if e == nil || e.Reason != reasonSize {
		t.Fatalf("glued call over the limit: %v", e)
	}
	// The old scanner accepts the same pair.
	so := &lineState{pend: map[int]*pending{}, maxLine: 1 << 12}
	handleLine("1  stat("+half+" <unfinished ...>", 1, so)
	if _, e := handleLine("1  <... stat resumed>"+half+") = 0", 2, so); e != nil && e.Reason == reasonSize {
		t.Errorf("old scanner refused with size: %v", e)
	}
}

// The reason-only and stricter-only rows added by PR 5, one test each (each row
// of the table in KILN04 section 4.9 names its pinning test).
func TestPR5Divergences(t *testing.T) {
	for _, c := range []struct{ name, line, newReason string }{
		{"annotation after the pid-namespace note",
			"1  clone(child_stack=NULL) = 2 /* 9 in strace's PID NS */<x>", reasonUnparsable},
		{"unicode space inside an exit line", "1  +++ exited with 0  +++", reasonUnparsable},
		{"a trailing space on a failing result", "1  stat(\"a\", 0x1) = x ", reasonUnparsable},
	} {
		t.Run(c.name, func(t *testing.T) {
			if got := reasonOfLine(t, c.line); got != c.newReason {
				t.Errorf("grammar path: %q, want %s", got, c.newReason)
			}
			if d, row := lineVerdict(c.line); d != "" || row == "" {
				t.Errorf("verdict %q row %q", d, row)
			}
		})
	}
	// A resumed line that is itself malformed, with nothing pending: the old
	// scanner said unmatched-resume, the grammar path reads the line first.
	line := "1  <... f resumed>\"x"
	if got := reasonOfLine(t, line); got != reasonUnparsable {
		t.Errorf("grammar path: %q", got)
	}
	if _, e := handleLine(line, 1, &lineState{pend: map[int]*pending{}}); e == nil || e.Reason != reasonUnmatched {
		t.Errorf("old scanner: %v", e)
	}
	if d, row := lineVerdict(line); d != "" || row == "" {
		t.Errorf("verdict %q row %q", d, row)
	}
}

// New accepting where the old refused must fail the cross-check, loudly.
func TestCrossCheckCatchesFailOpen(t *testing.T) {
	d, _ := compareParses(nil, 1<<20, nil, &Error{Reason: reasonUnparsable, Line: 3}, nil, nil)
	if !strings.Contains(d, "FAIL-OPEN") {
		t.Errorf("fail-open not flagged: %q", d)
	}
	// Both accepted but different: flagged.
	d, _ = compareParses(nil, 1<<20, &Trace{RootPid: 1}, nil, &Trace{RootPid: 2}, nil)
	if d == "" {
		t.Error("differing traces not flagged")
	}
	// The same refusal is fine; a different reason outside the table is not.
	if d, _ := compareParses(nil, 1<<20, nil, &Error{Reason: "arity", Line: 2}, nil, &Error{Reason: "arity", Line: 2}); d != "" {
		t.Error(d)
	}
	if d, _ := compareParses([]byte("1  f(a) = 0\n"), 1<<20, nil, &Error{Reason: "arity", Line: 1}, nil, &Error{Reason: "path-arg", Line: 1}); d == "" {
		t.Error("an unlisted reason change was allowed")
	}
}

// A mismatch panics with a bounded, sanitised window.
func TestCrossCheckMismatchWindowIsSanitised(t *testing.T) {
	data := []byte("1  a\n1  b\x1b[31m::error::x\n" + strings.Repeat("z", 500) + "\n")
	w := sanitisedWindow(data, 2)
	if strings.ContainsAny(w, "\x1b") || strings.Contains(w, "::error::x\n") {
		t.Errorf("window not sanitised: %q", w)
	}
	for _, l := range strings.Split(w, "\n") {
		if len(l) > 200 {
			t.Errorf("window line too long: %d", len(l))
		}
	}
}
