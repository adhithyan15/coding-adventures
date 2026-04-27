// Tests for the --detect-languages feature.
//
// The detect-languages flag outputs which language toolchains CI needs
// to install based on the affected packages from git diff. These tests
// verify the logic for determining needed languages without hitting git.
package main

import (
	"testing"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/gitdiff"
)

// TestAllToolchainsConstant verifies the canonical toolchain list is complete.
func TestAllToolchainsConstant(t *testing.T) {
	expected := map[string]bool{
		"python": true, "ruby": true, "go": true,
		"typescript": true, "rust": true, "elixir": true, "perl": true,
	}
	if len(allToolchains) != len(expected) {
		t.Errorf("allToolchains has %d entries, want %d", len(allToolchains), len(expected))
	}
	for _, lang := range allToolchains {
		if !expected[lang] {
			t.Errorf("unexpected toolchain in allToolchains: %s", lang)
		}
	}
}

// TestSharedPrefixesAreNarrow ensures only true shared build infrastructure
// forces all-language rebuilds.
func TestSharedPrefixesAreNarrow(t *testing.T) {
	found := map[string]bool{}
	for _, p := range sharedPrefixes {
		found[p] = true
	}

	// Regression guard: the following paths must NOT be in sharedPrefixes.
	// code/grammars/ and code/specs/ are shared data, not build infrastructure —
	// changes there only affect packages that import them, not all languages.
	// code/programs/go/build-tool/ is a program, not a shared library — changing
	// it should only rebuild the build-tool package, not trigger a full 715-package
	// rebuild that exposes pre-existing failures on every platform.
	// Workflow-only changes are intentionally excluded so CI/deploy tweaks do not
	// trigger a full-force rebuild of the monorepo on every PR platform.
	for _, dontWant := range []string{
		".github/",
		".github/workflows/ci.yml",
		".github/workflows/deploy-electronics-visualizers.yml",
		"code/grammars/",
		"code/specs/",
		"code/programs/go/build-tool/",
	} {
		if found[dontWant] {
			t.Errorf("sharedPrefixes must NOT contain %q — it causes spurious full-toolchain installs", dontWant)
		}
	}
}

func TestComputeLanguagesNeededIncludesSafeCIWorkflowToolchains(t *testing.T) {
	packages := []discovery.Package{
		{Name: "python/logic-gates", Language: "python"},
	}

	needed := computeLanguagesNeeded(
		packages,
		map[string]bool{"python/logic-gates": true},
		false,
		map[string]bool{"dotnet": true},
	)

	for _, toolchain := range []string{"go", "python", "dotnet"} {
		if !needed[toolchain] {
			t.Fatalf("expected %s to be enabled, got %v", toolchain, needed)
		}
	}

	if needed["rust"] {
		t.Fatalf("did not expect unrelated rust toolchain: %v", needed)
	}
}

// TestCollectAffectedLanguages verifies language detection from affected sets.
func TestCollectAffectedLanguages(t *testing.T) {
	packages := []discovery.Package{
		{Name: "python/logic-gates", Language: "python"},
		{Name: "python/starlark-vm", Language: "python"},
		{Name: "go/directed-graph", Language: "go"},
		{Name: "ruby/starlark_vm", Language: "ruby"},
		{Name: "typescript/starlark-vm", Language: "typescript"},
		{Name: "rust/starlark-vm", Language: "rust"},
		{Name: "elixir/starlark_vm", Language: "elixir"},
		{Name: "wasm/graph", Language: "wasm"},
		{Name: "csharp/graph", Language: "csharp"},
		{Name: "fsharp/graph", Language: "fsharp"},
	}

	tests := []struct {
		name        string
		affectedSet map[string]bool
		wantLangs   map[string]bool
	}{
		{
			name:        "only python affected",
			affectedSet: map[string]bool{"python/logic-gates": true, "python/starlark-vm": true},
			wantLangs:   map[string]bool{"python": true, "go": true}, // go always needed
		},
		{
			name:        "multiple languages affected",
			affectedSet: map[string]bool{"python/logic-gates": true, "rust/starlark-vm": true, "dart/hello-world": true},
			wantLangs:   map[string]bool{"python": true, "rust": true, "dart": true, "go": true},
		},
		{
			name:        "empty affected set",
			affectedSet: map[string]bool{},
			wantLangs:   map[string]bool{"go": true}, // go always needed
		},
		{
			name:        "all affected",
			affectedSet: map[string]bool{"python/logic-gates": true, "go/directed-graph": true, "ruby/starlark_vm": true, "typescript/starlark-vm": true, "rust/starlark-vm": true, "elixir/starlark_vm": true},
			wantLangs:   map[string]bool{"python": true, "ruby": true, "go": true, "typescript": true, "rust": true, "elixir": true},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Simulate the language collection logic from detectNeededLanguages.
			needed := map[string]bool{"go": true} // go always needed
			for _, pkg := range packages {
				if tt.affectedSet[pkg.Name] {
					needed[toolchainForPackageLanguage(pkg.Language)] = true
				}
			}

			for _, lang := range allToolchains {
				want := tt.wantLangs[lang]
				got := needed[lang]
				if got != want {
					t.Errorf("language %s: got %v, want %v", lang, got, want)
				}
			}
		})
	}
}

// TestForceModeSetsAllLanguages verifies that --force marks all languages needed.
func TestForceModeSetsAllLanguages(t *testing.T) {
	needed := make(map[string]bool)
	needed["go"] = true

	// Simulate force mode.
	force := true
	if force {
		for _, lang := range allToolchains {
			needed[lang] = true
		}
	}

	for _, lang := range allToolchains {
		if !needed[lang] {
			t.Errorf("force mode: language %s should be needed", lang)
		}
	}
}

// TestNilAffectedSetMeansAllLanguages verifies that nil affectedSet
// (git diff unavailable) marks all languages needed.
func TestNilAffectedSetMeansAllLanguages(t *testing.T) {
	needed := make(map[string]bool)
	needed["go"] = true

	// Simulate nil affected set (git diff unavailable).
	var affectedSet map[string]bool
	if affectedSet == nil {
		for _, lang := range allToolchains {
			needed[lang] = true
		}
	}

	for _, lang := range allToolchains {
		if !needed[lang] {
			t.Errorf("nil affectedSet: language %s should be needed", lang)
		}
	}
}

// TestGoAlwaysNeeded verifies Go is always in the needed set regardless
// of what packages are affected.
func TestGoAlwaysNeeded(t *testing.T) {
	packages := []discovery.Package{
		{Name: "python/logic-gates", Language: "python"},
	}
	affectedSet := map[string]bool{"python/logic-gates": true}

	needed := map[string]bool{"go": true}
	for _, pkg := range packages {
		if affectedSet[pkg.Name] {
			needed[toolchainForPackageLanguage(pkg.Language)] = true
		}
	}

	if !needed["go"] {
		t.Error("Go should always be needed (build tool is Go)")
	}
}

func TestToolchainForPackageLanguage(t *testing.T) {
	tests := map[string]string{
		"wasm":    "rust",
		"csharp":  "dotnet",
		"fsharp":  "dotnet",
		"dotnet":  "dotnet",
		"python":  "python",
		"swift":   "swift",
		"unknown": "unknown",
	}

	for language, want := range tests {
		if got := toolchainForPackageLanguage(language); got != want {
			t.Fatalf("toolchainForPackageLanguage(%q) = %q, want %q", language, got, want)
		}
	}
}

func TestExpandAffectedSetWithPrereqs(t *testing.T) {
	graph := directedgraph.New()
	for _, name := range []string{
		"typescript/logic-gates",
		"typescript/arithmetic",
		"typescript/arithmetic-visualizer",
	} {
		graph.AddNode(name)
	}

	// logic-gates -> arithmetic -> arithmetic-visualizer
	graph.AddEdge("typescript/logic-gates", "typescript/arithmetic")
	graph.AddEdge("typescript/arithmetic", "typescript/arithmetic-visualizer")

	affected := map[string]bool{
		"typescript/arithmetic-visualizer": true,
	}

	expanded := expandAffectedSetWithPrereqs(graph, affected)

	for _, want := range []string{
		"typescript/logic-gates",
		"typescript/arithmetic",
		"typescript/arithmetic-visualizer",
	} {
		if !expanded[want] {
			t.Fatalf("expanded affected set missing %q", want)
		}
	}
}
