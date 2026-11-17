package main

import (
	"os"
	"path/filepath"
	"testing"
)

// VOS-033 / VOS-D058: the grammar files the VOS crates embed with include_str!
// are not Cargo edges, so a row must select the consumers when they change.
func TestVosGrammarInputsSelectTheirConsumers(t *testing.T) {
	cases := []struct {
		path        string
		grammar, sh bool
	}{
		{"code/grammars/vos_contract/vos_contract.tokens", true, false},
		{"code/grammars/vos_contract/vos_contract.grammar", true, false},
		{"code/grammars/vos_document/vos_document.tokens", true, false},
		{"code/grammars/vos_contract2/x.tokens", false, false},
		{"code/grammars/vos_contract", false, false},
		{"code/grammars/toml/toml.tokens", false, false},
		{"code/grammars/rust/rust1.97.0-e2021.tokens", false, true},
		{"code/grammars/rust/rust1.97.0-e2024.tokens", false, true},
		{"code/grammars/rust/rust1.97.0-e2018.tokens", false, false},
		{"code\\grammars\\vos_contract\\x", false, false},
	}
	for _, c := range cases {
		if got := isVosGrammarInput(c.path); got != c.grammar {
			t.Errorf("isVosGrammarInput(%q) = %v", c.path, got)
		}
		if got := isVosShapeGrammarInput(c.path); got != c.sh {
			t.Errorf("isVosShapeGrammarInput(%q) = %v", c.path, got)
		}
	}
	want := map[string]bool{"rust/vos-manifest": false, "rust/programs/vos-link": false, "rust/vos-shape-check": false}
	for _, r := range repositoryInputConsumers {
		if _, ok := want[r.name]; ok {
			want[r.name] = true
			if r.language != "rust" || len(r.paths) == 0 {
				t.Errorf("%s: language %q paths %v", r.name, r.language, r.paths)
			}
		}
	}
	for n, ok := range want {
		if !ok {
			t.Errorf("no repository input row for %s", n)
		}
	}
}

// VOS-D066: the tools a VOS implementation's BUILD runs outside Cargo, and the implementation's
// own generated document, select that implementation.
func TestVosToolInputsSelectImplementations(t *testing.T) {
	var row *repositoryInputConsumer
	for i := range repositoryInputConsumers {
		if repositoryInputConsumers[i].name == "rust/vos-sched-fp-bitmap" {
			row = &repositoryInputConsumers[i]
		}
	}
	if row == nil || row.language != "rust" || len(row.paths) == 0 {
		t.Fatalf("no usable repository input row for rust/vos-sched-fp-bitmap: %+v", row)
	}
	for path, want := range map[string]bool{
		"code/programs/rust/vos-link/src/check.rs":       true,
		"code/programs/rust/vos-link/Cargo.toml":         true,
		"code/packages/rust/vos-manifest/src/part.rs":    true,
		"code/packages/rust/vos-shape-check/src/a.rs":    true,
		"code/packages/rust/warrant-grammar/src/a.rs":    true,
		"code/packages/rust/ca-capability-analyzer/a.rs": true,
		"code/packages/rust/lexer/src/a.rs":              true,
		"code/grammars/vos_document/vos_document.tokens": true,
		// The implementation's own generated document, and only its own.
		"code/specs/vos-sched-fp-bitmap.warrant":       true,
		"code/specs/vos-sched-any.warrant":             false,
		"code/specs/vos-sched-fp-bitmap.warrant.x":     false,
		"code/specs/sub/vos-sched-fp-bitmap.warrant":   false,
		"code/programs/rust/vos-link":                  false,
		"code/packages/rust/vos-manifest-x/src/a.rs":   false,
		"code/packages/rust/lexer-x/src/a.rs":          false,
		"code/programs/rust/vos-linker/src/a.rs":       false,
		"code/packages/rust/vos-sched-contract/a.part": false,
		"code\\programs\\rust\\vos-link\\src\\a.rs":    false,
	} {
		if got := row.matches(path); got != want {
			t.Errorf("rust/vos-sched-fp-bitmap matches(%q) = %v, want %v", path, got, want)
		}
	}
	other := vosImplementationRow("vos-sched-any")
	if !other.matches("code/specs/vos-sched-any.warrant") || other.matches("code/specs/vos-sched-fp-bitmap.warrant") {
		t.Error("a row selects its own document only")
	}
	// The registered row of the -any crate (VOS-012a): present, selects its own document and the
	// VOS tools, and nothing of its sibling.
	var anyRow *repositoryInputConsumer
	for i := range repositoryInputConsumers {
		if repositoryInputConsumers[i].name == "rust/vos-sched-any" {
			anyRow = &repositoryInputConsumers[i]
		}
	}
	if anyRow == nil || anyRow.language != "rust" || len(anyRow.paths) == 0 {
		t.Fatalf("no usable repository input row for rust/vos-sched-any: %+v", anyRow)
	}
	for path, want := range map[string]bool{
		"code/specs/vos-sched-any.warrant":          true,
		"code/specs/vos-sched-fp-bitmap.warrant":    false,
		"code/specs/vos-sched-any.warrant.x":        false,
		"code/programs/rust/vos-link/src/check.rs":  true,
		"code/packages/rust/vos-manifest/src/a.rs":  true,
		"code/packages/rust/vos-sched-any/src/a.rs": false,
	} {
		if got := anyRow.matches(path); got != want {
			t.Errorf("rust/vos-sched-any matches(%q) = %v, want %v", path, got, want)
		}
	}
}

// The registered row of the round-robin crate (VOS-012b): present, selects its own document and the
// VOS tools, and nothing of its siblings (the swap exercise: replacing one implementation
// regenerates only its own).
func TestVosRoundRobinRowSelectsOnlyItsOwnDocument(t *testing.T) {
	var rrRow *repositoryInputConsumer
	for i := range repositoryInputConsumers {
		if repositoryInputConsumers[i].name == "rust/vos-sched-rr" {
			rrRow = &repositoryInputConsumers[i]
		}
	}
	if rrRow == nil || rrRow.language != "rust" || len(rrRow.paths) == 0 {
		t.Fatalf("no usable repository input row for rust/vos-sched-rr: %+v", rrRow)
	}
	for path, want := range map[string]bool{
		"code/specs/vos-sched-rr.warrant":          true,
		"code/specs/vos-sched-fp-bitmap.warrant":   false,
		"code/specs/vos-sched-any.warrant":         false,
		"code/specs/vos-sched-rr.warrant.x":        false,
		"code/programs/rust/vos-link/src/check.rs": true,
		"code/packages/rust/vos-manifest/src/a.rs": true,
		"code/packages/rust/vos-sched-rr/src/a.rs": false,
	} {
		if got := rrRow.matches(path); got != want {
			t.Errorf("rust/vos-sched-rr matches(%q) = %v, want %v", path, got, want)
		}
	}
}

// Every directory a VOS tool path names exists, so a rename cannot silently orphan the row.
func TestVosToolPathsExist(t *testing.T) {
	for _, p := range vosToolPaths {
		if _, err := os.Stat(filepath.Join("..", "..", "..", "..", p)); err != nil {
			t.Errorf("vosToolPaths: %s: %v", p, err)
		}
	}
}
