package main

import (
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/hasher"
)

func TestHistoricalWebVectorsSelectionHashAndKilnInputsAreExact(t *testing.T) {
	packages := []discovery.Package{
		{Name: "python/historical-web", Language: "python"},
		{Name: "go/historical-web", Language: "go"},
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
	if err != nil || !reflect.DeepEqual(selected, map[string]bool{"python/historical-web": true, "go/historical-web": true, "rust/historical-web": true, "typescript/historical-web": true, "ruby/historical_web": true, "c/historical-web": true, "cpp/historical-web": true}) {
		t.Fatalf("vector selection = %v, err %v; want all historical-web packages", selected, err)
	}
	if selected, err := repositoryInputConsumersForChange([]string{historicalWebVectorsPath}, packages, "python"); err != nil || !reflect.DeepEqual(selected, map[string]bool{"python/historical-web": true}) {
		t.Fatalf("Python-only vector selection = %v, err %v", selected, err)
	}
	if selected, err := repositoryInputConsumersForChange([]string{historicalWebVectorsPath}, packages, "rust"); err != nil || !reflect.DeepEqual(selected, map[string]bool{"rust/historical-web": true}) {
		t.Fatalf("Rust-only vector selection = %v, err %v", selected, err)
	}
	if selected, err := repositoryInputConsumersForChange([]string{historicalWebHTML2ParagraphsPath}, packages, "all"); err != nil || !reflect.DeepEqual(selected, map[string]bool{"python/historical-web": true}) {
		t.Fatalf("HTML 2.0 paragraphs selection = %v, err %v; want Python only", selected, err)
	}
	if selected, err := repositoryInputConsumersForChange([]string{historicalWebHTTP10RequestLinePath}, packages, "all"); err != nil || !reflect.DeepEqual(selected, map[string]bool{"python/historical-web": true}) {
		t.Fatalf("HTTP/1.0 request-line selection = %v, err %v; want Python only", selected, err)
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

	wantInputs := []string{historicalWebVectorsPath, historicalWebHTML2ParagraphsPath, historicalWebHTTP10RequestLinePath}
	wantKilnInputs := []string{historicalWebHTML2ParagraphsPath, historicalWebHTTP10RequestLinePath, historicalWebVectorsPath}
	if got := repositoryInputsForPackage("python/historical-web"); !reflect.DeepEqual(got, wantInputs) {
		t.Fatalf("fallback hash inputs = %v; want %v", got, wantInputs)
	}
	inputs := repositoryInputsForPackage("python/historical-web")
	root := t.TempDir()
	packageRoot := filepath.Join(root, "code", "packages", "python", "historical-web")
	if err := os.MkdirAll(packageRoot, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(packageRoot, "marker.py"), []byte("# package\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	vector := filepath.Join(root, filepath.FromSlash(historicalWebVectorsPath))
	if err := os.MkdirAll(filepath.Dir(vector), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(vector, []byte("initial\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	paragraphVector := filepath.Join(root, filepath.FromSlash(historicalWebHTML2ParagraphsPath))
	if err := os.MkdirAll(filepath.Dir(paragraphVector), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(paragraphVector, []byte("initial paragraphs\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	http10Vector := filepath.Join(root, filepath.FromSlash(historicalWebHTTP10RequestLinePath))
	if err := os.MkdirAll(filepath.Dir(http10Vector), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(http10Vector, []byte("initial HTTP/1.0\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	pkg := discovery.Package{Name: "python/historical-web", Language: "python", Path: packageRoot}
	before, err := hasher.HashPackageWithRepositoryInputs(pkg, root, inputs)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(http10Vector, []byte("changed HTTP/1.0\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	after, err := hasher.HashPackageWithRepositoryInputs(pkg, root, inputs)
	if err != nil {
		t.Fatal(err)
	}
	if before == after {
		t.Fatal("editing vectors.json did not change the package hash")
	}

	checkout := toolchainFixtureRepoRoot(t)
	packages[0].Path = filepath.Join(checkout, filepath.FromSlash("code/packages/python/historical-web"))
	packages[1].Path = filepath.Join(checkout, filepath.FromSlash("code/packages/go/historical-web"))
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
	for _, node := range graph.Packages {
		if node.Name == "python/historical-web" {
			if !reflect.DeepEqual(node.RepoInputs, wantKilnInputs) || len(node.RepoInputGlobs) != 0 || node.RepoInputsUnknown {
				t.Fatalf("Kiln inputs = %v, globs = %v, unknown = %v; want %v", node.RepoInputs, node.RepoInputGlobs, node.RepoInputsUnknown, wantKilnInputs)
			}
			return
		}
	}
	t.Fatal("python/historical-web missing from Kiln graph")
}
