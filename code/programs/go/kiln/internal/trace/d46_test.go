package trace

import "testing"

// KILN04 D46: a chdir or fchdir on a cwd shared with another live process is
// accepted unless a sharer's cwd-dependent call overlaps its interval.
//
// Every test here goes through ParseOptions, which this package's tests hook
// to run BOTH trace readers and panic if they disagree (crosscheck_init_test.go),
// so each case is a parity check as well.
//
// The cast: pid 1 is the root; it clones pid 2 with CLONE_FS, so they share one
// cwd.  The initial cwd is /repo.

const (
	d46Share = "1  clone(child_stack=NULL, flags=CLONE_VM|CLONE_FS|SIGCHLD) = 2\n"
	d46Exits = "2  +++ exited with 0 +++\n1  +++ exited with 0 +++\n"
)

func d46Accept(t *testing.T, body string) *Trace {
	t.Helper()
	tr, err := parseAt(t, "/repo", d46Share+body+d46Exits, Options{})
	if err != nil {
		t.Fatalf("want accepted, got %v\n%s", err, body)
	}
	return tr
}

func d46Refuse(t *testing.T, body string) {
	t.Helper()
	_, err := parseAt(t, "/repo", d46Share+body+d46Exits, Options{})
	if err == nil || err.Error() != "trace-incomplete:"+reasonCwdRace {
		t.Fatalf("want cwd-race, got %v\n%s", err, body)
	}
}

// d46Cwd is the cwd the model gave the openat of file f.
func d46Cwd(t *testing.T, tr *Trace, f string) string {
	t.Helper()
	for _, e := range tr.Events {
		if e.Syscall == "openat" && e.Path == f {
			return e.Cwd
		}
	}
	t.Fatalf("no openat of %q in %+v", f, tr.Events)
	return ""
}

func TestD46ChdirWithNoOverlapIsAccepted(t *testing.T) {
	// The sharers' calls are wholly before or wholly after the chdir, so the
	// order is the line order, and later relative paths use the new cwd.
	tr := d46Accept(t,
		"2  openat(AT_FDCWD, \"before\", O_RDONLY) = 3</repo/before>\n"+
			"2  chdir(\"/x\") = 0\n"+
			"1  openat(AT_FDCWD, \"after\", O_RDONLY) = 4</x/after>\n")
	if got := d46Cwd(t, tr, "before"); got != "/repo" {
		t.Errorf("before = %q", got)
	}
	if got := d46Cwd(t, tr, "after"); got != "/x" {
		t.Errorf("after = %q", got)
	}
}

func TestD46InterruptedChdirWithSharerOutsideItsInterval(t *testing.T) {
	// The chdir spans lines, but the sharer's calls sit before its start and
	// after its completion.
	tr := d46Accept(t,
		"2  openat(AT_FDCWD, \"before\", O_RDONLY) = 3</repo/before>\n"+
			"1  chdir(\"/x\" <unfinished ...>\n"+
			"1  <... chdir resumed>)  = 0\n"+
			"2  openat(AT_FDCWD, \"after\", O_RDONLY) = 4</x/after>\n")
	if got := d46Cwd(t, tr, "after"); got != "/x" {
		t.Errorf("after = %q", got)
	}
}

func TestD46OverlappingRelativeCallRefuses(t *testing.T) {
	// A sharer's relative open that starts and ends inside the chdir.
	d46Refuse(t,
		"1  chdir(\"/x\" <unfinished ...>\n"+
			"2  openat(AT_FDCWD, \"f\", O_RDONLY) = 3</repo/f>\n"+
			"1  <... chdir resumed>)  = 0\n")
	// A sharer's open that starts before the chdir and ends after it began
	// (the chdir is uninterrupted: it is the single line in the middle).
	d46Refuse(t,
		"2  openat(AT_FDCWD, \"f\", O_RDONLY <unfinished ...>\n"+
			"1  chdir(\"/x\") = 0\n"+
			"2  <... openat resumed>)  = 3</x/f>\n")
	// A sharer's open that starts inside the chdir and ends after it.
	d46Refuse(t,
		"1  chdir(\"/x\" <unfinished ...>\n"+
			"2  openat(AT_FDCWD, \"f\", O_RDONLY <unfinished ...>\n"+
			"1  <... chdir resumed>)  = 0\n"+
			"2  <... openat resumed>)  = 3</x/f>\n")
	// A sharer's open that starts before the chdir and ends inside it.
	d46Refuse(t,
		"2  openat(AT_FDCWD, \"f\", O_RDONLY <unfinished ...>\n"+
			"1  chdir(\"/x\" <unfinished ...>\n"+
			"2  <... openat resumed>)  = 3</repo/f>\n"+
			"1  <... chdir resumed>)  = 0\n")
	// An open of a relative path under a dirfd of AT_FDCWD only; a stat too.
	d46Refuse(t,
		"1  chdir(\"/x\" <unfinished ...>\n"+
			"2  stat(\"f\", {st_mode=S_IFREG|0644}) = 0\n"+
			"1  <... chdir resumed>)  = 0\n")
}

func TestD46OverlappingCallThatIgnoresTheCwdIsFine(t *testing.T) {
	// An absolute path does not depend on the cwd.
	d46Accept(t,
		"1  chdir(\"/x\" <unfinished ...>\n"+
			"2  openat(AT_FDCWD, \"/abs/f\", O_RDONLY) = 3</abs/f>\n"+
			"1  <... chdir resumed>)  = 0\n")
	// A relative name under a real directory descriptor depends on that
	// descriptor (its -y path), not on the cwd.
	d46Accept(t,
		"1  chdir(\"/x\" <unfinished ...>\n"+
			"2  openat(5</d>, \"f\", O_RDONLY) = 3</d/f>\n"+
			"1  <... chdir resumed>)  = 0\n")
	// An absolute call that straddles the chdir the other way round.
	d46Accept(t,
		"2  openat(AT_FDCWD, \"/abs/f\", O_RDONLY <unfinished ...>\n"+
			"1  chdir(\"/x\") = 0\n"+
			"2  <... openat resumed>)  = 3</abs/f>\n")
}

func TestD46ForkChildIsNotASharer(t *testing.T) {
	// A fork child COPIES the cwd, so its relative open during the parent's
	// chdir is the child's own business.  Pid 3 is a copy of pid 1's cwd.
	text := "1  clone(child_stack=NULL, flags=SIGCHLD) = 3\n" +
		"1  chdir(\"/x\" <unfinished ...>\n" +
		"3  openat(AT_FDCWD, \"f\", O_RDONLY) = 3</repo/f>\n" +
		"1  <... chdir resumed>)  = 0\n" +
		"3  +++ exited with 0 +++\n" +
		"1  +++ exited with 0 +++\n"
	tr, err := parseAt(t, "/repo", text, Options{})
	if err != nil {
		t.Fatal(err)
	}
	if got := d46Cwd(t, tr, "f"); got != "/repo" {
		t.Errorf("child cwd = %q, want /repo", got)
	}
}

func TestD46CopyingSpawnIsCwdDependent(t *testing.T) {
	// A sharer that forks inside the chdir copies a cwd that is ambiguous.
	d46Refuse(t,
		"1  chdir(\"/x\" <unfinished ...>\n"+
			"2  fork() = 5\n"+
			"1  <... chdir resumed>)  = 0\n"+
			"5  +++ exited with 0 +++\n")
	// A sharer's CLONE_FS thread inside the chdir shares, it copies nothing.
	d46Accept(t,
		"1  chdir(\"/x\" <unfinished ...>\n"+
			"2  clone(child_stack=NULL, flags=CLONE_VM|CLONE_FS|SIGCHLD) = 6\n"+
			"1  <... chdir resumed>)  = 0\n"+
			"6  +++ exited with 0 +++\n")
}

func TestD46FailedChdirChangesNothing(t *testing.T) {
	// An absolute chdir that fails reads no cwd and writes none: overlapping
	// a sharer's relative open is fine, and the cwd stays.
	tr := d46Accept(t,
		"1  chdir(\"/nope\" <unfinished ...>\n"+
			"2  openat(AT_FDCWD, \"in\", O_RDONLY) = 3</repo/in>\n"+
			"1  <... chdir resumed>)  = -1 ENOENT (No such file or directory)\n"+
			"2  openat(AT_FDCWD, \"out\", O_RDONLY) = 4</repo/out>\n")
	if got := d46Cwd(t, tr, "out"); got != "/repo" {
		t.Errorf("cwd after a failed chdir = %q", got)
	}
	// ... and an absolute failed chdir overlapping a sharer's SUCCESSFUL chdir
	// is fine: only the successful one writes.
	d46Accept(t,
		"1  chdir(\"/nope\" <unfinished ...>\n"+
			"2  chdir(\"/x\") = 0\n"+
			"1  <... chdir resumed>)  = -1 ENOENT (No such file or directory)\n")
	// A failed RELATIVE chdir still read the cwd to resolve its name, so a
	// sharer's chdir inside its interval makes its result ambiguous.
	d46Refuse(t,
		"1  chdir(\"rel\" <unfinished ...>\n"+
			"2  chdir(\"/x\") = 0\n"+
			"1  <... chdir resumed>)  = -1 ENOENT (No such file or directory)\n")
}

func TestD46TwoOverlappingChdirsRefuse(t *testing.T) {
	// Both absolute: neither reads the cwd, but the final cwd depends on which
	// took effect last, and the lines cannot say.
	d46Refuse(t,
		"1  chdir(\"/a\" <unfinished ...>\n"+
			"2  chdir(\"/b\") = 0\n"+
			"1  <... chdir resumed>)  = 0\n")
	// Back to back they are ordered.
	tr := d46Accept(t,
		"1  chdir(\"/a\") = 0\n"+
			"2  chdir(\"/b\") = 0\n"+
			"1  openat(AT_FDCWD, \"f\", O_RDONLY) = 3</b/f>\n")
	if got := d46Cwd(t, tr, "f"); got != "/b" {
		t.Errorf("cwd = %q", got)
	}
}

func TestD46Fchdir(t *testing.T) {
	// The directory is the descriptor's -y name; later relative paths use it.
	tr := d46Accept(t,
		"2  fchdir(3</x>) = 0\n"+
			"1  openat(AT_FDCWD, \"f\", O_RDONLY) = 4</x/f>\n")
	if got := d46Cwd(t, tr, "f"); got != "/x" {
		t.Errorf("cwd = %q", got)
	}
	// Overlapping a sharer's relative call refuses ...
	d46Refuse(t,
		"1  fchdir(3</x> <unfinished ...>\n"+
			"2  openat(AT_FDCWD, \"f\", O_RDONLY) = 4</repo/f>\n"+
			"1  <... fchdir resumed>)  = 0\n")
	// ... an absolute call is fine ...
	d46Accept(t,
		"1  fchdir(3</x> <unfinished ...>\n"+
			"2  openat(AT_FDCWD, \"/abs/f\", O_RDONLY) = 4</abs/f>\n"+
			"1  <... fchdir resumed>)  = 0\n")
	// ... a failed fchdir changes nothing, so nothing can race with it ...
	d46Accept(t,
		"1  fchdir(3</x> <unfinished ...>\n"+
			"2  openat(AT_FDCWD, \"f\", O_RDONLY) = 4</repo/f>\n"+
			"1  <... fchdir resumed>)  = -1 ENOTDIR (Not a directory)\n")
	// ... and a sharer call that is wholly before is fine.
	d46Accept(t,
		"2  openat(AT_FDCWD, \"f\", O_RDONLY) = 4</repo/f>\n"+
			"1  fchdir(3</x>) = 0\n")
}

func TestD46UnsharedChdirIsNeverARace(t *testing.T) {
	// No sharer at all: overlapping lines of OTHER processes change nothing.
	text := "1  chdir(\"/x\" <unfinished ...>\n" +
		"1  <... chdir resumed>)  = 0\n" +
		"1  +++ exited with 0 +++\n"
	if _, err := parseAt(t, "/repo", text, Options{}); err != nil {
		t.Fatal(err)
	}
}
