package trace

import (
	"fmt"
	"strings"
	"testing"
)

// Regression tests for the security review of the stage-1a parser.

func parseAt(t *testing.T, cwd, text string, opts Options) (*Trace, error) {
	t.Helper()
	return ParseOptions(strings.NewReader(text), cwd, opts)
}

func TestChildCwdIsSnapshotAtCloneNotAtFirstLine(t *testing.T) {
	text := "100  clone(child_stack=NULL, flags=SIGCHLD) = 101\n" +
		"100  chdir(\"/other\") = 0\n" +
		"101  openat(AT_FDCWD, \"data.txt\", O_RDONLY) = 3</repo/data.txt>\n" +
		"101  +++ exited with 0 +++\n" +
		"100  +++ exited with 0 +++\n"
	tr, err := parseAt(t, "/repo", text, Options{})
	if err != nil {
		t.Fatal(err)
	}
	for _, e := range tr.Events {
		if e.Pid == 101 && e.Syscall == "openat" && e.Cwd != "/repo" {
			t.Fatalf("child cwd %q, want /repo", e.Cwd)
		}
	}
	// A CLONE_FS child instead SHARES the cell, so the same sequence is a race.
	text = "100  clone(child_stack=NULL, flags=CLONE_FS|SIGCHLD) = 101\n" +
		"100  chdir(\"/other\") = 0\n"
	if _, err := parseAt(t, "/repo", text, Options{}); err == nil || err.Error() != "trace-incomplete:cwd-race" {
		t.Fatalf("%v", err)
	}
}

func TestCwdRace(t *testing.T) {
	share := "1  clone(child_stack=NULL, flags=CLONE_VM|CLONE_FS|SIGCHLD) = 2\n"
	wantReason(t, share+"2  chdir(\"/x\") = 0\n", reasonCwdRace)
	wantReason(t, share+"1  fchdir(3</x>) = 0\n", reasonCwdRace)
	// A failed chdir changes nothing, so it is not a race.
	mustParse(t, share+"1  chdir(\"/x\") = -1 ENOENT (No such file or directory)\n2  +++ exited with 0 +++\n1  +++ exited with 0 +++\n")
	// Once the sharer exits, the cell is private again.
	mustParse(t, share+"2  +++ exited with 0 +++\n1  chdir(\"/x\") = 0\n1  +++ exited with 0 +++\n")
}

func TestSplitCallUsesCwdAtStart(t *testing.T) {
	text := "1  chdir(\"/a\") = 0\n" +
		"1  openat(AT_FDCWD, \"f\", O_RDONLY <unfinished ...>\n" +
		"1  <... openat resumed>)  = 3</a/f>\n" +
		"1  chdir(\"/b\") = 0\n" +
		"1  +++ exited with 0 +++\n"
	tr := mustParse(t, text)
	if tr.Events[1].Cwd != "/a" || tr.Events[2].Cwd != "/a" {
		t.Fatalf("%+v", tr.Events)
	}
}

func TestOrphanLazyCreationRefused(t *testing.T) {
	// 2 is spawned at line 1 and appears only afterwards, but it was created at
	// that return; a pid that was never spawned is an orphan.
	wantReason(t, "1  stat(\"a\", 0x1) = 0\n5  stat(\"a\", 0x1) = 0\n", reasonOrphanPid)
}

func TestDeepSpawnChainIsIterative(t *testing.T) {
	build := func(n int) string {
		var b strings.Builder
		// The deepest descendant speaks first, so its whole ancestor chain has
		// to be created at once.
		b.WriteString("1  stat(\"a\", 0x1) = 0\n")
		fmt.Fprintf(&b, "%d  stat(\"a\", 0x1) = 0\n", n)
		for k := 1; k < n; k++ {
			fmt.Fprintf(&b, "%d  fork( <unfinished ...>\n", k)
		}
		fmt.Fprintf(&b, "%d  +++ exited with 0 +++\n", n)
		for k := n - 1; k >= 1; k-- {
			fmt.Fprintf(&b, "%d  <... fork resumed>) = %d\n%d  +++ exited with 0 +++\n", k, k+1, k)
		}
		return b.String()
	}
	tr := mustParse(t, build(200))
	if len(tr.Procs) != 200 {
		t.Fatalf("%d procs", len(tr.Procs))
	}
	wantReason(t, build(maxProcessDepth+10), reasonProcessCycle)
}

func TestArityBoundedWhileScanning(t *testing.T) {
	many := strings.Repeat("0,", 100) + "0"
	wantReason(t, wrap(`stat(`+many+`) = 0`), reasonArity)
	wantReason(t, wrap(`frobnicate(`+many+`) = 0`), reasonArity)
	wantReason(t, wrap(`getcwd(`+many+`) = 0`), reasonArity)
	// An unterminated megabyte of commas is refused without building it.
	wantReason(t, wrap(`stat(`+strings.Repeat(",", 1000)), reasonUnparsable)
	// Exactly the cap for an ignored call is fine.
	mustParse(t, wrap(`getcwd(`+strings.Repeat("0,", maxUnknownItems-1)+`0) = 0`))
}

func TestEveryRecordCountsAndDefaultsAreBounded(t *testing.T) {
	var b strings.Builder
	b.WriteString("1  fork() = 2\n")
	for i := 0; i < 50; i++ {
		fmt.Fprintf(&b, "2  +++ exited with %d +++\n", i)
	}
	if _, err := Parse(strings.NewReader(b.String()), "/", Limits{MaxEvents: 10}); err == nil || err.Error() != "trace-incomplete:size" {
		t.Fatalf("%v", err)
	}
	// Start markers count as well.
	var s strings.Builder
	for i := 0; i < 50; i++ {
		fmt.Fprintf(&s, "1  stat(\"a\", 0x1 <unfinished ...>\n1  <... stat resumed>) = 0\n")
	}
	if _, err := Parse(strings.NewReader(s.String()), "/", Limits{MaxEvents: 20}); err == nil || err.Error() != "trace-incomplete:size" {
		t.Fatalf("%v", err)
	}
	d := DefaultLimits()
	if d.MaxTotalBytes != 64<<20 || d.MaxEvents != 1_000_000 {
		t.Fatalf("%+v", d)
	}
}

func TestFinalLineNeedsNewline(t *testing.T) {
	wantReason(t, "1  stat(\"a\", 0x1) = 0\n1  +++ exited with 0 +++", reasonUnparsable)
	wantReason(t, "1  +++ exited with 1", reasonUnparsable)
}

func TestNullAndNonSymbolicAddresses(t *testing.T) {
	wantReason(t, wrap(`socket(0x2, SOCK_STREAM, 0) = 3<socket:[1]>`), reasonSockaddr)
	wantReason(t, wrap(`socket(0x7ffd, SOCK_STREAM, 0) = 3<socket:[1]>`), reasonSockaddr)
	wantReason(t, wrap(`connect(3<socket:[1]>, NULL, 0) = -1 EFAULT (Bad address)`), reasonSockaddr)
	wantReason(t, wrap(`bind(3<socket:[1]>, NULL, 0) = -1 EFAULT (Bad address)`), reasonSockaddr)
	mustParse(t, wrap(`sendto(3<socket:[1]>, "x", 1, 0, NULL, 0) = 1`))
}

func TestTruncatedMessageArrays(t *testing.T) {
	wantReason(t, wrap(`sendmsg(3<socket:[1]>, {msg_name=NULL, msg_iov=[{iov_base="a", iov_len=1}, ...], msg_iovlen=40}, 0) = 1`), reasonSockaddr)
	wantReason(t, wrap(`sendmmsg(3<socket:[1]>, [{msg_hdr={msg_name={sa_family=AF_INET, sin_port=htons(7), sin_addr=inet_addr("5.6.7.8")}}, msg_len=1}, ...], 40, 0) = 1`), reasonSockaddr)
	// "..." inside a struct (elided fields) is not an array truncation.
	mustParse(t, wrap(`sendmsg(3<socket:[1]>, {msg_name=NULL, ...}, 0) = 1`))
	// A truncated string is still just a string.
	mustParse(t, wrap(`sendmsg(3<socket:[1]>, {msg_name=NULL, msg_iov=[{iov_base="a"..., iov_len=99}]}, 0) = 1`))
}

func TestEmptyPathWithDescriptor(t *testing.T) {
	tr := mustParse(t, wrap(
		`statx(3</w/f>, "", AT_EMPTY_PATH, STATX_ALL, {}) = 0`,
		`newfstatat(4</w/g>, "", {}, AT_EMPTY_PATH|AT_SYMLINK_NOFOLLOW) = 0`,
		`statx(3</w/f>, "", 0, STATX_ALL, {}) = -1 ENOENT (No such file or directory)`,
	))
	for i, want := range []string{"/w/f", "/w/g", ""} {
		if tr.Events[i].Path != want || (want != "" && tr.Events[i].DirfdPath != "") {
			t.Fatalf("event %d: %+v", i, tr.Events[i])
		}
	}
	if tr.Events[2].DirfdPath != "/w/f" {
		t.Fatalf("no AT_EMPTY_PATH keeps the dirfd: %+v", tr.Events[2])
	}
}

func TestExitPointersDoNotAlias(t *testing.T) {
	var b strings.Builder
	const n = 300
	for i := 0; i < n; i++ {
		b.WriteString("1  fork() = " + fmt.Sprint(i+2) + "\n")
	}
	for i := 0; i < n; i++ {
		fmt.Fprintf(&b, "%d  +++ exited with %d +++\n", i+2, i%200)
	}
	b.WriteString("1  +++ exited with 9 +++\n")
	tr := mustParse(t, b.String())
	for i := 0; i < n; i++ {
		p := tr.Procs[i+2]
		if p.Exit == nil || p.Exit.Pid != i+2 || p.Exit.Code != i%200 {
			t.Fatalf("pid %d: %+v", i+2, p.Exit)
		}
	}
	tr.Exits[0].Code = 77 // mutating the slice does not change the process view
	if tr.Procs[2].Exit.Code == 77 {
		t.Fatal("aliased")
	}
}

func TestPidLeadingZeros(t *testing.T) {
	wantReason(t, "01  stat(\"a\", 0x1) = 0\n", reasonUnparsable)
	wantReason(t, "[pid 007] stat(\"a\", 0x1) = 0\n", reasonUnparsable)
	mustParse(t, "10  +++ exited with 0 +++\n")
}

// The sandbox helper's process tree: helper (10) spawns one init child (11),
// which does the setup and then execs the step.
const helperRootSetup = "10  unshare(CLONE_NEWUSER) = 0\n" +
	"10  clone(child_stack=NULL, flags=CLONE_NEWPID|SIGCHLD) = 11\n"

const helperInitSetup = "11  unshare(CLONE_NEWNS) = 0\n" +
	"11  mount(\"none\", \"/\", NULL, MS_REC|MS_PRIVATE, NULL) = 0\n" +
	"11  pivot_root(\".\", \".\") = 0\n" +
	"11  chdir(\"/\") = 0\n" +
	"11  prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) = 0\n" +
	"11  seccomp(SECCOMP_SET_MODE_FILTER, 0, 0x1) = 0\n"

const helperExec = "11  execve(\"/bin/sh\", [\"sh\", \"-c\", \"make\"], 0x0 /* 1 vars */) = 0\n"

const helperEnd = "11  +++ exited with 5 +++\n10  wait4(11, [{WIFEXITED(s) && WEXITSTATUS(s) == 5}], 0, NULL) = 11\n10  +++ exited with 0 +++\n"

func TestHelperRootHappyPath(t *testing.T) {
	opts := Options{HelperRoot: true}
	text := helperRootSetup + helperInitSetup + helperExec +
		"11  openat(AT_FDCWD, \"a\", O_RDONLY) = 3</start/a>\n" + helperEnd
	tr, err := parseAt(t, "/start", text, opts)
	if err != nil {
		t.Fatal(err)
	}
	// Setup (unshare, mount, pivot_root, chdir) and every root event are gone.
	if len(tr.Events) != 2 || tr.Events[0].Syscall != "execve" || tr.Events[1].Syscall != "openat" {
		t.Fatalf("%+v", tr.Events)
	}
	for _, e := range tr.Events {
		if e.Pid != 11 || e.Cwd != "/start" {
			t.Fatalf("event %+v: want pid 11 in the executor's cwd", e)
		}
	}
	if tr.InitPid != 11 || tr.RootPid != 10 || tr.Procs[11].Parent != 10 {
		t.Fatalf("%d %d", tr.InitPid, tr.RootPid)
	}
	if ex := tr.StepExit(); ex == nil || ex.Pid != 11 || ex.Code != 5 {
		t.Fatalf("step exit %+v", ex)
	}
	if tr.Procs[10].Exit.Code != 0 {
		t.Fatal("root exit")
	}
	// Without the option the same trace is refused, and StepExit is the root's.
	_, err = parseAt(t, "/start", text, Options{})
	if err == nil || err.Error() != "trace-incomplete:refused:unshare" {
		t.Fatalf("%v", err)
	}
	plain := mustParse(t, wrap(`stat("a", 0x1) = 0`))
	if plain.InitPid != 0 || plain.StepExit().Pid != 1 {
		t.Fatal("plain StepExit")
	}
	if (&Trace{}).StepExit() != nil {
		t.Fatal("empty StepExit")
	}
}

func TestHelperRootRules(t *testing.T) {
	opts := Options{HelperRoot: true}
	ok := helperRootSetup + helperInitSetup + helperExec
	// The root spawns twice.
	wantReasonOpts(t, helperRootSetup+"10  fork() = 12\n"+helperInitSetup+helperExec+"12  +++ exited with 0 +++\n"+helperEnd, opts, reasonHelperSpawn)
	// The root does a handled call after the spawn, or a refused or unknown one.
	for _, call := range []string{"stat(\"a\", 0x1) = 0", "mount(\"a\", \"b\", NULL, 0, NULL) = 0", "capset(0x1) = 0"} {
		wantReasonOpts(t, helperRootSetup+"10  "+call+"\n"+helperInitSetup+helperExec+helperEnd, opts, reasonHelperActivity)
	}
	// ... but ignored calls are fine, and handled calls BEFORE the spawn are setup.
	mustHelper(t, "10  stat(\"a\", 0x1) = 0\n"+helperRootSetup+"10  close(3) = 0\n"+helperInitSetup+helperExec+helperEnd)
	// The init child spawns before exec.
	wantReasonOpts(t, helperRootSetup+"11  fork() = 12\n12  +++ exited with 0 +++\n"+helperExec+helperEnd, opts, reasonHelperSpawn)
	// The child never execs, or its exec fails.
	wantReasonOpts(t, helperRootSetup+helperInitSetup+"11  +++ exited with 1 +++\n10  +++ exited with 1 +++\n", opts, reasonHelperNoExec)
	wantReasonOpts(t, helperRootSetup+"11  execve(\"/bin/sh\", [\"sh\"], 0x0) = -1 ENOENT (No such file or directory)\n11  +++ exited with 1 +++\n10  +++ exited with 1 +++\n", opts, reasonHelperNoExec)
	// The root never spawns at all.
	wantReasonOpts(t, "10  unshare(CLONE_NEWUSER) = 0\n10  +++ exited with 0 +++\n", opts, reasonHelperNoInit)
	// After the exec, refused and unknown calls are refused as usual, and the
	// step may spawn.
	wantReasonOpts(t, ok+"11  mount(\"a\", \"b\", NULL, 0, NULL) = 0\n"+helperEnd, opts, "refused:mount")
	wantReasonOpts(t, ok+"11  capset(0x1) = 0\n"+helperEnd, opts, "unknown-syscall:capset")
	mustHelper(t, ok+"11  fork() = 12\n12  +++ exited with 0 +++\n"+helperEnd)
	// A split setup call (unknown name) is paired and discarded.
	mustHelper(t, helperRootSetup+"11  mount(\"a\", \"b\", NULL, 0 <unfinished ...>\n11  <... mount resumed>, NULL) = 0\n"+helperExec+helperEnd)
	// A setup call that parses but is odd does not leak into the events.
	tr := mustHelper(t, helperRootSetup+"11  openat(AT_FDCWD, \"/proc/self/uid_map\", O_WRONLY) = 3</proc/1/uid_map>\n"+helperExec+helperEnd)
	if len(tr.Events) != 1 {
		t.Fatalf("%+v", tr.Events)
	}
	// Setup chdirs are discarded: an absolute exec starts at the executor's cwd,
	// a relative one after a chdir is refused.
	tr = mustHelper(t, helperRootSetup+"11  chdir(\"/elsewhere\") = 0\n"+helperExec+helperEnd)
	if tr.Events[0].Cwd != "/start" {
		t.Fatalf("%+v", tr.Events[0])
	}
	wantReasonOpts(t, helperRootSetup+"11  chdir(\"/elsewhere\") = 0\n11  execve(\"sh\", [\"sh\"], 0x0) = 0\n"+helperEnd, opts, reasonHelperCwd)
	mustHelper(t, helperRootSetup+"11  execve(\"sh\", [\"sh\"], 0x0) = 0\n"+helperEnd)
}

func mustHelper(t *testing.T, text string) *Trace {
	t.Helper()
	tr, err := parseAt(t, "/start", text, Options{HelperRoot: true})
	if err != nil {
		t.Fatalf("%v\n%s", err, text)
	}
	return tr
}

func wantReasonOpts(t *testing.T, text string, opts Options, reason string) {
	t.Helper()
	_, err := parseAt(t, "/start", text, opts)
	if err == nil || err.Error() != "trace-incomplete:"+reason {
		t.Fatalf("want %s, got %v", reason, err)
	}
}
