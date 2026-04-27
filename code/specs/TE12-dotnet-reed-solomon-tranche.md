# TE12 - .NET Reed-Solomon Tranche

## Goal

Port the MA02 `reed-solomon` package to both C# and F# as pure in-language
 implementations.

This tranche builds on the already-ported `.NET` `gf256` packages and keeps the
 repo's teaching-oriented big-endian codeword convention and Berlekamp-Massey /
 Chien / Forney decode pipeline from the existing MA02 implementations.

## Scope

Add these publishable packages:

- `code/packages/csharp/reed-solomon`
- `code/packages/fsharp/reed-solomon`

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

- depend only on the pure local `.NET` `gf256` package
- build generator polynomials for positive even `nCheck` values
- encode systematic codewords as `[message bytes | check bytes]`
- compute syndromes in the repo's big-endian polynomial convention
- decode with:
  - Berlekamp-Massey
  - Chien search
  - Forney error magnitudes
- expose:
  - `BuildGenerator`
  - `Encode`
  - `Syndromes`
  - `Decode`
  - `ErrorLocator`

## Behavioral Notes

- `nCheck` must be positive and even.
- Total codeword length must not exceed `255`.
- The decoder should return the original message bytes when all syndromes are zero.
- When more than `t = nCheck / 2` errors are present, decoding should fail with a
  package-specific too-many-errors error.
- The F# package must be implemented directly in F# and must not wrap the C#
  package.
- No external finite-field or error-correction libraries may be used.

## Test Coverage Targets

Tests should cover at least:

- generator vectors such as `nCheck = 2 -> [8, 6, 1]`
- generator roots at consecutive powers of `alpha = 2`
- systematic encoding
- zero syndromes for valid codewords
- round trips with zero errors
- correction at capacity for `t = 1`, `t = 2`, and `t = 4`
- corruption in both message bytes and check bytes
- rejection when error count exceeds capacity
- empty input, single-byte input, and maximum-length accepted input
- invalid `nCheck`, oversized codewords, and too-short received buffers

## Out of Scope

- erasure decoding
- shortened-code convenience helpers
- streaming encoders or decoders
- `document-ast`
