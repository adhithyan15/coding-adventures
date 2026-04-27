# CodingAdventures::PDF417

ISO/IEC 15438:2015 PDF417 stacked barcode encoder for Perl.

## What is PDF417?

PDF417 (Portable Data File 417) is a stacked linear barcode invented by Ynjiun
P. Wang at Symbol Technologies in 1991.  The name encodes its geometry: each
codeword has exactly **4 bars** and **4 spaces** (8 alternating elements) and
every codeword occupies exactly **17 modules** of horizontal space.

### Where PDF417 is deployed

| Application    | Detail                                                    |
|----------------|-----------------------------------------------------------|
| AAMVA          | North American driver's licences and government IDs       |
| IATA BCBP      | Airline boarding passes                                   |
| USPS           | Domestic shipping labels                                  |
| US immigration | Form I-94, customs declarations                           |
| Healthcare     | Patient wristbands, medication labels                     |

## How it fits into the stack

```
paint-vm-svg / paint-vm-canvas / paint-metal
        │
paint-instructions (P2D00)
        │
    barcode-2d   ←── pdf417
                         │
                     (GF(929) arithmetic embedded)
```

`pdf417` depends on `barcode-2d` for the `ModuleGrid` type.  GF(929) arithmetic
is implemented directly in this package (no separate gf929 Perl package yet).

## Usage

```perl
use CodingAdventures::PDF417 qw(encode encode_str);

# Encode raw bytes
my $grid = encode([72, 101, 108, 108, 111]);   # "Hello"

# Encode a string
my $grid = encode_str("HELLO WORLD");

# With options
my $grid = encode_str("HELLO WORLD", {
    ecc_level  => 4,   # Reed-Solomon ECC level 0–8 (default: auto)
    columns    => 5,   # data columns 1–30 (default: auto)
    row_height => 4,   # module rows per logical row (default: 3)
});

# ModuleGrid structure
# $grid->{rows}    — total pixel rows (logical_rows × row_height)
# $grid->{cols}    — total pixel cols (69 + 17 × data_columns)
# $grid->{modules} — 2D arrayref [row][col] of 1 (dark) or 0 (light)
```

## Encoding pipeline

```
raw bytes
  → byte compaction    [924, c1, c2, ...]
  → length descriptor  (1 + n_data + n_ecc, prepended as codeword 0)
  → auto ECC level     (based on data length)
  → RS ECC             (GF(929) Reed-Solomon, b=3, α=3)
  → dimension choice   (c = ceil(sqrt(total/3)), r = ceil(total/c))
  → padding            (codeword 900 fills unused slots)
  → row indicators     (LRI + RRI, encode R/C/ECC level)
  → cluster tables     (codeword → 17-module bar/space pattern)
  → start/stop         (fixed per row: 17 + 18 modules)
  → ModuleGrid         (abstract boolean grid)
```

## GF(929) — the prime field

PDF417 uses Reed-Solomon error correction over **GF(929)**, not GF(256).  Since
929 is prime, GF(929) is simply the integers modulo 929 — no irreducible
polynomial construction is needed.

The generator is α = 3 (primitive root mod 929), as specified in ISO/IEC
15438:2015 Annex A.4.  Log and antilog tables are precomputed once at module
load time for O(1) multiplication.

## Three codeword clusters

PDF417 cycles through three bar/space pattern tables (clusters 0, 3, 6) on a
row-by-row basis.  Row r uses cluster index (r % 3).  This allows a scanner to
verify it is reading a complete row rather than an arbitrary horizontal slice.

## v0.1.0 scope

This release implements **byte compaction only**.  All input is treated as raw
bytes regardless of content.  The symbol is valid and scannable but may be less
compact than a fully-optimised encoder for pure text or numeric data.

Text compaction and numeric compaction are planned for v0.2.0.

## Running tests

```bash
cpanm --notest Test2::V0
prove -I../paint-instructions/lib -I../barcode-2d/lib -l -v t/
```

## Version

0.1.0

## Author

Adhithya Rajasekaran <adhithyan15@gmail.com>

## License

MIT
