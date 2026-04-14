# Changelog

## [0.1.0] - 2026-04-13

### Added

- Initial implementation of CMP06 Brotli compression for Ruby.
- `CodingAdventures::Brotli.compress(data)` — compress any binary string.
- `CodingAdventures::Brotli.decompress(data)` — decompress CMP06 wire format.
- 4 literal context buckets (space/punct=0, digit=1, uppercase=2, lowercase=3).
- 64 ICC codes bundling insert+copy lengths (full table from CMP06 spec).
- 32 distance codes extending window to 65535 bytes (codes 0–31).
- 65535-byte sliding window with O(n²) LZ matching, minimum match length 4.
- LSB-first bit packing (consistent with CMP05/DEFLATE).
- CMP06 wire format: 10-byte header + ICC table + dist table + 4 literal trees
  + bit stream.
- End-of-data sentinel: ICC code 63 (insert=0, copy=0).
- Single-symbol Huffman tree uses code "0" (length 1) per spec.
- Empty input special case: minimal 13-byte payload.
- Comprehensive minitest test suite covering all 8 spec test cases plus
  additional edge cases (context transitions, overlapping copies, long-distance
  matches, varied insert lengths).
- StandardRB linting compliance.
- Gemspec, Gemfile, Rakefile, BUILD, BUILD_windows, README, CHANGELOG.
