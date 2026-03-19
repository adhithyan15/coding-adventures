package resolver

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// makeFixture creates a temporary directory tree and returns its root path.
func makeFixture(t *testing.T, tree map[string]string) string {
	t.Helper()
	root := t.TempDir()
	for relPath, content := range tree {
		absPath := filepath.Join(root, filepath.FromSlash(relPath))
		if err := os.MkdirAll(filepath.Dir(absPath), 0755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(absPath, []byte(content), 0644); err != nil {
			t.Fatal(err)
		}
	}
	return root
}

// ---------------------------------------------------------------------------
// Tests for Python dependency parsing
// ---------------------------------------------------------------------------

func TestParsePythonDepsInline(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/pyproject.toml": `[project]
name = "coding-adventures-pkg-a"
version = "0.1.0"
dependencies = ["coding-adventures-pkg-b", "coding-adventures-pkg-c"]
`,
		"pkg-b/pyproject.toml": `[project]
name = "coding-adventures-pkg-b"
version = "0.1.0"
dependencies = []
`,
		"pkg-c/pyproject.toml": `[project]
name = "coding-adventures-pkg-c"
version = "0.1.0"
dependencies = []
`,
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), Language: "python"},
		{Name: "python/pkg-b", Path: filepath.Join(root, "pkg-b"), Language: "python"},
		{Name: "python/pkg-c", Path: filepath.Join(root, "pkg-c"), Language: "python"},
	}

	known := BuildKnownNames(packages)
	deps := parsePythonDeps(packages[0], known)

	if len(deps) != 2 {
		t.Fatalf("expected 2 deps, got %d: %v", len(deps), deps)
	}
}

func TestParsePythonDepsMultiline(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/pyproject.toml": `[project]
name = "coding-adventures-pkg-a"
version = "0.1.0"
dependencies = [
    "coding-adventures-pkg-b>=0.1.0",
    "coding-adventures-pkg-c",
]
`,
		"pkg-b/pyproject.toml": `[project]
name = "coding-adventures-pkg-b"
`,
		"pkg-c/pyproject.toml": `[project]
name = "coding-adventures-pkg-c"
`,
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), Language: "python"},
		{Name: "python/pkg-b", Path: filepath.Join(root, "pkg-b"), Language: "python"},
		{Name: "python/pkg-c", Path: filepath.Join(root, "pkg-c"), Language: "python"},
	}

	known := BuildKnownNames(packages)
	deps := parsePythonDeps(packages[0], known)

	if len(deps) != 2 {
		t.Fatalf("expected 2 deps, got %d: %v", len(deps), deps)
	}
}

func TestParsePythonDepsNoPyproject(t *testing.T) {
	root := t.TempDir()
	pkg := discovery.Package{Name: "python/pkg-x", Path: root, Language: "python"}
	deps := parsePythonDeps(pkg, map[string]string{})
	if len(deps) != 0 {
		t.Fatalf("expected 0 deps, got %d", len(deps))
	}
}

func TestParsePythonDepsExternalSkipped(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/pyproject.toml": `[project]
name = "coding-adventures-pkg-a"
dependencies = ["requests", "flask"]
`,
	})

	pkg := discovery.Package{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), Language: "python"}
	deps := parsePythonDeps(pkg, map[string]string{})
	if len(deps) != 0 {
		t.Fatalf("expected external deps to be skipped, got %d", len(deps))
	}
}

// ---------------------------------------------------------------------------
// Tests for Ruby dependency parsing
// ---------------------------------------------------------------------------

func TestParseRubyDeps(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"lib-foo/lib_foo.gemspec": `Gem::Specification.new do |spec|
  spec.name = "coding_adventures_lib_foo"
  spec.add_dependency "coding_adventures_lib_bar"
  spec.add_dependency "coding_adventures_lib_baz"
end
`,
	})

	packages := []discovery.Package{
		{Name: "ruby/lib-foo", Path: filepath.Join(root, "lib-foo"), Language: "ruby"},
		{Name: "ruby/lib_bar", Path: filepath.Join(root, "lib_bar"), Language: "ruby"},
		{Name: "ruby/lib_baz", Path: filepath.Join(root, "lib_baz"), Language: "ruby"},
	}

	known := BuildKnownNames(packages)
	deps := parseRubyDeps(packages[0], known)

	if len(deps) != 2 {
		t.Fatalf("expected 2 deps, got %d: %v", len(deps), deps)
	}
}

func TestParseRubyDepsNoGemspec(t *testing.T) {
	root := t.TempDir()
	pkg := discovery.Package{Name: "ruby/lib-x", Path: root, Language: "ruby"}
	deps := parseRubyDeps(pkg, map[string]string{})
	if len(deps) != 0 {
		t.Fatalf("expected 0 deps, got %d", len(deps))
	}
}

// ---------------------------------------------------------------------------
// Tests for Go dependency parsing
// ---------------------------------------------------------------------------

func TestParseGoDepsRequireBlock(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"app/go.mod": `module github.com/adhithyan15/coding-adventures/code/programs/go/app

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/other-lib v0.0.0
)
`,
		"directed-graph/go.mod": `module github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph

go 1.26
`,
		"other-lib/go.mod": `module github.com/adhithyan15/coding-adventures/code/packages/go/other-lib

go 1.26
`,
	})

	packages := []discovery.Package{
		{Name: "go/app", Path: filepath.Join(root, "app"), Language: "go"},
		{Name: "go/directed-graph", Path: filepath.Join(root, "directed-graph"), Language: "go"},
		{Name: "go/other-lib", Path: filepath.Join(root, "other-lib"), Language: "go"},
	}

	known := BuildKnownNames(packages)
	deps := parseGoDeps(packages[0], known)

	if len(deps) != 2 {
		t.Fatalf("expected 2 deps, got %d: %v", len(deps), deps)
	}
}

func TestParseGoDepsNoGoMod(t *testing.T) {
	root := t.TempDir()
	pkg := discovery.Package{Name: "go/lib-x", Path: root, Language: "go"}
	deps := parseGoDeps(pkg, map[string]string{})
	if len(deps) != 0 {
		t.Fatalf("expected 0 deps, got %d", len(deps))
	}
}

// ---------------------------------------------------------------------------
// Tests for ResolveDependencies
// ---------------------------------------------------------------------------

func TestResolveDepsNoDeps(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/pyproject.toml": `[project]
name = "coding-adventures-pkg-a"
dependencies = []
`,
		"pkg-b/pyproject.toml": `[project]
name = "coding-adventures-pkg-b"
dependencies = []
`,
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), Language: "python"},
		{Name: "python/pkg-b", Path: filepath.Join(root, "pkg-b"), Language: "python"},
	}

	graph := ResolveDependencies(packages)

	// Both nodes should exist.
	if !graph.HasNode("python/pkg-a") || !graph.HasNode("python/pkg-b") {
		t.Fatal("expected both nodes in graph")
	}

	// No edges.
	edges := graph.Edges()
	if len(edges) != 0 {
		t.Fatalf("expected 0 edges, got %d", len(edges))
	}
}

func TestResolveDepsWithDeps(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/pyproject.toml": `[project]
name = "coding-adventures-pkg-a"
dependencies = ["coding-adventures-pkg-b"]
`,
		"pkg-b/pyproject.toml": `[project]
name = "coding-adventures-pkg-b"
dependencies = []
`,
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), Language: "python"},
		{Name: "python/pkg-b", Path: filepath.Join(root, "pkg-b"), Language: "python"},
	}

	graph := ResolveDependencies(packages)

	// Edge: pkg-b → pkg-a (pkg-a depends on pkg-b).
	if !graph.HasEdge("python/pkg-b", "python/pkg-a") {
		t.Fatal("expected edge python/pkg-b → python/pkg-a")
	}
}

func TestResolveDiamondDeps(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/pyproject.toml": `[project]
name = "coding-adventures-pkg-a"
dependencies = ["coding-adventures-pkg-b", "coding-adventures-pkg-c"]
`,
		"pkg-b/pyproject.toml": `[project]
name = "coding-adventures-pkg-b"
dependencies = ["coding-adventures-pkg-d"]
`,
		"pkg-c/pyproject.toml": `[project]
name = "coding-adventures-pkg-c"
dependencies = ["coding-adventures-pkg-d"]
`,
		"pkg-d/pyproject.toml": `[project]
name = "coding-adventures-pkg-d"
dependencies = []
`,
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), Language: "python"},
		{Name: "python/pkg-b", Path: filepath.Join(root, "pkg-b"), Language: "python"},
		{Name: "python/pkg-c", Path: filepath.Join(root, "pkg-c"), Language: "python"},
		{Name: "python/pkg-d", Path: filepath.Join(root, "pkg-d"), Language: "python"},
	}

	graph := ResolveDependencies(packages)

	// Verify edge count: d→b, d→c, b→a, c→a = 4 edges.
	edges := graph.Edges()
	if len(edges) != 4 {
		t.Fatalf("expected 4 edges in diamond, got %d: %v", len(edges), edges)
	}

	// Verify independent groups.
	groups, err := graph.IndependentGroups()
	if err != nil {
		t.Fatal(err)
	}
	if len(groups) != 3 {
		t.Fatalf("expected 3 levels, got %d", len(groups))
	}
}

func TestBuildKnownNamesPython(t *testing.T) {
	packages := []discovery.Package{
		{Name: "python/logic-gates", Path: "/repo/packages/python/logic-gates", Language: "python"},
	}
	known := BuildKnownNames(packages)
	if known["coding-adventures-logic-gates"] != "python/logic-gates" {
		t.Fatalf("expected python/logic-gates, got %s", known["coding-adventures-logic-gates"])
	}
}

func TestBuildKnownNamesRuby(t *testing.T) {
	packages := []discovery.Package{
		{Name: "ruby/logic_gates", Path: "/repo/packages/ruby/logic_gates", Language: "ruby"},
	}
	known := BuildKnownNames(packages)
	if known["coding_adventures_logic_gates"] != "ruby/logic_gates" {
		t.Fatalf("expected ruby/logic_gates, got %s", known["coding_adventures_logic_gates"])
	}
}

func TestBuildKnownNamesTypescript(t *testing.T) {
	packages := []discovery.Package{
		{Name: "typescript/logic-gates", Path: "/repo/packages/typescript/logic-gates", Language: "typescript"},
	}
	known := BuildKnownNames(packages)
	if known["@coding-adventures/logic-gates"] != "typescript/logic-gates" {
		t.Fatalf("expected typescript/logic-gates, got %s", known["@coding-adventures/logic-gates"])
	}
}

func TestParseTypescriptDeps(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/package.json": `{
  "name": "@coding-adventures/pkg-a",
  "dependencies": {
    "@coding-adventures/pkg-b": "file:../pkg-b",
    "@coding-adventures/pkg-c": "file:../pkg-c"
  }
}`,
		"pkg-b/package.json": `{
  "name": "@coding-adventures/pkg-b"
}`,
		"pkg-c/package.json": `{
  "name": "@coding-adventures/pkg-c"
}`,
	})

	packages := []discovery.Package{
		{Name: "typescript/pkg-a", Path: filepath.Join(root, "pkg-a"), Language: "typescript"},
		{Name: "typescript/pkg-b", Path: filepath.Join(root, "pkg-b"), Language: "typescript"},
		{Name: "typescript/pkg-c", Path: filepath.Join(root, "pkg-c"), Language: "typescript"},
	}

	known := BuildKnownNames(packages)
	deps := parseTypescriptDeps(packages[0], known)

	if len(deps) != 2 {
		t.Fatalf("expected 2 deps, got %d: %v", len(deps), deps)
	}
}

func TestParseTypescriptDepsNoPackageJSON(t *testing.T) {
	root := t.TempDir()
	pkg := discovery.Package{Name: "typescript/pkg-x", Path: root, Language: "typescript"}
	deps := parseTypescriptDeps(pkg, map[string]string{})
	if len(deps) != 0 {
		t.Fatalf("expected 0 deps, got %d", len(deps))
	}
}

func TestParseTypescriptDepsExternalSkipped(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/package.json": `{
  "name": "@coding-adventures/pkg-a",
  "dependencies": {
    "vitest": "^1.0.0",
    "typescript": "^5.0.0"
  }
}`,
	})

	pkg := discovery.Package{Name: "typescript/pkg-a", Path: filepath.Join(root, "pkg-a"), Language: "typescript"}
	deps := parseTypescriptDeps(pkg, map[string]string{})
	if len(deps) != 0 {
		t.Fatalf("expected external deps to be skipped, got %d", len(deps))
	}
}

func TestBuildKnownNamesGo(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"directed-graph/go.mod": `module github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph

go 1.26
`,
	})
	packages := []discovery.Package{
		{Name: "go/directed-graph", Path: filepath.Join(root, "directed-graph"), Language: "go"},
	}
	known := BuildKnownNames(packages)
	expected := "go/directed-graph"
	if known["github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"] != expected {
		t.Fatalf("expected %s, got %s", expected, known["github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"])
	}
}
