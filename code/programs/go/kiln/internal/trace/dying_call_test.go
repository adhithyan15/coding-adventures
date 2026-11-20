package trace

import "testing"

// The dying-call rule (KILN04 D35).  When a thread group exits, the kernel kills
// its other threads wherever they are, and strace prints three shapes for a
// thread that dies inside a system call:
//
//  1. "<... tgkill resumed>) = ? <unavailable>"  strace could not fetch the
//     result because the tracee was already gone;
//  2. "<... clone resumed> <unfinished ...>) = ?"  printed at the thread's
//     PTRACE_EVENT_EXIT, the result unknown;
//  3. "openat(AT_FDCWD, 0x7f.., O_RDONLY <unfinished ...>" and then the exit
//     with no resume: killed out of its syscall-entry stop, so the path prints
//     as an address.
//
// Every Parse here runs both readers and the cross-check hook
// (crosscheck_init_test.go), so a disagreement between them panics.  The tests
// pin what is accepted, and each way in which a near miss stays refused.

const (
	// dyThread is a glibc/Go thread clone: it shares everything, so pid 2 is in
	// pid 1's thread group.
	dyThread = "flags=CLONE_VM|CLONE_FS|CLONE_FILES|CLONE_SIGHAND|CLONE_THREAD|CLONE_SYSVSEM"

	dyClone    = "1  clone(child_stack=0x7f0000001000, " + dyThread + ") = 2\n"
	dyGroupEnd = "1  exit_group(0) = ?\n"
	dyExit2    = "2  +++ exited with 0 +++\n"
	dyExit1    = "1  +++ exited with 0 +++\n"
)

// dyTrace is a thread group of pid 1 with one thread, pid 2.  body is what pid 2
// and pid 1 print after the thread is made; the trace ends with both exits
// unless body supplies them.
func dyTrace(body string) string { return dyClone + body + dyExit2 + dyExit1 }

// The three shapes, each as the exact text strace prints, for the calls the
// rule drops: an ignored-class call, and a spawn with allowed flags.
func TestDyingCallShapesAreAccepted(t *testing.T) {
	shapes := map[string]string{
		"1 tgkill resumed unavailable": "2  tgkill(1, 1, SIGURG <unfinished ...>\n" +
			dyGroupEnd + "2  <... tgkill resumed>) = ? <unavailable>\n",
		"2 clone resumed unfinished": "2  clone(child_stack=0x7f0000002000, " + dyThread + " <unfinished ...>\n" +
			dyGroupEnd + "2  <... clone resumed> <unfinished ...>) = ?\n",
		"3 clone killed at entry":  "2  clone(child_stack=0x7f0000002000, " + dyThread + " <unfinished ...>\n" + dyGroupEnd,
		"3 tgkill killed at entry": "2  tgkill(1, 1, SIGURG <unfinished ...>\n" + dyGroupEnd,
		"3 fork killed at entry":   "2  fork( <unfinished ...>\n" + dyGroupEnd,
		"3 vfork killed at entry":  "2  vfork( <unfinished ...>\n" + dyGroupEnd,
		"2 fork resumed unfinished": "2  fork( <unfinished ...>\n" + dyGroupEnd +
			"2  <... fork resumed> <unfinished ...>) = ?\n",
		// A whole-line call whose result is unavailable.
		"tgkill whole line": "2  tgkill(1, 1, SIGURG) = ? <unavailable>\n" +
			dyGroupEnd,
	}
	for name, body := range shapes {
		t.Run(name, func(t *testing.T) {
			tr := mustParse(t, dyTrace(body))
			// The dying call leaves no event and no process behind.
			for _, e := range tr.Events {
				if e.Pid == 2 {
					t.Errorf("an event of the dying thread was kept: %+v", e)
				}
			}
			if len(tr.Procs) != 2 {
				t.Errorf("%d processes, want 2: %+v", len(tr.Procs), tr.Procs)
			}
		})
	}
}

// The rule does not depend on Options.HelperRoot.  In the helper layout the step
// is the init child (pid 11) after its exec; a thread it makes is pid 55 in
// strace's own namespace.
func TestDyingCallInTheStep(t *testing.T) {
	const thread = "11  clone(child_stack=0x7f0000001000, " + dyThread + ") = 2 /* 55 in strace's PID NS */\n"
	const end = "11  exit_group(5) = ?\n55  +++ exited with 0 +++\n" + helperEnd
	pre := helperRootSetup + helperInitSetup + helperReExec + helperExec + thread
	for name, body := range map[string]string{
		"tgkill resumed": "55  tgkill(11, 11, SIGURG <unfinished ...>\n55  <... tgkill resumed>) = ? <unavailable>\n",
		"clone resumed": "55  clone(child_stack=0x7f0000002000, " + dyThread + " <unfinished ...>\n" +
			"55  <... clone resumed> <unfinished ...>) = ?\n",
	} {
		t.Run(name, func(t *testing.T) {
			// The group's exit_group comes after the thread's lines.
			mustHelper(t, pre+body+end)
		})
	}
	t.Run("a write is refused", func(t *testing.T) {
		text := pre + "55  openat(AT_FDCWD</start>, 0x7f3a6c001230, O_WRONLY|O_CREAT <unfinished ...>\n" + end
		e := refusal(t, text, true)
		if e.Reason != reasonUnresumed {
			t.Fatalf("refused %s, want %s", e.Reason, reasonUnresumed)
		}
	})
	// A call that reads is refused even with the group ending: the model infers
	// reads only from these records.
	for name, body := range map[string]string{
		"openat":     "55  openat(AT_FDCWD</start>, 0x7f3a6c001230, O_RDONLY|O_CLOEXEC <unfinished ...>\n",
		"newfstatat": "55  newfstatat(AT_FDCWD</start>, 0x7f3a6c001230, <unfinished ...>\n",
		"getdents64": "55  getdents64(3</start>, <unfinished ...>\n",
	} {
		t.Run("a read is refused: "+name, func(t *testing.T) {
			e := refusal(t, pre+body+end, true)
			if e.Reason != reasonUnresumed {
				t.Fatalf("refused %s, want %s", e.Reason, reasonUnresumed)
			}
		})
	}
	// A dying clone with flags that are not readable and allowed is refused, in
	// the helper layout as outside it.
	for name, flags := range dyBadCloneFlags {
		t.Run("clone "+name+" is refused", func(t *testing.T) {
			body := "55  clone(" + flags + " <unfinished ...>\n55  <... clone resumed> <unfinished ...>) = ?\n"
			e := refusal(t, pre+body+end, true)
			if e.Reason != reasonUnresumed {
				t.Fatalf("refused %s, want %s", e.Reason, reasonUnresumed)
			}
			body = "55  clone(" + flags + " <unfinished ...>\n"
			e = refusal(t, pre+body+end, true)
			if e.Reason != reasonUnresumed {
				t.Fatalf("at entry: refused %s, want %s", e.Reason, reasonUnresumed)
			}
		})
	}
}

func TestDyingCallRefused(t *testing.T) {
	open := func(flags string) string {
		return "2  openat(AT_FDCWD</work/repo>, 0x7f3a6c001230, " + flags + " <unfinished ...>\n"
	}
	cases := []struct {
		name, text, reason string
		line               int // the refused line; 0 = not checked
	}{
		// What may follow a dying call: only the pid's own exit.
		{"resumed unavailable then a call", dyTrace("2  tgkill(1, 1, SIGURG <unfinished ...>\n" + dyGroupEnd +
			"2  <... tgkill resumed>) = ? <unavailable>\n2  stat(\"/x\", 0x1) = 0\n"), reasonInterleaved, 5},
		{"resumed unavailable then a signal", dyTrace("2  tgkill(1, 1, SIGURG <unfinished ...>\n" + dyGroupEnd +
			"2  <... tgkill resumed>) = ? <unavailable>\n2  --- SIGCHLD {si_signo=SIGCHLD} ---\n"), reasonInterleaved, 5},
		{"resumed unavailable then a second resume", dyTrace("2  tgkill(1, 1, SIGURG <unfinished ...>\n" + dyGroupEnd +
			"2  <... tgkill resumed>) = ? <unavailable>\n2  <... tgkill resumed>) = ? <unavailable>\n"), reasonUnmatched, 5},
		{"clone resumed unfinished then a call", dyTrace("2  clone(child_stack=0x7f0000002000, " + dyThread + " <unfinished ...>\n" + dyGroupEnd +
			"2  <... clone resumed> <unfinished ...>) = ?\n2  getpid() = 2\n"), reasonInterleaved, 5},
		{"clone resumed unfinished, no exit", dyClone + dyGroupEnd +
			"2  clone(child_stack=0x7f0000002000, " + dyThread + " <unfinished ...>\n2  <... clone resumed> <unfinished ...>) = ?\n" + dyExit1, reasonUnresumed, 3},

		// The spelling is exact.
		{"unavailable with a value", dyTrace("2  stat(0x1, 0x2) = 3 <unavailable>\n" + dyGroupEnd), reasonResult, 2},
		{"unavailable with an errno", dyTrace("2  stat(0x1, 0x2) = -1 ENOENT <unavailable>\n" + dyGroupEnd), reasonResult, 2},
		{"unavailable glued to the question mark", dyTrace("2  stat(0x1, 0x2) = ?<unavailable>\n" + dyGroupEnd), reasonResult, 2},
		{"unavailable after two spaces", dyTrace("2  stat(0x1, 0x2) = ?  <unavailable>\n" + dyGroupEnd), reasonResult, 2},
		{"unavailable twice", dyTrace("2  stat(0x1, 0x2) = ? <unavailable> <unavailable>\n" + dyGroupEnd), reasonResult, 2},
		{"unavailable with trailing text", dyTrace("2  stat(0x1, 0x2) = ? <unavailable> x\n" + dyGroupEnd), reasonResult, 2},
		{"unavailable in the arguments", dyTrace("2  stat(<unavailable>, 0x2) = 0\n" + dyGroupEnd), reasonUnparsable, 2},
		{"clone resume with an argument", dyTrace("2  clone(child_stack=0x7f0000002000, " + dyThread + " <unfinished ...>\n" + dyGroupEnd +
			"2  <... clone resumed>, 0x1 <unfinished ...>) = ?\n"), reasonUnparsable, 4},
		{"clone resume with a result", dyTrace("2  clone(child_stack=0x7f0000002000, " + dyThread + " <unfinished ...>\n" + dyGroupEnd +
			"2  <... clone resumed> <unfinished ...>) = 0\n"), reasonUnparsable, 4},
		{"clone resume with a pid", dyTrace("2  clone(child_stack=0x7f0000002000, " + dyThread + " <unfinished ...>\n" + dyGroupEnd +
			"2  <... clone resumed> <unfinished ...>) = 3\n"), reasonUnparsable, 4},
		{"clone resume with no ?", dyTrace("2  clone(child_stack=0x7f0000002000, " + dyThread + " <unfinished ...>\n" + dyGroupEnd +
			"2  <... clone resumed> <unfinished ...>)\n"), reasonUnparsable, 4},
		{"clone resume of the wrong name", dyTrace("2  clone(child_stack=0x7f0000002000, " + dyThread + " <unfinished ...>\n" + dyGroupEnd +
			"2  <... fork resumed> <unfinished ...>) = ?\n"), reasonUnmatched, 4},
		{"clone resume with nothing pending", dyTrace(dyGroupEnd + "2  <... clone resumed> <unfinished ...>) = ?\n"), reasonUnmatched, 3},

		// Write-class, network, chdir, exec: refused whatever the shape.
		{"openat O_CREAT at entry", dyTrace(open("O_WRONLY|O_CREAT|O_TRUNC, 0644") + dyGroupEnd), reasonUnresumed, 2},
		{"openat O_WRONLY at entry", dyTrace(open("O_WRONLY|O_CLOEXEC") + dyGroupEnd), reasonUnresumed, 2},
		{"openat O_RDWR at entry", dyTrace(open("O_RDWR") + dyGroupEnd), reasonUnresumed, 2},
		{"openat O_TRUNC at entry", dyTrace(open("O_RDONLY|O_TRUNC") + dyGroupEnd), reasonUnresumed, 2},
		{"openat O_APPEND at entry", dyTrace(open("O_RDONLY|O_APPEND") + dyGroupEnd), reasonUnresumed, 2},
		{"openat O_TMPFILE at entry", dyTrace(open("O_RDONLY|O_TMPFILE") + dyGroupEnd), reasonUnresumed, 2},
		{"openat O_CREAT resumed unavailable", dyTrace(open("O_WRONLY|O_CREAT, 0644") + dyGroupEnd + "2  <... openat resumed>) = ? <unavailable>\n"), reasonUnresumed, 2},
		{"unlink at entry", dyTrace("2  unlink(0x7f3a6c001230 <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"unlink unavailable", dyTrace("2  unlink(0x7f3a6c001230) = ? <unavailable>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"mkdir at entry", dyTrace("2  mkdir(0x7f3a6c001230, 0755 <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"rename unavailable", dyTrace("2  rename(0x1, 0x2) = ? <unavailable>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"connect at entry", dyTrace("2  connect(3<socket:[1]>, {sa_family=AF_UNIX, sun_path=\"/x\"}, 110 <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"connect unavailable", dyTrace("2  connect(3<socket:[1]>, 0x7f3a6c001230, 110) = ? <unavailable>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"chdir at entry", dyTrace("2  chdir(0x7f3a6c001230 <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"fchdir at entry", dyTrace("2  fchdir(3</w> <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"execve at entry", dyTrace("2  execve(0x7f3a6c001230, 0x7f3a6c001240, 0x7f3a6c001250 /* 5 vars */ <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"execve unavailable", dyTrace("2  execve(0x1, 0x2, 0x3) = ? <unavailable>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"creat at entry", dyTrace("2  creat(0x7f3a6c001230, 0644 <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"openat2 at entry", dyTrace("2  openat2(AT_FDCWD</w>, 0x7f3a6c001230, {flags=O_RDONLY, mode=0, resolve=0}, 24 <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"clone3 at entry", dyTrace("2  clone3({flags=CLONE_VM, exit_signal=0} <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"clone3 unavailable", dyTrace("2  clone3({flags=CLONE_VM, exit_signal=0}, 88) = ? <unavailable>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"an unknown name unavailable", dyTrace("2  frobnicate(1) = ? <unavailable>\n" + dyGroupEnd), reasonUnresumed, 2},

		// An open whose flags cannot be read.
		{"openat hex flags", dyTrace(open("0x80000") + dyGroupEnd), reasonUnresumed, 2},
		{"openat a hex leftover", dyTrace(open("O_RDONLY|0x80000") + dyGroupEnd), reasonUnresumed, 2},
		{"openat no flags printed", dyTrace("2  openat(AT_FDCWD</w>, 0x7f3a6c001230, <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"openat no access mode", dyTrace(open("O_CLOEXEC") + dyGroupEnd), reasonUnresumed, 2},
		{"openat a lower-case word", dyTrace(open("o_rdonly") + dyGroupEnd), reasonUnresumed, 2},
		{"open no flags printed", dyTrace("2  open(0x7f3a6c001230, <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},

		// A dying clone whose flags are not readable and allowed.
		{"clone untraced at entry", dyTrace("2  clone(" + dyBadCloneFlags["CLONE_UNTRACED"] + " <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"clone newns at entry", dyTrace("2  clone(" + dyBadCloneFlags["CLONE_NEWNS"] + " <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"clone hex flag at entry", dyTrace("2  clone(" + dyBadCloneFlags["a hex flag"] + " <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"clone no flags at entry", dyTrace("2  clone(" + dyBadCloneFlags["no flags="] + " <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"clone unknown token at entry", dyTrace("2  clone(" + dyBadCloneFlags["an unknown token"] + " <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"clone untraced resumed", dyTrace("2  clone(" + dyBadCloneFlags["CLONE_UNTRACED"] + " <unfinished ...>\n" + dyGroupEnd + "2  <... clone resumed> <unfinished ...>) = ?\n"), reasonUnresumed, 2},
		{"clone newns resumed", dyTrace("2  clone(" + dyBadCloneFlags["CLONE_NEWNS"] + " <unfinished ...>\n" + dyGroupEnd + "2  <... clone resumed> <unfinished ...>) = ?\n"), reasonUnresumed, 2},
		{"clone hex flag resumed", dyTrace("2  clone(" + dyBadCloneFlags["a hex flag"] + " <unfinished ...>\n" + dyGroupEnd + "2  <... clone resumed> <unfinished ...>) = ?\n"), reasonUnresumed, 2},
		{"clone no flags resumed", dyTrace("2  clone(" + dyBadCloneFlags["no flags="] + " <unfinished ...>\n" + dyGroupEnd + "2  <... clone resumed> <unfinished ...>) = ?\n"), reasonUnresumed, 2},
		{"clone untraced unavailable", dyTrace("2  clone(" + dyBadCloneFlags["CLONE_UNTRACED"] + ") = ? <unavailable>\n" + dyGroupEnd), reasonUnresumed, 2},

		// A call that READS is refused in every shape, whatever the group did.
		{"openat at entry, group ending", dyTrace(open("O_RDONLY|O_CLOEXEC") + dyGroupEnd), reasonUnresumed, 2},
		{"openat at entry, no exit_group", dyTrace(open("O_RDONLY")), reasonUnresumed, 2},
		{"newfstatat at entry", dyTrace("2  newfstatat(AT_FDCWD</w>, 0x1, <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"getdents64 at entry", dyTrace("2  getdents64(3</w>, <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"stat at entry", dyTrace("2  stat(0x1, <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"readlink at entry", dyTrace("2  readlink(0x1, <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"open at entry", dyTrace("2  open(0x1, O_RDONLY <unfinished ...>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"openat resumed unavailable", dyTrace(open("O_RDONLY") + dyGroupEnd + "2  <... openat resumed>) = ? <unavailable>\n"), reasonUnresumed, 2},
		{"newfstatat resumed unavailable", dyTrace("2  newfstatat(AT_FDCWD</w>, 0x1, <unfinished ...>\n" + dyGroupEnd + "2  <... newfstatat resumed>) = ? <unavailable>\n"), reasonUnparsable, 4},
		{"getdents64 resumed unavailable", dyTrace("2  getdents64(3</w>, <unfinished ...>\n" + dyGroupEnd + "2  <... getdents64 resumed>) = ? <unavailable>\n"), reasonUnparsable, 4},
		{"stat whole line unavailable", dyTrace("2  stat(0x1, 0x2) = ? <unavailable>\n" + dyGroupEnd), reasonUnresumed, 2},
		{"leader killed, open at entry", dyClone + open("O_RDONLY") + dyExit2 + "1  +++ killed by SIGSEGV +++\n", reasonUnresumed, 2},
		{"exit_group first, open at entry", dyClone + dyGroupEnd + open("O_RDONLY") + dyExit2 + dyExit1, reasonUnresumed, 3},
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

// A dying clone records no spawn.  If a child did start, its first line has no
// spawn record to explain it: orphan-pid.
func TestDyingCloneChildIsAnOrphan(t *testing.T) {
	text := dyClone + "2  clone(child_stack=0x7f0000002000, " + dyThread + " <unfinished ...>\n" + dyGroupEnd +
		"2  <... clone resumed> <unfinished ...>) = ?\n3  openat(AT_FDCWD</w>, \"/x\", O_RDONLY) = 4</x>\n3  +++ exited with 0 +++\n" + dyExit2 + dyExit1
	e := refusal(t, text, false)
	if e.Reason != reasonOrphanPid {
		t.Fatalf("refused %s, want %s", e.Reason, reasonOrphanPid)
	}
}

// dyBadCloneFlags are clone arguments a dying clone must NOT be dropped for.
var dyBadCloneFlags = map[string]string{
	"CLONE_UNTRACED":   "child_stack=0x7f0000002000, flags=CLONE_VM|CLONE_UNTRACED|SIGCHLD",
	"CLONE_NEWNS":      "child_stack=0x7f0000002000, flags=CLONE_NEWNS|SIGCHLD",
	"a hex flag":       "child_stack=0x7f0000002000, flags=CLONE_VM|0x10000|SIGCHLD",
	"no flags=":        "child_stack=0x7f0000002000, 0x3d0f00",
	"an unknown token": "child_stack=0x7f0000002000, flags=CLONE_VM|CLONE_BOGUS|SIGCHLD",
}

// The policy table: which pending call is dropped.
func TestDyingPolicyTable(t *testing.T) {
	p := func(name string, args ...string) *pending { return &pending{name: name, args: args} }
	ok := "flags=CLONE_VM|CLONE_FS|CLONE_FILES|CLONE_SIGHAND|CLONE_THREAD|CLONE_SYSVSEM"
	for _, c := range []struct {
		p    *pending
		drop bool
	}{
		{p("tgkill"), true},
		{p("wait4"), true},
		{p("exit_group"), true},
		{p("fork"), true},
		{p("vfork"), true},
		{p("clone", "child_stack=0x1", ok), true},
		{p("clone", "flags=SIGCHLD"), true},
		{p("clone", "child_stack=0x1", "flags=CLONE_VM|CLONE_UNTRACED"), false},
		{p("clone", "flags=CLONE_NEWNS|SIGCHLD"), false},
		{p("clone", "flags=CLONE_NEWUSER"), false},
		{p("clone", "flags=CLONE_VM|0x10000"), false},
		{p("clone", "flags=CLONE_VM|CLONE_BOGUS"), false},
		{p("clone", "child_stack=0x1"), false},
		{p("clone"), false},
		{p("stat", "0x1"), false},
		{p("newfstatat", "AT_FDCWD</w>", "0x1"), false},
		{p("getdents64", "3</w>"), false},
		{p("openat", "AT_FDCWD</w>", "0x1", "O_RDONLY"), false},
		{p("open", "0x1", "O_RDONLY|O_CLOEXEC"), false},
		{p("openat", "AT_FDCWD</w>", "0x1", "O_WRONLY"), false},
		{p("openat2", "AT_FDCWD</w>", "0x1", "{flags=O_RDONLY}", "24"), false},
		{p("creat", "0x1", "0644"), false},
		{p("unlink", "0x1"), false},
		{p("chdir", "0x1"), false},
		{p("fchdir", "3</w>"), false},
		{p("execve", "0x1", "0x2", "0x3"), false},
		{p("connect", "3<socket:[1]>", "0x1", "110"), false},
		{p("clone3", "{flags=CLONE_VM}"), false},
		{p("mount"), false},
		{p("frobnicate"), false},
		{p("???"), false},
	} {
		if got := dyingPolicy(c.p); got != c.drop {
			t.Errorf("dyingPolicy(%s %v) = %v, want %v", c.p.name, c.p.args, got, c.drop)
		}
	}
}

// Every seed line gives the same verdict from both readers (or one the spec's
// stricter-only list allows).
func TestDyingLineSeedsAgree(t *testing.T) {
	for _, l := range dyingLineSeeds {
		if d, _ := lineVerdict(l); d != "" {
			t.Errorf("%s: %q", d, l)
		}
	}
}

// dyingLineSeeds and dyingTraceSeeds are the fuzzers' seeds for this rule
// (linediff_test.go).
var dyingLineSeeds = []string{
	"2  <... tgkill resumed>) = ? <unavailable>",
	"2  <... clone resumed> <unfinished ...>) = ?",
	"2  <... clone resumed> <unfinished ...>) = 0",
	"2  <... clone resumed>, 0x1 <unfinished ...>) = ?",
	"2  <... clone resumed> <unfinished ...>)",
	"2  <... clone resumed> <unfinished ...> <unfinished ...>) = ?",
	"2  tgkill(1, 1, SIGURG) = ? <unavailable>",
	"2  stat(0x1, 0x2) = ? <unavailable>",
	"2  stat(0x1, 0x2) = 3 <unavailable>",
	"2  stat(0x1, 0x2) = ?<unavailable>",
	"2  stat(0x1, 0x2) = ?  <unavailable>",
	"2  stat(0x1, 0x2) = -1 ENOENT <unavailable>",
	"2  stat(0x1, 0x2) = 0<unavailable>",
	"2  stat(<unavailable>) = 0",
	"2  openat(AT_FDCWD</w>, 0x7f3a6c001230, O_RDONLY|O_CLOEXEC <unfinished ...>",
	"2  newfstatat(AT_FDCWD</w>, 0x7f3a6c001230, <unfinished ...>",
	"2  openat(AT_FDCWD</w>, 0x7f3a6c001230, O_WRONLY|O_CREAT, 0644 <unfinished ...>",
	"2  <... openat resumed>) = ? <unavailable>",
	"2  <... ??? resumed>) = ? <unavailable>",
}

var dyingTraceSeeds = []string{
	dyTrace("2  clone(" + dyBadCloneFlags["CLONE_UNTRACED"] + " <unfinished ...>\n" + dyGroupEnd + "2  <... clone resumed> <unfinished ...>) = ?\n"),
	dyTrace("2  clone(" + dyBadCloneFlags["no flags="] + " <unfinished ...>\n" + dyGroupEnd),
	dyTrace("2  clone(" + dyBadCloneFlags["CLONE_NEWNS"] + ") = ? <unavailable>\n" + dyGroupEnd),
	dyTrace("2  tgkill(1, 1, SIGURG <unfinished ...>\n" + dyGroupEnd + "2  <... tgkill resumed>) = ? <unavailable>\n"),
	dyTrace("2  clone(child_stack=0x7f0000002000, " + dyThread + " <unfinished ...>\n" + dyGroupEnd + "2  <... clone resumed> <unfinished ...>) = ?\n"),
	dyTrace("2  openat(AT_FDCWD</w>, 0x7f3a6c001230, O_RDONLY|O_CLOEXEC <unfinished ...>\n" + dyGroupEnd),
	dyTrace("2  newfstatat(AT_FDCWD</w>, 0x7f3a6c001230, <unfinished ...>\n" + dyGroupEnd),
	dyTrace("2  openat(AT_FDCWD</w>, 0x7f3a6c001230, O_WRONLY|O_CREAT, 0644 <unfinished ...>\n" + dyGroupEnd),
	dyTrace("2  openat(AT_FDCWD</w>, 0x7f3a6c001230, O_RDONLY <unfinished ...>\n"),
	dyTrace("2  stat(0x1, 0x2) = ? <unavailable>\n2  stat(0x1, 0x2) = 0\n"),
	dyClone + "2  openat(AT_FDCWD</w>, 0x7f3a6c001230, O_RDONLY <unfinished ...>\n" + dyExit2 + dyGroupEnd + dyExit1,
	dyClone + "2  openat(AT_FDCWD</w>, 0x7f3a6c001230, O_RDONLY <unfinished ...>\n" + dyExit2 + "1  +++ killed by SIGSEGV +++\n",
	"1  vfork() = 2\n2  clone(child_stack=0x7f0000001000, " + dyThread + ") = 3\n3  stat(0x1, 0x2) = ? <unavailable>\n2  exit_group(0) = ?\n3  +++ exited with 0 +++\n2  +++ exited with 0 +++\n1  +++ exited with 0 +++\n",
}
