package main

import (
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

func TestWarrantGrammarInputsSelectOnlyTheirConsumer(t *testing.T) {
	packages := []discovery.Package{
		{Name: "rust/warrant-grammar", Language: "rust"},
		{Name: "rust/ct-compare", Language: "rust"},
		{Name: "rust/test-bootstrap", Language: "rust"},
		{Name: "rust/ca-assurance-report", Language: "rust"},
	}
	for _, input := range []string{
		"code/grammars/warrant/warrant.tokens",
		"code/grammars/warrant/warrant.grammar",
		"code/specs/fixtures/warrant-syntax-v1/cases.json",
		"code/specs/fixtures/warrant-syntax-v1/cases.tfv",
		"code/specs/fixtures/warrant-vectors-synthetic-v1/digests.tfv",
		"code/specs/fixtures/warrant-vectors-synthetic-v1/generate.py",
		"code/scripts/warrant_syntax_corpus.py",
		"code/scripts/tests/test_warrant_syntax_corpus.py",
		"code/specs/VER04-warrant-language.md",
		"code/specs/VER04a-warrant-source-admission.md",
	} {
		if !isWarrantGrammarInput(input) {
			t.Fatalf("missing Warrant input: %s", input)
		}
		got, err := repositoryInputConsumersForChange([]string{input}, packages, "all")
		if err != nil || !reflect.DeepEqual(got, map[string]bool{"rust/warrant-grammar": true}) {
			t.Fatalf("%s: got %v, err %v", input, got, err)
		}
		if _, err := repositoryInputConsumersForChange([]string{input}, packages[1:], "all"); err == nil {
			t.Fatal("missing Warrant consumer must fail closed")
		}
		if got, err := repositoryInputConsumersForChange([]string{input}, packages, "go"); err != nil || len(got) != 0 {
			t.Fatalf("language filtering: %v, %v", got, err)
		}
	}
	for _, input := range []string{
		"code/grammars/warrantish/warrant.tokens", "code/grammars/warrant",
		"code/specs/fixtures/warrant-syntax-v10/cases.json",
		"code/specs/fixtures/warrant-syntax-v1", "docs/code/grammars/warrant/warrant.tokens",
		"code/specs/fixtures/warrant-vectors-synthetic-v10/digests.tfv",
		"code/specs/fixtures/warrant-vectors-synthetic-v1",
		`code\grammars\warrant\warrant.tokens`, "code/scripts/warrant_syntax_corpus.pyc",
		"code/scripts/tests/test_warrant_syntax_corpus.pyc", "code/specs/VER04-warrant-language.md.bak",
	} {
		if isWarrantGrammarInput(input) {
			t.Errorf("Warrant lookalike selected: %s", input)
		}
	}
	got, err := repositoryInputConsumersForChange([]string{"code/scripts/tfv.py"}, packages, "all")
	want := map[string]bool{"rust/warrant-grammar": true, "rust/ct-compare": true, "rust/test-bootstrap": true, "rust/ca-assurance-report": true}
	if err != nil || !reflect.DeepEqual(got, want) {
		t.Fatalf("shared TFV must select every consumer: %v, %v", got, err)
	}
}

func TestWarrantLaneArtifactsSelectGrammarAndAssuranceConsumers(t *testing.T) {
	packages := []discovery.Package{
		{Name: "rust/warrant-grammar", Language: "rust"},
		{Name: "rust/ca-assurance-report", Language: "rust"},
		{Name: "rust/ct-compare", Language: "rust"},
		{Name: "rust/test-bootstrap", Language: "rust"},
	}
	for _, input := range []string{
		"code/specs/ct-compare.warrant",
		"code/specs/fixtures/ct-compare-warrant-v1/cases.tfv",
		"code/specs/fixtures/ct-compare-warrant-v1/corpus.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.schema.json",
	} {
		got, err := repositoryInputConsumersForChange([]string{input}, packages, "all")
		want := map[string]bool{"rust/warrant-grammar": true, "rust/ca-assurance-report": true}
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Fatalf("%s: got %v, err %v", input, got, err)
		}
	}
	got, err := repositoryInputConsumersForChange([]string{"code/scripts/tfv.py"}, packages, "all")
	want := map[string]bool{"rust/warrant-grammar": true, "rust/ca-assurance-report": true, "rust/ct-compare": true, "rust/test-bootstrap": true}
	if err != nil || !reflect.DeepEqual(got, want) {
		t.Fatalf("shared TFV: got %v, err %v", got, err)
	}
	for _, input := range []string{
		"code/specs/fixtures/ct-compare-warrant-v10/corpus.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.backup.json",
		"docs/code/specs/ct-compare.warrant",
	} {
		got, err := repositoryInputConsumersForChange([]string{input}, packages, "all")
		if err != nil || len(got) != 0 {
			t.Errorf("lookalike %s: got %v, err %v", input, got, err)
		}
	}
}
