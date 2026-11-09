# Changelog

All notable changes to `lz77` (C) are documented in this file.

## [Unreleased]

### Security

- Memory safety (CWE-125/CWE-191): lz77_decode computed b.len - offset without validating the offset, so a token with offset 0 or beyond the output wrapped the index and read outside (or through a NULL) buffer; lz77_decompress of {0,0,0,1, 0,5,3,'x'} segfaulted under ASan. Such streams are now rejected (return 0). The header token count was already clamped to the data present.
- Added hostile-header tests (header-claimed counts/lengths of 2^32-1, counts beyond the data) and round trips over every byte string of length <= 2 plus a seeded random sample to tests/*_test.c. Verified clean under -fsanitize=address,undefined (gcc) as well as the iso-harness gcc/clang runs.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `lz77` crate: `lz77_encode`, `decode`,
  `serialise`, `deserialise`, `compress`, `decompress` over `(offset, length,
  next_char)` tokens. malloc-owned outputs with overflow-guarded growth.
- Tests (via the shared `iso-harness`) covering token structure (literals + a
  backreference) and compress/decompress round-trips over assorted inputs —
  compiled and run under GCC, Clang, and MSVC with strict ISO-conformance flags.
