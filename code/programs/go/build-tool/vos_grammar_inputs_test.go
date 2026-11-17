package main

import "testing"

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
