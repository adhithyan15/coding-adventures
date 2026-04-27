# TE07 — Dotnet Compression Foundations

## Overview

This note scopes the first pure .NET compression packages for the monorepo.
The goal is to grow a native compression stack in **both** C# and F# without
wrapping one language from the other and without leaning on external
compression libraries.

For this work:

- C# packages must be implemented in C#
- F# packages must be implemented in F#
- Neither language may delegate compression logic to the other
- No third-party compression or entropy-coding libraries are used

The shared foundation is only the .NET runtime, the base class library, and
already-ported monorepo packages such as `heap`.

## Package Order

The broader compression chain is:

1. `huffman-tree`
2. `lz77`
3. `lz78`
4. `lzss`
5. `lzw`
6. `deflate`
7. `brotli`
8. `reed-solomon`

That order matters because:

- `huffman-tree` establishes deterministic prefix-code construction that later
  entropy-coding packages depend on
- `lz77` establishes the sliding-window token model that directly feeds
  `lzss` and `deflate`
- `deflate` composes `lzss`-style backreferences with Huffman coding
- `brotli` reuses many of the same mental models but adds its own block and
  context structure on top

## First Tranche

The first implementation tranche focuses on:

- `huffman-tree`
- `lz77`

This gets one entropy-coding foundation and one dictionary/sliding-window
foundation in place first. Together they unlock the next compression layer
without forcing the entire family to land in one jump.

## API Shape

The first tranche should keep public APIs small and reusable:

- `huffman-tree`
  - build a deterministic tree from `(symbol, frequency)` pairs
  - inspect code tables, canonical code tables, weight, depth, and leaves
  - decode a bit string back to symbols
- `lz77`
  - encode byte arrays into token streams
  - decode token streams back into bytes
  - provide simple serialise/deserialise helpers for teaching-oriented binary
    formats
  - provide one-shot `compress` / `decompress` entry points

The intent is to match the existing educational package boundaries in the repo,
not to expose a single giant "compression" facade package.

## Determinism Requirements

`huffman-tree` must use deterministic tie-breaking so all language ports agree
on tree shape and canonical code lengths:

1. lower weight first
2. leaves before internal nodes at equal weight
3. lower symbol value first among equal-weight leaves
4. earlier-created internal nodes first among equal-weight internal nodes

That deterministic behavior matters for later packages such as `deflate`,
where repeatable code generation across languages is part of the educational
value of the repo.

## Non-Goals For This Tranche

- full `deflate` or `brotli` support in one pass
- bitstream packing formats beyond the repo's teaching-oriented helpers
- performance optimizations such as suffix arrays, hash chains, or canonical
  decode tables
- streaming APIs before the foundational one-shot APIs are settled

## Success Criteria

The first tranche is successful when:

- both languages expose pure `huffman-tree` and `lz77` implementations
- both languages have xUnit coverage for core construction, edge cases, and
  round trips
- both languages follow the existing .NET BUILD isolation rules for Linux and
  Windows CI
- the public APIs are stable enough for `lzss` and `deflate` to depend on next
