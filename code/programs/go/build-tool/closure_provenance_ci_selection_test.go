package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/plan"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

const closureProvenanceAcceptanceSpec = "code/specs/CV02-checked-bounded-provenance-graphs.md"

func closureProvenanceSelectionPackages(root string) []discovery.Package {
	return []discovery.Package{
		{Name: "rust/programs/closurec", Path: filepath.Join(root, "code", "programs", "rust", "closurec"), Language: "rust"},
		{Name: "go/extra", Path: filepath.Join(root, "code", "packages", "go", "extra"), Language: "go"},
	}
}

func TestClosureProvenanceSpecSelectsNativeCompilerExactly(t *testing.T) {
	root := t.TempDir()
	packages := closureProvenanceSelectionPackages(root)
	for _, goos := range []string{"linux", "darwin", "windows"} {
		t.Run(goos, func(t *testing.T) {
			for _, language := range []string{"all", "rust"} {
				got, err := changedPackageRootsForPlatformAndLanguage([]string{closureProvenanceAcceptanceSpec}, packages, root, goos, language)
				if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), []string{"rust/programs/closurec"}) {
					t.Errorf("%s spec-only roots = %v, error = %v", language, got, err)
				}
			}
			got, err := changedPackageRootsForPlatform([]string{closureProvenanceAcceptanceSpec, "code/packages/go/extra/main.go"}, packages, root, goos)
			if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), []string{"go/extra", "rust/programs/closurec"}) {
				t.Errorf("spec and ordinary package union = %v, error = %v", got, err)
			}
			// No source file exists: deleted/renamed old paths must still trigger.
			got, err = changedPackageRootsForPlatform([]string{closureProvenanceAcceptanceSpec, "code/specs/CV02-renamed.md"}, packages, root, goos)
			if err != nil || !got["rust/programs/closurec"] {
				t.Errorf("deleted old spec roots = %v, error = %v", got, err)
			}
			got, err = changedPackageRootsForPlatform([]string{"code/specs/CV02-checked-bounded-provenance-graphs-example.md"}, packages, root, goos)
			if err != nil || len(got) != 0 {
				t.Errorf("near spec path selected roots = %v, error = %v", got, err)
			}
			_, err = changedPackageRootsForPlatform([]string{closureProvenanceAcceptanceSpec}, packages[1:], root, goos)
			if err == nil {
				t.Error("missing native compiler consumer was silently omitted")
			}
			got, err = changedPackageRootsForPlatformAndLanguage([]string{closureProvenanceAcceptanceSpec}, packages[1:], root, goos, "go")
			if err != nil || len(got) != 0 {
				t.Errorf("non-Rust language required compiler: roots = %v, error = %v", got, err)
			}
		})
	}
}

func TestClosureProvenanceSpecBuildPlanSelectsCompilerToolchainAndGate(t *testing.T) {
	root := t.TempDir()
	packages := closureProvenanceSelectionPackages(root)
	compilerDir := packages[0].Path
	if err := os.MkdirAll(compilerDir, 0o755); err != nil {
		t.Fatal(err)
	}
	// Read the actual native command rather than inventing a test-only command.
	build, err := os.ReadFile(filepath.Join(toolchainFixtureRepoRoot(t), "code", "programs", "rust", "closurec", "BUILD"))
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(compilerDir, "BUILD"), build, 0o644); err != nil {
		t.Fatal(err)
	}
	registry := filepath.Join(toolchainFixtureRepoRoot(t), "code", "specs", "data", "ci-gates.json")
	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}
	changedFiles := []string{closureProvenanceAcceptanceSpec}
	changed, err := changedPackageRootsForPlatform(changedFiles, packages, root, "linux")
	if err != nil {
		t.Fatal(err)
	}
	affected := affectedForGraph(graph, changed, nil, false)
	planPath := filepath.Join(root, "plan.json")
	if code := emitBuildPlan(packages, graph, affected, changed, changedFiles, false, nil, "origin/main", root, planPath, false, 0, false, registry, "all"); code != 0 {
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
	want := []string{"rust/programs/closurec"}
	if built.Force || !reflect.DeepEqual(built.AffectedPackages, want) {
		t.Errorf("spec-only plan force=%v affected=%v, want %v", built.Force, built.AffectedPackages, want)
	}
	for _, goos := range []string{"linux", "darwin", "windows"} {
		if got := built.StateForPlatform(goos).AffectedPackages; !reflect.DeepEqual(got, want) {
			t.Errorf("%s native affected packages = %v, want %v", goos, got, want)
		}
	}
	if !built.LanguagesNeeded["rust"] || built.LanguagesNeeded["go"] || !built.CIJobs["build-windows-os-suites"] {
		t.Errorf("spec-only plan languages=%v Windows gate=%v", built.LanguagesNeeded, built.CIJobs["build-windows-os-suites"])
	}
}

func TestClosureProvenanceSpecMissingConsumerWritesNoPlan(t *testing.T) {
	root := t.TempDir()
	packages := closureProvenanceSelectionPackages(root)[1:]
	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}
	planPath := filepath.Join(root, "missing-consumer.json")
	if code := emitBuildPlan(packages, graph, map[string]bool{}, map[string]bool{}, []string{closureProvenanceAcceptanceSpec}, false, nil, "origin/main", root, planPath, false, 0, false, "", "all"); code != 1 {
		t.Errorf("missing consumer plan exit=%d, want 1", code)
	}
	if _, err := os.Stat(planPath); !os.IsNotExist(err) {
		t.Errorf("missing consumer wrote a plan: %v", err)
	}
}
