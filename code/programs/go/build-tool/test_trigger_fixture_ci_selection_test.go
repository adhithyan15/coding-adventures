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

const testTriggerFixturePath = "code/specs/fixtures/build-tool-v1/cases/test-trigger-selection-one-hop.json"

func testTriggerFixturePackages(root string) []discovery.Package {
	return []discovery.Package{
		{Name: "go/programs/build-tool", Path: filepath.Join(root, "code", "programs", "go", "build-tool"), Language: "go", BuildContent: "# build-tool: deps=go/fixture-helper\n"},
		{Name: "rust/programs/build-tool", Path: filepath.Join(root, "code", "programs", "rust", "build-tool"), Language: "rust"},
		{Name: "go/fixture-helper", Path: filepath.Join(root, "code", "packages", "go", "fixture-helper"), Language: "go"},
		{Name: "python/extra", Path: filepath.Join(root, "code", "packages", "python", "extra"), Language: "python"},
	}
}

func testTriggerSelectedNames(roots map[string]bool) []string {
	result := make([]string, 0, len(roots))
	for name, selected := range roots {
		if selected {
			result = append(result, name)
		}
	}
	sort.Strings(result)
	return result
}

func TestTestTriggerFixtureSelectionIsExactPlatformIndependentAndUnioned(t *testing.T) {
	root := t.TempDir()
	packages := testTriggerFixturePackages(root)
	want := []string{"go/programs/build-tool", "rust/programs/build-tool"}
	for _, path := range []string{
		testTriggerFixturePath,
		"code/specs/fixtures/build-tool-v1/cases/test-trigger-selection-new-case.json",
	} {
		for _, goos := range []string{"linux", "darwin", "windows"} {
			got, err := changedPackageRootsForPlatform([]string{path}, packages, root, goos)
			if err != nil || !reflect.DeepEqual(testTriggerSelectedNames(got), want) {
				t.Fatalf("%s %s roots = %v, error = %v, want %v", goos, path, got, err, want)
			}
		}
	}
	for _, path := range []string{
		"code/specs/fixtures/build-tool-v1/cases/test-trigger-selection-.json",
		"code/specs/fixtures/build-tool-v1/cases/test-trigger-selection-one-hop.json.bak",
		"code/specs/fixtures/build-tool-v1/cases/nested/test-trigger-selection-one-hop.json",
		"code/specs/fixtures/build-tool-v1/cases/Test-trigger-selection-one-hop.json",
		"code\\specs\\fixtures\\build-tool-v1\\cases\\test-trigger-selection-one-hop.json",
		"code/specs/fixtures/build-tool-v1/cases/test-trigger-selection-one-hop",
	} {
		got, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("lookalike %q selected %v, error = %v", path, got, err)
		}
	}
	got, err := changedPackageRootsForPlatform([]string{testTriggerFixturePath, "code/packages/python/extra/src/lib.py"}, packages, root, "linux")
	if err != nil || !reflect.DeepEqual(testTriggerSelectedNames(got), []string{"go/programs/build-tool", "python/extra", "rust/programs/build-tool"}) {
		t.Fatalf("fixture and ordinary union = %v, error = %v", got, err)
	}
}

func TestTestTriggerFixtureSelectionHonorsLanguageAndFailsClosed(t *testing.T) {
	root := t.TempDir()
	packages := testTriggerFixturePackages(root)
	for _, tc := range []struct {
		language string
		want     []string
	}{
		{"go", []string{"go/programs/build-tool"}},
		{"rust", []string{"rust/programs/build-tool"}},
		{"python", []string{}},
	} {
		got, err := changedPackageRootsForPlatformAndLanguage([]string{testTriggerFixturePath}, packages, root, "linux", tc.language)
		if err != nil || !reflect.DeepEqual(testTriggerSelectedNames(got), tc.want) {
			t.Fatalf("%s roots = %v, error = %v, want %v", tc.language, got, err, tc.want)
		}
	}
	for _, tc := range []struct {
		language string
		packages []discovery.Package
		missing  string
	}{
		{"all", packages[1:], "go/programs/build-tool"},
		{"go", packages[1:], "go/programs/build-tool"},
		{"rust", append(packages[:1:1], packages[2:]...), "rust/programs/build-tool"},
	} {
		got, err := changedPackageRootsForPlatformAndLanguage([]string{testTriggerFixturePath}, tc.packages, root, "linux", tc.language)
		if err == nil || got != nil || !strings.Contains(err.Error(), tc.missing) {
			t.Fatalf("missing %s returned %v, error = %v", tc.missing, got, err)
		}
	}
}

func TestTestTriggerFixtureRenameKeepsDeletedSourcePath(t *testing.T) {
	root := t.TempDir()
	fixture := filepath.Join(root, filepath.FromSlash(testTriggerFixturePath))
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
	if !containsPath(changed, testTriggerFixturePath) {
		t.Fatalf("rename omitted old fixture path: %v", changed)
	}
	got, err := changedPackageRootsForPlatform(changed, testTriggerFixturePackages(root), root, "linux")
	if err != nil || !reflect.DeepEqual(testTriggerSelectedNames(got), []string{"go/programs/build-tool", "rust/programs/build-tool"}) {
		t.Fatalf("rename source roots = %v, error = %v", got, err)
	}
}

func TestTestTriggerFixturePlanKeepsPrerequisitesPlatformsAndToolchains(t *testing.T) {
	root := t.TempDir()
	packages := testTriggerFixturePackages(root)
	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}
	changed, err := changedPackageRootsForPlatform([]string{testTriggerFixturePath}, packages, root, "linux")
	if err != nil {
		t.Fatal(err)
	}
	affected := affectedForGraph(graph, changed, nil, false)
	if !affected["go/fixture-helper"] {
		t.Fatalf("fixture reader prerequisite missing from closure: %v", affected)
	}
	planPath := filepath.Join(root, "plan.json")
	if code := emitBuildPlan(packages, graph, affected, changed, []string{testTriggerFixturePath}, false, nil, "origin/main", root, planPath, false, 0, false, "", "all"); code != 0 {
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
	want := []string{"go/fixture-helper", "go/programs/build-tool", "rust/programs/build-tool"}
	if built.Force || !reflect.DeepEqual(built.AffectedPackages, want) {
		t.Fatalf("affected = %v, force = %t, want %v", built.AffectedPackages, built.Force, want)
	}
	for _, goos := range []string{"linux", "darwin", "windows"} {
		if got := built.StateForPlatform(goos).AffectedPackages; !reflect.DeepEqual(got, want) {
			t.Fatalf("%s affected = %v, want %v", goos, got, want)
		}
	}
	for _, language := range []string{"go", "rust"} {
		if !built.LanguagesNeeded[language] {
			t.Errorf("missing %s toolchain", language)
		}
	}
}

func TestTestTriggerFixtureConsumerMapTracksNativeReferences(t *testing.T) {
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
		case ".cs", ".dart", ".ex", ".exs", ".fs", ".fsx", ".go", ".hs", ".java", ".kt", ".lua", ".ml", ".mli", ".pl", ".pm", ".py", ".rb", ".rs", ".swift", ".t", ".ts":
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
		lowerPath := strings.ToLower("/" + portable + "/")
		lowerName := strings.ToLower(entry.Name())
		isTest := strings.Contains(lowerPath, "/tests/") ||
			strings.Contains(lowerPath, "/test/") || strings.Contains(lowerPath, "/spec/") ||
			strings.Contains(lowerPath, "/t/") || strings.Contains(lowerName, "test") ||
			strings.Contains(lowerName, "spec") ||
			(filepath.Ext(entry.Name()) == ".rs" && bytes.Contains(data, []byte("#[cfg(test)]")))
		if entry.Name() == "test_trigger_fixture_ci_selection_test.go" || !isTest {
			return nil
		}
		if bytes.Contains(data, []byte("test-trigger-selection-")) {
			references[parts[2]+"/programs/"+parts[3]] = true
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if got, want := testTriggerSelectedNames(references), []string{"go/programs/build-tool", "rust/programs/build-tool"}; !reflect.DeepEqual(got, want) {
		t.Fatalf("native references = %v, detector consumers = %v", got, want)
	}
}
