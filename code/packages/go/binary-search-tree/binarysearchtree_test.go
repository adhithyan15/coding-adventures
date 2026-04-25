package binarysearchtree

import (
	"reflect"
	"testing"
)

func populated() BST[int] {
	tree := Empty[int]()
	for _, value := range []int{5, 1, 8, 3, 7} {
		tree = tree.Insert(value)
	}
	return tree
}

func TestInsertSearchAndDelete(t *testing.T) {
	tree := populated()

	if got := tree.ToSortedArray(); !reflect.DeepEqual(got, []int{1, 3, 5, 7, 8}) {
		t.Fatalf("sorted = %#v", got)
	}
	if tree.Size() != 5 || !tree.Contains(7) || tree.Search(7).Value != 7 {
		t.Fatal("search/size mismatch")
	}
	if got, ok := tree.MinValue(); !ok || got != 1 {
		t.Fatalf("min = %v %v", got, ok)
	}
	if got, ok := tree.MaxValue(); !ok || got != 8 {
		t.Fatalf("max = %v %v", got, ok)
	}
	if got, ok := tree.Predecessor(5); !ok || got != 3 {
		t.Fatalf("predecessor = %v %v", got, ok)
	}
	if got, ok := tree.Successor(5); !ok || got != 7 {
		t.Fatalf("successor = %v %v", got, ok)
	}
	if tree.Rank(4) != 2 {
		t.Fatalf("rank = %d", tree.Rank(4))
	}
	if got, ok := tree.KthSmallest(4); !ok || got != 7 {
		t.Fatalf("kth = %v %v", got, ok)
	}

	deleted := tree.Delete(5)
	if deleted.Contains(5) || !deleted.IsValid() || !tree.Contains(5) {
		t.Fatal("delete did not preserve expected trees")
	}
}

func TestFromSortedArray(t *testing.T) {
	tree := FromSortedArray([]int{1, 2, 3, 4, 5, 6, 7})

	if got := tree.ToSortedArray(); !reflect.DeepEqual(got, []int{1, 2, 3, 4, 5, 6, 7}) {
		t.Fatalf("sorted = %#v", got)
	}
	if tree.Height() != 2 || tree.Size() != 7 || !tree.IsValid() {
		t.Fatal("balanced tree mismatch")
	}
}

func TestEmptyAndEdgeCases(t *testing.T) {
	tree := Empty[int]()

	if tree.Search(1) != nil {
		t.Fatal("empty search should miss")
	}
	if _, ok := tree.MinValue(); ok {
		t.Fatal("empty min should miss")
	}
	if _, ok := tree.MaxValue(); ok {
		t.Fatal("empty max should miss")
	}
	if _, ok := tree.Predecessor(1); ok {
		t.Fatal("empty predecessor should miss")
	}
	if _, ok := tree.Successor(1); ok {
		t.Fatal("empty successor should miss")
	}
	if _, ok := tree.KthSmallest(0); ok {
		t.Fatal("kth zero should miss")
	}
	if _, ok := tree.KthSmallest(1); ok {
		t.Fatal("empty kth should miss")
	}
	if tree.Rank(1) != 0 || tree.Height() != -1 || tree.Size() != 0 {
		t.Fatal("empty metrics mismatch")
	}
	if tree.String() != "BinarySearchTree(root=<nil>, size=0)" {
		t.Fatalf("string = %q", tree.String())
	}
}

func TestDuplicateAndSingleChildDelete(t *testing.T) {
	tree := FromSortedArray([]int{2, 4, 6, 8})

	if tree.Root().Value != 6 || tree.Root().Size() != 4 {
		t.Fatal("root mismatch")
	}
	duplicate := tree.Insert(4)
	if !reflect.DeepEqual(duplicate.ToSortedArray(), tree.ToSortedArray()) {
		t.Fatal("duplicate changed tree")
	}
	if got := tree.Delete(2).ToSortedArray(); !reflect.DeepEqual(got, []int{4, 6, 8}) {
		t.Fatalf("delete one child = %#v", got)
	}
}

func TestValidationFailures(t *testing.T) {
	badOrder := BST[int]{root: &Node[int]{Value: 5, Left: NewNode(6), size: 2}}
	badSize := BST[int]{root: &Node[int]{Value: 5, Left: NewNode(3), size: 99}}

	if badOrder.IsValid() {
		t.Fatal("bad order passed validation")
	}
	if badSize.IsValid() {
		t.Fatal("bad size passed validation")
	}
}
