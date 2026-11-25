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
		{"listen other errno is ptrace-other", "strace: ptrace(PTRACE_LISTEN, 4242, NULL, 0): No such process\n", 0, StraceErrPtraceOth},
		{"seize", "strace: attach: ptrace(PTRACE_SEIZE, 12): Operation not permitted\n", 0, StraceErrAttach},
		{"eperm alone", "strace: something: Operation not permitted\n", 0, StraceErrAttach},
		{"ptrace other", "strace: ptrace(PTRACE_GETREGSET, 9, NT_PRSTATUS): No such process\n", 0, StraceErrPtraceOth},
		{"oom", "strace: Out of memory\n", 0, StraceErrOOM},
		{"enomem", "strace: ptrace(PTRACE_SETOPTIONS, 1): Cannot allocate memory\n", 0, StraceErrOOM},
		{"wait4", "strace: wait4(__WALL): No child processes\n", 0, StraceErrWait},
		{"waitid", "strace: waitid(P_ALL): Interrupted system call\n", 0, StraceErrWait},
		{"killed, nothing matched", "strace: Process 7 detached\n", 9, StraceErrKilled},
		{"killed, but matched", "strace: wait4(__WALL): x\n", 9, StraceErrWait},
		{"unknown", "strace: Exit of unknown pid 5 ignored\n", 0, StraceErrUnknown},
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
	f := a.explain.Sidecar
	if f.StraceStderrCodes[StraceErrListenEIO] != 2 || f.StraceStderrCodes[StraceErrUnknown] != 1 || f.StraceStderrCodes[StraceErrUnrecorded] != 1 || len(f.StraceStderrCodes) != 3 {
		t.Errorf("codes: %v", f.StraceStderrCodes)
	}
	if f.StraceSignals["SIGKILL(9)"] != 2 || f.StraceSignals["SIGTERM(15)"] != 1 || len(f.StraceSignals) != 2 {
		t.Errorf("signals: %v", f.StraceSignals)
	}
	md := renderMarkdown(&CensusSummary{Explain: a.explain})
	for _, want := range []string{"strace stderr codes", "`ptrace-listen-eio` 2;", "`SIGKILL(9)` 2;"} {
		if !strings.Contains(md, want) {
			t.Errorf("the Markdown lacks %q", want)
		}
	}
}
