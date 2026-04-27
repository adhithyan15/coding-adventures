# Changelog

All notable changes to `coding-adventures-lzw` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZW (Lempel-Ziv-Welch, 1984) compression — CMP03.
- `M.compress(str)` — one-shot API: encodes a Lua string to CMP03 wire format.
- `M.decompress(data)` — one-shot API: decodes CMP03 wire format back to a string.
- `encode_codes(data)` — internal encoder: converts a byte array to a list of LZW codes including `CLEAR_CODE` and `STOP_CODE`.
- `decode_codes(codes)` — internal decoder: reconstructs a byte array from a list of LZW codes; handles the "tricky token" case (code == next_dict_slot).
- `pack_codes(codes, original_length)` — serialiser: packs LZW codes into LSB-first variable-width bit stream with 4-byte big-endian original_length header.
- `unpack_codes(data)` — deserialiser: reads CMP03 wire format into a codes list and original_length.
- `new_bit_writer()` — stateful LSB-first bit writer.
- `new_bit_reader(data)` — stateful LSB-first bit reader.
- Exported constants: `CLEAR_CODE`, `STOP_CODE`, `INITIAL_NEXT_CODE`, `INITIAL_CODE_SIZE`, `MAX_CODE_SIZE`, `VERSION`.
- Rockspec `coding-adventures-lzw-0.1.0-1.rockspec` for LuaRocks distribution.
- Comprehensive Busted test suite covering: spec vectors, round-trip invariants, wire format, tricky token, all-256-bytes, compression effectiveness, and robustness against malformed input.
- Literate programming style throughout — inline explanations of variable-width bit packing, the tricky token, dictionary seeding, and the LSB-first convention.
