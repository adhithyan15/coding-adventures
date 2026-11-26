//go:build unix

package record

import (
	"os"
	"strings"
	"testing"
)

// strace's stderr is classified on the runner: the sidecar carries a code, never
// the text, and the reason is still the old one (so no key changes).  The fixture's
// "strace" is a script, so this is not a real-kernel test; a real strace cannot be
// made to fail this way on demand (see the KILN04 note).
func TestStepRecordsAStraceStderrCode(t *testing.T) {
	for _, c := range []struct{ line, code string }{
		{"echo 'strace: ptrace(PTRACE_LISTEN, 4242, NULL, 0): Input/output error' >&2", StraceErrListenEIO},
		{"echo 'strace: attach: ptrace(PTRACE_SEIZE, 9): Operation not permitted' >&2", StraceErrAttach},
		{"echo 'strace: wait4(__WALL): No child processes' >&2", StraceErrWait},
		{"echo boom-secret-/home/x >&2", StraceErrUnknown},
	} {
		f := newStepFix(t)
		res := f.must(actZ, 0, 1, c.line)
		if res.Sidecar.StraceStderrCode != c.code || !containsStr(res.Sidecar.Unclean, UncleanStderr) {
			t.Fatalf("%q: %+v", c.line, res.Sidecar)
		}
		raw, err := os.ReadFile(StepFile(f.p.dir, actZ, 0, "json"))
		if err != nil {
			t.Fatal(err)
		}
		if !strings.Contains(string(raw), `"strace_stderr_code":"`+c.code+`"`) || strings.Contains(string(raw), "secret") || strings.Contains(string(raw), "4242") {
			t.Fatalf("sidecar text: %s", raw)
		}
	}
	// A step with no stderr carries no code (omitempty).
	f := newStepFix(t)
	f.must(actZ, 0, 1, "exit 0")
	raw, _ := os.ReadFile(StepFile(f.p.dir, actZ, 0, "json"))
	if strings.Contains(string(raw), "strace_stderr_code") {
		t.Fatalf("a code for a quiet strace: %s", raw)
	}
}

// A code the closed list does not have, or one beside no stderr, is refused.
func TestSidecarRefusesABadStraceStderrCode(t *testing.T) {
	f := newStepFix(t)
	res := f.must(actZ, 0, 1, "echo x >&2")
	for name, m := range map[string]func(*StepSidecar){
		"not in the list":  func(s *StepSidecar) { s.StraceStderrCode = "/home/x" },
		"beside no stderr": func(s *StepSidecar) { s.StraceStderrEmpty = true },
	} {
		s := res.Sidecar
		m(&s)
		if err := s.Validate(); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
}
