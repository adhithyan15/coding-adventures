# Changelog — qr-code (Rust)

## 0.3.0 — 2026-09-07

Purely-additive `pub` surface extension per `code/specs/MA04-qr-decoder.md`
§3, so the new `qr-decoder` crate can invert this crate's own encoder
without duplicating its private tables/geometry/traversal logic. No
existing signature changed; `encode()`'s observable behavior is unchanged
(the full pre-existing 25-test suite passes without modification).

### Added

- Promoted `symbol_size`/`num_raw_data_modules`/`num_data_codewords`/
  `num_remainder_bits`/`mask_condition` from private to `pub` (bodies
  unchanged).
- `ecc_codewords_per_block(ecc, version)` / `num_blocks(ecc, version)` —
  accessors over the existing private `ECC_CODEWORDS_PER_BLOCK`/
  `NUM_BLOCKS` tables (the raw `static` arrays stay private).
- `reserved_modules(version) -> Vec<Vec<bool>>` — which modules are
  function patterns (finder/separator/timing/alignment/format/version/dark
  module) vs. data/ECC, backed by the same `build_grid` helper `encode()`
  itself already used, so both share one implementation.
- `data_module_order(version) -> Vec<(usize, usize)>` — the zigzag
  data/ECC-module traversal order, extracted from `place_bits`'s inline
  loop; `place_bits` now walks this shared order instead of a private copy,
  so encode (writing) and decode (reading) are guaranteed to agree by
  construction.
- `read_format_info(bits15) -> Option<(EccLevel, u32)>` — promoted from the
  test-only private `format_info_valid` helper; BCH-validates a raw
  (pre-unmask-XOR) 15-bit format word read off the grid.
- `read_version_info(bits18) -> Option<usize>` — new; mirrors
  `compute_version_bits`'s BCH generator (`0x1F25`) in the read direction.
- `ecc_from_indicator(bits: u16) -> Option<EccLevel>` — inverse of the
  existing private `ecc_indicator`.
- `VERSION` bumped to `"0.3.0"`.

### Testing

- 12 new unit tests covering the new `pub` surface directly (reserved-
  module/traversal self-consistency, format/version-info round-trips
  against the existing write-direction BCH functions, the
  `ecc_from_indicator`/`ecc_indicator` inverse relationship, and the
  new table accessors against the raw private arrays).
- Found and documented (not fixed — out of this change's scope) a
  pre-existing, unrelated bug: `place_all_alignments` skips placing an
  alignment pattern whenever its center cell is already `reserved`, which
  incorrectly also skips positions that only overlap the *timing* pattern
  (not a finder pattern) for every version >= 7. This makes
  `reserved_modules`/`data_module_order` report more "free" modules than
  `num_raw_data_modules(version)` predicts for v>=7 — confirmed across the
  full v7-v40 range and covered by a dedicated regression test
  (`data_module_order_exceeds_formula_for_versions_with_timing_overlap_alignment`).
  Does not affect this crate's own `encode`/`decode` round-trip (see
  `qr-decoder`'s pipeline, which only ever reads the first
  `num_raw_data_modules(version)` traversal slots — exactly what
  `place_bits` actually writes), but does make the emitted grid
  non-ISO-18004-conformant for a real scanner in the affected region.
  Filed as a separate follow-up task.

## 0.2.0 — 2026-05-11

### Added

- `render_png(data, ecc, config) → Result<Vec<u8>, String>` — one-shot convenience
  wrapper that encodes a QR Code and renders it to PNG bytes in a single call.
  Delegates pixel rendering to `barcode_2d::render_scene_png`, which dispatches
  to the platform-default backend (Metal on macOS, Direct2D on Windows, Cairo on
  Linux, Skia as fallback).  Pass `None` for `config` to use
  `Barcode2DLayoutConfig::default()`.

- `qr_code_render_png_produces_valid_png` test — verifies the 8-byte PNG magic
  signature `[0x89, 'P', 'N', 'G', CR, LF, 0x1A, LF]` is present in the output.

### Changed

- `VERSION` constant bumped to `"0.2.0"`.

---

## 0.1.0 — 2026-04-23

Initial release.

### Added

- `encode(input, ecc) → Result<ModuleGrid, QRCodeError>` — Full QR Code encoding pipeline:
  - Auto mode selection: numeric (digits only) → alphanumeric (45-char set) → byte (UTF-8)
  - Version selection: minimum version 1–40 that fits the input at the ECC level
  - Bit stream assembly: mode indicator + char count + data + terminator + padding (0xEC/0x11)
  - Block splitting and Reed-Solomon ECC (b=0 convention, GF(256)/0x11D)
  - Interleaving: data codewords round-robin, then ECC codewords round-robin
  - Grid initialization: three finder patterns, separators, timing strips, alignment patterns (v2+), dark module
  - Two-column zigzag data placement (bottom-right to top-left, timing column skipped)
  - 8 mask patterns evaluated with 4-rule ISO penalty scoring
  - Format information: BCH(15,5) with generator 0x537, XOR mask 0x5412
  - Version information (v7+): BCH(18,6) with generator 0x1F25

- `encode_and_layout(input, ecc, config) → Result<PaintScene, QRCodeError>` — encode + barcode-2d layout

- `EccLevel` enum — `L`, `M`, `Q`, `H`

- `QRCodeError` enum — `InputTooLong(String)`

- `VERSION` constant — `"0.1.0"`

- 24 unit tests + 1 doc-test; all passing
