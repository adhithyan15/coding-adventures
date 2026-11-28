package main

import (
	"fmt"
	"os"
	"path/filepath"
	"sync"
	"testing"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

type repositoryTriggerSnapshot struct {
	once     sync.Once
	packages []discovery.Package
	graph    *directedgraph.Graph
	triggers resolver.TestTriggers
	err      error
}

var repositoryTriggerSnapshots sync.Map // canonical repository root -> *repositoryTriggerSnapshot

// repositoryTriggerSnapshotFor returns isolated copies of one repository
// discovery/resolution snapshot. Its process-local cache is keyed by the
// canonical checkout root, so it cannot mix worktrees or temp fixtures.
func repositoryTriggerSnapshotFor(t *testing.T) ([]discovery.Package, *directedgraph.Graph, resolver.TestTriggers) {
	t.Helper()
	root, err := filepath.Abs(toolchainFixtureRepoRoot(t))
	if err != nil {
		t.Fatalf("resolve repository root: %v", err)
	}
	root, err = filepath.EvalSymlinks(root)
	if err != nil {
		t.Fatalf("canonicalize repository root: %v", err)
	}
	info, err := os.Stat(root)
	if err != nil {
		t.Fatalf("stat repository root %s: %v", root, err)
	}
	if !info.IsDir() {
		t.Fatalf("repository root %s is not a directory", root)
	}

	entryValue, _ := repositoryTriggerSnapshots.LoadOrStore(root, &repositoryTriggerSnapshot{})
	entry := entryValue.(*repositoryTriggerSnapshot)
	entry.once.Do(func() {
		entry.packages, entry.err = discovery.DiscoverPackages(filepath.Join(root, "code"))
		if entry.err != nil {
			return
		}
		if len(entry.packages) == 0 {
			entry.err = fmt.Errorf("discovery found no packages under %s/code", root)
			return
		}
		entry.graph, entry.triggers, entry.err = resolver.ResolveWithTestTriggers(entry.packages)
	})
	if entry.err != nil {
		t.Fatalf("discover and resolve repository packages: %v", entry.err)
	}
	return cloneDiscoveredPackages(entry.packages), cloneDirectedGraph(entry.graph), cloneTestTriggers(entry.triggers)
}

func cloneDiscoveredPackages(packages []discovery.Package) []discovery.Package {
	cloned := make([]discovery.Package, len(packages))
	for i, pkg := range packages {
		cloned[i] = pkg
		cloned[i].BuildCommands = append([]string(nil), pkg.BuildCommands...)
		cloned[i].DeclaredSrcs = append([]string(nil), pkg.DeclaredSrcs...)
		cloned[i].DeclaredDeps = append([]string(nil), pkg.DeclaredDeps...)
		cloned[i].ExtraToolchains = append([]string(nil), pkg.ExtraToolchains...)
	}
	return cloned
}

func cloneDirectedGraph(graph *directedgraph.Graph) *directedgraph.Graph {
	cloned := directedgraph.New()
	for _, node := range graph.Nodes() {
		cloned.AddNode(node)
	}
	for _, edge := range graph.Edges() {
		cloned.AddEdge(edge[0], edge[1])
	}
	return cloned
}

func cloneTestTriggers(triggers resolver.TestTriggers) resolver.TestTriggers {
	cloned := make(resolver.TestTriggers, len(triggers))
	for dependency, dependents := range triggers {
		cloned[dependency] = append([]string(nil), dependents...)
	}
	return cloned
}

func TestRepositoryTriggerSnapshotReturnsIsolatedCopies(t *testing.T) {
	firstPackages, firstGraph, firstTriggers := repositoryTriggerSnapshotFor(t)
	var originalCommand string
	var emitterIndex int
	for i, pkg := range firstPackages {
		if pkg.Name == "rust/mosaic-emit-qt" && len(pkg.BuildCommands) > 0 {
			emitterIndex = i
			originalCommand = pkg.BuildCommands[0]
			break
		}
	}
	if originalCommand == "" {
		t.Fatal("repository snapshot omitted rust/mosaic-emit-qt BUILD commands")
	}
	if len(firstTriggers["rust/mosaic-emit-qt"]) == 0 {
		t.Fatal("repository snapshot omitted rust/mosaic-emit-qt test triggers")
	}
	firstPackages[emitterIndex].BuildCommands[0] = "mutated"
	if err := firstGraph.RemoveNode("rust/mosaic-emit-qt"); err != nil {
		t.Fatalf("mutate cloned graph: %v", err)
	}
	firstTriggers["rust/mosaic-emit-qt"][0] = "mutated"

	secondPackages, secondGraph, secondTriggers := repositoryTriggerSnapshotFor(t)
	if secondPackages[emitterIndex].BuildCommands[0] != originalCommand {
		t.Fatal("package mutation escaped into the cached snapshot")
	}
	if !secondGraph.HasNode("rust/mosaic-emit-qt") {
		t.Fatal("graph mutation escaped into the cached snapshot")
	}
	if secondTriggers["rust/mosaic-emit-qt"][0] == "mutated" {
		t.Fatal("trigger mutation escaped into the cached snapshot")
	}
}
