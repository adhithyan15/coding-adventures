package huffmantree_test

// test_huffman_tree.go — Comprehensive tests for DT27: Huffman Tree
//
// Tests cover:
//   - Construction from various frequency distributions
//   - Tie-breaking determinism
//   - Code table generation (CodeTable, CodeFor)
//   - Canonical code table (CanonicalCodeTable)
//   - Encoding and decoding round-trips (DecodeAll)
//   - Inspection methods (Weight, Depth, SymbolCount, Leaves)
//   - IsValid structural check
//   - Edge cases (single symbol, two symbols, identical frequencies)
//   - Error handling (empty input, zero/negative frequencies, exhausted stream)

import (
	"fmt"
	"sort"
	"strings"
	"testing"

	huffmantree "github.com/adhithyan15/coding-adventures/code/packages/go/huffman-tree"
)

// ─── Helpers ──────────────────────────────────────────────────────────────────

// wp is a shorthand constructor for WeightPair slices in tests.
func wp(symbol, freq int) huffmantree.WeightPair {
	return huffmantree.WeightPair{Symbol: symbol, Frequency: freq}
}

// mustBuild panics if Build returns an error. Used in tests where the input
// is known-valid, keeping test bodies concise.
func mustBuild(t *testing.T, weights []huffmantree.WeightPair) *huffmantree.HuffmanTree {
	t.Helper()
	tree, err := huffmantree.Build(weights)
	if err != nil {
		t.Fatalf("Build failed: %v", err)
	}
	return tree
}

// isPrefixFree verifies that no code in the map is a prefix of any other.
// This is the fundamental correctness property of a Huffman code.
func isPrefixFree(t *testing.T, table map[int]string) {
	t.Helper()
	codes := make([]string, 0, len(table))
	for _, c := range table {
		codes = append(codes, c)
	}
	for i, c1 := range codes {
		for j, c2 := range codes {
			if i != j && strings.HasPrefix(c2, c1) {
				t.Errorf("prefix-free violation: %q is a prefix of %q", c1, c2)
			}
		}
	}
}

// encode encodes a slice of symbols using the given code table.
func encode(symbols []int, table map[int]string) string {
	var sb strings.Builder
	for _, s := range symbols {
		sb.WriteString(table[s])
	}
	return sb.String()
}

// ─── Construction: Build ──────────────────────────────────────────────────────

func TestBuild_SingleSymbol(t *testing.T) {
	// A single symbol produces a tree with one leaf at the root.
	// weight = freq; symbolCount = 1.
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 5)})
	if got := huffmantree.SymbolCount(tree); got != 1 {
		t.Errorf("SymbolCount = %d, want 1", got)
	}
	if got := huffmantree.Weight(tree); got != 5 {
		t.Errorf("Weight = %d, want 5", got)
	}
}

func TestBuild_TwoSymbols(t *testing.T) {
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 3), wp(66, 1)})
	if got := huffmantree.SymbolCount(tree); got != 2 {
		t.Errorf("SymbolCount = %d, want 2", got)
	}
	if got := huffmantree.Weight(tree); got != 4 {
		t.Errorf("Weight = %d, want 4", got)
	}
}

func TestBuild_ThreeSymbols_AAABBC(t *testing.T) {
	// Classic example: A=3, B=2, C=1 → total weight 6
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 3), wp(66, 2), wp(67, 1)})
	if got := huffmantree.SymbolCount(tree); got != 3 {
		t.Errorf("SymbolCount = %d, want 3", got)
	}
	if got := huffmantree.Weight(tree); got != 6 {
		t.Errorf("Weight = %d, want 6", got)
	}
}

func TestBuild_EmptyWeightsReturnsError(t *testing.T) {
	_, err := huffmantree.Build([]huffmantree.WeightPair{})
	if err == nil {
		t.Fatal("expected error for empty weights, got nil")
	}
	if !strings.Contains(err.Error(), "empty") {
		t.Errorf("error %q does not contain 'empty'", err.Error())
	}
}

func TestBuild_ZeroFrequencyReturnsError(t *testing.T) {
	_, err := huffmantree.Build([]huffmantree.WeightPair{wp(65, 0)})
	if err == nil {
		t.Fatal("expected error for zero frequency, got nil")
	}
	if !strings.Contains(err.Error(), "positive") {
		t.Errorf("error %q does not contain 'positive'", err.Error())
	}
}

func TestBuild_NegativeFrequencyReturnsError(t *testing.T) {
	_, err := huffmantree.Build([]huffmantree.WeightPair{wp(65, -1)})
	if err == nil {
		t.Fatal("expected error for negative frequency, got nil")
	}
	if !strings.Contains(err.Error(), "positive") {
		t.Errorf("error %q does not contain 'positive'", err.Error())
	}
}

func TestBuild_IsValidAfterBuild(t *testing.T) {
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 3), wp(66, 2), wp(67, 1)})
	if !huffmantree.IsValid(tree) {
		t.Error("IsValid returned false for a freshly built tree")
	}
}

func TestBuild_LargeAlphabet(t *testing.T) {
	// 256-symbol alphabet: symbol i has frequency i+1.
	weights := make([]huffmantree.WeightPair, 256)
	for i := range weights {
		weights[i] = wp(i, i+1)
	}
	tree := mustBuild(t, weights)
	if got := huffmantree.SymbolCount(tree); got != 256 {
		t.Errorf("SymbolCount = %d, want 256", got)
	}
	if !huffmantree.IsValid(tree) {
		t.Error("IsValid returned false for 256-symbol tree")
	}
}

// ─── CodeTable ────────────────────────────────────────────────────────────────

func TestCodeTable_FrequentSymbolGetsShorterCode(t *testing.T) {
	// A (freq=3) should get a shorter code than B (freq=2) and C (freq=1).
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 3), wp(66, 2), wp(67, 1)})
	table := huffmantree.CodeTable(tree)
	if len(table[65]) >= len(table[66]) {
		t.Errorf("A code length %d should be < B code length %d", len(table[65]), len(table[66]))
	}
	if len(table[66]) > len(table[67]) {
		t.Errorf("B code length %d should be <= C code length %d", len(table[66]), len(table[67]))
	}
}

func TestCodeTable_SingleSymbolCodeIsZero(t *testing.T) {
	// Convention: single-symbol tree → code "0".
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 1)})
	table := huffmantree.CodeTable(tree)
	if got := table[65]; got != "0" {
		t.Errorf("single-symbol code = %q, want \"0\"", got)
	}
}

func TestCodeTable_AllCodesPrefixFree(t *testing.T) {
	// Prefix-free is the fundamental correctness property.
	weights := make([]huffmantree.WeightPair, 10)
	for i := range weights {
		weights[i] = wp(i, i+1)
	}
	tree := mustBuild(t, weights)
	table := huffmantree.CodeTable(tree)
	isPrefixFree(t, table)
}

func TestCodeTable_AAABBC_ExactCodes(t *testing.T) {
	// AAABBC: A=3, B=2, C=1.
	//
	// Construction trace:
	//   Heap initial: C(w=1,leaf,sym=67), B(w=2,leaf,sym=66), A(w=3,leaf,sym=65)
	//   Step 1: pop C (priority (1,0,67,-1)), pop B (priority (2,0,66,-1))
	//           → internal_0: weight=3, left=C, right=B, order=0
	//           → priority (3,1,-1,0)
	//   Step 2: heap has A(3,0,65,-1) and internal_0(3,1,-1,0)
	//           → A pops first (leaf before internal at equal weight)
	//           → internal_0 pops second
	//           → root: weight=6, left=A, right=internal_0
	//
	// Code table: A → "0", C → "10", B → "11"
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 3), wp(66, 2), wp(67, 1)})
	table := huffmantree.CodeTable(tree)
	if table[65] != "0" {
		t.Errorf("A code = %q, want \"0\"", table[65])
	}
	if table[67] != "10" {
		t.Errorf("C code = %q, want \"10\"", table[67])
	}
	if table[66] != "11" {
		t.Errorf("B code = %q, want \"11\"", table[66])
	}
}

// ─── CodeFor ──────────────────────────────────────────────────────────────────

func TestCodeFor_ExistingSymbol(t *testing.T) {
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 3), wp(66, 2), wp(67, 1)})
	table := huffmantree.CodeTable(tree)

	for _, sym := range []int{65, 66, 67} {
		code, ok := huffmantree.CodeFor(tree, sym)
		if !ok {
			t.Errorf("CodeFor(%d) returned false", sym)
		}
		if code != table[sym] {
			t.Errorf("CodeFor(%d) = %q, want %q", sym, code, table[sym])
		}
	}
}

func TestCodeFor_MissingSymbolReturnsFalse(t *testing.T) {
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 3), wp(66, 2)})
	_, ok := huffmantree.CodeFor(tree, 99)
	if ok {
		t.Error("CodeFor(99) should return false for missing symbol")
	}
}

func TestCodeFor_SingleSymbol(t *testing.T) {
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 5)})
	code, ok := huffmantree.CodeFor(tree, 65)
	if !ok {
		t.Error("CodeFor(65) returned false")
	}
	if code != "0" {
		t.Errorf("CodeFor(65) = %q, want \"0\"", code)
	}
}

// ─── CanonicalCodeTable ───────────────────────────────────────────────────────

func TestCanonicalCodeTable_AAABBC(t *testing.T) {
	// AAABBC: A=3→len1, B=2→len2, C=1→len2
	// sorted by (len, sym): A(1), B(2), C(2)
	// A → "0", B → "10", C → "11"
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 3), wp(66, 2), wp(67, 1)})
	canonical := huffmantree.CanonicalCodeTable(tree)
	if canonical[65] != "0" {
		t.Errorf("canonical A = %q, want \"0\"", canonical[65])
	}
	if canonical[66] != "10" {
		t.Errorf("canonical B = %q, want \"10\"", canonical[66])
	}
	if canonical[67] != "11" {
		t.Errorf("canonical C = %q, want \"11\"", canonical[67])
	}
}

func TestCanonicalCodeTable_LengthsMatchRegular(t *testing.T) {
	// Canonical codes must have the SAME lengths as regular tree-walk codes.
	weights := make([]huffmantree.WeightPair, 8)
	for i := range weights {
		weights[i] = wp(i, i+1)
	}
	tree := mustBuild(t, weights)
	regular := huffmantree.CodeTable(tree)
	canonical := huffmantree.CanonicalCodeTable(tree)
	for sym, rc := range regular {
		cc := canonical[sym]
		if len(rc) != len(cc) {
			t.Errorf("sym %d: regular len=%d, canonical len=%d", sym, len(rc), len(cc))
		}
	}
}

func TestCanonicalCodeTable_SingleSymbol(t *testing.T) {
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 5)})
	canonical := huffmantree.CanonicalCodeTable(tree)
	if canonical[65] != "0" {
		t.Errorf("canonical single symbol = %q, want \"0\"", canonical[65])
	}
}

func TestCanonicalCodeTable_PrefixFree(t *testing.T) {
	weights := make([]huffmantree.WeightPair, 10)
	for i := range weights {
		weights[i] = wp(i, i+1)
	}
	tree := mustBuild(t, weights)
	canonical := huffmantree.CanonicalCodeTable(tree)
	isPrefixFree(t, canonical)
}

// ─── DecodeAll ────────────────────────────────────────────────────────────────

func TestDecodeAll_SingleSymbolRoundTrip(t *testing.T) {
	// Single-symbol tree: each symbol encodes as "0".
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 5)})
	table := huffmantree.CodeTable(tree)
	symbols := []int{65, 65, 65}
	bits := encode(symbols, table)
	decoded, err := huffmantree.DecodeAll(tree, bits, 3)
	if err != nil {
		t.Fatalf("DecodeAll error: %v", err)
	}
	if len(decoded) != 3 || decoded[0] != 65 || decoded[1] != 65 || decoded[2] != 65 {
		t.Errorf("decoded = %v, want [65 65 65]", decoded)
	}
}

func TestDecodeAll_AAABBC_RoundTrip(t *testing.T) {
	// Standard AAABBC round-trip.
	symbols := []int{65, 65, 65, 66, 66, 67}
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 3), wp(66, 2), wp(67, 1)})
	table := huffmantree.CodeTable(tree)
	bits := encode(symbols, table)
	decoded, err := huffmantree.DecodeAll(tree, bits, len(symbols))
	if err != nil {
		t.Fatalf("DecodeAll error: %v", err)
	}
	for i, s := range symbols {
		if decoded[i] != s {
			t.Errorf("decoded[%d] = %d, want %d", i, decoded[i], s)
		}
	}
}

func TestDecodeAll_AllByteValues(t *testing.T) {
	// All 256 byte values round-trip correctly.
	weights := make([]huffmantree.WeightPair, 256)
	for i := range weights {
		weights[i] = wp(i, i+1)
	}
	tree := mustBuild(t, weights)
	table := huffmantree.CodeTable(tree)
	symbols := make([]int, 256)
	for i := range symbols {
		symbols[i] = i
	}
	bits := encode(symbols, table)
	decoded, err := huffmantree.DecodeAll(tree, bits, 256)
	if err != nil {
		t.Fatalf("DecodeAll error: %v", err)
	}
	for i, s := range symbols {
		if decoded[i] != s {
			t.Errorf("decoded[%d] = %d, want %d", i, decoded[i], s)
		}
	}
}

func TestDecodeAll_ExhaustedStreamReturnsError(t *testing.T) {
	// Requesting more symbols than bits available must return an error.
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 3), wp(66, 2), wp(67, 1)})
	_, err := huffmantree.DecodeAll(tree, "0", 5) // only enough bits for 1 symbol
	if err == nil {
		t.Fatal("expected error for exhausted stream, got nil")
	}
	if !strings.Contains(err.Error(), "exhausted") {
		t.Errorf("error %q does not contain 'exhausted'", err.Error())
	}
}

func TestDecodeAll_TwoSymbols(t *testing.T) {
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 3), wp(66, 1)})
	table := huffmantree.CodeTable(tree)
	symbols := []int{65, 66, 65}
	bits := encode(symbols, table)
	decoded, err := huffmantree.DecodeAll(tree, bits, len(symbols))
	if err != nil {
		t.Fatalf("DecodeAll error: %v", err)
	}
	for i, s := range symbols {
		if decoded[i] != s {
			t.Errorf("decoded[%d] = %d, want %d", i, decoded[i], s)
		}
	}
}

// ─── Inspection: Weight, Depth, SymbolCount, Leaves ──────────────────────────

func TestWeight_Single(t *testing.T) {
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 7)})
	if got := huffmantree.Weight(tree); got != 7 {
		t.Errorf("Weight = %d, want 7", got)
	}
}

func TestWeight_Multiple(t *testing.T) {
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 3), wp(66, 2), wp(67, 1)})
	if got := huffmantree.Weight(tree); got != 6 {
		t.Errorf("Weight = %d, want 6", got)
	}
}

func TestDepth_Single(t *testing.T) {
	// Single leaf at root → depth 0.
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 1)})
	if got := huffmantree.Depth(tree); got != 0 {
		t.Errorf("Depth = %d, want 0", got)
	}
}

func TestDepth_TwoSymbols(t *testing.T) {
	// Two leaves under root → depth 1.
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 3), wp(66, 1)})
	if got := huffmantree.Depth(tree); got != 1 {
		t.Errorf("Depth = %d, want 1", got)
	}
}

func TestDepth_ThreeSymbols(t *testing.T) {
	// AAABBC: A at depth 1, B and C at depth 2 → max depth 2.
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 3), wp(66, 2), wp(67, 1)})
	if got := huffmantree.Depth(tree); got != 2 {
		t.Errorf("Depth = %d, want 2", got)
	}
}

func TestSymbolCount(t *testing.T) {
	weights := make([]huffmantree.WeightPair, 10)
	for i := range weights {
		weights[i] = wp(i, i+1)
	}
	tree := mustBuild(t, weights)
	if got := huffmantree.SymbolCount(tree); got != 10 {
		t.Errorf("SymbolCount = %d, want 10", got)
	}
}

func TestLeaves_Order(t *testing.T) {
	// All three symbols must appear; order is left-to-right in-order.
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 3), wp(66, 2), wp(67, 1)})
	leaves := huffmantree.Leaves(tree)
	if len(leaves) != 3 {
		t.Fatalf("len(Leaves) = %d, want 3", len(leaves))
	}
	syms := map[int]bool{}
	for _, le := range leaves {
		syms[le.Symbol] = true
	}
	for _, s := range []int{65, 66, 67} {
		if !syms[s] {
			t.Errorf("symbol %d missing from Leaves", s)
		}
	}
}

func TestLeaves_SingleSymbol(t *testing.T) {
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 5)})
	leaves := huffmantree.Leaves(tree)
	if len(leaves) != 1 {
		t.Fatalf("len(Leaves) = %d, want 1", len(leaves))
	}
	if leaves[0].Symbol != 65 || leaves[0].Code != "0" {
		t.Errorf("leaf = %+v, want {65 \"0\"}", leaves[0])
	}
}

// ─── Tie-breaking determinism ─────────────────────────────────────────────────

func TestTieBreaking_EqualWeights_IsValid(t *testing.T) {
	// Four symbols with equal weight → deterministic, valid tree.
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 1), wp(66, 1), wp(67, 1), wp(68, 1)})
	if !huffmantree.IsValid(tree) {
		t.Error("IsValid returned false for equal-weight tree")
	}
	if got := huffmantree.SymbolCount(tree); got != 4 {
		t.Errorf("SymbolCount = %d, want 4", got)
	}
	if got := huffmantree.Weight(tree); got != 4 {
		t.Errorf("Weight = %d, want 4", got)
	}
}

func TestTieBreaking_EqualWeights_Deterministic(t *testing.T) {
	// Same input → same code table on every call.
	weights := make([]huffmantree.WeightPair, 8)
	for i := range weights {
		weights[i] = wp(i, 1)
	}
	tree1 := mustBuild(t, weights)
	tree2 := mustBuild(t, weights)
	t1 := huffmantree.CodeTable(tree1)
	t2 := huffmantree.CodeTable(tree2)
	for sym, c1 := range t1 {
		if c2 := t2[sym]; c1 != c2 {
			t.Errorf("sym %d: tree1=%q tree2=%q (non-deterministic)", sym, c1, c2)
		}
	}
}

func TestTieBreaking_LowerSymbolGetsLeftCode(t *testing.T) {
	// Two leaves with the same weight: lower symbol (65) pops first → left="0".
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 1), wp(66, 1)})
	table := huffmantree.CodeTable(tree)
	if len(table[65]) != 1 {
		t.Errorf("A code length = %d, want 1", len(table[65]))
	}
	if len(table[66]) != 1 {
		t.Errorf("B code length = %d, want 1", len(table[66]))
	}
	// Lower symbol wins the min-heap tie → pops first → becomes left child → "0".
	if table[65] != "0" {
		t.Errorf("A code = %q, want \"0\" (lower symbol should be left)", table[65])
	}
	if table[66] != "1" {
		t.Errorf("B code = %q, want \"1\"", table[66])
	}
}

// ─── IsValid ──────────────────────────────────────────────────────────────────

func TestIsValid_Valid(t *testing.T) {
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 3), wp(66, 2), wp(67, 1)})
	if !huffmantree.IsValid(tree) {
		t.Error("IsValid returned false for valid tree")
	}
}

func TestIsValid_LargeTree(t *testing.T) {
	weights := make([]huffmantree.WeightPair, 50)
	for i := range weights {
		weights[i] = wp(i, i+1)
	}
	tree := mustBuild(t, weights)
	if !huffmantree.IsValid(tree) {
		t.Error("IsValid returned false for large valid tree")
	}
}

// ─── Extra edge cases ─────────────────────────────────────────────────────────

func TestBuild_AllSameFrequency(t *testing.T) {
	// 5 symbols all with frequency=1.  Should still produce a valid tree.
	weights := []huffmantree.WeightPair{wp(10, 1), wp(20, 1), wp(30, 1), wp(40, 1), wp(50, 1)}
	tree := mustBuild(t, weights)
	if !huffmantree.IsValid(tree) {
		t.Error("IsValid returned false for uniform-frequency tree")
	}
	if got := huffmantree.SymbolCount(tree); got != 5 {
		t.Errorf("SymbolCount = %d, want 5", got)
	}
}

func TestDecodeAll_ExactSymbolCount(t *testing.T) {
	// DecodeAll must return exactly count symbols, no more, no less.
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 3), wp(66, 2), wp(67, 1)})
	table := huffmantree.CodeTable(tree)
	symbols := []int{65, 66, 67, 65}
	bits := encode(symbols, table)
	decoded, err := huffmantree.DecodeAll(tree, bits, 4)
	if err != nil {
		t.Fatalf("DecodeAll error: %v", err)
	}
	if len(decoded) != 4 {
		t.Errorf("len(decoded) = %d, want 4", len(decoded))
	}
}

func TestCodeTable_CoversAllSymbols(t *testing.T) {
	// Every symbol passed to Build must appear in the code table.
	weights := []huffmantree.WeightPair{wp(1, 5), wp(2, 3), wp(3, 1), wp(4, 2)}
	tree := mustBuild(t, weights)
	table := huffmantree.CodeTable(tree)
	for _, w := range weights {
		if _, ok := table[w.Symbol]; !ok {
			t.Errorf("symbol %d missing from CodeTable", w.Symbol)
		}
	}
}

func TestCanonicalCodeTable_CoversAllSymbols(t *testing.T) {
	weights := []huffmantree.WeightPair{wp(1, 5), wp(2, 3), wp(3, 1), wp(4, 2)}
	tree := mustBuild(t, weights)
	canonical := huffmantree.CanonicalCodeTable(tree)
	for _, w := range weights {
		if _, ok := canonical[w.Symbol]; !ok {
			t.Errorf("symbol %d missing from CanonicalCodeTable", w.Symbol)
		}
	}
}

func TestLeaves_CodesMatchCodeTable(t *testing.T) {
	// Leaves() codes must be identical to CodeTable() codes.
	tree := mustBuild(t, []huffmantree.WeightPair{wp(65, 3), wp(66, 2), wp(67, 1)})
	table := huffmantree.CodeTable(tree)
	for _, le := range huffmantree.Leaves(tree) {
		if le.Code != table[le.Symbol] {
			t.Errorf("Leaves sym %d: code=%q, table has %q", le.Symbol, le.Code, table[le.Symbol])
		}
	}
}

func TestBuild_LargeAlphabet_RoundTrip(t *testing.T) {
	// 256-symbol round-trip sanity check.
	weights := make([]huffmantree.WeightPair, 256)
	for i := range weights {
		weights[i] = wp(i, i+1)
	}
	tree := mustBuild(t, weights)
	table := huffmantree.CodeTable(tree)
	symbols := make([]int, 256)
	for i := range symbols {
		symbols[i] = i
	}
	bits := encode(symbols, table)
	decoded, err := huffmantree.DecodeAll(tree, bits, 256)
	if err != nil {
		t.Fatalf("DecodeAll error: %v", err)
	}
	for i, s := range symbols {
		if decoded[i] != s {
			t.Errorf("decoded[%d] = %d, want %d", i, decoded[i], s)
		}
	}
}

func TestWeightPair_ErrorMessage_ContainsSymbol(t *testing.T) {
	// Error message should reference the bad symbol value.
	_, err := huffmantree.Build([]huffmantree.WeightPair{{Symbol: 42, Frequency: -5}})
	if err == nil {
		t.Fatal("expected error, got nil")
	}
	if !strings.Contains(err.Error(), "42") {
		t.Errorf("error %q should mention symbol 42", err.Error())
	}
}

func TestDecodeAll_SingleSymbol_MultipleOccurrences(t *testing.T) {
	// Single-leaf: each symbol is one "0" bit → 5 zeros decode to 5 symbols.
	tree := mustBuild(t, []huffmantree.WeightPair{wp(99, 10)})
	decoded, err := huffmantree.DecodeAll(tree, "00000", 5)
	if err != nil {
		t.Fatalf("DecodeAll error: %v", err)
	}
	for i, s := range decoded {
		if s != 99 {
			t.Errorf("decoded[%d] = %d, want 99", i, s)
		}
	}
}

func TestCanonicalCodeTable_Sorted(t *testing.T) {
	// The canonical codes must be sorted by (length, symbol) and be
	// numerically increasing within the same length.
	weights := make([]huffmantree.WeightPair, 6)
	for i := range weights {
		weights[i] = wp(i+10, i+1)
	}
	tree := mustBuild(t, weights)
	canonical := huffmantree.CanonicalCodeTable(tree)

	type entry struct {
		sym  int
		code string
	}
	entries := make([]entry, 0, len(canonical))
	for sym, code := range canonical {
		entries = append(entries, entry{sym, code})
	}
	sort.Slice(entries, func(i, j int) bool {
		if len(entries[i].code) != len(entries[j].code) {
			return len(entries[i].code) < len(entries[j].code)
		}
		return entries[i].sym < entries[j].sym
	})

	// Within each length group, codes should be contiguous binary integers.
	prevLen := -1
	prevVal := -1
	for _, e := range entries {
		ln := len(e.code)
		val := 0
		for _, c := range e.code {
			val = val*2 + int(c-'0')
		}
		if ln == prevLen {
			if val != prevVal+1 {
				t.Errorf("canonical codes not contiguous: prev=%d cur=%d (sym=%d)",
					prevVal, val, e.sym)
			}
		}
		prevLen = ln
		prevVal = val
	}
}

func TestBuild_MultipleZeroFreqErrors(t *testing.T) {
	// If the first pair is valid but the second has freq=0, an error is returned.
	_, err := huffmantree.Build([]huffmantree.WeightPair{wp(1, 5), wp(2, 0)})
	if err == nil {
		t.Fatal("expected error for second zero-frequency pair, got nil")
	}
}

func TestFmt(t *testing.T) {
	// Smoke test: we can fmt.Sprint the error message from Build.
	_, err := huffmantree.Build(nil)
	if err == nil {
		t.Fatal("expected error")
	}
	s := fmt.Sprintf("%v", err)
	if s == "" {
		t.Error("error message is empty")
	}
}
