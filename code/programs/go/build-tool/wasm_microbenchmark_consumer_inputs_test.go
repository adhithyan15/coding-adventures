package main

import (
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/hasher"
)

func TestWasmMicrobenchmarkConsumerSelectsHashesAndExportsExactInput(t *testing.T) {
	consumer := wasmMicrobenchmarkConsumerPath
	root := t.TempDir()
	packageRoot := filepath.Join(root, "code", "packages", "wasm", "microbenchmark")
	if err := os.MkdirAll(packageRoot, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(packageRoot, "Cargo.toml"), []byte("[package]\nname='fixture'\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	consumerPath := filepath.Join(root, filepath.FromSlash(consumer))
	if err := os.MkdirAll(filepath.Dir(consumerPath), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(consumerPath, []byte("assertions-v1\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	pkg := discovery.Package{Name: "wasm/microbenchmark", Language: "wasm", Path: packageRoot}
	packages := []discovery.Package{pkg}

	selected, err := repositoryInputConsumersForChange([]string{consumer}, packages, "all")
	if err != nil || !reflect.DeepEqual(selected, map[string]bool{pkg.Name: true}) {
		t.Fatalf("consumer selection = %v, err %v; want %s", selected, err, pkg.Name)
	}
	selected, err = repositoryInputConsumersForChange([]string{consumer}, packages, "wasm")
	if err != nil || !reflect.DeepEqual(selected, map[string]bool{pkg.Name: true}) {
		t.Fatalf("WASM-only consumer selection = %v, err %v", selected, err)
	}
	if _, err := repositoryInputConsumersForChange([]string{consumer}, nil, "all"); err == nil {
		t.Fatal("missing WASM consumer package must fail closed")
	}
	for _, lookalike := range []string{consumer + ".bak", "code/tests/wasm-microbenchmark-consumer", `code\\tests\\wasm-microbenchmark-consumer\\consumer.mjs`} {
		if isWasmMicrobenchmarkConsumer(lookalike) {
			t.Errorf("consumer predicate accepted lookalike %q", lookalike)
		}
		selected, err := repositoryInputConsumersForChange([]string{lookalike}, packages, "all")
		if err != nil || len(selected) != 0 {
			t.Errorf("lookalike %q selected %v, err %v; want none", lookalike, selected, err)
		}
	}

	inputs := repositoryInputsForPackage(pkg.Name)
	if !reflect.DeepEqual(inputs, []string{consumer}) {
		t.Fatalf("hash inputs = %v; want only %s", inputs, consumer)
	}
	before, err := hasher.HashPackageWithRepositoryInputs(pkg, root, inputs)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(consumerPath, []byte("assertions-v2\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	after, err := hasher.HashPackageWithRepositoryInputs(pkg, root, inputs)
	if err != nil {
		t.Fatal(err)
	}
	if before == after {
		t.Fatal("editing the external consumer did not change the WASM package hash")
	}

	graph, err := buildKilnLegacyGraph(packages, root)
	if err != nil {
		t.Fatalf("build Kiln graph: %v", err)
	}
	for _, node := range graph.Packages {
		if node.Name == pkg.Name {
			if !reflect.DeepEqual(node.RepoInputs, []string{consumer}) || len(node.RepoInputGlobs) != 0 || node.RepoInputsUnknown {
				t.Fatalf("Kiln inputs = %v, globs = %v, unknown = %v; want exact consumer script", node.RepoInputs, node.RepoInputGlobs, node.RepoInputsUnknown)
			}
			return
		}
	}
	t.Fatal("wasm/microbenchmark missing from Kiln graph")
}
