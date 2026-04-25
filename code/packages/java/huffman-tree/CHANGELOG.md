# Changelog — java/huffman-tree

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-25

### Added

- `HuffmanTree` — optimal prefix-free code tree backed by `java.util.PriorityQueue`
- `HuffmanTree.build(List<int[]> weights)` — greedy min-heap construction from
  `[symbol, frequency]` pairs; throws `IllegalArgumentException` for empty list or
  non-positive frequency
- `codeTable()` — O(n) walk returning `{symbol → bit_string}` map
- `codeFor(int symbol)` — O(n) single-symbol lookup; returns null if absent
- `canonicalCodeTable()` — O(n log n) DEFLATE-style canonical codes from lengths
- `decodeAll(String bits, int count)` — O(bits) tree-walk decoder; throws when
  stream exhausted before `count` symbols
- `weight()` — O(1) total weight (root weight = sum of all frequencies)
- `depth()` — O(n) maximum code length (depth of deepest leaf)
- `symbolCount()` — O(1) number of distinct symbols
- `leavesWithCodes()` — O(n) in-order list of `[symbol, code]` pairs
- `isValid()` — structural validator: full binary tree, correct weights, unique symbols
- Deterministic tie-breaking: weight → leaf-before-internal → symbol value → insertion order
- 36 unit tests covering construction, code tables, canonical codes, round-trips,
  inspection methods, tie-breaking determinism, and isValid
