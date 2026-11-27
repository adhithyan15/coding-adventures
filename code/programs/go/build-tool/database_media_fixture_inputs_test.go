package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/graphdiff"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/hasher"
)

func databaseMediaPackages(root string) []discovery.Package {
	packages := []discovery.Package{
		{Name: "python/nand-flash", Language: "python"},
		{Name: "typescript/nand-flash", Language: "typescript"},
		{Name: "python/hdd-simulator", Language: "python"},
		{Name: "typescript/hdd-simulator", Language: "typescript"},
	}
	for index := range packages {
		packages[index].Path = filepath.Join(root, filepath.FromSlash("code/packages/"+packages[index].Name))
	}
	return packages
}

func TestDatabaseMediaFixtureSelectionAndKilnInputsAreExact(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	packages := databaseMediaPackages(root)
	cases := []struct {
		fixture          string
		predicate        func(string) bool
		readers          map[string]bool
		pythonReader     string
		typescriptReader string
		inputs           []string
	}{
		{
			fixture:          nandFlashFixturePath,
			predicate:        isNANDFlashFixture,
			readers:          map[string]bool{"python/nand-flash": true, "typescript/nand-flash": true},
			pythonReader:     "python/nand-flash",
			typescriptReader: "typescript/nand-flash",
			inputs:           nandFlashFixtureInputs,
		},
		{
			fixture:          hddSimulatorFixturePath,
			predicate:        isHDDSimulatorFixture,
			readers:          map[string]bool{"python/hdd-simulator": true, "typescript/hdd-simulator": true},
			pythonReader:     "python/hdd-simulator",
			typescriptReader: "typescript/hdd-simulator",
			inputs:           hddSimulatorFixtureInputs,
		},
	}

	for _, tc := range cases {
		selected, err := repositoryInputConsumersForChange([]string{tc.fixture}, packages, "all")
		if err != nil || !reflect.DeepEqual(selected, tc.readers) {
			t.Fatalf("all-language %s selection = %v, err %v; want %v", tc.fixture, selected, err, tc.readers)
		}
		for language, expected := range map[string]map[string]bool{
			"python":     {tc.pythonReader: true},
			"typescript": {tc.typescriptReader: true},
		} {
			got, err := repositoryInputConsumersForChange([]string{tc.fixture}, packages, language)
			if err != nil || !reflect.DeepEqual(got, expected) {
				t.Errorf("%s-only %s selection = %v, err %v; want %v", language, tc.fixture, got, err, expected)
			}
		}
		if !tc.predicate(tc.fixture) {
			t.Errorf("exact predicate rejected %s", tc.fixture)
		}
		for _, packageName := range []string{tc.pythonReader, tc.typescriptReader} {
			if got := repositoryInputsForPackage(packageName); !reflect.DeepEqual(got, tc.inputs) {
				t.Errorf("%s fallback hash inputs = %v; want exact %v", packageName, got, tc.inputs)
			}
			selectorInputs := repositoryInputsByPackage()[packageName]
			if selectorInputs == nil || !reflect.DeepEqual(selectorInputs.paths, tc.inputs) || len(selectorInputs.globs) != 0 || selectorInputs.unknown {
				t.Errorf("%s exported selector inputs = %#v; want exact paths %v", packageName, selectorInputs, tc.inputs)
			}
		}
	}

	negativeControls := []string{
		nandFlashFixturePath + ".bak",
		"code/specs/fixtures/database-media-v1/nand-flash/extra.json",
		"code/specs/fixtures/database-media-v1/nand-flash.json/",
		"code/specs/fixtures/database-media-v1ish/nand-flash.json",
		"code/specs/fixtures/database-media-v1/hdd-simulator.json.bak",
		"code/specs/fixtures/database-media-v2/nand-flash.json",
		"code/specs/fixtures/database-media-v1",
		"code/specs/fixtures/other-database-media-v1/nand-flash.json",
		`code\specs\fixtures\database-media-v1\nand-flash.json`,
	}
	for _, changed := range negativeControls {
		if isNANDFlashFixture(changed) || isHDDSimulatorFixture(changed) {
			t.Errorf("media predicate accepted lookalike %q", changed)
		}
		selected, err := repositoryInputConsumersForChange([]string{changed}, packages, "all")
		if err != nil || len(selected) != 0 {
			t.Errorf("lookalike %q selected %v, err %v; want no consumers", changed, selected, err)
		}
	}

	graph, err := buildKilnLegacyGraph(packages, root)
	if err != nil {
		t.Fatalf("build Kiln graph with database media inputs: %v", err)
	}
	for packageName, expected := range map[string][]string{
		"python/nand-flash":        {nandFlashFixturePath},
		"typescript/nand-flash":    {nandFlashFixturePath},
		"python/hdd-simulator":     {hddSimulatorFixturePath},
		"typescript/hdd-simulator": {hddSimulatorFixturePath},
	} {
		var got []string
		for _, pkg := range graph.Packages {
			if pkg.Name == packageName {
				got = pkg.RepoInputs
				if len(pkg.RepoInputGlobs) != 0 || pkg.RepoInputsUnknown {
					t.Errorf("%s unexpectedly exports repo globs/unknown=%v", packageName, pkg.RepoInputsUnknown)
				}
				break
			}
		}
		if !reflect.DeepEqual(got, expected) {
			t.Errorf("Kiln repo_inputs for %s = %v; want %v", packageName, got, expected)
		}
	}
}

func TestDatabaseMediaFixtureContentChangesOnlyItsReadersCacheHash(t *testing.T) {
	fixtures := []struct {
		path    string
		readers map[string]bool
	}{
		{
			path:    nandFlashFixturePath,
			readers: map[string]bool{"python/nand-flash": true, "typescript/nand-flash": true},
		},
		{
			path:    hddSimulatorFixturePath,
			readers: map[string]bool{"python/hdd-simulator": true, "typescript/hdd-simulator": true},
		},
	}
	for _, fixture := range fixtures {
		t.Run(fixture.path, func(t *testing.T) {
			root := t.TempDir()
			packages := databaseMediaPackages(root)
			for _, pkg := range packages {
				sourcePath := filepath.Join(pkg.Path, "src", "media")
				switch pkg.Language {
				case "python":
					sourcePath += ".py"
				case "typescript":
					sourcePath += ".ts"
				}
				if err := os.MkdirAll(filepath.Dir(sourcePath), 0o755); err != nil {
					t.Fatal(err)
				}
				if err := os.WriteFile(sourcePath, []byte("# stable local source\n"), 0o644); err != nil {
					t.Fatal(err)
				}
				if err := os.WriteFile(filepath.Join(pkg.Path, "BUILD"), []byte("echo media\n"), 0o644); err != nil {
					t.Fatal(err)
				}
			}
			for _, path := range []string{nandFlashFixturePath, hddSimulatorFixturePath} {
				fullPath := filepath.Join(root, filepath.FromSlash(path))
				if err := os.MkdirAll(filepath.Dir(fullPath), 0o755); err != nil {
					t.Fatal(err)
				}
				if err := os.WriteFile(fullPath, []byte("{\"version\":1}\n"), 0o644); err != nil {
					t.Fatal(err)
				}
			}
			hashAll := func() map[string]string {
				t.Helper()
				hashes := make(map[string]string, len(packages))
				for _, pkg := range packages {
					hash, err := hasher.HashPackageWithRepositoryInputs(pkg, root, repositoryInputsForPackage(pkg.Name))
					if err != nil {
						t.Fatalf("hash %s with registered media vector: %v", pkg.Name, err)
					}
					hashes[pkg.Name] = hash
				}
				return hashes
			}
			before := hashAll()
			changedPath := filepath.Join(root, filepath.FromSlash(fixture.path))
			if err := os.WriteFile(changedPath, []byte("{\"version\":2}\n"), 0o644); err != nil {
				t.Fatal(err)
			}
			after := hashAll()
			for _, pkg := range packages {
				_, shouldChange := fixture.readers[pkg.Name]
				if (before[pkg.Name] != after[pkg.Name]) != shouldChange {
					t.Errorf("%s hash changed=%t for %s; want %t (before %s, after %s)", pkg.Name, before[pkg.Name] != after[pkg.Name], fixture.path, shouldChange, before[pkg.Name], after[pkg.Name])
				}
			}
		})
	}
}

func TestDatabaseMediaFixtureBoundaryMatchesExactPackageInputs(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	data, err := os.ReadFile(filepath.Join(root, "code/specs/fixtures/build-tool-v1/repository-source-input-boundary.json"))
	if err != nil {
		t.Fatal(err)
	}
	var boundary graphdiff.RepositoryBoundary
	if err := json.Unmarshal(data, &boundary); err != nil {
		t.Fatal(err)
	}

	for _, tc := range []struct {
		id      string
		fixture string
		roots   []string
	}{
		{
			id:      "repository-database-media-nand-fixture-inputs",
			fixture: nandFlashFixturePath,
			roots:   []string{"code/packages/python/nand-flash", "code/packages/typescript/nand-flash"},
		},
		{
			id:      "repository-database-media-hdd-fixture-inputs",
			fixture: hddSimulatorFixturePath,
			roots:   []string{"code/packages/python/hdd-simulator", "code/packages/typescript/hdd-simulator"},
		},
	} {
		found := false
		for _, row := range boundary.Boundaries {
			if row.ID != tc.id {
				continue
			}
			found = true
			if row.AppliesTo.ExactRoots == nil || !reflect.DeepEqual(row.AppliesTo.ExactRoots, tc.roots) || len(row.AppliesTo.DescendantRoots) != 0 || len(row.AppliesTo.ExcludedRoots) != 0 {
				t.Errorf("boundary %s roots = %+v; want only %v", tc.id, row.AppliesTo, tc.roots)
			}
			if len(row.Inputs) != 1 || row.Inputs[0].Path != tc.fixture || row.Inputs[0].Role != "cross_package_exact" {
				t.Errorf("boundary %s inputs = %+v; want only exact %s", tc.id, row.Inputs, tc.fixture)
			}
		}
		if !found {
			t.Errorf("canonical repository source-input boundary omitted %s", tc.id)
		}
	}

	tsConfigRoots := []string{}
	for _, row := range boundary.Boundaries {
		if row.ID == "typescript-workspace-configuration" {
			tsConfigRoots = row.AppliesTo.ExactRoots
			break
		}
	}
	for _, root := range []string{"code/packages/typescript/nand-flash", "code/packages/typescript/hdd-simulator"} {
		index := sort.SearchStrings(tsConfigRoots, root)
		if index == len(tsConfigRoots) || tsConfigRoots[index] != root {
			t.Errorf("typescript workspace configuration boundary omitted %s", root)
		}
	}
}
