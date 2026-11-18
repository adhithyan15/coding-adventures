package trace

import (
	"errors"
	"reflect"
	"strconv"
	"strings"
	"testing"
)

// The phantom execve (D33): a thread that de_thread killed at a syscall-entry
// stop has strace print a byte-identical copy of the exec leader's unfinished
// execve under its own pid, then its exit.  See "A phantom execve" in line.go.
//
// Every test goes through ParseOptions, which the init hook cross-checks: the
// old scanner and the grammar reader must agree on every trace below.

const (
	// phantomExec is the captured execve text up to "<unfinished ...>".
	phantomExec = `execve("/usr/bin/sh", ["sh", "-c", "sleep 300 & sleep 1; echo started"], 0x24e1f3f4ab70 /* 5 vars */`
	// phantomThread is a plain thread clone's flags (no CLONE_FS, so the
	// threads do not share a working directory).
	phantomThread = threadFlags
)

// clones is parent spawning one thread per kid.
func clones(parent int, kids ...int) string {
	var b strings.Builder
	for _, k := range kids {
		b.WriteString(strconv.Itoa(parent) + "  clone(child_stack=0x7f0000001000, flags=" + phantomThread + ") = " + strconv.Itoa(k) + "\n")
	}
	return b.String()
}

// unf is pid's unfinished copy of the captured execve.
func unf(pid int) string { return strconv.Itoa(pid) + "  " + phantomExec + " <unfinished ...>\n" }

func exited(pid int) string { return strconv.Itoa(pid) + "  +++ exited with 0 +++\n" }

// resumed is pid's execve returning 0.
func resumed(pid int) string { return strconv.Itoa(pid) + "  <... execve resumed>) = 0\n" }

// capturedPlain is the captured lines in plain mode, completed with the clones
// that made the threads, the owner's resume and its exit.
var capturedPlain = clones(4288, 4289, 4291, 4292, 4293) +
	"4288  chdir(\"/tmp/TestSandboxRecordGates1638617378/002/repo/code/packages/x/a\") = 0\n" +
	unf(4288) + unf(4289) + exited(4289) + exited(4293) + exited(4292) + exited(4291) +
	resumed(4288) + exited(4288)

// helperGhost is the ghost of the helper-mode fixture: thread 13 of the init.
const helperGhost = "13  execve(\"/bin/sh\", [\"sh\", \"-c\", \"make\"], 0x0 /* 1 vars */ <unfinished ...>\n13  +++ exited with 0 +++\n"

// capturedHelper is the same shape under the sandbox helper: the init (11)
// has a Go runtime thread 13, made during setup, and the step's exec is a split
// call with mid printed inside it.
func capturedHelper(mid string) string {
	return helperRootSetup + "11  clone(child_stack=0x7f0000001000, flags=" + threadFlags +
		", parent_tid=[3 /* 13 in strace's PID NS */], tls=0x1) = 3 /* 13 in strace's PID NS */\n" +
		helperInitSetup +
		"11  execve(\"/bin/sh\", [\"sh\", \"-c\", \"make\"], 0x0 /* 1 vars */ <unfinished ...>\n" + mid +
		"11  <... execve resumed>) = 0\n" + helperEnd
}

// dropPids removes every execve line of the given pids, keeping their exits:
// the trace the phantom stands in for (the exit record stands).
func dropPids(text string, pids ...int) string {
	var out []string
	for _, l := range strings.SplitAfter(text, "\n") {
		drop := false
		for _, p := range pids {
			if strings.HasPrefix(l, strconv.Itoa(p)+"  execve(") {
				drop = true
			}
		}
		if !drop {
			out = append(out, l)
		}
	}
	return strings.Join(out, "")
}

func TestPhantomExecAccepted(t *testing.T) {
	// Plain: pid 1 leads, threads 2 and 3 (the captured shape, smaller).
	pl := func(body string) string { return clones(1, 2, 3) + body + exited(1) }
	cases := []struct {
		name   string
		text   string
		helper bool
		ghosts []int
	}{
		{"captured", capturedPlain, false, []int{4289}},
		{"two ghosts, exits in order", pl(unf(1) + unf(2) + unf(3) + exited(2) + exited(3) + resumed(1)), false, []int{2, 3}},
		{"two ghosts, exits reversed", pl(unf(1) + unf(2) + unf(3) + exited(3) + exited(2) + resumed(1)), false, []int{2, 3}},
		{"ghost, then a thread's exit, after the resume", pl(unf(1) + unf(2) + exited(2) + resumed(1) + exited(3)), false, []int{2}},
		{"captured in helper mode", capturedHelper(helperGhost), true, nil},
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
			var want *Trace
			ghosts := c.ghosts
			if c.helper {
				want, ghosts = parse(capturedHelper("13  +++ exited with 0 +++\n")), []int{3}
			} else {
				want = parse(dropPids(c.text, c.ghosts...))
			}
			for _, e := range got.Events {
				for _, p := range ghosts {
					if e.Pid == p {
						t.Fatalf("event for the ghost pid: %+v", e)
					}
				}
			}
			// (In helper mode the ghost is strace pid 13, translated to 3.)
			if g, w := noLines(got.Events), noLines(want.Events); !reflect.DeepEqual(g, w) {
				t.Fatalf("events differ from the trace without the ghost:\n got %+v\nwant %+v", g, w)
			}
			if !c.helper {
				for _, p := range c.ghosts {
					if pr := got.Procs[p]; pr == nil || pr.Exit == nil {
						t.Fatalf("ghost pid %d has no exit record", p)
					}
				}
			}
		})
	}
}

// If two execves are pending with the ghost's text, the EARLIEST is the owner,
// so the answer cannot depend on map iteration order: here the earliest (1)
// resumes = 0 while the later one (4) fails.
func TestPhantomExecOwnerIsTheEarliest(t *testing.T) {
	text := clones(1, 2) + "1  clone(child_stack=NULL, flags=SIGCHLD) = 4\n" +
		unf(1) + unf(4) + unf(2) + exited(2) +
		"4  <... execve resumed>) = -1 ENOENT (No such file or directory)\n4  +++ exited with 1 +++\n" +
		resumed(1) + exited(1)
	for i := 0; i < 50; i++ { // map iteration order is random
		mustParse(t, text)
	}
}

func refusal(t *testing.T, text string, helper bool) *Error {
	t.Helper()
	var err error
	if helper {
		_, err = parseAt(t, "/start", text, helperOpts)
	} else {
		_, err = parseText(t, text)
	}
	var e *Error
	if !errors.As(err, &e) {
		t.Fatalf("want a refusal, got %v\n%s", err, text)
	}
	return e
}

func TestPhantomExecRefused(t *testing.T) {
	// Plain layout: lines 1 and 2 are the clones of threads 2 and 3, line 3 is
	// the owner's unfinished execve and line 4 the ghost's.
	own := unf(1)
	mk := func(body string) string { return clones(1, 2, 3) + body + exited(1) }
	// ghostWith is the ghost's line with one byte-level change.
	ghostWith := func(from, to string) string { return strings.Replace(unf(2), from, to, 1) }
	relative := func(s string) string { return strings.Replace(s, `execve("/usr/bin/sh"`, `execve("sh"`, 1) }
	cases := []struct {
		name, text, reason string
		line               int // the refused line; 0 = not checked
	}{
		{"owner resumes ENOENT", mk(own + unf(2) + exited(2) + "1  <... execve resumed>) = -1 ENOENT (No such file or directory)\n"), reasonUnresumed, 4},
		{"owner never resumes", mk(own + unf(2) + exited(2)), reasonUnresumed, 3},
		{"owner exits first", mk(own + unf(2) + exited(1) + exited(2)), reasonUnresumed, 3},
		{"envp address differs", mk(own + ghostWith("0x24e1f3f4ab70", "0x24e1f3f4ab71") + exited(2) + resumed(1)), reasonUnresumed, 4},
		{"argv differs", mk(own + ghostWith("started", "stopped") + exited(2) + resumed(1)), reasonUnresumed, 4},
		{"relative path", mk(relative(own) + relative(unf(2)) + exited(2) + resumed(1)), reasonUnresumed, 4},
		{"execveat", mk("1  execveat(AT_FDCWD, \"/usr/bin/sh\", [\"sh\"], 0x1, 0 <unfinished ...>\n" +
			"2  execveat(AT_FDCWD, \"/usr/bin/sh\", [\"sh\"], 0x1, 0 <unfinished ...>\n" + exited(2) +
			"1  <... execveat resumed>) = 0\n"), reasonUnresumed, 4},
		{"ghost exits non-zero", mk(own + unf(2) + "2  +++ exited with 1 +++\n" + resumed(1)), reasonUnresumed, 4},
		{"ghost killed by a signal", mk(own + unf(2) + "2  +++ killed by SIGKILL +++\n" + resumed(1)), reasonUnresumed, 4},
		{"signal line between the ghost and its exit", mk(own + unf(2) + "2  --- SIGCHLD {si_signo=SIGCHLD} ---\n" + exited(2) + resumed(1)), reasonUnresumed, 4},
		{"ghost not in the owner's thread group", "1  clone(child_stack=NULL, flags=SIGCHLD) = 2\n" +
			own + unf(2) + exited(2) + resumed(1) + exited(1), reasonUnresumed, 3},
		{"ghost line precedes the owner's", mk(unf(2) + own + exited(2) + resumed(1)), reasonUnresumed, 3},
		{"ghost followed by another call", mk(own + unf(2) + "2  stat(\"/x\", 0x1) = 0\n" + exited(2) + resumed(1)), reasonInterleaved, 5},
		{"ghost with no owner at all", mk(unf(2) + exited(2)), reasonUnresumed, 3},
		{"ghost whose owner is another ghost", mk(unf(2) + unf(3) + exited(2) + exited(3) + own + resumed(1)), reasonUnresumed, 3},
		{"openat unfinished then exit", mk("2  openat(AT_FDCWD, \"/x\", O_RDONLY <unfinished ...>\n" + exited(2)), reasonUnresumed, 3},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			e := refusal(t, c.text, false)
			if e.Reason != c.reason || (c.line != 0 && e.Line != c.line) {
				t.Fatalf("refused %s at line %d, want %s at %d\n%s", e.Reason, e.Line, c.reason, c.line, c.text)
			}
		})
	}
}

// A ghost that goes on to resume is an ordinary call: no ghost, two execs.
func TestPhantomExecThenResumeIsAnOrdinaryCall(t *testing.T) {
	text := clones(1, 2) + unf(1) + unf(2) + resumed(2) + resumed(1) + exited(2) + exited(1)
	n := 0
	for _, e := range mustParse(t, text).Events {
		if e.Syscall == "execve" {
			n++
		}
	}
	if n != 2 {
		t.Fatalf("%d exec events, want 2", n)
	}
}

// In helper mode a ghost thread of the ROOT group (10) is in the wrong thread
// group: refused at the ghost's line.
func TestPhantomExecRefusedInHelperMode(t *testing.T) {
	root := strings.Replace(capturedHelper(strings.ReplaceAll(helperGhost, "13  ", "12  ")),
		"10  unshare(CLONE_NEWUSER) = 0\n",
		"10  unshare(CLONE_NEWUSER) = 0\n10  clone(child_stack=0x7f0000001000, flags="+threadFlags+", parent_tid=[12], tls=0x1) = 12\n", 1)
	e := refusal(t, root, true)
	if e.Reason != reasonUnresumed {
		t.Fatalf("%v", e)
	}
	if got := strings.Split(root, "\n")[e.Line-1]; !strings.HasPrefix(got, "12  execve(") {
		t.Fatalf("refused line %d is %q, want the ghost's", e.Line, got)
	}
}

// The owner resuming "= ?" is not a success: refused at the ghost's line.
func TestPhantomExecOwnerResumesUnknown(t *testing.T) {
	text := clones(1, 2) + unf(1) + unf(2) + exited(2) + "1  <... execve resumed>) = ?\n" + exited(1)
	e := refusal(t, text, false)
	if e.Reason != reasonUnresumed || e.Line != 3 {
		t.Fatalf("%s at %d", e.Reason, e.Line)
	}
}

// After the first identical owner resumed, a later ghost attaches to the next
// pending one (threads 2 and 4 of leader 1 are both in its group).
func TestPhantomExecSecondOwnerAfterTheFirstResumed(t *testing.T) {
	text := clones(1, 2, 3, 4) + unf(1) + resumed(1) + unf(4) + unf(2) + exited(2) + resumed(4) +
		exited(3) + exited(4) + exited(1)
	tr := mustParse(t, text)
	for _, e := range tr.Events {
		if e.Pid == 2 {
			t.Fatalf("event for the ghost: %+v", e)
		}
	}
}

// chain makes a thread chain: first spawns first+1, which spawns first+2, ...
// up to last, each by CLONE_THREAD.
func chain(first, last int) string {
	var b strings.Builder
	for k := first; k < last; k++ {
		b.WriteString(clones(k, k+1))
	}
	return b.String()
}

func exitsOf(first, last int) string {
	var b strings.Builder
	for k := last; k >= first; k-- {
		b.WriteString(exited(k))
	}
	return b.String()
}

// Deeper than maxProcessDepth the walk gives up (threadGroupOf): two threads
// whose walks stop at the same ancestor compare equal, which is sound because
// every link is CLONE_THREAD; threads of different groups never do.
func TestPhantomExecBeyondTheDepthBound(t *testing.T) {
	const depth = maxProcessDepth + 100
	// Siblings A and B hang off the end of one deep chain 1..depth.
	a, b := depth+1, depth+2
	same := chain(1, depth) + clones(depth, a, b) + unf(a) + unf(b) + exited(b) + resumed(a) +
		exitsOf(1, depth) + exited(a)
	tr := mustParse(t, same)
	for _, e := range tr.Events {
		if e.Pid == b {
			t.Fatalf("event for the ghost: %+v", e)
		}
	}
	// A deep chain in ANOTHER process (forked from 1): refused at the ghost's
	// line, wherever the walk stops.
	const p = 100000
	diff := chain(1, depth) + "1  clone(child_stack=NULL, flags=SIGCHLD) = " + strconv.Itoa(p) + "\n" +
		chain(p, p+depth) + unf(depth) + unf(p+depth) + exited(p+depth) + resumed(depth) +
		exitsOf(1, depth) + exitsOf(p, p+depth-1)
	e := refusal(t, diff, false)
	if e.Reason != reasonUnresumed {
		t.Fatalf("%s at %d", e.Reason, e.Line)
	}
	if got := strings.Split(diff, "\n")[e.Line-1]; !strings.HasPrefix(got, strconv.Itoa(p+depth)+"  execve(") {
		t.Fatalf("refused line %d is %q, want the ghost's", e.Line, got)
	}
}

// phantomExecSeeds feeds the fuzzers (FuzzParseDiff).
var phantomExecSeeds = []string{
	capturedPlain,
	capturedHelper(helperGhost),
	clones(1, 2) + unf(1) + unf(2) + exited(2) + "1  <... execve resumed>) = -1 ENOENT (No such file or directory)\n" + exited(1),
	clones(1, 2) + unf(1) + unf(2) + "2  --- SIGCHLD {si_signo=SIGCHLD} ---\n" + exited(2) + resumed(1) + exited(1),
}
