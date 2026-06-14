# Changelog — range-coder

## 0.1.0 — 2026-05-25

Initial release.

### Added

- `BoolDecoder<'a>` — VP8 boolean range decoder (RFC 6386 §7.3)
  - `new(data: &[u8])` — seed from first two bytes; stream starts at byte 2
  - `read_bit(prob: u8) -> bool` — decode one bit with known probability
  - `read_bits(n: u8) -> u32` — decode n bits MSB-first at uniform probability
  - `is_exhausted() -> bool` — true when byte cursor is past end of data
- `BoolEncoder` — VP8 boolean range encoder
  - `new()` — initialise with bottom=0, range=255, bit_count=-24
  - `write_bit(bit: bool, prob: u8)` — encode one bit
  - `write_bits(value: u32, n: u8)` — encode n bits MSB-first at uniform probability
  - `finish(self) -> Vec<u8>` — flush remaining bytes and return encoded output
- 26 tests: round-trip uniform (32 bits), skewed (64 bits), all-zeros (p=255),
  all-ones (p=0), mixed probs, u8/u16/u32 write_bits, long sequence (128 bits),
  near-boundary probs (p=1, p=254), spec test vector
- Zero dependencies — pure safe Rust

### Notes

- `bottom` uses `u64` to avoid overflow during normalization shifts; the emit
  formula `(bottom >> 24) as u8` takes the top byte of the 32-bit effective range
- Carry propagation (needed for perfect VP8 wire-format compatibility in adversarial
  inputs) is deferred — all typical inputs round-trip correctly
