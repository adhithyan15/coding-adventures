# coding_adventures_micro_qr

Elixir implementation of the Micro QR Code encoder — ISO/IEC 18004:2015 Annex E compliant.

## What it does

Encodes arbitrary strings into Micro QR Code symbols. Micro QR is the compact
variant of standard QR Code, designed for applications where even the smallest
standard QR (21×21) is too large, such as surface-mount component labels,
circuit board markings, and miniature industrial tags.

## Symbol sizes

```
M1: 11×11   M2: 13×13   M3: 15×15   M4: 17×17
formula: size = 2 × version_number + 9
```

## Key differences from regular QR Code

- **Single finder pattern** at top-left only (one 7×7 square, not three).
- **Timing at row 0 / col 0** (not row 6 / col 6).
- **Only 4 mask patterns** (not 8).
- **Format XOR mask 0x4445** (not 0x5412).
- **Single copy of format info** (not two).
- **2-module quiet zone** (not 4).
- **Narrower mode indicators** (0–3 bits instead of 4).
- **Single block** RS error correction (no interleaving).

## Encoding pipeline

```
input string
  → auto-select smallest symbol (M1..M4) and mode
  → build bit stream (mode indicator + char count + data + terminator + padding)
  → Reed-Solomon ECC (GF(256)/0x11D, b=0, single block)
  → initialize grid (finder, L-shaped separator, timing at row0/col0, format reserved)
  → zigzag data placement (two-column snake from bottom-right)
  → evaluate 4 mask patterns, pick lowest penalty
  → write format information (15 bits, single copy, XOR 0x4445)
  → ModuleGrid
```

## Where it fits in the pipeline

```
Input data
  → CodingAdventures.MicroQR.encode/3    ← THIS PACKAGE
  → ModuleGrid
  → CodingAdventures.Barcode2D.layout/2  ← barcode_2d package
  → PaintScene
  → backend (SVG, Metal, Canvas, terminal…)
```

## Usage

```elixir
alias CodingAdventures.MicroQR

# Auto-select smallest symbol:
{:ok, grid} = MicroQR.encode("1")
# grid.rows == 11, grid.cols == 11  (M1)

{:ok, grid} = MicroQR.encode("HELLO")
# grid.rows == 13, grid.cols == 13  (M2-L alphanumeric)

{:ok, grid} = MicroQR.encode("https://a.b")
# grid.rows == 17, grid.cols == 17  (M4-L byte)

# Force a specific version and ECC level:
{:ok, grid} = MicroQR.encode("HELLO", :m4, :q)
# grid.rows == 17  (M4-Q)

# Bang variant raises on error:
grid = MicroQR.encode!("12345")

# Encode and render in one step:
{:ok, scene} = MicroQR.encode_and_layout("HELLO")
# scene.width / scene.height are in pixels
```

## ECC levels

| Level       | Available in  | Recovery             |
|-------------|---------------|----------------------|
| `:detection`| M1 only       | Detects errors only  |
| `:l`        | M2, M3, M4    | ~7% of codewords     |
| `:m`        | M2, M3, M4    | ~15% of codewords    |
| `:q`        | M4 only       | ~25% of codewords    |

Level H is not available in any Micro QR symbol.

## Data capacities

| Symbol | Mode         | Max chars |
|--------|--------------|-----------|
| M1     | Numeric      | 5         |
| M2-L   | Numeric      | 10        |
| M2-L   | Alphanumeric | 6         |
| M2-L   | Byte         | 4         |
| M3-L   | Numeric      | 23        |
| M3-L   | Alphanumeric | 14        |
| M3-L   | Byte         | 9         |
| M4-L   | Numeric      | 35        |
| M4-L   | Alphanumeric | 21        |
| M4-L   | Byte         | 15        |
| M4-Q   | Numeric      | 21        |
| M4-Q   | Alphanumeric | 13        |
| M4-Q   | Byte         | 9         |

## Error handling

All public functions return `{:ok, result}` or `{:error, reason}`.

Common error reasons:
- `"InputTooLong: ..."` — input exceeds the capacity of all symbols.
- `"ECCNotAvailable: ..."` — the requested version/ECC combination does not exist.
- `"UnsupportedMode: ..."` — the input cannot be encoded in any mode supported by the chosen symbol.

## Running tests

```bash
cd code/packages/elixir/micro_qr
mix deps.get
mix test --cover
```

Expected: 58 tests, 0 failures, ~97% coverage.

## Dependencies

- [`coding_adventures_gf256`](../gf256) — GF(256) arithmetic for Reed-Solomon ECC.
- [`coding_adventures_barcode_2d`](../barcode_2d) — `ModuleGrid` type and `layout/2` renderer.
