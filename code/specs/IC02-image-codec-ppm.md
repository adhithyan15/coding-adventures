# IC02 — `image-codec-ppm`: PPM P6 Image Encoder/Decoder

## Overview

PPM (Portable Pixmap) is a Unix image format defined by Jef Poskanzer as part
of the Netpbm suite. It is deliberately minimal: a short ASCII header followed
by raw RGB bytes. There is no compression, no colour tables, no metadata
negotiation, and no optional fields.

The P6 binary variant is the most useful form:

- the header is a few lines of ASCII text
- pixel data is raw 8-bit RGB, packed with no alignment
- reading and writing are trivial loops
- every Unix image tool (ImageMagick, ffmpeg, Netpbm) accepts it

PPM stores **no alpha channel**. A `PixelContainer` with premultiplied
transparent pixels must be composited over a background before encoding.

---

## File Format

```
P6\n
<width> <height>\n
255\n
<width * height * 3 bytes: R G B per pixel, row-major>
```

Field breakdown:

| Part | Description |
|------|-------------|
| `P6` | Magic — identifies P6 binary PPM |
| `\n` | Newline after magic |
| `width` | Decimal ASCII integer |
| ` ` | Single space |
| `height` | Decimal ASCII integer |
| `\n` | Newline after dimensions |
| `255` | Max value — always 255 for 8-bit |
| `\n` | Newline after max value |
| pixel data | `width * height * 3` raw bytes, no separators |

Example header for a 640×480 image:

```
P6
640 480
255
[binary pixel data follows immediately]
```

### Pixel Data Layout

Three bytes per pixel in **RGB** order, row-major, top-left origin:

```
file[header_end + (y * width + x) * 3 + 0] = R
file[header_end + (y * width + x) * 3 + 1] = G
file[header_end + (y * width + x) * 3 + 2] = B
```

There is no padding, no row alignment, and no end-of-row marker.

### Alpha Handling

PPM has no alpha channel. The encoding strategy:

- **Encode**: drop the alpha byte. Write only R, G, B per pixel.
  This is a lossy operation for images with partial transparency, but acceptable
  because PPM is primarily used for opaque images and pipeline intermediates.
- **Decode**: set alpha = 255 for every decoded pixel.

If compositing over a background before encoding is needed, that is the
caller's responsibility before passing the `PixelContainer` to `encode`.

---

## Encode Algorithm

```
1. Build the ASCII header:
     "P6\n{width} {height}\n255\n"
2. Allocate output buffer:
     header bytes + width * height * 3
3. Copy header bytes
4. For each pixel (row-major, y=0..height, x=0..width):
     let (r, g, b, _a) = container.pixel_at(x, y)
     write r, g, b
5. Return the buffer
```

The header can be formatted with standard string formatting. No binary
encoding is needed for the header — it is plain ASCII text.

---

## Decode Algorithm

```
1. Verify magic: file starts with "P6"
2. Skip whitespace after magic
3. Skip any comment lines (lines starting with '#')
4. Read width (decimal ASCII integer)
5. Skip whitespace
6. Read height (decimal ASCII integer)
7. Skip whitespace
8. Read max value (decimal ASCII integer)
   → return Err if max value != 255 (only 8-bit depth supported)
9. Skip exactly one whitespace byte after max value
10. Verify remaining bytes == width * height * 3
11. For each pixel:
      read R, G, B
      store R, G, B, 255 into PixelContainer
12. Return Ok(PixelContainer::from_data(width, height, data))
```

**Comment handling**: The PPM spec allows `#`-prefixed comment lines between
any whitespace-separated tokens. Implementations should skip them. Comments
are rare in practice but required by the spec.

---

## API

```rust
pub struct PpmCodec;

impl ImageCodec for PpmCodec {
    fn mime_type(&self) -> &'static str {
        "image/x-portable-pixmap"
    }

    fn encode(&self, container: &PixelContainer) -> Vec<u8>;
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String>;
}

/// Convenience wrapper: encode a PixelContainer to PPM bytes.
pub fn encode_ppm(container: &PixelContainer) -> Vec<u8> {
    PpmCodec.encode(container)
}

/// Convenience wrapper: decode PPM bytes into a PixelContainer.
pub fn decode_ppm(bytes: &[u8]) -> Result<PixelContainer, String> {
    PpmCodec.decode(bytes)
}
```

---

## Error Cases

| Condition | Error message |
|-----------|---------------|
| Does not start with `P6` | `"PPM: invalid magic, expected P6"` |
| Width or height not parseable | `"PPM: invalid dimensions"` |
| Max value != 255 | `"PPM: unsupported max value {n}, only 255 supported"` |
| Pixel data truncated | `"PPM: pixel data truncated"` |

---

## Round-Trip Property (Opaque Images)

For any `PixelContainer` where all pixels have A=255:

```rust
let encoded = PpmCodec.encode(&p);
let decoded  = PpmCodec.decode(&encoded).unwrap();
assert_eq!(p.width,  decoded.width);
assert_eq!(p.height, decoded.height);
// RGB is preserved; A is always 255 after decode
for y in 0..p.height {
    for x in 0..p.width {
        let (r1, g1, b1, _) = p.pixel_at(x, y);
        let (r2, g2, b2, a) = decoded.pixel_at(x, y);
        assert_eq!((r1, g1, b1), (r2, g2, b2));
        assert_eq!(a, 255);
    }
}
```

Alpha information is not preserved by PPM; callers must not rely on it.

---

## Interoperability

A PPM file generated by this encoder can be read by:

- ImageMagick: `convert input.ppm output.png`
- ffmpeg: `ffmpeg -i input.ppm output.mp4`
- Netpbm: `pnmtopng input.ppm > output.png`
- Any viewer that supports Netpbm formats

A PPM file generated by those tools can be decoded by this crate.

---

## Crate Layout

```
code/packages/rust/image-codec-ppm/
├── Cargo.toml    # depends on pixel-container only
├── src/
│   └── lib.rs
├── BUILD
├── README.md
└── CHANGELOG.md
```

Dependencies: `pixel-container` only. Zero external libraries.
