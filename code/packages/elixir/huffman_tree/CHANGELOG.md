# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-04-12

### Added

- Initial implementation of the Huffman Tree data structure (DT27).
- `build/1` — greedy min-heap construction from `{symbol, frequency}` pairs;
  deterministic tie-breaking: (1) lowest weight first, (2) leaf-before-internal
  at equal weight, (3) lower symbol value among leaves, (4) FIFO insertion
  order among internal nodes.
- `code_table/1` — O(n) tree walk returning `%{symbol => bit_string}`;
  single-leaf tree convention: `%{symbol => "0"}`.
- `code_for/2` — direct symbol lookup without building the full table; returns
  `nil` for unknown symbols.
- `canonical_code_table/1` — DEFLATE-style canonical Huffman codes sorted by
  `{code_length, symbol_value}` and assigned numerically; only code lengths
  need to be transmitted, not the tree structure.
- `decode_all/3` — bit-string decoder walking the tree for exactly `count`
  symbols; handles single-leaf trees (each `"0"` decodes one symbol) and
  multi-leaf trees (bit index already advanced after reaching a leaf).
- `weight/1` — O(1) root weight (sum of all leaf frequencies).
- `depth/1` — O(n) maximum code length (deepest leaf depth).
- `symbol_count/1` — O(1) count of distinct symbols stored at construction.
- `leaves/1` — in-order (left-to-right) leaf traversal returning
  `[{symbol, code}, ...]`.
- `is_valid/1` — structural invariant checker: (1) full binary tree, (2)
  weight invariant, (3) no duplicate symbols.
- 60+ ExUnit tests covering spec vectors (AAABBC example), tie-breaking rules,
  single-symbol edge cases, prefix-free verification, round-trip encode/decode,
  256-symbol alphabet, balanced trees, and error handling.
- Depends on `coding_adventures_heap` (path `../heap`) for the `MinHeap`
  priority queue used during tree construction.
