package traced

import (
	"reflect"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// A phantom execve (D33, KILN04 section 4.5) is a copy of the step's own exec,
// printed under a runtime thread that the exec killed.  It must change nothing:
// the step's read and write sets are those of the same trace without the line.
func TestPhantomExecLeavesTheReadAndWriteSetsAlone(t *testing.T) {
	const execLine = `execve("/usr/bin/sh", ["sh", "-c", "step"], 0x0 /* 1 vars */`
	build := func(ghost bool) string {
		g := ""
		if ghost {
			g = "13  " + execLine + " <unfinished ...>\n"
		}
		return `10  execve("/usr/bin/kiln", ["kiln", "sandbox-exec"], 0x0 /* 1 vars */) = 0
10  unshare(CLONE_NEWUSER) = 0
10  clone(child_stack=NULL, flags=CLONE_NEWPID|SIGCHLD) = 11
11  clone(child_stack=0x7f0000001000, flags=CLONE_VM|CLONE_SIGHAND|CLONE_THREAD|CLONE_SYSVSEM, parent_tid=[3 /* 13 in strace's PID NS */], tls=0x1) = 3 /* 13 in strace's PID NS */
11  unshare(CLONE_NEWNS) = 0
11  chdir("/start") = 0
11  ` + execLine + ` <unfinished ...>
` + g + `13  +++ exited with 0 +++
11  <... execve resumed>) = 0
11  openat(AT_FDCWD, "/w/repo/code/p/a/src/x.go", O_RDONLY) = 3</w/repo/code/p/a/src/x.go>
11  openat(AT_FDCWD, "/w/repo/code/p/a/x.o", O_WRONLY|O_CREAT|O_TRUNC, 0666) = 4</w/repo/code/p/a/x.o>
11  +++ exited with 0 +++
10  wait4(11, [{WIFEXITED(s) && WEXITSTATUS(s) == 0}], 0, NULL) = 11
10  +++ exited with 0 +++
`
	}
	classify := func(ghost bool) RawRecord {
		text := build(ghost)
		tr, err := trace.ParseOptions(strings.NewReader(text), "/start",
			trace.Options{HelperRoot: true, HelperExe: "/usr/bin/kiln"})
		if err != nil {
			t.Fatalf("ghost=%v: %v\n%s", ghost, err, text)
		}
		h := newHarness(t)
		h.image("/usr/bin/sh")
		rec, err := Classify(h.cfg, Input{Package: "code/p/a", Traces: []*trace.Trace{tr}, Recipe: []string{"step"}})
		if err != nil {
			t.Fatal(err)
		}
		return *rec
	}
	with, without := classify(true), classify(false)
	eq(t, "writes", without.Writes, "code/p/a/x.o")
	if !reflect.DeepEqual(with.Entries, without.Entries) || !reflect.DeepEqual(with.Writes, without.Writes) ||
		!reflect.DeepEqual(with.Reasons, without.Reasons) {
		t.Fatalf("the ghost changed the record:\n with    %+v\n without %+v", with, without)
	}
}
