package main

import (
	"reflect"
	"strings"
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
		{Name: "go/programs/build-tool", Language: "go"},
	}
	both := map[string]bool{"rust/warrant-corpus-gen": true, "python/warrant-oracle": true}
	// The bindings also select the build tool, whose coverage test reads them (CV2-F13).
	bothFor := func(changed string) map[string]bool {
		want := map[string]bool{}
		for name := range both {
			want[name] = true
		}
		if strings.HasSuffix(changed, "/evidence.bindings") {
			want["go/programs/build-tool"] = true
		}
		return want
	}
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
		if err != nil || !reflect.DeepEqual(got, bothFor(changed)) {
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
		if err != nil || !reflect.DeepEqual(got, bothFor(changed)) {
			t.Fatalf("%s selected %v (err %v), want exactly the generator and the oracle", changed, got, err)
		}
		if got, _ := repositoryInputConsumersForChange([]string{changed}, packages, "rust"); !reflect.DeepEqual(got, map[string]bool{"rust/warrant-corpus-gen": true}) {
			t.Fatalf("%s: rust-only run selected %v", changed, got)
		}
		if got, _ := repositoryInputConsumersForChange([]string{changed}, packages, "python"); !reflect.DeepEqual(got, map[string]bool{"python/warrant-oracle": true}) {
			t.Fatalf("%s: python-only run selected %v", changed, got)
		}
	}
	// CV2 S10 adopter (lzw), VER04r 16.10 [D32]: the third row. Every kind of path of its fixture
	// directory (the largest one: 21 chunk files) and its bindings select exactly the generator and
	// the oracle, per language filter too.
	for _, changed := range []string{
		"code/specs/fixtures/lzw-warrant-v2/corpus.json",
		"code/specs/fixtures/lzw-warrant-v2/cases.tfv",
		"code/specs/fixtures/lzw-warrant-v2/corpus.profile",
		"code/specs/fixtures/lzw-warrant-v2/README.md",
		"code/specs/fixtures/lzw-warrant-v2/chunks/round_trip.txt",
		"code/specs/fixtures/lzw-warrant-v2/chunks/decompress_three_bytes_rejected.txt",
		"code/packages/rust/lzw/evidence.bindings",
	} {
		got, err := repositoryInputConsumersForChange([]string{changed}, packages, "all")
		if err != nil || !reflect.DeepEqual(got, bothFor(changed)) {
			t.Fatalf("%s selected %v (err %v), want exactly the generator and the oracle", changed, got, err)
		}
		if got, _ := repositoryInputConsumersForChange([]string{changed}, packages, "rust"); !reflect.DeepEqual(got, map[string]bool{"rust/warrant-corpus-gen": true}) {
			t.Fatalf("%s: rust-only run selected %v", changed, got)
		}
		if got, _ := repositoryInputConsumersForChange([]string{changed}, packages, "python"); !reflect.DeepEqual(got, map[string]bool{"python/warrant-oracle": true}) {
			t.Fatalf("%s: python-only run selected %v", changed, got)
		}
	}
	// CV2 S10 adopter (wasm-leb128), VER04r 16.10 [D32]: the fourth row. Every kind of path of its
	// fixture directory (44 chunk files) and its bindings select exactly the generator and the
	// oracle, per language filter too.
	for _, changed := range []string{
		"code/specs/fixtures/wasm-leb128-warrant-v2/corpus.json",
		"code/specs/fixtures/wasm-leb128-warrant-v2/cases.tfv",
		"code/specs/fixtures/wasm-leb128-warrant-v2/corpus.profile",
		"code/specs/fixtures/wasm-leb128-warrant-v2/README.md",
		"code/specs/fixtures/wasm-leb128-warrant-v2/chunks/encode_unsigned_matches_canonical_model.txt",
		"code/specs/fixtures/wasm-leb128-warrant-v2/chunks/decode_unsigned_three_bytes.txt",
		"code/packages/rust/wasm-leb128/evidence.bindings",
	} {
		got, err := repositoryInputConsumersForChange([]string{changed}, packages, "all")
		if err != nil || !reflect.DeepEqual(got, bothFor(changed)) {
			t.Fatalf("%s selected %v (err %v), want exactly the generator and the oracle", changed, got, err)
		}
		if got, _ := repositoryInputConsumersForChange([]string{changed}, packages, "rust"); !reflect.DeepEqual(got, map[string]bool{"rust/warrant-corpus-gen": true}) {
			t.Fatalf("%s: rust-only run selected %v", changed, got)
		}
		if got, _ := repositoryInputConsumersForChange([]string{changed}, packages, "python"); !reflect.DeepEqual(got, map[string]bool{"python/warrant-oracle": true}) {
			t.Fatalf("%s: python-only run selected %v", changed, got)
		}
	}
	// The wasm-leb128 spec selects the generator and the oracle too (the table adds the generator;
	// rust/warrant-grammar and rust/wasm-leb128's own rows are separate consumers).
	got, err := repositoryInputConsumersForChange([]string{"code/specs/wasm-leb128.warrant"}, packages, "all")
	if err != nil || !got["rust/warrant-corpus-gen"] || !got["python/warrant-oracle"] {
		t.Fatalf("wasm-leb128.warrant selected %v (err %v)", got, err)
	}
	// The lzw spec selects both too (it was already an oracle input; the table adds the generator).
	// rust/lzw's own row (the lane registry) is a separate consumer and is not asserted here.
	got, err = repositoryInputConsumersForChange([]string{"code/specs/lzw.warrant"}, packages, "all")
	if err != nil || !got["rust/warrant-corpus-gen"] || !got["python/warrant-oracle"] {
		t.Fatalf("lzw.warrant selected %v (err %v)", got, err)
	}
	// The x25519 spec selects both too (it was already an oracle input; the table adds the generator).
	got, err = repositoryInputConsumersForChange([]string{"code/specs/x25519.warrant"}, packages, "all")
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
		changes := []string{adopter.spec, adopter.fixture + "/corpus.json"}
		if adopter.bindings != "" {
			changes = append(changes, adopter.bindings)
		}
		for _, changed := range changes {
			if !isWarrantCorpusGenInput(changed) || !isWarrantOracleInput(changed) {
				t.Errorf("%s (adopter %s) is not selected by both predicates", changed, adopter.module)
			}
		}
	}
	if len(warrantAdopters) != 5 || !seen["ct-compare"] || !seen["x25519"] || !seen["lzw"] || !seen["wasm-leb128"] || !seen["logic-gates-basic"] {
		t.Errorf("warrantAdopters = %v, want exactly ct-compare, x25519, lzw, wasm-leb128 and logic-gates-basic", warrantAdopters)
	}
	// The one adopter without bindings (LGE-S1c) must not put an empty path in any input list, and
	// its spec and fixture files select the generator and the oracle.
	for _, list := range [][]string{warrantAdopterBindings(), warrantAdopterInputs(), warrantAdopterFixtures()} {
		for _, path := range list {
			if path == "" {
				t.Errorf("an adopter without bindings leaked an empty path into %v", list)
			}
		}
	}
	for _, changed := range []string{
		"code/specs/logic-gates-basic.warrant",
		"code/specs/fixtures/logic-gates-basic-warrant-v2/corpus.json",
		"code/specs/fixtures/logic-gates-basic-warrant-v2/chunks/not_truth_table.txt",
	} {
		if !isWarrantCorpusGenInput(changed) || !isWarrantOracleInput(changed) {
			t.Errorf("%s is not selected by both predicates", changed)
		}
	}
	for _, changed := range []string{"", "code/specs/logic-gates-basic.warrant.draft", "code/specs/fixtures/logic-gates-basic-warrant-v20/corpus.json"} {
		if isWarrantCorpusGenInput(changed) || isWarrantOracleInput(changed) {
			t.Errorf("%q must select neither the generator nor the oracle", changed)
		}
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
	// CV2-S10o-1: the oracle's off-by-one-consumed document, its profile and its committed faults and
	// refusals dumps are reproduced by the generator's slice S10o-2, read from the oracle's directory.
	// off_by_one_consumed_v1.warrant is a Warrant document, so rust/warrant-grammar is selected as well.
	for _, changed := range []string{
		"code/packages/python/warrant-oracle/tests/fixtures/off-by-one-consumed-v1/off_by_one_consumed_v1.warrant",
		"code/packages/python/warrant-oracle/tests/fixtures/off-by-one-consumed-v1/off_by_one_consumed_v1.profile",
		"code/packages/python/warrant-oracle/tests/fixtures/off-by-one-consumed-v1/off_by_one_consumed_v1.oracle-faults.json",
		"code/packages/python/warrant-oracle/tests/fixtures/off-by-one-consumed-v1/off_by_one_consumed_v1.oracle-refusals.json",
		"code/packages/python/warrant-oracle/tests/fixtures/off-by-one-consumed-v1/make_off_by_one_consumed.py",
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
	// CV2-S11f-2: the oracle's scalar-faults document, its profile and its committed faults and
	// refusals dumps are reproduced by the generator's slice S11f-2, read from the oracle's directory.
	// scalar_faults_v1.warrant is a Warrant document, so rust/warrant-grammar is selected as well.
	for _, changed := range []string{
		"code/packages/python/warrant-oracle/tests/fixtures/scalar-faults-v1/scalar_faults_v1.warrant",
		"code/packages/python/warrant-oracle/tests/fixtures/scalar-faults-v1/scalar_faults_v1.profile",
		"code/packages/python/warrant-oracle/tests/fixtures/scalar-faults-v1/scalar_faults_v1.oracle-faults.json",
		"code/packages/python/warrant-oracle/tests/fixtures/scalar-faults-v1/scalar_faults_v1.oracle-refusals.json",
		"code/packages/python/warrant-oracle/tests/fixtures/scalar-faults-v1/make_scalar_faults.py",
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
	// CV2-S11b: the oracle's data-forms document, its lowering dump and its refusals dump are
	// reproduced by the generator's slice S11b, read from the oracle's directory. data_forms_v1.warrant
	// is a Warrant document, so rust/warrant-grammar is selected as well.
	for _, changed := range []string{
		"code/packages/python/warrant-oracle/tests/fixtures/data-forms-v1/data_forms_v1.warrant",
		"code/packages/python/warrant-oracle/tests/fixtures/data-forms-v1/data_forms_v1.oracle-lowering.json",
		"code/packages/python/warrant-oracle/tests/fixtures/data-forms-v1/data_forms_v1.oracle-refusals.json",
		"code/packages/python/warrant-oracle/tests/fixtures/data-forms-v1/make_data_forms.py",
		"code/packages/python/warrant-oracle/tests/fixtures/data-forms-v1/README.md",
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
	// CV2-S11c-1: the oracle's data-eval document, profile and six dumps are the generator's byte-parity target
	// for slice S11c-2. data_eval_v1.warrant is a Warrant document, so rust/warrant-grammar is selected too.
	for _, changed := range []string{
		"code/packages/python/warrant-oracle/tests/fixtures/data-eval-v1/data_eval_v1.warrant",
		"code/packages/python/warrant-oracle/tests/fixtures/data-eval-v1/data_eval_v1.profile",
		"code/packages/python/warrant-oracle/tests/fixtures/data-eval-v1/data_eval_v1.oracle-lowering.json",
		"code/packages/python/warrant-oracle/tests/fixtures/data-eval-v1/data_eval_v1.oracle-evaluation.json",
		"code/packages/python/warrant-oracle/tests/fixtures/data-eval-v1/data_eval_v1.oracle-faults.json",
		"code/packages/python/warrant-oracle/tests/fixtures/data-eval-v1/data_eval_v1.oracle-values.json",
		"code/packages/python/warrant-oracle/tests/fixtures/data-eval-v1/data_eval_v1.oracle-domains.json",
		"code/packages/python/warrant-oracle/tests/fixtures/data-eval-v1/data_eval_v1.oracle-refusals.json",
		"code/packages/python/warrant-oracle/tests/fixtures/data-eval-v1/make_data_eval.py",
		"code/packages/python/warrant-oracle/tests/fixtures/data-eval-v1/README.md",
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
		"code/specs/fixtures/lzw-warrant-v20/corpus.json",
		"code/specs/fixtures/lzw-warrant-v2",
		"code/specs/fixtures/lzw-warrant-v2x/chunks/round_trip.txt",
		`code\specs\fixtures\lzw-warrant-v2\corpus.json`,
		"code/packages/rust/lzw/evidence.bindings.bak",
		"code/packages/rust/lzw/tests/generated/warrant_evidence.rs",
		"code/specs/fixtures/wasm-leb128-warrant-v20/corpus.json",
		"code/specs/fixtures/wasm-leb128-warrant-v2",
		"code/specs/fixtures/wasm-leb128-warrant-v2x/chunks/decode_unsigned_three_bytes.txt",
		`code\specs\fixtures\wasm-leb128-warrant-v2\corpus.json`,
		"code/packages/rust/wasm-leb128/evidence.bindings.bak",
		"code/packages/rust/wasm-leb128/tests/generated/warrant_evidence.rs",
		"code/packages/python/warrant-oracle/tests/fixtures/accept-blind-v20/corpus/corpus.json",
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-forms-v10/fixed_forms_v1.warrant",
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-forms-v1",
		"code/packages/python/warrant-oracle/tests/fixtures/omit-example-v20/corpus/corpus.json",
		"code/packages/python/warrant-oracle/tests/fixtures/omit-example-v2",
		"code/packages/python/warrant-oracle/tests/fixtures/tuple-forms-v10/tuple_forms_v1.warrant",
		"code/packages/python/warrant-oracle/tests/fixtures/tuple-forms-v1",
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-eval-v10/fixed_eval_v1.warrant",
		"code/packages/python/warrant-oracle/tests/fixtures/fixed-eval-v1",
		"code/packages/python/warrant-oracle/tests/fixtures/off-by-one-consumed-v10/off_by_one_consumed_v1.warrant",
		"code/packages/python/warrant-oracle/tests/fixtures/off-by-one-consumed-v1",
		"code/packages/python/warrant-oracle/tests/fixtures/scalar-faults-v10/scalar_faults_v1.warrant",
		"code/packages/python/warrant-oracle/tests/fixtures/scalar-faults-v1",
		"code/packages/python/warrant-oracle/tests/fixtures/data-forms-v10/data_forms_v1.warrant",
		"code/packages/python/warrant-oracle/tests/fixtures/data-forms-v1",
		"code/packages/python/warrant-oracle/tests/fixtures/data-eval-v10/data_eval_v1.warrant",
		"code/packages/python/warrant-oracle/tests/fixtures/data-eval-v1",
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
