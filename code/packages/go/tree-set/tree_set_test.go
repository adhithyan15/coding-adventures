package treeset

import "testing"

func TestTreeSet(t *testing.T) {
	set := FromList([]int{7, 3, 9, 1, 5, 3})
	if got := set.ToSlice(); len(got) != 5 || got[0] != 1 || got[4] != 9 {
		t.Fatalf("unexpected ordering: %v", got)
	}
	if rank := set.Rank(7); rank != 3 {
		t.Fatalf("expected rank 3, got %d", rank)
	}
	if kth, ok := set.KthSmallest(3); !ok || kth != 5 {
		t.Fatalf("expected kth=5, got %v %v", kth, ok)
	}
	if pred, ok := set.Predecessor(7); !ok || pred != 5 {
		t.Fatalf("expected predecessor 5, got %v %v", pred, ok)
	}
	if succ, ok := set.Successor(7); !ok || succ != 9 {
		t.Fatalf("expected successor 9, got %v %v", succ, ok)
	}
}
