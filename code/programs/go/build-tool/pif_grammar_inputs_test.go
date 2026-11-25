package main

import (
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// PIF-001: the grammar pair and the VER32 spec are embedded with include_str!,
// so they are not Cargo edges; a row must select rust/pif-grammar when they change.
func TestPifGrammarInputsSelectOnlyTheirConsumer(t *testing.T) {
	packages := []discovery.Package{
		{Name: "rust/pif-grammar", Language: "rust"},
		{Name: "rust/warrant-grammar", Language: "rust"},
	}
	for _, input := range []string{
		"code/grammars/pif/pif.tokens",
		"code/grammars/pif/pif.grammar",
		"code/grammars/pif/README.md",
		"code/specs/VER32-proof-interchange-format.md",
	} {
		if !isPifGrammarInput(input) {
			t.Fatalf("missing PIF input: %s", input)
		}
		got, err := repositoryInputConsumersForChange([]string{input}, packages, "all")
		if err != nil || !reflect.DeepEqual(got, map[string]bool{"rust/pif-grammar": true}) {
			t.Fatalf("%s: got %v, err %v", input, got, err)
		}
		if _, err := repositoryInputConsumersForChange([]string{input}, packages[1:], "all"); err == nil {
			t.Fatal("missing PIF consumer must fail closed")
		}
		if got, err := repositoryInputConsumersForChange([]string{input}, packages, "go"); err != nil || len(got) != 0 {
			t.Fatalf("language filtering: %v, %v", got, err)
		}
	}
	for _, input := range []string{
		"code/grammars/pifx/pif.tokens",
		"code/grammars/pif",
		"docs/code/grammars/pif/pif.tokens",
		`code\grammars\pif\pif.tokens`,
		"code/specs/VER32-proof-interchange-format.md.bak",
		"code/specs/VER32a-proof-interchange-format.md",
	} {
		if isPifGrammarInput(input) {
			t.Fatalf("lookalike selected: %s", input)
		}
	}
}
