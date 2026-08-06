# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-08-05

### Added

- Pure ISO C17 port of the Rust `deflate` crate (CMP05): DEFLATE (RFC 1951)
  compression built on LZSS tokenization (`c/lzss`, CMP02) plus a fixed/dynamic
  Huffman coder.
- API: `deflate_compress` / `deflate_decompress` (malloc'd out-params,
  `DeflateStatus` enum, mirroring `c/lzss`'s style), plus `deflate_free` for
  caller-owned output buffers.
- `deflate_compress` emits a standard RFC 1951 raw DEFLATE stream: a single
  final block, choosing fixed (`BTYPE=01`) or dynamic (`BTYPE=10`) Huffman by
  computing the exact bit cost of each and picking the smaller. Dynamic trees
  are length-limited to 15 bits (7 for the code-length alphabet) via the
  package-merge algorithm (Larmore-Hirschberg 1990), which provably yields the
  optimal length-limited code and always succeeds for our bounded alphabets
  (LL 286, distance 30, code-length 19, all well under `2^15`/`2^7`).
- `deflate_decompress` is the standard RFC 1951 `inflate`: reads all three
  block types (stored, fixed Huffman, dynamic Huffman) across as many blocks
  as the stream contains, so it decodes real `zlib`/`gzip`/ZIP/PNG streams —
  verified against a real `zlib`-produced dynamic-Huffman fixture in the test
  suite, not only this library's own encoder output.
- Robustness: output capped at `DEFLATE_MAX_OUTPUT` (256 MiB) against
  decompression bombs; every back-reference distance bounds-checked against
  bytes decoded so far; every decoded length/distance/code-length symbol
  range-checked before table lookup; stored-block `LEN`/`NLEN` cross-verified;
  no allocation sized directly from an attacker-controlled declared length.
  Malformed input returns `DEFLATE_ERR_MALFORMED` rather than undefined
  behaviour — verified via a 20,000-iteration random-byte-stream fuzz pass and
  a compressed-stream bit-flip sweep under AddressSanitizer + UBSan, with zero
  crashes, leaks, or sanitizer findings.
- Tests: the CMP05 spec's byte-exact vectors (`compress(b"") == 03 00`,
  `compress(b"AAABBC") == 73 74 74 74 72 72 06 00`, both cross-checked against
  Python's `zlib`), round trips across literals/matches/overlapping runs/all
  256 byte values/highly repetitive data/every length-code boundary, a
  hand-built stored block, a hand-built out-of-range back-reference, and a
  real `zlib`-produced dynamic-Huffman raw DEFLATE stream — under GCC and
  Clang via `iso-harness`.
