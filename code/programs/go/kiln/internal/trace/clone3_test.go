package trace

import (
	"strings"
	"testing"
)

// Lines of a real strace 6.8 run of the sandbox root (the first CI run of
// TestSandboxRealStrace), pids renumbered.  The Go runtime makes its threads
// with clone3, which strace prints as a struct with an "=> {...}" output
// annotation.
const (
	realThreadFlags = "CLONE_VM|CLONE_FS|CLONE_FILES|CLONE_SIGHAND|CLONE_THREAD|CLONE_SYSVSEM|CLONE_SETTLS|CLONE_PARENT_SETTID|CLONE_CHILD_CLEARTID"
	realClone3Tail  = ", child_tid=0x7fada74bf990, parent_tid=0x7fada74bf990, exit_signal=0, stack=0x7fada64bf000, stack_size=0xffff80, tls=0x7fada74bf6c0}"
)

func clone3Inline(pid, child int, flags string) string {
	return itoa(pid) + "  clone3({flags=" + flags + realClone3Tail + " => {parent_tid=[" + itoa(child) + "]}, 88) = " + itoa(child) + "\n"
}

func clone3Split(pid, child int, flags string) string {
	return itoa(pid) + "  clone3({flags=" + flags + realClone3Tail + " <unfinished ...>\n" +
		itoa(pid) + "  <... clone3 resumed> => {parent_tid=[" + itoa(child) + "]}, 88) = " + itoa(child) + "\n"
}

func itoa(n int) string {
	if n == 0 {
		return "0"
	}
	var b []byte
	for ; n > 0; n /= 10 {
		b = append([]byte{byte('0' + n%10)}, b...)
	}
	return string(b)
}

// The lines of a real root's startup that are not clone3.
const realRootLines = "10  access(\"/etc/ld.so.preload\", R_OK) = -1 ENOENT (No such file or directory)\n" +
	"10  openat(AT_FDCWD</start>, \"/etc/ld.so.cache\", O_RDONLY|O_CLOEXEC) = 3</etc/ld.so.cache>\n" +
	"10  openat(AT_FDCWD</start>, \"/proc/self/maps\", O_RDONLY|O_CLOEXEC) = 3</proc/10/maps>\n" +
	"11  tgkill(10, 13, SIGURG)        = 0\n" +
	"10  readlinkat(AT_FDCWD</start>, \"/proc/self/exe\", \"/tmp/go-build207559618/b001/kiln.test\", 128) = 37\n" +
	"10  getcwd(\"/start\", 4096) = 6\n" +
	"10  newfstatat(AT_FDCWD</start>, \"/tmp\", {st_mode=S_IFDIR|S_ISVTX|0777, st_size=4096, ...}, AT_SYMLINK_NOFOLLOW) = 0\n"

func TestClone3ThreadsOfTheHelperRoot(t *testing.T) {
	// Inline and split forms, threads of the root group; the root's own events
	// are discarded but every line must still parse and every thread be a spawn.
	text := helperRootExec + realRootLines +
		clone3Inline(10, 12, realThreadFlags) +
		clone3Inline(10, 13, realThreadFlags) +
		"10  clone3({flags=" + realThreadFlags + realClone3Tail + " <unfinished ...>\n" +
		"12  clone3({flags=" + realThreadFlags + realClone3Tail + " <unfinished ...>\n" +
		"10  <... clone3 resumed> => {parent_tid=[14]}, 88) = 14\n" +
		"12  <... clone3 resumed> => {parent_tid=[15]}, 88) = 15\n" +
		helperRootSetup[len(helperRootExec):] + helperInitSetup + helperReExec + helperExec +
		"12  +++ exited with 0 +++\n13  +++ exited with 0 +++\n14  +++ exited with 0 +++\n15  +++ exited with 0 +++\n" + helperEnd
	tr, err := parseAt(t, "/start", text, helperOpts)
	if err != nil {
		t.Fatal(err)
	}
	for _, pid := range []int{12, 13, 14, 15} {
		p := tr.Procs[pid]
		if p == nil || !strings.Contains(p.CloneFlags, "CLONE_THREAD") || !p.ShareFS {
			t.Errorf("thread %d: %+v", pid, p)
		}
	}
	if tr.Procs[15].Parent != 12 {
		t.Errorf("thread 15's parent is %d, want 12 (a thread of the root)", tr.Procs[15].Parent)
	}
}

func TestClone3InTheInitWindowIsTrackedLikeClone(t *testing.T) {
	// A thread by clone3: tolerated, and a later chdir is then a cwd-race with
	// it (it shares the cwd) until unshare(CLONE_FS).
	thr := clone3Inline(11, 20, realThreadFlags) + "20  futex(0x1, FUTEX_WAIT, 0, NULL) = 0\n"
	exit := "20  +++ exited with 0 +++\n"
	ok := helperRootSetup + thr + "11  unshare(CLONE_FS) = 0\n11  chdir(\"/elsewhere\") = 0\n11  chdir(\"/start\") = 0\n" + helperExec + exit + helperEnd
	mustHelper(t, ok)
	racy := helperRootSetup + thr + "11  chdir(\"/elsewhere\") = 0\n11  chdir(\"/start\") = 0\n" + helperExec + exit + helperEnd
	wantReasonOpts(t, racy, helperOpts, reasonCwdRace)
	// A split clone3 too.
	mustHelper(t, helperRootSetup+clone3Split(11, 21, realThreadFlags)+"21  +++ exited with 0 +++\n"+helperExec+helperEnd)
	// Never a second process in the init's window: helper-spawn.
	wantReasonOpts(t, helperRootSetup+clone3Inline(11, 22, "CLONE_VM|CLONE_VFORK")+helperExec+"22  +++ exited with 0 +++\n"+helperEnd, helperOpts, reasonHelperSpawn)
}

func TestClone3PolicyIsUnchangedOutsideTheSetupWindows(t *testing.T) {
	// The step's own clone3 is refused (the sandbox filter answers ENOSYS, so
	// a clone3 in a trace of the step means the filter failed).
	step := helperRootSetup + helperInitSetup + helperReExec + helperExec
	wantReasonOpts(t, step+clone3Inline(11, 30, realThreadFlags)+"30  +++ exited with 0 +++\n"+helperEnd, helperOpts, "refused:clone3")
	// And without the helper option, any clone3 is refused.
	_, err := parseAt(t, "/r", "1  clone3({flags=CLONE_VM"+realClone3Tail+", 88) = -1 ENOSYS (Function not implemented)\n1  +++ exited with 0 +++\n", Options{})
	if err == nil || err.Error() != "trace-incomplete:refused:clone3" {
		t.Fatalf("%v", err)
	}
}

func TestClone3StructsItCannotDecodeAreCloneFlags(t *testing.T) {
	cases := map[string]string{
		"a pointer, not a struct": "11  clone3(0x7ffd1234, 88) = 40\n",
		"no leading flags":        "11  clone3({exit_signal=0, flags=" + realThreadFlags + "}, 88) = 40\n",
		"truncated struct":        "11  clone3({flags=" + realThreadFlags + ", ...}, 88) = 40\n",
		"numeric flags":           "11  clone3({flags=0x7d0f00, exit_signal=0}, 88) = 40\n",
		"a namespace flag":        "11  clone3({flags=CLONE_NEWNS|CLONE_VM, exit_signal=0}, 88) = 40\n",
		"CLONE_UNTRACED":          "11  clone3({flags=CLONE_UNTRACED|CLONE_VM, exit_signal=0}, 88) = 40\n",
		"no size argument":        "11  clone3({flags=" + realThreadFlags + ", exit_signal=0}) = 40\n",
		"text after the struct":   "11  clone3({flags=" + realThreadFlags + ", exit_signal=0} junk, 88) = 40\n",
		"empty flags":             "11  clone3({flags=, exit_signal=0}, 88) = 40\n",
		"unbalanced":              "11  clone3({flags=" + realThreadFlags + ", exit_signal=0, 88) = 40\n",
	}
	for name, line := range cases {
		text := helperRootSetup + line + "40  +++ exited with 0 +++\n" + helperExec + helperEnd
		_, err := parseAt(t, "/start", text, helperOpts)
		if err == nil {
			t.Errorf("%s: accepted", name)
			continue
		}
		if name != "unbalanced" && !strings.Contains(err.Error(), "clone-flags") {
			t.Errorf("%s: %v, want clone-flags", name, err)
		}
	}
	// A failed clone3 spawns nothing, so there is nothing to decode.
	mustHelper(t, helperRootSetup+"11  clone3({flags=CLONE_VM, exit_signal=0}, 88) = -1 ENOSYS (Function not implemented)\n"+helperExec+helperEnd)
}
