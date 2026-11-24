package fuzz

import (
	"strings"
	"testing"
)

// Fixture-controlled sizes are capped (CWE-400): a hostile value is a reported
// problem, never a hang, an out-of-memory or a stack overflow.

func TestParseCorpusIsStrict(t *testing.T) {
	bad := map[string]string{
		"not json":         `nope`,
		"empty":            ``,
		"trailing data":    `{"schema_version":1,"cases":[]} {}`,
		"wrong version":    `{"schema_version":2,"cases":[]}`,
		"string version":   `{"schema_version":"1","cases":[]}`,
		"missing cases":    `{"schema_version":1}`,
		"extra key":        `{"schema_version":1,"cases":[],"x":1}`,
		"cases not array":  `{"schema_version":1,"cases":{}}`,
		"not an object":    `[1]`,
		"duplicate key":    `{"schema_version":1,"cases":[],"cases":[]}`,
		"nested duplicate": `{"schema_version":1,"cases":[{"id":"a","id":"b"}]}`,
		"truncated":        `{"schema_version":1,"cases":[`,
		"deep nesting":     `{"schema_version":1,"cases":` + strings.Repeat("[", 100) + strings.Repeat("]", 100) + `}`,
		"unterminated obj": `{"schema_version":1,"cases":[{"id":"a"`,
		"mismatched close": `{"schema_version":1,"cases":[}`,
		"oversized":        `{"schema_version":1,"cases":[],"pad":"` + strings.Repeat("a", 8<<20) + `"}`,
	}
	for name, doc := range bad {
		_, err := ParseCorpus([]byte(doc))
		if err == nil {
			t.Errorf("%s: accepted", name)
		} else if !strings.Contains(err.Error(), "fuzz corpus") {
			t.Errorf("%s: the error should name this corpus: %v", name, err)
		}
	}
	cases, err := ParseCorpus([]byte(`{"schema_version":1,"cases":[1,"a",null,true,{"x":[]}]}`))
	if err != nil || len(cases) != 5 {
		t.Fatalf("the reader must leave case-shape judgement to the evaluator: %v %v", cases, err)
	}
}

func TestOversizedValuesAreProblemsNotHangs(t *testing.T) {
	c := first(t, "mutate_sequence", nil)
	c["steps"] = intNode("1000000000")
	if !anyContains(problems(c), "outside [0,") {
		t.Fatal("steps = 10^9 accepted")
	}
	c["steps"] = intNode("-1")
	if !anyContains(problems(c), "outside [0,") {
		t.Fatal("negative steps accepted")
	}
	c["steps"] = intNode("10001")
	if !anyContains(problems(c), "outside [0,") {
		t.Fatal("steps = cap+1 accepted")
	}
	c["steps"] = intNode("10000")
	if anyContains(problems(c), "outside [0,") {
		t.Fatal("steps = cap rejected")
	}

	a := first(t, "corpus_admit", nil)
	a["existing_edges"] = append(make([]any, 0), repeatNode(intNode("0"), 10_001)...)
	if !anyContains(problems(a), "list exceeds") {
		t.Fatal("an over-long list was accepted")
	}
	b := first(t, "mutate_bit_flip", nil)
	b["input"] = bytesNode(strings.Repeat("00", 10_001))
	if !anyContains(problems(b), "bytes exceed") {
		t.Fatal("over-long bytes were accepted")
	}
	b["input"] = bytesNode(strings.Repeat("00", 10_000))
	if anyContains(problems(b), "bytes exceed") {
		t.Fatal("bytes at the cap were rejected")
	}
	d := first(t, "mutate_boundary", nil)
	d["val"] = intNode(strings.Repeat("9", 1001))
	if !anyContains(problems(d), "digits") {
		t.Fatal("a 1001-digit int was accepted")
	}
	d["val"] = intNode("-" + strings.Repeat("9", 1001))
	if !anyContains(problems(d), "digits") {
		t.Fatal("a negative 1001-digit int was accepted")
	}
	d["val"] = intNode(strings.Repeat("9", 1000))
	if anyContains(problems(d), "digits") {
		t.Fatal("a 1000-digit int was rejected")
	}
}

func repeatNode(n map[string]any, count int) []any {
	out := make([]any, count)
	for i := range out {
		out[i] = n
	}
	return out
}

func TestHugeButLegalIntegersStayFast(t *testing.T) {
	c := first(t, "mutate_bit_flip", nil)
	c["bit_offset"] = intNode(strings.Repeat("9", 900))
	c["count"] = intNode(strings.Repeat("9", 900))
	c["input"] = bytesNode(strings.Repeat("00", 5000))
	c["output"] = bytesNode(strings.Repeat("00", 5000))
	_ = problems(c) // a mismatch is fine; the point is that it returns
	for _, op := range []string{"mutate_byte_flip", "mutate_arithmetic", "mutate_boundary", "mutate_block_delete", "mutate_block_insert", "mutate_block_duplicate", "mutate_splice"} {
		c := first(t, op, nil)
		for k, v := range c {
			if m, ok := v.(map[string]any); ok && m["kind"] == "int" {
				m["value"] = "-" + strings.Repeat("9", 900)
			}
			_ = k
		}
		_ = problems(c)
	}
}

func TestDepthIsCapped(t *testing.T) {
	// A hand-built tree never went through ParseCorpus, so the evaluator must
	// cap depth itself.
	var deep any = "leaf"
	for i := 0; i < 200_000; i++ {
		deep = []any{deep}
	}
	c := first(t, "corpus_admit", nil)
	c["trace_edges"] = deep
	ps := problems(c)
	if !anyContains(ps, "nesting deeper than") {
		t.Fatalf("%v", ps)
	}
	c = first(t, "crash_dedup", nil)
	c["crash_a_frames"] = deep
	if !anyContains(problems(c), "nesting deeper than") {
		t.Fatal("frames")
	}
	// A deep raw node in a non-value position is clipped, not rendered whole.
	c = first(t, "mutate_bit_flip", nil)
	c["count"] = map[string]any{"kind": "int", "value": deep}
	for _, p := range problems(c) {
		if len(p) > 403 {
			t.Fatalf("a %d-byte problem", len(p))
		}
	}
	// The parser refuses depth > 64 but accepts 64.
	ok := `{"schema_version":1,"cases":` + strings.Repeat("[", 60) + strings.Repeat("]", 60) + `}`
	if _, err := ParseCorpus([]byte(ok)); err != nil {
		t.Fatalf("depth within the cap: %v", err)
	}
}

func TestProblemTextIsClipped(t *testing.T) {
	huge := strings.Repeat("A", 2<<20)
	c := first(t, "mutate_bit_flip", nil)
	c["id"] = huge
	c["bonus"] = true
	ps := problems(c)
	if len(ps) == 0 {
		t.Fatal("expected a problem")
	}
	for _, p := range ps {
		if len(p) > 403 {
			t.Fatalf("a %d-byte problem from a 2 MiB id", len(p))
		}
	}
	// A huge unknown op, a huge non-object case, a huge string field.
	for _, bad := range []any{
		map[string]any{"id": "x", "op": huge},
		huge,
		map[string]any{"op": "mutate_bit_flip", "id": 5, "pad": huge},
	} {
		for _, p := range problems(bad) {
			if len(p) > 403 {
				t.Fatalf("a %d-byte problem", len(p))
			}
		}
	}
	e := first(t, "mutate_arithmetic", nil)
	e["endian"] = huge
	for _, p := range problems(e) {
		if len(p) > 403 {
			t.Fatalf("a %d-byte problem from a huge endian", len(p))
		}
	}
	r := first(t, "private_reporting", nil)
	r["tap_line"] = huge
	for _, p := range problems(r) {
		if len(p) > 403 {
			t.Fatalf("a %d-byte problem from a huge tap line", len(p))
		}
	}
	// The case-id is also bounded inside every message about it.
	f := first(t, "mutate_bit_flip", nil)
	f["id"] = huge
	f["output"] = bytesNode("")
	for _, p := range problems(f) {
		if len(p) > 403 {
			t.Fatal("id not clipped")
		}
	}
}

func TestTooManyCasesAreCappedAndReported(t *testing.T) {
	one := first(t, "failure_id_valid", nil)
	cases := make([]any, 10_050)
	for i := range cases {
		c := deepCopy(t, one).(map[string]any)
		c["id"] = "case" + strings.Repeat("x", i%7) + string(rune('a'+i%26)) + itoa(i)
		cases[i] = c
	}
	report := EvaluateCorpusReport(cases, -1)
	if !anyContains(report.Problems, "exceeds the cap") || report.Evaluated != 10_000 {
		t.Fatalf("evaluated %d: %v", report.Evaluated, report.Problems[:1])
	}
}

func itoa(i int) string {
	if i == 0 {
		return "0"
	}
	var b []byte
	for ; i > 0; i /= 10 {
		b = append([]byte{byte('0' + i%10)}, b...)
	}
	return string(b)
}

func TestCorpusAdmitHandlesUnboundedEdgeCountsInBoundedTime(t *testing.T) {
	// 10,000 edges x 10,000 edges: the novelty check must not be quadratic in a
	// way that matters (it is a map, not a nested scan) ... the invariant check
	// in the runner is bounded by the list cap.
	a := first(t, "corpus_admit", nil)
	var ex, tr []any
	for i := 0; i < 10_000; i++ {
		ex = append(ex, intNode(itoa(i)))
		tr = append(tr, intNode(itoa(i+5_000)))
	}
	a["existing_edges"], a["trace_edges"] = ex, tr
	a["admitted"], a["new_edges"] = true, intNode("5000")
	if ps := problems(a); anyContains(ps, "corpus_admit") {
		t.Fatalf("%v", ps)
	}
}

func TestAggregateSequenceWorkIsBudgeted(t *testing.T) {
	// Each case alone is within the per-field caps (10,000 steps x 1,000 bytes
	// = 10^7 byte-steps); the corpus as a whole is not.
	one := first(t, "mutate_sequence", nil)
	one["steps"] = intNode("10000")
	one["input"] = bytesNode(strings.Repeat("00", 1000))
	one["output"] = bytesNode("")
	cases := make([]any, 40)
	for i := range cases {
		c := deepCopy(t, one).(map[string]any)
		c["id"] = "seq" + itoa(i)
		cases[i] = c
	}
	ps := EvaluateCorpusReport(cases, -1).Problems
	if !anyContains(ps, "work budget") {
		t.Fatal("an aggregate of 4x10^8 byte-steps was not refused")
	}
	// The first case (10^7 <= 2^24) was evaluated, not refused.
	for _, p := range ps {
		if strings.Contains(p, "seq0:") && strings.Contains(p, "work budget") {
			t.Fatal("the budget must not refuse the first case")
		}
	}
	// The real fixture is far below it.
	if p := EvaluateCorpus(loadCases(t)); len(p) != 0 {
		t.Fatal(p)
	}
}
