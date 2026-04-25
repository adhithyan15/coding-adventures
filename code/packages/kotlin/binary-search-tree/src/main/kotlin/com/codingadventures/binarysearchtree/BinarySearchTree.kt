package com.codingadventures.binarysearchtree

data class BstNode<T : Comparable<T>>(
    val value: T,
    val left: BstNode<T>? = null,
    val right: BstNode<T>? = null,
    val size: Int = 1,
) {
    companion object {
        fun <T : Comparable<T>> leaf(value: T): BstNode<T> = BstNode(value)

        fun <T : Comparable<T>> create(
            value: T,
            left: BstNode<T>?,
            right: BstNode<T>?,
        ): BstNode<T> = BstNode(value, left, right, 1 + size(left) + size(right))

        fun <T : Comparable<T>> size(node: BstNode<T>?): Int = node?.size ?: 0
    }
}

class BinarySearchTree<T : Comparable<T>>(val root: BstNode<T>? = null) {
    fun insert(value: T): BinarySearchTree<T> = BinarySearchTree(insertNode(root, value))

    fun delete(value: T): BinarySearchTree<T> = BinarySearchTree(deleteNode(root, value))

    fun search(value: T): BstNode<T>? = searchNode(root, value)

    fun contains(value: T): Boolean = search(value) != null

    fun minValue(): T? = minValue(root)

    fun maxValue(): T? = maxValue(root)

    fun predecessor(value: T): T? = predecessor(root, value)

    fun successor(value: T): T? = successor(root, value)

    fun kthSmallest(k: Int): T? = kthSmallest(root, k)

    fun rank(value: T): Int = rank(root, value)

    fun toSortedArray(): List<T> = toSortedArray(root)

    fun isValid(): Boolean = isValid(root)

    fun height(): Int = height(root)

    fun size(): Int = BstNode.size(root)

    override fun toString(): String {
        val rootValue = root?.value?.toString() ?: "null"
        return "BinarySearchTree(root=$rootValue, size=${size()})"
    }

    companion object {
        fun <T : Comparable<T>> empty(): BinarySearchTree<T> = BinarySearchTree()

        fun <T : Comparable<T>> fromSortedArray(values: List<T>): BinarySearchTree<T> =
            BinarySearchTree(buildBalanced(values, 0, values.size))

        fun <T : Comparable<T>> searchNode(root: BstNode<T>?, value: T): BstNode<T>? {
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

        fun <T : Comparable<T>> insertNode(root: BstNode<T>?, value: T): BstNode<T> {
            if (root == null) return BstNode.leaf(value)

            return when {
                value < root.value -> BstNode.create(root.value, insertNode(root.left, value), root.right)
                value > root.value -> BstNode.create(root.value, root.left, insertNode(root.right, value))
                else -> root
            }
        }

        fun <T : Comparable<T>> deleteNode(root: BstNode<T>?, value: T): BstNode<T>? {
            if (root == null) return null

            return when {
                value < root.value -> BstNode.create(root.value, deleteNode(root.left, value), root.right)
                value > root.value -> BstNode.create(root.value, root.left, deleteNode(root.right, value))
                root.left == null -> root.right
                root.right == null -> root.left
                else -> {
                    val extracted = extractMin(root.right)
                    BstNode.create(extracted.minimum, root.left, extracted.root)
                }
            }
        }

        fun <T : Comparable<T>> minValue(root: BstNode<T>?): T? {
            var current = root
            while (current?.left != null) {
                current = current.left
            }
            return current?.value
        }

        fun <T : Comparable<T>> maxValue(root: BstNode<T>?): T? {
            var current = root
            while (current?.right != null) {
                current = current.right
            }
            return current?.value
        }

        fun <T : Comparable<T>> predecessor(root: BstNode<T>?, value: T): T? {
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

        fun <T : Comparable<T>> successor(root: BstNode<T>?, value: T): T? {
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

        fun <T : Comparable<T>> kthSmallest(root: BstNode<T>?, k: Int): T? {
            if (root == null || k <= 0) return null

            val leftSize = BstNode.size(root.left)
            return when {
                k == leftSize + 1 -> root.value
                k <= leftSize -> kthSmallest(root.left, k)
                else -> kthSmallest(root.right, k - leftSize - 1)
            }
        }

        fun <T : Comparable<T>> rank(root: BstNode<T>?, value: T): Int {
            if (root == null) return 0

            return when {
                value < root.value -> rank(root.left, value)
                value > root.value -> BstNode.size(root.left) + 1 + rank(root.right, value)
                else -> BstNode.size(root.left)
            }
        }

        fun <T : Comparable<T>> toSortedArray(root: BstNode<T>?): List<T> {
            val output = mutableListOf<T>()
            inOrder(root, output)
            return output
        }

        fun <T : Comparable<T>> isValid(root: BstNode<T>?): Boolean =
            validate(root, null, null) != null

        fun <T : Comparable<T>> height(root: BstNode<T>?): Int =
            if (root == null) -1 else 1 + maxOf(height(root.left), height(root.right))

        private fun <T : Comparable<T>> extractMin(root: BstNode<T>): Extracted<T> {
            if (root.left == null) {
                return Extracted(root.right, root.value)
            }

            val extracted = extractMin(root.left)
            return Extracted(
                BstNode.create(root.value, extracted.root, root.right),
                extracted.minimum,
            )
        }

        private fun <T : Comparable<T>> buildBalanced(values: List<T>, start: Int, end: Int): BstNode<T>? {
            if (start >= end) return null

            val mid = start + (end - start) / 2
            return BstNode.create(
                values[mid],
                buildBalanced(values, start, mid),
                buildBalanced(values, mid + 1, end),
            )
        }

        private fun <T : Comparable<T>> inOrder(root: BstNode<T>?, output: MutableList<T>) {
            if (root == null) return
            inOrder(root.left, output)
            output += root.value
            inOrder(root.right, output)
        }

        private fun <T : Comparable<T>> validate(
            root: BstNode<T>?,
            minimum: T?,
            maximum: T?,
        ): Validation? {
            if (root == null) return Validation(-1, 0)
            if (minimum != null && root.value <= minimum) return null
            if (maximum != null && root.value >= maximum) return null

            val left = validate(root.left, minimum, root.value) ?: return null
            val right = validate(root.right, root.value, maximum) ?: return null
            val expectedSize = 1 + left.size + right.size
            if (root.size != expectedSize) return null
            return Validation(1 + maxOf(left.height, right.height), expectedSize)
        }

        private data class Extracted<T : Comparable<T>>(val root: BstNode<T>?, val minimum: T)

        private data class Validation(val height: Int, val size: Int)
    }
}
