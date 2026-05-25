# image-codec-webp

WebP image codec for the paint-instructions pixel pipeline.

Implements VP8L (lossless) encoding and decoding.  VP8 lossy support requires
the `range-coder` crate and will be added in a future release.

## Position in the Stack

```text
PixelContainer   ←— rendered by paint-metal / paint-vm-*
     │
     │   encode_webp_lossless()
     ▼
  Vec<u8>  (complete .webp file, RIFF container + VP8L chunk)
     │
     │   decode_webp()
     ▼
PixelContainer
```

## Architecture

A WebP file is a RIFF container:

```text
RIFF <file_size> WEBP
  VP8L <chunk_size> <vp8l-bitstream>
```

The VP8L bitstream format:

```text
[0x2F]                      VP8L signature byte
[14 bits] width - 1
[14 bits] height - 1
[1 bit]   alpha_is_used
[3 bits]  version (must be 0)
[1 bit]   has_transform       (0 in v0.1 — no transforms)
[4 bits]  color_cache_code_bits  (0 in v0.1 — no color cache)
[G-table] Huffman code for green channel (280 symbols)
[R-table] Huffman code for red channel   (256 symbols)
[B-table] Huffman code for blue channel  (256 symbols)
[A-table] Huffman code for alpha channel (256 symbols)
[D-table] Huffman code for distances     (40 symbols)
[pixel data: per pixel, G-symbol then R, B, A symbols]
```

## Usage

### Functional API

```rust
use image_codec_webp::{encode_webp_lossless, decode_webp};
use pixel_container::PixelContainer;

// Encode
let mut pixels = PixelContainer::new(64, 64);
pixels.fill(200, 100, 50, 255);
let webp_bytes = encode_webp_lossless(&pixels);

// Decode
let decoded = decode_webp(&webp_bytes).unwrap();
assert_eq!(decoded.pixel_at(0, 0), (200, 100, 50, 255));
```

### Trait API

```rust
use image_codec_webp::WebPCodec;
use paint_instructions::ImageCodec;

let codec = WebPCodec::new(90, true); // lossless mode
let bytes = codec.encode(&pixels);
let decoded = codec.decode(&bytes).unwrap();
```

## Module Structure

```
src/
  lib.rs              — public API (WebPCodec, encode_webp_lossless, decode_webp)
  riff.rs             — RIFF container builder
  vp8l/
    mod.rs            — VP8L encode/decode orchestration
    bitstream.rs      — LSB-first BitWriter and BitReader
    huffman.rs        — Canonical Huffman tables (encode + decode)
    lz77.rs           — LZ77 distance mapping table (decode stub)
    transforms.rs     — VP8L transform types and inverse-subtract-green
```

## Encoding Strategy (v0.1)

This release uses **literal-only** encoding:

- **No transforms** — the encoder writes `has_transform = 0`.
  The subtract-green, predictor, colour, and colour-index transforms are
  defined in `transforms.rs` but not applied.

- **No LZ77 back-references** — every pixel is encoded as four Huffman-coded
  channel values (G, R, B, A).  The distance Huffman group always uses a
  trivial one-symbol code.

- **No colour cache** — `color_cache_code_bits = 0`.

This produces valid VP8L output that round-trips perfectly.  Compression ratio
is lower than a fully-optimised encoder because spatial redundancy is not
exploited.

## Huffman Code Storage

VP8L encodes each Huffman group in one of two formats:

| Format    | When used                          | Bits written        |
|-----------|------------------------------------|---------------------|
| Simple-1  | Group has exactly 1 distinct symbol | `1, 0, symbol[8]` |
| Simple-2  | Group has exactly 2 distinct symbols | `1, 1, sym0_bits, sym1[8]` |
| Complex   | Group has ≥ 3 distinct symbols      | Meta-Huffman scheme |

For complex codes this crate uses a fixed meta-tree where meta-symbols 0-15
each have meta-length 4.  This is valid VP8L (the format allows any valid
meta-tree) but not maximally compact.

## Limitations and Future Work

| Feature                         | Status                |
|---------------------------------|-----------------------|
| VP8L literal encoding           | Implemented ✓        |
| VP8L subtract-green transform   | Decode only; not applied by encoder |
| VP8L predictor transform        | Not yet implemented   |
| VP8L colour transform           | Not yet implemented   |
| VP8L colour-index transform     | Not yet implemented   |
| VP8L LZ77 back-references       | Decoder returns error |
| VP8L colour cache               | Not yet implemented   |
| VP8 lossy encoding/decoding     | Requires `range-coder` crate |

## References

- [WebP Lossless Bitstream Specification](https://developers.google.com/speed/webp/docs/webp_lossless_bitstream_specification)
- [WebP Container Specification (RIFF)](https://developers.google.com/speed/webp/docs/riff_container)
- [RFC 6386 — VP8](https://www.rfc-editor.org/rfc/rfc6386)
