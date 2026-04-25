package com.codingadventures.avltree

data class AvlNode<T : Comparable<T>>(
    val value: T,
    val left: AvlNode<T>? = null,
    val right: AvlNode<T>? = null,
    val height: Int = 0,
    val size: Int = 1,
) {
    companion object {
        fun <T : Comparable<T>> leaf(value: T): AvlNode<T> = AvlNode(value)

        fun <T : Comparable<T>> create(
            value: T,
            left: AvlNode<T>?,
            right: AvlNode<T>?,
        ): AvlNode<T> = AvlNode(value, left, right, 1 + maxOf(height(left), height(right)), 1 + size(left) + size(right))

        fun <T : Comparable<T>> height(node: AvlNode<T>?): Int = node?.height ?: -1

        fun <T : Comparable<T>> size(node: AvlNode<T>?): Int = node?.size ?: 0
    }
}

class AvlTree<T : Comparable<T>>(val root: AvlNode<T>? = null) {
    fun insert(value: T): AvlTree<T> = AvlTree(insertNode(root, value))

    fun delete(value: T): AvlTree<T> = AvlTree(deleteNode(root, value))

    fun search(value: T): AvlNode<T>? = searchNode(root, value)

    fun contains(value: T): Boolean = search(value) != null

    fun minValue(): T? = minValue(root)

    fun maxValue(): T? = maxValue(root)

    fun predecessor(value: T): T? = predecessor(root, value)

    fun successor(value: T): T? = successor(root, value)

    fun kthSmallest(k: Int): T? = kthSmallest(root, k)

    fun rank(value: T): Int = rank(root, value)

    fun toSortedArray(): List<T> = toSortedArray(root)

    fun isValidBst(): Boolean = isValidBst(root)

    fun isValidAvl(): Boolean = isValidAvl(root)

    fun balanceFactor(node: AvlNode<T>?): Int = balanceFactorNode(node)

    fun height(): Int = AvlNode.height(root)

    fun size(): Int = AvlNode.size(root)

    override fun toString(): String {
        val rootValue = root?.value?.toString() ?: "null"
        return "AvlTree(root=$rootValue, size=${size()}, height=${height()})"
    }

    companion object {
        fun <T : Comparable<T>> empty(): AvlTree<T> = AvlTree()

        fun <T : Comparable<T>> fromValues(values: Iterable<T>): AvlTree<T> =
            values.fold(empty()) { tree, value -> tree.insert(value) }

        fun <T : Comparable<T>> searchNode(root: AvlNode<T>?, value: T): AvlNode<T>? {
            var current = root
            while (current != null) {
                current = when {
                    value < current.value -> current.left
                    value > current.value -> current.right
                    else -> return current
                }
            }
            return null
        }

        fun <T : Comparable<T>> insertNode(root: AvlNode<T>?, value: T): AvlNode<T> {
            if (root == null) return AvlNode.leaf(value)

            return when {
                value < root.value -> rebalance(AvlNode.create(root.value, insertNode(root.left, value), root.right))
                value > root.value -> rebalance(AvlNode.create(root.value, root.left, insertNode(root.right, value)))
                else -> root
            }
        }

        fun <T : Comparable<T>> deleteNode(root: AvlNode<T>?, value: T): AvlNode<T>? {
            if (root == null) return null

            return when {
                value < root.value -> rebalance(AvlNode.create(root.value, deleteNode(root.left, value), root.right))
                value > root.value -> rebalance(AvlNode.create(root.value, root.left, deleteNode(root.right, value)))
                root.left == null -> root.right
                root.right == null -> root.left
                else -> {
                    val extracted = extractMin(root.right)
                    rebalance(AvlNode.create(extracted.minimum, root.left, extracted.root))
                }
            }
        }

        fun <T : Comparable<T>> minValue(root: AvlNode<T>?): T? {
            var current = root
            while (current?.left != null) {
                current = current.left
            }
            return current?.value
        }

        fun <T : Comparable<T>> maxValue(root: AvlNode<T>?): T? {
            var current = root
            while (current?.right != null) {
                current = current.right
            }
            return current?.value
        }

        fun <T : Comparable<T>> predecessor(root: AvlNode<T>?, value: T): T? {
            var current = root
            var best: T? = null

            while (current != null) {
                if (value <= current.value) {
                    current = current.left
                } else {
                    best = current.value
                    current = current.right
                }
            }

            return best
        }

        fun <T : Comparable<T>> successor(root: AvlNode<T>?, value: T): T? {
            var current = root
            var best: T? = null

            while (current != null) {
                if (value >= current.value) {
                    current = current.right
                } else {
                    best = current.value
                    current = current.left
                }
            }

            return best
        }

        fun <T : Comparable<T>> kthSmallest(root: AvlNode<T>?, k: Int): T? {
            if (root == null || k <= 0) return null

            val leftSize = AvlNode.size(root.left)
            return when {
                k == leftSize + 1 -> root.value
                k <= leftSize -> kthSmallest(root.left, k)
                else -> kthSmallest(root.right, k - leftSize - 1)
            }
        }

        fun <T : Comparable<T>> rank(root: AvlNode<T>?, value: T): Int {
            if (root == null) return 0

            return when {
                value < root.value -> rank(root.left, value)
                value > root.value -> AvlNode.size(root.left) + 1 + rank(root.right, value)
                else -> AvlNode.size(root.left)
            }
        }

        fun <T : Comparable<T>> toSortedArray(root: AvlNode<T>?): List<T> {
            val output = mutableListOf<T>()
            inOrder(root, output)
            return output
        }

        fun <T : Comparable<T>> isValidBst(root: AvlNode<T>?): Boolean =
            validateBst(root, null, null)

        fun <T : Comparable<T>> isValidAvl(root: AvlNode<T>?): Boolean =
            validateAvl(root, null, null) != null

        fun <T : Comparable<T>> balanceFactorNode(node: AvlNode<T>?): Int =
            if (node == null) 0 else AvlNode.height(node.left) - AvlNode.height(node.right)

        private fun <T : Comparable<T>> rebalance(node: AvlNode<T>): AvlNode<T> {
            val balance = balanceFactorNode(node)

            if (balance > 1) {
                val left = if (node.left != null && balanceFactorNode(node.left) < 0) {
                    rotateLeft(node.left)
                } else {
                    node.left
                }
                return rotateRight(AvlNode.create(node.value, left, node.right))
            }

            if (balance < -1) {
                val right = if (node.right != null && balanceFactorNode(node.right) > 0) {
                    rotateRight(node.right)
                } else {
                    node.right
                }
                return rotateLeft(AvlNode.create(node.value, node.left, right))
            }

            return node
        }

        private fun <T : Comparable<T>> rotateLeft(root: AvlNode<T>): AvlNode<T> {
            val right = root.right ?: return root
            val newLeft = AvlNode.create(root.value, root.left, right.left)
            return AvlNode.create(right.value, newLeft, right.right)
        }

        private fun <T : Comparable<T>> rotateRight(root: AvlNode<T>): AvlNode<T> {
            val left = root.left ?: return root
            val newRight = AvlNode.create(root.value, left.right, root.right)
            return AvlNode.create(left.value, left.left, newRight)
        }

        private fun <T : Comparable<T>> extractMin(root: AvlNode<T>): Extracted<T> {
            if (root.left == null) {
                return Extracted(root.right, root.value)
            }

            val extracted = extractMin(root.left)
            return Extracted(
                rebalance(AvlNode.create(root.value, extracted.root, root.right)),
                extracted.minimum,
            )
        }

        private fun <T : Comparable<T>> inOrder(root: AvlNode<T>?, output: MutableList<T>) {
            if (root == null) return
            inOrder(root.left, output)
            output += root.value
            inOrder(root.right, output)
        }

        private fun <T : Comparable<T>> validateBst(
            root: AvlNode<T>?,
            minimum: T?,
            maximum: T?,
        ): Boolean {
            if (root == null) return true
            if (minimum != null && root.value <= minimum) return false
            if (maximum != null && root.value >= maximum) return false
            return validateBst(root.left, minimum, root.value) && validateBst(root.right, root.value, maximum)
        }

        private fun <T : Comparable<T>> validateAvl(
            root: AvlNode<T>?,
            minimum: T?,
            maximum: T?,
        ): Validation? {
            if (root == null) return Validation(-1, 0)
            if (minimum != null && root.value <= minimum) return null
            if (maximum != null && root.value >= maximum) return null

            val left = validateAvl(root.left, minimum, root.value) ?: return null
            val right = validateAvl(root.right, root.value, maximum) ?: return null
            val expectedHeight = 1 + maxOf(left.height, right.height)
            val expectedSize = 1 + left.size + right.size
            if (root.height != expectedHeight || root.size != expectedSize || kotlin.math.abs(left.height - right.height) > 1) {
                return null
            }
            return Validation(expectedHeight, expectedSize)
        }

        private data class Extracted<T : Comparable<T>>(val root: AvlNode<T>?, val minimum: T)

        private data class Validation(val height: Int, val size: Int)
    }
}
