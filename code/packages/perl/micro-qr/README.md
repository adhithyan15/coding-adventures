# CodingAdventures::MicroQR

ISO/IEC 18004:2015 Annex E compliant Micro QR Code encoder in Perl.

Micro QR Code is the compact variant of QR Code, designed for applications where even
the smallest standard QR (21×21) is too large — circuit board markings, surface-mount
component labels, miniature industrial tags.

## Symbol sizes

| Symbol | Size    | Numeric cap | Alphanumeric cap | Byte cap | ECC levels |
|--------|---------|-------------|------------------|----------|------------|
| M1     | 11×11   | 5           | —                | —        | Detection  |
| M2     | 13×13   | 10          | 6                | 4        | L, M       |
| M3     | 15×15   | 23          | 14               | 9        | L, M       |
| M4     | 17×17   | 35          | 21               | 15       | L, M, Q    |

## Key differences from regular QR Code

- **Single finder pattern** — top-left only; no top-right or bottom-left.
- **Timing at row 0 / col 0** — not row 6 / col 6 as in regular QR.
- **Only 4 mask patterns** — (r+c)%2, r%2, c%3, (r+c)%3.
- **Format XOR 0x4445** — not 0x5412.
- **Single format info copy** — not two.
- **2-module quiet zone** — not 4.

## Installation

```bash
cpanm --notest CodingAdventures::MicroQR
```

Or with the repo build tool:

```bash
./build-tool code/packages/perl/micro-qr
```

## Usage

```perl
use CodingAdventures::MicroQR qw(encode encode_at layout_grid);
use CodingAdventures::MicroQR;  # for constants

# Auto-select smallest symbol that fits
my $grid = encode("HELLO");               # M2 13×13
my $grid = encode("12345");              # M1 11×11 (5 numeric)
my $grid = encode("https://example.com"); # M4 17×17 (byte mode)

# Force a specific version and ECC level
# IMPORTANT: constants must be called with () under use strict
my $grid = encode_at("HELLO", CodingAdventures::MicroQR::M2(), CodingAdventures::MicroQR::ECC_L());
my $grid = encode("1",
    CodingAdventures::MicroQR::M4(),
    CodingAdventures::MicroQR::ECC_Q());

# Access the module grid
printf "Size: %d×%d\n", $grid->{rows}, $grid->{cols};
for my $row (@{ $grid->{modules} }) {
    print join('', map { $_ ? '█' : ' ' } @$row), "\n";
}

# Render to a PaintScene (requires barcode-2d)
my $scene = layout_grid($grid);   # default quiet zone = 2
my $scene = layout_grid($grid, { quiet_zone_modules => 4 });
```

## Constants

```perl
# Version constants (integers 1-4)
CodingAdventures::MicroQR::M1()  # 1
CodingAdventures::MicroQR::M2()  # 2
CodingAdventures::MicroQR::M3()  # 3
CodingAdventures::MicroQR::M4()  # 4

# ECC level constants (strings)
CodingAdventures::MicroQR::DETECTION()  # 'D'
CodingAdventures::MicroQR::ECC_L()      # 'L'
CodingAdventures::MicroQR::ECC_M()      # 'M'
CodingAdventures::MicroQR::ECC_Q()      # 'Q'
```

## Encoding pipeline

```
input string
  → auto-select smallest (version, ECC) that fits and supports the mode
  → build bit stream: [mode indicator] [char count] [data] [terminator] [padding]
  → Reed-Solomon ECC: GF(256)/0x11D, b=0 convention, single block
  → init grid: finder (7×7), L-shaped separator, timing (row 0 / col 0)
  → reserve 15 format info positions
  → zigzag data placement (two-column snake from bottom-right)
  → evaluate all 4 mask patterns, select lowest penalty
  → write format information (15 bits, XOR 0x4445)
  → return ModuleGrid hashref
```

## Dependencies

- `CodingAdventures::GF256` — GF(2^8) field arithmetic for Reed-Solomon
- `CodingAdventures::Barcode2D` — ModuleGrid type and layout rendering

## Related packages

- `CodingAdventures::QrCode` — full QR Code encoder (regular sizes V1–V40)
- `code/packages/rust/micro-qr` — Rust reference implementation
