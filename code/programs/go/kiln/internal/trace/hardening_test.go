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

// The sandbox helper's process tree: the Kiln binary (10) execs, spawns one
// init child (11), which sets up, re-execs the Kiln binary, and then execs the
// step.
const kilnExe = "/usr/bin/kiln"

var helperOpts = Options{HelperRoot: true, HelperExe: kilnExe}

const helperRootExec = "10  execve(\"" + kilnExe + "\", [\"kiln\", \"sandbox-exec\"], 0x0 /* 1 vars */) = 0\n"

const helperRootSetup = helperRootExec +
	"10  unshare(CLONE_NEWUSER) = 0\n" +
	"10  clone(child_stack=NULL, flags=CLONE_NEWPID|SIGCHLD) = 11\n"

const helperInitSetup = "11  unshare(CLONE_NEWNS) = 0\n" +
	"11  mount(\"none\", \"/\", NULL, MS_REC|MS_PRIVATE, NULL) = 0\n" +
	"11  pivot_root(\".\", \".\") = 0\n" +
	"11  chdir(\"/start\") = 0\n" +
	"11  prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) = 0\n" +
	"11  seccomp(SECCOMP_SET_MODE_FILTER, 0, 0x1) = 0\n"

const helperReExec = "11  execve(\"" + kilnExe + "\", [\"kiln\", \"sandbox-init\"], 0x0 /* 1 vars */) = 0\n"

const helperExec = "11  execve(\"/bin/sh\", [\"sh\", \"-c\", \"make\"], 0x0 /* 1 vars */) = 0\n"

const helperEnd = "11  +++ exited with 5 +++\n10  wait4(11, [{WIFEXITED(s) && WEXITSTATUS(s) == 5}], 0, NULL) = 11\n10  +++ exited with 0 +++\n"

const threadFlags = "CLONE_VM|CLONE_SIGHAND|CLONE_THREAD|CLONE_SYSVSEM"

func TestHelperRootHappyPath(t *testing.T) {
	text := helperRootSetup + helperInitSetup + helperReExec + "11  mount(\"a\", \"b\", NULL, 0, NULL) = 0\n" + helperExec +
		"11  openat(AT_FDCWD, \"a\", O_RDONLY) = 3</start/a>\n" + helperEnd
	tr, err := parseAt(t, "/start", text, helperOpts)
	if err != nil {
		t.Fatal(err)
	}
	// Setup (unshare, mount, pivot_root, chdir, the re-exec) and every root event are gone.
	if len(tr.Events) != 2 || tr.Events[0].Syscall != "execve" || tr.Events[0].Path != "/bin/sh" || tr.Events[1].Syscall != "openat" {
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
	// /proc/self/exe is the same binary.
	mustHelper(t, helperRootSetup+"11  execve(\"/proc/self/exe\", [\"kiln\"], 0x0) = 0\n"+helperExec+helperEnd)
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
	// HelperExe is required.
	for _, exe := range []string{"", "kiln", "/a\x00b"} {
		wantReasonOpts(t, text, Options{HelperRoot: true, HelperExe: exe}, reasonHelperConfig)
	}
}

func TestHelperRootRules(t *testing.T) {
	opts := helperOpts
	ok := helperRootSetup + helperInitSetup + helperExec
	// The root spawns twice.
	wantReasonOpts(t, helperRootSetup+"10  fork() = 12\n"+helperInitSetup+helperExec+"12  +++ exited with 0 +++\n"+helperEnd, opts, reasonHelperSpawn)
	// The root does a handled call after the spawn, or a refused or unknown one.
	for _, call := range []string{"stat(\"a\", 0x1) = 0", "mount(\"a\", \"b\", NULL, 0, NULL) = 0", "capset(0x1) = 0", "execve(\"/bin/sh\", [\"sh\"], 0x0) = 0"} {
		wantReasonOpts(t, helperRootSetup+"10  "+call+"\n"+helperInitSetup+helperExec+helperEnd, opts, reasonHelperActivity)
	}
	// ... but ignored calls are fine, and handled calls BEFORE the spawn are setup.
	mustHelper(t, helperRootExec+"10  stat(\"a\", 0x1) = 0\n"+helperRootSetup[len(helperRootExec):]+"10  close(3) = 0\n"+helperInitSetup+helperExec+helperEnd)
	// The root execs the shell: a second exec of the root.
	wantReasonOpts(t, helperRootExec+"10  execve(\"/bin/sh\", [\"sh\"], 0x0) = 0\n"+helperRootSetup[len(helperRootExec):]+helperInitSetup+helperExec+helperEnd, opts, reasonHelperActivity)
	// The root's FIRST exec is not the Kiln binary.
	wantReasonOpts(t, "10  execve(\"/bin/sh\", [\"sh\"], 0x0) = 0\n"+helperRootSetup[len(helperRootExec):]+helperInitSetup+helperExec+helperEnd, opts, reasonHelperExe)
	// A failed root exec does not count.
	mustHelper(t, "10  execve(\"/bin/nope\", [\"x\"], 0x0) = -1 ENOENT (No such file or directory)\n"+helperRootSetup+helperInitSetup+helperExec+helperEnd)
	// The init child spawns (non-thread) in its window.
	wantReasonOpts(t, helperRootSetup+"11  fork() = 2 /* 12 in strace's PID NS */\n12  +++ exited with 0 +++\n"+helperExec+helperEnd, opts, reasonHelperSpawn)
	wantReasonOpts(t, helperRootSetup+"11  clone(child_stack=NULL, flags=SIGCHLD) = 2 /* 12 in strace's PID NS */\n12  +++ exited with 0 +++\n"+helperExec+helperEnd, opts, reasonHelperSpawn)
	// The child never execs the step (the re-exec alone is not enough), or its exec fails.
	wantReasonOpts(t, helperRootSetup+helperInitSetup+helperReExec+"11  +++ exited with 1 +++\n10  +++ exited with 1 +++\n", opts, reasonHelperNoExec)
	wantReasonOpts(t, helperRootSetup+"11  execve(\"/bin/sh\", [\"sh\"], 0x0) = -1 ENOENT (No such file or directory)\n11  +++ exited with 1 +++\n10  +++ exited with 1 +++\n", opts, reasonHelperNoExec)
	// The root never spawns at all.
	wantReasonOpts(t, helperRootExec+"10  unshare(CLONE_NEWUSER) = 0\n10  +++ exited with 0 +++\n", opts, reasonHelperNoInit)
	// After the step's exec, refused and unknown calls are refused as usual, and
	// the step may spawn.
	wantReasonOpts(t, ok+"11  mount(\"a\", \"b\", NULL, 0, NULL) = 0\n"+helperEnd, opts, "refused:mount")
	wantReasonOpts(t, ok+"11  capset(0x1) = 0\n"+helperEnd, opts, "unknown-syscall:capset")
	mustHelper(t, ok+"11  fork() = 2 /* 12 in strace's PID NS */\n12  +++ exited with 0 +++\n"+helperEnd)
	// A split setup call (unknown name) is paired and discarded.
	mustHelper(t, helperRootSetup+"11  mount(\"a\", \"b\", NULL, 0 <unfinished ...>\n11  <... mount resumed>, NULL) = 0\n"+helperExec+helperEnd)
	// A setup call that parses but is odd does not leak into the events.
	tr := mustHelper(t, helperRootSetup+"11  openat(AT_FDCWD, \"/proc/self/uid_map\", O_WRONLY) = 3</proc/1/uid_map>\n"+helperExec+helperEnd)
	if len(tr.Events) != 1 {
		t.Fatalf("%+v", tr.Events)
	}
}

func TestHelperFailedExecProbesAreKept(t *testing.T) {
	tr := mustHelper(t, helperRootSetup+helperInitSetup+helperReExec+
		"11  execve(\"/usr/local/bin/sh\", [\"sh\"], 0x0) = -1 ENOENT (No such file or directory)\n"+
		"11  execve(\"/usr/bin/sh\", [\"sh\"], 0x0) = -1 EACCES (Permission denied)\n"+
		"11  execve(\""+kilnExe+"\", [\"kiln\"], 0x0) = -1 ENOENT (No such file or directory)\n"+
		helperExec+helperEnd)
	if len(tr.Events) != 3 {
		t.Fatalf("%+v", tr.Events)
	}
	if e := tr.Events[0]; e.Path != "/usr/local/bin/sh" || e.Result.OK || e.Result.Errno != "ENOENT" || e.Cwd != "/start" {
		t.Fatalf("%+v", e)
	}
	if tr.Events[1].Result.Errno != "EACCES" || !tr.Events[2].Result.OK || tr.Events[2].Path != "/bin/sh" {
		t.Fatalf("%+v", tr.Events)
	}
}

func TestHelperStepCwdMustEqualInitCwd(t *testing.T) {
	// Every exec path form: absolute after a setup chdir("/") ...
	wantReasonOpts(t, helperRootSetup+"11  chdir(\"/\") = 0\n"+helperExec+helperEnd, helperOpts, reasonHelperCwd)
	// ... relative ...
	wantReasonOpts(t, helperRootSetup+"11  chdir(\"/elsewhere\") = 0\n11  execve(\"sh\", [\"sh\"], 0x0) = 0\n"+helperEnd, helperOpts, reasonHelperCwd)
	// ... and a chdir back to the executor's directory is fine.
	mustHelper(t, helperRootSetup+"11  chdir(\"/elsewhere\") = 0\n11  chdir(\"/start\") = 0\n"+helperExec+helperEnd)
	mustHelper(t, helperRootSetup+"11  execve(\"sh\", [\"sh\"], 0x0) = 0\n"+helperEnd)
}

func TestHelperThreads(t *testing.T) {
	// Root threads: created before the init spawn, they run, even spawn the init
	// child, and are discarded.
	root := helperRootExec +
		"10  clone(child_stack=0x7f0000001000, flags=" + threadFlags + ", parent_tid=[12], tls=0x1) = 12\n" +
		"12  stat(\"/x\", 0x1) = 0\n" +
		"12  clone(child_stack=NULL, flags=CLONE_NEWPID|SIGCHLD) = 11\n"
	text := root + helperInitSetup + helperExec +
		"12  +++ exited with 0 +++\n" + helperEnd
	tr := mustHelper(t, text)
	if tr.InitPid != 11 || len(tr.Events) != 1 || tr.Procs[12].Parent != 10 {
		t.Fatalf("%d %+v", tr.InitPid, tr.Events)
	}
	// A root thread calling after the spawn is helper-activity, and so is a
	// second non-thread spawn by a thread.
	wantReasonOpts(t, root+"12  stat(\"/y\", 0x1) = 0\n"+helperInitSetup+helperExec+"12  +++ exited with 0 +++\n"+helperEnd, helperOpts, reasonHelperActivity)
	wantReasonOpts(t, root+"12  fork() = 13\n13  +++ exited with 0 +++\n"+helperInitSetup+helperExec+"12  +++ exited with 0 +++\n"+helperEnd, helperOpts, reasonHelperSpawn)
	// Init threads made during setup: part of the init group, discarded, and gone
	// (or silent) by the step's exec.
	initThread := helperRootSetup +
		"11  clone(child_stack=0x7f0000001000, flags=" + threadFlags + ", parent_tid=[3 /* 13 in strace's PID NS */], tls=0x1) = 3 /* 13 in strace's PID NS */\n" +
		"13  stat(\"/z\", 0x1) = 0\n" +
		"13  mount(\"a\", \"b\", NULL, 0, NULL) = 0\n" +
		"13  +++ exited with 0 +++\n"
	tr = mustHelper(t, initThread+helperInitSetup+helperExec+helperEnd)
	if len(tr.Events) != 1 {
		t.Fatalf("%+v", tr.Events)
	}
	// A window thread still making calls after the step's exec.
	late := helperRootSetup +
		"11  clone(child_stack=0x7f0000001000, flags=" + threadFlags + ", parent_tid=[3 /* 13 in strace's PID NS */], tls=0x1) = 3 /* 13 in strace's PID NS */\n" +
		helperExec +
		"13  stat(\"/late\", 0x1) = 0\n13  +++ exited with 0 +++\n" + helperEnd
	wantReasonOpts(t, late, helperOpts, reasonHelperActivity)
	// A thread the STEP makes after its exec is part of the step.
	step := helperRootSetup + helperExec +
		"11  clone(child_stack=0x7f0000001000, flags=" + threadFlags + ", parent_tid=[3 /* 13 in strace's PID NS */], tls=0x1) = 3 /* 13 in strace's PID NS */\n" +
		"13  stat(\"/ok\", 0x1) = 0\n13  +++ exited with 0 +++\n" + helperEnd
	tr = mustHelper(t, step)
	if len(tr.Events) != 3 || tr.Events[2].Pid != 13 {
		t.Fatalf("%+v", tr.Events)
	}
	// A thread clone with bad flags in the init window is still refused.
	wantReasonOpts(t, helperRootSetup+"11  clone(child_stack=NULL, flags=CLONE_THREAD|CLONE_UNTRACED) = 13\n13  +++ exited with 0 +++\n"+helperExec+helperEnd, helperOpts, reasonCloneFlags)
}

func TestNumericEmptyPathFlags(t *testing.T) {
	tr := mustParse(t, wrap(
		`statx(3</w/f>, "", 0x1000, STATX_ALL, {}) = 0`,
		`statx(3</w/g>, "", 4096, STATX_ALL, {}) = 0`,
		`statx(3</w/h>, "", AT_NO_AUTOMOUNT|0x1100, STATX_ALL, {}) = 0`,
		`statx(3</w/i>, "", 0x100, STATX_ALL, {}) = -1 ENOENT (No such file or directory)`,
		`statx(3</w/j>, "", 0, STATX_ALL, {}) = -1 ENOENT (No such file or directory)`,
	))
	want := []string{"/w/f", "/w/g", "/w/h", "", ""}
	for i, w := range want {
		if tr.Events[i].Path != w {
			t.Fatalf("event %d: %+v", i, tr.Events[i])
		}
	}
	if tr.Events[3].DirfdPath != "/w/i" {
		t.Fatalf("%+v", tr.Events[3])
	}
	for _, f := range []string{"0x", "0xzz", "99999999999999999999", "lower", "0x1000x", "A|"} {
		wantReason(t, wrap(`statx(3</w/f>, "", `+f+`, STATX_ALL, {}) = 0`), reasonFlags)
	}
}

func mustHelper(t *testing.T, text string) *Trace {
	t.Helper()
	tr, err := parseAt(t, "/start", text, helperOpts)
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

func TestRootGroupWalkIsLinear(t *testing.T) {
	// A deep chain of init-group threads whose last thread forks many times:
	// every fork asks whether its parent is in the root group, and the answer
	// must come from the memo, not from re-walking the chain.
	const depth, forks = 3000, 5000
	var b strings.Builder
	b.WriteString("10  clone(child_stack=NULL, flags=CLONE_NEWPID|SIGCHLD) = 11\n")
	for k := 11; k < 11+depth; k++ {
		fmt.Fprintf(&b, "%d  clone(child_stack=0x1, flags=%s, parent_tid=[%d], tls=0x1) = %d /* %d in strace's PID NS */\n", k, threadFlags, k+1, k+1, k+1)
	}
	deepest := 11 + depth
	for i := 0; i < forks; i++ {
		fmt.Fprintf(&b, "%d  fork() = %d /* %d in strace's PID NS */\n", deepest, 100000+i, 100000+i)
	}
	lim := Limits{}.withDefaults()
	recs, root, e := readRecords(strings.NewReader(b.String()), lim, true)
	if e != nil {
		t.Fatal(e)
	}
	m := newModel(lim, helperOpts, root, "/start")
	if e := m.buildSpawns(recs); e != nil {
		t.Fatal(e)
	}
	if m.initPid != 11 {
		t.Fatalf("init %d", m.initPid)
	}
	if m.walkSteps > 4*(depth+forks) {
		t.Fatalf("%d walk steps for %d threads and %d forks: not linear", m.walkSteps, depth, forks)
	}
	// And the answers are right: the chain is not the root group (the init is a
	// non-thread child), the root's own threads are.
	if m.inRootGroup(deepest) || !m.inRootGroup(10) {
		t.Fatal("group membership")
	}
}

func TestHelperUnshareFSMakesCwdPrivate(t *testing.T) {
	// A threaded init: a thread shares the cwd, so a plain chdir is a race ...
	thread := "11  clone(child_stack=0x1, flags=" + threadFlags + "|CLONE_FS|CLONE_FILES) = 2 /* 12 in strace's PID NS */\n" +
		"12  futex(0x1, FUTEX_WAIT_PRIVATE, 0, NULL) = 0\n"
	body := helperRootSetup + thread + "11  chdir(\"/elsewhere\") = 0\n11  chdir(\"/start\") = 0\n" + helperExec +
		"12  +++ exited with 0 +++\n" + helperEnd
	wantReasonOpts(t, body, helperOpts, reasonCwdRace)
	// ... and unshare(CLONE_FS) first (by name, or as a number) makes it private.
	for _, arg := range []string{"CLONE_FS", "CLONE_FS|CLONE_NEWNS", "0x200", "CLONE_NEWNS|0x200"} {
		priv := helperRootSetup + thread + "11  unshare(" + arg + ") = 0\n11  chdir(\"/elsewhere\") = 0\n11  chdir(\"/start\") = 0\n" + helperExec +
			"12  +++ exited with 0 +++\n" + helperEnd
		mustHelper(t, priv)
	}
	// A failed unshare, or one without CLONE_FS, changes nothing.
	for _, u := range []string{"11  unshare(CLONE_FS) = -1 EPERM (Operation not permitted)\n", "11  unshare(CLONE_NEWNS) = 0\n", "11  unshare(0x20000) = 0\n"} {
		wantReasonOpts(t, helperRootSetup+thread+u+"11  chdir(\"/elsewhere\") = 0\n"+helperExec+"12  +++ exited with 0 +++\n"+helperEnd, helperOpts, reasonCwdRace)
	}
}

func TestPidnsTranslationOfSpawns(t *testing.T) {
	// Inside the sandbox's PID namespace a returned pid is namespace-local
	// ("= 2") and strace --pidns-translation says which host pid it is.  The
	// spawn is recorded under the HOST pid, the one the line prefixes use.
	note := " /* 12 in strace's PID NS */"
	thread := "11  clone(child_stack=0x1, flags=" + threadFlags + ", parent_tid=[2" + note + "], tls=0x1) = 2" + note + "\n" +
		"12  stat(\"/z\", 0x1) = 0\n12  +++ exited with 0 +++\n"
	tr := mustHelper(t, helperRootSetup+thread+helperExec+helperEnd)
	if p := tr.Procs[12]; p == nil || p.Parent != 11 {
		t.Fatalf("the translated thread: %+v", tr.Procs[12])
	}
	if tr.Procs[2] != nil {
		t.Error("the namespace-local number became a process")
	}
	// Without the annotation the local number cannot be matched to anything: a
	// spawn by the init or its descendants must carry it.
	bare := "11  clone(child_stack=0x1, flags=" + threadFlags + ", tls=0x1) = 2\n"
	wantReasonOpts(t, helperRootSetup+bare+helperExec+helperEnd, helperOpts, reasonPidnsUntranslated)
	// ... and so must fork and vfork.
	for _, call := range []string{"fork()", "vfork()", "clone(child_stack=NULL, flags=SIGCHLD)"} {
		wantReasonOpts(t, helperRootSetup+"11  "+call+" = 2\n"+helperExec+helperEnd, helperOpts, reasonPidnsUntranslated)
	}
	// The helper root lives in strace's own namespace: no annotation, no fault.
	mustHelper(t, helperRootExec+"10  clone(child_stack=0x1, flags="+threadFlags+", tls=0x1) = 20\n20  +++ exited with 0 +++\n"+helperRootSetup[len(helperRootExec):]+helperInitSetup+helperExec+helperEnd)
}

func TestPidnsNoteIsExact(t *testing.T) {
	cases := map[string]bool{
		" /* 5030 in strace's PID NS */":        true,
		"/* 5030 in strace's PID NS */":         true,
		" /* 0 in strace's PID NS */":           false,
		" /* -3 in strace's PID NS */":          false,
		" /* x in strace's PID NS */":           false,
		" /* 5030 in strace's PID NS*/":         false,
		" /* 5030 in another PID NS */":         false,
		" /* 5030 */":                           false,
		" // 5030 in strace's PID NS */":        false,
		" /* 12345678901 in strace's PID NS */": false,
		"":                                      false,
	}
	for in, want := range cases {
		if _, _, ok := parsePidnsNote(in); ok != want {
			t.Errorf("%q: %v, want %v", in, ok, want)
		}
	}
	// A stray comment after a result is still refused.
	if _, r := parseResult("= 5 /* surprise */"); r == "" {
		t.Error("an unknown comment was accepted")
	}
	r, why := parseResult("= 2 /* 5030 in strace's PID NS */")
	if why != "" || r.value != 2 || r.host != 5030 || r.pid() != 5030 {
		t.Errorf("%+v %q", r, why)
	}
}

func TestPidnsNoteBoundsAndPlacement(t *testing.T) {
	// L1: no leading zero, at most 9 digits.
	for _, note := range []string{" /* 007 in strace's PID NS */", " /* 0012 in strace's PID NS */", " /* 1234567890 in strace's PID NS */", " /* 99999999999999999999 in strace's PID NS */"} {
		if _, _, ok := parsePidnsNote(note); ok {
			t.Errorf("%q accepted", note)
		}
		wantReasonOpts(t, helperRootSetup+"11  fork() = 2"+note+"\n12  +++ exited with 0 +++\n"+helperExec+helperEnd, helperOpts, reasonResult)
	}
	for _, note := range []string{" /* 7 in strace's PID NS */", " /* 999999999 in strace's PID NS */"} {
		if n, _, ok := parsePidnsNote(note); !ok || n <= 0 {
			t.Errorf("%q refused", note)
		}
	}
	// L2: only a spawn (or a wait that returns a pid) may carry the note.
	note := " /* 12 in strace's PID NS */"
	for _, call := range []string{`stat("/x", 0x1)`, `openat(AT_FDCWD, "/x", O_RDONLY)`, `execve("/x", ["x"], 0x0)`, `chdir("/x")`, `getdents64(3</d>, 0x1, 32768)`} {
		wantReasonOpts(t, helperRootSetup+"11  "+call+" = 3"+note+"\n"+helperExec+helperEnd, helperOpts, reasonResult)
	}
	// ... also outside helper mode, and on an ignored-class call that is not a wait.
	if _, err := parseAt(t, "/r", "1  getcwd(\"/r\", 4096) = 3"+note+"\n1  +++ exited with 0 +++\n", Options{}); err == nil || err.Error() != "trace-incomplete:result" {
		t.Errorf("%v", err)
	}
	// A wait returns a pid; strace translates it too.
	mustHelper(t, helperExec[:0]+helperRootSetup+"11  wait4(-1, 0x1, 0, NULL) = 2"+note+"\n"+helperExec+helperEnd)
}
