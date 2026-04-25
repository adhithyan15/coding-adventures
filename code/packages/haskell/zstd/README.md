# zstd — Haskell ZStd Compression Library (CMP07)

Educational Haskell implementation of the Zstandard (ZStd) lossless compression
format (RFC 8878). Part of the coding-adventures compression series.

## What it does

Compresses and decompresses data using the ZStd frame format:

- **LZ77 back-references** via the `lzss` package for match-finding.
- **FSE (Finite State Entropy)** coding for sequence descriptor symbols,
  using predefined tables (RFC 8878 Appendix B) — no per-frame overhead.
- **Three block types**: Raw (verbatim), RLE (identical bytes), Compressed
  (LZ77 + FSE). The encoder picks the smallest representation.

## How it fits in the stack

```
CMP00 (LZ77)     — Sliding-window back-references
CMP01 (LZ78)     — Explicit dictionary (trie)
CMP02 (LZSS)     — LZ77 + flag bits            ← dependency
CMP03 (LZW)      — LZ78 + pre-initialised alphabet; GIF
CMP04 (Huffman)  — Entropy coding
CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
CMP06 (Brotli)   — DEFLATE + context modelling + static dict
CMP07 (ZStd)     — LZ77 + FSE; high ratio + speed  ← this package
```

The `lzss` package provides LZSS tokenisation (LZ77 with a flag-bit stream).
This package converts those tokens into ZStd sequences and applies FSE entropy
coding to produce a standard RFC 8878 frame.

## Usage

```haskell
import Zstd (compress, decompress)
import qualified Data.ByteString as BS

let original = BS.pack [1..100]
let compressed = compress original
decompress compressed == Right original  -- True
```

## API

```haskell
-- | Compress a ByteString to ZStd format.
compress :: ByteString -> ByteString

-- | Decompress a ZStd frame.
decompress :: ByteString -> Either String ByteString
```

## Building and testing

```bash
cd code/packages/haskell/zstd
cabal test all
```

Requires GHC >= 9.x and the `lzss` sibling package.

## Spec

See `code/specs/` for the CMP07 specification document.

## Notes

This is a pedagogical implementation following Knuth-style literate programming.
It uses Raw_Literals (no Huffman coding) and predefined FSE tables only.
Custom dictionaries and Huffman literals are not supported.
