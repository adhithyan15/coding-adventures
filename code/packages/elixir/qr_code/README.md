# coding_adventures_qr_code

QR Code encoder — ISO/IEC 18004:2015 compliant Elixir implementation.

## What is a QR Code?

QR Code (Quick Response) was invented by Masahiro Hara at Denso Wave in 1994
to track automotive parts. It is now the most widely deployed 2D barcode on
earth, capable of encoding up to 7,089 characters in a single symbol.

A QR Code is a square matrix of dark and light modules. Decoders can find and
read the symbol from any orientation, at high speed, even when partially
damaged — thanks to the three finder patterns (corners), timing strips, and
Reed-Solomon error correction.

## Where this fits in the stack

```
input string
  → CodingAdventures.QrCode.encode/2   ← THIS PACKAGE
  → ModuleGrid                          (from coding_adventures_barcode_2d)
  → CodingAdventures.Barcode2D.layout/2
  → PaintScene                          (from coding_adventures_paint_instructions)
  → backend (SVG, ASCII, Metal, …)
```

## Quick Start

```elixir
{:ok, grid} = CodingAdventures.QrCode.encode("HELLO WORLD", :m)
# grid.rows == grid.cols == 21  (version 1 at ECC level M)
# grid.modules is a list-of-lists of booleans, true = dark module

# Pass to barcode_2d to get pixel coordinates:
{:ok, scene} = CodingAdventures.Barcode2D.layout(grid)
# scene.width == scene.height == 290.0  (21 + 2*4 quiet zone) * 10px
```

## Error Correction Levels

| Level | Recovery | Use case                           |
|-------|----------|------------------------------------|
| `:l`  | ~7%      | Maximum data density               |
| `:m`  | ~15%     | General-purpose (common default)   |
| `:q`  | ~25%     | Moderate noise or damage expected  |
| `:h`  | ~30%     | High damage risk, logo overlaid    |

## Encoding Modes

The encoder automatically selects the most compact mode:

| Mode         | Characters                         | Density    |
|--------------|------------------------------------|------------|
| Numeric      | `0`–`9`                            | ~3.3 b/ch  |
| Alphanumeric | `0`–`9`, `A`–`Z`, ` $%*+-./:` | ~5.5 b/ch  |
| Byte         | Any UTF-8 byte                     | 8.0 b/ch   |

## Versions

Version 1 is 21×21 modules. Each version step adds 4 modules per side.
Version 40 is 177×177. The encoder automatically picks the smallest version
that fits the input at the chosen ECC level.

## API Reference

### `encode/2`

```elixir
@spec encode(String.t(), :l | :m | :q | :h) ::
        {:ok, CodingAdventures.Barcode2D.ModuleGrid.t()} | {:error, atom()}
```

Encode a UTF-8 string into a QR Code. Returns `{:ok, grid}` on success or
`{:error, :input_too_long}` if the input exceeds version-40 capacity.

## Encoding Pipeline

```
input string
  → mode selection    (numeric / alphanumeric / byte)
  → version selection (smallest version that fits)
  → bit stream        (mode indicator + char count + data + padding)
  → blocks + RS ECC   (GF(256) b=0 convention, poly 0x11D)
  → interleave        (data CWs round-robin, then ECC CWs round-robin)
  → grid init         (finder, separator, timing, alignment, format, dark)
  → zigzag placement  (two-column snake from bottom-right)
  → mask evaluation   (8 patterns, lowest 4-rule penalty wins)
  → finalize          (format info + version info for v7+)
  → ModuleGrid        (abstract boolean grid, true = dark)
```

## Reed-Solomon ECC

QR Code uses the GF(256) Reed-Solomon b=0 convention (roots α⁰, α¹, …, αⁿ⁻¹),
distinct from the b=1 convention used by most general-purpose RS libraries.
This package implements the b=0 encoder in `CodingAdventures.QrCode.RS`,
separate from the general-purpose `coding_adventures_reed_solomon` package.

## Testing

```bash
mix test --cover
```

Target: >80% coverage. Actual: ~94%.

## License

Part of the coding-adventures monorepo.
