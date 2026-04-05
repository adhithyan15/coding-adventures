# IC01 — `image-codec-bmp`: BMP Image Encoder/Decoder

## Overview

BMP (Bitmap) is one of the oldest image formats, introduced with Windows 1.0 in
1985. It stores raw pixel data with a small binary header. There is no
compression in the variant we implement (BI_RGB), which makes BMP trivially
easy to encode and decode — every pixel maps directly to four bytes in the file.

BMP is not used for distribution (the files are large), but it is an excellent
teaching format:

- the header is completely fixed-size (always 54 bytes for our variant)
- pixel data follows immediately after the header
- the only transform needed is a single byte-swap per pixel (RGBA → BGRA)
- decoding is symmetric: undo the byte-swap, return the buffer

This crate implements BMP encode and decode for 32-bit BGRA images
(BITMAPINFOHEADER with `biBitCount=32`, `biCompression=BI_RGB`).

---

## BMP File Structure

A BMP file consists of two headers followed by raw pixel data:

```
┌──────────────────────────────────────────┐
│  BITMAPFILEHEADER  (14 bytes, offset 0)  │
├──────────────────────────────────────────┤
│  BITMAPINFOHEADER  (40 bytes, offset 14) │
├──────────────────────────────────────────┤
│  Pixel data        (width*height*4 bytes)│
└──────────────────────────────────────────┘
     Total: 54 + width * height * 4 bytes
```

All multi-byte integers are **little-endian** (least-significant byte first).

### BITMAPFILEHEADER (14 bytes)

| Offset | Size | Field        | Value                        | Notes                          |
|--------|------|--------------|------------------------------|--------------------------------|
| 0      | 2    | `bfType`     | `0x4D42`                     | ASCII `'BM'` — magic signature |
| 2      | 4    | `bfSize`     | `54 + width * height * 4`    | Total file size in bytes       |
| 6      | 2    | `bfReserved1`| `0`                          | Must be zero                   |
| 8      | 2    | `bfReserved2`| `0`                          | Must be zero                   |
| 10     | 4    | `bfOffBits`  | `54`                         | Byte offset to pixel data      |

The `bfType` magic `0x4D42` is the two ASCII bytes `'B'` (0x42) and `'M'`
(0x4D) stored in little-endian order. Reading it as a `u16` LE gives 0x4D42.

### BITMAPINFOHEADER (40 bytes, starts at offset 14)

| Offset | Size | Field            | Value               | Notes                              |
|--------|------|------------------|---------------------|------------------------------------|
| 14     | 4    | `biSize`         | `40`                | Size of this header                |
| 18     | 4    | `biWidth`        | `width as i32`      | Image width in pixels              |
| 22     | 4    | `biHeight`       | `-(height as i32)`  | Negative = top-down scanline order |
| 26     | 2    | `biPlanes`       | `1`                 | Always 1                           |
| 28     | 2    | `biBitCount`     | `32`                | 32 bits per pixel (BGRA8)          |
| 30     | 4    | `biCompression`  | `0`                 | BI_RGB = no compression            |
| 34     | 4    | `biSizeImage`    | `width * height * 4`| Size of pixel data in bytes        |
| 38     | 4    | `biXPelsPerMeter`| `0`                 | Horizontal resolution (ignored)    |
| 42     | 4    | `biYPelsPerMeter`| `0`                 | Vertical resolution (ignored)      |
| 46     | 4    | `biClrUsed`      | `0`                 | Colour table entries used          |
| 50     | 4    | `biClrImportant` | `0`                 | All colours are important          |

### Why Negative `biHeight`?

The original BMP format stores scanlines **bottom-up** (the last row of pixels
is first in the file). A positive `biHeight` means bottom-up layout.

A **negative** `biHeight` signals top-down layout, which matches our
`PixelContainer` (top-left origin, row 0 first). Using `-(height as i32)` lets
us write the pixel data in natural order without any row reversal.

### Pixel Data Layout

Each pixel is stored as four bytes in **BGRA** order:

```
file[54 + (y * width + x) * 4 + 0] = B
file[54 + (y * width + x) * 4 + 1] = G
file[54 + (y * width + x) * 4 + 2] = R
file[54 + (y * width + x) * 4 + 3] = A
```

Our `PixelContainer` stores pixels as RGBA. The only encoding transform is
swapping bytes 0 and 2 (R ↔ B) for every pixel.

---

## Encode Algorithm

```
1. Allocate output: Vec<u8> with capacity 54 + width * height * 4
2. Write BITMAPFILEHEADER (14 bytes, little-endian):
     [0x42, 0x4D]                         // 'BM'
     (54 + w*h*4) as u32 LE              // bfSize
     0u16 LE                              // bfReserved1
     0u16 LE                              // bfReserved2
     54u32 LE                             // bfOffBits
3. Write BITMAPINFOHEADER (40 bytes, little-endian):
     40u32 LE                             // biSize
     width as i32 LE                      // biWidth
     -(height as i32) LE                  // biHeight  (negative = top-down)
     1u16 LE                              // biPlanes
     32u16 LE                             // biBitCount
     0u32 LE                              // biCompression (BI_RGB)
     (width * height * 4) as u32 LE       // biSizeImage
     0i32 LE                              // biXPelsPerMeter
     0i32 LE                              // biYPelsPerMeter
     0u32 LE                              // biClrUsed
     0u32 LE                              // biClrImportant
4. For each pixel in row-major order:
     write B, G, R, A  (swap bytes 0 and 2 from RGBA source)
5. Return the buffer
```

---

## Decode Algorithm

```
1. Verify minimum length: at least 54 bytes
2. Verify magic: bytes[0..2] == [0x42, 0x4D]
3. Read biWidth  as i32 LE from bytes[18..22]  → width  = biWidth.abs() as u32
4. Read biHeight as i32 LE from bytes[22..26]  → height = biHeight.abs() as u32
5. Read bfOffBits as u32 LE from bytes[10..14] → pixel_offset
6. Read biBitCount as u16 LE from bytes[28..30]
   → return Err if biBitCount != 32
7. Read biCompression as u32 LE from bytes[30..34]
   → return Err if biCompression != 0  (only BI_RGB supported)
8. Check that bytes.len() >= pixel_offset + width * height * 4
9. Determine scanline direction:
   - biHeight < 0 → top-down (no reversal needed)
   - biHeight > 0 → bottom-up (reverse row order during copy)
10. For each pixel in output order:
      read B, G, R, A from file
      write R, G, B, A into PixelContainer data
11. Return Ok(PixelContainer::from_data(width, height, data))
```

---

## API

```rust
pub struct BmpCodec;

impl ImageCodec for BmpCodec {
    fn mime_type(&self) -> &'static str {
        "image/bmp"
    }

    fn encode(&self, container: &PixelContainer) -> Vec<u8>;
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String>;
}

/// Convenience wrapper: encode a PixelContainer to BMP bytes.
pub fn encode_bmp(container: &PixelContainer) -> Vec<u8> {
    BmpCodec.encode(container)
}

/// Convenience wrapper: decode BMP bytes into a PixelContainer.
pub fn decode_bmp(bytes: &[u8]) -> Result<PixelContainer, String> {
    BmpCodec.decode(bytes)
}
```

---

## Error Cases

| Condition | Error message |
|-----------|---------------|
| File shorter than 54 bytes | `"BMP: file too short"` |
| Magic bytes != 'BM' | `"BMP: invalid magic"` |
| `biBitCount` != 32 | `"BMP: unsupported bit depth {n}, only 32-bit BGRA supported"` |
| `biCompression` != 0 | `"BMP: unsupported compression {n}, only BI_RGB (0) supported"` |
| Pixel data truncated | `"BMP: pixel data truncated"` |

---

## Round-Trip Property

For any valid `PixelContainer` `p`:

```rust
let encoded = BmpCodec.encode(&p);
let decoded  = BmpCodec.decode(&encoded).unwrap();
assert_eq!(p.width,  decoded.width);
assert_eq!(p.height, decoded.height);
assert_eq!(p.data,   decoded.data);
```

Every encode/decode cycle must preserve all RGBA values exactly.

---

## Crate Layout

```
code/packages/rust/image-codec-bmp/
├── Cargo.toml    # depends on pixel-container only
├── src/
│   └── lib.rs
├── BUILD
├── README.md
└── CHANGELOG.md
```

Dependencies: `pixel-container` only. No compression library needed.
