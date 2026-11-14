package record

import (
	"bytes"
	"errors"
	"io/fs"
	"os"
	"reflect"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
)

// These tests use literal POSIX paths and a fake file system, so every refusal
// is exercised on every OS.

type fakeInfo struct {
	name string
	mode fs.FileMode
}

func (f fakeInfo) Name() string       { return f.name }
func (f fakeInfo) Size() int64        { return 0 }
func (f fakeInfo) Mode() fs.FileMode  { return f.mode }
func (f fakeInfo) ModTime() time.Time { return time.Time{} }
func (f fakeInfo) IsDir() bool        { return f.mode.IsDir() }
func (f fakeInfo) Sys() any           { return nil }

// fakeFS maps a path to a mode; a path not in the map does not exist.
type fakeFS struct {
	modes   map[string]fs.FileMode
	entries map[string]int // directory -> number of entries
	errOn   string         // lstat of this path fails with a non-ENOENT error
}

func (f *fakeFS) lstat(p string) (os.FileInfo, error) {
	if p == f.errOn {
		return nil, errors.New("boom")
	}
	m, ok := f.modes[p]
	if !ok {
		return nil, os.ErrNotExist
	}
	return fakeInfo{p, m}, nil
}

func (f *fakeFS) readDir(p string) ([]os.DirEntry, error) {
	n := f.entries[p]
	out := make([]os.DirEntry, n)
	for i := range out {
		out[i] = fs.FileInfoToDirEntry(fakeInfo{"e", 0})
	}
	return out, nil
}

const (
	dir = fs.ModeDir | 0o755
	lnk = fs.ModeSymlink | 0o777
)

// runnerFS is a hosted runner: image roots, the four $HOME toolchains.
func runnerFS() *fakeFS {
	return &fakeFS{modes: map[string]fs.FileMode{
		"/bin": lnk, "/sbin": lnk, "/lib": lnk, "/lib64": lnk, "/etc": dir, "/usr": dir,
		"/home/runner/.cargo/bin": dir, "/home/runner/.rustup": dir, "/home/runner/.dotnet": dir, "/home/runner/.ghcup": dir,
		"/opt/hostedtoolcache/go/1.26.1": dir,
	}, entries: map[string]int{}}
}

func hostRoots(roots ...legacy.HostRoot) *legacy.HostRoots { return &legacy.HostRoots{Roots: roots} }

// realRoots is host-roots.json as committed.
func realRoots() *legacy.HostRoots {
	return hostRoots(
		legacy.HostRoot{Path: "$HOME/.cargo/bin", Kind: legacy.RootTool, Tool: "cargo"},
		legacy.HostRoot{Path: "$HOME/.dotnet", Kind: legacy.RootTool, Tool: "dotnet"},
		legacy.HostRoot{Path: "$HOME/.ghcup", Kind: legacy.RootTool, Tool: "ghc"},
		legacy.HostRoot{Path: "$HOME/.rustup", Kind: legacy.RootTool, Tool: "rustc"},
		legacy.HostRoot{Path: "/bin", Kind: legacy.RootImage},
		legacy.HostRoot{Path: "/etc", Kind: legacy.RootImage},
		legacy.HostRoot{Path: "/lib", Kind: legacy.RootImage},
		legacy.HostRoot{Path: "/lib64", Kind: legacy.RootImage},
		legacy.HostRoot{Path: "/opt/hostedtoolcache/go/1.26.1", Kind: legacy.RootTool, Tool: "go"},
		legacy.HostRoot{Path: "/sbin", Kind: legacy.RootImage},
		legacy.HostRoot{Path: "/usr", Kind: legacy.RootImage},
	)
}

func baseInput(f *fakeFS) ConfigInput {
	return ConfigInput{
		Root: "/home/runner/work/repo/repo", Home: "/home/runner", Tmpdir: "/tmp/kiln-tmp", KilnExe: "/home/runner/work/_temp/kiln-bin/kiln",
		Roots: realRoots(),
		EnvByAction: map[string][]string{
			"code/packages/go/a":     {"HOME=/home/runner", "PATH=/opt/hostedtoolcache/go/1.26.1/x64/bin:/usr/bin", "TMPDIR=/tmp/kiln-tmp"},
			"code/packages/rust/foo": {"HOME=/home/runner", "PATH=/home/runner/.cargo/bin:/usr/bin", "TMPDIR=/tmp/kiln-tmp"},
		},
		Private: []Region{{"the record directory", "/home/runner/work/_temp/kiln-record"}, {"RUNNER_TEMP", "/home/runner/work/_temp"}},
		Lstat:   f.lstat, ReadDir: f.readDir,
	}
}

func TestBuildSandboxConfigOnAHostedRunner(t *testing.T) {
	cfg, err := BuildSandboxConfig(baseInput(runnerFS()))
	if err != nil {
		t.Fatal(err)
	}
	wantRO := []string{"/bin", "/etc", "/home/runner/.cargo/bin", "/home/runner/.dotnet", "/home/runner/.ghcup", "/home/runner/.rustup",
		"/lib", "/lib64", "/opt/hostedtoolcache/go/1.26.1", "/sbin", "/usr"}
	if !reflect.DeepEqual(cfg.RoRoots, wantRO) {
		t.Fatalf("ro_roots:\n got %v\nwant %v", cfg.RoRoots, wantRO)
	}
	// ~/.cargo/bin is the only root with an anchor, so only ~/.cargo's writable
	// directories are declared; ~/.rustup, ~/.dotnet and ~/.ghcup are bound as
	// they are.
	if !reflect.DeepEqual(cfg.StateRoots, []string{"/home/runner/.cargo/git", "/home/runner/.cargo/registry"}) {
		t.Fatalf("state_roots %v", cfg.StateRoots)
	}
	if !reflect.DeepEqual(cfg.Masks, []string{"/home/runner/work/repo/repo/.git"}) {
		t.Fatalf("masks %v", cfg.Masks)
	}
	if cfg.Home != "/home/runner" || cfg.Tmpdir != legacy.RecordTmpdir || cfg.RepoRoot != "/home/runner/work/repo/repo" {
		t.Fatalf("%+v", cfg)
	}
	// The result is something the sandbox itself accepts, from bytes.
	data, err := MarshalConfig(cfg)
	if err != nil {
		t.Fatal(err)
	}
	back, err := sandbox.ParseConfig(data)
	if err != nil {
		t.Fatalf("ParseConfig: %v\n%s", err, data)
	}
	if !reflect.DeepEqual(back, cfg) {
		t.Fatalf("round trip changed the configuration:\n%+v\n%+v", back, cfg)
	}
	if _, err := sandbox.Plan(back); err != nil {
		t.Fatalf("Plan: %v", err)
	}
	// Byte-stable: map keys are sorted, and a second marshal is identical.
	again, _ := MarshalConfig(cfg)
	if !bytes.Equal(data, again) || bytes.Contains(data, []byte("null")) {
		t.Fatalf("not canonical: %s", data)
	}
}

func TestBuildSandboxConfigDropsMissingRoots(t *testing.T) {
	f := runnerFS()
	for _, p := range []string{"/home/runner/.cargo/bin", "/home/runner/.ghcup", "/opt/hostedtoolcache/go/1.26.1", "/lib64"} {
		delete(f.modes, p)
	}
	cfg, err := BuildSandboxConfig(baseInput(f))
	if err != nil {
		t.Fatal(err)
	}
	for _, gone := range []string{"/home/runner/.cargo/bin", "/lib64", "/home/runner/.ghcup"} {
		for _, r := range cfg.RoRoots {
			if r == gone {
				t.Errorf("%s was kept though it does not exist", gone)
			}
		}
	}
	// No root under ~/.cargo, so no anchor and no state roots.
	if len(cfg.StateRoots) != 0 {
		t.Fatalf("state_roots %v", cfg.StateRoots)
	}
}

func TestBuildSandboxConfigNoToolRootsUnderHome(t *testing.T) {
	f := &fakeFS{modes: map[string]fs.FileMode{"/usr": dir, "/etc": dir}}
	in := baseInput(f)
	in.Home = "/tmp/anywhere" // allowed: no root lies beneath it (M2 applies only then)
	cfg, err := BuildSandboxConfig(in)
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(cfg.RoRoots, []string{"/etc", "/usr"}) {
		t.Fatalf("%v", cfg.RoRoots)
	}
}

func TestBuildSandboxConfigRefusals(t *testing.T) {
	cases := map[string]struct {
		mutate func(f *fakeFS, in *ConfigInput)
		want   string
	}{
		"a symlink root beneath home": {func(f *fakeFS, in *ConfigInput) {
			f.modes["/home/runner/.rustup"] = lnk
		}, "beneath home is a symlink"},
		"a symlink anchor-side root beneath home": {func(f *fakeFS, in *ConfigInput) {
			f.modes["/home/runner/.cargo/bin"] = lnk
		}, "beneath home is a symlink"},
		"a broad anchor": {func(f *fakeFS, in *ConfigInput) {
			f.modes["/home/runner/.local/bin"] = dir
			in.Roots = hostRoots(
				legacy.HostRoot{Path: "$HOME/.local/bin", Kind: legacy.RootTool, Tool: "x"},
				legacy.HostRoot{Path: "/usr", Kind: legacy.RootImage})
		}, "would anchor (seal read-only) all of \"/home/runner/.local\""},
		"home under /tmp with a root beneath it (M2)": {func(f *fakeFS, in *ConfigInput) {
			in.Home = "/tmp/k/home"
			f.modes["/tmp/k/home/.rustup"] = dir
		}, "lies beneath home"},
		"home inside tmpdir with a root beneath it": {func(f *fakeFS, in *ConfigInput) {
			in.Home = "/tmp/kiln-tmp/home"
			f.modes["/tmp/kiln-tmp/home/.rustup"] = dir
		}, "overlaps"},
		"a preloaded state root": {func(f *fakeFS, in *ConfigInput) {
			f.modes["/home/runner/.cargo/registry"] = dir
			f.entries["/home/runner/.cargo/registry"] = 3
		}, "unscrubbed:state-preloaded"},
		"a state root that is a symlink": {func(f *fakeFS, in *ConfigInput) {
			f.modes["/home/runner/.cargo/git"] = lnk
		}, "not a real directory"},
		"a state root that is a file": {func(f *fakeFS, in *ConfigInput) {
			f.modes["/home/runner/.cargo/git"] = 0o644
		}, "not a real directory"},
		"a root under $TMPDIR": {func(f *fakeFS, in *ConfigInput) {
			in.Roots = hostRoots(legacy.HostRoot{Path: "$TMPDIR/x", Kind: legacy.RootImage})
		}, "$TMPDIR"},
		"the record directory inside a root": {func(f *fakeFS, in *ConfigInput) {
			in.Private = []Region{{"the record directory", "/usr/share/kiln-record"}}
		}, "overlaps the record directory"},
		"a root containing RUNNER_TEMP": {func(f *fakeFS, in *ConfigInput) {
			f.modes["/opt/work"] = dir
			in.Private = []Region{{"RUNNER_TEMP", "/opt/work/_temp"}}
			in.Roots = hostRoots(legacy.HostRoot{Path: "/opt/work", Kind: legacy.RootImage}, legacy.HostRoot{Path: "/usr", Kind: legacy.RootImage})
		}, "overlaps RUNNER_TEMP"},
		"a root containing the kiln binary": {func(f *fakeFS, in *ConfigInput) {
			in.Private = []Region{{"the kiln binary", "/usr/local/bin/kiln"}}
		}, "overlaps the kiln binary"},
		"the directory of GITHUB_ENV": {func(f *fakeFS, in *ConfigInput) {
			in.Private = []Region{{"the directory of GITHUB_ENV", "/etc"}}
		}, "overlaps the directory of GITHUB_ENV"},
		"a declared state root over XDG_CONFIG_HOME": {func(f *fakeFS, in *ConfigInput) {
			in.XDGConfigHome = "/home/runner/.config"
			in.Roots = hostRoots(legacy.HostRoot{Path: "$HOME/.config", Kind: legacy.RootState, Ecosystem: "x"}, legacy.HostRoot{Path: "/usr", Kind: legacy.RootImage})
		}, "Kiln's Git calls"},
		"a declared state root over .gitconfig's directory": {func(f *fakeFS, in *ConfigInput) {
			in.Roots = hostRoots(legacy.HostRoot{Path: "$HOME/.gitconfig", Kind: legacy.RootState, Ecosystem: "x"}, legacy.HostRoot{Path: "/usr", Kind: legacy.RootImage})
		}, "Kiln's Git calls"},
		"a state root on the recorder's PATH": {func(f *fakeFS, in *ConfigInput) {
			in.Roots = hostRoots(legacy.HostRoot{Path: "/opt/hostedtoolcache/go", Kind: legacy.RootState, Ecosystem: "x"}, legacy.HostRoot{Path: "/usr", Kind: legacy.RootImage})
		}, "Kiln's Git calls or the recorder's PATH"},
		"the checkout is where the git config is read": {func(f *fakeFS, in *ConfigInput) {
			in.XDGConfigHome = "/home/runner/work/repo/repo/xdg"
		}, "Kiln's Git calls"},
		"no HOME":       {func(f *fakeFS, in *ConfigInput) { in.Home = "" }, "HOME"},
		"relative HOME": {func(f *fakeFS, in *ConfigInput) { in.Home = "home" }, "HOME"},
		"no host roots": {func(f *fakeFS, in *ConfigInput) { in.Roots = nil }, "host-roots.json is required"},
		"an lstat that fails for another reason": {func(f *fakeFS, in *ConfigInput) {
			f.errOn = "/usr"
		}, "boom"},
		"a state root lstat that fails": {func(f *fakeFS, in *ConfigInput) {
			f.errOn = "/home/runner/.cargo/registry"
		}, "boom"},
		"no root at all": {func(f *fakeFS, in *ConfigInput) {
			in.Roots = hostRoots(legacy.HostRoot{Path: "/nonexistent", Kind: legacy.RootImage})
		}, "ro_roots is empty"},
		"an ro root inside /tmp": {func(f *fakeFS, in *ConfigInput) {
			f.modes["/tmp/img"] = dir
			in.Roots = hostRoots(legacy.HostRoot{Path: "/tmp/img", Kind: legacy.RootImage})
		}, "builds itself"},
	}
	for name, c := range cases {
		t.Run(name, func(t *testing.T) {
			f := runnerFS()
			in := baseInput(f)
			c.mutate(f, &in)
			in.Lstat, in.ReadDir = f.lstat, f.readDir
			_, err := BuildSandboxConfig(in)
			if err == nil || !strings.Contains(err.Error(), c.want) {
				t.Fatalf("got %v, want %q", err, c.want)
			}
		})
	}
}

func TestBuildSandboxConfigDeclaredStateRootsAreKeptWhenEmpty(t *testing.T) {
	f := runnerFS()
	in := baseInput(f)
	in.Roots = hostRoots(
		legacy.HostRoot{Path: "$HOME/.cargo/registry", Kind: legacy.RootState, Ecosystem: "rust"},
		legacy.HostRoot{Path: "$HOME/.cargo/git", Kind: legacy.RootState, Ecosystem: "rust"},
		legacy.HostRoot{Path: "$HOME/.cargo/bin", Kind: legacy.RootTool, Tool: "cargo"},
		legacy.HostRoot{Path: "/proc/x", Kind: legacy.RootEphemeral},
		legacy.HostRoot{Path: "/usr", Kind: legacy.RootImage})
	cfg, err := BuildSandboxConfig(in)
	if err != nil {
		t.Fatal(err)
	}
	// Declared and table-derived entries are the same two paths: no duplicate.
	if !reflect.DeepEqual(cfg.StateRoots, []string{"/home/runner/.cargo/git", "/home/runner/.cargo/registry"}) {
		t.Fatalf("%v", cfg.StateRoots)
	}
	// An empty existing state root is fine.
	f.modes["/home/runner/.cargo/registry"] = dir
	if _, err := BuildSandboxConfig(in); err != nil {
		t.Fatal(err)
	}
}

func TestBuildSandboxConfigDefaultsToTheRealFileSystem(t *testing.T) {
	in := ConfigInput{Root: "/r/repo", Home: "/nonexistent-home", Tmpdir: "/tmp/kiln-tmp", KilnExe: "/opt/kiln",
		Roots:       hostRoots(legacy.HostRoot{Path: "/nonexistent-root-for-kiln-tests", Kind: legacy.RootImage}),
		EnvByAction: map[string][]string{"p": {"A=b"}}}
	if _, err := BuildSandboxConfig(in); err == nil || !strings.Contains(err.Error(), "ro_roots is empty") {
		t.Fatalf("%v", err)
	}
}

func TestMarshalConfigNeverWritesNull(t *testing.T) {
	data, err := MarshalConfig(&sandbox.Config{RepoRoot: "/r", Home: "/h", Tmpdir: "/t", KilnExe: "/k", EnvByAction: map[string][]string{"a": {}}})
	if err != nil {
		t.Fatal(err)
	}
	for _, want := range []string{`"ro_roots":[]`, `"state_roots":[]`, `"masks":[]`} {
		if !strings.Contains(string(data), want) {
			t.Errorf("%s missing in %s", want, data)
		}
	}
}
