// ============================================================================
// HuffmanTree.java — Optimal Prefix-Free Code Tree
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
//   3. Among leaves of equal weight, lower symbol value wins (symbolOrNeg1).
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
//          code[i] = (code[i-1] + 1) << (length[i] - length[i-1])
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
//       A → 0       (length 1, code = 0)
//       B → 10      (length 2, code = 0+1=1, shifted 1 bit → 10)
//       C → 11      (length 2, code = 10+1 = 11)
//
// ============================================================================

package com.codingadventures.huffmantree;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.PriorityQueue;

/**
 * A Huffman tree that assigns optimal prefix-free bit codes to integer symbols.
 *
 * <p>Build the tree once from symbol frequencies; then:
 * <ul>
 *   <li>Use {@link #codeTable()} to get a {@code {symbol → bit_string}} map for encoding.</li>
 *   <li>Use {@link #decodeAll(String, int)} to decode a bit stream back to symbols.</li>
 *   <li>Use {@link #canonicalCodeTable()} for DEFLATE-style transmissible codes.</li>
 * </ul>
 *
 * <p>All symbols are non-negative integers (typically 0–255 for byte-level
 * coding, but any non-negative integer is valid).  Frequencies must be positive.
 *
 * <p>The tree is immutable after construction.
 *
 * <pre>{@code
 * HuffmanTree tree = HuffmanTree.build(List.of(
 *     new int[]{65, 3},  // 'A' → frequency 3
 *     new int[]{66, 2},  // 'B' → frequency 2
 *     new int[]{67, 1}   // 'C' → frequency 1
 * ));
 *
 * Map<Integer, String> table = tree.codeTable();
 * // table.get(65) → "0"   (A gets the shortest code)
 * // table.get(66) → "10"
 * // table.get(67) → "11"
 *
 * List<Integer> decoded = tree.decodeAll("010011", 4);
 * // → [65, 65, 66, 67]  == [A, A, B, C]
 * }</pre>
 */
public final class HuffmanTree {

    // =========================================================================
    // Node types
    // =========================================================================

    /**
     * A leaf node representing a single symbol.
     *
     * <p>Leaves are always at the bottom of the tree; they carry the actual
     * encoded symbol.  Their code is the path from the root to this node.
     */
    static final class Leaf extends Node {
        final int symbol;

        Leaf(int symbol, int weight) {
            super(weight);
            this.symbol = symbol;
        }
    }

    /**
     * An internal node combining two sub-trees.
     *
     * <p>Internal nodes have no symbol — they are bookkeeping nodes that
     * encode branching structure.  Their weight is the sum of their children's
     * weights.
     */
    static final class Internal extends Node {
        final Node  left;
        final Node  right;
        final int   order;   // Insertion order for tie-breaking (FIFO for internals)

        Internal(int weight, Node left, Node right, int order) {
            super(weight);
            this.left  = left;
            this.right = right;
            this.order = order;
        }
    }

    /**
     * Common base for Leaf and Internal.
     *
     * <p>Holds only the weight so both node types share the same field.
     */
    static abstract class Node {
        final int weight;

        Node(int weight) {
            this.weight = weight;
        }
    }

    // =========================================================================
    // Priority comparator
    // =========================================================================

    /**
     * Comparator for the min-heap during tree construction.
     *
     * <p>Four-level ordering (lower = higher priority):
     * <ol>
     *   <li>Weight (lighter wins)</li>
     *   <li>Node type (leaf=0 before internal=1)</li>
     *   <li>Symbol value for leaves (lower symbol wins)</li>
     *   <li>Insertion order for internals (earlier wins = FIFO)</li>
     * </ol>
     */
    private static final Comparator<Node> PRIORITY = (a, b) -> {
        // Field 0: weight
        int cmp = Integer.compare(a.weight, b.weight);
        if (cmp != 0) return cmp;

        // Field 1: leaf (0) before internal (1)
        int typeA = (a instanceof Leaf) ? 0 : 1;
        int typeB = (b instanceof Leaf) ? 0 : 1;
        cmp = Integer.compare(typeA, typeB);
        if (cmp != 0) return cmp;

        // Field 2 / 3: among same type
        if (a instanceof Leaf la && b instanceof Leaf lb) {
            // Lower symbol wins
            return Integer.compare(la.symbol, lb.symbol);
        } else {
            // Both internal: earlier-created wins (FIFO)
            return Integer.compare(((Internal) a).order, ((Internal) b).order);
        }
    };

    // =========================================================================
    // Fields
    // =========================================================================

    private final Node root;
    private final int  symbolCount;

    // =========================================================================
    // Constructor (private — use build())
    // =========================================================================

    private HuffmanTree(Node root, int symbolCount) {
        this.root        = root;
        this.symbolCount = symbolCount;
    }

    // =========================================================================
    // Public factory
    // =========================================================================

    /**
     * Construct a Huffman tree from {@code (symbol, frequency)} pairs.
     *
     * <p>The greedy algorithm uses a min-heap.  At each step it pops the two
     * lowest-weight nodes, combines them into a new internal node, and pushes
     * the result back.  The single remaining node is the root.
     *
     * <p>Tie-breaking (for deterministic output across implementations):
     * <ol>
     *   <li>Lowest weight pops first.</li>
     *   <li>Leaves before internal nodes at equal weight.</li>
     *   <li>Lower symbol value wins among leaves of equal weight.</li>
     *   <li>Earlier-created internal node wins among internals of equal weight
     *       (FIFO insertion order).</li>
     * </ol>
     *
     * @param weights a list of {@code int[2]} arrays: {@code [symbol, frequency]}.
     *                Each symbol must be a non-negative integer; each frequency
     *                must be &gt; 0.
     * @return a {@code HuffmanTree} ready for encoding/decoding
     * @throws IllegalArgumentException if {@code weights} is empty, or if any
     *                                  frequency is ≤ 0
     */
    public static HuffmanTree build(List<int[]> weights) {
        if (weights == null || weights.isEmpty()) {
            throw new IllegalArgumentException("weights must not be empty");
        }
        for (int[] pair : weights) {
            if (pair[1] <= 0) {
                throw new IllegalArgumentException(
                    "frequency must be positive; got symbol=" + pair[0] + ", freq=" + pair[1]
                );
            }
        }

        PriorityQueue<Node> heap = new PriorityQueue<>(weights.size(), PRIORITY);
        for (int[] pair : weights) {
            heap.add(new Leaf(pair[0], pair[1]));
        }

        int orderCounter = 0;
        while (heap.size() > 1) {
            Node left  = heap.poll();
            Node right = heap.poll();
            heap.add(new Internal(left.weight + right.weight, left, right, orderCounter++));
        }

        return new HuffmanTree(heap.poll(), weights.size());
    }

    // =========================================================================
    // Encoding helpers
    // =========================================================================

    /**
     * Return a {@code {symbol → bit_string}} map for all symbols in the tree.
     *
     * <p>Left edges are {@code '0'}, right edges are {@code '1'}.  For a
     * single-symbol tree the convention is {@code {symbol → "0"}} (one bit
     * per occurrence).
     *
     * <p>Time: O(n) where n = number of distinct symbols.
     *
     * <pre>{@code
     * HuffmanTree tree = HuffmanTree.build(List.of(
     *     new int[]{65, 3}, new int[]{66, 2}, new int[]{67, 1}
     * ));
     * tree.codeTable().get(65); // "0"
     * tree.codeTable().get(66); // "10"
     * tree.codeTable().get(67); // "11"
     * }</pre>
     *
     * @return map from symbol to its bit string
     */
    public Map<Integer, String> codeTable() {
        Map<Integer, String> table = new HashMap<>();
        walkTree(root, "", table);
        return table;
    }

    /**
     * Return the bit string for a specific symbol, or {@code null} if not in
     * the tree.
     *
     * <p>Walks the tree searching for the leaf with the given symbol.  Does NOT
     * build the full code table.
     *
     * <p>Time: O(n) worst case (full tree traversal).
     *
     * @param symbol the symbol to look up
     * @return the bit string, or {@code null} if absent
     */
    public String codeFor(int symbol) {
        return findCode(root, symbol, "");
    }

    /**
     * Return canonical Huffman codes (DEFLATE-style).
     *
     * <p>Sorted by {@code (code_length, symbol_value)}; codes assigned
     * numerically.  Canonical codes allow transmitting only code lengths, not
     * the tree structure, which saves space.
     *
     * <p>Time: O(n log n).
     *
     * <pre>{@code
     * tree.canonicalCodeTable().get(65); // "0"
     * tree.canonicalCodeTable().get(66); // "10"
     * tree.canonicalCodeTable().get(67); // "11"
     * }</pre>
     *
     * @return map from symbol to its canonical bit string
     */
    public Map<Integer, String> canonicalCodeTable() {
        // Step 1: collect code lengths from tree
        Map<Integer, Integer> lengths = new HashMap<>();
        collectLengths(root, 0, lengths);

        // Single-leaf edge case: assign length 1 by convention
        if (lengths.size() == 1) {
            int sym = lengths.keySet().iterator().next();
            Map<Integer, String> result = new HashMap<>();
            result.put(sym, "0");
            return result;
        }

        // Step 2: sort by (length, symbol)
        List<Map.Entry<Integer, Integer>> sorted = new ArrayList<>(lengths.entrySet());
        sorted.sort((a, b) -> {
            int cmp = Integer.compare(a.getValue(), b.getValue());
            return cmp != 0 ? cmp : Integer.compare(a.getKey(), b.getKey());
        });

        // Step 3: assign canonical codes numerically
        Map<Integer, String> result = new LinkedHashMap<>();
        int codeVal  = 0;
        int prevLen  = sorted.get(0).getValue();

        for (Map.Entry<Integer, Integer> entry : sorted) {
            int sym    = entry.getKey();
            int length = entry.getValue();
            if (length > prevLen) {
                codeVal <<= (length - prevLen);
            }
            // Format as binary string, left-padded with zeros to 'length' bits
            result.put(sym, String.format("%" + length + "s",
                Integer.toBinaryString(codeVal)).replace(' ', '0'));
            codeVal++;
            prevLen = length;
        }
        return result;
    }

    // =========================================================================
    // Decoding
    // =========================================================================

    /**
     * Decode exactly {@code count} symbols from a bit string by walking the tree.
     *
     * <p>For each symbol: follow the tree from the root, taking the left child
     * on '0' and the right child on '1', until a leaf is reached.  The leaf's
     * symbol is appended to the output; the walk restarts at the root.
     *
     * <p>For a single-leaf tree, each '0' bit decodes to that symbol.
     *
     * <p>Time: O(total bits consumed).
     *
     * <pre>{@code
     * tree.decodeAll("010011", 4); // → [65, 65, 66, 67]
     * }</pre>
     *
     * @param bits  a string of '0' and '1' characters
     * @param count the exact number of symbols to decode
     * @return a list of decoded symbols of length {@code count}
     * @throws IllegalArgumentException if the bit stream is exhausted before
     *                                  {@code count} symbols are decoded
     */
    public List<Integer> decodeAll(String bits, int count) {
        List<Integer> result  = new ArrayList<>(count);
        Node          current = root;
        int           i       = 0;
        boolean       singleLeaf = (root instanceof Leaf);

        while (result.size() < count) {
            if (current instanceof Leaf leaf) {
                result.add(leaf.symbol);
                current = root;
                if (singleLeaf) {
                    // Consume the '0' bit for this symbol
                    if (i < bits.length()) i++;
                }
                continue;
            }

            if (i >= bits.length()) {
                throw new IllegalArgumentException(
                    "Bit stream exhausted after " + result.size() +
                    " symbols; expected " + count
                );
            }

            char bit = bits.charAt(i++);
            Internal internal = (Internal) current;
            current = (bit == '0') ? internal.left : internal.right;
        }

        return result;
    }

    // =========================================================================
    // Inspection
    // =========================================================================

    /**
     * Total weight of the tree = sum of all leaf frequencies = root weight.
     *
     * <p>O(1) — stored at the root.
     *
     * @return sum of all symbol frequencies
     */
    public int weight() {
        return root.weight;
    }

    /**
     * Maximum code length = depth of the deepest leaf.
     *
     * <p>O(n) — must traverse the tree.
     *
     * @return depth of the deepest leaf (0 for a single-symbol tree)
     */
    public int depth() {
        return maxDepth(root, 0);
    }

    /**
     * Number of distinct symbols = number of leaf nodes.
     *
     * <p>O(1) — stored at construction time.
     *
     * @return number of distinct symbols
     */
    public int symbolCount() {
        return symbolCount;
    }

    /**
     * In-order traversal of leaves.
     *
     * <p>Returns {@code (symbol, code)} pairs, left subtree before right
     * subtree.  Useful for visualisation and debugging.
     *
     * <p>Time: O(n).
     *
     * @return list of {@code int[2]} arrays: {@code [symbol, unused]}, paired
     *         with a String; actually returned as pairs via a 2D structure.
     *         Each element is a {@code int[]{symbol}} and the code string.
     */
    public List<int[]> leaves() {
        Map<Integer, String> table = codeTable();
        List<int[]> result = new ArrayList<>();
        // We store symbol and code-string together as a parallel pair.
        // Return as a list of [symbol, codeAsInt? — no, code is a string].
        // Use a separate method that returns the paired list.
        collectLeavesInOrder(root, result, table);
        return result;
    }

    /**
     * Return leaves as {@code (symbol, bitString)} pairs in in-order (left-to-right)
     * traversal order.
     *
     * <p>Time: O(n).
     *
     * @return list of {@code Object[]{Integer symbol, String code}} pairs
     */
    public List<Object[]> leavesWithCodes() {
        Map<Integer, String> table = codeTable();
        List<Object[]> result = new ArrayList<>();
        collectLeavesInOrderWithCodes(root, result, table);
        return result;
    }

    /**
     * Check structural invariants.
     *
     * <ol>
     *   <li>Every internal node has exactly 2 children (full binary tree).</li>
     *   <li>{@code weight(internal) == weight(left) + weight(right)}.</li>
     *   <li>No symbol appears in more than one leaf.</li>
     * </ol>
     *
     * <p>Returns {@code true} if all invariants hold.
     *
     * @return true if the tree is structurally valid
     */
    public boolean isValid() {
        java.util.Set<Integer> seen = new java.util.HashSet<>();
        return checkInvariants(root, seen);
    }

    // =========================================================================
    // Private helpers — tree walk
    // =========================================================================

    /**
     * Recursively walk the tree building the code table.
     *
     * <p>At each internal node we append '0' for the left branch and '1' for
     * the right branch.  At each leaf we record the accumulated prefix.
     */
    private static void walkTree(Node node, String prefix, Map<Integer, String> table) {
        if (node instanceof Leaf leaf) {
            // Single-leaf edge case: no edges traversed; use "0" by convention
            table.put(leaf.symbol, prefix.isEmpty() ? "0" : prefix);
            return;
        }
        Internal internal = (Internal) node;
        walkTree(internal.left,  prefix + "0", table);
        walkTree(internal.right, prefix + "1", table);
    }

    /**
     * Search the tree for a specific symbol, returning its code or {@code null}.
     */
    private static String findCode(Node node, int symbol, String prefix) {
        if (node instanceof Leaf leaf) {
            if (leaf.symbol == symbol) {
                return prefix.isEmpty() ? "0" : prefix;
            }
            return null;
        }
        Internal internal = (Internal) node;
        String left = findCode(internal.left,  symbol, prefix + "0");
        if (left != null) return left;
        return findCode(internal.right, symbol, prefix + "1");
    }

    /**
     * Collect code lengths for all leaves (depth at each leaf = code length).
     *
     * <p>For a single-leaf tree (depth 0) we assign length 1 by convention.
     */
    private static void collectLengths(Node node, int depth, Map<Integer, Integer> lengths) {
        if (node instanceof Leaf leaf) {
            lengths.put(leaf.symbol, depth > 0 ? depth : 1);
            return;
        }
        Internal internal = (Internal) node;
        collectLengths(internal.left,  depth + 1, lengths);
        collectLengths(internal.right, depth + 1, lengths);
    }

    /** Return the maximum depth of any leaf. */
    private static int maxDepth(Node node, int depth) {
        if (node instanceof Leaf) return depth;
        Internal internal = (Internal) node;
        return Math.max(maxDepth(internal.left,  depth + 1),
                        maxDepth(internal.right, depth + 1));
    }

    /** Collect leaf symbols in left-to-right (in-order) traversal. */
    private static void collectLeavesInOrder(
            Node node, List<int[]> result, Map<Integer, String> table) {
        if (node instanceof Leaf leaf) {
            result.add(new int[]{leaf.symbol});
            return;
        }
        Internal internal = (Internal) node;
        collectLeavesInOrder(internal.left,  result, table);
        collectLeavesInOrder(internal.right, result, table);
    }

    /** Collect (symbol, code) pairs in left-to-right in-order traversal. */
    private static void collectLeavesInOrderWithCodes(
            Node node, List<Object[]> result, Map<Integer, String> table) {
        if (node instanceof Leaf leaf) {
            result.add(new Object[]{leaf.symbol, table.get(leaf.symbol)});
            return;
        }
        Internal internal = (Internal) node;
        collectLeavesInOrderWithCodes(internal.left,  result, table);
        collectLeavesInOrderWithCodes(internal.right, result, table);
    }

    /** Recursively validate tree invariants. */
    private static boolean checkInvariants(Node node, java.util.Set<Integer> seen) {
        if (node instanceof Leaf leaf) {
            if (seen.contains(leaf.symbol)) return false;
            seen.add(leaf.symbol);
            return true;
        }
        Internal internal = (Internal) node;
        // Internal node weight must equal sum of children's weights
        if (internal.weight != internal.left.weight + internal.right.weight) {
            return false;
        }
        return checkInvariants(internal.left, seen) &&
               checkInvariants(internal.right, seen);
    }
}
