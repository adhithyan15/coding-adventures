// ============================================================================
// Trie.kt — Prefix Tree (Trie)
// ============================================================================
//
// A trie (pronounced "try", from re**trie**val) is a tree where each path from
// root to a node spells out a string prefix. Unlike a hash map that treats keys
// as opaque, a trie decomposes each key character by character and shares common
// prefixes among all words that begin the same way.
//
// The fundamental idea:
// ---------------------
// Imagine the words ["app", "apple", "apply", "apt", "banana"]:
//
//       (root)
//       ├── a
//       │   └── p
//       │       ├── p (*)   ← "app" ends here
//       │       │   └── l
//       │       │       ├── e (*)  ← "apple"
//       │       │       └── y (*)  ← "apply"
//       │       └── t (*)   ← "apt"
//       └── b
//           └── a → n → a → n → a (*)  ← "banana"
//
//   (*) marks nodes where a complete word ends (isEnd = true).
//
// Complexity:
//   insert, get, contains, delete — O(len(key))
//   keysWithPrefix(pfx)           — O(len(pfx) + matches × avg key len)
//   size                          — O(1) (maintained as a counter)
//

package com.codingadventures.trie

/**
 * A generic Trie (prefix tree) that maps [String] keys to values of type [V].
 *
 * Keys are arbitrary strings — the trie works character by character and
 * generalises to any Unicode input.
 *
 * ```kotlin
 * val t = Trie<Int>()
 * t.insert("apple", 1)
 * t.insert("app", 2)
 * t.insert("apply", 3)
 *
 * println(t["apple"])              // 1
 * println(t.contains("app"))       // true
 * println(t.keysWithPrefix("app")) // [app, apple, apply]
 * println(t.size)                  // 3
 * ```
 */
class Trie<V> {

    // =========================================================================
    // Node
    // =========================================================================

    private inner class Node {
        val children: MutableMap<Char, Node> = HashMap()
        var isEnd: Boolean = false
        var value: V? = null
    }

    // =========================================================================
    // Fields
    // =========================================================================

    private val root = Node()
    private var _size = 0

    /** The number of key–value pairs currently in the trie. */
    val size: Int get() = _size

    /** True if the trie contains no keys. */
    val isEmpty: Boolean get() = _size == 0

    // =========================================================================
    // Core operations
    // =========================================================================

    /**
     * Insert [key] → [value] into the trie.
     *
     * If the key already exists, the value is updated and [size] is unchanged.
     *
     * @throws IllegalArgumentException if key is null (Kotlin guards this via non-nullable type)
     */
    fun insert(key: String, value: V?) {
        var node = root
        for (c in key) {
            node.children.getOrPut(c) { Node() }.also { node = it }
        }
        if (!node.isEnd) {
            node.isEnd = true
            _size++
        }
        node.value = value
    }

    /**
     * Return the value associated with [key], or `null` if the key is not in
     * the trie.
     *
     * Use [contains] to distinguish a stored `null` value from a missing key.
     */
    operator fun get(key: String): V? {
        val node = findNode(key) ?: return null
        return if (node.isEnd) node.value else null
    }

    /**
     * Return true if [key] was inserted as a complete key.
     *
     * Note: [startsWith] and [contains] are different — the latter requires
     * that the key was inserted, not merely that it is a prefix.
     */
    fun contains(key: String): Boolean {
        val node = findNode(key) ?: return false
        return node.isEnd
    }

    /**
     * Return true if any inserted key starts with [prefix].
     */
    fun startsWith(prefix: String): Boolean = findNode(prefix) != null

    /**
     * Delete [key] from the trie.
     *
     * Only the end-marker is removed; intermediate nodes shared with other
     * keys are preserved. Returns true if the key was present and removed.
     */
    fun delete(key: String): Boolean {
        val node = findNode(key) ?: return false
        if (!node.isEnd) return false
        node.isEnd = false
        node.value = null
        _size--
        return true
    }

    // =========================================================================
    // Prefix queries
    // =========================================================================

    /**
     * Return all keys that start with [prefix].
     *
     * Returns an empty list if no keys match.
     */
    fun keysWithPrefix(prefix: String): List<String> {
        val results = mutableListOf<String>()
        val node = findNode(prefix) ?: return results
        collectKeys(node, StringBuilder(prefix), results)
        return results
    }

    /**
     * Return all keys in the trie (equivalent to `keysWithPrefix("")`).
     */
    fun keys(): List<String> = keysWithPrefix("")

    // =========================================================================
    // Internal helpers
    // =========================================================================

    private fun findNode(key: String): Node? {
        var node = root
        for (c in key) {
            node = node.children[c] ?: return null
        }
        return node
    }

    private fun collectKeys(node: Node, prefix: StringBuilder, results: MutableList<String>) {
        if (node.isEnd) results.add(prefix.toString())
        for ((c, child) in node.children) {
            prefix.append(c)
            collectKeys(child, prefix, results)
            prefix.deleteCharAt(prefix.length - 1)
        }
    }
}
