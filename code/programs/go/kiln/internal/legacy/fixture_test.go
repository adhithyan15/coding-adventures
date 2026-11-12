package legacy

import (
	"encoding/json"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// imager is a fixed host image.
type imager string

func (i imager) ImageID() string { return string(i) }

// fixture is a throw-away repository tree on disk.
type fixture struct {
	t   *testing.T
	dir string
}

const baseBoundary = `{
  "schema_version": 1,
  "language_source_input_registry_sha256": "00",
  "boundaries": [
    {"id": "one", "input_origin": "x",
     "applies_to": {"exact_roots": ["code/packages/x/a"], "descendant_roots": [], "excluded_roots": []},
     "inputs": [{"path": "code/shared/cfg.json", "role": "cross_package_exact"}],
     "reason": "r", "owner": "o"},
    {"id": "two", "input_origin": "x",
     "applies_to": {"exact_roots": [], "descendant_roots": ["code/packages/y"], "excluded_roots": ["code/packages/y/skip"]},
     "inputs": [{"path": "code/shared/y.json", "role": "shared_ancestor", "generated_component": "g"}],
     "reason": "r", "owner": "o"}
  ]
}
`

// newFixture writes the baseline tree: the policy files, the boundary file, a
// build tool (with a replaced in-repo module) and the packages of baseGraph.
//
//	x/a      deps x/b            dev_deps x/t
//	x/b      deps x/c            lives INSIDE x/outer (x/outer is its enclosing package)
//	x/c      has nested x/c-n    repo_inputs code/grammars/g (a directory)
//	x/t      dev_deps x/u        (a dev-dep's own dev-dep: NOT in a's closure)
//	x/z      unrelated
func newFixture(t *testing.T) *fixture {
	f := &fixture{t: t, dir: t.TempDir()}
	f.write("code/kiln/legacy/pinned.json", `{"languages": []}`)
	f.write("code/kiln/legacy/never.json", `{"packages": []}`)
	f.write(BoundaryPath, baseBoundary)
	f.write(BuildToolPath+"/go.mod", "module example.com/bt\n\ngo 1.26\n\nreplace example.com/lib => ../../../packages/go/lib // in repo\n\nreplace example.com/far => ../../../../../elsewhere\n")
	f.write(BuildToolPath+"/main.go", "package main\n")
	f.write("code/packages/go/lib/lib.go", "package lib\n")
	f.write("go.work", "go 1.26\n")
	f.write("mise.toml", "[tools]\n")
	f.write("README.md", "docs\n")
	f.write("code/packages/x/.npmrc", "a=b\n")
	f.write("code/packages/x/README.md", "docs\n")
	f.write("code/shared/cfg.json", "{}")
	f.write("code/shared/y.json", "{}")
	f.write("code/grammars/g/one.txt", "one")
	f.write("code/grammars/g/sub/two.txt", "two")
	f.write("code/lang/x.mll", "x")
	f.write("code/lang/.mll", "dot")
	for _, p := range []string{"a", "outer", "outer/b", "c", "c/ext/n", "t", "u", "z", "q"} {
		f.write("code/packages/x/"+p+"/BUILD", "echo "+p+"\n")
		f.write("code/packages/x/"+p+"/src.txt", p)
	}
	f.write("code/packages/x/a/BUILD_windows", "echo win\n")
	f.write("code/packages/x/outer/Cargo.toml", "[workspace]\n")
	return f
}

func (f *fixture) write(rel, content string) {
	f.t.Helper()
	full := filepath.Join(f.dir, filepath.FromSlash(rel))
	if err := os.MkdirAll(filepath.Dir(full), 0o755); err != nil {
		f.t.Fatal(err)
	}
	if err := os.WriteFile(full, []byte(content), 0o644); err != nil {
		f.t.Fatal(err)
	}
}

func (f *fixture) remove(rel string) {
	f.t.Helper()
	if err := os.Remove(filepath.Join(f.dir, filepath.FromSlash(rel))); err != nil {
		f.t.Fatal(err)
	}
}

// mkpkg builds a graph package whose three OSes share deps and BUILD file.
func mkpkg(name, rel, lang string, deps ...string) Package {
	b := "BUILD"
	return Package{
		Name: name, RelPath: rel, Language: lang,
		BuildFiles: map[string]*string{"linux": &b, "darwin": &b, "windows": &b},
		Deps:       map[string][]string{"linux": deps, "darwin": deps, "windows": deps},
	}
}

// baseGraph is the graph the fixture's comments describe, sorted by name.
func baseGraph() *Graph {
	a := mkpkg("x/a", "code/packages/x/a", "x", "x/b")
	a.DevDeps = []string{"x/t"}
	b := mkpkg("x/b", "code/packages/x/outer/b", "x", "x/c")
	b.RepoInputGlobs = []string{"code/**/?*.mll"}
	c := mkpkg("x/c", "code/packages/x/c", "x")
	c.RepoInputs = []string{"code/grammars/g"}
	cn := mkpkg("x/c-n", "code/packages/x/c/ext/n", "x")
	outer := mkpkg("x/outer", "code/packages/x/outer", "x")
	tt := mkpkg("x/t", "code/packages/x/t", "x")
	tt.DevDeps = []string{"x/u"}
	u := mkpkg("x/u", "code/packages/x/u", "x")
	z := mkpkg("x/z", "code/packages/x/z", "x")
	// Sorted: x/a x/b x/c x/c-n x/outer x/t x/u x/z
	return &Graph{Schema: GraphSchema, Packages: []Package{a, b, c, cn, outer, tt, u, z}}
}

func (f *fixture) opts(osName string) Options {
	return Options{Source: source.NewFolder(f.dir), Host: imager("test/image"), OS: osName, Arch: "x64", Workers: 3}
}

func (f *fixture) plan(g *Graph) *Plan {
	f.t.Helper()
	p, err := BuildPlan(g, f.opts("linux"))
	if err != nil {
		f.t.Fatalf("BuildPlan: %v", err)
	}
	return p
}

func (p *Plan) rec(t *testing.T, name string) Record {
	t.Helper()
	r, ok := p.Find(name)
	if !ok {
		t.Fatalf("no record %s", name)
	}
	return r
}

func hasGit(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("git"); err != nil {
		t.Skip("git not installed")
	}
}

// gitInit commits everything under dir.
func gitInit(t *testing.T, dir string) {
	t.Helper()
	hasGit(t)
	for _, args := range [][]string{
		{"init", "-q"}, {"config", "user.email", "k@example.com"}, {"config", "user.name", "k"},
		{"config", "commit.gpgsign", "false"}, {"add", "-A"}, {"commit", "-q", "-m", "x"},
	} {
		if out, err := exec.Command("git", append([]string{"-C", dir}, args...)...).CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v: %s", args, err, out)
		}
	}
}

func mustJSON(t *testing.T, v any) []byte {
	t.Helper()
	b, err := json.Marshal(v)
	if err != nil {
		t.Fatal(err)
	}
	return b
}

func contains(xs []string, want string) bool {
	for _, x := range xs {
		if x == want {
			return true
		}
	}
	return false
}

func hasPrefixReason(xs []string, prefix string) bool {
	for _, x := range xs {
		if strings.HasPrefix(x, prefix) {
			return true
		}
	}
	return false
}

var _ = io.EOF

// planner builds the planner for the baseline graph without hashing.
func (f *fixture) planner() (*planner, error) {
	pl, err := newPlanner(baseGraph(), f.opts("linux"))
	return pl, err
}
