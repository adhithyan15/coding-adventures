package com.codingadventures.binarytree

import org.junit.jupiter.api.Test
import kotlin.test.*

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
    private fun sample(): BinaryTree<Int> =
        BinaryTree.fromLevelOrder(listOf(1, 2, 3, 4, 5, null, 6))

    /**
     * Perfect tree of height 2:
     *         1
     *        / \
     *       2   3
     *      / \ / \
     *     4  5 6  7
     */
    private fun perfect(): BinaryTree<Int> =
        BinaryTree.fromLevelOrder(listOf(1, 2, 3, 4, 5, 6, 7))

    // -------------------------------------------------------------------------
    // Construction
    // -------------------------------------------------------------------------

    @Test fun emptyTreeHasSizeZeroAndHeightMinusOne() {
        val t = BinaryTree<Int>()
        assertEquals(0, t.size)
        assertEquals(-1, t.height())
        assertTrue(t.isEmpty)
    }

    @Test fun singleNodeTree() {
        val t = BinaryTree.fromLevelOrder(listOf(42))
        assertEquals(1, t.size)
        assertEquals(0, t.height())
        assertFalse(t.isEmpty)
        assertEquals(42, t.root?.value)
    }

    @Test fun fromLevelOrderBuildsCorrectTree() {
        val t = sample()
        assertEquals(6, t.size)
        assertEquals(2, t.height())
        assertEquals(1, t.root?.value)
        assertEquals(2, t.root?.left?.value)
        assertEquals(3, t.root?.right?.value)
        assertNull(t.root?.right?.left)
        assertEquals(6, t.root?.right?.right?.value)
    }

    @Test fun fromLevelOrderNullValuesCreateAbsentNodes() {
        val t = BinaryTree.fromLevelOrder(listOf(1, null, 3))
        assertNull(t.root?.left)
        assertEquals(3, t.root?.right?.value)
    }

    @Test fun fromLevelOrderEmptyListReturnsEmptyTree() {
        val t = BinaryTree.fromLevelOrder<Int>(emptyList())
        assertTrue(t.isEmpty)
    }

    // -------------------------------------------------------------------------
    // find / leftChild / rightChild
    // -------------------------------------------------------------------------

    @Test fun findLocatesExistingNode() {
        val node = sample().find(4)
        assertNotNull(node)
        assertEquals(4, node.value)
    }

    @Test fun findReturnsNullForAbsentValue() {
        assertNull(sample().find(99))
    }

    @Test fun findReturnsNullForEmptyTree() {
        assertNull(BinaryTree<Int>().find(1))
    }

    @Test fun leftChildReturnsCorrectChild() {
        val t = sample()
        assertEquals(4, t.leftChild(2)?.value)
        assertNull(t.leftChild(4))
    }

    @Test fun rightChildReturnsCorrectChild() {
        val t = sample()
        assertEquals(5, t.rightChild(2)?.value)
        assertEquals(6, t.rightChild(3)?.value)
    }

    @Test fun leftChildOfAbsentValueReturnsNull() {
        assertNull(sample().leftChild(99))
    }

    // -------------------------------------------------------------------------
    // isFull
    // -------------------------------------------------------------------------

    @Test fun isFullReturnsTrueForFullTree() {
        assertTrue(BinaryTree.fromLevelOrder(listOf(1, 2, 3, 4, 5, 6, 7)).isFull())
    }

    @Test fun isFullReturnsFalseWhenNodeHasOneChild() {
        assertFalse(sample().isFull())
    }

    @Test fun isFullReturnsTrueForSingleNode() {
        assertTrue(BinaryTree.fromLevelOrder(listOf(1)).isFull())
    }

    @Test fun isFullReturnsTrueForEmptyTree() {
        assertTrue(BinaryTree<Int>().isFull())
    }

    @Test fun isFullReturnsTrueForNodeWithBothChildren() {
        assertTrue(BinaryTree.fromLevelOrder(listOf(1, 2, 3)).isFull())
    }

    // -------------------------------------------------------------------------
    // isComplete
    // -------------------------------------------------------------------------

    @Test fun isCompleteReturnsTrueForCompleteTree() {
        assertTrue(BinaryTree.fromLevelOrder(listOf(1, 2, 3, 4, 5, 6)).isComplete())
    }

    @Test fun isCompleteReturnsFalseForNonCompleteTree() {
        assertFalse(sample().isComplete())
    }

    @Test fun isCompleteReturnsTrueForSingleNode() {
        assertTrue(BinaryTree.fromLevelOrder(listOf(1)).isComplete())
    }

    @Test fun isCompleteReturnsTrueForEmptyTree() {
        assertTrue(BinaryTree<Int>().isComplete())
    }

    @Test fun isCompleteReturnsTrueForPerfectTree() {
        assertTrue(perfect().isComplete())
    }

    // -------------------------------------------------------------------------
    // isPerfect
    // -------------------------------------------------------------------------

    @Test fun isPerfectReturnsTrueForPerfectTree() {
        assertTrue(perfect().isPerfect())
    }

    @Test fun isPerfectReturnsFalseForImperfectTree() {
        assertFalse(sample().isPerfect())
        assertFalse(BinaryTree.fromLevelOrder(listOf(1, 2, 3, 4, 5, 6)).isPerfect())
    }

    @Test fun isPerfectReturnsTrueForSingleNode() {
        assertTrue(BinaryTree.fromLevelOrder(listOf(1)).isPerfect())
    }

    @Test fun isPerfectReturnsTrueForEmptyTree() {
        assertTrue(BinaryTree<Int>().isPerfect())
    }

    // -------------------------------------------------------------------------
    // Traversals
    // -------------------------------------------------------------------------

    @Test fun inorderTraversalCorrect() {
        assertEquals(listOf(4, 2, 5, 1, 3, 6), sample().inorder())
    }

    @Test fun preorderTraversalCorrect() {
        assertEquals(listOf(1, 2, 4, 5, 3, 6), sample().preorder())
    }

    @Test fun postorderTraversalCorrect() {
        assertEquals(listOf(4, 5, 2, 6, 3, 1), sample().postorder())
    }

    @Test fun levelOrderTraversalCorrect() {
        assertEquals(listOf(1, 2, 3, 4, 5, 6), sample().levelOrder())
    }

    @Test fun traversalsOnEmptyTreeReturnEmptyList() {
        val t = BinaryTree<Int>()
        assertTrue(t.inorder().isEmpty())
        assertTrue(t.preorder().isEmpty())
        assertTrue(t.postorder().isEmpty())
        assertTrue(t.levelOrder().isEmpty())
    }

    @Test fun traversalsOnSingleNode() {
        val t = BinaryTree.fromLevelOrder(listOf(42))
        assertEquals(listOf(42), t.inorder())
        assertEquals(listOf(42), t.preorder())
        assertEquals(listOf(42), t.postorder())
        assertEquals(listOf(42), t.levelOrder())
    }

    // -------------------------------------------------------------------------
    // toArray
    // -------------------------------------------------------------------------

    @Test fun toArrayProducesLevelOrderWithNulls() {
        val arr = sample().toArray()
        assertEquals(7, arr.size)
        assertEquals(1,    arr[0])
        assertEquals(2,    arr[1])
        assertEquals(3,    arr[2])
        assertEquals(4,    arr[3])
        assertEquals(5,    arr[4])
        assertNull(         arr[5])
        assertEquals(6,    arr[6])
    }

    @Test fun toArrayOnEmptyTreeReturnsEmptyList() {
        assertTrue(BinaryTree<Int>().toArray().isEmpty())
    }

    // -------------------------------------------------------------------------
    // toAscii
    // -------------------------------------------------------------------------

    @Test fun toAsciiOnEmptyTreeReturnsEmptyString() {
        assertEquals("", BinaryTree<Int>().toAscii())
    }

    @Test fun toAsciiOnSingleNodeReturnsRootLine() {
        val ascii = BinaryTree.fromLevelOrder(listOf(42)).toAscii()
        assertTrue(ascii.contains("42"), "Expected '42' in: $ascii")
    }

    @Test fun toAsciiContainsAllValues() {
        val ascii = sample().toAscii()
        for (v in 1..6) {
            assertTrue(ascii.contains(v.toString()),
                "Expected '$v' in ASCII output:\n$ascii")
        }
    }

    // -------------------------------------------------------------------------
    // height / size
    // -------------------------------------------------------------------------

    @Test fun heightForVariousTrees() {
        assertEquals(-1, BinaryTree<Int>().height())
        assertEquals( 0, BinaryTree.fromLevelOrder(listOf(1)).height())
        assertEquals( 2, sample().height())
        assertEquals( 2, perfect().height())
    }

    @Test fun sizeForVariousTrees() {
        assertEquals(0, BinaryTree<Int>().size)
        assertEquals(1, BinaryTree.fromLevelOrder(listOf(1)).size)
        assertEquals(6, sample().size)
        assertEquals(7, perfect().size)
    }

    // -------------------------------------------------------------------------
    // toString
    // -------------------------------------------------------------------------

    @Test fun toStringShowsRootAndSize() {
        assertEquals("BinaryTree(root=null, size=0)", BinaryTree<Int>().toString())
        assertEquals("BinaryTree(root=1, size=6)", sample().toString())
    }

    // -------------------------------------------------------------------------
    // String values
    // -------------------------------------------------------------------------

    @Test fun worksWithStringValues() {
        val t = BinaryTree.fromLevelOrder(listOf("root", "left", "right", null, "leaf", null, null))
        assertEquals(listOf("root", "left", "right", "leaf"), t.levelOrder())
        assertNotNull(t.find("leaf"))
    }

    // -------------------------------------------------------------------------
    // Manual tree construction
    // -------------------------------------------------------------------------

    @Test fun manualTreeConstruction() {
        val t = BinaryTree<Int>()
        t.root = t.Node(10)
        t.root!!.left  = t.Node(5)
        t.root!!.right = t.Node(15)
        assertEquals(listOf(5, 10, 15), t.inorder())
        assertTrue(t.isFull())
        assertTrue(t.isComplete())
    }
}
