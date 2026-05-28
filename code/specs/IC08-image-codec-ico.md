# IC08 — ICO / CUR Image Codec

**Specification version**: 0.1  
**Status**: Draft  
**Depends on**: IC00 (pixel-container), IC01 (image-codec-bmp)  
**Implements**: Windows ICO and CUR file formats (Microsoft MSDN specification)

---

## 1. Overview

The ICO format is a container for one or more Windows icon images at different
sizes and color depths. A single `.ico` file can contain a 16×16 icon, a 32×32
icon, a 256×256 icon, and so on — the operating system picks the best size for
the rendering context. CUR files use the same structure but hold cursor images
with a "hot-spot" coordinate instead of reserved bytes.

**Key properties**:

- **Container**: flat header + directory array + image data blobs
- **Image encoding**: each frame is either a **BMP DIB** (Device-Independent
  Bitmap, without the 14-byte BITMAPFILEHEADER) or a **PNG** (full PNG file,
  detected by the `\x89PNG` magic bytes)
- **Color depths**: 1, 4, 8, 24, or 32 bits per pixel
- **AND mask**: 1-bpp mask stored after BMP pixel data for transparency (for
  non-32bpp images); 32bpp images embed alpha in the BGRA pixels directly
- **Royalty-free**: the format is fully documented in MSDN and has no patents
- **Max dimensions**: 256×256 per image; coordinates stored as 1-byte values
  (0 means 256)
- **Max images per file**: 65535 (u16 count field)

---

## 2. File Structure

```
ICO file layout
───────────────
Offset  Size  Field
0       2     reserved — always 0
2       2     type — 1 for ICO, 2 for CUR
4       2     count — number of images in the file

For each image i in 0..count:
  (6 + i*16) + 0   1   width  (pixels; 0 means 256)
  (6 + i*16) + 1   1   height (pixels; 0 means 256)
  (6 + i*16) + 2   1   color_count (palette size; 0 if truecolor or PNG)
  (6 + i*16) + 3   1   reserved (0 for ICO; hotspot X for CUR)
  (6 + i*16) + 4   2   planes (ICO: 1; CUR: hotspot X low word)
  (6 + i*16) + 6   2   bit_count (bits per pixel; 0 for PNG frames)
  (6 + i*16) + 8   4   bytes_in_res — byte length of the image data
  (6 + i*16) + 12  4   image_offset — byte offset from start of file

After the directory, at each image_offset:
  Either:
    \x89PNG\r\n\x1a\n …  → full PNG file (libjxl, macOS, etc. use this for 256×256)
  Or:
    BITMAPINFOHEADER (40 bytes)
    [palette: color_count * 4 bytes, RGBQUAD]
    [XOR pixel data: row-padded to 4-byte boundaries]
    [AND mask: 1bpp, same row-padding]
```

### 2.1 CUR vs ICO differences

The only structural difference between ICO and CUR is the `type` field (1 vs 2)
and the interpretation of bytes 4–5 in each directory entry:

| Field     | ICO         | CUR                        |
|-----------|-------------|----------------------------|
| type      | 1           | 2                          |
| planes(4) | 1 (planes)  | hotspot_x (cursor hot-spot) |
| bit_count | bpp         | hotspot_y                  |

This codec decodes both ICO and CUR, treating the hot-spot fields as opaque.

---

## 3. BMP DIB Image Data

When the image data is **not** a PNG (first 8 bytes ≠ `\x89PNG\r\n\x1a\n`), it
is a BMP **DIB** — a `BITMAPINFOHEADER` directly, without the 14-byte
`BITMAPFILEHEADER` prefix that a `.bmp` file would have.

### 3.1 BITMAPINFOHEADER (40 bytes)

```
biSize:          u32LE   = 40
biWidth:         i32LE   (image width in pixels)
biHeight:        i32LE   (2 × image height; positive means bottom-up row order)
biPlanes:        u16LE   = 1
biBitCount:      u16LE   (bits per pixel: 1, 4, 8, 24, or 32)
biCompression:   u32LE   = 0 (BI_RGB — uncompressed)
biSizeImage:     u32LE   (can be 0 for BI_RGB)
biXPelsPerMeter: i32LE   (ignored)
biYPelsPerMeter: i32LE   (ignored)
biClrUsed:       u32LE   (palette entries; 0 means 2^biBitCount for ≤8bpp)
biClrImportant:  u32LE   (ignored)
```

**Height encoding**: ICO stores `biHeight = 2 × pixel_height` because the DIB
contains both the XOR mask (color data) and the AND mask (1bpp transparency),
each of height `pixel_height`. The actual image height is `biHeight / 2`.

### 3.2 Palette (for 1, 4, 8 bpp images)

After the BITMAPINFOHEADER, a color palette follows:

```
palette_count = biClrUsed if biClrUsed > 0 else (1 << biBitCount)
For each entry (RGBQUAD, 4 bytes each):
  blue:     u8
  green:    u8
  red:      u8
  reserved: u8 (always 0)
```

### 3.3 XOR Pixel Data (color image)

Row-major order, **bottom row first** (bottom-up). Each row is padded to a
4-byte boundary.

Row stride (bytes) = `((width * biBitCount + 31) / 32) * 4`

| biBitCount | Pixel encoding |
|------------|---------------|
| 1          | 1 bit per pixel; 8 pixels per byte, MSB first |
| 4          | 4 bits per pixel; 2 pixels per byte, high nibble first |
| 8          | 1 byte per pixel (palette index) |
| 24         | 3 bytes: Blue, Green, Red |
| 32         | 4 bytes: Blue, Green, Red, Alpha (pre-multiplied or straight alpha) |

### 3.4 AND Mask (1 bpp transparency mask)

After the XOR data, a 1bpp AND mask follows. Each bit corresponds to one pixel:
- `0` = opaque (show XOR pixel)
- `1` = transparent (show whatever is behind the icon)

Row stride = `((width + 31) / 32) * 4`

For 32bpp images, the AND mask **may** be all zeros (alpha channel in the BGRA
data is the authoritative transparency source). Decoders should use alpha=0 for
any pixel where the AND mask bit is 1.

### 3.5 Compositing rule

```
final_pixel = if and_mask_bit == 1:
  transparent (alpha = 0)
elif biBitCount == 32:
  BGRA alpha channel in XOR data
else:
  fully opaque (alpha = 255)
```

---

## 4. PNG Image Data

When the first 8 bytes of the image data are `\x89PNG\r\n\x1a\n`, the entire
`bytes_in_res` block is a complete, self-contained PNG file. This is the
dominant encoding for 256×256 Vista-style icons.

Decode it via the `png` or `paint-codec-png` crate (already in the workspace).

---

## 5. Encoding (encode_ico)

For Phase 1, the encoder writes a single-image ICO from a `PixelContainer`:

```
1. Compute effective dimensions (min(width,255), min(height,255))
   — clamp to 255 because the directory byte field wraps 256→0
2. Build a 32bpp BGRA BMP DIB:
   a. BITMAPINFOHEADER with biHeight = 2 * height (XOR + AND)
   b. XOR pixel data — rows bottom-up, 4-byte padded
   c. AND mask — all zeros (alpha from BGRA)
3. Write ICO file:
   a. Header: reserved=0, type=1, count=1
   b. Directory entry: width, height, colorCount=0, reserved=0,
      planes=1, bitCount=32, bytesInRes, imageOffset=22
   c. BMP DIB data
```

Encoding uses 32bpp BGRA with an all-zero AND mask. This gives full RGBA
fidelity and is understood by all modern Windows / macOS / Linux icon renderers.

---

## 6. Decoding (decode_ico)

```
1. Check header: reserved==0, type==1 or 2, count >= 1.
2. Read directory entries.
3. Choose the best image:
   a. Prefer the largest dimensions (max width × height).
   b. Among equal sizes, prefer 32bpp BMP or PNG over lower bit depths.
4. Seek to image_offset.
5. If first 8 bytes == PNG magic → decode with png crate.
6. Else → decode BMP DIB:
   a. Parse BITMAPINFOHEADER.
   b. Read palette (if biBitCount ≤ 8).
   c. Read XOR rows (bottom-up → flip to top-down).
   d. Read AND mask rows.
   e. Convert to RGBA: apply palette, apply AND mask for transparency.
7. Return PixelContainer.
```

---

## 7. Error Cases

| Condition | Error message |
|-----------|--------------|
| File shorter than 6 bytes | `"ICO: file too short"` |
| `reserved != 0` | `"ICO: invalid reserved field (expected 0)"` |
| `type != 1 && type != 2` | `"ICO: unknown type {t} (expected 1=ICO or 2=CUR)"` |
| `count == 0` | `"ICO: no images in file"` |
| Image offset + size exceeds file | `"ICO: image data extends past end of file"` |
| BMP compression != 0 | `"ICO: compressed BMP DIB not supported"` |
| BMP biSize != 40 | `"ICO: unsupported BITMAPINFOHEADER size {n}"` |
| biBitCount not in {1,4,8,24,32} | `"ICO: unsupported bit depth {n}"` |
| PNG decode error | `"ICO: PNG decode error: {e}"` |
| biWidth / biHeight == 0 | `"ICO: zero-dimension image"` |

---

## 8. API

```rust
pub struct IcoCodec;

impl ImageCodec for IcoCodec {
    fn mime_type(&self) -> &'static str { "image/x-icon" }
    fn encode(&self, pixels: &PixelContainer) -> Vec<u8> { encode_ico(pixels) }
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> { decode_ico(bytes) }
}

/// Encode a PixelContainer as a single-image 32bpp ICO file.
pub fn encode_ico(pixels: &PixelContainer) -> Vec<u8>;

/// Decode the best-resolution image from an ICO or CUR file.
pub fn decode_ico(bytes: &[u8]) -> Result<PixelContainer, String>;
```

---

## 9. Crate Layout

```
image-codec-ico/
  Cargo.toml        deps: pixel-container, paint-instructions, png
  BUILD
  README.md
  CHANGELOG.md
  src/
    lib.rs          IcoCodec, VERSION, encode_ico, decode_ico, integration tests
    encoder.rs      Single-image 32bpp BGRA ICO encoder
    decoder.rs      ICO parser, image selector, BMP DIB + PNG dispatch
    bmp_dib.rs      BITMAPINFOHEADER parse, palette, XOR + AND decode
```

---

## 10. Test Plan

| Test | What it verifies |
|------|-----------------|
| `round_trip_solid_rgba` | Encode 4×4 RGBA → decode → pixel-exact |
| `round_trip_transparent` | Fully transparent image; AND mask all-ones |
| `round_trip_mixed_alpha` | Mixed opaque/transparent pixels |
| `encode_produces_correct_header` | Verify ICO header bytes (type=1, count=1) |
| `encode_bit_count_is_32` | directory bitCount field == 32 |
| `encode_image_offset_is_22` | imageOffset = 6 + 16 = 22 |
| `decode_selects_largest_image` | Multi-image ICO; largest is returned |
| `decode_bmp_24bpp` | Decode a hand-crafted 24bpp BMP DIB ICO |
| `decode_bmp_8bpp` | Decode a hand-crafted 8bpp palette ICO |
| `decode_png_frame` | Decode an ICO whose image data is a full PNG |
| `decode_cur_file` | CUR file (type=2) accepted without error |
| `decode_error_bad_magic` | Garbage input → Err |
| `decode_error_bad_type` | type=3 → Err |
| `decode_error_zero_count` | count=0 → Err |
| `mime_type` | `"image/x-icon"` |

Target: ≥ 95% coverage.

---

## 11. Teaching Notes

### Why bottom-up rows?

Windows bitmaps are stored bottom-up by default (biHeight > 0) because
the Windows GDI origin is at the bottom-left corner, matching mathematical
conventions. Most image formats (PNG, JPEG, WebP) store top-down. When
decoding a BMP DIB in an ICO, you must reverse the row order.

### Why two masks (XOR and AND)?

The original Windows 1.0 icon rendering used two bitwise operations on
the screen pixels:

```
screen = (screen AND and_mask) XOR xor_mask
```

- `and_mask = 1, xor_mask = 0` → black (transparent over black bg)
- `and_mask = 0, xor_mask = ?` → opaque pixel
- `and_mask = 1, xor_mask = 1` → inverted (XOR blend, rare)

Modern 32bpp icons bypass this entirely — the alpha channel in BGRA
provides per-pixel opacity, and the AND mask is conventionally all zeros.

### ICO vs PNG at 256×256

Windows Vista introduced PNG-compressed frames for the 256×256 size.
A modern `.ico` file typically contains both:
- 256×256 PNG frame (crisp at large size)
- 48×48, 32×32, 16×16 BMP DIB frames (for legacy rendering at exact sizes)

Our encoder emits only one frame for simplicity; our decoder picks the largest.

---

## 12. Relationship to Other Specs

| Dependency | Role |
|------------|------|
| IC00 (pixel-container) | RGBA pixel buffer |
| paint-instructions | `ImageCodec` trait |
| IC01 (image-codec-bmp) | BMP DIB decode concepts (shared approach, not code import) |
| png / paint-codec-png | PNG decode for Vista-style 256×256 frames |
