package trace

import "testing"

// KILN04 D48 step 5 pairs the nth exec event of a pid with the nth line of the
// exec-head sidecar, and refuses a pid whose trace has an exec that is killed,
// unfinished or without a result (`= ?`): the pairing cannot say whether that
// exec reached the kernel's loader.
//
// The classifier (internal/traced) keeps its own wall for such an event, but the
// first wall is HERE: this parser refuses the whole trace for each of these
// shapes, so no half-seen exec ever reaches a pairing.  This test pins that,
// because the classifier's rule leans on it: if the parser ever learned to keep
// one of these execs, this test fails and the pairing's refusal is what stands.
func TestKilledUnfinishedAndResultlessExecsRefuseTheTrace(t *testing.T) {
	const (
		fork = "11  clone(child_stack=NULL, flags=SIGCHLD) = 3 /* 12 in strace's PID NS */\n"
		exec = `12  execve("/tmp/x/t.test", ["t"], 0x0 /* 1 vars */`
		end  = "11  +++ exited with 0 +++\n10  +++ exited with 0 +++\n"
	)
	base := helperRootSetup + helperInitSetup + helperReExec + helperExec + fork
	for name, text := range map[string]string{
		"unfinished, then the pid exits":  base + exec + " <unfinished ...>\n12  +++ exited with 0 +++\n" + end,
		"unfinished, then killed":         base + exec + " <unfinished ...>\n12  +++ killed by SIGKILL +++\n" + end,
		"unfinished, never resumed":       base + exec + " <unfinished ...>\n" + end,
		"result `= ?`, then killed":       base + exec + ") = ?\n12  +++ killed by SIGKILL +++\n" + end,
		"resumed with `= ?`, then killed": base + exec + " <unfinished ...>\n12  <... execve resumed>) = ?\n12  +++ killed by SIGKILL +++\n" + end,
	} {
		t.Run(name, func(t *testing.T) {
			if tr, err := parseAt(t, "/start", text, helperOpts); err == nil {
				t.Fatalf("the trace parsed (%d events); the pairing relies on it being refused", len(tr.Events))
			}
		})
	}
	// The same trace with the exec completing is accepted: the refusals above are
	// about the exec, not the fixture.
	ok := base + exec + ") = 0\n12  +++ exited with 0 +++\n" + end
	if _, err := parseAt(t, "/start", ok, helperOpts); err != nil {
		t.Fatalf("the control trace was refused: %v", err)
	}
}
