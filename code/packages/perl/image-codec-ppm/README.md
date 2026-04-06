# CodingAdventures::ImageCodecPPM

PPM (Portable Pixmap) image encode/decode — IC02 in the coding-adventures image pipeline.

## What Is PPM?

PPM is the simplest colour image format in the Netpbm family.  Its entire
specification fits on one page:

```
P6\n
[optional # comment lines]\n
<width> <height>\n
<maxval>\n
<binary RGB pixel data, W*H*3 bytes>
```

This implementation supports the P6 (binary) variant with maxval ≤ 255
(one byte per channel).

## How It Fits in the Stack

```
IC00  PixelContainer  — raw RGBA8 pixel storage (dependency)
IC01  ImageCodecBMP   — BMP encode/decode
IC02  ImageCodecPPM   — PPM encode/decode       (THIS MODULE)
IC03  ImageCodecQOI   — QOI encode/decode
```

## Alpha Channel

PPM has no alpha channel.  **Encoding** silently drops the alpha byte.
**Decoding** sets alpha = 255 (fully opaque) for every pixel.

## Installation

```bash
cpanm ../pixel-container/
cpanm .
```

## Usage

```perl
use lib '../pixel-container/lib';
use CodingAdventures::PixelContainer;
use CodingAdventures::ImageCodecPPM qw(encode_ppm decode_ppm);

my $img = CodingAdventures::PixelContainer->new(4, 4);
$img->fill_pixels(0, 128, 255, 255);
my $bytes = encode_ppm($img);

my $img2 = decode_ppm($bytes);
my ($r, $g, $b, $a) = $img2->pixel_at(0, 0);
# ($r, $g, $b, $a) == (0, 128, 255, 255)
```

## Running Tests

```bash
PERL5LIB=../pixel-container/lib cpanm --installdeps --quiet .
PERL5LIB=../pixel-container/lib prove -l -v t/
```

## Format Details

- No padding between rows — each row is exactly `W * 3` bytes.
- Pixels are stored top-down (same order as display).
- Comment lines (starting with `#`) may appear anywhere in the header.
- Only P6 (binary) is supported, not P3 (ASCII text variant).
