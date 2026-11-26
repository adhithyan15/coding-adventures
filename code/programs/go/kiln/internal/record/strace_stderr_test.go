package record

import (
	"strings"
	"testing"
)

func TestClassifyStraceStderr(t *testing.T) {
	cases := []struct {
		name   string
		text   string
		signal int
		want   string
	}{
		{"empty", "", 0, ""},
		{"listen eio", "strace: ptrace(PTRACE_LISTEN, 4242, NULL, 0): Input/output error\n", 0, StraceErrListenEIO},
		{"listen other errno", "strace: ptrace(PTRACE_LISTEN, 4242, NULL, 0): No such process\n", 0, StraceErrTraceeGone},
		{"seize", "strace: attach: ptrace(PTRACE_SEIZE, 12): Operation not permitted\n", 0, StraceErrAttach},
		{"eperm from a ptrace call", "strace: ptrace(PTRACE_INTERRUPT, 3): Operation not permitted\n", 0, StraceErrPtraceOth},
		{"eperm, not ptrace", "strace: something: Operation not permitted\n", 0, StraceErrUnknown},
		{"helper exec", "kiln sandbox-exec: unshare: x\n", 0, StraceErrHelper},
		{"helper init", "kiln sandbox-init: no such file\n", 0, StraceErrHelper},
		{"helper notify", "kiln sandbox-init: exec-notify: listener gone\n", 0, StraceErrHelperNotify},
		{"helper before strace", "kiln sandbox-init: x\nstrace: wait4(__WALL): y\n", 0, StraceErrHelper},
		{"tracee gone", "strace: ptrace(PTRACE_GETREGSET, 9): No such process\n", 0, StraceErrTraceeGone},
		{"unknown pid", "strace: Exit of unknown pid 77 ignored\n", 0, StraceErrUnknownPid},
		{"umoven", "strace: umoven: short read (4 < 8) @0x7ffc\n", 0, StraceErrUmove},
		{"umovestr", "strace: umovestr: short read (1 < 2) @0x1\n", 0, StraceErrUmove},
		{"ptrace other", "strace: ptrace(PTRACE_GETEVENTMSG, 9): Invalid argument\n", 0, StraceErrPtraceOth},
		{"oom", "strace: Out of memory\n", 0, StraceErrOOM},
		{"enomem", "strace: ptrace(PTRACE_SETOPTIONS, 1): Cannot allocate memory\n", 0, StraceErrOOM},
		{"wait4", "strace: wait4(__WALL): No child processes\n", 0, StraceErrWait},
		{"waitid", "strace: waitid(P_ALL): Interrupted system call\n", 0, StraceErrWait},
		{"killed, nothing matched", "strace: Process 7 detached\n", 9, StraceErrKilled},
		{"killed, but matched", "strace: wait4(__WALL): x\n", 9, StraceErrWait},
		{"unknown", "strace: something new\n", 0, StraceErrUnknown},
	}
	for _, c := range cases {
		if got := ClassifyStraceStderr([]byte(c.text), c.signal); got != c.want {
			t.Errorf("%s: got %q, want %q", c.name, got, c.want)
		}
	}
}

func TestClassifyStraceStderrIsBoundedAndNeverEchoesTheText(t *testing.T) {
	// A pattern past the bound is not seen.
	big := strings.Repeat("x", MaxStraceStderrScan) + "ptrace(PTRACE_LISTEN: Input/output error"
	if got := ClassifyStraceStderr([]byte(big), 0); got != StraceErrUnknown {
		t.Errorf("past the bound: %q", got)
	}
	// Text with a host path: the result is a member of the closed list, and
	// contains none of the text.
	secret := "strace: /home/runner/secret-project/x: ptrace(PTRACE_LISTEN): Input/output error"
	for _, in := range []string{secret, "/home/runner/secret-project", strings.Repeat("/etc/shadow ", 1000)} {
		got := ClassifyStraceStderr([]byte(in), 0)
		if !validStraceStderrCode(got) || strings.Contains(got, "home") || strings.Contains(got, "shadow") || strings.Contains(got, "secret") {
			t.Errorf("%q leaked or left the list", got)
		}
	}
	// 1 MiB of input is fine.
	if got := ClassifyStraceStderr([]byte(strings.Repeat("a", 1<<20)), 0); got != StraceErrUnknown {
		t.Errorf("oversized: %q", got)
	}
}

func TestStraceExitLabel(t *testing.T) {
	for n, want := range map[int]string{0: "exit-0", 1: "exit-1", 125: "exit-125", 127: "exit-127", 3: "exit-other", 255: "exit-other"} {
		if got := StraceExitLabel(n); got != want {
			t.Errorf("%d: %q", n, got)
		}
	}
}

func TestStraceSignalLabel(t *testing.T) {
	for n, want := range map[int]string{9: "SIGKILL(9)", 11: "SIGSEGV(11)", 15: "SIGTERM(15)", 34: "SIG(34)", 64: "SIG(64)"} {
		if got := StraceSignalLabel(n); got != want {
			t.Errorf("%d: %q", n, got)
		}
	}
}

// The census counts the codes and the signals, and the Markdown shows them.
func TestCensusCountsStraceStderrCodesAndSignals(t *testing.T) {
	a := newCensusAcc(nil)
	base := StepSidecar{Started: true, StraceStdoutEmpty: true}
	for _, code := range []string{StraceErrListenEIO, StraceErrListenEIO, StraceErrUnknown, ""} { // "": an older sidecar
		s := base
		s.StraceStderrCode = code
		a.sidecarFacts(s)
	}
	for _, sig := range []int{9, 9, 15, 0} {
		s := base
		s.StraceSignal, s.StraceStderrEmpty = sig, true
		a.sidecarFacts(s)
	}
	for _, ex := range []int{1, 1, 77} { // steps with stderr text
		s := base
		s.StraceExit, s.StraceStderrCode = ex, StraceErrUnknown
		a.sidecarFacts(s)
	}
	f := a.explain.Sidecar
	if f.StraceStderrCodes[StraceErrListenEIO] != 2 || f.StraceStderrCodes[StraceErrUnknown] != 4 || f.StraceStderrCodes[StraceErrUnrecorded] != 1 || len(f.StraceStderrCodes) != 3 {
		t.Errorf("codes: %v", f.StraceStderrCodes)
	}
	if f.StraceExits["exit-0"] != 4 || f.StraceExits["exit-1"] != 2 || f.StraceExits["exit-other"] != 1 || len(f.StraceExits) != 3 {
		t.Errorf("exits: %v", f.StraceExits)
	}
	if f.StraceSignals["SIGKILL(9)"] != 2 || f.StraceSignals["SIGTERM(15)"] != 1 || len(f.StraceSignals) != 2 {
		t.Errorf("signals: %v", f.StraceSignals)
	}
	md := renderMarkdown(&CensusSummary{Explain: a.explain})
	for _, want := range []string{"strace stderr codes", "`ptrace-listen-eio` 2;", "`SIGKILL(9)` 2;", "`exit-1` 2;"} {
		if !strings.Contains(md, want) {
			t.Errorf("the Markdown lacks %q", want)
		}
	}
}

// strace_stderr_failed_steps counts the coded steps whose build step also
// failed: coded+failed counts, coded+passed does not, uncoded+failed does not,
// and the keys stay inside the closed code list (plus "unrecorded").
func TestCensusCountsStraceStderrFailedSteps(t *testing.T) {
	a := newCensusAcc(nil)
	step := func(code string, stderrEmpty bool, root int) StepSidecar {
		return StepSidecar{Started: true, StraceStdoutEmpty: true, StraceStderrCode: code, StraceStderrEmpty: stderrEmpty, RootStatus: root, StraceExit: -1}
	}
	a.sidecarFacts(step(StraceErrListenEIO, false, 1)) // coded, failed: counted
	a.sidecarFacts(step(StraceErrListenEIO, false, 2)) // coded, failed: counted
	a.sidecarFacts(step(StraceErrListenEIO, false, 0)) // coded, passed: not counted
	a.sidecarFacts(step("", true, 1))                  // uncoded, failed: not counted
	a.sidecarFacts(step("", false, 1))                 // older sidecar with text: "unrecorded"
	f := a.explain.Sidecar
	if f.StraceStderrFailedSteps[StraceErrListenEIO] != 2 || f.StraceStderrFailedSteps[StraceErrUnrecorded] != 1 || len(f.StraceStderrFailedSteps) != 2 {
		t.Errorf("failed steps: %v", f.StraceStderrFailedSteps)
	}
	if f.StraceStderrCodes[StraceErrListenEIO] != 3 {
		t.Errorf("codes: %v", f.StraceStderrCodes)
	}
	closed := map[string]bool{StraceErrUnrecorded: true}
	for _, c := range StraceStderrCodes {
		closed[c] = true
	}
	for k := range f.StraceStderrFailedSteps {
		if !closed[k] {
			t.Errorf("key %q is outside the closed code list", k)
		}
	}
	md := renderMarkdown(&CensusSummary{Explain: a.explain})
	if !strings.Contains(md, "whose build step also failed") || !strings.Contains(md, "`ptrace-listen-eio` 2;") {
		t.Errorf("the Markdown lacks the failed-step counts")
	}
}
