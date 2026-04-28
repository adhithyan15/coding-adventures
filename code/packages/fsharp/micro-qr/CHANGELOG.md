# Changelog — CodingAdventures.MicroQR.FSharp

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

---

## [0.1.0] — 2026-04-26

### Added

- **`MicroQR.fs`** — Full ISO/IEC 18004:2015 Annex E-compliant Micro QR Code encoder.

  - `encode : string -> MicroQROptions -> Result<ModuleGrid, MicroQRError>` — the
    single public entry point. Returns an immutable `ModuleGrid` (from
    `CodingAdventures.Barcode2D`) on success.

  - `ECCLevel` discriminated union: `Detection | L | M | Q`.

  - `MicroQROptions` record with optional `Symbol`, `ECCLevel`, and `MaskPattern`
    fields for fine-grained control; `defaultOptions` for fully-automatic
    selection.

  - `MicroQRError` discriminated union: `InputTooLong | InvalidECCLevel | InvalidOptions`.

  - `Version = "0.1.0"` package version constant.

- **Encoding modes** — automatic selection (most compact first):
  - **Numeric**: digits 0–9 only; groups of 3 → 10 bits, pair → 7 bits, single → 4 bits.
  - **Alphanumeric**: 45-character QR set (0–9, A–Z, space, `$%*+-./:`) with
    pair-packing (11 bits) and trailing single (6 bits).
  - **Byte**: raw UTF-8 bytes, each 8 bits.

- **All 8 symbol configurations** from Annex E:
  M1/Detection, M2/L, M2/M, M3/L, M3/M, M4/L, M4/M, M4/Q.

- **M1 half-codeword handling**: M1 uses 20 data-capacity bits (not a multiple
  of 8). The last data codeword carries data in its upper 4 bits and zeroes in
  the lower 4, as specified in the standard.

- **Reed-Solomon ECC** over GF(256)/0x11D with b=0 convention. Pre-computed
  monic generator polynomials for ECC counts 2, 5, 6, 8, 10, 14. LFSR
  polynomial-division algorithm, single block (no interleaving).

- **Grid construction**: finder pattern (7×7 top-left), L-shaped separator
  (row 7 + col 7), timing patterns at row 0 / col 0, format info reservation.

- **Two-column zigzag data placement** from bottom-right to top-left, skipping
  all reserved modules.

- **4 mask patterns** (subset of regular QR's 8):
  - Pattern 0: `(row + col) mod 2 = 0`
  - Pattern 1: `row mod 2 = 0`
  - Pattern 2: `col mod 3 = 0`
  - Pattern 3: `(row + col) mod 3 = 0`

- **Penalty scoring** (4 rules identical to regular QR):
  - Rule 1: adjacent runs of ≥5 same-colour modules.
  - Rule 2: 2×2 same-colour blocks.
  - Rule 3: finder-pattern-like 11-module sequences.
  - Rule 4: dark-module proportion deviation from 50%.

- **Format information**: pre-computed 32-entry table (8 symbol indicators × 4
  masks). 15-bit words already XOR-masked with 0x4445, placed in the L-shaped
  strip at row 8 (cols 1–8) and col 8 (rows 7–1).

- **`MicroQRTests.fs`** — xunit 2.9.2 test suite with 40+ tests covering:
  - Version constant
  - All four symbol sizes (M1=11, M2=13, M3=15, M4=17)
  - Grid is always square
  - ModuleShape is Square
  - Determinism (same input → same grid)
  - Auto-selection selects the smallest fitting symbol
  - All ECC levels (L, M, Q, Detection)
  - InputTooLong for over-capacity inputs
  - InvalidECCLevel for Q on M1/M2/M3, Detection on M2+
  - InvalidOptions for bad Symbol strings and out-of-range mask patterns
  - Forced mask pattern override
  - Numeric, alphanumeric, and byte mode encoding
  - M1 half-codeword encoding
  - Capacity boundary cases (at-cap and over-cap)
  - Structural module invariants (finder corner, separator corner, timing bit)

- **`README.md`** — package documentation with usage examples, API reference,
  ECC level availability table, encoding pipeline description, and dependency
  list.

- **`CHANGELOG.md`** — this file.

### Dependencies

- `CodingAdventures.Barcode2D.FSharp` `0.1.0` — `ModuleGrid`, `ModuleShape`,
  `Barcode2D.makeModuleGrid`, `Barcode2D.setModule`.
- `CodingAdventures.Gf256.FSharp` `0.1.0` — GF(256) field reference (RS
  arithmetic is implemented locally using the same primitive polynomial 0x11D).

### Notes

- This is a v0.1.0 release. Kanji mode is not implemented (Micro QR defines it
  for M4 only); the encoder falls back to byte mode for any input that includes
  Kanji characters.
- Level H ECC is not defined by the Micro QR standard and is not included.
