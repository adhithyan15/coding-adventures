package main

import (
	"bytes"
	"flag"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/globmatch"
)

var updateKilnGolden = flag.Bool("update-kiln-golden", false, "rewrite testdata/kiln-legacy-graph.golden.json")

// kilnFixtureTree builds a tiny repository under a temp dir. The tree
// exercises every field of the kiln-legacy-graph/1 export:
//
//	rust/core           plain library, one BUILD (all three OSes run it)
//	rust/parser         depends on core, dev-depends on support, and its BUILD
//	                    asks for a java toolchain (extra_toolchains)
//	rust/support        only ever a dev-dependency
//	rust/cyc-a          depends on cyc-b ...
//	rust/cyc-b          ... which dev-depends on cyc-a (a legal dev-dep cycle:
//	                    it must appear in dev_deps, never as a deps edge)
//	rust/unix-only      BUILD_mac_and_linux only: windows has no BUILD (null)
//	rust/win-only       BUILD_windows only: linux and darwin are null
//	rust/rng            named by the repositoryInputConsumers table (repo_inputs)
func kilnFixtureTree(t *testing.T) string {
	t.Helper()
	repoRoot := t.TempDir()
	files := map[string]string{
		"code/packages/rust/core/BUILD":      "cargo test\n",
		"code/packages/rust/core/Cargo.toml": "[package]\nname = \"core\"\nversion = \"0.1.0\"\n",

		"code/packages/rust/parser/BUILD": "# needs-toolchain: java\ncargo test\n",
		"code/packages/rust/parser/Cargo.toml": "[package]\nname = \"parser\"\nversion = \"0.1.0\"\n" +
			"[dependencies]\ncore = { path = \"../core\" }\n" +
			"[dev-dependencies]\nsupport = { path = \"../support\" }\n",

		"code/packages/rust/support/BUILD":      "cargo test\n",
		"code/packages/rust/support/Cargo.toml": "[package]\nname = \"support\"\nversion = \"0.1.0\"\n",

		"code/packages/rust/cyc-a/BUILD": "cargo test\n",
		"code/packages/rust/cyc-a/Cargo.toml": "[package]\nname = \"cyc-a\"\nversion = \"0.1.0\"\n" +
			"[dependencies]\ncyc-b = { path = \"../cyc-b\" }\n",
		"code/packages/rust/cyc-b/BUILD": "cargo test\n",
		"code/packages/rust/cyc-b/Cargo.toml": "[package]\nname = \"cyc-b\"\nversion = \"0.1.0\"\n" +
			"[dev-dependencies]\ncyc-a = { path = \"../cyc-a\" }\n",

		"code/packages/rust/unix-only/BUILD_mac_and_linux": "cargo test\n",
		"code/packages/rust/unix-only/Cargo.toml":          "[package]\nname = \"unix-only\"\nversion = \"0.1.0\"\n",

		"code/packages/rust/win-only/BUILD_windows": "cargo test\n",
		"code/packages/rust/win-only/Cargo.toml":    "[package]\nname = \"win-only\"\nversion = \"0.1.0\"\n",

		"code/packages/rust/rng/BUILD":      "cargo test\n",
		"code/packages/rust/rng/Cargo.toml": "[package]\nname = \"rng\"\nversion = \"0.1.0\"\n",
	}
	for rel, content := range files {
		full := filepath.Join(repoRoot, filepath.FromSlash(rel))
		if err := os.MkdirAll(filepath.Dir(full), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(full, []byte(content), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	return repoRoot
}

func kilnExportBytes(t *testing.T, repoRoot string) []byte {
	t.Helper()
	packages, err := discovery.DiscoverPackagesAllPlatforms(filepath.Join(repoRoot, "code"))
	if err != nil {
		t.Fatal(err)
	}
	out := filepath.Join(t.TempDir(), "graph.json")
	if code := emitKilnLegacyGraph(packages, repoRoot, out); code != 0 {
		t.Fatalf("emitKilnLegacyGraph exited %d", code)
	}
	data, err := os.ReadFile(out)
	if err != nil {
		t.Fatal(err)
	}
	return data
}

func TestKilnGraphMatchesGolden(t *testing.T) {
	data := kilnExportBytes(t, kilnFixtureTree(t))
	golden := filepath.Join("testdata", "kiln-legacy-graph.golden.json")
	if *updateKilnGolden {
		if err := os.MkdirAll("testdata", 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(golden, data, 0o644); err != nil {
			t.Fatal(err)
		}
	}
	want, err := os.ReadFile(golden)
	if err != nil {
		t.Fatal(err)
	}
	// Golden files may be checked out with CRLF on Windows.
	want = bytes.ReplaceAll(want, []byte("\r\n"), []byte("\n"))
	if !bytes.Equal(data, want) {
		t.Fatalf("export differs from %s (rerun with -update-kiln-golden after reviewing):\n%s", golden, data)
	}
}

func TestKilnGraphIsDeterministic(t *testing.T) {
	root := kilnFixtureTree(t)
	first := kilnExportBytes(t, root)
	second := kilnExportBytes(t, root)
	if !bytes.Equal(first, second) {
		t.Fatal("two exports of the same tree differ")
	}
}

func TestKilnGraphSemantics(t *testing.T) {
	root := kilnFixtureTree(t)
	packages, err := discovery.DiscoverPackagesAllPlatforms(filepath.Join(root, "code"))
	if err != nil {
		t.Fatal(err)
	}
	graph, err := buildKilnLegacyGraph(packages, root)
	if err != nil {
		t.Fatal(err)
	}
	if graph.Schema != "kiln-legacy-graph/1" {
		t.Fatalf("schema = %q", graph.Schema)
	}
	byName := map[string]kilnLegacyPackage{}
	for _, p := range graph.Packages {
		byName[p.Name] = p
	}
	if len(byName) != 8 {
		t.Fatalf("want every discovered package, got %d", len(byName))
	}

	parser := byName["rust/parser"]
	if !reflect.DeepEqual(parser.Deps["linux"], []string{"rust/core"}) {
		t.Errorf("parser deps = %v", parser.Deps)
	}
	if !reflect.DeepEqual(parser.DevDeps, []string{"rust/support"}) {
		t.Errorf("parser dev_deps = %v", parser.DevDeps)
	}
	if !reflect.DeepEqual(parser.ExtraToolchains, []string{"java"}) {
		t.Errorf("parser extra_toolchains = %v", parser.ExtraToolchains)
	}
	if parser.RelPath != "code/packages/rust/parser" || parser.Language != "rust" {
		t.Errorf("parser identity = %q %q", parser.RelPath, parser.Language)
	}

	// The dev-dependency cycle stays out of the build edges.
	if got := byName["rust/cyc-b"].DevDeps; !reflect.DeepEqual(got, []string{"rust/cyc-a"}) {
		t.Errorf("cyc-b dev_deps = %v", got)
	}
	if got := byName["rust/cyc-b"].Deps["linux"]; !reflect.DeepEqual(got, []string{"rust/cyc-a"}) && len(got) != 0 {
		t.Errorf("cyc-b deps = %v", got)
	}

	name := func(p *string) string {
		if p == nil {
			return "<null>"
		}
		return *p
	}
	unix := byName["rust/unix-only"].BuildFiles
	if name(unix["linux"]) != "BUILD_mac_and_linux" || name(unix["darwin"]) != "BUILD_mac_and_linux" || unix["windows"] != nil {
		t.Errorf("unix-only build_files = %v %v %v", name(unix["linux"]), name(unix["darwin"]), name(unix["windows"]))
	}
	win := byName["rust/win-only"].BuildFiles
	if win["linux"] != nil || win["darwin"] != nil || name(win["windows"]) != "BUILD_windows" {
		t.Errorf("win-only build_files = %v %v %v", name(win["linux"]), name(win["darwin"]), name(win["windows"]))
	}
	if name(byName["rust/core"].BuildFiles["darwin"]) != "BUILD" {
		t.Errorf("core darwin build file = %v", name(byName["rust/core"].BuildFiles["darwin"]))
	}

	if got := byName["rust/rng"].RepoInputs; !reflect.DeepEqual(got, []string{"code/specs/fixtures/rng-v1"}) {
		t.Errorf("rng repo_inputs = %v", got)
	}
	if byName["rust/core"].RepoInputs == nil || byName["rust/core"].DevDeps == nil {
		t.Error("empty lists must be non-nil so they serialize as []")
	}
}

// Every path a consumer declares must be one its own predicate accepts (as the
// path itself or as a file beneath it), so the exported list and the
// change-detection predicate cannot drift apart. Every row must also declare
// something, so no consumer exports as unknown today.
func TestRepositoryInputPathsAgreeWithPredicates(t *testing.T) {
	for _, consumer := range repositoryInputConsumers {
		if len(consumer.paths) == 0 && len(consumer.globs) == 0 {
			t.Errorf("%s declares neither paths nor globs", consumer.name)
		}
		for _, p := range consumer.paths {
			if !consumer.matches(p) && !consumer.matches(strings.TrimSuffix(p, "/")+"/x") {
				t.Errorf("%s: declared path %q is not accepted by its predicate", consumer.name, p)
			}
		}
	}
}

// The Mosaic globs must select exactly the files the predicate accepts,
// including near-misses the predicate rejects.
func TestMosaicGlobsAgreeWithPredicate(t *testing.T) {
	candidates := []string{
		"code/packages/mosaic/mosaic-pkg-toolkit/mosaic-package.toml",
		"code/programs/mosaic/task-app/src/TaskApp.mll",
		"code/packages/rust/mosaic-emit-qt/fixtures/x/src/Grid.msl",
		"code/a.mil", "code/x.mll", "code/mosaic-package.toml",
		"code/packages/mosaic/mosaic-pkg-toolkit/README.md",
		"code/x/mosaic-package.toml.bak", "code/x/a.mill", "docs/a.mil",
		"docs/mosaic-package.toml", "mosaic-package.toml", "a.mll",
		"code/x/.mll", "code/x/.mil", "code/x/mosaic-package.toml/y.txt",
		"code/x/sub.mll/readme", "code/x/A.MLL", "code/x/xmosaic-package.toml",
		"code\\packages\\mosaic\\x\\A.mil",
	}
	for _, c := range candidates {
		globHit := false
		for _, g := range mosaicRepositoryGlobs {
			if globmatch.MatchPath(g, c) {
				globHit = true
			}
		}
		if want := isMosaicRepositorySource(c); globHit != want {
			t.Errorf("%q: globs=%v predicate=%v", c, globHit, want)
		}
	}
}

func TestUnknownRepositoryInputsAreFlagged(t *testing.T) {
	saved := repositoryInputConsumers
	defer func() { repositoryInputConsumers = saved }()
	repositoryInputConsumers = []repositoryInputConsumer{{name: "rust/x", language: "rust"}}
	graph, err := buildKilnLegacyGraph([]discovery.Package{{Name: "rust/x", Language: "rust", Path: t.TempDir()}}, t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	if !graph.Packages[0].RepoInputsUnknown {
		t.Fatal("a consumer with no paths or globs must set repo_inputs_unknown")
	}
}
