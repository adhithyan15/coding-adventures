# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZW compression (CMP03).
- `compress/1` — one-shot compress to CMP03 wire format.
- `decompress/1` — one-shot decompress from CMP03 wire format.
- `encode_codes/1` — LZW encoder; emits CLEAR_CODE prefix, integer codes, and STOP_CODE.
- `decode_codes/1` — LZW decoder with tricky-token handling (code == next_code).
- `pack_codes/2` — variable-width LSB-first bit packer tracking code_size and next_code.
- `unpack_codes/1` — variable-width LSB-first bit unpacker with matching code_size tracking.
- Pre-seeded 256-entry dictionary; CLEAR_CODE=256, STOP_CODE=257, INITIAL_NEXT_CODE=258.
- Automatic ClearCode reset when dictionary reaches 2**16 entries (MAX_CODE_SIZE=16).
- 4-byte big-endian original_length header in CMP03 wire format.
- 40+ Test2::V0 tests covering spec vectors, tricky token, pack/unpack, round-trips,
  wire format, security (malformed input), and compression effectiveness.
