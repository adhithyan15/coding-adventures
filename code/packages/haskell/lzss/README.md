# lzss — Haskell (CMP02)

Educational implementation of the **LZSS** (Lempel-Ziv-Storer-Szymanski, 1982)
lossless compression algorithm, part of the CMP compression series.

## What is LZSS?

LZSS refines LZ77 by replacing the fixed-size `(offset, length, next_char)`
triple with a mixed token stream. A 1-byte **flag word** precedes each group of
8 tokens; each bit says whether the corresponding token is a raw byte (Literal)
or a back-reference (Match):

```
Literal  → 1 byte  : the raw byte value          (flag bit = 0)
Match    → 3 bytes : offset u16 BE + length u8   (flag bit = 1)
```

This makes literals cheap (1 byte vs 4 in LZ77) and keeps matches at 3 bytes
instead of 4, giving a strict improvement on any repetitive input.

## Package position in the series

```
CMP00 (LZ77,    1977) — Sliding-window back-references.
CMP01 (LZ78,    1978) — Explicit dictionary (trie).
CMP02 (LZSS,    1982) — LZ77 + flag bits. ← THIS PACKAGE
CMP03 (LZW,     1984) — LZ78 + pre-initialised alphabet; GIF.
CMP04 (Huffman, 1952) — Entropy coding.
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG.
```

## Usage

```haskell
import Lzss (compress, decompress)
import qualified Data.ByteString.Char8 as BC

let original   = BC.pack "hello hello hello"
let compressed = compress original
let restored   = decompress compressed

-- restored == original  (True)
```

### Low-level API

```haskell
import Lzss (Token(..), encode, decode,
             defaultWindowSize, defaultMaxMatch, defaultMinMatch)
import qualified Data.ByteString.Char8 as BC

-- Tokenise with default parameters
let tokens = encode defaultWindowSize defaultMaxMatch defaultMinMatch
               (BC.pack "ABABAB")
-- [Literal 65, Literal 66, Match {offset=2, matchLength=4}]

-- Reconstruct from tokens
let bs = decode tokens
-- "ABABAB"
```

## Wire format (CMP02)

```
Bytes 0–3:  original_length  (big-endian uint32)
Bytes 4–7:  block_count      (big-endian uint32)
Bytes 8+:   blocks

Each block:
  [1 byte]   flag byte  — bit i=0: Literal, bit i=1: Match (LSB = first token)
  [variable] symbol data — 1 byte per Literal, 3 bytes per Match
```

## Building and testing

```bash
cabal test all
```

Or via the repo build tool:

```bash
# From repo root
./build-tool
```

## Parameters

| Parameter    | Default | Meaning                                      |
|--------------|---------|----------------------------------------------|
| windowSize   | 4096    | Look-back distance for match search          |
| maxMatch     | 255     | Maximum match length (fits in uint8)         |
| minMatch     | 3       | Minimum match length to emit a Match token   |

## Dependencies

- `base >= 4.14`
- `bytestring >= 0.11`

No other dependencies. The implementation is self-contained.
