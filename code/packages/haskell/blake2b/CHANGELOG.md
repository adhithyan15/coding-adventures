# Changelog

All notable changes to this package are documented in this file.

## [0.1.0] — 2026-04-19

### Added
- Initial from-scratch BLAKE2b (RFC 7693) implementation in pure
  Haskell.  No external dependencies beyond `base`.
- `blake2b`, `blake2bHex`, `blake2bWith`, `blake2bHexWith` one-shot
  functions.
- `Params` record and `defaultParams` for specifying `digestSize`,
  `key`, `salt`, and `personal`.
- Test suite mirrors the Python / Go / TypeScript / Rust / Ruby /
  Elixir / Swift / Lua / Perl KAT tables: block boundaries (0, 1, 63,
  64, 65, 127, 128, 129, 255, 256, 257, 1024), variable digest sizes,
  keyed mode across 1/16/32/64-byte keys, salt+personal, and parameter
  validation.

### Notes
- Sequential mode only. Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
  BLAKE2Xb, and BLAKE3 are out of scope per the HF06 spec.
- A streaming `Hasher` value type is deliberately omitted to keep the
  public surface small.  The one-shot API covers every KAT in the
  cross-language suite; callers who need incremental hashing can wrap
  the parameterised form with their own buffering.
