# image-codec-cr2

Canon CR2 RAW image codec for Rust.

## What is CR2?

CR2 (Canon RAW version 2) is Canon's proprietary RAW camera format, introduced
in 2004 with the EOS 20D. It is a TIFF 6.0 container with two Canon-specific
extensions:

1. A 4-byte signature `"CR\x02\x00"` at bytes 8–11 of the file.
2. Full-resolution sensor data in **IFD3** (the 4th TIFF IFD), compressed as
   lossless JPEG (SOF3), while IFD0 holds a small JPEG thumbnail.

CR2 was used in Canon EOS DSLRs from 2004 to approximately 2018, spanning
models from the EOS 20D to the EOS 800D / 9000D. It was replaced by CR3
(ISO BMFF-based) in the mirrorless EOS R line.

## Where this fits in the stack

```
image-codec-cr2 (this crate)
  └── image-codec-tiff   ← TIFF parsing, strip decompress, Bayer demosaic, colour pipeline
        ├── pixel-container   ← RGBA8 pixel buffer
        └── paint-instructions ← ImageCodec trait
```

`image-codec-cr2` validates the CR2 file signature, selects the correct IFD
(IFD3 = full-resolution RAW), applies Canon-specific colour parameters
(hardcoded EOS 5D-era matrix, 14-bit black/white levels), and delegates all
TIFF parsing and colour rendering to `image-codec-tiff`.

## Usage

```rust
use image_codec_cr2::{decode_cr2, encode_cr2, Cr2Codec, VERSION};
use paint_instructions::ImageCodec;

// Decode a CR2 file to RGBA8
let cr2_bytes = std::fs::read("photo.CR2").unwrap();
let pixels = decode_cr2(&cr2_bytes)?;
println!("{}×{} image decoded", pixels.width, pixels.height);

// Encode a PixelContainer to a synthetic CR2 (test-only)
let synthetic = encode_cr2(&pixels);

// Or use the codec trait
let codec = Cr2Codec;
let decoded = codec.decode(&cr2_bytes)?;
let re_encoded = codec.encode(&decoded);
println!("MIME type: {}", codec.mime_type()); // image/x-canon-cr2

println!("crate version: {}", VERSION);
```

## Colour Pipeline

The decode pipeline (delegated to `image-codec-tiff`):

1. Validate CR2 signature + parse TIFF IFD chain
2. Locate IFD3 (full-resolution CFA sensor data)
3. Decompress the lossless JPEG strip
4. Subtract black level (hardcoded: 2047 for 14-bit sensors)
5. Bilinear Bayer demosaicing (RGGB pattern)
6. Apply white balance multipliers (D65 flat: `[1.0, 1.0, 1.0]`)
7. Apply camera-to-sRGB colour matrix (EOS 5D-era hardcoded)
8. sRGB gamma curve
9. Output RGBA8 `PixelContainer` (alpha = 255)

## Lossless JPEG (SOF3)

The `lossless_jpeg` module exposes a `decode_sof3` function that decodes
Canon's lossless JPEG strips. It handles:

- JPEG marker parsing (SOI, SOF3, DHT, SOS, EOI, DRI, RST0–RST7)
- Canonical Huffman table construction from BITS[1..16] + HUFFVAL
- DPCM prediction (predictor 1 = left) with restart intervals
- 1-component and 2-component interleaved scans

The `HuffTable` struct and `BitStream` are also public for downstream use.

## Limitations (v0.1)

- **Single hardcoded colour matrix**: all Canon DSLR models use the EOS 5D-era
  approximate matrix. A complete implementation would look up the exact model
  from the Canon MakerNote `CanonModelID` tag.
- **No MakerNote parsing**: white balance from the `ColorData` MakerNote tag
  is not extracted. The decoder uses flat D65 WB (`[1.0, 1.0, 1.0]`).
- **v0.1 SOF3 decoder**: complex Huffman tables or unusual restart intervals
  may not decode correctly. 4-component (rare) CR2 files are not guaranteed.
- **Encoder is test-only**: `encode_cr2` produces a valid round-trip file but
  is not a production Canon CR2 writer.

## References

- Laurent Clévy, "Inside Canon's CR2 files" — https://lclevy.free.fr/cr2/
- dcraw.c by Dave Coffin (GPL) — canonical reference decoder
- LibRaw — https://github.com/LibRaw/LibRaw
- Exiv2 Canon tag database — https://exiv2.org/tags-canon.html
- IC11 specification — `code/specs/IC11-image-codec-cr2.md`
