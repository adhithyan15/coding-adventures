# IC07 — GIF Image Codec

**Specification version**: 0.1  
**Status**: Draft  
**Depends on**: CMP03 (LZW compression, `code/specs/CMP03-lzw.md`)

---

## 1. Overview

GIF (Graphics Interchange Format) is a royalty-free, widely-used raster image
format developed by CompuServe in 1987. The LZW patents (Unisys/Sperry) fully
expired in 2003 (US) and 2004 (worldwide), making GIF completely free to implement.

GIF key properties:
- **Color model**: indexed color, up to 256 palette entries per image
- **Bit depth**: 1, 2, 3, 4, 5, 6, 7, or 8 bits per pixel (always ≤ 8)
- **Compression**: GIF-variant LZW (LSB-first, variable-width codes, configurable minimum code size)
- **Transparency**: one palette index can be designated transparent (GIF89a)
- **Animation**: multiple frames with per-frame delay and disposal (GIF89a)
- **Interlacing**: optional 4-pass scan-line ordering for progressive display

Two versions:
- **GIF87a** (1987): Core format — header, color table, image data.
- **GIF89a** (1989): Adds extensions — transparency, animation, comments.

This spec covers:
- Phase 1: **Decode** GIF87a and GIF89a, including transparency; return first frame for animated GIFs.
- Phase 1: **Encode** static GIF87a (no animation) with up to 256 colors.

---

## 2. File Layout

A GIF file has the following top-level structure:

```
Header                  6 bytes
Logical Screen Descriptor  7 bytes
[Global Color Table]     0 or 3·N bytes  (N = 2^(size+1))
{Block}*                 zero or more blocks
Trailer                  1 byte (0x3B)
```

Blocks appear in sequence until the Trailer. Each block is identified by its
first byte (the **introducer**):

| Introducer | Block type |
|------------|-----------|
| `0x2C` | Image Descriptor (followed by image data) |
| `0x21` | Extension (followed by a 1-byte label) |
| `0x3B` | Trailer — end of file |

---

## 3. Header

```
Signature: 3 bytes  "GIF"
Version:   3 bytes  "87a" or "89a"
```

Total: 6 bytes. A decoder must accept both `GIF87a` and `GIF89a`. Any other
signature byte at offset 0-5 is an error.

---

## 4. Logical Screen Descriptor (LSD)

Immediately following the Header, 7 bytes:

```
canvas_width:   u16 LE   (pixels)
canvas_height:  u16 LE   (pixels)
packed:         u8
  bit 7:        global_color_table_flag  (1 = GCT present)
  bits 4-6:     color_resolution         (unused by most decoders)
  bit 3:        sort_flag                (1 = GCT sorted by importance)
  bits 0-2:     size_of_gct              (see below)
background_color_index: u8   (palette index for background; ignored if no GCT)
pixel_aspect_ratio:     u8   (0 = unspecified; else ratio = (N + 15) / 64)
```

`size_of_gct`: if `global_color_table_flag = 1`, the GCT contains
`2^(size_of_gct + 1)` color entries (range 2 to 256).

---

## 5. Color Tables

### 5.1 Global Color Table (GCT)

Follows the LSD if `global_color_table_flag = 1`. Size: `3 · 2^(size_of_gct + 1)` bytes.
Each entry is 3 bytes: `R, G, B` (unsigned 8-bit values).

The GCT applies to all images in the file that do not have a Local Color Table.

### 5.2 Local Color Table (LCT)

Each Image Descriptor may specify a Local Color Table (see Section 6). The LCT
overrides the GCT for that image only.

### 5.3 Palette to RGBA conversion

When decoding to an RGBA pixel buffer:
- Palette entry `i` produces `(R[i], G[i], B[i], 255)` (fully opaque)
- If a Graphic Control Extension sets the transparent index to `t`, then
  pixel with palette index `t` produces `(R[t], G[t], B[t], 0)` (fully transparent)
- All other pixels are fully opaque

---

## 6. Image Descriptor

An image block starts with the `0x2C` introducer byte, followed by:

```
left:   u16 LE   (pixel offset from canvas left)
top:    u16 LE   (pixel offset from canvas top)
width:  u16 LE   (image width in pixels)
height: u16 LE   (image height in pixels)
packed: u8
  bit 7: local_color_table_flag   (1 = LCT present)
  bit 6: interlace_flag           (1 = image is interlaced)
  bit 5: sort_flag
  bits 3-4: reserved
  bits 0-2: size_of_lct           (same formula as GCT: 2^(n+1) entries)
```

If `local_color_table_flag = 1`, the LCT immediately follows (before image data).

---

## 7. GIF LZW Compression

GIF uses a specific variant of LZW that differs from the CMP03 baseline in
several ways. Implementors must use this GIF-specific LZW, not the generic
CMP03 encoding.

### 7.1 Minimum code size

The first byte of the image data section is `lzw_minimum_code_size` (range 2-8).

For a 256-color image, `lzw_minimum_code_size = 8`.
For a 4-color image,  `lzw_minimum_code_size = 2`.
For a 2-color image,  `lzw_minimum_code_size = 2` (minimum is always 2).

### 7.2 Control codes

```
CLEAR_CODE = 2^lzw_minimum_code_size
EOI_CODE   = CLEAR_CODE + 1
first_dynamic_code = EOI_CODE + 1
```

Initial code width = `lzw_minimum_code_size + 1` bits.

### 7.3 Code table

The code table is pre-initialized with `CLEAR_CODE` single-byte entries:
- Code 0 → byte value 0
- Code 1 → byte value 1
- …
- Code `CLEAR_CODE - 1` → byte value `CLEAR_CODE - 1`
- Code `CLEAR_CODE` → CLEAR (reset)
- Code `EOI_CODE` → End of information

After initialization, the next available code is `first_dynamic_code`.

### 7.4 Encoder algorithm

```
Initialize code table (CLEAR_CODE single-byte entries)
Emit CLEAR_CODE
code_size = lzw_minimum_code_size + 1
code_table_max = 2^code_size - 1

For each pixel p in the input:
  If (current_prefix + p) is in the code table:
    current_prefix = (current_prefix + p)
  Else:
    Emit code for current_prefix
    Add (current_prefix + p) to code table at next available code
    If next_code > code_table_max:
      If code_table_max < 4095:
        code_size += 1
        code_table_max = 2^code_size - 1
      Else:
        Emit CLEAR_CODE
        Reset code table to initial state
        code_size = lzw_minimum_code_size + 1
    current_prefix = {p}

Emit code for current_prefix
Emit EOI_CODE
```

Maximum table size: 4096 entries (12-bit codes). When the table is full,
the encoder emits CLEAR_CODE and resets (table-full strategy).

### 7.5 Decoder algorithm

```
Initialize code table (CLEAR_CODE single-byte entries)
code_size = lzw_minimum_code_size + 1
next_code = first_dynamic_code

Read first code; must be CLEAR_CODE → ignore and read next code
prev_code = first non-CLEAR code
Output code_table[prev_code]

Loop:
  Read code
  If code == EOI_CODE: stop
  If code == CLEAR_CODE:
    Reset code table
    code_size = lzw_minimum_code_size + 1
    next_code = first_dynamic_code
    prev_code = read next code
    Output code_table[prev_code]
    Continue

  If code < next_code:
    entry = code_table[code]
  Else if code == next_code:
    entry = code_table[prev_code] + [first byte of code_table[prev_code]]
  Else:
    Error: invalid code

  Output entry
  code_table[next_code] = code_table[prev_code] + [entry[0]]
  next_code += 1
  if next_code > 2^code_size and code_size < 12:
    code_size += 1
  prev_code = code
```

### 7.6 Bit packing

GIF LZW uses **LSB-first** bit packing within each byte. This is the same
convention as the `lzw` crate (CMP03). Codes are packed continuously across
byte boundaries.

### 7.7 Sub-blocks

GIF image data is stored in **sub-blocks**. Each sub-block is:
```
length: u8       (1-255 bytes of data; 0 = terminator)
data:   [u8; length]
```

The decoder reads sub-blocks one at a time, concatenating data bytes into a
flat byte stream for the LZW decoder. A zero-length sub-block terminates the
image data.

---

## 8. Extension Blocks (GIF89a)

Extensions start with `0x21` (Extension Introducer) followed by a 1-byte
label identifying the extension type.

### 8.1 Graphic Control Extension (label: `0xF9`)

Appears immediately before an Image Descriptor (or Plain Text Extension).
Provides transparency and animation timing for the following image.

```
0x21 0xF9 0x04   (fixed header: introducer, label, block size)
packed: u8
  bits 5-7: reserved (must be 0)
  bits 2-4: disposal_method (see below)
  bit 1:    user_input_flag  (wait for user input before advancing; rare)
  bit 0:    transparent_color_flag  (1 = transparent index is valid)
delay_time:                u16 LE   (centiseconds between frames; 0 = no delay)
transparent_color_index:   u8
0x00   (block terminator)
```

**Disposal methods**:
| Value | Meaning |
|-------|---------|
| 0 | Not specified (leave in place) |
| 1 | Do not dispose (accumulate frames) |
| 2 | Restore to background color |
| 3 | Restore to previous state |

### 8.2 Application Extension (label: `0xFF`)

Used by Netscape to add loop count for animated GIFs:

```
0x21 0xFF 0x0B   (11-byte application block header)
application_id:      8 bytes ASCII   "NETSCAPE"
authentication_code: 3 bytes ASCII   "2.0"
[sub-blocks]

Sub-block for loop count:
  0x03 0x01 loop_count_lo loop_count_hi
  (loop_count: u16 LE; 0 = loop forever)
```

### 8.3 Comment Extension (label: `0xFE`)

Arbitrary text comment. Contains one or more sub-blocks of UTF-8 text.
The decoder may ignore the content.

### 8.4 Plain Text Extension (label: `0x01`)

Rarely used text rendering extension. May be silently skipped by the decoder.

### 8.5 Unknown extensions

Extensions with unrecognized labels must be silently skipped by reading
and discarding all sub-blocks until a zero-length terminator is found.

---

## 9. Interlacing

When `interlace_flag = 1`, the decompressed pixel data is in interlaced order.
The 4-pass de-interlace mapping:

| Pass | Starting row | Row step |
|------|-------------|----------|
| 1 | 0 | 8 |
| 2 | 4 | 8 |
| 3 | 2 | 4 |
| 4 | 1 | 2 |

To de-interlace: create a flat output buffer of `width × height` pixels.
For each pass `p` with start `s` and step `t`:
- Row numbers in the decompressed pixel stream: s, s+t, s+2t, …
- Write each output row to the appropriate position in the output buffer

The encoder (Phase 1) always writes non-interlaced images.

---

## 10. Animation (GIF89a)

A GIF file may contain multiple Image Descriptor blocks, each preceded by an
optional Graphic Control Extension. Each image is one animation **frame**.

For Phase 1 (decode):
- **Static GIF**: Only one Image Descriptor → decode normally.
- **Animated GIF**: More than one Image Descriptor → return an error:
  `"GIF: animated GIF not supported (multiple frames detected)"`

Phase 2 (future): add `decode_gif_animated()` returning `Vec<(PixelContainer, Duration)>`.

---

## 11. API

```rust
/// Image codec implementing `paint_instructions::ImageCodec`.
pub struct GifCodec;

impl ImageCodec for GifCodec {
    fn mime_type(&self) -> &'static str { "image/gif" }
    fn encode(&self, pixels: &PixelContainer) -> Vec<u8> { encode_gif(pixels) }
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> { decode_gif(bytes) }
}

/// Encode a PixelContainer as a static GIF87a file.
///
/// The encoder quantizes the RGBA input to a 256-color palette (median cut or
/// simple histogram) and writes a single-frame non-interlaced GIF87a.
/// If the input has fully transparent pixels, a GIF89a Graphic Control
/// Extension is emitted with the transparent color index.
///
/// Output: a complete GIF byte stream starting with "GIF87a" or "GIF89a".
pub fn encode_gif(pixels: &PixelContainer) -> Vec<u8>;

/// Decode a GIF87a or GIF89a byte stream into a PixelContainer.
///
/// Returns the first (or only) frame as an RGBA8 image.
/// Transparent pixels (via GCE) are given alpha = 0.
/// Returns Err for animated GIFs, malformed data, or invalid LZW codes.
pub fn decode_gif(bytes: &[u8]) -> Result<PixelContainer, String>;
```

### 11.1 Encoder algorithm (Phase 1)

For the encoder, a simple palette quantization is acceptable in Phase 1:

1. Collect all distinct ARGB values in the input.
2. If `count <= 256`: use the exact set as the palette (no quantization needed).
3. If `count > 256`: use **median-cut** with 256 buckets, or fall back to
   a simple uniform quantization of the RGB cube. For Phase 1 a simple
   approach is acceptable; quality can be improved later.
4. Find the smallest `lzw_minimum_code_size` such that `2^n >= palette_size`
   (minimum 2).
5. Map each pixel to its closest palette index.
6. Write GIF header, LSD, GCT, optional GCE (if any fully transparent pixel),
   Image Descriptor, LZW data, Trailer.

### 11.2 Error cases

| Condition | Error message |
|-----------|---------------|
| Not a GIF (`!= "GIF"`) | `"GIF: not a GIF file"` |
| Invalid version | `"GIF: unknown version (expected 87a or 89a)"` |
| Truncated header/LSD | `"GIF: truncated header"` |
| No image in file | `"GIF: no image found"` |
| Animated GIF | `"GIF: animated GIF not supported"` |
| LZW decode error | `"GIF: LZW error: <details>"` |
| Invalid sub-block | `"GIF: invalid sub-block"` |
| Image exceeds canvas | `"GIF: image data exceeds canvas bounds"` |

---

## 12. Crate Layout

```
image-codec-gif/
  Cargo.toml      (deps: pixel-container, paint-instructions; no lzw dep — GIF LZW inline)
  BUILD
  README.md
  CHANGELOG.md
  src/
    lib.rs          GifCodec, encode_gif, decode_gif, VERSION
    lzw.rs          GIF-variant LZW encoder + decoder (NOT the generic CMP03 crate)
    palette.rs      Palette quantization (exact match + median-cut fallback)
    decoder.rs      GIF file parser: header, LSD, GCT, blocks, extensions, image data
    encoder.rs      GIF file writer: header, LSD, GCT, GCE (if transparent), image data
```

> **Why not reuse the `lzw` crate?**
> The CMP03 `lzw` crate uses a fixed 9-bit initial code size and CLEAR_CODE=256,
> hardcoded to the CMP03 wire format (with a 4-byte length prefix). GIF requires
> variable minimum code sizes (2-8 bits) and CLEAR_CODE = 2^mcs. Rather than
> complicate the CMP03 API, the GIF-specific LZW lives in `lzw.rs` within this
> crate.

---

## 13. Test Plan

| Test | Description |
|------|-------------|
| `decode_1x1_red` | Minimal GIF: 1×1 red pixel, GIF87a |
| `decode_2x2_4color` | 2×2 image using 4 palette entries |
| `decode_transparent_pixel` | GIF89a with one transparent pixel; alpha=0 |
| `round_trip_solid_color` | Encode 4×4 solid blue → decode → pixel-exact |
| `round_trip_gradient` | Encode 8×8 gradient → decode → pixel-exact |
| `round_trip_256_colors` | Exactly 256 distinct colors; no quantization needed |
| `decode_interlaced` | Interlaced GIF; verify de-interlaced output matches non-interlaced |
| `decode_with_comments` | GIF89a with Comment Extension; must not error |
| `decode_with_app_extension` | Netscape loop extension present; must decode frame |
| `decode_animated_error` | Animated GIF → Err with descriptive message |
| `decode_gif89a_version` | GIF89a header accepted |
| `decode_gif87a_version` | GIF87a header accepted |
| `decode_bad_magic` | "PNG" header → descriptive Err |
| `decode_truncated` | Data cut off in middle of image → Err |
| `lzw_encode_decode_simple` | LZW round-trip for small sequences |
| `lzw_code_size_grows` | Verify code width grows at correct threshold |
| `lzw_clear_code_resets` | CLEAR_CODE mid-stream resets and continues correctly |
| `palette_exact_256` | 256 distinct colors → exact palette, no quantization |
| `palette_overflow_quantizes` | >256 colors → quantization applied, round-trip close |

Target coverage: ≥95% on all non-stub code paths.

---

## 14. Relationship to Other Packages

| Dependency | Role |
|------------|------|
| pixel-container | RGBA pixel buffer |
| paint-instructions | `ImageCodec` trait |
| CMP03 (lzw crate) | Conceptually related, but GIF LZW is implemented inline |
| IC00-IC06 | Sibling image codecs (BMP, JPEG, PPM, QOI, WebP, JXL) |

---

## 15. Historical Context and Teaching Notes

### Why LZW?

CompuServe chose LZW for GIF in 1987 because it was the state of the art for
dictionary compression. LZW requires no pre-analysis of the data (unlike Huffman
which needs a frequency count pass) — it builds its dictionary on the fly, making
it suitable for streaming use.

### The Patent Problem

In 1994, Unisys announced it would enforce a software patent on LZW. This caused
a major controversy — millions of existing GIF files became legally encumbered.
The PNG format was developed as a royalty-free alternative. By 2004, all GIF/LZW
patents had expired worldwide, making GIF royalty-free again.

### Why is GIF still used?

Despite PNG's superior lossless compression and APNG/WebP's better animation,
GIF has near-universal browser support and strong cultural momentum (internet memes,
reaction GIFs). Many tools and platforms still use GIF for animation.

### Compression quality

GIF's LZW compression is significantly worse than DEFLATE (PNG) or even early
Huffman coding because it doesn't have a second-pass entropy coder. LZW alone
achieves roughly 30-60% size reduction on typical indexed images vs. raw, while
PNG achieves 60-80%.

### The 256-color limit

GIF's hard limit of 256 colors per frame comes from the 8-bit palette index.
This was a significant limitation even in 1987 when 24-bit color displays were
rare. The "GIF dithering" aesthetic (visible color banding) became a characteristic
look of early web images.

For animated GIFs, each frame can have its own Local Color Table, allowing a
wider effective color range across frames — though each frame is still limited
to 256 simultaneous colors.
