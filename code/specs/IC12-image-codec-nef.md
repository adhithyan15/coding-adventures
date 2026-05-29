# IC12 — Nikon NEF Image Codec

**Specification version**: 0.1  
**Status**: Draft  
**Depends on**: IC00 (pixel-container), IC09 (image-codec-tiff)  
**Implements**: Nikon Electronic Format (NEF) RAW

---

## 1. Overview

NEF (Nikon Electronic Format) is Nikon's proprietary RAW format used across
all Nikon DSLRs and mirrorless cameras. NEF is a TIFF 6.0 container extended
with Nikon-specific MakerNotes. Unlike DNG, NEF has no published specification
and has been entirely reverse-engineered (primarily by dcraw, LibRaw, and
Exiv2).

**Key properties**:

- **Container**: TIFF 6.0, almost always little-endian (II)
- **Magic**: standard TIFF (II + 42); NEF has no distinct file magic beyond TIFF
  It is identified by the `Make` tag = "Nikon Corporation" and file extension
- **Raw data location**: IFD0 or a sub-IFD pointed to by SubIFDs (tag 330)
- **Compression**: 
  - Uncompressed 12/14-bit (Compression = 1)
  - Nikon lossless (Compression = 34713): custom Huffman + DPCM
  - Nikon lossy (Compression = 34713 with specific curve): lossy 12-bit
- **White balance**: stored in Nikon MakerNote, often XOR-encrypted with a key
  derived from the camera serial number and shutter count
- **Pixel depth**: 12-bit (older bodies), 14-bit (D300 and newer)
- **Bayer pattern**: RGGB (most bodies), GRBG (some crop sensors)
- **Tone curve**: linear or a proprietary Nikon tone curve embedded in MakerNote

---

## 2. File Structure

NEF files typically contain:

```
IFD0:
  Make = "Nikon Corporation"
  Model = "NIKON D<model>"
  SubIFDs (tag 330) → [IFD_preview, IFD_raw]
  Exif IFD → Nikon MakerNote
  Compression, StripOffsets, StripByteCounts (thumbnail JPEG)

SubIFD0 (via SubIFDs[0]):  reduced-size preview JPEG
  PhotometricInterpretation = 6 (YCbCr) or 2 (RGB)
  Compression = 6 (JPEG)

SubIFD1 (via SubIFDs[1]): full-resolution CFA data
  PhotometricInterpretation = 32803 (CFA)
  Compression = 1 (uncompressed) or 34713 (Nikon compressed)
  BitsPerSample = 12 or 14
  SamplesPerPixel = 1
  ImageWidth / ImageLength = full sensor size (including masked pixels)
  StripOffsets[0] = offset of raw pixel data
  StripByteCounts[0] = size of raw pixel data
```

**Sub-IFD discovery**: tag 330 (`SubIFDs`, type LONG) in IFD0 is an array of
offsets to sub-IFDs. The raw image sub-IFD has PhotometricInterpretation = 32803.

---

## 3. Nikon Compressed RAW (Compression = 34713)

Nikon's proprietary compression uses Huffman coding with 2D DPCM prediction.
Two variants exist:

### 3.1 Lossless Compressed NEF

```
1. Read compressed strip bytes
2. Parse Huffman table: a fixed 15-entry canonical table (embedded in raw data
   header or hardcoded per camera generation — see dcraw.c `nikon_decrypt`)
3. DPCM decode per row:
   delta = huffman_decode()
   pixel = delta + left_neighbour   // predictor = left (same as lossless JPEG)
   left_neighbour resets at each row start
4. Apply linearisation curve (if non-linear) from MakerNote LinearizationTable
```

The Huffman tables differ between camera generations (D70/D80/D90/D3x/D800
etc.). For v0.1, support the most common variant (used in D70, D80, D90,
D3000, D5000 series) using the hardcoded table from dcraw.

### 3.2 Uncompressed NEF

Strip bytes are packed 12-bit or 14-bit values, MSB-first, tightly packed:

```
12-bit packing (2 pixels per 3 bytes):
  byte0 = p0[11:4]
  byte1 = (p0[3:0] << 4) | p1[11:8]
  byte2 = p1[7:0]

14-bit packing (4 pixels per 7 bytes):
  byte0 = p0[13:6]
  byte1 = (p0[5:0] << 2) | p1[13:12]
  byte2 = p1[11:4]
  byte3 = (p1[3:0] << 4) | p2[13:10]
  byte4 = p2[9:2]
  byte5 = (p2[1:0] << 6) | p3[13:8]
  byte6 = p3[7:0]
```

---

## 4. Nikon MakerNote

The Nikon MakerNote is at Exif tag 0x927C. It has its own TIFF-like header:

```
0   "Nikon\0"          — 6 bytes
6   version (u16)      — 0x0100 or 0x0200 or 0x0210
8   byte order (u16)   — 0x4949 (II) or 0x4D4D (MM)
10  TIFF magic (u16)   — 42
12  IFD offset (u32)   — offset from start of MakerNote
```

After the header, the MakerNote is a standard TIFF IFD. Key tags:

| Tag    | Name                | Type     | Description                                  |
|--------|---------------------|----------|----------------------------------------------|
| 0x0001 | MakerNoteVersion    | UNDEFINED | Version string ("0210" etc.)                |
| 0x000B | SerialNumber        | ASCII    | Camera serial number (used for WB key)       |
| 0x001D | ShutterCount        | LONG     | Total shutter actuations (used for WB key)   |
| 0x0097 | ColorBalance        | UNDEFINED| Encrypted white balance data                 |
| 0x0099 | RawImageCenter      | SHORT    | [x, y] of image center (for rotation)       |
| 0x00A7 | ShutterCount2       | LONG     | Alternative shutter count                    |
| 0x00C7 | VignetteControl     | SHORT    | Vignette correction applied                  |

### 4.1 Encrypted White Balance (Tag 0x0097)

Nikon encrypts white balance data in some camera models using RC4:

```
key = f(serial_number, shutter_count)
encrypted_bytes = makernote[0x0097][offset..]
decrypted = RC4_decrypt(key, encrypted_bytes)
```

The key derivation and encryption offset vary by camera version (version byte
at offset 0 of tag 0x0097 data: 0x02, 0x03, 0x04 indicate different layouts).

For v0.1: if decryption fails or version is unknown, use a default D65
white balance (no correction). A complete implementation requires the per-model
key derivation tables from LibRaw.

### 4.2 Linearisation Table

Tag 0x0097 (after decryption) may contain a linearisation curve:
a 12-bit LUT mapping raw sensor values to linear values. Apply before
black-level subtraction.

---

## 5. Colour Pipeline

```
1. Read 12/14-bit Bayer data (uncompressed packed, or Nikon compressed)
2. Apply linearisation table if present (from MakerNote)
3. Subtract black level (typically 0 for 12-bit, 0 or small value for 14-bit;
   embedded in MakerNote or use model-specific default)
4. Clip to [0, WhiteLevel] (typically 4095 for 12-bit, 16383 for 14-bit)
5. Normalize to [0.0, 1.0]
6. Bilinear Bayer demosaicing
7. Apply white balance [wbR, 1.0, wbB] from decrypted MakerNote or D65 default
8. Apply camera-to-sRGB colour matrix (model-specific lookup or generic Nikon)
9. sRGB gamma curve
10. Clip and convert to u8 RGBA (A = 255)
```

### 5.1 Hardcoded Nikon Colour Matrix (Generic)

For models not in the lookup table, use the dcraw Nikon D70 matrix as a
reasonable approximation:

```
// Nikon D70 (representative of early DSLR era):
[[ 1.392, -0.418, 0.026],
 [-0.254,  1.614, -0.360],
 [ 0.068, -0.584,  1.516]]
```

---

## 6. API

```rust
pub fn decode_nef(bytes: &[u8]) -> Result<PixelContainer, String>;
pub fn encode_nef(pixels: &PixelContainer) -> Vec<u8>;  // minimal for tests

pub struct NefCodec;
impl paint_instructions::ImageCodec for NefCodec {
    fn mime_type(&self) -> &'static str { "image/x-nikon-nef" }
    fn encode(&self, pixels: &PixelContainer) -> Vec<u8>;
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String>;
}

pub const VERSION: &str = "0.1.0";
```

---

## 7. Crate Layout

```
image-codec-nef/
  Cargo.toml        (deps: pixel-container, paint-instructions, image-codec-tiff)
  BUILD
  README.md
  CHANGELOG.md
  src/
    lib.rs
    makernote.rs      (Nikon MakerNote parser + WB extraction)
    compressed.rs     (Nikon compressed RAW 34713 decoder)
    uncompressed.rs   (12-bit and 14-bit packed pixel reader)
    color_matrices.rs (per-model camera-to-sRGB matrices)
    color.rs          (WB + matrix + gamma pipeline)
    decoder.rs        (top-level: find CFA sub-IFD, decode, colour-process)
    encoder.rs        (minimal test encoder)
```

---

## 8. Test Strategy (≥95% coverage target)

| Category                             | Tests |
|--------------------------------------|-------|
| NEF identification (Make tag)        | 1     |
| Sub-IFD discovery                    | 1     |
| 12-bit uncompressed unpack           | 2     |
| 14-bit uncompressed unpack           | 1     |
| Nikon compressed decode (basic)      | 1     |
| MakerNote version detection          | 1     |
| WB fallback (D65) when no key        | 1     |
| Colour pipeline round-trip           | 1     |
| Error: not a NEF file                | 1     |
| Error: no CFA sub-IFD                | 1     |
| MIME type + codec trait              | 1     |
| **Total**                            | **12**|

---

## 9. References

- dcraw.c by Dave Coffin (GPL) — canonical reference, `nikon_decrypt()` function
- LibRaw — https://github.com/LibRaw/LibRaw
- Exiv2 Nikon tag database — https://exiv2.org/tags-nikon.html
- rawpy Python library (wraps LibRaw) — https://github.com/letmaik/rawpy
- "Nikon RAW Image Format" by Dave Coffin — http://cybercom.net/~dcoffin/dcraw/
