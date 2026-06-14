# coding_adventures_aztec_code

Aztec Code encoder — ISO/IEC 24778:2008 compliant, written in pure Ruby.

## What is Aztec Code?

Aztec Code was invented in 1995 by Andrew Longacre Jr. at Welch Allyn and
released as a patent-free format. Unlike QR Code (three large finder squares
at three corners), Aztec uses a **central bullseye** — a concentric square
locator at the very middle of the symbol.

Practical wins of the bullseye design:

- **No large quiet zone.** Three of QR's four sides need a "white moat"
  before the scanner can latch on. A center bullseye does not.
- **Rotation-invariant.** The bullseye is point-symmetric, so the symbol
  reads from any 90° rotation without disambiguation.

You see Aztec Code on:

- IATA boarding passes
- Eurostar / Amtrak rail tickets
- PostNL, Deutsche Post, La Poste mail
- US military ID cards

### Symbol sizes

| Variant | Layers | Module size            |
|---------|--------|------------------------|
| Compact | 1 – 4  | 15×15 to 27×27         |
| Full    | 1 – 32 | 19×19 to 143×143       |

Compact: `size = 11 + 4·layers`. Full: `size = 15 + 4·layers`.

## Where it fits in the stack

```
code/packages/ruby/
  paint_instructions/   ← PaintScene IR (runtime dep)
  barcode_2d/           ← ModuleGrid + layout() (runtime dep)
  aztec_code/           ← this package
```

## Installation

```ruby
# In your Gemfile
gem "coding_adventures_aztec_code",          path: "path/to/aztec_code"
gem "coding_adventures_barcode_2d",          path: "path/to/barcode_2d"
gem "coding_adventures_paint_instructions",  path: "path/to/paint_instructions"
```

## Usage

```ruby
require "coding_adventures/aztec_code"

include CodingAdventures

# Auto-select the smallest symbol that fits at default 23 % ECC
grid = AztecCode.encode("Hello, Aztec!")
grid.rows           # => 15   (Compact / 1 layer)
grid.cols           # => 15
grid.module_shape   # => "square"

# Bump the ECC to 50 % — typical for boarding passes
grid = AztecCode.encode("BOARDING PASS DATA", min_ecc_percent: 50)

# Larger payload — Compact maxes out at 81 bytes; expect a Full symbol.
grid = AztecCode.encode("A" * 100)
grid.rows           # => 19   (Full / 1 layer)

# Encode + layout in one shot — produces a PaintScene
scene = AztecCode.encode_and_layout("12345")
scene.width   # => (15 + 2·2) · 10 px = 190
scene.height  # => 190
```

## Encoding pipeline (v0.1.0)

```
input string / bytes
  → Binary-Shift codewords from Upper mode
  → smallest symbol that fits at the requested ECC level
  → pad to exact codeword count (with all-zero last cw → 0xFF rescue)
  → GF(256)/0x12D Reed-Solomon ECC (b = 1 — roots α^1 … α^n)
  → bit stuffing (insert complement bit after every 4 identical bits)
  → GF(16) mode message (layers + cw count + 5 or 6 RS nibbles)
  → ModuleGrid (bullseye → orientation marks → mode msg → data spiral)
```

## v0.1.0 simplifications

This is a faithful port of the TypeScript v0.1.0 reference. The full
multi-mode encoder lands in v0.2.0. Current restrictions:

1. **Byte-mode only.** Every byte goes through one Binary-Shift escape
   from the Upper-mode start state. The 5-mode optimiser is v0.2.0.
2. **GF(256) RS only.** 4-bit and 5-bit codeword paths (which enable
   maximum-density small symbols via GF(16)/GF(32) RS) are v0.2.0.
3. **Default ECC = 23 %** (the standard's recommended minimum).
4. **Auto symbol selection.** No `force_compact:` toggle yet.

## Running tests

```bash
bundle install
bundle exec standardrb --no-fix lib/
bundle exec rspec
```
