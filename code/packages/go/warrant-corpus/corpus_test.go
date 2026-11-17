package warrantcorpus

import (
	"os"
	"path/filepath"
	"sort"
	"strings"
	"testing"
)

func realCorpusLoaded(t *testing.T) *Corpus {
	t.Helper()
	var corpus *Corpus
	if code := laneCode(t, func() { corpus = loadCorpus(realRepoRoot, realCorpus) }); code != "" {
		t.Fatalf("the real corpus was refused: %s", code)
	}
	return corpus
}

func TestLoadsTheRealCorpus(t *testing.T) {
	corpus := realCorpusLoaded(t)
	if corpus.Module != "ct_compare" || len(corpus.Examples) != 12 || len(corpus.Suites) != 9 || len(corpus.Controls) != 8 {
		t.Fatalf("%s: %d examples, %d suites, %d controls", corpus.Module, len(corpus.Examples), len(corpus.Suites), len(corpus.Controls))
	}
	total := 0
	for _, suite := range corpus.Suites {
		total += suite.Cases
	}
	if total != 572109 {
		t.Fatalf("%d exhaustive cases, want 572109", total)
	}
	if !corpus.Ops["select_bytes"].Fresh || corpus.Ops["eq"].Fresh || len(corpus.Ops["eq_fixed"].WidthParams) != 1 {
		t.Fatal("op signatures")
	}
	rows := listControlRows(realCorpus)
	if len(rows) != 8 || rows[0] != "ignore-last-byte:eq" {
		t.Fatalf("control rows: %v", rows)
	}
}

// tamperRefused loads a fixture corpus altered by change and requires the refusal code.
func tamperRefused(t *testing.T, name, code string, change func(document map[string]any)) {
	t.Helper()
	f := newFixture(t, fixtureOptions{mini: true, corpus: change})
	if got := laneCode(t, func() { loadCorpus(f.root, f.corpusDir) }); got != code {
		t.Errorf("%s: code %q, want %q", name, got, code)
	}
}

func suiteAt(document map[string]any, i int) map[string]any {
	return document["suites"].([]any)[i].(map[string]any)
}

func TestCorpusTamperingIsRefused(t *testing.T) {
	f := newFixture(t, fixtureOptions{mini: true})
	var corpus *Corpus
	wantCode(t, "", func() { corpus = loadCorpus(f.root, f.corpusDir) })
	if len(corpus.Suites) != 2 || len(corpus.Examples) != 3 || len(corpus.Controls) != 2 {
		t.Fatalf("mini corpus: %d %d %d", len(corpus.Suites), len(corpus.Examples), len(corpus.Controls))
	}
	cases := []struct {
		name, code string
		change     func(d map[string]any)
	}{
		{"unknown top key", "schema", func(d map[string]any) { d["extra"] = int64(1) }},
		{"missing top key", "schema", func(d map[string]any) { delete(d, "models") }},
		{"wrong profile", "schema", func(d map[string]any) { d["profile"] = "other" }},
		{"schema version", "schema", func(d map[string]any) { d["schema_version"] = int64(2) }},
		{"bad module", "schema", func(d map[string]any) { d["module"] = "Bad-Module" }},
		{"empty module", "schema", func(d map[string]any) { d["module"] = "" }},
		{"spec digest", "stale-evidence", func(d map[string]any) { d["spec_sha256"] = strings.Repeat("0", 64) }},
		{"spec digest shape", "schema", func(d map[string]any) { d["spec_sha256"] = "XYZ" }},
		{"spec escapes", "schema", func(d map[string]any) { d["spec"] = "../ct-compare.warrant" }},
		{"spec absolute", "schema", func(d map[string]any) { d["spec"] = "/etc/passwd" }},
		{"spec backslash", "schema", func(d map[string]any) { d["spec"] = `code\specs\x` }},
		{"spec colon", "schema", func(d map[string]any) { d["spec"] = "code/specs/a:b" }},
		{"spec empty part", "schema", func(d map[string]any) { d["spec"] = "code//ct-compare.warrant" }},
		{"spec missing", "schema", func(d map[string]any) { d["spec"] = "code/specs/none.warrant" }},
		{"types not object", "schema", func(d map[string]any) { d["types"] = []any{} }},
		{"error type name", "schema", func(d map[string]any) { d["types"] = map[string]any{"bad": []any{}} }},
		{"error variants", "schema", func(d map[string]any) { d["types"] = map[string]any{"E": []any{int64(1)}} }},
		{"error variants type", "schema", func(d map[string]any) { d["types"] = map[string]any{"E": "x"} }},
		{"ops empty", "schema", func(d map[string]any) { d["ops"] = map[string]any{} }},
		{"op name", "schema", func(d map[string]any) { d["ops"] = map[string]any{"Bad": map[string]any{}} }},
		{"op fields", "schema", func(d map[string]any) {
			d["ops"].(map[string]any)["eq"].(map[string]any)["extra"] = true
		}},
		{"op not object", "schema", func(d map[string]any) { d["ops"].(map[string]any)["eq"] = int64(1) }},
		{"op params not list", "schema", func(d map[string]any) { d["ops"].(map[string]any)["eq"].(map[string]any)["params"] = "x" }},
		{"op param shape", "schema", func(d map[string]any) {
			d["ops"].(map[string]any)["eq"].(map[string]any)["params"] = []any{[]any{"a"}}
		}},
		{"op width params", "schema", func(d map[string]any) {
			d["ops"].(map[string]any)["eq"].(map[string]any)["width_params"] = "N"
		}},
		{"op width params items", "schema", func(d map[string]any) {
			d["ops"].(map[string]any)["eq"].(map[string]any)["width_params"] = []any{int64(1)}
		}},
		{"op ret empty", "schema", func(d map[string]any) { d["ops"].(map[string]any)["eq"].(map[string]any)["ret"] = "" }},
		{"op ret type", "unsupported-type", func(d map[string]any) { d["ops"].(map[string]any)["eq"].(map[string]any)["ret"] = "List<U8>" }},
		{"models not object", "schema", func(d map[string]any) { d["models"] = []any{} }},
		{"examples not list", "schema", func(d map[string]any) { d["examples"] = "x" }},
		{"example fields", "schema", func(d map[string]any) { delete(d["examples"].([]any)[0].(map[string]any), "width") }},
		{"example outside module", "schema", func(d map[string]any) { d["examples"].([]any)[0].(map[string]any)["id"] = "warrant/other/example/x" }},
		{"example unknown op", "example", func(d map[string]any) { d["examples"].([]any)[0].(map[string]any)["op"] = "nope" }},
		{"example arity", "example", func(d map[string]any) { d["examples"].([]any)[0].(map[string]any)["args"] = []any{} }},
		{"example result type", "example", func(d map[string]any) { d["examples"].([]any)[0].(map[string]any)["result_type"] = "U64" }},
		{"example width missing", "example", func(d map[string]any) { d["examples"].([]any)[0].(map[string]any)["width"] = nil }},
		{"example width negative", "example", func(d map[string]any) { d["examples"].([]any)[0].(map[string]any)["width"] = int64(-1) }},
		{"example buffer width", "example", func(d map[string]any) { d["examples"].([]any)[1].(map[string]any)["width"] = int64(3) }},
		{"example duplicate", "schema", func(d map[string]any) {
			e := d["examples"].([]any)
			d["examples"] = append(e, e[0])
		}},
		{"example expected", "value", func(d map[string]any) { d["examples"].([]any)[0].(map[string]any)["expected"] = "yes" }},
		{"suites empty", "schema", func(d map[string]any) { d["suites"] = []any{} }},
		{"suite fields", "schema", func(d map[string]any) { delete(suiteAt(d, 0), "admit") }},
		{"suite id", "schema", func(d map[string]any) { suiteAt(d, 0)["id"] = "warrant/ct_compare/exhaustive/other" }},
		{"suite kind", "schema", func(d map[string]any) { suiteAt(d, 0)["kind"] = "property" }},
		{"suite ops empty", "schema", func(d map[string]any) { suiteAt(d, 0)["ops"] = []any{} }},
		{"suite ops unknown", "schema", func(d map[string]any) { suiteAt(d, 0)["ops"] = []any{"nope"} }},
		{"suite ops type", "schema", func(d map[string]any) { suiteAt(d, 0)["ops"] = []any{int64(1)} }},
		{"suite binders empty", "schema", func(d map[string]any) { suiteAt(d, 0)["binders"] = []any{} }},
		{"suite binders too many", "schema", func(d map[string]any) {
			var many []any
			for i := 0; i < 9; i++ {
				many = append(many, []any{"v" + itoa(i), map[string]any{"domain": "bool"}})
			}
			suiteAt(d, 0)["binders"] = many
		}},
		{"suite binder shape", "schema", func(d map[string]any) { suiteAt(d, 0)["binders"] = []any{[]any{"a"}} }},
		{"suite binder name", "schema", func(d map[string]any) {
			suiteAt(d, 0)["binders"] = []any{[]any{int64(1), map[string]any{"domain": "bool"}}}
		}},
		{"suite duplicate binder", "schema", func(d map[string]any) {
			suiteAt(d, 0)["binders"] = []any{[]any{"a", map[string]any{"domain": "bool"}}, []any{"a", map[string]any{"domain": "bool"}}}
		}},
		{"suite binder domain", "unsupported-domain", func(d map[string]any) {
			suiteAt(d, 0)["binders"] = []any{[]any{"a", map[string]any{"domain": "nope"}}}
		}},
		{"suite cases zero", "schema", func(d map[string]any) { suiteAt(d, 0)["cases"] = int64(0) }},
		{"suite cases negative", "schema", func(d map[string]any) { suiteAt(d, 0)["cases"] = int64(-1) }},
		{"suite cases product", "work", func(d map[string]any) { suiteAt(d, 0)["cases"] = int64(257) }},
		{"suite result type", "unsupported-type", func(d map[string]any) { suiteAt(d, 0)["result_type"] = "Foo" }},
		{"suite digest", "schema", func(d map[string]any) { suiteAt(d, 0)["inputs_sha256"] = "abc" }},
		{"suite domain text", "schema", func(d map[string]any) { suiteAt(d, 0)["domain_text"] = "" }},
		{"suite expected calls", "schema", func(d map[string]any) { suiteAt(d, 0)["expected_calls"] = "x" }},
		{"suite duplicate", "schema", func(d map[string]any) {
			s := d["suites"].([]any)
			d["suites"] = append(s, s[0])
		}},
		{"faults empty", "schema", func(d map[string]any) { d["faults"] = []any{} }},
		{"fault fields", "schema", func(d map[string]any) { d["faults"].([]any)[0].(map[string]any)["x"] = "y" }},
		{"fault unknown", "fault", func(d map[string]any) { d["faults"].([]any)[0].(map[string]any)["fault"] = "nope" }},
		{"fault op unknown", "fault", func(d map[string]any) { d["faults"].([]any)[0].(map[string]any)["op"] = "nope" }},
		{"fault not applicable", "fault", func(d map[string]any) { d["faults"].([]any)[0].(map[string]any)["fault"] = "swap-choice" }},
		{"fault stage", "fault", func(d map[string]any) { d["faults"].([]any)[0].(map[string]any)["applies"] = "sideways" }},
		{"fault duplicate", "fault", func(d map[string]any) {
			f := d["faults"].([]any)
			d["faults"] = append(f, f[0])
		}},
		{"fault op not in a suite", "fault", func(d map[string]any) { d["faults"].([]any)[0].(map[string]any)["op"] = "eq" }},
		{"controls not list", "schema", func(d map[string]any) { d["controls"] = "x" }},
		{"controls dropped", "control", func(d map[string]any) { d["controls"] = d["controls"].([]any)[:1] }},
		{"controls claims type", "schema", func(d map[string]any) { d["controls"].([]any)[0].(map[string]any)["required_claims"] = []any{int64(1)} }},
		{"controls claims not list", "schema", func(d map[string]any) { d["controls"].([]any)[0].(map[string]any)["required_claims"] = "x" }},
		{"controls row not object", "schema", func(d map[string]any) { d["controls"].([]any)[0] = "x" }},
		{"controls fields", "schema", func(d map[string]any) { d["controls"].([]any)[0].(map[string]any)["x"] = "y" }},
		{"controls wrong claim", "control", func(d map[string]any) {
			d["controls"].([]any)[0].(map[string]any)["required_claims"] = []any{"warrant/ct_compare/exhaustive/eq_fixed_agrees_w2", "other"}
		}},
		{"controls fewer claims", "control", func(d map[string]any) {
			d["controls"].([]any)[0].(map[string]any)["required_claims"] = []any{"warrant/ct_compare/exhaustive/eq_fixed_agrees_w2"}
		}},
		{"controls wrong target", "control", func(d map[string]any) { d["controls"].([]any)[0].(map[string]any)["target_kind"] = "property" }},
		{"controls swapped", "control", func(d map[string]any) {
			c := d["controls"].([]any)
			c[0], c[1] = c[1], c[0]
		}},
	}
	for _, c := range cases {
		tamperRefused(t, c.name, c.code, c.change)
	}
}

func TestWorkCaps(t *testing.T) {
	// A suite whose declared product exceeds the per-suite cap.
	tamperRefused(t, "per-suite cap", "work", func(d map[string]any) {
		s := suiteAt(d, 0)
		s["binders"] = []any{
			[]any{"a", map[string]any{"domain": "bytes_d1"}},
			[]any{"b", map[string]any{"domain": "bytes_d1"}},
			[]any{"c", map[string]any{"domain": "bytes_d1"}},
		}
		s["cases"] = int64(347 * 347 * 347)
		s["discarded"] = int64(0)
	})
	// A product that would overflow an int is refused before it is multiplied.
	tamperRefused(t, "overflow", "work", func(d map[string]any) {
		var binders []any
		for i := 0; i < 8; i++ {
			binders = append(binders, []any{"v" + itoa(i), map[string]any{"domain": "bytes_n(4096)"}})
		}
		suiteAt(d, 0)["binders"] = binders
	})
	// The total (suites once plus every control's re-runs) is capped. Six
	// controls that each re-run two near-million-case suites push it over.
	tamperRefused(t, "total cap", "work", func(d map[string]any) {
		var ids []any
		for i := 0; i < 2; i++ {
			s := suiteAt(d, i)
			s["ops"] = []any{"eq_fixed", "eq", "eq_u64"}
			s["binders"] = []any{
				[]any{"a", map[string]any{"domain": "bytes_d1"}},
				[]any{"b", map[string]any{"domain": "bytes_d1"}},
				[]any{"c", map[string]any{"domain": "bool"}},
				[]any{"d", map[string]any{"domain": "bool"}},
				[]any{"e", map[string]any{"domain": "bool"}},
			}
			s["cases"] = int64(347 * 347 * 8)
			s["discarded"] = int64(0)
			ids = append(ids, s["id"])
		}
		var faultRows []any
		var controlRows []map[string]any
		for _, row := range [][2]string{
			{"ignore-last-byte", "eq"}, {"invert", "eq"}, {"ignore-last-byte", "eq_fixed"}, {"invert", "eq_fixed"},
			{"ignore-high-bit", "eq_u64"}, {"invert", "eq_u64"},
		} {
			faultRows = append(faultRows, map[string]any{"applies": "input", "fault": row[0], "op": row[1]})
			controlRows = append(controlRows, map[string]any{
				"id": "warrant/ct_compare/negctl/exhaustive/" + row[1] + "/" + row[0], "fault": row[0], "op": row[1],
				"target_kind": "exhaustive", "required_claims": ids,
			})
		}
		sort.Slice(controlRows, func(i, j int) bool { return controlRows[i]["id"].(string) < controlRows[j]["id"].(string) })
		d["faults"] = faultRows
		var asAnyRows []any
		for _, row := range controlRows {
			asAnyRows = append(asAnyRows, row)
		}
		d["controls"] = asAnyRows
	})
}

func TestLoadCorpusRefusesBadFiles(t *testing.T) {
	f := newFixture(t, fixtureOptions{mini: true})
	cases := map[string]string{"corpus.json": "malformed-json", "cases.tfv": "tfv"}
	for name, code := range cases {
		path := filepath.Join(f.corpusDir, name)
		original := mustRead(t, path)
		mustWrite(t, path, []byte("\xff"))
		wantCode(t, code, func() { loadCorpus(f.root, f.corpusDir) })
		mustWrite(t, path, original)
	}
	if err := os.Remove(filepath.Join(f.corpusDir, "cases.tfv")); err != nil {
		t.Fatal(err)
	}
	wantCode(t, "io", func() { loadCorpus(f.root, f.corpusDir) })
	wantCode(t, "io", func() { listControlRows(filepath.Join(f.root, "nowhere")) })
}

func TestLoadCorpusMapsTypeConfusionToASchemaRefusal(t *testing.T) {
	// "binders" holds a number where a list of pairs belongs: not one of the
	// shapes the explicit checks name, but still a refusal rather than a crash.
	tamperRefused(t, "malformed field", "schema", func(d map[string]any) { suiteAt(d, 0)["binders"] = int64(5) })
}

func TestTFVCrossChecks(t *testing.T) {
	for name, edit := range map[string]func(string) string{
		"dropped record": func(s string) string { return strings.Join(strings.Split(s, "\n")[:2], "\n") + "\n" },
		"swapped records": func(s string) string {
			l := strings.Split(s, "\n")
			l[1], l[2] = l[2], l[1]
			return strings.Join(l, "\n")
		},
		"changed argument": func(s string) string {
			return strings.Replace(s, "Ka X000102030405060708090a0b0c0d0e0f", "Ka X000102030405060708090a0b0c0d0e00", 1)
		},
		"changed expectation": func(s string) string { return strings.Replace(s, "Kexpect T", "Kexpect F", 1) },
		"changed width":       func(s string) string { return strings.Replace(s, "Kwidth I0", "Kwidth I1", 1) },
		"changed op":          func(s string) string { return strings.Replace(s, "S65715f6669786564", "S6571", 1) },
		"extra field":         func(s string) string { return strings.Replace(s, "Kexpect", "Kextra N Kexpect", 1) },
	} {
		f := newFixture(t, fixtureOptions{mini: true, tfv: edit})
		if code := laneCode(t, func() { loadCorpus(f.root, f.corpusDir) }); code != "tfv" {
			t.Errorf("%s: code %q", name, code)
		}
	}
	// A spec edited after compilation is stale evidence.
	f := newFixture(t, fixtureOptions{mini: true, spec: func(b []byte) []byte { return append(b, '\n') }})
	wantCode(t, "stale-evidence", func() { loadCorpus(f.root, f.corpusDir) })
}
