package bplustree

// ---------------------------------------------------------------------------
// B+ tree tests
//
// Test strategy:
//  1. Basic CRUD with small trees (t=2, t=3).
//  2. Leaf linked list integrity — walk firstLeaf.next chain and verify order.
//  3. RangeScan — verified against linear scan.
//  4. FullScan — verified against Inorder.
//  5. Delete sub-cases: rotate-right, rotate-left, merge.
//  6. Bulk stress test with 10,000+ keys.
//  7. IsValid() after every operation.
// ---------------------------------------------------------------------------

import (
	"fmt"
	"math/rand"
	"sort"
	"testing"
)

// newIntTree creates a BPlusTree[int, string] with degree t.
func newIntTree(t int) *BPlusTree[int, string] {
	return New[int, string](t, func(a, b int) bool { return a < b })
}

// ins inserts key k with value "v<k>" and asserts IsValid.
func ins(tb testing.TB, tr *BPlusTree[int, string], k int) {
	tb.Helper()
	tr.Insert(k, fmt.Sprintf("v%d", k))
	if !tr.IsValid() {
		tb.Fatalf("IsValid()=false after Insert(%d)", k)
	}
}

// del deletes key k and asserts IsValid.
func del(tb testing.TB, tr *BPlusTree[int, string], k int) {
	tb.Helper()
	tr.Delete(k)
	if !tr.IsValid() {
		tb.Fatalf("IsValid()=false after Delete(%d)", k)
	}
}

// checkLeafChain walks firstLeaf → next → ... and verifies:
//  1. All keys in ascending order across leaves.
//  2. The total count matches expected.
func checkLeafChain(tb testing.TB, tr *BPlusTree[int, string], wantKeys []int) {
	tb.Helper()
	var got []int
	leaf := tr.firstLeaf
	for leaf != nil {
		got = append(got, leaf.keys...)
		leaf = leaf.next
	}
	if len(got) != len(wantKeys) {
		tb.Errorf("leaf chain length=%d, want %d; chain=%v", len(got), len(wantKeys), got)
		return
	}
	for i, k := range got {
		if k != wantKeys[i] {
			tb.Errorf("leaf chain[%d]=%d, want %d", i, k, wantKeys[i])
		}
	}
	// Verify ascending order.
	for i := 1; i < len(got); i++ {
		if got[i] <= got[i-1] {
			tb.Errorf("leaf chain not sorted at index %d: %d >= %d", i, got[i-1], got[i])
		}
	}
}

// ---------------------------------------------------------------------------
// Basic CRUD
// ---------------------------------------------------------------------------

func TestInsertAndSearch(t *testing.T) {
	tr := newIntTree(2)
	keys := []int{10, 20, 5, 15, 25, 1, 7, 12}
	for _, k := range keys {
		ins(t, tr, k)
	}
	for _, k := range keys {
		v, ok := tr.Search(k)
		if !ok {
			t.Errorf("Search(%d) not found", k)
		}
		want := fmt.Sprintf("v%d", k)
		if v != want {
			t.Errorf("Search(%d)=%q, want %q", k, v, want)
		}
	}
	_, ok := tr.Search(999)
	if ok {
		t.Error("Search(999) should return false")
	}
}

func TestContains(t *testing.T) {
	tr := newIntTree(3)
	ins(t, tr, 42)
	if !tr.Contains(42) {
		t.Error("Contains(42) should be true")
	}
	if tr.Contains(0) {
		t.Error("Contains(0) should be false")
	}
}

func TestUpdateExistingKey(t *testing.T) {
	tr := newIntTree(2)
	tr.Insert(10, "first")
	tr.Insert(10, "second")
	if !tr.IsValid() {
		t.Fatal("IsValid after update")
	}
	v, ok := tr.Search(10)
	if !ok || v != "second" {
		t.Errorf("after update, Search(10)=%q %v; want 'second' true", v, ok)
	}
	if tr.Len() != 1 {
		t.Errorf("Len()=%d after update, want 1", tr.Len())
	}
}

func TestDeleteNonExistent(t *testing.T) {
	tr := newIntTree(2)
	ins(t, tr, 5)
	del(t, tr, 999)
	if tr.Len() != 1 {
		t.Errorf("Len()=%d after no-op delete, want 1", tr.Len())
	}
}

func TestDeleteFromEmptyTree(t *testing.T) {
	tr := newIntTree(2)
	tr.Delete(1) // no panic
	if !tr.IsValid() {
		t.Error("IsValid should be true for empty tree")
	}
}

func TestSearchOnEmptyTree(t *testing.T) {
	tr := newIntTree(2)
	v, ok := tr.Search(42)
	if ok {
		t.Error("Search on empty tree should return false")
	}
	if v != "" {
		t.Errorf("Search on empty tree should return zero value, got %q", v)
	}
	if tr.Contains(42) {
		t.Error("Contains on empty tree should return false")
	}
}

// ---------------------------------------------------------------------------
// Len, Height, MinKey, MaxKey
// ---------------------------------------------------------------------------

func TestLen(t *testing.T) {
	tr := newIntTree(2)
	if tr.Len() != 0 {
		t.Errorf("Len()=%d for empty tree, want 0", tr.Len())
	}
	for i := 1; i <= 10; i++ {
		ins(t, tr, i)
		if tr.Len() != i {
			t.Errorf("Len()=%d, want %d", tr.Len(), i)
		}
	}
	del(t, tr, 5)
	if tr.Len() != 9 {
		t.Errorf("Len()=%d after delete, want 9", tr.Len())
	}
}

func TestHeight(t *testing.T) {
	tr := newIntTree(2)
	if tr.Height() != -1 {
		t.Errorf("Height()=%d for empty tree, want -1", tr.Height())
	}
	ins(t, tr, 1)
	if tr.Height() != 0 {
		t.Errorf("Height()=%d for single element, want 0", tr.Height())
	}
	for i := 2; i <= 20; i++ {
		ins(t, tr, i)
	}
	if tr.Height() < 1 {
		t.Errorf("Height()=%d after many inserts, want ≥1", tr.Height())
	}
}

func TestMinMaxKey(t *testing.T) {
	tr := newIntTree(3)
	_, err := tr.MinKey()
	if err == nil {
		t.Error("MinKey on empty tree should return error")
	}
	_, err = tr.MaxKey()
	if err == nil {
		t.Error("MaxKey on empty tree should return error")
	}

	for _, k := range []int{40, 10, 30, 20, 50} {
		ins(t, tr, k)
	}
	min, err := tr.MinKey()
	if err != nil || min != 10 {
		t.Errorf("MinKey()=%d %v; want 10 nil", min, err)
	}
	max, err := tr.MaxKey()
	if err != nil || max != 50 {
		t.Errorf("MaxKey()=%d %v; want 50 nil", max, err)
	}
}

// ---------------------------------------------------------------------------
// Leaf linked list integrity
// ---------------------------------------------------------------------------

func TestLeafChainAfterInserts(t *testing.T) {
	tr := newIntTree(2)
	keys := []int{30, 10, 50, 20, 40, 5, 25, 35, 45, 55}
	for _, k := range keys {
		ins(t, tr, k)
	}
	sorted := make([]int, len(keys))
	copy(sorted, keys)
	sort.Ints(sorted)
	checkLeafChain(t, tr, sorted)
}

func TestLeafChainAfterDeletes(t *testing.T) {
	tr := newIntTree(3)
	for i := 1; i <= 20; i++ {
		ins(t, tr, i)
	}
	// Delete every other key.
	for i := 2; i <= 20; i += 2 {
		del(t, tr, i)
	}
	var want []int
	for i := 1; i <= 20; i += 2 {
		want = append(want, i)
	}
	checkLeafChain(t, tr, want)
}

func TestLeafChainIntegrityLarge(t *testing.T) {
	const n = 1000
	tr := newIntTree(5)
	keys := rand.Perm(n)
	for _, k := range keys {
		ins(t, tr, k)
	}
	want := make([]int, n)
	for i := range want {
		want[i] = i
	}
	checkLeafChain(t, tr, want)
}

// ---------------------------------------------------------------------------
// FullScan and RangeScan
// ---------------------------------------------------------------------------

func TestFullScan(t *testing.T) {
	tr := newIntTree(3)
	for i := 1; i <= 15; i++ {
		ins(t, tr, i)
	}
	pairs := tr.FullScan()
	if len(pairs) != 15 {
		t.Fatalf("FullScan len=%d, want 15", len(pairs))
	}
	for i, p := range pairs {
		if p.Key != i+1 {
			t.Errorf("FullScan[%d].Key=%d, want %d", i, p.Key, i+1)
		}
	}
}

func TestFullScanEmpty(t *testing.T) {
	tr := newIntTree(2)
	if pairs := tr.FullScan(); len(pairs) != 0 {
		t.Errorf("FullScan on empty tree returned %v", pairs)
	}
}

func TestRangeScan(t *testing.T) {
	tr := newIntTree(3)
	for i := 1; i <= 20; i++ {
		ins(t, tr, i)
	}

	cases := []struct {
		low, high int
		wantLen   int
	}{
		{1, 20, 20},
		{5, 10, 6},
		{0, 100, 20},
		{100, 200, 0},
		{10, 10, 1},
		{-5, 0, 0},
	}
	for _, tc := range cases {
		result := tr.RangeScan(tc.low, tc.high)
		if len(result) != tc.wantLen {
			t.Errorf("RangeScan(%d,%d) len=%d, want %d", tc.low, tc.high, len(result), tc.wantLen)
		}
		// Verify sorted.
		for i := 1; i < len(result); i++ {
			if result[i].Key < result[i-1].Key {
				t.Errorf("RangeScan result not sorted at %d", i)
			}
		}
	}
}

func TestRangeScanEmpty(t *testing.T) {
	tr := newIntTree(2)
	if r := tr.RangeScan(1, 100); len(r) != 0 {
		t.Errorf("RangeScan on empty tree should return nil, got %v", r)
	}
}

// TestRangeScanCorrectness verifies RangeScan against a linear scan.
func TestRangeScanCorrectness(t *testing.T) {
	tr := newIntTree(3)
	all := make([]int, 50)
	for i := range all {
		all[i] = i * 2 // 0, 2, 4, ..., 98
		ins(t, tr, all[i])
	}

	for _, tc := range []struct{ low, high int }{
		{0, 0}, {0, 10}, {5, 95}, {50, 100}, {-1, 200},
	} {
		got := tr.RangeScan(tc.low, tc.high)
		var want []int
		for _, k := range all {
			if k >= tc.low && k <= tc.high {
				want = append(want, k)
			}
		}
		if len(got) != len(want) {
			t.Errorf("RangeScan(%d,%d): got %d, want %d", tc.low, tc.high, len(got), len(want))
			continue
		}
		for i := range got {
			if got[i].Key != want[i] {
				t.Errorf("RangeScan(%d,%d)[%d]: got %d, want %d", tc.low, tc.high, i, got[i].Key, want[i])
			}
		}
	}
}

// ---------------------------------------------------------------------------
// Delete sub-cases
// ---------------------------------------------------------------------------

// TestDeleteRotateRight: child deficient, left sibling has extra → rotate right.
func TestDeleteRotateRight(t *testing.T) {
	tr := newIntTree(2)
	for _, k := range []int{10, 20, 30, 5, 3} {
		ins(t, tr, k)
	}
	del(t, tr, 30)
	del(t, tr, 20)
	if tr.Contains(30) || tr.Contains(20) {
		t.Error("deleted keys should be gone")
	}
}

// TestDeleteRotateLeft: child deficient, right sibling has extra → rotate left.
func TestDeleteRotateLeft(t *testing.T) {
	tr := newIntTree(2)
	for _, k := range []int{10, 20, 30, 40, 50, 60} {
		ins(t, tr, k)
	}
	del(t, tr, 10)
	if tr.Contains(10) {
		t.Error("key 10 should be gone")
	}
}

// TestDeleteMerge: both siblings at minimum → merge.
func TestDeleteMerge(t *testing.T) {
	tr := newIntTree(2)
	for _, k := range []int{10, 20, 30, 5, 15, 25, 35} {
		ins(t, tr, k)
	}
	// Remove enough to force a merge.
	del(t, tr, 5)
	del(t, tr, 15)
	del(t, tr, 25)
	del(t, tr, 35)
	del(t, tr, 10)
	if tr.Contains(10) {
		t.Error("key 10 should be gone after merge")
	}
}

func TestDeleteAllKeys(t *testing.T) {
	tr := newIntTree(2)
	keys := []int{1, 2, 3, 4, 5, 6, 7}
	for _, k := range keys {
		ins(t, tr, k)
	}
	for _, k := range keys {
		del(t, tr, k)
	}
	if tr.Len() != 0 {
		t.Errorf("Len()=%d after deleting all keys, want 0", tr.Len())
	}
	if tr.Height() != -1 {
		t.Errorf("Height()=%d after emptying tree, want -1", tr.Height())
	}
	if !tr.IsValid() {
		t.Error("IsValid should be true for empty tree")
	}
	if tr.firstLeaf != nil {
		t.Error("firstLeaf should be nil after emptying tree")
	}
}

// ---------------------------------------------------------------------------
// Various minimum degrees
// ---------------------------------------------------------------------------

func TestDegreeTwo(t *testing.T) {
	tr := newIntTree(2)
	for i := 0; i < 30; i++ {
		ins(t, tr, i)
	}
	for i := 0; i < 30; i += 2 {
		del(t, tr, i)
	}
	for i := 0; i < 30; i += 2 {
		if tr.Contains(i) {
			t.Errorf("even key %d should be deleted", i)
		}
	}
	for i := 1; i < 30; i += 2 {
		if !tr.Contains(i) {
			t.Errorf("odd key %d should still exist", i)
		}
	}
}

func TestDegreeFive(t *testing.T) {
	tr := newIntTree(5)
	for i := 100; i >= 1; i-- {
		ins(t, tr, i)
	}
	pairs := tr.FullScan()
	for i, p := range pairs {
		if p.Key != i+1 {
			t.Errorf("FullScan[%d].Key=%d, want %d", i, p.Key, i+1)
		}
	}
}

// ---------------------------------------------------------------------------
// Large-scale stress tests
// ---------------------------------------------------------------------------

func TestLargeInsertAndDelete(t *testing.T) {
	const n = 10_000
	tr := newIntTree(3)
	keys := rand.Perm(n)
	for _, k := range keys {
		ins(t, tr, k)
	}
	if tr.Len() != n {
		t.Fatalf("Len()=%d after inserting %d keys", tr.Len(), n)
	}

	// Walk the full chain and verify it covers 0..n-1.
	all := tr.FullScan()
	if len(all) != n {
		t.Fatalf("FullScan len=%d, want %d", len(all), n)
	}
	for i, p := range all {
		if p.Key != i {
			t.Fatalf("FullScan[%d].Key=%d, want %d", i, p.Key, i)
		}
	}

	// Delete odd keys.
	for k := 1; k < n; k += 2 {
		del(t, tr, k)
	}
	if tr.Len() != n/2 {
		t.Fatalf("Len()=%d after deleting odds, want %d", tr.Len(), n/2)
	}
	for k := 0; k < n; k += 2 {
		if !tr.Contains(k) {
			t.Fatalf("even key %d missing", k)
		}
	}
	for k := 1; k < n; k += 2 {
		if tr.Contains(k) {
			t.Fatalf("odd key %d still present", k)
		}
	}
}

func TestLargeT2(t *testing.T) {
	const n = 10_001
	tr := newIntTree(2)
	for i := 0; i < n; i++ {
		ins(t, tr, i)
	}
	if tr.Len() != n {
		t.Fatalf("Len()=%d, want %d", tr.Len(), n)
	}
	min, _ := tr.MinKey()
	max, _ := tr.MaxKey()
	if min != 0 || max != n-1 {
		t.Errorf("min=%d max=%d, want 0 and %d", min, max, n-1)
	}
}

// ---------------------------------------------------------------------------
// Random insert/delete with full validation
// ---------------------------------------------------------------------------

func TestRandomInsertDelete(t *testing.T) {
	for _, degree := range []int{2, 3, 5} {
		t.Run(fmt.Sprintf("t=%d", degree), func(t *testing.T) {
			tr := newIntTree(degree)
			present := make(map[int]bool)
			rng := rand.New(rand.NewSource(123))

			for round := 0; round < 500; round++ {
				k := rng.Intn(100)
				if rng.Intn(2) == 0 {
					tr.Insert(k, fmt.Sprintf("v%d", k))
					present[k] = true
				} else {
					tr.Delete(k)
					delete(present, k)
				}
				if !tr.IsValid() {
					t.Fatalf("round %d: IsValid()=false", round)
				}
			}

			// Verify final state.
			for k := 0; k < 100; k++ {
				if present[k] != tr.Contains(k) {
					t.Errorf("key %d: present=%v Contains=%v", k, present[k], tr.Contains(k))
				}
			}
			if tr.Len() != len(present) {
				t.Errorf("Len()=%d, want %d", tr.Len(), len(present))
			}

			// Verify leaf chain covers exactly the present keys.
			var wantKeys []int
			for k := range present {
				wantKeys = append(wantKeys, k)
			}
			sort.Ints(wantKeys)
			checkLeafChain(t, tr, wantKeys)
		})
	}
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

func TestIsValidEmptyTree(t *testing.T) {
	tr := newIntTree(2)
	if !tr.IsValid() {
		t.Error("empty tree should be valid")
	}
}

func TestIsValidSingleElement(t *testing.T) {
	tr := newIntTree(2)
	ins(t, tr, 1)
	if !tr.IsValid() {
		t.Error("single-element tree should be valid")
	}
}

func TestNewPanicsOnBadDegree(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Error("New with t=1 should panic")
		}
	}()
	New[int, string](1, func(a, b int) bool { return a < b })
}

func TestKeyValueString(t *testing.T) {
	kv := KeyValue[int, string]{Key: 1, Value: "one"}
	s := kv.String()
	if s == "" {
		t.Error("KeyValue.String() should return non-empty string")
	}
}

// ---------------------------------------------------------------------------
// firstLeaf pointer correctness
// ---------------------------------------------------------------------------

func TestFirstLeafAfterSplits(t *testing.T) {
	tr := newIntTree(2)
	// Insert in descending order so root splits happen with small keys.
	for i := 20; i >= 1; i-- {
		ins(t, tr, i)
	}
	// firstLeaf should be the leaf containing key=1.
	if tr.firstLeaf == nil {
		t.Fatal("firstLeaf is nil")
	}
	if len(tr.firstLeaf.keys) == 0 || tr.firstLeaf.keys[0] != 1 {
		t.Errorf("firstLeaf.keys[0]=%v, want 1", tr.firstLeaf.keys)
	}
}

// ---------------------------------------------------------------------------
// Additional coverage tests
// ---------------------------------------------------------------------------

// TestSearchAfterMultipleSplits exercises the search path through multi-level
// internal nodes with various routing cases.
func TestSearchAfterMultipleSplits(t *testing.T) {
	// Use t=2 to force frequent splits.
	tr := newIntTree(2)
	// Insert in a pattern that hits many routing branches.
	for i := 0; i < 50; i += 2 {
		ins(t, tr, i) // evens
	}
	for i := 1; i < 50; i += 2 {
		ins(t, tr, i) // odds interleaved
	}
	// Every key from 0..49 should be found.
	for i := 0; i < 50; i++ {
		v, ok := tr.Search(i)
		if !ok {
			t.Errorf("Search(%d) not found", i)
		}
		want := fmt.Sprintf("v%d", i)
		if v != want {
			t.Errorf("Search(%d)=%q, want %q", i, v, want)
		}
	}
}

// TestDeleteIntermediateInternalKey exercises deletion where the key matches
// an internal routing key (the routing key stays after deletion because it's
// just a sentinel in B+ trees).
func TestDeleteSeparatorKey(t *testing.T) {
	tr := newIntTree(2)
	// Build: keys 1..15
	for i := 1; i <= 15; i++ {
		ins(t, tr, i)
	}
	// Delete keys that are likely internal separator copies.
	for i := 1; i <= 15; i += 3 {
		del(t, tr, i)
	}
	if !tr.IsValid() {
		t.Error("IsValid should be true after deleting separator keys")
	}
}

// TestRangeScanSingleKey checks RangeScan when low == high.
func TestRangeScanSingleKey(t *testing.T) {
	tr := newIntTree(3)
	for i := 1; i <= 10; i++ {
		ins(t, tr, i)
	}
	for _, k := range []int{1, 5, 10} {
		r := tr.RangeScan(k, k)
		if len(r) != 1 || r[0].Key != k {
			t.Errorf("RangeScan(%d,%d) = %v, want single result", k, k, r)
		}
	}
}

// TestInsertDescendingOrder inserts keys in descending order to exercise
// left-biased splits.
func TestInsertDescendingOrder(t *testing.T) {
	tr := newIntTree(3)
	for i := 100; i >= 1; i-- {
		ins(t, tr, i)
	}
	pairs := tr.FullScan()
	if len(pairs) != 100 {
		t.Fatalf("FullScan len=%d, want 100", len(pairs))
	}
	for i, p := range pairs {
		if p.Key != i+1 {
			t.Errorf("FullScan[%d].Key=%d, want %d", i, p.Key, i+1)
		}
	}
}

// TestDeleteRangeQuery verifies that RangeScan stays correct after deletions.
func TestDeleteThenRangeScan(t *testing.T) {
	tr := newIntTree(3)
	for i := 0; i < 30; i++ {
		ins(t, tr, i)
	}
	// Delete the even ones.
	for i := 0; i < 30; i += 2 {
		del(t, tr, i)
	}
	// Odd keys from 1 to 29 in range [0, 28]: 1,3,5,7,9,11,13,15,17,19,21,23,25,27 = 14
	r := tr.RangeScan(0, 28)
	if len(r) != 14 {
		t.Errorf("RangeScan after delete: got %d, want 14; results=%v", len(r), r)
	}
	for _, p := range r {
		if p.Key%2 == 0 {
			t.Errorf("even key %d should be deleted", p.Key)
		}
	}
}

// TestDeleteLeafMergeAndShrink: force root to shrink by merging.
func TestDeleteForceShrink(t *testing.T) {
	tr := newIntTree(2)
	// Insert exactly enough to make a 2-level tree.
	for _, k := range []int{1, 2, 3, 4, 5, 6, 7} {
		ins(t, tr, k)
	}
	initialHeight := tr.Height()
	// Delete all but one key.
	for _, k := range []int{1, 2, 3, 4, 5, 6} {
		del(t, tr, k)
	}
	if tr.Height() > initialHeight {
		t.Error("Height should not grow after deletions")
	}
	if tr.Len() != 1 {
		t.Errorf("Len()=%d, want 1", tr.Len())
	}
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

func BenchmarkInsert(b *testing.B) {
	tr := newIntTree(5)
	for i := 0; i < b.N; i++ {
		tr.Insert(i, "v")
	}
}

func BenchmarkSearch(b *testing.B) {
	tr := newIntTree(5)
	for i := 0; i < 1000; i++ {
		tr.Insert(i, "v")
	}
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		tr.Search(i % 1000)
	}
}

func BenchmarkFullScan(b *testing.B) {
	tr := newIntTree(5)
	for i := 0; i < 1000; i++ {
		tr.Insert(i, "v")
	}
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		tr.FullScan()
	}
}
