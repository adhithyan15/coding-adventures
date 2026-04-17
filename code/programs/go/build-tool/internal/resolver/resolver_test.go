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
// Tests for Swift dependency parsing
// ---------------------------------------------------------------------------

func TestParseSwiftDepsSupportsNestedPackagePaths(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"programs/hash-breaker/Package.swift": `import PackageDescription

let package = Package(
    name: "HashBreaker",
    dependencies: [
        .package(path: "../../../packages/swift/md5"),
    ]
)
`,
		"packages/md5/Package.swift": `import PackageDescription

let package = Package(name: "md5")
`,
	})

	packages := []discovery.Package{
		{Name: "swift/programs/hash-breaker", Path: filepath.Join(root, "programs/hash-breaker"), Language: "swift"},
		{Name: "swift/md5", Path: filepath.Join(root, "packages/md5"), Language: "swift"},
	}

	known := BuildKnownNames(packages)
	deps := parseSwiftDeps(packages[0], known)

	if len(deps) != 1 || deps[0] != "swift/md5" {
		t.Fatalf("expected swift/md5 dependency, got %v", deps)
	}
}

func TestResolveDependenciesWasmCanReferenceRustCrate(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"wasm-graph/Cargo.toml": `[package]
name = "graph-wasm"

[dependencies]
graph = { path = "../../rust/graph" }
`,
		"rust-graph/Cargo.toml": `[package]
name = "graph"
`,
	})

	packages := []discovery.Package{
		{Name: "wasm/graph", Path: filepath.Join(root, "wasm-graph"), Language: "wasm"},
		{Name: "rust/graph", Path: filepath.Join(root, "rust-graph"), Language: "rust"},
	}

	graph := ResolveDependencies(packages)
	if !graph.HasEdge("rust/graph", "wasm/graph") {
		t.Fatalf("expected rust/graph -> wasm/graph edge, got %v", graph.Edges())
	}
}

func TestResolveDependenciesWasmPrefersRustOnSharedBasename(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"wasm-avl-tree/Cargo.toml": `[package]
name = "avl-tree-wasm"

[dependencies]
avl-tree = { path = "../../rust/avl-tree" }
`,
		"rust-avl-tree/Cargo.toml": `[package]
name = "avl-tree"
`,
	})

	packages := []discovery.Package{
		{Name: "wasm/avl-tree", Path: filepath.Join(root, "wasm-avl-tree"), Language: "wasm"},
		{Name: "rust/avl-tree", Path: filepath.Join(root, "rust-avl-tree"), Language: "rust"},
	}

	graph := ResolveDependencies(packages)
	if graph.HasEdge("wasm/avl-tree", "wasm/avl-tree") {
		t.Fatalf("did not expect wasm self-loop, got %v", graph.Edges())
	}
	if !graph.HasEdge("rust/avl-tree", "wasm/avl-tree") {
		t.Fatalf("expected rust/avl-tree -> wasm/avl-tree edge, got %v", graph.Edges())
	}
}

func TestBuildKnownNamesForLanguageWasmDoesNotClaimBareRustCrateNames(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"wasm-avl-tree/Cargo.toml": `[package]
name = "avl-tree-wasm"
`,
	})

	packages := []discovery.Package{
		{Name: "wasm/avl-tree", Path: filepath.Join(root, "wasm-avl-tree"), Language: "wasm"},
	}

	known := buildKnownNamesForLanguage(packages, "wasm")
	if _, ok := known["avl-tree"]; ok {
		t.Fatalf("did not expect wasm scope to claim bare rust crate name avl-tree: %v", known)
	}
	if got := known["avl-tree-wasm"]; got != "wasm/avl-tree" {
		t.Fatalf("expected avl-tree-wasm -> wasm/avl-tree, got %q", got)
	}
}

func TestResolveDependenciesDotnetScopeSupportsCrossLanguageProjectReferences(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"csharp-graph/CodingAdventures.Graph.csproj": `<Project Sdk="Microsoft.NET.Sdk">
  <ItemGroup>
    <ProjectReference Include="../fsharp-helpers/CodingAdventures.Helpers.fsproj" />
  </ItemGroup>
</Project>
`,
		"fsharp-helpers/CodingAdventures.Helpers.fsproj": `<Project Sdk="Microsoft.NET.Sdk">
</Project>
`,
	})

	packages := []discovery.Package{
		{Name: "csharp/graph", Path: filepath.Join(root, "csharp-graph"), Language: "csharp"},
		{Name: "fsharp/helpers", Path: filepath.Join(root, "fsharp-helpers"), Language: "fsharp"},
	}

	graph := ResolveDependencies(packages)
	if !graph.HasEdge("fsharp/helpers", "csharp/graph") {
		t.Fatalf("expected fsharp/helpers -> csharp/graph edge, got %v", graph.Edges())
	}
}

func TestResolveDependenciesDotnetPrefersSameLanguageOnSharedBasename(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"csharp/graph/CodingAdventures.Graph.csproj": `<Project Sdk="Microsoft.NET.Sdk">
  <ItemGroup>
    <ProjectReference Include="../bitset/CodingAdventures.Bitset.csproj" />
  </ItemGroup>
</Project>
`,
		"csharp/bitset/CodingAdventures.Bitset.csproj": `<Project Sdk="Microsoft.NET.Sdk">
</Project>
`,
		"fsharp/bitset/CodingAdventures.Bitset.fsproj": `<Project Sdk="Microsoft.NET.Sdk">
</Project>
`,
	})

	packages := []discovery.Package{
		{Name: "csharp/graph", Path: filepath.Join(root, "csharp", "graph"), Language: "csharp"},
		{Name: "csharp/bitset", Path: filepath.Join(root, "csharp", "bitset"), Language: "csharp"},
		{Name: "fsharp/bitset", Path: filepath.Join(root, "fsharp", "bitset"), Language: "fsharp"},
	}

	graph := ResolveDependencies(packages)
	if !graph.HasEdge("csharp/bitset", "csharp/graph") {
		t.Fatalf("expected csharp/bitset -> csharp/graph edge, got %v", graph.Edges())
	}
	if graph.HasEdge("fsharp/bitset", "csharp/graph") {
		t.Fatalf("did not expect fsharp/bitset -> csharp/graph edge, got %v", graph.Edges())
	}
}

func TestParseHaskellDepsSkipsSelfReference(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"build-tool/coding-adventures-build-tool.cabal": `cabal-version: 3.0
name:          coding-adventures-build-tool
version:       0.1.0

library
    exposed-modules:  BuildTool
    build-depends:    base >=4.14

test-suite spec
    type:             exitcode-stdio-1.0
    main-is:          Spec.hs
    build-depends:    base >=4.14
                    , coding-adventures-build-tool
                    , coding-adventures-logic-gates
`,
		"logic-gates/coding-adventures-logic-gates.cabal": `name: coding-adventures-logic-gates`,
	})

	packages := []discovery.Package{
		{Name: "haskell/programs/build-tool", Path: filepath.Join(root, "build-tool"), Language: "haskell"},
		{Name: "haskell/logic-gates", Path: filepath.Join(root, "logic-gates"), Language: "haskell"},
	}

	known := BuildKnownNames(packages)
	deps := parseHaskellDeps(packages[0], known)

	if len(deps) != 1 || deps[0] != "haskell/logic-gates" {
		t.Fatalf("expected only haskell/logic-gates dependency, got %v", deps)
	}
}

func TestDependencyScopeMapsSharedToolchainFamilies(t *testing.T) {
	tests := map[string]string{
		"python": "python",
		"wasm":   "wasm",
		"csharp": "dotnet",
		"fsharp": "dotnet",
		"dotnet": "dotnet",
	}

	for language, want := range tests {
		if got := dependencyScope(language); got != want {
			t.Fatalf("dependencyScope(%q) = %q, want %q", language, got, want)
		}
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

func TestResolveDependenciesKeepsLanguageScopedNameMappings(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"elixir/document_ast_sanitizer/mix.exs": `defmodule X do
  def project, do: [app: :x, version: "0.1.0", deps: deps()]
  defp deps, do: [{:coding_adventures_document_ast, path: "../document_ast"}]
end
`,
		"elixir/document_ast/mix.exs": `defmodule X do
  def project, do: [app: :coding_adventures_document_ast, version: "0.1.0", deps: []]
end
`,
		"ruby/document_ast/document_ast.gemspec": `Gem::Specification.new do |spec|
  spec.name = "coding_adventures_document_ast"
end
`,
	})

	packages := []discovery.Package{
		{Name: "elixir/document_ast_sanitizer", Path: filepath.Join(root, "elixir/document_ast_sanitizer"), Language: "elixir"},
		{Name: "elixir/document_ast", Path: filepath.Join(root, "elixir/document_ast"), Language: "elixir"},
		{Name: "ruby/document_ast", Path: filepath.Join(root, "ruby/document_ast"), Language: "ruby"},
	}

	graph := ResolveDependencies(packages)

	if !graph.HasEdge("elixir/document_ast", "elixir/document_ast_sanitizer") {
		t.Fatal("expected Elixir dependency to resolve to the Elixir package")
	}
	if graph.HasEdge("ruby/document_ast", "elixir/document_ast_sanitizer") {
		t.Fatal("did not expect Elixir dependency to resolve to the Ruby package")
	}
}

func TestResolveElixirDepsSupportsPlainAppAtoms(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"elixir/sql_csv_source/mix.exs": `defmodule X do
  def project, do: [app: :coding_adventures_sql_csv_source, version: "0.1.0", deps: deps()]
  defp deps, do: [{:csv_parser, path: "../csv_parser"}]
end
`,
		"elixir/csv_parser/mix.exs": `defmodule X do
  def project, do: [app: :csv_parser, version: "0.1.0", deps: []]
end
`,
	})

	packages := []discovery.Package{
		{Name: "elixir/sql_csv_source", Path: filepath.Join(root, "elixir/sql_csv_source"), Language: "elixir"},
		{Name: "elixir/csv_parser", Path: filepath.Join(root, "elixir/csv_parser"), Language: "elixir"},
	}

	graph := ResolveDependencies(packages)

	if !graph.HasEdge("elixir/csv_parser", "elixir/sql_csv_source") {
		t.Fatal("expected Elixir dependency with plain app atom to resolve")
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
	if known["logic-gates"] != "typescript/logic-gates" {
		t.Fatalf("expected unscoped mapping for typescript/logic-gates, got %s", known["logic-gates"])
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

func TestParseTypescriptDepsIncludesDevDependenciesAndUnscopedNames(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/package.json": `{
  "name": "pkg-a",
  "dependencies": {
    "@coding-adventures/pkg-b": "file:../pkg-b"
  },
  "devDependencies": {
    "pkg-c": "file:../pkg-c"
  }
}`,
		"pkg-b/package.json": `{
  "name": "@coding-adventures/pkg-b"
}`,
		"pkg-c/package.json": `{
  "name": "pkg-c"
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

func TestBuildKnownNamesDart(t *testing.T) {
	packages := []discovery.Package{
		{Name: "dart/logic-gates", Path: "/repo/packages/dart/logic-gates", Language: "dart"},
	}
	known := BuildKnownNames(packages)
	if known["coding_adventures_logic_gates"] != "dart/logic-gates" {
		t.Fatalf("expected dart/logic-gates, got %s", known["coding_adventures_logic_gates"])
	}
	if known["logic_gates"] != "dart/logic-gates" {
		t.Fatalf("expected unprefixed mapping for dart/logic-gates, got %s", known["logic_gates"])
	}
}

func TestParseDartDeps(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/pubspec.yaml": `name: pkg_a
dependencies:
  coding_adventures_pkg_b: ^0.1.0
dev_dependencies:
  pkg_c:
    path: ../pkg-c
`,
		"pkg-b/pubspec.yaml": `name: coding_adventures_pkg_b
`,
		"pkg-c/pubspec.yaml": `name: pkg_c
`,
	})

	packages := []discovery.Package{
		{Name: "dart/pkg-a", Path: filepath.Join(root, "pkg-a"), Language: "dart"},
		{Name: "dart/pkg-b", Path: filepath.Join(root, "pkg-b"), Language: "dart"},
		{Name: "dart/pkg-c", Path: filepath.Join(root, "pkg-c"), Language: "dart"},
	}

	known := BuildKnownNames(packages)
	deps := parseDartDeps(packages[0], known)

	if len(deps) != 2 {
		t.Fatalf("expected 2 deps, got %d: %v", len(deps), deps)
	}
}

func TestParseDartDepsExternalSkipped(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/pubspec.yaml": `name: pkg_a
dependencies:
  collection: ^1.18.0
  args: ^2.6.0
`,
	})

	pkg := discovery.Package{Name: "dart/pkg-a", Path: filepath.Join(root, "pkg-a"), Language: "dart"}
	deps := parseDartDeps(pkg, map[string]string{})
	if len(deps) != 0 {
		t.Fatalf("expected external deps to be skipped, got %d", len(deps))
	}
}

func TestBuildKnownNamesRust(t *testing.T) {
	packages := []discovery.Package{
		{Name: "rust/logic-gates", Path: "/repo/packages/rust/logic-gates", Language: "rust"},
	}
	known := BuildKnownNames(packages)
	if known["logic-gates"] != "rust/logic-gates" {
		t.Fatalf("expected rust/logic-gates, got %s", known["logic-gates"])
	}
}

func TestParseRustDeps(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"arithmetic/Cargo.toml": `[package]
name = "arithmetic"
version = "0.1.0"
edition = "2021"

[dependencies]
logic-gates = { path = "../logic-gates" }
`,
		"logic-gates/Cargo.toml": `[package]
name = "logic-gates"
version = "0.1.0"
edition = "2021"
`,
	})

	packages := []discovery.Package{
		{Name: "rust/arithmetic", Path: filepath.Join(root, "arithmetic"), Language: "rust"},
		{Name: "rust/logic-gates", Path: filepath.Join(root, "logic-gates"), Language: "rust"},
	}

	known := BuildKnownNames(packages)
	deps := parseRustDeps(packages[0], known)

	if len(deps) != 1 {
		t.Fatalf("expected 1 dep, got %d: %v", len(deps), deps)
	}
	if deps[0] != "rust/logic-gates" {
		t.Fatalf("expected rust/logic-gates, got %s", deps[0])
	}
}

func TestParseRustDepsNoCargoToml(t *testing.T) {
	root := t.TempDir()
	pkg := discovery.Package{Name: "rust/pkg-x", Path: root, Language: "rust"}
	deps := parseRustDeps(pkg, map[string]string{})
	if len(deps) != 0 {
		t.Fatalf("expected 0 deps, got %d", len(deps))
	}
}

// ---------------------------------------------------------------------------
// Tests for Perl dependency parsing
// ---------------------------------------------------------------------------

func TestParsePerlDepsSingleDep(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"bitset/cpanfile": `requires 'coding-adventures-logic-gates';

on 'test' => sub {
    requires 'Test2::V0';
};
`,
		"logic-gates/cpanfile": `on 'test' => sub {
    requires 'Test2::V0';
};
`,
	})

	packages := []discovery.Package{
		{Name: "perl/bitset", Path: filepath.Join(root, "bitset"), Language: "perl"},
		{Name: "perl/logic-gates", Path: filepath.Join(root, "logic-gates"), Language: "perl"},
	}

	known := BuildKnownNames(packages)
	deps := parsePerlDeps(packages[0], known)

	if len(deps) != 1 {
		t.Fatalf("expected 1 dep, got %d: %v", len(deps), deps)
	}
	if deps[0] != "perl/logic-gates" {
		t.Errorf("expected perl/logic-gates, got %s", deps[0])
	}
}

func TestParsePerlDepsMultipleDeps(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"arithmetic/cpanfile": `requires 'coding-adventures-logic-gates';
requires 'coding-adventures-bitset', '>= 0.01';

on 'test' => sub {
    requires 'Test2::V0';
};
`,
		"logic-gates/cpanfile": "",
		"bitset/cpanfile":      "",
	})

	packages := []discovery.Package{
		{Name: "perl/arithmetic", Path: filepath.Join(root, "arithmetic"), Language: "perl"},
		{Name: "perl/logic-gates", Path: filepath.Join(root, "logic-gates"), Language: "perl"},
		{Name: "perl/bitset", Path: filepath.Join(root, "bitset"), Language: "perl"},
	}

	known := BuildKnownNames(packages)
	deps := parsePerlDeps(packages[0], known)

	if len(deps) != 2 {
		t.Fatalf("expected 2 deps, got %d: %v", len(deps), deps)
	}
}

func TestParsePerlDepsExternalSkipped(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"my-pkg/cpanfile": `requires 'Moo';
requires 'JSON::PP';

on 'test' => sub {
    requires 'Test2::V0';
};
`,
	})

	packages := []discovery.Package{
		{Name: "perl/my-pkg", Path: filepath.Join(root, "my-pkg"), Language: "perl"},
	}

	known := BuildKnownNames(packages)
	deps := parsePerlDeps(packages[0], known)

	if len(deps) != 0 {
		t.Fatalf("expected 0 deps (all external), got %d: %v", len(deps), deps)
	}
}

func TestParsePerlDepsCommentSkipped(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"my-pkg/cpanfile": `# requires 'coding-adventures-logic-gates';
requires 'coding-adventures-bitset';
`,
		"logic-gates/cpanfile": "",
		"bitset/cpanfile":      "",
	})

	packages := []discovery.Package{
		{Name: "perl/my-pkg", Path: filepath.Join(root, "my-pkg"), Language: "perl"},
		{Name: "perl/logic-gates", Path: filepath.Join(root, "logic-gates"), Language: "perl"},
		{Name: "perl/bitset", Path: filepath.Join(root, "bitset"), Language: "perl"},
	}

	known := BuildKnownNames(packages)
	deps := parsePerlDeps(packages[0], known)

	if len(deps) != 1 {
		t.Fatalf("expected 1 dep (comment skipped), got %d: %v", len(deps), deps)
	}
	if deps[0] != "perl/bitset" {
		t.Errorf("expected perl/bitset, got %s", deps[0])
	}
}

func TestParsePerlDepsMissingCpanfile(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"my-pkg/lib/Foo.pm": "package Foo; 1;",
	})

	packages := []discovery.Package{
		{Name: "perl/my-pkg", Path: filepath.Join(root, "my-pkg"), Language: "perl"},
	}

	known := BuildKnownNames(packages)
	deps := parsePerlDeps(packages[0], known)

	if len(deps) != 0 {
		t.Fatalf("expected 0 deps (no cpanfile), got %d: %v", len(deps), deps)
	}
}

func TestParsePerlDepsDoubleQuotes(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"my-pkg/cpanfile":      `requires "coding-adventures-logic-gates";`,
		"logic-gates/cpanfile": "",
	})

	packages := []discovery.Package{
		{Name: "perl/my-pkg", Path: filepath.Join(root, "my-pkg"), Language: "perl"},
		{Name: "perl/logic-gates", Path: filepath.Join(root, "logic-gates"), Language: "perl"},
	}

	known := BuildKnownNames(packages)
	deps := parsePerlDeps(packages[0], known)

	if len(deps) != 1 {
		t.Fatalf("expected 1 dep (double quotes), got %d: %v", len(deps), deps)
	}
}

func TestBuildKnownNamesPerl(t *testing.T) {
	packages := []discovery.Package{
		{Name: "perl/logic-gates", Path: "/repo/code/packages/perl/logic-gates", Language: "perl"},
		{Name: "perl/bitset", Path: "/repo/code/packages/perl/bitset", Language: "perl"},
	}
	known := BuildKnownNames(packages)

	if known["coding-adventures-logic-gates"] != "perl/logic-gates" {
		t.Errorf("expected perl/logic-gates, got %s", known["coding-adventures-logic-gates"])
	}
	if known["coding-adventures-bitset"] != "perl/bitset" {
		t.Errorf("expected perl/bitset, got %s", known["coding-adventures-bitset"])
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
