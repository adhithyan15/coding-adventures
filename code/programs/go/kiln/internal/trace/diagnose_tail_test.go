package trace

import (
	"strings"
	"testing"
)

// TestDiagnoseKeyLineKeepsItsTail: the shape that gets a real trace refused sits
// at the END of a long line, so the stop line and the pid's pending half show
// head and tail; a neighbour keeps only its head.
func TestDiagnoseKeyLineKeepsItsTail(t *testing.T) {
	t.Setenv("KILN_TRACE_DIFF", "1")
	long := "200  clone(child_stack=NULL, flags=" + strings.Repeat("CLONE_X|", 80) + "CLONE_LAST, <unfinished ...>"
	if len(long) < 400 {
		t.Fatal("fixture too short")
	}
	data := "100  execve(\"/bin/true\", [\"true\"], []) = 0\n" +
		long + "\n" + // 2: pid 200's pending half, far above
		"100  stat(\"" + strings.Repeat("N", 500) + "TAILNEIGHBOUR\", {}) = 0\n" + // 3: a neighbour
		"100  stat(\"/b\", {}) = 0\n" + "100  stat(\"/c\", {}) = 0\n" + "100  stat(\"/d\", {}) = 0\n" + "100  stat(\"/e\", {}) = 0\n" +
		"200  " + strings.Repeat("X", 450) + " <unfinished ...>) = ?\n" // 8: the stop line
	out := Diagnose([]byte(data), false)
	t.Log("\n" + out)
	if !strings.Contains(out, "<unfinished ...>) = ?") {
		t.Errorf("the stop line's tail is missing:\n%s", out)
	}
	if !strings.Contains(out, "CLONE_LAST, <unfinished ...>") || !strings.Contains(out, "last earlier line of pid 200") {
		t.Errorf("the same-pid line's tail is missing:\n%s", out)
	}
	if strings.Contains(out, "TAILNEIGHBOUR") {
		t.Errorf("a neighbour must stay head-only:\n%s", out)
	}
	// The same-pid line comes before the window, so a cap cannot drop it.
	if strings.Index(out, "last earlier line of pid") > strings.Index(out, "window around") {
		t.Errorf("the same-pid line must precede the window:\n%s", out)
	}
}

// TestDiagnoseErrorLineAndPassNote: an unresumed call at the end of the input is
// reported at the call's start while the stop line is past the end; the Error
// line is printed too.  The report says it is pass 1 only.
func TestDiagnoseErrorLineAndPassNote(t *testing.T) {
	t.Setenv("KILN_TRACE_DIFF", "1")
	out := Diagnose([]byte("1  execve(\"/bin/true\", [\"true\"], []) = 0\n1  stat(\"/a\", 0x1 <unfinished ...>\n"), false)
	t.Log("\n" + out)
	if !strings.Contains(out, "pass 1 only") {
		t.Errorf("no pass-1 note:\n%s", out)
	}
	if !strings.Contains(out, "unresumed-call") || !strings.Contains(out, "error line (") {
		t.Errorf("the Error line differing from the stop line is not printed:\n%s", out)
	}
}
