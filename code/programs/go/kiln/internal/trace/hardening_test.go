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

const helperSetup = "10  unshare(CLONE_NEWUSER|CLONE_NEWNS|CLONE_NEWPID) = 0\n" +
	"10  mount(\"none\", \"/\", NULL, MS_REC|MS_PRIVATE, NULL) = 0\n" +
	"10  capset(0x1, 0x2) = 0\n" +
	"10  pivot_root(\".\", \".\") = 0\n" +
	"10  chdir(\"/repo\") = 0\n"

func TestHelperRoot(t *testing.T) {
	opts := Options{HelperRoot: true}
	text := helperSetup +
		"10  execve(\"/bin/sh\", [\"sh\"], 0x0 /* 1 vars */) = 0\n" +
		"10  openat(AT_FDCWD, \"a\", O_RDONLY) = 3</repo/a>\n" +
		"10  +++ exited with 0 +++\n"
	tr, err := parseAt(t, "/start", text, opts)
	if err != nil {
		t.Fatal(err)
	}
	if len(tr.Events) != 2 || tr.Events[0].Syscall != "execve" || tr.Events[1].Cwd != "/repo" {
		t.Fatalf("%+v", tr.Events)
	}
	// Without the option the same trace is refused.
	_, err = parseAt(t, "/start", text, Options{})
	if err == nil || err.Error() != "trace-incomplete:refused:unshare" {
		t.Fatalf("%v", err)
	}
	// After the exec, refused calls are refused as usual.
	_, err = parseAt(t, "/start", helperSetup+"10  execve(\"/bin/sh\", [\"sh\"], 0x0) = 0\n10  mount(\"a\", \"b\", NULL, 0, NULL) = 0\n10  +++ exited with 0 +++\n", opts)
	if err == nil || err.Error() != "trace-incomplete:refused:mount" {
		t.Fatalf("%v", err)
	}
	// Unknown names after the exec as well.
	wantReasonOpts(t, helperSetup+"10  execve(\"/bin/sh\", [\"sh\"], 0x0) = 0\n10  capset(0x1) = 0\n10  +++ exited with 0 +++\n", opts, "unknown-syscall:capset")
	// The exec must happen; a failed exec does not count.
	wantReasonOpts(t, helperSetup+"10  +++ exited with 0 +++\n", opts, reasonHelperNoExec)
	wantReasonOpts(t, helperSetup+"10  execve(\"/bin/sh\", [\"sh\"], 0x0) = -1 ENOENT (No such file or directory)\n10  +++ exited with 1 +++\n", opts, reasonHelperNoExec)
	// The helper must not spawn before the exec.
	wantReasonOpts(t, helperSetup+"10  fork() = 11\n11  +++ exited with 0 +++\n10  +++ exited with 0 +++\n", opts, reasonHelperSpawn)
	// A split setup call is paired and discarded too.
	tr, err = parseAt(t, "/", "10  mount(\"a\", \"b\", NULL, 0 <unfinished ...>\n10  <... mount resumed>, NULL) = 0\n10  execve(\"/x\", [], 0x0) = 0\n10  +++ exited with 0 +++\n", opts)
	if err != nil || len(tr.Events) != 1 {
		t.Fatalf("%v %+v", err, tr)
	}
	// A child spawned AFTER the exec is normal.
	_, err = parseAt(t, "/", helperSetup+"10  execve(\"/x\", [], 0x0) = 0\n10  fork() = 11\n11  +++ exited with 0 +++\n10  +++ exited with 0 +++\n", opts)
	if err != nil {
		t.Fatal(err)
	}
}

func wantReasonOpts(t *testing.T, text string, opts Options, reason string) {
	t.Helper()
	_, err := parseAt(t, "/start", text, opts)
	if err == nil || err.Error() != "trace-incomplete:"+reason {
		t.Fatalf("want %s, got %v", reason, err)
	}
}
