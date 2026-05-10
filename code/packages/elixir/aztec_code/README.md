# coding_adventures_aztec_code

Aztec Code encoder for Elixir — ISO/IEC 24778:2008 compliant.

Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
published as a patent-free format. Unlike QR Code (which needs three square
finder patterns scattered at the corners), Aztec Code uses a single
**bullseye finder pattern at the center** of the symbol. The scanner finds
the center first, then reads the data outward in a clockwise spiral — no
large quiet zone is required.

## Where Aztec Code is used today

- **IATA boarding passes** — the barcode on every airline boarding pass
- **Eurostar and Amtrak rail tickets** — printed and on-screen
- **PostNL, Deutsche Post, La Poste** — European postal routing
- **US driver's licences** (some states), **US military ID cards**

## Symbol variants

```
Compact: 1–4 layers,  size = 11 + 4 × layers  (15×15 to 27×27)
Full:    1–32 layers, size = 15 + 4 × layers  (19×19 to 143×143)
```

The encoder automatically selects the smallest variant that fits the data.

## Usage

```elixir
# Encode a string — returns {:ok, grid} or {:error, :input_too_long}
{:ok, grid} = CodingAdventures.AztecCode.encode("Hello, World!")

grid.rows    # => 19 (or larger, depending on input length)
grid.cols    # => 19
grid.modules # => [[true, false, true, ...], ...]  (true = dark, false = light)

# Raise on error
grid = CodingAdventures.AztecCode.encode!("https://example.com")

# Debug ASCII art
IO.puts CodingAdventures.AztecCode.render_ascii("A")

# Custom ECC percentage (default 23%)
{:ok, grid} = CodingAdventures.AztecCode.encode("Hi", %{min_ecc_percent: 40})
```

## How it fits in the stack

```
paint-vm-svg  paint-vm-canvas  paint-metal
       └──────────┬───────────────┘
              paint-vm
                 │
          paint-instructions          MA02 reed-solomon
                 │                         │
             barcode-2d ←── aztec_code ────┘
                                │
                             MA01 gf256 (not a dep — GF(16) and GF(256)/0x12D
                                         are implemented inline)
```

`aztec_code` is a self-contained encoder that produces a raw
`%{rows, cols, modules}` module grid. Pass it to `barcode_2d` to get a
`PaintScene`, then to a `paint-vm-*` backend for SVG/Canvas/Metal output.

## Encoding pipeline (v0.1.0 — byte-mode only)

```
input string / bytes
  → Binary-Shift from Upper mode  (5-bit escape + length + raw bytes)
  → symbol size selection          (smallest compact/full at 23% ECC)
  → pad to codeword count          (zero-fill; last 0x00 → 0xFF)
  → GF(256)/0x12D RS ECC           (Data Matrix polynomial, b=1 roots)
  → bit stuffing                   (complement after every 4 identical bits)
  → GF(16) mode message            (layers + cw-count + 5 or 6 RS nibbles)
  → grid init                      (bullseye → orientation → mode ring)
  → clockwise data spiral          (innermost layer to outermost)
  → ModuleGrid
```

## GF arithmetic

Two independent GF implementations are included:

| Field | Polynomial | Used for |
|-------|-----------|---------|
| GF(16)  | `x^4 + x + 1 = 0x13` | Mode message RS (both compact and full) |
| GF(256) | `x^8 + x^5 + x^4 + x^2 + x + 1 = 0x12D` | Data codeword RS |

The GF(256) polynomial is the same as Data Matrix ECC200, different from
QR Code (which uses `0x11D`). Both are valid GF(256) fields.

## Adding to your project

```elixir
# mix.exs
{:coding_adventures_aztec_code, path: "path/to/aztec_code"}
```

## Running tests

```bash
mix deps.get
mix test --cover
```

Coverage: **96.67%** (112 tests).

## v0.2.0 roadmap

- Multi-mode encoding (Digit/Upper/Lower/Mixed/Punct/Binary segments) for
  minimum codeword count — URLs typically compress 20–30% more than byte-only.
- GF(32) RS for 5-bit codeword sequences.
- Forced compact/full mode option.
- `explain/2` API with per-module role annotations for the interactive
  visualizer.
