# Changelog — qr-code (Rust)

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
