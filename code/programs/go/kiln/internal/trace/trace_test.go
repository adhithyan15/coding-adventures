package trace

import (
	"errors"
	"os"
	"reflect"
	"strings"
	"testing"
)

func parseText(t *testing.T, text string) (*Trace, error) {
	t.Helper()
	return Parse(strings.NewReader(text), "/work/repo", Limits{})
}

func mustParse(t *testing.T, text string) *Trace {
	t.Helper()
	tr, err := parseText(t, text)
	if err != nil {
		t.Fatalf("Parse: %v", err)
	}
	return tr
}

func fixture(t *testing.T, name string) string {
	t.Helper()
	b, err := os.ReadFile("testdata/" + name)
	if err != nil {
		t.Fatal(err)
	}
	return string(b)
}

func wantReason(t *testing.T, text, reason string) {
	t.Helper()
	_, err := parseText(t, text)
	var e *Error
	if !errors.As(err, &e) {
		t.Fatalf("want trace-incomplete:%s, got %v", reason, err)
	}
	if e.Reason != reason || err.Error() != "trace-incomplete:"+reason {
		t.Fatalf("reason = %q (%v), want %q", e.Reason, err, reason)
	}
}

// wrap makes a one-process trace around body lines.
func wrap(body ...string) string {
	return "1  " + strings.Join(body, "\n1  ") + "\n1  +++ exited with 0 +++\n"
}

func TestBuildFixtureEveryClass(t *testing.T) {
	tr := mustParse(t, fixture(t, "build.strace"))
	if tr.RootPid != 100 {
		t.Fatalf("root %d", tr.RootPid)
	}
	byName := map[string][]Event{}
	for _, e := range tr.Events {
		byName[e.Syscall] = append(byName[e.Syscall], e)
	}
	// Open: the -y annotation of the returned fd, flags raw.
	op := byName["openat"][0]
	if op.Kind != KindOpen || op.Path != "/etc/ld.so.cache" || op.ReturnFDPath != "/etc/ld.so.cache" ||
		op.Flags != "O_RDONLY|O_CLOEXEC" || !op.Result.OK || op.Result.Value != 3 || op.Cwd != "/work/repo" || op.DirfdPath != "" {
		t.Fatalf("openat: %+v", op)
	}
	// Metadata, including a failed one.
	if e := byName["newfstatat"][0]; e.Kind != KindMetadata || e.Path != "go.mod" || e.Cwd != "/work/repo" {
		t.Fatalf("newfstatat: %+v", e)
	}
	if e := byName["stat"][0]; e.Result.OK || e.Result.Errno != "ENOENT" || e.Path != "missing.h" {
		t.Fatalf("stat: %+v", e)
	}
	// Directory: chdir moves cwd for later events only.
	if e := byName["chdir"][0]; e.Kind != KindDirectory || e.Cwd != "/work/repo" {
		t.Fatalf("chdir: %+v", e)
	}
	if e := byName["openat"][1]; e.Cwd != "/work/repo/pkg" || e.Path != "src/main.c" {
		t.Fatalf("openat after chdir: %+v", e)
	}
	// The child copied the parent's cwd.
	if e := byName["execve"][1]; e.Pid != 101 || e.Cwd != "/work/repo/pkg" || e.Path != "/usr/bin/cc" {
		t.Fatalf("child execve: %+v", e)
	}
	// Listing.
	if e := byName["getdents64"][0]; e.Kind != KindListing || e.Path != "/work/repo/pkg/src" {
		t.Fatalf("getdents64: %+v", e)
	}
	// A split open, resumed: flags and result glued together.
	op = byName["openat"][3]
	if op.Path != "main.o" || op.Flags != "O_WRONLY|O_CREAT|O_TRUNC" || op.ReturnFDPath != "/work/repo/pkg/main.o" || op.Pid != 101 {
		t.Fatalf("resumed openat: %+v", op)
	}
	// Write class.
	if e := byName["rename"][0]; e.Kind != KindWrite || e.Path != "main.o" || e.Path2 != "build.o" {
		t.Fatalf("rename: %+v", e)
	}
	if e := byName["mkdir"][0]; e.Result.Errno != "EEXIST" || e.Result.OK {
		t.Fatalf("mkdir: %+v", e)
	}
	if e := byName["unlinkat"][0]; e.Flags != "AT_REMOVEDIR" {
		t.Fatalf("unlinkat: %+v", e)
	}
	if e := byName["symlink"][0]; e.Path != "link.o" || e.Target != "main.o" {
		t.Fatalf("symlink: %+v", e)
	}
	if e := byName["utimensat"][0]; e.Path != "/work/repo/pkg/main.o" || e.DirfdPath != "" {
		t.Fatalf("utimensat NULL: %+v", e)
	}
	if e := byName["linkat"][0]; e.Path != "a" || e.Path2 != "b" || e.DirfdPath != "" || e.Dirfd2Path != "/work/repo" {
		t.Fatalf("linkat: %+v", e)
	}
	// Process tree and exits.
	p := tr.Procs[101]
	if p.Parent != 100 || p.LastExec != "/usr/bin/cc" || p.ShareFS || p.Exit == nil || p.Exit.Code != 0 {
		t.Fatalf("proc 101: %+v", p)
	}
	if !reflect.DeepEqual(tr.Procs[100].Children, []int{101}) || tr.Procs[100].LastExec != "/usr/bin/sh" {
		t.Fatalf("proc 100: %+v", tr.Procs[100])
	}
	if len(tr.Exits) != 2 || tr.Exits[0].Pid != 101 || tr.Exits[1].Pid != 100 {
		t.Fatalf("exits: %+v", tr.Exits)
	}
	if !reflect.DeepEqual(tr.Order, []int{100, 101}) {
		t.Fatalf("order %v", tr.Order)
	}
}

func TestWorkingDirectoryTracking(t *testing.T) {
	tr := mustParse(t, fixture(t, "cwd.strace"))
	cwd := map[string]string{}
	for _, e := range tr.Events {
		if e.Syscall == "stat" {
			cwd[e.Path] = e.Cwd
		}
	}
	// 201 was created with CLONE_FS before its parent's return line was
	// printed; it shares 200's cwd cell, so its chdir("../b") is seen by 200.
	want := map[string]string{
		"x": "/work/a/../b", // parent sees the thread's chdir
		"y": "/work/a/../b",
		"z": "/work/a/../b", // fork copies
		"w": "/srv/data",    // fchdir uses the -y path verbatim
		"v": "/work/a/../b", // the fork child's chdirs did not leak
		"u": "/work/a/../b", // vfork child copies
	}
	if !reflect.DeepEqual(cwd, want) {
		t.Fatalf("cwd = %v\nwant %v", cwd, want)
	}
	if !tr.Procs[201].ShareFS || tr.Procs[202].ShareFS {
		t.Fatalf("ShareFS: 201=%v 202=%v", tr.Procs[201].ShareFS, tr.Procs[202].ShareFS)
	}
	if e := tr.Procs[202].Exit; e == nil || !e.Killed || e.Signal != "SIGKILL" {
		t.Fatalf("202 exit: %+v", e)
	}
	if e := tr.Procs[203].Exit; e == nil || e.Code != 3 || e.Killed {
		t.Fatalf("203 exit: %+v", e)
	}
	if !reflect.DeepEqual(tr.Procs[200].Children, []int{201, 202, 203}) {
		t.Fatalf("children %v", tr.Procs[200].Children)
	}
}

func TestChildPrecedingParentReturnGetsCwd(t *testing.T) {
	text := "1  chdir(\"/a\") = 0\n" +
		"1  fork( <unfinished ...>\n" +
		"2  stat(\"x\", 0x1) = 0\n" +
		"2  +++ exited with 0 +++\n" +
		"1  <... fork resumed>) = 2\n" +
		"1  +++ exited with 0 +++\n"
	tr := mustParse(t, text)
	if tr.Events[len(tr.Events)-1].Cwd != "/a" || tr.Procs[2].Parent != 1 {
		t.Fatalf("%+v", tr.Events)
	}
}

func TestAdversarialPaths(t *testing.T) {
	tr := mustParse(t, fixture(t, "paths.strace"))
	var got []string
	for _, e := range tr.Events {
		got = append(got, e.Path)
	}
	want := []string{
		`/tmp/a"b`, `/tmp/lt<gt>`, `/tmp/p) = 5`, `/tmp/sp ace`, "/tmp/nl\nx\ttab\\back",
		"/tmp/éAA", `/tmp/+++ exited with 0 +++`, `/tmp/<unfinished ...>`, "f",
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("paths\n got %q\nwant %q", got, want)
	}
	if tr.Events[0].ReturnFDPath != `/tmp/a"b` || tr.Events[1].ReturnFDPath != "/tmp/lt<gt>" {
		t.Fatalf("return fd paths: %q %q", tr.Events[0].ReturnFDPath, tr.Events[1].ReturnFDPath)
	}
	last := tr.Events[8]
	if last.DirfdPath != "/tmp/dir with space" || last.ReturnFDPath != "/tmp/dir with space/f" {
		t.Fatalf("dirfd with space: %+v", last)
	}
}

func TestAmbiguousAnnotations(t *testing.T) {
	// A raw '>' in a directory path: the annotation closes early and text is
	// left over.
	wantReason(t, wrap(`openat(4</tmp/we>ird>, "g", O_RDONLY) = -1 ENOENT (No such file or directory)`), reasonAmbiguousPath)
	// A raw '<' inside an annotation.
	wantReason(t, wrap(`openat(4</tmp/we<ird>, "g", O_RDONLY) = -1 ENOENT (x)`), reasonAmbiguousPath)
	// An attacker's filename that tries to end the call early and fake its
	// result: leftover text after the fake result is refused.
	wantReason(t, wrap(`stat("/x", 0x1) = 0 "extra"`), reasonResult)
	wantReason(t, wrap(`openat(3</a>, "g", O_RDONLY) = 5</q>, "f", O_RDONLY) = 3</real>`), reasonAmbiguousPath)
	// Return annotation with trailing text.
	wantReason(t, wrap(`open("/x", O_RDONLY) = 3</x> junk`), reasonAmbiguousPath)
}

func TestStringErrors(t *testing.T) {
	cases := map[string]string{
		`stat("/tmp/abc"..., 0x1) = 0`:                    reasonTruncated,
		`stat("/tmp/\q", 0x1) = 0`:                        reasonAmbiguousPath,
		`stat("/tmp/\400", 0x1) = 0`:                      reasonAmbiguousPath,
		`stat("/tmp/\xZZ", 0x1) = 0`:                      reasonAmbiguousPath,
		`stat("/tmp/\`:                                    reasonUnparsable,
		`stat("/tmp/a\000b", 0x1) = 0`:                    reasonAmbiguousPath, // NUL
		`stat("/tmp/\377", 0x1) = 0`:                      reasonNonUTF8,
		`stat(0x7ffd0000, 0x1) = -1 EFAULT (Bad address)`: reasonPathArg,
		`stat("/a" "/b", 0x1) = 0`:                        reasonPathArg,
		`openat(3</x (deleted)>, "g", O_RDONLY) = 3</y>`:  reasonDeleted,
		`openat(3</x>, "g", O_RDONLY) = 3</y (deleted)>`:  reasonDeleted,
		`openat(3<pipe:[1]>, "g", O_RDONLY) = 3</y>`:      reasonFDPath,
		`openat(3, "g", O_RDONLY) = 3</y>`:                reasonFDPath,
		`openat(x, "g", O_RDONLY) = 3</y>`:                reasonDirfd,
		`openat(AT_FDCWD, "g", O_RDONLY) = 3`:             reasonFDPath,
		`stat("a")  = 0`:                                  reasonArity,
		`stat("a", 0x1) = ?`:                              reasonResult,
		`stat("a", 0x1) = -2 ENOENT (x)`:                  reasonResult,
		`stat("a", 0x1) = -1`:                             reasonResult,
		`stat("a", 0x1) = -1 enoent`:                      reasonResult,
		`stat("a", 0x1) 0`:                                reasonResult,
		`stat("a", 0x1) = 0 (a "b")`:                      reasonResult,
		`stat("a", 0x1) = 0x`:                             reasonResult,
		`stat("a", 0x1) = abc`:                            reasonResult,
		`stat("a", {x=1) = 0`:                             reasonUnparsable,
		`stat("a", 0x1] = 0`:                              reasonUnparsable,
		`stat("a", 0x1`:                                   reasonUnparsable,
		`stat("a", 0x1,) = 0`:                             reasonUnparsable,
		`stat("a", <x) = 0`:                               reasonUnparsable,
		`stat("a", 3<x) = 0`:                              reasonUnparsable,
		`Stat("a", 0x1) = 0`:                              reasonUnparsable,
		`stat "a"`:                                        reasonUnparsable,
	}
	for line, reason := range cases {
		t.Run(line, func(t *testing.T) { wantReason(t, wrap(line), reason) })
	}
}

func TestLineShapes(t *testing.T) {
	wantReason(t, "", reasonEmpty)
	wantReason(t, "\n", reasonUnparsable)
	wantReason(t, "garbage\n", reasonUnparsable)
	wantReason(t, "1\n", reasonUnparsable)
	wantReason(t, "1  \n", reasonUnparsable)
	wantReason(t, "0  stat(\"a\", 0x1) = 0\n", reasonUnparsable)
	wantReason(t, "[pid x] stat(\"a\", 0x1) = 0\n", reasonUnparsable)
	wantReason(t, "[pid 1 stat(\"a\", 0x1) = 0\n", reasonUnparsable)
	wantReason(t, "1  stat(\"a\", 0x1) = 0\r\n", reasonUnparsable)
	wantReason(t, "1  stat(\"a\x00\", 0x1) = 0\n", reasonUnparsable)
	wantReason(t, "strace: Process 5 attached\n", reasonUnparsable)
	wantReason(t, "1  +++ exited with x +++\n", reasonUnparsable)
	wantReason(t, "1  +++ exited with 0\n", reasonUnparsable)
	wantReason(t, "1  +++ killed by sigkill +++\n", reasonUnparsable)
	wantReason(t, "1  +++ vanished +++\n", reasonUnparsable)
	wantReason(t, "1  --- SIGCHLD\n", reasonUnparsable)
	wantReason(t, "1  <... stat\n", reasonUnparsable)
	// The [pid N] form and a missing final newline are accepted.
	tr := mustParse(t, "[pid 7] stat(\"a\", 0x1) = 0\n[pid 7] +++ exited with 0 +++")
	if tr.RootPid != 7 || len(tr.Events) != 1 {
		t.Fatalf("%+v", tr)
	}
	// Signal lines and killed-by with core dump.
	tr = mustParse(t, "1  --- SIGSEGV {si_signo=SIGSEGV} ---\n1  +++ killed by SIGSEGV (core dumped) +++\n")
	if e := tr.Exits[0]; !e.Killed || !e.CoreDumped || e.Signal != "SIGSEGV" {
		t.Fatalf("%+v", e)
	}
}

func TestPersonality(t *testing.T) {
	wantReason(t, "1  stat(\"a\", 0x1) = 0\n[ Process PID=1 runs in 32 bit mode. ]\n1  +++ exited with 0 +++\n", reasonPersonality)
}

func TestInterleaving(t *testing.T) {
	text := "1  stat(\"a\", 0x1 <unfinished ...>\n" +
		"2  stat(\"b\", 0x1) = 0\n" +
		"1  <... stat resumed>)                   = 0\n" +
		"2  stat(\"c\", 0x1 <unfinished ...>\n" +
		"2  +++ exited with 1 +++\n"
	// pid 2's stat never resumed, and it is not an ignorable call.
	_, err := Parse(strings.NewReader("1  fork() = 2\n"+text+"1  +++ exited with 0 +++\n"), "/", Limits{})
	if err == nil || err.Error() != "trace-incomplete:unresumed-call" {
		t.Fatalf("%v", err)
	}
	ok := "1  fork() = 2\n" +
		"1  stat(\"a\", 0x1 <unfinished ...>\n" +
		"2  stat(\"b\", 0x1) = 0\n" +
		"1  <... stat resumed>)                   = 0\n" +
		"2  +++ exited with 1 +++\n" +
		"1  +++ exited with 0 +++\n"
	tr := mustParse(t, ok)
	if len(tr.Events) != 3 || tr.Events[1].Path != "b" || tr.Events[2].Path != "a" || tr.Events[2].Pid != 1 {
		t.Fatalf("completion order: %+v", tr.Events)
	}
	// An ignored call (wait4) interrupted by SIGKILL needs no resume.
	mustParse(t, "1  wait4(-1,  <unfinished ...>\n1  +++ killed by SIGKILL +++\n")
	// Unfinished exit_group discharged by the exit line.
	mustParse(t, "1  exit_group(0 <unfinished ...>\n1  +++ exited with 0 +++\n")

	wantReason(t, wrap(`<... stat resumed>) = 0`), reasonUnmatched)
	wantReason(t, "1  stat(\"a\", 0x1 <unfinished ...>\n1  <... open resumed>) = 0\n1  +++ exited with 0 +++\n", reasonUnmatched)
	wantReason(t, "1  stat(\"a\", 0x1 <unfinished ...>\n1  stat(\"b\", 0x1) = 0\n", reasonInterleaved)
	wantReason(t, "1  stat(\"a\", 0x1 <unfinished ...>\n", reasonUnresumed)
	wantReason(t, "1  stat(\"a\", 0x1 <unfinished ...>\n1  <... stat resumed>, <unfinished ...>\n", reasonUnparsable)
	wantReason(t, "1  stat(\"a\", 0x1 <unfinished ...>\n1  <... stat resumed>)  = junk\n", reasonResult)
	// A forged marker inside a path is not a marker.
	mustParse(t, wrap(`stat("/x <unfinished ...>", 0x1) = 0`))
	// ... and a completed line whose annotation merely ends in the marker text
	// is ambiguous, not unfinished.
	wantReason(t, wrap(`open("/x", O_RDONLY) = 3</x<unfinished ...>>`), reasonAmbiguousPath)
}

func TestExitCompleteness(t *testing.T) {
	wantReason(t, "1  stat(\"a\", 0x1) = 0\n", reasonMissingExit)
	// Child never printed an exit line.
	wantReason(t, "1  fork() = 2\n2  stat(\"a\", 0x1) = 0\n1  +++ exited with 0 +++\n", reasonMissingExit)
	// Child printed nothing at all (untraced).
	wantReason(t, "1  fork() = 2\n1  +++ exited with 0 +++\n", reasonMissingExit)
	wantReason(t, "1  +++ exited with 0 +++\n1  +++ exited with 0 +++\n", reasonDuplicateExit)
	wantReason(t, "1  +++ exited with 0 +++\n1  stat(\"a\", 0x1) = 0\n", reasonEventAfterExit)
	// A pid nobody spawned.
	wantReason(t, "1  stat(\"a\", 0x1) = 0\n9  stat(\"a\", 0x1) = 0\n9  +++ exited with 0 +++\n1  +++ exited with 0 +++\n", reasonOrphanPid)
	// pid reuse and self-spawn.
	wantReason(t, "1  fork() = 2\n1  fork() = 2\n", reasonPidReuse)
	wantReason(t, "1  fork() = 1\n", reasonPidReuse)
	wantReason(t, "1  fork() = 2\n2  fork() = 1\n", reasonPidReuse)
	// Cycle: 2 spawned by 3 and 3 spawned by 2.
	wantReason(t, "1  stat(\"a\", 0x1) = 0\n2  fork() = 3\n3  fork() = 2\n", reasonProcessCycle)
	// Bad fork results.
	wantReason(t, "1  fork() = 0\n", reasonResult)
	wantReason(t, "1  fork() = ?\n", reasonResult)
	// A failed fork spawns nothing.
	mustParse(t, wrap(`fork() = -1 EAGAIN (Resource temporarily unavailable)`))
}

func TestClosedTable(t *testing.T) {
	wantReason(t, wrap(`frobnicate("a") = 0`), "unknown-syscall:frobnicate")
	wantReason(t, wrap(`read(3</x>, "a", 1) = 1`), "unknown-syscall:read")
	// Unknown even when it never resumes.
	wantReason(t, "1  frobnicate( <unfinished ...>\n", "unknown-syscall:frobnicate")
	for _, n := range []string{
		"io_uring_setup", "open_by_handle_at", "name_to_handle_at", "mount", "umount2", "fsopen",
		"fsmount", "fsconfig", "fspick", "open_tree", "move_mount", "mount_setattr", "pivot_root",
		"chroot", "unshare", "setns", "ptrace", "process_vm_readv", "pidfd_getfd", "bpf", "clone3",
	} {
		wantReason(t, wrap(n+`(0) = -1 EPERM (Operation not permitted)`), "refused:"+n)
	}
	wantReason(t, wrap(`clone3({flags=CLONE_VM, exit_signal=SIGCHLD}, 88) = 2`), "refused:clone3")
	// Ignored names parse but produce nothing.
	tr := mustParse(t, wrap(`getcwd("/work", 4096) = 6`, `close(3</x>) = 0`, `dup2(3, 4) = 4`, `pipe2([3, 4], O_CLOEXEC) = 0`, `kill(5, SIGKILL) = 0`))
	if len(tr.Events) != 0 {
		t.Fatalf("%+v", tr.Events)
	}
	if lookup("wait4").class != classIgnored || lookup("mount").class != classRefused || lookup("openat").class != classHandled {
		t.Fatal("classes")
	}
}

func TestEveryHandledNameBuildsAnEvent(t *testing.T) {
	// One canonical line per handled name, so each table row is exercised.
	lines := map[string]string{
		"open": `open("a", O_RDONLY) = 3</r/a>`, "openat": `openat(AT_FDCWD, "a", O_RDONLY, 0644) = 3</r/a>`,
		"openat2": `openat2(AT_FDCWD, "a", {flags=O_RDONLY, resolve=0}, 24) = 3</r/a>`, "creat": `creat("a", 0644) = 3</r/a>`,
		"stat": `stat("a", {}) = 0`, "lstat": `lstat("a", {}) = 0`, "newfstatat": `newfstatat(AT_FDCWD, "a", {}, 0) = 0`,
		"statx": `statx(AT_FDCWD, "a", 0, STATX_BASIC_STATS, {}) = 0`, "access": `access("a", R_OK) = 0`,
		"faccessat": `faccessat(AT_FDCWD, "a", R_OK) = 0`, "faccessat2": `faccessat2(AT_FDCWD, "a", R_OK, 0) = 0`,
		"readlink": `readlink("a", "b", 4) = 1`, "readlinkat": `readlinkat(AT_FDCWD, "a", "b", 4) = 1`,
		"getxattr": `getxattr("a", "u.x", 0x0, 0) = -1 ENODATA (No data available)`, "lgetxattr": `lgetxattr("a", "u.x", 0x0, 0) = -1 ENODATA (No data available)`,
		"listxattr": `listxattr("a", 0x0, 0) = 0`, "llistxattr": `llistxattr("a", 0x0, 0) = 0`, "statfs": `statfs("a", {}) = 0`,
		"inotify_add_watch": `inotify_add_watch(3<anon_inode:inotify>, "a", IN_OPEN) = 1`,
		"fanotify_mark":     `fanotify_mark(3<anon_inode:[fanotify]>, FAN_MARK_ADD, FAN_OPEN, AT_FDCWD, "a") = 0`,
		"getdents":          `getdents(3</r>, 0x0, 10) = 0`, "getdents64": `getdents64(3</r>, 0x0, 10) = 0`,
		"chdir": `chdir("a") = 0`, "fchdir": `fchdir(3</r/z>) = 0`,
		"mkdir": `mkdir("a", 0755) = 0`, "mkdirat": `mkdirat(AT_FDCWD, "a", 0755) = 0`, "rmdir": `rmdir("a") = 0`, "unlink": `unlink("a") = 0`,
		"unlinkat": `unlinkat(AT_FDCWD, "a", 0) = 0`, "rename": `rename("a", "b") = 0`,
		"renameat":  `renameat(AT_FDCWD, "a", AT_FDCWD, "b") = 0`,
		"renameat2": `renameat2(AT_FDCWD, "a", AT_FDCWD, "b", RENAME_NOREPLACE) = 0`, "link": `link("a", "b") = 0`,
		"linkat": `linkat(AT_FDCWD, "a", AT_FDCWD, "b", 0) = 0`, "symlink": `symlink("t", "a") = 0`,
		"symlinkat": `symlinkat("t", AT_FDCWD, "a") = 0`, "chmod": `chmod("a", 0755) = 0`,
		"fchmodat": `fchmodat(AT_FDCWD, "a", 0755) = 0`, "fchmodat2": `fchmodat2(AT_FDCWD, "a", 0755, 0) = 0`,
		"chown": `chown("a", 1, 2) = 0`, "lchown": `lchown("a", 1, 2) = 0`, "fchownat": `fchownat(AT_FDCWD, "a", 1, 2, 0) = 0`,
		"truncate": `truncate("a", 0) = 0`, "utime": `utime("a", NULL) = 0`, "utimes": `utimes("a", NULL) = 0`,
		"utimensat": `utimensat(AT_FDCWD, "a", NULL, 0) = 0`, "futimesat": `futimesat(AT_FDCWD, "a", NULL) = 0`,
		"setxattr": `setxattr("a", "u.x", "v", 1, 0) = 0`, "lsetxattr": `lsetxattr("a", "u.x", "v", 1, 0) = 0`,
		"removexattr": `removexattr("a", "u.x") = 0`, "lremovexattr": `lremovexattr("a", "u.x") = 0`,
		"mknod": `mknod("a", S_IFIFO|0600, 0) = 0`, "mknodat": `mknodat(AT_FDCWD, "a", S_IFIFO|0600, 0) = 0`,
		"execve": `execve("a", ["a"], 0x0 /* 1 vars */) = 0`, "execveat": `execveat(AT_FDCWD, "a", ["a"], 0x0 /* 1 vars */, 0) = 0`,
		"fork": `fork() = 2`, "vfork": `vfork() = 2`,
		"clone":  `clone(child_stack=NULL, flags=SIGCHLD) = 2`,
		"socket": `socket(AF_INET, SOCK_STREAM, 0) = 3<socket:[1]>`, "connect": `connect(3<socket:[1]>, {sa_family=AF_NETLINK, nl_pid=0, nl_groups=00000000}, 12) = 0`,
		"bind": `bind(3<socket:[1]>, {sa_family=AF_UNIX}, 2) = 0`, "listen": `listen(3<socket:[1]>, 1) = 0`,
		"sendto":  `sendto(3<socket:[1]>, "a", 1, 0, NULL, 0) = 1`,
		"sendmsg": `sendmsg(3<socket:[1]>, {msg_name=NULL, msg_namelen=0}, 0) = 1`, "sendmmsg": `sendmmsg(3<socket:[1]>, [], 0, 0) = 0`,
	}
	for name, ent := range table {
		if ent.class != classHandled {
			continue
		}
		line, ok := lines[name]
		if !ok {
			t.Errorf("no sample line for handled call %s", name)
			continue
		}
		body := line
		if name == "fork" || name == "vfork" || name == "clone" {
			text := "1  " + body + "\n2  +++ exited with 0 +++\n1  +++ exited with 0 +++\n"
			tr := mustParse(t, text)
			if len(tr.Events) != 1 || tr.Events[0].Syscall != name || tr.Events[0].Result.Value != 2 {
				t.Errorf("%s: %+v", name, tr.Events)
			}
			continue
		}
		tr := mustParse(t, wrap(body))
		if len(tr.Events) != 1 || tr.Events[0].Syscall != name || tr.Events[0].Kind != ent.kind {
			t.Errorf("%s: %+v", name, tr.Events)
		}
	}
	tr := mustParse(t, wrap(lines["sendmmsg"]))
	if tr.Events[0].Addr != nil {
		t.Fatal("sendmmsg without names has no addr")
	}
}

func TestOpenAndFdKinds(t *testing.T) {
	tr := mustParse(t, wrap(
		`creat("n", 0644) = 3</work/repo/n>`,
		`openat(AT_FDCWD, "/proc/self/fd/0", O_RDONLY) = 4</dev/null>`,
		`openat(AT_FDCWD, "/x", O_RDONLY) = 5<pipe:[77]>`,
		`utimensat(AT_FDCWD</work/repo>, "a", NULL, 0) = 0`,
		`inotify_add_watch(3<anon_inode:inotify>, "/etc", IN_OPEN) = 1`,
	))
	if e := tr.Events[0]; e.Flags != "O_CREAT|O_WRONLY|O_TRUNC" || e.Kind != KindOpen {
		t.Fatalf("creat: %+v", e)
	}
	if tr.Events[1].ReturnFDPath != "/dev/null" || tr.Events[2].ReturnFDPath != "pipe:[77]" {
		t.Fatalf("%+v %+v", tr.Events[1], tr.Events[2])
	}
	// AT_FDCWD's own annotation is never relied on.
	if tr.Events[3].DirfdPath != "" {
		t.Fatalf("%+v", tr.Events[3])
	}
	// A failed open has no fd to annotate.
	mustParse(t, wrap(`openat(AT_FDCWD, "/x", O_RDONLY) = -1 EACCES (Permission denied)`))
	// utimensat(AT_FDCWD, NULL) is not a thing.
	wantReason(t, wrap(`utimensat(AT_FDCWD, NULL, NULL, 0) = 0`), reasonDirfd)
	// Listing/fchdir need a numeric fd with a path.
	wantReason(t, wrap(`getdents64(AT_FDCWD, 0x0, 1) = 0`), reasonFDPath)
	wantReason(t, wrap(`getdents64(3, 0x0, 1) = 0`), reasonFDPath)
	wantReason(t, wrap(`getdents64(3<pipe:[1]>, 0x0, 1) = 0`), reasonFDPath)
	wantReason(t, wrap(`fchdir(x) = 0`), reasonDirfd)
	// A failed fchdir changes nothing.
	tr = mustParse(t, wrap(`fchdir(3</elsewhere>) = -1 EACCES (Permission denied)`, `stat("a", 0x1) = 0`))
	if tr.Events[1].Cwd != "/work/repo" {
		t.Fatalf("%+v", tr.Events[1])
	}
	// A failed chdir changes nothing; relative chdirs concatenate.
	tr = mustParse(t, wrap(`chdir("nope") = -1 ENOENT (No such file or directory)`, `chdir("a") = 0`, `chdir("..") = 0`, `stat("s", 0x1) = 0`))
	if tr.Events[3].Cwd != "/work/repo/a/.." {
		t.Fatalf("%+v", tr.Events[3])
	}
	long := strings.Repeat("a/", 2100)
	wantReason(t, wrap(`chdir("`+long+`") = 0`), reasonCwdTooLong)
	// Exec records the last successful exec; fexecve uses the fd path.
	tr = mustParse(t, wrap(`execve("/bin/a", ["a"], 0x0) = -1 ENOENT (No such file or directory)`, `execve("/bin/b", ["b"], 0x0) = 0`, `execveat(3</bin/c>, "", ["c"], 0x0, AT_EMPTY_PATH) = 0`))
	if tr.Procs[1].LastExec != "/bin/c" {
		t.Fatalf("%+v", tr.Procs[1])
	}
}

func TestCloneFlags(t *testing.T) {
	bad := []string{
		`clone(child_stack=NULL, flags=CLONE_UNTRACED|SIGCHLD) = 2`,
		`clone(child_stack=NULL, flags=CLONE_VM|CLONE_UNTRACED|SIGCHLD) = -1 EPERM (Operation not permitted)`,
		`clone(child_stack=NULL, flags=CLONE_NEWUSER|SIGCHLD) = 2`,
		`clone(child_stack=NULL, flags=CLONE_NEWNS) = 2`,
		`clone(child_stack=NULL, flags=CLONE_NEWPID|CLONE_NEWNET|CLONE_NEWCGROUP|CLONE_NEWIPC) = 2`,
		`clone(child_stack=NULL, flags=SIGCHLD|0x400000) = 2`,
		`clone(child_stack=NULL) = 2`,
		`clone(child_stack=NULL, flags=lower) = 2`,
	}
	for _, line := range bad {
		wantReason(t, "1  "+line+"\n1  +++ exited with 0 +++\n", reasonCloneFlags)
	}
	// A clone whose result is bad.
	wantReason(t, "1  clone(child_stack=NULL, flags=SIGCHLD) = 0\n", reasonResult)
}

func TestNetwork(t *testing.T) {
	tr := mustParse(t, fixture(t, "net.strace"))
	var ev []Event
	for _, e := range tr.Events {
		if e.Kind == KindNetwork && e.Addr != nil && e.Syscall != "socket" {
			ev = append(ev, e)
		}
	}
	got := []Addr{}
	for _, e := range ev {
		got = append(got, *e.Addr)
	}
	want := []Addr{
		{Family: "AF_INET", Address: "93.184.216.34", Port: 443},
		{Family: "AF_INET6", Address: "2001:db8::1", Port: 80},
		{Family: "AF_UNIX", Address: "/run/daemon.sock", Port: -1},
		{Family: "AF_UNIX", Address: "abstract name", Port: -1, Abstract: true},
		{Family: "AF_UNIX", Address: "/tmp/s", Port: -1},
		{Family: "AF_INET", Address: "8.8.8.8", Port: 53},
		{Family: "AF_INET", Address: "1.2.3.4", Port: 9}, // sendmsg; the one in the buffer text is ignored
		{Family: "AF_INET", Address: "5.6.7.8", Port: 7},
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("addrs\n got %+v\nwant %+v", got, want)
	}
	// EINPROGRESS is a failed call but still an event with its address.
	c := ev[0]
	if c.Syscall != "connect" || c.Result.OK || c.Result.Errno != "EINPROGRESS" || c.Path != "" {
		t.Fatalf("%+v", c)
	}
	if tr.Events[0].Addr.Family != "AF_INET" || tr.Events[0].Flags != "AF_INET, SOCK_STREAM|SOCK_CLOEXEC, IPPROTO_IP" {
		t.Fatalf("%+v", tr.Events[0])
	}
	// getsockname is ignored.
	for _, e := range tr.Events {
		if e.Syscall == "getsockname" {
			t.Fatal("getsockname recorded")
		}
	}
}

func TestSockaddrErrors(t *testing.T) {
	cases := map[string]string{
		`connect(3, 0x7ffd, 16) = 0`:                                                                    reasonSockaddr,
		`connect(3, {sin_port=htons(1)}, 16) = 0`:                                                       reasonSockaddr,
		`connect(3, {sa_family=AF_INET, sin_port=htons(1)}, 16) = 0`:                                    reasonSockaddr,
		`connect(3, {sa_family=AF_INET, sin_addr=inet_addr("1.1.1.1")}, 16) = 0`:                        reasonSockaddr,
		`connect(3, {sa_family=AF_INET, sin_port=htons(99999), sin_addr=inet_addr("1.1.1.1")}, 16) = 0`: reasonSockaddr,
		`connect(3, {sa_family=AF_INET, sin_port=80, sin_addr=inet_addr("1.1.1.1")}, 16) = 0`:           reasonSockaddr,
		`connect(3, {sa_family=AF_INET, sin_port=htons(1), sin_addr=inet_addr(1.1.1.1)}, 16) = 0`:       reasonSockaddr,
		`connect(3, {sa_family=AF_INET6, sin6_port=htons(1)}, 28) = 0`:                                  reasonSockaddr,
		`connect(3, {sa_family=AF_INET6, sin6_port=htons(1), inet_pton(AF_INET6, ::1, &x)}, 28) = 0`:    reasonSockaddr,
		`connect(3, {sa_family=AF_INET6, sin6_port=htons(1), inet_pton(AF_INET6, "::1"...)}, 28) = 0`:   reasonSockaddr,
		`connect(3, {sa_family=AF_UNIX, sun_path="/tmp/very"...}, 110) = 0`:                             reasonTruncated,
		`connect(3, {sa_family=AF_UNIX, sun_path="/tmp/\377"}, 110) = 0`:                                reasonNonUTF8,
		`connect(3, {sa_family=AF_UNIX, sun_path=/tmp/x}, 110) = 0`:                                     reasonSockaddr,
		`connect(3, {sa_family=AF_UNIX, sun_path="a" "b"}, 110) = 0`:                                    reasonSockaddr,
		`connect(3, {sa_family=AF_UNIX, sun_path="/tmp/\q"}, 110) = 0`:                                  reasonAmbiguousPath,
		`connect(3, {sa_family=unix}, 110) = 0`:                                                         reasonSockaddr,
		`sendmsg(3, {msg_name={sa_family=AF_INET}, msg_namelen=1}, 0) = 1`:                              reasonSockaddr,
		`sendmsg(3, {msg_name={sa_family=AF_INET, sin_port=htons(1)}}, 0) = 1`:                          reasonSockaddr,
		`sendmsg(3, {msg_name=[}, 0) = 1`:                                                               reasonUnparsable,
		`sendmsg(3, "abc, 0) = 1`:                                                                       reasonUnparsable,
	}
	for line, reason := range cases {
		t.Run(line, func(t *testing.T) { wantReason(t, wrap(line), reason) })
	}
	// A struct with a bad escape in its text.
	wantReason(t, wrap(`sendmsg(3, {msg_name={sa_family=AF_UNIX, sun_path="\q"}}, 0) = 1`), reasonAmbiguousPath)
}

func TestLimits(t *testing.T) {
	text := "1  stat(\"a\", 0x1) = 0\n1  +++ exited with 0 +++\n"
	_, err := Parse(strings.NewReader(text), "/", Limits{MaxLineBytes: 10})
	if err == nil || err.Error() != "trace-incomplete:size" {
		t.Fatalf("line cap: %v", err)
	}
	_, err = Parse(strings.NewReader(text), "/", Limits{MaxTotalBytes: 30})
	if err == nil || err.Error() != "trace-incomplete:size" {
		t.Fatalf("total cap: %v", err)
	}
	_, err = Parse(strings.NewReader(text), "/", Limits{MaxEvents: 1})
	if err != nil {
		t.Fatalf("one event fits: %v", err)
	}
	two := "1  stat(\"a\", 0x1) = 0\n1  stat(\"b\", 0x1) = 0\n1  +++ exited with 0 +++\n"
	if _, err = Parse(strings.NewReader(two), "/", Limits{MaxEvents: 1}); err == nil || err.Error() != "trace-incomplete:size" {
		t.Fatalf("event cap: %v", err)
	}
	// Exactly at the line cap is accepted; a huge line without newline is not.
	if _, err = Parse(strings.NewReader(text), "/", Limits{MaxLineBytes: len("1  +++ exited with 0 +++\n")}); err != nil {
		t.Fatalf("at cap: %v", err)
	}
	huge := "1  stat(\"" + strings.Repeat("a", 200000) + "\", 0x1) = 0"
	if _, err = Parse(strings.NewReader(huge), "/", Limits{MaxLineBytes: 1000}); err == nil || err.Error() != "trace-incomplete:size" {
		t.Fatalf("huge: %v", err)
	}
	if d := DefaultLimits(); d.MaxLineBytes <= 0 || d.MaxTotalBytes <= 0 || d.MaxEvents <= 0 {
		t.Fatal("defaults")
	}
}

type errReader struct{}

func (errReader) Read([]byte) (int, error) { return 0, errors.New("boom") }

func TestReadErrorAndInitialCwd(t *testing.T) {
	_, err := Parse(errReader{}, "/", Limits{})
	var e *Error
	if !errors.As(err, &e) || e.Reason != reasonRead || e.Unwrap() == nil {
		t.Fatalf("%v", err)
	}
	for _, cwd := range []string{"", "rel", "/a\x00b", "/" + strings.Repeat("a", 5000)} {
		if _, err := Parse(strings.NewReader(wrap()), cwd, Limits{}); err == nil || err.Error() != "trace-incomplete:initial-cwd" {
			t.Fatalf("cwd %q: %v", cwd, err)
		}
	}
}

func TestErrorLineNumber(t *testing.T) {
	_, err := parseText(t, "1  stat(\"a\", 0x1) = 0\n1  bogus(1) = 0\n")
	var e *Error
	if !errors.As(err, &e) || e.Line != 2 {
		t.Fatalf("%+v", e)
	}
}
