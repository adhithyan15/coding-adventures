# image-codec-raf

Fujifilm RAF (RAW image Format) encoder and decoder for the
`coding-adventures` monorepo.

## What is RAF?

RAF is Fujifilm's proprietary RAW format, used by every Fujifilm digital
camera since the early 2000s.  Unlike most other RAW formats (Nikon NEF,
Canon CR2), RAF does **not** use a TIFF container — it has its own binary
layout with a 116-byte outer header, an embedded JPEG thumbnail, a CFA
metadata header, and 12-bit packed pixel data.

## Key features

- **No external dependencies** — just `pixel-container` and `paint-instructions`
- **Bayer RAF decoding**: RGGB bilinear demosaicing (FinePix compacts, older bodies)
- **X-Trans RAF decoding**: 6×6 simplified bilinear demosaicing (X-Pro, X-T, X-E, X100 series)
- **Colour pipeline**: WB normalisation → Fujifilm X-T2 colour matrix → sRGB gamma
- **Security-first**: all offsets validated, dimension capped at 4096×4096, checked arithmetic

## How it fits in the stack

```
RAF bytes
   ↓  image-codec-raf::decode_raf()
PixelContainer  (RGBA8, standard interchange)
   ↓  any downstream codec / renderer
Display / PNG / BMP ...
```

`RafCodec` implements the `ImageCodec` trait from `paint-instructions`, so it
plugs into the same pipeline as `BmpCodec`, `QoiCodec`, etc.

## Usage

```rust
use image_codec_raf::{RafCodec, decode_raf, encode_raf, VERSION};
use paint_instructions::ImageCodec;

// Decode a RAF file
let bytes = std::fs::read("photo.raf").unwrap();
let pixels = decode_raf(&bytes).unwrap();
println!("Decoded {}×{} image", pixels.width, pixels.height);

// Use via trait
let pixels2 = RafCodec.decode(&bytes).unwrap();

// Encode (minimal test encoder — not a production RAF encoder)
let raf_bytes = encode_raf(&pixels);

println!("Version: {}", VERSION); // "0.1.0"
```

## File structure

```
image-codec-raf/
  Cargo.toml
  BUILD                — cargo test -p image-codec-raf -- --nocapture
  README.md
  CHANGELOG.md
  src/
    lib.rs             — public API, RafCodec, VERSION, all tests
    header.rs          — 116-byte outer header parser + magic check
    cfa_header.rs      — CFA metadata tag-block parser
    unpack.rs          — 12-bit big-endian packer/unpacker
    bayer.rs           — 2×2 Bayer bilinear demosaicing
    xtrans.rs          — 6×6 X-Trans simplified bilinear demosaicing
    color.rs           — WB normalisation, colour matrix, sRGB gamma
    decoder.rs         — top-level decode_raf orchestrator
    encoder.rs         — minimal test encoder (RGGB, neutral WB)
```

## RAF file format (summary)

```
Offset  Size   Field
     0    16   Magic: "FUJIFILMCCD-RAW " (with trailing space)
    16     4   Format version (ASCII)
    20     8   Camera model ID
    28    32   Camera model string
    60     4   Directory version
    64    20   Reserved
    84     4   JPEG offset (u32 BE)
    88     4   JPEG length (u32 BE)
    92     4   CFA header offset (u32 BE)
    96     4   CFA header length (u32 BE)
   100     4   CFA pixel data offset (u32 BE)
   104     4   CFA pixel data length (u32 BE)
   108     8   Second CFA offset/length (usually 0)
```

All outer header integers are **big-endian**.

## References

- dcraw.c `fuji_load_raw()` — reference implementation
- LibRaw — https://github.com/LibRaw/LibRaw
- ExifTool RAF tags — https://exiftool.org/TagNames/Fujifilm.html
- Spec: `code/specs/IC14-image-codec-raf.md`
