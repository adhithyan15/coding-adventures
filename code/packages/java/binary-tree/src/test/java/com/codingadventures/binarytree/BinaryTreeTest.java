package com.codingadventures.binarytree;

import com.codingadventures.binarytree.BinaryTree.BinaryTreeNode;
import org.junit.jupiter.api.Test;

import java.util.Arrays;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class BinaryTreeTest {

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    /**
     * Build the tree used in most tests:
     *
     *        1
     *       / \
     *      2   3
     *     / \   \
     *    4   5   6
     */
    private BinaryTree<Integer> sample() {
        return BinaryTree.fromLevelOrder(Arrays.asList(1, 2, 3, 4, 5, null, 6));
    }

    /**
     * Perfect tree of height 2:
     *         1
     *        / \
     *       2   3
     *      / \ / \
     *     4  5 6  7
     */
    private BinaryTree<Integer> perfect() {
        return BinaryTree.fromLevelOrder(List.of(1, 2, 3, 4, 5, 6, 7));
    }

    // -------------------------------------------------------------------------
    // Construction
    // -------------------------------------------------------------------------

    @Test
    void emptyTreeHasSizeZeroAndHeightMinusOne() {
        BinaryTree<Integer> t = new BinaryTree<>();
        assertEquals(0, t.size());
        assertEquals(-1, t.height());
        assertTrue(t.isEmpty());
    }

    @Test
    void singleNodeTree() {
        BinaryTree<Integer> t = BinaryTree.fromLevelOrder(List.of(42));
        assertEquals(1, t.size());
        assertEquals(0, t.height());
        assertFalse(t.isEmpty());
        assertEquals(42, t.root.value);
    }

    @Test
    void fromLevelOrderBuildsCorrectTree() {
        BinaryTree<Integer> t = sample();
        assertEquals(6, t.size());
        assertEquals(2, t.height());
        assertEquals(1, t.root.value);
        assertEquals(2, t.root.left.value);
        assertEquals(3, t.root.right.value);
        assertNull(t.root.right.left);
        assertEquals(6, t.root.right.right.value);
    }

    @Test
    void fromLevelOrderNullValuesCreateAbsentNodes() {
        // [1, null, 3] → root=1, no left child, right child=3
        BinaryTree<Integer> t = BinaryTree.fromLevelOrder(Arrays.asList(1, null, 3));
        assertNull(t.root.left);
        assertEquals(3, t.root.right.value);
    }

    @Test
    void fromLevelOrderEmptyListReturnsEmptyTree() {
        BinaryTree<Integer> t = BinaryTree.fromLevelOrder(List.<Integer>of());
        assertTrue(t.isEmpty());
    }

    @Test
    void constructorWithRootValue() {
        BinaryTree<String> t = new BinaryTree<>("hello");
        assertEquals("hello", t.root.value);
        assertEquals(1, t.size());
    }

    // -------------------------------------------------------------------------
    // find / leftChild / rightChild
    // -------------------------------------------------------------------------

    @Test
    void findLocatesExistingNode() {
        BinaryTree<Integer> t = sample();
        BinaryTreeNode<Integer> node = t.find(4);
        assertNotNull(node);
        assertEquals(4, node.value);
    }

    @Test
    void findReturnsNullForAbsentValue() {
        assertNull(sample().find(99));
    }

    @Test
    void findReturnsNullForEmptyTree() {
        assertNull(new BinaryTree<Integer>().find(1));
    }

    @Test
    void leftChildReturnsCorrectChild() {
        BinaryTree<Integer> t = sample();
        assertEquals(4, t.leftChild(2).value);
        assertNull(t.leftChild(4));   // leaf has no left child
    }

    @Test
    void rightChildReturnsCorrectChild() {
        BinaryTree<Integer> t = sample();
        assertEquals(5, t.rightChild(2).value);
        assertEquals(6, t.rightChild(3).value);
    }

    @Test
    void leftChildOfAbsentValueReturnsNull() {
        assertNull(sample().leftChild(99));
    }

    // -------------------------------------------------------------------------
    // Shape predicates: isFull
    // -------------------------------------------------------------------------

    @Test
    void isFullReturnsTrueForFullTree() {
        // Full tree: every node has 0 or 2 children
        BinaryTree<Integer> t = BinaryTree.fromLevelOrder(List.of(1, 2, 3, 4, 5, 6, 7));
        assertTrue(t.isFull());
    }

    @Test
    void isFullReturnsFalseWhenNodeHasOneChild() {
        assertFalse(sample().isFull());  // node 3 has only right child
    }

    @Test
    void isFullReturnsTrueForSingleNode() {
        assertTrue(new BinaryTree<>(1).isFull());
    }

    @Test
    void isFullReturnsTrueForEmptyTree() {
        assertTrue(new BinaryTree<Integer>().isFull());
    }

    @Test
    void isFullReturnsTrueForNodeWithBothChildren() {
        BinaryTree<Integer> t = BinaryTree.fromLevelOrder(List.of(1, 2, 3));
        assertTrue(t.isFull());
    }

    // -------------------------------------------------------------------------
    // Shape predicates: isComplete
    // -------------------------------------------------------------------------

    @Test
    void isCompleteReturnsTrueForCompleteTree() {
        // Level-order [1,2,3,4,5,6] → all levels filled except rightmost of last
        BinaryTree<Integer> t = BinaryTree.fromLevelOrder(List.of(1, 2, 3, 4, 5, 6));
        assertTrue(t.isComplete());
    }

    @Test
    void isCompleteReturnsFalseForNonCompleteTree() {
        // Node at index 5 is null but index 6 has a node → not left-to-right filled
        assertFalse(sample().isComplete());
    }

    @Test
    void isCompleteReturnsTrueForSingleNode() {
        assertTrue(new BinaryTree<>(1).isComplete());
    }

    @Test
    void isCompleteReturnsTrueForEmptyTree() {
        assertTrue(new BinaryTree<Integer>().isComplete());
    }

    @Test
    void isCompleteReturnsTrueForPerfectTree() {
        assertTrue(perfect().isComplete());
    }

    // -------------------------------------------------------------------------
    // Shape predicates: isPerfect
    // -------------------------------------------------------------------------

    @Test
    void isPerfectReturnsTrueForPerfectTree() {
        assertTrue(perfect().isPerfect());
    }

    @Test
    void isPerfectReturnsFalseForImperfectTree() {
        assertFalse(sample().isPerfect());
        assertFalse(BinaryTree.fromLevelOrder(List.of(1, 2, 3, 4, 5, 6)).isPerfect());
    }

    @Test
    void isPerfectReturnsTrueForSingleNode() {
        assertTrue(new BinaryTree<>(1).isPerfect());
    }

    @Test
    void isPerfectReturnsTrueForEmptyTree() {
        assertTrue(new BinaryTree<Integer>().isPerfect());
    }

    // -------------------------------------------------------------------------
    // Traversals
    // -------------------------------------------------------------------------

    @Test
    void inorderTraversalCorrect() {
        assertEquals(List.of(4, 2, 5, 1, 3, 6), sample().inorder());
    }

    @Test
    void preorderTraversalCorrect() {
        assertEquals(List.of(1, 2, 4, 5, 3, 6), sample().preorder());
    }

    @Test
    void postorderTraversalCorrect() {
        assertEquals(List.of(4, 5, 2, 6, 3, 1), sample().postorder());
    }

    @Test
    void levelOrderTraversalCorrect() {
        assertEquals(List.of(1, 2, 3, 4, 5, 6), sample().levelOrder());
    }

    @Test
    void traversalsOnEmptyTreeReturnEmptyList() {
        BinaryTree<Integer> t = new BinaryTree<>();
        assertTrue(t.inorder().isEmpty());
        assertTrue(t.preorder().isEmpty());
        assertTrue(t.postorder().isEmpty());
        assertTrue(t.levelOrder().isEmpty());
    }

    @Test
    void traversalsOnSingleNode() {
        BinaryTree<Integer> t = new BinaryTree<>(42);
        assertEquals(List.of(42), t.inorder());
        assertEquals(List.of(42), t.preorder());
        assertEquals(List.of(42), t.postorder());
        assertEquals(List.of(42), t.levelOrder());
    }

    // -------------------------------------------------------------------------
    // toArray
    // -------------------------------------------------------------------------

    @Test
    void toArrayProducesLevelOrderWithNulls() {
        // sample: [1, 2, 3, 4, 5, null, 6]
        List<Integer> arr = sample().toArray();
        assertEquals(7, arr.size());   // 2^3 - 1 = 7
        assertEquals(1,    arr.get(0));
        assertEquals(2,    arr.get(1));
        assertEquals(3,    arr.get(2));
        assertEquals(4,    arr.get(3));
        assertEquals(5,    arr.get(4));
        assertNull(         arr.get(5));
        assertEquals(6,    arr.get(6));
    }

    @Test
    void toArrayOnEmptyTreeReturnsEmptyList() {
        assertTrue(new BinaryTree<Integer>().toArray().isEmpty());
    }

    // -------------------------------------------------------------------------
    // toAscii
    // -------------------------------------------------------------------------

    @Test
    void toAsciiOnEmptyTreeReturnsEmptyString() {
        assertEquals("", new BinaryTree<Integer>().toAscii());
    }

    @Test
    void toAsciiOnSingleNodeReturnsRootLine() {
        String ascii = new BinaryTree<>(42).toAscii();
        assertTrue(ascii.contains("42"), "Expected '42' in: " + ascii);
    }

    @Test
    void toAsciiContainsAllValues() {
        String ascii = sample().toAscii();
        for (int v : new int[]{1, 2, 3, 4, 5, 6}) {
            assertTrue(ascii.contains(String.valueOf(v)),
                "Expected '" + v + "' in ASCII output:\n" + ascii);
        }
    }

    // -------------------------------------------------------------------------
    // height / size
    // -------------------------------------------------------------------------

    @Test
    void heightForVariousTrees() {
        assertEquals(-1, new BinaryTree<Integer>().height());
        assertEquals( 0, new BinaryTree<>(1).height());
        assertEquals( 2, sample().height());
        assertEquals( 2, perfect().height());
    }

    @Test
    void sizeForVariousTrees() {
        assertEquals(0, new BinaryTree<Integer>().size());
        assertEquals(1, new BinaryTree<>(1).size());
        assertEquals(6, sample().size());
        assertEquals(7, perfect().size());
    }

    // -------------------------------------------------------------------------
    // toString
    // -------------------------------------------------------------------------

    @Test
    void toStringShowsRootAndSize() {
        assertEquals("BinaryTree(root=null, size=0)", new BinaryTree<Integer>().toString());
        assertEquals("BinaryTree(root=1, size=6)", sample().toString());
    }

    // -------------------------------------------------------------------------
    // String values
    // -------------------------------------------------------------------------

    @Test
    void worksWithStringValues() {
        BinaryTree<String> t = BinaryTree.fromLevelOrder(
            Arrays.asList("root", "left", "right", null, "leaf", null, null));
        assertEquals(List.of("root", "left", "right", "leaf"), t.levelOrder());
        assertNotNull(t.find("leaf"));
    }

    // -------------------------------------------------------------------------
    // Manual tree construction
    // -------------------------------------------------------------------------

    @Test
    void manualTreeConstruction() {
        BinaryTreeNode<Integer> root = new BinaryTreeNode<>(10);
        root.left = new BinaryTreeNode<>(5);
        root.right = new BinaryTreeNode<>(15);
        BinaryTree<Integer> t = new BinaryTree<>(root);
        assertEquals(List.of(5, 10, 15), t.inorder());
        assertTrue(t.isFull());
        assertTrue(t.isComplete());
    }
}
