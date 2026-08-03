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

Output is a genuine `.zst` frame: it interoperates with the real `zstd` CLI
in both directions (`zstd -d` on our output, and our decompressor on `zstd`'s
output), not just with itself. Getting there required following RFC 8878
§3.1.1.3.2.1.2's Sequences-section bitstream field order exactly — FSE state
init/update order, per-sequence extra-bits order, and the special
no-bits-emitted handling for the very last sequence's state. A decoder that
only round-trips against its own encoder can still be silently
non-conformant; see `CHANGELOG.md` for the specific bugs this caught.

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

## Security

- **Decompression-bomb guard**: total decompressed output is capped at
  256 MB regardless of what the untrusted `Frame_Content_Size` field claims,
  checked incrementally (including inside a Compressed block's per-sequence
  loop, not just once per block — a single ~128 KB block can carry enough
  FSE-coded sequences to expand far past the cap on its own).
- **Block-size cap**: a block header claiming `Block_Size > 128 KB` is
  rejected before any allocation or copy is attempted.
- **Offset bounds**: a match's back-reference offset is validated against
  how much output has been produced so far before any copy happens.
- FSE tables are always the RFC 8878 Appendix B predefined distributions —
  this decoder never parses a table description off the wire, so there's no
  attacker-controlled-table attack surface to validate.

## Running tests

```bash
cd code/packages/kotlin/zstd
gradle test                              # run the test suite
gradle jacocoTestCoverageVerification    # enforce the 80% line-coverage gate
```
