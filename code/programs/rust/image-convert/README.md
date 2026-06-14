# image-convert

Universal image format converter — the pandoc of image files.

Converts between PNG, BMP, PPM, QOI, JPEG, WebP, JPEG XL, GIF, ICO, TIFF,
and all major camera RAW formats (DNG, CR2, NEF, ARW, RAF, ORF, RW2) by
routing through the shared RGBA8 `PixelContainer` intermediate representation.

## Quick start

```bash
# Develop camera RAW files
image-convert photo.nef photo.png
image-convert photo.cr2 photo.tiff
image-convert photo.raf photo.jpg --quality 90

# Convert between standard formats
image-convert banner.gif banner.webp
image-convert icon.ico icon.png
image-convert logo.bmp logo.qoi

# Show all supported formats
image-convert --list-formats
```

## How it works

```
input file → detect format → decode → PixelContainer → encode → output file
                                       (RGBA8 pixels)
```

Every image codec in this monorepo implements the same `ImageCodec` trait and
decodes to a shared `PixelContainer` (RGBA8). The converter detects the input
format by magic bytes (not just the extension), decodes, and re-encodes.

## Supported formats

| Format | Extension(s) | Input | Output |
|---|---|---|---|
| PNG | .png | ✅ | ✅ |
| JPEG | .jpg .jpeg | ✅ | ✅ |
| BMP | .bmp | ✅ | ✅ |
| PPM/PGM | .ppm .pgm | ✅ | ✅ |
| QOI | .qoi | ✅ | ✅ |
| WebP | .webp | ✅ | ✅ |
| JPEG XL | .jxl | ✅ | ✅ |
| GIF | .gif | ✅ | ✅ |
| ICO/CUR | .ico .cur | ✅ | ✅ |
| TIFF | .tif .tiff | ✅ | ✅ |
| Adobe DNG | .dng | ✅ | ❌ RAW only |
| Canon CR2 | .cr2 | ✅ | ❌ RAW only |
| Nikon NEF | .nef | ✅ | ❌ RAW only |
| Sony ARW | .arw | ✅ | ❌ RAW only |
| Fujifilm RAF | .raf | ✅ | ❌ RAW only |
| Olympus ORF | .orf | ✅ | ❌ RAW only |
| Panasonic RW2 | .rw2 | ✅ | ❌ RAW only |

RAW formats are input-only — re-encoding sensor data to a proprietary camera
format is meaningless. Convert RAW → PNG/TIFF/JPEG instead.

## Usage

```
image-convert [OPTIONS] <INPUT> <OUTPUT>

Arguments:
  <INPUT>    Path to input image file
  <OUTPUT>   Path to output image file (extension sets output format)

Options:
  -q, --quality <N>     Encode quality 1–100 for lossy formats (default: 85)
  --from <FORMAT>       Force input format (e.g. jpg, nef, dng)
  --to <FORMAT>         Force output format (e.g. png, tiff)
  --list-formats        Print all supported formats and exit
  -h, --help            Print help
  -V, --version         Print version
```

## Format detection

Magic bytes are checked first — a `.tiff` file that is actually a DNG is
identified correctly. Extensions are used as fallback for TIFF-family formats
(DNG/NEF/ARW/ORF) that share the same TIFF magic.

## Alpha channel handling

The `PixelContainer` always carries RGBA8. Formats without alpha support (JPEG,
PPM, BMP) receive pixels composited over a solid white background:
`out = (alpha × fg + (255 − alpha) × 255) / 255`. Semi-transparent images will
appear with white halos in JPEG output.

## Building

```bash
cd code/programs/rust/image-convert
cargo build --release
./target/release/image-convert --help
```

## Testing

```bash
cargo test -p image-convert -- --nocapture
```

45 unit tests covering magic byte detection, extension detection, round-trip
encode/decode for PNG/BMP/TIFF/QOI/PPM/ICO, alpha compositing, and RAW
encode rejection.

## Spec

See [`code/specs/IC17-image-convert.md`](../../../../specs/IC17-image-convert.md).
