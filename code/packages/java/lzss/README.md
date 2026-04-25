# lzss (Java) — CMP02

LZSS lossless compression for Java.  LZSS (Lempel-Ziv-Storer-Szymanski, 1982)
is a refinement of LZ77 that eliminates the wasted literal byte present in every
LZ77 token by replacing the fixed `(offset, length, next_char)` triple with a
flag-bit scheme: each symbol is either a bare literal byte or a bare
back-reference, never both.

## What it does

`LZSS` provides:

| Method | Description |
|--------|-------------|
| `compress(byte[])` | Encode bytes into CMP02 wire format |
| `decompress(byte[])` | Decode CMP02 wire format |
| `encode(byte[])` | Encode to `List<Token>` |
| `decode(List<Token>, int)` | Decode token stream |

## Token types

| Type | Fields | Wire size |
|------|--------|-----------|
| `Literal` | `value` (0–255) | 1 byte |
| `Match` | `offset` (1..4096), `length` (3..255) | 3 bytes |

## Quick start

```java
byte[] original   = "hello hello hello".getBytes();
byte[] compressed = LZSS.compress(original);
byte[] recovered  = LZSS.decompress(compressed);
assert Arrays.equals(original, recovered);
```

## Wire format (CMP02)

```
Bytes 0–3:  original_length  (big-endian uint32)
Bytes 4–7:  block_count      (big-endian uint32)
Bytes 8+:   blocks

Each block:
  [1 byte]  flag_byte (bit i → 0=Literal, 1=Match)
  [variable] symbol data:
      Literal: 1 byte  (byte value)
      Match:   3 bytes (offset BE uint16 + length uint8)
```

## Running tests

```
gradle test
```

35+ tests covering round-trip, token stream, wire format, edge cases,
overlapping matches, effectiveness, and determinism.
