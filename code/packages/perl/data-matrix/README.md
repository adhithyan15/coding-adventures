# CodingAdventures::DataMatrix

ISO/IEC 16022:2006 Data Matrix ECC200 encoder in Perl.

Data Matrix ECC200 was standardised by RVSI Acuity CiMatrix in the mid-1990s
and is now ubiquitous wherever small, damage-tolerant marks are required on
physical objects. Unlike ink-printed labels, a Data Matrix can be etched
directly into metal — dot-peened onto a PCB, laser-etched onto a surgical
implant, or chemically etched onto an aircraft rivet — and still be decoded
decades later.

## Where Data Matrix is used

- **Printed circuit boards** — every PCB carries a Data Matrix traceability mark.
- **Pharmaceuticals** — US FDA DSCSA mandate on unit-dose packaging.
- **Aerospace parts** — rivets, shims, brackets etched into metal.
- **Surgical instruments** — GS1 DataMatrix on implantables and sterile tools.

## Key differences from QR Code

| Property          | Data Matrix ECC200 | QR Code        |
|-------------------|--------------------|----------------|
| GF polynomial     | 0x12D              | 0x11D          |
| RS root convention| b=1 (α^1..α^n)     | b=0 (α^0..α^{n-1}) |
| Finder pattern    | L-shaped (2 sides) | 3 finder squares |
| Placement         | Utah diagonal      | Two-column zigzag |
| Masking           | None               | 8 mask patterns |

## Symbol sizes

**Square (24 sizes):** 10×10 through 144×144

**Rectangular (6 sizes):** 8×18, 8×32, 12×26, 12×36, 16×36, 16×48

## Installation

```bash
cpanm --notest CodingAdventures::DataMatrix
```

Or via the repo build tool:

```bash
./build-tool code/packages/perl/data-matrix
```

## Usage

```perl
use CodingAdventures::DataMatrix qw(encode_data_matrix);

# Auto-select smallest square symbol.
my $grid = encode_data_matrix("Hello, World!");

# Print as terminal art.
for my $row (@{ $grid->{modules} }) {
    print join('', map { $_ ? '##' : '  ' } @$row), "\n";
}

# Encode raw bytes (e.g. a UTF-8 sequence).
use Encode qw(encode_utf8);
my $g = encode_data_matrix( encode_utf8("こんにちは") );

# Request a rectangular symbol.
my $rect = encode_data_matrix("LOT:1234", { shape => 'rectangular' });

# Try both shapes and pick the smaller.
my $small = encode_data_matrix("ABC", { shape => 'any' });
```

The returned hashref:

```perl
{
    rows         => 10,         # symbol height in modules
    cols         => 10,         # symbol width in modules
    modules      => \@aoa,      # 2D arrayref of 0/1 (1 = dark)
    module_shape => 'square',
}
```

Compatible with `CodingAdventures::Barcode2D::layout()` for pixel rendering.

## Encoding pipeline

```
input bytes/string
  -> ASCII encoding       (char+1; digit pairs packed into one codeword)
  -> symbol size selection (smallest symbol whose dataCW >= codeword count)
  -> pad to capacity      (scrambled-pad codewords fill unused slots)
  -> RS ECC per block     (GF(256)/0x12D, b=1 convention, LFSR division)
  -> interleave blocks    (data round-robin, then ECC round-robin)
  -> grid initialization  (L-finder + timing + alignment borders)
  -> Utah diagonal placement (no masking)
  -> ModuleGrid hashref
```

## v0.1.0 notes

- ASCII mode only. C40/Text/X12/EDIFACT/Base256 encoding is v0.2.0.
- Square symbol selection by default. Pass `{ shape => 'rectangular' }` or
  `{ shape => 'any' }` for rectangular support.
- GF(256)/0x12D arithmetic and RS generator polynomials are implemented
  privately. They use the same primitive polynomial as the Aztec Code package
  in this repo (both formats use 0x12D, not QR's 0x11D).

## Dependencies

- `CodingAdventures::Barcode2D` — `ModuleGrid` shape constant.
- `Carp` — error reporting (`croak`).
- `POSIX` — `floor` function.

## Related packages

- `code/packages/typescript/data-matrix/` — TypeScript reference (algorithm source of truth).
- `code/packages/go/data-matrix/` — Go implementation (very literate, great reference).
- `code/packages/perl/aztec-code/` — sibling package using the same GF(256)/0x12D field.
- `code/packages/perl/barcode-2d/` — shared 2D barcode abstraction layer.
