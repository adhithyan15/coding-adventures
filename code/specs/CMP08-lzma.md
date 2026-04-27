# CMP08 — LZMA (Lempel-Ziv-Markov chain Algorithm)

## Overview

LZMA (2001) is a lossless compression algorithm developed by Igor Pavlov for the
**7-Zip** archiver. It combines an **LZ77 match-finder** with **range coding** (an
exact arithmetic coder) and an **adaptive probability model** that updates symbol
probabilities bit-by-bit as it encodes. LZMA achieves compression ratios typically
5–15% better than DEFLATE, at the cost of much higher compression time and memory.

```
Series:
  CMP00 (LZ77,     1977) — Sliding-window backreferences.
  CMP01 (LZ78,     1978) — Explicit dictionary (trie).
  CMP02 (LZSS,     1982) — LZ77 + flag bits; no wasted literals.
  CMP03 (LZW,      1984) — LZ78 + pre-initialised alphabet; GIF.
  CMP04 (Huffman,  1952) — Entropy coding; prerequisite for DEFLATE.
  CMP05 (DEFLATE,  1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
  CMP06 (Brotli,   2013) — DEFLATE successor; HTTP/2 standard.
  CMP07 (ZStd,     2016) — FSE + LZ77; Linux kernel / npm / macOS.
  CMP08 (LZMA,     2001) — Range coding + LZ77; 7-Zip / XZ.  ← YOU ARE HERE
```

LZMA is used in:
- **7-Zip** (.7z container format) — primary compression method
- **XZ** (.xz, .xz streams inside .tar.xz) — the dominant `xz` utility format
- **Linux initramfs** — many distributions compress initramfs with LZMA/XZ
- **NSIS** (Windows installer) and many embedded firmware compressors
- **LZMA2** — a chunked variant of LZMA used inside 7-Zip's newer containers

Note: ZIP uses DEFLATE (CMP05), **not** LZMA. `.tar.xz` uses LZMA2 inside the XZ
container. This spec covers the raw LZMA1 stream format.

## Historical Context

Igor Pavlov designed LZMA for 7-Zip starting in 1999, releasing version 1.0 in 2000.
The key insight was combining LZ77's dictionary matching with **range coding**, a
generalisation of arithmetic coding that operates on arbitrary-probability bit trees
rather than symbol trees. This produces fractional-bit savings for every literal and
match-length token, pushing ratios closer to the information-theoretic entropy limit.

The Markov chain part of the name refers to the **context model**: the probability of
the next bit depends on a context derived from recent decoded bits and the current
encoder state (7 distinct states tracking what the previous token was). This adaptive
model continuously learns the statistical structure of the input without a pre-scan.

## Key Concepts

### Range Coding (Arithmetic Coding Variant)

Huffman coding rounds each symbol's code length to an integer number of bits.
Range coding encodes a sequence of symbols by progressively narrowing a real-valued
interval `[low, high)` inside `[0, 1)`:

```
Analogy:
  Think of the interval as a real number between 0 and 1.
  Each symbol subdivision is like a number system where digits don't have equal weights.
  A 90%-probable symbol takes only 0.15 bits on average; a 1%-probable symbol takes ~6.6 bits.
  The final interval encodes the entire message as a single (very precise) real number.
```

LZMA's range coder uses 32-bit fixed-point arithmetic:

```
Encoder initial state:
  range ← 0xFFFFFFFF
  low   ← 0 (uint64, to detect carry)

Encode one bit with probability prob (11-bit, 0–2047, where 1024 = 50%):
  bound = (range >> 11) * prob

  if actual_bit == 0:
    range = bound
    prob += (2048 - prob) >> 5   # increase probability toward 1.0
  else:
    low  += bound
    range -= bound
    prob -= prob >> 5             # decrease probability toward 0.0

  # Renormalise: keep range ≥ 2^24 to maintain precision
  while range < 0x01000000:
    emit_byte((low >> 24) & 0xFF)
    low   = (low << 8) & 0xFFFFFFFF
    range = (range << 8) & 0xFFFFFFFF

Decoder mirrors exactly:
  Read 5 initialisation bytes into `code` (uint32).
  range ← 0xFFFFFFFF
  For each bit:
    bound = (range >> 11) * prob
    if code < bound:
      bit = 0; range = bound; update prob upward
    else:
      bit = 1; code -= bound; range -= bound; update prob downward
    while range < 0x01000000:
      code = (code << 8) | read_byte()
      range <<= 8
```

### Probability Model — Adaptive Bit Trees

All symbols in LZMA are encoded as sequences of bits, each bit drawn from a separate
`prob[]` context. Contexts are indexed by recent history:

```
Bit trees:
  - A literal is encoded as 8 bits, each from a prob[context][bit_position] table.
  - A match length is encoded as a sequence of mode bits + value bits.
  - An offset distance uses 6 "distance slots" + extra bits for large offsets.
```

The probability tables start at `prob = 1024` (50%) and self-adjust with every decoded
bit. This means **no pre-scan is needed** — the model adapts online.

### LZ77 Matching in LZMA

```
Dictionary (sliding window): 1 KB – 4 GB (set in stream header)
Minimum match length: 2 bytes
Match length range: 2 – 273 bytes (encoded as length codes 0–271)
Distance range: 1 – DictSize
```

**4 Repeat Distance Slots** (similar to ZStd's 3):
- R0, R1, R2, R3 hold the four most recently used match distances
- A "short rep" (match of length 1 at R0) is a common special case

### LZMA States (7 States)

LZMA tracks the type of the most recent token to choose probability contexts:

```
State  Last 2 tokens       Context implication
─────  ──────────────────  ─────────────────────────────────────
0      Lit, Lit            Pure literal run → high literal probs
1      Match, Lit          Literal after back-ref → post-match distrib
2      Rep, Lit            Literal after repeat match
3      ShortRep, Lit       Literal after 1-byte repeat
4      Lit, Match          Back-ref after literal run
5      Lit, Rep            Repeat after literal run
6      Lit, ShortRep       Short repeat after literal run
```

Each state uses different probability tables, so the model learns whether literals
typically follow other literals, or follow matches (common in binary data).

### LZMA2 (brief note)

LZMA2, used in modern 7-Zip containers, is a chunked wrapper around LZMA1:
- Input is divided into chunks (up to 64 KB compressed)
- Each chunk is either Raw (uncompressed), LZMA with full header, or LZMA with reset
- This allows multi-threaded compression and recovery after corruption
- **This spec covers LZMA1 only**; LZMA2 is a straightforward extension

## Wire Format — Raw LZMA Stream

```
Offset  Size  Field
──────  ────  ─────────────────────────────────────────────────────────
0       1     Properties byte = lc + lp*9 + pb*9*5
              lc = literal context bits (0–8, default 3)
              lp = literal position bits (0–4, default 0)
              pb = position bits (0–4, default 2)
1       4     Dictionary_Size (uint32_le)
              Valid values: 2^n or 2^n + 2^(n-1), n = 0..30
              Default: 0x00100000 (1 MB = 2^20)
5       8     Uncompressed_Size (uint64_le)
              0xFFFFFFFFFFFFFFFF = unknown (stream ends with special marker)
13      5     Range coder initialisation (first 5 bytes of compressed payload)
              byte[0] must be 0x00 (reserved / flush byte)
18+     …     Range-coded payload
```

### Properties Byte Encoding

```
lc + lp*9 + pb*9*5

Valid ranges:
  lc ∈ [0, 8]  — how many high bits of the previous literal byte to use as context
  lp ∈ [0, 4]  — which bits of the current byte position affect literal context
  pb ∈ [0, 4]  — which bits of the current byte position affect other probability contexts

Default (lc=3, lp=0, pb=2):
  properties_byte = 3 + 0*9 + 2*9*5 = 3 + 0 + 90 = 93 = 0x5D
```

### Compressed Payload Structure

There is no explicit structure beyond the range-coded bit stream. The decoder reconstructs
the symbol sequence by:
1. Reading the first 5 bytes as range-coder initialisation
2. Decoding one token at a time via the LZMA state machine
3. Each token is: literal | match | short-repeat | rep[0–3] | end-of-stream marker

The end-of-stream marker is a special match with `distance = 0xFFFFFFFF` and `length = 2`
(encoded as match length code 0, distance code for 2^32-1). When `Uncompressed_Size` is
known, the decoder stops after that many bytes without requiring the marker.

## Educational Simplification

| Feature | Full LZMA | Educational |
|---------|-----------|-------------|
| lc / lp / pb | Configurable | Fixed: lc=3, lp=0, pb=2 |
| Dictionary size | 1 KB – 4 GB | Fixed: 1 MB |
| Uncompressed_Size | Known or 0xFF…FF | Always written (known upfront) |
| Match finder | Hash chains / binary trees | Simple hash-chain (2–3 byte hashes) |
| LZMA2 | Yes | No |
| .xz / .7z container | Yes | No — raw LZMA stream only |
| End-of-stream marker | Optional (if size known) | Always emit |

The fixed lc=3/lp=0/pb=2/DictSize=1MB defaults must be encoded in the stream header
exactly so that any compliant LZMA decoder (including `xz --format=lzma`) can decode it.

## Public API

```
compress(data: bytes) → bytes
decompress(data: bytes) → bytes
```

Same interface as CMP00–CMP07.

## Package Naming

| Language   | Package name                 | Module / namespace             |
|------------|------------------------------|--------------------------------|
| Python     | `coding-adventures-lzma`     | `coding_adventures_lzma`       |
| Go         | module `…/go/lzma`           | package `lzma`                 |
| Ruby       | `coding_adventures_lzma`     | `CodingAdventures::Lzma`       |
| TypeScript | `@coding-adventures/lzma`    | `CodingAdventures.Lzma`        |
| Rust       | `coding-adventures-lzma`     | `coding_adventures_lzma`       |
| Elixir     | `:coding_adventures_lzma`    | `CodingAdventures.Lzma`        |
| Lua        | `coding-adventures-lzma`     | `coding_adventures.lzma`       |
| Perl       | `CodingAdventures::Lzma`     | `CodingAdventures::Lzma`       |
| Swift      | `CodingAdventuresLzma`       | `CodingAdventures.Lzma`        |

## Test Cases

### TC-1: Round-trip empty input
```
assert decompress(compress(b"")) == b""
```

### TC-2: Round-trip single byte
```
assert decompress(compress(b"\x42")) == b"\x42"
```

### TC-3: Round-trip all 256 byte values
```
input = bytes(range(256))
assert decompress(compress(input)) == input
```

### TC-4: Round-trip highly repetitive data
```
input = b"A" * 1024
output = decompress(compress(input))
assert output == input
assert len(compress(input)) < 40   # LZMA compresses repetitive data extremely well
```

### TC-5: Round-trip English prose
```
text  = "the quick brown fox jumps over the lazy dog " * 25
input = text.encode("utf-8")
assert decompress(compress(input)) == input
assert len(compress(input)) < len(input) * 0.75   # LZMA beats DEFLATE ratio
```

### TC-6: Round-trip binary blob (PRNG)
```
# LCG: seed=42, a=1664525, c=1013904223, m=2^32
seed = 42
data = []
for _ in range(512):
    seed = (seed * 1664525 + 1013904223) & 0xFFFFFFFF
    data.append(seed & 0xFF)
input = bytes(data)
assert decompress(compress(input)) == input
```

### TC-7: Probability adaptation (shifting statistics)
```
# First half: all 'A'; second half: all 'B'
# Adaptive model should improve ratio over static model
input = b"A" * 512 + b"B" * 512
output = decompress(compress(input))
assert output == input
assert len(compress(input)) < 50   # adaptive coding should compress this very well
```

### TC-8: Repeat distance slots
```
# A string where the same 4 offsets appear repeatedly
chunk = b"HELLO"
spacer = b"-" * 16
input = chunk
for _ in range(20):
    input += spacer + chunk    # chunk always at distance len(spacer)+len(chunk) from last
output = decompress(compress(input))
assert output == input
# Repeat-distance mechanism should encode efficiently
```

### TC-9: Cross-language / interoperability
```
# Compress with our implementation; decompress with `xz --format=lzma -d`
# Compress with `xz --format=lzma`; decompress with our implementation
text = "the quick brown fox jumps over the lazy dog " * 25
# Both directions must round-trip exactly
```
*Manual or subprocess-based. The raw LZMA stream format must be compatible with `xz`.*

### TC-10: Header parsing — known properties
```
# Construct a valid 13-byte LZMA header:
#   properties = 0x5D (lc=3, lp=0, pb=2)
#   dict_size  = 0x00100000 (1 MB, little-endian)
#   uncompressed_size = 5 (little-endian uint64)
# Feed header + valid range-coded payload for b"hello"
# Verify decompress() returns b"hello"
```

## Security Considerations

- **Memory cap**: the dictionary size field is attacker-controlled. Cap at a reasonable
  maximum (e.g., 64 MB) regardless of what the header claims; do not allocate dictionary
  size bytes up front.
- **Uncompressed size**: `Uncompressed_Size = 0xFF…FF` means unknown — in this case cap
  decompressed output at a configurable limit (e.g., 256 MB) before returning an error.
- **Properties byte validation**: `lc + lp > 4` is invalid per the LZMA spec; reject it.
  `pb > 4`, `lp > 4`, `lc > 8` are also invalid.
- **Range coder**: the first byte of the payload must be `0x00`; reject streams where it
  is not (this is a diagnostic check, not data).
- **Distance bounds**: a match offset `> bytes_decoded_so_far` would read before the
  start of the dictionary; this is a corrupt stream — return an error.
- **Probability table bounds**: `prob` values are maintained in `[1, 2047]` by the update
  rule; they never underflow or overflow when implemented correctly, but validate on read
  if loading a serialised model.
