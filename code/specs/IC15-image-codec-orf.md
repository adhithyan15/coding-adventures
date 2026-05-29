# IC15 — Olympus ORF Image Codec

**Specification version**: 0.1  
**Status**: Draft  
**Depends on**: IC00 (pixel-container), IC09 (image-codec-tiff)  
**Implements**: Olympus RAW Format (ORF)

---

## 1. Overview

ORF (Olympus RAW Format) is the proprietary RAW format used in Olympus (now
OM System) interchangeable-lens cameras since the E-1 (2003). ORF is a TIFF
6.0 container with Olympus-specific MakerNote extensions.

**Key properties**:

- **Container**: TIFF 6.0; can be either little-endian (II) or big-endian (MM)
- **Identification**: Make = "OLYMPUS IMAGING CORP." or "OLYMPUS CORPORATION"
  or "OM Digital Solutions" (newer); file extension .orf
- **Magic**: TIFF magic (II/42 or MM/42) + Olympus-specific IFD0 Make tag
  The first two bytes of the TIFF may be `II` **or** `IIRO` (`0x4949 0x524F`)
  for Olympus's non-standard variant — treat "IIROxxxxxxx" as little-endian TIFF
- **Raw data location**: IFD0 (directly in main IFD) or Sub-IFD (tag 330)
- **Compression**:
  - Uncompressed 12-bit (Compression = 1)
  - Olympus compressed (Compression = 32767): proprietary 12-bit RLE
- **Pixel depth**: 12-bit (all supported models)
- **Bayer pattern**: mostly RGGB; some models use GRBG
- **Sensor size**: Micro Four Thirds (17.3×13.0mm); full-frame sensors not used

---

## 2. File Structure

```
Typical ORF layout (E-M1, E-M5 style):

IFD0:
  Make = "OLYMPUS CORPORATION"
  Model = camera model string
  SubIFDs (tag 330) → [sub-IFD-0]
  Exif IFD → Olympus MakerNote

SubIFD0: full-resolution CFA
  ImageWidth, ImageLength = sensor dimensions
  BitsPerSample = 12
  Compression = 1 (uncompressed) or 32767 (Olympus compressed)
  PhotometricInterpretation = 32803 (CFA)
  CFAPattern = [0,1,1,2] or [1,0,2,1] depending on model
  StripOffsets[0], StripByteCounts[0]

Older ORF (E-1, E-500):
  Raw data directly in IFD0 (no SubIFDs)
```

---

## 3. Olympus Compressed RAW (Compression = 32767)

Olympus uses a proprietary 12-bit RLE compression:

```
Each row is independently compressed:
  Read byte header per row — byte count for that row

Within a compressed row:
  Read a bit stream (MSB-first):
  "NBITS" flag (variable): determines how many bits the next value uses
  Value: signed delta from previous pixel (DPCM predictor = left)

The NBITS encoding uses a pseudo-prefix code table:
  Code length    Meaning
  0              delta = 0 (repeated pixel)
  10             delta in [-1, +1], 2-bit value follows
  110            delta in [-3, +3], 3-bit value follows
  1110           delta in [-7, +7], 4-bit value follows
  11110          delta in [-15, +15], 5-bit value follows
  111110         delta in [-31, +31], 6-bit value follows
  1111110        delta in [-63, +63], 7-bit value follows
  11111110       delta in [-127, +127], 8-bit value follows
  111111110      delta in [-255, +255], 9-bit value follows
  1111111110     delta in [-511, +511], 10-bit value follows
  11111111110    delta in [-1023, +1023], 11-bit value follows
  111111111110   full 12-bit value follows (reset / long jump)
```

For **uncompressed ORF** (Compression = 1):

```
12-bit little-endian packing (same as Canon CR2 except byte order):
  byte0 = p0[7:0]          (low 8 bits of p0)
  byte1 = (p0[11:8]) | (p1[3:0] << 4)
  byte2 = p1[11:4]

Wait — actually Olympus uses a variant:
  Every 2 pixels packed in 3 bytes, big-endian within pixels:
  byte0 = p0[11:4]
  byte1 = (p0[3:0] << 4) | p1[11:8]
  byte2 = p1[7:0]
```

The exact byte packing order varies by model. For v0.1, target the most
common variant (big-endian 12-bit packing as above).

---

## 4. Olympus MakerNote

The Olympus MakerNote is at Exif tag 0x927C. Format:

```
0   "OLYMPUS\0" or "OLYMP\0" — 8-byte magic
8   u16: IFD version (0x0100 typical)
10  IFD data (standard TIFF IFD, absolute offsets from start of file)
```

Key Olympus MakerNote tags:

| Tag    | Name                  | Type     | Description                           |
|--------|-----------------------|----------|---------------------------------------|
| 0x0100 | ThumbnailImage        | UNDEFINED| Embedded thumbnail JPEG               |
| 0x0200 | SpecialMode           | LONG[]   | Shooting mode flags                   |
| 0x0202 | Quality               | SHORT    | Quality setting                       |
| 0x0203 | Macro                 | SHORT    | Macro mode flag                       |
| 0x0204 | DigitalZoom           | RATIONAL | Digital zoom factor                   |
| 0x0207 | SoftwareRelease       | ASCII    |                                       |
| 0x0208 | PictureInfo           | ASCII    | Camera settings dump                  |
| 0x0209 | CameraID              | BYTE     | Camera serial number bytes            |
| 0x100B | FlashBias             | SRATIONAL|                                       |
| 0x1017 | RedBalance            | RATIONAL | Red WB multiplier (relative to green) |
| 0x1018 | BlueBalance           | RATIONAL | Blue WB multiplier (relative to green)|
| 0x2010 | Equipment             | UNDEFINED| Lens + body info sub-IFD              |
| 0x2020 | CameraSettings        | UNDEFINED| Camera settings sub-IFD               |
| 0x2040 | RawDevelopment        | UNDEFINED| Raw processing settings sub-IFD       |

### 4.1 White Balance

Tags 0x1017 (RedBalance) and 0x1018 (BlueBalance) give WB multipliers as
RATIONAL values relative to green. White balance application:

```rust
let wb = [red_balance as f64, 1.0, blue_balance as f64];
```

---

## 5. Colour Pipeline

```
1. Read 12-bit Bayer data (uncompressed packed or Olympus compressed)
2. Subtract black level (typically 256 for 12-bit ORF; model-specific)
3. Clip to [0, WhiteLevel] (typically 4095)
4. Normalize to [0.0, 1.0]
5. Bilinear Bayer demosaicing (RGGB or GRBG per CFAPattern)
6. Apply WB from MakerNote tags 0x1017 / 0x1018
7. Apply camera-to-sRGB colour matrix
8. sRGB gamma curve
9. Clip and convert to u8 RGBA (A = 255)
```

### 5.1 Colour Matrix (Hardcoded)

```
// Olympus E-M1 Mark II (representative):
[[ 1.476, -0.490, 0.014],
 [-0.254,  1.619, -0.365],
 [ 0.069, -0.497,  1.428]]
```

---

## 6. API

```rust
pub fn decode_orf(bytes: &[u8]) -> Result<PixelContainer, String>;
pub fn encode_orf(pixels: &PixelContainer) -> Vec<u8>;  // minimal for tests

pub struct OrfCodec;
impl paint_instructions::ImageCodec for OrfCodec {
    fn mime_type(&self) -> &'static str { "image/x-olympus-orf" }
    fn encode(&self, pixels: &PixelContainer) -> Vec<u8>;
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String>;
}

pub const VERSION: &str = "0.1.0";
```

---

## 7. Crate Layout

```
image-codec-orf/
  Cargo.toml        (deps: pixel-container, paint-instructions, image-codec-tiff)
  BUILD
  README.md
  CHANGELOG.md
  src/
    lib.rs
    makernote.rs      (Olympus MakerNote parser)
    compressed.rs     (Olympus 32767 RLE decoder)
    uncompressed.rs   (12-bit packed pixel reader)
    color_matrices.rs (Olympus per-model matrices)
    color.rs          (WB + matrix + gamma)
    decoder.rs        (top-level decode_orf: find CFA IFD)
    encoder.rs        (minimal test encoder)
```

---

## 8. Test Strategy (≥95% coverage target)

| Category                            | Tests |
|-------------------------------------|-------|
| ORF identification (Make tag)       | 1     |
| IFD0 vs SubIFD raw location         | 1     |
| 12-bit uncompressed unpack          | 2     |
| Olympus compressed decode (basic)   | 2     |
| MakerNote WB parsing                | 1     |
| Colour pipeline round-trip          | 1     |
| GRBG pattern support                | 1     |
| Error: not an ORF                   | 1     |
| MIME type + codec trait             | 1     |
| **Total**                           | **11**|

---

## 9. References

- dcraw.c `olympus_load_raw()` — reference implementation
- LibRaw Olympus decoders
- Exiv2 Olympus tag database — https://exiv2.org/tags-olympus.html
- rawspeed Olympus support
