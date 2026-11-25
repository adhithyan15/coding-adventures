package main

import (
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

func TestMicrobenchmarkPairsFixtureChangesFallbackPackageHash(t *testing.T) {
	packages := []discovery.Package{
		{Name: "go/programs/benchmark-tool", Language: "go"},
		{Name: "csharp/microbenchmark", Language: "csharp"},
		{Name: "csharp/rank-statistics", Language: "csharp"},
		{Name: "fsharp/microbenchmark", Language: "fsharp"},
		{Name: "fsharp/rank-statistics", Language: "fsharp"},
		{Name: "java/microbenchmark", Language: "java"},
		{Name: "java/rank-statistics", Language: "java"},
		{Name: "kotlin/microbenchmark", Language: "kotlin"},
		{Name: "kotlin/rank-statistics", Language: "kotlin"},
		{Name: "c/microbenchmark", Language: "c"},
		{Name: "c/rank-statistics", Language: "c"},
		{Name: "swift/microbenchmark", Language: "swift"},
		{Name: "swift/rank-statistics", Language: "swift"},
		{Name: "cpp/microbenchmark", Language: "cpp"},
		{Name: "cpp/rank-statistics", Language: "cpp"},
		{Name: "ruby/microbenchmark", Language: "ruby"},
		{Name: "ruby/rank_statistics", Language: "ruby"},
		{Name: "rust/microbenchmark", Language: "rust"},
		{Name: "rust/rank-statistics", Language: "rust"},
		{Name: "typescript/microbenchmark", Language: "typescript"},
		{Name: "typescript/rank-statistics", Language: "typescript"},
		{Name: "go/microbenchmark", Language: "go"},
		{Name: "go/rank-statistics", Language: "go"},
		{Name: "python/microbenchmark", Language: "python"},
		{Name: "python/rank-statistics", Language: "python"},
	}
	wantInputs := []string{microbenchmarkPairsFixturePath}

	for _, pkg := range packages {
		t.Run(pkg.Name, func(t *testing.T) {
			root := t.TempDir()
			packageRoot := filepath.Join(root, "code", "packages", filepath.FromSlash(pkg.Name))
			if pkg.Name == "go/programs/benchmark-tool" {
				packageRoot = filepath.Join(root, "code", "programs", "go", "benchmark-tool")
			}
			if err := os.MkdirAll(packageRoot, 0o755); err != nil {
				t.Fatal(err)
			}
			if err := os.WriteFile(filepath.Join(packageRoot, "cache_probe.txt"), []byte("source\n"), 0o644); err != nil {
				t.Fatal(err)
			}
			fixturePath := filepath.Join(root, filepath.FromSlash(microbenchmarkPairsFixturePath))
			if err := os.MkdirAll(filepath.Dir(fixturePath), 0o755); err != nil {
				t.Fatal(err)
			}
			original := []byte("{\"pairs\":[1]}\n")
			if err := os.WriteFile(fixturePath, original, 0o644); err != nil {
				t.Fatal(err)
			}
			inputs := repositoryInputsForPackage(pkg.Name)
			if !reflect.DeepEqual(inputs, wantInputs) {
				t.Fatalf("repository inputs = %v, want %v", inputs, wantInputs)
			}
			pkg.Path = packageRoot
			fallback := func(discovery.Package) (string, error) { return "fallback", nil }
			before, err := hashPackageWithRepositoryInputs(pkg, root, fallback)
			if err != nil {
				t.Fatalf("hash package with fixture: %v", err)
			}
			if err := os.WriteFile(fixturePath, []byte("{\"pairs\":[2]}\n"), 0o644); err != nil {
				t.Fatal(err)
			}
			after, err := hashPackageWithRepositoryInputs(pkg, root, fallback)
			if err != nil {
				t.Fatalf("hash package with changed fixture: %v", err)
			}
			if before == after {
				t.Fatal("fallback package hash did not change when canonical fixture bytes changed")
			}
		})
	}
}
