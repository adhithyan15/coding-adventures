package traced

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

func TestFollowRules(t *testing.T) {
	for _, c := range []struct {
		sys, flags string
		meta, wr   bool
	}{
		{"stat", "", true, true},
		{"lstat", "", false, true},
		{"readlink", "", false, true},
		{"newfstatat", "AT_SYMLINK_NOFOLLOW", false, false},
		{"statx", "AT_SYMLINK_NOFOLLOW|AT_STATX_SYNC_AS_STAT", false, false},
		{"inotify_add_watch", "IN_DONT_FOLLOW", false, true},
		{"lchown", "", true, false},
		{"lsetxattr", "", true, false},
	} {
		e := trace.Event{Syscall: c.sys, Flags: c.flags}
		if got := metaFollow(&e); c.sys != "lchown" && c.sys != "lsetxattr" && got != c.meta {
			t.Errorf("metaFollow(%s %s) = %v", c.sys, c.flags, got)
		}
		if got := writeFollow(&e); got != c.wr {
			t.Errorf("writeFollow(%s %s) = %v", c.sys, c.flags, got)
		}
	}
	// A change in place does not follow a link it was told not to.
	h := newHarness(t)
	h.tree.link("code/p/b/ln", "b.txt")
	h.fs.link(abs("code/p/b/ln"), "b.txt")
	nofollow := write("fchownat", "code/p/b/ln")
	nofollow.Flags = "AT_SYMLINK_NOFOLLOW"
	rec := h.run(nofollow)
	eq(t, "tracked", rec.TrackedWrites, "code/p/b/ln")
	rec = h.run(write("chmod", "code/p/b/ln"))
	eq(t, "tracked", rec.TrackedWrites, "code/p/b/b.txt") // the link is only crossed; its target is written
}

func TestLinkAndRenameFailures(t *testing.T) {
	h := newHarness(t)
	rec := h.run(
		fail(link("code/p/b/b.txt", "code/p/a/x"), "EEXIST"),
		fail(link("code/p/b/gone", "code/p/a/y"), "ENOENT"),
	)
	eq(t, "entries", rec.Entries, "pkg:code/p/a", "pkg:code/p/b")
	eq(t, "writes", rec.Writes)
	// The failed ENOENT did not make the destination "provably absent".
	rec = h.run(
		fail(rename("code/p/b/gone", "code/p/a/z"), "ENOENT"),
		open("code/p/a/z", "O_WRONLY|O_CREAT", abs("code/p/a/z")),
	)
	eq(t, "untracked", rec.UntrackedObservations, "code/p/a/z")
}

func TestRenamedLinksAndHardLinksAreNeverStable(t *testing.T) {
	// A renamed link is not "created once": the rename is a change the trace
	// cannot vouch for, so reading through the new name is unstable.
	h := newHarness(t)
	h.fs.link(abs("code/p/a/final-link"), "../b")
	rec := h.run(
		symlink("../b", "code/p/a/tmp-link"),
		rename("code/p/a/tmp-link", "code/p/a/final-link"),
		stat("code/p/a/final-link/b.txt"),
	)
	wantReason(t, rec, "trace-incomplete:unstable-path")

	// A directory renamed into place and then only named, not traversed.
	h = newHarness(t)
	h.fs.dir(abs("code/p/a/d2"))
	rec = h.run(mkdir("code/p/a/d1"), rename("code/p/a/d1", "code/p/a/d2"), lstat("code/p/a/d2"))
	eq(t, "entries", rec.Entries, "pkg:code/p/a")
	wantNoReasons(t, rec)
	// A followed stat of the renamed name cannot be vouched for.
	rec = h.run(mkdir("code/p/a/d1"), rename("code/p/a/d1", "code/p/a/d2"), stat("code/p/a/d2"))
	wantReason(t, rec, "trace-incomplete:unstable-path")

	// Hard-linking a symlink: both names are aliases the trace cannot follow.
	h = newHarness(t)
	h.fs.link(abs("code/p/a/s1"), "../b")
	h.fs.link(abs("code/p/a/s2"), "../b")
	rec = h.run(
		symlink("../b", "code/p/a/s1"),
		link("code/p/a/s1", "code/p/a/s2"),
		stat("code/p/a/s1/b.txt"),
	)
	wantReason(t, rec, "trace-incomplete:unstable-path")
	h.fs.link(abs("code/p/a/s3"), "../b")
	rec = h.run(symlink("../b", "code/p/a/s3"), link("code/p/b/b.txt", "code/p/a/s3x"), stat("code/p/a/s3/b.txt"))
	if hasReason(rec, "trace-incomplete:unstable-path") {
		t.Fatalf("an unrelated link() poisoned s3: %q", rec.Reasons)
	}

	// RENAME_EXCHANGE swaps what the names are.
	h = newHarness(t)
	h.fs.link(abs("code/p/a/s1"), "../a")
	h.fs.link(abs("code/p/a/s2"), "../b")
	ex := rename("code/p/a/s1", "code/p/a/s2")
	ex.Syscall, ex.Flags = "renameat2", "RENAME_EXCHANGE"
	rec = h.run(symlink("../b", "code/p/a/s1"), symlink("../a", "code/p/a/s2"), ex, stat("code/p/a/s1/README"))
	wantReason(t, rec, "trace-incomplete:unstable-path")
	h = newHarness(t)
	h.fs.link(abs("code/p/a/s1"), "../b").dir(abs("code/p/a/s2"))
	ex2 := rename("code/p/a/s1", "code/p/a/s2")
	ex2.Syscall, ex2.Flags = "renameat2", "RENAME_EXCHANGE"
	wantNoReasons(t, h.run(symlink("../b", "code/p/a/s1"), mkdir("code/p/a/s2"), ex2))
}

func TestFirstAbsentStopsAtAFile(t *testing.T) {
	h := newHarness(t)
	// A name below a tracked FILE: the file is where the tree stops agreeing.
	rec := h.run(fail(stat("go.work/a/b"), "ENOTDIR"))
	eq(t, "entries", rec.Entries, "path:go.work")
	// And the same for coarsening.
	rec = h.run(write("chmod", "go.work/x"))
	eq(t, "writes", rec.Writes, "go.work/x")
}

func TestChdirAndFchdir(t *testing.T) {
	h := newHarness(t)
	cd := base("chdir", trace.KindDirectory, "code/p/b")
	fcd := base("fchdir", trace.KindDirectory, abs("code/scripts"))
	bad := fail(base("chdir", trace.KindDirectory, "nowhere"), "ENOENT")
	rec := h.run(cd, fcd, bad)
	// chdir is an observation; fchdir is not (its descriptor was observed
	// when it was opened).
	eq(t, "entries", rec.Entries, "path:nowhere", "pkg:code/p/b")
}

func TestProcessEventsAreNotObservations(t *testing.T) {
	h := newHarness(t)
	fork := base("clone", trace.KindProcess, "")
	fork.Flags = "CLONE_VM"
	rec := h.run(fork, base("fork", trace.KindProcess, ""))
	eq(t, "entries", rec.Entries)
	wantNoReasons(t, rec)
}

func TestOpenOfNonFilesAndTmpfile(t *testing.T) {
	h := newHarness(t)
	// A returned descriptor that is not a file (an anonymous inode).
	rec := h.run(open("code/p/a/README", "O_RDONLY", "anon_inode:[eventfd]"))
	eq(t, "entries", rec.Entries, "pkg:code/p/a")
	// O_TMPFILE makes an unnamed file inside a directory: the directory is
	// observed, the path is not created.
	tf := open("code/p/a", "O_RDWR|O_TMPFILE", abs("code/p/a"))
	rec = h.run(tf, stat("code/p/a"))
	eq(t, "entries", rec.Entries, "pkg:code/p/a")
	eq(t, "writes", rec.Writes)
}

func TestExecOfNonExistentIsAnAbsentProbe(t *testing.T) {
	h := newHarness(t)
	rec := h.run(fail(execve("code/p/b/nope", "x"), "ENOENT"), fail(stat("code/p/b/nope"), "ENOENT"))
	eq(t, "entries", rec.Entries, "pkg:code/p/b")
	eq(t, "untracked", rec.UntrackedObservations)
}
