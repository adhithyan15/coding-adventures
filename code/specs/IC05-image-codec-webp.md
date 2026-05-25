# IC05 — `image-codec-webp`: WebP Image Encoder/Decoder

## Overview

WebP is a modern image format developed by Google and released in 2010. It is
built on top of VP8 — a video codec originally designed for YouTube and similar
streaming services. Google contributed VP8 under an irrevocable royalty-free
patent license through the WebM project, making WebP effectively free to
implement and distribute.

### Why WebP Exists

JPEG and PNG dominated the web for decades, but both have limitations:

- **JPEG** is lossy (throws away detail), does not support transparency, and
  uses 8×8 DCT blocks that produce visible "ringing" artefacts at high
  compression.
- **PNG** is lossless and supports transparency, but uses DEFLATE, which is a
  general-purpose compressor that doesn't exploit spatial patterns in images.

WebP provides **two distinct sub-formats** to replace both:

| Goal | Sub-format | Replaces |
|------|-----------|---------|
| Smaller than JPEG at equivalent quality | VP8 (lossy) | JPEG |
| Smaller than PNG with full alpha | VP8L (lossless) | PNG |

A third sub-format, **VP8X** (extended), is a container that adds animation,
alpha channels on top of lossy VP8, ICC colour profiles, EXIF metadata, and
XMP metadata.

### File Basics

| Property | Value |
|----------|-------|
| File extension | `.webp` |
| MIME type | `image/webp` |
| Byte order | Little-endian throughout |
| Reference implementation | Google's `libwebp` |
| Typical lossy savings | ~25–35% smaller than JPEG at equivalent SSIM |
| Typical lossless savings | ~25–34% smaller than PNG |

Understanding WebP requires knowing two key ideas:

1. **Intra-prediction**: each block of pixels predicts itself from its already-
   decoded neighbours and only stores the *error* (residual). This removes
   spatial redundancy that block transforms alone cannot — the same principle
   used in video codecs.
2. **Spatial LZ77**: VP8L uses LZ77 back-references *on pixels*, not bytes,
   combined with entropy-coded residuals. The image is treated as a 2D stream
   where identical pixel patterns can be referenced from any earlier position.

---

## RIFF Container

Every WebP file is a **RIFF** (Resource Interchange File Format) container —
the same container format used by WAV audio files and AVI video files.

RIFF was designed by Microsoft and IBM in 1991 as a generic "chunks within a
file" format. A RIFF file is a tree of named chunks, where each chunk has a
four-byte identifier (FourCC), a size field, and arbitrary data. This design
means a reader that encounters an unknown chunk type can safely skip it.

### Outer RIFF Header (12 bytes)

```
Offset  Size  Field         Value / Notes
──────  ────  ─────         ─────────────
0       4     "RIFF"        ASCII: 0x52 0x49 0x46 0x46
4       4     file_size-8   u32 LE — total file size minus these 8 bytes
8       4     "WEBP"        ASCII: 0x57 0x45 0x42 0x50
```

The `file_size - 8` formula is a RIFF convention: the size field counts
everything *after* the `RIFF` FourCC and the size field itself.

### General Chunk Layout

```
┌─────────────────┬───────────────────┬──────────────────────┬────────────┐
│  FourCC (4 B)   │  Data size (4 B)  │  Data (size bytes)   │  Pad (0-1B)│
└─────────────────┴───────────────────┴──────────────────────┴────────────┘
```

The **pad byte** exists because RIFF requires all chunks to start on even byte
boundaries. If the data length is odd, one zero byte is appended after the
data. The size field does **not** count the pad byte.

### Three Top-Level Chunk Types

| FourCC | Content |
|--------|---------|
| `VP8 ` | VP8 lossy bitstream (note the trailing space — it is part of the FourCC) |
| `VP8L` | VP8L lossless bitstream |
| `VP8X` | Extended format — signals animation, alpha, ICC, EXIF, XMP |

A minimal lossy WebP file looks like:

```
RIFF .... WEBP   (12 bytes)
VP8  .... <vp8-bitstream>
```

A minimal lossless WebP file looks like:

```
RIFF .... WEBP   (12 bytes)
VP8L .... <vp8l-bitstream>
```

---

## VP8 Lossy Format

### Conceptual Background

VP8 is derived from VP8 intra-frame coding, which is in turn derived from VP6
and earlier On2 Technologies video codecs. The key insight is that video codecs
already solved the problem of efficiently compressing individual frames — by
using intra-prediction and variable-size block transforms.

Compare this to JPEG's approach:

| Technique | JPEG | VP8 |
|-----------|------|-----|
| Block size | 8×8 pixels | 16×16 macroblock (MB), subdivided into 4×4 |
| Intra-prediction | None — each block is independent | Yes — predict from decoded neighbours |
| Colour space | YCbCr 4:2:0 | YUV 4:2:0 |
| Luma transform | 8×8 DCT | 4×4 DCT on residuals; 4×4 WHT on DC values |
| Entropy coding | Huffman (fixed tables) | Boolean range coder (adaptive probabilities) |

The extra step — predicting from neighbours before transforming — is why VP8
can achieve better quality at the same bit rate: the residuals (prediction
errors) are smaller than the original blocks, so they compress better.

### Colour Space: YUV 4:2:0

VP8, like JPEG, converts RGB pixels to YUV before encoding:

- **Y** (luma) — brightness; one value per pixel
- **U / Cb** (blue-difference chroma) — colour; one value per 2×2 block
- **V / Cr** (red-difference chroma) — colour; one value per 2×2 block

The 4:2:0 notation means chroma is subsampled 2× in both horizontal and
vertical directions. A 640×480 image has:

- 640×480 = 307 200 Y samples
- 320×240 = 76 800 U samples
- 320×240 = 76 800 V samples

Human vision is far more sensitive to brightness differences than colour
differences, so discarding half the chroma samples is nearly invisible. This is
why colour subsampling is used in almost every lossy image and video codec.

### Macroblock Structure

The frame is divided into 16×16 pixel macroblocks (MBs). Each MB contains:

- One 16×16 luma (Y) block, which is further divided into sixteen 4×4 sub-blocks
- Two 8×8 chroma blocks (U and V), each divided into four 4×4 sub-blocks

The decoder processes macroblocks in raster scan order (left to right, top to
bottom). When a macroblock is being decoded, its left neighbour and top
neighbour have already been decoded — this is what makes intra-prediction
possible.

### Intra-Prediction

Intra-prediction is the key difference from JPEG. Instead of transforming the
raw pixel values in a block, VP8:

1. Chooses a **predictor** (one of several extrapolation modes) that looks at
   the row of pixels above the block and the column of pixels to its left.
2. Fills the block with the predicted values.
3. Computes the **residual** (actual value − prediction).
4. Transforms and quantises the residuals.

The residuals are small when the prediction is good — flat areas have near-zero
residuals, sharp edges have larger residuals only along the edge itself.

#### 16×16 Luma Prediction Modes (coarse)

| Mode | Symbol | Description |
|------|--------|-------------|
| DC | `B_DC_PRED_NOFILT` | Fill block with mean of top row + left column |
| Vertical | `V_PRED` | Copy the row of pixels immediately above downward |
| Horizontal | `H_PRED` | Copy the column of pixels to the left rightward |
| TrueMotion | `TM_PRED` | Extrapolate: `pixel = top + left - top_left` (gradient prediction) |

#### 4×4 Luma Prediction Modes (fine, 10 modes)

When the encoder chooses to encode each 4×4 sub-block individually, it picks
from 10 modes:

| Mode | Description |
|------|-------------|
| `B_DC_PRED` | Mean of available top/left samples |
| `B_TM_PRED` | TrueMotion gradient for each position |
| `B_VE_PRED` | Vertical — copy top row |
| `B_HE_PRED` | Horizontal — copy left column |
| `B_LD_PRED` | Left-diagonal: predict from top-right pixels moving left and down |
| `B_RD_PRED` | Right-diagonal: predict from top-left pixels moving right and down |
| `B_VR_PRED` | Vertical-right: mix of vertical and diagonal |
| `B_VL_PRED` | Vertical-left: mirror of VR |
| `B_HD_PRED` | Horizontal-down: mix of horizontal and diagonal |
| `B_HU_PRED` | Horizontal-up: mirror of HD |

#### 8×8 Chroma Prediction Modes

Four modes (DC, Vertical, Horizontal, TrueMotion), applied to the 8×8 U and V
blocks as a unit (both chroma channels share the same mode choice).

### 4×4 DCT on Residuals

After prediction, each 4×4 block of residuals is transformed with a **4×4
integer DCT** (Discrete Cosine Transform). This is analogous to JPEG's 8×8
DCT: the transform concentrates energy into a few low-frequency coefficients
while the high-frequency coefficients are near zero and can be quantised away.

The VP8 4×4 DCT uses only additions and shifts — no floating-point arithmetic —
making it hardware-friendly.

For each macroblock, there are also 16 DC coefficients (one per 4×4 luma
block). These are collected and subjected to a second transform: the **4×4
Walsh-Hadamard Transform (WHT)**. The WHT exploits correlation between the DC
values of neighbouring sub-blocks.

### Quantisation

Quantisation is the lossy step: each DCT coefficient is divided by a
quantisation step size `q` and rounded, discarding small values. A larger `q`
means more loss and smaller files; `q = 1` is near-lossless.

VP8 uses a base quantisation index (0–127) that maps to separate step sizes for
luma DC, luma AC, chroma DC, and chroma AC coefficients. The encoder exposes
this as a single quality parameter (0–100), which the encoder maps to a
quantisation index.

### Boolean Range Coder

VP8 does **not** use Huffman coding. Instead it uses a **boolean range coder**
— a form of binary arithmetic coding.

Think of it this way: a Huffman code assigns bit patterns to symbols and must
use a whole number of bits per symbol. Arithmetic coding represents symbols as
ranges within [0, 1) and can use fractional bits. A range coder is a practical
integer approximation of arithmetic coding.

VP8's boolean range coder works on single binary decisions (0 or 1). Each
decision has an associated **probability** value `p` in [0, 255]:

```
// The encoder maintains: low (u32), range (u32)
// Initially: low = 0, range = 255

// To code bit b with probability p of being 0:
split = 1 + (((range - 1) * p) >> 8)
if b == 0:
    range = split
else:
    low  += split
    range -= split

// When range drops below 128, renormalise by outputting bits and shifting
```

The adaptive probability tables are trained per-VP8-version and embedded in the
bitstream as probability updates. This gives the range coder its power: it
assigns near-optimal code lengths to every binary decision.

### Frame Bitstream Layout

The VP8 bitstream begins with a 3-byte frame tag:

```
Byte 0:
  bit  0:    key_frame — 0 = intra (key frame), 1 = inter
  bits 1–2:  version   — 0 = bicubic, 1 = bilinear reconstruction filter
  bit  3:    show_frame — always 1 for still images
  bits 4–18: first_part_size — byte length of the first partition
              (spans bytes 0–2, continuing into bytes 1 and 2)
```

If `key_frame == 0` (intra frame, which is always the case for still images),
three start-code bytes follow: `0x9D 0x01 0x2A`.

Then the frame dimensions:

```
u16 LE — bits 13-0: width in pixels, bits 15-14: horizontal scaling flag
u16 LE — bits 13-0: height in pixels, bits 15-14: vertical scaling flag
```

After the header, the boolean range coder bitstream carries:

- Colour space flag and clamping flag
- Quantisation indices for Y, U, V channels
- Loop filter parameters
- Macroblock-level prediction modes and coefficients for every macroblock

### Reconstruction Loop and Deblocking Filter

After IDCT and inverse quantisation, the reconstructed block is added back to
the prediction. A **deblocking filter** is then applied at macroblock and
sub-block boundaries to smooth the block artefacts that quantisation introduces.

The deblocking filter examines the difference across each 4-pixel edge; if the
difference exceeds a threshold (derived from the quantisation index), it applies
a smoothing kernel. This is why VP8 images look smoother than JPEG at equivalent
file size — the deblocking filter is part of the normative decode loop, not a
post-processing option.

---

## VP8L Lossless Format

### Design Philosophy

VP8L ("L" for lossless) is a completely separate codec from VP8 lossy. Where VP8
lossy borrows from video intra-frame coding, VP8L is a purpose-built lossless
image codec designed specifically to outperform PNG.

PNG uses DEFLATE (LZ77 + Huffman) after applying one of five per-scanline
filters. VP8L improves on this in several ways:

1. **Spatial LZ77**: back-references are 2D — a pixel can reference any
   previously seen pixel in any scanline, not just the current one.
2. **Colour-space transforms**: before entropy coding, channels are
   decorrelated using a series of pixel-level transforms.
3. **Palette compression**: if the image uses few unique colours, VP8L stores
   a palette and encodes only palette indices.
4. **Per-tile Huffman codes**: different image regions can use different
   entropy codes, adapting to local statistics.

### VP8L Signature Byte

The VP8L data (inside the `VP8L` RIFF chunk) begins with the signature byte
`0x2F`. A decoder must verify this byte before attempting to parse the
bitstream. The signature distinguishes VP8L from VP8 lossy in contexts where
the chunk type alone is unavailable.

### Transform Pipeline

VP8L allows up to four reversible transforms to be applied to the image before
entropy coding. On decode, the transforms are reversed in the opposite order they
were applied.

The transforms are:

#### 1. Predictor Transform

The encoder chooses, for each pixel, one of 14 spatial predictor modes. The
choice is encoded in a separate "transform image" (at reduced resolution —
blocks of pixels share a predictor mode).

Each predictor mode extrapolates from the pixel's already-decoded neighbours
(left, top, top-left, top-right):

| Predictor | Formula |
|-----------|---------|
| 0 | Constant: 0xFF000000 (fully opaque black) |
| 1 | Left pixel |
| 2 | Top pixel |
| 3 | Top-right pixel |
| 4 | Top-left pixel |
| 5 | Average of (left + 2×top + top-right) / 4 |
| 6 | Average of (left + top-left) / 2 |
| 7 | Average of (left + top) / 2 |
| 8 | Average of (top-left + top) / 2 |
| 9 | Average of (top + top-right) / 2 |
| 10 | Average of (left + top-left + top + top-right) / 4 |
| 11 | Select: pick left or top based on which is closer to top-left |
| 12 | Clamp add sub: left + top − top-left (TrueMotion with clamping) |
| 13 | Clamp add sub half: left + (top − top-left) / 2 |

The stored value for each pixel becomes `actual - predicted` (modulo 256 per
channel), which is much closer to zero than the raw pixel value.

#### 2. Color Transform

A colour transform decorrelates the three colour channels. It is stored as a
separate transform image (one transform entry per block of pixels).

Each transform entry stores two values:

- `green_to_red`: how much green contributes to red
- `red_to_blue` and `green_to_blue`: how much red/green contribute to blue

On encode:

```
new_red   = red   − (green_to_red  * green) >> 5
new_blue  = blue  − (red_to_blue   * red)   >> 5
                  − (green_to_blue * green) >> 5
```

On decode, the inverse is applied. This exploits the fact that in natural images
the RGB channels are highly correlated — adjusting for cross-channel leakage
reduces entropy in the residuals.

#### 3. Subtract Green Transform

This is the simplest transform — it has no parameters. It subtracts the green
channel from the red and blue channels:

```
new_red  = (red  − green) & 0xFF
new_blue = (blue − green) & 0xFF
```

This is equivalent to a simplified YCbCr decorrelation — for achromatic (grey)
pixels, after subtracting green, red and blue residuals become zero, making them
highly compressible.

#### 4. Color Indexing Transform (Palette)

If the image contains at most 256 unique ARGB colours, a palette (colour lookup
table) is stored and each pixel is replaced by its palette index. The palette
itself is delta-coded (each entry is stored as the difference from the previous
entry) to improve its own compression.

If the palette has 16 or fewer entries, 2 or more palette indices can be packed
into a single byte, further reducing the index image size.

### Entropy Coding: LZ77 + Prefix Codes

After the transforms, the image data is entropy coded using a combination of
LZ77 back-references and canonical Huffman prefix codes.

#### LZ77 Back-References

The pixel stream is treated as a 1D sequence (row-major order). For a run of
pixels that appeared earlier in the stream, the encoder emits a
`(length, distance)` pair instead of the raw pixel values.

The distance is computed from a 2D offset: VP8L uses a special mapping from
2D pixel offsets (dx, dy) to linearised distances, which allows back-references
to pixels in previous scanlines. This is the key improvement over DEFLATE,
which only operates on bytes in a 32 KB sliding window.

#### Prefix Codes (Canonical Huffman)

All symbols are encoded with canonical Huffman codes — codes where the code
lengths uniquely determine the code words. The prefix codes use five groups of
symbols:

| Group | Alphabet size | Encodes |
|-------|--------------|---------|
| G | 256 (literals) + 24 (lengths) + 40 (extra distance codes) = 320 | Green channel values OR copy length |
| R | 256 | Red channel values |
| B | 256 | Blue channel values |
| A | 256 | Alpha channel values |
| Dist | 40 | Copy distance |

Why pack literal values and copy lengths into the G group? VP8L encodes each
pixel as one of three things:

1. A **literal** ARGB pixel: four Huffman symbols (one per channel)
2. A **back-reference**: a length (via G group) plus a distance (via Dist group);
   the R, B, A symbols are skipped
3. A **color cache** reference: an index into a 16-slot cache of recently seen
   colours; encoded as a special symbol in the G group

Values 0–255 in the G group are literal green channel bytes. Values 256–279 are
copy lengths (corresponding to lengths 3–10). Values 280+ encode longer lengths
with extra bits. Values 256+24 to 256+24+39 are colour cache references.

#### Spatial Entropy Coding (Meta-Huffman)

The image is divided into tiles (blocks of pixels). Each tile is assigned a
Huffman code group index. Different regions of the image can thus use different
Huffman tables, adapting to local statistics. The meta-tile assignment is itself
stored as a small image (one pixel per tile; the green channel carries the code
group index).

---

## VP8X Extended Format

VP8X is a super-format that layers additional capabilities on top of VP8 or
VP8L. A VP8X file must have the `VP8X` chunk as the **first** chunk immediately
after the RIFF header.

### VP8X Chunk Layout (10 bytes of data)

```
Byte 0: Reserved (must be 0)
Byte 1-3: Flags (u24 LE)
  bit 1: ICC profile present
  bit 2: alpha channel present
  bit 3: EXIF metadata present
  bit 4: XMP metadata present
  bit 5: animation present
Bytes 4-6: Canvas width  − 1 (u24 LE) — stored as value minus 1
Bytes 7-9: Canvas height − 1 (u24 LE) — stored as value minus 1
```

Storing `width − 1` allows a 24-bit field to represent widths from 1 to
16 777 216 (2^24) inclusive, avoiding the ambiguity of a zero-sized dimension.

### Extended Chunk Ordering

The additional chunks must appear in this order after the `VP8X` chunk:

```
VP8X      ← always first
ICCP      ← ICC colour profile (optional)
ANIM      ← animation global parameters (optional)
ANMF...   ← one or more animation frames (optional, repeating)
ALPH      ← alpha channel (optional; only for single-frame lossy)
VP8       ← lossy bitstream (for single-frame lossy)
  OR
VP8L      ← lossless bitstream (for single-frame lossless)
EXIF      ← EXIF metadata (optional)
XMP       ← XMP metadata (optional)
```

### ICCP Chunk

Contains a raw ICC colour profile (binary blob). ICC profiles describe the
colour gamut and transfer function of the image, allowing colour-accurate
display on calibrated monitors. The profile is passed as-is to the OS colour
management system.

### ANIM Chunk (8 bytes)

Global animation parameters:

```
Bytes 0-3: Background colour (BGRA, u32 LE) — shown between frames
Bytes 4-5: Loop count (u16 LE) — 0 means loop forever
```

### ANMF Chunk (per-frame data)

```
Bytes 0-2:  Frame X offset / 2 (u24 LE) — top-left X = value × 2
Bytes 3-5:  Frame Y offset / 2 (u24 LE) — top-left Y = value × 2
Bytes 6-8:  Frame width  − 1   (u24 LE)
Bytes 9-11: Frame height − 1   (u24 LE)
Bytes 12-14: Frame duration in milliseconds (u24 LE)
Byte 15:    Flags:
              bit 1: dispose method — 0 = do not clear, 1 = fill background
              bit 0: blending method — 0 = use alpha blending, 1 = no blending
Bytes 16+:  Frame data — either VP8 or VP8L chunk (complete, including FourCC)
```

Offsets are stored halved (multiplied by 2 on decode) because WebP requires
all frame offsets to be even numbers. This lets the offset fit in 24 bits while
supporting canvas sizes up to 16 384 × 16 384 with 2-pixel granularity.

### ALPH Chunk (Alpha Channel)

The alpha channel for a lossy VP8 frame is stored separately. An `ALPH` chunk
must appear immediately before the `VP8 ` chunk if alpha is present.

```
Byte 0: Compression and flags
  bits 0-1: compression — 0 = uncompressed, 1 = compressed (VP8L)
  bits 2-3: filtering method — 0 = none, 1 = horizontal, 2 = vertical, 3 = gradient
  bits 4-5: pre-processing — 0 = none, 1 = level reduction
Bytes 1+:  Alpha data
```

The alpha data is a grayscale image (single channel) compressed using VP8L
(when compression == 1). The decoder decodes it, then composites it with the
VP8 colour image.

### EXIF and XMP Chunks

These chunks carry image metadata verbatim:

- **EXIF**: Binary EXIF blob, starting with the EXIF marker (`II` or `MM` for
  byte order, followed by `0x002A`). Contains camera make/model, GPS
  coordinates, orientation, etc.
- **XMP **: UTF-8 XML blob (note trailing space in FourCC) containing XMP
  Dublin Core metadata (title, description, creator, copyright, etc.).

---

## Codec Comparison Table

| Feature | JPEG | PNG | VP8 (WebP lossy) | VP8L (WebP lossless) |
|---------|------|-----|-----------------|---------------------|
| Lossless | No | Yes | No | Yes |
| Alpha channel | No | Yes | No (VP8X + ALPH) | Yes (native) |
| Animation | No | APNG (extension) | Yes (VP8X + ANMF) | Yes (VP8X + ANMF) |
| Colour space | YCbCr 4:2:0 | RGB / RGBA | YUV 4:2:0 | RGBA (with transforms) |
| Block transform | 8×8 DCT | None (DEFLATE) | 4×4 DCT + WHT | LZ77 (pixel-level) |
| Intra-prediction | No | No (filters only) | Yes (16 modes) | Yes (14 predictor modes) |
| Entropy coding | Huffman | DEFLATE | Boolean range coder | Prefix codes (Huffman) |
| Deblocking filter | Optional post-processing | N/A | Yes (normative) | N/A (lossless) |
| Typical savings vs. baseline | — | — | ~25–35% vs. JPEG | ~25–34% vs. PNG |
| Metadata | JFIF / EXIF / XMP | tEXt / iTXt | VP8X EXIF/XMP | VP8X EXIF/XMP |
| ICC colour profile | Optional (APP2) | Optional (iCCP) | VP8X ICCP | VP8X ICCP |

---

## Building Blocks Already in This Repo

| Need | Available crate | Notes |
|------|----------------|-------|
| DCT (4×4, 8×8) | `dsp-dct` | Covers transform kernel; VP8 uses 4×4 blocks |
| Huffman coding | `huffman-tree` | VP8L prefix codes are canonical Huffman |
| LZ77 back-references | `lzss` (via `deflate`) | VP8L uses LZ77 with VP8L-specific 2D distance encoding |
| DEFLATE inflate | `deflate` (with inflate) | VP8L entropy is similar but not identical to DEFLATE |
| Pixel container | `pixel-container` | Standard RGBA image buffer used by all IC codecs |

---

## Dependencies to Build Before Implementation

| Component | Proposed crate | Notes |
|-----------|---------------|-------|
| Boolean range coder | `range-coder` | Required for VP8 lossy bitstream; a future CMP10 spec will cover this |
| VP8 intra-prediction kernels | part of `image-codec-webp` | 14+ prediction modes, implemented as SIMD-friendly functions |
| VP8L transform pipeline | part of `image-codec-webp` | Subtract-green, colour transform, predictor transform |
| VP8L LZ77 with 2D distance | part of `image-codec-webp` | VP8L distance mapping is different from DEFLATE |

---

## Public API (Future — Implementation Not in Scope for This PR)

```rust
/// Codec parameters for WebP encoding.
pub struct WebPCodec {
    /// Encoding quality, 0–100.
    /// Ignored when `lossless` is true.
    pub quality: u8,

    /// If true, encode as VP8L (lossless).
    /// If false, encode as VP8 (lossy) with the given quality.
    pub lossless: bool,
}

impl WebPCodec {
    pub fn new(quality: u8, lossless: bool) -> Self;
}

impl ImageCodec for WebPCodec {
    /// Always returns "image/webp".
    fn mime_type(&self) -> &'static str;

    /// Encode a PixelContainer to WebP bytes.
    /// Uses VP8L when self.lossless, VP8 otherwise.
    fn encode(&self, pixels: &PixelContainer) -> Vec<u8>;

    /// Decode WebP bytes into a PixelContainer.
    /// Supports VP8, VP8L, and VP8X (single-frame only in v1).
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String>;
}

/// Encode with VP8 lossy at the given quality (0–100).
pub fn encode_webp(pixels: &PixelContainer, quality: u8) -> Vec<u8>;

/// Encode with VP8L lossless.
pub fn encode_webp_lossless(pixels: &PixelContainer) -> Vec<u8>;

/// Decode any WebP file (VP8, VP8L, or single-frame VP8X).
pub fn decode_webp(bytes: &[u8]) -> Result<PixelContainer, String>;
```

---

## Error Cases (Future Decoder)

| Condition | Error message |
|-----------|--------------|
| File shorter than 12 bytes | `"WebP decode: file too short"` |
| First 4 bytes != `"RIFF"` | `"WebP decode: missing RIFF header"` |
| Bytes 8–11 != `"WEBP"` | `"WebP decode: not a WebP file"` |
| Chunk type not `VP8 `, `VP8L`, or `VP8X` | `"WebP decode: unsupported chunk type XXXX"` |
| VP8L first byte != `0x2F` | `"WebP decode: invalid VP8L signature byte"` |
| VP8 start code mismatch | `"WebP decode: invalid VP8 start code"` |
| Corrupted boolean range coder state | `"WebP decode: range coder overflow"` |
| VP8X flags indicate animation | `"WebP decode: animation not yet supported"` |
| Truncated chunk data | `"WebP decode: chunk data truncated"` |
| Alpha ALPH chunk decompression failed | `"WebP decode: alpha channel decompression failed"` |

---

## Teaching Notes

### Why Video Codec Technology Becomes Image Codecs

WebP, HEIC (from H.265/HEVC), and AVIF (from AV1) all follow the same pattern:
a video codec's intra-frame compression is extracted and used as a still-image
format. This is natural — the hardest problem in video compression is how to
efficiently encode a single frame from scratch (without motion prediction from
other frames). That same technology, when applied to still images, outperforms
formats designed only for stills.

### The Boolean Range Coder vs. Huffman

Huffman coding assigns a whole number of bits to each symbol. For a symbol with
probability 0.9 (appears 90% of the time), the optimal code length is
`−log₂(0.9) ≈ 0.15 bits`. Huffman must assign 1 bit — a 6× overhead.

A range coder operates on binary decisions and can assign *fractional* bits by
narrowing a range rather than emitting fixed bit patterns. This is why VP8's
boolean range coder is more efficient than Huffman for the highly skewed
probability distributions that appear in image prediction residuals.

### VP8L's Transform Pipeline: Decorrelation Before Entropy Coding

The subtract-green and colour transforms serve the same purpose as YCbCr in
JPEG: they remove correlation between channels before entropy coding. If G
closely predicts R and B (as happens in natural images), subtracting G from R
and B produces small residuals — near zero for achromatic pixels. The entropy
coder then spends fewer bits on those residuals.

The key difference from JPEG's fixed YCbCr matrix is that VP8L's colour
transform is **adaptive** — the transform parameters are chosen per image
region and stored in the bitstream. A high-contrast artistic image with unusual
colour relationships can use a different decorrelation than a natural photograph.

### Why VP8 Beats JPEG at Equivalent Quality

JPEG uses 8×8 blocks. At the block boundary, adjacent blocks are transformed
independently, which is why JPEG produces "ringing" or "blockiness" at high
compression. VP8's intra-prediction feeds the predictor signal from already-
decoded neighbours *before* the 4×4 DCT. The residual to be transformed is
smaller, so quantisation causes less damage. The normative deblocking filter
further smooths the remaining block artefacts.

The smaller 4×4 block size also helps: finer granularity means prediction errors
are more localised and the transform adapts more closely to local image content.

### The RIFF Container Pattern

RIFF's chunk-based design (FourCC + size + data) is a lesson in extensible file
formats. Any parser that understands chunks can safely skip unknown chunk types
by reading the size field and jumping forward. WebP exploits this with VP8X:
the extended format can carry ICC profiles, EXIF, XMP, or animation frames, and
a minimal decoder that doesn't support them can skip the chunks and decode only
the image data.

Compare this to JPEG's marker-based design, where each APP segment has a
similar skip-ahead property (the marker length field), but the top-level
structure is flat rather than nested.

---

## Crate Layout

```
code/packages/rust/image-codec-webp/
├── Cargo.toml      # depends on pixel-container, dsp-dct, huffman-tree, lzss
├── src/
│   ├── lib.rs      # public API: WebPCodec, encode_webp, decode_webp
│   ├── riff.rs     # RIFF container parser/writer
│   ├── vp8/
│   │   ├── mod.rs
│   │   ├── predict.rs    # intra-prediction modes (16×16 and 4×4)
│   │   ├── dct.rs        # 4×4 DCT and WHT wrappers
│   │   ├── quantize.rs   # quantisation tables and step-size mapping
│   │   └── range_coder.rs # boolean range coder (or depend on future range-coder crate)
│   └── vp8l/
│       ├── mod.rs
│       ├── transform.rs  # predictor, colour, subtract-green, palette transforms
│       ├── huffman.rs    # canonical Huffman prefix codes
│       └── lz77.rs       # VP8L LZ77 with 2D distance mapping
├── BUILD
├── README.md
└── CHANGELOG.md
```

Dependencies: `pixel-container`, `dsp-dct`, `huffman-tree`, `lzss`. No
external compression libraries. The boolean range coder will be implemented
inline until a standalone `range-coder` crate is introduced.
