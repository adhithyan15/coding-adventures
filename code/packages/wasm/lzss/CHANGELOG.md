# Changelog

## [0.1.0] - 2026-04-25

### Added

- Initial implementation: WebAssembly bindings for LZSS compression (CMP02).
- `compress(data: &[u8]) -> Vec<u8>` — encodes bytes into the CMP02 wire
  format (8-byte header + flag-bit blocks).
- `decompress(data: &[u8]) -> Vec<u8>` — recovers original bytes from CMP02
  wire format.
- Native (`cargo test`) test suite covering round-trip fidelity, wire-format
  correctness, compression effectiveness, and safety against malformed input.
- `wasm-bindgen-test` stubs for browser/Node execution via `wasm-pack test`.
- Literate inline comments explaining the CMP02 flag-bit block structure and
  the CMP series context (LZ77 → LZ78 → LZSS → LZW → Huffman → DEFLATE).
