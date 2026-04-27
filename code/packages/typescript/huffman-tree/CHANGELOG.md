# Changelog — @coding-adventures/huffman-tree

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-12

### Added

- `HuffmanTree` class with greedy min-heap construction (DT27).
- `build(weights)` static factory — accepts `[symbol, frequency][]` pairs;
  validates that the array is non-empty and all frequencies are positive.
- `codeTable()` — full `Map<symbol, bit_string>` via recursive tree walk;
  left edge = `'0'`, right edge = `'1'`; single-leaf convention assigns `'0'`.
- `codeFor(symbol)` — search for a single symbol's code without building the
  full table; returns `undefined` for unknown symbols.
- `canonicalCodeTable()` — DEFLATE-style canonical codes derived from code
  lengths; sorted by `(length, symbol)` and assigned numerically.
- `decodeAll(bits, count)` — decode exactly `count` symbols by walking the
  tree; handles the multi-leaf vs. single-leaf distinction correctly;
  throws on bit-stream exhaustion for multi-leaf trees.
- `weight()` — total weight (sum of all leaf frequencies); O(1).
- `depth()` — maximum code length (depth of deepest leaf); O(n).
- `symbolCount()` — number of distinct symbols; O(1).
- `leaves()` — in-order traversal of `[symbol, code]` pairs; O(n).
- `isValid()` — structural invariant checker (full binary tree, correct
  weights, no duplicate symbols); for testing.
- Tie-breaking key `[weight, isInternal, symbolOrMax, orderOrMax]` ensures
  deterministic, cross-implementation identical trees.
- 79 unit tests with vitest; 96.6% line coverage.
- Depends on `@coding-adventures/heap` for `MinHeap`.
- Literate-programming comments throughout the source.
