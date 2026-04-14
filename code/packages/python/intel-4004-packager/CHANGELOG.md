# Changelog — intel-4004-packager

## [0.1.0] — 2026-04-13

### Added

- `encode_hex(binary, origin=0) -> str` — converts raw binary bytes to Intel HEX format
- `decode_hex(hex_text) -> (origin, bytes)` — parses Intel HEX back to binary for round-trip testing
- `Intel4004Packager` — orchestrates the full Nib → Intel HEX pipeline in a single `pack_source()` call
- `PackageResult` — frozen dataclass holding all pipeline artifacts (typed AST, raw IR, optimized IR, assembly text, binary, Intel HEX)
- `PackageError` — wraps failures from any pipeline stage with a `stage` field for diagnosis
- End-to-end integration tests using `Intel4004Simulator.execute()` to verify compiled binaries run correctly
- Intel HEX encoder produces standard 16-byte-per-record format (ihex16), compatible with all EPROM programmers
- Checksum verification in `decode_hex` catches bit errors in HEX files
- Support for `origin` parameter to place ROM image at non-zero base addresses
