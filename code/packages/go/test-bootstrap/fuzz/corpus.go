package fuzz

import (
	"encoding/hex"
	"fmt"
	"math/big"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/test-bootstrap/internal/corpusjson"
	"github.com/adhithyan15/coding-adventures/code/packages/go/test-bootstrap/tfv"
)

// This file is the Go lane's production runner for the shared fuzz-v1 corpus
// (code/specs/fixtures/fuzz-v1/cases.json, VER06 section 8).  It takes the
// already-parsed JSON (ParseCorpus, or any []any tree), evaluates every case
// with THIS package's own exported engine functions and returns human-readable
// problems.  An empty list means "conforms".
//
// It does no file I/O (the package is capability-free); the test that reads the
// committed fixture is corpus_test.go.
//
// A runner that cannot report a problem is worse than none, so this one is
// strict (the same rules as the pbt runner, from the same internal package):
//
//   - a case has exactly the fields of its op, in canonical encodings (ints are
//     decimal strings, bytes are even-length lowercase hex);
//   - an unknown op or a malformed value is a PROBLEM, never a silent skip;
//   - every op appears, a failure id is accepted and one rejected, and the case
//     count equals FuzzV1CaseCount, so deleting cases is itself a problem;
//   - fixture-controlled sizes are capped (CWE-400): a hostile steps = 10^9 is a
//     reported problem, not a hang.
//
// Case encoding (JSON): {"kind":"int","value":"-12"}, {"kind":"bytes","hex":"00ff"},
// true/false, lists of those, and plain strings (endian, ids, frames).

// FuzzV1CaseCount is how many cases fuzz-v1 holds; a different count is a
// problem (a truncated list, or a corpus that grew without this lane being
// reviewed).
const FuzzV1CaseCount = 25

// CorpusMaxSize is the cap on every fixture-controlled size (shared).
const CorpusMaxSize = corpusjson.MaxSize

// corpusMaxCases bounds how many cases are evaluated.
const corpusMaxCases = 10_000

// ParseCorpus strictly decodes a fuzz-v1 document into its case list (see
// corpusjson.Parse): duplicate keys, trailing data, nesting deeper than 64 and
// documents over 8 MiB are refused; case shapes are left to EvaluateCorpus.
func ParseCorpus(data []byte) ([]any, error) { return corpusjson.Parse(data, "fuzz corpus") }

var shapes = map[string][]string{
	"mutate_bit_flip":        {"input", "bit_offset", "count", "output"},
	"mutate_byte_flip":       {"input", "byte_offset", "count", "output"},
	"mutate_arithmetic":      {"input", "byte_offset", "delta", "width", "endian", "output"},
	"mutate_boundary":        {"input", "byte_offset", "width", "val", "endian", "output"},
	"mutate_block_delete":    {"input", "offset", "length", "output"},
	"mutate_block_insert":    {"input", "offset", "insert_bytes", "output"},
	"mutate_block_duplicate": {"input", "src_offset", "length", "dst_offset", "output"},
	"mutate_splice":          {"target", "donor", "target_split", "donor_split", "output"},
	"mutate_sequence":        {"input", "seed", "steps", "output"},
	"corpus_admit":           {"existing_edges", "trace_edges", "admitted", "new_edges"},
	"crash_dedup":            {"crash_a_kind", "crash_a_frames", "crash_b_kind", "crash_b_frames", "is_duplicate"},
	"failure_id_valid":       {"input", "valid"},
	"private_reporting":      {"target", "failure_id", "raw_payload", "sig_hash", "tap_line", "contains_raw_payload"},
	"corpus_reject":          {"text"},
}

// The engine entry points the runner calls, as variables so the negative
// controls can sabotage each one (an unexported hook is not public API).
// tfvValidateReal is the real validator the hook starts with.
var tfvValidateReal = tfv.Validate

var (
	bitFlipFn        = BitFlipBig
	byteFlipFn       = ByteFlipBig
	byteArithmeticFn = ByteArithmeticBig
	boundaryInsertFn = BoundaryInsertBig
	blockDeleteFn    = BlockDeleteBig
	blockInsertFn    = BlockInsertBig
	blockDuplicateFn = BlockDuplicateBig
	spliceFn         = SpliceBig
	mutateSequenceFn = MutateSequence
	corpusAdmitFn    = CorpusAdmit
	isDuplicateFn    = IsDuplicateCrash
	isFailureIDFn    = IsFuzzFailureID
	formatTAPFailFn  = FormatTAPFailure
	tfvValidateFn    = tfvValidateReal
)

// CorpusReport is the result of evaluating a corpus: how many cases were fully
// evaluated (match or not) and every problem found.
type CorpusReport struct {
	Evaluated int
	Problems  []string
}

// EvaluateCorpus evaluates every case and returns the problems (empty means the
// engine conforms).
func EvaluateCorpus(cases []any) []string {
	return EvaluateCorpusReport(cases, FuzzV1CaseCount).Problems
}

func show(v any) string { return corpusjson.Show(v) }

func clip(s string, n int) string { return corpusjson.Clip(s, n) }

func caseErr(format string, args ...any) error { return corpusjson.CaseErr(format, args...) }

func fieldSetMatches(c map[string]any, op string) bool {
	if len(c) != len(shapes[op])+2 {
		return false
	}
	for _, k := range append([]string{"id", "op"}, shapes[op]...) {
		if _, ok := c[k]; !ok {
			return false
		}
	}
	return true
}

func sortedKeys(m map[string]any) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return keys
}

// EvaluateCorpusReport is EvaluateCorpus with the evaluated count.  A negative
// expected skips the case-count check (for ad-hoc case lists in tests).
func EvaluateCorpusReport(cases []any, expected int) CorpusReport {
	var problems []string
	evaluated := 0
	if expected >= 0 && len(cases) != expected {
		problems = append(problems, fmt.Sprintf("fuzz-v1: expected %d cases, got %d", expected, len(cases)))
	}
	if len(cases) > corpusMaxCases {
		problems = append(problems, fmt.Sprintf("fuzz-v1: %d cases exceeds the cap of %d", len(cases), corpusMaxCases))
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
	acceptedIDSeen, rejectedIDSeen := false, false

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
		found, ranOK, accepted := evaluateCase(cid, op, c)
		problems = append(problems, found...)
		if !ranOK {
			continue
		}
		if op == "failure_id_valid" {
			acceptedIDSeen = acceptedIDSeen || accepted
			rejectedIDSeen = rejectedIDSeen || !accepted
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
		problems = append(problems, fmt.Sprintf("fuzz-v1: operations never exercised: %v", missingOps))
	}
	if !acceptedIDSeen {
		problems = append(problems, "fuzz-v1: failure_id_valid needs a registered-id acceptance")
	}
	if !rejectedIDSeen {
		problems = append(problems, "fuzz-v1: failure_id_valid needs an unregistered-id rejection")
	}
	var unregistered []string
	for _, id := range FuzzFailureIDs {
		if !isFailureIDFn(id) {
			unregistered = append(unregistered, id)
		}
	}
	if len(unregistered) > 0 {
		problems = append(problems, fmt.Sprintf("fuzz-v1: engine rejects registered ids %v", unregistered))
	}
	for i, p := range problems {
		problems[i] = clip(p, corpusjson.MaxProblemLen)
	}
	return CorpusReport{Evaluated: evaluated, Problems: problems}
}

// evaluateCase decodes and checks one case.  It recovers from any panic (a
// malformed case or an engine that panicked) and reports it as a problem.
func evaluateCase(cid, op string, c map[string]any) (problems []string, ranOK, accepted bool) {
	defer func() {
		if r := recover(); r != nil {
			problems = []string{fmt.Sprintf("%s: %s could not be evaluated: panic: %s", cid, op, clip(fmt.Sprint(r), corpusjson.MaxFragmentLen))}
			ranOK = false
		}
	}()
	v := map[string]any{}
	for k, node := range c {
		if k == "id" || k == "op" {
			continue
		}
		d, err := corpusjson.Decode(node)
		if err != nil {
			return []string{fmt.Sprintf("%s: %s could not be evaluated: %v", cid, op, err)}, false, false
		}
		v[k] = d
	}
	var found []string
	var err error
	switch op {
	case "corpus_admit":
		found, err = checkCorpusAdmit(cid, v)
	case "crash_dedup":
		found, err = checkCrashDedup(cid, v)
	case "failure_id_valid":
		in, isStr := v["input"].(string)
		accepted = isStr && isFailureIDFn(in)
		if !corpusjson.Same(accepted, v["valid"]) {
			found = []string{fmt.Sprintf("%s: failure id membership disagrees for %s", cid, show(v["input"]))}
		}
	case "private_reporting":
		found, err = checkPrivateReporting(cid, v)
	case "corpus_reject":
		text, isStr := v["text"].(string)
		if !isStr {
			err = caseErr("text must be a string")
		} else if tfvValidateFn(text) == nil {
			found = []string{fmt.Sprintf("%s: corpus_reject text was accepted by the strict parser", cid)}
		}
	default: // the nine byte-in, byte-out mutators
		found, err = checkMutator(cid, op, v)
	}
	if err != nil {
		return []string{fmt.Sprintf("%s: %s could not be evaluated: %v", cid, op, err)}, false, false
	}
	return found, true, accepted
}

// -----------------------------------------------------------------------------
// Typed field access
// -----------------------------------------------------------------------------

func bytesOf(v any, what string) ([]byte, error) {
	b, ok := v.([]byte)
	if !ok {
		return nil, caseErr("%s must be bytes", what)
	}
	return b, nil
}

func textOf(v any, what string) (string, error) {
	s, ok := v.(string)
	if !ok {
		return "", caseErr("%s must be a string", what)
	}
	return s, nil
}

func boolOf(v any, what string) (bool, error) {
	b, ok := v.(bool)
	if !ok {
		return false, caseErr("%s must be a bool", what)
	}
	return b, nil
}

func intOf(v any, what string) (*big.Int, error) {
	b, ok := v.(*big.Int)
	if !ok {
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

// widthOf maps a corpus width to an int; anything outside [0, 64] becomes -1,
// which the mutators treat as "not one of 1, 2, 4, 8" (input unchanged).
func widthOf(v any) (int, error) {
	b, err := intOf(v, "width")
	if err != nil {
		return 0, err
	}
	if !b.IsInt64() || b.Int64() < 0 || b.Int64() > 64 {
		return -1, nil
	}
	return int(b.Int64()), nil
}

func endianOf(v any) (Endian, error) {
	s, err := textOf(v, "endian")
	if err != nil {
		return 0, err
	}
	switch s {
	case "little":
		return LittleEndian, nil
	case "big":
		return BigEndian, nil
	}
	return 0, caseErr("endian must be 'little' or 'big', got %s", show(s))
}

func textsOf(v any, what string) ([]string, error) {
	list, ok := v.([]any)
	if !ok {
		return nil, caseErr("%s must be a list of strings", what)
	}
	out := make([]string, len(list))
	for i, item := range list {
		s, isStr := item.(string)
		if !isStr {
			return nil, caseErr("%s must be a list of strings", what)
		}
		out[i] = s
	}
	return out, nil
}

// edgesOf reads a list of edge ids; each must fit in a uint64.
func edgesOf(v any, what string) ([]uint64, error) {
	list, ok := v.([]any)
	if !ok {
		return nil, caseErr("%s must be a list of ints", what)
	}
	out := make([]uint64, len(list))
	for i, item := range list {
		b, err := intOf(item, what)
		if err != nil {
			return nil, caseErr("%s must be a list of ints", what)
		}
		if !b.IsUint64() {
			return nil, caseErr("%s element %s outside [0, 2^64)", what, show(b))
		}
		out[i] = b.Uint64()
	}
	return out, nil
}

// -----------------------------------------------------------------------------
// Per-op checks
// -----------------------------------------------------------------------------

func checkMutator(cid, op string, v map[string]any) ([]string, error) {
	var got []byte
	in, e0 := bytesOf(v["input"], "input")
	var err error
	// each op reads its own fields; the first error wins
	switch op {
	case "mutate_bit_flip", "mutate_byte_flip":
		off, count := "bit_offset", "count"
		if op == "mutate_byte_flip" {
			off = "byte_offset"
		}
		o, e1 := intOf(v[off], off)
		n, e2 := intOf(v[count], count)
		if err = firstErr(e0, e1, e2); err == nil {
			if op == "mutate_bit_flip" {
				got = bitFlipFn(in, o, n)
			} else {
				got = byteFlipFn(in, o, n)
			}
		}
	case "mutate_arithmetic":
		o, e1 := intOf(v["byte_offset"], "byte_offset")
		d, e2 := intOf(v["delta"], "delta")
		w, e3 := widthOf(v["width"])
		en, e4 := endianOf(v["endian"])
		if err = firstErr(e0, e1, e2, e3, e4); err == nil {
			got = byteArithmeticFn(in, o, d, w, en)
		}
	case "mutate_boundary":
		o, e1 := intOf(v["byte_offset"], "byte_offset")
		w, e2 := widthOf(v["width"])
		val, e3 := intOf(v["val"], "val")
		en, e4 := endianOf(v["endian"])
		if err = firstErr(e0, e1, e2, e3, e4); err == nil {
			got = boundaryInsertFn(in, o, w, val, en)
		}
	case "mutate_block_delete":
		o, e1 := intOf(v["offset"], "offset")
		l, e2 := intOf(v["length"], "length")
		if err = firstErr(e0, e1, e2); err == nil {
			got = blockDeleteFn(in, o, l)
		}
	case "mutate_block_insert":
		o, e1 := intOf(v["offset"], "offset")
		ins, e2 := bytesOf(v["insert_bytes"], "insert_bytes")
		if err = firstErr(e0, e1, e2); err == nil {
			got = blockInsertFn(in, o, ins)
		}
	case "mutate_block_duplicate":
		s, e1 := intOf(v["src_offset"], "src_offset")
		l, e2 := intOf(v["length"], "length")
		d, e3 := intOf(v["dst_offset"], "dst_offset")
		if err = firstErr(e0, e1, e2, e3); err == nil {
			got = blockDuplicateFn(in, s, l, d)
		}
	case "mutate_splice":
		tgt, e1 := bytesOf(v["target"], "target")
		donor, e2 := bytesOf(v["donor"], "donor")
		ts, e3 := intOf(v["target_split"], "target_split")
		ds, e4 := intOf(v["donor_split"], "donor_split")
		if err = firstErr(e1, e2, e3, e4); err == nil {
			got = spliceFn(tgt, donor, ts, ds)
			problems := compare(cid, op, got, v)
			if len(got) != clampSplit(satInt64(ts), len(tgt))+len(donor)-clampSplit(satInt64(ds), len(donor)) {
				problems = append(problems, fmt.Sprintf("%s: splice length is not prefix + suffix", cid))
			}
			return problems, nil
		}
	default: // mutate_sequence
		seed, e1 := intOf(v["seed"], "seed")
		steps, e2 := sizeOf(v["steps"], "steps")
		if err = firstErr(e0, e1, e2); err == nil {
			got, err = mutateSequenceFn(in, low64(seed), steps, MaxInputBytes)
		}
	}
	if err != nil {
		return nil, err
	}
	problems := compare(cid, op, got, v)
	// Engine invariant, independent of the corpus's expected bytes.
	switch op {
	case "mutate_bit_flip", "mutate_byte_flip", "mutate_arithmetic", "mutate_boundary":
		if len(got) != len(in) {
			problems = append(problems, fmt.Sprintf("%s: %s changed the length of its input", cid, op))
		}
	}
	return problems, nil
}

func firstErr(errs ...error) error {
	for _, e := range errs {
		if e != nil {
			return e
		}
	}
	return nil
}

func compare(cid, op string, got []byte, v map[string]any) []string {
	want, ok := v["output"].([]byte)
	if !ok {
		return []string{fmt.Sprintf("%s: %s output must be bytes", cid, op)}
	}
	if corpusjson.Same(got, want) {
		return nil
	}
	return []string{fmt.Sprintf("%s: %s expected %s, got %s", cid, op, clip(hex.EncodeToString(want), corpusjson.MaxFragmentLen), clip(hex.EncodeToString(got), corpusjson.MaxFragmentLen))}
}

func checkCorpusAdmit(cid string, v map[string]any) ([]string, error) {
	existing, e1 := edgesOf(v["existing_edges"], "existing_edges")
	trace, e2 := edgesOf(v["trace_edges"], "trace_edges")
	admitted, e3 := boolOf(v["admitted"], "admitted")
	newEdges, e4 := intOf(v["new_edges"], "new_edges")
	if err := firstErr(e1, e2, e3, e4); err != nil {
		return nil, err
	}
	gotAdmitted, gotNew := corpusAdmitFn(existing, trace)
	var problems []string
	if gotAdmitted != admitted {
		problems = append(problems, fmt.Sprintf("%s: corpus_admit admitted %t, corpus %t", cid, gotAdmitted, admitted))
	}
	if !corpusjson.Same(big.NewInt(int64(gotNew)), newEdges) {
		problems = append(problems, fmt.Sprintf("%s: corpus_admit new_edges %d, corpus %s", cid, gotNew, show(newEdges)))
	}
	// Invariant: admission is exactly "some trace edge is unseen".
	have := make(map[uint64]struct{}, len(existing))
	for _, e := range existing {
		have[e] = struct{}{}
	}
	novel := false
	for _, e := range trace {
		if _, seen := have[e]; !seen {
			novel = true
			break
		}
	}
	if gotAdmitted != novel {
		problems = append(problems, fmt.Sprintf("%s: corpus_admit disagrees with set difference", cid))
	}
	return problems, nil
}

func checkCrashDedup(cid string, v map[string]any) ([]string, error) {
	ka, e1 := textOf(v["crash_a_kind"], "crash_a_kind")
	fa, e2 := textsOf(v["crash_a_frames"], "crash_a_frames")
	kb, e3 := textOf(v["crash_b_kind"], "crash_b_kind")
	fb, e4 := textsOf(v["crash_b_frames"], "crash_b_frames")
	want, e5 := boolOf(v["is_duplicate"], "is_duplicate")
	if err := firstErr(e1, e2, e3, e4, e5); err != nil {
		return nil, err
	}
	if got := isDuplicateFn(ka, fa, kb, fb); got != want {
		return []string{fmt.Sprintf("%s: crash_dedup is_duplicate %t, corpus %t", cid, got, want)}, nil
	}
	return nil, nil
}

func checkPrivateReporting(cid string, v map[string]any) ([]string, error) {
	raw, e1 := bytesOf(v["raw_payload"], "raw_payload")
	target, e2 := textOf(v["target"], "target")
	fid, e3 := textOf(v["failure_id"], "failure_id")
	sig, e4 := textOf(v["sig_hash"], "sig_hash")
	tap, e5 := textOf(v["tap_line"], "tap_line")
	wantLeak, e6 := boolOf(v["contains_raw_payload"], "contains_raw_payload")
	if err := firstErr(e1, e2, e3, e4, e5, e6); err != nil {
		return nil, err
	}
	line, err := formatTAPFailFn(1, target, fid, sig)
	if err != nil {
		return nil, caseErr("format_tap_failure refused: %v", err)
	}
	var problems []string
	if line != tap {
		problems = append(problems, fmt.Sprintf("%s: tap_line expected %s, got %s", cid, clip(tap, corpusjson.MaxFragmentLen), clip(line, corpusjson.MaxFragmentLen)))
	}
	leaked := len(raw) > 0 && strings.Contains(line, hex.EncodeToString(raw))
	if leaked != wantLeak {
		problems = append(problems, fmt.Sprintf("%s: privacy verdict wrong, payload leaked: %t", cid, leaked))
	}
	if strings.ContainsAny(line, "\r\n") {
		problems = append(problems, fmt.Sprintf("%s: failure line spans more than one line", cid))
	}
	return problems, nil
}
