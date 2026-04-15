# Changelog — coding-adventures-brotli (Lua)

All notable changes to the Lua Brotli package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-13

### Added

- Initial implementation of CMP06 Brotli compression for Lua.
- `M.compress(data)` — compresses a byte array (table of integers) or string
  using the CMP06 Brotli algorithm; returns a byte array.
- `M.decompress(data)` — decompresses a CMP06 wire-format byte array or string;
  returns a byte array.
- `M.compress_string(s)` — convenience wrapper: accepts and returns Lua strings.
- `M.decompress_string(s)` — convenience wrapper: accepts and returns Lua strings.
- Full ICC table (64 codes) bundling insert-length and copy-length ranges.
- 32 distance codes covering offsets 1–65535 (extends DEFLATE's 24 codes / 4096).
- 65535-byte sliding window with minimum match length 4.
- 4-bucket literal context model (space/punct, digit, uppercase, lowercase).
- LSB-first bit packing consistent with CMP05 (DEFLATE) wire format.
- 10-byte wire format header covering original length + 6 entry counts.
- Empty input special case: produces minimal valid wire format.
- Single-symbol Huffman trees use code `"0"` (code length = 1).
- Rockspec: `coding-adventures-brotli-0.1.0-1.rockspec`.
- BUILD and BUILD_windows scripts (models `../deflate/BUILD`).
- README.md with usage examples, wire format reference, and series position.
- Comprehensive busted test suite covering all 8 spec test cases plus extras:
  - Test 1: Empty input round-trip.
  - Test 2: Single byte round-trip (0x42, 0x00, 0xFF, 'A').
  - Test 3: All 256 distinct bytes (incompressible, but round-trip exact).
  - Test 4: 1024 × 'A' (all copies; verifies overlapping-copy encoding).
  - Test 5: English prose ≥ 1024 bytes, compressed < 80% of original.
  - Test 6: 512 pseudo-random binary bytes (exact round-trip, no ratio req).
  - Test 7: Cross-context literals ("abc123ABC") — all 4 context buckets.
  - Test 8: Long-distance match (offset > 4096 and > 8192).
  - Wire format header verification tests.
  - String API (`compress_string`/`decompress_string`) tests.
  - Various copy-length boundary tests.
  - Large-input (10000 bytes) tests.

### Implementation Notes

- LZ matching is done inline (O(n × window) scan) rather than delegating to
  the LZSS package, because Brotli's command structure (insert-and-copy bundles)
  differs from LZSS's flat token stream.
- The ICC code search scans all 63 non-sentinel codes and picks the first that
  covers both the insert and copy ranges. For pathological insert lengths that
  no single code covers, the excess is emitted as literals in the final flush
  command before the copy command.
- Literal context is computed from `history[-1]` (the last byte emitted), not
  from the last two bytes as in full RFC 7932, which uses 64 buckets.
