# Changelog — CodingAdventures::QrCode

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This package uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of ISO/IEC 18004:2015 compliant QR Code encoder.
- `encode($data, level => $ecc)` — encodes a UTF-8 string into a `ModuleGrid`
  hashref. Returns a `(4V+17) × (4V+17)` boolean grid.
- All four error correction levels: **L** (~7% recovery), **M** (~15%),
  **Q** (~25%), **H** (~30%).
- Three encoding modes: **numeric** (digits only), **alphanumeric** (45-char
  QR set), **byte** (arbitrary UTF-8).
- Automatic version selection (versions 1–40) — picks the smallest version
  whose data codeword capacity fits the input at the chosen ECC level.
- Reed-Solomon error correction codewords computed inline with the b=0
  convention (generator g(x) = ∏(x + αⁱ) for i=0..n-1 over GF(256)
  with primitive polynomial 0x11D), matching the QR Code standard.
- Interleaving of RS blocks (data codewords round-robin, then ECC codewords
  round-robin) as required by ISO 18004 Section 7.6.
- All function patterns: three 7×7 finder patterns, 1-module separators,
  horizontal/vertical timing strips, alignment patterns (versions 2+), dark
  module, format information (two copies), and version information (v7+).
- All 8 ISO 18004 mask patterns evaluated; lowest 4-rule penalty score wins.
- Penalty scoring: Rule 1 (same-colour runs ≥5), Rule 2 (2×2 blocks),
  Rule 3 (finder-pattern-like sequences), Rule 4 (dark ratio deviation).
- BCH(15,5) format information with 0x5412 mask and corrected bit ordering
  (f14 MSB at col 0/row n-8; f0 LSB at row 0/col n-1) per lessons.md
  2026-04-23 bug record.
- BCH(18,6) version information for symbols v7+.
- `$CodingAdventures::QrCode::VERSION` set to `'0.1.0'`.

### Implementation notes

- Format info bit placement follows the Rust port (verified with `zbarimg`),
  **not** the original TypeScript reference. The TypeScript reference had a
  reversed bit order that produced undecodable symbols. See `lessons.md`
  entry dated 2026-04-23.
- RS encoding is implemented inline (not via `CodingAdventures::ReedSolomon`)
  because the QR Code standard mandates the b=0 convention, which differs
  from the convention used by the reed-solomon package.
- The WorkGrid is mutable during construction for performance; the final
  `ModuleGrid` returned to callers is a plain hashref compatible with
  `CodingAdventures::Barcode2D`.
