# Changelog — coding_adventures_brotli (Elixir)

## [0.1.0] — 2026-04-13

### Added

- Initial implementation of CMP06 Brotli compression and decompression.
- `CodingAdventures.Brotli.compress/1` — accepts binary or byte list, returns
  CMP06 wire-format bytes.
- `CodingAdventures.Brotli.decompress/1` — accepts CMP06 wire-format bytes,
  returns original binary.
- 64-entry ICC table covering insert lengths 0–32 and copy lengths 4–769.
- 32-entry distance code table covering offsets 1–65535.
- 4 literal context buckets based on preceding byte character class
  (space/punct=0, digit=1, uppercase=2, lowercase=3).
- O(n²) sliding window LZ matching (window=65535, min match=4, max match=258).
- LSB-first bit packing and unpacking.
- Canonical Huffman tree construction via `coding_adventures_huffman_tree` (DT27).
- Flush literal encoding: trailing literals emitted after ICC=63 sentinel,
  decoded by the decompressor until `original_length` bytes are produced.
- Empty input special case: minimal 13-byte wire format.
- 33 ExUnit tests covering all 8 spec test cases plus additional edge cases.
- 99.15% test coverage (threshold: 80%).

### Design Decisions

- **Flush literals after sentinel**: trailing bytes that have no LZ match following
  them are encoded in the bit stream after ICC=63, not as a separate command. This
  handles pure-literal inputs (too short for any LZ match) without resorting to
  dummy copy operations.
- **ICC copy snapping**: `find_best_icc_copy/2` snaps copy lengths down to the
  nearest representable ICC value to handle gaps in the table (e.g., copy=7 is not
  directly representable).
- **Insert limit per ICC command**: commands with more than 32 insert literals cannot
  be represented by a single ICC code; in that case, the insert bytes overflow into
  the flush literal pool rather than forcing an invalid split.
