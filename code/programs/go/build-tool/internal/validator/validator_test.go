package validator

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

func makePackages(t *testing.T, defs []struct {
	name     string
	relPath  string
	lang     string
	commands []string
}) []discovery.Package {
	t.Helper()

	root := t.TempDir()
	var pkgs []discovery.Package
	for _, def := range defs {
		abs := filepath.Join(root, filepath.FromSlash(def.relPath))
		if err := os.MkdirAll(abs, 0755); err != nil {
			t.Fatal(err)
		}
		pkgs = append(pkgs, discovery.Package{
			Name:          def.name,
			Path:          abs,
			Language:      def.lang,
			BuildCommands: def.commands,
		})
	}
	return pkgs
}

func graphWithEdges(edges ...[2]string) *directedgraph.Graph {
	g := directedgraph.New()
	for _, edge := range edges {
		g.AddNode(edge[0])
		g.AddNode(edge[1])
		g.AddEdge(edge[0], edge[1])
	}
	return g
}

func TestValidateBuildFilesAllowsMatchingPythonClosure(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{name: "python/a", relPath: "code/packages/python/a", lang: "python"},
		{
			name:    "python/b",
			relPath: "code/packages/python/b",
			lang:    "python",
			commands: []string{
				`uv pip install -e ../a -e ".[dev]" --quiet`,
			},
		},
		{
			name:    "python/c",
			relPath: "code/packages/python/c",
			lang:    "python",
			commands: []string{
				`uv pip install -e ../a -e ../b -e ".[dev]" --quiet`,
			},
		},
	})

	graph := graphWithEdges(
		[2]string{"python/a", "python/b"},
		[2]string{"python/b", "python/c"},
	)

	if err := ValidateBuildFiles(pkgs, graph); err != nil {
		t.Fatalf("expected validation to pass, got %v", err)
	}
}

func TestValidateBuildFilesFailsMissingStandalonePrereq(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{name: "python/a", relPath: "code/packages/python/a", lang: "python"},
		{name: "python/b", relPath: "code/packages/python/b", lang: "python"},
		{
			name:    "python/c",
			relPath: "code/packages/python/c",
			lang:    "python",
			commands: []string{
				`uv pip install -e ../b -e ".[dev]" --quiet`,
			},
		},
	})

	graph := graphWithEdges(
		[2]string{"python/a", "python/b"},
		[2]string{"python/b", "python/c"},
	)

	err := ValidateBuildFiles(pkgs, graph)
	if err == nil {
		t.Fatal("expected validation failure")
	}
	if !strings.Contains(err.Error(), "missing prerequisite refs for standalone builds: python/a") {
		t.Fatalf("expected missing prerequisite message, got %v", err)
	}
}

func TestValidateBuildFilesFailsHiddenReference(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{name: "ruby/a", relPath: "code/packages/ruby/a", lang: "ruby"},
		{name: "ruby/b", relPath: "code/packages/ruby/b", lang: "ruby"},
		{
			name:    "ruby/c",
			relPath: "code/packages/ruby/c", lang: "ruby",
			commands: []string{
				`cd ../a && bundle install --quiet && cd ../c && bundle exec rake test`,
			},
		},
	})

	graph := graphWithEdges(
		[2]string{"ruby/b", "ruby/c"},
	)

	err := ValidateBuildFiles(pkgs, graph)
	if err == nil {
		t.Fatal("expected validation failure")
	}
	if !strings.Contains(err.Error(), "undeclared local package refs: ruby/a") {
		t.Fatalf("expected hidden reference message, got %v", err)
	}
}

func TestValidateBuildFilesIgnoresSelfReference(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{
			name:    "typescript/demo",
			relPath: "code/packages/typescript/demo",
			lang:    "typescript",
			commands: []string{
				`cd ../demo && npm ci && npx vitest run`,
			},
		},
	})

	graph := directedgraph.New()
	graph.AddNode("typescript/demo")

	if err := ValidateBuildFiles(pkgs, graph); err != nil {
		t.Fatalf("expected self reference to be allowed, got %v", err)
	}
}

func TestValidateBuildFilesFailsFullBuildWorkflowWithoutNormalizedToolchains(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{name: "elixir/actor", relPath: "code/packages/elixir/actor", lang: "elixir"},
		{name: "python/actor", relPath: "code/packages/python/actor", lang: "python"},
	})

	repoRoot := inferRepoRoot(pkgs)
	if repoRoot == "" {
		t.Fatal("expected repo root inference to succeed")
	}

	ciPath := filepath.Join(repoRoot, ".github", "workflows")
	if err := os.MkdirAll(ciPath, 0755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(ciPath, "ci.yml"), []byte(`
jobs:
  detect:
    outputs:
      needs_python: ${{ steps.detect.outputs.needs_python }}
      needs_elixir: ${{ steps.detect.outputs.needs_elixir }}
  build:
    steps:
      - name: Full build on main merge
        run: ./build-tool -root . -force -validate-build-files -language all
`), 0644); err != nil {
		t.Fatal(err)
	}

	graph := directedgraph.New()
	graph.AddNode("elixir/actor")
	graph.AddNode("python/actor")

	err := ValidateBuildFiles(pkgs, graph)
	if err == nil {
		t.Fatal("expected CI validation failure")
	}
	msg := err.Error()
	if !strings.Contains(msg, ".github/workflows/ci.yml") {
		t.Fatalf("expected ci.yml to be mentioned, got %v", err)
	}
	if !strings.Contains(msg, "elixir") || !strings.Contains(msg, "python") {
		t.Fatalf("expected missing toolchain languages in message, got %v", err)
	}
}

func TestValidateBuildFilesAllowsFullBuildWorkflowWithNormalizedToolchains(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{name: "elixir/actor", relPath: "code/packages/elixir/actor", lang: "elixir"},
		{name: "python/actor", relPath: "code/packages/python/actor", lang: "python"},
	})

	repoRoot := inferRepoRoot(pkgs)
	if repoRoot == "" {
		t.Fatal("expected repo root inference to succeed")
	}

	ciPath := filepath.Join(repoRoot, ".github", "workflows")
	if err := os.MkdirAll(ciPath, 0755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(ciPath, "ci.yml"), []byte(`
jobs:
  detect:
    outputs:
      needs_python: ${{ steps.toolchains.outputs.needs_python }}
      needs_elixir: ${{ steps.toolchains.outputs.needs_elixir }}
    steps:
      - name: Normalize toolchain requirements
        id: toolchains
        run: |
          printf '%s\n' \
            'needs_python=true' \
            'needs_elixir=true' >> "$GITHUB_OUTPUT"
  build:
    steps:
      - name: Full build on main merge
        run: ./build-tool -root . -force -validate-build-files -language all
`), 0644); err != nil {
		t.Fatal(err)
	}

	graph := directedgraph.New()
	graph.AddNode("elixir/actor")
	graph.AddNode("python/actor")

	if err := ValidateBuildFiles(pkgs, graph); err != nil {
		t.Fatalf("expected CI validation to pass, got %v", err)
	}
}
