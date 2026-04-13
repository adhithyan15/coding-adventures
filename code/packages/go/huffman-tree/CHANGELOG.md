# Changelog — huffman-tree (Go)

## [0.1.0] — 2026-04-12

### Added
- Initial implementation of DT27 Huffman Tree data structure.
- `Build(weights []WeightPair) (*HuffmanTree, error)` — greedy min-heap construction with 4-field deterministic tie-breaking (weight, isInternal, symbol, insertionOrder).
- `CodeTable(t) map[int]string` — tree-walk encoding table (left="0", right="1"; single-leaf convention "0").
- `CodeFor(t, symbol) (string, bool)` — single-symbol lookup without building full table.
- `CanonicalCodeTable(t) map[int]string` — DEFLATE-style canonical codes derived from code lengths.
- `DecodeAll(t, bits, count) ([]int, error)` — bit-stream decoder; handles single-leaf trees (one "0" bit per symbol) and multi-leaf trees correctly.
- `Weight(t)`, `Depth(t)`, `SymbolCount(t)`, `Leaves(t)`, `IsValid(t)` — inspection helpers.
- Comprehensive test suite (>90% coverage) covering: construction, tie-breaking determinism, code table correctness, prefix-free property, canonical code assignment, round-trip encode/decode, edge cases, and error handling.
- Depends on `code/packages/go/heap` (MinHeap generic implementation).
