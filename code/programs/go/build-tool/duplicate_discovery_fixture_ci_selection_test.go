package main

import (
	"bytes"
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

var duplicateDiscoveryConsumerRoots = []struct {
	name string
	path string
	lang string
}{
	{"elixir/programs/build-tool", "code/programs/elixir/build-tool", "elixir"},
	{"go/programs/build-tool", "code/programs/go/build-tool", "go"},
	{"ruby/programs/build-tool", "code/programs/ruby/build-tool", "ruby"},
	{"rust/programs/build-tool", "code/programs/rust/build-tool", "rust"},
	{"swift/programs/build-tool", "code/programs/swift/build-tool", "swift"},
	{"typescript/programs/build-tool", "code/programs/typescript/build-tool", "typescript"},
}

func duplicateDiscoveryPackages(root string) []discovery.Package {
	packages := make([]discovery.Package, 0, len(duplicateDiscoveryConsumerRoots)+2)
	for _, consumer := range duplicateDiscoveryConsumerRoots {
		pkg := discovery.Package{
			Name: consumer.name, Path: filepath.Join(root, filepath.FromSlash(consumer.path)), Language: consumer.lang,
		}
		if consumer.name == "go/programs/build-tool" {
			pkg.BuildContent = "# build-tool: deps=go/fixture-helper\n"
		}
		packages = append(packages, pkg)
	}
	packages = append(packages,
		discovery.Package{Name: "go/fixture-helper", Path: filepath.Join(root, "code", "packages", "go", "fixture-helper"), Language: "go"},
		discovery.Package{Name: "rust/extra", Path: filepath.Join(root, "code", "packages", "rust", "extra"), Language: "rust"},
	)
	return packages
}

func duplicateDiscoveryNames(roots map[string]bool) []string {
	result := make([]string, 0, len(roots))
	for name, selected := range roots {
		if selected {
			result = append(result, name)
		}
	}
	sort.Strings(result)
	return result
}

func duplicateDiscoveryWantedConsumers() []string {
	want := make([]string, 0, len(duplicateDiscoveryConsumerRoots))
	for _, consumer := range duplicateDiscoveryConsumerRoots {
		want = append(want, consumer.name)
	}
	sort.Strings(want)
	return want
}

func TestDuplicateDiscoveryFixtureSelectsExactNativeConsumersOnEveryPlatform(t *testing.T) {
	root := t.TempDir()
	packages := duplicateDiscoveryPackages(root)
	want := duplicateDiscoveryWantedConsumers()
	for _, goos := range []string{"linux", "darwin", "windows"} {
		got, err := changedPackageRootsForPlatform([]string{duplicateDiscoveryFixturePath}, packages, root, goos)
		if err != nil {
			t.Fatalf("%s: %v", goos, err)
		}
		if names := duplicateDiscoveryNames(got); !reflect.DeepEqual(names, want) {
			t.Fatalf("%s: roots = %v, want %v", goos, names, want)
		}
		if got["go/fixture-helper"] || got["rust/extra"] {
			t.Fatalf("%s: fixture change forced a non-reader root: %v", goos, got)
		}
	}
}

func TestDuplicateDiscoveryFixtureSelectionIsExactAndUnionsPackageEdits(t *testing.T) {
	root := t.TempDir()
	packages := duplicateDiscoveryPackages(root)
	lookalikes := []string{
		"code/specs/fixtures/build-tool-v1/cases/discovery-duplicate-identity-.json",
		"code/specs/fixtures/build-tool-v1/cases/discovery-duplicate-identity.json.bak",
		"code/specs/fixtures/build-tool-v1/cases/nested/discovery-duplicate-identity.json",
		"code/specs/fixtures/build-tool-v1/cases/Discovery-duplicate-identity.json",
		"code\\specs\\fixtures\\build-tool-v1\\cases\\discovery-duplicate-identity.json",
		"code/specs/fixtures/build-tool-v1/cases/discovery-duplicate-identity",
		"code/specs/fixtures/build-tool-v1/cases/discovery-language-registry.json.example",
	}
	for _, path := range lookalikes {
		got, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("lookalike %q selected roots = %v, error = %v", path, got, err)
		}
	}

	got, err := changedPackageRootsForPlatform([]string{
		duplicateDiscoveryFixturePath,
		"code/packages/rust/extra/src/lib.rs",
	}, packages, root, "linux")
	if err != nil {
		t.Fatal(err)
	}
	want := append(duplicateDiscoveryWantedConsumers(), "rust/extra")
	sort.Strings(want)
	if names := duplicateDiscoveryNames(got); !reflect.DeepEqual(names, want) {
		t.Fatalf("fixture and package union = %v, want %v", names, want)
	}
}

func TestDuplicateDiscoveryFixtureSelectionHonorsLanguageFilter(t *testing.T) {
	root := t.TempDir()
	packages := duplicateDiscoveryPackages(root)
	for _, consumer := range duplicateDiscoveryConsumerRoots {
		got, err := changedPackageRootsForPlatformAndLanguage(
			[]string{duplicateDiscoveryFixturePath}, packages, root, "linux", consumer.lang,
		)
		if err != nil || !reflect.DeepEqual(duplicateDiscoveryNames(got), []string{consumer.name}) {
			t.Fatalf("%s roots = %v, error = %v", consumer.lang, got, err)
		}
	}
	got, err := changedPackageRootsForPlatformAndLanguage(
		[]string{duplicateDiscoveryFixturePath}, packages, root, "linux", "haskell",
	)
	if err != nil || len(got) != 0 {
		t.Fatalf("unregistered language selected roots = %v, error = %v", got, err)
	}
}

func TestDuplicateDiscoveryFixtureSelectionFailsClosedWhenConsumerMissing(t *testing.T) {
	root := t.TempDir()
	packages := duplicateDiscoveryPackages(root)[1:]
	got, err := changedPackageRootsForPlatform([]string{duplicateDiscoveryFixturePath}, packages, root, "linux")
	if err == nil || got != nil {
		t.Fatalf("missing native consumer returned roots = %v, error = %v", got, err)
	}
	if !strings.Contains(err.Error(), `duplicate-discovery fixture consumer "elixir/programs/build-tool" is missing`) {
		t.Fatalf("unstable missing-consumer error: %v", err)
	}
}

func TestDuplicateDiscoveryFixtureRenameKeepsDeletedSourcePath(t *testing.T) {
	root := t.TempDir()
	fixture := filepath.Join(root, filepath.FromSlash(duplicateDiscoveryFixturePath))
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
	if err := os.Rename(fixture, filepath.Join(filepath.Dir(fixture), "discovery-duplicate-identity-renamed.json")); err != nil {
		t.Fatal(err)
	}
	git("add", "-A")
	git("-c", "user.name=Fixture Test", "-c", "user.email=fixture@example.invalid", "commit", "-qm", "rename")
	changed := gitdiff.GetChangedFiles(root, "base")
	if !containsPath(changed, duplicateDiscoveryFixturePath) {
		t.Fatalf("rename diff omitted old fixture path: %v", changed)
	}
	got, err := changedPackageRootsForPlatform(changed, duplicateDiscoveryPackages(root), root, "linux")
	if err != nil || !reflect.DeepEqual(duplicateDiscoveryNames(got), duplicateDiscoveryWantedConsumers()) {
		t.Fatalf("rename source roots = %v, error = %v", got, err)
	}
}

func TestDuplicateDiscoveryFixtureBuildPlanFailsAtomicallyWhenConsumerMissing(t *testing.T) {
	root := t.TempDir()
	packages := duplicateDiscoveryPackages(root)[1:]
	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}
	planPath := filepath.Join(root, "missing-consumer-plan.json")
	if code := emitBuildPlan(
		packages, graph, map[string]bool{}, map[string]bool{},
		[]string{duplicateDiscoveryFixturePath}, false, nil, "origin/main",
		root, planPath, false, 0, false, "", "all",
	); code != 1 {
		t.Fatalf("missing fixture consumer plan exit = %d, want 1", code)
	}
	if _, err := os.Stat(planPath); !os.IsNotExist(err) {
		t.Fatalf("plan unexpectedly written on missing consumer: %v", err)
	}
}

func TestDuplicateDiscoveryFixtureBuildPlanKeepsPrerequisitesPlatformsAndToolchains(t *testing.T) {
	root := t.TempDir()
	packages := duplicateDiscoveryPackages(root)
	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}
	changed, err := changedPackageRootsForPlatform([]string{duplicateDiscoveryFixturePath}, packages, root, "linux")
	if err != nil {
		t.Fatal(err)
	}
	affected := affectedForGraph(graph, changed, nil, false)
	if !affected["go/fixture-helper"] {
		t.Fatalf("fixture reader prerequisite missing from affected closure: %v", affected)
	}
	planPath := filepath.Join(root, "plan.json")
	if code := emitBuildPlan(
		packages, graph, affected, changed, []string{duplicateDiscoveryFixturePath},
		false, nil, "origin/main", root, planPath, false, 0, false, "", "all",
	); code != 0 {
		t.Fatalf("emitBuildPlan exit = %d", code)
	}
	data, err := os.ReadFile(planPath)
	if err != nil {
		t.Fatal(err)
	}
	var built plan.BuildPlan
	if err := json.Unmarshal(data, &built); err != nil {
		t.Fatal(err)
	}
	wantAffected := append(duplicateDiscoveryWantedConsumers(), "go/fixture-helper")
	sort.Strings(wantAffected)
	if built.Force || !reflect.DeepEqual(built.AffectedPackages, wantAffected) {
		t.Fatalf("fixture plan affected = %v, want %v", built.AffectedPackages, wantAffected)
	}
	for _, goos := range []string{"linux", "darwin", "windows"} {
		if got := built.StateForPlatform(goos).AffectedPackages; !reflect.DeepEqual(got, wantAffected) {
			t.Fatalf("%s affected packages = %v, want %v", goos, got, wantAffected)
		}
	}
	for _, toolchain := range []string{"elixir", "go", "ruby", "rust", "swift", "typescript"} {
		if !built.LanguagesNeeded[toolchain] {
			t.Errorf("missing native fixture toolchain %s", toolchain)
		}
	}
}

func TestDuplicateDiscoveryFixtureConsumerMapTracksNativeReferences(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	programs := filepath.Join(root, "code", "programs")
	references := map[string]bool{}
	err := filepath.WalkDir(programs, func(path string, entry fs.DirEntry, walkErr error) error {
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
		switch filepath.Ext(entry.Name()) {
		case ".go", ".exs", ".rb", ".rs", ".swift", ".ts":
		default:
			return nil
		}
		rel, err := filepath.Rel(root, path)
		if err != nil {
			return err
		}
		portable := filepath.ToSlash(rel)
		parts := strings.Split(portable, "/")
		if len(parts) < 5 || parts[0] != "code" || parts[1] != "programs" || !strings.HasPrefix(parts[3], "build-tool") {
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
		isTest := strings.Contains("/"+portable+"/", "/tests/") ||
			strings.Contains("/"+portable+"/", "/test/") ||
			strings.Contains(entry.Name(), "Test") || strings.Contains(entry.Name(), "test") ||
			(strings.HasSuffix(portable, "rust/build-tool/src/discovery.rs") && bytes.Contains(data, []byte("#[cfg(test)]")))
		if entry.Name() == "duplicate_discovery_fixture_ci_selection_test.go" || !isTest {
			return nil
		}
		if bytes.Contains(data, []byte("discovery-duplicate-identity.json")) {
			references[parts[2]+"/programs/"+parts[3]] = true
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if got, want := duplicateDiscoveryNames(references), duplicateDiscoveryWantedConsumers(); !reflect.DeepEqual(got, want) {
		t.Fatalf("direct native references = %v, detector consumers = %v", got, want)
	}
}
