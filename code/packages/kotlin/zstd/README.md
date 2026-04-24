# kotlin/zstd — CMP07

Pure-Kotlin implementation of the Zstandard (ZStd) lossless compression
algorithm (RFC 8878), part of the **CMP** series in coding-adventures.

## What it does

ZStd combines two techniques for high-ratio, high-speed compression:

1. **LZ77 back-references** — finds repeated byte sequences and encodes them
   as (offset, length) pairs instead of literal bytes. Implemented via the
   `com.codingadventures:lzss` package with a 32 KB sliding window.

2. **FSE (Finite State Entropy)** — encodes the sequence descriptors (literal
   length, match length, match offset codes) using predefined asymmetric
   numeral system tables that approach the Shannon entropy limit in a single
   pass. No per-frame Huffman or FSE table description is transmitted.

## Usage

```kotlin
import com.codingadventures.zstd.Zstd

val original = "the quick brown fox jumps over the lazy dog".encodeToByteArray()
val compressed = Zstd.compress(original)
val restored = Zstd.decompress(compressed)
assert(original.contentEquals(restored))
```

## API

```kotlin
object Zstd {
    fun compress(data: ByteArray): ByteArray
    fun decompress(data: ByteArray): ByteArray  // throws IOException on corrupt input
}
```

## Frame format

```
┌────────┬─────┬──────────────────────┬────────┬──────────────────┐
│ Magic  │ FHD │ Frame_Content_Size   │ Blocks │ [Checksum]       │
│ 4 B LE │ 1 B │ 1/2/4/8 B (LE)      │ ...    │ 4 B (optional)   │
└────────┴─────┴──────────────────────┴────────┴──────────────────┘
```

Each block header is 3 bytes LE:
- bit 0: Last_Block flag
- bits [2:1]: Block_Type (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
- bits [23:3]: Block_Size

## Compression series

```
CMP00 (LZ77)     — Sliding-window back-references
CMP01 (LZ78)     — Explicit dictionary (trie)
CMP02 (LZSS)     — LZ77 + flag bits
CMP03 (LZW)      — LZ78 + pre-initialised alphabet; GIF
CMP04 (Huffman)  — Entropy coding
CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
CMP06 (Brotli)   — DEFLATE + context modelling + static dict
CMP07 (ZStd)     — LZ77 + FSE; high ratio + speed  ← this package
```

## Running tests

```bash
cd code/packages/kotlin/zstd
gradle test
```
