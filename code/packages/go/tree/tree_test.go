// tree_test.go -- Comprehensive Tests for the Tree Package
// ========================================================
//
// Organized by category:
//
//  1. Construction -- creating trees, verifying initial state
//  2. AddChild -- building trees, error cases
//  3. RemoveSubtree -- pruning branches, error cases
//  4. Queries -- Parent, Children, Siblings, IsLeaf, IsRoot, Depth, Height, etc.
//  5. Traversals -- Preorder, Postorder, LevelOrder
//  6. PathTo -- root-to-node paths
//  7. LCA -- lowest common ancestor
//  8. Subtree -- extracting subtrees
//  9. ToAscii -- ASCII visualization
//  10. Edge cases -- single-node trees, deep chains, wide trees
//  11. Graph -- accessing the underlying Graph
package tree

import (
	"errors"
	"fmt"
	"sort"
	"testing"
)

// =========================================================================
// Helper: Build a sample tree for many tests
// =========================================================================
//
//	    A
//	   / \
//	  B   C
//	 / \   \
//	D   E   F
//
// /
//
//	G
func makeSampleTree() *Tree {
	t := New("A")
	_ = t.AddChild("A", "B")
	_ = t.AddChild("A", "C")
	_ = t.AddChild("B", "D")
	_ = t.AddChild("B", "E")
	_ = t.AddChild("C", "F")
	_ = t.AddChild("D", "G")
	return t
}

// assertEqual is a test helper that checks two values are equal.
func assertEqual(t *testing.T, got, want interface{}) {
	t.Helper()
	if fmt.Sprintf("%v", got) != fmt.Sprintf("%v", want) {
		t.Errorf("got %v, want %v", got, want)
	}
}

func assertStringSliceEqual(t *testing.T, got, want []string) {
	t.Helper()
	if len(got) != len(want) {
		t.Errorf("got %v (len=%d), want %v (len=%d)", got, len(got), want, len(want))
		return
	}
	for i := range got {
		if got[i] != want[i] {
			t.Errorf("at index %d: got %q, want %q", i, got[i], want[i])
			return
		}
	}
}

// =========================================================================
// 1. Construction
// =========================================================================

func TestConstruction_CreateTreeWithRoot(t *testing.T) {
	tr := New("root")
	assertEqual(t, tr.Root(), "root")
}

func TestConstruction_NewTreeHasSizeOne(t *testing.T) {
	tr := New("root")
	assertEqual(t, tr.Size(), 1)
}

func TestConstruction_NewTreeRootIsLeaf(t *testing.T) {
	tr := New("root")
	isLeaf, _ := tr.IsLeaf("root")
	assertEqual(t, isLeaf, true)
}

func TestConstruction_NewTreeRootIsRoot(t *testing.T) {
	tr := New("root")
	isRoot, _ := tr.IsRoot("root")
	assertEqual(t, isRoot, true)
}

func TestConstruction_NewTreeRootHasNoParent(t *testing.T) {
	tr := New("root")
	parent, _ := tr.Parent("root")
	assertEqual(t, parent, "")
}

func TestConstruction_NewTreeRootHasNoChildren(t *testing.T) {
	tr := New("root")
	children, _ := tr.Children("root")
	assertStringSliceEqual(t, children, []string{})
}

func TestConstruction_NewTreeRootHasDepthZero(t *testing.T) {
	tr := New("root")
	d, _ := tr.Depth("root")
	assertEqual(t, d, 0)
}

func TestConstruction_NewTreeHeightZero(t *testing.T) {
	tr := New("root")
	assertEqual(t, tr.Height(), 0)
}

func TestConstruction_NewTreeHasRootInNodes(t *testing.T) {
	tr := New("root")
	nodes := tr.Nodes()
	found := false
	for _, n := range nodes {
		if n == "root" {
			found = true
		}
	}
	if !found {
		t.Error("root not found in nodes")
	}
}

func TestConstruction_String(t *testing.T) {
	tr := New("root")
	assertEqual(t, tr.String(), `Tree(root="root", size=1)`)
}

// =========================================================================
// 2. AddChild
// =========================================================================

func TestAddChild_AddOneChild(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "child")
	assertEqual(t, tr.Size(), 2)
}

func TestAddChild_ChildHasCorrectParent(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "child")
	parent, _ := tr.Parent("child")
	assertEqual(t, parent, "root")
}

func TestAddChild_ParentHasChildInChildrenList(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "child")
	children, _ := tr.Children("root")
	found := false
	for _, c := range children {
		if c == "child" {
			found = true
		}
	}
	if !found {
		t.Error("child not found in children list")
	}
}

func TestAddChild_MultipleChildren(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "A")
	_ = tr.AddChild("root", "B")
	_ = tr.AddChild("root", "C")
	children, _ := tr.Children("root")
	assertStringSliceEqual(t, children, []string{"A", "B", "C"})
}

func TestAddChild_ToNonRoot(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "mid")
	_ = tr.AddChild("mid", "leaf")
	parent, _ := tr.Parent("leaf")
	assertEqual(t, parent, "mid")
}

func TestAddChild_BuildDeepTree(t *testing.T) {
	tr := New("level0")
	for i := 1; i < 10; i++ {
		_ = tr.AddChild(fmt.Sprintf("level%d", i-1), fmt.Sprintf("level%d", i))
	}
	assertEqual(t, tr.Size(), 10)
	d, _ := tr.Depth("level9")
	assertEqual(t, d, 9)
}

func TestAddChild_NonexistentParentReturnsError(t *testing.T) {
	tr := New("root")
	err := tr.AddChild("nonexistent", "child")
	var nf *NodeNotFoundError
	if !errors.As(err, &nf) {
		t.Errorf("expected NodeNotFoundError, got %v", err)
	}
	assertEqual(t, nf.Node, "nonexistent")
}

func TestAddChild_DuplicateChildReturnsError(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "child")
	err := tr.AddChild("root", "child")
	var dup *DuplicateNodeError
	if !errors.As(err, &dup) {
		t.Errorf("expected DuplicateNodeError, got %v", err)
	}
	assertEqual(t, dup.Node, "child")
}

func TestAddChild_RootAsChildReturnsError(t *testing.T) {
	tr := New("root")
	err := tr.AddChild("root", "root")
	var dup *DuplicateNodeError
	if !errors.As(err, &dup) {
		t.Errorf("expected DuplicateNodeError, got %v", err)
	}
}

func TestAddChild_MakesParentNotLeaf(t *testing.T) {
	tr := New("root")
	isLeaf, _ := tr.IsLeaf("root")
	assertEqual(t, isLeaf, true)
	_ = tr.AddChild("root", "child")
	isLeaf, _ = tr.IsLeaf("root")
	assertEqual(t, isLeaf, false)
}

func TestAddChild_NewChildIsLeaf(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "child")
	isLeaf, _ := tr.IsLeaf("child")
	assertEqual(t, isLeaf, true)
}

// =========================================================================
// 3. RemoveSubtree
// =========================================================================

func TestRemoveSubtree_RemoveLeaf(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "leaf")
	_ = tr.RemoveSubtree("leaf")
	assertEqual(t, tr.Size(), 1)
	assertEqual(t, tr.HasNode("leaf"), false)
}

func TestRemoveSubtree_RemovesDescendants(t *testing.T) {
	tr := makeSampleTree()
	_ = tr.RemoveSubtree("B")
	assertEqual(t, tr.Size(), 3)
	assertEqual(t, tr.HasNode("B"), false)
	assertEqual(t, tr.HasNode("D"), false)
	assertEqual(t, tr.HasNode("E"), false)
	assertEqual(t, tr.HasNode("G"), false)
}

func TestRemoveSubtree_PreservesSiblings(t *testing.T) {
	tr := makeSampleTree()
	_ = tr.RemoveSubtree("B")
	assertEqual(t, tr.HasNode("C"), true)
	assertEqual(t, tr.HasNode("F"), true)
	children, _ := tr.Children("A")
	assertStringSliceEqual(t, children, []string{"C"})
}

func TestRemoveSubtree_DeepSubtree(t *testing.T) {
	tr := makeSampleTree()
	_ = tr.RemoveSubtree("D")
	assertEqual(t, tr.Size(), 5)
	assertEqual(t, tr.HasNode("D"), false)
	assertEqual(t, tr.HasNode("G"), false)
	children, _ := tr.Children("B")
	assertStringSliceEqual(t, children, []string{"E"})
}

func TestRemoveSubtree_RootReturnsError(t *testing.T) {
	tr := New("root")
	err := tr.RemoveSubtree("root")
	var rre *RootRemovalError
	if !errors.As(err, &rre) {
		t.Errorf("expected RootRemovalError, got %v", err)
	}
}

func TestRemoveSubtree_NonexistentReturnsError(t *testing.T) {
	tr := New("root")
	err := tr.RemoveSubtree("nonexistent")
	var nf *NodeNotFoundError
	if !errors.As(err, &nf) {
		t.Errorf("expected NodeNotFoundError, got %v", err)
	}
}

func TestRemoveSubtree_ThenReadd(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "child")
	_ = tr.RemoveSubtree("child")
	_ = tr.AddChild("root", "child")
	assertEqual(t, tr.HasNode("child"), true)
}

func TestRemoveSubtree_ParentBecomesLeaf(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "only_child")
	_ = tr.RemoveSubtree("only_child")
	isLeaf, _ := tr.IsLeaf("root")
	assertEqual(t, isLeaf, true)
}

// =========================================================================
// 4. Queries
// =========================================================================

func TestParent_OfChild(t *testing.T) {
	parent, _ := makeSampleTree().Parent("B")
	assertEqual(t, parent, "A")
}

func TestParent_OfGrandchild(t *testing.T) {
	parent, _ := makeSampleTree().Parent("G")
	assertEqual(t, parent, "D")
}

func TestParent_OfRootIsEmpty(t *testing.T) {
	parent, _ := makeSampleTree().Parent("A")
	assertEqual(t, parent, "")
}

func TestParent_NonexistentReturnsError(t *testing.T) {
	_, err := makeSampleTree().Parent("Z")
	var nf *NodeNotFoundError
	if !errors.As(err, &nf) {
		t.Errorf("expected NodeNotFoundError, got %v", err)
	}
}

func TestChildren_OfRoot(t *testing.T) {
	children, _ := makeSampleTree().Children("A")
	assertStringSliceEqual(t, children, []string{"B", "C"})
}

func TestChildren_OfInternalNode(t *testing.T) {
	children, _ := makeSampleTree().Children("B")
	assertStringSliceEqual(t, children, []string{"D", "E"})
}

func TestChildren_OfLeaf(t *testing.T) {
	children, _ := makeSampleTree().Children("G")
	assertStringSliceEqual(t, children, []string{})
}

func TestChildren_NonexistentReturnsError(t *testing.T) {
	_, err := makeSampleTree().Children("Z")
	var nf *NodeNotFoundError
	if !errors.As(err, &nf) {
		t.Errorf("expected NodeNotFoundError, got %v", err)
	}
}

func TestSiblings_OfNodeWithSibling(t *testing.T) {
	sibs, _ := makeSampleTree().Siblings("B")
	assertStringSliceEqual(t, sibs, []string{"C"})
}

func TestSiblings_AreMutual(t *testing.T) {
	sibs, _ := makeSampleTree().Siblings("C")
	assertStringSliceEqual(t, sibs, []string{"B"})
}

func TestSiblings_OfOnlyChild(t *testing.T) {
	sibs, _ := makeSampleTree().Siblings("F")
	assertStringSliceEqual(t, sibs, []string{})
}

func TestSiblings_OfRoot(t *testing.T) {
	sibs, _ := makeSampleTree().Siblings("A")
	assertStringSliceEqual(t, sibs, []string{})
}

func TestSiblings_NonexistentReturnsError(t *testing.T) {
	_, err := makeSampleTree().Siblings("Z")
	var nf *NodeNotFoundError
	if !errors.As(err, &nf) {
		t.Errorf("expected NodeNotFoundError, got %v", err)
	}
}

func TestSiblings_Multiple(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "A")
	_ = tr.AddChild("root", "B")
	_ = tr.AddChild("root", "C")
	_ = tr.AddChild("root", "D")
	sibs, _ := tr.Siblings("B")
	assertStringSliceEqual(t, sibs, []string{"A", "C", "D"})
}

func TestIsLeaf_True(t *testing.T) {
	tr := makeSampleTree()
	for _, node := range []string{"G", "E", "F"} {
		isLeaf, _ := tr.IsLeaf(node)
		assertEqual(t, isLeaf, true)
	}
}

func TestIsLeaf_False(t *testing.T) {
	tr := makeSampleTree()
	for _, node := range []string{"A", "B"} {
		isLeaf, _ := tr.IsLeaf(node)
		assertEqual(t, isLeaf, false)
	}
}

func TestIsLeaf_NonexistentReturnsError(t *testing.T) {
	_, err := makeSampleTree().IsLeaf("Z")
	var nf *NodeNotFoundError
	if !errors.As(err, &nf) {
		t.Errorf("expected NodeNotFoundError, got %v", err)
	}
}

func TestIsRoot_True(t *testing.T) {
	isRoot, _ := makeSampleTree().IsRoot("A")
	assertEqual(t, isRoot, true)
}

func TestIsRoot_False(t *testing.T) {
	isRoot, _ := makeSampleTree().IsRoot("B")
	assertEqual(t, isRoot, false)
}

func TestIsRoot_NonexistentReturnsError(t *testing.T) {
	_, err := makeSampleTree().IsRoot("Z")
	var nf *NodeNotFoundError
	if !errors.As(err, &nf) {
		t.Errorf("expected NodeNotFoundError, got %v", err)
	}
}

func TestDepth_Root(t *testing.T) {
	d, _ := makeSampleTree().Depth("A")
	assertEqual(t, d, 0)
}

func TestDepth_LevelOne(t *testing.T) {
	tr := makeSampleTree()
	d, _ := tr.Depth("B")
	assertEqual(t, d, 1)
	d, _ = tr.Depth("C")
	assertEqual(t, d, 1)
}

func TestDepth_LevelTwo(t *testing.T) {
	tr := makeSampleTree()
	for _, n := range []string{"D", "E", "F"} {
		d, _ := tr.Depth(n)
		assertEqual(t, d, 2)
	}
}

func TestDepth_LevelThree(t *testing.T) {
	d, _ := makeSampleTree().Depth("G")
	assertEqual(t, d, 3)
}

func TestDepth_NonexistentReturnsError(t *testing.T) {
	_, err := makeSampleTree().Depth("Z")
	var nf *NodeNotFoundError
	if !errors.As(err, &nf) {
		t.Errorf("expected NodeNotFoundError, got %v", err)
	}
}

func TestHeight_SampleTree(t *testing.T) {
	assertEqual(t, makeSampleTree().Height(), 3)
}

func TestHeight_SingleNode(t *testing.T) {
	assertEqual(t, New("root").Height(), 0)
}

func TestHeight_FlatTree(t *testing.T) {
	tr := New("root")
	for i := 0; i < 5; i++ {
		_ = tr.AddChild("root", fmt.Sprintf("child%d", i))
	}
	assertEqual(t, tr.Height(), 1)
}

func TestHeight_DeepChain(t *testing.T) {
	tr := New("0")
	for i := 1; i < 20; i++ {
		_ = tr.AddChild(fmt.Sprintf("%d", i-1), fmt.Sprintf("%d", i))
	}
	assertEqual(t, tr.Height(), 19)
}

func TestSize_Sample(t *testing.T) {
	assertEqual(t, makeSampleTree().Size(), 7)
}

func TestSize_AfterAdd(t *testing.T) {
	tr := New("root")
	assertEqual(t, tr.Size(), 1)
	_ = tr.AddChild("root", "A")
	assertEqual(t, tr.Size(), 2)
}

func TestNodes_ReturnsAll(t *testing.T) {
	assertStringSliceEqual(t, makeSampleTree().Nodes(), []string{"A", "B", "C", "D", "E", "F", "G"})
}

func TestLeaves_Sample(t *testing.T) {
	assertStringSliceEqual(t, makeSampleTree().Leaves(), []string{"E", "F", "G"})
}

func TestLeaves_SingleNode(t *testing.T) {
	assertStringSliceEqual(t, New("root").Leaves(), []string{"root"})
}

func TestLeaves_FlatTree(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "A")
	_ = tr.AddChild("root", "B")
	_ = tr.AddChild("root", "C")
	assertStringSliceEqual(t, tr.Leaves(), []string{"A", "B", "C"})
}

func TestHasNode_True(t *testing.T) {
	assertEqual(t, makeSampleTree().HasNode("A"), true)
}

func TestHasNode_False(t *testing.T) {
	assertEqual(t, makeSampleTree().HasNode("Z"), false)
}

// =========================================================================
// 5. Traversals
// =========================================================================

func TestPreorder_Sample(t *testing.T) {
	assertStringSliceEqual(t, makeSampleTree().Preorder(), []string{"A", "B", "D", "G", "E", "C", "F"})
}

func TestPreorder_SingleNode(t *testing.T) {
	assertStringSliceEqual(t, New("root").Preorder(), []string{"root"})
}

func TestPreorder_FlatTree(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "C")
	_ = tr.AddChild("root", "A")
	_ = tr.AddChild("root", "B")
	assertStringSliceEqual(t, tr.Preorder(), []string{"root", "A", "B", "C"})
}

func TestPreorder_DeepChain(t *testing.T) {
	tr := New("A")
	_ = tr.AddChild("A", "B")
	_ = tr.AddChild("B", "C")
	assertStringSliceEqual(t, tr.Preorder(), []string{"A", "B", "C"})
}

func TestPostorder_Sample(t *testing.T) {
	assertStringSliceEqual(t, makeSampleTree().Postorder(), []string{"G", "D", "E", "B", "F", "C", "A"})
}

func TestPostorder_SingleNode(t *testing.T) {
	assertStringSliceEqual(t, New("root").Postorder(), []string{"root"})
}

func TestPostorder_FlatTree(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "C")
	_ = tr.AddChild("root", "A")
	_ = tr.AddChild("root", "B")
	assertStringSliceEqual(t, tr.Postorder(), []string{"A", "B", "C", "root"})
}

func TestPostorder_DeepChain(t *testing.T) {
	tr := New("A")
	_ = tr.AddChild("A", "B")
	_ = tr.AddChild("B", "C")
	assertStringSliceEqual(t, tr.Postorder(), []string{"C", "B", "A"})
}

func TestLevelOrder_Sample(t *testing.T) {
	assertStringSliceEqual(t, makeSampleTree().LevelOrder(), []string{"A", "B", "C", "D", "E", "F", "G"})
}

func TestLevelOrder_SingleNode(t *testing.T) {
	assertStringSliceEqual(t, New("root").LevelOrder(), []string{"root"})
}

func TestLevelOrder_FlatTree(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "C")
	_ = tr.AddChild("root", "A")
	_ = tr.AddChild("root", "B")
	assertStringSliceEqual(t, tr.LevelOrder(), []string{"root", "A", "B", "C"})
}

func TestLevelOrder_DeepChain(t *testing.T) {
	tr := New("A")
	_ = tr.AddChild("A", "B")
	_ = tr.AddChild("B", "C")
	assertStringSliceEqual(t, tr.LevelOrder(), []string{"A", "B", "C"})
}

func TestTraversals_SameLength(t *testing.T) {
	tr := makeSampleTree()
	assertEqual(t, len(tr.Preorder()), 7)
	assertEqual(t, len(tr.Postorder()), 7)
	assertEqual(t, len(tr.LevelOrder()), 7)
}

func TestTraversals_SameElements(t *testing.T) {
	tr := makeSampleTree()
	pre := append([]string{}, tr.Preorder()...)
	post := append([]string{}, tr.Postorder()...)
	level := append([]string{}, tr.LevelOrder()...)
	sort.Strings(pre)
	sort.Strings(post)
	sort.Strings(level)
	assertStringSliceEqual(t, pre, post)
	assertStringSliceEqual(t, pre, level)
}

func TestPreorder_RootIsFirst(t *testing.T) {
	assertEqual(t, makeSampleTree().Preorder()[0], "A")
}

func TestPostorder_RootIsLast(t *testing.T) {
	po := makeSampleTree().Postorder()
	assertEqual(t, po[len(po)-1], "A")
}

func TestLevelOrder_RootIsFirst(t *testing.T) {
	assertEqual(t, makeSampleTree().LevelOrder()[0], "A")
}

// =========================================================================
// 6. PathTo
// =========================================================================

func TestPathTo_Root(t *testing.T) {
	path, _ := makeSampleTree().PathTo("A")
	assertStringSliceEqual(t, path, []string{"A"})
}

func TestPathTo_Child(t *testing.T) {
	path, _ := makeSampleTree().PathTo("B")
	assertStringSliceEqual(t, path, []string{"A", "B"})
}

func TestPathTo_Grandchild(t *testing.T) {
	path, _ := makeSampleTree().PathTo("D")
	assertStringSliceEqual(t, path, []string{"A", "B", "D"})
}

func TestPathTo_DeepNode(t *testing.T) {
	path, _ := makeSampleTree().PathTo("G")
	assertStringSliceEqual(t, path, []string{"A", "B", "D", "G"})
}

func TestPathTo_RightBranch(t *testing.T) {
	path, _ := makeSampleTree().PathTo("F")
	assertStringSliceEqual(t, path, []string{"A", "C", "F"})
}

func TestPathTo_NonexistentReturnsError(t *testing.T) {
	_, err := makeSampleTree().PathTo("Z")
	var nf *NodeNotFoundError
	if !errors.As(err, &nf) {
		t.Errorf("expected NodeNotFoundError, got %v", err)
	}
}

func TestPathTo_LengthEqualsDepthPlusOne(t *testing.T) {
	tr := makeSampleTree()
	for _, node := range tr.Nodes() {
		path, _ := tr.PathTo(node)
		d, _ := tr.Depth(node)
		assertEqual(t, len(path), d+1)
	}
}

// =========================================================================
// 7. LCA
// =========================================================================

func TestLCA_SameNode(t *testing.T) {
	lca, _ := makeSampleTree().LCA("D", "D")
	assertEqual(t, lca, "D")
}

func TestLCA_Siblings(t *testing.T) {
	lca, _ := makeSampleTree().LCA("D", "E")
	assertEqual(t, lca, "B")
}

func TestLCA_ParentChild(t *testing.T) {
	lca, _ := makeSampleTree().LCA("B", "D")
	assertEqual(t, lca, "B")
}

func TestLCA_ChildParent(t *testing.T) {
	lca, _ := makeSampleTree().LCA("D", "B")
	assertEqual(t, lca, "B")
}

func TestLCA_Cousins(t *testing.T) {
	lca, _ := makeSampleTree().LCA("D", "F")
	assertEqual(t, lca, "A")
}

func TestLCA_RootAndLeaf(t *testing.T) {
	lca, _ := makeSampleTree().LCA("A", "G")
	assertEqual(t, lca, "A")
}

func TestLCA_DeepNodes(t *testing.T) {
	lca, _ := makeSampleTree().LCA("G", "E")
	assertEqual(t, lca, "B")
}

func TestLCA_BothLeavesDifferentSubtrees(t *testing.T) {
	lca, _ := makeSampleTree().LCA("G", "F")
	assertEqual(t, lca, "A")
}

func TestLCA_NonexistentAReturnsError(t *testing.T) {
	_, err := makeSampleTree().LCA("Z", "A")
	var nf *NodeNotFoundError
	if !errors.As(err, &nf) {
		t.Errorf("expected NodeNotFoundError, got %v", err)
	}
}

func TestLCA_NonexistentBReturnsError(t *testing.T) {
	_, err := makeSampleTree().LCA("A", "Z")
	var nf *NodeNotFoundError
	if !errors.As(err, &nf) {
		t.Errorf("expected NodeNotFoundError, got %v", err)
	}
}

func TestLCA_RootWithRoot(t *testing.T) {
	lca, _ := makeSampleTree().LCA("A", "A")
	assertEqual(t, lca, "A")
}

// =========================================================================
// 8. Subtree
// =========================================================================

func TestSubtree_Leaf(t *testing.T) {
	sub, _ := makeSampleTree().Subtree("G")
	assertEqual(t, sub.Root(), "G")
	assertEqual(t, sub.Size(), 1)
}

func TestSubtree_InternalNode(t *testing.T) {
	sub, _ := makeSampleTree().Subtree("B")
	assertEqual(t, sub.Root(), "B")
	assertEqual(t, sub.Size(), 4)
	assertEqual(t, sub.HasNode("D"), true)
	assertEqual(t, sub.HasNode("E"), true)
	assertEqual(t, sub.HasNode("G"), true)
}

func TestSubtree_PreservesStructure(t *testing.T) {
	sub, _ := makeSampleTree().Subtree("B")
	children, _ := sub.Children("B")
	assertStringSliceEqual(t, children, []string{"D", "E"})
	dChildren, _ := sub.Children("D")
	assertStringSliceEqual(t, dChildren, []string{"G"})
	isLeafG, _ := sub.IsLeaf("G")
	assertEqual(t, isLeafG, true)
	isLeafE, _ := sub.IsLeaf("E")
	assertEqual(t, isLeafE, true)
}

func TestSubtree_Root(t *testing.T) {
	tr := makeSampleTree()
	sub, _ := tr.Subtree("A")
	assertEqual(t, sub.Size(), tr.Size())
	assertStringSliceEqual(t, sub.Nodes(), tr.Nodes())
}

func TestSubtree_DoesNotModifyOriginal(t *testing.T) {
	tr := makeSampleTree()
	origSize := tr.Size()
	_, _ = tr.Subtree("B")
	assertEqual(t, tr.Size(), origSize)
}

func TestSubtree_NonexistentReturnsError(t *testing.T) {
	_, err := makeSampleTree().Subtree("Z")
	var nf *NodeNotFoundError
	if !errors.As(err, &nf) {
		t.Errorf("expected NodeNotFoundError, got %v", err)
	}
}

func TestSubtree_IsIndependent(t *testing.T) {
	tr := makeSampleTree()
	sub, _ := tr.Subtree("B")
	_ = sub.AddChild("E", "new_node")
	assertEqual(t, tr.HasNode("new_node"), false)
}

func TestSubtree_RightBranch(t *testing.T) {
	sub, _ := makeSampleTree().Subtree("C")
	assertEqual(t, sub.Root(), "C")
	assertEqual(t, sub.Size(), 2)
	children, _ := sub.Children("C")
	assertStringSliceEqual(t, children, []string{"F"})
}

// =========================================================================
// 9. ToAscii
// =========================================================================

func TestToAscii_SingleNode(t *testing.T) {
	assertEqual(t, New("root").ToAscii(), "root")
}

func TestToAscii_RootWithOneChild(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "child")
	assertEqual(t, tr.ToAscii(), "root\n└── child")
}

func TestToAscii_RootWithTwoChildren(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "A")
	_ = tr.AddChild("root", "B")
	assertEqual(t, tr.ToAscii(), "root\n├── A\n└── B")
}

func TestToAscii_SampleTree(t *testing.T) {
	expected := "A\n├── B\n│   ├── D\n│   │   └── G\n│   └── E\n└── C\n    └── F"
	assertEqual(t, makeSampleTree().ToAscii(), expected)
}

func TestToAscii_DeepChain(t *testing.T) {
	tr := New("A")
	_ = tr.AddChild("A", "B")
	_ = tr.AddChild("B", "C")
	assertEqual(t, tr.ToAscii(), "A\n└── B\n    └── C")
}

func TestToAscii_WideTree(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "A")
	_ = tr.AddChild("root", "B")
	_ = tr.AddChild("root", "C")
	_ = tr.AddChild("root", "D")
	assertEqual(t, tr.ToAscii(), "root\n├── A\n├── B\n├── C\n└── D")
}

// =========================================================================
// 10. Edge Cases
// =========================================================================

func TestEdge_SingleNodeTraversals(t *testing.T) {
	tr := New("solo")
	assertStringSliceEqual(t, tr.Preorder(), []string{"solo"})
	assertStringSliceEqual(t, tr.Postorder(), []string{"solo"})
	assertStringSliceEqual(t, tr.LevelOrder(), []string{"solo"})
}

func TestEdge_SingleNodeLeaves(t *testing.T) {
	assertStringSliceEqual(t, New("solo").Leaves(), []string{"solo"})
}

func TestEdge_DeepChainHeight(t *testing.T) {
	tr := New("n0")
	for i := 1; i < 100; i++ {
		_ = tr.AddChild(fmt.Sprintf("n%d", i-1), fmt.Sprintf("n%d", i))
	}
	assertEqual(t, tr.Height(), 99)
	assertEqual(t, tr.Size(), 100)
}

func TestEdge_WideTreeHeight(t *testing.T) {
	tr := New("root")
	for i := 0; i < 100; i++ {
		_ = tr.AddChild("root", fmt.Sprintf("child%d", i))
	}
	assertEqual(t, tr.Height(), 1)
	assertEqual(t, tr.Size(), 101)
}

func TestEdge_BalancedBinaryTree(t *testing.T) {
	tr := New("1")
	_ = tr.AddChild("1", "2")
	_ = tr.AddChild("1", "3")
	_ = tr.AddChild("2", "4")
	_ = tr.AddChild("2", "5")
	_ = tr.AddChild("3", "6")
	_ = tr.AddChild("3", "7")
	assertEqual(t, tr.Size(), 7)
	assertEqual(t, tr.Height(), 2)
	assertStringSliceEqual(t, tr.Leaves(), []string{"4", "5", "6", "7"})
}

func TestEdge_NodeNamesWithSpaces(t *testing.T) {
	tr := New("my root")
	_ = tr.AddChild("my root", "my child")
	parent, _ := tr.Parent("my child")
	assertEqual(t, parent, "my root")
}

func TestEdge_NodeNamesWithSpecialChars(t *testing.T) {
	tr := New("root:main")
	_ = tr.AddChild("root:main", "child.1")
	assertEqual(t, tr.HasNode("child.1"), true)
}

func TestEdge_PathToSingleNode(t *testing.T) {
	path, _ := New("solo").PathTo("solo")
	assertStringSliceEqual(t, path, []string{"solo"})
}

func TestEdge_LCAInSingleNodeTree(t *testing.T) {
	lca, _ := New("solo").LCA("solo", "solo")
	assertEqual(t, lca, "solo")
}

func TestEdge_SubtreeOfSingleNode(t *testing.T) {
	sub, _ := New("solo").Subtree("solo")
	assertEqual(t, sub.Root(), "solo")
	assertEqual(t, sub.Size(), 1)
}

func TestEdge_RemoveAndRebuild(t *testing.T) {
	tr := New("root")
	_ = tr.AddChild("root", "A")
	_ = tr.AddChild("A", "B")
	_ = tr.RemoveSubtree("A")
	_ = tr.AddChild("root", "A")
	_ = tr.AddChild("A", "C")
	children, _ := tr.Children("A")
	assertStringSliceEqual(t, children, []string{"C"})
	assertEqual(t, tr.HasNode("B"), false)
}

// =========================================================================
// 11. Graph property
// =========================================================================

func TestGraph_HasCorrectNodes(t *testing.T) {
	tr := makeSampleTree()
	nodes := tr.Graph().Nodes()
	sort.Strings(nodes)
	assertStringSliceEqual(t, nodes, []string{"A", "B", "C", "D", "E", "F", "G"})
}

func TestGraph_HasCorrectEdges(t *testing.T) {
	edges := makeSampleTree().Graph().Edges()
	edgeMap := make(map[string]bool)
	for _, e := range edges {
		edgeMap[e[0]+"->"+e[1]] = true
	}
	for _, e := range []string{"A->B", "A->C", "B->D", "B->E", "C->F", "D->G"} {
		if !edgeMap[e] {
			t.Errorf("missing edge: %s", e)
		}
	}
}

func TestGraph_EdgeCount(t *testing.T) {
	assertEqual(t, len(makeSampleTree().Graph().Edges()), 6)
}

func TestGraph_HasNoCycles(t *testing.T) {
	assertEqual(t, makeSampleTree().Graph().HasCycle(), false)
}

func TestGraph_TopologicalSortStartsWithRoot(t *testing.T) {
	topo, _ := makeSampleTree().Graph().TopologicalSort()
	assertEqual(t, topo[0], "A")
}
