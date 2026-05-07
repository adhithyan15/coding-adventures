# Changelog — AztecCode

All notable changes to this package are documented here.
Follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) conventions.

---

## [0.1.0] — 2026-05-06

### Added

- **Complete Aztec Code encoder** — ISO/IEC 24778:2008 compliant.

- **Compact Aztec support** — 1–4 layers (15×15 to 27×27 modules).

- **Full Aztec support** — 1–32 layers (19×19 to 143×143 modules).

- **Bullseye finder pattern** — concentric rings at Chebyshev distances 0–5
  (compact) or 0–7 (full). Inner 3×3 core always dark; rings alternate
  light/dark from distance 2 outward.

- **Orientation marks** — four dark corner modules at the mode message ring
  corners, breaking rotational symmetry for scanner orientation detection.

- **Mode message encoding** — GF(16)/0x13 Reed-Solomon protection for the
  layer count and data codeword count:
  - Compact: (7,2) RS code over GF(16) → 28-bit mode message
  - Full:    (10,4) RS code over GF(16) → 40-bit mode message

- **GF(256)/0x12D Reed-Solomon ECC** — same primitive polynomial as Data
  Matrix ECC200 (0x12D), distinct from QR Code's 0x11D. b=1 convention
  (roots α^1..α^n).  Implemented inline (the repo's GF256 package uses 0x11D).

- **Bit stuffing** — inserts one complement bit after every run of 4
  consecutive identical bits, preventing scanner interference with the
  reference grid.

- **Reference grid** — for full symbols, alternating dark/light grid lines
  every 16 modules from center (horizontal and vertical), aiding perspective
  correction.

- **Binary-Shift encoding** — all input encoded via Binary-Shift escape from
  Upper mode.  Handles strings up to 2047 bytes via 11-bit length field.

- **Automatic symbol selection** — picks the smallest compact or full symbol
  that fits the input at the requested ECC level (default 23%).  Conservative
  20% stuffing overhead factored into the fit test.

- **`AztecCode.encode(_:options:)`** — main public API returning `[[Bool]]`.

- **`AztecCode.encodeData(_:options:)`** — encode raw `[UInt8]` bytes.

- **`AztecCode.encodeToGrid(_:options:)`** — returns a `ModuleGrid` for use
  with the `Barcode2D` rendering pipeline.

- **`AztecOptions`** — `minEccPercent` (default 23, clamped to 10–90).

- **`AztecError`** — `inputTooLong` and `internalError` error cases.

- **62 unit tests** in 9 suites:
  - GF(16) arithmetic (7 tests)
  - GF(256)/0x12D RS encoding (6 tests)
  - Bit stuffing (9 tests)
  - Symbol size selection (7 tests)
  - Bullseye finder pattern (7 tests)
  - Full encode integration (9 tests)
  - Options (4 tests)
  - Binary-Shift data encoding (6 tests)
  - Cross-language test vectors (7 tests)

### v0.1.0 simplifications (documented for future work)

- Byte-mode only via Binary-Shift from Upper mode; multi-mode optimisation
  (Digit/Upper/Lower/Mixed/Punct segmentation) deferred to v0.2.0.
- GF(16) and GF(32) RS for 4-bit/5-bit data codewords deferred to v0.2.0.
- Force-compact option deferred to v0.2.0.
