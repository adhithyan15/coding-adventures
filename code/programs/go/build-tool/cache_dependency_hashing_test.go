package main

import (
	"testing"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/hasher"
)

func TestCacheDependencyHashesWaitForLaterPackageHashes(t *testing.T) {
	graph := directedgraph.New()
	graph.AddNode("typescript/game-input-frame")
	graph.AddNode("rust/typescript-warrant-ir")
	graph.AddEdge("rust/typescript-warrant-ir", "typescript/game-input-frame")

	providerHash := "provider-source-hash"
	consumerHash := "consumer-source-hash"
	packages := []discovery.Package{
		{Name: "typescript/game-input-frame"},
		{Name: "rust/typescript-warrant-ir"},
	}
	want, err := hasher.HashDeps(
		"typescript/game-input-frame",
		graph,
		map[string]string{"typescript/game-input-frame": consumerHash, "rust/typescript-warrant-ir": providerHash},
	)
	if err != nil {
		t.Fatal(err)
	}

	packageHashes, depsHashes, failed, err := hashPackagesAndDependencies(
		packages,
		graph,
		func(pkg discovery.Package) (string, error) {
			if pkg.Name == "typescript/game-input-frame" {
				return consumerHash, nil
			}
			return providerHash, nil
		},
	)
	if err != nil {
		t.Fatalf("hash cache inputs for %s: %v", failed, err)
	}
	if packageHashes["rust/typescript-warrant-ir"] != providerHash {
		t.Fatal("provider package hash was not recorded")
	}

	if got := depsHashes["typescript/game-input-frame"]; got != want {
		t.Fatalf("consumer dependency hash was computed before its later provider: got %s, want %s", got, want)
	}
}
