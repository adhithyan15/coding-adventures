# image-codec-gif — IC07

GIF (Graphics Interchange Format) image codec for the coding-adventures stack.

Decodes GIF87a and GIF89a files into RGBA8 `PixelContainer` buffers and encodes
`PixelContainer` pixels as static GIF87a/GIF89a files.  Implements the
`paint_instructions::ImageCodec` trait so it plugs directly into the renderer.

---

## Overview

GIF was designed at CompuServe in 1987 to transfer images efficiently over slow
modems.  Its two distinctive constraints — **indexed colour** (≤ 256 palette
entries per image) and **GIF-variant LZW compression** — still make it the
dominant format for short animations on the web today.  This crate handles the
static-image subset of the format.

### What this crate does

| Feature | Support |
|---------|---------|
| GIF87a static decode | ✓ |
| GIF89a static decode | ✓ |
| GIF89a transparency (alpha = 0) | ✓ |
| Interlaced images (4-pass de-interlace) | ✓ |
| GIF87a encode (fully opaque images) | ✓ |
| GIF89a encode (images with transparency) | ✓ |
| Exact palette (≤ 256 distinct colours) | ✓ |
| Median-cut quantisation (> 256 colours) | ✓ |
| Animated GIF decode | ✗ (error on second frame) |
| Animated GIF encode | ✗ |

---

## Quick start

```rust
use image_codec_gif::{encode_gif, decode_gif};
use pixel_container::PixelContainer;

// Encode a 2×2 red image.
let mut px = PixelContainer::new(2, 2);
px.fill(255, 0, 0, 255);
let bytes = encode_gif(&px);
assert!(bytes.starts_with(b"GIF"));

// Decode it back.
let recovered = decode_gif(&bytes).unwrap();
assert_eq!(recovered.width, 2);
assert_eq!(recovered.height, 2);
```

### Using the `ImageCodec` trait

```rust
use image_codec_gif::GifCodec;
use paint_instructions::ImageCodec;
use pixel_container::PixelContainer;

let codec = GifCodec;
assert_eq!(codec.mime_type(), "image/gif");

let mut px = PixelContainer::new(4, 4);
px.fill(0, 128, 255, 255);
let bytes = codec.encode(&px);
let recovered = codec.decode(&bytes).unwrap();
```

---

## Format details

### GIF file structure

```
"GIF87a" or "GIF89a"          (6 bytes — header)
Logical Screen Descriptor      (7 bytes)
Global Color Table             (3 × 2^(n+1) bytes, if present)
[Graphic Control Extension]    (GIF89a only — for transparency / timing)
Image Descriptor               (10 bytes)
[Local Color Table]            (optional override)
Image Data                     (LZW-compressed, sub-block framed)
Trailer 0x3B                   (1 byte)
```

### LZW compression (GIF variant)

GIF uses a configurable-width LZW variant:

- **Minimum code size** (`lzw_minimum_code_size`): 2–8 bits, derived from
  `ceil(log2(palette_size))`.  With 4 colours it starts at 2; with 256 colours
  at 8.
- **Special codes**: `CLEAR = 2^mcs` and `EOI = CLEAR + 1`.  Dynamic codes
  start at `EOI + 1`.
- **Code growth**: code width grows by 1 bit each time `next_code` exceeds
  `2^current_width`, up to 12 bits.
- **Sub-block framing**: the raw bit stream is wrapped in blocks of ≤ 255 bytes,
  each prefixed by its 1-byte length.  A zero-length block terminates the data.
- **Bit packing**: LSB-first within each byte.

### Transparency

- Pixels with alpha < 128 are treated as fully transparent.
- When any transparent pixel is present, the encoder outputs GIF89a with a
  Graphic Control Extension designating one palette index as transparent.
- On decode, pixels whose index matches the transparent index receive alpha = 0;
  all others receive alpha = 255.

### Palette quantisation

When an image has more than 256 distinct opaque colours, the encoder applies
**median-cut quantisation**: it recursively splits the colour space along the
axis with the greatest range until 256 buckets remain, then assigns each pixel
to the nearest bucket centroid using squared Euclidean distance.

---

## Crate layout

```
image-codec-gif/
  src/
    lib.rs      — Public API, GifCodec struct, integration tests
    encoder.rs  — GIF87a/89a encoder; palette building; median-cut
    decoder.rs  — GIF87a/89a parser; de-interlacing
    lzw.rs      — GIF-variant LZW encoder + decoder; BitWriter/BitReader
```

---

## Dependencies

| Crate | Role |
|-------|------|
| `pixel-container` | RGBA8 pixel buffer (IC-stack layer 0) |
| `paint-instructions` | `ImageCodec` trait (IC-stack layer 0) |

No third-party crates.

---

## Position in the IC stack

```
paint-canvas  (renderer)
      │
paint-instructions  (ImageCodec trait)
      │
image-codec-gif  ← you are here   (IC07)
      │
pixel-container  (RGBA8 buffer)
```

---

## Changelog

See [CHANGELOG.md](CHANGELOG.md).
