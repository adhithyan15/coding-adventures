# TE10 - .NET Compression DEFLATE Tranche

## Goal

Port the CMP05 `deflate` package to both C# and F# as pure in-language implementations.

This tranche builds on the already-ported `.NET` `lzss` and `huffman-tree` packages and
keeps the repo's teaching-oriented CMP05 wire format rather than aiming for RFC 1951
byte-for-byte compatibility.

## Scope

Add these publishable packages:

- `code/packages/csharp/deflate`
- `code/packages/fsharp/deflate`

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

Both implementations should:

- use the pure `.NET` `lzss` package for tokenization
- use the pure `.NET` `huffman-tree` package for canonical Huffman code generation
- implement the CMP05 combined literal/length alphabet
- implement the CMP05 distance-code table
- emit the documented 8-byte header and code-length tables
- pack Huffman codes and raw extra bits LSB-first
- expose:
  - `Compress`
  - `Decompress`

## Behavioral Notes

- Empty input should produce the minimal CMP05 encoding with a single end-of-data symbol.
- Literal-only inputs should emit an LL tree and no distance tree.
- Match decoding must support overlapping back-references.
- The F# package must be implemented directly in F# and must not wrap the C# package.
- No external compression or bitstream libraries may be used.

## Test Coverage Targets

Tests should cover at least:

- empty input
- single-byte values
- literal-only examples
- the spec's one-match example (`AABCBBABC`)
- overlapping matches
- repetitive text
- binary round trips
- distance-table presence or absence depending on whether matches exist
- header `original_length` correctness
- a simple compression-ratio sanity check on repetitive input

## Out of Scope

- RFC 1951 dynamic-block interoperability
- zlib or gzip framing
- `brotli`
- `reed-solomon`
