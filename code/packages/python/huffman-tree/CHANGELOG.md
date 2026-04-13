# Changelog — huffman-tree (Python)

## 0.1.0 — 2026-04-11

### Added
- `HuffmanTree.build(weights)` — greedy min-heap construction from (symbol, frequency) pairs
- `code_table()` — returns {symbol: bit_string} map
- `code_for(symbol)` — returns bit string for a single symbol or None
- `canonical_code_table()` — DEFLATE-style canonical codes from lengths
- `decode_all(bits, count)` — decode exactly N symbols from a bit string
- `weight()`, `depth()`, `symbol_count()`, `leaves()`, `is_valid()` inspection methods
- Deterministic tie-breaking: weight → leaf-before-internal → symbol value → insertion order
- Full unit test suite with >90% coverage
