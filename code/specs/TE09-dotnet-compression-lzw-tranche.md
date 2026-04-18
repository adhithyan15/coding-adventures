# TE09 - .NET Compression LZW Tranche

## Goal

Port the CMP03 `lzw` package to both C# and F# as pure in-language implementations.

This tranche follows the earlier .NET ports for `huffman-tree`, `lz77`, `lz78`, and
`lzss`. It is the first .NET compression package that needs explicit bit-packing
helpers as part of the public API surface, because CMP03 stores variable-width codes
instead of fixed-width teaching tokens.

## Scope

Add these publishable packages:

- `code/packages/csharp/lzw`
- `code/packages/fsharp/lzw`

Each package must include:

- native implementation code only
- tests
- `BUILD`
- `BUILD_windows`
- `README.md`
- `CHANGELOG.md`
- package metadata
- `required_capabilities.json`

## Functional Requirements

Both implementations should expose:

- CMP03 constants:
  - `CLEAR_CODE = 256`
  - `STOP_CODE = 257`
  - `INITIAL_NEXT_CODE = 258`
  - `INITIAL_CODE_SIZE = 9`
  - `MAX_CODE_SIZE = 16`
- bit I/O helpers for LSB-first code packing:
  - `BitWriter`
  - `BitReader`
- code-level helpers:
  - `EncodeCodes`
  - `DecodeCodes`
  - `PackCodes`
  - `UnpackCodes`
- byte-level helpers:
  - `Compress`
  - `Decompress`

## Behavioral Notes

- The encoder must pre-seed codes `0..255` with all single-byte sequences.
- Streams should begin with `CLEAR_CODE` and end with `STOP_CODE`.
- Bit packing must be LSB-first within each byte, matching the CMP03 spec.
- The decoder must support the LZW "tricky token" case where `code == nextCode`.
- The packed wire format should begin with a big-endian `original_length` header.
- The F# package must be implemented directly in F# and must not wrap the C# package.
- No external compression or bitstream libraries may be used.

## Test Coverage Targets

Tests should cover at least:

- constant values
- `BitWriter` and `BitReader` round trips
- empty input
- single-byte input
- `AB`
- `ABABAB`
- `AAAAAAA` tricky-token case
- `AABABC`
- repetitive text round trips
- binary round trips
- `PackCodes` / `UnpackCodes` symmetry
- header `original_length` correctness
- invalid-code handling in `DecodeCodes`

## Out of Scope

- `deflate`
- `brotli`
- custom dictionary sizes or non-default code widths
- GIF palette-width variants beyond the shared CMP03 package behavior
