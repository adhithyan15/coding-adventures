# IC13 — Sony ARW Image Codec

**Specification version**: 0.1  
**Status**: Draft  
**Depends on**: IC00 (pixel-container), IC09 (image-codec-tiff)  
**Implements**: Sony Alpha RAW (ARW) versions 1.0–3.0

---

## 1. Overview

ARW (Alpha RAW) is Sony's proprietary RAW format used in Sony Alpha DSLR and
mirrorless cameras since 2006. ARW is a TIFF 6.0 container with Sony-specific
MakerNote extensions. ARW comes in several versions:

- **ARW 1.0** (2006–2008): uncompressed or simple compressed; α100, α700
- **ARW 2.x** (2008–2018): Sony compressed RAW; α900, α99, A7 series I–III
- **ARW 3.0** (2018+): new compression; A7R IV and later

This spec targets ARW 1.0 and ARW 2.x (the dominant formats). ARW 3.0 is
flagged as "unsupported; return Err" in v0.1.

**Key properties**:

- **Container**: TIFF 6.0, little-endian (II)
- **Identification**: Make = "SONY", Model starts with "DSLR-" or "ILCE-" etc.
- **Raw location**: Sub-IFD via SubIFDs (tag 330) in IFD0
- **Compression**:
  - ARW 1.0: Compression = 32767 (Sony uncompressed, 12-bit packed)
  - ARW 2.x: Compression = 32767 (Sony compressed RAW v2)
- **Pixel depth**: 12-bit (ARW 1.0), 14-bit (ARW 2.x)
- **Bayer pattern**: RGGB (most bodies)
- **White balance**: in Sony MakerNote (tag 0x7300+ in Exif)
- **Active area**: stored in Sony MakerNote SonyModelID-indexed table or TIFF

---

## 2. File Structure

```
IFD0:
  Make = "SONY"
  Model = camera name
  SubIFDs (tag 330) → [sub-IFD-0, sub-IFD-1]
  Exif IFD → Sony MakerNote
  Compression = 6 (JPEG thumbnail in main IFD0 strips)
  StripOffsets, StripByteCounts → JPEG thumbnail

SubIFD-0: reduced-resolution preview
  Compression = 6 or 7 (JPEG)

SubIFD-1: full-resolution CFA data
  ImageWidth, ImageLength = full sensor dimensions
  BitsPerSample = 12 or 14
  Compression = 32767 (Sony)
  PhotometricInterpretation = 32803 (CFA)
  StripOffsets[0] = raw data offset
  StripByteCounts[0] = raw data byte count
```

---

## 3. Sony Compression Format (ARW 2.x)

Sony ARW 2.x uses a per-row variable-length compression scheme:

```
Each compressed row header (8 bytes):
  u16: width of this row (== ImageWidth)
  u16: byte length of this row's compressed data
  u32: reserved

Compressed row data uses 7-bit to 12-bit codes:
  - Codes are packed MSB-first
  - Short codes encode small deltas; long codes encode larger deltas
  - The lookup table has 128 entries (7-bit prefix):
      if prefix < 0x40: delta = prefix - 0x40 (signed 7-bit)
      else:             read more bits (code length determined by prefix)
  - Prediction: horizontal DPCM (pixel = delta + left_neighbour)
```

For **ARW 1.0** (Compression = 32767 but uncompressed):

```
12-bit big-endian packed (opposite byte order from Nikon):
  byte0 = p0[11:4]
  byte1 = (p0[3:0] << 4) | p1[11:8]
  byte2 = p1[7:0]
```

Distinguish ARW 1.0 from ARW 2.x via the Sony MakerNote `SonyModelID` or
by checking whether the strip byte count equals `ceil(width * height * 12 / 8)`
(uncompressed) or not.

---

## 4. Sony MakerNote

The Sony MakerNote is at Exif tag 0x927C. Format:

```
0  "SONY DSC \0\0\0" — 12-byte header
12  IFD start (standard TIFF IFD, relative to MakerNote start)
```

Key Sony MakerNote tags:

| Tag    | Name             | Type     | Description                              |
|--------|------------------|----------|------------------------------------------|
| 0x0102 | Quality          | LONG     | Image quality setting                    |
| 0x0104 | FlashExposureComp| SRATIONAL| Flash EV compensation                    |
| 0x0105 | Teleconverter    | LONG     | Teleconverter type                       |
| 0x0112 | WhiteBalance     | LONG     | WB mode (0=auto, 1=daylight, etc.)       |
| 0x0115 | ColorTemperature | LONG     | WB colour temperature in K               |
| 0x0116 | ColorCompensationFilter | LONG | Green/magenta balance              |
| 0x2001 | SonyMakerNote2   | UNDEFINED| Second MakerNote block (WB multipliers!) |
| 0x7300+ | (various)       | various  | Model-specific tags                      |

### 4.1 White Balance Extraction

Sony stores WB multipliers in `SonyMakerNote2` (tag 0x2001), a nested IFD.
The multipliers are at a model-specific offset within that block.

For v0.1: use a fixed D65 white balance (no model-specific decryption needed).
A future version can add per-model WB extraction from LibRaw's tables.

---

## 5. Colour Pipeline

```
1. Read compressed row data → 12/14-bit Bayer pixels per row
2. Subtract black level (typically 512 for 12-bit ARW 1.0; 200 for ARW 2.x)
3. Clip to [0, WhiteLevel] (4095 for 12-bit; 16383 for 14-bit)
4. Normalize to [0.0, 1.0]
5. Bilinear Bayer demosaicing (RGGB)
6. Apply white balance multipliers [wbR, 1.0, wbB]
7. Apply camera-to-sRGB colour matrix
8. sRGB gamma curve
9. Clip and convert to u8 RGBA (A = 255)
```

### 5.1 Colour Matrix (Hardcoded)

```
// Sony A7R II (representative mid-range matrix):
[[ 1.318, -0.398, 0.080],
 [-0.213,  1.586, -0.373],
 [ 0.047, -0.474,  1.427]]
```

---

## 6. API

```rust
pub fn decode_arw(bytes: &[u8]) -> Result<PixelContainer, String>;
pub fn encode_arw(pixels: &PixelContainer) -> Vec<u8>;  // minimal for tests

pub struct ArwCodec;
impl paint_instructions::ImageCodec for ArwCodec {
    fn mime_type(&self) -> &'static str { "image/x-sony-arw" }
    fn encode(&self, pixels: &PixelContainer) -> Vec<u8>;
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String>;
}

pub const VERSION: &str = "0.1.0";
```

---

## 7. Crate Layout

```
image-codec-arw/
  Cargo.toml        (deps: pixel-container, paint-instructions, image-codec-tiff)
  BUILD
  README.md
  CHANGELOG.md
  src/
    lib.rs
    makernote.rs      (Sony MakerNote parser)
    compressed.rs     (ARW 2.x row-compression decoder)
    uncompressed.rs   (12-bit ARW 1.0 packing)
    color_matrices.rs (Sony per-model matrices)
    color.rs          (WB + matrix + gamma)
    decoder.rs        (top-level decode_arw)
    encoder.rs        (minimal test encoder)
```

---

## 8. Test Strategy (≥95% coverage target)

| Category                           | Tests |
|------------------------------------|-------|
| ARW identification (Make=SONY)     | 1     |
| ARW 1.0 12-bit unpack              | 1     |
| ARW 2.x compressed row decode      | 2     |
| Sub-IFD discovery                  | 1     |
| Black level subtraction            | 1     |
| Colour pipeline round-trip         | 1     |
| ARW 3.0 returns Err (unsupported)  | 1     |
| Error: not an ARW file             | 1     |
| MIME type + codec trait            | 1     |
| **Total**                          | **10**|

---

## 9. References

- dcraw.c `sony_arw2_load_raw()` and `sony_arw_load_raw()` functions
- LibRaw Sony decoders — https://github.com/LibRaw/LibRaw/blob/master/src/decoders/
- Exiv2 Sony tag database — https://exiv2.org/tags-sony.html
- rawspeed Sony decoders — https://github.com/darktable-org/rawspeed
