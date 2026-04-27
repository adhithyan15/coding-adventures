# Changelog — HuffmanTree (Swift)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-12

### Added

- Initial implementation of `HuffmanTree` (DT27).
- Depends on the standalone `Heap` package for the generic min-heap used during
  deterministic greedy construction.
- Private `PriorityKey: Comparable` struct encoding the four-level tie-breaking
  rule: `{weight, leafFlag, symbolOrMax, orderOrMax}`.
- Private `Node` indirect enum with `.leaf(symbol:weight:)` and
  `.internal(weight:left:right:order:)` cases.
- `HuffmanTree.build(_ weights:)` — `throws` greedy construction from
  `[(symbol: Int, frequency: Int)]` pairs with deterministic tie-breaking
  matching the Python reference implementation.
- `codeTable()` — returns `[Int: String]` for all symbols.
- `codeFor(_ symbol:)` — single-symbol lookup without building the full table.
- `canonicalCodeTable()` — DEFLATE-style canonical Huffman codes.
- `decodeAll(_ bits:count:)` — `throws` decoder for exact symbol count.
- `weight: Int` computed property — total weight (O(1)).
- `depth: Int` computed property — maximum code length (O(n)).
- `symbolCount: Int` — number of distinct symbols (O(1)).
- `leaves()` — in-order left-to-right traversal returning `[(Int, String)]`.
- `isValid()` — structural invariant checker.
- `HuffmanTree.HuffmanError` enum: `.emptyWeights`, `.invalidFrequency`,
  `.bitStreamExhausted`.
- XCTest suite covering: build validation, code table, prefix-free property,
  canonical codes, decode round-trips, edge cases (single/two/eight symbols,
  all equal weights), determinism, byte-range round-trip.
- `Package.swift` (swift-tools-version 5.9) with library and test targets.
- `README.md` with usage examples and API reference.
