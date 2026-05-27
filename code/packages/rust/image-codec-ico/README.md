# image-codec-ico

ICO (Windows Icon) and CUR (cursor) image codec — IC08.

Encodes a `PixelContainer` as a single-image 32bpp ICO file and decodes the
best-resolution image from any ICO/CUR file into an RGBA8 `PixelContainer`.

## Where this fits in the stack

```
paint-instructions  ←  ImageCodec trait
      │
image-codec-ico     ←  YOU ARE HERE
      │
pixel-container     ←  PixelContainer (RGBA8 pixel buffer)
      │
png                 ←  PNG frame decoding (for PNG-embedded ICO frames)
```

ICO is not a compression format — it is a **container** that bundles one or more
images (each stored as a BMP DIB or a full PNG file) inside a directory.
`image-codec-ico` is deliberately minimal: it always encodes as a single-image
32bpp BMP DIB file and decodes by picking the largest frame.

## Quick start

```rust
use image_codec_ico::{encode_ico, decode_ico};
use pixel_container::PixelContainer;

// Encode a 2×2 red ICO.
let mut px = PixelContainer::new(2, 2);
px.fill(255, 0, 0, 255);
let bytes = encode_ico(&px);
assert_eq!(&bytes[2..4], &[1, 0]); // type = ICO (not CUR)

// Decode it back.
let recovered = decode_ico(&bytes).unwrap();
assert_eq!(recovered.width, 2);
assert_eq!(recovered.height, 2);

// Use via the ImageCodec trait.
use paint_instructions::ImageCodec;
use image_codec_ico::IcoCodec;

let codec = IcoCodec;
assert_eq!(codec.mime_type(), "image/x-icon");
let encoded = codec.encode(&px);
let decoded = codec.decode(&encoded).unwrap();
```

## File format overview

```
Offset  Len  Field
──────  ───  ──────────────────────────────────────
0       2    Reserved (must be 0)
2       2    Type: 1 = ICO, 2 = CUR
4       2    Image count
────── directory entry (16 bytes per image) ──────
6       1    Width  (1-255; 0 means 256)
7       1    Height (1-255; 0 means 256)
8       1    Color count (0 = truecolor)
9       1    Reserved
10      2    Planes
12      2    Bit count (32 for this crate)
14      4    Bytes in image data
18      4    File offset of image data
────── image data at IMAGE_OFFSET = 22 ───────────
22      40   BITMAPINFOHEADER
62      n    XOR pixel data (BGRA, rows bottom-up)
62+n    m    AND mask (1bpp, rows bottom-up)
```

## Encoding

`encode_ico` always writes:
- One directory entry (image count = 1)
- 32bpp BGRA BMP DIB (full alpha in the BGRA byte)
- All-zero AND mask (alpha transparency handled by BGRA)
- `biHeight = 2 × pixel_height` (XOR + AND stacked per BMP DIB convention)

Dimensions are clamped to 255 × 255 (the ICO directory byte range for explicit
sizes; 0 encodes 256 but we stay in the 1-255 range to avoid ambiguity).

## Decoding

`decode_ico` selects the **best** frame from the directory using this priority:
1. Largest area (width × height)
2. On tie: PNG frame preferred over BMP; among BMP frames, higher bpp wins

Each frame is dispatched to:
- `decode_png_frame` — calls `png::decode_png_rgba` for PNG-embedded frames
- `decode_bmp_frame` — calls `bmp_dib::decode_bmp_dib` for BMP DIB frames

BMP DIB decoding handles 1, 4, 8, 24, and 32 bpp. The AND mask overrides the
per-pixel alpha: AND bit = 1 forces that pixel to fully transparent.

### Safety

- Maximum image dimensions: 4096 × 4096 (hard-coded limit before any allocation)
- Invalid header sizes, bad magic bytes, truncated data — all return `Err`

## API

```rust
pub fn encode_ico(pixels: &PixelContainer) -> Vec<u8>;
pub fn decode_ico(bytes: &[u8]) -> Result<PixelContainer, String>;

pub struct IcoCodec;
impl paint_instructions::ImageCodec for IcoCodec { ... }
pub const VERSION: &str = "0.1.0";
```

## Testing

```
cargo test -p image-codec-ico
```

34 unit + integration tests cover:

| Category                  | Tests |
|---------------------------|-------|
| Round-trip (various sizes) | 5    |
| Header / directory bytes   | 5    |
| BMP DIB decoder (1/4/8/24/32 bpp) | 7 |
| AND mask transparency      | 1    |
| Error cases                | 5    |
| MIME type                  | 1    |
| Codec trait                | 1    |
| Doc-test                   | 1    |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Spec

See [`code/specs/IC08-image-codec-ico.md`](../../../../specs/IC08-image-codec-ico.md).
