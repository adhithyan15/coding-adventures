# Changelog

## [0.1.2] - 2026-11-10

### Security

- SEC-003d: `compress` returns `Err` for input longer than `u32::MAX` bytes
  instead of writing `data.len() as u32`, which kept only the low 32 bits of
  the length (a 4 GiB + 3 byte input would decompress to 3 bytes with no
  error). The bound also stops the `u32` frequency counts from overflowing.
  This matches the C and C++ encoders (SEC-003c). The boundary is tested
  through the new private `header_length` helper, so no test allocates 4 GiB.
- `unpack_bits_lsb_first` sizes its buffer with `len.saturating_mul(8)`, so a
  32-bit target can no longer wrap the capacity hint.

### Documentation

- The module doc and an encoder comment said code lengths are 1–16. They
  are 1–56: `decompress` refuses 0 and anything above `MAX_CODE_LEN` (56).

### Known limit (backlog SEC-003e)

- `compress` still materialises one `char` per output bit, so input near the
  4 GiB cap needs tens of GB of memory. The length check fails closed, but it
  is not a resource bound. Streaming bit packing is tracked separately.

## [0.1.1] - 2026-11-10

### Security

- SEC-003: `decompress` no longer overflows `code <<= len - prev_len` on a
  hostile code-length table. Lengths of 0 or above 56, unsorted tables and
  over-subscribed (Kraft sum > 1) tables now return `Err`; shifts are checked.
- Decode loop stops once the accumulated bits exceed the table's longest code
  (was O(bits^2) rehashing), and no longer uses `chars().nth(pos)` (also O(n^2)).
- SEC-002 prep: a declared original length larger than the payload bit count
  is rejected before allocation (every code is >= 1 bit). No API change.
  Residual: output can still be up to 8x the input; a caller-set `max_output`
  awaits the WX-H / LP-003 API decision.
- `table_end` computed with checked arithmetic.
- Tables with more than 256 entries or a repeated symbol return `Err`
  (Kraft alone admits ~2^56 long codes, a memory amplification).
- CWE-789 (SEC-001): the output capacity hint is also capped at the payload bit
  count, and `tests/hostile_headers.rs` adds hostile-header regression tests.

## [0.1.0] - 2026-04-12

### Added

- Initial implementation of Huffman encoding and decoding (CMP04 spec).
- `compress` function: frequency histogram → DT27 tree → canonical codes →
  LSB-first bit packing → CMP04 wire format.
- `decompress` function: header parsing → canonical code reconstruction →
  LSB-first bit unpacking → prefix-free symbol decoding.
- `pack_bits_lsb_first` and `unpack_bits_lsb_first` internal helpers.
- Edge case handling: empty input (8-byte header only), single distinct symbol.
- Error handling: malformed header, truncated code-lengths table, exhausted
  bit stream all return `Err(String)` rather than panicking.
- 26 unit tests covering round-trips, wire format bytes, bit I/O helpers,
  compression effectiveness, error paths, and determinism.
- Depends on `huffman-tree` (DT27) for all tree construction and code derivation.
