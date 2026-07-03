# range-coder

VP8 boolean arithmetic range coder — the entropy coding engine at the heart of VP8 lossy video and WebP lossy still images.

Implements the boolean range coder as specified in **RFC 6386 §7.3**. Zero dependencies. Pure safe Rust.

## What is a boolean range coder?

A boolean range coder is a binary arithmetic coder: it encodes and decodes one bit at a time, each with a known probability, achieving near information-theoretic optimal compression.

VP8 expresses every syntax element — prediction modes, DCT coefficient signs, motion vectors, segmentation flags — as a series of binary questions with calibrated probabilities. The range coder handles them all, using fixed offline-trained probability tables (unlike CABAC, which adapts per-stream).

```
Series context:
  CMP04 (Huffman, 1952)      — entropy coding; predecessor
  CMP05 (DEFLATE, 1996)      — Huffman in practice; PNG/gzip/zip
  CMP10 (BoolRangeCoder, VP8) — binary arithmetic coding; this crate
```

## Probability convention

`prob` encodes P(bit = 0) × 256 as a u8:

| prob | Meaning                    |
|------|----------------------------|
|   0  | bit is almost certainly 1  |
| 128  | 50/50 (uniform)            |
| 255  | bit is almost certainly 0  |

## Usage

```rust
use range_coder::{BoolEncoder, BoolDecoder};

// Encode
let mut enc = BoolEncoder::new();
enc.write_bit(true,  128);  // 50/50
enc.write_bit(false, 200);  // ~78% likely to be 0 — and it is
enc.write_bit(true,   64);  // ~25% likely to be 0 — so it's 1
let bytes = enc.finish();

// Decode (same probabilities, same order)
let mut dec = BoolDecoder::new(&bytes);
assert_eq!(dec.read_bit(128), true);
assert_eq!(dec.read_bit(200), false);
assert_eq!(dec.read_bit(64),  true);
```

Multi-bit values:

```rust
let mut enc = BoolEncoder::new();
enc.write_bits(0xAB, 8);   // 8-bit value, uniform probability
let bytes = enc.finish();

let mut dec = BoolDecoder::new(&bytes);
assert_eq!(dec.read_bits(8), 0xAB);
```

## Round-trip guarantee

For any sequence of `(bit, prob)` pairs, encoding then decoding with identical probabilities in the same order reproduces every bit exactly.

## How it works

The decoder maintains two state variables:

- `range` — coding interval width, kept in [128, 255] by renormalization
- `value` — the current position within the interval (seeded from the first two bytes of the stream)

To decode one bit with probability `prob`:

```
split    = 1 + (((range - 1) * prob) >> 8)
bigsplit = split << 8

if value >= bigsplit:
    bit    = 1
    range -= split
    value -= bigsplit
else:
    bit    = 0
    range  = split

// Renormalize while range < 128
while range < 128:
    range <<= 1
    value  = (value << 1) | next_msb_bit_from_stream
```

The encoder is the symmetric inverse: it tracks the lower bound of the current interval and emits output bytes as high-order bits become determined.

## Wire format

Output is a raw MSB-first byte stream:
- Bytes 0–1 seed the decoder's `value` register: `value = (data[0] << 8) | data[1]`
- Subsequent bytes feed the normalization step, one bit at a time, MSB-first within each byte

No framing header. Length comes from the VP8 frame header.

## Spec

See `code/specs/CMP10-range-coder.md` for the full specification including test vectors, teaching notes, and comparison with Huffman coding and H.264 CABAC.
