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
		"typescript": true, "rust": true, "elixir": true, "lua": true, "perl": true,
		"swift": true, "dart": true, "java": true, "kotlin": true, "haskell": true, "dotnet": true,
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

	if found[gitdiff.CIWorkflowPath] {
		t.Fatalf("%q should be analyzed diff-by-diff, not blindly forced via sharedPrefixes", gitdiff.CIWorkflowPath)
	}

	if gitdiff.CIWorkflowPath != ".github/workflows/ci.yml" {
		t.Fatalf("unexpected ci workflow path: %q", gitdiff.CIWorkflowPath)
	}

	// Regression guard: the following paths must NOT be in sharedPrefixes.
	// code/grammars/ and code/specs/ are shared data, not build infrastructure —
	// changes there only affect packages that import them, not all languages.
	// code/programs/go/build-tool/ is a program, not a shared library — changing
	// it should only rebuild the build-tool package, not trigger a full 715-package
	// rebuild that exposes pre-existing failures on every platform.
	// Deployment workflows are intentionally excluded so GitHub Pages changes
	// do not trigger a full-force rebuild of the monorepo.
	for _, dontWant := range []string{
		".github/",
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

	for _, toolchain := range []string{"python", "dotnet"} {
		if !needed[toolchain] {
			t.Fatalf("expected %s to be enabled, got %v", toolchain, needed)
		}
	}

	for _, toolchain := range []string{"go", "rust"} {
		if needed[toolchain] {
			t.Fatalf("did not expect unrelated %s toolchain: %v", toolchain, needed)
		}
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
		{Name: "dart/hello-world", Language: "dart"},
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
			wantLangs:   map[string]bool{"python": true},
		},
		{
			name:        "multiple languages affected",
			affectedSet: map[string]bool{"python/logic-gates": true, "rust/starlark-vm": true, "dart/hello-world": true},
			wantLangs:   map[string]bool{"python": true, "rust": true, "dart": true},
		},
		{
			name:        "empty affected set",
			affectedSet: map[string]bool{},
			wantLangs:   map[string]bool{},
		},
		{
			name:        "all affected",
			affectedSet: map[string]bool{"python/logic-gates": true, "go/directed-graph": true, "ruby/starlark_vm": true, "typescript/starlark-vm": true, "rust/starlark-vm": true, "elixir/starlark_vm": true},
			wantLangs:   map[string]bool{"python": true, "ruby": true, "go": true, "typescript": true, "rust": true, "elixir": true},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			needed := computeLanguagesNeeded(packages, tt.affectedSet, false, nil)

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
	needed := computeLanguagesNeeded(nil, nil, true, nil)

	for _, lang := range allToolchains {
		if !needed[lang] {
			t.Errorf("force mode: language %s should be needed", lang)
		}
	}
}

// TestNilAffectedSetMeansAllLanguages verifies that nil affectedSet
// (git diff unavailable) marks all languages needed.
func TestNilAffectedSetMeansAllLanguages(t *testing.T) {
	needed := computeLanguagesNeeded(nil, nil, false, nil)

	for _, lang := range allToolchains {
		if !needed[lang] {
			t.Errorf("nil affectedSet: language %s should be needed", lang)
		}
	}
}

// TestGoOnlyNeededForGoPackages verifies the Go language flag means Go package
// code is affected, not merely that CI must compile the build tool.
func TestGoOnlyNeededForGoPackages(t *testing.T) {
	packages := []discovery.Package{
		{Name: "python/logic-gates", Language: "python"},
		{Name: "go/directed-graph", Language: "go"},
	}

	needed := computeLanguagesNeeded(packages, map[string]bool{"python/logic-gates": true}, false, nil)
	if needed["go"] {
		t.Error("Go should not be marked needed for a Python-only package change")
	}

	needed = computeLanguagesNeeded(packages, map[string]bool{"go/directed-graph": true}, false, nil)
	if !needed["go"] {
		t.Error("Go should be marked needed when a Go package changes")
	}
}

func TestToolchainForPackageLanguage(t *testing.T) {
	tests := map[string]string{
		"wasm":    "rust",
		"csharp":  "dotnet",
		"fsharp":  "dotnet",
		"dotnet":  "dotnet",
		"dart":    "dart",
		"java":    "java",
		"kotlin":  "kotlin",
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
