# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `range-coder` crate: the VP8 boolean range coder
  (RFC 6386 §7) — a binary arithmetic coder — with the same split/renormalize
  arithmetic in the encoder and decoder.
- Encoder API: `rc_encoder_init`, `rc_encoder_write_bit`,
  `rc_encoder_write_bits`, `rc_encoder_finish` (hands off the malloc'd output;
  `RC_ERR_ALLOC` on out-of-memory), `rc_encoder_free`. Output buffer is
  overflow-guarded.
- Decoder API: `rc_decoder_init` (borrows the input), `rc_decoder_read_bit`,
  `rc_decoder_read_bits`, `rc_decoder_is_exhausted`. MSB-first; an exhausted
  stream reads as zeros; the two-byte seed and empty/single-byte inputs are
  handled exactly as the crate.
- Arithmetic in `uint64_t`/`uint32_t` (no 128-bit integers, no libm).
- Tests round-trip single bits, mixed/skewed probability sequences, and
  `write_bits`/`read_bits` for 8/16/32-bit fields, plus seeding, exhaustion, and
  determinism, under GCC and Clang via `iso-harness`.
