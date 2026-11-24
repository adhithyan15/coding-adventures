package pbt

import (
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/packages/go/test-bootstrap/tfv"
)

// Regression tests for the VER05d security review: generation budget
// (CWE-400), bounded problem text, and recursion depth.

func firstOfOp(t *testing.T, cases []any, op string) string {
	t.Helper()
	for _, c := range cases {
		if c.(map[string]any)["op"] == op {
			return caseID(c)
		}
	}
	t.Fatalf("no %s case", op)
	return ""
}

func repeatNodes(n int, node any) []any {
	out := make([]any, n)
	for i := range out {
		out[i] = node
	}
	return out
}

func TestGenerationBudgetBoundsCountTimesWidth(t *testing.T) {
	cases := loadCases(t)
	start := time.Now()

	// 10,000 x 10,000 bytes is about 100 MB: each cap alone is allowed, the
	// product is not.  The outputs list has exactly count entries, so only the
	// budget can reject it.
	id := firstOfOp(t, cases, "gen_bytes")
	got := EvaluateCorpus(with(t, cases, id, func(c map[string]any) {
		c["count"] = intNode(10000)
		c["min_len"] = intNode(0)
		c["max_len"] = intNode(10000)
		c["outputs"] = repeatNodes(10000, map[string]any{"kind": "bytes", "hex": ""})
	}))
	if !mentions(got, id) || !strings.Contains(strings.Join(got, "|"), "generation budget") {
		t.Fatalf("gen_bytes product not rejected by the budget: %v", got)
	}

	// 10,000 draws over 1,000-digit bounds (about 415 bytes each) is over 4 MB.
	id = firstOfOp(t, cases, "gen_bounded")
	wide := map[string]any{"kind": "int", "value": strings.Repeat("9", CorpusMaxIntDigits)}
	got = EvaluateCorpus(with(t, cases, id, func(c map[string]any) {
		c["count"] = intNode(10000)
		c["min"] = intNode(0)
		c["max"] = wide
		c["outputs"] = repeatNodes(10000, intNode(0))
	}))
	if !mentions(got, id) || !strings.Contains(strings.Join(got, "|"), "generation budget") {
		t.Fatalf("gen_bounded product not rejected by the budget: %v", got)
	}

	// Outputs whose length is not count are refused before anything is generated.
	for _, op := range []string{"gen_primitive", "gen_bounded", "gen_bytes"} {
		id := firstOfOp(t, cases, op)
		got := EvaluateCorpus(with(t, cases, id, func(c map[string]any) {
			c["count"] = intNode(5000)
			c["outputs"] = []any{}
		}))
		if !strings.Contains(strings.Join(got, "|"), "exactly count") {
			t.Errorf("%s: length mismatch not reported up front: %v", op, got)
		}
	}
	if time.Since(start) > 10*time.Second {
		t.Fatal("budget checks must run before generation")
	}

	// The library entry point enforces the same total.
	if _, err := GenBytes(1, 0, 10000, 10000); err == nil {
		t.Fatal("GenBytes must refuse a 100 MB request")
	}
	if _, err := GenBytes(1, 0, 100, 1000); err != nil {
		t.Fatalf("a modest request must work: %v", err)
	}
}

func TestProblemTextIsBoundedForHostileFixtures(t *testing.T) {
	cases := loadCases(t)
	huge := strings.Repeat("A", 2<<20)
	first := caseID(cases[0])

	mutations := map[string][]any{
		"huge id": with(t, cases, first, func(c map[string]any) { c["id"] = huge }),
		"huge op": with(t, cases, first, func(c map[string]any) { c["op"] = huge }),
		"huge unknown field": with(t, cases, first, func(c map[string]any) {
			c[huge] = true
		}),
		"huge seed node": with(t, cases, first, func(c map[string]any) {
			c["seed"] = map[string]any{"kind": "int", "value": huge}
		}),
		"huge type": with(t, cases, first, func(c map[string]any) { c["type"] = huge }),
		"huge property": with(t, cases, firstOfOp(t, cases, "shrink_int"), func(c map[string]any) {
			c["property"] = huge
		}),
		"huge failure input": with(t, cases, firstOfOp(t, cases, "failure_id_valid"), func(c map[string]any) {
			c["input"] = huge
		}),
		"huge non-object case": append(deepCopy(cases).([]any), huge),
	}
	for name, mutated := range mutations {
		problems := EvaluateCorpusReport(mutated, -1).Problems
		// A huge but well-formed string id is legal; everything else is a problem.
		if len(problems) == 0 && name != "huge id" {
			t.Errorf("%s: no problem reported", name)
		}
		for _, p := range problems {
			if len(p) > maxProblemLen+3 {
				t.Errorf("%s: a problem is %d bytes long", name, len(p))
			}
		}
	}

	// tfv errors clip the quoted token.
	err := tfv.Validate("tfv 1\nr R1 K" + huge + " N\n")
	if err == nil || len(err.Error()) > 400 {
		t.Fatalf("tfv error not bounded: %v", err)
	}
	err = tfv.Validate("tfv 1\nr R1 Ka I" + huge + "\n")
	if err == nil || len(err.Error()) > 400 {
		t.Fatalf("tfv integer error not bounded: %v", err)
	}
	err = tfv.Validate("tfv 1\n" + huge + " R0\n")
	if err == nil || len(err.Error()) > 400 {
		t.Fatalf("tfv id error not bounded: %v", err)
	}
}

func deepList(n int) any {
	var v any = []any{}
	for i := 0; i < n; i++ {
		v = []any{v}
	}
	return v
}

func TestEvaluateCorpusSurvivesDeepHandBuiltTrees(t *testing.T) {
	cases := loadCases(t)
	deep := deepList(100_000)
	// Straight to the exported entry point: no ParseCorpus depth cap applied.
	for _, op := range []string{"gen_primitive", "shrink_list", "shrink_int", "failure_id_valid"} {
		id := firstOfOp(t, cases, op)
		for _, field := range []string{"outputs", "initial", "expected_minimal", "input", "seed"} {
			present := false
			for _, c := range cases {
				if caseID(c) == id {
					_, present = c.(map[string]any)[field]
				}
			}
			if !present {
				continue
			}
			mutated := with(t, cases, id, func(c map[string]any) { c[field] = deep })
			if got := EvaluateCorpus(mutated); !mentions(got, id) { // must return, not overflow the stack
				t.Errorf("%s.%s: a 100k-deep value must be a problem: %v", op, field, got)
			}
		}
	}
	id := firstOfOp(t, cases, "gen_primitive")
	got := EvaluateCorpus(with(t, cases, id, func(c map[string]any) { c["outputs"] = deep }))
	if !mentions(got, id) || !strings.Contains(strings.Join(got, "|"), "nesting deeper") {
		t.Fatalf("deep outputs not reported as nesting: %v", got)
	}
	// sameValue itself is depth-capped: two equal 100k-deep lists are not "same".
	if sameValue(deepList(100_000), deepList(100_000)) {
		t.Fatal("sameValue must give up on absurd depth")
	}
	if !sameValue(deepList(10), deepList(10)) {
		t.Fatal("shallow equal lists must still compare equal")
	}
	if _, err := decode(deep); err == nil {
		t.Fatal("decode must refuse absurd depth")
	}
}
