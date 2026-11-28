package main

import (
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/hasher"
)

func TestGoHistoricalWebVectorsSelectionHashAndKilnInputsAreExact(t *testing.T) {
	packages := []discovery.Package{
		{Name: "go/historical-web", Language: "go"},
		{Name: "python/historical-web", Language: "python"},
		{Name: "rust/historical-web", Language: "rust"},
		{Name: "typescript/historical-web", Language: "typescript"},
		{Name: "rust/test-bootstrap", Language: "rust"},
		{Name: "typescript/test-bootstrap", Language: "typescript"},
	}
	selected, err := repositoryInputConsumersForChange([]string{historicalWebVectorsPath}, packages, "all")
	if err != nil || !reflect.DeepEqual(selected, map[string]bool{"go/historical-web": true, "python/historical-web": true, "rust/historical-web": true, "typescript/historical-web": true}) {
		t.Fatalf("all-language vector selection = %v, err %v; want all historical-web packages", selected, err)
	}
	if selected, err := repositoryInputConsumersForChange([]string{historicalWebVectorsPath}, packages, "go"); err != nil || !reflect.DeepEqual(selected, map[string]bool{"go/historical-web": true}) {
		t.Fatalf("Go-only vector selection = %v, err %v", selected, err)
	}
	if selected, err := repositoryInputConsumersForChange([]string{historicalWebVectorsPath}, packages, "python"); err != nil || !reflect.DeepEqual(selected, map[string]bool{"python/historical-web": true}) {
		t.Fatalf("Python-only vector selection = %v, err %v", selected, err)
	}
	if selected, err := repositoryInputConsumersForChange([]string{historicalWebVectorsPath}, packages, "rust"); err != nil || !reflect.DeepEqual(selected, map[string]bool{"rust/historical-web": true}) {
		t.Fatalf("Rust-only vector selection = %v, err %v", selected, err)
	}
	if selected, err := repositoryInputConsumersForChange([]string{historicalWebVectorsPath}, packages, "typescript"); err != nil || !reflect.DeepEqual(selected, map[string]bool{"typescript/historical-web": true}) {
		t.Fatalf("TypeScript-only vector selection = %v, err %v", selected, err)
	}

	for _, changed := range []string{
		historicalWebVectorsPath + ".bak",
		"code/specs/fixtures/historical-web-v1",
		"code/specs/fixtures/historical-web-v1/README.md",
		"code/specs/fixtures/historical-web-v2/vectors.json",
		`code\specs\fixtures\historical-web-v1\vectors.json`,
	} {
		if isHistoricalWebVectors(changed) {
			t.Errorf("predicate accepted lookalike %q", changed)
		}
		selected, err := repositoryInputConsumersForChange([]string{changed}, packages, "all")
		if err != nil || len(selected) != 0 {
			t.Errorf("lookalike %q selected %v, err %v; want none", changed, selected, err)
		}
	}

	inputs := repositoryInputsForPackage("go/historical-web")
	if !reflect.DeepEqual(inputs, []string{historicalWebVectorsPath}) {
		t.Fatalf("fallback hash inputs = %v; want exactly vectors.json", inputs)
	}
	rustInputs := repositoryInputsForPackage("rust/historical-web")
	if !reflect.DeepEqual(rustInputs, []string{historicalWebVectorsPath}) {
		t.Fatalf("Rust fallback hash inputs = %v; want exactly vectors.json", rustInputs)
	}
	tsInputs := repositoryInputsForPackage("typescript/historical-web")
	if !reflect.DeepEqual(tsInputs, []string{historicalWebVectorsPath}) {
		t.Fatalf("TypeScript fallback hash inputs = %v; want exactly vectors.json", tsInputs)
	}
	root := t.TempDir()
	packageRoot := filepath.Join(root, "code", "packages", "go", "historical-web")
	if err := os.MkdirAll(packageRoot, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(packageRoot, "marker.go"), []byte("package historicalweb\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	vector := filepath.Join(root, filepath.FromSlash(historicalWebVectorsPath))
	if err := os.MkdirAll(filepath.Dir(vector), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(vector, []byte("initial\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	pkg := discovery.Package{Name: "go/historical-web", Language: "go", Path: packageRoot}
	before, err := hasher.HashPackageWithRepositoryInputs(pkg, root, inputs)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(vector, []byte("changed\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	after, err := hasher.HashPackageWithRepositoryInputs(pkg, root, inputs)
	if err != nil {
		t.Fatal(err)
	}
	if before == after {
		t.Fatal("editing vectors.json did not change the Go package hash")
	}

	checkout := toolchainFixtureRepoRoot(t)
	packages[0].Path = filepath.Join(checkout, filepath.FromSlash("code/packages/go/historical-web"))
	packages[1].Path = filepath.Join(checkout, filepath.FromSlash("code/packages/python/historical-web"))
	packages[2].Path = filepath.Join(checkout, filepath.FromSlash("code/packages/rust/historical-web"))
	packages[3].Path = filepath.Join(checkout, filepath.FromSlash("code/packages/typescript/historical-web"))
	packages[4].Path = filepath.Join(checkout, filepath.FromSlash("code/packages/rust/test-bootstrap"))
	packages[5].Path = filepath.Join(checkout, filepath.FromSlash("code/packages/typescript/test-bootstrap"))
	graph, err := buildKilnLegacyGraph(packages, checkout)
	if err != nil {
		t.Fatalf("build Kiln graph: %v", err)
	}
	foundGo, foundRust, foundTypeScript := false, false, false
	for _, node := range graph.Packages {
		if node.Name != "go/historical-web" && node.Name != "rust/historical-web" && node.Name != "typescript/historical-web" {
			continue
		}
		if !reflect.DeepEqual(node.RepoInputs, []string{historicalWebVectorsPath}) || len(node.RepoInputGlobs) != 0 || node.RepoInputsUnknown {
			t.Fatalf("Kiln inputs for %s = %v, globs = %v, unknown = %v; want only vectors.json", node.Name, node.RepoInputs, node.RepoInputGlobs, node.RepoInputsUnknown)
		}
		if node.Name == "go/historical-web" {
			foundGo = true
		} else if node.Name == "rust/historical-web" {
			foundRust = true
		} else {
			foundTypeScript = true
		}
	}
	if !foundGo || !foundRust || !foundTypeScript {
		t.Fatalf("Kiln graph missing historical-web nodes: Go=%v Rust=%v TypeScript=%v", foundGo, foundRust, foundTypeScript)
	}
}
