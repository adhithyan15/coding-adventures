package btree

// ---------------------------------------------------------------------------
// B-tree tests
//
// We test every public method, every delete sub-case, and structural
// invariants after each operation.  Coverage target: 95%+.
//
// Test strategy:
//  1. Small trees (t=2, t=3) — easy to reason about manually.
//  2. Larger trees (t=5) — exercise multi-level structure.
//  3. Bulk insert/delete with 10,000+ keys — stress test.
//  4. Edge cases: empty tree, single element, duplicates, out-of-range queries.
// ---------------------------------------------------------------------------

import (
	"fmt"
	"math/rand"
	"sort"
	"testing"
)

// newIntTree creates a tree keyed by int, valued by string, with degree t.
func newIntTree(t int) *BTree[int, string] {
	return New[int, string](t, func(a, b int) bool { return a < b })
}

// ins inserts key k with value "v<k>" into the tree and asserts IsValid.
func ins(tb testing.TB, tr *BTree[int, string], k int) {
	tb.Helper()
	tr.Insert(k, fmt.Sprintf("v%d", k))
	if !tr.IsValid() {
		tb.Fatalf("IsValid() = false after Insert(%d)", k)
	}
}

// del deletes key k from the tree and asserts IsValid.
func del(tb testing.TB, tr *BTree[int, string], k int) {
	tb.Helper()
	tr.Delete(k)
	if !tr.IsValid() {
		tb.Fatalf("IsValid() = false after Delete(%d)", k)
	}
}

// ---------------------------------------------------------------------------
// Basic CRUD
// ---------------------------------------------------------------------------

func TestInsertAndSearch(t *testing.T) {
	tr := newIntTree(2)
	for _, k := range []int{10, 20, 5, 15, 25, 1, 7, 12} {
		ins(t, tr, k)
	}
	for _, k := range []int{10, 20, 5, 15, 25, 1, 7, 12} {
		v, ok := tr.Search(k)
		if !ok {
			t.Errorf("Search(%d) not found", k)
		}
		want := fmt.Sprintf("v%d", k)
		if v != want {
			t.Errorf("Search(%d) = %q, want %q", k, v, want)
		}
	}
	// Non-existent key.
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
	if !tr.IsValid() {
		t.Fatal("IsValid after first insert")
	}
	tr.Insert(10, "second")
	if !tr.IsValid() {
		t.Fatal("IsValid after update")
	}
	v, ok := tr.Search(10)
	if !ok || v != "second" {
		t.Errorf("after update, Search(10) = %q, %v; want 'second', true", v, ok)
	}
	if tr.Len() != 1 {
		t.Errorf("Len() = %d after update, want 1", tr.Len())
	}
}

func TestDeleteNonExistentKey(t *testing.T) {
	tr := newIntTree(2)
	ins(t, tr, 5)
	del(t, tr, 999) // Should be a no-op.
	if tr.Len() != 1 {
		t.Errorf("Len() = %d, want 1 after deleting nonexistent key", tr.Len())
	}
}

func TestDeleteFromEmptyTree(t *testing.T) {
	tr := newIntTree(2)
	tr.Delete(1) // Should not panic.
	if tr.IsValid() == false {
		t.Error("IsValid should be true for empty tree")
	}
}

// ---------------------------------------------------------------------------
// Len, Height, MinKey, MaxKey
// ---------------------------------------------------------------------------

func TestLen(t *testing.T) {
	tr := newIntTree(2)
	if tr.Len() != 0 {
		t.Errorf("Len() = %d, want 0 for empty tree", tr.Len())
	}
	for i := 1; i <= 10; i++ {
		ins(t, tr, i)
		if tr.Len() != i {
			t.Errorf("Len() = %d, want %d", tr.Len(), i)
		}
	}
	del(t, tr, 5)
	if tr.Len() != 9 {
		t.Errorf("Len() = %d after delete, want 9", tr.Len())
	}
}

func TestHeight(t *testing.T) {
	tr := newIntTree(2)
	if tr.Height() != -1 {
		t.Errorf("Height() = %d for empty tree, want -1", tr.Height())
	}
	ins(t, tr, 1)
	if tr.Height() != 0 {
		t.Errorf("Height() = %d for single-element tree, want 0", tr.Height())
	}
	// Insert enough to force a split and grow the tree.
	for i := 2; i <= 20; i++ {
		ins(t, tr, i)
	}
	if tr.Height() < 1 {
		t.Errorf("Height() = %d after many inserts, expected ≥ 1", tr.Height())
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
		t.Errorf("MinKey() = %d, %v; want 10, nil", min, err)
	}
	max, err := tr.MaxKey()
	if err != nil || max != 50 {
		t.Errorf("MaxKey() = %d, %v; want 50, nil", max, err)
	}
}

// ---------------------------------------------------------------------------
// Inorder traversal
// ---------------------------------------------------------------------------

func TestInorder(t *testing.T) {
	tr := newIntTree(2)
	keys := []int{50, 30, 70, 20, 40, 60, 80, 10, 25, 35, 45}
	for _, k := range keys {
		ins(t, tr, k)
	}
	pairs := tr.Inorder()
	if len(pairs) != len(keys) {
		t.Fatalf("Inorder length = %d, want %d", len(pairs), len(keys))
	}
	sort.Ints(keys)
	for i, p := range pairs {
		if p.Key != keys[i] {
			t.Errorf("Inorder[%d].Key = %d, want %d", i, p.Key, keys[i])
		}
	}
}

func TestInorderEmpty(t *testing.T) {
	tr := newIntTree(2)
	if pairs := tr.Inorder(); len(pairs) != 0 {
		t.Errorf("Inorder on empty tree returned %v", pairs)
	}
}

// ---------------------------------------------------------------------------
// RangeQuery
// ---------------------------------------------------------------------------

func TestRangeQuery(t *testing.T) {
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
		result := tr.RangeQuery(tc.low, tc.high)
		if len(result) != tc.wantLen {
			t.Errorf("RangeQuery(%d,%d) len=%d, want %d; got %v",
				tc.low, tc.high, len(result), tc.wantLen, result)
		}
		// Verify sorted order.
		for i := 1; i < len(result); i++ {
			if result[i].Key < result[i-1].Key {
				t.Errorf("RangeQuery result not sorted at index %d", i)
			}
		}
	}
}

func TestRangeQueryEmpty(t *testing.T) {
	tr := newIntTree(2)
	if r := tr.RangeQuery(1, 100); len(r) != 0 {
		t.Errorf("RangeQuery on empty tree should return empty, got %v", r)
	}
}

// ---------------------------------------------------------------------------
// Delete — all three main cases
// ---------------------------------------------------------------------------

// TestDeleteCase1LeafKey tests deleting a key that lives in a leaf node.
func TestDeleteCase1LeafKey(t *testing.T) {
	tr := newIntTree(2)
	// Build a small tree and delete a leaf key.
	for _, k := range []int{10, 20, 30, 5, 15} {
		ins(t, tr, k)
	}
	del(t, tr, 5) // 5 is in a leaf
	if tr.Contains(5) {
		t.Error("key 5 should be gone after deletion")
	}
}

// TestDeleteCase2aInternalKeyLeftRich tests deletion of a key in an internal
// node when the left child has ≥ t keys (replace with in-order predecessor).
func TestDeleteCase2aInternalKeyLeftRich(t *testing.T) {
	tr := newIntTree(2)
	// Carefully construct a tree where the target internal key's left child
	// has ≥ t keys.  Insert in an order that achieves this.
	keys := []int{1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15}
	for _, k := range keys {
		ins(t, tr, k)
	}
	// Delete the root's key (or some internal key) and verify validity.
	del(t, tr, 8)
	if tr.Contains(8) {
		t.Error("key 8 should be gone")
	}
	if !tr.IsValid() {
		t.Error("tree invalid after deleting internal key")
	}
}

// TestDeleteCase2bInternalKeyRightRich tests the symmetric case.
func TestDeleteCase2bInternalKeyRightRich(t *testing.T) {
	tr := newIntTree(3)
	for i := 1; i <= 30; i++ {
		ins(t, tr, i)
	}
	del(t, tr, 15)
	if tr.Contains(15) {
		t.Error("key 15 should be gone")
	}
}

// TestDeleteCase2cMerge tests deletion of an internal key when both children
// have exactly t-1 keys (triggers a merge).
func TestDeleteCase2cMerge(t *testing.T) {
	tr := newIntTree(2)
	// Build tree, then delete keys from children to put them at t-1 threshold.
	keys := []int{10, 20, 30, 5, 15, 25, 35}
	for _, k := range keys {
		ins(t, tr, k)
	}
	// Delete enough to put children at t-1=1 key each.
	del(t, tr, 5)
	del(t, tr, 15)
	del(t, tr, 25)
	del(t, tr, 35)
	// Now the internal nodes may have t-1 keys. Delete an internal key.
	del(t, tr, 20)
	if tr.Contains(20) {
		t.Error("key 20 should be gone after merge-delete")
	}
}

// TestDeleteCase3RotateRight tests filling a deficient child by rotating from
// the left sibling.
func TestDeleteCase3RotateRight(t *testing.T) {
	tr := newIntTree(2)
	// Insert a sequence where right-rotation is needed.
	for _, k := range []int{10, 20, 30, 40, 50, 5, 3} {
		ins(t, tr, k)
	}
	del(t, tr, 50)
	del(t, tr, 40)
	if tr.Contains(50) || tr.Contains(40) {
		t.Error("deleted keys should be gone")
	}
}

// TestDeleteCase3RotateLeft tests filling a deficient child by rotating from
// the right sibling.
func TestDeleteCase3RotateLeft(t *testing.T) {
	tr := newIntTree(2)
	for _, k := range []int{10, 20, 30, 40, 50, 60, 70} {
		ins(t, tr, k)
	}
	del(t, tr, 10)
	if tr.Contains(10) {
		t.Error("key 10 should be gone")
	}
}

// TestDeleteAllKeys deletes every key in the tree and verifies emptiness.
func TestDeleteAllKeys(t *testing.T) {
	tr := newIntTree(2)
	keys := []int{5, 10, 15, 20, 25, 30}
	for _, k := range keys {
		ins(t, tr, k)
	}
	for _, k := range keys {
		del(t, tr, k)
	}
	if tr.Len() != 0 {
		t.Errorf("Len() = %d after deleting all keys, want 0", tr.Len())
	}
	if tr.Height() != -1 {
		t.Errorf("Height() = %d after emptying tree, want -1", tr.Height())
	}
	if !tr.IsValid() {
		t.Error("IsValid should be true for empty tree")
	}
}

// ---------------------------------------------------------------------------
// Various minimum degrees
// ---------------------------------------------------------------------------

func TestDegreeTwoSmall(t *testing.T) {
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
	pairs := tr.Inorder()
	for i, p := range pairs {
		if p.Key != i+1 {
			t.Errorf("Inorder[%d].Key = %d, want %d", i, p.Key, i+1)
		}
	}
}

// ---------------------------------------------------------------------------
// Large-scale stress test (10,000+ keys)
// ---------------------------------------------------------------------------

func TestLargeInsertAndDelete(t *testing.T) {
	const n = 10_000
	tr := newIntTree(3)
	keys := rand.Perm(n)

	// Insert all keys.
	for _, k := range keys {
		ins(t, tr, k)
	}
	if tr.Len() != n {
		t.Fatalf("Len() = %d after inserting %d keys", tr.Len(), n)
	}

	// Verify all keys are searchable.
	for k := 0; k < n; k++ {
		if !tr.Contains(k) {
			t.Fatalf("key %d not found after bulk insert", k)
		}
	}

	// Delete half.
	for k := 0; k < n; k += 2 {
		del(t, tr, k)
	}
	if tr.Len() != n/2 {
		t.Fatalf("Len() = %d after deleting half, want %d", tr.Len(), n/2)
	}

	// Verify remaining keys.
	for k := 1; k < n; k += 2 {
		if !tr.Contains(k) {
			t.Fatalf("odd key %d missing after deleting evens", k)
		}
	}
	for k := 0; k < n; k += 2 {
		if tr.Contains(k) {
			t.Fatalf("even key %d still present after deletion", k)
		}
	}
}

func TestLargeInsertT2(t *testing.T) {
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
// IsValid edge cases
// ---------------------------------------------------------------------------

func TestIsValidEmptyTree(t *testing.T) {
	tr := newIntTree(2)
	if !tr.IsValid() {
		t.Error("empty tree should be valid")
	}
}

func TestIsValidSingleNode(t *testing.T) {
	tr := newIntTree(2)
	ins(t, tr, 42)
	if !tr.IsValid() {
		t.Error("single-node tree should be valid")
	}
}

// ---------------------------------------------------------------------------
// New panics on invalid t
// ---------------------------------------------------------------------------

func TestNewPanicsOnBadDegree(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Error("New with t=1 should panic")
		}
	}()
	New[int, string](1, func(a, b int) bool { return a < b })
}

// ---------------------------------------------------------------------------
// KeyValue String
// ---------------------------------------------------------------------------

func TestKeyValueString(t *testing.T) {
	kv := KeyValue[int, string]{Key: 1, Value: "one"}
	s := kv.String()
	if s == "" {
		t.Error("KeyValue.String() should return non-empty string")
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
			rng := rand.New(rand.NewSource(42))

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
					t.Fatalf("round %d: IsValid() = false", round)
				}
			}

			// Verify final state.
			for k := 0; k < 100; k++ {
				if present[k] != tr.Contains(k) {
					t.Errorf("key %d: present=%v but Contains=%v", k, present[k], tr.Contains(k))
				}
			}
			if tr.Len() != len(present) {
				t.Errorf("Len()=%d, want %d", tr.Len(), len(present))
			}
		})
	}
}

// ---------------------------------------------------------------------------
// RangeQuery completeness: verify against linear scan
// ---------------------------------------------------------------------------

func TestRangeQueryCorrectness(t *testing.T) {
	tr := newIntTree(3)
	all := make([]int, 50)
	for i := range all {
		all[i] = i * 2 // Even numbers 0, 2, 4, ..., 98
		ins(t, tr, all[i])
	}

	for _, tc := range []struct{ low, high int }{
		{0, 0}, {0, 10}, {5, 95}, {50, 100}, {-1, 200},
	} {
		got := tr.RangeQuery(tc.low, tc.high)
		var want []int
		for _, k := range all {
			if k >= tc.low && k <= tc.high {
				want = append(want, k)
			}
		}
		if len(got) != len(want) {
			t.Errorf("RangeQuery(%d,%d): got %d results, want %d", tc.low, tc.high, len(got), len(want))
			continue
		}
		for i := range got {
			if got[i].Key != want[i] {
				t.Errorf("RangeQuery(%d,%d)[%d]: got %d, want %d", tc.low, tc.high, i, got[i].Key, want[i])
			}
		}
	}
}

// ---------------------------------------------------------------------------
// Benchmark
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
