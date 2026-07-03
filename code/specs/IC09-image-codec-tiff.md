# IC09 — TIFF Image Codec

**Specification version**: 0.1  
**Status**: Draft  
**Depends on**: IC00 (pixel-container)  
**Implements**: TIFF 6.0 baseline + selected extensions (Adobe, Exif)

---

## 1. Overview

TIFF (Tagged Image File Format) is a flexible, extensible raster image container
developed by Aldus and now maintained by Adobe. Version 6.0 (1992) is the
canonical baseline. TIFF uses a linked list of **IFDs** (Image File Directories)
to describe one or more images, each IFD holding typed key-value tags that
describe dimensions, colour model, compression, and where to find pixel data.

**Key properties**:

- **Byte order**: either little-endian (`II`, Intel) or big-endian (`MM`,
  Motorola), declared in the first two bytes of the file
- **Tag-based extensibility**: any field is a (tag, type, count, value) tuple;
  unknown tags can be skipped without error
- **Multiple images**: IFDs form a singly-linked list; each IFD is one "page"
  or "sub-image"
- **Compression**: uncompressed, PackBits RLE, LZW, and JPEG are the baseline
  modes; this spec targets those four
- **Colour models**: BlackIsZero/WhiteIsZero (grayscale), RGB, Palette,
  YCbCr, and CFA (Bayer; PhotometricInterpretation = 32803)
- **Strip layout**: pixel data is split into horizontal strips, each with its
  own offset and byte-count arrays
- **Tile layout** (optional): pixel data split into rectangular tiles; tiles
  use TileOffsets / TileByteCounts instead of strips
- **Royalty-free**: TIFF 6.0 is a public standard with no known active patents

**Why TIFF matters for RAW codecs**:  
Canon CR2, Nikon NEF, Sony ARW, Olympus ORF, and Adobe DNG are all
TIFF-container files. The `image-codec-tiff` crate is the shared foundation for
all of those downstream codecs.

---

## 2. File Structure

```
TIFF file layout
────────────────
Offset  Size  Field
0       2     ByteOrder — 0x4949 ("II") = little-endian, 0x4D4D ("MM") = big-endian
2       2     Magic — 42 (0x002A) for Classic TIFF; 43 (0x002B) for BigTIFF (not supported here)
4       4     Offset of first IFD (IFD0)

IFD at offset N:
  N       2   Entry count (number of IFD entries)
  N+2     12  IFD entry 0
  N+14    12  IFD entry 1
  …           …
  N+2+12*k  4  Offset of next IFD (0 = no more IFDs)

Each IFD entry (12 bytes):
  0   2   Tag     — field identifier (see §3)
  2   2   Type    — data type code (see §3.1)
  4   4   Count   — number of values of that type
  8   4   ValueOffset — if (count * typeSize) ≤ 4: value stored inline, left-justified;
                        else: file offset of the value data
```

### 2.1 IFD Entry Data Types

| Code | Name       | Byte size | Description                          |
|------|------------|-----------|--------------------------------------|
| 1    | BYTE       | 1         | Unsigned 8-bit                       |
| 2    | ASCII      | 1         | 7-bit ASCII string, NUL-terminated   |
| 3    | SHORT      | 2         | Unsigned 16-bit                      |
| 4    | LONG       | 4         | Unsigned 32-bit                      |
| 5    | RATIONAL   | 8         | Two LONGs: numerator / denominator   |
| 6    | SBYTE      | 1         | Signed 8-bit                         |
| 7    | UNDEFINED  | 1         | Raw bytes (any content)              |
| 8    | SSHORT     | 2         | Signed 16-bit                        |
| 9    | SLONG      | 4         | Signed 32-bit                        |
| 10   | SRATIONAL  | 8         | Two SLONGs: numerator / denominator  |
| 11   | FLOAT      | 4         | IEEE 754 single-precision            |
| 12   | DOUBLE     | 8         | IEEE 754 double-precision            |

---

## 3. Baseline Tags

### 3.1 Required Tags (Baseline TIFF Reader)

| Tag  | Name                       | Type   | Notes                                        |
|------|----------------------------|--------|----------------------------------------------|
| 256  | ImageWidth                 | SHORT/LONG | Pixel columns                            |
| 257  | ImageLength                | SHORT/LONG | Pixel rows                               |
| 258  | BitsPerSample              | SHORT  | Bits per channel; one value per SamplesPerPixel |
| 259  | Compression                | SHORT  | See §4                                       |
| 262  | PhotometricInterpretation  | SHORT  | See §5                                       |
| 277  | SamplesPerPixel            | SHORT  | Number of channels (1=gray, 3=RGB, etc.)     |
| 278  | RowsPerStrip               | SHORT/LONG | Rows per strip; 2^32-1 = single strip    |
| 279  | StripByteCounts            | SHORT/LONG | Byte count of each compressed strip      |
| 273  | StripOffsets               | SHORT/LONG | File offset of each strip                |
| 284  | PlanarConfiguration        | SHORT  | 1=chunky (RGBRGB…), 2=planar (RRR…GGG…BBB…) |

### 3.2 Optional but Commonly Used Tags

| Tag  | Name                | Type     | Notes                                      |
|------|---------------------|----------|--------------------------------------------|
| 254  | NewSubfileType      | LONG     | 0=full image, 1=reduced, 2=single page     |
| 269  | DocumentName        | ASCII    |                                            |
| 270  | ImageDescription    | ASCII    |                                            |
| 271  | Make                | ASCII    | Camera manufacturer                        |
| 272  | Model               | ASCII    | Camera model                               |
| 282  | XResolution         | RATIONAL | Pixels per ResolutionUnit                  |
| 283  | YResolution         | RATIONAL |                                            |
| 296  | ResolutionUnit      | SHORT    | 1=no abs unit, 2=inch, 3=cm               |
| 305  | Software            | ASCII    |                                            |
| 306  | DateTime            | ASCII    | "YYYY:MM:DD HH:MM:SS"                      |
| 315  | Artist              | ASCII    |                                            |
| 320  | ColorMap            | SHORT    | Palette; 3*2^BitsPerSample entries (RGB)   |
| 338  | ExtraSamples        | SHORT    | 0=unassoc alpha, 1=premul alpha, 2=other   |
| 339  | SampleFormat        | SHORT    | 1=uint, 2=sint, 3=float, 4=undefined       |
| 322  | TileWidth           | SHORT/LONG | Tile width (if tile layout)              |
| 323  | TileLength          | SHORT/LONG | Tile height (if tile layout)             |
| 324  | TileOffsets         | LONG     | File offset of each tile                   |
| 325  | TileByteCounts      | LONG     | Byte count of each tile                    |
| 530  | YCbCrSubSampling    | SHORT    | [Hfactor, Vfactor] for YCbCr images       |
| 532  | ReferenceBlackWhite | RATIONAL | YCbCr black/reference white               |
| 34665| ExifIFD             | LONG     | Offset of Exif sub-IFD                     |
| 34853| GPSIFD              | LONG     | Offset of GPS sub-IFD                      |

### 3.3 CFA (Bayer) Tags — Used by RAW formats

| Tag   | Name                | Type     | Notes                                    |
|-------|---------------------|----------|------------------------------------------|
| 33421 | CFARepeatPatternDim | SHORT    | [rows, cols] of the CFA pattern tile     |
| 33422 | CFAPattern          | BYTE     | Pattern bytes: 0=R,1=G,2=B,3=Cyan,etc.  |

---

## 4. Compression Codecs

### 4.1 Uncompressed (Compression = 1)

Strip bytes are raw pixel data, packed at BitsPerSample bits per channel,
MSB-first within each byte, rows padded to byte boundaries.

### 4.2 PackBits (Compression = 32773)

Simple byte-level RLE used in many TIFF writers (macOS Preview, etc.):

```
Read a header byte h:
  if h == -128 (0x80):      nop (skip)
  if -127 ≤ h ≤ -1:         repeat next byte (1 - h) times
  if 0 ≤ h ≤ 127:           copy next (h + 1) literal bytes
Stop when decompressed size == expected row bytes.
```

### 4.3 LZW (Compression = 5)

LZW with a 12-bit code table and MSB-first bit packing (same algorithm as
GIF but with a different clear-code convention):

- Clear code = 2^BitsPerSample; End-of-information code = clear+1
- Variable-width codes start at BitsPerSample+1 bits, growing to 12 bits
- Horizontal differencing predictor (Predictor tag = 2) may be applied:
  `delta[x] = pixel[x] - pixel[x-1]` per channel, stored; decode = cumsum

### 4.4 JPEG (Compression = 7 — "New-Style" JPEG)

Each strip or tile contains a complete JPEG bitstream (SOI…EOI). Decode by
passing the strip bytes to a standard JPEG decoder. The JPEG colour space
may differ from the TIFF PhotometricInterpretation — if TIFF says RGB but
JPEG is YCbCr, the JPEG decoder's output is already converted to RGB.

The `image-codec-tiff` crate delegates to `image-codec-jpeg` for these
strips; callers that do not want the JPEG dependency may stub these out.

---

## 5. PhotometricInterpretation Values

| Value | Name            | Description                                         |
|-------|-----------------|-----------------------------------------------------|
| 0     | WhiteIsZero     | Grayscale; 0 = white                                |
| 1     | BlackIsZero     | Grayscale; 0 = black (standard)                     |
| 2     | RGB             | Red, Green, Blue channels                           |
| 3     | Palette         | 8-bit indices into ColorMap                         |
| 4     | TransparencyMask| Bit mask; 0=transparent                            |
| 5     | CMYK            | Cyan, Magenta, Yellow, Black (not decoded here)     |
| 6     | YCbCr           | Luma + chroma; conversion formula in TIFF spec §21  |
| 32803 | CFA             | Color Filter Array (Bayer); used by camera RAW      |
| 34892 | LinearRaw       | Linear sensor data; used by DNG                     |

---

## 6. Multi-Strip and Tile Assembly

### Strip layout

```
strip_index = row / RowsPerStrip
byte_offset = StripOffsets[strip_index]
byte_count  = StripByteCounts[strip_index]
row_within_strip = row % RowsPerStrip
```

Row stride (uncompressed, chunky):
```
bytes_per_sample = ceil(BitsPerSample / 8)   // for each channel
row_stride = ImageWidth * SamplesPerPixel * bytes_per_sample
row_stride = ceil(row_stride, 4)              // TIFF does NOT require 4-byte alignment
                                              // but many writers add it; our decoder
                                              // does NOT add padding when reading
```

### Tile layout

```
tiles_across = ceil(ImageWidth  / TileWidth)
tiles_down   = ceil(ImageLength / TileLength)
tile_index   = tile_row * tiles_across + tile_col
```

Each tile is `TileWidth × TileLength` pixels and may be padded at the
right/bottom edges. Decoders must clip to the actual image dimensions.

---

## 7. Bayer Decode (CFA Support)

When PhotometricInterpretation = 32803 (CFA), the image is a single-channel
Bayer mosaic. The mosaic pattern is described by:

- `CFARepeatPatternDim` = [2, 2] (almost always 2×2)
- `CFAPattern` = 4 bytes in row-major order, e.g., `[0, 1, 1, 2]` = RGGB

Pixel values are stored at `BitsPerSample` bits (typically 12 or 14).

**Bilinear demosaicing (reference algorithm)**:

```
For RGGB pattern (top-left pixel is Red):
  Positions:  R at (even_row, even_col)
              G at (even_row, odd_col) and (odd_row, even_col)
              B at (odd_row, odd_col)

For each output pixel (r, c) → (R, G, B):
  Use the average of available neighbours for missing channels.
  Clamp coordinates to image boundary (replicate border).
```

The demosaicing is shared across all RAW codecs as an internal module. Output
is scaled to u16 (0–65535) per channel, then gamma-corrected and converted to
sRGB u8 for the `PixelContainer` (RGBA8, A=255).

---

## 8. Colour Pipeline (for 16-bit grayscale and CFA images)

```
1. Read raw 12/14/16-bit pixel values from strips/tiles
2. If CFA: bilinear demosaicing → linear RGB (16-bit per channel)
3. Apply black-level subtraction (from BlackLevel tag if present)
4. Apply white balance multipliers (if callers supply them)
5. Apply 3×3 colour matrix (camera RGB → XYZ D50, then XYZ → sRGB)
6. Apply sRGB gamma: y = 12.92*x (x≤0.0031308), else 1.055*x^(1/2.4)-0.055
7. Clip to [0,1] and round to u8
8. Set alpha = 255
```

Steps 4–5 use metadata from downstream codecs (DNG, NEF, etc.). The TIFF
crate itself does steps 1–3 and 6–8 using identity matrices (camera = sRGB)
as defaults; callers override via `DecodeOptions`.

---

## 9. API

```rust
/// Decode the first full-resolution image from a TIFF byte stream.
/// Returns RGBA8 pixels in a PixelContainer.
pub fn decode_tiff(bytes: &[u8]) -> Result<PixelContainer, String>;

/// Decode with options (white balance, colour matrix, strip selection).
pub fn decode_tiff_with_opts(bytes: &[u8], opts: &TiffDecodeOptions)
    -> Result<PixelContainer, String>;

/// Encode a PixelContainer as uncompressed RGB TIFF.
pub fn encode_tiff(pixels: &PixelContainer) -> Vec<u8>;

/// Low-level: parse all IFDs from a TIFF byte stream.
pub fn parse_ifd_chain(bytes: &[u8]) -> Result<Vec<Ifd>, String>;

/// ImageCodec implementation.
pub struct TiffCodec;
impl paint_instructions::ImageCodec for TiffCodec {
    fn mime_type(&self) -> &'static str { "image/tiff" }
    fn encode(&self, pixels: &PixelContainer) -> Vec<u8>;
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String>;
}

pub const VERSION: &str = "0.1.0";

/// Decoded IFD (one image/page in the TIFF file).
pub struct Ifd {
    pub width: u32,
    pub height: u32,
    pub bits_per_sample: Vec<u16>,
    pub compression: u16,
    pub photometric: u16,
    pub samples_per_pixel: u16,
    pub rows_per_strip: u32,
    pub strip_offsets: Vec<u64>,
    pub strip_byte_counts: Vec<u64>,
    pub tile_width: Option<u32>,
    pub tile_length: Option<u32>,
    pub tile_offsets: Vec<u64>,
    pub tile_byte_counts: Vec<u64>,
    pub planar_config: u16,
    pub cfa_pattern: Option<[u8; 4]>,
    pub extra_tags: HashMap<u16, IfdValue>,
}

/// Arbitrary IFD tag value.
pub enum IfdValue {
    Bytes(Vec<u8>),
    Shorts(Vec<u16>),
    Longs(Vec<u32>),
    Rationals(Vec<(u32, u32)>),
    SLongs(Vec<i32>),
    SRationals(Vec<(i32, i32)>),
    Doubles(Vec<f64>),
    Ascii(String),
}

/// Decode options passed by RAW codec wrappers.
pub struct TiffDecodeOptions {
    /// Index of IFD to decode (0 = first / largest image).
    pub ifd_index: usize,
    /// White balance multipliers [R, G, B] applied after black-level subtract.
    /// Default: [1.0, 1.0, 1.0] (no correction).
    pub wb_multipliers: [f64; 3],
    /// 3×3 matrix: camera RGB → linear sRGB. Row-major.
    /// Default: identity (camera already in sRGB).
    pub color_matrix: [[f64; 3]; 3],
    /// Black level per channel (subtracted before WB).
    /// Default: 0 per channel.
    pub black_level: [u32; 4],
    /// White level (saturation point). Values ≥ white_level → 1.0.
    /// Default: (1 << BitsPerSample) - 1.
    pub white_level: u32,
}

impl Default for TiffDecodeOptions {
    fn default() -> Self {
        Self {
            ifd_index: 0,
            wb_multipliers: [1.0, 1.0, 1.0],
            color_matrix: [[1.0,0.0,0.0],[0.0,1.0,0.0],[0.0,0.0,1.0]],
            black_level: [0; 4],
            white_level: u32::MAX,
        }
    }
}
```

---

## 10. Crate Layout

```
image-codec-tiff/
  Cargo.toml        (deps: pixel-container, paint-instructions, image-codec-jpeg)
  BUILD             (cargo test -p image-codec-tiff -- --nocapture)
  README.md
  CHANGELOG.md
  src/
    lib.rs          (pub API, TiffCodec, VERSION, module re-exports)
    ifd.rs          (IFD parsing: byte order, tag reading, value decoding)
    strips.rs       (strip assembly + tile assembly)
    compression/
      mod.rs
      uncompressed.rs
      packbits.rs
      lzw.rs
      jpeg.rs       (delegates to image-codec-jpeg)
    bayer.rs        (bilinear Bayer demosaicing, shared by RAW crates)
    color.rs        (colour pipeline: black level, WB, matrix, gamma, clip)
    encoder.rs      (uncompressed TIFF writer)
    decoder.rs      (top-level decode_tiff / decode_tiff_with_opts)
```

---

## 11. Test Strategy (≥95% coverage target)

| Category                          | Tests |
|-----------------------------------|-------|
| Round-trip (RGB, grayscale)       | 3     |
| Header/byte-order (II and MM)     | 2     |
| PackBits decompression            | 2     |
| LZW decompression                 | 2     |
| JPEG strip (delegates)            | 1     |
| CFA / Bayer decode (RGGB)         | 2     |
| Multi-strip assembly              | 1     |
| Tile layout assembly              | 1     |
| Palette (ColorMap) decode         | 1     |
| YCbCr decode                      | 1     |
| 16-bit grayscale decode           | 1     |
| Error: truncated IFD              | 2     |
| Error: bad magic                  | 1     |
| Error: unsupported compression    | 1     |
| MIME type + codec trait           | 1     |
| **Total**                         | **22**|

---

## 12. Security Constraints

- Maximum image dimensions: 32768 × 32768 pixels
- Maximum IFD count: 256 (prevents unbounded linked-list traversal)
- Maximum strip / tile count: 65536
- All offsets must lie within `bytes.len()` — reject any out-of-bounds offset
- `count * typeSize` overflow must be checked with saturating arithmetic
- LZW decompressor: output buffer cap = 4 × compressed size, reject if exceeded

---

## 13. References

- TIFF Revision 6.0 Final — Adobe Systems, 1992  
  https://www.adobe.io/open/standards/TIFF.html
- TIFF Technical Note 2 (JPEG in TIFF)  
- LibTIFF source — https://libtiff.gitlab.io/libtiff/
- Exif 2.32 specification — CIPA DC-008-2019
