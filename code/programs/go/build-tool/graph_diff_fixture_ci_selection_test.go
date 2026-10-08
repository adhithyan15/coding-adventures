package main

import (
	"encoding/json"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"sort"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/gitdiff"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/plan"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

const graphDiffFixturePrefix = "code/specs/fixtures/build-tool-v1/cases/"

var graphDiffFixtureConsumers = []struct{ name, path, language string }{
	{"dotnet/programs/build-tool-csharp", "code/programs/dotnet/build-tool-csharp", "csharp"},
	{"dotnet/programs/build-tool-fsharp", "code/programs/dotnet/build-tool-fsharp", "fsharp"},
	{"java/programs/build-tool", "code/programs/java/build-tool", "java"},
	{"kotlin/programs/build-tool", "code/programs/kotlin/build-tool", "kotlin"},
	{"dart/programs/build-tool", "code/programs/dart/build-tool", "dart"},
	{"ocaml/programs/build-tool", "code/programs/ocaml/build-tool", "ocaml"},
	{"go/programs/build-tool", "code/programs/go/build-tool", "go"},
	{"haskell/programs/build-tool", "code/programs/haskell/build-tool", "haskell"},
	{"perl/programs/build-tool", "code/programs/perl/build-tool", "perl"},
	{"python/programs/build-tool", "code/programs/python/build-tool", "python"},
	{"swift/programs/build-tool", "code/programs/swift/build-tool", "swift"},
}

func graphDiffPackages(root string) []discovery.Package {
	packages := make([]discovery.Package, 0, len(graphDiffFixtureConsumers)+1)
	for _, reader := range graphDiffFixtureConsumers {
		packages = append(packages, discovery.Package{
			Name: reader.name, Path: filepath.Join(root, filepath.FromSlash(reader.path)), Language: reader.language,
		})
	}
	return append(packages, discovery.Package{
		Name: "rust/extra", Path: filepath.Join(root, "code", "packages", "rust", "extra"), Language: "rust",
	})
}

func graphDiffNames() []string {
	names := make([]string, 0, len(graphDiffFixtureConsumers))
	for _, reader := range graphDiffFixtureConsumers {
		names = append(names, reader.name)
	}
	sort.Strings(names)
	return names
}

func TestGraphDiffFixtureSelectsExactFlatFamilies(t *testing.T) {
	root := t.TempDir()
	packages := graphDiffPackages(root)
	want := graphDiffNames()
	for _, name := range []string{"graph-empty.json", "diff-selection-transitive.json"} {
		for _, goos := range []string{"linux", "darwin", "windows"} {
			got, err := changedPackageRootsForPlatform([]string{graphDiffFixturePrefix + name}, packages, root, goos)
			if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), want) {
				t.Fatalf("%s %s selected %v, error %v, want %v", goos, name, sortedChangedRoots(got), err, want)
			}
		}
	}
	for _, path := range []string{
		graphDiffFixturePrefix + "graph-.json",
		graphDiffFixturePrefix + "diff-selection-.json",
		graphDiffFixturePrefix + "graph-empty.json.bak",
		graphDiffFixturePrefix + "diff-selection-transitive.JSON",
		graphDiffFixturePrefix + "Graph-empty.json",
		graphDiffFixturePrefix + "nested/graph-empty.json",
		graphDiffFixturePrefix + "graph-nested/empty.json",
		graphDiffFixturePrefix + "graph-empty\\child.json",
		"code/specs/fixtures/build-tool-v1/other/graph-empty.json",
		graphDiffFixturePrefix + "source-collection-extension.json",
	} {
		got, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("lookalike %q selected %v, error %v", path, got, err)
		}
	}
	for _, changed := range [][]string{nil, {}} {
		got, err := changedPackageRootsForPlatform(changed, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("empty diff selected %v, error %v", got, err)
		}
	}
	got, err := changedPackageRootsForPlatform([]string{
		graphDiffFixturePrefix + "graph-empty.json",
		graphDiffFixturePrefix + "diff-selection-transitive.json",
		"code/packages/rust/extra/src/lib.rs",
	}, packages, root, "linux")
	want = append(want, "rust/extra")
	sort.Strings(want)
	if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), want) {
		t.Fatalf("fixture union and ordinary edit = %v, error %v, want %v", sortedChangedRoots(got), err, want)
	}
}

func TestGraphDiffFixtureLanguageAndMissingRoot(t *testing.T) {
	root := t.TempDir()
	packages := graphDiffPackages(root)
	for _, name := range []string{"graph-empty.json", "diff-selection-transitive.json"} {
		path := graphDiffFixturePrefix + name
		for index, reader := range graphDiffFixtureConsumers {
			got, err := changedPackageRootsForPlatformAndLanguage([]string{path}, packages, root, "linux", reader.language)
			if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), []string{reader.name}) {
				t.Fatalf("%s %s selected %v, error %v", reader.language, name, sortedChangedRoots(got), err)
			}
			missing := append([]discovery.Package(nil), packages[:index]...)
			missing = append(missing, packages[index+1:]...)
			for _, language := range []string{"all", reader.language} {
				got, err := changedPackageRootsForPlatformAndLanguage([]string{path}, missing, root, "linux", language)
				if err == nil || !strings.Contains(err.Error(), reader.name) || got != nil {
					t.Fatalf("%s missing %s returned %v, error %v", language, reader.name, got, err)
				}
			}
		}
		got, err := changedPackageRootsForPlatformAndLanguage([]string{path}, packages, root, "linux", "ruby")
		if err != nil || len(got) != 0 {
			t.Fatalf("unrelated language selected %v, error %v", got, err)
		}
	}
}

func TestGraphDiffFixtureRenameRetainsDeletedSource(t *testing.T) {
	root := t.TempDir()
	path := graphDiffFixturePrefix + "graph-empty.json"
	fixture := filepath.Join(root, filepath.FromSlash(path))
	if err := os.MkdirAll(filepath.Dir(fixture), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(fixture, []byte("fixture\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	git := func(args ...string) {
		t.Helper()
		cmd := exec.Command("git", args...)
		cmd.Dir = root
		if output, err := cmd.CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v: %s", args, err, output)
		}
	}
	git("init", "-q")
	git("-c", "user.name=Fixture Test", "-c", "user.email=fixture@example.invalid", "add", ".")
	git("-c", "user.name=Fixture Test", "-c", "user.email=fixture@example.invalid", "commit", "-qm", "baseline")
	git("branch", "base")
	if err := os.Rename(fixture, filepath.Join(filepath.Dir(fixture), "renamed.json")); err != nil {
		t.Fatal(err)
	}
	git("add", "-A")
	git("-c", "user.name=Fixture Test", "-c", "user.email=fixture@example.invalid", "commit", "-qm", "rename")
	changed := gitdiff.GetChangedFiles(root, "base")
	if !containsPath(changed, path) {
		t.Fatalf("rename omitted deleted source: %v", changed)
	}
	got, err := changedPackageRootsForPlatform(changed, graphDiffPackages(root), root, "linux")
	if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), graphDiffNames()) {
		t.Fatalf("renamed source selected %v, error %v", sortedChangedRoots(got), err)
	}
}

func TestGraphDiffFixturePlanIsUnforcedCompleteAndAtomic(t *testing.T) {
	root := t.TempDir()
	packages := graphDiffPackages(root)
	missing := packages[1:]
	missingGraph, err := resolver.ResolveDependencies(missing)
	if err != nil {
		t.Fatal(err)
	}
	path := graphDiffFixturePrefix + "graph-empty.json"
	planPath := filepath.Join(root, "plan.json")
	if code := emitBuildPlan(missing, missingGraph, map[string]bool{}, map[string]bool{},
		[]string{path}, false, nil, "origin/main", root, planPath,
		false, 0, false, "", "all"); code != 1 {
		t.Fatalf("missing consumer plan exit %d, want 1", code)
	}
	if _, err := os.Stat(planPath); !os.IsNotExist(err) {
		t.Fatalf("partial plan written: %v", err)
	}
	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}
	want := graphDiffNames()
	for _, name := range []string{"graph-empty.json", "diff-selection-transitive.json"} {
		path := graphDiffFixturePrefix + name
		changed, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil {
			t.Fatal(err)
		}
		affected := affectedForGraph(graph, changed, nil, false)
		if code := emitBuildPlan(packages, graph, affected, changed,
			[]string{path}, false, nil, "origin/main", root, planPath,
			false, 0, false, "", "all"); code != 0 {
			t.Fatalf("%s plan exit %d", name, code)
		}
		data, err := os.ReadFile(planPath)
		if err != nil {
			t.Fatal(err)
		}
		var built plan.BuildPlan
		if err := json.Unmarshal(data, &built); err != nil {
			t.Fatal(err)
		}
		if built.Force || !reflect.DeepEqual(sortedStrings(built.AffectedPackages), want) {
			t.Fatalf("%s unforced affected %v, force %t, want %v", name, built.AffectedPackages, built.Force, want)
		}
		for _, goos := range []string{"linux", "darwin", "windows"} {
			if got := sortedStrings(built.StateForPlatform(goos).AffectedPackages); !reflect.DeepEqual(got, want) {
				t.Fatalf("%s %s affected %v, want %v", name, goos, got, want)
			}
		}
		wantTools := map[string]bool{}
		for _, reader := range graphDiffFixtureConsumers {
			language := reader.language
			if language == "csharp" || language == "fsharp" {
				language = "dotnet"
			}
			wantTools[language] = true
		}
		for toolchain, needed := range built.LanguagesNeeded {
			if needed != wantTools[toolchain] {
				t.Errorf("%s toolchain %s = %t, want %t", name, toolchain, needed, wantTools[toolchain])
			}
		}
		for toolchain := range wantTools {
			if !built.LanguagesNeeded[toolchain] {
				t.Errorf("%s missing toolchain %s", name, toolchain)
			}
		}
	}
}

func TestGraphDiffFixtureCheckedCorpusAndNativeReaders(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	caseDir := filepath.Join(root, "code", "specs", "fixtures", "build-tool-v1", "cases")
	entries, err := os.ReadDir(caseDir)
	if err != nil {
		t.Fatal(err)
	}
	counts := map[string]int{"graph": 0, "diff-selection": 0}
	for _, entry := range entries {
		if entry.IsDir() || !strings.HasSuffix(entry.Name(), ".json") {
			continue
		}
		for family := range counts {
			if strings.HasPrefix(entry.Name(), family+"-") {
				counts[family]++
			}
		}
	}
	if counts["graph"] != 8 || counts["diff-selection"] != 12 {
		t.Fatalf("checked graph/diff fixture counts = %v", counts)
	}
	// Independent native test sources provide the reader evidence. The JVM,
	// Dart, and OCaml cores dispatch by decoded domain rather than filenames.
	readers := []struct {
		path    string
		markers []string
	}{
		{"dotnet/build-tool-csharp/tests/BuildTool.CSharp.Tests/GraphDiffConformanceTests.cs", []string{"graph-*.json", "diff-selection-*.json"}},
		{"dotnet/build-tool-fsharp/tests/BuildTool.FSharp.Tests/GraphDiffConformanceTests.fs", []string{"graph", "diff-selection"}},
		{"java/build-tool/src/test/java/com/codingadventures/buildtool/BuildToolCoreTest.java", []string{"graph/empty", "diff_selection"}},
		{"kotlin/build-tool/src/test/kotlin/com/codingadventures/buildtool/BuildToolCoreTest.kt", []string{"graph/empty", "diff_selection"}},
		{"dart/build-tool/test/build_tool_core_test.dart", []string{"graph/empty", "diff_selection"}},
		{"ocaml/build-tool/test/test_build_tool_core.ml", []string{"graph/empty", "diff_selection"}},
		{"go/build-tool/internal/graphdiff/graphdiff_test.go", []string{"graph-empty.json", "diff-selection-"}},
		{"haskell/build-tool/test/GraphDiffSpec.hs", []string{"graph-empty.json", "diff-selection-"}},
		{"perl/build-tool/t/15-graph-diff.t", []string{"graph-empty.json", "diff-selection-"}},
		{"python/build-tool/tests/test_graph_diff.py", []string{"graph-empty.json", "diff-selection-"}},
		{"swift/build-tool/Tests/BuildToolCoreTests/GraphDiffConformanceTests.swift", []string{"graph-empty.json", "diff-selection-"}},
	}
	if len(readers) != len(graphDiffFixtureConsumers) {
		t.Fatalf("native reader roster %d differs from selected roots %d", len(readers), len(graphDiffFixtureConsumers))
	}
	for index, reader := range readers {
		path := filepath.Join(root, "code", "programs", filepath.FromSlash(reader.path))
		data, err := os.ReadFile(path)
		if err != nil {
			t.Fatal(err)
		}
		if !strings.HasPrefix(graphDiffFixtureConsumers[index].path, "code/programs/"+strings.Split(reader.path, "/")[0]+"/") {
			t.Fatalf("reader %q does not correspond to selected root %q", reader.path, graphDiffFixtureConsumers[index].path)
		}
		for _, marker := range reader.markers {
			if !strings.Contains(string(data), marker) {
				t.Errorf("native reader %s lacks %q", reader.path, marker)
			}
		}
	}
}

func TestGraphDiffFixtureConsumersResolveThroughDiscovery(t *testing.T) {
	root := t.TempDir()
	for _, reader := range graphDiffFixtureConsumers {
		dir := filepath.Join(root, filepath.FromSlash(reader.path))
		if err := os.MkdirAll(dir, 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(dir, "BUILD"), []byte("echo native fixture\n"), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	packages, err := discovery.DiscoverPackages(filepath.Join(root, "code"))
	if err != nil {
		t.Fatal(err)
	}
	for _, goos := range []string{"linux", "darwin", "windows"} {
		got, err := changedPackageRootsForPlatform([]string{graphDiffFixturePrefix + "graph-empty.json"}, packages, root, goos)
		if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), graphDiffNames()) {
			t.Fatalf("%s discovered roots %v, error %v", goos, sortedChangedRoots(got), err)
		}
	}
}
