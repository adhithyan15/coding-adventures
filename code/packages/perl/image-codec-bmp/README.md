# CodingAdventures::ImageCodecBMP

BMP (Windows Bitmap) image encode/decode — IC01 in the coding-adventures image pipeline.

## What Is BMP?

BMP is one of the oldest and simplest raster image formats.  A BMP file is:

```
 Offset  Bytes  Section
      0     14  BITMAPFILEHEADER  — magic 'BM', file size, pixel data offset
     14     40  BITMAPINFOHEADER  — width, height, bit depth, compression
     54    ...  Pixel data        — rows in BGRA byte order
```

This implementation:
- **Encodes** as 32bpp top-down BMP (negative biHeight in the header)
- **Decodes** both top-down (negative height) and bottom-up (positive height)
- **Decodes** both 24bpp and 32bpp variants

## How It Fits in the Stack

```
IC00  PixelContainer  — raw RGBA8 pixel storage (dependency)
IC01  ImageCodecBMP   — BMP encode/decode        (THIS MODULE)
IC02  ImageCodecPPM   — PPM encode/decode
IC03  ImageCodecQOI   — QOI encode/decode
```

## Installation

```bash
# Install PixelContainer first
cpanm ../pixel-container/

# Install BMP codec
cpanm .
```

## Usage

```perl
use lib '../pixel-container/lib';
use CodingAdventures::PixelContainer;
use CodingAdventures::ImageCodecBMP qw(encode_bmp decode_bmp);

# Create a 4x4 red image and encode it
my $img   = CodingAdventures::PixelContainer->new(4, 4);
$img->fill_pixels(255, 0, 0, 255);
my $bytes = encode_bmp($img);

# Decode it back
my $img2          = decode_bmp($bytes);
my ($r, $g, $b, $a) = $img2->pixel_at(0, 0);
# ($r, $g, $b, $a) == (255, 0, 0, 255)
```

## Running Tests

```bash
PERL5LIB=../pixel-container/lib cpanm --installdeps --quiet .
PERL5LIB=../pixel-container/lib prove -l -v t/
```

## Format Notes

- Pixel channels are stored **BGRA** in BMP files (Blue first), not RGBA.
  The codec swaps R↔B automatically.
- 32bpp rows are already aligned to 4 bytes, so no padding logic is needed.
- 24bpp rows may need padding: `stride = (W*3 + 3) & ~3`.
