package trace

import (
	"reflect"
	"strings"
	"testing"
)

// KILN04 D19: signals in the trace.  strace runs with -e signal=all, so the
// trace now carries "--- SIG ---" delivery lines and a "+++ killed by SIG +++"
// line for every signal death, and the parser must (a) ignore the first,
// (b) accept the second, and (c) note where the init's exit_group began.

// stepPrefix is the helper tree up to and including the step's exec: line 1 is
// the root's exec, line 11 the step's.  The step process is pid 11.
const stepPrefix = helperRootSetup + helperInitSetup + helperReExec + helperExec

const stepPrefixLines = 11

// spawnThread is a CLONE_THREAD clone by parent, with the PID-namespace note
// the sandbox's spawns must carry (the new thread is `host` outside).
func spawnThread(parent, host int) string {
	return itoa(parent) + "  clone(child_stack=NULL, flags=" + threadFlags + ") = 2" + ns(host) + "\n"
}

func TestSignalLinesAreIgnoredAndDeathsParse(t *testing.T) {
	text := stepPrefix +
		"11  clone(child_stack=NULL, flags=CLONE_VM|CLONE_VFORK|SIGCHLD) = 2" + ns(12) + "\n" +
		"11  --- SIGCHLD {si_signo=SIGCHLD, si_code=CLD_KILLED, si_pid=2, si_uid=1000, si_status=SIGTERM, si_utime=0, si_stime=0} ---\n" +
		"12  --- stopped by SIGSTOP ---\n" +
		"12  +++ killed by SIGTERM +++\n" +
		"11  +++ killed by SIGSEGV (core dumped) +++\n" +
		"10  +++ exited with 0 +++\n"
	tr := mustHelper(t, text)
	var term, segv *Exit
	for i := range tr.Exits {
		switch tr.Exits[i].Signal {
		case "SIGTERM":
			term = &tr.Exits[i]
		case "SIGSEGV":
			segv = &tr.Exits[i]
		}
	}
	if term == nil || !term.Killed || term.Pid != 12 || term.CoreDumped {
		t.Errorf("SIGTERM exit: %+v", term)
	}
	if segv == nil || !segv.Killed || segv.Pid != 11 || !segv.CoreDumped {
		t.Errorf("SIGSEGV exit: %+v", segv)
	}
}

func TestInitExitLine(t *testing.T) {
	end := "11  +++ exited with 0 +++\n10  +++ exited with 0 +++\n"
	child := "11  clone(child_stack=NULL, flags=SIGCHLD) = 2" + ns(12) + "\n"
	cases := []struct {
		name string
		text string
		want int
	}{
		{"one-line form", stepPrefix + "11  exit_group(0) = ?\n" + end, stepPrefixLines + 1},
		{"unfinished and resumed", stepPrefix + "11  exit_group(0 <unfinished ...>\n11  <... exit_group resumed>) = ?\n" + end, stepPrefixLines + 1},
		{"the first one counts", stepPrefix + "11  exit_group(0) = ?\n11  exit_group(0) = ?\n" + end, stepPrefixLines + 1},
		{"a thread of init", stepPrefix + spawnThread(11, 12) + "12  exit_group(0) = ?\n12  +++ exited with 0 +++\n" + end, stepPrefixLines + 2},
		{"a thread of a thread of init", stepPrefix + spawnThread(11, 12) +
			"12  clone(child_stack=NULL, flags=" + threadFlags + ") = 3" + ns(13) + "\n" +
			"13  exit_group(0) = ?\n13  +++ exited with 0 +++\n12  +++ exited with 0 +++\n" + end, stepPrefixLines + 3},
		{"a child's exit_group is not init's", stepPrefix + child +
			"12  exit_group(0) = ?\n12  +++ exited with 0 +++\n" + end, 0},
		{"a child's, then init's", stepPrefix + child +
			"12  exit_group(0) = ?\n12  +++ exited with 0 +++\n11  exit_group(0) = ?\n" + end, stepPrefixLines + 4},
		{"none at all (killed from outside)", stepPrefix + "11  +++ killed by SIGKILL +++\n10  +++ exited with 0 +++\n", 0},
		{"the root's exit_group is not init's", stepPrefix +
			"10  exit_group(0) = ?\n11  +++ exited with 0 +++\n10  +++ exited with 0 +++\n", 0},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			tr, err := parseAt(t, "/start", c.text, helperOpts)
			if err != nil {
				t.Fatalf("%v\n%s", err, c.text)
			}
			if tr.InitExitLine != c.want {
				t.Errorf("InitExitLine = %d, want %d\n%s", tr.InitExitLine, c.want, c.text)
			}
		})
	}
}

// An init exit_group before the step's exec (the setup window) is not the
// step's: the trace has no step exec, so it is refused outright, and the
// parser never reports a line for it.
func TestInitExitLineIgnoresTheSetupWindow(t *testing.T) {
	text := helperRootSetup + "11  exit_group(0) = ?\n11  +++ exited with 0 +++\n10  +++ exited with 0 +++\n"
	tr, err := parseAt(t, "/start", text, helperOpts)
	if err == nil && tr.InitExitLine != 0 {
		t.Fatalf("setup-window exit_group recorded at line %d", tr.InitExitLine)
	}
}

func TestExitLinesAlignWithExits(t *testing.T) {
	text := stepPrefix +
		"11  clone(child_stack=NULL, flags=SIGCHLD) = 2" + ns(12) + "\n" + // line 12
		"12  +++ killed by SIGPIPE +++\n" + // line 13
		"11  exit_group(0) = ?\n" + // line 14
		"11  +++ exited with 0 +++\n" + // line 15
		"10  +++ exited with 0 +++\n" // line 16
	tr := mustHelper(t, text)
	if !reflect.DeepEqual(tr.ExitLines, []int{13, 15, 16}) || len(tr.ExitLines) != len(tr.Exits) {
		t.Fatalf("ExitLines %v for %+v", tr.ExitLines, tr.Exits)
	}
	if tr.Exits[0].Pid != 12 || tr.Exits[0].Signal != "SIGPIPE" || tr.InitExitLine != 14 {
		t.Fatalf("%+v %d", tr.Exits, tr.InitExitLine)
	}
}

// A failed exit_group (EPERM from a seccomp filter the step installed) does not
// mean the group is dying, so it must not set the marker; in the split form the
// marker taken at the start line is withdrawn when the resumed half shows a
// real result.  A later exit_group that does not return takes it.
func TestInitExitLineNeedsACallThatDidNotReturn(t *testing.T) {
	end := "11  +++ exited with 0 +++\n10  +++ exited with 0 +++\n"
	const eperm = "-1 EPERM (Operation not permitted)"
	cases := []struct {
		name string
		body string
		want int
	}{
		{"one-line failure", "11  exit_group(0) = " + eperm + "\n", 0},
		{"unfinished, resumed with a failure", "11  exit_group(0 <unfinished ...>\n11  <... exit_group resumed>) = " + eperm + "\n", 0},
		{"unfinished, resumed unknown", "11  exit_group(0 <unfinished ...>\n11  <... exit_group resumed>) = ?\n", stepPrefixLines + 1},
		{"a failure, then the real one", "11  exit_group(0) = " + eperm + "\n11  exit_group(0) = ?\n", stepPrefixLines + 2},
		{"a split failure, then the real one",
			"11  exit_group(0 <unfinished ...>\n11  <... exit_group resumed>) = " + eperm + "\n11  exit_group(0) = ?\n", stepPrefixLines + 3},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			tr, err := parseAt(t, "/start", stepPrefix+c.body+end, helperOpts)
			if err != nil {
				t.Fatal(err)
			}
			if tr.InitExitLine != c.want {
				t.Errorf("InitExitLine = %d, want %d", tr.InitExitLine, c.want)
			}
		})
	}
}

// D18 interaction, pinned: a signal-delivery line ("--- SIG ---", which
// -e signal=all now prints) on a thread with a "???(" entry pending is
// "interleaved".  SIGKILL has no delivery stop, so a real trace should never
// show it; if one does, the parser refuses it.  A future relaxation must be
// deliberate, and must change this test.
func TestSignalLineOnAPendingDeadEntryIsInterleaved(t *testing.T) {
	for _, sig := range []string{
		"2  --- SIGCHLD {si_signo=SIGCHLD, si_code=CLD_KILLED, si_pid=3, si_uid=1000, si_status=SIGKILL, si_utime=0, si_stime=0} ---\n",
		"2  --- SIGKILL {si_signo=SIGKILL, si_code=SI_USER, si_pid=3, si_uid=1000} ---\n",
		"2  --- stopped by SIGSTOP ---\n",
	} {
		text := thr("2  ???( <unfinished ...>\n" + sig + "2  <... ??? resumed>) = ?\n")
		wantReason(t, text, reasonInterleaved)
	}
}

// signalSeeds feed FuzzParse.
var signalSeeds = []string{
	stepPrefix + "11  exit_group(0) = -1 EPERM (Operation not permitted)\n11  +++ exited with 0 +++\n10  +++ exited with 0 +++\n",
	stepPrefix + "11  exit_group(0 <unfinished ...>\n11  <... exit_group resumed>) = -1 EPERM (Operation not permitted)\n11  +++ exited with 0 +++\n10  +++ exited with 0 +++\n",
	stepPrefix + "11  exit_group(0) = ?\n11  +++ exited with 0 +++\n10  +++ exited with 0 +++\n",
	stepPrefix + "11  exit_group(0 <unfinished ...>\n11  <... exit_group resumed>) = ?\n11  +++ exited with 0 +++\n10  +++ exited with 0 +++\n",
	stepPrefix + spawnThread(11, 12) + "12  exit_group(0) = ?\n12  +++ exited with 0 +++\n11  +++ exited with 0 +++\n10  +++ exited with 0 +++\n",
	"1  --- SIGCHLD {si_signo=SIGCHLD, si_code=CLD_EXITED, si_pid=2, si_uid=0, si_status=0, si_utime=0, si_stime=0} ---\n1  +++ exited with 0 +++\n",
	"1  --- stopped by SIGSTOP ---\n1  +++ exited with 0 +++\n",
	"1  +++ killed by SIGTERM +++\n",
	"1  +++ killed by SIGSEGV (core dumped) +++\n",
}

func TestSignalSeedsParseOrRefuseCleanly(t *testing.T) {
	for _, s := range signalSeeds {
		opts := Options{HelperRoot: strings.Contains(s, "kiln"), HelperExe: kilnExe}
		if _, err := parseAt(t, "/start", s, opts); err != nil && !strings.HasPrefix(err.Error(), "trace-incomplete:") {
			t.Errorf("%q: %v", s, err)
		}
	}
}
