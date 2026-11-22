package trace

import (
	"math/rand"
	"reflect"
	"strconv"
	"strings"
	"testing"
)

// The stateful differential: SEQUENCES of lines with shared pending state.  The
// line differential starts every line from an empty state, so it cannot see a
// divergence that needs an earlier line (a call left open, a killed-at-entry
// call, an interleaved pid).  This one runs both implementations over the same
// 1 to 4 lines, with one state each, and compares records, pending state and
// reasons line by line.
//
// There is NO allow-list in the fail-open direction: the old scanner refusing
// and the grammar path accepting is always a failure.  A stricter-only refusal
// by the grammar path must be on the spec's list, and ends the sequence (the two
// states have parted).

// seqVerdict returns "" when the sequence agrees, else what differs.  failOpen
// reports whether the problem is the forbidden direction.
func seqVerdict(lines []string, helper bool) (problem string, failOpen bool, rows []string) {
	so := &lineState{pend: map[int]*pending{}, helper: helper}
	sn := &lineState{pend: map[int]*pending{}, helper: helper}
	data := []byte(strings.Join(lines, "\n") + "\n")
	max := DefaultLimits().MaxLineBytes
	for i, l := range lines {
		n := i + 1
		ro, eo := handleLine(l, n, so)
		before := grammarPanics.Load()
		rn, en := handleLineG(l, n, sn)
		if grammarPanics.Load() != before {
			return "recovered panic at line " + strconv.Itoa(n), false, rows
		}
		switch {
		case eo != nil && en == nil:
			return "FAIL-OPEN at line " + strconv.Itoa(n) + ": old " + eo.Reason, true, rows
		case eo == nil && en != nil:
			if r := stricterRow(l, "", en.Reason, data, n, max); r != "" {
				return "", false, append(rows, r)
			}
			return "stricter outside the list at line " + strconv.Itoa(n) + ": new " + en.Reason, false, rows
		case eo != nil:
			if eo.Reason == en.Reason && eo.Line == en.Line {
				return "", false, rows
			}
			if r := stricterRow(l, eo.Reason, en.Reason, data, n, max); r != "" && eo.Line == en.Line {
				return "", false, append(rows, r)
			}
			// An unresumed-call is reported at the pending call's own line: the old
			// scanner accepted THIS line's syntax and refused its effect.
			if r := stricterRow(l, "", en.Reason, data, n, max); r != "" && eo.Reason == reasonUnresumed {
				return "", false, append(rows, r)
			}
			return "reasons differ at line " + strconv.Itoa(n) + ": old " + eo.Reason + "@" + strconv.Itoa(eo.Line) + " new " + en.Reason + "@" + strconv.Itoa(en.Line), false, rows
		}
		if !reflect.DeepEqual(ro, rn) || !reflect.DeepEqual(so.pend, sn.pend) {
			return "records or pending state differ at line " + strconv.Itoa(n), false, rows
		}
	}
	// End of input: the same pending calls must be left.
	return "", false, rows
}

var seqNames = []string{"stat", "wait4", "openat", "exit_group", "exit", "f", "execve", "clone"}
var seqArgs = []string{"", "a", "-1", "\"/x\"", "AT_FDCWD, \"/x\", O_RDONLY", " ", " ", "a,  ", " ,", "a,", ",", "a,,b", "3</x>", "[", "]", "\"", "{a=1}", " ", "a  "}
var seqSep = []string{" ", "  ", "", "  "}
var seqRes = []string{" = 0", " = ?", " = -1 ENOENT (No such file or directory)", " = 3</x>", "", " = 0 ", " = 2 /* 5 in strace's PID NS */"}

func genSeqLine(r *rand.Rand) string {
	pid := []string{"1  ", "2  ", "[pid 1] ", "[pid 2] "}[r.Intn(4)]
	name := seqNames[r.Intn(len(seqNames))]
	args := seqArgs[r.Intn(len(seqArgs))]
	switch r.Intn(12) {
	case 0, 1, 2, 3: // open call
		return pid + name + "(" + args + seqSep[r.Intn(len(seqSep))] + "<unfinished ...>"
	case 4, 5, 6: // resumed
		rest := []string{")", ", a)", "a)", ") ", "<x>)", "\")", "))"}[r.Intn(7)]
		return pid + "<... " + name + " resumed>" + rest + seqRes[r.Intn(len(seqRes))]
	case 7: // closed call
		return pid + name + "(" + args + ")" + seqRes[r.Intn(len(seqRes))]
	case 8: // dead shapes
		return pid + []string{"???( <unfinished ...>", "???()  = ?", "<... ??? resumed>)  = ?", "???() = 0", "<... ??? resumed>) = 0"}[r.Intn(5)]
	case 9:
		return pid + []string{"--- SIGCHLD {si_signo=SIGCHLD} ---", "--- SIGKILL ---"}[r.Intn(2)]
	case 10:
		return pid + []string{"+++ exited with 0 +++", "+++ killed by SIGKILL +++", "+++ exited with 3  +++"}[r.Intn(3)]
	}
	return genLine(r, []string{pid + name + "(" + args + ") = 0"})
}

func genSeq(r *rand.Rand) []string {
	n := 1 + r.Intn(4)
	out := make([]string, n)
	for i := range out {
		out[i] = genSeqLine(r)
	}
	return out
}

func TestSeqDiffSeeded(t *testing.T) {
	n := 600000
	if testing.Short() {
		n = 60000
	}
	r := rand.New(rand.NewSource(7))
	counts := map[string]int{}
	bad, failOpen := 0, 0
	for i := 0; i < n; i++ {
		seq := genSeq(r)
		p, fo, rows := seqVerdict(seq, i%2 == 0)
		for _, row := range rows {
			counts[row]++
		}
		if p != "" {
			if fo {
				failOpen++
			}
			if bad++; bad <= 10 {
				t.Errorf("%s: %q", p, seq)
			}
		}
	}
	if failOpen != 0 {
		t.Errorf("failOpen = %d", failOpen)
	}
	if g := grammarPanics.Load(); g != 0 {
		t.Fatalf("%d recovered panics", g)
	}
	t.Logf("%d sequences, failOpen=%d, other problems=%d; allowed rows: %v", n, failOpen, bad-failOpen, counts)
}

// A split call that is over its arity cap once glued, and has a later syntax
// defect: the old scanner says arity, the grammar path says a syntax reason.  The
// arity row is judged on the GLUED text (a resumed line has no call name).
func TestGluedOverArityRow(t *testing.T) {
	seq := []string{"1  fork( <unfinished ...>", "1  <... fork resumed>AT_FDCWD</w>>,) = ?"}
	p, fo, rows := seqVerdict(seq, false)
	if p != "" || fo {
		t.Fatalf("%s (failOpen=%v)", p, fo)
	}
	if len(rows) != 1 || rows[0] != "too many arguments and a later error" {
		t.Errorf("rows = %v", rows)
	}
}

// FuzzSeqDiff: up to four lines (split on newline), shared state.
func FuzzSeqDiff(f *testing.F) {
	for _, s := range []string{
		"2  openat( ,  <unfinished ...>\n2  <... openat resumed>) = 0",
		"1  wait4(-1,  , <unfinished ...>\n1  <... wait4 resumed>) = 0",
		"1  wait4(-1,   <unfinished ...>\n1  <... wait4 resumed>) = 0",
		"1  wait4( ,  <unfinished ...>\n1  <... wait4 resumed>) = 0",
		"1  ???( <unfinished ...>\n1  <... ??? resumed>)  = ?\n1  <... ??? resumed>)  = ?",
		"1  execve(\"/a\", [\"a\"], 0x1 <unfinished ...>\n2  execve(\"/a\", [\"a\"], 0x1 <unfinished ...>\n2  +++ exited with 0 +++\n1  <... execve resumed>) = 0",
		"1  stat( <unfinished ...>\n1  stat(a) = 0",
		"1  exit_group(0 <unfinished ...>\n1  <... exit_group resumed>) = ?",
		"1  fork( <unfinished ...>\n1  <... fork resumed>AT_FDCWD</w>>,) = ?",
	} {
		f.Add(s)
	}
	f.Fuzz(func(t *testing.T, in string) {
		lines := strings.Split(in, "\n")
		if len(lines) > 4 {
			lines = lines[:4]
		}
		for _, helper := range []bool{false, true} {
			if p, fo, _ := seqVerdict(lines, helper); p != "" {
				t.Fatalf("failOpen=%v %s: %q", fo, p, lines)
			}
		}
		if g := grammarPanics.Load(); g != 0 {
			t.Fatal("recovered panic")
		}
	})
}
