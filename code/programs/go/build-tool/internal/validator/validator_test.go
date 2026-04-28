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
