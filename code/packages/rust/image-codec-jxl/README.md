# image-codec-jxl

A JPEG XL Modular lossless encoder and decoder (IC09).

## What it does

Converts `PixelContainer` RGBA images to and from a simplified JXL Modular
codestream.  The encoder emits a **naked codestream** (magic bytes `FF 0A`);
the decoder accepts both naked codestreams and ISOBMFF-wrapped files.

Compression is lossless: every pixel survives a round-trip unchanged.

## Where it fits in the stack

```
PixelContainer (pixel-container)
       │
       ▼
image-codec-jxl   ←  this crate
       │
       ▼
Vec<u8> JXL bytes   (ready to write to disk or transmit over the network)
```

Dependencies: `pixel-container`, `rans`.

## How JPEG XL Modular works (simplified)

1. **SizeHeader** — width and height are encoded as raw MSB-first bits using a
   compact variable-length scheme from the JXL spec (§4.1).

2. **Gradient predictor** — for each channel (R, G, B, and optionally A) the
   encoder visits pixels in raster order.  It predicts the current pixel from
   its four available neighbours (W, N, NW, NE) using the formula:
   ```
   grad = W + N − NW
   pred = clamp(grad, min(W,N,NW,NE), max(W,N,NW,NE))
   ```
   The *residual* (actual − predicted) is small and concentrated near zero.

3. **Sign/magnitude split** — since residuals can be in [−255, 255] and the
   rANS crate uses a `u8` (≤ 256-symbol) alphabet, each residual is split into:
   - a **sign** symbol (0 = zero, 1 = positive, 2 = negative)
   - a **magnitude** symbol `|r| − 1 ∈ [0, 254]` (for non-zero residuals)

4. **rANS entropy coding** — both the sign and magnitude streams are compressed
   with the `rans` crate (Range Asymmetric Numeral Systems), which achieves
   near-Shannon compression in O(1) per symbol.

## Usage

```rust
use image_codec_jxl::{encode_jxl, decode_jxl, JxlCodec};
use pixel_container::{ImageCodec, PixelContainer};

// ── Encode ──────────────────────────────────────────────────────────
let mut src = PixelContainer::new(640, 480);
src.fill(200, 100, 50, 255);

let bytes = encode_jxl(&src);
assert_eq!(&bytes[..2], &[0xFF, 0x0A]); // naked codestream magic

// ── Decode ──────────────────────────────────────────────────────────
let dst = decode_jxl(&bytes).expect("round-trip decode");
assert_eq!(dst.pixel_at(0, 0), (200, 100, 50, 255));

// ── Via the ImageCodec trait ─────────────────────────────────────────
let bytes2 = JxlCodec.encode(&src);
let dst2   = JxlCodec.decode(&bytes2).unwrap();
assert_eq!(dst2.pixel_at(319, 239), (200, 100, 50, 255));
```

## Wire format

```
[FF 0A]                   naked codestream magic (2 bytes)
[SizeHeader bits]         variable-length MSB-first bitfield (spec §4.1)
[padding]                 zero-pad to next byte boundary
[num_channels u8]         3 (RGB) or 4 (RGBA)
[width  u32 LE]
[height u32 LE]
For each channel:
  [sign rANS block]       signs: 0=zero, 1=pos, 2=neg
  [magnitude rANS block]  |r|−1 for non-zero residuals
```

Each rANS block is self-describing:
```
[num_symbols u32 LE]
[alphabet_size u32 LE]
[counts: alphabet_size × u32 LE]
[data_len u32 LE]
[data_len bytes of rANS bitstream]
```

## Module map

| Module | Purpose |
|---|---|
| `lib.rs` | Public API: `encode_jxl`, `decode_jxl`, `JxlCodec` |
| `encoder.rs` | Top-level encode pipeline |
| `decoder.rs` | Top-level decode pipeline |
| `modular.rs` | Gradient predictor, residual compute/reconstruct |
| `entropy.rs` | rANS block encode/decode, sign/magnitude split |
| `container.rs` | Naked vs ISOBMFF detection, `jxlc` box search |
| `bitwriter.rs` | MSB-first bit packing |
| `bitreader.rs` | MSB-first bit extraction |
| `rct.rs` | YCoCg reversible colour transform (for future use) |

## Running tests

```sh
cargo test -p image-codec-jxl -- --nocapture
```

## Version

0.1.0
