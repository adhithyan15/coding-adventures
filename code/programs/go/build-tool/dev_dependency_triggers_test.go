package main

import (
	"path/filepath"
	"reflect"
	"runtime"
	"testing"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

// triggerWorld is a hand-built graph plus dev-dependency triggers:
//
//	base ──▶ emit ──▶ builder ──▶ app-lib          (build edges)
//	           ⋮         ⋮
//	           ⋮         └ ┄ ┄▶ harness ──▶ harness-user
//	           └ ┄ ┄ ┄ ┄ ┄ ┄ ┄▶ emit-tests           (┄▶ dev-dependency)
//	lonely ┄ ┄▶ other
//	cyc-a ──▶ cyc-b,  cyc-b ┄ ┄▶ cyc-a              (a dev-dependency cycle)
func triggerWorld() (*directedgraph.Graph, resolver.TestTriggers) {
	graph := directedgraph.New()
	for _, name := range []string{"base", "emit", "builder", "app-lib", "harness", "harness-user", "emit-tests", "lonely", "other", "cyc-a", "cyc-b"} {
		graph.AddNode(name)
	}
	graph.AddEdge("base", "emit")
	graph.AddEdge("emit", "builder")
	graph.AddEdge("builder", "app-lib")
	graph.AddEdge("harness", "harness-user")
	graph.AddEdge("cyc-a", "cyc-b")
	triggers := resolver.TestTriggers{
		"builder": {"harness"},
		"emit":    {"emit-tests"},
		"lonely":  {"other"},
		"cyc-b":   {"cyc-a"},
	}
	return graph, triggers
}

func TestDevDependentsAreSelectedOneHopPastTheClosure(t *testing.T) {
	graph, triggers := triggerWorld()
	got := affectedForGraphWithTriggers(graph, triggers, map[string]bool{"emit": true}, nil, false)
	want := map[string]bool{
		// The build closure of emit, as before.
		"emit": true, "builder": true, "app-lib": true,
		// Dev-dependents of anything in that closure.
		"emit-tests": true, "harness": true,
		// Prerequisites, so a fresh runner can build.
		"base": true,
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("affected = %v, want %v", got, want)
	}
	// harness-user links harness's library, which did not change.
	if got["harness-user"] {
		t.Fatal("a dev-dependent's own dependents must not be selected")
	}
}

func TestPrerequisitesDoNotTriggerDevDependents(t *testing.T) {
	graph, triggers := triggerWorld()
	graph.AddEdge("lonely", "app-lib") // lonely becomes a prerequisite of app-lib
	got := affectedForGraphWithTriggers(graph, triggers, map[string]bool{"app-lib": true}, nil, false)
	if !got["lonely"] {
		t.Fatal("lonely should be scheduled as a prerequisite")
	}
	if got["other"] {
		t.Fatal("a prerequisite did not change, so its dev-dependents must not be selected")
	}
}

func TestDevDependencyCycleIsAppliedOnceAndTerminates(t *testing.T) {
	graph, triggers := triggerWorld()
	if _, err := graph.IndependentGroups(); err != nil {
		t.Fatalf("triggers live outside the graph, which must stay acyclic: %v", err)
	}
	for _, changed := range []string{"cyc-a", "cyc-b"} {
		got := affectedForGraphWithTriggers(graph, triggers, map[string]bool{changed: true}, nil, false)
		if want := map[string]bool{"cyc-a": true, "cyc-b": true}; !reflect.DeepEqual(got, want) {
			t.Fatalf("change to %s: affected = %v, want %v", changed, got, want)
		}
	}
}

func TestWithDevDependentsEdgeCases(t *testing.T) {
	_, triggers := triggerWorld()
	if got := withDevDependents(nil, triggers); got != nil {
		t.Fatalf("nil (everything) must stay nil, got %v", got)
	}
	closure := map[string]bool{"emit": true}
	if got := withDevDependents(closure, nil); !reflect.DeepEqual(got, closure) {
		t.Fatalf("no triggers must leave the closure alone, got %v", got)
	}
	withTests := withDevDependents(closure, triggers)
	if !withTests["emit-tests"] || len(closure) != 1 {
		t.Fatalf("got %v; the input closure must not be modified (%v)", withTests, closure)
	}
	// The trigger-free wrapper keeps the old behaviour for force, fallback
	// and empty change lists.
	graph, _ := triggerWorld()
	if got := affectedForGraphWithTriggers(graph, triggers, map[string]bool{"emit": true}, nil, true); got != nil {
		t.Fatalf("force must mean everything, got %v", got)
	}
	fallback := map[string]bool{"x": true}
	if got := affectedForGraphWithTriggers(graph, triggers, nil, fallback, false); !reflect.DeepEqual(got, fallback) {
		t.Fatalf("nil change list must return the fallback, got %v", got)
	}
	if got := affectedForGraphWithTriggers(graph, triggers, map[string]bool{}, nil, false); len(got) != 0 || got == nil {
		t.Fatalf("empty change list must select nothing, got %v", got)
	}
}

// TestRealRepositoryEmitterChangeSelectsMosaicSuites is the end-to-end check
// against the checked-out repository: a one-line change to the Qt emitter
// must re-run the Mosaic package and app smoke suites, which reach the
// emitter only through their Cargo dev-dependencies on the compilers and the
// artifact builder. Before dev-dependency triggers, neither was selected.
func TestRealRepositoryEmitterChangeSelectsMosaicSuites(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	packages, err := discovery.DiscoverPackages(filepath.Join(root, "code"))
	if err != nil {
		t.Fatal(err)
	}
	graph, triggers, err := resolver.ResolveWithTestTriggers(packages)
	if err != nil {
		t.Fatalf("the repository's Cargo dev-dependency tables must all be readable: %v", err)
	}
	if _, err := graph.IndependentGroups(); err != nil {
		t.Fatalf("the build graph must stay acyclic: %v", err)
	}

	changedFile := "code/packages/rust/mosaic-emit-qt/src/pipeline.rs"
	changed, err := changedPackageRootsForPlatformAndLanguage([]string{changedFile}, packages, root, runtime.GOOS, "all")
	if err != nil {
		t.Fatal(err)
	}
	if !changed["rust/mosaic-emit-qt"] {
		t.Fatalf("%s should map to rust/mosaic-emit-qt, got %v", changedFile, changed)
	}

	without := affectedForGraph(graph, changed, nil, false)
	with := affectedForGraphWithTriggers(graph, triggers, changed, nil, false)
	// venture-browser-macos reaches the artifact builder only through a
	// [target.'cfg(target_os = "macos")'.dev-dependencies] table.
	for _, name := range []string{"mosaic/mosaic-pkg-notes", "mosaic/programs/engram-app", "rust/venture-browser-macos"} {
		if without[name] {
			t.Fatalf("%s is reachable through build edges alone; this test no longer pins the dev-dependency path", name)
		}
		if !with[name] {
			t.Fatalf("%s must be affected by a change to %s", name, changedFile)
		}
	}
	for name := range without {
		if !with[name] {
			t.Fatalf("triggers may only add packages, but %s was dropped", name)
		}
	}
}
