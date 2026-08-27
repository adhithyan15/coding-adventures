# ibm704-encoder

Canonical Rust construction and transport for IBM 704 36-bit words.

The crate implements both historical instruction layouts:

- Type A: 3-bit prefix, 15-bit decrement, 3-bit tag, 15-bit address.
- Type B: signed operation code, required zero bits, unused field, tag, and
  15-bit address.

`encode_type_a` returns a typed error when a prefix is wider than three bits
or has IBM bits 1–2 both zero, since either case would not identify a canonical
Type A word.

IBM displays Type B operation codes as values such as `+0500` (CLA) and
`-0500` (CAL). HTR is `+0000`; `+0420` is HPR, the distinct resumable halt.

## Transport

Every word is masked to 36 bits and packed into five bytes, most-significant
group first. The high nibble of the first byte is reserved and zero:

```text
word bits 35..32, 31..24, 23..16, 15..8, 7..0
```

`unpack_word` and `unpack_words` reject a non-zero reserved nibble and partial
words. They never guess the legacy byte order.

```rust
use ibm704_encoder::{encode_cla, encode_htr, pack_word, unpack_words};

let bytes = [pack_word(encode_cla(2)), pack_word(encode_htr(0))].concat();
assert_eq!(unpack_words(&bytes).unwrap(), vec![0x1_4000_0002, 0]);
```

This package is the producer contract for the Rust IBM 704 backend and
functional simulator. See
[`ibm704-encoder.md`](../../../specs/ibm704-encoder.md) for exact fields and
the historical source.
