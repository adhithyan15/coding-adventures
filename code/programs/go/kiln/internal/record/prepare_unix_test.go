//go:build unix

package record

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"
	"syscall"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// realHome is the user's home directory as the process started, before any
// test points $HOME at a temp directory. A few tests need a read-only root that
// lies outside /tmp, which the sandbox refuses as a root.
var realHome, _ = os.UserHomeDir()

// prep is a throw-away shard: a committed fixture repository, a record
// directory to be, binaries outside the repository, and a fake build tool.
type prep struct {
	t         *testing.T
	base      string // canonical, holds everything but the repository
	root      string // the checkout
	dir       string // the record directory (absent)
	home      string
	tmpdir    string
	kiln, bt  string
	strace    string
	gitBin    string // the real git, resolved
	gitSum    string
	sha       string // HEAD
	env       []string
	getenv    map[string]string
	roots     []string // extra host-roots.json entries, as JSON objects
	steps     string   // the steps export (default: built from the graph)
	graph     string   // the graph export (default: the three fixture packages)
	toolErr   map[string]error
	toolCalls [][]string
	toolEnvs  [][]string
	files     map[string]string
}

const fixtureToolchains = `{
  "cc": {"always": true, "pins": {"cc": "image"}, "probes": [["cc", "--version"]], "roots": {"cc": ["/nonexistent/kiln-test-bin"]}},
  "sh": {"always": true, "pins": {"sh": "image"}},
  "x": {"pins": {"xtool": "1.2.3"}, "probes": [["xtool", "--version"]], "roots": {"xtool": ["/opt/x/bin"]}}
}
`

func writeBytes(t *testing.T, p string, data []byte, mode os.FileMode) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(p, data, mode); err != nil {
		t.Fatal(err)
	}
}

// realGit finds the real git, resolved through symlinks, and hashes it: the
// recorder runs exactly that file.
func realGit(t *testing.T) (string, string) {
	t.Helper()
	g, err := exec.LookPath("git")
	if err != nil {
		t.Skip("git is not installed")
	}
	g, err = filepath.EvalSymlinks(g)
	if err != nil {
		t.Fatal(err)
	}
	b, err := ResolveBinary("git", g)
	if err != nil {
		t.Fatal(err)
	}
	return b.Path, b.SHA256
}

func newPrep(t *testing.T) *prep {
	t.Helper()
	p := &prep{t: t, base: canonTemp(t), toolErr: map[string]error{}}
	p.home = filepath.Join(p.base, "home")
	p.tmpdir = filepath.Join(p.base, "kiln-tmp")
	p.dir = filepath.Join(p.base, "record")
	for _, d := range []string{p.home} {
		if err := os.MkdirAll(d, 0o755); err != nil {
			t.Fatal(err)
		}
	}
	// The binaries live outside the repository.
	p.kiln, p.bt, p.strace = filepath.Join(p.base, "bin", "kiln"), filepath.Join(p.base, "bin", "build-tool"), filepath.Join(p.base, "bin", "strace")
	writeBytes(t, p.kiln, []byte("kiln binary"), 0o755)
	writeBytes(t, p.bt, []byte("build tool binary"), 0o755)
	writeBytes(t, p.strace, []byte("strace"), 0o755)
	p.gitBin, p.gitSum = realGit(t)
	p.files = map[string]string{
		"code/kiln/legacy/pinned.json":        `{"languages": []}`,
		"code/kiln/legacy/never.json":         `{"packages": []}`,
		"code/kiln/legacy/toolchains.json":    fixtureToolchains,
		"code/kiln/legacy/env.json":           `{"names": ["FIXTURE_*", "LANG"]}`,
		"code/programs/go/build-tool/go.mod":  "module example.com/bt\n\ngo 1.26\n",
		"code/programs/go/build-tool/main.go": "package main\n",
		"code/packages/x/a/BUILD":             "echo a\n",
		"code/packages/x/a/src.txt":           "a\n",
		"code/packages/x/b/BUILD":             "echo b1\necho b2\n",
		"code/packages/x/z/BUILD":             "echo z\n",
	}
	p.env = []string{"HOME=" + p.home, "FIXTURE_A=1", "OTHER=2", "PATH=/usr/bin:/bin", "GITHUB_TOKEN=not-a-real-token", "LANG=C.UTF-8"}
	return p
}

// commit writes host-roots.json and commits the repository.
func (p *prep) commit() {
	p.t.Helper()
	roots := append([]string{`{"path": "/bin", "kind": "image"}`, `{"path": "/sbin", "kind": "image"}`}, p.roots...)
	sort.Slice(roots, func(a, b int) bool { return rootPath(roots[a]) < rootPath(roots[b]) })
	p.files["code/kiln/legacy/host-roots.json"] = `{"roots": [` + strings.Join(roots, ",") + `], "ephemeral": ["/dev/null"], "absent-ok": []}`
	repo := newRepo(p.t, p.files)
	root, err := filepath.EvalSymlinks(repo)
	if err != nil {
		p.t.Fatal(err)
	}
	p.root = root
	p.sha = runGit(p.t, root, "rev-parse", "HEAD")
	p.getenv = map[string]string{
		"GITHUB_SHA": p.sha, "GITHUB_REPOSITORY": "owner/repo", "GITHUB_RUN_ID": "43117337685", "GITHUB_RUN_ATTEMPT": "2",
	}
}

func rootPath(j string) string {
	var r struct{ Path string }
	if err := json.Unmarshal([]byte(j), &r); err != nil {
		panic(err)
	}
	return r.Path
}

// The fake build tool's exports, derived from the fixture's three packages.
func (p *prep) graphJSON() []byte {
	if p.graph != "" {
		return []byte(p.graph)
	}
	b := "BUILD"
	mk := func(name, rel string, deps ...string) legacy.Package {
		if deps == nil {
			deps = []string{}
		}
		return legacy.Package{
			Name: name, RelPath: rel, Language: "x",
			BuildFiles: map[string]*string{"linux": &b, "darwin": &b, "windows": &b},
			Deps:       map[string][]string{"linux": deps, "darwin": deps, "windows": deps},
			DevDeps:    []string{}, RepoInputs: []string{}, RepoInputGlobs: []string{}, ExtraToolchains: []string{},
		}
	}
	g := legacy.Graph{Schema: legacy.GraphSchema, Packages: []legacy.Package{
		mk("x/a", "code/packages/x/a", "x/b"), mk("x/b", "code/packages/x/b"), mk("x/z", "code/packages/x/z"),
	}}
	data, err := json.Marshal(g)
	if err != nil {
		p.t.Fatal(err)
	}
	return data
}

func (p *prep) stepsJSON() []byte {
	if p.steps != "" {
		return []byte(p.steps)
	}
	return []byte(`{"schema":"kiln-legacy-steps/1","os":"linux","packages":[` +
		`{"package":"code/packages/x/a","steps":["echo a"]},` +
		`{"package":"code/packages/x/b","steps":["echo b1","echo b2"]},` +
		`{"package":"code/packages/x/z","steps":["echo z"]}]}`)
}

const fixturePlan = `{
  "schema_version": 1, "force": true,
  "packages": [
    {"name": "x/a", "rel_path": "code/packages/x/a", "language": "x"},
    {"name": "x/b", "rel_path": "code/packages/x/b", "language": "x"},
    {"name": "x/z", "rel_path": "code/packages/x/z", "language": "x"}
  ],
  "shards": [
    {"index": 0, "name": "s0", "assigned_packages": ["x/a", "x/z"], "package_names": ["x/a", "x/b", "x/z"]},
    {"index": 1, "name": "s1", "assigned_packages": ["x/b"], "package_names": ["x/b"]}
  ]
}`

var plan = fixturePlan

// run is the fake build tool: it writes the three exports where the arguments
// say, as the real one does.
func (p *prep) run(r ToolRun) error {
	p.toolCalls = append(p.toolCalls, append([]string{r.Path}, r.Args...))
	p.toolEnvs = append(p.toolEnvs, r.Env)
	arg := func(flag string) string {
		for i, a := range r.Args {
			if a == flag && i+1 < len(r.Args) {
				return r.Args[i+1]
			}
		}
		return ""
	}
	if g := arg("-emit-kiln-graph"); g != "" {
		if err := p.toolErr["graph"]; err != nil {
			return err
		}
		if err := os.WriteFile(g, p.graphJSON(), 0o644); err != nil {
			return err
		}
		return os.WriteFile(arg("-emit-kiln-steps"), p.stepsJSON(), 0o644)
	}
	if err := p.toolErr["plan"]; err != nil {
		return err
	}
	return os.WriteFile(arg("-emit-plan"), []byte(plan), 0o644)
}

var fixedNow = time.Date(2026, 11, 13, 2, 3, 4, 0, time.UTC)

func (p *prep) options() Options {
	return Options{
		Root: p.root, Dir: p.dir, BuildTool: p.bt, KilnExe: p.kiln, Strace: p.strace, Git: p.gitBin,
		ShardIndex: 0, ShardCount: 2, JobStart: 1794000000,
		Image:   "github/linux/amd64/ubuntu24/20260101.1",
		Getenv:  func(k string) string { return p.getenv[k] },
		Environ: p.env,
		Tmpdir:  p.tmpdir,
		Now:     func() time.Time { return fixedNow },
		RunTool: p.run,
		StraceVersion: func(path string) (string, error) {
			if path != p.strace {
				return "", errors.New("wrong strace path")
			}
			return "strace -- version 6.8", nil
		},
		Kernel: func() (string, error) { return "6.8.0-test", nil },
	}
}

// writable makes a sealed record directory removable again, so the temp
// directory's cleanup does not fail on a read-only tree.
func (p *prep) writable() {
	filepath.Walk(p.dir, func(path string, info os.FileInfo, err error) error {
		if err == nil {
			os.Chmod(path, 0o755)
		}
		return nil
	})
}

func (p *prep) prepare() (*PrepareResult, error) {
	p.t.Helper()
	p.t.Cleanup(p.writable)
	return Prepare(p.options())
}

func (p *prep) mustPrepare() *PrepareResult {
	p.t.Helper()
	res, err := p.prepare()
	if err != nil {
		p.t.Fatalf("Prepare: %v", err)
	}
	return res
}

func readPre(t *testing.T, res *PrepareResult, rel string) []byte {
	t.Helper()
	data, err := os.ReadFile(filepath.Join(res.PreDir, filepath.FromSlash(rel)))
	if err != nil {
		t.Fatal(err)
	}
	return data
}

func sumOf(b []byte) string {
	s := sha256.Sum256(b)
	return hex.EncodeToString(s[:])
}

// ---------------------------------------------------------------------------

func TestPrepareWritesTheSealedRecord(t *testing.T) {
	p := newPrep(t)
	p.commit()
	res := p.mustPrepare()
	if res.Actions != 3 || res.Refused != 0 {
		t.Fatalf("%+v", res)
	}

	// Every file the recfmt decoders know is decoded strictly, and re-encoding
	// what was decoded gives the file's bytes back: the bytes ARE the encoding.
	bindBytes := readPre(t, res, "binding.json")
	b, err := recfmt.DecodeBinding(bindBytes)
	if err != nil {
		t.Fatal(err)
	}
	if again, err := recfmt.EncodeBinding(b); err != nil || !bytes.Equal(again, bindBytes) {
		t.Fatalf("binding is not its own encoding: %v", err)
	}
	if res.BindingID != sumOf(bindBytes) {
		t.Fatal("binding_id is not the digest of the file")
	}
	factsBytes := readPre(t, res, "host-facts.json")
	facts, err := recfmt.DecodeHostFacts(factsBytes)
	if err != nil {
		t.Fatal(err)
	}
	if again, _ := recfmt.EncodeHostFacts(facts); !bytes.Equal(again, factsBytes) {
		t.Fatal("host facts are not their own encoding")
	}
	invBytes := readPre(t, res, "inventory.json")
	inv, err := recfmt.DecodeInventory(invBytes)
	if err != nil {
		t.Fatal(err)
	}
	if again, _ := recfmt.EncodeInventory(inv); !bytes.Equal(again, invBytes) {
		t.Fatal("inventory is not its own encoding")
	}

	// The binding says what it was made of.
	want := recfmt.Binding{
		Schema: recfmt.SchemaBinding, Repository: "owner/repo", Run: "43117337685", Attempt: 2, Revision: p.sha,
		ShardIndex: 0, ShardCount: 2,
		GraphSHA256: sumOf(readPre(t, res, "graph.json")), StepsSHA256: sumOf(readPre(t, res, "steps.json")),
		HostFactsSHA256: sumOf(factsBytes), InventorySHA256: sumOf(invBytes),
		SandboxConfigSHA256: sumOf(readPre(t, res, "sandbox.json")),
		KilnSHA256:          sumOf([]byte("kiln binary")), BuildToolSHA256: sumOf([]byte("build tool binary")),
		StraceSHA256: sumOf([]byte("strace")), GitSHA256: p.gitSum,
		Recorder: legacy.RecorderVersion, StraceVersion: "strace -- version 6.8", Kernel: "6.8.0-test",
		JobStart: 1794000000, PreparedAt: "2026-11-13T02:03:04Z",
	}
	part, err := ReadPartition(filepath.Join(res.PreDir, "build-plan.json"), 2)
	if err != nil {
		t.Fatal(err)
	}
	want.PartitionSHA256, _ = part.Digest()
	if b != want {
		t.Fatalf("binding:\n got %+v\nwant %+v", b, want)
	}

	// Host facts: the machine, and the scrubbed environment WITHOUT PATH and
	// without anything env.json does not name.
	if facts.Image != "github/linux/amd64/ubuntu24/20260101.1" || facts.Root != p.root || facts.Home != p.home || facts.Tmpdir != p.tmpdir {
		t.Fatalf("%+v", facts)
	}
	if !equalStrings(facts.ExecutorArgs, legacy.RecordExecutorArgs) {
		t.Fatalf("%v", facts.ExecutorArgs)
	}
	wantEnv := []string{"FIXTURE_A=1", "HOME=" + p.home, "LANG=C.UTF-8", "TMPDIR=" + p.tmpdir}
	if !equalStrings(facts.Env, wantEnv) {
		t.Fatalf("env %v, want %v", facts.Env, wantEnv)
	}
	for _, e := range facts.Env {
		if strings.Contains(e, "TOKEN") || strings.HasPrefix(e, "PATH=") || strings.HasPrefix(e, "OTHER=") {
			t.Fatalf("a variable that must not travel did: %s", e)
		}
	}

	// G4: from the facts and the tree alone, the merge's recomputation gives
	// every weak key the shard recorded.
	g, err := legacy.ReadGraphFile(filepath.Join(res.PreDir, "graph.json"))
	if err != nil {
		t.Fatal(err)
	}
	arch, _ := legacy.HostArch()
	tr, err := legacy.NewTraced(g, legacy.Options{Source: source.NewGit(p.root), Host: facts, OS: "linux", Arch: arch, Environ: facts.Environ()}, p.root, facts.ExecutorArgs)
	if err != nil {
		t.Fatal(err)
	}
	cfg, err := sandbox.ParseConfig(readPre(t, res, "sandbox.json"))
	if err != nil {
		t.Fatal(err)
	}
	for _, rel := range []string{"code/packages/x/a", "code/packages/x/b", "code/packages/x/z"} {
		raw := readPre(t, res, ActionFile(rel))
		a, err := DecodeAction(raw)
		if err != nil {
			t.Fatal(err)
		}
		wk, err := tr.WeakKey(rel)
		if err != nil {
			t.Fatal(err)
		}
		if a.Action != rel || a.Language != "x" || a.Refusal != "" || a.Weak != hex.EncodeToString(wk.Key[:]) {
			t.Fatalf("%+v", a)
		}
		env, err := cfg.EnvFor(rel)
		if err != nil || !equalStrings(env, wk.Environ()) {
			t.Fatalf("env_by_action[%s] = %v (%v), want %v", rel, env, err, wk.Environ())
		}
	}
	a, _ := DecodeAction(readPre(t, res, ActionFile("code/packages/x/b")))
	if !equalStrings(a.Steps, []string{"echo b1", "echo b2"}) {
		t.Fatalf("%v", a.Steps)
	}
	// Shard 1's package list is not this shard's: x/a and x/z are assigned to
	// shard 0, and shard 0 also runs x/b (a prerequisite copy).
	if len(cfg.EnvByAction) != 3 {
		t.Fatalf("%v", cfg.Actions())
	}

	// The configuration: the checkout, .git masked, fresh home and tmpdir, and
	// roots that exist.
	if cfg.RepoRoot != p.root || cfg.Home != p.home || cfg.Tmpdir != p.tmpdir || cfg.KilnExe != p.kiln ||
		!equalStrings(cfg.Masks, []string{p.root + "/.git"}) || len(cfg.RoRoots) != 2 || len(cfg.StateRoots) != 0 {
		t.Fatalf("%+v", cfg)
	}

	// The wrapper contract.
	var wrapper []string
	if err := json.Unmarshal(readPre(t, res, "exec-wrapper.json"), &wrapper); err != nil {
		t.Fatal(err)
	}
	if !equalStrings(wrapper, []string{p.kiln, "record", "step", "--dir", p.dir, "--"}) || !equalStrings(res.ExecWrapper, wrapper) {
		t.Fatalf("%v", wrapper)
	}

	// The build tool ran twice, from outside the tree, with the arguments of
	// the design, and its environment carried no credential.
	if len(p.toolCalls) != 2 {
		t.Fatalf("%v", p.toolCalls)
	}
	for _, c := range p.toolCalls {
		if c[0] != p.bt || c[1] != "-root" || c[2] != p.root {
			t.Fatalf("%v", c)
		}
	}
	if !strings.Contains(strings.Join(p.toolCalls[0], " "), "-emit-kiln-graph "+filepath.Join(res.PreDir, "graph.json")+" -emit-kiln-steps "+filepath.Join(res.PreDir, "steps.json")) {
		t.Fatalf("%v", p.toolCalls[0])
	}
	if !strings.Contains(strings.Join(p.toolCalls[1], " "), "-force -language all -shard-count 2 -shard-weights "+ShardWeightsPath+" -emit-plan "+filepath.Join(res.PreDir, "build-plan.json")) {
		t.Fatalf("%v", p.toolCalls[1])
	}

	// SEALED names every other file with its digest, and the tree is read-only.
	sealed := string(readPre(t, res, "SEALED"))
	var sealedNames []string
	for _, line := range strings.Split(strings.TrimSuffix(sealed, "\n"), "\n") {
		sum, name, ok := strings.Cut(line, "  ")
		if !ok {
			t.Fatalf("%q", line)
		}
		sealedNames = append(sealedNames, name)
		if got := sumOf(readPre(t, res, name)); got != sum {
			t.Errorf("SEALED digest of %s is wrong", name)
		}
	}
	var onDisk []string
	filepath.Walk(res.PreDir, func(path string, info os.FileInfo, err error) error {
		if err == nil && !info.IsDir() && filepath.Base(path) != "SEALED" {
			rel, _ := filepath.Rel(res.PreDir, path)
			onDisk = append(onDisk, filepath.ToSlash(rel))
		}
		if err == nil && info.IsDir() && info.Mode().Perm() != 0o555 {
			t.Errorf("%s is %v, want 0555", path, info.Mode().Perm())
		}
		if err == nil && !info.IsDir() && info.Mode().Perm() != 0o444 {
			t.Errorf("%s is %v, want 0444", path, info.Mode().Perm())
		}
		return nil
	})
	sort.Strings(onDisk)
	if !equalStrings(sealedNames, onDisk) {
		t.Fatalf("SEALED lists %v, disk has %v", sealedNames, onDisk)
	}
	if st, _ := os.Stat(p.dir); st.Mode().Perm() != 0o700 {
		t.Fatalf("--dir is %v", st.Mode().Perm())
	}
	if _, err := os.Stat(p.tmpdir); err != nil {
		t.Fatalf("the step's tmpdir was not created: %v", err)
	}
	if err := os.WriteFile(filepath.Join(res.PreDir, "binding.json"), []byte("x"), 0o600); err == nil && os.Geteuid() != 0 {
		t.Fatal("pre/ is writable after sealing")
	}
}

func TestPrepareIsDeterministic(t *testing.T) {
	p := newPrep(t)
	p.commit()
	snapshot := func() map[string]string {
		out := map[string]string{}
		filepath.Walk(filepath.Join(p.dir, "pre"), func(path string, info os.FileInfo, err error) error {
			if err == nil && !info.IsDir() {
				data, _ := os.ReadFile(path)
				rel, _ := filepath.Rel(p.dir, path)
				out[rel] = string(data)
			}
			return nil
		})
		return out
	}
	p.mustPrepare()
	first := snapshot()
	p.writable()
	if err := os.RemoveAll(p.dir); err != nil {
		t.Fatal(err)
	}
	if err := os.RemoveAll(p.tmpdir); err != nil {
		t.Fatal(err)
	}
	p.mustPrepare()
	second := snapshot()
	if len(first) != len(second) || len(first) < 8 {
		t.Fatalf("%d vs %d files", len(first), len(second))
	}
	for name, data := range first {
		if second[name] != data {
			t.Errorf("%s differs between two runs", name)
		}
	}
}

func TestPrepareRecordsRefusedPackagesToo(t *testing.T) {
	p := newPrep(t)
	// x/z needs a toolchain entry that does not exist: its weak key is refused,
	// but it still runs, so it still gets an environment and an action file.
	p.files["code/kiln/legacy/toolchains.json"] = fixtureToolchains
	p.commit()
	res, err := func() (*PrepareResult, error) {
		opts := p.options()
		opts.RunTool = func(r ToolRun) error {
			if err := p.run(r); err != nil {
				return err
			}
			for i, a := range r.Args {
				if a == "-emit-kiln-graph" {
					// Re-write the graph with x/z in a language nobody pinned.
					data := bytes.Replace(p.graphJSON(), []byte(`"rel_path":"code/packages/x/z","language":"x"`), []byte(`"rel_path":"code/packages/x/z","language":"nonesuch"`), 1)
					return os.WriteFile(r.Args[i+1], data, 0o644)
				}
			}
			return nil
		}
		t.Cleanup(p.writable)
		return Prepare(opts)
	}()
	if err != nil {
		t.Fatal(err)
	}
	if res.Refused != 1 {
		t.Fatalf("%+v", res)
	}
	a, err := DecodeAction(readPre(t, res, ActionFile("code/packages/x/z")))
	if err != nil || a.Weak != "" || a.Refusal != "unknown-toolchain:nonesuch" {
		t.Fatalf("%+v %v", a, err)
	}
	cfg, _ := sandbox.ParseConfig(readPre(t, res, "sandbox.json"))
	if _, err := cfg.EnvFor("code/packages/x/z"); err != nil {
		t.Fatal(err)
	}
}

func TestPrepareWithoutActionsStillSeals(t *testing.T) {
	// A shard that runs nothing is legal for the plan; it fails at the
	// sandbox's own validation (env_by_action may not be empty), loudly.
	p := newPrep(t)
	p.commit()
	plan = strings.Replace(fixturePlan, `"package_names": ["x/b"]`, `"package_names": []`, 1)
	plan = strings.Replace(plan, `"assigned_packages": ["x/b"]`, `"assigned_packages": []`, 1)
	defer func() { plan = fixturePlan }()
	opts := p.options()
	opts.ShardIndex = 1
	t.Cleanup(p.writable)
	if _, err := Prepare(opts); err == nil || !strings.Contains(err.Error(), "env_by_action is empty") {
		t.Fatal(err)
	}
}

// ---------------------------------------------------------------------------
// Refusals. Each leaves no SEALED file behind, so no later command accepts the
// directory.

func TestPrepareRefusals(t *testing.T) {
	cases := map[string]struct {
		setup  func(p *prep)
		before func(p *prep) // after commit, before Prepare
		opts   func(p *prep, o *Options)
		want   string
	}{
		"the build tool inside the checkout": {before: func(p *prep) {
			writeBytes(p.t, filepath.Join(p.root, "tools", "build-tool"), []byte("bt"), 0o755)
			p.bt = filepath.Join(p.root, "tools", "build-tool")
		}, want: "lies inside the checkout"},
		"the kiln binary inside the checkout": {before: func(p *prep) {
			writeBytes(p.t, filepath.Join(p.root, "kiln"), []byte("k"), 0o755)
			p.kiln = filepath.Join(p.root, "kiln")
		}, want: "lies inside the checkout"},
		"the kiln binary inside tmpdir": {before: func(p *prep) {
			writeBytes(p.t, filepath.Join(p.tmpdir, "kiln"), []byte("k"), 0o755)
			p.kiln = filepath.Join(p.tmpdir, "kiln")
		}, want: "lies inside tmpdir"},
		"the record directory inside the checkout": {before: func(p *prep) {
			p.dir = filepath.Join(p.root, "record")
		}, want: "lies inside the checkout"},
		"a build tool that is a symlink": {before: func(p *prep) {
			link := filepath.Join(p.base, "bin", "bt-link")
			if err := os.Symlink(p.bt, link); err != nil {
				p.t.Fatal(err)
			}
			p.bt = link
		}, want: "not canonical"},
		"a symlinked --dir": {before: func(p *prep) {
			real := filepath.Join(p.base, "real-record")
			if err := os.Mkdir(real, 0o700); err != nil {
				p.t.Fatal(err)
			}
			if err := os.Symlink(real, p.dir); err != nil {
				p.t.Fatal(err)
			}
		}, want: "symlink"},
		"a --dir with content": {before: func(p *prep) {
			writeBytes(p.t, filepath.Join(p.dir, "stale"), []byte("x"), 0o600)
		}, want: "not empty"},
		"a dirty tree": {before: func(p *prep) {
			writeBytes(p.t, filepath.Join(p.root, "code/packages/x/a/src.txt"), []byte("changed\n"), 0o644)
		}, want: "verify tree"},
		"a staged edit": {before: func(p *prep) {
			writeBytes(p.t, filepath.Join(p.root, "code/packages/x/a/src.txt"), []byte("staged\n"), 0o644)
			runGit(p.t, p.root, "add", "-A")
		}, want: "verify tree"},
		"an untracked BUILD_linux": {before: func(p *prep) {
			writeBytes(p.t, filepath.Join(p.root, "code/packages/x/a/BUILD_linux"), []byte("echo hijack\n"), 0o644)
		}, want: "BUILD_linux"},
		"an untracked package": {before: func(p *prep) {
			writeBytes(p.t, filepath.Join(p.root, "code/packages/x/new/BUILD"), []byte("echo new\n"), 0o644)
		}, want: "code/packages/x/new/BUILD"},
		"a hard-linked tracked file": {before: func(p *prep) {
			if err := os.Link(filepath.Join(p.root, "code/packages/x/a/src.txt"), filepath.Join(p.root, "code/packages/x/a/alias.txt")); err != nil {
				p.t.Fatal(err)
			}
		}, want: "a hard link"},
		"GITHUB_SHA is another commit": {before: func(p *prep) {
			p.getenv["GITHUB_SHA"] = strings.Repeat("a", 40)
		}, want: "GITHUB_SHA"},
		"no run id":       {before: func(p *prep) { delete(p.getenv, "GITHUB_RUN_ID") }, want: "GITHUB_RUN_ID"},
		"attempt zero":    {before: func(p *prep) { p.getenv["GITHUB_RUN_ATTEMPT"] = "0" }, want: "GITHUB_RUN_ATTEMPT"},
		"no HOME":         {before: func(p *prep) { p.env = []string{"FIXTURE_A=1"} }, want: "HOME"},
		"a relative HOME": {before: func(p *prep) { p.env = []string{"HOME=home"} }, want: "HOME"},
		"a credential-like value in an allowed variable": {before: func(p *prep) {
			p.env = append(p.env, "FIXTURE_B=ghp_abcdef")
		}, want: "looks like a credential"},
		"a secret-like name matching env.json": {before: func(p *prep) {
			p.env = append(p.env, "FIXTURE_TOKEN=x")
		}, want: "secret"},
		"the graph export fails": {before: func(p *prep) { p.toolErr["graph"] = errors.New("exit status 1") }, want: "graph and steps export"},
		"the plan export fails":  {before: func(p *prep) { p.toolErr["plan"] = errors.New("exit status 1") }, want: "plan export"},
		"a steps export for another OS": {before: func(p *prep) {
			p.steps = strings.Replace(string(p.stepsJSON()), `"os":"linux"`, `"os":"windows"`, 1)
		}, want: "not linux"},
		"a malformed steps export":       {before: func(p *prep) { p.steps = `{"schema":"x"}` }, want: "steps"},
		"a plan for another shard count": {opts: func(p *prep, o *Options) { o.ShardCount = 3 }, want: "has 2 shards, want exactly 3"},
		"a shard index out of range":     {opts: func(p *prep, o *Options) { o.ShardIndex = 2 }, want: "out of range"},
		"a negative job start":           {opts: func(p *prep, o *Options) { o.JobStart = -1 }, want: "negative"},
		"a relative root":                {opts: func(p *prep, o *Options) { o.Root = "repo" }, want: "absolute"},
		"a relative strace":              {opts: func(p *prep, o *Options) { o.Strace = "strace" }, want: "absolute"},
		"strace that is missing":         {opts: func(p *prep, o *Options) { o.Strace = filepath.Join(p.base, "bin", "nostrace") }, want: "no such file"},
		"strace inside the checkout": {before: func(p *prep) {
			writeBytes(p.t, filepath.Join(p.root, "strace"), []byte("s"), 0o755)
			p.strace = filepath.Join(p.root, "strace")
		}, want: "lies inside the checkout"},
		"strace that fails": {opts: func(p *prep, o *Options) {
			o.StraceVersion = func(string) (string, error) { return "", errors.New("no strace here") }
		}, want: "no strace here"},
		"a kernel that cannot be read": {opts: func(p *prep, o *Options) {
			o.Kernel = func() (string, error) { return "", errors.New("no /proc") }
		}, want: "no /proc"},
		"a plan package the graph does not know": {before: func(p *prep) {
			plan = strings.Replace(fixturePlan, `"rel_path": "code/packages/x/z"`, `"rel_path": "code/packages/x/ghost"`, 1)
		}, want: "graph export does not know"},
		"a non-empty tmpdir": {before: func(p *prep) {
			writeBytes(p.t, filepath.Join(p.tmpdir, "stale"), []byte("x"), 0o644)
		}, want: "not empty"},
		"a tmpdir that is a symlink": {before: func(p *prep) {
			if err := os.Symlink(p.base, p.tmpdir); err != nil {
				p.t.Fatal(err)
			}
		}, want: "not a real directory"},
		"a read-only root that is RUNNER_TEMP": {before: func(p *prep) {
			p.getenv["RUNNER_TEMP"] = "/bin"
		}, want: "overlaps RUNNER_TEMP"},
		"a read-only root that holds GITHUB_ENV": {before: func(p *prep) {
			p.getenv["GITHUB_ENV"] = "/sbin/envfile"
		}, want: "GITHUB_ENV"},
		"a symlink read-only root beneath home": {setup: func(p *prep) {
			p.roots = []string{`{"path": "$HOME/.xtool", "kind": "tool", "tool": "xtool"}`}
			p.files["code/kiln/legacy/toolchains.json"] = strings.Replace(fixtureToolchains, `"roots": {"xtool": ["/opt/x/bin"]}`, `"roots": {"xtool": ["/opt/x/bin"]}`, 1)
		}, before: func(p *prep) {
			if err := os.Symlink(p.base, filepath.Join(p.home, ".xtool")); err != nil {
				p.t.Fatal(err)
			}
		}, want: "beneath home is a symlink"},
		"a malformed graph export": {before: func(p *prep) { p.graph = `{"schema":"nope"}` }, want: "graph"},
		"a stale graph (it names a BUILD the tree lacks)": {before: func(p *prep) {
			p.graph = strings.Replace(string(p.graphJSON()), `"rel_path":"code/packages/x/z"`, `"rel_path":"code/packages/x/gone"`, 1)
		}, want: "stale graph"},
		"a malformed plan": {before: func(p *prep) { plan = `{` }, want: "plan:"},
		"no host image id": {opts: func(p *prep, o *Options) { o.Image = "" }, want: "image_id"},
		"the build tool's source is missing from the tree": {setup: func(p *prep) {
			delete(p.files, "code/programs/go/build-tool/go.mod")
			delete(p.files, "code/programs/go/build-tool/main.go")
		}, want: "build-tool"},
		"the kiln binary does not exist": {opts: func(p *prep, o *Options) { o.KilnExe = filepath.Join(p.base, "bin", "nokiln") }, want: "no such file"},
		"the build tool does not exist":  {opts: func(p *prep, o *Options) { o.BuildTool = filepath.Join(p.base, "bin", "nobt") }, want: "no such file"},
		"a root that is a symlink": {before: func(p *prep) {
			link := filepath.Join(p.base, "root-link")
			if err := os.Symlink(p.root, link); err != nil {
				p.t.Fatal(err)
			}
			p.root = link
		}, want: "not canonical"},
		"an unreadable directory in the tree": {before: func(p *prep) {
			if os.Geteuid() == 0 {
				p.t.Skip("root reads everything")
			}
			d := filepath.Join(p.root, "locked")
			if err := os.Mkdir(d, 0o755); err != nil {
				p.t.Fatal(err)
			}
			if err := os.Chmod(d, 0); err != nil {
				p.t.Fatal(err)
			}
			p.t.Cleanup(func() { os.Chmod(d, 0o755) })
		}, want: "permission denied"},
		"the record directory inside a state root (which then is not empty)": {setup: func(p *prep) {
			p.roots = []string{`{"path": "$HOME/.state", "kind": "state", "ecosystem": "x"}`}
		}, before: func(p *prep) {
			if err := os.Mkdir(filepath.Join(p.home, ".state"), 0o755); err != nil {
				p.t.Fatal(err)
			}
			p.dir = filepath.Join(p.home, ".state", "rec")
		}, want: "state-preloaded"},
		"a preloaded state root": {setup: func(p *prep) {
			p.roots = []string{`{"path": "$HOME/.state", "kind": "state", "ecosystem": "x"}`}
		}, before: func(p *prep) {
			writeBytes(p.t, filepath.Join(p.home, ".state", "cached"), []byte("x"), 0o644)
		}, want: "state-preloaded"},
	}
	for name, c := range cases {
		t.Run(name, func(t *testing.T) {
			plan = fixturePlan
			defer func() { plan = fixturePlan }()
			p := newPrep(t)
			if c.setup != nil {
				c.setup(p)
			}
			p.commit()
			if c.before != nil {
				c.before(p)
			}
			o := p.options()
			if c.opts != nil {
				c.opts(p, &o)
			}
			t.Cleanup(p.writable)
			_, err := Prepare(o)
			if err == nil || !strings.Contains(err.Error(), c.want) {
				t.Fatalf("got %v, want %q", err, c.want)
			}
			if _, statErr := os.Lstat(filepath.Join(o.Dir, "pre", "SEALED")); statErr == nil {
				t.Fatal("a refused prepare left a SEALED record")
			}
		})
	}
}

// A socket or FIFO in a read-only root voids the shard. The root must lie
// outside /tmp (the sandbox refuses a root there), so this one is made under
// the user's real home directory.
func TestPrepareRefusesASocketOrFifoInARoRoot(t *testing.T) {
	if realHome == "" {
		t.Skip("no home directory")
	}
	root, err := os.MkdirTemp(realHome, ".kiln-prep-root-")
	if err != nil {
		t.Skipf("cannot make a directory under the home directory: %v", err)
	}
	defer os.RemoveAll(root)
	if root, err = filepath.EvalSymlinks(root); err != nil {
		t.Fatal(err)
	}
	if err := syscall.Mkfifo(filepath.Join(root, "pipe"), 0o600); err != nil {
		t.Fatal(err)
	}
	p := newPrep(t)
	p.roots = []string{fmt.Sprintf(`{"path": %q, "kind": "image"}`, root)}
	p.commit()
	// Home is the fixture's own, so the root is not beneath it (and M2 is
	// not in play).
	t.Cleanup(p.writable)
	_, err = Prepare(p.options())
	if err == nil || !strings.Contains(err.Error(), "host scan") || !strings.Contains(err.Error(), "pipe") {
		t.Fatalf("%v", err)
	}
}

func TestPrepareARoRootContainingTheRecordDirectoryIsRefused(t *testing.T) {
	if realHome == "" {
		t.Skip("no home directory")
	}
	root, err := os.MkdirTemp(realHome, ".kiln-prep-root-")
	if err != nil {
		t.Skipf("cannot make a directory under the home directory: %v", err)
	}
	defer os.RemoveAll(root)
	if root, err = filepath.EvalSymlinks(root); err != nil {
		t.Fatal(err)
	}
	p := newPrep(t)
	p.roots = []string{fmt.Sprintf(`{"path": %q, "kind": "image"}`, root)}
	p.commit()
	p.dir = filepath.Join(root, "record")
	t.Cleanup(p.writable)
	_, err = Prepare(p.options())
	if err == nil || !strings.Contains(err.Error(), "overlaps the record directory") {
		t.Fatalf("%v", err)
	}
}

func TestPrepareWorksWithTheDefaultTmpdirOption(t *testing.T) {
	// Options.Tmpdir defaults to legacy.RecordTmpdir, which a test must not
	// create in the real /tmp: assert the default and stop before any write.
	o := Options{Root: "/r", Dir: "/d", BuildTool: "/b", KilnExe: "/k", Strace: "/s", Git: "/g", ShardCount: 1}
	if err := o.normalize(); err != nil || o.Tmpdir != legacy.RecordTmpdir || o.RunTool == nil || o.StraceVersion == nil || o.Kernel == nil || o.Now == nil || o.Getenv == nil {
		t.Fatalf("%+v %v", o, err)
	}
}

func equalStrings(a, b []string) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

// P3 (decided): the configuration may lie beneath home, which is a fresh tmpfs
// in the sandbox; on a hosted runner RUNNER_TEMP is beneath home.
func TestPrepareAcceptsARecordDirectoryBeneathHome(t *testing.T) {
	p := newPrep(t)
	p.commit()
	p.env = []string{"HOME=" + p.base, "FIXTURE_A=1", "PATH=/usr/bin:/bin"}
	p.mustPrepare()
}
