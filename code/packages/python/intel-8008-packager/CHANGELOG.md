# Changelog — intel-8008-packager

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-20

### Added

- Initial release: Intel HEX encoder/decoder for Intel 8008 binary images.
- `encode_hex(binary, origin=0) -> str`: converts raw binary to Intel HEX format.
- `decode_hex(hex_text) -> (origin, bytes)`: parses Intel HEX back to binary.

#### Design

- Same Intel HEX record format as the `intel-4004-packager` (`:LLAAAATTDD...CC`).
- Key difference from 4004 packager: image size cap is 16 KB (0x4000 bytes) for the
  8008's 14-bit address space, vs 4 KB for the 4004's 12-bit space.
- 16 bytes per data record (standard "ihex16" format).
- Checksum computation: two's complement of the byte-sum of all record fields.
- `decode_hex` rejects images larger than 16 KB to prevent allocation attacks.
- Handles both ROM (0x0000–0x1FFF) and RAM (0x2000–0x3FFF) address regions.

#### Error handling

- `encode_hex` raises `ValueError` on: empty binary, origin out of 16-bit range,
  image overflow of 16-bit address space.
- `decode_hex` raises `ValueError` on: missing colon, invalid hex characters,
  record too short, byte count / record length mismatch, checksum mismatch,
  unsupported record type, and decoded image exceeding 16 KB.
