# image-codec-tiff

TIFF (Tagged Image File Format) image codec — IC09. Foundation for all camera
RAW format decoders: Canon CR2, Nikon NEF, Sony ARW, Olympus ORF, and Adobe
DNG are all TIFF container files.

## Where this fits in the stack

```
paint-instructions  ←  ImageCodec trait
      │
image-codec-tiff    ←  YOU ARE HERE
      │
pixel-container     ←  PixelContainer (RGBA8 pixel buffer)
      │
image-codec-dng / image-codec-cr2 / image-codec-nef / ...
      └─ depend on image-codec-tiff for IFD parsing + Bayer demosaicing
```

## Quick start

```rust
use image_codec_tiff::{encode_tiff, decode_tiff};
use pixel_container::PixelContainer;

// Encode a 4×4 red image as TIFF.
let mut px = PixelContainer::new(4, 4);
px.fill(255, 0, 0, 255);
let bytes = encode_tiff(&px);

// Decode it back.
let recovered = decode_tiff(&bytes).unwrap();
assert_eq!(recovered.width, 4);
assert_eq!(recovered.pixel_at(0, 0), (255, 0, 0, 255));

// Use the ImageCodec trait.
use paint_instructions::ImageCodec;
use image_codec_tiff::TiffCodec;
assert_eq!(TiffCodec.mime_type(), "image/tiff");
```

## File format overview

```
TIFF header (8 bytes):
  Offset  Size  Field
  0       2     Byte order: "II" (LE) or "MM" (BE)
  2       2     Magic: 42
  4       4     Offset of first IFD

IFD (linked list of image directories):
  [2 bytes] entry count
  [12 bytes × count] tag entries
  [4 bytes] offset of next IFD (0 = end)

Each IFD entry (12 bytes):
  [2] tag  [2] type  [4] count  [4] inline value or file offset
```

## Supported features

| Feature                  | Status      |
|--------------------------|-------------|
| Little-endian (II) TIFF  | ✅           |
| Big-endian (MM) TIFF     | ✅           |
| Uncompressed (1)         | ✅           |
| PackBits (32773)         | ✅           |
| LZW (5)                  | ✅           |
| JPEG strips (7)          | ❌ (Err)    |
| RGB (PhotometricInterp 2)| ✅           |
| BlackIsZero grayscale (1)| ✅           |
| CFA/Bayer (32803)        | ✅ bilinear  |
| 8-bit per channel        | ✅           |
| 16-bit per channel       | ✅           |
| Multi-strip layout       | ✅           |
| Tile layout              | ✅           |
| Custom WB + colour matrix| ✅ via opts  |

## For downstream RAW codecs

```rust
use image_codec_tiff::{parse_ifd_chain, decode_tiff_with_opts, TiffDecodeOptions};

// Parse all IFDs (for RAW format inspection).
let ifds = parse_ifd_chain(&bytes)?;
let raw_ifd = &ifds[0];
let width = raw_ifd.width;

// Decode with camera-specific colour parameters.
let opts = TiffDecodeOptions {
    wb_multipliers: [2.1, 1.0, 1.7],
    color_matrix: [[1.5, -0.3, -0.1],
                   [-0.2, 1.4, -0.1],
                   [0.0, -0.1, 1.2]],
    black_level: [512; 4],
    white_level: 4095,
    ..Default::default()
};
let pixels = decode_tiff_with_opts(&bytes, &opts)?;
```

## Testing

```
cargo test -p image-codec-tiff
```

80 unit tests cover:

| Category                  | Tests |
|---------------------------|-------|
| Round-trip (RGB)          | 4     |
| Byte order (LE + BE)      | 2     |
| PackBits                  | 2     |
| LZW                       | 2     |
| Grayscale 16-bit          | 1     |
| CFA / Bayer               | 3     |
| IFD chain parsing         | 5     |
| Strip assembly            | 4     |
| Encoder correctness       | 5     |
| Error cases               | 5     |
| Codec trait               | 3     |
| Colour pipeline           | 6     |
| Bayer demosaicing         | 8+    |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Spec

See [`code/specs/IC09-image-codec-tiff.md`](../../../../specs/IC09-image-codec-tiff.md).
