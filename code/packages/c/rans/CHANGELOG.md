# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `rans` crate: table-based rANS entropy coding
  with the same largest-remainder table normalisation, reverse-order encoder,
  and O(1)-lookup decoder.
- `AnsTable` (`ans_table_new` / `ans_table_free` + `m` / `log2m` /
  `alphabet_size` / `freq` / `cumfreq` accessors); `RansEncoder`
  (`rans_encoder_init` / `_put` / `_finish` / `_free`); `RansDecoder`
  (`rans_decoder_init` / `_get` / `_is_exhausted`).
- Status codes for every error the crate reports (empty / too-large alphabet /
  all-zero / M-too-large / zero-frequency / short-data / symbol-out-of-range /
  alloc). Allocations use `calloc`'s checked multiply; the encoder output is
  overflow-guarded.
- Safety: the flat decode table is built so `slot - cumfreq ∈ [0, freq)` for
  every slot, so the decoder stays in bounds and never underflows for any input
  state (a malformed byte stream cannot cause an out-of-bounds access).
  Arithmetic is 64-bit (no 128-bit integers).
- Tests cover the table vectors, error cases, short-data rejection, symbol round
  trips (including skewed distributions), and determinism, under GCC and Clang
  via `iso-harness`.
