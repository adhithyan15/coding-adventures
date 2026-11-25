package pbt

import (
	"encoding/json"
	"fmt"
	"math/big"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/packages/go/test-bootstrap/prng"
)

// These are the negative controls (VER05d section 6).  A conformance run that
// cannot fail is the LH-003 false pass this repository has been burned by, so
// every claim "the corpus passes" is paired with a test that corrupts the
// corpus, or the engine, and demands that the runner notice.

func deepCopy(v any) any {
	switch n := v.(type) {
	case map[string]any:
		out := make(map[string]any, len(n))
		for k, x := range n {
			out[k] = deepCopy(x)
		}
		return out
	case []any:
		out := make([]any, len(n))
		for i, x := range n {
			out[i] = deepCopy(x)
		}
		return out
	}
	return v
}

func caseID(c any) string { return c.(map[string]any)["id"].(string) }

// with returns a deep copy of cases where the case with the given id has been
// edited by edit.
func with(t *testing.T, cases []any, id string, edit func(map[string]any)) []any {
	t.Helper()
	out := deepCopy(cases).([]any)
	for _, c := range out {
		if caseID(c) == id {
			edit(c.(map[string]any))
			return out
		}
	}
	t.Fatalf("no case %q", id)
	return nil
}

func mentions(problems []string, id string) bool {
	for _, p := range problems {
		if strings.HasPrefix(p, id+":") {
			return true
		}
	}
	return false
}

// intNode builds the JSON form of an integer.
func intNode(v int64) map[string]any {
	return map[string]any{"kind": "int", "value": fmt.Sprint(v)}
}

func bumpInt(node any, by int64) any {
	n := node.(map[string]any)
	v, _ := new(big.Int).SetString(n["value"].(string), 10)
	v.Add(v, big.NewInt(by))
	return map[string]any{"kind": "int", "value": v.String()}
}

func flipHex(node any) any {
	n := node.(map[string]any)
	h := n["hex"].(string)
	if h == "" {
		return map[string]any{"kind": "bytes", "hex": "00"}
	}
	first := "0"
	if h[0] == '0' {
		first = "1"
	}
	return map[string]any{"kind": "bytes", "hex": first + h[1:]}
}

// corruptions: for each op, edits that make a real case wrong in one category.
var corruptions = map[string]map[string]func(map[string]any){
	"gen_primitive": {
		"flip an output": func(c map[string]any) {
			outs := c["outputs"].([]any)
			if b, isBool := outs[0].(bool); isBool {
				outs[0] = !b
			} else {
				outs[0] = bumpInt(outs[0], 1)
			}
		},
		"drop an output":  func(c map[string]any) { c["outputs"] = c["outputs"].([]any)[1:] },
		"wrong seed":      func(c map[string]any) { c["seed"] = bumpInt(c["seed"], 1) },
		"unknown type":    func(c map[string]any) { c["type"] = "u7" },
		"type not string": func(c map[string]any) { c["type"] = []any{} },
	},
	"gen_bounded": {
		"flip an output": func(c map[string]any) { c["outputs"].([]any)[0] = bumpInt(c["outputs"].([]any)[0], 1) },
		"wrong max":      func(c map[string]any) { c["max"] = bumpInt(c["max"], 1) },
		"min above max":  func(c map[string]any) { c["min"] = bumpInt(c["max"], 1) },
	},
	"gen_bytes": {
		"flip an output": func(c map[string]any) {
			outs := c["outputs"].([]any)
			for i := range outs {
				if outs[i].(map[string]any)["hex"].(string) != "" {
					outs[i] = flipHex(outs[i])
					return
				}
			}
			outs[0] = flipHex(outs[0]) // all-empty outputs: a non-empty claim is wrong too
		},
		"wrong length bound": func(c map[string]any) { c["max_len"] = bumpInt(c["max_len"], 3) },
	},
	"shrink_int": {
		"steps plus one": func(c map[string]any) { c["expected_steps"] = bumpInt(c["expected_steps"], 1) },
		"wrong minimal":  func(c map[string]any) { c["expected_minimal"] = bumpInt(c["expected_minimal"], 1) },
		"unknown property": func(c map[string]any) {
			c["property"] = "int_le_101"
		},
		"initial passes": func(c map[string]any) { c["initial"] = intNode(0) },
	},
	"shrink_refinement": {
		"steps plus one": func(c map[string]any) { c["expected_steps"] = bumpInt(c["expected_steps"], 1) },
		"wrong minimal":  func(c map[string]any) { c["expected_minimal"] = bumpInt(c["expected_minimal"], -1) },
	},
	"shrink_list": {
		"steps plus one": func(c map[string]any) { c["expected_steps"] = bumpInt(c["expected_steps"], 1) },
		"wrong minimal": func(c map[string]any) {
			m := c["expected_minimal"].([]any)
			m[0] = bumpInt(m[0], 1)
		},
		"extra element": func(c map[string]any) {
			c["expected_minimal"] = append(c["expected_minimal"].([]any), intNode(0))
		},
		"bytes property on a list": func(c map[string]any) { c["property"] = "bytes_len_lt_5" },
	},
	"shrink_bytes": {
		"steps plus one": func(c map[string]any) { c["expected_steps"] = bumpInt(c["expected_steps"], 1) },
		"wrong minimal":  func(c map[string]any) { c["expected_minimal"] = flipHex(c["expected_minimal"]) },
	},
	"replay_consistency": {
		"wrong initial_fail": func(c map[string]any) { c["initial_fail"] = bumpInt(c["initial_fail"], 1) },
		"wrong minimal":      func(c map[string]any) { c["expected_minimal"] = bumpInt(c["expected_minimal"], 1) },
	},
	"filter_exhausted": {
		"wrong failure id": func(c map[string]any) { c["failure_id"] = "property-panic" },
		"verdict pass":     func(c map[string]any) { c["verdict"] = "pass" },
		"zero retries":     func(c map[string]any) { c["max_retries"] = intNode(0) },
	},
	"failure_id_valid": {
		"flip valid": func(c map[string]any) { c["valid"] = !c["valid"].(bool) },
		"input is not a string": func(c map[string]any) {
			c["input"] = []any{"property-panic"}
			c["valid"] = true
		},
	},
	"corpus_reject": {
		"accepted text":   func(c map[string]any) { c["text"] = "tfv 1\n" },
		"text not string": func(c map[string]any) { c["text"] = true },
	},
}

func TestEveryOpHasCorruptionsAndEveryCaseIsCaught(t *testing.T) {
	cases := loadCases(t)
	tested := map[string]int{}
	for _, c := range cases {
		m := c.(map[string]any)
		op := m["op"].(string)
		edits, known := corruptions[op]
		if !known {
			t.Fatalf("op %s has no corruption set: add one", op)
		}
		id := m["id"].(string)
		for name, edit := range edits {
			mutated := with(t, cases, id, edit)
			problems := EvaluateCorpus(mutated)
			if !mentions(problems, id) {
				t.Errorf("%s / %s: corrupted case not reported (problems: %v)", id, name, problems)
			}
			tested[op]++
		}
	}
	for op := range shapes {
		if tested[op] == 0 {
			t.Errorf("op %s was never corrupted", op)
		}
	}
}

func TestTruncatedAndStructurallyBrokenCorporaFail(t *testing.T) {
	cases := loadCases(t)
	if got := EvaluateCorpus(cases[:len(cases)-1]); len(got) == 0 {
		t.Fatal("a corpus missing its last case must not conform")
	}
	if got := EvaluateCorpus(nil); len(got) == 0 {
		t.Fatal("an empty corpus must not conform")
	}
	for op := range shapes {
		var kept []any
		for _, c := range cases {
			if c.(map[string]any)["op"] != op {
				kept = append(kept, c)
			}
		}
		report := EvaluateCorpusReport(kept, -1)
		found := false
		for _, p := range report.Problems {
			found = found || strings.Contains(p, "operations never exercised") && strings.Contains(p, op)
		}
		if !found {
			t.Errorf("dropping every %s case was not reported: %v", op, report.Problems)
		}
	}
	// With the count check disabled, the last case being absent is still not
	// silent for a one-case op: dropping the only corpus_reject-like coverage
	// is covered above; here check the count check itself is the one firing.
	report := EvaluateCorpusReport(cases[:24], PBTV1CaseCount)
	if len(report.Problems) == 0 || !strings.Contains(report.Problems[0], "expected 25 cases, got 24") {
		t.Fatalf("count gate did not fire first: %v", report.Problems)
	}
}

func TestRegisteredIDsAndRejectionAreRequired(t *testing.T) {
	cases := loadCases(t)
	var kept []any
	for _, c := range cases {
		m := c.(map[string]any)
		if m["op"] == "failure_id_valid" && m["valid"] == false {
			continue // drop the unregistered-id rejection
		}
		kept = append(kept, c)
	}
	report := EvaluateCorpusReport(kept, -1)
	if !strings.Contains(strings.Join(report.Problems, "|"), "unregistered-id rejection") {
		t.Fatalf("missing rejection case not reported: %v", report.Problems)
	}
	kept = nil
	for _, c := range cases {
		m := c.(map[string]any)
		if m["input"] == "property-timeout" {
			continue
		}
		kept = append(kept, c)
	}
	report = EvaluateCorpusReport(kept, -1)
	if !strings.Contains(strings.Join(report.Problems, "|"), "registered failure ids missing") {
		t.Fatalf("missing registered id not reported: %v", report.Problems)
	}
}

func TestMalformedCasesAreProblemsNotCrashes(t *testing.T) {
	cases := loadCases(t)
	first := caseID(cases[0])
	problemsFor := func(mutated []any) []string { return EvaluateCorpusReport(mutated, -1).Problems }

	check := func(name string, mutated []any, wantSub string) {
		t.Helper()
		got := strings.Join(problemsFor(mutated), "\n")
		if !strings.Contains(got, wantSub) {
			t.Errorf("%s: want a problem containing %q, got:\n%s", name, wantSub, got)
		}
	}

	check("unknown op", with(t, cases, first, func(c map[string]any) { c["op"] = "gen_quantum" }), "unknown op")
	check("op not a string", with(t, cases, first, func(c map[string]any) { c["op"] = 7.0 }), "unknown op")
	check("extra field", with(t, cases, first, func(c map[string]any) { c["extra"] = true }), "fields")
	check("missing field", with(t, cases, first, func(c map[string]any) { delete(c, "outputs") }), "fields")
	check("renamed field", with(t, cases, first, func(c map[string]any) { c["outs"] = c["outputs"]; delete(c, "outputs") }), "fields")
	check("numeric id", with(t, cases, first, func(c map[string]any) { c["id"] = 12.0 }), "case id")
	check("list id", with(t, cases, first, func(c map[string]any) { c["id"] = []any{"a"} }), "case id")
	check("empty id", with(t, cases, first, func(c map[string]any) { c["id"] = "" }), "case id")
	check("missing id", with(t, cases, first, func(c map[string]any) { delete(c, "id") }), "case id")

	dup := deepCopy(cases).([]any)
	dup[1].(map[string]any)["id"] = first
	check("duplicate id", dup, "duplicate or missing case id")

	notObj := deepCopy(cases).([]any)
	notObj[0] = "just a string"
	check("case is not an object", notObj, "not an object")

	for name, node := range map[string]any{
		"leading zero":        map[string]any{"kind": "int", "value": "007"},
		"negative zero":       map[string]any{"kind": "int", "value": "-0"},
		"plus sign":           map[string]any{"kind": "int", "value": "+1"},
		"underscore":          map[string]any{"kind": "int", "value": "1_0"},
		"space":               map[string]any{"kind": "int", "value": " 1"},
		"empty":               map[string]any{"kind": "int", "value": ""},
		"value not string":    map[string]any{"kind": "int", "value": 1.0},
		"extra key":           map[string]any{"kind": "int", "value": "1", "x": "y"},
		"unknown kind":        map[string]any{"kind": "float", "value": "1"},
		"json number":         json.Number("5"),
		"null":                nil,
		"too many digits":     map[string]any{"kind": "int", "value": strings.Repeat("9", CorpusMaxIntDigits+1)},
		"too many neg digits": map[string]any{"kind": "int", "value": "-" + strings.Repeat("9", CorpusMaxIntDigits+1)},
	} {
		node := node
		check("seed "+name, with(t, cases, first, func(c map[string]any) { c["seed"] = node }), "could not be evaluated")
	}

	// A canonical 1000-digit seed is allowed (it reduces modulo 2^64).
	big1000 := map[string]any{"kind": "int", "value": strings.Repeat("9", CorpusMaxIntDigits)}
	if got := problemsFor(with(t, cases, first, func(c map[string]any) { c["seed"] = big1000 })); !mentions(got, first) {
		t.Errorf("a changed seed must change the outputs, got %v", got)
	}

	bytesID := ""
	for _, c := range cases {
		if c.(map[string]any)["op"] == "shrink_bytes" {
			bytesID = caseID(c)
		}
	}
	for name, node := range map[string]any{
		"uppercase hex": map[string]any{"kind": "bytes", "hex": "FF"},
		"odd hex":       map[string]any{"kind": "bytes", "hex": "abc"},
		"hex not text":  map[string]any{"kind": "bytes", "hex": 5.0},
		"non hex":       map[string]any{"kind": "bytes", "hex": "zz"},
		"extra key":     map[string]any{"kind": "bytes", "hex": "00", "x": 1.0},
		"too long":      map[string]any{"kind": "bytes", "hex": strings.Repeat("00", CorpusMaxSize+1)},
	} {
		node := node
		check("bytes "+name, with(t, cases, bytesID, func(c map[string]any) { c["initial"] = node }), "could not be evaluated")
	}
	check("initial is an int", with(t, cases, bytesID, func(c map[string]any) { c["initial"] = intNode(5) }), "must be bytes")
	check("property not a string", with(t, cases, bytesID, func(c map[string]any) { c["property"] = 3.0 }), "could not be evaluated")
	check("unknown bytes property", with(t, cases, bytesID, func(c map[string]any) { c["property"] = "bytes_len_lt_6" }), "unknown property")
	check("bytes initial passes", with(t, cases, bytesID, func(c map[string]any) { c["initial"] = map[string]any{"kind": "bytes", "hex": ""} }), "could not be evaluated")

	listID := ""
	for _, c := range cases {
		if c.(map[string]any)["op"] == "shrink_list" {
			listID = caseID(c)
		}
	}
	check("list initial not a list", with(t, cases, listID, func(c map[string]any) { c["initial"] = intNode(1) }), "list of ints")
	check("list element not an int", with(t, cases, listID, func(c map[string]any) { c["initial"] = []any{true} }), "must be an int")
	check("elem_target not an int", with(t, cases, listID, func(c map[string]any) { c["elem_target"] = "0" }), "elem_target")
	check("list initial passes", with(t, cases, listID, func(c map[string]any) { c["initial"] = []any{} }), "could not be evaluated")
	check("list property unknown", with(t, cases, listID, func(c map[string]any) { c["property"] = "list_sum_le_21" }), "unknown property")
}

func TestHostileSizesAreProblemsNotHangs(t *testing.T) {
	cases := loadCases(t)
	idOf := func(op string) string {
		for _, c := range cases {
			if c.(map[string]any)["op"] == op {
				return caseID(c)
			}
		}
		t.Fatalf("no %s case", op)
		return ""
	}
	huge := intNode(1_000_000_000)
	negative := intNode(-1)
	deadline := time.Now().Add(20 * time.Second)

	type tweak struct {
		op, field string
		node      any
	}
	for _, tw := range []tweak{
		{"gen_primitive", "count", huge},
		{"gen_primitive", "count", negative},
		{"gen_bounded", "count", huge},
		{"gen_bytes", "count", huge},
		{"gen_bytes", "max_len", huge},
		{"gen_bytes", "min_len", negative},
		{"filter_exhausted", "max_retries", huge},
		{"filter_exhausted", "max_retries", negative},
	} {
		id := idOf(tw.op)
		got := EvaluateCorpus(with(t, cases, id, func(c map[string]any) { c[tw.field] = tw.node }))
		if !mentions(got, id) {
			t.Errorf("%s.%s hostile size not reported: %v", tw.op, tw.field, got)
		}
	}

	// A list or bytes value above the element cap.
	tooLong := make([]any, CorpusMaxSize+1)
	for i := range tooLong {
		tooLong[i] = intNode(1)
	}
	listID := idOf("shrink_list")
	if got := EvaluateCorpus(with(t, cases, listID, func(c map[string]any) { c["initial"] = tooLong })); !mentions(got, listID) {
		t.Errorf("oversized list not reported: %v", got)
	}

	// Too many cases.
	many := make([]any, 0, corpusMaxCases+5)
	for len(many) < corpusMaxCases+5 {
		many = append(many, cases...)
	}
	if got := EvaluateCorpusReport(many, -1); !strings.Contains(strings.Join(got.Problems, "|"), "exceeds the cap") {
		t.Errorf("case-count cap not reported: %v", got.Problems)
	}
	if time.Now().After(deadline) {
		t.Fatal("hostile sizes took too long: a cap is missing")
	}
}

func TestSeedsAreReducedModulo2To64(t *testing.T) {
	cases := loadCases(t)
	id := ""
	for _, c := range cases {
		m := c.(map[string]any)
		if m["op"] == "gen_primitive" && m["type"] == "u64" && m["seed"].(map[string]any)["value"] == "0" {
			id = caseID(c)
			break
		}
	}
	if id == "" {
		t.Fatal("no u64 seed-0 case in the corpus")
	}
	// Seed 2^64 is the same stream as seed 0: the case must still conform.
	plus := with(t, cases, id, func(c map[string]any) {
		c["seed"] = map[string]any{"kind": "int", "value": "18446744073709551616"}
	})
	if got := EvaluateCorpus(plus); len(got) != 0 {
		t.Fatalf("seed 2^64 must equal seed 0: %v", got)
	}
	a, err1 := GenPrimitive(^uint64(0), "u64", 3)
	b, err2 := GenPrimitive(mustSeed(t, big.NewInt(-1)), "u64", 3)
	if err1 != nil || err2 != nil || !sameValue(a, b) {
		t.Fatalf("seed -1 must equal 2^64-1: %v %v %v %v", a, b, err1, err2)
	}
}

func mustSeed(t *testing.T, v *big.Int) uint64 {
	t.Helper()
	s, err := seedOf(v)
	if err != nil {
		t.Fatal(err)
	}
	return s
}

func TestSeedOfRejectsNonIntegers(t *testing.T) {
	if _, err := seedOf("0"); err == nil {
		t.Fatal("a string seed must be rejected")
	}
}

// -----------------------------------------------------------------------------
// Engine sabotage: break the engine, keep the (correct) corpus, and the runner
// must say so.  Each sabotage restores the hook on exit.
// -----------------------------------------------------------------------------

func TestEngineSabotageIsCaughtByTheCorpus(t *testing.T) {
	cases := loadCases(t)
	if got := EvaluateCorpus(cases); len(got) != 0 {
		t.Fatalf("baseline must conform before sabotage: %v", got)
	}
	sabotages := map[string]func() func(){
		"unit step moves by two": func() func() {
			old := unitCandidate
			unitCandidate = func(current, diff *big.Int) *big.Int {
				return new(big.Int).Sub(current, big.NewInt(2*int64(diff.Sign())))
			}
			return func() { unitCandidate = old }
		},
		"unit step increment is zero": func() func() {
			old := unitCandidate
			unitCandidate = func(current, diff *big.Int) *big.Int { return new(big.Int).Set(current) }
			return func() { unitCandidate = old }
		},
		"bisection takes a third": func() func() {
			old := bisectCandidate
			bisectCandidate = func(current, diff *big.Int) *big.Int {
				return new(big.Int).Sub(current, new(big.Int).Div(diff, big.NewInt(3)))
			}
			return func() { bisectCandidate = old }
		},
		"bisection never moves": func() func() {
			old := bisectCandidate
			bisectCandidate = func(current, diff *big.Int) *big.Int { return new(big.Int).Set(current) }
			return func() { bisectCandidate = old }
		},
		"deletion phase skipped": func() func() {
			skipDeletionPhase = true
			return func() { skipDeletionPhase = false }
		},
		"rng stream shifted": func() func() {
			old := newRNG
			newRNG = func(seed uint64) *prng.SplitMix64 { return prng.New(seed + 1) }
			return func() { newRNG = old }
		},
		"property table weakened": func() func() {
			old := intProps["int_le_100"]
			intProps["int_le_100"] = func(x *big.Int) bool { return x.Cmp(big.NewInt(99)) <= 0 }
			return func() { intProps["int_le_100"] = old }
		},
	}
	for name, apply := range sabotages {
		func() {
			restore := apply()
			defer restore()
			if got := EvaluateCorpus(cases); len(got) == 0 {
				t.Errorf("sabotage %q was not detected by the corpus run", name)
			}
		}()
	}
	if got := EvaluateCorpus(cases); len(got) != 0 {
		t.Fatalf("hooks were not restored: %v", got)
	}
}

func TestAnEvaluatorThatPanicsIsAProblem(t *testing.T) {
	cases := loadCases(t)
	old := unitCandidate
	defer func() { unitCandidate = old }()
	unitCandidate = func(current, diff *big.Int) *big.Int { panic("engine bug") }
	got := EvaluateCorpus(cases)
	if !strings.Contains(strings.Join(got, "|"), "panic: engine bug") {
		t.Fatalf("an engine panic must be a reported problem, got %v", got)
	}
}
