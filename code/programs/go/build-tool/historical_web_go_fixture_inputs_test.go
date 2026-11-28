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
		{Name: "ruby/historical_web", Language: "ruby"},
		{Name: "ruby/test_bootstrap", Language: "ruby"},
		{Name: "c/historical-web", Language: "c"},
		{Name: "c/iso-harness", Language: "c"},
		{Name: "c/test-bootstrap", Language: "c"},
		{Name: "c/bounded-json", Language: "c"},
		{Name: "cpp/historical-web", Language: "cpp"},
		{Name: "cpp/bounded-json", Language: "cpp"},
		{Name: "cpp/test-bootstrap", Language: "cpp"},
	}
	selected, err := repositoryInputConsumersForChange([]string{historicalWebVectorsPath}, packages, "all")
	if err != nil || !reflect.DeepEqual(selected, map[string]bool{"go/historical-web": true, "python/historical-web": true, "rust/historical-web": true, "typescript/historical-web": true, "ruby/historical_web": true, "c/historical-web": true, "cpp/historical-web": true}) {
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
	if selected, err := repositoryInputConsumersForChange([]string{historicalWebVectorsPath}, packages, "ruby"); err != nil || !reflect.DeepEqual(selected, map[string]bool{"ruby/historical_web": true}) {
		t.Fatalf("Ruby-only vector selection = %v, err %v", selected, err)
	}
	if selected, err := repositoryInputConsumersForChange([]string{historicalWebVectorsPath}, packages, "c"); err != nil || !reflect.DeepEqual(selected, map[string]bool{"c/historical-web": true}) {
		t.Fatalf("C-only vector selection = %v, err %v", selected, err)
	}
	if selected, err := repositoryInputConsumersForChange([]string{historicalWebVectorsPath}, packages, "cpp"); err != nil || !reflect.DeepEqual(selected, map[string]bool{"cpp/historical-web": true}) {
		t.Fatalf("C++-only vector selection = %v, err %v", selected, err)
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
	rubyInputs := repositoryInputsForPackage("ruby/historical_web")
	if !reflect.DeepEqual(rubyInputs, []string{historicalWebVectorsPath}) {
		t.Fatalf("Ruby fallback hash inputs = %v; want exactly vectors.json", rubyInputs)
	}
	cInputs := repositoryInputsForPackage("c/historical-web")
	if !reflect.DeepEqual(cInputs, []string{historicalWebVectorsPath}) {
		t.Fatalf("C fallback hash inputs = %v; want exactly vectors.json", cInputs)
	}
	cppInputs := repositoryInputsForPackage("cpp/historical-web")
	if !reflect.DeepEqual(cppInputs, []string{historicalWebVectorsPath}) {
		t.Fatalf("C++ fallback hash inputs = %v; want exactly vectors.json", cppInputs)
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
	packages[6].Path = filepath.Join(checkout, filepath.FromSlash("code/packages/ruby/historical_web"))
	packages[7].Path = filepath.Join(checkout, filepath.FromSlash("code/packages/ruby/test_bootstrap"))
	packages[8].Path = filepath.Join(checkout, filepath.FromSlash("code/packages/c/historical-web"))
	packages[9].Path = filepath.Join(checkout, filepath.FromSlash("code/packages/c/iso-harness"))
	packages[10].Path = filepath.Join(checkout, filepath.FromSlash("code/packages/c/test-bootstrap"))
	packages[11].Path = filepath.Join(checkout, filepath.FromSlash("code/packages/c/bounded-json"))
	packages[12].Path = filepath.Join(checkout, filepath.FromSlash("code/packages/cpp/historical-web"))
	packages[13].Path = filepath.Join(checkout, filepath.FromSlash("code/packages/cpp/bounded-json"))
	packages[14].Path = filepath.Join(checkout, filepath.FromSlash("code/packages/cpp/test-bootstrap"))
	graph, err := buildKilnLegacyGraph(packages, checkout)
	if err != nil {
		t.Fatalf("build Kiln graph: %v", err)
	}
	foundGo, foundRust, foundTypeScript, foundRuby, foundC, foundCPP := false, false, false, false, false, false
	for _, node := range graph.Packages {
		if node.Name != "go/historical-web" && node.Name != "rust/historical-web" && node.Name != "typescript/historical-web" && node.Name != "ruby/historical_web" && node.Name != "c/historical-web" && node.Name != "cpp/historical-web" {
			continue
		}
		if !reflect.DeepEqual(node.RepoInputs, []string{historicalWebVectorsPath}) || len(node.RepoInputGlobs) != 0 || node.RepoInputsUnknown {
			t.Fatalf("Kiln inputs for %s = %v, globs = %v, unknown = %v; want only vectors.json", node.Name, node.RepoInputs, node.RepoInputGlobs, node.RepoInputsUnknown)
		}
		if node.Name == "go/historical-web" {
			foundGo = true
		} else if node.Name == "rust/historical-web" {
			foundRust = true
		} else if node.Name == "typescript/historical-web" {
			foundTypeScript = true
		} else if node.Name == "ruby/historical_web" {
			foundRuby = true
		} else if node.Name == "c/historical-web" {
			foundC = true
		} else {
			foundCPP = true
		}
	}
	if !foundGo || !foundRust || !foundTypeScript || !foundRuby || !foundC || !foundCPP {
		t.Fatalf("Kiln graph missing historical-web nodes: Go=%v Rust=%v TypeScript=%v Ruby=%v C=%v C++=%v", foundGo, foundRust, foundTypeScript, foundRuby, foundC, foundCPP)
	}
}
