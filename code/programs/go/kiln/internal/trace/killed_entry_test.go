package trace

import (
	"reflect"
	"strings"
	"testing"
)

// The "???(" family (D18): a thread killed at syscall entry.  See "A thread
// killed at syscall entry" in line.go.

// thr is a plain (non-helper) trace: pid 1 spawns thread 2 and sandwiches body
// between the spawn and 1's exit.
func thr(body string) string {
	return "1  clone(child_stack=NULL, flags=" + threadFlags + ") = 2\n" + body + "1  +++ exited with 0 +++\n"
}

// stripDead removes every "???" line: the trace the kernel's death marker
// stands in for.
func stripDead(text string) string {
	var out []string
	for _, l := range strings.SplitAfter(text, "\n") {
		if !strings.Contains(l, "???") {
			out = append(out, l)
		}
	}
	return strings.Join(out, "")
}

func noLines(evs []Event) []Event {
	out := append([]Event(nil), evs...)
	for i := range out {
		out[i].Line, out[i].StartLine = 0, 0
	}
	return out
}

// helperThreadTrace is the CI trace shape in helper mode: the root (10) has
// CLONE_THREAD threads 12 and 13, spawns the init child, which execs the step;
// then the group dies.  mid is placed between the step's last call and the
// threads' exit lines.
func helperThreadTrace(mid string) string {
	return helperRootExec +
		"10  clone(child_stack=0x7f0000001000, flags=" + threadFlags + ", parent_tid=[12], tls=0x1) = 12\n" +
		"10  clone(child_stack=0x7f0000002000, flags=" + threadFlags + ", parent_tid=[13], tls=0x1) = 13\n" +
		helperRootSetup[len(helperRootExec):] + helperInitSetup + helperExec +
		"11  openat(AT_FDCWD, \"a\", O_RDONLY) = 3</start/a>\n" + mid +
		"13  +++ exited with 0 +++\n12  +++ exited with 0 +++\n" +
		"11  +++ exited with 5 +++\n10  +++ exited with 0 +++\n"
}

func TestKilledAtEntryAccepted(t *testing.T) {
	pad := strings.Repeat(" ", 10)
	cases := []struct {
		name   string
		text   string
		helper bool
		dead   int
	}{
		{"CI regression S1", helperThreadTrace(
			"10  exit_group(0 <unfinished ...>\n12  ???( <unfinished ...>\n10  <... exit_group resumed>)         = ?\n"), true, 12},
		{"S2 single space", helperThreadTrace(
			"10  exit_group(0 <unfinished ...>\n12  ???( <unfinished ...>\n10  <... exit_group resumed>) = ?\n12  <... ??? resumed>) = ?\n"), true, 12},
		{"S2 padded", helperThreadTrace(
			"10  exit_group(0 <unfinished ...>\n12  ???( <unfinished ...>\n10  <... exit_group resumed>) = ?\n12  <... ??? resumed>)" + pad + "= ?\n"), true, 12},
		{"S3 whole line", helperThreadTrace(
			"10  exit_group(0 <unfinished ...>\n12  ???()" + pad + "= ?\n10  <... exit_group resumed>) = ?\n"), true, 12},
		{"S3 single space", helperThreadTrace("12  ???() = ?\n"), true, 12},
		{"before the sibling's exit_group", helperThreadTrace(
			"12  ???( <unfinished ...>\n10  exit_group(0 <unfinished ...>\n10  <... exit_group resumed>) = ?\n"), true, 12},
		{"two dead threads", helperThreadTrace(
			"10  exit_group(0 <unfinished ...>\n12  ???( <unfinished ...>\n13  ???()  = ?\n10  <... exit_group resumed>) = ?\n"), true, 13},
		{"init-group thread killed by the step's exec", helperRootSetup + "11  clone(child_stack=0x7f0000001000, flags=" + threadFlags +
			", parent_tid=[3 /* 13 in strace's PID NS */], tls=0x1) = 3 /* 13 in strace's PID NS */\n" +
			helperInitSetup + helperExec + "13  ???( <unfinished ...>\n13  +++ exited with 0 +++\n" + helperEnd, true, 13},
		{"non-helper mid-trace", "1  clone(child_stack=NULL, flags=" + threadFlags + ") = 2\n" +
			"1  clone(child_stack=NULL, flags=SIGCHLD) = 3\n" +
			"2  ???( <unfinished ...>\n3  openat(AT_FDCWD, \"/x\", O_RDONLY) = 4</x>\n2  <... ??? resumed>) = ?\n" +
			"2  +++ exited with 0 +++\n3  stat(\"/y\", 0x1) = 0\n3  +++ exited with 0 +++\n1  +++ exited with 0 +++\n", false, 2},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			parse := func(text string) *Trace {
				if c.helper {
					return mustHelper(t, text)
				}
				return mustParse(t, text)
			}
			got := parse(c.text)
			want := parse(stripDead(c.text))
			for _, e := range got.Events {
				if e.Pid == c.dead {
					t.Fatalf("event for the dead pid: %+v", e)
				}
			}
			if p := got.Procs[c.dead]; p == nil || p.Exit == nil {
				t.Fatalf("dead pid %d has no exit: %+v", c.dead, p)
			}
			// Removing lines renumbers the rest, so compare without line numbers.
			if g, w := noLines(got.Events), noLines(want.Events); !reflect.DeepEqual(g, w) {
				t.Fatalf("events differ from the trace without the ??? lines:\n got %+v\nwant %+v", g, w)
			}
		})
	}
}

func TestKilledAtEntryRefused(t *testing.T) {
	open := "2  ???( <unfinished ...>\n"
	exit := "2  +++ exited with 0 +++\n"
	cases := []struct {
		name, text, reason string
	}{
		{"resume = 0", thr(open + "2  <... ??? resumed>) = 0\n" + exit), reasonUnparsable},
		{"resume with a descriptor", thr(open + "2  <... ??? resumed>) = 3</x>\n" + exit), reasonUnparsable},
		{"resume errno", thr(open + "2  <... ??? resumed>) = -1 ENOENT (No such file or directory)\n" + exit), reasonUnparsable},
		{"resume unavailable", thr(open + "2  <... ??? resumed>) = ? <unavailable>\n" + exit), reasonUnparsable},
		{"resume with arguments", thr(open + "2  <... ??? resumed>0x1) = ?\n" + exit), reasonUnparsable},
		{"resume without a space before =", thr(open + "2  <... ??? resumed>)= ?\n" + exit), reasonUnparsable},
		{"complete call follows", thr(open + "2  stat(\"a\", 0x1) = 0\n" + exit), reasonInterleaved},
		{"unfinished call follows", thr(open + "2  stat( <unfinished ...>\n" + exit), reasonInterleaved},
		{"signal line follows", thr(open + "2  --- SIGCHLD {si_signo=SIGCHLD} ---\n" + exit), reasonInterleaved},
		{"never seen again", thr(open), reasonUnresumed},
		{"no exit after S2", thr(open + "2  <... ??? resumed>) = ?\n"), reasonUnresumed},
		{"no exit after S3", thr("2  ???() = ?\n"), reasonUnresumed},
		{"completed with a result", thr("2  ???() = 0\n" + exit), reasonUnparsable},
		{"completed with arguments", thr("2  ???(0x1, 0x2) = ?\n" + exit), reasonUnparsable},
		{"unfinished with arguments", thr("2  ???(0x1 <unfinished ...>\n" + exit), reasonUnparsable},
		{"resume of another name", thr(open + "2  <... execve resumed>) = 0\n" + exit), reasonUnmatched},
		{"resume of openat", thr(open + "2  <... openat resumed>) = 0\n" + exit), reasonUnmatched},
		// The leader L is at an entry stop when thread T execs.  strace prints
		// "L  +++ superseded by execve in pid T +++" (leader's pid as the
		// prefix), which is unparsable.  Without that line, "2 ???(" followed by
		// "2 <... execve resumed>) = 0" is the "resume of another name" case
		// above: unmatched-resume.
		{"superseded leader", thr("2  ???( <unfinished ...>\n2  +++ superseded by execve in pid 3 +++\n2  <... execve resumed>) = 0\n" + exit), reasonUnparsable},
		{"two entries", thr(open + open + exit), reasonInterleaved},
		{"two resumes", thr(open + "2  <... ??? resumed>) = ?\n2  <... ??? resumed>) = ?\n" + exit), reasonUnmatched},
		{"resume with nothing pending", thr("2  <... ??? resumed>) = ?\n" + exit), reasonUnmatched},
		{"entry while openat is pending", thr("2  openat(AT_FDCWD, \"/x\", O_RDONLY <unfinished ...>\n" + open + exit), reasonInterleaved},
		{"S3 then a call", thr("2  ???() = ?\n2  stat(\"a\", 0x1) = 0\n" + exit), reasonInterleaved},
		{"S3 then an entry", thr("2  ???() = ?\n" + open + exit), reasonInterleaved},
		{"S3 then a resume", thr("2  ???() = ?\n2  <... ??? resumed>) = ?\n" + exit), reasonUnmatched},
		{"last line without a newline", "1  clone(child_stack=NULL, flags=" + threadFlags + ") = 2\n2  ???( <unfinished ...>", reasonUnparsable},
		{"??(", thr("2  ??(\n" + exit), reasonUnparsable},
		{"????(", thr("2  ????( <unfinished ...>\n" + exit), reasonUnparsable},
		{"???  (", thr("2  ???  ( <unfinished ...>\n" + exit), reasonUnparsable},
		{"trailing text", thr("2  ???( <unfinished ...> x\n" + exit), reasonUnparsable},
		{"entry with a trailing space", thr("2  ???( <unfinished ...> \n" + exit), reasonUnparsable},
		{"S3 with trailing text", thr("2  ???() = ? x\n" + exit), reasonUnparsable},
		{"unknown syscall number", thr("2  syscall_0x1c8(0x1) = -1 ENOSYS (Function not implemented)\n" + exit), "unknown-syscall:syscall_0x1c8"},
		{"openat unfinished then exit", thr("2  openat(AT_FDCWD, \"/x\", O_RDONLY <unfinished ...>\n" + exit), reasonUnresumed},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) { wantReason(t, c.text, c.reason) })
	}
}

func TestKilledAtEntryRefusedInHelperMode(t *testing.T) {
	open := "12  ???( <unfinished ...>\n"
	cases := []struct{ name, text, reason string }{
		{"call after entry", helperThreadTrace(open + "12  close(3) = 0\n"), reasonInterleaved},
		{"no exit line", strings.Replace(helperThreadTrace(open), "12  +++ exited with 0 +++\n", "", 1), reasonUnresumed},
		{"resume = 0", helperThreadTrace(open + "12  <... ??? resumed>) = 0\n"), reasonUnparsable},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) { wantReasonOpts(t, c.text, helperOpts, c.reason) })
	}
}

func TestKilledAtEntryFuzzSeedsParse(t *testing.T) {
	// The accepted shapes are also fuzz seeds (FuzzParse); keep them in sync.
	for _, s := range killedEntrySeeds {
		if _, err := parseText(t, s); err != nil {
			t.Errorf("%q: %v", s, err)
		}
	}
}

// killedEntrySeeds feeds FuzzParse.
var killedEntrySeeds = []string{
	thr("2  ???( <unfinished ...>\n2  +++ exited with 0 +++\n"),
	thr("2  ???( <unfinished ...>\n2  <... ??? resumed>)    = ?\n2  +++ exited with 0 +++\n"),
	thr("2  ???()    = ?\n2  +++ exited with 0 +++\n"),
}
