# CodingAdventures::ImageCodecQOI

QOI (Quite OK Image) lossless encode/decode — IC03 in the coding-adventures image pipeline.

## What Is QOI?

QOI was designed in 2021 by Dominic Szablewski.  The goal: a lossless image
codec as simple to implement as BMP but as compact as PNG.

QOI achieves compression through six chunk types:

| Chunk          | Tag        | Size     | When used |
|----------------|------------|----------|-----------|
| `QOI_OP_RUN`   | `11xxxxxx` | 1 byte   | Run of 1–62 identical pixels |
| `QOI_OP_INDEX` | `00xxxxxx` | 1 byte   | Pixel seen recently (hash table) |
| `QOI_OP_DIFF`  | `01xxxxxx` | 1 byte   | Tiny deltas: dr, dg, db in −2..1 |
| `QOI_OP_LUMA`  | `10xxxxxx` | 2 bytes  | Medium deltas: dg in −32..31 |
| `QOI_OP_RGB`   | `0xFE`     | 4 bytes  | Full RGB, keep alpha |
| `QOI_OP_RGBA`  | `0xFF`     | 5 bytes  | Full RGBA |

## How It Fits in the Stack

```
IC00  PixelContainer  — raw RGBA8 pixel storage (dependency)
IC01  ImageCodecBMP   — BMP encode/decode
IC02  ImageCodecPPM   — PPM encode/decode
IC03  ImageCodecQOI   — QOI encode/decode       (THIS MODULE)
```

## Installation

```bash
cpanm ../pixel-container/
cpanm .
```

## Usage

```perl
use lib '../pixel-container/lib';
use CodingAdventures::PixelContainer;
use CodingAdventures::ImageCodecQOI qw(encode_qoi decode_qoi);

my $img   = CodingAdventures::PixelContainer->new(256, 256);
$img->fill_pixels(100, 150, 200, 255);
my $bytes = encode_qoi($img);

my $img2 = decode_qoi($bytes);
my ($r, $g, $b, $a) = $img2->pixel_at(0, 0);
# ($r, $g, $b, $a) == (100, 150, 200, 255)
```

## Running Tests

```bash
PERL5LIB=../pixel-container/lib cpanm --installdeps --quiet .
PERL5LIB=../pixel-container/lib prove -l -v t/
```

## Key Formulas

**Seen-pixels hash:**
```
index = (R*3 + G*5 + B*7 + A*11) % 64
```

**Signed delta (wrap-around modular arithmetic):**
```
delta = (new - prev + 256) % 256
if delta > 127: delta -= 256
```

**QOI_OP_DIFF bias:** `stored = delta + 2` (range 0..3 in 2 bits)

**QOI_OP_LUMA bias:** dg+32 (6 bits), dr_rel+8 (4 bits), db_rel+8 (4 bits)
where `dr_rel = dr - dg` and `db_rel = db - dg`.
