package binarytree

import (
	"reflect"
	"strings"
	"testing"
)

func values[T comparable](items []*T) []any {
	out := make([]any, len(items))
	for index, item := range items {
		if item == nil {
			out[index] = nil
		} else {
			out[index] = *item
		}
	}
	return out
}

func TestLevelOrderRoundTrip(t *testing.T) {
	tree := FromLevelOrder([]*int{Ptr(1), Ptr(2), Ptr(3), Ptr(4), Ptr(5), Ptr(6), Ptr(7)})

	if tree.Root() == nil || tree.Root().Value != 1 {
		t.Fatal("root mismatch")
	}
	if got := tree.LevelOrder(); !reflect.DeepEqual(got, []int{1, 2, 3, 4, 5, 6, 7}) {
		t.Fatalf("level order = %#v", got)
	}
	if got := values(tree.ToArray()); !reflect.DeepEqual(got, []any{1, 2, 3, 4, 5, 6, 7}) {
		t.Fatalf("array = %#v", got)
	}
}

func TestShapeQueries(t *testing.T) {
	tree := FromLevelOrder([]*int{Ptr(1), Ptr(2), nil})

	if tree.IsFull() {
		t.Fatal("tree should not be full")
	}
	if !tree.IsComplete() {
		t.Fatal("tree should be complete")
	}
	if tree.IsPerfect() {
		t.Fatal("tree should not be perfect")
	}
	if tree.Height() != 1 || tree.Size() != 2 {
		t.Fatalf("height/size = %d/%d", tree.Height(), tree.Size())
	}
	if tree.LeftChild(1).Value != 2 {
		t.Fatal("left child mismatch")
	}
	if tree.RightChild(1) != nil {
		t.Fatal("right child should be nil")
	}
	if tree.Find(999) != nil {
		t.Fatal("unexpected find result")
	}
}

func TestTraversals(t *testing.T) {
	tree := FromLevelOrder([]*int{Ptr(1), Ptr(2), Ptr(3), Ptr(4), nil, Ptr(5), nil})

	if got := tree.Preorder(); !reflect.DeepEqual(got, []int{1, 2, 4, 3, 5}) {
		t.Fatalf("preorder = %#v", got)
	}
	if got := tree.Inorder(); !reflect.DeepEqual(got, []int{4, 2, 1, 5, 3}) {
		t.Fatalf("inorder = %#v", got)
	}
	if got := tree.Postorder(); !reflect.DeepEqual(got, []int{4, 2, 5, 3, 1}) {
		t.Fatalf("postorder = %#v", got)
	}
	if got := values(tree.ToArray()); !reflect.DeepEqual(got, []any{1, 2, 3, 4, nil, 5, nil}) {
		t.Fatalf("array = %#v", got)
	}
}

func TestPerfectTree(t *testing.T) {
	tree := FromLevelOrder([]*string{Ptr("A"), Ptr("B"), Ptr("C"), Ptr("D"), Ptr("E"), Ptr("F"), Ptr("G")})

	if !tree.IsFull() || !tree.IsComplete() || !tree.IsPerfect() {
		t.Fatal("perfect tree shape mismatch")
	}
	if tree.LeftChild("A").Value != "B" || tree.RightChild("A").Value != "C" {
		t.Fatal("children mismatch")
	}
}

func TestEmptyTree(t *testing.T) {
	tree := New[int]()

	if tree.Root() != nil || !tree.IsFull() || !tree.IsComplete() || !tree.IsPerfect() {
		t.Fatal("empty tree shape mismatch")
	}
	if tree.Height() != -1 || tree.Size() != 0 {
		t.Fatalf("height/size = %d/%d", tree.Height(), tree.Size())
	}
	if len(tree.ToArray()) != 0 || len(tree.LevelOrder()) != 0 || tree.ToASCII() != "" {
		t.Fatal("empty projections mismatch")
	}
	if tree.String() != "BinaryTree(root=<nil>, size=0)" {
		t.Fatalf("string = %q", tree.String())
	}
}

func TestWithRootAndASCII(t *testing.T) {
	root := &Node[string]{
		Value: "root",
		Left:  NewNode("left"),
		Right: NewNode("right"),
	}
	tree := WithRoot(root)

	ascii := tree.ToASCII()
	if !strings.Contains(ascii, "root") || !strings.Contains(ascii, "left") || !strings.Contains(ascii, "right") {
		t.Fatalf("ascii missing values: %q", ascii)
	}
	if tree.String() != "BinaryTree(root=root, size=3)" {
		t.Fatalf("string = %q", tree.String())
	}
}

func TestFreeFunctions(t *testing.T) {
	root := &Node[int]{Value: 1, Left: NewNode(2)}

	if Find(root, 2).Value != 2 || Find(root, 3) != nil {
		t.Fatal("find mismatch")
	}
	if IsFull(root) || !IsComplete(root) || IsPerfect(root) {
		t.Fatal("shape mismatch")
	}
	if Height(root) != 1 || Size(root) != 2 {
		t.Fatal("height/size mismatch")
	}
}
