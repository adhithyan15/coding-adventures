# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZW compression (CMP03).
- `encode_codes/1` — LZ78-style dictionary encoder pre-seeded with all 256 single-byte entries; emits `CLEAR_CODE` (256) at the start and `STOP_CODE` (257) at the end; resets dictionary and re-emits `CLEAR_CODE` when the dictionary reaches 65536 entries.
- `decode_codes/1` — dictionary decoder with tricky-token handling (`code == next_code` → `dict[prev_code] ++ [dict[prev_code][0]]`) and `CLEAR_CODE` mid-stream reset support.
- `pack_codes/2` — variable-width LSB-first bit-packing; code size starts at 9 bits and grows by 1 when `next_code > (1 <<< code_size)`; encoder and decoder both track `next_code` for lockstep size bumping.
- `unpack_codes/1` — variable-width LSB-first bit-reader with symmetric code-size tracking; returns `{codes, original_length}`.
- `compress/1` — one-shot compress to CMP03 wire format (4-byte big-endian `original_length` header + bit-packed codes).
- `decompress/1` — one-shot decompress from CMP03 wire format; returns `{:error, :too_short}` for inputs under 4 bytes; truncates over-produced output to `original_length`.
- 50+ ExUnit tests covering spec vectors (empty, single byte, AB, ABABAB, AAAAAAA, all 256 bytes), round-trip breadth, wire format, compression effectiveness, and robustness/security.
