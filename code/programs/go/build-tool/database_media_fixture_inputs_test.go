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
		{Name: "c/nand-flash", Language: "c"},
		{Name: "cpp/nand-flash", Language: "cpp"},
		{Name: "go/nand-flash", Language: "go"},
		{Name: "go/nand-page-program-cuts", Language: "go"},
		{Name: "go/nand-block-erase-cuts", Language: "go"},
		{Name: "go/ftl-page-map", Language: "go"},
		{Name: "python/hdd-simulator", Language: "python"},
		{Name: "typescript/hdd-simulator", Language: "typescript"},
		{Name: "c/hdd-simulator", Language: "c"},
		{Name: "go/hdd-simulator", Language: "go"},
		{Name: "go/hdd-write-crash-cuts", Language: "go"},
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
		goReader         string
		cppReader        string
		inputs           []string
	}{
		{
			fixture:          nandFlashFixturePath,
			predicate:        isNANDFlashFixture,
			readers:          map[string]bool{"python/nand-flash": true, "typescript/nand-flash": true, "c/nand-flash": true, "cpp/nand-flash": true, "go/nand-flash": true},
			pythonReader:     "python/nand-flash",
			typescriptReader: "typescript/nand-flash",
			goReader:         "go/nand-flash",
			cppReader:        "cpp/nand-flash",
			inputs:           nandFlashFixtureInputs,
		},
		{
			fixture:   nandPageProgramCutsFixturePath,
			predicate: isNANDPageProgramCutsFixture,
			readers:   map[string]bool{"go/nand-page-program-cuts": true},
			goReader:  "go/nand-page-program-cuts",
			inputs:    nandPageProgramCutsFixtureInputs,
		},
		{
			fixture:   nandBlockEraseCutsFixturePath,
			predicate: isNANDBlockEraseCutsFixture,
			readers:   map[string]bool{"go/nand-block-erase-cuts": true},
			goReader:  "go/nand-block-erase-cuts",
			inputs:    nandBlockEraseCutsFixtureInputs,
		},
		{
			fixture:          hddSimulatorFixturePath,
			predicate:        isHDDSimulatorFixture,
			readers:          map[string]bool{"python/hdd-simulator": true, "typescript/hdd-simulator": true, "c/hdd-simulator": true, "go/hdd-simulator": true},
			pythonReader:     "python/hdd-simulator",
			typescriptReader: "typescript/hdd-simulator",
			goReader:         "go/hdd-simulator",
			inputs:           hddSimulatorFixtureInputs,
		},
		{
			fixture:   hddWriteCrashCutsFixturePath,
			predicate: isHDDWriteCrashCutsFixture,
			readers:   map[string]bool{"go/hdd-write-crash-cuts": true},
			goReader:  "go/hdd-write-crash-cuts",
			inputs:    hddWriteCrashCutsFixtureInputs,
		},
		{
			fixture:   ftlPageMapFixturePath,
			predicate: isFTLPageMapFixture,
			readers:   map[string]bool{"go/ftl-page-map": true},
			goReader:  "go/ftl-page-map",
			inputs:    ftlPageMapFixtureInputs,
		},
		{
			fixture:   ftlPageReclamationFixturePath,
			predicate: isFTLPageMapFixture,
			readers:   map[string]bool{"go/ftl-page-map": true},
			goReader:  "go/ftl-page-map",
			inputs:    ftlPageMapFixtureInputs,
		},
	}

	for _, tc := range cases {
		selected, err := repositoryInputConsumersForChange([]string{tc.fixture}, packages, "all")
		if err != nil || !reflect.DeepEqual(selected, tc.readers) {
			t.Fatalf("all-language %s selection = %v, err %v; want %v", tc.fixture, selected, err, tc.readers)
		}
		cReaders := map[string]bool{}
		for _, packageName := range []string{"c/nand-flash", "c/hdd-simulator"} {
			if tc.readers[packageName] {
				cReaders[packageName] = true
			}
		}
		goReaders := map[string]bool{}
		if tc.goReader != "" {
			goReaders[tc.goReader] = true
		}
		cppReaders := map[string]bool{}
		if tc.cppReader != "" {
			cppReaders[tc.cppReader] = true
		}
		languageReaders := map[string]map[string]bool{"c": cReaders, "cpp": cppReaders, "go": goReaders}
		if tc.pythonReader != "" {
			languageReaders["python"] = map[string]bool{tc.pythonReader: true}
		} else {
			languageReaders["python"] = map[string]bool{}
		}
		if tc.typescriptReader != "" {
			languageReaders["typescript"] = map[string]bool{tc.typescriptReader: true}
		} else {
			languageReaders["typescript"] = map[string]bool{}
		}
		for language, expected := range languageReaders {
			got, err := repositoryInputConsumersForChange([]string{tc.fixture}, packages, language)
			if err != nil || !reflect.DeepEqual(got, expected) {
				t.Errorf("%s-only %s selection = %v, err %v; want %v", language, tc.fixture, got, err, expected)
			}
		}
		if !tc.predicate(tc.fixture) {
			t.Errorf("exact predicate rejected %s", tc.fixture)
		}
		for _, packageName := range []string{tc.pythonReader, tc.typescriptReader, tc.goReader, tc.cppReader, "c/nand-flash", "c/hdd-simulator"} {
			if !tc.readers[packageName] {
				continue
			}
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
		nandPageProgramCutsFixturePath + ".bak",
		nandBlockEraseCutsFixturePath + ".bak",
		"code/specs/fixtures/database-media-v1/nand-block-erase-cuts/extra.json",
		"code/specs/fixtures/database-media-v2/nand-block-erase-cuts.json",
		"code/specs/fixtures/database-media-v1/nand-page-program-cuts/extra.json",
		"code/specs/fixtures/database-media-v2/nand-page-program-cuts.json",
		"code/specs/fixtures/database-media-v1/nand-flash/extra.json",
		"code/specs/fixtures/database-media-v1/nand-flash.json/",
		"code/specs/fixtures/database-media-v1ish/nand-flash.json",
		"code/specs/fixtures/database-media-v1/hdd-simulator.json.bak",
		"code/specs/fixtures/database-media-v2/hdd-simulator.json",
		"code/specs/fixtures/database-media-v1/hdd-simulator.json/extra",
		hddWriteCrashCutsFixturePath + ".bak",
		ftlPageMapFixturePath + ".bak",
		ftlPageReclamationFixturePath + ".bak",
		"code/specs/fixtures/database-media-v1/ftl-page-map.json/extra",
		"code/specs/fixtures/database-media-v1/ftl-page-reclamation.json.bak",
		"code/specs/fixtures/database-media-v2/ftl-page-map.json",
		"code/specs/fixtures/database-media-v1/hdd-write-crash-cuts/extra.json",
		"code/specs/fixtures/database-media-v2/nand-flash.json",
		"code/specs/fixtures/database-media-v1",
		"code/specs/fixtures/other-database-media-v1/nand-flash.json",
		`code\specs\fixtures\database-media-v1\nand-flash.json`,
		`code\specs\fixtures\database-media-v1\hdd-simulator.json`,
	}
	for _, changed := range negativeControls {
		if isNANDFlashFixture(changed) || isNANDPageProgramCutsFixture(changed) || isNANDBlockEraseCutsFixture(changed) || isHDDSimulatorFixture(changed) || isHDDWriteCrashCutsFixture(changed) || isFTLPageMapFixture(changed) {
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
		"python/nand-flash":         {nandFlashFixturePath},
		"typescript/nand-flash":     {nandFlashFixturePath},
		"c/nand-flash":              {nandFlashFixturePath},
		"cpp/nand-flash":            {nandFlashFixturePath},
		"go/nand-flash":             {nandFlashFixturePath},
		"go/nand-page-program-cuts": {nandPageProgramCutsFixturePath},
		"go/nand-block-erase-cuts":  {nandBlockEraseCutsFixturePath},
		"python/hdd-simulator":      {hddSimulatorFixturePath},
		"typescript/hdd-simulator":  {hddSimulatorFixturePath},
		"c/hdd-simulator":           {hddSimulatorFixturePath},
		"go/hdd-simulator":          {hddSimulatorFixturePath},
		"go/hdd-write-crash-cuts":   {hddWriteCrashCutsFixturePath},
		"go/ftl-page-map":           {ftlPageMapFixturePath, ftlPageReclamationFixturePath},
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

func TestDeletedHDDSimulatorFixtureStillSelectsEachReader(t *testing.T) {
	root := t.TempDir()
	packages := databaseMediaPackages(root)
	missing := filepath.Join(root, filepath.FromSlash(hddSimulatorFixturePath))
	if _, err := os.Stat(missing); !os.IsNotExist(err) {
		t.Fatalf("fixture should be absent for deletion case: %v", err)
	}
	for _, language := range []string{"c", "go", "python", "typescript", "all"} {
		got, err := repositoryInputConsumersForChange([]string{hddSimulatorFixturePath}, packages, language)
		want := map[string]bool{"go/hdd-simulator": true}
		switch language {
		case "c":
			want = map[string]bool{"c/hdd-simulator": true}
		case "python":
			want = map[string]bool{"python/hdd-simulator": true}
		case "typescript":
			want = map[string]bool{"typescript/hdd-simulator": true}
		case "all":
			want = map[string]bool{
				"c/hdd-simulator": true, "go/hdd-simulator": true,
				"python/hdd-simulator": true, "typescript/hdd-simulator": true,
			}
		}
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Fatalf("deleted HDD timing fixture %s selection = %v, err %v; want %v", language, got, err, want)
		}
	}
}

func TestDeletedNANDFixtureStillSelectsEachReader(t *testing.T) {
	root := t.TempDir()
	packages := databaseMediaPackages(root)
	missing := filepath.Join(root, filepath.FromSlash(nandFlashFixturePath))
	if _, err := os.Stat(missing); !os.IsNotExist(err) {
		t.Fatalf("fixture should be absent for deletion case: %v", err)
	}
	for _, language := range []string{"c", "cpp", "go", "python", "typescript", "all"} {
		got, err := repositoryInputConsumersForChange([]string{nandFlashFixturePath}, packages, language)
		want := map[string]bool{language + "/nand-flash": true}
		if language == "all" {
			want = map[string]bool{
				"c/nand-flash": true, "cpp/nand-flash": true, "go/nand-flash": true,
				"python/nand-flash": true, "typescript/nand-flash": true,
			}
		}
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Errorf("deleted NAND fixture %s selection = %v, err %v; want %v", language, got, err, want)
		}
	}
}

func TestDeletedNANDPageProgramCutsFixtureStillSelectsGoReader(t *testing.T) {
	root := t.TempDir()
	packages := databaseMediaPackages(root)
	missing := filepath.Join(root, filepath.FromSlash(nandPageProgramCutsFixturePath))
	if _, err := os.Stat(missing); !os.IsNotExist(err) {
		t.Fatalf("fixture should be absent for deletion case: %v", err)
	}
	for _, language := range []string{"go", "c", "cpp", "python", "typescript", "all"} {
		got, err := repositoryInputConsumersForChange([]string{nandPageProgramCutsFixturePath}, packages, language)
		want := map[string]bool{}
		if language == "go" || language == "all" {
			want = map[string]bool{"go/nand-page-program-cuts": true}
		}
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Errorf("deleted NAND page-program fixture %s selection = %v, err %v; want %v", language, got, err, want)
		}
	}
}

func TestDeletedNANDBlockEraseCutsFixtureStillSelectsGoReader(t *testing.T) {
	root := t.TempDir()
	packages := databaseMediaPackages(root)
	missing := filepath.Join(root, filepath.FromSlash(nandBlockEraseCutsFixturePath))
	if _, err := os.Stat(missing); !os.IsNotExist(err) {
		t.Fatalf("fixture should be absent for deletion case: %v", err)
	}
	for _, language := range []string{"go", "c", "cpp", "python", "typescript", "all"} {
		got, err := repositoryInputConsumersForChange([]string{nandBlockEraseCutsFixturePath}, packages, language)
		want := map[string]bool{}
		if language == "go" || language == "all" {
			want = map[string]bool{"go/nand-block-erase-cuts": true}
		}
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Errorf("deleted NAND block-erase fixture %s selection = %v, err %v; want %v", language, got, err, want)
		}
	}
}

func TestDeletedFTLPageMapFixturesStillSelectGoReader(t *testing.T) {
	root := t.TempDir()
	packages := databaseMediaPackages(root)
	for _, fixture := range []string{ftlPageMapFixturePath, ftlPageReclamationFixturePath} {
		missing := filepath.Join(root, filepath.FromSlash(fixture))
		if _, err := os.Stat(missing); !os.IsNotExist(err) {
			t.Fatalf("fixture should be absent for deletion case: %v", err)
		}
		for _, language := range []string{"go", "python", "all"} {
			got, err := repositoryInputConsumersForChange([]string{fixture}, packages, language)
			want := map[string]bool{}
			if language == "go" || language == "all" {
				want = map[string]bool{"go/ftl-page-map": true}
			}
			if err != nil || !reflect.DeepEqual(got, want) {
				t.Errorf("deleted FTL fixture %s selection for %s = %v, err %v; want %v", fixture, language, got, err, want)
			}
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
			readers: map[string]bool{"python/nand-flash": true, "typescript/nand-flash": true, "c/nand-flash": true, "cpp/nand-flash": true, "go/nand-flash": true},
		},
		{
			path:    nandPageProgramCutsFixturePath,
			readers: map[string]bool{"go/nand-page-program-cuts": true},
		},
		{
			path:    nandBlockEraseCutsFixturePath,
			readers: map[string]bool{"go/nand-block-erase-cuts": true},
		},
		{
			path:    hddSimulatorFixturePath,
			readers: map[string]bool{"python/hdd-simulator": true, "typescript/hdd-simulator": true, "c/hdd-simulator": true, "go/hdd-simulator": true},
		},
		{
			path:    hddWriteCrashCutsFixturePath,
			readers: map[string]bool{"go/hdd-write-crash-cuts": true},
		},
		{
			path:    ftlPageMapFixturePath,
			readers: map[string]bool{"go/ftl-page-map": true},
		},
		{
			path:    ftlPageReclamationFixturePath,
			readers: map[string]bool{"go/ftl-page-map": true},
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
				case "c":
					sourcePath += ".c"
				case "cpp":
					sourcePath += ".cpp"
				case "go":
					sourcePath += ".go"
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
			for _, path := range []string{nandFlashFixturePath, nandPageProgramCutsFixturePath, nandBlockEraseCutsFixturePath, hddSimulatorFixturePath, hddWriteCrashCutsFixturePath, ftlPageMapFixturePath, ftlPageReclamationFixturePath} {
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
			roots:   []string{"code/packages/c/nand-flash", "code/packages/cpp/nand-flash", "code/packages/go/nand-flash", "code/packages/python/nand-flash", "code/packages/typescript/nand-flash"},
		},
		{
			id:      "repository-database-media-nand-page-program-cuts-fixture-inputs",
			fixture: nandPageProgramCutsFixturePath,
			roots:   []string{"code/packages/go/nand-page-program-cuts"},
		},
		{
			id:      "repository-database-media-nand-block-erase-cuts-fixture-inputs",
			fixture: nandBlockEraseCutsFixturePath,
			roots:   []string{"code/packages/go/nand-block-erase-cuts"},
		},
		{
			id:      "repository-database-media-hdd-fixture-inputs",
			fixture: hddSimulatorFixturePath,
			roots:   []string{"code/packages/c/hdd-simulator", "code/packages/go/hdd-simulator", "code/packages/python/hdd-simulator", "code/packages/typescript/hdd-simulator"},
		},
		{
			id:      "repository-database-media-hdd-crash-cuts-fixture-inputs",
			fixture: hddWriteCrashCutsFixturePath,
			roots:   []string{"code/packages/go/hdd-write-crash-cuts"},
		},
		{
			id:      "repository-database-media-ftl-page-map-fixture-inputs",
			fixture: ftlPageMapFixturePath,
			roots:   []string{"code/packages/go/ftl-page-map"},
		},
		{
			id:      "repository-database-media-ftl-page-reclamation-fixture-inputs",
			fixture: ftlPageReclamationFixturePath,
			roots:   []string{"code/packages/go/ftl-page-map"},
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
