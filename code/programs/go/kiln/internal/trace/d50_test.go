package trace

import (
	"errors"
	"strings"
	"testing"
)

// KILN04 D50: two shapes the model used to refuse that are provably safe.
//
//	rule 2b  a fork, vfork or clone that ended `= ? ERESTARTNOINTR`
//	rule 2c  a successful read-only open or openat with no -y annotation
//
// Every Parse here runs both readers under the cross-check hook, so each case
// is also a parity test between them.

const (
	d50Flags   = "child_stack=NULL, flags=CLONE_CHILD_CLEARTID|CLONE_CHILD_SETTID|SIGCHLD, child_tidptr=0x7f0000001000"
	d50Restart = " = ? ERESTARTNOINTR (To be restarted)"
)

func d50Reason(t *testing.T, text string) string {
	t.Helper()
	_, err := parseText(t, text)
	if err == nil {
		return ""
	}
	var e *Error
	if !errors.As(err, &e) {
		t.Fatalf("not a trace error: %v", err)
	}
	return e.Reason
}

func TestRestartedSpawnAccepted(t *testing.T) {
	forms := map[string]string{
		"clone inline": "1  clone(" + d50Flags + ")" + d50Restart + "\n",
		"clone split":  "1  clone(" + d50Flags + " <unfinished ...>\n1  <... clone resumed>)" + d50Restart + "\n",
		"fork inline":  "1  fork()" + d50Restart + "\n",
		"vfork inline": "1  vfork()" + d50Restart + "\n",
		"fork split":   "1  fork( <unfinished ...>\n1  <... fork resumed>)" + d50Restart + "\n",
	}
	for name, line := range forms {
		t.Run(name, func(t *testing.T) {
			tr := mustParse(t, line+"1  +++ exited with 0 +++\n")
			if len(tr.Events) != 0 || len(tr.Procs) != 1 {
				t.Fatalf("a restarted spawn left events %+v or processes %v", tr.Events, tr.Procs)
			}
			if tr.RestartedSpawns != 1 {
				t.Fatalf("RestartedSpawns = %d, want 1", tr.RestartedSpawns)
			}
		})
	}
}

// The restart is not paired with the call that follows: a later clone of the
// same pid spawns normally.
func TestRestartedSpawnDoesNotPairWithTheNextClone(t *testing.T) {
	text := "1  clone(" + d50Flags + ")" + d50Restart + "\n" +
		"1  --- SIGUSR1 {si_signo=SIGUSR1, si_code=SI_USER, si_pid=7, si_uid=0} ---\n" +
		"1  clone(child_stack=NULL, flags=SIGCHLD) = 2\n" +
		"2  +++ exited with 0 +++\n1  +++ exited with 0 +++\n"
	tr := mustParse(t, text)
	if len(tr.Procs) != 2 || tr.Procs[2] == nil || tr.Procs[2].Parent != 1 || tr.RestartedSpawns != 1 {
		t.Fatalf("procs %v, restarted %d", tr.Procs, tr.RestartedSpawns)
	}
	// Split first call, whole second call.
	text = "1  clone(" + d50Flags + " <unfinished ...>\n1  <... clone resumed>)" + d50Restart + "\n" +
		"1  clone(" + d50Flags + ") = 3\n3  +++ exited with 0 +++\n1  +++ exited with 0 +++\n"
	if tr := mustParse(t, text); tr.Procs[3] == nil || tr.RestartedSpawns != 1 {
		t.Fatalf("procs %v", tr.Procs)
	}
}

// A restarted split call spends its saved start: the next relative open uses
// the cwd it sees, not one left over from the restarted clone.
func TestRestartedSplitSpawnLeavesNoStaleStart(t *testing.T) {
	text := "1  clone(" + d50Flags + " <unfinished ...>\n1  <... clone resumed>)" + d50Restart + "\n" +
		"1  chdir(\"/other\") = 0\n1  openat(AT_FDCWD, \"x\", O_RDONLY) = 3</other/x>\n1  +++ exited with 0 +++\n"
	tr := mustParse(t, text)
	for _, e := range tr.Events {
		if e.Syscall == "openat" && e.Cwd != "/other" {
			t.Fatalf("cwd %q", e.Cwd)
		}
	}
}

func TestRestartedSpawnNeighboursRefused(t *testing.T) {
	cases := map[string]struct{ text, reason string }{
		"ERESTARTSYS on clone":  {"1  clone(" + d50Flags + ") = ? ERESTARTSYS (To be restarted if SA_RESTART is set)\n", reasonResult},
		"ERESTARTNOHAND":        {"1  clone(" + d50Flags + ") = ? ERESTARTNOHAND (To be restarted if no handler)\n", reasonResult},
		"ERESTART_RESTARTBLOCK": {"1  fork() = ? ERESTART_RESTARTBLOCK (Interrupted by signal)\n", reasonResult},
		"ERESTARTSYS on fork":   {"1  fork() = ? ERESTARTSYS (To be restarted if SA_RESTART is set)\n", reasonResult},
		"on openat":             {"1  openat(AT_FDCWD, \"/repo/a\", O_RDONLY)" + d50Restart + "\n", reasonResult},
		"on stat":               {"1  stat(\"/repo/a\", 0x1)" + d50Restart + "\n", reasonResult},
		"bare unknown":          {"1  clone(" + d50Flags + ") = ?\n", reasonResult},
		"bare unknown fork":     {"1  fork() = ?\n", reasonResult},
		"clone3":                {"1  clone3({flags=CLONE_VM|CLONE_VFORK, exit_signal=SIGCHLD, stack=0x7f0a4b1ff000, stack_size=0x9000}, 88)" + d50Restart + "\n", ""},
		"unknown flag token":    {"1  clone(child_stack=NULL, flags=CLONE_BOGUS|SIGCHLD)" + d50Restart + "\n", reasonResult},
		"CLONE_UNTRACED":        {"1  clone(child_stack=NULL, flags=CLONE_UNTRACED|SIGCHLD)" + d50Restart + "\n", reasonCloneFlags},
		"CLONE_NEWNS":           {"1  clone(child_stack=NULL, flags=CLONE_NEWNS|SIGCHLD)" + d50Restart + "\n", reasonCloneFlags},
		"hex flags":             {"1  clone(child_stack=NULL, flags=0x11)" + d50Restart + "\n", reasonCloneFlags},
		"errno name suffix":     {"1  clone(" + d50Flags + ") = ? ERESTARTNOINTRX (To be restarted)\n", reasonResult},
	}
	for name, c := range cases {
		t.Run(name, func(t *testing.T) {
			got := d50Reason(t, c.text+"1  +++ exited with 0 +++\n")
			if got == "" {
				t.Fatalf("accepted")
			}
			if c.reason != "" && got != c.reason {
				t.Fatalf("reason %q, want %q", got, c.reason)
			}
		})
	}
}

// ---- rule 2c ----

func d50Open(call string) string { return "1  " + call + "\n1  +++ exited with 0 +++\n" }

func TestBareReadOnlyOpenAccepted(t *testing.T) {
	for _, flags := range []string{
		"O_RDONLY", "O_RDONLY|O_CLOEXEC", "O_RDONLY|O_NONBLOCK|O_NDELAY", "O_RDONLY|O_DIRECTORY|O_CLOEXEC",
		"O_RDONLY|O_NOFOLLOW|O_LARGEFILE|O_NOCTTY|O_NOATIME", "O_PATH|O_CLOEXEC", "O_PATH|O_DIRECTORY", "O_RDONLY|O_PATH",
	} {
		for _, call := range []string{
			"openat(AT_FDCWD, \"/repo/a.txt\", " + flags + ") = 3",
			"openat(AT_FDCWD, \"rel/a.txt\", " + flags + ") = 3",
			"open(\"/repo/a.txt\", " + flags + ") = 3",
			"openat(AT_FDCWD, \"/repo/a.txt\", " + flags + ")             = 3",
		} {
			tr := mustParse(t, d50Open(call))
			if len(tr.Events) != 1 || tr.Events[0].ReturnFDPath != "" || !tr.Events[0].Result.OK || tr.BareOpens != 1 {
				t.Fatalf("%s: %+v bare=%d", call, tr.Events, tr.BareOpens)
			}
		}
	}
	// A directory descriptor with an annotation is an existing rule, kept.
	tr := mustParse(t, "1  openat(3</repo>, \"a.txt\", O_RDONLY|O_CLOEXEC) = 4\n1  +++ exited with 0 +++\n")
	if tr.Events[0].DirfdPath != "/repo" || tr.BareOpens != 1 {
		t.Fatalf("%+v", tr.Events[0])
	}
	// A split call.
	tr = mustParse(t, "1  openat(AT_FDCWD, \"/repo/a\", O_RDONLY <unfinished ...>\n1  <... openat resumed>) = 3\n1  +++ exited with 0 +++\n")
	if len(tr.Events) != 1 || tr.BareOpens != 1 {
		t.Fatalf("%+v", tr.Events)
	}
}

func TestBareOpenAnnotatedIsNotCounted(t *testing.T) {
	tr := mustParse(t, d50Open("openat(AT_FDCWD, \"/repo/a\", O_RDONLY) = 3</repo/a>"))
	if tr.BareOpens != 0 || tr.Events[0].ReturnFDPath != "/repo/a" {
		t.Fatalf("%+v", tr)
	}
}

func TestBareOpenNeighboursRefused(t *testing.T) {
	cases := map[string]struct{ call, reason string }{
		"O_RDWR":                     {"openat(AT_FDCWD, \"/repo/a\", O_RDWR) = 3", reasonFDPath},
		"O_WRONLY":                   {"openat(AT_FDCWD, \"/repo/a\", O_WRONLY) = 3", reasonFDPath},
		"O_CREAT":                    {"openat(AT_FDCWD, \"/repo/a\", O_RDONLY|O_CREAT, 0644) = 3", reasonFDPath},
		"O_TRUNC":                    {"openat(AT_FDCWD, \"/repo/a\", O_RDONLY|O_TRUNC) = 3", reasonFDPath},
		"O_TMPFILE":                  {"openat(AT_FDCWD, \"/repo\", O_RDONLY|O_TMPFILE, 0600) = 3", reasonFDPath},
		"O_APPEND":                   {"openat(AT_FDCWD, \"/repo/a\", O_RDONLY|O_APPEND) = 3", reasonFDPath},
		"O_EXCL":                     {"openat(AT_FDCWD, \"/repo/a\", O_RDONLY|O_EXCL) = 3", reasonFDPath},
		"unknown token":              {"openat(AT_FDCWD, \"/repo/a\", O_RDONLY|O_BOGUS) = 3", reasonFDPath},
		"numeric residue":            {"openat(AT_FDCWD, \"/repo/a\", O_RDONLY|0x40000) = 3", reasonFDPath},
		"decimal residue":            {"openat(AT_FDCWD, \"/repo/a\", O_RDONLY|64) = 3", reasonFDPath},
		"bare number flags":          {"openat(AT_FDCWD, \"/repo/a\", 0) = 3", reasonFDPath},
		"no access mode":             {"openat(AT_FDCWD, \"/repo/a\", O_CLOEXEC) = 3", reasonFDPath},
		"O_DIRECTORY alone":          {"openat(AT_FDCWD, \"/repo/a\", O_DIRECTORY) = 3", reasonFDPath},
		"lowercase":                  {"openat(AT_FDCWD, \"/repo/a\", o_rdonly) = 3", reasonFDPath},
		"hex result":                 {"openat(AT_FDCWD, \"/repo/a\", O_RDONLY) = 0x3", reasonFDPath},
		"pidns note":                 {"openat(AT_FDCWD, \"/repo/a\", O_RDONLY) = 3 /* 4 in strace's PID NS */", ""},
		"openat2":                    {"openat2(AT_FDCWD, \"/repo/a\", {flags=O_RDONLY, mode=0, resolve=0}, 24) = 3", reasonFDPath},
		"creat":                      {"creat(\"/repo/a\", 0644) = 3", reasonFDPath},
		"hex path":                   {"openat(AT_FDCWD, 0x7ffd12345678, O_RDONLY) = 3", ""},
		"NULL path":                  {"openat(AT_FDCWD, NULL, O_RDONLY) = 3", ""},
		"malformed annot":            {"openat(AT_FDCWD, \"/repo/a\", O_RDONLY) = 3</repo/a", ""},
		"dirfd number without annot": {"openat(3, \"a\", O_RDONLY|O_CLOEXEC) = 4", ""},
	}
	for name, c := range cases {
		t.Run(name, func(t *testing.T) {
			got := d50Reason(t, d50Open(c.call))
			if got == "" {
				t.Fatalf("accepted: %s", c.call)
			}
			if c.reason != "" && got != c.reason {
				t.Fatalf("reason %q, want %q", got, c.reason)
			}
		})
	}
}

// A later call on the returned descriptor that has no annotation of its own is
// refused by its own rule: the open's acceptance vouches for nothing else.
func TestBareOpenLaterCallsStillNeedAnnotations(t *testing.T) {
	for _, later := range []string{
		"getdents64(3, 0x7f0000001000 /* 0 entries */, 32768) = 0",
		"fchdir(3) = 0",
		"newfstatat(3, \"\", {st_mode=S_IFREG|0644, st_size=0, ...}, AT_EMPTY_PATH) = 0",
		"openat(3, \"x\", O_RDONLY) = 4</repo/x>",
	} {
		text := "1  openat(AT_FDCWD, \"/repo\", O_RDONLY|O_DIRECTORY) = 3\n1  " + later + "\n1  +++ exited with 0 +++\n"
		if got := d50Reason(t, text); got == "" {
			t.Errorf("accepted a later %s on an un-annotated descriptor", later)
		}
	}
	// fstat on the descriptor is an ignored-class call: the model never looks at
	// it, before this rule or after.
	if got := d50Reason(t, "1  openat(AT_FDCWD, \"/repo\", O_RDONLY) = 3\n1  fstat(3, {st_mode=S_IFDIR|0755, st_size=4096, ...}) = 0\n1  +++ exited with 0 +++\n"); strings.Contains(got, "xx") {
		t.Fatal(got)
	}
}
