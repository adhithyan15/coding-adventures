package validator

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

type luaWindowsSiblingFixture struct {
	Input struct {
		Options struct {
			Packages []struct {
				Name                        string   `json:"name"`
				RelPath                     string   `json:"rel_path"`
				Language                    string   `json:"language"`
				BuildFileState              string   `json:"build_file_state"`
				CanonicalLuaSiblingInstalls []string `json:"canonical_lua_sibling_installs"`
				WindowsBuildFileState       string   `json:"windows_build_file_state"`
				WindowsLuaSiblingInstalls   []string `json:"windows_lua_sibling_installs"`
			} `json:"packages"`
		} `json:"options"`
	} `json:"input"`
	Expected struct {
		Diagnostics []struct {
			Code    string `json:"code"`
			Path    string `json:"path"`
			Package string `json:"package"`
			Details struct {
				MissingSiblingInstalls []string `json:"missing_sibling_installs"`
				WindowsBuildFileState  string   `json:"windows_build_file_state"`
			} `json:"details"`
		} `json:"diagnostics"`
	} `json:"expected"`
}

func loadLuaWindowsSiblingFixture(t *testing.T, name string) luaWindowsSiblingFixture {
	t.Helper()
	_, sourceFile, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("could not locate validator test source")
	}
	repoRoot := filepath.Clean(filepath.Join(
		filepath.Dir(sourceFile), "..", "..", "..", "..", "..", "..",
	))
	fixturePath := filepath.Join(
		repoRoot, "code", "specs", "fixtures", "build-tool-v1", "cases", name,
	)
	data, err := os.ReadFile(fixturePath)
	if err != nil {
		t.Fatalf("read shared fixture %s: %v", name, err)
	}
	var fixture luaWindowsSiblingFixture
	if err := json.Unmarshal(data, &fixture); err != nil {
		t.Fatalf("decode shared fixture %s: %v", name, err)
	}
	return fixture
}

func luaSiblingInstallLine(packageName string, windows bool) string {
	directory := strings.TrimPrefix(packageName, "lua/")
	rock := strings.ReplaceAll(directory, "_", "-")
	prefix := "../"
	if windows {
		prefix = `..\`
	}
	return fmt.Sprintf(
		"(cd %s%s && luarocks make --local --deps-mode=none coding-adventures-%s-0.1.0-1.rockspec)",
		prefix, directory, rock,
	)
}

func luaSelfInstallLine(packageName string) string {
	directory := strings.TrimPrefix(packageName, "lua/")
	rock := strings.ReplaceAll(directory, "_", "-")
	return fmt.Sprintf(
		"luarocks make --local --deps-mode=none coding-adventures-%s-0.1.0-1.rockspec",
		rock,
	)
}

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

func writeBuildFile(t *testing.T, pkgPath, name, content string) {
	t.Helper()
	if err := os.WriteFile(filepath.Join(pkgPath, name), []byte(content), 0644); err != nil {
		t.Fatal(err)
	}
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

func TestValidateBuildFilesAllowsIntentionalPlatformSkip(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{name: "perl/a", relPath: "code/packages/perl/a", lang: "perl"},
		{
			name:    "perl/b",
			relPath: "code/packages/perl/b",
			lang:    "perl",
			commands: []string{
				`echo Perl testing is not supported on Windows - skipping`,
			},
		},
	})

	graph := graphWithEdges([2]string{"perl/a", "perl/b"})

	if err := ValidateBuildFiles(pkgs, graph); err != nil {
		t.Fatalf("expected platform skip to pass validation, got %v", err)
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

// TestValidateBuildFilesAllowsDeclaredCrossLanguageBuildDep covers the
// "# build-tool: deps=..." comment escape hatch: a Swift BUILD_windows file
// that stages a Rust-built native library via shell commands (invisible to
// the Package.swift .package(path:) scanner that builds the dependency
// graph) should not be flagged as a hidden reference when it declares that
// dependency via the comment.
func TestValidateBuildFilesAllowsDeclaredCrossLanguageBuildDep(t *testing.T) {
	root := t.TempDir()
	rustPath := filepath.Join(root, filepath.FromSlash("code/packages/rust/lib"))
	swiftPath := filepath.Join(root, filepath.FromSlash("code/packages/swift/pkg"))
	if err := os.MkdirAll(rustPath, 0755); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(swiftPath, 0755); err != nil {
		t.Fatal(err)
	}

	pkgs := []discovery.Package{
		{Name: "rust/lib", Path: rustPath, Language: "rust"},
		{
			Name:          "swift/pkg",
			Path:          swiftPath,
			Language:      "swift",
			BuildCommands: []string{`cd ../../rust/lib && cargo build --release`},
			BuildContent:  "# build-tool: deps=rust/lib\ncd ../../rust/lib && cargo build --release\n",
		},
	}

	graph := directedgraph.New()
	graph.AddNode("rust/lib")
	graph.AddNode("swift/pkg")

	if err := ValidateBuildFiles(pkgs, graph); err != nil {
		t.Fatalf("expected declared cross-language dep to be allowed, got %v", err)
	}
}

// TestValidateBuildFilesFailsUndeclaredCrossLanguageBuildDep is the
// companion negative case: the same shape, but without the "# build-tool:
// deps=" comment, must still be flagged.
func TestValidateBuildFilesFailsUndeclaredCrossLanguageBuildDep(t *testing.T) {
	root := t.TempDir()
	rustPath := filepath.Join(root, filepath.FromSlash("code/packages/rust/lib"))
	swiftPath := filepath.Join(root, filepath.FromSlash("code/packages/swift/pkg"))
	if err := os.MkdirAll(rustPath, 0755); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(swiftPath, 0755); err != nil {
		t.Fatal(err)
	}

	pkgs := []discovery.Package{
		{Name: "rust/lib", Path: rustPath, Language: "rust"},
		{
			Name:          "swift/pkg",
			Path:          swiftPath,
			Language:      "swift",
			BuildCommands: []string{`cd ../../rust/lib && cargo build --release`},
			BuildContent:  "cd ../../rust/lib && cargo build --release\n",
		},
	}

	graph := directedgraph.New()
	graph.AddNode("rust/lib")
	graph.AddNode("swift/pkg")

	err := ValidateBuildFiles(pkgs, graph)
	if err == nil {
		t.Fatal("expected validation failure")
	}
	if !strings.Contains(err.Error(), "undeclared local package refs: rust/lib") {
		t.Fatalf("expected hidden reference message, got %v", err)
	}
}

func TestValidateBuildFilesAllowsPerlTestDependencyClosure(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{name: "perl/a", relPath: "code/packages/perl/a", lang: "perl"},
		{
			name:     "perl/b",
			relPath:  "code/packages/perl/b",
			lang:     "perl",
			commands: []string{`cd ../a`},
		},
		{
			name:     "perl/c",
			relPath:  "code/packages/perl/c",
			lang:     "perl",
			commands: []string{`cd ../a`, `cd ../b`},
		},
	})
	writeBuildFile(t, pkgs[2].Path, "cpanfile", `
on 'test' => sub {
    requires 'coding-adventures-b';
};
`)

	graph := graphWithEdges([2]string{"perl/a", "perl/b"})
	graph.AddNode("perl/c")
	if err := ValidateBuildFiles(pkgs, graph); err != nil {
		t.Fatalf("expected Perl test dependency closure to be allowed, got %v", err)
	}
}

func TestValidateBuildFilesRejectsPerlReferenceOutsideTestPhase(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{name: "perl/a", relPath: "code/packages/perl/a", lang: "perl"},
		{
			name:     "perl/b",
			relPath:  "code/packages/perl/b",
			lang:     "perl",
			commands: []string{`cd ../a`},
		},
	})
	writeBuildFile(t, pkgs[1].Path, "cpanfile", `requires 'coding-adventures-a';`)

	graph := directedgraph.New()
	graph.AddNode("perl/a")
	graph.AddNode("perl/b")
	err := ValidateBuildFiles(pkgs, graph)
	if err == nil || !strings.Contains(err.Error(), "undeclared local package refs: perl/a") {
		t.Fatalf("expected top-level reference to require a runtime graph edge, got %v", err)
	}
}

func TestValidateBuildFilesSkipsUnknownLanguagePackages(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{name: "unknown/a", relPath: "code/packages/custom/a", lang: "unknown"},
		{
			name:    "unknown/b",
			relPath: "code/packages/custom/b",
			lang:    "unknown",
			commands: []string{
				`cd ../a && custom-build-tool test`,
			},
		},
	})

	graph := directedgraph.New()
	graph.AddNode("unknown/a")
	graph.AddNode("unknown/b")

	if err := ValidateBuildFiles(pkgs, graph); err != nil {
		t.Fatalf("expected unknown packages to be ignored, got %v", err)
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
		{name: "swift/actor", relPath: "code/packages/swift/actor", lang: "swift"},
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
	graph.AddNode("swift/actor")

	err := ValidateBuildFiles(pkgs, graph)
	if err == nil {
		t.Fatal("expected CI validation failure")
	}
	msg := err.Error()
	if !strings.Contains(msg, ".github/workflows/ci.yml") {
		t.Fatalf("expected ci.yml to be mentioned, got %v", err)
	}
	if !strings.Contains(msg, "elixir") || !strings.Contains(msg, "python") || !strings.Contains(msg, "swift") {
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
		{name: "swift/actor", relPath: "code/packages/swift/actor", lang: "swift"},
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
      needs_swift: ${{ steps.toolchains.outputs.needs_swift }}
    steps:
      - name: Normalize toolchain requirements
        id: toolchains
        run: |
          printf '%s\n' \
            'needs_python=true' \
            'needs_elixir=true' \
            'needs_swift=true' >> "$GITHUB_OUTPUT"
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
	graph.AddNode("swift/actor")

	if err := ValidateBuildFiles(pkgs, graph); err != nil {
		t.Fatalf("expected CI validation to pass, got %v", err)
	}
}

// A cpp package must force the CI workflow to bind and normalize needs_cpp on
// the forced main full-build path. This locks in the ciManagedToolchainLanguages
// entry added for the C/C++ multi-compiler lane (CCPP01 PR2).
func TestValidateBuildFilesRequiresNeedsCppForCppPackages(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{name: "cpp/static-vector", relPath: "code/packages/cpp/static-vector", lang: "cpp"},
	})

	repoRoot := inferRepoRoot(pkgs)
	if repoRoot == "" {
		t.Fatal("expected repo root inference to succeed")
	}
	ciDir := filepath.Join(repoRoot, ".github", "workflows")
	if err := os.MkdirAll(ciDir, 0755); err != nil {
		t.Fatal(err)
	}

	graph := directedgraph.New()
	graph.AddNode("cpp/static-vector")

	// Workflow missing the needs_cpp binding entirely → must fail and name cpp.
	if err := os.WriteFile(filepath.Join(ciDir, "ci.yml"), []byte(`
jobs:
  detect:
    outputs:
      needs_swift: ${{ steps.toolchains.outputs.needs_swift }}
    steps:
      - name: Normalize toolchain requirements
        id: toolchains
        run: |
          printf '%s\n' 'needs_swift=true' >> "$GITHUB_OUTPUT"
  build:
    steps:
      - name: Full build on main merge
        run: ./build-tool -root . -force -validate-build-files -language all
`), 0644); err != nil {
		t.Fatal(err)
	}
	err := ValidateBuildFiles(pkgs, graph)
	if err == nil {
		t.Fatal("expected CI validation failure for missing needs_cpp")
	}
	if !strings.Contains(err.Error(), "cpp") {
		t.Fatalf("expected cpp to be named in the error, got %v", err)
	}

	// Workflow with the binding + forced main value → must pass.
	if err := os.WriteFile(filepath.Join(ciDir, "ci.yml"), []byte(`
jobs:
  detect:
    outputs:
      needs_cpp: ${{ steps.toolchains.outputs.needs_cpp }}
    steps:
      - name: Normalize toolchain requirements
        id: toolchains
        run: |
          printf '%s\n' 'needs_cpp=true' >> "$GITHUB_OUTPUT"
  build:
    steps:
      - name: Full build on main merge
        run: ./build-tool -root . -force -validate-build-files -language all
`), 0644); err != nil {
		t.Fatal(err)
	}
	if err := ValidateBuildFiles(pkgs, graph); err != nil {
		t.Fatalf("expected CI validation to pass with needs_cpp bound, got %v", err)
	}
}

func TestValidateBuildFilesFailsLuaBuildWithForeignRemoveAndBadOrder(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{name: "lua/directed_graph", relPath: "code/packages/lua/directed_graph", lang: "lua"},
		{name: "lua/state_machine", relPath: "code/packages/lua/state_machine", lang: "lua"},
		{name: "lua/branch_predictor", relPath: "code/packages/lua/branch_predictor", lang: "lua"},
		{name: "lua/problem_pkg", relPath: "code/packages/lua/problem_pkg", lang: "lua"},
	})

	for _, pkg := range pkgs {
		switch filepath.Base(pkg.Path) {
		case "problem_pkg":
			writeBuildFile(t, pkg.Path, "BUILD", `
luarocks remove --force coding-adventures-branch-predictor 2>/dev/null || true
(cd ../state_machine && luarocks make --local coding-adventures-state-machine-0.1.0-1.rockspec)
(cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks make --local coding-adventures-problem-pkg-0.1.0-1.rockspec
`)
		default:
			writeBuildFile(t, pkg.Path, "BUILD", "echo ok\n")
		}
	}

	graph := graphWithEdges(
		[2]string{"lua/directed_graph", "lua/state_machine"},
		[2]string{"lua/state_machine", "lua/problem_pkg"},
	)
	graph.AddNode("lua/branch_predictor")

	err := ValidateBuildFiles(pkgs, graph)
	if err == nil {
		t.Fatal("expected Lua BUILD validation failure")
	}
	msg := err.Error()
	if !strings.Contains(msg, "coding-adventures-branch-predictor") {
		t.Fatalf("expected foreign remove message, got %v", err)
	}
	if !strings.Contains(msg, "state_machine before directed_graph") {
		t.Fatalf("expected dependency order message, got %v", err)
	}
}

func TestValidateBuildFilesFailsLuaGuardedInstallWithoutDepsModeNone(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{name: "lua/transistors", relPath: "code/packages/lua/transistors", lang: "lua"},
		{name: "lua/gatelevel", relPath: "code/packages/lua/gatelevel", lang: "lua"},
	})

	for _, pkg := range pkgs {
		switch filepath.Base(pkg.Path) {
		case "gatelevel":
			writeBuildFile(t, pkg.Path, "BUILD", `
luarocks show coding-adventures-transistors >/dev/null 2>&1 || (cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
luarocks make --local coding-adventures-gatelevel-0.1.0-1.rockspec
`)
		default:
			writeBuildFile(t, pkg.Path, "BUILD", "echo ok\n")
		}
	}

	graph := graphWithEdges([2]string{"lua/transistors", "lua/gatelevel"})

	err := ValidateBuildFiles(pkgs, graph)
	if err == nil {
		t.Fatal("expected guarded-install validation failure")
	}
	if !strings.Contains(err.Error(), "--deps-mode=none or --no-manifest") {
		t.Fatalf("expected deps-mode guidance, got %v", err)
	}
}

func TestValidateBuildFilesAllowsSafeLuaIsolatedBuildPatterns(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{name: "lua/directed_graph", relPath: "code/packages/lua/directed_graph", lang: "lua"},
		{name: "lua/state_machine", relPath: "code/packages/lua/state_machine", lang: "lua"},
		{name: "lua/safe_pkg", relPath: "code/packages/lua/safe_pkg", lang: "lua"},
	})

	for _, pkg := range pkgs {
		switch filepath.Base(pkg.Path) {
		case "safe_pkg":
			writeBuildFile(t, pkg.Path, "BUILD", `
luarocks remove --force coding-adventures-safe-pkg 2>/dev/null || true
luarocks show coding-adventures-directed-graph >/dev/null 2>&1 || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks show coding-adventures-state-machine >/dev/null 2>&1 || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
`)
			writeBuildFile(t, pkg.Path, "BUILD_windows", `
luarocks show coding-adventures-directed-graph 1>nul 2>nul || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks show coding-adventures-state-machine 1>nul 2>nul || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
`)
			writeBuildFile(t, pkg.Path, "coding-adventures-safe-pkg-0.1.0-1.rockspec", `
package = "coding-adventures-safe-pkg"
version = "0.1.0-1"
dependencies = {
  "lua >= 5.4",
  "coding-adventures-state-machine >= 0.1.0",
}
`)
		default:
			writeBuildFile(t, pkg.Path, "BUILD", "echo ok\n")
		}
	}

	graph := graphWithEdges(
		[2]string{"lua/directed_graph", "lua/state_machine"},
		[2]string{"lua/state_machine", "lua/safe_pkg"},
	)

	if err := ValidateBuildFiles(pkgs, graph); err != nil {
		t.Fatalf("expected safe Lua BUILD validation to pass, got %v", err)
	}
}

func TestValidateBuildFilesFailsLuaSelfManagedBuildWithoutExplicitLocalDeps(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{name: "lua/wasm_leb128", relPath: "code/packages/lua/wasm_leb128", lang: "lua"},
		{name: "lua/wasm_types", relPath: "code/packages/lua/wasm_types", lang: "lua"},
	})

	writeBuildFile(t, pkgs[0].Path, "BUILD", "echo ok\n")
	writeBuildFile(t, pkgs[1].Path, "BUILD", `
luarocks make --local --deps-mode=none coding-adventures-wasm-types-0.1.0-1.rockspec
`)
	writeBuildFile(t, pkgs[1].Path, "coding-adventures-wasm-types-0.1.0-1.rockspec", `
package = "coding-adventures-wasm-types"
version = "0.1.0-1"
dependencies = {
  "lua >= 5.4",
  "coding-adventures-wasm-leb128 >= 0.1.0",
}
`)

	graph := graphWithEdges([2]string{"lua/wasm_leb128", "lua/wasm_types"})

	err := ValidateBuildFiles(pkgs, graph)
	if err == nil {
		t.Fatal("expected Lua self-managed dependency validation failure")
	}
	if !strings.Contains(err.Error(), "does not bootstrap local rockspec dependencies") {
		t.Fatalf("expected self-managed dependency guidance, got %v", err)
	}
	if !strings.Contains(err.Error(), "coding-adventures-wasm-leb128") {
		t.Fatalf("expected missing local rock dependency, got %v", err)
	}
}

func TestValidateBuildFilesFailsWindowsLuaSiblingDrift(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{name: "lua/arm1_gatelevel", relPath: "code/packages/lua/arm1_gatelevel", lang: "lua"},
	})

	writeBuildFile(t, pkgs[0].Path, "BUILD", `
(cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
(cd ../logic_gates && luarocks make --local coding-adventures-logic-gates-0.1.0-1.rockspec)
(cd ../arithmetic && luarocks make --local coding-adventures-arithmetic-0.1.0-1.rockspec)
(cd ../arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
`)
	writeBuildFile(t, pkgs[0].Path, "BUILD_windows", `
(cd ..\arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
`)

	graph := directedgraph.New()
	graph.AddNode("lua/arm1_gatelevel")

	err := ValidateBuildFiles(pkgs, graph)
	if err == nil {
		t.Fatal("expected Lua BUILD_windows validation failure")
	}
	if !strings.Contains(err.Error(), "BUILD_windows is missing sibling installs present in BUILD") {
		t.Fatalf("expected missing sibling install message, got %v", err)
	}
	if !strings.Contains(err.Error(), "../logic_gates") || !strings.Contains(err.Error(), "../arithmetic") {
		t.Fatalf("expected missing sibling package names, got %v", err)
	}
	if !strings.Contains(err.Error(), "final self-install does not pass --deps-mode=none or --no-manifest") {
		t.Fatalf("expected deps-mode guidance, got %v", err)
	}
}

func TestValidateBuildFilesConsumesAbsentWindowsLuaSiblingFixture(t *testing.T) {
	fixture := loadLuaWindowsSiblingFixture(
		t,
		"validation-lua-windows-sibling-parity-absent.json",
	)
	definitions := make([]struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}, 0, len(fixture.Input.Options.Packages))
	for _, packageRecord := range fixture.Input.Options.Packages {
		definitions = append(definitions, struct {
			name     string
			relPath  string
			lang     string
			commands []string
		}{
			name: packageRecord.Name, relPath: packageRecord.RelPath, lang: packageRecord.Language,
		})
	}
	packages := makePackages(t, definitions)
	byName := make(map[string]discovery.Package, len(packages))
	for _, pkg := range packages {
		byName[pkg.Name] = pkg
	}

	for _, packageRecord := range fixture.Input.Options.Packages {
		pkg := byName[packageRecord.Name]
		canonical := make([]string, 0, len(packageRecord.CanonicalLuaSiblingInstalls)+1)
		for _, sibling := range packageRecord.CanonicalLuaSiblingInstalls {
			canonical = append(canonical, luaSiblingInstallLine(sibling, false))
		}
		if packageRecord.BuildFileState == "present" {
			canonical = append(canonical, luaSelfInstallLine(packageRecord.Name))
			writeBuildFile(t, pkg.Path, "BUILD", strings.Join(canonical, "\n")+"\n")
		}

		if packageRecord.WindowsBuildFileState == "present" {
			windows := make([]string, 0, len(packageRecord.WindowsLuaSiblingInstalls)+1)
			for _, sibling := range packageRecord.WindowsLuaSiblingInstalls {
				windows = append(windows, luaSiblingInstallLine(sibling, true))
			}
			windows = append(windows, luaSelfInstallLine(packageRecord.Name))
			writeBuildFile(t, pkg.Path, "BUILD_windows", strings.Join(windows, "\n")+"\n")
		}
	}

	if len(fixture.Expected.Diagnostics) != 1 {
		t.Fatalf("expected one shared diagnostic, got %d", len(fixture.Expected.Diagnostics))
	}
	diagnostic := fixture.Expected.Diagnostics[0]
	if diagnostic.Code != "STANDALONE_PREREQUISITE_MISSING" {
		t.Fatalf("unexpected shared diagnostic code %q", diagnostic.Code)
	}
	if diagnostic.Details.WindowsBuildFileState != "missing" {
		t.Fatalf("fixture must exercise an absent BUILD_windows override")
	}
	if _, err := os.Stat(filepath.Join(byName[diagnostic.Package].Path, "BUILD_windows")); !os.IsNotExist(err) {
		t.Fatalf("fixture BUILD_windows must be absent, got %v", err)
	}

	graph := directedgraph.New()
	for _, pkg := range packages {
		graph.AddNode(pkg.Name)
	}
	err := ValidateBuildFiles(packages, graph)
	if err == nil {
		t.Fatal("expected absent Lua BUILD_windows validation failure")
	}
	message := err.Error()
	if !strings.Contains(message, "BUILD_windows is missing sibling installs present in BUILD") {
		t.Fatalf("expected missing sibling install message, got %v", err)
	}
	if !strings.Contains(filepath.ToSlash(message), diagnostic.Path) {
		t.Fatalf("expected fixture diagnostic path %q, got %v", diagnostic.Path, err)
	}
	for _, sibling := range diagnostic.Details.MissingSiblingInstalls {
		directory := strings.TrimPrefix(sibling, "lua/")
		if !strings.Contains(message, "../"+directory) {
			t.Fatalf("expected missing sibling %q, got %v", sibling, err)
		}
	}
}

func TestValidateBuildFilesFailsWindowsLuaSiblingHardeningDrift(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{name: "lua/intel4004_gatelevel", relPath: "code/packages/lua/intel4004_gatelevel", lang: "lua"},
	})

	writeBuildFile(t, pkgs[0].Path, "BUILD", `
luarocks show coding-adventures-transistors >/dev/null 2>&1 || (cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
luarocks show coding-adventures-logic-gates >/dev/null 2>&1 || (cd ../logic_gates && luarocks make --local --deps-mode=none coding-adventures-logic-gates-0.1.0-1.rockspec)
luarocks show coding-adventures-arithmetic >/dev/null 2>&1 || (cd ../arithmetic && luarocks make --local --deps-mode=none coding-adventures-arithmetic-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-intel4004-gatelevel-0.1.0-1.rockspec
`)
	writeBuildFile(t, pkgs[0].Path, "BUILD_windows", `
luarocks show coding-adventures-transistors 1>nul 2>nul || (cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
luarocks show coding-adventures-logic-gates 1>nul 2>nul || (cd ../logic_gates && luarocks make --local coding-adventures-logic-gates-0.1.0-1.rockspec)
luarocks show coding-adventures-arithmetic 1>nul 2>nul || (cd ../arithmetic && luarocks make --local coding-adventures-arithmetic-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-intel4004-gatelevel-0.1.0-1.rockspec
`)

	graph := directedgraph.New()
	graph.AddNode("lua/intel4004_gatelevel")

	err := ValidateBuildFiles(pkgs, graph)
	if err == nil {
		t.Fatal("expected Lua BUILD_windows hardening validation failure")
	}
	if !strings.Contains(err.Error(), "sibling installs are missing --deps-mode=none/--no-manifest hardening present in BUILD") {
		t.Fatalf("expected sibling hardening drift guidance, got %v", err)
	}
	if !strings.Contains(err.Error(), "../logic_gates") || !strings.Contains(err.Error(), "../arithmetic") {
		t.Fatalf("expected hardened sibling package names, got %v", err)
	}
}

func TestValidateBuildFilesFailsPerlTestBootstrapWithoutNotest(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{name: "perl/draw-instructions-svg", relPath: "code/packages/perl/draw-instructions-svg", lang: "perl"},
	})

	writeBuildFile(t, pkgs[0].Path, "BUILD", `
cpanm --quiet Test2::V0
prove -l -I../draw-instructions/lib -v t/
`)

	graph := directedgraph.New()
	graph.AddNode("perl/draw-instructions-svg")

	err := ValidateBuildFiles(pkgs, graph)
	if err == nil {
		t.Fatal("expected Perl BUILD validation failure")
	}
	if !strings.Contains(err.Error(), "Test2::V0 without --notest") {
		t.Fatalf("expected Perl bootstrap warning, got %v", err)
	}
}

// A BUILD file is not a shell script: `discovery.readLines` splits it on
// newlines and each line runs as its own `sh -c`. A trailing backslash
// therefore truncates the command silently rather than continuing it — the
// shape that made lang-aot's BUILD run 15 lib tests while appearing to watch
// 39 test targets. It must be rejected, not quietly executed.
func TestValidateBuildFilesRejectsBackslashLineContinuation(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{
			name:    "rust/wrapped",
			relPath: "code/packages/rust/wrapped",
			lang:    "rust",
			commands: []string{
				`cargo test -p wrapped --lib \`,
				`--test alpha \`,
				`--test beta`,
			},
		},
	})

	err := ValidateBuildFiles(pkgs, graphWithEdges())
	if err == nil {
		t.Fatal("expected a backslash-continued BUILD command to be rejected, got nil")
	}
	if !strings.Contains(err.Error(), "line-continuation") {
		t.Fatalf("error should name the continuation as the problem, got %v", err)
	}
}

// A trailing bare `&` backgrounds the command: `sh -c 'false &'` exits 0, so a
// failing build is recorded as passing. Same silent-green class as the
// continuation, so it is rejected the same way. `&&` is a normal separator and
// must NOT be caught.
func TestValidateBuildFilesRejectsTrailingBackgroundAmpersand(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{
			name:     "rust/backgrounded",
			relPath:  "code/packages/rust/backgrounded",
			lang:     "rust",
			commands: []string{`cargo test -p backgrounded &`},
		},
	})

	err := ValidateBuildFiles(pkgs, graphWithEdges())
	if err == nil {
		t.Fatal("expected a backgrounded BUILD command to be rejected, got nil")
	}
	if !strings.Contains(err.Error(), "exit status") {
		t.Fatalf("error should explain the discarded exit status, got %v", err)
	}
}

// The guard keys on the trailing continuation of an executed command. Comment
// lines never become commands (readLines drops them), and `&&` is an ordinary
// separator — neither may trip it.
func TestValidateBuildFilesAllowsPlainSingleLineCommands(t *testing.T) {
	pkgs := makePackages(t, []struct {
		name     string
		relPath  string
		lang     string
		commands []string
	}{
		{
			name:     "rust/flat",
			relPath:  "code/packages/rust/flat",
			lang:     "rust",
			commands: []string{`cargo test -p flat --lib --test alpha --test beta`},
		},
		{
			name:     "rust/chained",
			relPath:  "code/packages/rust/chained",
			lang:     "rust",
			commands: []string{`cargo build && cargo test -p chained`},
		},
	})

	if err := ValidateBuildFiles(pkgs, graphWithEdges()); err != nil {
		t.Fatalf("expected single-line commands to pass, got %v", err)
	}
}

// The continuation character belongs to the shell that will run the line, not
// to the host doing the validating. `cmd /C` continues with `^` and treats `\`
// as the path separator — three real Perl BUILD_windows files end their command
// `prove -l -v t\`, so applying the sh rule there would reject correct code and
// break the Windows CI gate. This is host-independent on purpose: the macOS and
// Linux runners must still prove the Windows branch is right.
func TestContinuationCharFollowsTheShellNotTheHost(t *testing.T) {
	cases := []struct {
		buildFile        string
		wantContinuation string
	}{
		{"code/packages/rust/x/BUILD", `\`},
		{"code/packages/perl/hmac/BUILD_windows", "^"},
		{`code\packages\perl\hmac\BUILD_windows`, "^"},
	}
	for _, c := range cases {
		got, _ := continuationCharForBuildFile(c.buildFile)
		if got != c.wantContinuation {
			t.Errorf("%s: continuation = %q, want %q", c.buildFile, got, c.wantContinuation)
		}
	}
}

// The concrete regression: the real Perl BUILD_windows command must pass under
// the cmd rule, and would fail under the sh rule.
func TestWindowsPathSeparatorIsNotAContinuation(t *testing.T) {
	const cmd = `set "PERL5LIB=lib;..\hyperloglog\lib;%PERL5LIB%" && prove -l -v t\`

	continuation, _ := continuationCharForBuildFile("code/packages/perl/hmac/BUILD_windows")
	if strings.HasSuffix(strings.TrimSpace(cmd), continuation) {
		t.Fatalf("a trailing Windows path separator must not read as a continuation: %q", cmd)
	}

	shContinuation, _ := continuationCharForBuildFile("code/packages/perl/hmac/BUILD")
	if !strings.HasSuffix(strings.TrimSpace(cmd), shContinuation) {
		t.Fatal("guard is not proving anything: this command should trip the sh rule")
	}
}
