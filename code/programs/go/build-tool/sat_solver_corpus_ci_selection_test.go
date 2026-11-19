package main

import (
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

func satSolverCorpusPackages(root string) []discovery.Package {
	return []discovery.Package{
		{Name: "go/sat-solver", Path: filepath.Join(root, "code/packages/go/sat-solver"), Language: "go"},
		{Name: "typescript/sat-solver", Path: filepath.Join(root, "code/packages/typescript/sat-solver"), Language: "typescript"},
		{Name: "go/other", Path: filepath.Join(root, "code/packages/go/other"), Language: "go"},
	}
}

func TestSatSolverSharedCorpusSelectsBothExactConsumers(t *testing.T) {
	root := t.TempDir()
	packages := satSolverCorpusPackages(root)
	for _, platform := range []string{"linux", "darwin", "windows"} {
		for _, language := range []string{"all", "go", "typescript", "rust"} {
			got, err := changedPackageRootsForPlatformAndLanguage([]string{satSolverSharedCorpusPath}, packages, root, platform, language)
			want := map[string]bool{}
			if language == "all" || language == "go" {
				want["go/sat-solver"] = true
			}
			if language == "all" || language == "typescript" {
				want["typescript/sat-solver"] = true
			}
			if err != nil || !reflect.DeepEqual(got, want) {
				t.Fatalf("%s/%s got %v/%v, want %v", platform, language, got, err, want)
			}
		}
	}

	for _, path := range []string{
		strings.ToUpper(satSolverSharedCorpusPath),
		strings.ReplaceAll(satSolverSharedCorpusPath, "/", "\\"),
		satSolverSharedCorpusPath + ".bak",
		strings.Replace(satSolverSharedCorpusPath, "cases.json", "schema.json", 1),
		strings.Replace(satSolverSharedCorpusPath, "cases.json", "nested/cases.json", 1),
	} {
		got, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("lookalike %s: %v/%v", path, got, err)
		}
	}

	// Exact-path selection must work for a deletion or rename-old path: the
	// fixture is not stat'ed by the selector.
	if got, err := changedPackageRootsForPlatform([]string{satSolverSharedCorpusPath}, packages, root, "linux"); err != nil || len(got) != 2 {
		t.Fatalf("missing fixture still selects both readers: %v/%v", got, err)
	}
	for _, missing := range []string{"go/sat-solver", "typescript/sat-solver"} {
		remaining := make([]discovery.Package, 0, len(packages)-1)
		for _, pkg := range packages {
			if pkg.Name != missing {
				remaining = append(remaining, pkg)
			}
		}
		got, err := changedPackageRootsForPlatform([]string{satSolverSharedCorpusPath}, remaining, root, "linux")
		if err == nil || got != nil {
			t.Fatalf("missing %s reader: %v/%v", missing, got, err)
		}
	}
}
