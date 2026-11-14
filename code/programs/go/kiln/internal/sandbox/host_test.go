//go:build !windows

package sandbox

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// hostFixture builds a real directory layout and a Config for it.  Paths are
// canonical (symlinks resolved) because CheckHost requires it, and macOS's
// temporary directory is itself behind one.
type hostFixture struct {
	base string
	cfg  *Config
	exe  string
	conf string
}

func newHostFixture(t *testing.T) *hostFixture {
	t.Helper()
	base, err := filepath.EvalSymlinks(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	f := &hostFixture{base: base}
	repo := filepath.Join(base, "repo")
	for _, d := range []string{"repo/pkg", "repo/.git", "home", "home/present-mask", "tmp", "bin", "state"} {
		if err := os.MkdirAll(filepath.Join(base, d), 0o755); err != nil {
			t.Fatal(err)
		}
	}
	f.exe = filepath.Join(base, "bin", "kiln")
	f.conf = filepath.Join(base, "cfg.json")
	for _, p := range []string{f.exe, f.conf} {
		if err := os.WriteFile(p, []byte("x"), 0o755); err != nil {
			t.Fatal(err)
		}
	}
	f.cfg = &Config{
		RepoRoot:    repo,
		Home:        filepath.Join(base, "home"),
		Tmpdir:      filepath.Join(base, "tmp"),
		StateRoots:  []string{filepath.Join(base, "state"), filepath.Join(base, "state2", "deeper")},
		RoRoots:     []string{"/usr", "/bin"},
		Masks:       []string{filepath.Join(repo, ".git")},
		EnvByAction: map[string][]string{"pkg": {"A=b"}},
		KilnExe:     f.exe,
	}
	if err := f.cfg.Validate(); err != nil {
		t.Fatal(err)
	}
	return f
}

func (f *hostFixture) facts() HostFacts {
	return HostFacts{Exe: f.exe, Cwd: filepath.Join(f.cfg.RepoRoot, "pkg"), ConfigPath: f.conf}
}

func TestCheckHostAccepts(t *testing.T) {
	f := newHostFixture(t)
	if err := f.cfg.CheckHost("pkg", f.facts()); err != nil {
		t.Fatal(err)
	}
}

// Home is a fresh tmpfs in the sandbox, so a configuration beneath it that no
// bind exposes is safe (KILN04 4.7): this is where RUNNER_TEMP lives on a
// hosted runner.
func TestCheckHostAcceptsAConfigurationBeneathHome(t *testing.T) {
	f := newHostFixture(t)
	h := f.facts()
	h.ConfigPath = filepath.Join(f.cfg.Home, "work", "_temp", "kiln-record", "pre", "sandbox.json")
	if err := os.MkdirAll(filepath.Dir(h.ConfigPath), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(h.ConfigPath, []byte("x"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := f.cfg.CheckHost("pkg", h); err != nil {
		t.Fatal(err)
	}
}

func TestCheckHostRefusals(t *testing.T) {
	cases := []struct {
		name string
		edit func(f *hostFixture, h *HostFacts)
		want string
	}{
		{"wrong binary", func(f *hostFixture, h *HostFacts) { h.Exe = filepath.Join(f.base, "bin", "other") }, "running binary"},
		{"deleted binary", func(f *hostFixture, h *HostFacts) { h.Exe += " (deleted)" }, "running binary"},
		{"wrong cwd", func(f *hostFixture, h *HostFacts) { h.Cwd = f.cfg.RepoRoot }, "package directory"},
		{"cwd elsewhere", func(f *hostFixture, h *HostFacts) { h.Cwd = f.base }, "package directory"},
		{"relative config path", func(f *hostFixture, h *HostFacts) { h.ConfigPath = "cfg.json" }, "not absolute"},
		{"config in repo", func(f *hostFixture, h *HostFacts) { h.ConfigPath = filepath.Join(f.cfg.RepoRoot, "cfg.json") }, "rewrite"},
		{"config in an ro root", func(f *hostFixture, h *HostFacts) { h.ConfigPath = "/usr/share/cfg.json" }, "rewrite"},
		{"config in an ro root beneath home", func(f *hostFixture, h *HostFacts) {
			root := filepath.Join(f.cfg.Home, "present-mask")
			f.cfg.RoRoots = append(f.cfg.RoRoots, root)
			h.ConfigPath = filepath.Join(root, "cfg.json")
		}, "rewrite"},
		{"config over a state root", func(f *hostFixture, h *HostFacts) { h.ConfigPath = filepath.Join(f.base, "state") }, "rewrite"},
		{"config in tmpdir", func(f *hostFixture, h *HostFacts) { h.ConfigPath = filepath.Join(f.cfg.Tmpdir, "cfg.json") }, "rewrite"},
		{"config in state", func(f *hostFixture, h *HostFacts) { h.ConfigPath = filepath.Join(f.base, "state", "cfg.json") }, "rewrite"},
		{"repo missing", func(f *hostFixture, h *HostFacts) {
			os.RemoveAll(f.cfg.RepoRoot)
		}, "no such file"},
		{"repo is a file", func(f *hostFixture, h *HostFacts) {
			os.RemoveAll(f.cfg.RepoRoot)
			os.WriteFile(f.cfg.RepoRoot, []byte("x"), 0o644)
			h.Cwd = filepath.Join(f.cfg.RepoRoot, "pkg")
		}, "not a real directory"},
		{"home missing", func(f *hostFixture, h *HostFacts) { os.RemoveAll(f.cfg.Home) }, "no such file"},
		{"tmpdir is a symlink", func(f *hostFixture, h *HostFacts) {
			os.Remove(f.cfg.Tmpdir)
			os.Symlink(f.cfg.Home, f.cfg.Tmpdir)
		}, "not a real directory"},
		{"repo behind a symlinked ancestor", func(f *hostFixture, h *HostFacts) {
			link := filepath.Join(f.base, "link")
			os.Symlink(f.base, link)
			f.cfg.RepoRoot = filepath.Join(link, "repo")
			f.cfg.Masks = nil
			h.Cwd = filepath.Join(f.cfg.RepoRoot, "pkg")
		}, "not canonical"},
		{"state root behind a symlink", func(f *hostFixture, h *HostFacts) {
			os.Symlink(f.base, filepath.Join(f.base, "evil"))
			f.cfg.StateRoots = []string{filepath.Join(f.base, "evil", "state")}
		}, "not canonical"},
		{"state root is a file", func(f *hostFixture, h *HostFacts) {
			os.WriteFile(filepath.Join(f.base, "afile"), nil, 0o644)
			f.cfg.StateRoots = []string{filepath.Join(f.base, "afile")}
		}, "not a real directory"},
		{"state root under a file", func(f *hostFixture, h *HostFacts) {
			os.WriteFile(filepath.Join(f.base, "afile"), nil, 0o644)
			f.cfg.StateRoots = []string{filepath.Join(f.base, "afile", "x")}
		}, ""},
		{"mask is a symlink", func(f *hostFixture, h *HostFacts) {
			os.Remove(filepath.Join(f.cfg.RepoRoot, ".git"))
			os.Symlink(f.base, filepath.Join(f.cfg.RepoRoot, ".git"))
		}, "symlink"},
		{"mask reached through a symlink", func(f *hostFixture, h *HostFacts) {
			os.Symlink(f.cfg.RepoRoot, filepath.Join(f.base, "viaroot"))
			os.MkdirAll(filepath.Join(f.base, "elsewhere", "x"), 0o755)
			os.Symlink(filepath.Join(f.base, "elsewhere"), filepath.Join(f.cfg.RepoRoot, "linkdir"))
			f.cfg.Masks = []string{filepath.Join(f.cfg.RepoRoot, "linkdir", "x")}
		}, "not canonical"},
		{"mask missing inside the checkout", func(f *hostFixture, h *HostFacts) {
			os.RemoveAll(filepath.Join(f.cfg.RepoRoot, ".git"))
		}, "does not exist"},
		{"ro root missing", func(f *hostFixture, h *HostFacts) {
			f.cfg.RoRoots = []string{"/usr", filepath.Join(f.base, "no-such-root")}
		}, "no such file"},
		{"ro root behind a symlinked ancestor", func(f *hostFixture, h *HostFacts) {
			os.MkdirAll(filepath.Join(f.base, "ro1"), 0o755)
			os.Symlink(f.base, filepath.Join(f.base, "viaroot"))
			f.cfg.RoRoots = []string{filepath.Join(f.base, "viaroot", "ro1")}
		}, "not canonical"},
		{"mask missing outside the checkout", func(f *hostFixture, h *HostFacts) {
			f.cfg.Masks = []string{filepath.Join(f.base, "nope")}
		}, "does not exist"},
		{"mask under a symlinked ancestor", func(f *hostFixture, h *HostFacts) {
			os.Symlink(f.base, filepath.Join(f.base, "evil"))
			f.cfg.Masks = []string{filepath.Join(f.base, "evil", "home")}
		}, "not canonical"},
		{"kiln path is a symlink", func(f *hostFixture, h *HostFacts) {
			link := filepath.Join(f.base, "kiln-link")
			os.Symlink(f.exe, link)
			f.cfg.KilnExe, h.Exe = link, link
		}, "not canonical"},
		{"config path is a symlink", func(f *hostFixture, h *HostFacts) {
			link := filepath.Join(f.base, "cfg-link.json")
			os.Symlink(f.conf, link)
			h.ConfigPath = link
		}, "not canonical"},
		{"config missing", func(f *hostFixture, h *HostFacts) { os.Remove(f.conf) }, "cannot resolve the configuration"},
		{"binary in a state root", func(f *hostFixture, h *HostFacts) {
			f.cfg.StateRoots = []string{filepath.Join(f.base, "bin")}
		}, "inside"},
		{"binary inside the checkout via symlink", func(f *hostFixture, h *HostFacts) {
			real := filepath.Join(f.cfg.RepoRoot, "kiln-real")
			os.WriteFile(real, []byte("x"), 0o755)
			os.Remove(f.exe)
			os.Symlink(real, f.exe)
		}, "the kiln binary is inside"},
		{"binary missing", func(f *hostFixture, h *HostFacts) { os.Remove(f.exe) }, "cannot resolve"},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			f := newHostFixture(t)
			h := f.facts()
			tc.edit(f, &h)
			err := f.cfg.CheckHost("pkg", h)
			if err == nil || !strings.Contains(err.Error(), tc.want) {
				t.Fatalf("%v, want an error containing %q", err, tc.want)
			}
		})
	}
}

func TestCheckHostMaskMayBeAFile(t *testing.T) {
	f := newHostFixture(t)
	file := filepath.Join(f.base, "github_env")
	os.WriteFile(file, nil, 0o644)
	f.cfg.Masks = []string{file}
	if err := f.cfg.CheckHost("pkg", f.facts()); err != nil {
		t.Fatal(err)
	}
}

func TestDeepestExisting(t *testing.T) {
	d := t.TempDir()
	if got, err := deepestExisting(filepath.Join(d, "a", "b")); err != nil || got != d {
		t.Errorf("%q %v", got, err)
	}
	if got, err := deepestExisting(d); err != nil || got != d {
		t.Errorf("%q %v", got, err)
	}
	if got, err := deepestExisting("relative/none/x"); err != nil || got != "." {
		t.Errorf("a relative path ends at the current directory: %q %v", got, err)
	}
}

func TestCheckHostRoRootMayBeASymlinkOrAFile(t *testing.T) {
	f := newHostFixture(t)
	// A symlink root (/bin on most images) is recreated, never followed.
	os.MkdirAll(filepath.Join(f.base, "ro1"), 0o755)
	link := filepath.Join(f.base, "rolink")
	os.Symlink(filepath.Join(f.base, "ro1"), link)
	file := filepath.Join(f.base, "rofile")
	os.WriteFile(file, nil, 0o644)
	f.cfg.RoRoots = []string{"/usr", link, file}
	if err := f.cfg.CheckHost("pkg", f.facts()); err != nil {
		t.Fatal(err)
	}
}

func TestCheckHostRoRootSymlinkParentMustBeCanonical(t *testing.T) {
	f := newHostFixture(t)
	os.MkdirAll(filepath.Join(f.base, "real"), 0o755)
	os.Symlink(filepath.Join(f.base, "real"), filepath.Join(f.base, "alias"))
	os.Symlink("/usr", filepath.Join(f.base, "real", "entry"))
	// The entry itself is a symlink whose PARENT is reached through another.
	f.cfg.RoRoots = []string{"/usr", filepath.Join(f.base, "alias", "entry")}
	if err := f.cfg.CheckHost("pkg", f.facts()); err == nil || !strings.Contains(err.Error(), "ro root parent") {
		t.Fatalf("%v", err)
	}
	f.cfg.RoRoots = []string{"/usr", filepath.Join(f.base, "real", "entry")}
	if err := f.cfg.CheckHost("pkg", f.facts()); err != nil {
		t.Fatal(err)
	}
}
