package main

import (
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

func TestMicrobenchmarkPlansSelectExportAndInvalidateProgram(t *testing.T) {
	root := t.TempDir()
	pkg := discovery.Package{Name: "go/programs/microbenchmark-experiments", Language: "go", Path: filepath.Join(root, "code", "programs", "go", "microbenchmark-experiments")}
	if err := os.MkdirAll(pkg.Path, 0755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(pkg.Path, "main.go"), []byte("package main\n"), 0644); err != nil {
		t.Fatal(err)
	}
	packages := []discovery.Package{pkg, {Name: "go/microbenchmark", Language: "go"}, {Name: "go/programs/benchmark-tool", Language: "go"}}
	for _, input := range microbenchmarkExperimentPlanPaths {
		if err := os.MkdirAll(filepath.Dir(filepath.Join(root, filepath.FromSlash(input))), 0755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(root, filepath.FromSlash(input)), []byte("{\"version\":1}\n"), 0644); err != nil {
			t.Fatal(err)
		}
		for _, language := range []string{"all", "go"} {
			got, err := repositoryInputConsumersForChange([]string{input}, packages, language)
			if err != nil || !reflect.DeepEqual(got, map[string]bool{pkg.Name: true}) {
				t.Fatalf("selection %s: %v %v", language, got, err)
			}
		}
		if got, err := repositoryInputConsumersForChange([]string{input}, packages, "python"); err != nil || len(got) != 0 {
			t.Fatalf("unrelated language: %v %v", got, err)
		}
		if got, err := repositoryInputConsumersForChange([]string{input}, packages[1:], "all"); err == nil || len(got) != 0 {
			t.Fatalf("missing consumer did not fail closed: %v %v", got, err)
		}
		if got, err := repositoryInputConsumersForChange([]string{input + ".bak"}, packages, "all"); err != nil || len(got) != 0 {
			t.Fatalf("neighbor selected: %v %v", got, err)
		}
	}
	if !reflect.DeepEqual(repositoryInputsForPackage(pkg.Name), microbenchmarkExperimentPlanPaths) {
		t.Fatal("fallback inputs missing")
	}
	graph, err := buildKilnLegacyGraph([]discovery.Package{pkg}, root)
	if err != nil {
		t.Fatal(err)
	}
	found := false
	for _, exported := range graph.Packages {
		if exported.Name == pkg.Name {
			found = true
			if !reflect.DeepEqual(exported.RepoInputs, microbenchmarkExperimentPlanPaths) || len(exported.RepoInputGlobs) != 0 || exported.RepoInputsUnknown {
				t.Fatalf("wrong exported inputs %+v", exported)
			}
		}
	}
	if !found {
		t.Fatal("program absent from graph")
	}
	fallback := func(discovery.Package) (string, error) { return "same-source", nil }
	for _, input := range microbenchmarkExperimentPlanPaths {
		before, err := hashPackageWithRepositoryInputs(pkg, root, fallback)
		if err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(root, filepath.FromSlash(input)), []byte("{\"version\":2}\n"), 0644); err != nil {
			t.Fatal(err)
		}
		after, err := hashPackageWithRepositoryInputs(pkg, root, fallback)
		if err != nil || before == after {
			t.Fatalf("plan bytes not hashed: %v", err)
		}
	}
}
