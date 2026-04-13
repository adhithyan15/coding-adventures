# Changelog — CodingAdventures::HuffmanTree (Perl)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-12

### Added

- Initial implementation of `CodingAdventures::HuffmanTree` (DT27).
- Depends on `CodingAdventures::Heap` for the comparator-based min-heap used
  during deterministic greedy construction.
- `build(\@weights)` — greedy construction from `[[$symbol, $freq], ...]` pairs
  with four-level tie-breaking rules matching the Python reference implementation.
- `code_table()` — returns `{symbol => bitstring}` hashref for all symbols.
- `code_for($symbol)` — single-symbol lookup without building the full table.
- `canonical_code_table()` — DEFLATE-style canonical Huffman codes.
- `decode_all($bits, $count)` — decode exactly `$count` symbols from a bit string.
- `weight()` — total weight (root weight = sum of all leaf frequencies).
- `depth()` — maximum code length (depth of deepest leaf).
- `symbol_count()` — number of distinct symbols.
- `leaves()` — in-order left-to-right traversal returning `[$symbol, $code]` pairs.
- `is_valid()` — structural invariant checker returning 1 or 0.
- Comprehensive Test2::V0 test suite covering: build validation, code table
  correctness, prefix-free property, canonical codes, decode round-trips,
  edge cases (single symbol, two symbols, all equal weights), determinism.
- `Makefile.PL`, `cpanfile` for CPAN distribution.
- `README.md` with usage examples and API reference.
