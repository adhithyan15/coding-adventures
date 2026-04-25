package com.codingadventures.binarytree

import java.util.ArrayDeque
import java.util.LinkedList
import java.util.Queue

data class BinaryTreeNode<T>(
    val value: T,
    val left: BinaryTreeNode<T>? = null,
    val right: BinaryTreeNode<T>? = null,
)

class BinaryTree<T>(val root: BinaryTreeNode<T>? = null) {
    fun find(value: T): BinaryTreeNode<T>? = find(root, value)

    fun leftChild(value: T): BinaryTreeNode<T>? = find(value)?.left

    fun rightChild(value: T): BinaryTreeNode<T>? = find(value)?.right

    fun isFull(): Boolean = isFull(root)

    fun isComplete(): Boolean = isComplete(root)

    fun isPerfect(): Boolean = isPerfect(root)

    fun height(): Int = height(root)

    fun size(): Int = size(root)

    fun inOrder(): List<T> = inOrder(root)

    fun preOrder(): List<T> = preOrder(root)

    fun postOrder(): List<T> = postOrder(root)

    fun levelOrder(): List<T> = levelOrder(root)

    fun toArray(): List<T?> = toArray(root)

    fun toAscii(): String = toAscii(root)

    override fun toString(): String {
        val rootValue = root?.value?.toString() ?: "null"
        return "BinaryTree(root=$rootValue, size=${size()})"
    }

    companion object {
        fun <T> empty(): BinaryTree<T> = BinaryTree()

        fun <T> singleton(value: T): BinaryTree<T> = BinaryTree(BinaryTreeNode(value))

        fun <T> withRoot(root: BinaryTreeNode<T>?): BinaryTree<T> = BinaryTree(root)

        fun <T> fromLevelOrder(values: List<T?>): BinaryTree<T> =
            BinaryTree(buildFromLevelOrder(values, 0))

        fun <T> find(root: BinaryTreeNode<T>?, value: T): BinaryTreeNode<T>? {
            if (root == null) return null
            if (root.value == value) return root
            return find(root.left, value) ?: find(root.right, value)
        }

        fun <T> leftChild(root: BinaryTreeNode<T>?, value: T): BinaryTreeNode<T>? =
            find(root, value)?.left

        fun <T> rightChild(root: BinaryTreeNode<T>?, value: T): BinaryTreeNode<T>? =
            find(root, value)?.right

        fun <T> isFull(root: BinaryTreeNode<T>?): Boolean =
            when {
                root == null -> true
                root.left == null && root.right == null -> true
                root.left == null || root.right == null -> false
                else -> isFull(root.left) && isFull(root.right)
            }

        fun <T> isComplete(root: BinaryTreeNode<T>?): Boolean {
            val queue: Queue<BinaryTreeNode<T>?> = LinkedList()
            queue.add(root)
            var seenNull = false

            while (queue.isNotEmpty()) {
                val node = queue.remove()
                if (node == null) {
                    seenNull = true
                    continue
                }
                if (seenNull) return false
                queue.add(node.left)
                queue.add(node.right)
            }

            return true
        }

        fun <T> isPerfect(root: BinaryTreeNode<T>?): Boolean {
            val treeHeight = height(root)
            return if (treeHeight < 0) {
                size(root) == 0
            } else {
                size(root) == (1 shl (treeHeight + 1)) - 1
            }
        }

        fun <T> height(root: BinaryTreeNode<T>?): Int =
            if (root == null) -1 else 1 + maxOf(height(root.left), height(root.right))

        fun <T> size(root: BinaryTreeNode<T>?): Int =
            if (root == null) 0 else 1 + size(root.left) + size(root.right)

        fun <T> inOrder(root: BinaryTreeNode<T>?): List<T> {
            val output = mutableListOf<T>()
            inOrder(root, output)
            return output
        }

        fun <T> preOrder(root: BinaryTreeNode<T>?): List<T> {
            val output = mutableListOf<T>()
            preOrder(root, output)
            return output
        }

        fun <T> postOrder(root: BinaryTreeNode<T>?): List<T> {
            val output = mutableListOf<T>()
            postOrder(root, output)
            return output
        }

        fun <T> levelOrder(root: BinaryTreeNode<T>?): List<T> {
            if (root == null) return emptyList()

            val output = mutableListOf<T>()
            val queue: Queue<BinaryTreeNode<T>> = ArrayDeque()
            queue.add(root)

            while (queue.isNotEmpty()) {
                val node = queue.remove()
                output += node.value
                node.left?.let(queue::add)
                node.right?.let(queue::add)
            }

            return output
        }

        fun <T> toArray(root: BinaryTreeNode<T>?): List<T?> {
            val treeHeight = height(root)
            if (treeHeight < 0) return emptyList()

            val output = MutableList<T?>((1 shl (treeHeight + 1)) - 1) { null }
            fillArray(root, 0, output)
            return output
        }

        fun <T> toAscii(root: BinaryTreeNode<T>?): String {
            if (root == null) return ""

            val output = StringBuilder()
            renderAscii(root, "", true, output)
            return output.toString().trimEnd()
        }

        private fun <T> buildFromLevelOrder(values: List<T?>, index: Int): BinaryTreeNode<T>? {
            if (index >= values.size) return null
            val value = values[index] ?: return null
            return BinaryTreeNode(
                value,
                buildFromLevelOrder(values, (2 * index) + 1),
                buildFromLevelOrder(values, (2 * index) + 2),
            )
        }

        private fun <T> inOrder(root: BinaryTreeNode<T>?, output: MutableList<T>) {
            if (root == null) return
            inOrder(root.left, output)
            output += root.value
            inOrder(root.right, output)
        }

        private fun <T> preOrder(root: BinaryTreeNode<T>?, output: MutableList<T>) {
            if (root == null) return
            output += root.value
            preOrder(root.left, output)
            preOrder(root.right, output)
        }

        private fun <T> postOrder(root: BinaryTreeNode<T>?, output: MutableList<T>) {
            if (root == null) return
            postOrder(root.left, output)
            postOrder(root.right, output)
            output += root.value
        }

        private fun <T> fillArray(root: BinaryTreeNode<T>?, index: Int, output: MutableList<T?>) {
            if (root == null || index >= output.size) return
            output[index] = root.value
            fillArray(root.left, (2 * index) + 1, output)
            fillArray(root.right, (2 * index) + 2, output)
        }

        private fun <T> renderAscii(
            node: BinaryTreeNode<T>,
            prefix: String,
            isTail: Boolean,
            output: StringBuilder,
        ) {
            output
                .append(prefix)
                .append(if (isTail) "`-- " else "|-- ")
                .append(node.value)
                .append(System.lineSeparator())

            val children = listOfNotNull(node.left, node.right)
            val nextPrefix = prefix + if (isTail) "    " else "|   "
            children.forEachIndexed { index, child ->
                renderAscii(child, nextPrefix, index + 1 == children.size, output)
            }
        }
    }
}
