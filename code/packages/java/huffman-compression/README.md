# huffman-compression (Java) — CMP04

Huffman lossless compression for Java.  Implements the CMP04 wire format:
variable-length, prefix-free canonical Huffman codes packed LSB-first into a
compact byte stream.

## What it does

`HuffmanCompression` exposes two static methods:

| Method | Description |
|--------|-------------|
| `compress(byte[])` | Encode bytes using canonical Huffman coding; returns CMP04 wire-format bytes |
| `decompress(byte[])` | Decode CMP04 wire-format bytes; returns the original bytes |

## Where it fits

```
CMP00 (LZ77,    1977) — Sliding-window back-references
CMP01 (LZ78,    1978) — Explicit dictionary (trie)
CMP02 (LZSS,    1982) — LZ77 + flag bits
CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; powers GIF
CMP04 (Huffman, 1952) — Entropy coding  ← this package
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard
```

Huffman coding exploits symbol *statistics* (frequent symbols get short codes,
rare symbols get long codes), while LZ-family algorithms exploit *repetition*
(duplicate substrings).  DEFLATE combines both.

## Dependencies

- `com.codingadventures:huffman-tree` — DT27 Huffman tree data structure
  (tree construction, canonical code derivation).  Declared via Gradle
  composite build so no Maven publishing is needed.

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

Bit packing is LSB-first (same convention as LZW/GIF): the first bit of the
stream goes into bit 0 (LSB) of the first byte.

## Quick start

```java
byte[] original   = "AAABBC".getBytes();
byte[] compressed = HuffmanCompression.compress(original);
byte[] recovered  = HuffmanCompression.decompress(compressed);
assert Arrays.equals(original, recovered);
```

## Example — "AAABBC"

```
Frequencies:  A=3, B=2, C=1
Canonical codes: A→"0" (len 1), B→"10" (len 2), C→"11" (len 2)

Header:        00 00 00 06  00 00 00 03
Code table:    41 01  42 02  43 02
Bit stream:    "000101011" → 0xA8 0x01
Total:         16 bytes  (raw: 6 bytes — overhead wins for tiny inputs,
               Huffman wins for larger/skewed inputs)
```

## Running tests

```
gradle test
```

42 tests covering round-trip fidelity, exact wire-format bytes, edge cases
(empty, single symbol, null bytes), compression effectiveness, determinism,
and error handling.
