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

// TestDSPFiltersBootstrapChangesSelectTheirConsumers exercises the repository
// manifests and BUILD files, rather than a hand-built graph. Python's explicit
// BUILD annotation declares its local test-bootstrap input; TypeScript's local
// devDependency is part of the parsed package graph. A source edit in either
// bootstrap package must schedule its same-language dsp-filters tests.
func TestDSPFiltersBootstrapChangesSelectTheirConsumers(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	packages, err := discovery.DiscoverPackages(filepath.Join(root, "code"))
	if err != nil {
		t.Fatal(err)
	}
	graph, triggers, err := resolver.ResolveWithTestTriggers(packages)
	if err != nil {
		t.Fatalf("the repository dependency graph must resolve: %v", err)
	}
	graphs := map[string]*directedgraph.Graph{"all": graph}
	triggerSets := map[string]resolver.TestTriggers{"all": triggers}
	packageSets := map[string][]discovery.Package{"all": packages}
	for _, language := range []string{"python", "typescript"} {
		filtered := make([]discovery.Package, 0)
		for _, pkg := range packages {
			if pkg.Language == language {
				filtered = append(filtered, pkg)
			}
		}
		languageGraph, languageTriggers, err := resolver.ResolveWithTestTriggersAgainst(filtered, packages)
		if err != nil {
			t.Fatalf("the %s dependency graph must resolve: %v", language, err)
		}
		graphs[language] = languageGraph
		triggerSets[language] = languageTriggers
		packageSets[language] = filtered
	}

	required := map[string]string{
		"python/test-bootstrap":     "python",
		"python/dsp-filters":        "python",
		"typescript/test-bootstrap": "typescript",
		"typescript/dsp-filters":    "typescript",
	}
	for name, language := range required {
		found := false
		for _, pkg := range packages {
			if pkg.Name == name && pkg.Language == language {
				found = true
				break
			}
		}
		if !found {
			t.Fatalf("required %s consumer package %q is missing from discovery", language, name)
		}
	}

	for _, edge := range [][2]string{
		{"python/test-bootstrap", "python/dsp-filters"},
		{"typescript/test-bootstrap", "typescript/dsp-filters"},
	} {
		if !graph.HasEdge(edge[0], edge[1]) {
			t.Fatalf("actual BUILD/manifest metadata must declare dependency edge %s -> %s", edge[0], edge[1])
		}
	}

	tests := []struct {
		name     string
		path     string
		language string
		want     map[string]bool
	}{
		{
			name:     "Python edit with all languages",
			path:     "code/packages/python/test-bootstrap/src/coding_adventures_test_bootstrap/pbt.py",
			language: "all",
			want:     map[string]bool{"python/dsp-filters": true},
		},
		{
			name:     "Python-only edit",
			path:     "code/packages/python/test-bootstrap/src/coding_adventures_test_bootstrap/fuzz.py",
			language: "python",
			want:     map[string]bool{"python/dsp-filters": true},
		},
		{
			name:     "Python edit excluded from TypeScript run",
			path:     "code/packages/python/test-bootstrap/src/coding_adventures_test_bootstrap/pbt.py",
			language: "typescript",
			want:     map[string]bool{},
		},
		{
			name:     "TypeScript edit with all languages",
			path:     "code/packages/typescript/test-bootstrap/src/pbt.ts",
			language: "all",
			want:     map[string]bool{"typescript/dsp-filters": true},
		},
		{
			name:     "TypeScript-only edit",
			path:     "code/packages/typescript/test-bootstrap/src/fuzz.ts",
			language: "typescript",
			want:     map[string]bool{"typescript/dsp-filters": true},
		},
		{
			name:     "TypeScript edit excluded from Python run",
			path:     "code/packages/typescript/test-bootstrap/src/pbt.ts",
			language: "python",
			want:     map[string]bool{},
		},
		{
			name:     "Python package lookalike",
			path:     "code/packages/python/test-bootstrap-v10/src/coding_adventures_test_bootstrap/pbt.py",
			language: "all",
			want:     map[string]bool{},
		},
		{
			name:     "TypeScript package lookalike",
			path:     "code/packages/typescript/test-bootstrapper/src/pbt.ts",
			language: "all",
			want:     map[string]bool{},
		},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			selectedPackages := packageSets[tc.language]
			selectedGraph := graphs[tc.language]
			selectedTriggers := triggerSets[tc.language]
			if selectedGraph == nil {
				t.Fatalf("missing dependency graph for language %q", tc.language)
			}
			changed, err := changedPackageRootsForPlatformAndLanguage(
				[]string{tc.path}, selectedPackages, root, runtime.GOOS, tc.language,
			)
			if err != nil {
				t.Fatal(err)
			}
			got := affectedForGraphWithTriggers(selectedGraph, selectedTriggers, changed, nil, false)
			selected := make(map[string]bool)
			for name := range tc.want {
				if got[name] {
					selected[name] = true
				}
			}
			for _, name := range []string{"python/dsp-filters", "typescript/dsp-filters"} {
				if got[name] && !tc.want[name] {
					selected[name] = true
				}
			}
			if !reflect.DeepEqual(selected, tc.want) {
				t.Fatalf("affected consumers = %v, want %v (roots %v)", selected, tc.want, changed)
			}
		})
	}
}
