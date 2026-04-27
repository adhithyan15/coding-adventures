// HuffmanTree.swift — DT27: Huffman Tree
// ============================================================================
//
// A Huffman tree is a full binary tree (every internal node has exactly two
// children) built from a symbol alphabet so that each symbol gets a unique
// variable-length bit code. Symbols that appear often get short codes;
// symbols that appear rarely get long codes. The total bits needed to encode
// a message is minimised — it is the theoretically optimal prefix-free code
// for a given symbol frequency distribution.
//
// Think of it like Morse code. In Morse, "E" is "." (one dot) and "Z" is
// "--.." (four symbols). The designers knew "E" is the most common letter in
// English so they gave it the shortest code. Huffman's algorithm does this
// automatically and optimally for any alphabet with any frequency distribution.
import Heap
//
// ============================================================
// Algorithm: Greedy construction via min-heap
// ============================================================
//
// 1. Create one leaf node per distinct symbol, each with its frequency as
//    weight. Push all leaves onto a min-heap keyed by priority tuple.
//
// 2. While the heap has more than one node:
//      a. Pop the two nodes with the smallest weight.
//      b. Create a new internal node whose weight = sum of the two children.
//      c. Set left = first popped, right = second popped.
//      d. Push the new internal node back onto the heap.
//
// 3. The one remaining node is the root of the Huffman tree.
//
// Tie-breaking rules (for deterministic output across implementations):
//   1. Lowest weight pops first.
//   2. Leaf nodes have higher priority than internal nodes at equal weight.
//   3. Among leaves of equal weight, lower symbol value wins.
//   4. Among internal nodes of equal weight, earlier-created node wins (FIFO).
//
// ============================================================
// Prefix-Free Property
// ============================================================
//
// Symbols live ONLY at leaves, never at internal nodes. The code for a symbol
// is the path from root to its leaf (left edge = "0", right edge = "1").
// Since one leaf is never an ancestor of another, no code can be a prefix of
// another code — the bit stream can be decoded unambiguously without separator
// characters: just walk the tree bit by bit until you hit a leaf.
//
// ============================================================
// Canonical Codes (DEFLATE / zlib style)
// ============================================================
//
// The standard tree-walk produces valid codes, but different tree shapes can
// produce different codes for the same symbol lengths. Canonical codes
// normalise this: given only the code *lengths*, you can reconstruct the
// exact canonical code table without transmitting the tree structure.
//
// Algorithm:
//   1. Collect (symbol, code_length) pairs from the tree.
//   2. Sort by (code_length, symbol_value).
//   3. Assign codes numerically:
//        code[0] = 0 (left-padded to length[0] bits)
//        code[i] = (code[i-1] + 1) << (length[i] - length[i-1])
//
// This is exactly what DEFLATE uses: the compressed stream contains only the
// length table, not the tree, saving space.
//
// ============================================================
// Heap Package
// ============================================================
//
// This package depends on the standalone Heap package for the generic min-heap
// used during greedy construction. Heap items are stored as `(PriorityKey, Node)`
// pairs so the Huffman-specific tie-breaking remains local to this module.
// ============================================================================

// MARK: - Priority Key

/// A 4-field priority key used to compare nodes during heap operations.
///
/// The four fields implement the tie-breaking rules in order:
///
///   1. `weight`      — lower weight = higher priority (pops first)
///   2. `leafFlag`    — 0 = leaf (higher priority), 1 = internal
///   3. `symbolOrMax` — leaf: symbol value; internal: Int.max
///   4. `orderOrMax`  — internal: insertion order (FIFO); leaf: Int.max
///
/// Comparison is lexicographic: first compare weight, on tie compare
/// leafFlag, on tie compare symbolOrMax, on tie compare orderOrMax.
private struct PriorityKey: Comparable {
    let weight:      Int
    let leafFlag:    Int   // 0 = leaf, 1 = internal
    let symbolOrMax: Int   // leaf symbol or Int.max
    let orderOrMax:  Int   // internal order or Int.max

    static func < (lhs: PriorityKey, rhs: PriorityKey) -> Bool {
        if lhs.weight      != rhs.weight      { return lhs.weight      < rhs.weight      }
        if lhs.leafFlag    != rhs.leafFlag    { return lhs.leafFlag    < rhs.leafFlag    }
        if lhs.symbolOrMax != rhs.symbolOrMax { return lhs.symbolOrMax < rhs.symbolOrMax }
        return lhs.orderOrMax < rhs.orderOrMax
    }
}

// MARK: - Tree Nodes

/// A single node in the Huffman tree.
///
/// Nodes are either leaves (holding a symbol) or internal (holding children).
/// We use an indirect enum so that nodes can recursively contain other nodes
/// without Swift needing to know the size at compile time — `indirect` tells
/// the compiler to heap-allocate the recursive cases.
///
/// Example tree for A(freq=3), B(freq=2), C(freq=1):
///
///       Internal(6)
///       /          \
///   Leaf(A,3)   Internal(3)
///               /          \
///           Leaf(C,1)   Leaf(B,2)
///
private indirect enum Node {
    /// A leaf node representing a single symbol with the given frequency.
    case leaf(symbol: Int, weight: Int)

    /// An internal (routing) node combining two sub-trees.
    /// The `order` field is a monotonic counter used only for tie-breaking.
    case `internal`(weight: Int, left: Node, right: Node, order: Int)

    /// The weight of this node (= frequency for leaves, sum for internal).
    var weight: Int {
        switch self {
        case .leaf(_, let w):         return w
        case .internal(let w, _, _, _): return w
        }
    }

    /// Computes the heap priority key for this node.
    var priorityKey: PriorityKey {
        switch self {
        case .leaf(let sym, let w):
            return PriorityKey(
                weight:      w,
                leafFlag:    0,
                symbolOrMax: sym,
                orderOrMax:  Int.max
            )
        case .internal(let w, _, _, let ord):
            return PriorityKey(
                weight:      w,
                leafFlag:    1,
                symbolOrMax: Int.max,
                orderOrMax:  ord
            )
        }
    }
}

// MARK: - HuffmanTree

/// A full binary tree that assigns optimal prefix-free bit codes to symbols.
///
/// Build the tree once from symbol frequencies; then:
/// - Use `codeTable()` to get a `[symbol: bitString]` dictionary for encoding.
/// - Use `decodeAll(_:count:)` to decode a bit stream back to symbols.
/// - Use `canonicalCodeTable()` for DEFLATE-style transmissible codes.
///
/// All symbols are integers. Frequencies must be positive.
/// The tree is immutable after construction.
///
/// Example:
/// ```swift
/// let tree = try HuffmanTree.build([(65, 3), (66, 2), (67, 1)])
/// let table = tree.codeTable()
/// // table[65] == "0"   (A gets the shortest code)
/// // table[67] == "10"  (C)
/// // table[66] == "11"  (B)
/// let decoded = try tree.decodeAll("0", count: 1)
/// // decoded == [65]
/// ```
public struct HuffmanTree {

    // MARK: - Errors

    /// Errors that can be thrown by HuffmanTree operations.
    public enum HuffmanError: Error, Equatable {
        /// Returned when `weights` is empty.
        case emptyWeights
        /// Returned when a frequency is zero or negative.
        case invalidFrequency(symbol: Int, frequency: Int)
        /// Returned when the bit stream runs out before `count` symbols decoded.
        case bitStreamExhausted(decoded: Int, expected: Int)
    }

    // MARK: - Storage

    /// The root node of the tree. Either a leaf (single symbol) or internal.
    private let root: Node

    /// The number of distinct symbols (= number of leaf nodes).
    private let _symbolCount: Int

    // MARK: - Private init

    private init(root: Node, symbolCount: Int) {
        self.root         = root
        self._symbolCount = symbolCount
    }

    // MARK: - build

    /// Constructs a Huffman tree from `(symbol, frequency)` pairs.
    ///
    /// The greedy algorithm uses a min-heap. At each step it pops the two
    /// lowest-weight nodes, combines them into a new internal node, and pushes
    /// the internal node back. The single remaining node is the root.
    ///
    /// Tie-breaking (for deterministic output across implementations):
    /// 1. Lowest weight pops first.
    /// 2. Leaves before internal nodes at equal weight.
    /// 3. Lower symbol value wins among leaves of equal weight.
    /// 4. Earlier-created internal node wins among internal nodes of equal weight.
    ///
    /// - Parameter weights: An array of `(symbol: Int, frequency: Int)` tuples.
    ///   Each symbol must be a non-negative integer; each frequency must be > 0.
    /// - Returns: A `HuffmanTree` ready for encoding/decoding.
    /// - Throws: `HuffmanError.emptyWeights` or `HuffmanError.invalidFrequency`.
    public static func build(_ weights: [(symbol: Int, frequency: Int)]) throws -> HuffmanTree {
        guard !weights.isEmpty else { throw HuffmanError.emptyWeights }
        for (sym, freq) in weights {
            guard freq > 0 else {
                throw HuffmanError.invalidFrequency(symbol: sym, frequency: freq)
            }
        }

        // Heap element: (priority key, node)
        var heap = MinHeap<(PriorityKey, Node)> {
            $0.0 < $1.0
        }

        // Seed: one leaf per symbol.
        for (sym, freq) in weights {
            let leaf = Node.leaf(symbol: sym, weight: freq)
            heap.push((leaf.priorityKey, leaf))
        }

        var orderCounter = 0

        // Merge phase.
        while heap.count > 1 {
            let (_, left)  = heap.pop()!
            let (_, right) = heap.pop()!
            let combined   = Node.internal(
                weight: left.weight + right.weight,
                left:   left,
                right:  right,
                order:  orderCounter
            )
            orderCounter += 1
            heap.push((combined.priorityKey, combined))
        }

        let (_, root) = heap.pop()!
        return HuffmanTree(root: root, symbolCount: weights.count)
    }

    // MARK: - codeTable

    /// Returns `[symbol: bitString]` for all symbols in the tree.
    ///
    /// Left edges are `"0"`, right edges are `"1"`.
    /// For a single-symbol tree the convention is `[symbol: "0"]`.
    ///
    /// Time: O(n).
    ///
    /// - Returns: A dictionary mapping each symbol to its bit string.
    public func codeTable() -> [Int: String] {
        var table: [Int: String] = [:]
        walk(node: root, prefix: "", into: &table)
        return table
    }

    /// Recursive tree walk building the code table.
    private func walk(node: Node, prefix: String, into table: inout [Int: String]) {
        switch node {
        case .leaf(let sym, _):
            table[sym] = prefix.isEmpty ? "0" : prefix
        case .internal(_, let left, let right, _):
            walk(node: left,  prefix: prefix + "0", into: &table)
            walk(node: right, prefix: prefix + "1", into: &table)
        }
    }

    // MARK: - codeFor

    /// Returns the bit string for a specific symbol, or `nil` if not in the tree.
    ///
    /// Searches the tree without building the full table.
    /// Time: O(n) worst case.
    ///
    /// - Parameter symbol: The symbol to look up.
    /// - Returns: The bit string code, or `nil` if not found.
    public func codeFor(_ symbol: Int) -> String? {
        return findCode(node: root, symbol: symbol, prefix: "")
    }

    /// Recursive search for a specific symbol's code.
    private func findCode(node: Node, symbol: Int, prefix: String) -> String? {
        switch node {
        case .leaf(let sym, _):
            if sym == symbol {
                return prefix.isEmpty ? "0" : prefix
            }
            return nil
        case .internal(_, let left, let right, _):
            if let result = findCode(node: left, symbol: symbol, prefix: prefix + "0") {
                return result
            }
            return findCode(node: right, symbol: symbol, prefix: prefix + "1")
        }
    }

    // MARK: - canonicalCodeTable

    /// Returns canonical Huffman codes (DEFLATE-style).
    ///
    /// Sorted by `(code_length, symbol_value)`; codes assigned numerically.
    /// Useful when you need to transmit only code lengths, not the tree structure.
    ///
    /// Time: O(n log n).
    ///
    /// - Returns: A dictionary mapping each symbol to its canonical bit string.
    public func canonicalCodeTable() -> [Int: String] {
        // Step 1: collect lengths.
        var lengths: [Int: Int] = [:]
        collectLengths(node: root, depth: 0, into: &lengths)

        // Single-leaf edge case.
        if _symbolCount == 1 {
            let sym = lengths.keys.first!
            return [sym: "0"]
        }

        // Step 2: sort by (length, symbol).
        let sorted = lengths.sorted { a, b in
            a.value != b.value ? a.value < b.value : a.key < b.key
        }

        // Step 3: assign codes numerically.
        var codeVal  = 0
        var prevLen  = sorted[0].value
        var result: [Int: String] = [:]

        for (sym, len) in sorted {
            if len > prevLen {
                codeVal <<= (len - prevLen)
            }
            // Format as zero-padded binary string of length `len`.
            let bits = String(codeVal, radix: 2)
            let padded = String(repeating: "0", count: max(0, len - bits.count)) + bits
            result[sym] = padded
            codeVal  += 1
            prevLen   = len
        }

        return result
    }

    /// Recursively collects code lengths for all leaves.
    private func collectLengths(node: Node, depth: Int, into lengths: inout [Int: Int]) {
        switch node {
        case .leaf(let sym, _):
            // Single-leaf tree has depth 0, but canonical length is 1 by convention.
            lengths[sym] = depth > 0 ? depth : 1
        case .internal(_, let left, let right, _):
            collectLengths(node: left,  depth: depth + 1, into: &lengths)
            collectLengths(node: right, depth: depth + 1, into: &lengths)
        }
    }

    // MARK: - decodeAll

    /// Decodes exactly `count` symbols from a bit string by walking the tree.
    ///
    /// Decoding: walk left on `"0"`, right on `"1"`. When a leaf is reached,
    /// emit its symbol and return to root. Repeat until `count` symbols decoded.
    ///
    /// Single-leaf edge case: the root is a leaf, so there are no edges.
    /// Each symbol is encoded as a single `"0"` bit, which is consumed.
    ///
    /// Multi-leaf: after reaching a leaf (after consuming the last edge bit),
    /// do NOT advance the index again — it is already past the consumed bit.
    ///
    /// - Parameters:
    ///   - bits: A `String` of `"0"` and `"1"` characters.
    ///   - count: The exact number of symbols to decode.
    /// - Returns: An array of decoded symbols of length `count`.
    /// - Throws: `HuffmanError.bitStreamExhausted` if bits run out early.
    public func decodeAll(_ bits: String, count: Int) throws -> [Int] {
        var result:     [Int] = []
        var current:    Node  = root
        var i           = bits.startIndex
        var singleLeaf  = false
        if case .leaf = root { singleLeaf = true }

        while result.count < count {
            switch current {
            case .leaf(let sym, _):
                result.append(sym)
                current = root
                if singleLeaf {
                    // Consume one '0' bit per symbol.
                    if i < bits.endIndex {
                        i = bits.index(after: i)
                    }
                }
                // Multi-leaf: index already advanced past the last edge bit.

            case .internal(_, let left, let right, _):
                guard i < bits.endIndex else {
                    throw HuffmanError.bitStreamExhausted(
                        decoded:  result.count,
                        expected: count
                    )
                }
                let bit = bits[i]
                i = bits.index(after: i)
                current = (bit == "0") ? left : right
            }
        }

        return result
    }

    // MARK: - Inspection

    /// The total weight of the tree (= sum of all leaf frequencies = root weight).
    ///
    /// O(1) — stored at the root.
    public var weight: Int {
        root.weight
    }

    /// The maximum code length (= depth of the deepest leaf).
    ///
    /// O(n) — must traverse the tree.
    public var depth: Int {
        maxDepth(node: root, d: 0)
    }

    /// Recursively finds the maximum leaf depth.
    private func maxDepth(node: Node, d: Int) -> Int {
        switch node {
        case .leaf:
            return d
        case .internal(_, let left, let right, _):
            return max(maxDepth(node: left, d: d + 1), maxDepth(node: right, d: d + 1))
        }
    }

    /// The number of distinct symbols (= number of leaf nodes).
    ///
    /// O(1) — stored at construction time.
    public var symbolCount: Int {
        _symbolCount
    }

    /// Returns an in-order (left-to-right) traversal of all leaves.
    ///
    /// Each element is a `(symbol, code)` tuple.
    /// Time: O(n).
    public func leaves() -> [(Int, String)] {
        let table = codeTable()
        var result: [(Int, String)] = []
        collectLeaves(node: root, table: table, into: &result)
        return result
    }

    /// Recursively collects leaves in in-order (left subtree first).
    private func collectLeaves(node: Node, table: [Int: String], into result: inout [(Int, String)]) {
        switch node {
        case .leaf(let sym, _):
            result.append((sym, table[sym] ?? "0"))
        case .internal(_, let left, let right, _):
            collectLeaves(node: left,  table: table, into: &result)
            collectLeaves(node: right, table: table, into: &result)
        }
    }

    /// Checks structural invariants of the tree.
    ///
    /// Invariants:
    /// 1. Every internal node has exactly 2 children (full binary tree).
    /// 2. `weight(internal) == weight(left) + weight(right)`.
    /// 3. No symbol appears in more than one leaf.
    ///
    /// Returns `true` if all invariants hold.
    /// For testing and assertions only.
    public func isValid() -> Bool {
        var seen = Set<Int>()
        return checkInvariants(node: root, seen: &seen)
    }

    /// Recursively validates tree invariants.
    private func checkInvariants(node: Node, seen: inout Set<Int>) -> Bool {
        switch node {
        case .leaf(let sym, _):
            guard !seen.contains(sym) else { return false }
            seen.insert(sym)
            return true
        case .internal(let w, let left, let right, _):
            guard w == left.weight + right.weight else { return false }
            return checkInvariants(node: left, seen: &seen)
                && checkInvariants(node: right, seen: &seen)
        }
    }
}
