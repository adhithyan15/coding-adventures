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

const toolchainDetectionFixturePath = "code/specs/fixtures/build-tool-v1/cases/toolchain-detection-shared.json"

// The synthetic registry mirrors direct native fronts, with one unrelated
// package to prove that a fixture edit never becomes a forced full build.
func toolchainDetectionFixturePackages(root string) []discovery.Package {
	return discoveryRegistryPackages(root)
}

func toolchainDetectionFixtureNames() []string {
	want := make([]string, 0, len(discoveryRegistryConsumerRoots))
	for _, consumer := range discoveryRegistryConsumerRoots {
		want = append(want, consumer.name)
	}
	sort.Strings(want)
	return want
}

func TestToolchainFixtureSelectsExactFlatFamilyOnEveryPlatform(t *testing.T) {
	root := t.TempDir()
	packages := toolchainDetectionFixturePackages(root)
	want := toolchainDetectionFixtureNames()
	for _, path := range []string{
		toolchainDetectionFixturePath,
		"code/specs/fixtures/build-tool-v1/cases/toolchain-detection-new-case.json",
	} {
		for _, goos := range []string{"linux", "darwin", "windows"} {
			got, err := changedPackageRootsForPlatform([]string{path}, packages, root, goos)
			if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), want) {
				t.Fatalf("%s %q roots = %v, error = %v, want %v", goos, path, sortedChangedRoots(got), err, want)
			}
		}
	}
	for _, path := range []string{
		"code/specs/fixtures/build-tool-v1/cases/toolchain-detection.json",
		"code/specs/fixtures/build-tool-v1/cases/toolchain-detection-.json",
		"code/specs/fixtures/build-tool-v1/cases/toolchain-detection-shared.json.bak",
		"code/specs/fixtures/build-tool-v1/cases/Toolchain-detection-shared.json",
		"code/specs/fixtures/build-tool-v1/cases/nested/toolchain-detection-shared.json",
		"code/specs/fixtures/build-tool-v1/cases/toolchain-detection-nested/child.json",
		"code/specs/fixtures/build-tool-v1/other/toolchain-detection-shared.json",
		"code/specs/fixtures/build-tool-v1/cases/toolchain-detection-shared\\child.json",
		"code/specs/fixtures/build-tool-v1/cases/ci-gate-selection-shared.json",
	} {
		got, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("lookalike %q selected %v, error = %v", path, got, err)
		}
	}
	for _, changed := range [][]string{nil, {}} {
		got, err := changedPackageRootsForPlatform(changed, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("empty diff selected %v, error = %v", got, err)
		}
	}
	got, err := changedPackageRootsForPlatform(
		[]string{toolchainDetectionFixturePath, "code/packages/rust/extra/src/lib.rs"},
		packages, root, "linux",
	)
	if err != nil || len(got) != len(want)+1 || !got["rust/extra"] {
		t.Fatalf("fixture and ordinary edit union = %v, error = %v", got, err)
	}
}

func TestToolchainFixtureLanguageFilterAndMissingRootFailClosed(t *testing.T) {
	root := t.TempDir()
	packages := toolchainDetectionFixturePackages(root)
	for _, consumer := range discoveryRegistryConsumerRoots {
		got, err := changedPackageRootsForPlatformAndLanguage(
			[]string{toolchainDetectionFixturePath}, packages, root, "linux", consumer.lang,
		)
		if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), []string{consumer.name}) {
			t.Fatalf("%s roots = %v, error = %v", consumer.lang, got, err)
		}
	}
	got, err := changedPackageRootsForPlatformAndLanguage(
		[]string{toolchainDetectionFixturePath}, packages, root, "linux", "dart",
	)
	if err != nil || len(got) != 0 {
		t.Fatalf("unrelated language roots = %v, error = %v", got, err)
	}
	for index, consumer := range discoveryRegistryConsumerRoots {
		missing := append([]discovery.Package(nil), packages[:index]...)
		missing = append(missing, packages[index+1:]...)
		for _, language := range []string{"all", consumer.lang} {
			got, err := changedPackageRootsForPlatformAndLanguage(
				[]string{toolchainDetectionFixturePath}, missing, root, "linux", language,
			)
			if err == nil || !strings.Contains(err.Error(), consumer.name) || got != nil {
				t.Fatalf("%s missing %s returned roots = %v, error = %v", language, consumer.name, got, err)
			}
		}
	}
}

func TestToolchainFixtureRenameKeepsDeletedSource(t *testing.T) {
	root := t.TempDir()
	fixture := filepath.Join(root, filepath.FromSlash(toolchainDetectionFixturePath))
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
	if !containsPath(changed, toolchainDetectionFixturePath) {
		t.Fatalf("rename omitted deleted source: %v", changed)
	}
	selected, err := changedPackageRootsForPlatform(changed, toolchainDetectionFixturePackages(root), root, "linux")
	if err != nil || !reflect.DeepEqual(sortedChangedRoots(selected), toolchainDetectionFixtureNames()) {
		t.Fatalf("renamed source selected %v, error = %v", selected, err)
	}
}

func TestToolchainFixturePlanIsUnforcedCompleteAndAtomic(t *testing.T) {
	root := t.TempDir()
	packages := toolchainDetectionFixturePackages(root)
	planPath := filepath.Join(root, "plan.json")
	missing := packages[1:]
	missingGraph, err := resolver.ResolveDependencies(missing)
	if err != nil {
		t.Fatal(err)
	}
	if code := emitBuildPlan(missing, missingGraph, map[string]bool{}, map[string]bool{},
		[]string{toolchainDetectionFixturePath}, false, nil, "origin/main", root, planPath,
		false, 0, false, "", "all"); code != 1 {
		t.Fatalf("missing consumer plan exit = %d, want 1", code)
	}
	if _, err := os.Stat(planPath); !os.IsNotExist(err) {
		t.Fatalf("partial plan written: %v", err)
	}
	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}
	changed, err := changedPackageRootsForPlatform([]string{toolchainDetectionFixturePath}, packages, root, "linux")
	if err != nil {
		t.Fatal(err)
	}
	affected := affectedForGraph(graph, changed, nil, false)
	if code := emitBuildPlan(packages, graph, affected, changed,
		[]string{toolchainDetectionFixturePath}, false, nil, "origin/main", root, planPath,
		false, 0, false, "", "all"); code != 0 {
		t.Fatalf("complete plan exit = %d", code)
	}
	data, err := os.ReadFile(planPath)
	if err != nil {
		t.Fatal(err)
	}
	var built plan.BuildPlan
	if err := json.Unmarshal(data, &built); err != nil {
		t.Fatal(err)
	}
	want := toolchainDetectionFixtureNames()
	if built.Force || !reflect.DeepEqual(sortedStrings(built.AffectedPackages), want) {
		t.Fatalf("unforced affected roots = %v, force = %t", built.AffectedPackages, built.Force)
	}
	for _, goos := range []string{"linux", "darwin", "windows"} {
		if got := sortedStrings(built.StateForPlatform(goos).AffectedPackages); !reflect.DeepEqual(got, want) {
			t.Fatalf("%s affected roots = %v, want %v", goos, got, want)
		}
	}
	for _, toolchain := range []string{"dotnet", "elixir", "go", "haskell", "lua", "perl", "python", "ruby", "rust", "swift", "typescript"} {
		if !built.LanguagesNeeded[toolchain] {
			t.Errorf("missing native toolchain %s from %v", toolchain, built.LanguagesNeeded)
		}
	}
}

func TestToolchainFixtureConsumerMapTracksNativeReaders(t *testing.T) {
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
		switch filepath.Ext(entry.Name()) {
		case ".go", ".cs", ".exs", ".fs", ".hs", ".lua", ".t", ".py", ".rb", ".rs", ".swift", ".ts":
		default:
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
		pathWithSlashes := "/" + filepath.ToSlash(rel) + "/"
		isTest := strings.Contains(pathWithSlashes, "/tests/") ||
			strings.Contains(pathWithSlashes, "/test/") ||
			strings.Contains(pathWithSlashes, "/t/") ||
			strings.Contains(entry.Name(), "Test") || strings.Contains(entry.Name(), "test") ||
			(strings.HasSuffix(filepath.ToSlash(rel), "rust/build-tool/src/toolchain_detection.rs") && bytes.Contains(data, []byte("#[cfg(test)]")))
		if entry.Name() == "toolchain_detection_fixture_ci_selection_test.go" || !isTest {
			return nil
		}
		if bytes.Contains(data, []byte("toolchain-detection-")) {
			references[parts[2]+"/programs/"+parts[3]] = true
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if got, want := sortedChangedRoots(references), toolchainDetectionFixtureNames(); !reflect.DeepEqual(got, want) {
		t.Fatalf("native toolchain readers = %v, expected consumers = %v", got, want)
	}
}

func TestToolchainFixtureConsumersResolveThroughDiscovery(t *testing.T) {
	root := t.TempDir()
	for _, consumer := range discoveryRegistryConsumerRoots {
		dir := filepath.Join(root, filepath.FromSlash(consumer.path))
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
		got, err := changedPackageRootsForPlatform([]string{toolchainDetectionFixturePath}, packages, root, goos)
		if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), toolchainDetectionFixtureNames()) {
			t.Fatalf("%s discovered native roots = %v, error = %v", goos, got, err)
		}
	}
}
