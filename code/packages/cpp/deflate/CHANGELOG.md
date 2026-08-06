# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-08-05

### Added

- Pure ISO C++17, header-only port of the Rust `deflate` crate (CMP05), in
  namespace `ca::deflate`: full RFC 1951 DEFLATE — stored, fixed-Huffman, and
  dynamic-Huffman blocks, in both directions.
- `compress(const Bytes&) -> Bytes`: LZSS tokenization (via the sibling
  `lzss` package, CMP02, full 32768-byte window) followed by exact-bit-count
  comparison between a fixed-Huffman and a length-limited (package-merge,
  Larmore–Hirschberg 1990) dynamic-Huffman encoding of the same token stream,
  emitting whichever is smaller as a single `BFINAL=1` block. Never fails.
- `inflate(const Bytes&) -> Bytes` / `decompress(const Bytes&) -> Bytes`
  (alias): decodes all three RFC 1951 block types, including the full
  distance-code (0–29) and length-code (up to symbol 285) alphabets real
  producers (zlib, gzip, Microsoft Office) use — not just the subset this
  library's own encoder emits. Throws `ca::deflate::DeflateException`
  (carrying a `DeflateError`) on malformed input, mirroring the sibling
  `canonical-cbor` package's exception-based convention.
- Robustness: 256 MB output cap against decompression bombs
  (`detail::MAX_INFLATE_OUTPUT`), overflow-checked size arithmetic before
  every allocation/`push_back`, back-reference distance validated against
  bytes decoded so far, and length/distance Huffman symbols validated against
  their tables before use.
- Length-limited Huffman code construction (package-merge) carries a hard,
  always-on (not debug-only) Kraft-inequality and max-length invariant check,
  matching the Rust reference's release-mode `assert!`s — a violation throws
  rather than silently emitting an invalid RFC 1951 stream.
- Tests: the CMP05 spec's exact byte-level vectors (empty input, "AAABBC"
  fixed encoding, overlapping back-references), round-trip invariants across
  all 256 byte values and mixed/repetitive data, dynamic-vs-fixed block
  selection, and adversarial malformed-input cases (truncated streams,
  reserved BTYPE, corrupted stored-block LEN/NLEN, out-of-range
  back-reference) — plus decoding a **real** dynamic-Huffman raw-DEFLATE blob
  produced independently by CPython's `zlib.compressobj(9, zlib.DEFLATED,
  -15)`, proving the decoder reads dynamic Huffman it never produced itself.
- Verified clean under GCC and Clang with `-std=c++17 -pedantic-errors
  -Wall -Wextra -Werror`, plus `-Wconversion -Wsign-conversion -Wshadow` as
  an additional local check (MSVC is exercised in CI via the shared
  `iso-harness`).
