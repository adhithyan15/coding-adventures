# coding_adventures_data_matrix

**Data Matrix ECC200 encoder for Elixir** — ISO/IEC 16022:2006 compliant.

Part of the `coding-adventures` monorepo. This package is the Elixir port of
the reference Rust implementation at `code/packages/rust/data-matrix/`.

## What is Data Matrix?

Data Matrix is a two-dimensional matrix barcode standardised as ISO/IEC
16022:2006. It is used everywhere a small, dense, damage-tolerant mark is
needed:

- **PCB traceability** — every board carries an etched or printed Data Matrix
- **Pharmaceuticals** — US FDA DSCSA mandates unit-dose Data Matrix
- **Aerospace** — rivets, shims, brackets marked by dot-peen or laser on metal
- **Medical devices** — surgical instruments, implants (GS1 DataMatrix)

## Key features

| Property | Data Matrix | QR Code (contrast) |
|----------|-------------|---------------------|
| GF polynomial | **0x12D** | 0x11D |
| RS root convention | **b=1** (α^1..α^n) | b=0 (α^0..α^{n-1}) |
| Finder pattern | **L-shaped + clock border** | Three 7×7 finder squares |
| Data placement | **Utah diagonal zigzag** | Two-column zigzag |
| Masking | **None** | 8 patterns evaluated |

## Encoding pipeline

```
input string
  → ASCII encoding   (chars+1; consecutive digit pairs packed into one codeword)
  → symbol selection (smallest size whose capacity ≥ codeword count)
  → pad to capacity  (scrambled-pad codewords fill unused slots)
  → RS blocks + ECC  (GF(256)/0x12D, b=1 convention, blocks interleaved)
  → grid init        (L-finder + timing + alignment borders)
  → Utah placement   (diagonal codeword placement, no masking)
  → ModuleGrid
```

## Symbol sizes

| Size | Data codewords | Max ASCII chars |
|------|---------------|-----------------|
| 10×10 | 3 | 1 |
| 12×12 | 5 | 3 |
| 14×14 | 8 | 6 |
| 16×16 | 12 | 10 |
| 18×18 | 18 | 16 |
| 20×20 | 22 | 20 |
| ... | ... | ... |
| 144×144 | 1558 | 1556 |

Plus 6 rectangular sizes (8×18 to 16×48).

## Usage

```elixir
# Encode a string (returns {:ok, grid} or {:error, reason})
{:ok, grid} = CodingAdventures.DataMatrix.encode("Hello World")
IO.inspect(grid.rows)   # 16
IO.inspect(grid.cols)   # 16

# Each module is true (dark) or false (light)
grid.modules |> Enum.each(fn row ->
  row |> Enum.each(fn dark -> IO.write(if dark, do: "█", else: " ") end)
  IO.puts("")
end)

# Encode and raise on error
grid = CodingAdventures.DataMatrix.encode!("A")

# ASCII art debug output
IO.puts(CodingAdventures.DataMatrix.render_ascii("Test123"))

# Rectangular symbols
{:ok, grid} = CodingAdventures.DataMatrix.encode("Hi", %{shape: :rectangular})

# Digit-pair optimization (automatic)
# "12345678" → 4 codewords instead of 8
assert length(CodingAdventures.DataMatrix.encode_ascii("12345678")) == 4
```

## Installation

This package lives in the coding-adventures monorepo and is not published to
Hex.pm. Add it as a local path dependency in your `mix.exs`:

```elixir
def deps do
  [
    {:coding_adventures_data_matrix, path: "../data_matrix"}
  ]
end
```

## Testing

```bash
mix deps.get
mix test --cover
```

Expected: 60+ tests, all passing, coverage > 80%.

## Dependencies

- `coding_adventures_gf256` — GF(256) field arithmetic (local path)
- `coding_adventures_barcode_2d` — `ModuleGrid` type and `layout()` (local path)

## Architecture

The Data Matrix stack in this repo:

```
data-matrix (this package)
    ↓ depends on
barcode_2d          ← ModuleGrid type, layout()
gf256               ← GF(256)/0x12D field arithmetic
```

## Specification

See `code/specs/data-matrix.md` for the full encoder specification, worked
examples, and cross-language test vectors.
