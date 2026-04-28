// ============================================================================
// HuffmanTree.kt — Optimal Prefix-Free Code Tree
// ============================================================================
//
// A Huffman tree is a full binary tree (every internal node has exactly two
// children) built from a symbol alphabet so that each symbol gets a unique
// variable-length bit code.  Symbols that appear often get short codes;
// symbols that appear rarely get long codes.  The total bits needed to
// encode a message is minimised — it is the theoretically optimal prefix-free
// code for a given symbol frequency distribution.
//
// Think of it like Morse code.  In Morse, 'E' is '.' (one dot) and 'Z' is
// '--..' (four symbols).  The designers knew 'E' is the most common letter
// in English so they gave it the shortest code.  Huffman's algorithm does
// this automatically and optimally for any alphabet with any frequency
// distribution.
//
// ============================================================================
// Algorithm: Greedy construction via min-heap
// ============================================================================
//
// 1. Create one leaf node per distinct symbol, each with its frequency as
//    its weight.  Push all leaves onto a min-heap keyed by weight.
//
// 2. While the heap has more than one node:
//      a. Pop the two nodes with the smallest weight.
//      b. Create a new internal node whose weight = sum of the two children.
//      c. Set left = the first popped node, right = the second popped node.
//      d. Push the new internal node back onto the heap.
//
// 3. The one remaining node is the root of the Huffman tree.
//
// ============================================================================
// Tie-breaking for determinism
// ============================================================================
//
// Without tie-breaking, different implementations build structurally different
// trees from the same input — producing different (but equally valid) codes.
// We enforce these rules (lower key = higher priority):
//
//   Priority tuple: (weight, isInternal, symbolOrNeg1, insertionOrder)
//
//   1. Lowest weight pops first.
//   2. Leaf nodes (isInternal=0) beat internal nodes (isInternal=1) at equal
//      weight.
//   3. Among leaves of equal weight, lower symbol value wins.
//   4. Among internal nodes of equal weight, earlier-created node wins
//      (insertion order, FIFO).
//
// ============================================================================
// Prefix-free property: why it works
// ============================================================================
//
//   In a Huffman tree:
//     - Symbols live ONLY at the leaves, never at internal nodes.
//     - The code for a symbol is the path from root to its leaf
//       (left edge = '0', right edge = '1').
//
//   Since one leaf is never an ancestor of another, no code can be a prefix
//   of another code.  This means the bit stream can be decoded unambiguously
//   without separator characters: just walk the tree bit by bit until you
//   reach a leaf.
//
// ============================================================================
// Canonical codes (DEFLATE / zlib style)
// ============================================================================
//
//   The standard tree-walk produces valid codes, but different tree shapes
//   can produce different codes for the same symbol lengths.  Canonical codes
//   normalise this: given only the code *lengths*, you can reconstruct the
//   exact canonical code table without transmitting the tree structure.
//
//   Algorithm:
//     1. Collect (symbol, code_length) pairs from the tree.
//     2. Sort by (code_length, symbol_value).
//     3. Assign codes numerically:
//          code[0] = 0  (left-padded to length[0] bits)
//          code[i] = (code[i-1] + 1) shl (length[i] - length[i-1])
//
//   This is exactly what DEFLATE uses: the compressed stream contains only
//   the length table, not the tree, saving space.
//
//   Example with AAABBC (A=3, B=2, C=1):
//     Tree:      [6]
//                / \
//               A  [3]
//              (3)  / \
//                  B   C
//                 (2) (1)
//     Lengths: A=1, B=2, C=2
//     Sorted by (length, symbol): A(1), B(2), C(2)
//     Canonical codes:
//       A → "0"   (length 1, code = 0)
//       B → "10"  (length 2, code = 0+1=1, shifted 1 bit → 10)
//       C → "11"  (length 2, code = 10+1 = 11)
//
// ============================================================================

package com.codingadventures.huffmantree

import java.util.PriorityQueue

// ============================================================================
// Node types
// ============================================================================

/**
 * Base class for all nodes in the Huffman tree.
 *
 * Every node carries a [weight]: for leaves it is the symbol's frequency;
 * for internal nodes it is the sum of its children's weights.
 */
sealed class Node(open val weight: Int)

/**
 * A leaf node representing a single symbol.
 *
 * Leaves are always at the bottom of the tree.  Their code is the
 * path from the root to this node (left='0', right='1').
 *
 * @param symbol   the encoded symbol (non-negative integer)
 * @param weight   the symbol's frequency
 */
data class Leaf(val symbol: Int, override val weight: Int) : Node(weight)

/**
 * An internal node combining two sub-trees.
 *
 * Internal nodes have no symbol — they are bookkeeping nodes encoding
 * branching structure.  Their [weight] = [left].weight + [right].weight.
 *
 * @param weight  sum of children's weights
 * @param left    left child (code bit '0')
 * @param right   right child (code bit '1')
 * @param order   monotonic insertion counter for tie-breaking (FIFO among internals)
 */
data class Internal(
    override val weight: Int,
    val left:   Node,
    val right:  Node,
    val order:  Int = 0
) : Node(weight)

// ============================================================================
// HuffmanTree
// ============================================================================

/**
 * A Huffman tree that assigns optimal prefix-free bit codes to integer symbols.
 *
 * Build the tree once from symbol frequencies; then:
 * - Use [codeTable] to get a `{symbol → bit_string}` map for encoding.
 * - Use [decodeAll] to decode a bit stream back to symbols.
 * - Use [canonicalCodeTable] for DEFLATE-style transmissible codes.
 *
 * All symbols are non-negative integers (typically 0–255 for byte-level
 * coding, but any non-negative integer is valid).  Frequencies must be
 * positive.  The tree is immutable after construction.
 *
 * ```kotlin
 * val tree = HuffmanTree.build(listOf(
 *     intArrayOf(65, 3),  // 'A' → frequency 3
 *     intArrayOf(66, 2),  // 'B' → frequency 2
 *     intArrayOf(67, 1)   // 'C' → frequency 1
 * ))
 *
 * val table = tree.codeTable()
 * // table[65] → "0"    (A gets the shortest code)
 * // table[66] → "10"
 * // table[67] → "11"
 *
 * val decoded = tree.decodeAll("010011", 4)
 * // → [65, 65, 66, 67]
 * ```
 */
class HuffmanTree private constructor(
    private val root:        Node,
    private val symbolCount: Int
) {

    // =========================================================================
    // Companion (factory)
    // =========================================================================

    companion object {

        /**
         * Comparator for the min-heap during tree construction.
         *
         * Four-level ordering (lower = higher priority):
         * 1. Weight (lighter wins)
         * 2. Node type: leaf (0) before internal (1)
         * 3. Symbol value for leaves (lower wins)
         * 4. Insertion order for internals (earlier = FIFO)
         */
        private val PRIORITY = Comparator<Node> { a, b ->
            // Field 0: weight
            a.weight.compareTo(b.weight).takeIf { it != 0 }?.let { return@Comparator it }

            // Field 1: leaf (0) before internal (1)
            val typeA = if (a is Leaf) 0 else 1
            val typeB = if (b is Leaf) 0 else 1
            typeA.compareTo(typeB).takeIf { it != 0 }?.let { return@Comparator it }

            // Field 2 / 3: among same type
            when {
                a is Leaf && b is Leaf -> a.symbol.compareTo(b.symbol)
                a is Internal && b is Internal -> a.order.compareTo(b.order)
                else -> 0
            }
        }

        /**
         * Construct a Huffman tree from `(symbol, frequency)` pairs.
         *
         * Each element of [weights] must be an `IntArray` of size ≥ 2:
         * `[symbol, frequency, ...]`.  Symbols must be non-negative integers;
         * frequencies must be > 0.
         *
         * Tie-breaking (for deterministic output):
         * 1. Lowest weight pops first.
         * 2. Leaves before internal nodes at equal weight.
         * 3. Lower symbol value wins among leaves of equal weight.
         * 4. Earlier-created internal wins among internals of equal weight (FIFO).
         *
         * @param weights list of `IntArray(symbol, frequency)` pairs
         * @return a [HuffmanTree] ready for encoding/decoding
         * @throws IllegalArgumentException if [weights] is empty or any frequency ≤ 0
         */
        fun build(weights: List<IntArray>): HuffmanTree {
            require(weights.isNotEmpty()) { "weights must not be empty" }
            for ((sym, freq) in weights.map { it[0] to it[1] }) {
                require(freq > 0) {
                    "frequency must be positive; got symbol=$sym, freq=$freq"
                }
            }

            val heap = PriorityQueue(weights.size, PRIORITY)
            for (pair in weights) heap.add(Leaf(pair[0], pair[1]))

            var orderCounter = 0
            while (heap.size > 1) {
                val left  = heap.poll()
                val right = heap.poll()
                heap.add(Internal(left.weight + right.weight, left, right, orderCounter++))
            }

            return HuffmanTree(heap.poll(), weights.size)
        }
    }

    // =========================================================================
    // Encoding helpers
    // =========================================================================

    /**
     * Return a `{symbol → bit_string}` map for all symbols in the tree.
     *
     * Left edges are `'0'`, right edges are `'1'`.  For a single-symbol tree
     * the convention is `{symbol → "0"}` (one bit per occurrence).
     *
     * Time: O(n) where n = number of distinct symbols.
     *
     * ```kotlin
     * tree.codeTable()[65]  // "0"
     * tree.codeTable()[66]  // "10"
     * tree.codeTable()[67]  // "11"
     * ```
     */
    fun codeTable(): Map<Int, String> {
        val table = mutableMapOf<Int, String>()
        walkTree(root, "", table)
        return table
    }

    /**
     * Return the bit string for [symbol], or `null` if not in the tree.
     *
     * Does NOT build the full code table — traverses only until found.
     *
     * Time: O(n) worst case.
     */
    fun codeFor(symbol: Int): String? = findCode(root, symbol, "")

    /**
     * Return canonical Huffman codes (DEFLATE-style).
     *
     * Sorted by `(code_length, symbol_value)`; codes assigned numerically.
     * Allows transmitting only code lengths, not the tree structure.
     *
     * Time: O(n log n).
     */
    fun canonicalCodeTable(): Map<Int, String> {
        val lengths = mutableMapOf<Int, Int>()
        collectLengths(root, 0, lengths)

        // Single-leaf edge case
        if (lengths.size == 1) {
            val sym = lengths.keys.first()
            return mapOf(sym to "0")
        }

        // Sort by (length, symbol)
        val sorted = lengths.entries.sortedWith(compareBy({ it.value }, { it.key }))

        val result  = mutableMapOf<Int, String>()
        var codeVal = 0
        var prevLen = sorted.first().value

        for ((sym, length) in sorted) {
            if (length > prevLen) codeVal = codeVal shl (length - prevLen)
            result[sym] = Integer.toBinaryString(codeVal).padStart(length, '0')
            codeVal++
            prevLen = length
        }
        return result
    }

    // =========================================================================
    // Decoding
    // =========================================================================

    /**
     * Decode exactly [count] symbols from a bit string by walking the tree.
     *
     * For each symbol: follow the tree from the root, taking the left child
     * on '0' and the right child on '1', until a leaf is reached.  The leaf's
     * symbol is appended; the walk restarts at the root.
     *
     * For a single-leaf tree, each '0' bit decodes to that symbol.
     *
     * Time: O(total bits consumed).
     *
     * @throws IllegalArgumentException if the bit stream is exhausted before
     *                                  [count] symbols are decoded
     */
    fun decodeAll(bits: String, count: Int): List<Int> {
        val result     = mutableListOf<Int>()
        var current    = root
        var i          = 0
        val singleLeaf = root is Leaf

        while (result.size < count) {
            if (current is Leaf) {
                result.add(current.symbol)
                current = root
                if (singleLeaf && i < bits.length) i++
                continue
            }

            if (i >= bits.length) {
                throw IllegalArgumentException(
                    "Bit stream exhausted after ${result.size} symbols; expected $count"
                )
            }

            val bit = bits[i++]
            val internal = current as Internal
            current = if (bit == '0') internal.left else internal.right
        }

        return result
    }

    // =========================================================================
    // Inspection
    // =========================================================================

    /** Total weight = sum of all leaf frequencies = root weight. O(1). */
    fun weight(): Int = root.weight

    /** Maximum code length = depth of the deepest leaf. O(n). */
    fun depth(): Int = maxDepth(root, 0)

    /** Number of distinct symbols. O(1). */
    fun symbolCount(): Int = symbolCount

    /**
     * In-order list of `(symbol, code)` pairs (left subtree before right).
     *
     * Useful for visualisation and debugging.
     *
     * Time: O(n).
     */
    fun leavesWithCodes(): List<Pair<Int, String>> {
        val table  = codeTable()
        val result = mutableListOf<Pair<Int, String>>()
        collectLeavesInOrder(root, result, table)
        return result
    }

    /**
     * Check structural invariants:
     * 1. Every internal node has exactly 2 children (full binary tree).
     * 2. `weight(internal) == weight(left) + weight(right)`.
     * 3. No symbol appears in more than one leaf.
     *
     * @return true if all invariants hold
     */
    fun isValid(): Boolean {
        val seen = mutableSetOf<Int>()
        return checkInvariants(root, seen)
    }

    // =========================================================================
    // Private helpers — tree walk
    // =========================================================================

    private fun walkTree(node: Node, prefix: String, table: MutableMap<Int, String>) {
        when (node) {
            is Leaf     -> table[node.symbol] = if (prefix.isEmpty()) "0" else prefix
            is Internal -> {
                walkTree(node.left,  prefix + "0", table)
                walkTree(node.right, prefix + "1", table)
            }
        }
    }

    private fun findCode(node: Node, symbol: Int, prefix: String): String? = when (node) {
        is Leaf     -> if (node.symbol == symbol) (if (prefix.isEmpty()) "0" else prefix) else null
        is Internal -> findCode(node.left, symbol, prefix + "0")
                    ?: findCode(node.right, symbol, prefix + "1")
    }

    private fun collectLengths(node: Node, depth: Int, lengths: MutableMap<Int, Int>) {
        when (node) {
            is Leaf     -> lengths[node.symbol] = if (depth > 0) depth else 1
            is Internal -> {
                collectLengths(node.left,  depth + 1, lengths)
                collectLengths(node.right, depth + 1, lengths)
            }
        }
    }

    private fun maxDepth(node: Node, depth: Int): Int = when (node) {
        is Leaf     -> depth
        is Internal -> maxOf(maxDepth(node.left, depth + 1), maxDepth(node.right, depth + 1))
    }

    private fun collectLeavesInOrder(
        node: Node, result: MutableList<Pair<Int, String>>, table: Map<Int, String>
    ) {
        when (node) {
            is Leaf     -> result.add(node.symbol to table.getValue(node.symbol))
            is Internal -> {
                collectLeavesInOrder(node.left,  result, table)
                collectLeavesInOrder(node.right, result, table)
            }
        }
    }

    private fun checkInvariants(node: Node, seen: MutableSet<Int>): Boolean = when (node) {
        is Leaf -> {
            if (node.symbol in seen) false
            else { seen.add(node.symbol); true }
        }
        is Internal -> {
            if (node.weight != node.left.weight + node.right.weight) false
            else checkInvariants(node.left, seen) && checkInvariants(node.right, seen)
        }
    }
}
