package traced

import "testing"

// Scenarios the differential fuzzer found: the post-run disk shows the END
// state, so a name the action made and later removed or replaced must not be
// trusted to say what an earlier call crossed.
func TestFuzzFindingsRemovedOrReplacedLinks(t *testing.T) {
	h := newHarness(t)
	// A link followed by a failing stat, then removed: the disk no longer has
	// it, so "g is absent" would hide that the call went on to /w/repo/missing.
	rec := h.run(
		symlink(abs("missing")+"/", "g"),
		fail(stat("g/"), "ENOENT"),
		write("unlink", "g"),
	)
	wantReason(t, rec, "trace-incomplete:unstable-path")

	// ".." after the removed link.
	h = newHarness(t)
	rec = h.run(
		symlink(abs("lnkx/g/"), "g"),
		fail(write("mkdir", "g/../"), "ENOENT"),
		write("unlink", "g"),
	)
	wantReason(t, rec, "trace-incomplete:unstable-path")

	// The same name recreated as a directory afterwards.
	h = newHarness(t)
	h.fs.dir(abs("g"))
	rec = h.run(
		symlink("/tmp/kiln-1/zz/", "g"),
		fail(stat("g/"), "ENOENT"),
		write("unlink", "g"),
		mkdir("g"),
	)
	wantReason(t, rec, "trace-incomplete:unstable-path")

	// A name that was never touched and is absent stays an ordinary absence.
	h = newHarness(t)
	rec = h.run(fail(stat("nothing/x"), "ENOENT"))
	wantNoReasons(t, rec)

	// A followed final name the action changed must satisfy the creation rule;
	// one it never touched is just what the disk says.
	h = newHarness(t)
	h.fs.file(abs("code/p/a/built"))
	wantNoReasons(t, h.run(stat("code/p/a/built")))
	// (Relaxed by the open-only rule of section 5.5: the stat follows a name
	// whose whole change log is an O_CREAT open that completed first.  The
	// negative cases are in relax_test.go.)
	rec = h.run(open("code/p/a/built", "O_WRONLY|O_CREAT|O_TRUNC", abs("code/p/a/built")), stat("code/p/a/built"))
	wantNoReasons(t, rec)
	// A successful open's -y path settles its final component.
	rec = h.run(open("code/p/a/built", "O_WRONLY|O_CREAT|O_TRUNC", abs("code/p/a/built")),
		open("code/p/a/built", "O_RDONLY", abs("code/p/a/built")))
	wantNoReasons(t, rec)

	// A file in the middle of a path, touched by the action.
	h = newHarness(t)
	h.fs.file(abs("code/p/a/f"))
	rec = h.run(fail(stat("code/p/a/f/x"), "ENOTDIR"))
	wantNoReasons(t, rec)
	rec = h.run(write("chmod", "code/p/a/f"), fail(stat("code/p/a/f/x"), "ENOTDIR"))
	_ = rec
	h.fs.file(abs("code/p/a/f2"))
	rec = h.run(open("code/p/a/f2", "O_WRONLY|O_CREAT|O_EXCL", abs("code/p/a/f2")), fail(stat("code/p/a/f2/x"), "ENOTDIR"))
	wantReason(t, rec, "trace-incomplete:unstable-path")

	// Trailing slashes on a successful mkdir or rmdir are not "follow".
	h = newHarness(t)
	h.fs.dir(abs("code/p/a/d"))
	rec = h.run(mkdir("code/p/a/d/"), write("rmdir", "code/p/a/d/"))
	wantNoReasons(t, rec)
}
