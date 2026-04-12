package skiplist

import "testing"

func TestSkipListBasicOperations(t *testing.T) {
	list := New(func(a, b int) bool { return a < b })
	if !list.Insert(5) || !list.Insert(2) || !list.Insert(8) || list.Insert(5) {
		t.Fatal("unexpected insert behavior")
	}
	if !list.Contains(2) || list.Len() != 3 {
		t.Fatalf("unexpected state: contains(2)=%v len=%d", list.Contains(2), list.Len())
	}
	if got := list.ToSlice(); len(got) != 3 || got[0] != 2 || got[2] != 8 {
		t.Fatalf("unexpected order: %v", got)
	}
	if got, ok := list.KthSmallest(2); !ok || got != 5 {
		t.Fatalf("expected kth=5, got %v %v", got, ok)
	}
	if rank := list.Rank(6); rank != 2 {
		t.Fatalf("expected rank 2, got %d", rank)
	}
	if !list.Delete(5) || list.Contains(5) {
		t.Fatal("delete failed")
	}
}
