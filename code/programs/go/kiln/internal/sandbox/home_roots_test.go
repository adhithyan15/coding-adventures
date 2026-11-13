package sandbox

// Tests of S1 (KILN04 section 4.7): read-only roots beneath $HOME.  They are
// pure, so they run on every OS; the namespace half is
// TestSandboxROBeneathHome in the kiln package.

import (
	"reflect"
	"strings"
	"testing"
)

// TestValidateRoRootsBeneathHome: a tool root under $HOME is allowed (strictly
// beneath it), and every other overlap is still refused.
func TestValidateRoRootsBeneathHome(t *testing.T) {
	ok := map[string][]string{
		"direct child":        {"/usr", "/home/runner/.rustup"},
		"deeper":              {"/usr", "/home/runner/.cargo/bin"},
		"several, one anchor": {"/usr", "/home/runner/.cargo/bin", "/home/runner/.cargo/env"},
		"mixed":               {"/usr", "/home/runner/.cargo/bin", "/home/runner/.rustup", "/home/runner/.dotnet", "/home/runner/.ghcup"},
	}
	for name, roots := range ok {
		c := goodConfig()
		c.RoRoots = roots
		if err := c.Validate(); err != nil {
			t.Errorf("%s: %v", name, err)
		}
	}
	bad := []struct {
		name string
		edit func(*Config)
		want string
	}{
		{"is home", func(c *Config) { c.RoRoots = []string{"/usr", c.Home} }, "overlaps"},
		{"contains home", func(c *Config) { c.RoRoots = []string{"/usr", "/home"} }, "overlaps"},
		{"under the checkout", func(c *Config) { c.RoRoots = []string{"/usr", c.RepoRoot + "/tools"} }, "overlaps"},
		{"under tmpdir", func(c *Config) { c.RoRoots = []string{"/usr", c.Tmpdir + "/tools"} }, "overlaps"},
		{"under a state root", func(c *Config) { c.RoRoots = []string{"/usr", c.StateRoots[0] + "/x"} }, "overlaps"},
		{"anchor holds the checkout", func(c *Config) { c.RoRoots = []string{"/usr", "/home/runner/work/tools"} }, "also holds"},
		{"anchor holds tmpdir", func(c *Config) {
			c.Tmpdir = "/home/runner/.cache/tmp"
			c.RoRoots = []string{"/usr", "/home/runner/.cache/bin"}
		}, "also holds"},
		{"nested under home", func(c *Config) {
			c.RoRoots = []string{"/usr", "/home/runner/.cargo/bin", "/home/runner/.cargo/bin/x"}
		}, "inside ro root"},
		{"anchor is another root", func(c *Config) {
			c.RoRoots = []string{"/usr", "/home/runner/.cargo", "/home/runner/.cargo/bin"}
		}, "overlaps"}, // the shallower one already overlaps the state root
		{"outside home, inside /tmp", func(c *Config) { c.RoRoots = []string{"/usr", "/tmp/tools"} }, "builds itself"},
	}
	for _, tc := range bad {
		c := goodConfig()
		tc.edit(c)
		if err := c.Validate(); err == nil || !strings.Contains(err.Error(), tc.want) {
			t.Errorf("%s: %v, want an error containing %q", tc.name, err, tc.want)
		}
	}
	// Home may itself live under /tmp (the test fixtures' does): its roots are
	// bound after every scratch tmpfs, so /tmp cannot hide them.
	c0 := goodConfig()
	c0.Home, c0.Tmpdir, c0.RepoRoot, c0.StateRoots, c0.Masks = "/tmp/h", "/var/tmp/t", "/work/repo", nil, []string{"/work/repo/.git"}
	c0.RoRoots = []string{"/usr", "/tmp/h/.cargo/bin"}
	if err := c0.Validate(); err != nil {
		t.Errorf("home under /tmp: %v", err)
	}
	// Nesting with no state root in the way.
	c := goodConfig()
	c.StateRoots = nil
	c.RoRoots = []string{"/usr", "/home/runner/.cargo", "/home/runner/.cargo/bin"}
	if err := c.Validate(); err == nil || !strings.Contains(err.Error(), "inside ro root") {
		t.Errorf("nested roots beneath home: %v", err)
	}
}

func TestHomeAnchor(t *testing.T) {
	for _, tc := range []struct {
		home, r, anchor string
		ok              bool
	}{
		{"/h", "/h/.cargo/bin", "/h/.cargo", true},
		{"/h", "/h/.rustup", "/h/.rustup", true},
		{"/h", "/h/a/b/c", "/h/a", true},
		{"/h", "/h", "", false},
		{"/h", "/usr", "", false},
		{"/h", "/h2/x", "", false}, // a sibling sharing the prefix is not beneath home
	} {
		a, ok := HomeAnchor(tc.home, tc.r)
		if a != tc.anchor || ok != tc.ok {
			t.Errorf("HomeAnchor(%q, %q) = %q, %v; want %q, %v", tc.home, tc.r, a, ok, tc.anchor, tc.ok)
		}
	}
}

// homeRootsConfig is goodConfig with the tool roots host-roots.json declares
// beneath $HOME: ~/.cargo/bin shares the anchor ~/.cargo with the writable
// state root ~/.cargo/registry.
func homeRootsConfig() *Config {
	c := goodConfig()
	c.RoRoots = append(c.RoRoots, "/home/runner/.cargo/bin", "/home/runner/.rustup", "/home/runner/.dotnet")
	return c
}

// TestPlanRoRootsBeneathHome pins the S1 order: nothing beneath home is bound
// before home's tmpfs; direct children are plain binds; a deeper root goes into
// a read-only anchor tmpfs that is mounted first and sealed last, with the
// mount point of a state root beneath it created before the seal.
func TestPlanRoRootsBeneathHome(t *testing.T) {
	c := homeRootsConfig()
	plan, err := Plan(c)
	if err != nil {
		t.Fatal(err)
	}
	index := func(kind MountKind, target string) int {
		t.Helper()
		for i, m := range plan {
			if m.Kind == kind && m.Target == target {
				return i
			}
		}
		t.Fatalf("no %s step for %s", kind, target)
		return -1
	}
	home := index(KindTmpfs, c.Home)
	tmpdir := index(KindTmpfs, c.Tmpdir)
	anchor := index(KindTmpfs, "/home/runner/.cargo")
	bin := index(KindRORoot, "/home/runner/.cargo/bin")
	seal := index(KindSealDir, "/home/runner/.cargo")
	regDir := -1
	for i, m := range plan {
		if m.Kind == KindEnsureDir && m.Target == "/home/runner/.cargo/registry" && i < seal {
			regDir = i
		}
	}
	if regDir < 0 {
		t.Error("the mount point of the state root beneath the anchor is not created before the seal")
	}
	if !(home < anchor && tmpdir < anchor && anchor < bin && bin < seal) {
		t.Errorf("order: home %d, tmpdir %d, anchor %d, bin %d, seal %d", home, tmpdir, anchor, bin, seal)
	}
	for _, r := range []string{"/home/runner/.rustup", "/home/runner/.dotnet"} {
		if i := index(KindRORoot, r); i < home {
			t.Errorf("%s is bound before home's tmpfs hides it", r)
		}
	}
	// The state root's writable bind comes later (step 7), after the seal: a
	// mount over an existing directory of a read-only tmpfs is allowed.
	if b := index(KindBind, "/home/runner/.cargo/registry"); b < seal {
		t.Error("the state root is bound before the anchor is sealed")
	}
	// Exactly one anchor (both ~/.cargo roots share it), and a direct child
	// gets no tmpfs of its own.
	var seals []string
	for _, m := range plan {
		if m.Kind == KindSealDir {
			seals = append(seals, m.Target)
		}
	}
	if !reflect.DeepEqual(seals, []string{"/home/runner/.cargo"}) {
		t.Errorf("seal-dir steps: %v", seals)
	}
	for _, p := range targets(plan, KindTmpfs) {
		if p == "/home/runner/.rustup" {
			t.Error("a direct child of home got an anchor tmpfs")
		}
	}
	// The anchor tmpfs is mounted before the checkout's bind, so it hides
	// nothing the checkout needs.
	if anchor > index(KindBind, c.RepoRoot) {
		t.Error("the anchor tmpfs would hide the checkout")
	}
	for _, m := range plan {
		if m.Kind == KindRORoot && m.Flags != MsBind {
			t.Errorf("ro root flags %#x", m.Flags)
		}
	}
}

// TestPlanAnchorsAreSortedAndSeparate: two anchors, each sealed after its own
// roots.
func TestPlanAnchorsAreSortedAndSeparate(t *testing.T) {
	c := goodConfig()
	c.RoRoots = append(c.RoRoots, "/home/runner/.z/bin", "/home/runner/.a/x/y")
	plan, err := Plan(c)
	if err != nil {
		t.Fatal(err)
	}
	var seq []string
	for _, m := range plan {
		switch m.Kind {
		case KindTmpfs:
			if strings.HasPrefix(m.Target, "/home/runner/.") {
				seq = append(seq, "tmpfs "+m.Target)
			}
		case KindRORoot:
			if strings.HasPrefix(m.Target, "/home/runner/") {
				seq = append(seq, "bind "+m.Target)
			}
		case KindSealDir:
			seq = append(seq, "seal "+m.Target)
		}
	}
	want := []string{
		"tmpfs /home/runner/.a", "bind /home/runner/.a/x/y", "seal /home/runner/.a",
		"tmpfs /home/runner/.z", "bind /home/runner/.z/bin", "seal /home/runner/.z",
	}
	if !reflect.DeepEqual(seq, want) {
		t.Errorf("got %v\nwant %v", seq, want)
	}
}

// TestPlanWithoutHomeRootsHasNoSealDir: a configuration with no root beneath
// home plans exactly what it did before S1.
func TestPlanWithoutHomeRootsHasNoSealDir(t *testing.T) {
	plan, err := Plan(goodConfig())
	if err != nil {
		t.Fatal(err)
	}
	if got := targets(plan, KindSealDir); len(got) != 0 {
		t.Errorf("seal-dir steps: %v", got)
	}
}
