# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `lz78` crate (CMP01): the LZ78 lossless
  compression algorithm with the same trie-cursor encoder, parallel-dictionary
  decoder, and wire format.
- API: `lz78_encode` / `lz78_decode` (token arrays), `lz78_compress` /
  `lz78_decompress` (one-shot CMP01 wire format), and the reusable
  `Lz78TrieCursor` (`_new`/`_step`/`_insert`/`_reset`/`_dict_id`/`_at_root`/
  `_free`). Returns `LZ78_OK` / `LZ78_ERR_ALLOC`; malloc-owned outputs; all
  growable buffers overflow-guarded.
- Robustness: `lz78_decode` / `lz78_decompress` bounds- and cycle-check the
  dictionary so malformed input cannot cause an out-of-bounds read or infinite
  loop (the Rust would panic / hang); output is identical for well-formed
  streams.
- Tests use the crate's own token vectors (`AABCBBABC`, `ABABAB`, …), text and
  binary round trips, the max-dict cap, the wire-size invariant, determinism,
  and a malformed-input safety check, under GCC and Clang via `iso-harness`.
