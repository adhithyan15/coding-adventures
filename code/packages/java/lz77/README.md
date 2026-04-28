# lz77 (Java) — CMP00

LZ77 lossless compression for Java.  The foundational sliding-window algorithm
(Lempel & Ziv, 1977) that is the ancestor of LZSS, LZW, DEFLATE, zstd, LZ4,
and virtually every compressor used in ZIP, gzip, PNG, and zlib.

## What it does

`LZ77` exposes:

| Method | Description |
|--------|-------------|
| `compress(byte[])` | Encode bytes into CMP00 wire format |
| `decompress(byte[])` | Decode CMP00 wire format back to original bytes |
| `encode(byte[])` | Encode to `List<Token>` (token stream) |
| `decode(List<Token>)` | Decode token stream back to bytes |

Tokens are `LZ77.Token` records: `(offset, length, nextChar)`.

## The algorithm

```
┌─────────────────────────────────┬──────────────────┐
│         SEARCH BUFFER           │ LOOKAHEAD BUFFER  │
│  (last window_size bytes)       │  (next max_match) │
└─────────────────────────────────┴──────────────────┘
                                  ↑ cursor
```

At each cursor position: search for the longest match in the search buffer.
- Match ≥ `min_match`: emit `Token(offset, length, next_char)`; advance `length+1`.
- No match: emit `Token(0, 0, byte)`; advance 1.

## Wire format (CMP00)

```
Bytes 0–3:  token_count  (big-endian uint32)
Bytes 4+:   token_count × 4 bytes:
              [0–1] offset    (big-endian uint16)
              [2]   length    (uint8)
              [3]   next_char (uint8)
```

## Quick start

```java
byte[] original   = "hello hello hello".getBytes();
byte[] compressed = LZ77.compress(original);
byte[] recovered  = LZ77.decompress(compressed);
assert Arrays.equals(original, recovered);
```

## Running tests

```
gradle test
```

38 tests covering round-trip fidelity, token correctness, wire format,
edge cases, overlapping matches, compression effectiveness, and determinism.
