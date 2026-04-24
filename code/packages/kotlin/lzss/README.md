# kotlin/lzss — CMP02

Kotlin implementation of the LZSS (Lempel-Ziv-Storer-Szymanski, 1982)
lossless compression algorithm — package **CMP02** in the coding-adventures
compression series.

## What is LZSS?

LZSS is a refinement of the LZ77 sliding-window algorithm.  LZ77 (1977)
always emitted a three-part tuple `(offset, length, next_char)`, which wasted
bytes when no back-reference existed.  LZSS replaces the mandatory next-char
field with a **flag-bit scheme**: every group of up to 8 tokens shares a
single flag byte, where each bit records whether the corresponding token is a
raw Literal byte (bit=0) or a back-reference Match (bit=1).

Break-even analysis:

| Token type | Wire cost  | Worth it when…               |
|------------|-----------|-------------------------------|
| Literal    | 1 byte    | always (no match ≥ 3 found)   |
| Match      | 3 bytes   | match length ≥ 3              |

## Compression series

```
CMP00 (LZ77,    1977) — Sliding-window back-references.
CMP01 (LZ78,    1978) — Explicit dictionary (trie).
CMP02 (LZSS,    1982) — LZ77 + flag bits.  ← this package
CMP03 (LZW,     1984) — LZ78 + pre-initialised alphabet; GIF.
CMP04 (Huffman, 1952) — Entropy coding.
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP / gzip / PNG / zlib.
```

## Wire format (CMP02)

```
Bytes 0–3  : original_length  (big-endian uint32)
Bytes 4–7  : block_count      (big-endian uint32)
Bytes 8+   : blocks
  Each block:
    [flag_byte]    1 byte — bit i=0 → Literal, bit i=1 → Match
    [symbol data]  1 byte per Literal | 2-byte offset (BE) + 1-byte length per Match
```

## Usage

```kotlin
import com.codingadventures.lzss.Lzss

// One-shot compression / decompression
val original   = "hello hello hello".encodeToByteArray()
val compressed = Lzss.compress(original)
val restored   = Lzss.decompress(compressed)
assert(original.contentEquals(restored))

// Token-level access
val tokens  = Lzss.encode(original)
val decoded = Lzss.decode(tokens)
```

## Token types

```kotlin
sealed class LzssToken {
    data class Literal(val value: Byte)              : LzssToken()
    data class Match(val offset: Int, val length: Int) : LzssToken()
}
```

## Configuration

```kotlin
Lzss.encode(
    data       = myBytes,
    windowSize = 4096,   // search window (default: 4096)
    maxMatch   = 255,    // max match length (default: 255)
    minMatch   = 3       // min match length (default: 3)
)
```

## Running tests

```bash
cd code/packages/kotlin/lzss
gradle test
```

## Cross-language compatibility

The CMP02 wire format is identical across all language implementations in this
monorepo (Rust `lzss`, C# `lzss`, Java `lzss`, Kotlin `lzss`).  A stream
compressed by any one implementation can be decompressed by any other.
