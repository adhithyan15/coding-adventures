package trace

import (
	"strings"
	"testing"
)

// diagTrace is a trace both readers refuse at line 9: a thread-death line
// "NAME(args <unfinished ...>) = ?", the shape strace prints when a thread dies
// under a pending call.  Line 2 is the last earlier line of pid 200 (the stop
// line's pid), far above the +-3 window; the other lines carry hostile content.
func diagTrace() string {
	long := strings.Repeat("A", 400)
	return "100  execve(\"/bin/true\", [\"true\"], []) = 0\n" + // 1
		"200  stat(\"/a\", 0x1 <unfinished ...>\n" + // 2: pid 200's pending half
		"100  stat(\"/::notice::\\x01\\x1b[31m\", {}) = 0\n" + // 3
		"100  stat(\"/x\", {}) = 0\n" + // 4
		"100  stat(\"" + long + "\", {}) = 0\n" + // 5: a long line
		"100  stat(\"/b\", {}) = 0\n" + // 6
		"100  stat(\"/c\", {}) = 0\n" + // 7
		"100  stat(\"/d\", {}) = 0\n" + // 8
		"200  wait4(-1, <unfinished ...>) = ?\n" // 9
}

func TestDiagnoseReportsReasonsAndLines(t *testing.T) {
	t.Setenv("KILN_TRACE_DIFF", "")
	out := Diagnose([]byte(diagTrace()), false)
	// Always: reasons and numbers.  Both readers are named, with a reason.
	for _, want := range []string{"old scanner: refused", "grammar reader: refused", "stop line", "grammar panics recovered: 0", "ends with newline: true"} {
		if !strings.Contains(out, want) {
			t.Errorf("missing %q in:\n%s", want, out)
		}
	}
	// Without KILN_TRACE_DIFF no trace text is printed.
	if strings.Contains(out, "window") || strings.Contains(out, "stat(") || strings.Contains(out, "::error") {
		t.Errorf("trace text printed without KILN_TRACE_DIFF:\n%s", out)
	}
}

func TestDiagnoseWindow(t *testing.T) {
	t.Setenv("KILN_TRACE_DIFF", "1")
	out := Diagnose([]byte(diagTrace()), false)
	t.Log("\n" + out)
	if !strings.Contains(out, "window around line") {
		t.Fatalf("no window:\n%s", out)
	}
	// The stop line is marked, and the pid's last earlier line (line 2) is
	// shown although it is outside the +-3 window.
	if !strings.Contains(out, "> 9: ") {
		t.Errorf("the stop line is not marked:\n%s", out)
	}
	if !strings.Contains(out, "last earlier line of pid 200") || !strings.Contains(out, "2: \"200  stat(") {
		t.Errorf("the pending half of the stop line's pid is missing:\n%s", out)
	}
	// Escaping: no raw control character, no line starting "::", long lines cut.
	for _, l := range strings.Split(out, "\n") {
		if strings.HasPrefix(l, "::") {
			t.Errorf("a line starts with ::: %q", l)
		}
		for _, r := range l {
			if r < 0x20 || r == 0x7f || r > 0x7e {
				t.Errorf("raw control or non-ASCII character %U in %q", r, l)
			}
		}
	}
	if strings.Contains(out, strings.Repeat("A", 200)) || !strings.Contains(out, "...") {
		t.Errorf("a long line is not cut to 160 bytes:\n%s", out)
	}
}

// TestDiagnoseHostileStopLine: the refused line itself starts with a workflow
// command and holds raw control characters; it must come out quoted and indented.
func TestDiagnoseHostileStopLine(t *testing.T) {
	t.Setenv("KILN_TRACE_DIFF", "1")
	out := Diagnose([]byte("100  execve(\"/bin/true\", [\"true\"], []) = 0\n::error::forged\x01\x1b[31mline\n"), false)
	if !strings.Contains(out, `> 2: "::error::forged\x01\x1b[31mline"`) {
		t.Errorf("the hostile line is not quoted:\n%s", out)
	}
	for _, l := range strings.Split(out, "\n") {
		if strings.HasPrefix(l, "::") || strings.ContainsAny(l, "\x01\x1b") {
			t.Errorf("unsafe output line %q", l)
		}
	}
}

func TestDiagnoseIsBounded(t *testing.T) {
	t.Setenv("KILN_TRACE_DIFF", "1")
	if maxDiagnoseBytes != 8192 {
		t.Fatalf("the documented cap is 8 KB, not %d", maxDiagnoseBytes)
	}
	// The real window stays under the cap; lowering it shows the cut is made.
	out := Diagnose([]byte(diagTrace()), false)
	if len(out) > maxDiagnoseBytes || strings.Contains(out, "(truncated)") {
		t.Errorf("a normal diagnosis is %d bytes: %s", len(out), out)
	}
	defer func(old int) { maxDiagnoseBytes = old }(maxDiagnoseBytes)
	maxDiagnoseBytes = 300
	out = Diagnose([]byte(diagTrace()), false)
	if len(out) > 300 || !strings.HasSuffix(out, "(truncated)") {
		t.Errorf("%d bytes under a 300-byte cap:\n%s", len(out), out)
	}
}

func TestDiagnoseNoNewlineAndAccepted(t *testing.T) {
	t.Setenv("KILN_TRACE_DIFF", "1")
	out := Diagnose([]byte("1  exit_group(0) = ?"), false)
	if !strings.Contains(out, "ends with newline: false") || !strings.Contains(out, "1 lines") {
		t.Errorf("a trace cut mid-line is not reported:\n%s", out)
	}
	if out := Diagnose(nil, false); !strings.Contains(out, "0 lines") {
		t.Errorf("empty data:\n%s", out)
	}
}
