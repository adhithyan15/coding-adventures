package main

import (
	"bytes"
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

// A neutral runner reading a case does not make it a native BUILD reader.
// Keep this test relation independent of the production selector.
var planExpectedReaders = map[string]string{
	"affected-empty":        "",
	"affected-null":         "",
	"future-version":        "",
	"portable-package-path": "T",
	"replace-existing":      "Y",
}

var planTestRoots = map[rune]struct{ name, path, language string }{
	'Y': {"python/programs/build-tool", "code/programs/python/build-tool", "python"},
	'T': {"typescript/programs/build-tool", "code/programs/typescript/build-tool", "typescript"},
}

const planTestPrefix = "code/specs/fixtures/build-tool-v1/cases/plan-"

func planTestPath(stem string) string { return planTestPrefix + stem + ".json" }

func planTestPackages(root string) []discovery.Package {
	packages := make([]discovery.Package, 0, len(planTestRoots)+1)
	for _, reader := range planTestRoots {
		packages = append(packages, discovery.Package{Name: reader.name, Path: filepath.Join(root, filepath.FromSlash(reader.path)), Language: reader.language})
	}
	return append(packages, discovery.Package{Name: "rust/extra", Path: filepath.Join(root, "code", "packages", "rust", "extra"), Language: "rust"})
}

func planTestNames(stem string) []string {
	names := []string{}
	for _, code := range planExpectedReaders[stem] {
		names = append(names, planTestRoots[code].name)
	}
	sort.Strings(names)
	return names
}

func TestPlanFixtureExactReadersAndPlatforms(t *testing.T) {
	root := t.TempDir()
	packages := planTestPackages(root)
	for stem := range planExpectedReaders {
		want := planTestNames(stem)
		for _, goos := range []string{"linux", "darwin", "windows"} {
			got, err := changedPackageRootsForPlatform([]string{planTestPath(stem)}, packages, root, goos)
			if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), want) && !(len(got) == 0 && len(want) == 0) {
				t.Fatalf("%s %s: got %v, err %v; want %v", goos, stem, sortedChangedRoots(got), err, want)
			}
		}
	}
	for _, path := range []string{
		planTestPrefix + ".json", planTestPrefix + "replace-existing.JSON",
		planTestPrefix + "replace-existing.json.bak", planTestPrefix + "replace-existing\\child.json",
		"code/specs/fixtures/build-tool-v1/cases/nested/plan-replace-existing.json",
		"code/specs/fixtures/build-tool-v1/cases/plan-nested/replace-existing.json",
		"code/specs/fixtures/build-tool-v1/other/plan-replace-existing.json",
		"code/specs/fixtures/build-tool-v1/cases/Plan-replace-existing.json",
	} {
		got, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("lookalike %q: %v, %v", path, got, err)
		}
	}
	got, err := changedPackageRootsForPlatform([]string{planTestPath("replace-existing"), "code/packages/rust/extra/src/lib.rs"}, packages, root, "linux")
	want := append(planTestNames("replace-existing"), "rust/extra")
	sort.Strings(want)
	if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), want) {
		t.Fatalf("fixture plus ordinary edit: %v, %v; want %v", sortedChangedRoots(got), err, want)
	}
}

func TestPlanFixtureUnknownLanguageAndMissingReader(t *testing.T) {
	root := t.TempDir()
	packages := planTestPackages(root)
	for _, language := range []string{"all", "ruby"} {
		got, err := changedPackageRootsForPlatformAndLanguage([]string{planTestPath("future-case")}, packages, root, "linux", language)
		if err == nil || got != nil {
			t.Fatalf("unknown case %s: got %v, error %v", language, got, err)
		}
	}
	for stem, codes := range planExpectedReaders {
		for code, reader := range planTestRoots {
			got, err := changedPackageRootsForPlatformAndLanguage([]string{planTestPath(stem)}, packages, root, "linux", reader.language)
			want := []string{}
			if strings.ContainsRune(codes, code) {
				want = []string{reader.name}
			}
			if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), want) && !(len(want) == 0 && len(got) == 0) {
				t.Fatalf("%s %s: got %v, error %v; want %v", stem, reader.language, got, err, want)
			}
			if len(want) == 0 {
				continue
			}
			missing := []discovery.Package{}
			for _, pkg := range packages {
				if pkg.Name != reader.name {
					missing = append(missing, pkg)
				}
			}
			got, err = changedPackageRootsForPlatformAndLanguage([]string{planTestPath(stem)}, missing, root, "linux", reader.language)
			if err == nil || !strings.Contains(err.Error(), reader.name) || got != nil {
				t.Fatalf("missing %s %s: got %v, error %v", stem, reader.name, got, err)
			}
		}
	}
}

func TestPlanFixtureCheckedCorpusAndNativeReferences(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	caseDir := filepath.Join(root, "code", "specs", "fixtures", "build-tool-v1", "cases")
	entries, err := os.ReadDir(caseDir)
	if err != nil {
		t.Fatal(err)
	}
	actual := []string{}
	for _, entry := range entries {
		name := entry.Name()
		if entry.IsDir() || !strings.HasPrefix(name, "plan-") || !strings.HasSuffix(name, ".json") {
			continue
		}
		stem := strings.TrimSuffix(strings.TrimPrefix(name, "plan-"), ".json")
		actual = append(actual, stem)
		data, err := os.ReadFile(filepath.Join(caseDir, name))
		if err != nil {
			t.Fatal(err)
		}
		var envelope struct {
			Domain string `json:"domain"`
		}
		if err := json.Unmarshal(data, &envelope); err != nil || envelope.Domain != "plan" {
			t.Fatalf("%s domain %q, error %v", name, envelope.Domain, err)
		}
	}
	expected := make([]string, 0, len(planExpectedReaders))
	for stem := range planExpectedReaders {
		expected = append(expected, stem)
	}
	sort.Strings(actual)
	sort.Strings(expected)
	if !reflect.DeepEqual(actual, expected) || len(actual) != 5 {
		t.Fatalf("checked plan cases %v differ from native-reader relation %v", actual, expected)
	}
	for code, source := range map[rune]string{
		'Y': "python/build-tool/tests/test_plan.py",
		'T': "typescript/build-tool/tests/plan.test.ts",
	} {
		data, err := os.ReadFile(filepath.Join(root, "code", "programs", filepath.FromSlash(source)))
		if err != nil {
			t.Fatal(err)
		}
		for stem, codes := range planExpectedReaders {
			found := bytes.Contains(data, []byte("plan-"+stem+".json"))
			if found != strings.ContainsRune(codes, code) {
				t.Errorf("native reader %c case %s: source reference %t, route %t", code, stem, found, strings.ContainsRune(codes, code))
			}
		}
	}
}

func TestPlanFixtureUnforcedPlanAndAtomicity(t *testing.T) {
	root := t.TempDir()
	packages := planTestPackages(root)
	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}
	for stem := range planExpectedReaders {
		path := planTestPath(stem)
		changed, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil {
			t.Fatal(err)
		}
		affected := affectedForGraph(graph, changed, nil, false)
		planPath := filepath.Join(root, stem+"-plan.json")
		if code := emitBuildPlan(packages, graph, affected, changed, []string{path}, false, nil, "origin/main", root, planPath, false, 0, false, "", "all"); code != 0 {
			t.Fatalf("%s plan exit %d", stem, code)
		}
		data, err := os.ReadFile(planPath)
		if err != nil {
			t.Fatal(err)
		}
		var built plan.BuildPlan
		if err := json.Unmarshal(data, &built); err != nil {
			t.Fatal(err)
		}
		want := planTestNames(stem)
		if built.Force || !reflect.DeepEqual(sortedStrings(built.AffectedPackages), want) && !(len(want) == 0 && len(built.AffectedPackages) == 0) {
			t.Fatalf("%s affected %v, force %t; want %v", stem, built.AffectedPackages, built.Force, want)
		}
		for _, goos := range []string{"linux", "darwin", "windows"} {
			got := sortedStrings(built.StateForPlatform(goos).AffectedPackages)
			if !reflect.DeepEqual(got, want) && !(len(got) == 0 && len(want) == 0) {
				t.Fatalf("%s %s affected %v; want %v", stem, goos, got, want)
			}
		}
		for code, reader := range planTestRoots {
			if built.LanguagesNeeded[reader.language] != strings.ContainsRune(planExpectedReaders[stem], code) {
				t.Fatalf("%s toolchain %s: %v", stem, reader.language, built.LanguagesNeeded)
			}
		}
	}
	for _, tc := range []struct{ name, path string }{
		{"missing-reader", planTestPath("replace-existing")},
		{"unknown-case", planTestPath("new-case")},
	} {
		available := packages
		if tc.name == "missing-reader" {
			available = []discovery.Package{packages[len(packages)-1]}
		}
		availableGraph, err := resolver.ResolveDependencies(available)
		if err != nil {
			t.Fatal(err)
		}
		out := filepath.Join(root, tc.name+"-plan.json")
		if code := emitBuildPlan(available, availableGraph, map[string]bool{}, map[string]bool{}, []string{tc.path}, false, nil, "origin/main", root, out, false, 0, false, "", "all"); code != 1 {
			t.Fatalf("%s plan exit %d", tc.name, code)
		}
		if _, err := os.Stat(out); !os.IsNotExist(err) {
			t.Fatalf("partial %s plan: %v", tc.name, err)
		}
	}
}

func TestPlanFixtureRenameSelectsDeletedSource(t *testing.T) {
	root := t.TempDir()
	path := planTestPath("portable-package-path")
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
	got, err := changedPackageRootsForPlatform(changed, planTestPackages(root), root, "linux")
	if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), planTestNames("portable-package-path")) {
		t.Fatalf("renamed source selected %v, error %v", sortedChangedRoots(got), err)
	}
}
