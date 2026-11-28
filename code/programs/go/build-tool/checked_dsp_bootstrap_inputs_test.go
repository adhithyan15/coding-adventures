package main

import (
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/validator"
)

// TestDSPFiltersBootstrapChangesSelectTheirConsumers exercises the repository
// manifests and BUILD files, rather than a hand-built graph. The Python and
// TypeScript dev dependencies must resolve to their actual package nodes, and
// each discovered recipe must declare a runnable local install. A bootstrap
// source edit must schedule its real same-language test consumers.
func TestDSPFiltersBootstrapChangesSelectTheirConsumers(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	packages, graph, triggers := repositoryTriggerSnapshotFor(t)
	var err error
	validationPackages := make([]discovery.Package, 0, 4)
	for _, pkg := range packages {
		switch pkg.Name {
		case "python/dsp-filters", "python/dsp-stft", "python/music-analysis", "typescript/dsp-filters":
			validationPackages = append(validationPackages, pkg)
		}
	}
	if len(validationPackages) != 4 {
		t.Fatalf("discovered %d of the 4 changed test consumers", len(validationPackages))
	}
	if err := validator.ValidateBuildFilesAgainst(validationPackages, packages, graph); err != nil {
		t.Fatalf("actual repository BUILD files must declare graph prerequisites: %v", err)
	}
	for _, recipe := range []struct{ path, command string }{
		{"code/packages/python/dsp-filters/BUILD_windows", "# build-tool: deps=python/test-bootstrap"},
		{"code/packages/python/dsp-filters/BUILD_windows", "uv pip install --no-deps -e ../test-bootstrap"},
		{"code/packages/python/dsp-stft/BUILD_windows", "uv pip install --no-deps -e ../test-bootstrap"},
		{"code/packages/typescript/dsp-filters/BUILD_windows", "npm ci --prefix ../test-bootstrap"},
		{"code/packages/typescript/dsp-filters/BUILD_windows", "npm run build --prefix ../test-bootstrap"},
	} {
		contents, err := os.ReadFile(filepath.Join(root, recipe.path))
		if err != nil {
			t.Fatalf("read Windows recipe %s: %v", recipe.path, err)
		}
		if !strings.Contains(string(contents), recipe.command) {
			t.Errorf("Windows recipe %s omits the real bootstrap setup command %q", recipe.path, recipe.command)
		}
	}
	windowsPackages := packagesForPlatform(packages, "windows")
	windowsGraph, windowsTriggers, err := resolver.ResolveWithTestTriggersAgainst(windowsPackages, windowsPackages)
	if err != nil {
		t.Fatalf("the actual Windows dependency graph must resolve: %v", err)
	}
	windowsConsumers := make([]discovery.Package, 0, 3)
	for _, name := range []string{"python/dsp-filters", "python/dsp-stft", "typescript/dsp-filters"} {
		var found bool
		for _, pkg := range windowsPackages {
			if pkg.Name != name {
				continue
			}
			buildPath := discovery.GetBuildFileForPlatform(pkg.Path, "windows")
			if filepath.Base(buildPath) != "BUILD_windows" {
				t.Fatalf("%s Windows selection resolved to %s", name, buildPath)
			}
			windowsConsumers = append(windowsConsumers, pkg)
			found = true
			break
		}
		if !found {
			t.Fatalf("missing discovered Windows consumer %s", name)
		}
	}
	if !windowsGraph.HasEdge("python/test-bootstrap", "python/dsp-filters") {
		t.Fatal("the Windows-selected graph must include bootstrap -> Python DSP filters")
	}
	if err := validator.ValidateBuildFilesAgainst(windowsConsumers, windowsPackages, windowsGraph); err != nil {
		t.Fatalf("Windows-selected DSP recipes must validate their actual prerequisites: %v", err)
	}
	// Prove the Windows edge assertion is sensitive to platform-selected
	// content: removing only its annotation must remove the edge even though
	// the POSIX BUILD still declares it.
	withoutWindowsAnnotation := append([]discovery.Package(nil), windowsPackages...)
	for i := range withoutWindowsAnnotation {
		if withoutWindowsAnnotation[i].Name != "python/dsp-filters" {
			continue
		}
		const annotation = "# build-tool: deps=python/test-bootstrap"
		if !strings.Contains(withoutWindowsAnnotation[i].BuildContent, annotation) {
			t.Fatal("Windows Python DSP filters recipe has no prerequisite annotation to mutate")
		}
		withoutWindowsAnnotation[i].BuildContent = strings.Replace(withoutWindowsAnnotation[i].BuildContent, annotation+"\n", "", 1)
		withoutWindowsAnnotation[i].BuildCommands = strings.Split(withoutWindowsAnnotation[i].BuildContent, "\n")
	}
	withoutGraph, withoutTriggers, err := resolver.ResolveWithTestTriggersAgainst(withoutWindowsAnnotation, withoutWindowsAnnotation)
	if err != nil {
		t.Fatalf("the mutated Windows graph must resolve: %v", err)
	}
	if withoutGraph.HasEdge("python/test-bootstrap", "python/dsp-filters") {
		t.Fatal("removing the Windows-only annotation must remove the bootstrap edge")
	}
	for _, source := range []struct{ language, path string }{
		{"python", "code/packages/python/test-bootstrap/src/coding_adventures_test_bootstrap/pbt.py"},
		{"typescript", "code/packages/typescript/test-bootstrap/src/pbt.ts"},
	} {
		changed, err := changedPackageRootsForPlatformAndLanguage(
			[]string{source.path}, windowsPackages, root, "windows", source.language,
		)
		if err != nil {
			t.Fatalf("resolve Windows %s bootstrap change: %v", source.language, err)
		}
		got := affectedForGraphWithTriggers(windowsGraph, windowsTriggers, changed, nil, false)
		consumer := source.language + "/dsp-filters"
		if !got[consumer] {
			t.Errorf("Windows affected graph omits %s after bootstrap edit (roots %v; full set %v)", consumer, changed, got)
		}
		if source.language == "python" {
			withoutRoots, err := changedPackageRootsForPlatformAndLanguage(
				[]string{source.path}, withoutWindowsAnnotation, root, "windows", source.language,
			)
			if err != nil {
				t.Fatalf("resolve mutated Windows Python roots: %v", err)
			}
			withoutAffected := affectedForGraphWithTriggers(withoutGraph, withoutTriggers, withoutRoots, nil, false)
			if withoutAffected[consumer] {
				t.Fatal("Python filters must not be selected after its Windows-only edge is removed")
			}
		}
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
		"python/dsp-stft":           "python",
		"python/music-analysis":     "python",
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
		{"python/dsp-filters", "python/dsp-stft"},
		{"python/dsp-filters", "python/music-analysis"},
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
		absent   []string
	}{
		{
			name:     "Python edit with all languages",
			path:     "code/packages/python/test-bootstrap/src/coding_adventures_test_bootstrap/pbt.py",
			language: "all",
			want:     map[string]bool{"python/dsp-filters": true, "python/dsp-stft": true, "python/music-analysis": true},
			absent:   []string{"typescript/dsp-filters"},
		},
		{
			name:     "Python-only edit",
			path:     "code/packages/python/test-bootstrap/src/coding_adventures_test_bootstrap/fuzz.py",
			language: "python",
			want:     map[string]bool{"python/dsp-filters": true, "python/dsp-stft": true, "python/music-analysis": true},
			absent:   []string{"typescript/dsp-filters"},
		},
		{
			name:     "Python edit excluded from TypeScript run",
			path:     "code/packages/python/test-bootstrap/src/coding_adventures_test_bootstrap/pbt.py",
			language: "typescript",
			want:     map[string]bool{},
			absent:   []string{"python/dsp-filters", "python/dsp-stft", "python/music-analysis"},
		},
		{
			name:     "TypeScript edit with all languages",
			path:     "code/packages/typescript/test-bootstrap/src/pbt.ts",
			language: "all",
			want:     map[string]bool{"typescript/dsp-filters": true},
			absent:   []string{"python/dsp-filters", "python/dsp-stft", "python/music-analysis"},
		},
		{
			name:     "TypeScript-only edit",
			path:     "code/packages/typescript/test-bootstrap/src/fuzz.ts",
			language: "typescript",
			want:     map[string]bool{"typescript/dsp-filters": true},
			absent:   []string{"python/dsp-filters", "python/dsp-stft", "python/music-analysis"},
		},
		{
			name:     "TypeScript edit excluded from Python run",
			path:     "code/packages/typescript/test-bootstrap/src/pbt.ts",
			language: "python",
			want:     map[string]bool{},
			absent:   []string{"python/dsp-filters", "python/dsp-stft", "python/music-analysis"},
		},
		{
			name:     "Python package lookalike",
			path:     "code/packages/python/test-bootstrap-v10/src/coding_adventures_test_bootstrap/pbt.py",
			language: "all",
			want:     map[string]bool{},
			absent:   []string{"python/dsp-filters", "python/dsp-stft", "python/music-analysis"},
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
			for name := range tc.want {
				if !got[name] {
					t.Errorf("affected set omits required consumer %q (roots %v; full set %v)", name, changed, got)
				}
			}
			for _, name := range tc.absent {
				if got[name] {
					t.Errorf("affected set unexpectedly includes %q (roots %v; full set %v)", name, changed, got)
				}
			}
			if tc.name == "Python package lookalike" || tc.name == "TypeScript package lookalike" {
				if len(got) != 0 {
					t.Fatalf("lookalike path selected packages %v (roots %v)", got, changed)
				}
			}
		})
	}
}
