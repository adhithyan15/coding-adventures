# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-09-07

Initial release, per `code/specs/MA04-qr-decoder.md`.

### Added

- `decode(grid: &ModuleGrid) -> Result<Vec<u8>, QrDecodeError>` — the sole
  public function. Inverts `qr-code`'s own encoder: validates grid size,
  reads format info (Copy 1 then Copy 2 fallback), reads version info for
  V>=7 (Copy 1 then Copy 2, must agree with the size-derived version),
  unmasks every non-reserved module, walks the shared zigzag traversal
  order to recover the raw bit stream, de-interleaves into per-block
  (data, ecc) sequences using the same block-size arithmetic as the
  encoder's own block construction, corrects each block via
  `reed_solomon::decode_with_base(..., 0)`, and parses the one data segment
  (numeric / alphanumeric / byte mode) qr-code's encoder can ever produce.
- `QrDecodeError` with six distinct, content-free variants: `InvalidSize`,
  `FormatInfoUnreadable`, `VersionInfoUnreadable`,
  `UnrecoverableBlock(usize)`, `UnsupportedMode(u8)`, `Truncated`.
- `#![forbid(unsafe_code)]`; every bit/byte/module read is bounds-checked
  (no `.unwrap()`/`.expect()` reachable on a malformed-but-correctly-sized
  `ModuleGrid`, since `ModuleGrid` is publicly hand-constructible).
- 30 tests (28 unit tests in `src/tests.rs` + 2 doctests): an exhaustive
  byte-mode round-trip across all 40 versions x all 4 ECC levels (160
  encode-then-decode cases, each sized to force its exact target version),
  representative numeric- and alphanumeric-mode sweeps spanning version 1
  through version 35+, an `otpauth://totp/...` URI round trip (the actual
  driving use case) at every ECC level, corruption-and-recovery within a
  single RS block's correction capacity (data bytes, ECC bytes, and a
  multi-block grid), beyond-capacity corruption correctly rejected as
  `UnrecoverableBlock` (not silently wrong), format-info Copy-2 fallback,
  version-info-both-copies-corrupted and version-disagrees-with-size
  rejection, five `InvalidSize` shapes (non-square, wrong-size-square,
  too-small, too-large, and a ragged hand-built `modules` field that must
  not panic), a direct encoder/decoder wire-format cross-check, and
  dedicated tests for every `QrDecodeError` variant.

### Depends on new `pub` surface added to sibling crates in this same change

- `qr-code` 0.2.0: promoted `symbol_size`/`num_raw_data_modules`/
  `num_data_codewords`/`num_remainder_bits`/`mask_condition` to `pub`;
  added `ecc_codewords_per_block`/`num_blocks`/`reserved_modules`/
  `data_module_order`/`read_format_info`/`read_version_info`/
  `ecc_from_indicator`. All additive; `encode()`'s observable behavior is
  unchanged (proven by its full pre-existing test suite passing unmodified).
- `reed-solomon` 0.1.0 (still 0.1.0, no published bump yet): added
  `build_generator_with_base`/`syndromes_with_base`/`decode_with_base(_, _,
  b)` so QR's b=0 Reed-Solomon root convention can be decoded without
  reimplementing Berlekamp-Massey/Chien-search/Forney. Found and fixed a
  real bug in the process: Forney's error-magnitude formula needs a
  base-dependent correction factor that vanishes at b=1 (this crate's
  original-only convention) but is required at b=0. Also relaxed the
  `_with_base` variants' `n_check` validation to allow odd values, since
  real QR codes use odd ECC-codewords-per-block counts (e.g. 7 at version 1
  / ECC level L) — the even-only restriction was never a mathematical
  requirement, just this crate's original API choice, preserved unchanged
  in the b=1-only `build_generator`/`decode` wrappers.

### Out of scope

- **Locating a QR code within an arbitrary raster image** — pixel
  binarization, finder-pattern scanning, perspective correction. This
  crate starts one step later than a real-world scanner: it consumes an
  already-located, already-sampled `ModuleGrid`. Tracked as separate
  follow-up work (`MA04-qr-decoder.md` §6).
- **Full format-info/version-info error correction** (nearest-valid-
  codeword search by Hamming distance) — only the two-copy fallback ISO
  18004 already provides is implemented, not active error correction on a
  single damaged copy.
- **Kanji mode, ECI, structured append, mixed-mode segments** — none of
  these exist in the paired `qr-code` encoder either.
- **Micro QR, rMQR** — tracks whatever `qr-code` eventually supports, not
  ahead of it.
