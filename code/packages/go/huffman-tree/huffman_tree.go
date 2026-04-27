// Package huffmantree implements DT27: the Huffman Tree data structure.
//
// A Huffman tree is a full binary tree (every internal node has exactly two
// children) built from a symbol alphabet so that each symbol gets a unique
// variable-length bit code.  Symbols that appear often get short codes;
// symbols that appear rarely get long codes.  The total bits needed to encode
// a message is minimised — it is the theoretically optimal prefix-free code
// for a given symbol frequency distribution.
//
// Think of it like Morse code.  In Morse, "E" is "." (one dot) and
// "Z" is "--.." (four symbols).  The designers knew "E" is the most common
// letter in English so they gave it the shortest code.  Huffman's algorithm
// does this automatically and optimally for any alphabet with any frequency
// distribution.
//
// # Algorithm: Greedy construction via min-heap
//
//  1. Create one leaf node per distinct symbol, each with its frequency as its
//     weight.  Push all leaves onto a min-heap keyed by weight.
//
//  2. While the heap has more than one node:
//     a. Pop the two nodes with the smallest weight.
//     b. Create a new internal node whose weight = sum of the two children.
//     c. Set left = the first popped node, right = the second popped node.
//     d. Push the new internal node back onto the heap.
//
//  3. The one remaining node is the root of the Huffman tree.
//
// # Tie-breaking rules (for deterministic output across implementations)
//
//  1. Lowest weight pops first.
//  2. Leaf nodes have higher priority than internal nodes at equal weight
//     ("leaf-before-internal" rule).
//  3. Among leaves of equal weight, lower symbol value wins.
//  4. Among internal nodes of equal weight, earlier-created node wins
//     (insertion-order FIFO).
//
// Why these rules?  Without tie-breaking, different implementations could
// build structurally different trees from the same input — producing different
// (but equally valid) code lengths.  Deterministic tie-breaking ensures the
// canonical code table is identical everywhere.
//
// # Canonical codes (DEFLATE / zlib style)
//
// The standard tree-walk produces valid codes, but different tree shapes can
// produce different codes for the same symbol lengths.  Canonical codes
// normalise this: given only the code lengths, you can reconstruct the exact
// canonical code table without transmitting the tree structure.
//
// Algorithm:
//  1. Collect (symbol, code_length) pairs from the tree.
//  2. Sort by (code_length, symbol_value).
//  3. Assign codes numerically:
//     code[0] = 0 (left-padded to length[0] bits)
//     code[i] = (code[i-1] + 1) << (length[i] - length[i-1])
//
// This is exactly what DEFLATE uses: the compressed stream contains only the
// length table, not the tree, saving space.
//
// Example with AAABBC:
//
//	A: weight=3, B: weight=2, C: weight=1
//	Tree:      [6]
//	           / \
//	          A   [3]
//	         (3)  / \
//	             B   C
//	            (2) (1)
//	Lengths: A=1, B=2, C=2
//	Sorted by (length, symbol): A(1), B(2), C(2)
//	Canonical codes:
//	  A → 0        (length 1,  code = 0)
//	  B → 10       (length 2,  code = 0+1=1, shifted 1 bit → 10)
//	  C → 11       (length 2,  code = 10+1 = 11)
package huffmantree

import (
	"fmt"
	"sort"

	"github.com/adhithyan15/coding-adventures/code/packages/go/heap"
)

// ─── Node types ───────────────────────────────────────────────────────────────

// nodeKind distinguishes leaf nodes from internal nodes.
type nodeKind int

const (
	kindLeaf     nodeKind = 0
	kindInternal nodeKind = 1
)

// node is a single node in the Huffman tree.
//
// For a leaf:   kind=kindLeaf, symbol is set, left/right are nil.
// For internal: kind=kindInternal, left/right are set, symbol is irrelevant.
// The _order field is only meaningful for internal nodes: it records
// insertion order into the heap, used for deterministic tie-breaking.
type node struct {
	weight  int
	kind    nodeKind
	symbol  int // leaf only
	left    *node
	right   *node
	_order  int // internal only; monotonic counter
}

// isLeaf returns true when this node is a leaf.
func (n *node) isLeaf() bool {
	return n.kind == kindLeaf
}

// ─── Heap item ────────────────────────────────────────────────────────────────

// heapItem wraps a node with its 4-tuple priority for the min-heap.
//
// The priority tuple (weight, isInternal, symbolOrNeg1, orderOrNeg1) encodes
// all four tie-breaking rules in a single comparable structure:
//
//   - Field 0 (weight): lower weight pops first.
//   - Field 1 (isInternal): 0=leaf pops before 1=internal at equal weight.
//   - Field 2 (symbol): among equal-weight leaves, lower symbol wins.
//   - Field 3 (order): among equal-weight internals, earlier created (smaller
//     order) wins.  Leaves carry -1 here (never compared against internals due
//     to field 1).
type heapItem struct {
	priority [4]int
	n        *node
}

// heapLess is the comparator for the min-heap.  It performs a lexicographic
// comparison of the 4-tuple priorities so that the "smallest" item in the
// heap pops first.
func heapLess(a, b heapItem) bool {
	for i := 0; i < 4; i++ {
		if a.priority[i] != b.priority[i] {
			return a.priority[i] < b.priority[i]
		}
	}
	return false
}

// makePriority builds the 4-tuple for a node.
//
//	Leaf:     (weight, 0, symbol,  -1)
//	Internal: (weight, 1,      -1, order)
func makePriority(n *node) [4]int {
	if n.isLeaf() {
		return [4]int{n.weight, 0, n.symbol, -1}
	}
	return [4]int{n.weight, 1, -1, n._order}
}

// ─── HuffmanTree ──────────────────────────────────────────────────────────────

// HuffmanTree is a full binary tree that assigns optimal prefix-free bit codes
// to symbols.
//
// Build the tree once from symbol frequencies; then:
//   - Use CodeTable to get a map[symbol]bitString for encoding.
//   - Use DecodeAll to decode a bit stream back to symbols.
//   - Use CanonicalCodeTable for DEFLATE-style transmissible codes.
//
// All symbols are integers (typically 0..255 for byte-level coding, but any
// non-negative integer is valid).  Frequencies must be positive integers.
//
// The tree is immutable after construction.  Build a new tree if frequencies
// change.
//
// Example:
//
//	tree, _ := Build([]WeightPair{{65, 3}, {66, 2}, {67, 1}})
//	table := CodeTable(tree)
//	fmt.Println(table[65]) // "0"
type HuffmanTree struct {
	root         *node
	symbolCount  int
}

// WeightPair holds a (symbol, frequency) pair used to build the tree.
type WeightPair struct {
	Symbol    int
	Frequency int
}

// Build constructs a Huffman tree from (symbol, frequency) pairs.
//
// The greedy algorithm uses a min-heap.  At each step it pops the two
// lowest-weight nodes, combines them into a new internal node, and pushes the
// internal node back.  The single remaining node is the root.
//
// Returns an error if weights is empty or any frequency is ≤ 0.
func Build(weights []WeightPair) (*HuffmanTree, error) {
	if len(weights) == 0 {
		return nil, fmt.Errorf("weights must not be empty")
	}
	for _, wp := range weights {
		if wp.Frequency <= 0 {
			return nil, fmt.Errorf(
				"frequency must be positive; got symbol=%d, freq=%d",
				wp.Symbol, wp.Frequency,
			)
		}
	}

	// Initialise the min-heap with one leaf per symbol.
	h := heap.NewMinHeap[heapItem](heapLess)
	for _, wp := range weights {
		leaf := &node{
			weight: wp.Frequency,
			kind:   kindLeaf,
			symbol: wp.Symbol,
		}
		h.Push(heapItem{priority: makePriority(leaf), n: leaf})
	}

	// Greedy merge: pop two smallest, create internal, push back.
	orderCounter := 0
	for h.Len() > 1 {
		leftItem, _ := h.Pop()
		rightItem, _ := h.Pop()
		internal := &node{
			weight: leftItem.n.weight + rightItem.n.weight,
			kind:   kindInternal,
			left:   leftItem.n,
			right:  rightItem.n,
			_order: orderCounter,
		}
		orderCounter++
		h.Push(heapItem{priority: makePriority(internal), n: internal})
	}

	rootItem, _ := h.Pop()
	return &HuffmanTree{
		root:        rootItem.n,
		symbolCount: len(weights),
	}, nil
}

// ─── Encoding helpers ─────────────────────────────────────────────────────────

// CodeTable returns a map from symbol to bit-string for all symbols in the
// tree.
//
// Left edges are "0", right edges are "1".  For a single-symbol tree the
// convention is map[symbol]"0" (one bit per occurrence).
//
// Time: O(n) where n = number of distinct symbols.
//
// Example:
//
//	tree, _ := Build([]WeightPair{{65, 3}, {66, 2}, {67, 1}})
//	table := CodeTable(tree)
//	// table[65]="0", table[66]="10", table[67]="11"
func CodeTable(t *HuffmanTree) map[int]string {
	table := make(map[int]string)
	walk(t.root, "", table)
	return table
}

// CodeFor returns the bit-string for a specific symbol, or ("", false) if the
// symbol is not in the tree.
//
// Walks the tree searching for the leaf with symbol; does NOT build the full
// code table.
//
// Time: O(n) worst case (full tree traversal).
func CodeFor(t *HuffmanTree, symbol int) (string, bool) {
	code, found := findCode(t.root, symbol, "")
	return code, found
}

// CanonicalCodeTable returns canonical Huffman codes (DEFLATE-style).
//
// Sorted by (code_length, symbol_value); codes assigned numerically.
// Useful when you need to transmit only code lengths, not the tree.
//
// Time: O(n log n).
//
// Example:
//
//	tree, _ := Build([]WeightPair{{65, 3}, {66, 2}, {67, 1}})
//	canonical := CanonicalCodeTable(tree)
//	// canonical[65]="0", canonical[66]="10", canonical[67]="11"
func CanonicalCodeTable(t *HuffmanTree) map[int]string {
	// Step 1: collect code lengths for each leaf.
	lengths := make(map[int]int)
	collectLengths(t.root, 0, lengths)

	// Single-leaf edge case: assign length 1 by convention.
	if len(lengths) == 1 {
		for sym := range lengths {
			return map[int]string{sym: "0"}
		}
	}

	// Step 2: sort by (length, symbol).
	type symLen struct {
		sym int
		ln  int
	}
	pairs := make([]symLen, 0, len(lengths))
	for sym, ln := range lengths {
		pairs = append(pairs, symLen{sym, ln})
	}
	sort.Slice(pairs, func(i, j int) bool {
		if pairs[i].ln != pairs[j].ln {
			return pairs[i].ln < pairs[j].ln
		}
		return pairs[i].sym < pairs[j].sym
	})

	// Step 3: assign canonical codes numerically.
	//   code[0] = 0 (padded to length[0] bits)
	//   code[i] = (code[i-1] + 1) << (length[i] - length[i-1])
	codeVal := 0
	prevLen := pairs[0].ln
	result := make(map[int]string, len(pairs))

	for _, sl := range pairs {
		if sl.ln > prevLen {
			codeVal <<= (sl.ln - prevLen)
		}
		// Format as zero-padded binary string of exactly sl.ln bits.
		result[sl.sym] = formatBinary(codeVal, sl.ln)
		codeVal++
		prevLen = sl.ln
	}

	return result
}

// ─── Decoding ─────────────────────────────────────────────────────────────────

// DecodeAll decodes exactly count symbols from a bit string by walking the
// tree.
//
// bits is a string of '0' and '1' characters.  Returns a slice of decoded
// symbols of length == count, or an error if the bit stream is exhausted
// before count symbols are decoded.
//
// For a single-leaf tree, each '0' bit decodes to that symbol.
//
// Time: O(total bits consumed).
//
// Example:
//
//	tree, _ := Build([]WeightPair{{65, 3}, {66, 2}, {67, 1}})
//	symbols, _ := DecodeAll(tree, "010011", 4)
//	// symbols == [65, 65, 66, 67]
func DecodeAll(t *HuffmanTree, bits string, count int) ([]int, error) {
	result := make([]int, 0, count)
	current := t.root
	i := 0
	// Single-leaf trees encode each symbol as a single '0' bit.
	// Multi-leaf trees: reaching a leaf means i is already past the last
	// consumed bit — no extra advance needed.
	singleLeaf := t.root.isLeaf()

	for len(result) < count {
		if current.isLeaf() {
			// We have arrived at a leaf: record the symbol and reset to root.
			result = append(result, current.symbol)
			current = t.root
			if singleLeaf {
				// Consume the '0' bit placeholder for this symbol.
				if i < len(bits) {
					i++
				}
			}
			continue
		}

		if i >= len(bits) {
			return nil, fmt.Errorf(
				"bit stream exhausted after %d symbols; expected %d",
				len(result), count,
			)
		}
		bit := bits[i]
		i++
		if bit == '0' {
			current = current.left
		} else {
			current = current.right
		}
	}

	return result, nil
}

// ─── Inspection ───────────────────────────────────────────────────────────────

// Weight returns the total weight of the tree = sum of all leaf frequencies =
// root weight.
//
// Time: O(1) — stored at the root.
func Weight(t *HuffmanTree) int {
	return t.root.weight
}

// Depth returns the maximum code length = depth of the deepest leaf.
//
// Time: O(n) — must traverse the tree.
func Depth(t *HuffmanTree) int {
	return maxDepth(t.root, 0)
}

// SymbolCount returns the number of distinct symbols (= number of leaf nodes).
//
// Time: O(1) — stored at construction time.
func SymbolCount(t *HuffmanTree) int {
	return t.symbolCount
}

// Leaves returns the leaves of the tree in left-to-right (in-order) order.
//
// Each entry is a (symbol, code) pair.  Useful for visualisation and
// debugging.
//
// Time: O(n).
func Leaves(t *HuffmanTree) []LeafEntry {
	table := CodeTable(t)
	result := make([]LeafEntry, 0, t.symbolCount)
	inOrderLeaves(t.root, &result, table)
	return result
}

// LeafEntry is a (Symbol, Code) pair returned by Leaves.
type LeafEntry struct {
	Symbol int
	Code   string
}

// IsValid checks structural invariants.  Intended for testing only.
//
//  1. Every internal node has exactly 2 children (full binary tree).
//  2. weight(internal) == weight(left) + weight(right).
//  3. No symbol appears in more than one leaf.
//
// Returns true if all invariants hold.
func IsValid(t *HuffmanTree) bool {
	seen := make(map[int]bool)
	return checkInvariants(t.root, seen)
}

// ─── Private helpers ──────────────────────────────────────────────────────────

// walk recursively populates table with {symbol → bit-string} entries.
//
// When a single-leaf tree calls walk, prefix is "" and the convention is to
// use "0" rather than the empty string, matching the spec.
func walk(n *node, prefix string, table map[int]string) {
	if n.isLeaf() {
		if prefix == "" {
			table[n.symbol] = "0"
		} else {
			table[n.symbol] = prefix
		}
		return
	}
	walk(n.left, prefix+"0", table)
	walk(n.right, prefix+"1", table)
}

// findCode searches the tree for symbol and returns its code.
func findCode(n *node, symbol int, prefix string) (string, bool) {
	if n.isLeaf() {
		if n.symbol == symbol {
			if prefix == "" {
				return "0", true
			}
			return prefix, true
		}
		return "", false
	}
	if code, ok := findCode(n.left, symbol, prefix+"0"); ok {
		return code, true
	}
	return findCode(n.right, symbol, prefix+"1")
}

// collectLengths populates lengths with {symbol → depth} for all leaves.
//
// For a single-leaf tree the depth is 0 at the root, but the convention is
// code length 1, so we store max(depth, 1).
func collectLengths(n *node, d int, lengths map[int]int) {
	if n.isLeaf() {
		if d == 0 {
			lengths[n.symbol] = 1 // single-leaf convention
		} else {
			lengths[n.symbol] = d
		}
		return
	}
	collectLengths(n.left, d+1, lengths)
	collectLengths(n.right, d+1, lengths)
}

// maxDepth returns the maximum depth of any leaf below n.
func maxDepth(n *node, d int) int {
	if n.isLeaf() {
		return d
	}
	l := maxDepth(n.left, d+1)
	r := maxDepth(n.right, d+1)
	if l > r {
		return l
	}
	return r
}

// inOrderLeaves appends leaf entries in left-to-right order.
func inOrderLeaves(n *node, result *[]LeafEntry, table map[int]string) {
	if n.isLeaf() {
		*result = append(*result, LeafEntry{Symbol: n.symbol, Code: table[n.symbol]})
		return
	}
	inOrderLeaves(n.left, result, table)
	inOrderLeaves(n.right, result, table)
}

// checkInvariants recursively validates tree invariants.
func checkInvariants(n *node, seen map[int]bool) bool {
	if n.isLeaf() {
		if seen[n.symbol] {
			return false
		}
		seen[n.symbol] = true
		return true
	}
	// Internal node: both children must exist and weights must sum correctly.
	if n.left == nil || n.right == nil {
		return false
	}
	if n.weight != n.left.weight+n.right.weight {
		return false
	}
	return checkInvariants(n.left, seen) && checkInvariants(n.right, seen)
}

// formatBinary formats v as a zero-padded binary string of exactly width bits.
//
// Example: formatBinary(1, 3) → "001"
// Example: formatBinary(3, 2) → "11"
func formatBinary(v, width int) string {
	if width == 0 {
		return ""
	}
	buf := make([]byte, width)
	for i := width - 1; i >= 0; i-- {
		if v&1 == 1 {
			buf[i] = '1'
		} else {
			buf[i] = '0'
		}
		v >>= 1
	}
	return string(buf)
}
