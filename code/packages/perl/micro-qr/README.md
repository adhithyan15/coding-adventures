# CodingAdventures::MicroQR

Perl implementation of the **Micro QR Code** encoder, compliant with
ISO/IEC 18004:2015 Annex E.

Micro QR Code is the compact variant of QR Code, designed for applications
where the smallest standard QR symbol (21×21) is still too large. Common use
cases include surface-mount component labels, circuit board markings, and
miniature industrial tags.

## Symbol sizes

| Symbol | Size    | Max numeric | Max alphanumeric | Max bytes |
|--------|---------|-------------|------------------|-----------|
| M1     | 11×11   | 5           | —                | —         |
| M2-L   | 13×13   | 10          | 6                | 4         |
| M2-M   | 13×13   | 8           | 5                | 3         |
| M3-L   | 15×15   | 23          | 14               | 9         |
| M3-M   | 15×15   | 18          | 11               | 7         |
| M4-L   | 17×17   | 35          | 21               | 15        |
| M4-M   | 17×17   | 30          | 18               | 13        |
| M4-Q   | 17×17   | 21          | 13               | 9         |

## Key differences from regular QR Code

- **Single finder pattern** at top-left only (not three corner squares)
- **Timing at row 0 / col 0** (not row 6 / col 6)
- **Only 4 mask patterns** (not 8)
- **Format XOR mask 0x4445** (not 0x5412)
- **Single copy of format info** (not two)
- **2-module quiet zone** (not 4)
- **Variable-width mode indicator**: 0 bits (M1), 1 bit (M2), 2 bits (M3), 3 bits (M4)
- **No interleaving** — single RS block

## Installation

```sh
cpanm .
```

## Usage

```perl
use CodingAdventures::MicroQR qw(encode encode_at layout_grid M1 M2 M3 M4
                                   ECC_L ECC_M ECC_Q DETECTION);

# Auto-select smallest symbol and ECC
my $grid = encode('12345');       # M1, 11×11
my $grid = encode('HELLO');       # M2-L, 13×13, alphanumeric
my $grid = encode('https://a.b'); # M4-L, 17×17, byte mode

# Force version and/or ECC
my $grid = encode('1', M4, ECC_Q);    # M4-Q, 17×17 (extra error correction)
my $grid = encode_at('1', M1, DETECTION);  # explicit version required

# Convert to PaintScene for rendering (uses Barcode2D::layout)
my $scene = layout_grid($grid);
my $scene = layout_grid($grid, { module_size_px => 20, quiet_zone_modules => 2 });
```

The `encode` function returns a `ModuleGrid` hashref compatible with
`CodingAdventures::Barcode2D`:

```perl
{
    rows         => 11,       # or 13, 15, 17
    cols         => 11,
    modules      => \@grid,   # 2D array: 1 = dark, 0 = light
    module_shape => 'square',
}
```

## Encoding pipeline

```
input string
  → select smallest (version, ECC) that fits
  → mode: numeric | alphanumeric | byte
  → bit stream: [mode indicator] [char count] [data] [terminator] [pad]
  → Reed-Solomon ECC (GF(256)/0x11D, b=0, single block)
  → flatten to bit array
  → build grid (finder 7×7, L-separator, timing row0/col0, format reserved)
  → zigzag data placement (two-column snake, bottom-right → top-left)
  → evaluate 4 mask patterns — pick lowest penalty
  → write format information (15 bits, XOR 0x4445, single copy)
  → return ModuleGrid
```

## Dependencies

| Package                        | Role                              |
|--------------------------------|-----------------------------------|
| `CodingAdventures::GF256`      | GF(256) multiply for Reed-Solomon |
| `CodingAdventures::Barcode2D`  | ModuleGrid type and layout()      |

## In this stack

- Sits above `GF256` (field arithmetic) and `Barcode2D` (grid + renderer)
- Parallel implementations: Rust (`micro-qr`), TypeScript, Python, Ruby,
  Go, Elixir, Swift, Lua, Perl (this package)

## Tests

```sh
cpanm --notest Test2::V0
prove -I../gf256/lib -I../barcode-2d/lib -I../paint-instructions/lib -l -v t/
```

58 tests covering symbol dimensions, auto-selection, structural patterns,
format information, ECC constraints, encoding modes, capacity boundaries,
error conditions, determinism, and cross-language corpus verification.

## License

MIT
