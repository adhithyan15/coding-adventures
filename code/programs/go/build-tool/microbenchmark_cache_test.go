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
		{Name: "go/microbenchmark", Language: "go"},
		{Name: "go/rank-statistics", Language: "go"},
		{Name: "python/microbenchmark", Language: "python"},
		{Name: "python/rank-statistics", Language: "python"},
		{Name: "typescript/rank-statistics", Language: "typescript"},
	}
	wantInputs := []string{microbenchmarkPairsFixturePath}

	for _, pkg := range packages {
		t.Run(pkg.Name, func(t *testing.T) {
			root := t.TempDir()
			packageRoot := filepath.Join(root, "code", "packages", filepath.FromSlash(pkg.Name))
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
