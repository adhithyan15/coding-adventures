# Changelog — micro-qr (Kotlin)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning 2.0](https://semver.org/).

---

## [0.1.0] — 2026-04-26

### Added

- **`MicroQR.kt`** — full Micro QR Code encoder, ISO/IEC 18004:2015 Annex E.
  - Supports all four symbol versions: M1 (11×11), M2 (13×13), M3 (15×15),
    M4 (17×17).
  - Supports all valid ECC levels: DETECTION (M1), L/M (M2/M3), L/M/Q (M4).
  - Automatic symbol and ECC selection: iterates configurations smallest-first
    and returns the first that fits the input.
  - Three encoding modes: NUMERIC (0–9), ALPHANUMERIC (45-char set), BYTE
    (raw UTF-8 bytes).
  - Reed-Solomon error correction over GF(256)/0x11D with b=0 convention.
    Pre-computed generator polynomials for all six ECC counts used by Micro QR
    (2, 5, 6, 8, 10, 14 codewords).
  - Two-column zigzag data placement scanning bottom-right to top-left.
  - Four mask patterns (0–3) with full four-rule QR penalty scoring; auto or
    forced selection via `MicroQROptions.maskPattern`.
  - Pre-computed 15-bit format information table (all 32 entries, XOR 0x4445).
  - Returns an immutable `ModuleGrid` (both inner row lists and outer list are
    unmodifiable).

- **`MicroQROptions`** data class — all fields optional (`null` = auto):
  - `symbol`: "M1"/"M2"/"M3"/"M4" or null (case-insensitive).
  - `eccLevel`: `ECCLevel` enum value or null.
  - `maskPattern`: 0–3 or null.

- **`ECCLevel`** enum: `DETECTION`, `L`, `M`, `Q`.

- **`MicroQRError`** sealed class hierarchy (subtypes of `Exception`):
  - `InputTooLong` — data exceeds symbol capacity.
  - `InvalidECCLevel` — requested version+ECC combination is not in the spec.
  - `InvalidOptions` — out-of-range mask pattern or unrecognised symbol string.

- **`VERSION`** top-level constant = `"0.1.0"`.

- **`MicroQRTest.kt`** — comprehensive JUnit 5 test suite (100+ assertions):
  - VERSION constant value.
  - Error hierarchy: each `MicroQRError` subtype is both a `MicroQRError` and
    an `Exception`.
  - Smoke tests (encode returns non-null `ModuleGrid`).
  - Grid shape (always square, `rows == cols`).
  - Module list dimensions match `rows`/`cols` fields.
  - Module values are Booleans.
  - Determinism — same input always produces identical output.
  - Auto-version selection for single digit, 5/6 digits, alphanumeric, byte,
    and URL inputs.
  - All four symbol versions explicitly tested.
  - Module counts: 121 (M1), 169 (M2), 225 (M3), 289 (M4).
  - Finder pattern: outer ring dark, inner ring light, 3×3 core dark.
  - Finder pattern invariant across all symbol sizes.
  - Separator: row 7 cols 0–7 and col 7 rows 0–7 all light.
  - Timing: row 0 and col 0 from position 8 onward alternate dark/light.
  - Format info: non-zero, differs across ECC levels.
  - ECC levels: all 8 valid configurations tested.
  - L vs M produce different grids.
  - M4 L/M/Q all differ.
  - Invalid ECC combinations throw `MicroQRError.InvalidECCLevel`.
  - Invalid options throw `MicroQRError.InvalidOptions`.
  - Too-long input throws `MicroQRError.InputTooLong`.
  - RS encoder: all-zero data → all-zero ECC, output length, different data
    different ECC, idempotency.
  - Mask conditions 0–3, out-of-range returns false.
  - Penalty scorer: all-dark (178), all-light (168), checkerboard (0),
    run-of-5, 2×2 block.
  - Forced mask patterns 0–3 all produce valid grids.
  - Capacity boundary: M1 max, M1 overflow, M2-L max, M4 max, empty string.
  - Single-character edge cases: digit 0, letter A, lowercase a.
  - Cross-language corpus (6 inputs, expected symbol sizes from spec).
  - ModuleGrid immutability: inner row and outer list throw on mutation attempt.

- **`README.md`** — installation, usage examples, API reference, ECC table,
  dependency list, and encoding pipeline summary.

- **`CHANGELOG.md`** — this file.

### Implementation notes

- Ported closely from `code/packages/java/micro-qr/src/main/java/…/MicroQR.java`
  with idiomatic Kotlin style: sealed classes, data classes, `when` expressions,
  extension-like top-level functions, `internal` visibility for test helpers.

- The `WorkGrid` internal class uses mutable `Array<BooleanArray>` for
  performance during encoding; the final `ModuleGrid` returned to callers uses
  `List<List<Boolean>>` which is fully immutable.

- M1 half-codeword handling: the last data codeword for M1 contributes only 4
  bits (upper nibble).  The RS encoder still receives 3 full bytes; the lower
  nibble is always 0.

- `rsEncode` and `maskCondition` and `computePenalty` are `internal` to allow
  direct unit testing from the test module without making them part of the
  public API.
