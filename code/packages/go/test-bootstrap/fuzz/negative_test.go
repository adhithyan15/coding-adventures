package fuzz

import (
	"encoding/json"
	"fmt"
	"math/big"
	"slices"
	"strings"
	"testing"
)

// A runner that cannot fail proves nothing, so most of this file corrupts a
// real case and checks that EvaluateCorpus notices.

func deepCopy(t testing.TB, v any) any {
	t.Helper()
	raw, err := json.Marshal(v)
	if err != nil {
		t.Fatal(err)
	}
	var out any
	if err := json.Unmarshal(raw, &out); err != nil {
		t.Fatal(err)
	}
	return out
}

// first returns a deep copy of the first case of an op that satisfies pred.
func first(t testing.TB, op string, pred func(map[string]any) bool) map[string]any {
	t.Helper()
	for _, raw := range loadCases(t) {
		c := raw.(map[string]any)
		if c["op"] == op && (pred == nil || pred(c)) {
			return deepCopy(t, c).(map[string]any)
		}
	}
	t.Fatalf("no %s case in the corpus", op)
	return nil
}

func intNode(n string) map[string]any   { return map[string]any{"kind": "int", "value": n} }
func bytesNode(h string) map[string]any { return map[string]any{"kind": "bytes", "hex": h} }

func problems(cases ...any) []string { return EvaluateCorpusReport(cases, -1).Problems }

func anyContains(ps []string, needle string) bool {
	for _, p := range ps {
		if strings.Contains(p, needle) {
			return true
		}
	}
	return false
}

func nodeInt(c map[string]any, key string) int64 {
	v, _ := new(big.Int).SetString(c[key].(map[string]any)["value"].(string), 10)
	return v.Int64()
}

func TestFixtureIsTheCommittedTwentyFiveCaseCorpus(t *testing.T) {
	cases := loadCases(t)
	if len(cases) != 25 || FuzzV1CaseCount != 25 {
		t.Fatalf("%d cases", len(cases))
	}
	ops := map[string]bool{}
	for _, c := range cases {
		ops[c.(map[string]any)["op"].(string)] = true
	}
	for op := range shapes {
		if !ops[op] {
			t.Errorf("the fixture has no %s case", op)
		}
	}
	if len(ops) != len(shapes) {
		t.Errorf("the fixture has ops this runner does not know")
	}
}

// -----------------------------------------------------------------------------
// Unknown or malformed input is a problem, never a skip
// -----------------------------------------------------------------------------

func TestUnknownOpIsAProblemNotASkip(t *testing.T) {
	bogus := map[string]any{"id": "fuzz/bogus", "op": "mutate_quantum"}
	if !anyContains(problems(bogus), "unknown op") {
		t.Fatal("unknown op accepted")
	}
	report := EvaluateCorpusReport(append(loadCases(t), bogus), -1)
	if report.Evaluated != 25 || len(report.Problems) == 0 {
		t.Fatalf("evaluated %d, problems %v", report.Evaluated, report.Problems)
	}
	if !anyContains(problems(map[string]any{"id": "x", "op": []any{"a"}}), "unknown op") {
		t.Fatal("a non-string op is an unknown op")
	}
	if !anyContains(problems(map[string]any{"id": "x"}), "unknown op") {
		t.Fatal("a missing op is an unknown op")
	}
}

func TestMissingAndExtraFieldsAreProblems(t *testing.T) {
	for _, field := range shapes["mutate_bit_flip"] {
		c := first(t, "mutate_bit_flip", nil)
		delete(c, field)
		if !anyContains(problems(c), "fields") {
			t.Errorf("deleting %s went unnoticed", field)
		}
	}
	c := first(t, "mutate_bit_flip", nil)
	c["bonus"] = true
	if !anyContains(problems(c), "fields") {
		t.Fatal("an extra field went unnoticed")
	}
	// Same count, wrong name: swap one key.
	c = first(t, "mutate_bit_flip", nil)
	c["bogus"] = c["count"]
	delete(c, "count")
	if !anyContains(problems(c), "fields") {
		t.Fatal("a renamed field went unnoticed")
	}
}

func TestNonCanonicalEncodingsAreProblems(t *testing.T) {
	for _, bad := range []string{"007", "+5", "-0", " 5", "1_0", "", "0x10", "1.0", "-"} {
		c := first(t, "mutate_bit_flip", nil)
		c["bit_offset"] = intNode(bad)
		if !anyContains(problems(c), "could not be evaluated") {
			t.Errorf("int %q accepted", bad)
		}
	}
	for _, bad := range []string{"FF", "f", "ff ", "0G", "0x00"} {
		c := first(t, "mutate_bit_flip", nil)
		c["output"] = bytesNode(bad)
		if !anyContains(problems(c), "could not be evaluated") {
			t.Errorf("hex %q accepted", bad)
		}
	}
	// A node with an extra key, a non-string value, and an unknown kind.
	for name, node := range map[string]any{
		"extra key":    map[string]any{"kind": "int", "value": "1", "x": 1},
		"numeric":      map[string]any{"kind": "int", "value": 1},
		"unknown kind": map[string]any{"kind": "float", "value": "1"},
		"null":         nil,
		"number":       float64(5),
		"hex number":   map[string]any{"kind": "bytes", "hex": 5},
	} {
		c := first(t, "mutate_bit_flip", nil)
		c["count"] = node
		if !anyContains(problems(c), "could not be evaluated") {
			t.Errorf("%s accepted", name)
		}
	}
}

func TestWronglyTypedFieldsAreProblems(t *testing.T) {
	type tc struct {
		c      map[string]any
		needle string
	}
	var cases []tc
	add := func(op string, pred func(map[string]any) bool, mut func(map[string]any), needle string) {
		c := first(t, op, pred)
		mut(c)
		cases = append(cases, tc{c, needle})
	}
	add("mutate_bit_flip", nil, func(c map[string]any) { c["input"] = intNode("5") }, "must be bytes")
	add("mutate_bit_flip", nil, func(c map[string]any) { c["count"] = true }, "must be an int")
	add("mutate_bit_flip", nil, func(c map[string]any) { c["bit_offset"] = bytesNode("00") }, "must be an int")
	add("mutate_byte_flip", nil, func(c map[string]any) { c["byte_offset"] = "x" }, "must be an int")
	add("mutate_arithmetic", nil, func(c map[string]any) { c["endian"] = intNode("1") }, "must be a string")
	add("mutate_arithmetic", nil, func(c map[string]any) { c["width"] = "2" }, "must be an int")
	add("mutate_arithmetic", nil, func(c map[string]any) { c["delta"] = true }, "must be an int")
	add("mutate_boundary", nil, func(c map[string]any) { c["val"] = bytesNode("00") }, "must be an int")
	add("mutate_block_delete", nil, func(c map[string]any) { c["length"] = "1" }, "must be an int")
	add("mutate_block_insert", nil, func(c map[string]any) { c["insert_bytes"] = intNode("1") }, "must be bytes")
	add("mutate_block_duplicate", nil, func(c map[string]any) { c["dst_offset"] = true }, "must be an int")
	add("mutate_splice", nil, func(c map[string]any) { c["donor"] = true }, "must be bytes")
	add("mutate_splice", nil, func(c map[string]any) { c["target_split"] = "1" }, "must be an int")
	add("mutate_sequence", nil, func(c map[string]any) { c["seed"] = "1" }, "must be an int")
	add("mutate_sequence", nil, func(c map[string]any) { c["steps"] = bytesNode("") }, "must be an int")
	add("corpus_admit", nil, func(c map[string]any) { c["trace_edges"] = "oops" }, "list of ints")
	add("corpus_admit", nil, func(c map[string]any) { c["existing_edges"] = []any{bytesNode("00")} }, "list of ints")
	add("corpus_admit", nil, func(c map[string]any) { c["admitted"] = intNode("1") }, "must be a bool")
	add("corpus_admit", nil, func(c map[string]any) { c["new_edges"] = true }, "must be an int")
	add("crash_dedup", nil, func(c map[string]any) { c["crash_a_frames"] = []any{intNode("1")} }, "list of strings")
	add("crash_dedup", nil, func(c map[string]any) { c["crash_b_frames"] = "x" }, "list of strings")
	add("crash_dedup", nil, func(c map[string]any) { c["crash_a_kind"] = intNode("1") }, "must be a string")
	add("crash_dedup", nil, func(c map[string]any) { c["is_duplicate"] = "yes" }, "must be a bool")
	add("private_reporting", nil, func(c map[string]any) { c["target"] = true }, "must be a string")
	add("private_reporting", nil, func(c map[string]any) { c["raw_payload"] = "deadbeef" }, "must be bytes")
	add("private_reporting", nil, func(c map[string]any) { c["contains_raw_payload"] = "no" }, "must be a bool")
	add("corpus_reject", nil, func(c map[string]any) { c["text"] = true }, "must be a string")
	for i, c := range cases {
		if !anyContains(problems(c.c), c.needle) {
			t.Errorf("case %d (%v): no problem containing %q: %v", i, c.c["id"], c.needle, problems(c.c))
		}
	}
}

func TestUnsupportedEndianInTheFixtureIsAProblem(t *testing.T) {
	for _, bad := range []string{"middle", "Little", "", "le"} {
		c := first(t, "mutate_arithmetic", nil)
		c["endian"] = bad
		if !anyContains(problems(c), "endian") {
			t.Errorf("endian %q accepted", bad)
		}
	}
	c := first(t, "mutate_boundary", nil)
	c["endian"] = "native"
	if !anyContains(problems(c), "endian") {
		t.Error("boundary endian accepted")
	}
}

func TestEdgeIdsOutsideUint64AreProblems(t *testing.T) {
	for _, bad := range []string{"-1", "18446744073709551616", "1" + strings.Repeat("0", 100)} {
		c := first(t, "corpus_admit", nil)
		c["trace_edges"] = []any{intNode(bad)}
		if !anyContains(problems(c), "outside [0, 2^64)") {
			t.Errorf("edge %s accepted", bad)
		}
	}
	c := first(t, "corpus_admit", nil)
	c["trace_edges"] = []any{intNode("18446744073709551615")}
	if anyContains(problems(c), "outside") {
		t.Error("the largest uint64 is a valid edge id")
	}
}

func TestNonObjectIdlessAndDuplicateCasesAreProblems(t *testing.T) {
	if !anyContains(problems("nope"), "not an object") {
		t.Fatal("string case")
	}
	if !anyContains(problems(nil), "not an object") {
		t.Fatal("nil case")
	}
	if !anyContains(problems(map[string]any{"op": "mutate_bit_flip"}), "missing") {
		t.Fatal("idless case")
	}
	dup := first(t, "failure_id_valid", nil)
	if !anyContains(problems(dup, dup), "duplicate") {
		t.Fatal("duplicate ids")
	}
	for _, bad := range []any{[]any{1}, map[string]any{"a": 1}, 5.0, nil, true, ""} {
		c := first(t, "failure_id_valid", nil)
		c["id"] = bad
		ps := problems(c)
		if len(ps) == 0 || !anyContains(ps, "id") {
			t.Errorf("id %v: %v", bad, ps)
		}
	}
	a, b := first(t, "failure_id_valid", nil), first(t, "failure_id_valid", nil)
	a["id"], b["id"] = []any{"same"}, []any{"same"}
	if len(problems(a, b)) == 0 {
		t.Fatal("two unhashable ids")
	}
}

// -----------------------------------------------------------------------------
// Truncation and dropped cases
// -----------------------------------------------------------------------------

func TestTruncatedCaseListIsAProblem(t *testing.T) {
	cases := loadCases(t)
	for _, cut := range []int{0, 1, 5, 24} {
		if len(EvaluateCorpus(cases[:cut])) == 0 {
			t.Errorf("truncation to %d not detected", cut)
		}
	}
	if !anyContains(EvaluateCorpus(cases[:24]), "expected 25 cases") {
		t.Fatal("the count check must name the expectation")
	}
	if len(EvaluateCorpus(append(slices.Clone(cases), cases[0]))) == 0 {
		t.Fatal("a 26th (duplicate) case must be a problem")
	}
}

func TestDroppingEveryCaseOfAnOpIsAProblem(t *testing.T) {
	for op := range shapes {
		var kept []any
		for _, c := range loadCases(t) {
			if c.(map[string]any)["op"] != op {
				kept = append(kept, c)
			}
		}
		ps := EvaluateCorpusReport(kept, -1).Problems
		if !anyContains(ps, "never exercised") || !anyContains(ps, op) {
			t.Errorf("dropping %s: %v", op, ps)
		}
	}
}

func TestFailureIDAcceptanceAndRejectionAreBothRequired(t *testing.T) {
	filter := func(drop string) []any {
		var kept []any
		for _, c := range loadCases(t) {
			if c.(map[string]any)["input"] != drop {
				kept = append(kept, c)
			}
		}
		return kept
	}
	if !anyContains(EvaluateCorpusReport(filter("fuzz-crash"), -1).Problems, "acceptance") {
		t.Fatal("no acceptance case must be a problem")
	}
	if !anyContains(EvaluateCorpusReport(filter("fuzz-arbitrary-error"), -1).Problems, "rejection") {
		t.Fatal("no rejection case must be a problem")
	}
}

// -----------------------------------------------------------------------------
// Negative controls: a corrupted expectation must be reported
// -----------------------------------------------------------------------------

type corruption struct {
	name  string
	build func(t *testing.T) map[string]any
}

func flipOutput(op string) corruption {
	return corruption{op + "/flip an output byte", func(t *testing.T) map[string]any {
		c := first(t, op, nil)
		out := c["output"].(map[string]any)
		h := out["hex"].(string)
		repl := "ff"
		if strings.HasPrefix(h, "ff") {
			repl = "00"
		}
		out["hex"] = repl + h[2:]
		return c
	}}
}

func corruptions() []corruption {
	cs := []corruption{}
	for _, op := range []string{"mutate_bit_flip", "mutate_byte_flip", "mutate_arithmetic", "mutate_boundary", "mutate_block_delete", "mutate_block_insert", "mutate_block_duplicate", "mutate_splice", "mutate_sequence"} {
		cs = append(cs, flipOutput(op))
	}
	bump := func(op string, pred func(map[string]any) bool, key string, delta int64) corruption {
		return corruption{fmt.Sprintf("%s/%s%+d", op, key, delta), func(t *testing.T) map[string]any {
			c := first(t, op, pred)
			c[key] = intNode(fmt.Sprint(nodeInt(c, key) + delta))
			return c
		}}
	}
	cs = append(cs,
		corruption{"mutate_bit_flip/lengthened output", func(t *testing.T) map[string]any {
			c := first(t, "mutate_bit_flip", nil)
			o := c["output"].(map[string]any)
			o["hex"] = o["hex"].(string) + "00"
			return c
		}},
		bump("mutate_bit_flip", func(c map[string]any) bool { return nodeInt(c, "bit_offset") == 9 }, "bit_offset", -1),
		bump("mutate_byte_flip", func(c map[string]any) bool { return nodeInt(c, "count") == 2 }, "count", -1),
		bump("mutate_arithmetic", nil, "delta", 1),
		bump("mutate_boundary", func(c map[string]any) bool { return nodeInt(c, "width") == 4 }, "val", -1),
		bump("mutate_block_delete", nil, "length", 1),
		bump("mutate_block_insert", nil, "offset", 1),
		bump("mutate_block_duplicate", nil, "dst_offset", -1),
		bump("mutate_splice", nil, "donor_split", 1),
		bump("mutate_sequence", nil, "seed", 1),
		bump("mutate_sequence", nil, "steps", 1),
		bump("corpus_admit", func(c map[string]any) bool { return c["admitted"] == true }, "new_edges", 1),
		corruption{"mutate_arithmetic/wrong endian", func(t *testing.T) map[string]any {
			c := first(t, "mutate_arithmetic", func(c map[string]any) bool { return c["endian"] == "big" })
			c["endian"] = "little"
			return c
		}},
		corruption{"mutate_boundary/wrong endian", func(t *testing.T) map[string]any {
			c := first(t, "mutate_boundary", func(c map[string]any) bool { return nodeInt(c, "val") == 2147483648 })
			c["endian"] = "big"
			return c
		}},
		corruption{"corpus_admit/flip admitted", func(t *testing.T) map[string]any {
			c := first(t, "corpus_admit", nil)
			c["admitted"] = !c["admitted"].(bool)
			return c
		}},
		corruption{"corpus_admit/extra trace edge", func(t *testing.T) map[string]any {
			c := first(t, "corpus_admit", func(c map[string]any) bool { return c["admitted"] == false })
			c["trace_edges"] = append(c["trace_edges"].([]any), intNode("99"))
			return c
		}},
		corruption{"crash_dedup/duplicate claimed distinct", func(t *testing.T) map[string]any {
			c := first(t, "crash_dedup", func(c map[string]any) bool { return c["is_duplicate"] == true })
			c["is_duplicate"] = false
			return c
		}},
		corruption{"crash_dedup/distinct claimed duplicate", func(t *testing.T) map[string]any {
			c := first(t, "crash_dedup", func(c map[string]any) bool { return c["is_duplicate"] == false })
			c["is_duplicate"] = true
			return c
		}},
		corruption{"crash_dedup/frames reordered", func(t *testing.T) map[string]any {
			c := first(t, "crash_dedup", func(c map[string]any) bool { return c["is_duplicate"] == true })
			f := c["crash_a_frames"].([]any)
			f[0], f[1] = f[1], f[0]
			return c
		}},
		corruption{"failure_id_valid/registered claimed invalid", func(t *testing.T) map[string]any {
			c := first(t, "failure_id_valid", func(c map[string]any) bool { return c["valid"] == true })
			c["valid"] = false
			return c
		}},
		corruption{"failure_id_valid/unregistered claimed valid", func(t *testing.T) map[string]any {
			c := first(t, "failure_id_valid", func(c map[string]any) bool { return c["valid"] == false })
			c["valid"] = true
			return c
		}},
		corruption{"failure_id_valid/non-string input claimed valid", func(t *testing.T) map[string]any {
			c := first(t, "failure_id_valid", func(c map[string]any) bool { return c["valid"] == false })
			c["input"] = intNode("1")
			c["valid"] = true
			return c
		}},
		corruption{"private_reporting/wrong tap line", func(t *testing.T) map[string]any {
			c := first(t, "private_reporting", nil)
			c["tap_line"] = c["tap_line"].(string) + " raw=deadbeef"
			return c
		}},
		corruption{"private_reporting/claims a leak", func(t *testing.T) map[string]any {
			c := first(t, "private_reporting", nil)
			c["contains_raw_payload"] = true
			return c
		}},
		corruption{"private_reporting/wrong hash", func(t *testing.T) map[string]any {
			c := first(t, "private_reporting", nil)
			c["sig_hash"] = "00000000"
			return c
		}},
		corruption{"private_reporting/payload in the line", func(t *testing.T) map[string]any {
			c := first(t, "private_reporting", nil)
			// The line really contains the payload hex; the case denies it.
			c["target"] = "ct_deadbeef"
			c["tap_line"] = "not ok 1 - ct_deadbeef [fuzz-crash:3a8f19b2]"
			return c
		}},
		corruption{"private_reporting/unregistered id", func(t *testing.T) map[string]any {
			c := first(t, "private_reporting", nil)
			c["failure_id"] = "fuzz-arbitrary-error"
			return c
		}},
		corruption{"corpus_reject/accepted text", func(t *testing.T) map[string]any {
			c := first(t, "corpus_reject", nil)
			c["text"] = "tfv 1\n"
			return c
		}},
	)
	return cs
}

func TestCorruptedExpectationIsReported(t *testing.T) {
	cases := loadCases(t)
	for _, c := range corruptions() {
		t.Run(c.name, func(t *testing.T) {
			bad := c.build(t)
			// On its own a case always also trips "operations never exercised",
			// so insist on a problem that NAMES the corrupted case.
			if !anyContains(problems(bad), fmt.Sprint(bad["id"])) {
				t.Fatalf("undetected on its own: %v", problems(bad))
			}
			swapped := make([]any, len(cases))
			for i, orig := range cases {
				swapped[i] = orig
				if orig.(map[string]any)["id"] == bad["id"] {
					swapped[i] = bad
				}
			}
			if len(EvaluateCorpus(swapped)) == 0 {
				t.Fatal("undetected inside the full corpus")
			}
		})
	}
}

func TestEveryOpHasAtLeastOneNegativeControl(t *testing.T) {
	covered := map[string]bool{}
	for _, c := range corruptions() {
		covered[strings.SplitN(c.name, "/", 2)[0]] = true
	}
	for op := range shapes {
		if !covered[op] {
			t.Errorf("%s has no negative control", op)
		}
	}
}

// -----------------------------------------------------------------------------
// Engine sabotage through the unexported hooks
// -----------------------------------------------------------------------------

func TestRunnerCatchesASabotagedEngine(t *testing.T) {
	if p := EvaluateCorpus(loadCases(t)); len(p) != 0 {
		t.Fatal(p)
	}
	type sabotage struct {
		name  string
		apply func() (restore func())
	}
	swap := func(name string, set func()) sabotage {
		return sabotage{name, func() func() {
			set()
			return restoreHooks()
		}}
	}
	one := big.NewInt(1)
	add1 := func(v *big.Int) *big.Int { return new(big.Int).Add(v, one) }
	for _, s := range []sabotage{
		swap("bit_flip off by one", func() { bitFlipFn = func(d []byte, o, c *big.Int) []byte { return BitFlipBig(d, add1(o), c) } }),
		swap("byte_flip count+1", func() { byteFlipFn = func(d []byte, o, c *big.Int) []byte { return ByteFlipBig(d, o, add1(c)) } }),
		swap("arithmetic endian swapped", func() {
			byteArithmeticFn = func(d []byte, o, dl *big.Int, w int, e Endian) []byte {
				return ByteArithmeticBig(d, o, dl, w, 1-e)
			}
		}),
		swap("boundary value xor 1", func() {
			boundaryInsertFn = func(d []byte, o *big.Int, w int, v *big.Int, e Endian) []byte {
				return BoundaryInsertBig(d, o, w, new(big.Int).Xor(v, one), e)
			}
		}),
		swap("block_delete length+1", func() { blockDeleteFn = func(d []byte, o, l *big.Int) []byte { return BlockDeleteBig(d, o, add1(l)) } }),
		swap("block_insert offset+1", func() {
			blockInsertFn = func(d []byte, o *big.Int, ins []byte) []byte { return BlockInsertBig(d, add1(o), ins) }
		}),
		swap("block_duplicate dst+1", func() {
			blockDuplicateFn = func(d []byte, s, l, dst *big.Int) []byte { return BlockDuplicateBig(d, s, l, add1(dst)) }
		}),
		swap("splice target split+1", func() {
			spliceFn = func(a, b []byte, ts, ds *big.Int) []byte { return SpliceBig(a, b, add1(ts), ds) }
		}),
		swap("mutate_sequence seed+1", func() {
			mutateSequenceFn = func(d []byte, s uint64, n, m int) ([]byte, error) { return MutateSequence(d, s+1, n, m) }
		}),
		swap("corpus_admit always admits", func() { corpusAdmitFn = func(e, t []uint64) (bool, int) { return true, 1 } }),
		swap("dedup says always duplicate", func() { isDuplicateFn = func(string, []string, string, []string) bool { return true } }),
		swap("failure id: everything valid", func() { isFailureIDFn = func(string) bool { return true } }),
		swap("tap line leaks the target twice", func() {
			formatTAPFailFn = func(n int, t, f, s string) (string, error) {
				l, err := FormatTAPFailure(n, t, f, s)
				return l + " " + t, err
			}
		}),
		swap("tfv accepts everything", func() { tfvValidateFn = func(string) error { return nil } }),
	} {
		restore := s.apply()
		ps := EvaluateCorpus(loadCases(t))
		restore()
		if len(ps) == 0 {
			t.Errorf("sabotage %q was not caught", s.name)
		}
	}
	if p := EvaluateCorpus(loadCases(t)); len(p) != 0 {
		t.Fatalf("hooks were not restored: %v", p)
	}
}

// restoreHooks snapshots every hook and returns a function that puts them back.
func restoreHooks() func() {
	// snapshot AFTER the sabotage was applied would be wrong, so the real values
	// are re-bound explicitly.
	return func() {
		bitFlipFn, byteFlipFn, byteArithmeticFn, boundaryInsertFn = BitFlipBig, ByteFlipBig, ByteArithmeticBig, BoundaryInsertBig
		blockDeleteFn, blockInsertFn, blockDuplicateFn, spliceFn = BlockDeleteBig, BlockInsertBig, BlockDuplicateBig, SpliceBig
		mutateSequenceFn, corpusAdmitFn, isDuplicateFn, isFailureIDFn = MutateSequence, CorpusAdmit, IsDuplicateCrash, IsFuzzFailureID
		formatTAPFailFn, tfvValidateFn = FormatTAPFailure, tfvValidateReal
	}
}

func TestEngineInvariantsAreCheckedIndependentlyOfExpectedValues(t *testing.T) {
	defer restoreHooks()()
	// A flip that changes the length is reported even when the corpus is
	// rewritten to agree with it.
	c := first(t, "mutate_bit_flip", nil)
	bitFlipFn = func(d []byte, o, n *big.Int) []byte { return append(slices.Clone(d), 0) }
	c["output"] = bytesNode(c["input"].(map[string]any)["hex"].(string) + "00")
	if !anyContains(problems(c), "changed the length") {
		t.Error("length invariant")
	}
	restoreHooks()()

	c = first(t, "mutate_splice", nil)
	spliceFn = func(a, b []byte, ts, ds *big.Int) []byte { return []byte{1} }
	c["output"] = bytesNode("01")
	if !anyContains(problems(c), "prefix + suffix") {
		t.Error("splice length invariant")
	}
	restoreHooks()()

	c = first(t, "corpus_admit", func(c map[string]any) bool { return c["admitted"] == true })
	corpusAdmitFn = func(e, tr []uint64) (bool, int) { return false, 0 }
	c["admitted"], c["new_edges"] = false, intNode("0")
	if !anyContains(problems(c), "set difference") {
		t.Error("admission invariant")
	}
	restoreHooks()()

	c = first(t, "private_reporting", nil)
	want := c["tap_line"].(string)
	formatTAPFailFn = func(n int, t, f, s string) (string, error) { return want + "\n", nil }
	if !anyContains(problems(c), "more than one line") {
		t.Error("single-line invariant")
	}
	restoreHooks()()

	isFailureIDFn = func(s string) bool { return s == "fuzz-crash" }
	if !anyContains(EvaluateCorpus(loadCases(t)), "rejects registered ids") {
		t.Error("registered ids the engine rejects must be reported")
	}
}

func TestAPanickingEngineIsAProblemNotACrash(t *testing.T) {
	defer restoreHooks()()
	bitFlipFn = func(d []byte, o, n *big.Int) []byte { panic("engine bug " + strings.Repeat("x", 5000)) }
	ps := EvaluateCorpus(loadCases(t))
	if !anyContains(ps, "panic: engine bug") {
		t.Fatalf("%v", ps)
	}
	for _, p := range ps {
		if len(p) > 403 {
			t.Fatalf("an unclipped problem of %d bytes", len(p))
		}
	}
	restoreHooks()()
	// A refusing formatter is a problem too.
	formatTAPFailFn = func(int, string, string, string) (string, error) { return "", ErrFailureID }
	if !anyContains(EvaluateCorpus(loadCases(t)), "refused") {
		t.Fatal("a refused TAP line must be reported")
	}
}

func TestAMutatorCaseWithANonBytesOutputIsAProblem(t *testing.T) {
	c := first(t, "mutate_bit_flip", nil)
	c["output"] = intNode("1")
	if !anyContains(problems(c), "output must be bytes") {
		t.Fatalf("%v", problems(c))
	}
}
