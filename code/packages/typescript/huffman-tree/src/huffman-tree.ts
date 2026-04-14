// =============================================================================
// DT27: Huffman Tree
// =============================================================================
//
// A Huffman tree is a full binary tree (every internal node has exactly two
// children) built from a symbol alphabet so that each symbol gets a unique
// variable-length bit code.  Symbols that appear often get short codes;
// symbols that appear rarely get long codes.  The total bits needed to
// encode a message is minimised — it is the theoretically optimal prefix-free
// code for a given symbol frequency distribution.
//
// Think of it like Morse code.  In Morse, `E` is `.` (one dot) and
// `Z` is `--..` (four symbols).  The designers knew `E` is the most
// common letter in English so they gave it the shortest code.  Huffman's
// algorithm does this automatically and optimally for any alphabet with any
// frequency distribution.
//
// ============================================================
// Algorithm: Greedy construction via min-heap
// ============================================================
//
// 1. Create one leaf node per distinct symbol, each with its frequency as its
//    weight.  Push all leaves onto a min-heap keyed by weight.
//
// 2. While the heap has more than one node:
//      a. Pop the two nodes with the smallest weight.
//      b. Create a new internal node whose weight = sum of the two children.
//      c. Set left = the first popped node, right = the second popped node.
//      d. Push the new internal node back onto the heap.
//
// 3. The one remaining node is the root of the Huffman tree.
//
// Tie-breaking rules (for deterministic output across implementations):
//   1. Lowest weight pops first.
//   2. Leaf nodes have higher priority than internal nodes at equal weight
//      ("leaf-before-internal" rule).
//   3. Among leaves of equal weight, lower symbol value wins.
//   4. Among internal nodes of equal weight, earlier-created node wins
//      (insertion-order FIFO).
//
// Why these rules?  Without tie-breaking, different implementations could
// build structurally different trees from the same input — producing different
// (but equally valid) code lengths.  Deterministic tie-breaking ensures the
// canonical code table is identical everywhere.
//
// ============================================================
// Prefix-free property: why it works
// ============================================================
//
// In a Huffman tree:
//   - Symbols live ONLY at the leaves, never at internal nodes.
//   - The code for a symbol is the path from root to its leaf
//     (left edge = '0', right edge = '1').
//
// Since one leaf is never an ancestor of another leaf, no code can be a
// prefix of another code.  This is the prefix-free property, and it means the
// bit stream can be decoded unambiguously without separator characters: just
// walk the tree bit by bit until you hit a leaf.
//
// ============================================================
// Canonical codes (DEFLATE / zlib style)
// ============================================================
//
// The standard tree-walk produces valid codes, but different tree shapes can
// produce different codes for the same symbol lengths.  Canonical codes
// normalise this: given only the code *lengths*, you can reconstruct the exact
// canonical code table without transmitting the tree structure.
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
// Example with AAABBC:
//   A: weight=3, B: weight=2, C: weight=1
//   Tree:      [6]
//              / \
//             A   [3]
//            (3)  / \
//                B   C
//               (2) (1)
//   Lengths: A=1, B=2, C=2
//   Sorted by (length, symbol): A(1), B(2), C(2)
//   Canonical codes:
//     A -> 0        (length 1,  code = 0)
//     B -> 10       (length 2,  code = 0+1=1, shifted 1 bit -> 10)
//     C -> 11       (length 2,  code = 10+1 = 11)
// =============================================================================

import { MinHeap } from "@coding-adventures/heap";

// ---------------------------------------------------------------------------
// Node types
// ---------------------------------------------------------------------------

/**
 * A leaf node representing a single symbol and its frequency weight.
 *
 * Leaves are the only nodes that carry symbol data.  The symbol is an
 * integer (typically 0..255 for byte-level coding, but any non-negative
 * integer is valid).
 */
export interface Leaf {
  readonly kind: "leaf";
  readonly symbol: number;
  readonly weight: number;
}

/**
 * An internal node combining two sub-trees.
 *
 * The weight of an internal node is always the sum of its children's weights.
 * The `_order` field is used only for tie-breaking during construction —
 * it records the insertion order of this node into the heap.
 */
export interface Internal {
  readonly kind: "internal";
  readonly weight: number;
  readonly left: Node;
  readonly right: Node;
  /** Insertion order for tie-breaking among internal nodes (FIFO). */
  readonly _order: number;
}

/** A union of leaf and internal nodes — the two kinds of Huffman tree nodes. */
export type Node = Leaf | Internal;

// ---------------------------------------------------------------------------
// Priority key (tie-breaking comparator)
// ---------------------------------------------------------------------------

/**
 * Returns a 4-tuple used as the heap key (lower = higher priority).
 *
 * Fields:
 *   [0] weight         — lower weight wins
 *   [1] isInternal     — 0=leaf (higher priority), 1=internal
 *   [2] symbolOrMax    — leaf: symbol value; internal: Number.MAX_SAFE_INTEGER
 *   [3] orderOrMax     — internal: insertion order (FIFO); leaf: Number.MAX_SAFE_INTEGER
 *
 * Why this four-field key?
 *   Huffman's algorithm is greedy: it always merges the two cheapest nodes.
 *   When two nodes have equal weight, we need a deterministic tiebreaker so
 *   every implementation (Python, TypeScript, Go, Ruby…) produces the same
 *   tree and therefore the same code lengths.
 *
 *   Rule 1: lighter node first (standard min-heap).
 *   Rule 2: a leaf beats an internal node at equal weight — intuitively, we
 *           prefer to "finish" individual symbols before merging groups.
 *   Rule 3: among leaves of equal weight, the lower symbol value wins —
 *           gives a stable alphabetical ordering.
 *   Rule 4: among internal nodes of equal weight, FIFO (first created wins) —
 *           preserves the temporal structure of the merging process.
 */
function nodePriority(node: Node): [number, number, number, number] {
  if (node.kind === "leaf") {
    return [node.weight, 0, node.symbol, Number.MAX_SAFE_INTEGER];
  } else {
    return [node.weight, 1, Number.MAX_SAFE_INTEGER, node._order];
  }
}

/**
 * Compare two priority 4-tuples lexicographically.
 *
 * Returns negative if `a` has higher priority (should pop first),
 * positive if `b` has higher priority, 0 if equal.
 *
 * This is used by the MinHeap to decide which node to pop next.
 */
function comparePriority(
  a: [number, number, number, number],
  b: [number, number, number, number]
): number {
  for (let i = 0; i < 4; i++) {
    if (a[i] !== b[i]) {
      return a[i]! < b[i]! ? -1 : 1;
    }
  }
  return 0;
}

// ---------------------------------------------------------------------------
// HuffmanTree class
// ---------------------------------------------------------------------------

/**
 * A full binary tree that assigns optimal prefix-free bit codes to symbols.
 *
 * Build the tree once from symbol frequencies; then:
 *   - Use `codeTable()` to get a `{symbol -> bit_string}` map for encoding.
 *   - Use `decodeAll()` to decode a bit stream back to symbols.
 *   - Use `canonicalCodeTable()` for DEFLATE-style transmissible codes.
 *
 * All symbols are integers (typically 0..255 for byte-level coding, but any
 * non-negative integer is valid).  Frequencies must be positive integers.
 *
 * The tree is immutable after construction.  Build a new tree if frequencies
 * change.
 *
 * Example:
 *   const tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]]);
 *   const table = tree.codeTable();
 *   table.get(65)  // 'A' gets the shortest code -> '0'
 *   tree.decodeAll('010011', 4)  // -> [65, 65, 66, 67]
 */
export class HuffmanTree {
  private readonly _root: Node;
  private readonly _symbolCount: number;

  private constructor(root: Node, symbolCount: number) {
    this._root = root;
    this._symbolCount = symbolCount;
  }

  // -------------------------------------------------------------------------
  // Construction
  // -------------------------------------------------------------------------

  /**
   * Construct a Huffman tree from `[symbol, frequency]` pairs.
   *
   * The greedy algorithm uses a min-heap.  At each step it pops the two
   * lowest-weight nodes, combines them into a new internal node, and pushes
   * the internal node back.  The single remaining node is the root.
   *
   * Tie-breaking (for deterministic output across implementations):
   *   1. Lowest weight pops first.
   *   2. Leaves before internal nodes at equal weight.
   *   3. Lower symbol value wins among leaves of equal weight.
   *   4. Earlier-created internal node wins among internal nodes of equal
   *      weight (FIFO insertion order).
   *
   * @param weights - An array of `[symbol, frequency]` pairs.  Each symbol
   *                  must be a non-negative integer; each frequency must be > 0.
   * @returns A `HuffmanTree` instance ready for encoding/decoding.
   * @throws Error if `weights` is empty or any frequency is <= 0.
   *
   * @example
   *   const tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]]);
   *   tree.symbolCount()  // -> 3
   */
  static build(weights: Array<[number, number]>): HuffmanTree {
    if (weights.length === 0) {
      throw new Error("weights must not be empty");
    }
    for (const [sym, freq] of weights) {
      if (freq <= 0) {
        throw new Error(
          `frequency must be positive; got symbol=${sym}, freq=${freq}`
        );
      }
    }

    // Build the min-heap.  Each element is a [priority, node] pair.
    // The comparator compares priority tuples lexicographically.
    type HeapEntry = [[number, number, number, number], Node];

    const heap = new MinHeap<HeapEntry>(
      (a: HeapEntry, b: HeapEntry) => comparePriority(a[0], b[0])
    );

    for (const [sym, freq] of weights) {
      const leaf: Leaf = { kind: "leaf", symbol: sym, weight: freq };
      heap.push([nodePriority(leaf), leaf]);
    }

    let orderCounter = 0; // monotonic counter for internal node insertion order

    while (heap.size > 1) {
      const [, left] = heap.pop();
      const [, right] = heap.pop();
      const combinedWeight = left.weight + right.weight;
      const internal: Internal = {
        kind: "internal",
        weight: combinedWeight,
        left,
        right,
        _order: orderCounter,
      };
      orderCounter++;
      heap.push([nodePriority(internal), internal]);
    }

    const [, root] = heap.pop();
    return new HuffmanTree(root, weights.length);
  }

  // -------------------------------------------------------------------------
  // Encoding helpers
  // -------------------------------------------------------------------------

  /**
   * Return `{symbol -> bit_string}` for all symbols in the tree.
   *
   * Left edges are `'0'`, right edges are `'1'`.  For a single-symbol
   * tree the convention is `{symbol: '0'}` (one bit per occurrence).
   *
   * Time: O(n) where n = number of distinct symbols.
   *
   * @example
   *   const tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]]);
   *   const table = tree.codeTable();
   *   // table.get(65) === '0'
   *   // table.get(66) === '10'
   *   // table.get(67) === '11'
   */
  codeTable(): Map<number, string> {
    const table = new Map<number, string>();
    walkTree(this._root, "", table);
    return table;
  }

  /**
   * Return the bit string for a specific symbol, or `undefined` if not in
   * the tree.
   *
   * Walks the tree searching for the leaf with `symbol`; does NOT build
   * the full code table.
   *
   * Time: O(n) worst case (full tree traversal).
   *
   * @example
   *   const tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]]);
   *   tree.codeFor(65)  // -> '0'
   *   tree.codeFor(99)  // -> undefined
   */
  codeFor(symbol: number): string | undefined {
    return findCode(this._root, symbol, "");
  }

  /**
   * Return canonical Huffman codes (DEFLATE-style).
   *
   * Sorted by `(code_length, symbol_value)`; codes assigned numerically.
   * Useful when you need to transmit only code lengths, not the tree.
   *
   * Time: O(n log n).
   *
   * Why canonical codes?
   *   Two different Huffman trees can assign different codes to the same
   *   symbol — but both are correct (they have the same code lengths).
   *   Canonical codes normalise this so the decoder only needs to know the
   *   code lengths, not the full tree structure.  DEFLATE exploits this to
   *   store smaller headers.
   *
   * @example
   *   const tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]]);
   *   const canonical = tree.canonicalCodeTable();
   *   // canonical.get(65) === '0'
   *   // canonical.get(66) === '10'
   *   // canonical.get(67) === '11'
   */
  canonicalCodeTable(): Map<number, string> {
    // Step 1: collect lengths from the tree
    const lengths = new Map<number, number>();
    collectLengths(this._root, 0, lengths);

    // Single-leaf edge case: assign length 1 by convention
    if (lengths.size === 1) {
      const sym = lengths.keys().next().value!;
      return new Map([[sym, "0"]]);
    }

    // Step 2: sort by (length, symbol)
    const sortedSyms = [...lengths.entries()].sort(
      ([symA, lenA], [symB, lenB]) => lenA !== lenB ? lenA - lenB : symA - symB
    );

    // Step 3: assign canonical codes numerically
    let codeVal = 0;
    let prevLen = sortedSyms[0]![1];
    const result = new Map<number, string>();

    for (const [sym, length] of sortedSyms) {
      if (length > prevLen) {
        codeVal <<= (length - prevLen);
      }
      result.set(sym, codeVal.toString(2).padStart(length, "0"));
      codeVal++;
      prevLen = length;
    }

    return result;
  }

  // -------------------------------------------------------------------------
  // Decoding
  // -------------------------------------------------------------------------

  /**
   * Decode exactly `count` symbols from a bit string by walking the tree.
   *
   * @param bits  - A string of `'0'` and `'1'` characters.
   * @param count - The exact number of symbols to decode.
   * @returns An array of decoded symbols of length == `count`.
   * @throws Error if the bit stream is exhausted before `count` symbols
   *         are decoded.
   *
   * For a single-leaf tree, each `'0'` bit decodes to that symbol.
   *
   * Multi-leaf tree decoding:
   *   Walk the tree consuming one bit per edge.  When you reach a leaf,
   *   record the symbol and reset to the root — do NOT consume an extra bit.
   *   The next bit starts a new path from the root.
   *
   * Time: O(total bits consumed).
   *
   * @example
   *   const tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]]);
   *   tree.decodeAll('010011', 4)  // -> [65, 65, 66, 67]
   */
  decodeAll(bits: string, count: number): number[] {
    const result: number[] = [];
    let node: Node = this._root;
    let i = 0;
    const singleLeaf = this._root.kind === "leaf";

    while (result.length < count) {
      if (node.kind === "leaf") {
        // We have arrived at a leaf — record the symbol and reset to root.
        result.push(node.symbol);
        node = this._root;
        if (singleLeaf) {
          // Single-leaf trees encode each symbol as a '0' bit.
          // Consume that '0' before continuing.
          if (i < bits.length) {
            i++;
          }
        }
        // For multi-leaf trees, i is already past the last consumed bit.
        // The next iteration begins at the root without advancing i.
        continue;
      }

      if (i >= bits.length) {
        throw new Error(
          `Bit stream exhausted after ${result.length} symbols; expected ${count}`
        );
      }

      const bit = bits[i]!;
      i++;
      node = bit === "0" ? node.left : node.right;
    }

    return result;
  }

  // -------------------------------------------------------------------------
  // Inspection
  // -------------------------------------------------------------------------

  /**
   * Total weight of the tree = sum of all leaf frequencies = root weight.
   * O(1) — stored at the root.
   *
   * @example
   *   const tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]]);
   *   tree.weight()  // -> 6
   */
  weight(): number {
    return this._root.weight;
  }

  /**
   * Maximum code length = depth of the deepest leaf.
   * O(n) — must traverse the tree.
   *
   * @example
   *   const tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]]);
   *   tree.depth()  // -> 2
   */
  depth(): number {
    return maxDepth(this._root, 0);
  }

  /**
   * Number of distinct symbols (= number of leaf nodes).
   * O(1) — stored at construction time.
   *
   * @example
   *   const tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]]);
   *   tree.symbolCount()  // -> 3
   */
  symbolCount(): number {
    return this._symbolCount;
  }

  /**
   * In-order traversal of leaves.
   *
   * Returns `[[symbol, code], ...]`, left subtree before right subtree.
   * Useful for visualisation and debugging.
   *
   * Time: O(n).
   *
   * @example
   *   const tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]]);
   *   tree.leaves()
   *   // -> [[65, '0'], [66, '10'], [67, '11']]
   */
  leaves(): Array<[number, string]> {
    const table = this.codeTable();
    const result: Array<[number, string]> = [];
    inOrderLeaves(this._root, result, table);
    return result;
  }

  /**
   * Check structural invariants.  For testing only.
   *
   *   1. Every internal node has exactly 2 children (full binary tree).
   *   2. `weight(internal) == weight(left) + weight(right)`.
   *   3. No symbol appears in more than one leaf.
   *
   * Returns `true` if all invariants hold.
   *
   * @example
   *   const tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]]);
   *   tree.isValid()  // -> true
   */
  isValid(): boolean {
    const seen = new Set<number>();
    return checkInvariants(this._root, seen);
  }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/**
 * Recursively walk the tree building the code table.
 *
 * We append '0' at each left edge and '1' at each right edge.
 * When we reach a leaf, we store the accumulated prefix as the code.
 * The edge case: a single-leaf tree has no edges, so we assign '0' by
 * convention (one bit per symbol).
 */
function walkTree(node: Node, prefix: string, table: Map<number, string>): void {
  if (node.kind === "leaf") {
    // Single-leaf edge case: no edges traversed; use '0' by convention.
    table.set(node.symbol, prefix || "0");
    return;
  }
  walkTree(node.left, prefix + "0", table);
  walkTree(node.right, prefix + "1", table);
}

/**
 * Search the tree for a specific symbol, returning its code or undefined.
 *
 * Returns undefined if the symbol is not found in any leaf.
 */
function findCode(node: Node, symbol: number, prefix: string): string | undefined {
  if (node.kind === "leaf") {
    if (node.symbol === symbol) {
      return prefix || "0";
    }
    return undefined;
  }
  const leftResult = findCode(node.left, symbol, prefix + "0");
  if (leftResult !== undefined) {
    return leftResult;
  }
  return findCode(node.right, symbol, prefix + "1");
}

/**
 * Collect code lengths for all leaves.
 *
 * The depth `d` is the number of edges from root to the current node.
 * For a single-leaf tree, depth is 0 but we assign length 1 by convention.
 */
function collectLengths(
  node: Node,
  d: number,
  lengths: Map<number, number>
): void {
  if (node.kind === "leaf") {
    lengths.set(node.symbol, d > 0 ? d : 1); // single-leaf: depth=0, length=1
    return;
  }
  collectLengths(node.left, d + 1, lengths);
  collectLengths(node.right, d + 1, lengths);
}

/**
 * Return the maximum depth of any leaf in the tree.
 *
 * Depth is the number of edges from root to a node.
 */
function maxDepth(node: Node, d: number): number {
  if (node.kind === "leaf") {
    return d;
  }
  return Math.max(maxDepth(node.left, d + 1), maxDepth(node.right, d + 1));
}

/**
 * Collect leaves in left-to-right (in-order) traversal.
 *
 * For each leaf, we look up its code in the pre-built table and append
 * [symbol, code] to the result array.
 */
function inOrderLeaves(
  node: Node,
  result: Array<[number, string]>,
  table: Map<number, string>
): void {
  if (node.kind === "leaf") {
    result.push([node.symbol, table.get(node.symbol)!]);
    return;
  }
  inOrderLeaves(node.left, result, table);
  inOrderLeaves(node.right, result, table);
}

/**
 * Recursively validate tree invariants.
 *
 * Returns false immediately on the first violation found.
 */
function checkInvariants(node: Node, seen: Set<number>): boolean {
  if (node.kind === "leaf") {
    if (seen.has(node.symbol)) {
      return false; // duplicate symbol
    }
    seen.add(node.symbol);
    return true;
  }
  // Internal node: weight must equal sum of children's weights.
  if (node.weight !== node.left.weight + node.right.weight) {
    return false;
  }
  return checkInvariants(node.left, seen) && checkInvariants(node.right, seen);
}
