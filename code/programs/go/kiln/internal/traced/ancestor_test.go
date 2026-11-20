package traced

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// KILN04 D39: the ancestor-node rule and the fresh-tmpfs absence rule.
//
// The sandbox's directories have parents, and a path walk looks at every one
// of them: stat("/opt/hostedtoolcache/go/1.26/bin/go") visits /, /opt,
// /opt/hostedtoolcache and /opt/hostedtoolcache/go on the way.  Those nodes are
// real directories by construction (each is an ancestor of an anchor that
// the classifier already checked), and the model treats a directory's type and
// metadata as a non-input.  A LISTING of them would show sibling names, which
// are host state, so that stays a host-read.
//
// The harness config declares the anchors: the checkout /w/repo, Home
// /home/runner, Tmpdir /tmp/kiln-1, the tool root /opt/tc/go/1.26 and the
// ephemeral entries /dev/null and /dev/fd/*.  The tests add a hostedtoolcache
// tool root so the two-level ancestor chain of a real runner is covered.

func ancestorHarness(t *testing.T) *harness {
	t.Helper()
	h := newHarness(t)
	h.cfg.HostRoots.Roots = append(h.cfg.HostRoots.Roots,
		legacy.HostRoot{Path: "/opt/hostedtoolcache/go/1.26", Kind: legacy.RootTool, Tool: "go"})
	for _, d := range []string{"/opt", "/opt/hostedtoolcache", "/opt/hostedtoolcache/go", "/w", "/opt/tc"} {
		h.fs.dir(d)
	}
	return h
}

func TestAncestorNodesAreAllowed(t *testing.T) {
	h := ancestorHarness(t)
	for _, p := range []string{
		"/",                       // the parent of everything
		"/home",                   // the parent of Home
		"/w",                      // the parent of the checkout
		"/opt",                    // the parent of a tool root
		"/opt/hostedtoolcache",    // two levels up from a tool root
		"/opt/hostedtoolcache/go", // one level up
		"/tmp",                    // the parent of Tmpdir
		"/dev",                    // the parent of an ephemeral entry
	} {
		for name, ev := range map[string]func() []string{
			"stat":  func() []string { return h.run(stat(p)).Reasons },
			"lstat": func() []string { return h.run(lstat(p)).Reasons },
			"access": func() []string {
				return h.run(base("access", trace.KindMetadata, p)).Reasons
			},
			"open-dir": func() []string {
				return h.run(open(p, "O_RDONLY|O_DIRECTORY", p)).Reasons
			},
			"open": func() []string { return h.run(openAbs(p)).Reasons },
		} {
			if r := ev(); len(r) != 0 {
				t.Errorf("%s of ancestor %s: reasons %v, want none", name, p, r)
			}
		}
	}
}

func TestAncestorWalkOfToolFile(t *testing.T) {
	h := ancestorHarness(t)
	// A walk to a file under a keyed tool root visits every ancestor.
	wantNoReasons(t, h.run(stat("/opt/hostedtoolcache/go/1.26/bin/go")))
}

func TestAncestorListingIsRefused(t *testing.T) {
	h := ancestorHarness(t)
	for _, p := range []string{"/opt/hostedtoolcache", "/", "/home", "/tmp", "/dev", "/w"} {
		wantReason(t, h.run(getdents(p)), "host-read:"+p)
	}
}

func TestAncestorChildProbesAreRefused(t *testing.T) {
	h := ancestorHarness(t)
	// Go's upward go.work search: a CHILD of an ancestor is not an ancestor.
	wantReason(t, h.run(fail(stat("/home/go.work"), "ENOENT")), "host-read:/home/go.work")
	wantReason(t, h.run(fail(stat("/go.work"), "ENOENT")), "host-read:/go.work")
	wantReason(t, h.run(fail(stat("/opt/hostedtoolcache/other"), "ENOENT")), "host-read:/opt/hostedtoolcache/other")
	// A present sibling of an ancestor is real host state.
	wantReason(t, h.run(openAbs("/home/other")), "host-read:/home/other")
	wantReason(t, h.run(openAbs("/opt/hostedtoolcache/python")), "host-read:/opt/hostedtoolcache/python")
	wantReason(t, h.run(stat("/var")), "host-read:/var") // not an ancestor of any anchor
}

func TestAnchorsKeepTheirOwnRules(t *testing.T) {
	h := ancestorHarness(t)
	// A tool root that is not keyed is refused: the anchor is not its own
	// ancestor, and a root's rule outranks the ancestor rule.
	wantReason(t, h.run(stat("/opt/tc/ruby/3.4")), "host-read:tool-not-keyed:ruby")
	wantReason(t, h.run(fail(stat("/opt/tc/ruby/3.4/bin"), "ENOENT")), "host-read:tool-not-keyed:ruby")
	// The checkout's own ancestors are not the checkout: /w/repo is judged by
	// the repository rules, and a tracked file is still a pkg entry.
	wantNoReasons(t, h.run(stat("/w/repo")))
	// The state root keeps its rule: an unpinned read is refused, an absent
	// probe is allowed (state roots start empty).
	old := fetchersEnabled
	fetchersEnabled = true
	defer func() { fetchersEnabled = old }()
	h.cfg.Pinned = map[string]bool{}
	wantReason(t, h.run(openAbs("/home/runner/.cargo/registry/index")), "host-read:$HOME/.cargo/registry/index")
	wantNoReasons(t, h.run(fail(stat("/home/runner/.cargo/registry/absent"), "ENOENT")))
}

func TestAbsentUnderFreshTmpfsIsAllowed(t *testing.T) {
	h := ancestorHarness(t)
	wantNoReasons(t, h.run(fail(stat("/home/runner/.proverc"), "ENOENT")))
	wantNoReasons(t, h.run(fail(stat("/home/runner/go"), "ENOENT")))
	wantNoReasons(t, h.run(fail(stat("/home/runner/.cache/go-build/00"), "ENOENT")))
	wantNoReasons(t, h.run(fail(stat("/tmp/kiln-1/never"), "ENOENT")))
	// A directory the action made under TMPDIR, removed, then probed again.
	wantNoReasons(t, h.run(
		mkdir("/tmp/kiln-1/gotest"),
		write("rmdir", "/tmp/kiln-1/gotest"),
		fail(stat("/tmp/kiln-1/gotest"), "ENOENT"),
	))
}

func TestPresentUncreatedUnderFreshTmpfsIsRefused(t *testing.T) {
	h := ancestorHarness(t)
	wantReason(t, h.run(stat("/home/runner/x")), "host-read:$HOME/x")
	wantReason(t, h.run(openAbs("/tmp/kiln-1/y")), "host-read:$TMPDIR/y")
}

func TestUnderTmpExcludesCheckoutAndRoots(t *testing.T) {
	h := ancestorHarness(t)
	c := newClassifier(mustPrepare(t, h.cfg))
	for _, p := range []string{"/w/repo", "/w/repo/x", "/w", "/opt/tc/go/1.26/x", "/opt/tc", "/home/runner/.cargo/registry/x", "/home/runner/.cargo"} {
		if c.underTmp(p) {
			t.Errorf("underTmp(%q) is true; a checkout, root or ancestor of one must never be fresh", p)
		}
	}
	for _, p := range []string{"/home/runner/x", "/tmp/kiln-1/x"} {
		if !c.underTmp(p) {
			t.Errorf("underTmp(%q) is false", p)
		}
	}
}

func mustPrepare(t *testing.T, cfg Config) *prepared {
	t.Helper()
	p, err := prepare(cfg)
	if err != nil {
		t.Fatal(err)
	}
	return p
}

func TestLiteralPrefix(t *testing.T) {
	for in, want := range map[string]string{
		"/dev/null":                  "/dev/null",
		"/dev/fd/*":                  "/dev/fd",
		"/sys/devices/system/cpu/**": "/sys/devices/system/cpu",
		"/a/b?/c":                    "/a",
		"/*":                         "/",
	} {
		if got := literalPrefix(in); got != want {
			t.Errorf("literalPrefix(%q) = %q, want %q", in, got, want)
		}
	}
}
