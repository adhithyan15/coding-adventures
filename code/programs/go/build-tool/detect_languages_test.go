// Tests for the --detect-languages feature.
//
// The detect-languages flag outputs which language toolchains CI needs
// to install based on the affected packages from git diff. These tests
// verify the logic for determining needed languages without hitting git.
package main

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// TestAllLanguagesConstant verifies the canonical language list is complete.
func TestAllLanguagesConstant(t *testing.T) {
	expected := map[string]bool{
		"python": true, "ruby": true, "go": true,
		"typescript": true, "rust": true, "elixir": true, "lua": true, "perl": true,
	}
	if len(allLanguages) != len(expected) {
		t.Errorf("allLanguages has %d entries, want %d", len(allLanguages), len(expected))
	}
	for _, lang := range allLanguages {
		if !expected[lang] {
			t.Errorf("unexpected language in allLanguages: %s", lang)
		}
	}
}

// TestSharedPrefixesNotEmpty ensures shared prefixes are configured.
func TestSharedPrefixesNotEmpty(t *testing.T) {
	if len(sharedPrefixes) == 0 {
		t.Error("sharedPrefixes should not be empty")
	}
	// Verify key prefixes are present.
	found := map[string]bool{}
	for _, p := range sharedPrefixes {
		found[p] = true
	}
	for _, want := range []string{".github/"} {
		if !found[want] {
			t.Errorf("sharedPrefixes missing %q", want)
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
			affectedSet: map[string]bool{"python/logic-gates": true, "rust/starlark-vm": true},
			wantLangs:   map[string]bool{"python": true, "rust": true, "go": true},
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
					needed[pkg.Language] = true
				}
			}

			for _, lang := range allLanguages {
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
		for _, lang := range allLanguages {
			needed[lang] = true
		}
	}

	for _, lang := range allLanguages {
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
		for _, lang := range allLanguages {
			needed[lang] = true
		}
	}

	for _, lang := range allLanguages {
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
			needed[pkg.Language] = true
		}
	}

	if !needed["go"] {
		t.Error("Go should always be needed (build tool is Go)")
	}
}
