# IC14 — Fujifilm RAF Image Codec

**Specification version**: 0.1  
**Status**: Draft  
**Depends on**: IC00 (pixel-container)  
**Implements**: Fujifilm RAW (RAF) format

---

## 1. Overview

RAF (RAW image Format) is Fujifilm's proprietary RAW format. Unlike the other
RAW formats in this series, RAF does **not** use a TIFF container — it has its
own custom binary structure. RAF is notable for two reasons:

1. **X-Trans sensor**: Fujifilm's flagship cameras (X-Pro, X-T, X100 series)
   use an X-Trans color filter array with a 6×6 pattern instead of the standard
   2×2 Bayer pattern. This requires a different demosaicing algorithm.
2. **Classic Bayer sensors**: Compact cameras (FinePix S/F series) use a
   standard RGGB Bayer pattern — the same demosaicing as other RAW codecs.

This spec implements RAF decoding for:
- **Classic Bayer RAF** (FinePix S/F series, compact cameras): RGGB bilinear
- **X-Trans RAF** (X-Pro, X-T, X-E, X100 series): 6×6 pattern with custom
  demosaicing (simplified bilinear for v0.1)

**Key properties**:

- **Container**: Custom RAF header, embedded JPEG thumbnail, embedded metadata
- **Magic**: "FUJIFILMCCD-RAW " (16 bytes at offset 0)
- **Raw data**: packed 12/14-bit pixels at a fixed offset given in the header
- **Compression**: uncompressed (packed bits); some models use lossless JPEG
- **White balance**: embedded in header block as RGB multipliers
- **Active area**: embedded in header

---

## 2. File Structure

```
RAF file layout
───────────────
Offset  Size    Field
0       16      Magic: "FUJIFILMCCD-RAW " (note trailing space)
16      4       Format version: "0100", "0101", "0200", "0201" (ASCII)
20      8       Camera model ID (ASCII, NUL-padded)
28      32      Camera model string (ASCII, NUL-padded)
60      4       Directory version: "0100" or similar (ASCII)
64      20      Unknown / reserved
84      4       JPEG offset (u32 BE): offset of embedded full-quality JPEG
88      4       JPEG length (u32 BE): byte count of embedded JPEG
92      4       CFA header offset (u32 BE)
96      4       CFA header length (u32 BE)
100     4       CFA offset (u32 BE): offset of raw pixel data
104     4       CFA length (u32 BE): byte count of raw pixel data
108     4       Second CFA offset (u32 BE) — used in some dual-pixel modes
112     4       Second CFA length (u32 BE)
```

All multi-byte integers in the RAF outer header are **big-endian**.

### 2.1 CFA Header

The CFA header (at offset = header[92], length = header[96]) is a
variable-length block describing the image geometry:

```
CFA Header structure:
  Tag blocks, each:
    u16 (BE) tag
    u16 (BE) byte_count
    [byte_count bytes] value

Known CFA header tags:
  0x0100  Image size:     u16 width, u16 height (BE)
  0x0110  Raw image size: u16 raw_width, u16 raw_height (BE)
  0x0111  CFA pattern:    u8[4] — 0=R,1=G,2=B (or 6×6 for X-Trans)
  0x0130  WB for auto:    u32[3] — [R, G, B] multipliers (little-endian in value)
  0x0131  WB for fine:    u32[3]
  0x0141  Black levels:   u32[4] — per CFA plane
  0x0142  White level:    u32[1]
```

X-Trans cameras set the CFA pattern tag to a 6×6 array (36 bytes).

---

## 3. Raw Pixel Data

### 3.1 Classic Bayer (12-bit packed, big-endian)

Most FinePix compact cameras and older DSLRs:

```
12-bit big-endian packing (2 pixels per 3 bytes):
  byte0 = p0[11:4]
  byte1 = (p0[3:0] << 4) | p1[11:8]
  byte2 = p1[7:0]
```

Width is padded to a multiple of 16 pixels.

### 3.2 X-Trans (12/14-bit)

X-Trans cameras store pixels in the same 12-bit or 14-bit packed format,
but the colour filter pattern is 6×6 instead of 2×2:

```
Standard X-Trans pattern (X-Pro1, X-T1, X-T2):
  G B G G R G
  R G R B G B
  G B G G R G
  G R G G B G
  B G B R G R
  G R G G B G
```

The 6×6 tile is decoded identically to Bayer (packed 12-bit), but demosaicing
must account for the irregular pattern.

---

## 4. Demosaicing

### 4.1 Classic Bayer (RGGB)

Standard bilinear Bayer demosaicing — identical to TIFF CFA demosaicing (§7
of IC09).

### 4.2 X-Trans Simplified Bilinear

For v0.1, use a simplified bilinear interpolation for X-Trans:

```
For each output pixel (r, c):
  Look up colour channel from 6×6 pattern table: ch = pattern[r%6][c%6]
  For missing R/G/B values, average the nearest same-channel neighbours
  within a 5×5 window (same border-replication rule as Bayer bilinear)
```

This produces noticeable colour fringing at edges (a known limitation of
bilinear for X-Trans). Full AHD or X-Trans-specific algorithms (as in
darktable or Rawtherapee) are not required for v0.1.

---

## 5. White Balance

CFA header tag 0x0130 (auto WB) contains raw multipliers [R, G, B].
Normalise to green = 1.0:

```rust
let g = wb[1] as f64;
let wb_norm = [wb[0] as f64 / g, 1.0, wb[2] as f64 / g];
```

---

## 6. Colour Pipeline

```
1. Read 12/14-bit packed pixels from CFA offset
2. Subtract black levels (from CFA header tag 0x0141 per plane)
3. Clip to [0, white_level - black_level]
4. Normalize to [0.0, 1.0]
5. Demosaicing (bilinear Bayer or X-Trans simplified bilinear)
6. Apply white balance from tag 0x0130
7. Apply camera-to-sRGB colour matrix
8. sRGB gamma curve
9. Clip and convert to u8 RGBA (A = 255)
```

### 6.1 Colour Matrix (Hardcoded)

```
// Fujifilm X-T2 (representative):
[[ 1.469, -0.491, 0.022],
 [-0.272,  1.559, -0.287],
 [ 0.050, -0.380,  1.330]]
```

---

## 7. API

```rust
pub fn decode_raf(bytes: &[u8]) -> Result<PixelContainer, String>;
pub fn encode_raf(pixels: &PixelContainer) -> Vec<u8>;  // minimal for tests

pub struct RafCodec;
impl paint_instructions::ImageCodec for RafCodec {
    fn mime_type(&self) -> &'static str { "image/x-fuji-raf" }
    fn encode(&self, pixels: &PixelContainer) -> Vec<u8>;
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String>;
}

pub const VERSION: &str = "0.1.0";
```

---

## 8. Crate Layout

```
image-codec-raf/
  Cargo.toml        (deps: pixel-container, paint-instructions)
  BUILD
  README.md
  CHANGELOG.md
  src/
    lib.rs
    header.rs         (magic check + outer header parser)
    cfa_header.rs     (CFA header tag block parser)
    unpack.rs         (12-bit packed pixel reader)
    xtrans.rs         (6×6 X-Trans pattern table + simplified bilinear demosaic)
    bayer.rs          (standard 2×2 bilinear demosaicing)
    color_matrices.rs (Fujifilm per-model matrices)
    color.rs          (WB + matrix + gamma)
    decoder.rs        (top-level decode_raf)
    encoder.rs        (minimal test encoder)
```

---

## 9. Test Strategy (≥95% coverage target)

| Category                              | Tests |
|---------------------------------------|-------|
| Magic byte detection                  | 1     |
| Outer header parse (offsets/lengths)  | 1     |
| CFA header tag parsing                | 3     |
| 12-bit unpack (big-endian)            | 2     |
| Bayer demosaic (classic RAF)          | 1     |
| X-Trans demosaic (simplified)         | 1     |
| White balance normalisation           | 1     |
| Colour pipeline round-trip            | 1     |
| Error: not a RAF file                 | 1     |
| Error: truncated header               | 1     |
| MIME type + codec trait               | 1     |
| **Total**                             | **14**|

---

## 10. References

- dcraw.c `fuji_load_raw()` and `xtrans_load_raw()` — reference implementation
- LibRaw Fujifilm decoders — https://github.com/LibRaw/LibRaw
- rawspeed Fujifilm support — https://github.com/darktable-org/rawspeed
- "Understanding X-Trans" by Iliah Borg — technical blog post (signal-estimator.com)
- exiftool RAF tag dump — https://exiftool.org/TagNames/Fujifilm.html
