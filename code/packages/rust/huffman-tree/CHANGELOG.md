# Changelog — huffman-tree

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-12

### Added

- Initial implementation of `HuffmanTree` (DT27).
- `Node` enum with `Leaf` and `Internal` variants.
- `HuffmanTree::build` — greedy min-heap construction from `(symbol, frequency)`
  pairs using the `heap` crate's `MinHeap<T: Ord>`.
- Deterministic 4-tuple tie-breaking: `(weight, is_internal, symbol_or_max,
  order_or_max)` — ensures identical tree shapes across all language
  implementations in this monorepo.
- `code_table` — full `HashMap<u16, String>` of tree-walk codes.
- `code_for` — single-symbol lookup without building the full table.
- `canonical_code_table` — DEFLATE-style canonical codes, sorted by
  `(length, symbol)` and assigned numerically.
- `decode_all` — decode exactly `count` symbols from a bit string.
  - Single-leaf trees consume one `'0'` bit per symbol by convention.
  - Multi-leaf trees do not over-advance the bit index after landing on a leaf.
- `weight`, `depth`, `symbol_count`, `leaves`, `is_valid` inspection methods.
- 32 unit tests and 12 doc-tests covering:
  - Construction errors (empty, zero frequency).
  - Single-symbol edge case.
  - Classic AAABBC 3-symbol example with exact tie-breaking verification.
  - Round-trip encode → decode for 2-symbol, 4-symbol, and 256-symbol alphabets.
  - Prefix-free property verification.
  - Canonical code length invariant.
  - Decode stream exhaustion error.
