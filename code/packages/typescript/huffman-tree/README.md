# @coding-adventures/huffman-tree

**DT27** — Huffman tree data structure. Greedy min-heap construction producing optimal prefix-free codes, with canonical code generation (DEFLATE-style).

Used by **CMP04** (Huffman compression) in the coding-adventures monorepo.

## What is a Huffman Tree?

A Huffman tree is a full binary tree where each leaf holds a symbol and its
frequency weight. The tree is built so that frequent symbols get short bit
codes and rare symbols get long ones — minimising the total bits required to
encode a message.

Think Morse code: `E` is `.` (one symbol) because it is the most common letter
in English. Huffman's algorithm constructs such optimal codes automatically for
any alphabet and any frequency distribution.

## Algorithm

**Construction** — greedy min-heap:

1. Create one leaf per symbol. Push all leaves onto a min-heap.
2. While the heap has more than one node:
   - Pop the two lowest-weight nodes (left = first pop, right = second pop).
   - Create an internal node with weight = sum of children.
   - Push the internal node back.
3. The surviving node is the root.

**Tie-breaking** (deterministic across implementations):

| Priority | Rule |
|----------|------|
| 1 | Lower weight pops first |
| 2 | Leaf beats internal node at equal weight |
| 3 | Lower symbol value wins among equal-weight leaves |
| 4 | Earlier-created internal node wins (FIFO) |

**Prefix-free property** — codes live only at leaves, so no code is a prefix
of another. The decoder can walk the tree bit-by-bit without separators.

**Canonical codes (DEFLATE-style)** — given only the code lengths, assign
codes numerically sorted by `(length, symbol)`. DEFLATE transmits only the
length table, not the tree structure.

## Installation

```bash
npm install @coding-adventures/huffman-tree
```

## Usage

```typescript
import { HuffmanTree } from "@coding-adventures/huffman-tree";

// Build from (symbol, frequency) pairs
// Symbol 65 = 'A', 66 = 'B', 67 = 'C'
const tree = HuffmanTree.build([
  [65, 3],  // A appears 3 times
  [66, 2],  // B appears 2 times
  [67, 1],  // C appears 1 time
]);

// Get the code table
const table = tree.codeTable();
// Map { 65 => '0', 67 => '10', 66 => '11' }

// Get a specific symbol's code
tree.codeFor(65);  // '0'

// Canonical codes (DEFLATE-style — sorted by length then symbol)
const canonical = tree.canonicalCodeTable();
// Map { 65 => '0', 66 => '10', 67 => '11' }

// Encode a message manually: A A C B -> '0' '0' '10' '11'
const bits = [65, 65, 67, 66].map(s => table.get(s)!).join(""); // '001011'

// Decode
tree.decodeAll(bits, 4);  // [65, 65, 67, 66]

// Inspect
tree.weight();       // 6 (total frequency)
tree.depth();        // 2 (longest code length)
tree.symbolCount();  // 3
tree.isValid();      // true
tree.leaves();       // [[65,'0'], [67,'10'], [66,'11']]
```

## Single-symbol edge case

A tree with one symbol has no edges. By convention its code is `'0'` (one bit
per symbol occurrence):

```typescript
const tree = HuffmanTree.build([[65, 5]]);
tree.codeTable();          // Map { 65 => '0' }
tree.decodeAll('000', 3);  // [65, 65, 65]
```

## API

### `HuffmanTree.build(weights: [symbol, frequency][]): HuffmanTree`

Construct a Huffman tree. Throws if `weights` is empty or any frequency ≤ 0.

### `tree.codeTable(): Map<number, string>`

Return `{symbol → bit_string}` for all symbols. Left edge = `'0'`, right = `'1'`.

### `tree.codeFor(symbol: number): string | undefined`

Return the bit string for a specific symbol, or `undefined` if not present.

### `tree.canonicalCodeTable(): Map<number, string>`

Return DEFLATE-style canonical codes sorted by `(length, symbol)`.

### `tree.decodeAll(bits: string, count: number): number[]`

Decode exactly `count` symbols from a bit string. Throws on stream exhaustion
(multi-leaf trees only).

### `tree.weight(): number`

Sum of all leaf frequencies (= root weight).

### `tree.depth(): number`

Depth of the deepest leaf = maximum code length.

### `tree.symbolCount(): number`

Number of distinct symbols.

### `tree.leaves(): [number, string][]`

In-order traversal of leaves: `[[symbol, code], ...]`.

### `tree.isValid(): boolean`

Check structural invariants (full binary tree, correct weights, no duplicate
symbols). For testing only.

## Position in the Stack

```
CMP04 (Huffman compression) — uses DT27
DT27  (Huffman tree)        — this package, uses DT26 (heap)
DT26  (heap)                — @coding-adventures/heap
```

## License

MIT
