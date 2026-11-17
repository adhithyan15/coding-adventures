package main

import (
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/globmatch"
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
		{Name: "python/warrant-corpus", Language: "python"},
		{Name: "python/ct-compare", Language: "python"},
		{Name: "typescript/warrant-corpus", Language: "typescript"},
		{Name: "typescript/ct-compare", Language: "typescript"},
		{Name: "go/warrant-corpus", Language: "go"},
		{Name: "go/ct-compare", Language: "go"},
		{Name: "python/warrant-oracle", Language: "python"},
		{Name: "rust/warrant-corpus-gen", Language: "rust"},
		{Name: "rust/arithmetic", Language: "rust"},
		{Name: "rust/gf256", Language: "rust"},
		{Name: "rust/logic-gates", Language: "rust"},
		{Name: "rust/lzw", Language: "rust"},
		{Name: "rust/wasm-leb128", Language: "rust"},
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
		if isWarrantPythonLaneCorpusInput(input) {
			// WD-101: the Python lane runner and the ct-compare pilot also
			// read the spec, the corpus and the registry.
			want["python/warrant-corpus"] = true
			want["python/ct-compare"] = true
		}
		if isWarrantTypeScriptLaneCorpusInput(input) {
			// WD-102: the TypeScript lane runner and its ct-compare pilot
			// read the same spec, corpus and registry.
			want["typescript/warrant-corpus"] = true
			want["typescript/ct-compare"] = true
		}
		if isWarrantGoLaneCorpusInput(input) {
			// WD-103: the Go lane runner and its ct-compare pilot read the same
			// spec, corpus and registry.
			want["go/warrant-corpus"] = true
			want["go/ct-compare"] = true
		}
		if isWarrantLaneRegistryInput(input) {
			// WD-004b2: every assurance_report.json lists a rung per registry lane.
			for _, name := range []string{"arithmetic", "ct-compare", "gf256", "logic-gates", "lzw", "wasm-leb128"} {
				want["rust/"+name] = true
			}
		}
		if isWarrantOracleInput(input) {
			// VER04r S4: the independent oracle parses the spec and gates the corpus.
			want["python/warrant-oracle"] = true
		}
		if isWarrantCorpusGenInput(input) {
			// CV2-S3d: the generator's BUILD runs --check against the committed v2 fixture.
			want["rust/warrant-corpus-gen"] = true
		}
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
	} {
		got, err := repositoryInputConsumersForChange([]string{input}, packages, "all")
		if err != nil || len(got) != 0 {
			t.Errorf("lookalike %s: got %v, err %v", input, got, err)
		}
	}
	// A lookalike *path* to the ct-compare document is still a .warrant file,
	// so it selects the grammar guard (and only that): the guard checks every
	// document wherever it lives.
	got, err = repositoryInputConsumersForChange([]string{"docs/code/specs/ct-compare.warrant"}, packages, "all")
	if err != nil || !reflect.DeepEqual(got, map[string]bool{"rust/warrant-grammar": true}) {
		t.Errorf("nested ct-compare.warrant: got %v, err %v", got, err)
	}
}

// WD-101: the Python Warrant lane. The runner package is installed into the
// pilot as a sibling editable dev dependency, which is not a build-graph edge,
// so only a registered repository input makes a runner edit rebuild the pilot.
func TestPythonWarrantLaneInputsSelectTheirConsumers(t *testing.T) {
	packages := []discovery.Package{
		{Name: "python/warrant-corpus", Language: "python"},
		{Name: "python/ct-compare", Language: "python"},
		{Name: "python/warrant-oracle", Language: "python"},
	}
	corpusFiles := []string{
		"code/specs/ct-compare.warrant",
		"code/specs/fixtures/ct-compare-warrant-v1/cases.tfv",
		"code/specs/fixtures/ct-compare-warrant-v1/corpus.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.json",
	}
	for _, input := range corpusFiles {
		// The language filter isolates the Python consumers from the Rust
		// ones that read the same spec and corpus.
		got, err := repositoryInputConsumersForChange([]string{input}, packages, "python")
		want := map[string]bool{"python/warrant-corpus": true, "python/ct-compare": true}
		if isWarrantOracleInput(input) {
			want["python/warrant-oracle"] = true
		}
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Fatalf("%s: got %v, err %v", input, got, err)
		}
		if _, err := repositoryInputConsumersForChange([]string{input}, packages[:1], "python"); err == nil {
			t.Fatalf("%s: a missing Python lane consumer must fail closed", input)
		}
		if got, err := repositoryInputConsumersForChange([]string{input}, packages, "ruby"); err != nil || len(got) != 0 {
			t.Fatalf("%s: language filtering: %v, %v", input, got, err)
		}
	}

	// An edit to the runner selects the pilot (the runner selects itself
	// through the normal package mapping, not through this table).
	got, err := repositoryInputConsumersForChange([]string{"code/packages/python/warrant-corpus/src/coding_adventures_warrant_corpus/runner.py"}, packages, "python")
	want := map[string]bool{"python/ct-compare": true}
	if err != nil || !reflect.DeepEqual(got, want) {
		t.Fatalf("runner edit: got %v, err %v", got, err)
	}

	for _, input := range []string{
		"code/specs/fixtures/ct-compare-warrant-v10/corpus.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.schema.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.backup.json",
		"code/packages/python/warrant-corpus-extra/src/x.py",
		"code/packages/python/warrant-corpus",
		"docs/code/specs/ct-compare.warrant",
		`code\packages\python\warrant-corpus\src\x.py`,
	} {
		if isWarrantPythonLanePilotInput(input) {
			t.Errorf("lookalike selected the pilot: %s", input)
		}
	}
}

// WD-102: the TypeScript Warrant lane. The pilot names the runner as a file:
// devDependency, but its committed manifest embeds the runner's source digest,
// so a runner edit selects the pilot by an explicit registered prefix too.
func TestTypeScriptWarrantLaneInputsSelectTheirConsumers(t *testing.T) {
	packages := []discovery.Package{
		{Name: "typescript/warrant-corpus", Language: "typescript"},
		{Name: "typescript/ct-compare", Language: "typescript"},
	}
	corpusFiles := []string{
		"code/specs/ct-compare.warrant",
		"code/specs/fixtures/ct-compare-warrant-v1/cases.tfv",
		"code/specs/fixtures/ct-compare-warrant-v1/corpus.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.json",
	}
	for _, input := range corpusFiles {
		// The language filter isolates the TypeScript consumers from the Rust
		// and Python ones that read the same spec and corpus.
		got, err := repositoryInputConsumersForChange([]string{input}, packages, "typescript")
		want := map[string]bool{"typescript/warrant-corpus": true, "typescript/ct-compare": true}
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Fatalf("%s: got %v, err %v", input, got, err)
		}
		if _, err := repositoryInputConsumersForChange([]string{input}, packages[:1], "typescript"); err == nil {
			t.Fatalf("%s: a missing TypeScript lane consumer must fail closed", input)
		}
		if got, err := repositoryInputConsumersForChange([]string{input}, packages, "ruby"); err != nil || len(got) != 0 {
			t.Fatalf("%s: language filtering: %v, %v", input, got, err)
		}
	}

	// An edit to the runner selects the pilot (the runner selects itself
	// through the normal package mapping, not through this table).
	got, err := repositoryInputConsumersForChange([]string{"code/packages/typescript/warrant-corpus/src/runner.ts"}, packages, "typescript")
	want := map[string]bool{"typescript/ct-compare": true}
	if err != nil || !reflect.DeepEqual(got, want) {
		t.Fatalf("runner edit: got %v, err %v", got, err)
	}

	for _, input := range []string{
		"code/specs/fixtures/ct-compare-warrant-v10/corpus.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.schema.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.backup.json",
		"code/packages/typescript/warrant-corpus-extra/src/x.ts",
		"code/packages/typescript/warrant-corpus",
		"code/packages/python/warrant-corpus/src/coding_adventures_warrant_corpus/runner.py",
		"docs/code/specs/ct-compare.warrant",
		`code\packages\typescript\warrant-corpus\src\x.ts`,
	} {
		if isWarrantTypeScriptLanePilotInput(input) {
			t.Errorf("lookalike selected the pilot: %s", input)
		}
	}
}

// WD-103: the Go Warrant lane. The pilot's go.mod requires the runner (a graph
// edge), but its committed manifest embeds the runner's source digest, so a
// runner edit selects the pilot by an explicit registered prefix too.
func TestGoWarrantLaneInputsSelectTheirConsumers(t *testing.T) {
	packages := []discovery.Package{
		{Name: "go/warrant-corpus", Language: "go"},
		{Name: "go/ct-compare", Language: "go"},
	}
	corpusFiles := []string{
		"code/specs/ct-compare.warrant",
		"code/specs/fixtures/ct-compare-warrant-v1/cases.tfv",
		"code/specs/fixtures/ct-compare-warrant-v1/corpus.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.json",
	}
	for _, input := range corpusFiles {
		// The language filter isolates the Go consumers from the Rust, Python
		// and TypeScript ones that read the same spec and corpus.
		got, err := repositoryInputConsumersForChange([]string{input}, packages, "go")
		want := map[string]bool{"go/warrant-corpus": true, "go/ct-compare": true}
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Fatalf("%s: got %v, err %v", input, got, err)
		}
		if _, err := repositoryInputConsumersForChange([]string{input}, packages[:1], "go"); err == nil {
			t.Fatalf("%s: a missing Go lane consumer must fail closed", input)
		}
		if got, err := repositoryInputConsumersForChange([]string{input}, packages, "ruby"); err != nil || len(got) != 0 {
			t.Fatalf("%s: language filtering: %v, %v", input, got, err)
		}
	}

	// An edit to the runner selects the pilot (the runner selects itself through
	// the normal package mapping, not through this table).
	got, err := repositoryInputConsumersForChange([]string{"code/packages/go/warrant-corpus/runner.go"}, packages, "go")
	want := map[string]bool{"go/ct-compare": true}
	if err != nil || !reflect.DeepEqual(got, want) {
		t.Fatalf("runner edit: got %v, err %v", got, err)
	}

	for _, input := range []string{
		"code/specs/fixtures/ct-compare-warrant-v10/corpus.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.schema.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.backup.json",
		"code/packages/go/warrant-corpus-extra/x.go",
		"code/packages/go/warrant-corpus",
		"code/packages/typescript/warrant-corpus/src/runner.ts",
		"docs/code/specs/ct-compare.warrant",
		`code\packages\go\warrant-corpus\x.go`,
	} {
		if isWarrantGoLanePilotInput(input) {
			t.Errorf("lookalike selected the pilot: %s", input)
		}
	}
}

// WD-100: the shared runner conformance fixture is read by every lane runner's
// own tests, so an edit selects each runner package and nothing else; a
// lookalike directory selects nothing, and a missing runner fails closed.
func TestWarrantConformanceFixtureSelectsEveryRunnerPackage(t *testing.T) {
	packages := []discovery.Package{
		{Name: "python/warrant-corpus", Language: "python"},
		{Name: "python/ct-compare", Language: "python"},
		{Name: "typescript/warrant-corpus", Language: "typescript"},
		{Name: "typescript/ct-compare", Language: "typescript"},
	}
	for _, input := range []string{
		"code/specs/fixtures/warrant-corpus-conformance-v1/json.json",
		"code/specs/fixtures/warrant-corpus-conformance-v1/index.json",
		"code/specs/fixtures/warrant-corpus-conformance-v1/.gitattributes",
	} {
		got, err := repositoryInputConsumersForChange([]string{input}, packages, "all")
		want := map[string]bool{"python/warrant-corpus": true, "typescript/warrant-corpus": true}
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Fatalf("%s: got %v, err %v", input, got, err)
		}
		if _, err := repositoryInputConsumersForChange([]string{input}, packages[1:2], "python"); err == nil {
			t.Fatalf("%s: a missing runner package must fail closed", input)
		}
	}
	for _, input := range []string{
		"code/specs/fixtures/warrant-corpus-conformance-v10/json.json",
		"code/specs/fixtures/warrant-corpus-conformance-v1",
		"docs/code/specs/fixtures/warrant-corpus-conformance-v1/json.json",
	} {
		got, err := repositoryInputConsumersForChange([]string{input}, packages, "all")
		if err != nil || len(got) != 0 {
			t.Errorf("lookalike %s: got %v, err %v", input, got, err)
		}
	}
}

// VER04r S4: the independent oracle re-derives a corpus from the adopter specs,
// so each spec and the v1 corpus it gates select it, and nothing else does (a
// lookalike path must not).
func TestWarrantOracleInputsSelectOnlyTheOracle(t *testing.T) {
	packages := []discovery.Package{
		{Name: "rust/warrant-grammar", Language: "rust"},
		{Name: "rust/ca-assurance-report", Language: "rust"},
		{Name: "python/warrant-corpus", Language: "python"},
		{Name: "python/ct-compare", Language: "python"},
		{Name: "typescript/warrant-corpus", Language: "typescript"},
		{Name: "typescript/ct-compare", Language: "typescript"},
		{Name: "go/warrant-corpus", Language: "go"},
		{Name: "go/ct-compare", Language: "go"},
		{Name: "python/warrant-oracle", Language: "python"},
	}
	for _, input := range warrantOraclePaths {
		got, err := repositoryInputConsumersForChange([]string{input}, packages, "python")
		if err != nil || !got["python/warrant-oracle"] {
			t.Fatalf("%s did not select the oracle: %v, err %v", input, got, err)
		}
	}
	// A spec the oracle reads selects the oracle and, because every .warrant document
	// does (isWarrantDocument), the grammar guard; nothing else in this table.
	got, err := repositoryInputConsumersForChange([]string{"code/specs/gf256.warrant"}, packages, "all")
	if err != nil || !reflect.DeepEqual(got, map[string]bool{"python/warrant-oracle": true, "rust/warrant-grammar": true}) {
		t.Fatalf("gf256 spec: got %v, err %v", got, err)
	}
	for _, input := range []string{
		"code/specs/ct-compare.warrant.bak", "code/specs/fixtures/ct-compare-warrant-v10/corpus.json",
		"code/specs/gf256.warrant/x", "code/specs/zzz.warrant", "code/specs/x25519.warrant~",
	} {
		got, err := repositoryInputConsumersForChange([]string{input}, packages, "all")
		if err != nil || got["python/warrant-oracle"] {
			t.Fatalf("%s selected the oracle: %v, err %v", input, got, err)
		}
	}
}

// A .warrant document anywhere in the repository selects rust/warrant-grammar,
// whose v0_compat test parses and checks every one of them. Without this, a
// PR that adds an invalid document never runs the only test that would catch
// it (#19417-#19474), and a later unrelated PR fails on the stale file.
func TestAnyWarrantDocumentSelectsOnlyTheGrammarGuard(t *testing.T) {
	packages := []discovery.Package{
		{Name: "rust/warrant-grammar", Language: "rust"},
		{Name: "rust/ca-assurance-report", Language: "rust"},
		{Name: "rust/ct-compare", Language: "rust"},
		{Name: "python/x", Language: "python"},
	}
	want := map[string]bool{"rust/warrant-grammar": true}
	for _, input := range []string{
		"code/specs/foo.warrant",
		"code/packages/python/x/x.warrant",
		"code/packages/rust/y/y.warrant",
		"code/packages/typescript/z/src/deep/z.warrant",
		"docs/outside-code.warrant",
		"a.warrant",
	} {
		if !isWarrantDocument(input) {
			t.Errorf("%s: not recognised as a Warrant document", input)
		}
		got, err := repositoryInputConsumersForChange([]string{input}, packages, "all")
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Errorf("%s: got %v, err %v; want only rust/warrant-grammar", input, got, err)
		}
		if got, err := repositoryInputConsumersForChange([]string{input}, packages, "python"); err != nil || len(got) != 0 {
			t.Errorf("%s: language filtering: %v, %v", input, got, err)
		}
	}
	for _, input := range []string{
		"code/specs/foo.warrant.draft",
		"code/packages/python/x/x.warrant.draft",
		"code/specs/foo.warrant.bak",
		"code/specs/foo.warrants",
		"code/specs/.warrant",
		"code/specs/foo.Warrant",
		"code/specs/warrant/readme.md",
		"code/specs/foo.warrant/readme.md",
		`code\specs\foo.warrant`,
	} {
		if isWarrantDocument(input) {
			t.Errorf("%s: wrongly recognised as a Warrant document", input)
		}
		got, err := repositoryInputConsumersForChange([]string{input}, packages, "all")
		if err != nil || len(got) != 0 {
			t.Errorf("%s: must select nothing, got %v, err %v", input, got, err)
		}
	}
	// Fail closed when the guard is missing from discovery.
	if _, err := repositoryInputConsumersForChange([]string{"code/specs/foo.warrant"}, packages[1:], "all"); err == nil {
		t.Fatal("missing Warrant guard must fail closed")
	}
}

// The declared glob must agree with the predicate, so the Kiln export keys the
// guard on the same files change detection selects it for.
func TestWarrantDocumentGlobAgreesWithPredicate(t *testing.T) {
	for _, c := range []string{
		"code/specs/foo.warrant", "code/packages/python/x/x.warrant", "a.warrant", "docs/a.warrant",
		"code/specs/foo.warrant.draft", "code/specs/.warrant", "code/specs/foo.Warrant",
		"code/specs/foo.warrant/readme.md", "code/specs/foo.warrants",
	} {
		hit := globmatch.MatchPath(warrantDocumentGlob, c)
		if want := isWarrantDocument(c); hit != want {
			t.Errorf("%q: glob=%v predicate=%v", c, hit, want)
		}
	}
}
