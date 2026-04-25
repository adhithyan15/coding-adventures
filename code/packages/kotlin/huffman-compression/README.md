# huffman-compression (Kotlin) — CMP04

Huffman lossless compression for Kotlin.  Implements the CMP04 wire format:
variable-length, prefix-free canonical Huffman codes packed LSB-first into a
compact byte stream.

## What it does

`HuffmanCompression` (an `object`) exposes two functions:

| Function | Description |
|----------|-------------|
| `compress(ByteArray?)` | Encode bytes using canonical Huffman coding; returns CMP04 wire-format bytes |
| `decompress(ByteArray?)` | Decode CMP04 wire-format bytes; returns the original bytes |

Both functions treat `null` as empty input and return `ByteArray(0)` or an
8-byte zero header respectively.

## Where it fits

```
CMP00 (LZ77,    1977) — Sliding-window back-references
CMP01 (LZ78,    1978) — Explicit dictionary (trie)
CMP02 (LZSS,    1982) — LZ77 + flag bits
CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; powers GIF
CMP04 (Huffman, 1952) — Entropy coding  ← this package
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard
```

## Dependencies

- `com.codingadventures:huffman-tree` (Kotlin) — DT27 Huffman tree.
  Declared via Gradle composite build; no Maven publishing needed.

## Wire format (CMP04)

```
Bytes 0–3:    original_length  (big-endian uint32)
Bytes 4–7:    symbol_count     (big-endian uint32) — distinct symbols
Bytes 8–8+2N: code-lengths table — N × 2 bytes:
                [0] symbol value  (uint8, 0–255)
                [1] code length   (uint8, 1–16)
              Sorted by (code_length, symbol_value) ascending.
Bytes 8+2N+:  bit stream — packed LSB-first, zero-padded to byte boundary.
```

## Quick start

```kotlin
val original   = "AAABBC".toByteArray()
val compressed = HuffmanCompression.compress(original)
val recovered  = HuffmanCompression.decompress(compressed)
check(original.contentEquals(recovered))
```

## Running tests

```
gradle test
```

45 tests covering round-trip fidelity, exact wire-format bytes, edge cases,
compression effectiveness, determinism, and error handling.
