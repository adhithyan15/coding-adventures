# TE11 - .NET Compression Brotli Tranche

## Goal

Port the CMP06 `brotli` package to both C# and F# as pure in-language implementations.

This tranche builds on the already-ported `.NET` `huffman-tree` package and keeps the
repo's teaching-oriented CMP06 wire format rather than aiming for RFC 7932 bitstream
compatibility.

## Scope

Add these publishable packages:

- `code/packages/csharp/brotli`
- `code/packages/fsharp/brotli`

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

- implement the CMP06 insert-and-copy command table with the sentinel code `63`
- implement the CMP06 extended distance-code table up to the `65535` sliding window
- implement the four literal context buckets from the spec
- perform their own sliding-window match search in-language
- emit the documented 10-byte header and code-length tables
- pack Huffman codes and raw extra bits LSB-first
- expose:
  - `Compress`
  - `Decompress`

## Behavioral Notes

- Empty input should produce the minimal CMP06 encoding with only the sentinel ICC tree.
- Literal-only inputs should still emit the sentinel and then flush remaining literals.
- Match decoding must support overlapping back-references.
- The F# package must be implemented directly in F# and must not wrap the C# package.
- No external compression or bitstream libraries may be used.

## Test Coverage Targets

Tests should cover at least:

- empty input
- single-byte values
- all-`A` repetitive input
- English prose round trips
- binary round trips
- mixed-context literals such as `abc123ABC`
- long-distance matches above the DEFLATE-sized window ranges
- deterministic output for repeated compressions of the same input
- manual wire-format payloads for empty input and a one-byte payload
- simple compression-ratio sanity checks on repetitive input

## Out of Scope

- RFC 7932 stream interoperability
- Brotli static dictionaries
- custom quality/parameter tuning knobs
- `reed-solomon`
