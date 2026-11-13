package traced

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// Tests for the three hit-rate relaxations of KILN04 section 5.5:
//
//	(a) a successful open settles its final name,
//	(b) open-only change logs,
//	(c) $HOME itself.
//
// Each has its record tests (the pattern now records) and its boundary tests
// (everything one step outside the precondition stays unstable).

const rUnst = "trace-incomplete:unstable-path"

// created opens a name with the flags a compiler uses; the descriptor's -y
// path is the name itself.
func created(rel, flags string) trace.Event { return open(rel, flags, abs(rel)) }

func unlinkOf(rel string) trace.Event { return write("unlink", rel) }

func wantUnstable(t *testing.T, what string, rec *RawRecord) {
	t.Helper()
	if !hasReason(rec, rUnst) {
		t.Errorf("%s: reasons %q lack %q", what, rec.Reasons, rUnst)
	}
}

func wantStable(t *testing.T, what string, rec *RawRecord) {
	t.Helper()
	if hasReason(rec, rUnst) {
		t.Errorf("%s: reasons %q", what, rec.Reasons)
	}
}

// quiet is a harness whose disk is stated by the test, not derived.
func quiet(t *testing.T) *harness {
	h := newHarness(t)
	h.noSettle = true
	return h
}

func TestRelaxedPatternsRecord(t *testing.T) {
	const o, x, tmp = "code/p/a/out", "code/p/a/out/x", "code/p/a/tmp.o"

	// A compiler output that is stat'ed later, and kept.
	h := quiet(t)
	h.fs.dir(abs(o)).file(abs(x))
	rec := h.run(mkdir(o), created(x, "O_WRONLY|O_CREAT|O_TRUNC"), created(x, "O_RDONLY"), stat(x))
	wantNoReasons(t, rec)
	eq(t, "entries", rec.Entries, "pkg:code/p/a")
	eq(t, "writes", rec.Writes, o)
	eq(t, "absent", rec.AbsentObservations, o)
	eq(t, "untracked", rec.UntrackedObservations)

	// A temporary that is created, read and unlinked.
	h = quiet(t)
	rec = h.run(created(tmp, "O_WRONLY|O_CREAT|O_TRUNC"), created(tmp, "O_RDONLY"), unlinkOf(tmp))
	wantNoReasons(t, rec)
	eq(t, "entries", rec.Entries, "pkg:code/p/a")
	eq(t, "writes", rec.Writes, tmp)
	eq(t, "absent", rec.AbsentObservations, tmp)

	// An exclusively created temporary that is stat'ed and unlinked.
	h = quiet(t)
	rec = h.run(created(tmp, "O_WRONLY|O_CREAT|O_EXCL"), stat(tmp), unlinkOf(tmp))
	wantNoReasons(t, rec)
	eq(t, "entries", rec.Entries, "pkg:code/p/a")
	eq(t, "writes", rec.Writes, tmp)

	// The same, with a plain O_CREAT (not "clean"): the name is still a file.
	h = quiet(t)
	rec = h.run(created(tmp, "O_WRONLY|O_CREAT"), stat(tmp), unlinkOf(tmp))
	wantStable(t, "plain O_CREAT", rec)

	// With the checkout under $HOME (hosted runners), $HOME itself.
	const root = "/home/runner/work/repo"
	h = newHarness(t)
	h.cfg.Root, h.fs.root = root, root
	h.fs.dir("/home/runner").dir("/home/runner/work")
	rec = h.run(stat("/home/runner"), getdents("/home/runner"))
	wantNoReasons(t, rec)
	eq(t, "entries", rec.Entries)
}

func TestRelaxedExecOfAnOutput(t *testing.T) {
	const prog = "code/p/a/prog"
	h := quiet(t)
	h.fs.bytes(abs(prog), buildELFStatic())
	rec := h.run(created(prog, "O_WRONLY|O_CREAT|O_TRUNC"), execve(prog, "prog"))
	wantNoReasons(t, rec)
	// ... but not when the exec overlapped the first open.
	h = quiet(t)
	h.fs.bytes(abs(prog), buildELFStatic())
	rec = h.runSteps(mtrace(
		timed(created(prog, "O_WRONLY|O_CREAT|O_TRUNC"), 2, 3, 6),
		timed(execve(prog, "prog"), 1, 4, 7)))
	wantUnstable(t, "exec started before the open completed", rec)
}

// (a) ------------------------------------------------------------------------

func TestOpenSettlesOnlyTheFinalComponent(t *testing.T) {
	// A link swapped in for an INTERMEDIATE component: src is tracked, the
	// action opens a new file in it, then renames it away and puts a link in
	// its place.  The open's -y path is src/n, but the disk walks through the
	// link, and the link is a traversed component with a change log.
	h := quiet(t)
	h.fs.link(abs("code/p/a/src"), abs("code/p/b")).dir(abs("code/p/a/old")).file(abs("code/p/a/old/n"))
	rec := h.run(
		created("code/p/a/src/n", "O_WRONLY|O_CREAT|O_TRUNC"),
		rename("code/p/a/src", "code/p/a/old"),
		symlink(abs("code/p/b"), "code/p/a/src"),
	)
	wantUnstable(t, "link swapped in for an intermediate component", rec)

	// The same with the final component only: no intermediate change, so it records.
	h = quiet(t)
	h.fs.file(abs("code/p/a/src/n"))
	wantStable(t, "final only", h.run(created("code/p/a/src/n", "O_WRONLY|O_CREAT|O_TRUNC")))

	// A -y path that differs from the walked path is not "settled".  In the
	// repository that is the existing unstable-path rule.
	h = quiet(t)
	h.fs.file(abs("code/p/a/o")).file(abs("code/p/b/o"))
	rec = h.run(open("code/p/a/o", "O_WRONLY|O_CREAT|O_TRUNC", abs("code/p/b/o")))
	wantUnstable(t, "-y differs inside the repository", rec)

	// Outside the repository the skip itself is what is being refused.  A name
	// the action made, then opened again and removed: the disk shows it absent,
	// and an absent stop on a name made before the lookup is unstable, unless
	// the open's -y path settles the name.
	const n = "/var/run/n"
	run := func(ret string) *RawRecord {
		h := quiet(t)
		h.fs.dir("/var/run")
		return h.run(open(n, "O_WRONLY|O_CREAT|O_EXCL", n), open(n, "O_RDONLY", ret), unlinkOf(n))
	}
	wantStable(t, "-y equals the name", run(n))
	wantUnstable(t, "-y differs outside the repository", run("/var/run/m"))

	// A failed open settles nothing: its result is not the kernel's path.
	h = quiet(t)
	h.fs.dir("/var/run")
	rec = h.run(
		open(n, "O_WRONLY|O_CREAT|O_EXCL", n),
		fail(open(n, "O_RDONLY", ""), "EACCES"),
		unlinkOf(n),
	)
	wantUnstable(t, "failed open", rec)
}

// (b) ------------------------------------------------------------------------

func TestOpenOnlyBoundaries(t *testing.T) {
	const f, g = "code/p/a/out.o", "code/p/a/other.o"
	open1 := created(f, "O_WRONLY|O_CREAT|O_TRUNC")
	for name, tc := range map[string]struct {
		events []trace.Event
		diskF  bool // the disk shows f
	}{
		"a rename onto the name": {[]trace.Event{
			open1, created(g, "O_WRONLY|O_CREAT|O_EXCL"), rename(g, f), stat(f)}, true},
		"a rename away": {[]trace.Event{
			open1, stat(f), rename(f, g)}, false},
		"a link source": {[]trace.Event{
			open1, link(f, g), stat(f)}, true},
		"a link destination": {[]trace.Event{
			created(g, "O_WRONLY|O_CREAT|O_EXCL"), link(g, f), open1, stat(f)}, true},
		"a symlink onto it": {[]trace.Event{
			open1, unlinkOf(f), symlink(abs("code/p/b"), f), stat(f)}, true},
		"a mkdir after": {[]trace.Event{
			open1, unlinkOf(f), mkdir(f), stat(f)}, true},
		"a second unlink": {[]trace.Event{
			open1, stat(f), unlinkOf(f), created(f, "O_WRONLY|O_CREAT|O_TRUNC"), unlinkOf(f)}, false},
		"an exchange": {[]trace.Event{
			open1, created(g, "O_WRONLY|O_CREAT|O_EXCL"), exchange(f, g), stat(f)}, true},
		"the disk shows the file despite an unlink": {[]trace.Event{
			open1, stat(f), unlinkOf(f)}, true},
		"an O_CREAT without a path from the kernel": {[]trace.Event{
			open("code/p/a/out.o", "O_WRONLY|O_CREAT", "pipe:[1]"), stat(f)}, true},
	} {
		h := quiet(t)
		if tc.diskF {
			h.fs.file(abs(f)).file(abs(g))
		}
		rec := h.run(tc.events...)
		wantUnstable(t, name, rec)
	}
}

func exchange(a, b string) trace.Event {
	e := rename(a, b)
	e.Syscall, e.Flags = "renameat2", "RENAME_EXCHANGE"
	return e
}

func TestOpenOnlyOverlaps(t *testing.T) {
	const f = "code/p/a/out.o"
	// The first open completes AFTER the stat started: the stat may have run
	// first and followed whatever was there.
	h := quiet(t)
	h.fs.file(abs(f))
	rec := h.runSteps(mtrace(
		timed(stat(f), 1, 3, 6),
		timed(created(f, "O_WRONLY|O_CREAT|O_TRUNC"), 2, 4, 5)))
	wantUnstable(t, "open completes during the stat", rec)
	// Completing before the stat starts is fine.
	h = quiet(t)
	h.fs.file(abs(f))
	rec = h.runSteps(mtrace(
		timed(created(f, "O_WRONLY|O_CREAT|O_TRUNC"), 2, 2, 3),
		timed(stat(f), 1, 4, 5)))
	wantStable(t, "open before the stat", rec)

	// An unlink overlapping the stat.
	h = quiet(t)
	rec = h.runSteps(mtrace(
		timed(created(f, "O_WRONLY|O_CREAT|O_TRUNC"), 1, 2, 3),
		timed(stat(f), 1, 4, 8),
		timed(unlinkOf(f), 2, 6, 9)))
	wantUnstable(t, "unlink overlaps the stat", rec)
	// An unlink that started before the stat did.
	h = quiet(t)
	rec = h.runSteps(mtrace(
		timed(created(f, "O_WRONLY|O_CREAT|O_TRUNC"), 1, 2, 3),
		timed(stat(f), 1, 5, 7),
		timed(unlinkOf(f), 2, 4, 9)))
	wantUnstable(t, "unlink starts before the stat", rec)
	// An unlink after the stat completed.
	h = quiet(t)
	rec = h.runSteps(mtrace(
		timed(created(f, "O_WRONLY|O_CREAT|O_TRUNC"), 1, 2, 3),
		timed(stat(f), 1, 4, 5),
		timed(unlinkOf(f), 2, 6, 7)))
	wantStable(t, "unlink after the stat", rec)
	// The LAST lookup is what counts: a second stat overlapping the unlink.
	h = quiet(t)
	rec = h.runSteps(mtrace(
		timed(created(f, "O_WRONLY|O_CREAT|O_TRUNC"), 1, 2, 3),
		timed(stat(f), 1, 4, 5),
		timed(unlinkOf(f), 2, 6, 8),
		timed(stat(f), 1, 7, 9)))
	wantUnstable(t, "a later stat overlaps the unlink", rec)
}

func TestOpenOnlyAncestorsKeepTheStrictRule(t *testing.T) {
	const o, x = "code/p/a/out", "code/p/a/out/x"
	// The parent directory is renamed away after the file was made: the stat of
	// out/x sees another directory than the open did.
	h := quiet(t)
	h.fs.dir(abs(o)).file(abs(x)).dir(abs("code/p/a/gone"))
	rec := h.run(mkdir(o), created(x, "O_WRONLY|O_CREAT|O_TRUNC"), rename(o, "code/p/a/gone"), mkdir(o), stat(x))
	wantUnstable(t, "parent replaced", rec)
	// A tracked parent that was renamed.
	h = quiet(t)
	h.fs.file(abs("code/p/b/n"))
	rec = h.run(created("code/p/b/n", "O_WRONLY|O_CREAT"), stat("code/p/b/n"), rename("code/p/b", "code/p/c"))
	wantUnstable(t, "tracked parent renamed", rec)
}

func TestOpenOnlyDoesNotRelaxOtherCalls(t *testing.T) {
	const f = "code/p/a/out.o"
	// A failed open, an lstat-free chmod and a rename source all still need the
	// creation rule: the relaxation is for a successful stat or exec only.
	h := quiet(t)
	h.fs.file(abs(f))
	rec := h.run(created(f, "O_WRONLY|O_CREAT|O_TRUNC"), write("chmod", f))
	wantUnstable(t, "chmod", rec)
	h = quiet(t)
	h.fs.file(abs(f))
	rec = h.run(created(f, "O_WRONLY|O_CREAT|O_TRUNC"), fail(stat(f), "EACCES"))
	wantUnstable(t, "failed stat", rec)
	// A directory is not a file the rule speaks about.
	h = quiet(t)
	h.fs.dir(abs(f))
	rec = h.run(created(f, "O_WRONLY|O_CREAT|O_TRUNC"), stat(f))
	wantUnstable(t, "directory on the disk", rec)
}

// (c) ------------------------------------------------------------------------

func TestHomeItselfOnly(t *testing.T) {
	const root = "/home/runner/work/repo"
	mk := func() *harness {
		h := newHarness(t)
		h.cfg.Root, h.fs.root = root, root
		h.fs.dir("/home/runner").dir("/home/runner/work")
		return h
	}
	h := mk()
	wantNoReasons(t, h.run(stat("/home/runner")))
	// Anything under it, and the checkout's ancestors, are host reads.
	wantReason(t, mk().run(stat("/home/runner/work")), "host-read:$HOME/work")
	wantReason(t, mk().run(fail(stat("/home/runner/.gitconfig"), "ENOENT")), "host-read:$HOME/.gitconfig")
	wantReason(t, mk().run(getdents("/home/runner/work")), "host-read:$HOME/work")
	wantReason(t, mk().run(openAbs("/home/runner/x")), "host-read:$HOME/x")
	// A sibling that merely shares the prefix.
	wantReason(t, mk().run(stat("/home/runner2")), "host-read:/home/runner2")
	// And the checkout is not "$HOME itself".
	h = mk()
	wantNoReasons(t, h.run(stat(root+"/go.work")))
}

// A successful followed lookup of a name that the post-run disk shows absent
// and the action never made: something removed it afterwards (here an untracked
// leftover link, which the stat followed into code/p/b), so the call may have
// gone somewhere the disk can no longer show.  Found by the differential
// fuzzer's "leftover replaced" burst.
func TestSuccessfulLookupOfARemovedLeftover(t *testing.T) {
	const l = "code/p/a/leftlnk"
	h := quiet(t)
	wantUnstable(t, "stat then unlink", h.run(stat(l), unlinkOf(l)))
	// A successful stat of an absent name nothing ever made or removed.
	h = quiet(t)
	wantUnstable(t, "stat of a name that is not there", h.run(stat(l)))
	// A name the action made is judged by its change log instead.
	h = quiet(t)
	wantStable(t, "own", h.run(created(l, "O_WRONLY|O_CREAT|O_EXCL"), stat(l), unlinkOf(l)))
	// Not followed, or failing: nothing to contradict.
	h = quiet(t)
	wantStable(t, "lstat", h.run(lstat(l), unlinkOf(l)))
	h = quiet(t)
	wantStable(t, "failed stat", h.run(fail(stat(l), "ENOENT")))
	// A failure that says something was there (EISDIR: the link led to a
	// directory) is as good as a success; found by the fuzzer.
	h = quiet(t)
	wantUnstable(t, "EISDIR then removed", h.run(
		fail(open(l, "O_WRONLY|O_CREAT|O_TRUNC", ""), "EISDIR"), rename(l, "code/p/a/elsewhere")))
	// An ENOENT before the removal of a name the action did not make may have
	// been the missing target of a link that is gone now; after the removal it
	// is just an absence.
	h = quiet(t)
	wantUnstable(t, "ENOENT before the removal", h.run(fail(stat(l), "ENOENT"), unlinkOf(l)))
	h = quiet(t)
	wantStable(t, "ENOENT after the removal", h.run(unlinkOf(l), fail(stat(l), "ENOENT")))
	// A name made before the lookup is judged by its change log.
	h = quiet(t)
	wantStable(t, "made, ENOENT, removed", h.run(created(l, "O_WRONLY|O_CREAT|O_EXCL"), unlinkOf(l), fail(stat(l), "ENOENT")))
}
