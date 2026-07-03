# IC11 — Canon CR2 Image Codec

**Specification version**: 0.1  
**Status**: Draft  
**Depends on**: IC00 (pixel-container), IC09 (image-codec-tiff)  
**Implements**: Canon RAW 2 (CR2) format

---

## 1. Overview

CR2 (Canon RAW Version 2) is Canon's proprietary RAW format introduced in 2004
with the EOS 20D. It replaced CR1/CRW and was itself replaced by CR3 (ISO
BMFF-based, 2018) in the EOS R mirrorless line. CR2 is used by Canon DSLRs
produced from 2004 to ~2018 (EOS 20D through EOS 77D, 800D, 9000D).

**Key properties**:

- **Container**: TIFF 6.0 with Canon MakerNote extensions
- **Magic**: II (little-endian), TIFF magic 42, followed at offset 8 by "CR\x02\x00"
  (the CR2 signature: bytes 8–11 = 0x43 0x52 0x02 0x00)
- **Raw data location**: IFD3 (fourth IFD) holds the full-resolution sensor data
- **Raw compression**: lossless JPEG (TIFF Compression = 6 or 7) using a
  Canon-specific lossless JPEG variant (2–4 components, Huffman-coded)
- **Bayer pattern**: RGGB (most common), GRBG, GBRG, or BGGR depending on model
- **Pixel depth**: 14-bit (most models); stored in lossless JPEG as 14-bit
- **MakerNote**: Canon-specific tag 0x927C in ExifIFD; contains white balance,
  colour data, sensor info, and camera settings

---

## 2. File Structure

```
Offset  Content
0       "II" (little-endian)
2       0x002A (TIFF magic)
4       IFD0 offset (typically 0x10 = 16)
8       "CR" (0x43 0x52) — CR2 signature bytes
10      CR2 version (0x02 0x00 = version 2.0)
12      IFD3 strip offset high word (0x0000 typically)
14      IFD3 strip offset low word

IFD0:  JPEG thumbnail + camera metadata tags
IFD1:  Not present or additional reduced-size image
IFD2:  Not present or reduced-size raw
IFD3:  Full-resolution CFA sensor data (lossless JPEG strips)
```

The four IFDs are linked in a chain from IFD0. IFD3 contains:

```
ImageWidth  = full sensor width (including masked pixels)
ImageLength = full sensor height
Compression = 6 (old-JPEG, lossless) — each strip is one lossless JPEG
StripOffsets[0] = file offset of the single lossless JPEG strip
StripByteCounts[0] = byte count of that strip
```

**Important**: CR2 uses exactly one strip for the full image, stored as a
lossless JPEG with the JPEG restart marker interval set to the image width
(each row is one restart interval — this is the "JPEG scan" row encoding).

---

## 3. Lossless JPEG in CR2

CR2 uses a lossless JPEG (SOF3 marker) with the following characteristics:

- **SOF3**: Start of Frame (lossless, sequential, Huffman)
- **Components**: 2 or 4 (for the Bayer mosaic, two or four Bayer channels
  are interleaved — one CR2 "component" per Bayer column offset)
- **Precision**: 14 bits per component
- **Prediction**: JPEG predictor 1 (Ra = left) or 7 (Ra + Rb - Rc) depending on
  the model
- **Restart intervals**: Each row of Bayer data forms one restart interval

### 3.1 Decoding Lossless JPEG (SOF3)

```
1. Parse JPEG markers: SOI, APP*, DQT (ignored for lossless), SOF3, DHT, SOS, EOI
2. SOF3 gives: precision, height, width, component count
3. DHT gives: Huffman tables for each component (DC only — lossless uses DC pred)
4. SOS: decode rows using DPCM prediction

Prediction formula (predictor 1 — left):
  decoded_value = huffman_decoded_difference + left_neighbour
  left_neighbour = 0 at start of restart interval

Restart markers (0xFFD0–0xFFD7) reset the predictor to 0 for the next row.
```

### 3.2 Reassembling the Bayer Grid

With 2 components (most CR2 files):

```
Component 0 encodes the even columns of the Bayer row
Component 1 encodes the odd columns of the Bayer row

Interleaved output per scan row:
  pixel[row][0] = component_0[0]
  pixel[row][1] = component_1[0]
  pixel[row][2] = component_0[1]
  pixel[row][3] = component_1[1]
  ...
```

The Bayer pattern from `CFAPattern` / MakerNote tells which channels are
R, G1, G2, B.

---

## 4. Canon MakerNote Tags

The Canon MakerNote is at Exif IFD tag 0x927C. It is itself an IFD (no TIFF
header — starts directly with entry count). All values are little-endian.
Key sub-tags:

| Tag    | Name                 | Type    | Description                             |
|--------|----------------------|---------|-----------------------------------------|
| 0x0001 | CanonCameraSettings  | SHORT[] | Camera mode, AE/AF, etc.                |
| 0x0002 | CanonFocalLength     | SHORT[] | Focal length data                       |
| 0x0004 | CanonShotInfo        | SHORT[] | ISO, shutter, aperture                  |
| 0x0007 | CanonPanoramaInfo    | SHORT[] |                                         |
| 0x0010 | CanonModelID         | LONG    | Camera model identifier                 |
| 0x0024 | CanonAFInfo2         | SHORT[] | AF point data                           |
| 0x0029 | CanonFileInfo        | SHORT[] |                                         |
| 0x0100 | CanonImageType       | ASCII   | "Canon EOS XXD" etc.                    |
| 0x0101 | CanonFirmwareVersion | ASCII   |                                         |
| 0x0102 | FileNumber           | LONG    |                                         |
| 0x0153 | ColorData            | SHORT[] | White balance and colour matrix data    |

### 4.1 White Balance from ColorData

Canon stores white balance in the `ColorData` tag. The layout varies by
camera generation (indexed by CanonModelID ranges). A simplified extraction:

```
// ColorData versions 1–9 (most DSLR models):
// The "as-shot" multipliers are at fixed offsets within the SHORT array.
// Rather than implementing all 9 versions, use the fallback:
// AsShotNeutral from Exif WB tags or default D65 white point [1.0, 1.0, 1.0]
```

For v0.1, use Exif `LightSource` and `WB_RGBLevels` (if present) or D65
identity white balance. A complete ColorData parser is future work.

### 4.2 Colour Matrix

Canon does not embed a colour matrix in CR2. For v0.1, use a hardcoded
approximate camera-to-sRGB matrix derived from dcraw / LibRaw:

```
// Generic Canon DSLR (EOS 5D-era) approximate matrix:
// camera RGB → linear sRGB (D65 adapted)
[[ 1.901824, -0.972035, 0.070223],
 [-0.229410,  1.659384, -0.429974],
 [ 0.042003, -0.519400,  1.477397]]
```

Each codec implementation should embed a small lookup table of model-specific
matrices keyed on `CanonModelID`. For models not in the table, use the generic
matrix above.

---

## 5. Colour Pipeline

```
1. Decode lossless JPEG strip from IFD3 → 14-bit Bayer grid
2. Apply black level: subtract BlackLevel (MakerNote or hardcoded ~2047)
3. Clip to [0, WhiteLevel - BlackLevel] where WhiteLevel ~ 15383
4. Normalize to [0.0, 1.0]
5. Bilinear Bayer demosaicing (RGGB or model-specific pattern)
6. Apply white balance multipliers [wbR, 1.0, wbB]
7. Apply camera-to-sRGB colour matrix (model lookup or generic)
8. sRGB gamma curve
9. Clip and convert to u8 RGBA (A = 255)
```

---

## 6. API

```rust
pub fn decode_cr2(bytes: &[u8]) -> Result<PixelContainer, String>;
pub fn encode_cr2(pixels: &PixelContainer) -> Vec<u8>;  // minimal test encoder only

pub struct Cr2Codec;
impl paint_instructions::ImageCodec for Cr2Codec {
    fn mime_type(&self) -> &'static str { "image/x-canon-cr2" }
    fn encode(&self, pixels: &PixelContainer) -> Vec<u8>;
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String>;
}

pub const VERSION: &str = "0.1.0";
```

---

## 7. Crate Layout

```
image-codec-cr2/
  Cargo.toml        (deps: pixel-container, paint-instructions, image-codec-tiff)
  BUILD
  README.md
  CHANGELOG.md
  src/
    lib.rs
    lossless_jpeg.rs  (SOF3 lossless JPEG decoder)
    makernote.rs      (Canon MakerNote IFD parser)
    color_matrices.rs (hardcoded per-model matrices keyed by CanonModelID)
    bayer.rs          (bilinear demosaicing — shared with other RAW crates)
    color.rs          (WB + matrix + gamma pipeline)
    decoder.rs        (top-level: find IFD3, decode, colour-process)
    encoder.rs        (minimal synthetic CR2 for round-trip tests)
```

---

## 8. Test Strategy (≥95% coverage target)

| Category                           | Tests |
|------------------------------------|-------|
| CR2 signature detection            | 1     |
| IFD3 location                      | 1     |
| Lossless JPEG decode (2-component) | 2     |
| Bayer reassembly (RGGB)            | 1     |
| MakerNote parsing                  | 1     |
| Colour pipeline (WB + matrix)      | 2     |
| Round-trip (synthetic CR2)         | 1     |
| Error: not a CR2 file              | 1     |
| Error: truncated lossless JPEG     | 1     |
| MIME type + codec trait            | 1     |
| **Total**                          | **12**|

---

## 9. References

- Laurent Clévy, "Inside Canon's CR2 files" — https://lclevy.free.fr/cr2/
- dcraw.c by Dave Coffin — canonical reference decoder (GPL)
- LibRaw source — https://github.com/LibRaw/LibRaw
- Exiv2 Canon tag database — https://exiv2.org/tags-canon.html
