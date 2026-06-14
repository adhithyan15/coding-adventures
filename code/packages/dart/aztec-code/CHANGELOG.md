# Changelog

All notable changes to `coding_adventures_aztec_code` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-04-26

### Added

- Initial release: Aztec Code encoder conforming to ISO/IEC 24778:2008.

#### Constants

- `aztecCodeVersion` — package version string `'0.1.0'`.

#### Public API

- `encode(String data, {AztecOptions options})` — encode a string into the
  smallest Aztec symbol satisfying the requested ECC level.  Returns a
  `ModuleGrid` from `coding_adventures_barcode_2d`.
- `encodeAndLayout(String data, {AztecOptions?, Barcode2DLayoutConfig?})` —
  convenience wrapper: `encode` then `layoutGrid` in one step.
- `layoutGrid(ModuleGrid grid, {Barcode2DLayoutConfig?})` — convert an
  already-encoded `ModuleGrid` to a `PaintScene`.
- `explain(String data, {AztecOptions?})` — encode and return an
  `AnnotatedModuleGrid` (annotations are stubs in v0.1.0; per-module roles
  are planned for v0.2.0).

#### Types

- `AztecOptions` — encoding options (`minEccPercent`, default 23).
- `AztecError` — base error class (`implements Exception`).
- `InputTooLongError extends AztecError` — thrown when the payload exceeds
  the maximum capacity of a 32-layer full symbol.

#### Encoding

- **Binary-Shift from Upper mode** — all input encoded as raw 8-bit bytes
  using the 5-bit escape code (`11111`) followed by a 5-bit (short) or
  16-bit (long) length field.
- **Symbol selection** — compact layers 1–4 (15×15 to 27×27 modules) tried
  first; full layers 1–32 (19×19 to 143×143 modules) tried if compact
  cannot fit the payload.
- **Stuffing-overhead estimate** — 20% overhead (12/10 ceiling factor) applied
  before sizing to guarantee the stuffed stream always fits.

#### Error correction

- **GF(256)/0x12D Reed-Solomon** — same primitive polynomial as Data Matrix
  ECC200; b=1 convention (roots α^1..α^n).  Inline exp/log tables; no
  external GF package needed.
- **All-zero codeword avoidance** — last data codeword substituted with 0xFF
  when the padded bit stream would produce 0x00, per ISO/IEC 24778:2008 §7.3.1.1.
- **Bit stuffing** — complement bit inserted after every run of 4 identical
  bits in the combined data+ECC stream.

#### Mode message

- **GF(16) Reed-Solomon** — primitive polynomial x^4+x+1 = 0x13; inline
  log/antilog tables.
- **Compact mode message** — 2 data nibbles + 5 ECC nibbles = 28 bits.
- **Full mode message** — 4 data nibbles + 6 ECC nibbles = 40 bits.

#### Grid construction

- **Reference grid** (full symbols only) — alternating-parity lines at
  multiples of 16 modules from the centre.
- **Bullseye** — concentric square rings at the centre, dark at odd
  Chebyshev distances (and the 3×3 core), light at even distances.
- **Orientation marks** — 4 corners of the mode-message ring are always DARK,
  enabling 90°-rotation-invariant symbol decoding.
- **Mode message ring** — placed clockwise starting immediately after the
  top-left corner of the mode-message perimeter.
- **Data spiral** — clockwise layer-by-layer placement starting from the
  innermost data layer, 2 modules wide per layer.

#### Tests

- 11 test groups covering: package metadata, error types, symbol dimensions,
  compact symbol selection, full symbol selection, determinism, size scaling,
  `InputTooLongError`, `minEccPercent` option, layout wrappers, and
  structural invariants.
- Total: 33 unit tests.

### Dependencies

- `coding_adventures_barcode_2d` (path: `../barcode-2d`) — `ModuleGrid`,
  `AnnotatedModuleGrid`, `ModuleShape`, `Barcode2DLayoutConfig`, `layout()`,
  `PaintScene`.
