# Changelog — coding-adventures-huffman-tree (Lua)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-12

### Added

- Initial implementation of `CodingAdventures.HuffmanTree` (DT27).
- Depends on `coding-adventures-heap` for the comparator-based min-heap used
  during deterministic greedy construction.
- `HuffmanTree.build(weights)` — greedy construction from `{{symbol, freq}, ...}` pairs
  with four-level tie-breaking rules identical to the Python reference implementation.
- `tree:code_table()` — returns `{[symbol] = bitstring}` for all symbols.
- `tree:code_for(symbol)` — single-symbol lookup without building the full table.
- `tree:canonical_code_table()` — DEFLATE-style canonical Huffman codes.
- `tree:decode_all(bits, count)` — decode exactly `count` symbols from a bit string.
- `tree:weight()` — total weight (root weight = sum of all leaf frequencies).
- `tree:depth()` — maximum code length (depth of deepest leaf).
- `tree:symbol_count()` — number of distinct symbols.
- `tree:leaves()` — in-order left-to-right traversal returning `{{symbol, code}, ...}`.
- `tree:is_valid()` — structural invariant checker (full binary tree, weight sums,
  no duplicate symbols).
- Comprehensive Busted test suite with 50+ tests covering: build validation,
  code table correctness, prefix-free property, canonical codes, decode round-trips,
  edge cases (single symbol, two symbols, all equal weights), and determinism.
- `coding-adventures-huffman-tree-0.1.0-1.rockspec` for LuaRocks distribution.
- `README.md` with usage examples, algorithm explanation, and API reference.
