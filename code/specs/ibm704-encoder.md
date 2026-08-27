# `ibm704-encoder` spec

> **Status:** v0.2.0 canonical encoding contract, 2026-08-27.

## Purpose

Pure-Rust construction and transport of IBM 704 36-bit words. The encoder has
no IR knowledge. It exposes the historical Type A and Type B field layouts and
packs words in the canonical five-byte big-endian transport consumed by the
IBM 704 simulator.

This contract supersedes the v0.1 idealized nine-bit-opcode layout. That layout
placed the operation at bits 35–27, called `+0420` HTR, and serialized words
least-significant byte first. It was never an executable IBM 704 format.

## Word geometry

| Constant | Value | Meaning |
|---|---:|---|
| `WORD_BITS` | `36` | Width of one IBM 704 word |
| `WORD_MASK` | `0xF_FFFF_FFFF` | Mask for the valid word bits |
| `BYTES_PER_WORD` | `5` | Transport bytes per word |
| `ADDR_BITS` | `15` | Address width |
| `ADDR_MASK` | `0x7FFF` | Address mask |
| `OPCODE_SHIFT` | `24` | Type B operation magnitude occupies raw bits 32–24 |
| `DECREMENT_SHIFT` | `18` | Type A decrement begins at raw bit 18 |
| `TAG_SHIFT` | `15` | Three-bit tag begins at raw bit 15 |

## Type B words

IBM bit numbering runs left to right from `S` through 35:

```text
IBM bits:  S  1..2    3..11      12..17   18..20       21..35
          +--+-----+-----------+----------+--------+----------------+
          |S | 00  | 9-bit op  | unused   | tag    | 15-bit address |
          +--+-----+-----------+----------+--------+----------------+
raw bits: 35 34..33   32..24      23..18    17..15       14..0
```

The sign and nine-bit operation magnitude together form IBM's displayed
signed operation code. For example, `CLA` is `+0500`, `CAL` is `-0500`, HTR is
`+0000`, and HPR is `+0420`.

`encode_type_b(negative, opcode, tag, address)` masks the opcode to nine bits,
the tag to three bits, and the address to 15 bits. `encode_instruction` remains
the positive, tag-zero convenience form. `encode_htr`, `encode_hpr`, and
`encode_cla` are mnemonic conveniences.

## Type A words

```text
IBM bits:  S..2       3..17       18..20       21..35
          +--------+-------------+--------+----------------+
          | prefix | decrement   | tag    | 15-bit address |
          +--------+-------------+--------+----------------+
raw bits: 35..33      32..18       17..15       14..0
```

`encode_type_a(prefix, decrement, tag, address)` returns a typed error when the
prefix exceeds three bits or when bits 1–2 are both zero; the latter pattern
architecturally identifies Type B. It masks the remaining fields to their
architectural widths.

## Canonical transport

`pack_word` serializes a masked word most-significant group first:

```text
byte 0       byte 1       byte 2       byte 3       byte 4
0000 w35..32 w31..24      w23..16      w15..8       w7..0
```

The high nibble of byte 0 is zero. `unpack_word` is the exact inverse and
rejects a non-zero reserved nibble. `unpack_words` additionally rejects streams
whose length is not a multiple of five.

`HTR_HALT_BYTES` is `[0, 0, 0, 0, 0]`, the canonical packing of `HTR 0`.

## Backend integration

`ibm704-backend` must emit this transport. `CLA Y` reads memory at `Y`; it is
not an immediate instruction. Constants are therefore emitted into a literal
pool and `CLA` addresses that pool. The byte stream for `const 42; ret` is:

1. `CLA 2`
2. `HTR 0`
3. the sign-magnitude data word `+42`

## Tests

- Type A and Type B field-boundary vectors.
- Positive and negative Type B operation codes.
- Correct distinction between HTR `+0000` and HPR `+0420`.
- Big-endian byte vectors, reserved-nibble rejection, and round trips.
- Oversized fields are masked deterministically.
- Backend output decodes to executable `CLA literal; HTR; literal` programs.

## References

- *IBM 704 Electronic Data-Processing Machine: Manual of Operation*, form
  24-6661-2 (1955), figures 10 and 11 and “Instruction Types.”
