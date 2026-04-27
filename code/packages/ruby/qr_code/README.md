# coding_adventures_qr_code

A complete QR Code encoder in Ruby, following the ISO/IEC 18004:2015 standard.
Produces scannable QR Codes from any UTF-8 string.

## Overview

QR Code (Quick Response) was invented by Masahiro Hara at Denso Wave in 1994 to
track automotive parts at 10× the speed of 1D barcodes.  Today it appears on
every product label, restaurant menu, bus stop timetable, and business card.

This package is part of the **coding-adventures** monorepo.  It sits in the
barcode stack between `barcode_2d` (the abstract `ModuleGrid` type) and
`paint_instructions` / `paint_vm` (the rendering backends).

```
Input string
  → QrCode.encode()      ← this package
  → ModuleGrid
  → Barcode2D.layout()
  → PaintScene
  → SVG / Metal / Canvas / terminal
```

## Installation

Add to your Gemfile:

```ruby
gem "coding_adventures_qr_code", path: "../qr_code"
```

## Usage

```ruby
require "qr_code"

# Encode a string — returns a CodingAdventures::Barcode2D::ModuleGrid
grid = QrCode.encode("https://example.com", level: :M)

grid.rows        # => 29   (version 3, 29×29 for this URL at M level)
grid.cols        # => 29
grid.modules[0][0]  # => true (dark module — top-left corner of finder pattern)

# Encode and convert to a PaintScene for rendering
scene = QrCode.encode_to_scene("https://example.com", level: :M)
# scene.instructions is an array of PaintRect commands
```

## API

### `QrCode.encode(data, level: :M, version: 0, mode: nil)`

Encodes `data` (a UTF-8 String) into a `ModuleGrid`.

| Parameter | Type         | Default | Description                                      |
|-----------|--------------|---------|--------------------------------------------------|
| `data`    | `String`     | —       | Input to encode (UTF-8)                          |
| `level:`  | `:L/:M/:Q/:H`| `:M`    | Error correction level                           |
| `version:`| `Integer`    | `0`     | QR version 1–40; 0 = auto-select smallest        |
| `mode:`   | Symbol/nil   | `nil`   | `:numeric`/`:alphanumeric`/`:byte`; nil = auto   |

**Returns** a frozen `CodingAdventures::Barcode2D::ModuleGrid`.

**Raises** `QrCode::InputTooLongError` if the input exceeds version 40 capacity.

### `QrCode.encode_to_scene(data, level: :M, version: 0, mode: nil, config: nil)`

Encodes and immediately converts to a `PaintScene` via `Barcode2D.layout()`.
The `config` hash is passed to `layout()` (controls `module_size_px`,
`quiet_zone_modules`, `foreground`, `background`).

## Error correction levels

| Level | Recovery | Use case                        |
|-------|----------|---------------------------------|
| `:L`  | ~7%      | Maximum data density            |
| `:M`  | ~15%     | General-purpose (common default)|
| `:Q`  | ~25%     | Moderate noise or damage        |
| `:H`  | ~30%     | Heavy damage, logo overlaid     |

## Encoding pipeline

1. **Mode selection** — numeric if all digits; alphanumeric if in the 45-char
   set; otherwise byte (raw UTF-8).
2. **Version selection** — smallest version 1–40 whose data capacity fits.
3. **Bit stream** — mode indicator (4b) + char count + data + terminator + padding.
4. **ECC blocks** — data split across blocks; each block gets Reed-Solomon ECC
   codewords computed over GF(256) with b=0 convention.
5. **Interleaving** — round-robin across blocks (data first, then ECC).
6. **Grid init** — finder patterns, separators, timing strips, alignment patterns,
   format/version info reserved.
7. **Zigzag placement** — bits placed in two-column snake from bottom-right.
8. **Mask evaluation** — all 8 ISO 18004 patterns scored; lowest-penalty wins.
9. **Finalize** — format information and version information written.
10. **Return** — frozen `ModuleGrid`.

## Dependencies

| Package                         | Role                                 |
|---------------------------------|--------------------------------------|
| `gf256`                         | GF(2^8) arithmetic for Reed-Solomon  |
| `coding_adventures_barcode_2d`  | `ModuleGrid` type and `layout()`     |
| `coding_adventures_paint_instructions` | `PaintScene` (transitive)     |

## Running tests

```bash
cd code/packages/ruby/qr_code
bundle install
bundle exec rake test
```

## Linting

```bash
bundle exec standardrb lib/
```
