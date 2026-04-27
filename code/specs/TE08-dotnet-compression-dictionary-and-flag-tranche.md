# TE08 — Dotnet Compression Dictionary And Flag Tranche

## Overview

This note scopes the second pure .NET compression tranche after the initial
`huffman-tree` and `lz77` foundations.

The goal here is to cover the two immediate follow-on models in the compression
series:

- `lz78` — explicit dictionary / trie-based compression
- `lzss` — flag-bit refinement of the `lz77` sliding-window model

As with the first tranche:

- C# packages must be implemented in C#
- F# packages must be implemented in F#
- Neither language may wrap or delegate to the other
- No external compression libraries are used

## Why These Two Next

`lz78` and `lzss` are the natural second step because they branch directly from
the foundations already landed:

- `lz78` extends the dictionary side of the compression curriculum and sets up
  the mental model for `lzw`
- `lzss` extends the `lz77` sliding-window token model and is the direct
  conceptual predecessor to `deflate`

Together they advance both major tracks of the compression series without
jumping straight to higher-composition formats.

## Package Boundaries

### `lz78`

The public API should stay teaching-oriented:

- encode bytes into `(dictIndex, nextChar)` tokens
- decode tokens back into bytes
- provide simple serialise/deserialise helpers for the CMP01 wire format
- provide one-shot `compress` / `decompress` entry points

The encoder may use an internal trie or equivalent byte-keyed dictionary state,
but that machinery remains package-internal.

### `lzss`

The public API should expose:

- literal and match token constructors or equivalents
- byte-array encode/decode helpers
- block-flag serialisation matching the CMP02 teaching format
- one-shot `compress` / `decompress` entry points

`lzss` should remain independent from `lz77` at the package boundary even if it
shares the same underlying matching intuition.

## Wire-Format Expectations

### CMP01 / `lz78`

- `original_length` as BE uint32
- `token_count` as BE uint32
- fixed-width 4-byte token records

### CMP02 / `lzss`

- `original_length` as BE uint32
- `block_count` as BE uint32
- 8-symbol flag groups with 1 byte of flags followed by literal or match data

Both implementations must be careful with short, malformed, or truncated input
so deserialisers degrade safely instead of indexing past available bytes.

## Edge Cases Worth Testing

The second tranche should explicitly cover:

- empty input
- single-byte input
- repetition-free literal-heavy input
- repeated substrings that force dictionary growth in `lz78`
- flush-token behavior in `lz78`
- self-referential overlap in `lzss`
- capped dictionary or block metadata during serialisation/deserialisation
- round trips over binary data containing `0x00` and `0xFF`

## Success Criteria

This tranche is successful when:

- both languages expose pure `lz78` and `lzss` implementations
- both languages pass package-local BUILD verification
- both languages have tests covering the main spec vectors and edge cases
- the public APIs are stable enough for `lzw` and `deflate` to build on next
