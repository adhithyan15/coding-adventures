# CodingAdventures::PixelContainer

In-memory RGBA8 pixel buffer — IC00 in the coding-adventures image pipeline.

## What Is a Pixel Container?

A pixel container is the simplest possible representation of a raster image:
a flat array of colour values, one element per pixel.  Each pixel is four bytes
in RGBA order:

| Byte | Channel | Range   | Meaning                              |
|------|---------|---------|--------------------------------------|
| 0    | Red     | 0..255  | red intensity                        |
| 1    | Green   | 0..255  | green intensity                      |
| 2    | Blue    | 0..255  | blue intensity                       |
| 3    | Alpha   | 0..255  | opacity (0 = transparent, 255 = opaque) |

Pixel (x, y) lives at byte offset `(y * width + x) * 4` in the flat buffer.

## How It Fits in the Stack

```
IC00  PixelContainer  — raw RGBA8 pixel storage        (THIS MODULE)
IC01  ImageCodecBMP   — BMP encode/decode
IC02  ImageCodecPPM   — PPM encode/decode
IC03  ImageCodecQOI   — QOI encode/decode
```

Also included: `CodingAdventures::ImageCodec` — the interface documentation
module that all codec packages implement.

## Installation

```bash
cpanm .
```

## Usage

```perl
use CodingAdventures::PixelContainer;

# Create a 320×240 blank (transparent black) image
my $img = CodingAdventures::PixelContainer->new(320, 240);

# Draw a red pixel at (10, 20)
$img->set_pixel(10, 20, 255, 0, 0, 255);

# Read it back
my ($r, $g, $b, $a) = $img->pixel_at(10, 20);
# ($r, $g, $b, $a) == (255, 0, 0, 255)

# Fill the entire image with solid white
$img->fill_pixels(255, 255, 255, 255);

# Access the raw byte buffer (e.g. for a codec to encode)
my $raw_ref = $img->data;   # scalar ref to the byte string
my $bytes   = $$raw_ref;    # copy of the raw bytes
```

## API

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `($width, $height)` | Allocate blank W×H image; dies on invalid dims |
| `width` | `()` | Return pixel width |
| `height` | `()` | Return pixel height |
| `data` | `()` | Return scalar ref to internal byte buffer |
| `pixel_at` | `($x, $y)` | Read `($r,$g,$b,$a)`; returns `(0,0,0,0)` OOB |
| `set_pixel` | `($x,$y,$r,$g,$b,$a)` | Write pixel; no-op OOB |
| `fill_pixels` | `($r,$g,$b,$a)` | Fill whole image with one colour |

## Running Tests

```bash
cpanm --installdeps --quiet .
prove -l -v t/
```

## Design Notes

- **Byte buffer**: Perl strings are used as raw byte arrays — `"\x00" x N`
  allocates N zero bytes, `pack`/`unpack` serialise integers, and `substr`
  splices bytes in-place.
- **Out-of-bounds**: reads return `(0,0,0,0)`, writes are silent no-ops.
  This matches the HTML Canvas API convention.
- **fill_pixels efficiency**: uses the Perl `x` repetition operator rather
  than a Perl-level loop, so the copy is done in C inside the interpreter.
