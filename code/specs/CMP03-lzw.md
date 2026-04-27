# CMP03 — LZW

## Overview

LZW (Lempel-Ziv-Welch, 1984) is a refinement of LZ78 (CMP01) that eliminates LZ78's
mandatory `next_char` byte by **pre-seeding the dictionary** with all 256 single-byte
sequences. Because every possible byte already has a dictionary code (0–255), the encoder
never needs to emit a raw literal alongside a back-reference — every emitted symbol is a
dictionary code.

This small change has large consequences:
- Tokens shrink from `(dict_index, next_char)` tuples to just **codes** (unsigned integers)
- Output is a pure code stream, enabling **variable-width bit-packing** — the de-facto
  standard used in GIF (1987), TIFF, and Unix `compress`
- Compression typically improves 10–30% over LZ78 on typical text

```
Series:
  CMP00 (LZ77,    1977) — Sliding-window backreferences.
  CMP01 (LZ78,    1978) — Explicit dictionary (trie).
  CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
  CMP03 (LZW,     1984) — LZ78 + pre-initialised alphabet; GIF.   ← YOU ARE HERE
  CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
  CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
```

## Historical Context

Terry Welch published "A Technique for High-Performance Data Compression" in the June 1984
issue of *IEEE Computer*, describing the algorithm as an extension of LZ78. Sperry (later
Unisys) held a patent on LZW until the 2000s, which caused significant controversy in the
open-source community when GIF's LZW compression became subject to licensing requirements.
The patent expired (US) in 2003, opening LZW to royalty-free use.

GIF (Graphics Interchange Format, 1987) used a variant of LZW with:
- A palette-size-dependent initial code size (not always 8 bits)
- A "clear code" and "end-of-information code" as reserved codes
- LSB-first bit packing within bytes

This implementation uses the same conventions, generalised to 8-bit input (alphabet = 256).

## Key Concepts

### The Pre-Seeded Dictionary

LZ78 starts with an empty dictionary and emits `(0, byte)` for every literal that has no
dictionary match. This means single-byte sequences always cost 3 bytes (2 for index, 1 for
the literal) until the dictionary fills in.

LZW initialises the dictionary upfront:

```
Code   Sequence
─────  ────────
0      [0x00]
1      [0x01]
...
255    [0xFF]
256    CLEAR_CODE  — instructs decoder to reset to initial 256-entry state
257    STOP_CODE   — marks end of code stream
258    first dynamically added entry
259    second dynamically added entry
...
```

Because every byte is already in the dictionary, the encoder can always emit a valid code
and then attempt to extend the current prefix — it never needs to emit a raw byte outside
the code stream.

### Variable-Width Codes

With LZ78's fixed 4-byte tokens, all codes are 16 bits. In LZW the token stream contains
only codes (no `next_char`), so we can pack them at their natural bit-width:

```
Codes 0–257:    need 9 bits (2^9 = 512 > 258)
Codes 258–511:  still 9 bits
Codes 512–1023: need 10 bits
...
Codes 32768–65535: need 16 bits
```

The encoder grows `code_size` from 9 upward as `next_code` crosses each power-of-2
boundary. The decoder mirrors this exactly, so both sides always agree on the current width.

### CLEAR and STOP Codes

Two reserved codes manage the code stream:

| Code | Value | Meaning |
|------|-------|---------|
| CLEAR_CODE | 256 | Reset dictionary to 256 pre-seeded entries; restart with `next_code = 258`, `code_size = 9` |
| STOP_CODE  | 257 | End of compressed data; decoder should stop reading |

The encoder emits CLEAR_CODE at the start of every stream and whenever the dictionary
reaches its maximum size (next_code == 2^max_code_size). The encoder emits STOP_CODE as
the very last code.

### The "Tricky Token" Edge Case

LZW has a famous decoder edge case that does not exist in LZ78. Consider encoding the
string `"ABABAB"`:

```
Step  Input  Match  New entry  Code emitted
────  ─────  ─────  ─────────  ────────────
1     A      "A"=65  —          —
2     B      no match           65 ("A"); add "AB"=258; w="B"
3     A      no match           66 ("B"); add "BA"=259; w="A"
4     B      "AB"=258  —        —
5     A      no match           258 ("AB"); add "ABA"=260; w="A"
6     EOF                       65 ("A"); STOP
```

Now consider `"AAAAAA"`:

```
Step  Input  Match  New entry  Code emitted
────  ─────  ─────  ─────────  ────────────
1     A      "A"=65  —          —
2     A      no match           65 ("A"); add "AA"=258; w="A"
3     A      "AA"=258  —        —
4     A      no match           258 ("AA"); add "AAA"=259; w="A"
5     A      "AA"=258  — (still in progress)
6     EOF                       258 ("AA"); STOP
```

During decoding, when the decoder reads code 259 ("AAA"), it tries to look up entry 259 in
its table — but entry 259 was just added in step 4 as `dict[prev_code] + entry[0]`. The
decoder receives code 259 **before** it has finished adding entry 259 to its own dict.

This is the "tricky token" (also called the "SC == NC" case — self-referential code):

```
When the decoder receives code C and C == next_code (the code about to be added):
  entry = dict[prev_code] + dict[prev_code][0]   ← first byte of previous entry
```

This works because any self-referential code must encode a sequence that starts and ends
with the same byte as the previous match (by construction of the encoder's loop).

## Algorithm

### Encoding

```
CONSTANTS:
  CLEAR_CODE    = 256
  STOP_CODE     = 257
  INITIAL_NEXT  = 258
  MAX_CODE_SIZE = 16          (65536 max entries)

INIT:
  dict: HashMap<Vec<u8>, u16>
  for b in 0..=255: dict[&[b]] = b as u16
  next_code  = INITIAL_NEXT
  code_size  = 9
  bit_writer = BitWriter::new()

ENCODE:
  bit_writer.write(CLEAR_CODE, code_size)
  w = Vec::new()

  for each byte b in input:
    w.push(b)
    if w not in dict:
      w.pop()                           // remove b; w is the longest known prefix
      bit_writer.write(dict[w], code_size)

      if next_code < 2^MAX_CODE_SIZE:
        let mut new_entry = w.clone()
        new_entry.push(b)
        dict[new_entry] = next_code
        next_code += 1
        if next_code > 2^code_size:
          code_size += 1
      elif next_code == 2^MAX_CODE_SIZE:
        bit_writer.write(CLEAR_CODE, code_size)
        dict.clear()
        for b in 0..=255: dict[&[b]] = b as u16
        next_code = INITIAL_NEXT
        code_size = 9

      w = vec![b]                       // restart with unmatched byte

  // flush remaining prefix
  if w not empty:
    bit_writer.write(dict[w], code_size)

  bit_writer.write(STOP_CODE, code_size)
  bit_writer.flush()
  return bit_writer.bytes()
```

### Decoding

```
INIT:
  dict: Vec<Vec<u8>>          (index = code, value = sequence)
  for b in 0..=255: dict.push(vec![b])
  dict.push(vec![])           // slot 256 = CLEAR_CODE placeholder
  dict.push(vec![])           // slot 257 = STOP_CODE placeholder
  next_code  = 258
  code_size  = 9
  prev_code  = None
  output     = Vec::new()
  bit_reader = BitReader::new(compressed_bytes)

  first_code = bit_reader.read(code_size)
  assert first_code == CLEAR_CODE        // well-formed stream starts with CLEAR

DECODE LOOP:
  loop:
    code = bit_reader.read(code_size)

    if code == CLEAR_CODE:
      dict.truncate(258)
      next_code = 258
      code_size = 9
      prev_code = None
      continue

    if code == STOP_CODE:
      break

    // Resolve entry
    if code < dict.len():
      entry = dict[code].clone()
    elif code == next_code:              // tricky token
      entry = dict[prev_code.unwrap()].clone()
      entry.push(entry[0])
    else:
      return Err("invalid code")

    output.extend_from_slice(&entry)

    // Add new entry
    if let Some(prev) = prev_code:
      if next_code < 2^MAX_CODE_SIZE:
        let mut new_entry = dict[prev].clone()
        new_entry.push(entry[0])
        dict.push(new_entry)            // dict[next_code] = new entry
        next_code += 1
        if next_code > 2^code_size and code_size < MAX_CODE_SIZE:
          code_size += 1

    prev_code = Some(code)

  return output
```

## Wire Format (CMP03)

```
Bytes 0–3:  original_length — BE uint32. Length of the uncompressed data.
Bytes 4+:   bit-packed variable-width codes, LSB-first within each byte.

Code stream structure:
  1. CLEAR_CODE (256) at code_size = 9
  2. Data codes, each at the current code_size
  3. STOP_CODE  (257) at the current code_size (may be > 9 if dict grew)
  4. Zero-padding bits to align the final byte boundary
```

### Why `original_length`?

STOP_CODE terminates the code stream, but the final bit-packed byte may contain trailing
zero-padding bits. The decoder could produce a few extra zero bytes after STOP_CODE if the
partial byte happens to encode valid sequences. `original_length` lets the decoder trim
any such trailing bytes and return exactly the original data.

### Bit-Packing Convention (LSB-first)

Bits within each byte are filled from the **least significant bit** upward. This matches
the GIF specification and the Unix `compress` tool.

```
write_code(code: u16, code_size: u8, state: &mut BitState):
  state.buffer |= (code as u64) << state.bit_pos
  state.bit_pos += code_size
  while state.bit_pos >= 8:
    emit byte (state.buffer & 0xFF)
    state.buffer >>= 8
    state.bit_pos -= 8

flush(state: &mut BitState):
  if state.bit_pos > 0:
    emit byte (state.buffer & 0xFF)
    state.bit_pos = 0
    state.buffer = 0
```

Reading mirrors writing:

```
read_code(code_size: u8, state: &mut BitState, bytes: &[u8]) -> u16:
  while state.bit_pos < code_size:
    state.buffer |= (next_byte as u64) << state.bit_pos
    state.bit_pos += 8
  code = (state.buffer & ((1 << code_size) - 1)) as u16
  state.buffer >>= code_size
  state.bit_pos -= code_size
  return code
```

### Byte-Cost Analysis

```
Header:         4 bytes (original_length)
Per code:       variable (9–16 bits)
CLEAR_CODE:     always emitted at start and on dict-full reset
STOP_CODE:      1 code at current code_size

For highly repetitive data (long runs of same byte):
  "AAAAAAA...A" (N bytes) → O(log N) codes as dict grows
  Compression ratio approaches log(N)/N

For incompressible data (random bytes):
  Every byte is its own code → N codes at 9 bits each → ~12.5% expansion
  (vs. LZ78: N tokens × 4 bytes = 4× expansion for pure literals)
```

## Parameters

| Constant | Value | Meaning |
|----------|-------|---------|
| `CLEAR_CODE` | 256 | Reset code; always at position 0 in the code stream |
| `STOP_CODE`  | 257 | End-of-stream code |
| `INITIAL_NEXT_CODE` | 258 | First dynamically assigned code |
| `INITIAL_CODE_SIZE` | 9 | Starting bit-width of codes |
| `MAX_CODE_SIZE` | 16 | Maximum bit-width; dict caps at 65536 entries |

## Test Vectors

All vectors assume `max_code_size = 16`. Codes shown are the logical values; actual bytes
depend on bit-packing.

### Vector 1 — Empty input

```
Input:  b""
Codes:  CLEAR(256), STOP(257)
Output: 4-byte header (original_length=0) + 3 bytes (18 bits packed + padding)
Round-trip: decompress(compress(b"")) == b""
```

### Vector 2 — Single byte

```
Input:  b"A"  (0x41 = 65)
Codes:  CLEAR(256), 65, STOP(257)
Output: header + codes at 9-bit each
Round-trip: decompress(compress(b"A")) == b"A"
```

### Vector 3 — Two distinct bytes, no repetition

```
Input:  b"AB"
Codes:  CLEAR(256), 65(A), 66(B), STOP(257)
        dict after: 258 = "AB" (added but never emitted)
Round-trip check passes.
```

### Vector 4 — Repeated pair ("ABABAB")

```
Input:  b"ABABAB"

Encoding trace:
  w=""
  b=A: w="A" → in dict
  b=B: w="AB" → not in dict → emit 65("A"); add 258="AB"; w="B"
  b=A: w="BA" → not in dict → emit 66("B"); add 259="BA"; w="A"
  b=B: w="AB" → in dict (258)
  b=A: w="ABA" → not in dict → emit 258("AB"); add 260="ABA"; w="A"
  b=B: w="AB" → in dict (258)
  EOF:  emit 258("AB"); STOP

Codes:  CLEAR, 65, 66, 258, 258, STOP

Decoded:
  65  → "A"
  66  → "B"; dict[258]="AB"
  258 → "AB"; dict[259]="BA"
  258 → "AB"; dict[260]="ABA"
  → output = "A" + "B" + "AB" + "AB" = "ABABAB" ✓
```

### Vector 5 — All-same bytes ("AAAAAAA" — 7 bytes)

Demonstrates the tricky-token case in the decoder:

```
Input:  b"AAAAAAA"

Encoding trace:
  b=A: w="A" → in dict
  b=A: w="AA" → not in dict → emit 65("A"); add 258="AA"; w="A"
  b=A: w="AA" → in dict (258)
  b=A: w="AAA" → not in dict → emit 258("AA"); add 259="AAA"; w="A"
  b=A: w="AA" → in dict (258)
  b=A: w="AAA" → in dict (259)
  b=A: w="AAAA" → not in dict → emit 259("AAA"); add 260="AAAA"; w="A"
  EOF:  emit 65("A"); STOP

Codes:  CLEAR, 65, 258, 259, 65, STOP

Decoding trace (tricky-token at code 259):
  65  → "A"   ; prev=65
  258 → "AA"  ; add dict[258]="A"+"A"="AA" (prev=65, entry[0]='A') → wait, 258 is
        already in dict (next_code is 258, dict grows to 259 after this step)
        Actually: receive 258; dict has 258 entries (0-257); 258 == next_code (258)
        → tricky token: entry = dict[65]+"A" = "A"+"A" = "AA"
        output "AA"; add dict[258]="A"+"A"="AA"; next_code=259; prev=258
  259 → code 259; dict has 259 entries; 259 == next_code (259)
        → tricky token: entry = dict[258][0..]+"A"[0] = "AA"+"A" = "AAA"

  Wait — let me re-trace carefully:

  Initialisation: dict[0..255] = single bytes; dict[256]=CLEAR; dict[257]=STOP; next_code=258
  Read CLEAR: reset. next_code=258. code_size=9. prev=None.

  Read 65:
    entry = dict[65] = "A"
    output "A"
    prev=None → no new entry added (need prev to build new entry)
    prev = 65

  Read 258:
    258 >= dict.len (258) and 258 == next_code (258) → TRICKY TOKEN
    entry = dict[prev=65] + dict[65][0] = "A" + 'A' = "AA"
    output "AA"
    prev=65 → add dict[258] = dict[65] + entry[0] = "A" + 'A' = "AA"; next_code=259
    if 259 > 2^9=512? No. code_size stays 9.
    prev = 258

  Read 259:
    259 >= dict.len (259) and 259 == next_code (259) → TRICKY TOKEN
    entry = dict[prev=258] + dict[258][0] = "AA" + 'A' = "AAA"
    output "AAA"
    prev=258 → add dict[259] = dict[258] + entry[0] = "AA" + 'A' = "AAA"; next_code=260
    prev = 259

  Read 65:
    entry = dict[65] = "A"
    output "A"
    prev=259 → add dict[260] = dict[259] + entry[0] = "AAA" + 'A' = "AAAA"; next_code=261
    prev = 65

  Read STOP → done.

  Output: "A" + "AA" + "AAA" + "A" = "AAAAAAA" ✓
```

### Vector 6 — Round-trip, all bytes

```
Round-trip property:
  for all byte strings s:
    decompress(compress(s)) == s
```

## Comparison Table

| Property | LZ77 (CMP00) | LZ78 (CMP01) | LZSS (CMP02) | LZW (CMP03) |
|----------|-------------|-------------|-------------|------------|
| Dictionary | Sliding window | Trie, empty start | Sliding window | Trie, 256 pre-seeded |
| Token type | (offset,len,char) | (dict_idx,char) | Literal \| Match | code (u16) |
| Token size | Fixed 4 bytes | Fixed 4 bytes | 1 or 3 bytes | 9–16 bits |
| Literal cost | 4 bytes | 3 bytes | 1 byte | 9 bits |
| Match cost | 4 bytes | ≥ 4 bytes | 3 bytes | ≤ 9 bits |
| Wire format | Fixed records | Fixed records | Flag blocks | Bit-packed |
| Patent history | None | None | None | Unisys 1985–2003 |
| Real usage | base for DEFLATE | base for LZW | embedded systems | GIF, TIFF |

## Implementation Notes

### Dictionary Representation

**Encoder:** A flat `HashMap<Vec<u8>, u16>` mapping byte sequences to codes. Sequences grow
one byte at a time; the encoder never needs parent-chain traversal.

**Decoder:** A `Vec<Vec<u8>>` (array indexed by code). Each new entry is built as
`dict[prev_code].clone() + [entry[0]]`. No trie cursor needed — parent chains are flattened
at construction time.

This is simpler than LZ78, which needed a TrieCursor abstraction. LZW can use plain hash
maps and arrays.

### Bit I/O

Each language implementation must provide private `BitWriter` and `BitReader` helpers:

```
BitWriter:
  buffer: u64      (accumulates bits)
  bit_pos: u8      (how many valid bits are in buffer)
  bytes: Vec<u8>   (output bytes emitted so far)

  write(code: u16, size: u8):
    buffer |= (code as u64) << bit_pos
    bit_pos += size
    while bit_pos >= 8:
      bytes.push(buffer & 0xFF)
      buffer >>= 8
      bit_pos -= 8

  flush():
    if bit_pos > 0:
      bytes.push(buffer & 0xFF)

BitReader:
  data: &[u8]
  pos: usize       (byte index)
  buffer: u64
  bit_pos: u8

  read(size: u8) -> u16:
    while bit_pos < size:
      buffer |= (data[pos] as u64) << bit_pos
      pos += 1
      bit_pos += 8
    code = (buffer & ((1 << size) - 1)) as u16
    buffer >>= size
    bit_pos -= size
    return code
```

These helpers are private to each package — they are not a separate shared library.

### Security: Decoder Input Validation

The decoder must guard against malformed input:

1. **Invalid code:** If `code > next_code`, return an error — this is not the tricky-token
   case and represents a malformed stream.
2. **Missing CLEAR_CODE at start:** If the first non-CLEAR code arrives before any CLEAR,
   reject or skip gracefully.
3. **Dict growth cap:** The decoder's dict grows to at most 65536 entries before CLEAR is
   expected; a stream that never emits CLEAR after hitting 65536 is malformed — stop adding
   entries (do not panic or grow unboundedly).
4. **Buffer bounds:** BitReader must not read past the end of the input byte slice.

### Language-Specific Notes

**Python:** No dependency on `trie` package. `BitWriter`/`BitReader` as plain classes.
`BUILD` does NOT need `uv pip install -e ../trie`.

**Elixir:** Remember `import Bitwise` for `|||`, `&&&`, `<<<`.

**Swift:** Create `.gitignore` with `.build/` **before** running any Swift commands.

**Swift (Windows CI):** `BUILD_windows` must use:
```
where swift >nul 2>nul && swift test || echo Swift not available on this runner — skipping
```

**Rust:** Add `"lzw"` to `code/packages/rust/Cargo.toml` workspace members.

## Package Matrix

| Language | Package name | Module / import path |
|----------|-------------|----------------------|
| Python | `coding-adventures-lzw` | `coding_adventures_lzw` |
| Go | `coding-adventures-lzw` | `github.com/adhithyan15/coding-adventures/code/packages/go/lzw` |
| Ruby | `coding_adventures_lzw` | `coding_adventures/lzw` |
| TypeScript | `@coding-adventures/lzw` | `@coding-adventures/lzw` |
| Rust | `coding-adventures-lzw` | `coding_adventures_lzw` |
| Elixir | `:coding_adventures_lzw` | `CodingAdventures.LZW` |
| Lua | `coding-adventures-lzw` | `coding_adventures.lzw` |
| Perl | `CodingAdventures-LZW` | `CodingAdventures::LZW` |
| Swift | `LZW` | `import LZW` |
