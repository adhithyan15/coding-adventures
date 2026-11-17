package trace

import (
	"bytes"
	"math/rand"
	"os"
	"reflect"
	"strconv"
	"strings"
	"testing"
)

// P2 PR 5 moved handleLineG from the ASTNode path (straceparser.ParseLineTokens
// and the ASTNode helpers) to the compact path (one reused LineParser per
// trace).  The move must change NOTHING: these tests run the production
// handleLineG against handleLineGLegacy (grammar_legacy_test.go, a frozen copy
// of the ASTNode version) and demand the same record, the same refusal (reason,
// line, wrapped error) and the same pending map, line by line.  The older
// differentials (linediff_test.go, seqdiff_test.go, the cross-check hook) still
// pin the grammar path to the old hand-written scanner.

// sameOutcome returns "" when the two results are identical.
func sameOutcome(ra *record, ea *Error, rb *record, eb *Error, pa, pb map[int]*pending) string {
	switch {
	case !reflect.DeepEqual(ea, eb):
		return "refusals differ: compact=" + errString(ea) + " legacy=" + errString(eb)
	case !reflect.DeepEqual(ra, rb):
		return "records differ"
	case !reflect.DeepEqual(pa, pb):
		return "pending state differs"
	}
	return ""
}

func errString(e *Error) string {
	if e == nil {
		return "<nil>"
	}
	return e.Reason + "@" + strconv.Itoa(e.Line)
}

// compactVsLegacy runs the lines through both implementations with one state
// each (so sequences with shared pending state are covered) and returns the
// first difference.
func compactVsLegacy(lines []string, helper bool, maxLine int) string {
	sc := &lineState{pend: map[int]*pending{}, helper: helper, maxLine: maxLine}
	sl := &lineState{pend: map[int]*pending{}, helper: helper, maxLine: maxLine}
	for i, l := range lines {
		rc, ec := handleLineG(l, i+1, sc)
		rl, el := handleLineGLegacy(l, i+1, sl)
		if d := sameOutcome(rc, ec, rl, el, sc.pend, sl.pend); d != "" {
			return "line " + strconv.Itoa(i+1) + ": " + d
		}
		if ec != nil { // a refusal ends a trace: the states are not used further
			return ""
		}
	}
	return ""
}

func TestCompactEqualsLegacyFixtures(t *testing.T) {
	for _, n := range []string{"build.strace", "cwd.strace", "paths.strace", "net.strace"} {
		b, err := os.ReadFile("testdata/" + n)
		if err != nil {
			t.Fatal(err)
		}
		lines := strings.Split(strings.TrimRight(string(b), "\n"), "\n")
		for _, helper := range []bool{false, true} {
			if d := compactVsLegacy(lines, helper, 0); d != "" {
				t.Errorf("%s helper=%v: %s", n, helper, d)
			}
		}
	}
	// Every fixture line alone, too (an empty state: the resumed and exit shapes
	// then take their refusal paths).
	for _, l := range fixtureLines(t) {
		if d := compactVsLegacy([]string{l}, false, 0); d != "" {
			t.Errorf("%q: %s", l, d)
		}
	}
}

func TestCompactEqualsLegacyLinesSeeded(t *testing.T) {
	corpus := fixtureLines(t)
	n := 300000
	if testing.Short() {
		n = 30000
	}
	r := rand.New(rand.NewSource(1234))
	bad := 0
	for i := 0; i < n; i++ {
		l := genLine(r, corpus)
		// every 7th line runs under a small line limit, to reach the size refusals
		max := 0
		if i%7 == 0 {
			max = 24 + r.Intn(64)
		}
		if d := compactVsLegacy([]string{l}, i%2 == 0, max); d != "" {
			if bad++; bad <= 10 {
				t.Errorf("%s: %q (max %d)", d, l, max)
			}
		}
	}
	if g := grammarPanics.Load(); g != 0 {
		t.Fatalf("%d recovered panics", g)
	}
}

func TestCompactEqualsLegacySeqSeeded(t *testing.T) {
	n := 600000
	if testing.Short() {
		n = 60000
	}
	r := rand.New(rand.NewSource(99))
	bad := 0
	for i := 0; i < n; i++ {
		seq := genSeq(r)
		if d := compactVsLegacy(seq, i%2 == 0, 0); d != "" {
			if bad++; bad <= 10 {
				t.Errorf("%s: %q", d, seq)
			}
		}
	}
	if g := grammarPanics.Load(); g != 0 {
		t.Fatalf("%d recovered panics", g)
	}
}

// A fuzz target for the same property, on top of the seeded runs.
func FuzzCompactEqualsLegacy(f *testing.F) {
	for _, l := range fixtureLines(f) {
		f.Add(l + "\n" + l)
	}
	for _, s := range []string{
		"1  f( , <unfinished ...>\n1  <... f resumed>) = 0",
		"1  wait4(   <unfinished ...>\n1  <... wait4 resumed>) = 0",
		"1  stat( <unfinished ...>\n1  <... stat resumed>\"a\", 0x1) = 0",
		"1  ???( <unfinished ...>\n1  <... ??? resumed>)  = ?",
	} {
		f.Add(s)
	}
	f.Fuzz(func(t *testing.T, in string) {
		lines := strings.Split(in, "\n")
		if len(lines) > 5 {
			lines = lines[:5]
		}
		for _, helper := range []bool{false, true} {
			if d := compactVsLegacy(lines, helper, 0); d != "" {
				t.Fatalf("%s: %q", d, lines)
			}
		}
	})
}

// The all-space argument guard (review H1): an argument item made only of
// spaces, including Unicode spaces such as U+00A0 that the grammar sees as
// ordinary text, must be refused or dropped exactly as the old scanner does,
// WHATEVER the parser says.  "1  f( , <unfinished ...>" is accepted by every
// parser path with the raw argument " "; kiln's trimArgs, not the parser, turns
// that into the refusal.  Each case is checked against the old scanner, the
// legacy grammar path and the compact one, and against the expected verdict.
func TestAllSpaceItemGuardOnCompactPath(t *testing.T) {
	nb := " "
	for _, c := range []struct {
		line   string
		accept bool
	}{
		{"1  f( , <unfinished ...>", false},
		{"1  f(" + nb + ", <unfinished ...>", false},
		{"1  f( " + nb + " , <unfinished ...>", false},
		{"1  openat( ,  <unfinished ...>", false},
		{"1  openat(" + nb + "," + nb + " <unfinished ...>", false},
		{"1  wait4(-1,  , <unfinished ...>", false},
		{"1  wait4(-1, " + nb + ", <unfinished ...>", false},
		{"1  wait4(-1,   <unfinished ...>", true},
		{"1  wait4(-1, " + nb + " <unfinished ...>", true},
		{"1  stat(" + nb + ") = 0", true},
		{"1  stat(a," + nb + ") = 0", false},
		{"1  stat(," + nb + ") = 0", false},
	} {
		so := &lineState{pend: map[int]*pending{}}
		sc := &lineState{pend: map[int]*pending{}}
		sl := &lineState{pend: map[int]*pending{}}
		ro, eo := handleLine(c.line, 1, so)
		rc, ec := handleLineG(c.line, 1, sc)
		rl, el := handleLineGLegacy(c.line, 1, sl)
		if d := sameOutcome(rc, ec, rl, el, sc.pend, sl.pend); d != "" {
			t.Errorf("%q: compact vs legacy: %s", c.line, d)
		}
		if (eo == nil) != c.accept || (ec == nil) != c.accept {
			t.Errorf("%q: old %s, compact %s, want accept=%v", c.line, errString(eo), errString(ec), c.accept)
		}
		if eo == nil && ec == nil && (!reflect.DeepEqual(ro, rc) || !reflect.DeepEqual(so.pend, sc.pend)) {
			t.Errorf("%q: old and compact records differ", c.line)
		}
		if eo != nil && ec != nil && eo.Reason != ec.Reason {
			t.Errorf("%q: reasons old=%s compact=%s", c.line, eo.Reason, ec.Reason)
		}
	}
	// The same through the resumed half: the glued line is parsed by the compact
	// path again, into the same reused LineParser.
	for _, tr := range [][]string{
		{"1  wait4( , <unfinished ...>", "1  <... wait4 resumed>) = 0"},
		{"1  wait4(-1,  <unfinished ...>", "1  <... wait4 resumed>" + nb + ", 0, NULL) = 2"},
	} {
		if d := compactVsLegacy(tr, false, 0); d != "" {
			t.Errorf("%q: %s", tr, d)
		}
	}
}

// ---- benchmark ----------------------------------------------------------

// benchCorpus is every fixture trace as pre-split lines, plus its byte size.
func benchCorpus(b *testing.B) (lines []string, bytesN int) {
	for _, n := range []string{"build.strace", "cwd.strace", "paths.strace", "net.strace"} {
		d, err := os.ReadFile("testdata/" + n)
		if err != nil {
			b.Fatal(err)
		}
		bytesN += len(d)
		lines = append(lines, strings.Split(strings.TrimRight(string(d), "\n"), "\n")...)
	}
	return
}

// BenchmarkLineHandler compares the three line readers on the same lines, with
// the same state machine and limits: the old hand-written scanner (handleLine),
// the grammar path on the ASTNode tree (handleLineGLegacy, the state before P2
// PR 5) and the grammar path on the compact LineParser (handleLineG).  Each
// iteration reads every line of the four fixture traces with one state (the
// pending map and, for the compact path, the LineParser live as long as one
// trace parse, as in production).  Run:
//
//	go test ./internal/trace -run '^$' -bench 'LineHandler|Pass1' -benchmem
func BenchmarkLineHandler(b *testing.B) {
	lines, size := benchCorpus(b)
	for _, c := range []struct {
		name string
		h    func(string, int, *lineState) (*record, *Error)
	}{
		{"old-scanner", handleLine},
		{"grammar-ast", handleLineGLegacy},
		{"grammar-compact", handleLineG},
	} {
		b.Run(c.name, func(b *testing.B) {
			b.SetBytes(int64(size))
			b.ReportAllocs()
			for i := 0; i < b.N; i++ {
				st := &lineState{pend: map[int]*pending{}, helper: true}
				for n, l := range lines {
					_, _ = c.h(l, n+1, st)
				}
			}
		})
	}
}

// BenchmarkPass1 is the production pass-1 reader (bufio, readLine, the handler)
// over the same bytes, old scanner against grammar path (compact since PR 5).
func BenchmarkPass1(b *testing.B) {
	var all []byte
	for _, n := range []string{"build.strace", "cwd.strace", "paths.strace", "net.strace"} {
		d, err := os.ReadFile("testdata/" + n)
		if err != nil {
			b.Fatal(err)
		}
		all = append(all, d...)
	}
	for _, grammar := range []bool{false, true} {
		name := "old-scanner"
		if grammar {
			name = "grammar-compact"
		}
		b.Run(name, func(b *testing.B) {
			b.SetBytes(int64(len(all)))
			b.ReportAllocs()
			for i := 0; i < b.N; i++ {
				_, _, _, _ = readRecordsAt(bytes.NewReader(all), DefaultLimits(), true, grammar)
			}
		})
	}
}
