# IC16 — Panasonic RW2 Image Codec

**Specification version**: 0.1  
**Status**: Draft  
**Depends on**: IC00 (pixel-container)  
**Implements**: Panasonic RAW 2 (RW2) format

---

## 1. Overview

RW2 (RAW version 2) is Panasonic's proprietary RAW format used in Lumix cameras
since the GH1 (2009). RW2 replaced the older Panasonic RAW (.raw) format. Like
Fujifilm RAF, RW2 has its own file header structure that is **not** standard
TIFF, though it does embed TIFF-like structures internally.

**Key properties**:

- **Container**: Custom RW2 header + embedded JPEG thumbnail + raw pixel data
- **Magic**: `IIU\0` (bytes 0–3: 0x49 0x49 0x55 0x00) — a modified TIFF magic
  where the "II" indicates little-endian and "U" (0x55 = 85) replaces the TIFF
  version byte 42 (0x2A)
- **IFD structure**: after the custom header, an IFD is located at offset 8
  (standard TIFF IFD format, little-endian)
- **Raw data**: packed 12-bit or 16-bit pixels at a fixed offset in the file
- **Compression**: uncompressed packed (12 or 16 bit) OR Panasonic lossless (v5+)
- **Pixel depth**: 12-bit (most Lumix models); 16-bit (rare)
- **Bayer pattern**: RGGB (micro-4/3 and S-series bodies)
- **White balance**: in the IFD as Panasonic private tags
- **Active area**: from Panasonic private tag 0x002E (`SensorTopBorder`,
  `SensorLeftBorder`, `SensorBottomBorder`, `SensorRightBorder`)

---

## 2. File Structure

```
Offset  Size  Field
0       2     Byte order: "II" (0x4949) — always little-endian for RW2
2       2     Version: 0x0055 (85) — NOT 42; this is the RW2 discriminator
4       4     Offset of first IFD (usually 8)

IFD at offset 8: standard TIFF IFD (entry_count + 12-byte entries + next_ifd)
  Contains camera metadata and Panasonic private tags
  One entry points to raw pixel data via StripOffsets / custom offset tag
```

The IFD usually contains:

| Tag    | Name                | Notes                                           |
|--------|---------------------|-------------------------------------------------|
| 0x0001 | (Panasonic) PanasonicRawVersion | u8[4] version bytes                 |
| 0x0002 | SensorWidth         | Full sensor pixel width                        |
| 0x0003 | SensorHeight        | Full sensor pixel height                       |
| 0x0004 | SensorTopBorder     | Top crop border in sensor pixels               |
| 0x0005 | SensorLeftBorder    | Left crop border                               |
| 0x0006 | SensorBottomBorder  | Bottom crop border                             |
| 0x0007 | SensorRightBorder   | Right crop border                              |
| 0x0011 | RedBalance          | WB red/green ratio × 256 (u16)                 |
| 0x0012 | BlueBalance         | WB blue/green ratio × 256 (u16)                |
| 0x0022 | ImageWidth          | Actual image width after crop                  |
| 0x0023 | ImageHeight         | Actual image height after crop                 |
| 0x0024 | ImageDepth          | Bits per pixel (12 or 16)                      |
| 0x002E | JpegFromRaw         | Offset and size of embedded JPEG thumbnail     |
| 0x008C | (Makernote IFD offset)                                            |
| 0x0097 | (Raw data strip offset)                                           |

The raw pixel data offset is obtained from tag 0x0097 or via a Panasonic-specific
mechanism described in §3.

---

## 3. Raw Pixel Format

### 3.1 Standard 12-bit Packed (Most Lumix Models)

```
12-bit little-endian packing (2 pixels per 3 bytes):
  byte0 = p0[7:0]           (low 8 bits)
  byte1 = p0[11:8] | (p1[3:0] << 4)
  byte2 = p1[11:4]

Whole rows are packed; row size = ceil(SensorWidth * 12 / 8) bytes
```

Width includes masked (optical black) columns. The active image area is
determined by SensorTopBorder/SensorLeftBorder/SensorBottomBorder/SensorRightBorder.

### 3.2 Panasonic Lossless (v5+ cameras, GH5/S1/S5 etc.)

Panasonic introduced lossless compression in the GH5 (2017). The format uses
a row-by-row variable-length scheme similar to Sony ARW2:

```
For each row:
  u32 (LE): byte length of this row's compressed data
  Compressed data bytes

Within compressed data (MSB-first bit stream):
  Prefix code determines number of bits for delta:
    0              → delta = 0
    10 + N bits    → delta ∈ [-2^N+1, 2^N-1], N determined by prefix
  DPCM prediction: pixel = delta + left_neighbour
  Restart at each row boundary
```

For v0.1, detect lossless compression by checking if the strip byte count
is less than the uncompressed size. If compressed, decode with the scheme above.
If the exact decompressor is unavailable, return an informative Err rather than
producing garbage output.

---

## 4. White Balance

From IFD tags:
- Tag 0x0011: `RedBalance` = (R/G) × 256 as u16
- Tag 0x0012: `BlueBalance` = (B/G) × 256 as u16

```rust
let wb = [
    red_balance as f64 / 256.0,
    1.0,
    blue_balance as f64 / 256.0,
];
```

---

## 5. Colour Pipeline

```
1. Read 12-bit packed (or lossless) pixels for the full sensor area
2. Crop to active area using sensor border tags
3. Subtract black level (typically 240 for 12-bit RW2)
4. Clip to [0, WhiteLevel - BlackLevel] (typically 4095 - 240 = 3855)
5. Normalize to [0.0, 1.0]
6. Bilinear Bayer demosaicing (RGGB)
7. Apply white balance [wbR, 1.0, wbB] from tags 0x0011/0x0012
8. Apply camera-to-sRGB colour matrix (per-model lookup or generic)
9. sRGB gamma curve
10. Clip and convert to u8 RGBA (A = 255)
```

### 5.1 Colour Matrix (Hardcoded)

```
// Panasonic Lumix GH5 (representative):
[[ 1.512, -0.518, 0.006],
 [-0.202,  1.590, -0.388],
 [ 0.055, -0.413,  1.358]]
```

---

## 6. API

```rust
pub fn decode_rw2(bytes: &[u8]) -> Result<PixelContainer, String>;
pub fn encode_rw2(pixels: &PixelContainer) -> Vec<u8>;  // minimal for tests

pub struct Rw2Codec;
impl paint_instructions::ImageCodec for Rw2Codec {
    fn mime_type(&self) -> &'static str { "image/x-panasonic-rw2" }
    fn encode(&self, pixels: &PixelContainer) -> Vec<u8>;
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String>;
}

pub const VERSION: &str = "0.1.0";
```

---

## 7. Crate Layout

```
image-codec-rw2/
  Cargo.toml        (deps: pixel-container, paint-instructions)
  BUILD
  README.md
  CHANGELOG.md
  src/
    lib.rs
    header.rs         (RW2 magic check + IFD parse)
    unpack.rs         (12-bit LE packed pixel reader)
    lossless.rs       (Panasonic lossless decoder; returns Err for unknowns)
    color_matrices.rs (Panasonic per-model matrices)
    color.rs          (WB + matrix + gamma)
    decoder.rs        (top-level decode_rw2)
    encoder.rs        (minimal test encoder)
```

---

## 8. Test Strategy (≥95% coverage target)

| Category                             | Tests |
|--------------------------------------|-------|
| RW2 magic detection (IIU\0)          | 1     |
| IFD parsing (private tags)           | 2     |
| 12-bit LE unpack                     | 2     |
| Sensor border crop                   | 1     |
| White balance from tags              | 1     |
| Colour pipeline round-trip           | 1     |
| Lossless: returns Err (v0.1)         | 1     |
| Error: not an RW2 file               | 1     |
| MIME type + codec trait              | 1     |
| **Total**                            | **11**|

---

## 9. References

- dcraw.c `panasonic_load_raw()` — reference
- LibRaw Panasonic decoders — https://github.com/LibRaw/LibRaw
- Exiv2 Panasonic tag database — https://exiv2.org/tags-panasonic.html
- rawspeed Panasonic support — https://github.com/darktable-org/rawspeed
- "RW2 format notes" — various reverse-engineering blog posts
