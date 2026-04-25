# lzw (Java) — CMP03

LZW lossless compression for Java.  LZW (Lempel-Ziv-Welch, 1984) is LZ78
with a pre-seeded 256-entry dictionary, variable-width LSB-first bit-packed
codes, CLEAR/STOP control codes, and the tricky-token edge case handled.
This is the algorithm that powers GIF compression.

## What it does

`LZW` provides:

| Method | Description |
|--------|-------------|
| `compress(byte[])` | Encode bytes into CMP03 wire format |
| `decompress(byte[])` | Decode CMP03 wire format |

## Quick start

```java
byte[] original   = "hello hello hello".getBytes();
byte[] compressed = LZW.compress(original);
byte[] recovered  = LZW.decompress(compressed);
assert Arrays.equals(original, recovered);
```

## Wire format (CMP03)

```
Bytes 0–3:  original_length  (big-endian uint32)
Bytes 4+:   variable-width codes, LSB-first bit-packed

  Code sizes:
    Start at 9 bits (covers codes 0–511)
    Grow when next_code exceeds current power-of-2 boundary
    Maximum 16 bits (dictionary capped at 65536 entries)

  Reserved codes:
    0–255:  Pre-seeded single bytes
    256:    CLEAR_CODE (reset dictionary)
    257:    STOP_CODE  (end of stream)
    258+:   Dynamic entries
```

## Running tests

```
gradle test
```

38+ tests covering round-trip, code stream structure, wire format, edge cases,
tricky token, bit I/O, effectiveness, and determinism.
