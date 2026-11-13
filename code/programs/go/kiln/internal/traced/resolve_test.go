package traced

import (
	"errors"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// fixture links on the post-run disk, all inside the checkout.
func (h *harness) tracked(paths ...string) {
	for _, p := range paths {
		h.fs.dir(p)
	}
}

func TestTrackedLinkIsObservedAndFollowed(t *testing.T) {
	h := newHarness(t)
	// code/p/a/lnk is a tracked link to src/x.go.  Opening it reads x.go; both
	// the link and the file are observations.
	h.fs.link(abs("code/p/a/lnk"), "src/x.go")
	rec := h.run(open("code/p/a/lnk", "O_RDONLY", abs("code/p/a/src/x.go")))
	eq(t, "entries", rec.Entries, "pkg:code/p/a")
	eq(t, "traversed", rec.Traversed, "code", "code/p", "code/p/a", "code/p/a/lnk", "code/p/a/src")
	wantNoReasons(t, rec)
}

func TestLinkIntoAnotherPackageAddsThatPackage(t *testing.T) {
	h := newHarness(t)
	// A tracked link in a that points into b: reading through it depends on b.
	h.tree.link("code/p/a/tob", "../b/b.txt")
	h.fs.link(abs("code/p/a/tob"), "../b/b.txt")
	rec := h.run(open("code/p/a/tob", "O_RDONLY", abs("code/p/b/b.txt")))
	eq(t, "entries", rec.Entries, "pkg:code/p/a", "pkg:code/p/b")
	wantNoReasons(t, rec)
}

func TestSymlinkCreatedByTheActionIntoAnotherPackage(t *testing.T) {
	h := newHarness(t)
	// The post-run disk shows the link, so it is a component the disk can vouch
	// for: created once, before the read, untouched since.
	h.fs.link(abs("code/p/a/mylink"), "../b/b.txt")
	rec := h.run(
		symlink("../b/b.txt", "code/p/a/mylink"),
		open("code/p/a/mylink", "O_RDONLY", abs("code/p/b/b.txt")),
	)
	// The link is the action's own (its creation is an absence observation);
	// what it pointed at is not.
	eq(t, "entries", rec.Entries, "pkg:code/p/a", "pkg:code/p/b")
	wantNoReasons(t, rec)

	// Removed afterwards, the post-run disk no longer shows what the read
	// crossed: unstable, however well the trace knows it.
	h = newHarness(t)
	rec = h.run(
		symlink("../b/b.txt", "code/p/a/mylink"),
		open("code/p/a/mylink", "O_RDONLY", abs("code/p/b/b.txt")),
		write("unlink", "code/p/a/mylink"),
	)
	wantReason(t, rec, "trace-incomplete:unstable-path")
}

func TestDotDotAppliesToTheResolvedDirectory(t *testing.T) {
	h := newHarness(t)
	// L -> code/p/b/ (a link to a directory).  "L/../x" is x BESIDE b, not
	// beside L.  Lexically it would be code/p/a/x.
	h.tree.link("code/p/a/L", "../b")
	h.fs.link(abs("code/p/a/L"), "../b")
	h.fs.dir(abs("code/p/b"))
	rec := h.run(fail(stat("code/p/a/L/../absent.txt"), "ENOENT"))
	// Resolved: code/p/b/../absent.txt = code/p/absent.txt, outside packages,
	// first absent component code/p/absent.txt.
	eq(t, "entries", rec.Entries, "path:code/p/absent.txt", "pkg:code/p/a")
	wantNoReasons(t, rec)
}

func TestAbsoluteLinkTargetsAndLinkChains(t *testing.T) {
	h := newHarness(t)
	h.fs.link(abs("l1"), "l2")
	h.fs.link(abs("l2"), abs("code/p/b"))
	rec := h.run(fail(stat("l1/nope"), "ENOENT"))
	// l1 and l2 are untracked links the action did not make: observed.
	eq(t, "entries", rec.Entries, "path:l1", "path:l2", "pkg:code/p/b")
	eq(t, "untracked", rec.UntrackedObservations, "l1", "l2")
	// Without following the last component, the link itself is the object.
	rec = h.run(lstat("l1"))
	eq(t, "entries", rec.Entries, "path:l1")
	eq(t, "traversed", rec.Traversed)
}

func TestTrailingSlashFollowsTheLink(t *testing.T) {
	h := newHarness(t)
	h.fs.link(abs("l1"), abs("code/p/b"))
	rec := h.run(lstat("l1/"))
	eq(t, "entries", rec.Entries, "path:l1", "pkg:code/p/b")
}

func TestSymlinkLoopIsRefused(t *testing.T) {
	h := newHarness(t)
	h.fs.link(abs("a"), "b")
	h.fs.link(abs("b"), "a")
	rec := h.run(fail(stat("a"), "ELOOP"))
	wantReason(t, rec, "trace-incomplete:symlink-loop")
	// Exactly the limit passes; one more does not.
	h = newHarness(t)
	for i := 0; i < 41; i++ {
		h.fs.link(abs("h"+string(rune('A'+i))), "h"+string(rune('A'+i+1)))
	}
	rec = h.run(stat("hA"))
	wantReason(t, rec, "trace-incomplete:symlink-loop") // hA..hAN are 40 links plus the 41st name
	h = newHarness(t)
	for i := 0; i < 39; i++ {
		h.fs.link(abs("h"+string(rune('A'+i))), "h"+string(rune('A'+i+1)))
	}
	if hasReason(h.run(stat("hA")), "trace-incomplete:symlink-loop") {
		t.Fatal("39 hops is within the limit")
	}
}

func TestUnresolvablePaths(t *testing.T) {
	h := newHarness(t)
	h.fs.errs[abs("code")] = errors.New("permission denied")
	wantReason(t, h.run(stat("code/p/a/x")), "trace-incomplete:unresolvable-path")
	h = newHarness(t)
	h.fs.info[abs("badlink")] = FileInfo{Kind: KindLink}
	wantReason(t, h.run(stat("badlink")), "trace-incomplete:unresolvable-path")
}

func TestKernelAndPostRunDiskMustAgree(t *testing.T) {
	h := newHarness(t)
	// The kernel opened code/p/b/b.txt, but the disk now says the link points
	// at a.  Something changed under the action: unstable.
	h.fs.link(abs("code/p/a/L"), "README")
	h.tree.link("code/p/a/L", "README")
	rec := h.run(open("code/p/a/L", "O_RDONLY", abs("code/p/b/b.txt")))
	wantReason(t, rec, "trace-incomplete:unstable-path")
	// The kernel's path wins for the entry.
	eq(t, "entries", rec.Entries, "pkg:code/p/a", "pkg:code/p/b")
	// A disagreement entirely outside the repository is not a stability issue:
	// the -y path is used.
	h = newHarness(t)
	h.image("/etc/real")
	rec = h.run(open("/etc/alias", "O_RDONLY", "/etc/real"))
	wantNoReasons(t, rec)
	rec = h.run(open("/dev/null", "O_RDONLY", "/dev/null"))
	wantNoReasons(t, rec)
}

func TestUnstableLinks(t *testing.T) {
	h := newHarness(t)
	h.fs.link(abs("code/p/a/L"), "../b")
	h.tree.link("code/p/a/L", "../b")
	// The action reads through L, then removes and re-creates it: the post-run
	// disk holds the NEW link, which is not what the first read crossed.
	rec := h.run(
		stat("code/p/a/L/b.txt"),
		write("unlink", "code/p/a/L"),
		symlink("../a", "code/p/a/L"),
	)
	wantReason(t, rec, "trace-incomplete:unstable-path")
	wantReason(t, rec, "writes-tracked:code/p/a/L")

	// A link the action made and never touched again is stable.
	h = newHarness(t)
	h.fs.link(abs("code/p/a/M"), "../b")
	rec = h.run(symlink("../b", "code/p/a/M"), stat("code/p/a/M/b.txt"))
	wantNoReasons(t, rec)
	// Replaced after the read: unstable (a second change).
	h = newHarness(t)
	h.fs.link(abs("code/p/a/M"), "../a")
	rec = h.run(symlink("../b", "code/p/a/M"), stat("code/p/a/M/b.txt"),
		write("unlink", "code/p/a/M"), symlink("../a", "code/p/a/M"))
	wantReason(t, rec, "trace-incomplete:unstable-path")
	// Re-created BEFORE the read is not "created exactly once": unstable too.
	rec = h.run(symlink("../b", "code/p/a/M"), write("unlink", "code/p/a/M"),
		symlink("../a", "code/p/a/M"), stat("code/p/a/M/README"))
	wantReason(t, rec, "trace-incomplete:unstable-path")
	// An untracked leftover of an earlier action is no stable component either.
	h = newHarness(t)
	h.fs.dir(abs("code/p/a/node_modules"))
	rec = h.run(stat("code/p/a/node_modules/x/index.js"))
	wantReason(t, rec, "trace-incomplete:unstable-path")
}

func TestComponentCreatedAfterTheCallIsUnstable(t *testing.T) {
	h := newHarness(t)
	// The probe ran when out/ did not exist; the action made it afterwards.
	// The post-run disk has out/, but the call never reached it, and a
	// creation that completes after the lookup started does not vouch for it.
	h.fs.dir(abs("code/p/a/out"))
	rec := h.run(
		fail(stat("code/p/a/out/f"), "ENOENT"),
		mkdir("code/p/a/out"),
	)
	wantReason(t, rec, "trace-incomplete:unstable-path")
	// Completion against start: a lookup that STARTED before the creation
	// completed (a split call) is unstable even though its own line is later.
	h = newHarness(t)
	h.fs.dir(abs("code/p/a/out"))
	tr := step(fail(stat("code/p/a/out/f"), "ENOENT"), mkdir("code/p/a/out"))
	tr.Events[1].StartLine, tr.Events[1].Line = 2, 5 // the stat began early, completed late
	tr.Events[2].StartLine, tr.Events[2].Line = 3, 3
	rec = h.runSteps(tr)
	wantReason(t, rec, "trace-incomplete:unstable-path")
	// A creation that completed before the lookup started is stable.
	h = newHarness(t)
	h.fs.dir(abs("code/p/a/out"))
	rec = h.run(mkdir("code/p/a/out"), fail(stat("code/p/a/out/f"), "ENOENT"))
	wantNoReasons(t, rec)
	// A tracked directory the action removed and re-made after reading through
	// it is unstable.
	h = newHarness(t)
	h.fs.dir(abs("code/p/b"))
	rec = h.run(stat("code/p/b/b.txt"), write("rmdir", "code/p/b"), mkdir("code/p/b"))
	wantReason(t, rec, "trace-incomplete:unstable-path")
	// Any ancestor's change counts: renaming a tracked directory away.
	h = newHarness(t)
	rec = h.run(stat("code/p/b/b.txt"), rename("code/p/b", "code/p/c"))
	wantReason(t, rec, "trace-incomplete:unstable-path")
}

func TestTraversedListsRepositoryComponents(t *testing.T) {
	h := newHarness(t)
	h.fs.dir(abs("code")).dir(abs("code/p")).dir(abs("code/p/a")).dir(testRoot).dir("/w")
	h.fs.link(abs("code/p/a/lnk"), "src/x.go")
	rec := h.run(open("code/p/a/lnk", "O_RDONLY", abs("code/p/a/src/x.go")))
	eq(t, "traversed", rec.Traversed, "code", "code/p", "code/p/a", "code/p/a/lnk", "code/p/a/src")
}

func TestProcPaths(t *testing.T) {
	h := newHarness(t)
	// The ephemeral allowlist.
	rec := h.run(openAbs("/proc/cpuinfo"), openAbs("/proc/self/exe"), stat("/dev/fd/3"), fail(stat("/dev/fd/9"), "EBADF"))
	wantNoReasons(t, rec)
	// They reveal the tracer.
	rec = h.run(openAbs("/proc/self/status"), openAbs("/proc/self/stat"))
	eq(t, "reasons", rec.Reasons, "trace-visible:/proc/self/stat", "trace-visible:/proc/self/status")
	// Everything else is incomplete: numeric pids, root, thread-self, fd listings.
	for _, p := range []string{"/proc/1234/status", "/proc/self/root/etc/passwd", "/proc/thread-self/stat",
		"/proc/self/fd/3", "/proc/self/environ", "/proc", "/dev/fd", "/proc/self/cwd/x"} {
		rec = h.run(stat(p))
		wantReason(t, rec, "trace-incomplete:proc-path")
	}
	// /proc is never walked through the post-run disk's own /proc/self.
	h.fs.link("/proc/self", "999")
	wantReason(t, h.run(stat("/proc/self/root/x")), "trace-incomplete:proc-path")
	// ".." out of /proc resumes the walk of the real disk.
	h.fs.link("/etc/l", "/etc/target")
	h.image("/etc/l", "/etc/target")
	rec = h.run(stat("/proc/../etc/l"))
	wantNoReasons(t, rec)
	// A listing of a numeric pid directory.
	wantReason(t, h.run(getdents("/proc/42")), "trace-incomplete:proc-path")
}

func TestOpenOfFDPathsIsClassifiedByTheDashYPath(t *testing.T) {
	h := newHarness(t)
	// /dev/fd/N open: the descriptor is a repository file.
	rec := h.run(open("/dev/fd/5", "O_RDONLY", abs("code/p/b/b.txt")))
	eq(t, "entries", rec.Entries, "pkg:code/p/b")
	wantNoReasons(t, rec)
	// A pipe or socket (process substitution) is nothing.
	rec = h.run(open("/proc/self/fd/6", "O_RDONLY", "pipe:[77]"), open("/dev/fd/7", "O_RDONLY", "socket:[8]"))
	eq(t, "entries", rec.Entries)
	wantNoReasons(t, rec)
	// It can write the file it names.
	rec = h.run(open("/dev/fd/5", "O_WRONLY|O_TRUNC", abs("code/p/b/b.txt")))
	wantReason(t, rec, "writes-tracked:code/p/b/b.txt")
	// An out-of-repository file it names gets the host rules.
	rec = h.run(open("/proc/self/fd/5", "O_RDONLY", "/var/secret"))
	wantReason(t, rec, "host-read:/var/secret")
	// A failed open of /proc/self/fd/N has no -y path to judge by.
	wantReason(t, h.run(fail(open("/proc/self/fd/5", "O_RDONLY", ""), "ENOENT")), "trace-incomplete:proc-path")
	// A non-numeric name under fd/ is not a descriptor link.
	wantReason(t, h.run(open("/proc/self/fd/x", "O_RDONLY", "/etc/x")), "trace-incomplete:proc-path")
}

func TestWorkingDirectoryExpressionIsWalked(t *testing.T) {
	h := newHarness(t)
	// The parser leaves cwd unresolved: /w/repo/code/p/a/L/../x.
	h.fs.link(abs("code/p/a/L"), "../b")
	h.tree.link("code/p/a/L", "../b")
	e := fail(stat("gone"), "ENOENT")
	e.Cwd = abs("code/p/a/L/..")
	rec := h.run(e)
	// L -> ../b is code/p/b, whose parent is code/p: so cwd is code/p.
	eq(t, "entries", rec.Entries, "path:code/p/gone", "pkg:code/p/a")
	// A directory descriptor's -y path is used for relative names.
	d := fail(stat("gone"), "ENOENT")
	d.DirfdPath = abs("code/p/b")
	d.Cwd = "/nowhere"
	rec = h.run(d)
	eq(t, "entries", rec.Entries, "pkg:code/p/b")
}

func TestEventWithKernelPathNotUnderTheRoot(t *testing.T) {
	h := newHarness(t)
	// The kernel opened something inside the repository by an outside name
	// (a bind mount, say): flagged, and the kernel's path is used.
	rec := h.run(open("/mnt/alias/b.txt", "O_RDONLY", abs("code/p/b/b.txt")))
	wantReason(t, rec, "trace-incomplete:unstable-path")
	eq(t, "entries", rec.Entries, "pkg:code/p/b")
}

func TestOpenFlagForms(t *testing.T) {
	for text, want := range map[string]openFlags{
		"O_RDONLY|O_CLOEXEC":                                  {},
		"O_WRONLY|O_CREAT|O_EXCL":                             {wronly: true, creat: true, excl: true},
		"O_RDWR|O_NOFOLLOW":                                   {rdwr: true, nofollow: true},
		"{flags=O_WRONLY|O_TRUNC, mode=0644, resolve=0}":      {wronly: true, trunc: true},
		"{mode=0, flags=O_RDWR, resolve=RESOLVE_NO_SYMLINKS}": {rdwr: true},
		"O_RDONLY|O_DIRECTORY|O_TMPFILE":                      {tmpfile: true},
	} {
		got := parseOpenFlags(text)
		if got != want {
			t.Errorf("%q: %+v, want %+v", text, got, want)
		}
	}
	if (openFlags{tmpfile: true, wronly: true}).writes() {
		t.Error("O_TMPFILE is not a write of the path")
	}
	// A symlink in the last component is not followed with O_NOFOLLOW or O_CREAT|O_EXCL.
	h := newHarness(t)
	h.fs.link(abs("code/p/b/ln"), "b.txt")
	h.tree.link("code/p/b/ln", "b.txt")
	rec := h.run(open("code/p/b/ln", "O_RDONLY|O_NOFOLLOW", abs("code/p/b/ln")))
	wantNoReasons(t, rec)
	eq(t, "traversed", rec.Traversed, "code", "code/p", "code/p/b")
	_ = trace.Event{}
}
