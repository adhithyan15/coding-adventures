package main

import (
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// CV2-S3d: the committed ct-compare warrant-corpus-v2 fixture is checked by two
// packages whose own directories do not contain it. rust/warrant-corpus-gen's
// BUILD runs the generator's --check against it, and python/warrant-oracle's
// BUILD runs verify-v2 on it. Without a row, a PR that edits only the spec, a
// fixture file or the Rust bindings maps (by path prefix) to ct-compare, the
// spec directory or nothing, and the stale fixture merges green.
func TestWarrantCorpusV2FixtureSelectsGeneratorAndOracle(t *testing.T) {
	packages := []discovery.Package{
		{Name: "rust/warrant-corpus-gen", Language: "rust"},
		{Name: "python/warrant-oracle", Language: "python"},
		{Name: "rust/warrant-grammar", Language: "rust"},
		{Name: "rust/ca-assurance-report", Language: "rust"},
		{Name: "rust/ct-compare", Language: "rust"},
		{Name: "python/warrant-corpus", Language: "python"},
		{Name: "python/ct-compare", Language: "python"},
		{Name: "typescript/warrant-corpus", Language: "typescript"},
		{Name: "typescript/ct-compare", Language: "typescript"},
		{Name: "go/warrant-corpus", Language: "go"},
		{Name: "go/ct-compare", Language: "go"},
		{Name: "rust/arithmetic", Language: "rust"},
		{Name: "rust/gf256", Language: "rust"},
		{Name: "rust/logic-gates", Language: "rust"},
		{Name: "rust/lzw", Language: "rust"},
		{Name: "rust/wasm-leb128", Language: "rust"},
		{Name: "rust/test-bootstrap", Language: "rust"},
		{Name: "rust/x25519", Language: "rust"},
	}
	both := map[string]bool{"rust/warrant-corpus-gen": true, "python/warrant-oracle": true}
	// Every file of the fixture directory, and the bindings, select exactly the two.
	for _, changed := range []string{
		"code/specs/fixtures/ct-compare-warrant-v2/corpus.json",
		"code/specs/fixtures/ct-compare-warrant-v2/cases.tfv",
		"code/specs/fixtures/ct-compare-warrant-v2/corpus.profile",
		"code/specs/fixtures/ct-compare-warrant-v2/README.md",
		"code/specs/fixtures/ct-compare-warrant-v2/chunks/eq_agrees.txt",
		"code/specs/fixtures/ct-compare-warrant-v2/chunks/select_rejects_mismatch.txt",
		"code/packages/rust/ct-compare/evidence.bindings",
	} {
		got, err := repositoryInputConsumersForChange([]string{changed}, packages, "all")
		if err != nil || !reflect.DeepEqual(got, both) {
			t.Fatalf("%s selected %v (err %v), want exactly the generator and the oracle", changed, got, err)
		}
		// A single-language run keeps only that language's consumer.
		if got, _ := repositoryInputConsumersForChange([]string{changed}, packages, "rust"); !reflect.DeepEqual(got, map[string]bool{"rust/warrant-corpus-gen": true}) {
			t.Fatalf("%s: rust-only run selected %v", changed, got)
		}
		if got, _ := repositoryInputConsumersForChange([]string{changed}, packages, "python"); !reflect.DeepEqual(got, map[string]bool{"python/warrant-oracle": true}) {
			t.Fatalf("%s: python-only run selected %v", changed, got)
		}
	}
	// CV2 S10 adopter (x25519), VER04r 16.10 [D32]: the table-driven rows give the second
	// adopter the same selection as ct-compare. Every file of its fixture directory and its
	// bindings select exactly the generator and the oracle, per language filter too.
	for _, changed := range []string{
		"code/specs/fixtures/x25519-warrant-v2/corpus.json",
		"code/specs/fixtures/x25519-warrant-v2/cases.tfv",
		"code/specs/fixtures/x25519-warrant-v2/corpus.profile",
		"code/specs/fixtures/x25519-warrant-v2/README.md",
		"code/specs/fixtures/x25519-warrant-v2/chunks/clamp_is_idempotent.txt",
		"code/packages/rust/x25519/evidence.bindings",
	} {
		got, err := repositoryInputConsumersForChange([]string{changed}, packages, "all")
		if err != nil || !reflect.DeepEqual(got, both) {
			t.Fatalf("%s selected %v (err %v), want exactly the generator and the oracle", changed, got, err)
		}
		if got, _ := repositoryInputConsumersForChange([]string{changed}, packages, "rust"); !reflect.DeepEqual(got, map[string]bool{"rust/warrant-corpus-gen": true}) {
			t.Fatalf("%s: rust-only run selected %v", changed, got)
		}
		if got, _ := repositoryInputConsumersForChange([]string{changed}, packages, "python"); !reflect.DeepEqual(got, map[string]bool{"python/warrant-oracle": true}) {
			t.Fatalf("%s: python-only run selected %v", changed, got)
		}
	}
	// The x25519 spec selects both too (it was already an oracle input; the table adds the generator).
	got, err := repositoryInputConsumersForChange([]string{"code/specs/x25519.warrant"}, packages, "all")
	if err != nil || !got["rust/warrant-corpus-gen"] || !got["python/warrant-oracle"] {
		t.Fatalf("x25519.warrant selected %v (err %v)", got, err)
	}
	// Every adopter row selects both packages through each of its three path kinds, and the
	// rows are distinct (a copy-pasted row would silently drop an adopter).
	seen := map[string]bool{}
	for _, adopter := range warrantAdopters {
		if seen[adopter.module] {
			t.Errorf("duplicate adopter row %s", adopter.module)
		}
		seen[adopter.module] = true
		for _, changed := range []string{adopter.spec, adopter.fixture + "/corpus.json", adopter.bindings} {
			if !isWarrantCorpusGenInput(changed) || !isWarrantOracleInput(changed) {
				t.Errorf("%s (adopter %s) is not selected by both predicates", changed, adopter.module)
			}
		}
	}
	if len(warrantAdopters) != 2 || !seen["ct-compare"] || !seen["x25519"] {
		t.Errorf("warrantAdopters = %v, want exactly ct-compare and x25519", warrantAdopters)
	}
	// The spec selects both too (among the other Warrant consumers of the document).
	got, err = repositoryInputConsumersForChange([]string{"code/specs/ct-compare.warrant"}, packages, "all")
	if err != nil || !got["rust/warrant-corpus-gen"] || !got["python/warrant-oracle"] {
		t.Fatalf("ct-compare.warrant selected %v (err %v)", got, err)
	}
	// The oracle's synthetic accept-blind corpus is regenerated by the generator's tests
	// and lies in the oracle's directory: the generator needs the row, the oracle has the
	// path prefix.
	got, err = repositoryInputConsumersForChange([]string{"code/packages/python/warrant-oracle/tests/fixtures/accept-blind-v2/corpus/corpus.json"}, packages, "all")
	if err != nil || !reflect.DeepEqual(got, map[string]bool{"rust/warrant-corpus-gen": true}) {
		t.Fatalf("accept-blind fixture selected %v (err %v)", got, err)
	}
	// CV2-S10b: the oracle's ir.fixed document and its lowering dump are reproduced by the
	// generator's tests/fixed_parity.rs, read from the oracle's directory: the generator needs
	// the row, the oracle has the path prefix.
	for _, changed := range []string{
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-forms-v1/fixed_forms_v1.warrant",
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-forms-v1/fixed_forms_v1.oracle-lowering.json",
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-forms-v1/make_fixed_forms.py",
	} {
		want := map[string]bool{"rust/warrant-corpus-gen": true}
		// Every *.warrant document anywhere in the repository is also an input of
		// rust/warrant-grammar (isWarrantDocument): its syntax corpus admits each one.
		if isWarrantDocument(changed) {
			want["rust/warrant-grammar"] = true
		}
		got, err = repositoryInputConsumersForChange([]string{changed}, packages, "all")
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Fatalf("%s selected %v, want %v (err %v)", changed, got, want, err)
		}
	}
	// CV2-S10-1: the oracle's omit_example fixture (omit.warrant + corpus.profile -> corpus/) is
	// regenerated byte for byte by the generator's tests/omit_example.rs, read from the oracle's
	// directory: the generator needs the row, the oracle has the path prefix. Its omit.warrant is
	// a Warrant document, so rust/warrant-grammar is selected as well (isWarrantDocument).
	for _, changed := range []string{
		"code/packages/python/warrant-oracle/tests/fixtures/omit-example-v2/omit.warrant",
		"code/packages/python/warrant-oracle/tests/fixtures/omit-example-v2/corpus.profile",
		"code/packages/python/warrant-oracle/tests/fixtures/omit-example-v2/make_fixture.py",
		"code/packages/python/warrant-oracle/tests/fixtures/omit-example-v2/README.md",
		"code/packages/python/warrant-oracle/tests/fixtures/omit-example-v2/corpus/corpus.json",
		"code/packages/python/warrant-oracle/tests/fixtures/omit-example-v2/corpus/cases.tfv",
		"code/packages/python/warrant-oracle/tests/fixtures/omit-example-v2/corpus/chunks/echo_is_identity.txt",
	} {
		want := map[string]bool{"rust/warrant-corpus-gen": true}
		if isWarrantDocument(changed) {
			want["rust/warrant-grammar"] = true
		}
		got, err = repositoryInputConsumersForChange([]string{changed}, packages, "all")
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Fatalf("%s selected %v, want %v (err %v)", changed, got, want, err)
		}
	}
	// CV2-S10t-2: the oracle's ir.tuple document, its profile and its four committed dumps are
	// reproduced by the generator's tests/tuple_parity.rs, read from the oracle's directory.
	// tuple_forms_v1.warrant is a Warrant document, so rust/warrant-grammar is selected as well.
	for _, changed := range []string{
		"code/packages/python/warrant-oracle/tests/fixtures/tuple-forms-v1/tuple_forms_v1.warrant",
		"code/packages/python/warrant-oracle/tests/fixtures/tuple-forms-v1/tuple_forms_v1.profile",
		"code/packages/python/warrant-oracle/tests/fixtures/tuple-forms-v1/tuple_forms_v1.oracle-lowering.json",
		"code/packages/python/warrant-oracle/tests/fixtures/tuple-forms-v1/tuple_forms_v1.oracle-evaluation.json",
		"code/packages/python/warrant-oracle/tests/fixtures/tuple-forms-v1/tuple_forms_v1.oracle-faults.json",
		"code/packages/python/warrant-oracle/tests/fixtures/tuple-forms-v1/tuple_forms_v1.oracle-refusals.json",
		"code/packages/python/warrant-oracle/tests/fixtures/tuple-forms-v1/make_tuple_forms.py",
	} {
		want := map[string]bool{"rust/warrant-corpus-gen": true}
		if isWarrantDocument(changed) {
			want["rust/warrant-grammar"] = true
		}
		got, err = repositoryInputConsumersForChange([]string{changed}, packages, "all")
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Fatalf("%s selected %v, want %v (err %v)", changed, got, want, err)
		}
	}
	// CV2-S10c-2: the oracle's ir.fixed evaluation document, its profile and its five committed dumps
	// (lowering, evaluation, faults, values, domains) are reproduced by the generator's
	// tests/fixed_eval_parity.rs, read from the oracle's directory. fixed_eval_v1.warrant is a Warrant
	// document, so rust/warrant-grammar is selected as well.
	for _, changed := range []string{
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-eval-v1/fixed_eval_v1.warrant",
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-eval-v1/fixed_eval_v1.profile",
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-eval-v1/fixed_eval_v1.oracle-lowering.json",
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-eval-v1/fixed_eval_v1.oracle-evaluation.json",
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-eval-v1/fixed_eval_v1.oracle-faults.json",
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-eval-v1/fixed_eval_v1.oracle-values.json",
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-eval-v1/fixed_eval_v1.oracle-domains.json",
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-eval-v1/make_fixed_eval.py",
	} {
		want := map[string]bool{"rust/warrant-corpus-gen": true}
		if isWarrantDocument(changed) {
			want["rust/warrant-grammar"] = true
		}
		got, err = repositoryInputConsumersForChange([]string{changed}, packages, "all")
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Fatalf("%s selected %v, want %v (err %v)", changed, got, want, err)
		}
	}
	// Lookalikes select neither: another version directory, the directory itself (a
	// changed *file* is what is mapped), a backslash spelling, another crate's bindings.
	for _, changed := range []string{
		"code/specs/fixtures/ct-compare-warrant-v20/corpus.json",
		"code/specs/fixtures/ct-compare-warrant-v2",
		"code/specs/fixtures/ct-compare-warrant-v2x/chunks/eq_agrees.txt",
		`code\specs\fixtures\ct-compare-warrant-v2\corpus.json`,
		"docs/code/specs/fixtures/ct-compare-warrant-v2/corpus.json",
		"code/packages/rust/ct-compare/evidence.bindings.bak",
		"code/packages/rust/gf256/evidence.bindings",
		"code/specs/fixtures/x25519-warrant-v20/corpus.json",
		"code/specs/fixtures/x25519-warrant-v2",
		"code/specs/fixtures/x25519-warrant-v2x/chunks/clamp_is_idempotent.txt",
		`code\specs\fixtures\x25519-warrant-v2\corpus.json`,
		"code/packages/rust/x25519/evidence.bindings.bak",
		"code/packages/rust/x25519/tests/warrant_evidence.rs",
		"code/packages/python/warrant-oracle/tests/fixtures/accept-blind-v20/corpus/corpus.json",
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-forms-v10/fixed_forms_v1.warrant",
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-forms-v1",
		"code/packages/python/warrant-oracle/tests/fixtures/omit-example-v20/corpus/corpus.json",
		"code/packages/python/warrant-oracle/tests/fixtures/omit-example-v2",
		"code/packages/python/warrant-oracle/tests/fixtures/tuple-forms-v10/tuple_forms_v1.warrant",
		"code/packages/python/warrant-oracle/tests/fixtures/tuple-forms-v1",
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-eval-v10/fixed_eval_v1.warrant",
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-eval-v1",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.backup.json",
	} {
		got, err := repositoryInputConsumersForChange([]string{changed}, packages, "all")
		if err != nil || got["rust/warrant-corpus-gen"] || got["python/warrant-oracle"] {
			t.Errorf("lookalike %s selected %v (err %v)", changed, got, err)
		}
		if isWarrantCorpusGenInput(changed) || isWarrantOracleInput(changed) {
			t.Errorf("lookalike %s is accepted by a predicate", changed)
		}
	}
	// A consumer that is missing from discovery is a hard error, never a silent skip: a
	// check that quietly stops being scheduled is indistinguishable from one that passes.
	for _, drop := range []string{"rust/warrant-corpus-gen", "python/warrant-oracle"} {
		var kept []discovery.Package
		for _, p := range packages {
			if p.Name != drop {
				kept = append(kept, p)
			}
		}
		if _, err := repositoryInputConsumersForChange([]string{"code/specs/fixtures/ct-compare-warrant-v2/corpus.json"}, kept, "all"); err == nil {
			t.Errorf("a fixture edit without %s in discovery must fail closed", drop)
		}
	}
}
