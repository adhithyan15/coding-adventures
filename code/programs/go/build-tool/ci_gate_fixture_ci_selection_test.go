package main

import (
	"encoding/json"
	"io/fs"
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

const ciGateFixturePath = "code/specs/fixtures/build-tool-v1/cases/ci-gate-selection-shared-pattern-at-limit.json"

func sortedStrings(values []string) []string {
	result := append([]string(nil), values...)
	sort.Strings(result)
	return result
}

func ciGateFixturePackages(root string) []discovery.Package {
	return []discovery.Package{
		{Name: "dotnet/programs/build-tool-csharp", Path: filepath.Join(root, "code", "programs", "dotnet", "build-tool-csharp"), Language: "csharp"},
		{Name: "dotnet/programs/build-tool-fsharp", Path: filepath.Join(root, "code", "programs", "dotnet", "build-tool-fsharp"), Language: "fsharp"},
		{Name: "go/programs/build-tool", Path: filepath.Join(root, "code", "programs", "go", "build-tool"), Language: "go"},
		{Name: "python/programs/build-tool", Path: filepath.Join(root, "code", "programs", "python", "build-tool"), Language: "python"},
		{Name: "rust/extra", Path: filepath.Join(root, "code", "packages", "rust", "extra"), Language: "rust"},
	}
}

func TestCIGateFixtureNativeSelectionIsExactAndPlatformIndependent(t *testing.T) {
	root := t.TempDir()
	packages := ciGateFixturePackages(root)
	want := []string{"dotnet/programs/build-tool-csharp", "dotnet/programs/build-tool-fsharp", "go/programs/build-tool", "python/programs/build-tool"}
	for _, path := range []string{ciGateFixturePath, "code/specs/fixtures/build-tool-v1/cases/ci-gate-selection-new-case.json"} {
		for _, goos := range []string{"linux", "darwin", "windows"} {
			got, err := changedPackageRootsForPlatform([]string{path}, packages, root, goos)
			if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), want) {
				t.Fatalf("%s %s native roots = %v, error = %v, want %v", goos, path, got, err, want)
			}
		}
	}
	for _, path := range []string{
		"code/specs/fixtures/build-tool-v1/cases/ci-gate-selection.json",
		"code/specs/fixtures/build-tool-v1/cases/ci-gate-selection-.json",
		"code/specs/fixtures/build-tool-v1/cases/ci-gate-selection-sibling.json.bak",
		"code/specs/fixtures/build-tool-v1/cases/nested/ci-gate-selection-sibling.json",
		"code/specs/fixtures/build-tool-v1/cases/CI-gate-selection-sibling.json",
		"code/specs/fixtures/build-tool-v1/cases/discovery-language-registry-example.json",
	} {
		got, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("near path %q selected %v, error = %v", path, got, err)
		}
	}
	for _, changed := range [][]string{nil, {}} {
		got, err := changedPackageRootsForPlatform(changed, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("empty diff selected %v, error = %v", got, err)
		}
	}
	got, err := changedPackageRootsForPlatform([]string{ciGateFixturePath, "code/packages/rust/extra/src/lib.rs"}, packages, root, "linux")
	if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), []string{"dotnet/programs/build-tool-csharp", "dotnet/programs/build-tool-fsharp", "go/programs/build-tool", "python/programs/build-tool", "rust/extra"}) {
		t.Fatalf("native and ordinary root union = %v, error = %v", got, err)
	}
}

func TestCIGateFixtureNativeSelectionHonorsLanguageAndFailsClosed(t *testing.T) {
	root := t.TempDir()
	packages := ciGateFixturePackages(root)
	for _, tc := range []struct {
		language string
		want     []string
	}{
		{"csharp", []string{"dotnet/programs/build-tool-csharp"}},
		{"fsharp", []string{"dotnet/programs/build-tool-fsharp"}},
		{"go", []string{"go/programs/build-tool"}},
		{"python", []string{"python/programs/build-tool"}},
		{"rust", []string{}},
	} {
		got, err := changedPackageRootsForPlatformAndLanguage([]string{ciGateFixturePath}, packages, root, "linux", tc.language)
		if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), tc.want) {
			t.Fatalf("%s roots = %v, error = %v, want %v", tc.language, got, err, tc.want)
		}
	}
	for _, tc := range []struct {
		language string
		packages []discovery.Package
		missing  string
	}{
		{"all", packages[1:], "dotnet/programs/build-tool-csharp"},
		{"csharp", packages[1:], "dotnet/programs/build-tool-csharp"},
		{"fsharp", packages[:1], "dotnet/programs/build-tool-fsharp"},
		{"go", packages[:2], "go/programs/build-tool"},
		{"python", packages[:3], "python/programs/build-tool"},
	} {
		_, err := changedPackageRootsForPlatformAndLanguage([]string{ciGateFixturePath}, tc.packages, root, "linux", tc.language)
		if err == nil || !strings.Contains(err.Error(), tc.missing) {
			t.Fatalf("missing %s consumer error = %v", tc.missing, err)
		}
	}
}

func TestCIGateFixtureRenameRetainsDeletedSource(t *testing.T) {
	root := t.TempDir()
	fixture := filepath.Join(root, filepath.FromSlash(ciGateFixturePath))
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
	if !containsPath(changed, ciGateFixturePath) {
		t.Fatalf("rename omitted old path: %v", changed)
	}
	selected, err := changedPackageRootsForPlatform(changed, ciGateFixturePackages(root), root, "linux")
	if err != nil || len(selected) != 4 {
		t.Fatalf("renamed source selected %v, error = %v", selected, err)
	}
}

func TestCIGateFixtureNativePlanFailsClosedAndSelectsToolchains(t *testing.T) {
	root := t.TempDir()
	packages := ciGateFixturePackages(root)
	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}
	planPath := filepath.Join(root, "plan.json")
	missing := packages[1:]
	missingGraph, err := resolver.ResolveDependencies(missing)
	if err != nil {
		t.Fatal(err)
	}
	if code := emitBuildPlan(missing, missingGraph, map[string]bool{}, map[string]bool{}, []string{ciGateFixturePath}, false, nil, "origin/main", root, planPath, false, 0, false, "", "all"); code != 1 {
		t.Fatalf("missing consumer exit = %d, want 1", code)
	}
	if _, err := os.Stat(planPath); !os.IsNotExist(err) {
		t.Fatalf("partial plan written: %v", err)
	}
	changed, err := changedPackageRootsForPlatform([]string{ciGateFixturePath}, packages, root, "linux")
	if err != nil {
		t.Fatal(err)
	}
	affected := affectedForGraph(graph, changed, nil, false)
	if code := emitBuildPlan(packages, graph, affected, changed, []string{ciGateFixturePath}, false, nil, "origin/main", root, planPath, false, 0, false, "", "all"); code != 0 {
		t.Fatalf("plan exit = %d", code)
	}
	data, err := os.ReadFile(planPath)
	if err != nil {
		t.Fatal(err)
	}
	var built plan.BuildPlan
	if err := json.Unmarshal(data, &built); err != nil {
		t.Fatal(err)
	}
	if built.Force || !reflect.DeepEqual(sortedStrings(built.AffectedPackages), []string{"dotnet/programs/build-tool-csharp", "dotnet/programs/build-tool-fsharp", "go/programs/build-tool", "python/programs/build-tool"}) {
		t.Fatalf("unforced affected roots = %v, force = %t", built.AffectedPackages, built.Force)
	}
	for _, goos := range []string{"linux", "darwin", "windows"} {
		if got := built.StateForPlatform(goos).AffectedPackages; !reflect.DeepEqual(sortedStrings(got), []string{"dotnet/programs/build-tool-csharp", "dotnet/programs/build-tool-fsharp", "go/programs/build-tool", "python/programs/build-tool"}) {
			t.Fatalf("%s roots = %v", goos, got)
		}
	}
	if !built.LanguagesNeeded["dotnet"] || !built.LanguagesNeeded["go"] || !built.LanguagesNeeded["python"] || built.LanguagesNeeded["rust"] {
		t.Fatalf("toolchains = %v", built.LanguagesNeeded)
	}
}

func TestCIGateFixtureConsumerMapTracksNativeReaders(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	references := map[string]bool{}
	err := filepath.WalkDir(filepath.Join(root, "code", "programs"), func(path string, entry fs.DirEntry, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		if entry.IsDir() {
			switch entry.Name() {
			case "node_modules", ".build", "target", "bin", "obj", "coverage", ".venv":
				return filepath.SkipDir
			}
			return nil
		}
		if !strings.HasSuffix(entry.Name(), "_test.go") && !strings.HasPrefix(entry.Name(), "test_") && !strings.HasSuffix(entry.Name(), "Tests.cs") && !strings.HasSuffix(entry.Name(), "Tests.fs") {
			return nil
		}
		if filepath.Ext(entry.Name()) != ".go" && filepath.Ext(entry.Name()) != ".py" && filepath.Ext(entry.Name()) != ".cs" && filepath.Ext(entry.Name()) != ".fs" {
			return nil
		}
		rel, err := filepath.Rel(root, path)
		if err != nil {
			return err
		}
		parts := strings.Split(filepath.ToSlash(rel), "/")
		if len(parts) < 5 || parts[0] != "code" || parts[1] != "programs" || !strings.HasPrefix(parts[3], "build-tool") {
			return nil
		}
		if entry.Name() == "ci_gate_fixture_ci_selection_test.go" {
			return nil
		}
		info, err := entry.Info()
		if err != nil {
			return err
		}
		if info.Size() > 1<<20 {
			return nil
		}
		data, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		if strings.Contains(string(data), "ci-gate-selection-*.json") {
			references[parts[2]+"/programs/"+parts[3]] = true
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	want := make([]string, 0, len(ciGateFixtureConsumers))
	for _, consumer := range ciGateFixtureConsumers {
		want = append(want, consumer.name)
	}
	if got := sortedChangedRoots(references); !reflect.DeepEqual(got, want) {
		t.Fatalf("native readers = %v, detector consumers = %v", got, want)
	}
}

func TestCIGateFixtureConsumersResolveThroughDiscovery(t *testing.T) {
	root := t.TempDir()
	for _, consumer := range ciGateFixturePackages(root)[:4] {
		if err := os.MkdirAll(consumer.Path, 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(consumer.Path, "BUILD"), []byte("echo native fixture\n"), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	packages, err := discovery.DiscoverPackages(filepath.Join(root, "code"))
	if err != nil {
		t.Fatal(err)
	}
	for _, goos := range []string{"linux", "darwin", "windows"} {
		changed, err := changedPackageRootsForPlatform([]string{ciGateFixturePath}, packages, root, goos)
		if err != nil || !reflect.DeepEqual(sortedChangedRoots(changed), []string{"dotnet/programs/build-tool-csharp", "dotnet/programs/build-tool-fsharp", "go/programs/build-tool", "python/programs/build-tool"}) {
			t.Fatalf("%s discovery roots = %v, error = %v", goos, changed, err)
		}
	}
}
