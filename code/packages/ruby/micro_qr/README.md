# coding_adventures_micro_qr

Micro QR Code encoder — ISO/IEC 18004:2015 Annex E compliant, written in pure Ruby.

## What is Micro QR Code?

Micro QR Code is the compact variant of QR Code, designed for applications where even
the smallest regular QR Code (21×21 at version 1) is too large. Common use cases include
surface-mount component labels, PCB markings, and miniature industrial tags.

### Symbol sizes

```
M1: 11×11    M2: 13×13    M3: 15×15    M4: 17×17
formula: size = 2 × version_number + 9
```

### Key differences from regular QR Code

- **Single finder pattern** — one 7×7 square at top-left only (not three)
- **Timing at row 0 / col 0** — not row 6 / col 6
- **Only 4 mask patterns** — not 8
- **Format XOR mask 0x4445** — not 0x5412
- **Single copy of format info** — not two
- **2-module quiet zone** — not 4
- **Narrower mode indicators** — 0–3 bits instead of 4
- **Single block** — no RS interleaving

## Where it fits in the stack

```
code/packages/ruby/
  gf256/              ← GF(256) field arithmetic (require_relative dep)
  paint_instructions/ ← PaintScene IR (runtime dep)
  barcode_2d/         ← ModuleGrid + layout() (runtime dep)
  micro_qr/           ← this package
```

## Installation

```ruby
# In your Gemfile
gem "coding_adventures_micro_qr", path: "path/to/micro_qr"
gem "coding_adventures_barcode_2d", path: "path/to/barcode_2d"
gem "coding_adventures_paint_instructions", path: "path/to/paint_instructions"
```

## Usage

```ruby
require "coding_adventures_micro_qr"

include CodingAdventures::MicroQR

# Auto-select smallest symbol
grid = encode("HELLO")
grid.rows  # => 13   (M2 symbol, 13×13 modules)
grid.cols  # => 13
grid.module_shape  # => "square"

# Force a specific version and ECC level
m4_grid = encode("HELLO",
  version: MicroQRVersion::M4,
  ecc:     MicroQREccLevel::Q
)
m4_grid.rows  # => 17

# Encode and convert to PaintScene in one call
scene = encode_and_layout("12345")
scene.width   # => 150 px  (11 + 4 quiet) × 10 px/module
scene.height  # => 150 px

# Render at a custom module size
scene = encode_and_layout("HELLO",
  config: { module_size_px: 5, quiet_zone_modules: 2 }
)
```

## ECC level availability

| Level     | Available in     | Recovery  |
|-----------|-----------------|-----------|
| Detection | M1 only         | detects errors only |
| L         | M2, M3, M4      | ~7 % of codewords |
| M         | M2, M3, M4      | ~15 % of codewords |
| Q         | M4 only         | ~25 % of codewords |

Level H is not available in any Micro QR symbol.

## Data capacities

| Symbol | Numeric | Alphanumeric | Byte |
|--------|---------|-------------|------|
| M1     | 5       | —           | —    |
| M2-L   | 10      | 6           | 4    |
| M2-M   | 8       | 5           | 3    |
| M3-L   | 23      | 14          | 9    |
| M3-M   | 18      | 11          | 7    |
| M4-L   | 35      | 21          | 15   |
| M4-M   | 30      | 18          | 13   |
| M4-Q   | 21      | 13          | 9    |

## Encoding pipeline

```
input string
  → auto-select symbol (M1..M4) and mode (numeric / alphanumeric / byte)
  → build bit stream (mode indicator + char count + data + terminator + padding)
  → Reed-Solomon ECC — GF(256)/0x11D, b=0, single block
  → initialize grid (finder, L-shaped separator, timing, format reserved)
  → zigzag data placement (two-column snake from bottom-right)
  → evaluate 4 mask patterns, pick lowest penalty
  → write format information (15 bits, single copy, XOR 0x4445)
  → ModuleGrid
```

## Running tests

```bash
bundle install
bundle exec rake test
```
