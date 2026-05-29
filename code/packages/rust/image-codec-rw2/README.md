# image-codec-rw2

Panasonic RW2 RAW image decoder for Rust.

## What is RW2?

RW2 (RAW version 2) is Panasonic's proprietary camera format used in every
Lumix body since the GH1 (2009). It stores uncompressed 12-bit Bayer data
from the camera sensor alongside white balance and crop metadata.

## How it fits in the stack

```
RW2 bytes
  → Rw2Codec::decode()
  → PixelContainer (RGBA8)
  → PngCodec::encode()
  → PNG bytes
```

`image-codec-rw2` depends only on `pixel-container` (the shared RGBA buffer
type) and `paint-instructions` (the `ImageCodec` trait). It has no dependency
on `image-codec-tiff` — it parses its own TIFF-like IFD.

## Usage

```rust
use image_codec_rw2::{decode_rw2, encode_rw2, Rw2Codec, VERSION};
use paint_instructions::ImageCodec;

// Decode
let bytes = std::fs::read("DSCF0001.RW2").unwrap();
let pixels = decode_rw2(&bytes).expect("not a valid RW2 file");
println!("{}×{} image", pixels.width, pixels.height);

// Via trait
let pixels = Rw2Codec.decode(&bytes).unwrap();

// Minimal test encoder (produces valid RW2 that decodes back)
let rw2_bytes = encode_rw2(&pixels);

println!("Version: {VERSION}");
```

## Decode pipeline

1. Validate the 8-byte RW2 magic (`"II"` + version 85)
2. Parse the TIFF-like IFD for Panasonic private tags
3. Unpack 12-bit little-endian packed pixels (2 pixels per 3 bytes)
4. Crop to the active sensor area using border tags
5. Bilinear Bayer demosaicing (RGGB pattern)
6. White balance from tags 0x0011/0x0012
7. 3×3 Panasonic colour matrix (GH5 D65)
8. sRGB gamma curve
9. Build `PixelContainer` (RGBA8, A=255)

## Limitations (v0.1)

- Only **12-bit packed** (uncompressed) RW2 is supported.
- **Panasonic lossless** compression (GH5/S1/S5 v5+) returns `Err`.
- **16-bit depth** returns `Err`.
- A single hardcoded colour matrix (GH5 D65) is used for all models.
- Sensor dimensions above 4096×4096 are rejected.

## File format reference

```
Offset  Size  Field
0       2     "II" — always little-endian
2       2     0x0055 (85) — RW2 version marker (NOT TIFF's 42)
4       4     Offset of first IFD (u32 LE, usually 8)
8+      —     TIFF-like IFD with Panasonic private tags
?       —     12-bit LE packed raw Bayer data
```

## Module structure

| File          | Responsibility                                    |
|---------------|---------------------------------------------------|
| `lib.rs`      | Public API, `Rw2Codec`, `VERSION`                 |
| `header.rs`   | Magic validation, IFD parsing                     |
| `unpack.rs`   | 12-bit LE packed pixel reader                     |
| `bayer.rs`    | RGGB bilinear Bayer demosaicing                   |
| `color.rs`    | White balance, colour matrix, sRGB gamma          |
| `decoder.rs`  | Top-level `decode_rw2` pipeline                   |
| `encoder.rs`  | Minimal test encoder                              |

## References

- dcraw.c `panasonic_load_raw()` — reference C implementation
- LibRaw Panasonic decoders — https://github.com/LibRaw/LibRaw
- Exiv2 Panasonic tag database — https://exiv2.org/tags-panasonic.html
- rawspeed — https://github.com/darktable-org/rawspeed
