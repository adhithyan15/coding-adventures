package traced

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// Tests of KILN04 D43, the sealed root.  The sandbox's new root holds only what
// internal/sandbox/plan.go mounts, so an absent probe of a path the plan does
// not contain is a constant, whatever the HOST's disk says.  Every test here
// is host-independent: the fixture disk is an in-memory fake that, where it
// matters, LIES (it holds the file the host would have), so a classifier that
// still asked it would see something present.

// scratchDirs is sandbox.ScratchDirs, spelled here because this package does
// not import the sandbox (internal/record pins that the two agree).
var scratchDirs = []string{"/tmp", "/var/tmp", "/run"}

func sealedHarness(t *testing.T) *harness {
	t.Helper()
	h := newHarness(t)
	h.cfg.ScratchDirs = scratchDirs
	return h
}

// sealedRealHome is the hosted-runner layout with a Home that is NOT an
// ancestor of the checkout, so the checkout's ancestors are sealed-root nodes.
func sealedRealHome(t *testing.T) *harness {
	t.Helper()
	const root = "/home/runner/work/x/x"
	h := sealedHarness(t)
	h.cfg.Root, h.fs.root = root, root
	h.cfg.Home = "/home/kiln"
	h.fs.dir("/home/runner").dir("/home/runner/work").dir("/home/runner/work/x").dir(root)
	return h
}

func TestSealedRootAbsentProbesAreConstants(t *testing.T) {
	probes := []string{
		"/run/systemd/resolve/stub-resolv.conf",
		"/package.json",
		"/home/package.json",
		"/var/run",
		"/var/run/x",
		"/opt/rh",
		"/go.work",
	}
	for _, p := range probes {
		for _, ev := range []trace.Event{fail(stat(p), "ENOENT"), fail(openAbs(p), "ENOENT"), fail(lstat(p), "ENOENT")} {
			h := sealedHarness(t)
			// The host HAS these files; the sandbox does not.
			h.fs.file(p)
			rec := h.run(ev)
			wantNoReasons(t, rec)
		}
		// Teeth: without the sealed root the same probe is a host-read.
		h := newHarness(t)
		if rec := h.run(fail(stat(p), "ENOENT")); !hasHostRead(rec) && !strings.HasPrefix(p, "/opt") {
			t.Errorf("%s: the pre-D43 classifier did not call this a host-read (%q): the test has no teeth", p, rec.Reasons)
		}
	}
}

// The checkout's ancestors, with the checkout beneath a real-looking home.
func TestSealedRootChecksOutAncestorsChildren(t *testing.T) {
	h := sealedRealHome(t)
	for _, p := range []string{
		"/home/runner/package.json", "/home/runner/work/package.json", "/home/runner/work/x/package.json", "/home/package.json", "/package.json",
	} {
		h.fs.file(p)
		wantNoReasons(t, h.run(fail(stat(p), "ENOENT")))
	}
	// $HOME/work/package.json with Home an ancestor of the checkout.
	h = realHomeHarness(t)
	h.cfg.ScratchDirs = scratchDirs
	wantNoReasons(t, h.run(fail(stat("/home/runner/work/package.json"), "ENOENT")))
	// A look at an ancestor itself is still D39's rule, and a LISTING of one is
	// still a host-read: the sealed root does not change what a directory shows.
	h = sealedRealHome(t)
	wantNoReasons(t, h.run(stat("/home/runner/work")))
	wantReason(t, h.run(getdents("/home/runner/work")), "host-read:/home/runner/work")
}

// A call that SUCCEEDED where the plan has nothing contradicts the model.
func TestSealedRootSuccessAtAnAbsentPathIsRefused(t *testing.T) {
	for _, p := range []string{"/package.json", "/var/run/docker.sock", "/home/package.json", "/run/user"} {
		h := sealedHarness(t)
		rec := h.run(openAbs(p))
		if p != "/run/user" && !hasHostRead(rec) {
			t.Errorf("a successful open of %s has no host-read: %q", p, rec.Reasons)
		}
		h = sealedHarness(t)
		if rec := h.run(stat(p)); p != "/run/user" && !hasHostRead(rec) {
			t.Errorf("a successful stat of %s has no host-read: %q", p, rec.Reasons)
		}
	}
	// Creating something there cannot work on a read-only root either.
	for name, ev := range map[string]trace.Event{
		"mkdir":   mkdir("/home/newdir"),
		"create":  open("/package.json", "O_WRONLY|O_CREAT|O_TRUNC", "/package.json"),
		"symlink": symlink("x", "/var/y"),
		"unlink":  write("unlink", "/home/f"),
		"chmod":   write("chmod", "/package.json"),
		"rename":  rename("/home/a", "/home/b"),
	} {
		h := sealedHarness(t)
		if rec := h.run(ev); !hasHostRead(rec) {
			t.Errorf("%s at a sandbox-absent path: no host-read: %q", name, rec.Reasons)
		}
	}
	// The same successes at paths the plan DOES contain are untouched.
	h := sealedHarness(t)
	wantNoReasons(t, h.run(mkdir("/tmp/ok"), open("/run/x", "O_WRONLY|O_CREAT|O_TRUNC", "/run/x"), mkdir("/var/tmp/y")))
}

// The checkout is judged exactly as before.
func TestSealedRootLeavesTheCheckoutAlone(t *testing.T) {
	evs := []trace.Event{
		fail(stat(abs("code/p/a/missing")), "ENOENT"),
		fail(stat(abs("untracked-top")), "ENOENT"),
		openAbs(abs("code/p/a/README")),
	}
	a := newHarness(t).run(evs...)
	b := sealedHarness(t).run(evs...)
	eq(t, "entries", b.Entries, a.Entries...)
	eq(t, "absent", b.AbsentObservations, a.AbsentObservations...)
	eq(t, "reasons", b.Reasons, a.Reasons...)
	if len(b.AbsentObservations) == 0 {
		t.Fatal("the fixture recorded no absence: the test pins nothing")
	}
}

// A scratch directory is writable: an uncreated name is absent, a name the
// action made is its own, and one it made and removed is absent again.
func TestSealedRootScratchDirectories(t *testing.T) {
	h := sealedHarness(t)
	r := "/run/x"
	rec := h.run(
		fail(stat("/run/systemd"), "ENOENT"),
		open(r, "O_WRONLY|O_CREAT|O_EXCL", r), openAbs(r), write("unlink", r),
		fail(stat(r), "ENOENT"), fail(stat("/run/x/y"), "ENOTDIR"),
	)
	wantNoReasons(t, rec)
	// The scratch directories themselves are nodes of the plan.
	wantNoReasons(t, sealedHarness(t).run(stat("/run"), stat("/tmp"), stat("/var/tmp"), stat("/var")))
	// A name nobody made, found PRESENT: the model is wrong, so refuse.
	wantReason(t, sealedHarness(t).run(openAbs("/tmp/ghost")), "host-read:/tmp/ghost")
	// ".." out of a scratch directory leads to "/", where the plan has no
	// /package.json either.
	wantNoReasons(t, sealedHarness(t).run(fail(stat("/run/../package.json"), "ENOENT"), fail(stat("/tmp/kiln-1/../../package.json"), "ENOENT")))
	// A path the action MADE under the fresh tmpfs is found there, not the host.
	h = sealedHarness(t)
	rec = h.run(mkdir("/var/tmp/d"), open("/var/tmp/d/f", "O_WRONLY|O_CREAT|O_EXCL", "/var/tmp/d/f"), openAbs("/var/tmp/d/f"))
	wantNoReasons(t, rec)
}

// A scratch directory under the checkout's own path (checkout at /tmp/w/x) is
// not fresh: the plan binds the checkout there.
func TestSealedRootScratchHoldingTheCheckout(t *testing.T) {
	h := sealedHarness(t)
	h.cfg.Root, h.fs.root = "/tmp/w/x", "/tmp/w/x"
	h.fs.dir("/tmp/w").dir("/tmp/w/x")
	if c := newClassifier(mustPrepare(t, h.cfg)); c.underTmp("/tmp/w") || c.underTmp("/tmp/w/x/f") || !c.underTmp("/tmp/w/y") {
		t.Fatal("underTmp misjudges the checkout's place under /tmp")
	}
	// /tmp/w/package.json is a child of an ancestor-chain node in a writable
	// tmpfs: absent unless the action makes it.
	wantNoReasons(t, h.run(fail(stat("/tmp/w/package.json"), "ENOENT")))
	rec := h.run(open("/tmp/w/q", "O_WRONLY|O_CREAT|O_EXCL", "/tmp/w/q"), openAbs("/tmp/w/q"))
	wantNoReasons(t, rec)
}

// (b) A symlink the action made in a writable root, pointing at a path the
// sealed root lacks.  The walk follows it, so the probe names the target: absent
// in the sandbox as the kernel found it.
func TestSealedRootSymlinkToAnAbsentPath(t *testing.T) {
	noHostRead := func(what string, rec *RawRecord) {
		t.Helper()
		if hasHostRead(rec) {
			t.Errorf("%s: an absent target is a constant, but got %q", what, rec.Reasons)
		}
	}
	for _, l := range []string{tTmp + "/l", tHome + "/l", abs("code/p/a/l")} {
		h := sealedHarness(t)
		noHostRead("link "+l, h.run(symlink("/package.json", l), fail(stat(l), "ENOENT")))
		// A relative link that climbs out of a scratch mount lands on "/".
		h = sealedHarness(t)
		noHostRead("relative link "+l, h.run(symlink("../../../../package.json", l), fail(stat(l), "ENOENT")))
		// The same link followed by a call that SUCCEEDED contradicts the plan.
		h = sealedHarness(t)
		if rec := h.run(symlink("/package.json", l), stat(l)); len(rec.Reasons) == 0 {
			t.Errorf("link %s: a successful stat through a link to a sandbox-absent path has no reason", l)
		}
	}
	// A link in a scratch directory is left to the old rules, which fail closed.
	h := sealedHarness(t)
	if rec := h.run(symlink("/package.json", "/tmp/l"), stat("/tmp/l")); len(rec.Reasons) == 0 {
		t.Error("a successful stat through /tmp/l has no reason")
	}
}

// (a) The stale cwd.  The trace keeps a cwd as TEXT; the kernel resolves ".."
// physically.  After a directory is renamed, text that names a sandbox-absent
// path (or a fresh scratch name) can physically reach the checkout, and an ENOENT
// there is an observation of the checkout that was never recorded.
func TestSealedRootStaleCwdAfterADirectoryRename(t *testing.T) {
	stat1 := func(cwd, p string) trace.Event {
		e := fail(stat(p), "ENOENT")
		e.Cwd = cwd
		return e
	}
	t.Run("scratch text, checkout physical", func(t *testing.T) {
		h := realHomeHarness(t)
		h.cfg.ScratchDirs = scratchDirs
		// cwd text /tmp/a/b (depth 3); after the rename the real cwd is /tmp/b2.
		// ../../home/runner/work/x/x/nope is /tmp/home/... as text (a fresh name
		// in /tmp) and /home/runner/work/x/x/nope for the kernel: the CHECKOUT.
		rec := h.run(
			mkdir("/tmp/a"), mkdir("/tmp/a/b"),
			rename("/tmp/a/b", "/tmp/b2"),
			stat1("/tmp/a/b", "../../home/runner/work/x/x/nope"),
		)
		if len(rec.Reasons) == 0 {
			t.Fatal("a probe from a stale cwd that really reached the checkout has no reason")
		}
		// ... and re-creating the old directory changes nothing.
		h = realHomeHarness(t)
		h.cfg.ScratchDirs = scratchDirs
		rec = h.run(
			mkdir("/tmp/a"), mkdir("/tmp/a/b"),
			rename("/tmp/a/b", "/tmp/b2"), mkdir("/tmp/a/b"),
			stat1("/tmp/a/b", "../../home/runner/work/x/x/nope"),
		)
		if len(rec.Reasons) == 0 {
			t.Fatal("re-created directory: no reason")
		}
	})
	t.Run("sealed text, checkout physical", func(t *testing.T) {
		// The directory the action renames is inside the checkout itself.
		h := sealedHarness(t)
		R := testRoot
		rec := h.run(
			mkdir(R+"/q"), mkdir(R+"/d1"), mkdir(R+"/d1/d2"),
			rename(R+"/q", R+"/d1/d2/q"),
			stat1(R+"/q", "../../../x"),
		)
		if len(rec.Reasons) == 0 {
			t.Fatal("a probe from a stale cwd inside the checkout has no reason")
		}
	})
	t.Run("exchange and unplaced renames", func(t *testing.T) {
		for name, evs := range map[string][]trace.Event{
			"exchange of directories": {mkdir("/tmp/p"), mkdir("/tmp/q"), exchange("/tmp/p", "/tmp/q")},
			"foreign name":            {rename("/tmp/preloaded", "/tmp/z")},
			"a directory of the host": {rename(abs("code/p/a/nested"), abs("code/p/a/other"))},
		} {
			h := sealedHarness(t)
			c := newClassifier(mustPrepare(t, h.cfg))
			c.runTrace(step(evs...))
			if !c.reshaped {
				t.Errorf("%s did not set reshaped", name)
			}
		}
		// Moving a regular file the action made, into place, does not.
		h := sealedHarness(t)
		c := newClassifier(mustPrepare(t, h.cfg))
		c.runTrace(step(open("/tmp/a", "O_WRONLY|O_CREAT|O_EXCL", "/tmp/a"), rename("/tmp/a", "/tmp/b")))
		if c.reshaped {
			t.Error("renaming an own regular file set reshaped")
		}
	})
	// The sealed-root absences are refused by the whole-action check alone: the
	// only thing wrong with this trace is that a directory was moved.
	t.Run("a move refuses every allowed absence", func(t *testing.T) {
		h := sealedHarness(t)
		rec := h.run(mkdir("/tmp/d"), rename("/tmp/d", "/tmp/e"), fail(stat("/package.json"), "ENOENT"))
		wantReason(t, rec, "host-read:/package.json")
		h = sealedHarness(t)
		rec = h.run(rename(abs("code/p/a/nested"), abs("code/p/a/other")), fail(stat("/run/systemd/resolve/stub-resolv.conf"), "ENOENT"))
		wantReason(t, rec, "host-read:/run/systemd/resolve/stub-resolv.conf")
	})
	// Found while writing D43 and true without ScratchDirs: a directory renamed
	// away from a cwd text, and a failed probe through "..", reached the
	// checkout while the classifier believed it was in the fresh tmpfs.
	t.Run("a cwd renamed away, failed probe through dotdot", func(t *testing.T) {
		h := realHomeHarness(t)
		rec := h.run(mkdir("/home/runner/a"), mkdir("/home/runner/a/b"), rename("/home/runner/a/b", "/home/runner/b2"),
			stat1("/home/runner/a/b", "../work/x/x/nope"))
		wantReason(t, rec, rUnstable)
	})
	// Without a rename the same probes stay constants.
	h := sealedHarness(t)
	wantNoReasons(t, h.run(mkdir("/tmp/a"), mkdir("/tmp/a/b"), stat1("/tmp/a/b", "../../home/runner/x")))
}

// SandboxHas is the exported form of the exists-set.
func TestSandboxHasTable(t *testing.T) {
	h := sealedRealHome(t)
	h.cfg.HostBinds = []string{"/home/kiln/.cargo/git"}
	for p, want := range map[string]bool{
		"/": true, "/home": true, "/home/runner": true, "/home/runner/work/x": true, "/home/runner/work/x/x": true,
		"/home/runner/work/x/x/code": true,
		"/usr/bin/sh":                true, "/etc/passwd": true, "/opt/tc": true, "/opt/tc/go/1.26/bin": true,
		"/home/kiln": true, "/home/kiln/.cache/x": true, "/home/kiln/.cargo/git/db": true,
		"/tmp": true, "/tmp/kiln-1": true, "/tmp/zz": true, "/var": true, "/var/tmp/q": true, "/run": true, "/run/x": true,
		"/dev/null": true, "/proc/self": true, "/sys/devices": true,
		"/package.json": false, "/home/package.json": false, "/home/runner/package.json": false, "/home/runner/work/package.json": false,
		"/var/run": false, "/var/lib": false, "/opt/rh": false, "/include": false, "/pyvenv.cfg": false, "/selinux": false,
		"/mnt": false, "/root": false, "/srv": false,
	} {
		got, err := SandboxHas(h.cfg, p)
		if err != nil {
			t.Fatal(err)
		}
		if got != want {
			t.Errorf("SandboxHas(%s) = %v, want %v", p, got, want)
		}
	}
}

func TestPrepareRefusesABadScratchDir(t *testing.T) {
	for _, d := range []string{"tmp", "/", "/tmp/", "/a/../b"} {
		h := newHarness(t)
		h.cfg.ScratchDirs = []string{d}
		if _, err := prepare(h.cfg); err == nil {
			t.Errorf("scratch dir %q was accepted", d)
		}
	}
}

// A declared root that is a symlink on the host (merged /usr: /bin -> usr/bin)
// with a policy that does not also declare its target.  The walk runs on the
// host's disk, so /bin/sh lands on /usr/bin/sh.  The sealed root must judge
// that path, and the target's ancestors, exactly as before D43: found on
// Linux CI, where internal/record's fixture declares only /bin and /sbin and
// an exec of sh turned from a host-read into trace-incomplete:unstable-path.
func TestSealedRootFollowsASymlinkedDeclaredRoot(t *testing.T) {
	merged := func(sealed bool) *harness {
		h := newHarness(t)
		if sealed {
			h.cfg.ScratchDirs = scratchDirs
		}
		h.fs.link("/bin", "usr/bin")
		var roots []legacy.HostRoot
		for _, r := range h.cfg.HostRoots.Roots {
			if r.Path != "/usr" {
				roots = append(roots, r)
			}
		}
		h.cfg.HostRoots.Roots = roots
		return h
	}
	for name, ev := range map[string]trace.Event{
		"exec": execve("/bin/sh", "sh"),
		"stat": stat("/bin/sh"),
		"open": openAbs("/bin/sh"),
	} {
		before := merged(false).run(ev)
		after := merged(true).run(ev)
		if strings.Join(before.Reasons, "|") != strings.Join(after.Reasons, "|") {
			t.Errorf("%s: the sealed root changed the verdict: before %q, after %q", name, before.Reasons, after.Reasons)
		}
		for _, r := range after.Reasons {
			if strings.HasPrefix(r, rUnstable) {
				t.Errorf("%s: %q", name, after.Reasons)
			}
		}
	}
	// A path beside the target that no root reaches is still sandbox-absent.
	h := merged(true)
	h.fs.file("/usr/share/x")
	for _, r := range h.run(fail(stat("/usr/share/x"), "ENOENT")).Reasons {
		if strings.Contains(r, "/usr/share") {
			t.Errorf("an absent probe beside the root's target asked the host: %q", r)
		}
	}
}
