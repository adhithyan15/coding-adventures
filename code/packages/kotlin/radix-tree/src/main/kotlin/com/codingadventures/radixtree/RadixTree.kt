package com.codingadventures.radixtree

import java.util.TreeMap

class RadixTree<V> {
    private val root = Node<V>()
    private var size = 0

    fun insert(key: String, value: V) {
        if (insertRecursive(root, key, value)) size += 1
    }

    fun search(key: String): V? {
        var node = root
        var remaining = key
        while (remaining.isNotEmpty()) {
            val edge = node.children[firstChar(remaining)] ?: return null
            val commonLength = commonPrefixLength(remaining, edge.label)
            if (commonLength < edge.label.length) return null
            remaining = remaining.substring(commonLength)
            node = edge.child
        }
        return if (node.isEnd) node.value else null
    }

    fun containsKey(key: String): Boolean = search(key) != null || keyExists(key)

    fun delete(key: String): Boolean {
        val result = deleteRecursive(root, key)
        if (result.deleted) size -= 1
        return result.deleted
    }

    fun startsWith(prefix: String): Boolean {
        if (prefix.isEmpty()) return size > 0
        var node = root
        var remaining = prefix
        while (remaining.isNotEmpty()) {
            val edge = node.children[firstChar(remaining)] ?: return false
            val commonLength = commonPrefixLength(remaining, edge.label)
            if (commonLength == remaining.length) return true
            if (commonLength < edge.label.length) return false
            remaining = remaining.substring(commonLength)
            node = edge.child
        }
        return node.isEnd || node.children.isNotEmpty()
    }

    fun wordsWithPrefix(prefix: String): List<String> {
        var node = root
        var remaining = prefix
        val path = StringBuilder()

        if (remaining.isEmpty()) {
            return buildList { collectKeys(root, "", this) }
        }

        while (remaining.isNotEmpty()) {
            val edge = node.children[firstChar(remaining)] ?: return emptyList()
            val commonLength = commonPrefixLength(remaining, edge.label)
            if (commonLength == remaining.length) {
                if (commonLength == edge.label.length) {
                    path.append(edge.label)
                    node = edge.child
                    remaining = ""
                } else {
                    return buildList { collectKeys(edge.child, path.toString() + edge.label, this) }
                }
            } else if (commonLength < edge.label.length) {
                return emptyList()
            } else {
                path.append(edge.label)
                remaining = remaining.substring(commonLength)
                node = edge.child
            }
        }

        return buildList { collectKeys(node, path.toString(), this) }
    }

    fun longestPrefixMatch(key: String): String? {
        var node = root
        var remaining = key
        var consumed = 0
        var best: String? = if (node.isEnd) "" else null

        while (remaining.isNotEmpty()) {
            val edge = node.children[firstChar(remaining)] ?: break
            val commonLength = commonPrefixLength(remaining, edge.label)
            if (commonLength < edge.label.length) break
            consumed += commonLength
            remaining = remaining.substring(commonLength)
            node = edge.child
            if (node.isEnd) best = key.substring(0, consumed)
        }

        return best
    }

    fun toMap(): Map<String, V> = sortedMapOf<String, V>().also { collectValues(root, "", it) }
    fun keys(): List<String> = buildList { collectKeys(root, "", this) }
    fun len(): Int = size
    fun isEmpty(): Boolean = size == 0
    fun nodeCount(): Int = countNodes(root)

    override fun toString(): String = "RadixTree($size keys: ${toMap().entries.take(5)})"

    private fun keyExists(key: String): Boolean {
        var node = root
        var remaining = key
        while (remaining.isNotEmpty()) {
            val edge = node.children[firstChar(remaining)] ?: return false
            val commonLength = commonPrefixLength(remaining, edge.label)
            if (commonLength < edge.label.length) return false
            remaining = remaining.substring(commonLength)
            node = edge.child
        }
        return node.isEnd
    }

    private fun insertRecursive(node: Node<V>, key: String, value: V): Boolean {
        if (key.isEmpty()) {
            val added = !node.isEnd
            node.isEnd = true
            node.value = value
            return added
        }

        val first = firstChar(key)
        val edge = node.children.remove(first)
        if (edge == null) {
            node.children[first] = Edge(key, Node.leaf(value))
            return true
        }

        val commonLength = commonPrefixLength(key, edge.label)
        if (commonLength == edge.label.length) {
            val added = insertRecursive(edge.child, key.substring(commonLength), value)
            node.children[first] = edge
            return added
        }

        val common = edge.label.substring(0, commonLength)
        val labelRest = edge.label.substring(commonLength)
        val keyRest = key.substring(commonLength)
        val splitNode = Node<V>()
        splitNode.children[firstChar(labelRest)] = Edge(labelRest, edge.child)

        if (keyRest.isEmpty()) {
            splitNode.isEnd = true
            splitNode.value = value
        } else {
            splitNode.children[firstChar(keyRest)] = Edge(keyRest, Node.leaf(value))
        }

        node.children[firstChar(common)] = Edge(common, splitNode)
        return true
    }

    private fun deleteRecursive(node: Node<V>, key: String): DeleteResult {
        if (key.isEmpty()) {
            if (!node.isEnd) return DeleteResult(false, false)
            node.isEnd = false
            node.value = null
            return DeleteResult(true, !node.isEnd && node.children.size == 1)
        }

        val first = firstChar(key)
        val edge = node.children.remove(first) ?: return DeleteResult(false, false)
        val commonLength = commonPrefixLength(key, edge.label)
        if (commonLength < edge.label.length) {
            node.children[first] = edge
            return DeleteResult(false, false)
        }

        val result = deleteRecursive(edge.child, key.substring(commonLength))
        if (!result.deleted) {
            node.children[first] = edge
            return result
        }

        when {
            result.childMergeable -> {
                val grandchild = edge.child.children.firstEntry().value
                val mergedLabel = edge.label + grandchild.label
                node.children[firstChar(mergedLabel)] = Edge(mergedLabel, grandchild.child)
            }
            !edge.child.isEnd && edge.child.children.isEmpty() -> Unit
            else -> node.children[first] = edge
        }

        return DeleteResult(true, !node.isEnd && node.children.size == 1)
    }

    private fun collectKeys(node: Node<V>, current: String, results: MutableList<String>) {
        if (node.isEnd) results += current
        node.children.values.forEach { edge -> collectKeys(edge.child, current + edge.label, results) }
    }

    private fun collectValues(node: Node<V>, current: String, results: MutableMap<String, V>) {
        if (node.isEnd) results[current] = node.value as V
        node.children.values.forEach { edge -> collectValues(edge.child, current + edge.label, results) }
    }

    private fun countNodes(node: Node<V>): Int = 1 + node.children.values.sumOf { countNodes(it.child) }

    private fun commonPrefixLength(left: String, right: String): Int {
        var index = 0
        val limit = minOf(left.length, right.length)
        while (index < limit && left[index] == right[index]) index += 1
        return index
    }

    private fun firstChar(value: String): Char = value[0]

    private class Node<V> {
        var isEnd = false
        var value: V? = null
        val children = TreeMap<Char, Edge<V>>()

        companion object {
            fun <V> leaf(value: V): Node<V> = Node<V>().also {
                it.isEnd = true
                it.value = value
            }
        }
    }

    private data class Edge<V>(val label: String, val child: Node<V>)
    private data class DeleteResult(val deleted: Boolean, val childMergeable: Boolean)
}
