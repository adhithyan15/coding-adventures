package pbt

import (
	"bytes"
	"errors"
	"fmt"
	"math/big"
	"sort"

	"github.com/adhithyan15/coding-adventures/code/packages/go/test-bootstrap/internal/corpusjson"
	"github.com/adhithyan15/coding-adventures/code/packages/go/test-bootstrap/tfv"
)

// This file is the Go lane's production runner for the shared pbt-v1 corpus
// (code/specs/fixtures/pbt-v1/cases.json, VER05 section 6).  It takes the raw
// JSON, strictly decodes it, evaluates every case with this package's own
// public engine functions and returns human-readable problems.  An empty list
// means "conforms".
//
// It does no file I/O (the package is capability-free); the test that reads the
// committed fixture lives in corpus_test.go.
//
// Why a runner that can fail matters: a conformance check that cannot report a
// problem is worse than none.  So the runner is strict:
//
//   - a case must have exactly the fields of its op, with canonical encodings
//     (ints are decimal strings, bytes are even-length lowercase hex);
//   - an unknown op, unknown property name or malformed value is a problem,
//     never a silent skip;
//   - every op and all four failure ids must appear, at least one unregistered
//     id must be rejected, and the number of cases must equal PBTV1CaseCount,
//     so deleting cases from the list is itself a problem.
//
// Case encoding (JSON):
//
//	{"kind": "int",   "value": "-12"}      an integer (any width, decimal text)
//	{"kind": "bytes", "hex": "00ff"}       a byte string
//	true / false                           a bool
//	[ ... ]                                a list of the above
//	"text"                                 a string (types, property names)

const (
	// PBTV1CaseCount is how many cases pbt-v1 holds; a different count is a
	// problem (a truncated list, or a corpus that grew without this lane being
	// reviewed).
	PBTV1CaseCount = 25
	// CorpusMaxSize bounds every fixture-controlled size: counts, retries,
	// lengths, list and bytes elements.  The fixture is data, so a hostile value
	// such as count = 10^9 must be a reported problem, not a hang (CWE-400).
	CorpusMaxSize = corpusjson.MaxSize
	// CorpusMaxIntDigits bounds the decimal digits of any integer in a case.
	CorpusMaxIntDigits = corpusjson.MaxIntDigits
	// corpusMaxCases bounds how many cases are evaluated.
	corpusMaxCases = 10_000
	// corpusMaxGenBytes bounds the total bytes one gen case may generate and
	// hold: count times the per-item width.  The separate caps on count and
	// width alone would still allow 10,000 x 10,000 bytes (CWE-400).
	corpusMaxGenBytes = 1 << 20
	// corpusMaxBytes bounds the JSON document ParseCorpus will read.
	corpusMaxBytes = corpusjson.MaxBytes
	// maxProblemLen bounds one reported problem; maxFragmentLen one fragment.
	maxProblemLen  = corpusjson.MaxProblemLen
	maxFragmentLen = corpusjson.MaxFragmentLen
)

// The strict-JSON reader, value decoding, equality and clipping are shared with
// the fuzz lane and live in internal/corpusjson; these are the names this
// package (and its tests) use.
var (
	clip           = corpusjson.Clip
	show           = corpusjson.Show
	isCanonicalInt = corpusjson.IsCanonicalInt
	isLowercaseHex = corpusjson.IsLowercaseHex
	decode         = corpusjson.Decode
	sameValue      = corpusjson.Same
)

// ParseCorpus strictly decodes a pbt-v1 document into its case list.  The
// document must be exactly {"schema_version": 1, "cases": [...]}.  Duplicate
// object keys, trailing data, nesting deeper than 64 and documents over 8 MiB
// are refused.  Objects become map[string]any, arrays []any, numbers
// json.Number, and cases are returned unvalidated (EvaluateCorpus judges them).
func ParseCorpus(data []byte) ([]any, error) { return corpusjson.Parse(data, "pbt corpus") }

type caseError = corpusjson.CaseError

func caseErr(format string, args ...any) error { return corpusjson.CaseErr(format, args...) }

func bigsToAny(in []*big.Int) []any {
	out := make([]any, len(in))
	for i, v := range in {
		out[i] = v
	}
	return out
}

func bytesToAny(in [][]byte) []any {
	out := make([]any, len(in))
	for i, v := range in {
		out[i] = v
	}
	return out
}

var mask64 = new(big.Int).SetUint64(^uint64(0))

// seedOf reduces a corpus seed modulo 2^64, as the reference masks it
// (negative seeds wrap in two's complement).
func seedOf(v any) (uint64, error) {
	b, isBig := v.(*big.Int)
	if !isBig {
		return 0, caseErr("seed must be an int")
	}
	return new(big.Int).And(b, mask64).Uint64(), nil
}

func intOf(v any, what string) (*big.Int, error) {
	b, isBig := v.(*big.Int)
	if !isBig {
		return nil, caseErr("%s must be an int", what)
	}
	return b, nil
}

// sizeOf is an int that drives a loop or an allocation: 0 <= n <= CorpusMaxSize.
func sizeOf(v any, what string) (int, error) {
	b, err := intOf(v, what)
	if err != nil {
		return 0, err
	}
	if b.Sign() < 0 || b.Cmp(big.NewInt(CorpusMaxSize)) > 0 {
		return 0, caseErr("%s %s outside [0, %d]", what, show(b), CorpusMaxSize)
	}
	return int(b.Int64()), nil
}

// -----------------------------------------------------------------------------
// Properties named by the corpus.  true means "holds".
// -----------------------------------------------------------------------------

var (
	big100  = big.NewInt(100)
	big50   = big.NewInt(50)
	bigM10  = big.NewInt(-10)
	big10   = big.NewInt(10)
	big20   = big.NewInt(20)
	bigZero = big.NewInt(0)
)

var intProps = map[string]func(*big.Int) bool{
	"int_le_100":            func(x *big.Int) bool { return x.Cmp(big100) <= 0 },
	"int_in_range_neg10_50": func(x *big.Int) bool { return x.Cmp(bigM10) >= 0 && x.Cmp(big50) <= 0 },
	"int_is_even":           func(x *big.Int) bool { return x.Bit(0) == 0 }, // two's complement: correct for negatives
	"int_abs_le_10":         func(x *big.Int) bool { return x.CmpAbs(big10) <= 0 },
	"refinement_x_le_50":    func(x *big.Int) bool { return x.Cmp(big50) <= 0 },
}

var listProps = map[string]func([]*big.Int) bool{
	"list_no_consecutive_duplicates": func(l []*big.Int) bool {
		for i := 0; i+1 < len(l); i++ {
			if l[i].Cmp(l[i+1]) == 0 {
				return false
			}
		}
		return true
	},
	"list_sum_le_20": func(l []*big.Int) bool {
		sum := new(big.Int)
		for _, v := range l {
			sum.Add(sum, v)
		}
		return sum.Cmp(big20) <= 0
	},
}

var bytesProps = map[string]func([]byte) bool{
	"bytes_len_lt_5":     func(b []byte) bool { return len(b) < 5 },
	"bytes_no_zero_byte": func(b []byte) bool { return bytes.IndexByte(b, 0) < 0 },
}

func propName(v any) (string, error) {
	name, isStr := v.(string)
	if !isStr {
		return "", caseErr("unknown property %s", show(v))
	}
	return name, nil
}

// -----------------------------------------------------------------------------
// Case shapes
// -----------------------------------------------------------------------------

// shapes is the closed field set of each op (besides id and op).
var shapes = map[string][]string{
	"gen_primitive":      {"type", "seed", "count", "outputs"},
	"gen_bounded":        {"min", "max", "seed", "count", "outputs"},
	"gen_bytes":          {"min_len", "max_len", "seed", "count", "outputs"},
	"shrink_int":         {"initial", "target", "min", "max", "property", "expected_minimal", "expected_steps"},
	"shrink_refinement":  {"initial", "target", "min", "max", "property", "expected_minimal", "expected_steps"},
	"shrink_list":        {"initial", "elem_target", "property", "expected_minimal", "expected_steps"},
	"shrink_bytes":       {"initial", "property", "expected_minimal", "expected_steps"},
	"replay_consistency": {"seed", "property", "initial_fail", "expected_minimal"},
	"filter_exhausted":   {"max_retries", "verdict", "failure_id"},
	"failure_id_valid":   {"input", "valid"},
	"corpus_reject":      {"text"},
}

func fieldSetMatches(c map[string]any, op string) bool {
	if len(c) != len(shapes[op])+2 {
		return false
	}
	if _, ok := c["id"]; !ok {
		return false
	}
	if _, ok := c["op"]; !ok {
		return false
	}
	for _, f := range shapes[op] {
		if _, ok := c[f]; !ok {
			return false
		}
	}
	return true
}

// CorpusReport is the result of evaluating a corpus: how many cases were fully
// evaluated (match or not) and every problem found.
type CorpusReport struct {
	Evaluated int
	Problems  []string
}

// EvaluateCorpus evaluates every case and returns the problems (empty means the
// engine conforms).
func EvaluateCorpus(cases []any) []string {
	return EvaluateCorpusReport(cases, PBTV1CaseCount).Problems
}

// EvaluateCorpusReport is EvaluateCorpus with the evaluated count.  A negative
// expected skips the case-count check (for ad-hoc case lists in tests).
func EvaluateCorpusReport(cases []any, expected int) CorpusReport {
	var problems []string
	evaluated := 0
	if expected >= 0 && len(cases) != expected {
		problems = append(problems, fmt.Sprintf("pbt-v1: expected %d cases, got %d", expected, len(cases)))
	}
	if len(cases) > corpusMaxCases {
		problems = append(problems, fmt.Sprintf("pbt-v1: %d cases exceeds the cap of %d", len(cases), corpusMaxCases))
		cases = cases[:corpusMaxCases]
	}

	// Ids are compared as strings only: a non-string id is a reported problem.
	seenIDs := map[string]bool{}
	for _, raw := range cases {
		c, isMap := raw.(map[string]any)
		id, isStr := "", false
		if isMap {
			id, isStr = c["id"].(string)
		}
		if !isStr || id == "" || seenIDs[id] {
			problems = append(problems, "duplicate or missing case id")
			break
		}
		seenIDs[id] = true
	}

	opsSeen := map[string]bool{}
	registeredSeen := map[string]bool{}
	rejectedIDSeen := false

	for _, raw := range cases {
		c, isMap := raw.(map[string]any)
		if !isMap {
			problems = append(problems, fmt.Sprintf("case is not an object: %s", show(raw)))
			continue
		}
		cid, isStr := c["id"].(string)
		if !isStr || cid == "" {
			problems = append(problems, fmt.Sprintf("case missing a string id: %s", show(c)))
			continue
		}
		cid = clip(cid, 100)
		op, _ := c["op"].(string)
		if _, known := shapes[op]; !known {
			problems = append(problems, fmt.Sprintf("%s: unknown op %s", cid, show(c["op"])))
			continue
		}
		if !fieldSetMatches(c, op) {
			problems = append(problems, fmt.Sprintf("%s: fields %s != %v", cid, show(sortedKeys(c)), append([]string{"id", "op"}, shapes[op]...)))
			continue
		}
		found, ranOK, validID, registered := evaluateCase(cid, op, c)
		problems = append(problems, found...)
		if !ranOK {
			continue
		}
		if op == "failure_id_valid" {
			if registered {
				registeredSeen[validID] = true
			} else {
				rejectedIDSeen = true
			}
		}
		opsSeen[op] = true
		evaluated++
	}

	var missingOps []string
	for op := range shapes {
		if !opsSeen[op] {
			missingOps = append(missingOps, op)
		}
	}
	sort.Strings(missingOps)
	if len(missingOps) > 0 {
		problems = append(problems, fmt.Sprintf("pbt-v1: operations never exercised: %v", missingOps))
	}
	var missingIDs []string
	for _, id := range FailureIDs {
		if !registeredSeen[id] {
			missingIDs = append(missingIDs, id)
		}
	}
	if len(missingIDs) > 0 {
		problems = append(problems, fmt.Sprintf("pbt-v1: registered failure ids missing: %v", missingIDs))
	}
	if !rejectedIDSeen {
		problems = append(problems, "pbt-v1: failure_id_valid needs an unregistered-id rejection")
	}
	for i, p := range problems {
		problems[i] = clip(p, maxProblemLen)
	}
	return CorpusReport{Evaluated: evaluated, Problems: problems}
}

func sortedKeys(m map[string]any) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return keys
}

// evaluateCase decodes and checks one case.  It recovers from any panic (a
// malformed case or an engine that panicked) and reports it as a problem.
func evaluateCase(cid, op string, c map[string]any) (problems []string, ranOK bool, candidate string, registered bool) {
	defer func() {
		if r := recover(); r != nil {
			problems = []string{fmt.Sprintf("%s: %s could not be evaluated: panic: %s", cid, op, clip(safeString(r), maxFragmentLen))}
			ranOK = false
		}
	}()
	v := map[string]any{}
	for k, node := range c {
		if k == "id" || k == "op" {
			continue
		}
		d, err := decode(node)
		if err != nil {
			return []string{fmt.Sprintf("%s: %s could not be evaluated: %v", cid, op, err)}, false, "", false
		}
		v[k] = d
	}
	var found []string
	var err error
	switch op {
	case "gen_primitive", "gen_bounded", "gen_bytes":
		found, err = checkGen(cid, op, v)
	case "shrink_int", "shrink_refinement":
		found, err = checkShrinkInt(cid, op, v)
	case "shrink_list":
		found, err = checkShrinkList(cid, v)
	case "shrink_bytes":
		found, err = checkShrinkBytes(cid, v)
	case "replay_consistency":
		found, err = checkReplay(cid, v)
	case "filter_exhausted":
		found, err = checkFilterExhausted(cid, v)
	case "failure_id_valid":
		in, isStr := v["input"].(string)
		got := isStr && IsPBTFailureID(in)
		if !sameValue(got, v["valid"]) {
			found = []string{fmt.Sprintf("%s: failure id membership disagrees for %s", cid, show(v["input"]))}
		}
		candidate, registered = in, got
	default: // corpus_reject
		text, isStr := v["text"].(string)
		if !isStr {
			err = caseErr("text must be a string")
		} else if tfv.Validate(text) == nil {
			found = []string{fmt.Sprintf("%s: corpus_reject text was accepted by the strict parser", cid)}
		}
	}
	if err != nil {
		return []string{fmt.Sprintf("%s: %s could not be evaluated: %v", cid, op, err)}, false, "", false
	}
	return found, true, candidate, registered
}

func checkGen(cid, op string, v map[string]any) ([]string, error) {
	seed, err := seedOf(v["seed"])
	if err != nil {
		return nil, err
	}
	count, err := sizeOf(v["count"], "count")
	if err != nil {
		return nil, err
	}
	// The expected outputs are in hand already: a case whose list length is not
	// count cannot conform, so refuse it before generating anything.
	expected, isList := v["outputs"].([]any)
	if !isList || len(expected) != count {
		return nil, caseErr("outputs must be a list of exactly count (%d) values", count)
	}
	var got []any
	switch op {
	case "gen_primitive":
		name, isStr := v["type"].(string)
		if !isStr {
			return nil, caseErr("type must be a string")
		}
		got, err = GenPrimitive(seed, name, count)
	case "gen_bounded":
		lo, e1 := intOf(v["min"], "min")
		hi, e2 := intOf(v["max"], "max")
		if err = errors.Join(e1, e2); err != nil {
			return nil, err
		}
		// Each draw costs about as many bytes as the widest bound.
		width := (max(lo.BitLen(), hi.BitLen()) + 7) / 8
		if width < 1 {
			width = 1
		}
		if count*width > corpusMaxGenBytes {
			return nil, caseErr("count %d x %d-byte bounds exceeds the generation budget of %d bytes", count, width, corpusMaxGenBytes)
		}
		var ints []*big.Int
		ints, err = GenBounded(seed, lo, hi, count)
		got = bigsToAny(ints)
	default:
		minLen, e1 := sizeOf(v["min_len"], "min_len")
		maxLen, e2 := sizeOf(v["max_len"], "max_len")
		if err = errors.Join(e1, e2); err != nil {
			return nil, err
		}
		if count*maxLen > corpusMaxGenBytes {
			return nil, caseErr("count %d x max_len %d exceeds the generation budget of %d bytes", count, maxLen, corpusMaxGenBytes)
		}
		var raw [][]byte
		raw, err = GenBytes(seed, minLen, maxLen, count)
		got = bytesToAny(raw)
	}
	if err != nil {
		return nil, err
	}
	if !sameValue(got, v["outputs"]) {
		return []string{fmt.Sprintf("%s: %s outputs mismatch", cid, op)}, nil
	}
	return nil, nil
}

func checkShrinkInt(cid, op string, v map[string]any) ([]string, error) {
	name, err := propName(v["property"])
	if err != nil {
		return nil, err
	}
	prop, known := intProps[name]
	if !known {
		return nil, caseErr("unknown property %s for %s", show(name), op)
	}
	var ints [4]*big.Int
	for i, f := range []string{"initial", "target", "min", "max"} {
		if ints[i], err = intOf(v[f], f); err != nil {
			return nil, err
		}
	}
	initial, target, lo, hi := ints[0], ints[1], ints[2], ints[3]
	minimal, steps, err := ShrinkInt(initial, target, lo, hi, prop, 1000)
	if err != nil {
		return nil, err
	}
	var problems []string
	if !sameValue(minimal, v["expected_minimal"]) {
		problems = append(problems, fmt.Sprintf("%s: %s minimal mismatch: got %s, corpus %s", cid, op, show(minimal), show(v["expected_minimal"])))
	}
	if !sameValue(big.NewInt(int64(steps)), v["expected_steps"]) {
		problems = append(problems, fmt.Sprintf("%s: %s step count mismatch: got %d, corpus %s", cid, op, steps, show(v["expected_steps"])))
	}
	// Engine invariants, independent of the corpus's expected values.
	if prop(minimal) {
		problems = append(problems, fmt.Sprintf("%s: %s minimal %s does not falsify the property", cid, op, show(minimal)))
	}
	if !inBounds(minimal, lo, hi) {
		problems = append(problems, fmt.Sprintf("%s: %s minimal %s left [%s, %s]", cid, op, show(minimal), show(lo), show(hi)))
	}
	return problems, nil
}

func checkShrinkList(cid string, v map[string]any) ([]string, error) {
	name, err := propName(v["property"])
	if err != nil {
		return nil, err
	}
	prop, known := listProps[name]
	if !known {
		return nil, caseErr("unknown property %s for shrink_list", show(name))
	}
	items, isList := v["initial"].([]any)
	if !isList {
		return nil, caseErr("initial must be a list of ints")
	}
	initial := make([]*big.Int, len(items))
	for i, item := range items {
		if initial[i], err = intOf(item, "element"); err != nil {
			return nil, err
		}
	}
	elemTarget, err := intOf(v["elem_target"], "elem_target")
	if err != nil {
		return nil, err
	}
	minimal, steps, err := ShrinkList(initial, elemTarget, prop, 1000)
	if err != nil {
		return nil, err
	}
	var problems []string
	if !sameValue(bigsToAny(minimal), v["expected_minimal"]) {
		problems = append(problems, fmt.Sprintf("%s: shrink_list minimal mismatch: got %s, corpus %s", cid, Canon(minimal), Canon(v["expected_minimal"])))
	}
	if !sameValue(big.NewInt(int64(steps)), v["expected_steps"]) {
		problems = append(problems, fmt.Sprintf("%s: shrink_list steps mismatch: got %d, corpus %s", cid, steps, show(v["expected_steps"])))
	}
	if prop(minimal) {
		problems = append(problems, fmt.Sprintf("%s: shrink_list minimal %s does not falsify the property", cid, Canon(minimal)))
	}
	return problems, nil
}

func checkShrinkBytes(cid string, v map[string]any) ([]string, error) {
	name, err := propName(v["property"])
	if err != nil {
		return nil, err
	}
	prop, known := bytesProps[name]
	if !known {
		return nil, caseErr("unknown property %s for shrink_bytes", show(name))
	}
	initial, isBytes := v["initial"].([]byte)
	if !isBytes {
		return nil, caseErr("initial must be bytes")
	}
	minimal, steps, err := ShrinkBytes(initial, prop, 1000)
	if err != nil {
		return nil, err
	}
	var problems []string
	if !sameValue(minimal, v["expected_minimal"]) {
		problems = append(problems, fmt.Sprintf("%s: shrink_bytes minimal mismatch: got %s, corpus %s", cid, Canon(minimal), Canon(v["expected_minimal"])))
	}
	if !sameValue(big.NewInt(int64(steps)), v["expected_steps"]) {
		problems = append(problems, fmt.Sprintf("%s: shrink_bytes steps mismatch: got %d, corpus %s", cid, steps, show(v["expected_steps"])))
	}
	if prop(minimal) {
		problems = append(problems, fmt.Sprintf("%s: shrink_bytes minimal %s does not falsify the property", cid, Canon(minimal)))
	}
	return problems, nil
}

func checkReplay(cid string, v map[string]any) ([]string, error) {
	seed, err := seedOf(v["seed"])
	if err != nil {
		return nil, err
	}
	name, err := propName(v["property"])
	if err != nil {
		return nil, err
	}
	prop, known := intProps[name]
	if !known {
		return nil, caseErr("unknown property %s for replay_consistency", show(name))
	}
	initialFail, minimal, err := SimulateReplay(seed, prop)
	if err != nil {
		return nil, err
	}
	var problems []string
	if !sameValue(initialFail, v["initial_fail"]) {
		problems = append(problems, fmt.Sprintf("%s: replay initial_fail mismatch: got %s, corpus %s", cid, show(initialFail), show(v["initial_fail"])))
	}
	if !sameValue(minimal, v["expected_minimal"]) {
		problems = append(problems, fmt.Sprintf("%s: replay minimal mismatch: got %s, corpus %s", cid, show(minimal), show(v["expected_minimal"])))
	}
	return problems, nil
}

func checkFilterExhausted(cid string, v map[string]any) ([]string, error) {
	maxRetries, err := sizeOf(v["max_retries"], "max_retries")
	if err != nil {
		return nil, err
	}
	if maxRetries < 1 {
		return nil, caseErr("max_retries must be at least 1")
	}
	calls := 0
	never := func(*big.Int) bool { calls++; return false }
	gen := FilterN(Int(big.NewInt(0), big.NewInt(10)), never, maxRetries)
	verdict, err := CheckProperty(0, gen, func(*big.Int) bool { return true }, &Options{NumTests: 1, MaxShrinkSteps: 1000, CI: CIOn})
	if err != nil {
		return nil, err
	}
	var problems []string
	if !sameValue("fail", v["verdict"]) || verdict.Passed {
		problems = append(problems, fmt.Sprintf("%s: an unsatisfiable filter must be a fail verdict", cid))
	}
	if !sameValue(verdict.FailureID, v["failure_id"]) {
		problems = append(problems, fmt.Sprintf("%s: failure id %q, corpus %s", cid, verdict.FailureID, show(v["failure_id"])))
	}
	if calls != maxRetries {
		problems = append(problems, fmt.Sprintf("%s: filter sampled %d times, expected %d", cid, calls, maxRetries))
	}
	return problems, nil
}
