package traced

import "testing"

// Tests for KILN04 D42: absent-before-made, metadata writes by path, and the
// /dev/null redirection.  Each pattern has its record test and a negative one.

// TestAbsentBeforeMadeLinkerPattern is the real-build shape: stat the output,
// find nothing, create it with a plain O_CREAT open, stat it again, chmod it,
// run it.
func TestAbsentBeforeMadeLinkerPattern(t *testing.T) {
	const f = "code/p/a/gcc-x-tests"
	h := quiet(t)
	h.fs.bytes(abs(f), buildELFStatic())
	rec := h.run(
		fail(stat(f), "ENOENT"),
		created(f, "O_WRONLY|O_CREAT|O_TRUNC"),
		stat(f),
		write("chmod", f),
		execve(f, "x"),
	)
	wantNoReasons(t, rec)
	wantComplete(t, "linker pattern", rec, true)
	eq(t, "writes", rec.Writes, f)
	eq(t, "absent", rec.AbsentObservations, f)

	// The same with a plain O_CREAT (no O_TRUNC): the open is still keyed by
	// the descriptor's path.
	h = quiet(t)
	h.fs.file(abs(f))
	rec = h.run(fail(stat(f), "ENOENT"), created(f, "O_WRONLY|O_CREAT"), write("chmod", f))
	wantStable(t, "plain O_CREAT", rec)
	wantComplete(t, "plain O_CREAT", rec, true)
}

func TestAbsentBeforeMadeBoundaries(t *testing.T) {
	const f = "code/p/a/gcc-x-tests"

	// A TRACKED name: an ENOENT about it was about something else.
	const tracked = "code/p/b/b.txt"
	h := quiet(t)
	h.fs.file(abs(tracked))
	rec := h.run(fail(stat(tracked), "ENOENT"), created(tracked, "O_WRONLY|O_CREAT"))
	wantUnstable(t, "tracked name", rec)

	// A removal BEFORE the creation: the name was a dangling link the lookup
	// failed through, and the action removed it and made a file in its place.
	h = quiet(t)
	h.fs.file(abs(f))
	rec = h.run(fail(stat(f), "ENOENT"), unlinkOf(f), created(f, "O_WRONLY|O_CREAT|O_TRUNC"))
	wantUnstable(t, "unlink before the creation", rec)

	// A removal AFTER the creation does not matter: the name was absent first.
	h = quiet(t)
	h.fs.file(abs(f))
	rec = h.run(fail(stat(f), "ENOENT"), created(f, "O_WRONLY|O_CREAT|O_TRUNC"), unlinkOf(f),
		created(f, "O_WRONLY|O_CREAT|O_TRUNC"))
	wantStable(t, "unlink after the creation", rec)

	// A lookup that overlaps the creation in time (a split call): the creation
	// may have taken effect before the lookup read the name.
	h = quiet(t)
	h.fs.file(abs(f))
	rec = h.runSteps(mtrace(
		timed(fail(stat(f), "ENOENT"), 1, 2, 5),
		timed(created(f, "O_WRONLY|O_CREAT|O_TRUNC"), 2, 3, 4)))
	wantUnstable(t, "lookup overlaps the creation", rec)

	// ... and one that finished before the creation began is fine.
	h = quiet(t)
	h.fs.file(abs(f))
	rec = h.runSteps(mtrace(
		timed(fail(stat(f), "ENOENT"), 1, 2, 3),
		timed(created(f, "O_WRONLY|O_CREAT|O_TRUNC"), 2, 4, 5)))
	wantStable(t, "lookup before the creation", rec)

	// The first entry must PROVE the action made the name: a rename into it
	// (or a mknod) may have replaced something the lookup could not see.
	h = quiet(t)
	h.fs.file(abs(f)).file(abs("code/p/a/src2"))
	rec = h.run(fail(stat(f), "ENOENT"), created("code/p/a/src2", "O_WRONLY|O_CREAT|O_TRUNC"), rename("code/p/a/src2", f))
	wantUnstable(t, "rename into the name", rec)

	// A failed call that can WRITE puts its lookup in the write set too: the
	// untracked name is accepted, the tracked one voids the write set (W8).
	h = quiet(t)
	h.fs.file(abs(f))
	rec = h.run(fail(write("chmod", f), "ENOENT"), created(f, "O_WRONLY|O_CREAT|O_TRUNC"))
	wantStable(t, "failed chmod, then create", rec)
	wantComplete(t, "failed chmod, then create", rec, true)
	h = quiet(t)
	h.fs.file(abs(tracked))
	rec = h.run(fail(write("chmod", tracked), "ENOENT"), created(tracked, "O_WRONLY|O_CREAT"))
	wantUnstable(t, "failed chmod of a tracked name", rec)
	wantComplete(t, "failed chmod of a tracked name", rec, false)

	// A hard link makes an alias the trace cannot follow.
	h = quiet(t)
	h.fs.file(abs(f)).file(abs("code/p/a/other"))
	rec = h.run(fail(stat(f), "ENOENT"), created(f, "O_WRONLY|O_CREAT|O_TRUNC"), link(f, "code/p/a/other"))
	wantUnstable(t, "hard link", rec)

	// An ENOENT from exec says nothing about the final name (it may be the
	// interpreter that is missing), so it keeps today's rule.
	h = quiet(t)
	h.fs.file(abs(f))
	rec = h.run(fail(execve(f, "x"), "ENOENT"), created(f, "O_WRONLY|O_CREAT|O_TRUNC"))
	wantUnstable(t, "failed exec", rec)

	// A failed rename names two paths: not attributable to either.
	h = quiet(t)
	h.fs.file(abs(f)).file(abs("code/p/a/src2"))
	rec = h.run(fail(rename("code/p/a/src2", f+"/"), "ENOENT"), created(f, "O_WRONLY|O_CREAT|O_TRUNC"))
	wantUnstable(t, "failed rename", rec)
}

func TestAbsentBeforeMadeDirectory(t *testing.T) {
	const d = "code/p/a/_build"
	h := quiet(t)
	h.fs.dir(abs(d)).dir(abs(d + "/sub"))
	rec := h.run(fail(stat(d), "ENOENT"), mkdir(d), mkdir(d+"/sub"))
	wantNoReasons(t, rec)
	wantComplete(t, "mkdir -p", rec, true)

	// Replaced by a symlink behind the lookup's back: the name is a link now.
	h = quiet(t)
	h.fs.link(abs(d), abs("code/p/b"))
	rec = h.run(fail(stat(d), "ENOENT"), mkdir(d), rename(d, "code/p/a/old"), symlink(abs("code/p/b"), d))
	wantUnstable(t, "directory replaced by a link", rec)
}

func TestMetadataWriteByPathUsesOpenOnly(t *testing.T) {
	const f = "code/p/a/out.o"
	for _, sys := range []string{"chmod", "fchmodat", "chown", "fchownat", "utimensat", "setxattr"} {
		h := quiet(t)
		h.fs.file(abs(f))
		rec := h.run(created(f, "O_WRONLY|O_CREAT|O_TRUNC"), write(sys, f))
		wantStable(t, sys+" of an own file", rec)
		wantComplete(t, sys+" of an own file", rec, true)
	}

	// Not relaxed: a file whose open log is disqualified.
	h := quiet(t)
	h.fs.file(abs(f))
	rec := h.run(created(f, "O_WRONLY|O_CREAT|O_TRUNC"), rename(f, "code/p/a/old.o"),
		created(f, "O_WRONLY|O_CREAT|O_TRUNC"), write("chmod", f))
	wantUnstable(t, "chmod after a rename away", rec)

	// A tracked file the action did not create is still a tracked write.
	h = quiet(t)
	rec = h.run(write("chmod", "code/p/a/README"))
	wantReason(t, rec, "writes-tracked:code/p/a/README")

	// An untracked leftover the action did not create is still an observation.
	h = quiet(t)
	h.fs.file(abs(f))
	rec = h.run(write("chmod", f))
	eq(t, "untracked", rec.UntrackedObservations, f)

	// A directory is not a file the open-only rule speaks about.
	h = quiet(t)
	h.fs.dir(abs(f))
	rec = h.run(created(f, "O_WRONLY|O_CREAT|O_TRUNC"), write("chmod", f))
	wantUnstable(t, "chmod of a directory on the disk", rec)
}

func TestDevNullRedirectionLogsNothing(t *testing.T) {
	h := newHarness(t)
	h.fs.file("/dev/null")
	const o = "O_WRONLY|O_CREAT|O_TRUNC"
	rec := h.run(open("/dev/null", o, "/dev/null"), stat("/dev/null"),
		open("/dev/null", o, "/dev/null"), open("/dev/null", "O_RDONLY", "/dev/null"))
	wantStable(t, "/dev/null", rec)

	c := newClassifier(&prepared{ephemeral: []string{"/dev/null", "/usr/special/**"}})
	for p, want := range map[string]bool{
		"/dev/null": true, "/dev/sda": false, "/usr/special/x": false, "/dev/null/x": false,
	} {
		if got := c.ephemeralDevice(p); got != want {
			t.Errorf("ephemeralDevice(%q) = %v, want %v", p, got, want)
		}
	}
}
