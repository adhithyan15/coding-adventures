# IC06 — JPEG XL Image Codec

**Specification version**: 0.1  
**Status**: Draft  
**Depends on**: CMP11 (rANS entropy coder, `code/specs/CMP11-rans.md`)  
**Implements**: ISO/IEC 18181 (JPEG XL) — Modular lossless mode (Phase 1)

---

## 1. Overview

JPEG XL (JXL) is a royalty-free, ISO-standardized image format that replaces
JPEG for lossy compression and PNG for lossless compression. It offers:

- **Lossless compression** via the *Modular* entropy coding mode
- **Lossy compression** via the *VarDCT* mode (JPEG-compatible or better quality)
- Wide color gamut, HDR, animation, alpha, layers, metadata (EXIF/XMP/ICC)
- Progressive decode and Brotli-compressed metadata

This spec covers **Phase 1: Modular lossless decode and encode** for the common
case of a single-frame sRGB or RGBA image. VarDCT lossy is described in Section
11 as Phase 2 (future implementation).

---

## 2. File Structure

A JPEG XL file is a sequence of **containers**. The two relevant container
formats are:

### 2.1 Naked codestream

When the file begins with the 2-byte signature `FF 0A`, the entire file is a
**naked codestream** — no box wrapping, just the JXL codestream directly.

```
Byte 0: 0xFF
Byte 1: 0x0A
Byte 2…: JXL codestream (SizeHeader + ImageMetadata + frames)
```

### 2.2 ISOBMFF box container

When the file begins with `00 00 00 0C 4A 58 4C 20 0D 0A 87 0A` (the "JXL "
box signature), the file uses the ISO Base Media File Format (ISOBMFF) box
structure:

```
[JXL  box] — 12-byte file type box, always first
[jxlc box] — JXL codestream box (contains the raw codestream)
[Exif box] — optional EXIF metadata (big-endian TIFF)
[xml  box] — optional XMP metadata
[jbrd box] — optional JPEG reconstruction data
```

Each box has the layout:
```
box_size:  u32 big-endian (includes the 8-byte header itself)
box_type:  4 bytes ASCII (e.g. "jxlc")
box_data:  box_size - 8 bytes
```

For Phase 1 we support both naked codestreams and the ISOBMFF container.
Unknown boxes are silently skipped.

---

## 3. Codestream Structure

The JXL codestream (whether naked or inside a `jxlc` box) has three sections:

```
SizeHeader        — image dimensions (packed bit-aligned)
ImageMetadata     — color space, bit depth, extra channels, etc.
Frames            — one or more frames, each with its own header + coding
```

All codestream fields are encoded with the JXL **entropy-coded bitstream** —
an rANS decoder reading big-endian bytes, using **ANS tokens** with fixed or
adaptive distributions. The bitstream is read bit-by-bit using a variable-length
prefix (see Section 5 for the entropy coding details).

---

## 4. SizeHeader

The SizeHeader encodes image width and height using a compact variable-length
scheme. It is read first, before any entropy coding is initialized.

```
div8:      1 bit
if div8:
  h_div8:  5 bits; height = (h_div8 + 1) * 8     → max height = 256
else:
  h_sel:   2 bits
  if h_sel == 0:   h_bits = 9
  if h_sel == 1:   h_bits = 13
  if h_sel == 2:   h_bits = 18
  if h_sel == 3:   h_bits = 30
  height: h_bits bits  (value is height - 1; add 1 to get final height)

ratio: 3 bits
if ratio == 0:   (no implied ratio — explicit width follows)
  [width encoded identically to height]
if ratio == 1:   width = height          (1:1 square)
if ratio == 2:   width = (12 * height) / 8  (3:2)
if ratio == 3:   width = (16 * height) / 8  (2:1 widescreen)
if ratio == 4:   width = (4  * height) / 3  (4:3)
if ratio == 5:   width = (3  * height) / 2  (3:2 alternate)
if ratio == 6:   width = (2  * height) / 1  (2:1)
if ratio == 7:   width = (5  * height) / 4  (5:4)
```

All SizeHeader fields are packed as raw bits with no entropy coding (the
entropy coder is not yet initialized at this point).

---

## 5. ImageMetadata

After the SizeHeader, the decoder initializes the entropy coder and reads
ImageMetadata. The core fields for Phase 1:

```
all_default: 1 bit (if 1, all fields have default values and this struct ends)
extra_fields: 1 bit
if extra_fields:
  orientation: 3 bits (1-8, EXIF convention; 1 = no rotation)
  have_animation: 1 bit (must be 0 for Phase 1)
  have_preview: 1 bit

bit_depth:
  floating_point: 1 bit (0 = integer, 1 = float)
  if not floating_point:
    bits_per_sample: U32 (distribution 0: 8, 10, 12, 16; else 1-32)
    exp_bits: 0   (integer mode)
  else: (float — float16/bfloat16/float32 etc.)
    bits_per_sample: U32
    exp_bits: U32

modular_16bit_buffers: 1 bit

num_extra_channels: U32 (count of extra channels beyond color; typically 0 or 1 for alpha)
[for each extra channel:]
  d_alpha: 1 bit (1 = this extra channel is alpha)
  if not d_alpha:
    type: U32  (0=alpha,1=depth,2=spotcolour,3=selection,4=black,5=cie_l,6=thermal,7=non_optional,8=optional)
    bits: U32
    exp_bits: U32
    dim_shift: U32
    name: string (u32 length + bytes)
    if type == alpha:
      premultiplied: 1 bit

color_encoding:
  all_default: 1 bit (if 1: sRGB, gamma=2.2, D65, no ICC; recommended for RGBA8)
  if not all_default:
    want_icc: 1 bit (if 1: ignore remaining fields, read ICC in metadata box)
    color_space: U32 (0=RGB, 1=grey, 2=XYB, 3=custom)
    white_point: U32 (0=D65, 1=custom, 2=E, 3=DCI)
    primaries: U32 (0=sRGB, 1=custom, 2=2100, 3=P3)
    transfer_function: U32 (0=BT709, 1=unknown, 2=linear, 4=PQ, 5=sRGB≈gamma2.2,
                            6=HLG, 7=DCI, 8=custom_gamma)

transform_data: (empty for lossless Modular)
preview: (absent when have_preview = 0)
```

The `U32` encoding is a JXL-specific variable-length integer (see Section 6).

---

## 6. Entropy Coding

JXL uses **rANS** (Range ANS) for all entropy-coded integers in the codestream.
The implementation builds on the `rans` crate (CMP11).

### 6.1 Cluster map and ANS distributions

JXL codestreams group symbols into **clusters**. Each cluster has its own
frequency table. The cluster for any given syntax element is fixed by the
spec (e.g., SizeHeader fields use cluster 0).

Before reading any ANS-coded symbols, the decoder reads a **prefix-code header**
that specifies the frequency tables for all clusters used in the current section.
This header is itself entropy-coded using a fixed bootstrap distribution.

For Phase 1 (Modular), two ANS table sizes are used:
- **ANS_TAB = 4096** (M = 4096) — main image data
- **ANS_TAB = 256** (M = 256) — small auxiliary fields

### 6.2 U32 encoding

Many metadata fields use the U32 small-value varint:

```
token: 2 bits
if token == 0: value = small_val (read from distribution 0)
if token == 1: value = 8  + (small_val from distribution 1)
if token == 2: value = 16 + (small_val from distribution 2)
if token == 3: value = 32-bit extension (read 32 raw bits)
```

In practice for integer images the common values (8, 10, 12, 16 bpp) are
encoded as small_val from distributions 0-1.

### 6.3 Hybrid integer coding

JXL's ANS symbols are **hybrid integers**: the symbol carries the top bits
of the value, and a variable number of raw bits carry the bottom bits. This
avoids needing huge alphabet tables for 32-bit values:

```
token = ANS.decode(cluster)
if token < split_exponent:   // small value — all bits in token
  value = token
else:                        // large value — extra raw bits
  n_bits = (token - split_exponent) + split_log2  // extra bits to read
  low    = bitstream.read_bits(n_bits)
  value  = (1 << n_bits) + low - (split_exponent << split_log2)
```

The `split_exponent` and `split_log2` parameters are specified per syntax
element by the JXL spec.

---

## 7. Frame Header

Each frame has a header encoding its dimensions, coding mode, and passes.

```
all_default: 1 bit
if not all_default:
  encoding: 1 bit (0 = VarDCT, 1 = Modular)
  flags: U64
  do_ycbcr: 1 bit (only for VarDCT; must be 0 for Modular)
  color_transform: U32 (0=XYB, 1=none, 2=YCbCr; Modular typically uses 1=none)
  save_before_ct: 1 bit

  [frame size fields — may equal image size or be a cropped tile]
  frame_size_div8: 1 bit
  if not frame_size_div8: [explicit width/height]
  [frame origin if cropping is active]

  blending_info: (present when have_extra_channels or multiple frames)
  num_passes: U32 (1 for lossless)

  [save_as_reference / reference_frame fields]

  name: string (u32 length + bytes; typically empty)
```

For Phase 1 (single-frame lossless sRGB/RGBA), the typical frame header:
- `encoding = 1` (Modular)
- `color_transform = 1` (none — no XYB)
- `num_passes = 1`

---

## 8. Modular Coding

Modular is JXL's lossless integer coding mode. It operates on a collection of
**channels** (planes) that are coded jointly using predictions and a
per-channel rANS entropy model.

### 8.1 Channels

An RGBA image at 8 bpp produces the following channels:

```
Channel 0: R   (width × height, values 0-255)
Channel 1: G
Channel 2: B
Channel 3: A   (alpha extra channel, if present)
```

Before coding, optional **meta-transforms** may be applied to reduce entropy
(see Section 8.3).

### 8.2 MAdecoder (Modular ANS Decoder)

The MA decoder decodes residuals from a prediction. For each pixel:

```
prediction = compute_predictor(x, y, channel)
residual   = MA.decode(context)
pixel      = prediction + residual
```

The **context** for residual coding depends on the spatial neighborhood (the
properties used for context selection are: values of left, top, top-right, and
top-left pixels, first derivative, second derivative, and the channel number).

The residual symbol is a **signed integer** encoded as a token via the hybrid
integer scheme of Section 6.3. Positive residuals map to even tokens, negative
to odd tokens: `token = |v| * 2 - (v < 0 ? 1 : 0)`.

### 8.3 Meta-transforms

Before coding, one or more **meta-transforms** can be applied to the channel
list. Each transform records its parameters (stored before the image data) and
applies an inverse during decode. The supported transforms:

| ID | Name | Description |
|----|------|-------------|
| 0 | RCT | Reversible Color Transform — decorrelates channels (e.g., YCoCg) |
| 1 | Palette | Replaces per-pixel channel values with palette indices |
| 2 | Squeeze | Subsamples channels at reduced resolution for progressive coding |

For Phase 1, all meta-transforms may be present in the bitstream; the decoder
must handle them. The encoder can emit zero transforms for simplicity.

### 8.4 RCT (Reversible Color Transform)

RCT decorrelates color channels using integer arithmetic. The most common
variant is YCoCg:

```
// Forward (encoder):
Co = R - B
tmp = B + (Co >> 1)  // >> 1 is arithmetic right shift
Cg = G - tmp
Y  = tmp + (Cg >> 1)

// Inverse (decoder):
tmp = Y - (Cg >> 1)
G  = Cg + tmp
B  = tmp - (Co >> 1)
R  = B + Co
```

The RCT type parameter selects which channels are transformed and in which
order. Type 6 = YCoCg applied to channels (0, 1, 2).

### 8.5 Predictors

For each pixel, the predictor combines a fixed set of neighbors:

```
W = left neighbor  (or 0 if x==0)
N = top neighbor   (or W if y==0)
NW = top-left      (or W if x==0 or y==0)
NE = top-right     (or N if x==width-1 or y==0)
```

The JXL predictor is **gradient prediction**:

```
predictor = W + N - NW
```

This is clamped to the range [min(W,N,NW,NE), max(W,N,NW,NE)].

Additional predictors are indexed 0–12; the encoder chooses per-channel.

### 8.6 Context Model

The context model maps a 7-property neighborhood vector to an ANS context
(cluster index). The 7 properties are:

| Property | Description |
|----------|-------------|
| 0 | Channel index |
| 1 | Value at left-of-left (LL) — or 0 |
| 2 | N - NW (vertical first derivative) |
| 3 | W - NW (horizontal first derivative) |
| 4 | W value |
| 5 | N value |
| 6 | N - NE (directional spread) |

The MA tree (Modular ANS decision tree) partitions this property space.
Each leaf of the tree assigns a **predictor** and a **context** (ANS cluster).
The MA tree is stored in the bitstream before the image data.

For Phase 1 implementation simplicity, a **flat 1-leaf MA tree** is
acceptable: one cluster for all residuals, gradient predictor for all pixels.

---

## 9. Wire Format Summary

Reading a JPEG XL lossless image:

```
1. Detect container:
   - Bytes 0-1 == FF 0A → naked codestream; skip to step 3
   - Bytes 0-11 == JXL box signature → scan ISOBMFF boxes for "jxlc"

2. Find "jxlc" box; extract codestream bytes

3. Read SizeHeader (raw bits — no ANS yet):
   - width, height

4. Initialize ANS decoder

5. Read ImageMetadata:
   - bits_per_sample, num_extra_channels, color_encoding

6. Read FrameHeader:
   - encoding == Modular (1), num_passes == 1

7. Read GlobalModular:
   - meta-transforms: [RCT, Palette, Squeeze] (0 or more)
   - MA tree (context model)

8. Read ANS cluster distributions (one per MA leaf)

9. For each pixel (row-major order), for each channel:
   a. Compute predictor from neighbors
   b. Decode signed residual from ANS
   c. pixel = predictor + residual

10. Apply meta-transform inverses (in reverse order)

11. Return RGBA pixel buffer
```

---

## 10. API

```rust
pub struct JxlCodec {
    pub lossless: bool,  // Phase 1: must be true
}

impl ImageCodec for JxlCodec {
    fn mime_type(&self) -> &'static str { "image/jxl" }
    fn encode(&self, pixels: &PixelContainer) -> Vec<u8> { encode_jxl(pixels) }
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> { decode_jxl(bytes) }
}

pub fn encode_jxl(pixels: &PixelContainer) -> Vec<u8>;
pub fn decode_jxl(bytes: &[u8]) -> Result<PixelContainer, String>;
```

### 10.1 Encoder decisions (Phase 1)

The Phase 1 encoder targets correctness over compression ratio. Permitted
simplifications:

- Emit naked codestream (no ISOBMFF box wrapping)
- `all_default` = 0 in ImageMetadata (write explicit fields)
- No meta-transforms (no RCT, no Palette, no Squeeze)
- Flat MA tree: 1 context cluster, gradient predictor for all channels
- 8 bpp integer, sRGB color encoding

### 10.2 Decoder requirements (Phase 1)

The decoder must handle:

- Both naked codestreams and ISOBMFF `jxlc` box containers
- 8 bpp RGBA and RGB images
- Modular encoding with gradient predictor
- RCT transform (type 6 / YCoCg) — commonly emitted by libjxl
- A flat (1-leaf) or depth-1 MA tree
- Unknown extra boxes in ISOBMFF containers (skip silently)

### 10.3 Error cases

| Condition | Error |
|-----------|-------|
| Unknown signature bytes | `"JXL: not a JPEG XL file"` |
| No `jxlc` box in ISOBMFF container | `"JXL: no codestream box"` |
| Encoding == VarDCT (0) | `"JXL: VarDCT lossy not yet implemented"` |
| have_animation == 1 | `"JXL: animated JXL not supported"` |
| bits_per_sample != 8 | `"JXL: only 8 bpp is supported in Phase 1"` |
| Truncated data | `"JXL: unexpected end of data"` |

---

## 11. Crate Layout

```
image-codec-jxl/
  Cargo.toml      deps: rans, pixel-container, paint-instructions
  BUILD
  README.md
  CHANGELOG.md
  src/
    lib.rs              WebPCodec impl, encode_jxl, decode_jxl, VERSION
    container.rs        ISOBMFF box scanner + naked codestream detection
    bitreader.rs        Raw bit reader (for SizeHeader and bootstrap fields)
    entropy.rs          ANS cluster table decoder + hybrid integer decode
    metadata.rs         SizeHeader, ImageMetadata, FrameHeader parsers
    modular.rs          MA tree, predictors, meta-transforms, channel decode
    rct.rs              Reversible Color Transform (YCoCg and variants)
```

---

## 12. Test Plan

| Test | What it verifies |
|------|-----------------|
| `signature_naked_codestream` | `FF 0A` prefix accepted |
| `signature_isobmff` | JXL box signature accepted |
| `round_trip_solid_rgba` | 4×4 solid RGBA; pixel-exact after decode |
| `round_trip_gradient_rgb` | 8×8 gradient; pixel-exact |
| `round_trip_large` | 64×64 random pixels; pixel-exact |
| `round_trip_alpha_channel` | RGBA8 with non-trivial alpha; pixel-exact |
| `decode_libjxl_solid` | Decode a reference JXL file produced by libjxl |
| `decode_error_bad_magic` | Garbage input → descriptive Err |
| `decode_error_lossy` | VarDCT frame → Err with message |
| `decode_error_animated` | Animated JXL → Err with message |
| `rct_round_trip` | Apply + invert RCT; verify identity |
| `gradient_predictor_edge_cases` | x=0, y=0, x=0 y=0 corners |

Target: ≥95% coverage on all non-FFI, non-stub code paths.

---

## 13. Relationship to Other Specs / Packages

| Dependency | Role |
|------------|------|
| CMP11 (rans) | rANS entropy decode/encode (Section 6) |
| pixel-container | `PixelContainer` — RGBA pixel buffer |
| paint-instructions | `ImageCodec` trait |
| IC00-IC05 | Sibling image codecs (BMP, JPEG, PPM, QOI, WebP, PNG) |

---

## 14. VarDCT Lossy (Phase 2 — Future)

VarDCT is JXL's lossy mode. It is deferred because:
1. It requires DCT-based transform coding (not yet implemented for JXL's 8×8 + 64×64 block hierarchy)
2. It requires a quantization table (context-adaptive, not the same as JPEG quant)
3. It requires a deblocking filter (the "Gaborish" separable filter)

Phase 2 will add:
- `encode_jxl_lossy(pixels, quality)` — quality 1-100 maps to quantization step
- `decode_jxl_lossy(bytes)` — full VarDCT decode including IDCT and Gaborish

---

## 15. Teaching Notes

### Why two coding modes?

JPEG XL was designed to replace *both* PNG (lossless) and JPEG (lossy).
Unifying them in one format required two fundamentally different coding strategies:

- **Modular**: works like PNG with better prediction + rANS instead of DEFLATE.
  "Modular" means the pixel values go through prediction, the residuals are
  entropy-coded. No frequency domain transform. This preserves exact pixel values.

- **VarDCT**: works like JPEG (DCT → quantize → entropy code) but with:
  - Variable block sizes (8×8 to 64×64) chosen adaptively
  - XYB perceptual color space (tuned to human vision)
  - Psychovisual-informed quantization tables
  - Gaborish deblocking filter to suppress ringing

### Why rANS and not DEFLATE?

PNG uses DEFLATE (LZ77 + Huffman). JXL uses rANS because:
- rANS achieves closer-to-Shannon compression with simpler code
- rANS decodes in O(1) per symbol; DEFLATE has more branching
- rANS is parallelizable (multiple interleaved streams can be decoded independently)
- DEFLATE requires a sliding window (memory); rANS has O(1) state

### The hybrid integer trick

Shannon entropy theory says the optimal code for a symbol with probability p
uses -log₂(p) bits. For rare symbols, this is many bits; for common symbols,
it's a fraction of a bit. rANS handles the fractions. But for values that can
be arbitrarily large (like pixel residuals), you'd need a huge alphabet.

JXL solves this with **hybrid integers**: the ANS alphabet covers tokens 0-63
(or similar), and each token encodes a range of actual values. The token gives
the top bits; raw (unentropy-coded) bits give the bottom bits. The rANS step
thus only needs a small-alphabet table; the extra bits are appended raw.

This is why JPEG XL can compress any bit depth (8, 16, float) efficiently
without needing rANS tables with millions of entries.
