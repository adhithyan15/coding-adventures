# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-04-12

### Added

- Initial release of `coding-adventures-huffman-tree` (DT27).
- `CodingAdventures::HuffmanTree.build` — greedy min-heap construction with
  deterministic tie-breaking (weight → leaf-before-internal → symbol-value →
  FIFO insertion order).
- `HuffmanTree#code_table` — returns `{symbol => bit_string}` encoding map;
  single-leaf trees use the convention `{symbol => "0"}`.
- `HuffmanTree#code_for` — look up the bit string for a single symbol without
  building the full table; returns `nil` for unknown symbols.
- `HuffmanTree#canonical_code_table` — DEFLATE-compatible canonical codes
  derived from code lengths alone; allows the receiver to reconstruct the
  table without the tree structure.
- `HuffmanTree#decode_all` — decode exactly N symbols from a '0'/'1' bit
  string by walking the tree; raises `ArgumentError` on stream exhaustion.
- `HuffmanTree#weight` — root weight (sum of all leaf frequencies), O(1).
- `HuffmanTree#depth` — maximum code length (deepest leaf depth), O(n).
- `HuffmanTree#symbol_count` — number of distinct symbols, O(1).
- `HuffmanTree#leaves` — in-order traversal returning `[[symbol, code], ...]`.
- `HuffmanTree#valid?` — structural invariant check (full binary tree, weight
  sums, no duplicate symbols).
- `HuffmanLeaf` and `HuffmanInternal` node classes.
- Depends on `coding_adventures_heap` for the min-heap.
- RSpec test suite with >90% coverage (`spec/huffman_tree_spec.rb`).
- `BUILD` file: runs `standardrb` lint then `rspec`.
