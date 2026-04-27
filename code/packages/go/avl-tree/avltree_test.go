package avltree

import (
	"reflect"
	"testing"
)

func TestRotationsRebalance(t *testing.T) {
	rightHeavy := FromValues([]int{10, 20, 30})
	leftHeavy := FromValues([]int{30, 20, 10})

	if rightHeavy.Root().Value != 20 || leftHeavy.Root().Value != 20 {
		t.Fatal("rotation root mismatch")
	}
	if got := rightHeavy.ToSortedArray(); !reflect.DeepEqual(got, []int{10, 20, 30}) {
		t.Fatalf("sorted = %#v", got)
	}
	if !rightHeavy.IsValidBST() || !rightHeavy.IsValidAVL() || rightHeavy.Height() != 1 || rightHeavy.Size() != 3 {
		t.Fatal("validity/metadata mismatch")
	}
}

func TestSearchAndOrderStatistics(t *testing.T) {
	tree := FromValues([]int{40, 20, 60, 10, 30, 50, 70})

	if tree.Search(20).Value != 20 || !tree.Contains(50) {
		t.Fatal("search mismatch")
	}
	if got, ok := tree.MinValue(); !ok || got != 10 {
		t.Fatalf("min = %v %v", got, ok)
	}
	if got, ok := tree.MaxValue(); !ok || got != 70 {
		t.Fatalf("max = %v %v", got, ok)
	}
	if got, ok := tree.Predecessor(40); !ok || got != 30 {
		t.Fatalf("pred = %v %v", got, ok)
	}
	if got, ok := tree.Successor(40); !ok || got != 50 {
		t.Fatalf("succ = %v %v", got, ok)
	}
	if got, ok := tree.KthSmallest(4); !ok || got != 40 {
		t.Fatalf("kth = %v %v", got, ok)
	}
	if tree.Rank(35) != 3 {
		t.Fatalf("rank = %d", tree.Rank(35))
	}

	deleted := tree.Delete(20)
	if deleted.Contains(20) || !deleted.IsValidAVL() || !tree.Contains(20) {
		t.Fatal("delete mismatch")
	}
}

func TestEmptyDuplicatesAndValidation(t *testing.T) {
	empty := Empty[int]()
	if empty.Search(1) != nil || empty.Contains(1) || empty.BalanceFactor(nil) != 0 {
		t.Fatal("empty search mismatch")
	}
	if _, ok := empty.MinValue(); ok {
		t.Fatal("empty min should miss")
	}
	if _, ok := empty.MaxValue(); ok {
		t.Fatal("empty max should miss")
	}
	if _, ok := empty.Predecessor(1); ok {
		t.Fatal("empty predecessor should miss")
	}
	if _, ok := empty.Successor(1); ok {
		t.Fatal("empty successor should miss")
	}
	if _, ok := empty.KthSmallest(0); ok {
		t.Fatal("empty kth should miss")
	}
	if empty.Rank(1) != 0 {
		t.Fatal("empty rank mismatch")
	}

	tree := FromValues([]int{30, 20, 40, 10, 25, 35, 50})
	if !reflect.DeepEqual(tree.Insert(25).ToSortedArray(), tree.ToSortedArray()) {
		t.Fatal("duplicate changed tree")
	}
	if !reflect.DeepEqual(tree.Delete(999).ToSortedArray(), tree.ToSortedArray()) {
		t.Fatal("missing delete changed tree")
	}

	badOrder := AVLTree[int]{root: &Node[int]{Value: 5, Left: NewNode(6), Height: 1, size: 2}}
	badHeight := AVLTree[int]{root: &Node[int]{Value: 5, Left: NewNode(3), Height: 99, size: 2}}
	if badOrder.IsValidBST() || badOrder.IsValidAVL() || badHeight.IsValidAVL() {
		t.Fatal("bad tree passed validation")
	}
}

func TestDoubleRotations(t *testing.T) {
	if FromValues([]int{30, 10, 20}).Root().Value != 20 {
		t.Fatal("left-right rotation failed")
	}
	if FromValues([]int{10, 30, 20}).Root().Value != 20 {
		t.Fatal("right-left rotation failed")
	}
}
