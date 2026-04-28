# CodingAdventures::AztecCode

ISO/IEC 24778:2008 Aztec Code encoder in Perl.

Aztec Code (Andrew Longacre Jr., Welch Allyn, 1995) is a patent-free 2D
barcode that places a single bullseye finder pattern at the center of the
symbol. Scanners locate the bullseye and then read outward in a spiral,
which lets the symbol work without the large quiet zone QR Code requires.

## Where Aztec is used

- **IATA boarding passes** — every airline boarding pass.
- **Eurostar and Amtrak rail tickets**.
- **PostNL, Deutsche Post, La Poste** — European postal routing.
- **US military ID cards**.

## Symbol variants

| Variant | Layers | Size formula        | Example |
|---------|--------|---------------------|---------|
| Compact | 1–4    | `11 + 4 * layers`   | 15×15 .. 27×27 |
| Full    | 1–32   | `15 + 4 * layers`   | 19×19 .. 143×143 |

The encoder picks the smallest variant that fits at the requested ECC level
(compact preferred, then full).

## Installation

```bash
cpanm --notest CodingAdventures::AztecCode
```

Or with the repo build tool:

```bash
./build-tool code/packages/perl/aztec-code
```

## Usage

```perl
use CodingAdventures::AztecCode qw(encode);

# Auto-select smallest symbol at default 23% ECC.
my $grid = encode("Hello, World!");

# Print as terminal art.
for my $row (@{ $grid->{modules} }) {
    print join('', map { $_ ? '##' : '  ' } @$row), "\n";
}

# Force a higher ECC level (range 10..90).
my $strong = encode("Hello", { min_ecc_percent => 50 });

# Encode raw bytes (e.g. a UTF-8 sequence).
use Encode qw(encode_utf8);
my $g = encode( encode_utf8("こんにちは") );
```

The returned hashref has the shape:

```perl
{
    rows         => $size,        # e.g. 15 for 'A'
    cols         => $size,
    modules      => \@aoa,        # 2D array of 0/1 (1 = dark)
    module_shape => 'square',
}
```

It is compatible with `CodingAdventures::Barcode2D` so you can call
`CodingAdventures::Barcode2D->layout($grid, \%cfg)` to produce a `PaintScene`.

## Encoding pipeline

```
input bytes
  -> Binary-Shift escape from Upper mode (5-bit escape + 5/16-bit length + bytes)
  -> select smallest symbol (compact 1..4 then full 1..32) at min_ecc_percent
  -> pad to exact data codeword count (last 0x00 -> 0xFF)
  -> GF(256)/0x12D Reed-Solomon ECC (poly 0x12D, roots alpha^1..alpha^n)
  -> bit stuffing (insert complement after 4 identical bits)
  -> GF(16) mode message (layers + data-cw count + RS over alpha)
  -> draw bullseye + reference grid (full only) + orientation ring + mode bits
  -> place data bits in clockwise 2-wide spiral bands
  -> ModuleGrid hashref
```

## v0.1.0 simplifications

- Byte-mode only — all input is encoded via the Binary-Shift escape from
  Upper mode. Multi-mode (Digit/Upper/Lower/Mixed/Punct) optimization is
  planned for v0.2.0.
- 8-bit data codewords only — GF(16)/GF(32) data codewords are v0.2.0.
- Default ECC is 23%.
- Auto compact/full selection only — `force_compact` flag is v0.2.0.

## Dependencies

- `CodingAdventures::Barcode2D` — `ModuleGrid` shape constant and rendering.
- `Carp` — error reporting.
- `POSIX` — `ceil` for symbol-fit math.

GF(256)/0x12D arithmetic is implemented privately in this module because the
repo's `CodingAdventures::GF256` package uses the QR Code polynomial 0x11D.

## Related packages

- `code/packages/typescript/aztec-code/` — TypeScript reference implementation.
- `code/packages/perl/micro-qr/` — sibling Micro QR encoder using the same
  `Barcode2D` substrate.
- `code/packages/perl/qr-code/` — full QR Code encoder.
