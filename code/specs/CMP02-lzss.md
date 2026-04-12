# CMP02 — LZSS

## Overview

LZSS (Lempel-Ziv-Storer-Szymanski, 1982) is a refinement of LZ77 (CMP00) that
eliminates a systematic waste in LZ77's output: in LZ77, every token emits a
`next_char` byte even when a long back-reference was found. Storer and Szymanski
showed that this is unnecessary — a flag bit can distinguish literals from
back-references, so each symbol is **one or the other, never both**.

The result is a strictly better encoding than LZ77 for the same underlying
sliding-window algorithm: literals cost 1 byte instead of 4, and back-references
cost 3 bytes instead of 4.

```
Series:
  CMP00 (LZ77, 1977) — Sliding-window backreferences.           ← predecessor
  CMP01 (LZ78, 1978) — Explicit dictionary (trie).
  CMP02 (LZSS, 1982) — LZ77 + flag bits; no wasted literals.   ← YOU ARE HERE
  CMP03 (LZW,  1984) — LZ78 + pre-initialised alphabet; GIF.
  CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
  CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
```

## Key Concepts

### Why LZ77 Is Wasteful

LZ77 tokens have the form `(offset, length, next_char)`:

```
LZ77 token for "hello" (5 bytes → copy 4 from window, then 'o')
──────────────────────────────────────────────────────────────
  offset  = 7    (7 bytes back in the search buffer)
  length  = 4    (copy 4 bytes)
  next_char = 'o'  ← always present, even though 'o' could be a separate literal
```

This is 4 bytes per token regardless. For a long repetition like 255-byte match,
the token emits 3 bytes of match metadata plus 1 byte literal — the literal is
wasted; it only exists to keep the cursor advancing.

### The LZSS Fix: Flag Bits

LZSS replaces the token stream with a mixed stream of two kinds of symbols:

```
Literal    — 1 byte : the actual byte value
Match      — 3 bytes: offset (2 bytes) + length (1 byte)
```

A **flag byte** precedes each group of 8 symbols, using 1 bit per symbol
(LSB = first symbol). Bit `0` → literal, bit `1` → back-reference match.

```
Flag byte:  0 0 1 0 0 0 1 0    (bits 7→0)
            │ │ │ │ │ │ │ └── symbol 0: literal (1 byte follows)
            │ │ │ │ │ │ └──── symbol 1: literal
            │ │ │ │ │ └────── symbol 2: match  (3 bytes follow)
            │ │ │ │ └──────── symbol 3: literal
            │ │ │ └────────── symbol 4: literal
            │ │ └──────────── symbol 5: literal
            │ └────────────── symbol 6: match
            └──────────────── symbol 7: literal
```

8 literals in this group cost 1 (flag) + 8 × 1 = 9 bytes.
8 matches in this group cost 1 (flag) + 8 × 3 = 25 bytes.
Compare with LZ77: either case would cost 8 × 4 = 32 bytes.

### The Break-Even Point

Encoding a match in LZSS costs 3 bytes (offset + length).
Encoding a literal costs 1 byte.
A 3-byte match stored as a match costs exactly the same as 3 literals — no gain.
The match must cover **at least 3 bytes** (`min_match = 3`) to be worth using.

```
Length 1:  match costs 3 bytes, literal costs 1 byte → use literal
Length 2:  match costs 3 bytes, 2 literals cost 2 bytes → use literals
Length 3:  match costs 3 bytes, 3 literals cost 3 bytes → break even; use literal
Length 4+: match costs 3 bytes, 4+ literals cost 4+ bytes → use match
```

So `min_match = 4` gives a strict saving; `min_match = 3` is traditionally
used as the threshold (break-even), equivalent to LZ77's practice.

### Comparison: LZ77 vs. LZSS

```
Input: "ABABABABABABABAB" (16 bytes, pattern "AB" × 8)

LZ77 tokens:
  (0, 0, 'A')       → 4 bytes  (literal A)
  (0, 0, 'B')       → 4 bytes  (literal B)
  (2, 13, '\0')     → 4 bytes  (back-ref + sentinel)
Total: 12 bytes

LZSS tokens:
  [flag=0b00000011]  (1 byte: symbols 0–1 are literals, symbols 2 is a match)
  'A'                (1 byte)
  'B'                (1 byte)
  offset=2, length=14  (3 bytes)
Total: 6 bytes
```

LZSS saves the extra literal bytes. On highly repetitive data, savings approach
75% compared to LZ77.

## Token Types

LZSS uses two distinct token types (unlike LZ77's single three-field struct):

```
Literal(byte: int)           — a single byte, 0–255
Match(offset: int, length: int)
  offset ∈ [1, window_size]  — how many bytes back the match starts
  length ∈ [min_match, max_match]  — how many bytes to copy
```

There is no `next_char` field. A match followed by a new byte simply emits two
tokens: `Match(...)` then `Literal(new_byte)`.

## Wire Format (CMP02)

```
Bytes 0–3:  original_length — BE uint32. Length of the uncompressed data.
Bytes 4–7:  block_count — BE uint32. Number of flag blocks.
Bytes 8+:   blocks

Each block:
  [1 byte]  flag_byte — 8 bits, LSB = first symbol.
              bit i = 0 → symbol i is a Literal  (1 byte follows)
              bit i = 1 → symbol i is a Match    (3 bytes follow)
  [variable] symbol data — 1 or 3 bytes per symbol, in order.

The last block may contain fewer than 8 symbols.
Unused flag bits in the last block are 0.
```

### Why `original_length`?

LZ77 could omit this because `next_char` always advanced by exactly one byte per
token, making the output length deterministic. In LZSS, a trailing match may
expand to multiple bytes; without `original_length`, the decoder cannot
distinguish valid trailing bytes from alignment padding in the last block.

### Byte costs at a glance

```
Preamble:  8 bytes
Per block: 1 byte (flag) + (N_literals × 1) + (N_matches × 3)
Max 8 symbols per block → max 1 + 8×3 = 25 bytes per block.
```

On all-literal input (worst case for LZSS), every block is 9 bytes covering 8
input bytes — 12.5% overhead. This is the trade-off versus LZ77's fixed 4 bytes
per symbol with the sliding window.

## Encoding Algorithm

```
function encode(data: bytes,
                window_size: int = 4096,
                max_match:   int = 255,
                min_match:   int = 3) -> list[Token]:

    tokens  ← []
    cursor  ← 0

    while cursor < len(data):

        # Search the window for the longest match.
        win_start ← max(0, cursor - window_size)
        (offset, length) ← find_longest_match(
                                data, cursor,
                                win_start, max_match)

        if length >= min_match:
            # Good enough match — emit a back-reference.
            tokens.append(Match(offset=offset, length=length))
            cursor ← cursor + length
        else:
            # No useful match — emit the byte as a literal.
            tokens.append(Literal(byte=data[cursor]))
            cursor ← cursor + 1

    return tokens


function find_longest_match(data, cursor, win_start, max_match)
        -> (offset, length):

    best_len    ← 0
    best_offset ← 0

    for pos in range(win_start, cursor):
        length ← 0
        while (length < max_match
               and cursor + length < len(data)
               and data[pos + length] == data[cursor + length]):
            length ← length + 1

        if length > best_len:
            best_len    ← length
            best_offset ← cursor - pos   # distance back

    return (best_offset, best_len)
```

Key difference from LZ77: when a match is found, we advance `cursor` by
`length` — not `length + 1`. There is no trailing `next_char`.

### Worked Example 1: "AABCBBABC"

Window size 4096, min_match 3. The same input used in LZ77 and LZ78 examples.

```
Input: A  A  B  C  B  B  A  B  C
       0  1  2  3  4  5  6  7  8

cursor=0: window empty → Literal('A')
cursor=1: window=[A], match A@1: length=1 < 3 → Literal('A')
cursor=2: window=[AA], match B → no → Literal('B')
cursor=3: window=[AAB], match C → no → Literal('C')
cursor=4: window=[AABC], match B@1(from pos3): length=1 < 3 → Literal('B')
cursor=5: window=[AABCB], match B@1: length=1 < 3 → Literal('B')
cursor=6: window=[AABCBB]:
  try pos=0 (A): A=A✓, B=B✓, C≠B → length=2
  try pos=1 (A): A=A✓, B=B✓, C≠B → length=2
  try pos=2 (B): B≠A → 0
  try pos=3 (C): C≠A → 0
  try pos=4 (B): B≠A → 0
  try pos=5 (B): B≠A → 0
  best: length=2 < 3 → Literal('A')
cursor=7: window=[AABCBBA]:
  try pos=2 (B): B=B✓, C≠C... B=B✓, C=C? data[7]='B' data[8]='C' pos+0='B'✓ pos+1='C'? window[3]='C'✓ pos+2='B'? window[4]='B'✓ → length=3
  best: length=3, offset=cursor-pos=7-2=5 → Match(offset=5, length=3)
cursor=10: end of input.

Tokens: [Lit('A'), Lit('A'), Lit('B'), Lit('C'), Lit('B'), Lit('B'), Match(5,3)]
```

7 tokens for 9 bytes:
- 6 literals (1 byte each in wire format)
- 1 match (3 bytes in wire format)
The match compresses "ABC" from 3 bytes to 3 bytes — break-even at `min_match=3`.

### Worked Example 2: "ABABAB"

```
Input: A  B  A  B  A  B
       0  1  2  3  4  5

cursor=0: window empty → Literal('A')
cursor=1: window=[A] → B not in window → Literal('B')
cursor=2: window=[AB]:
  pos=0: A=A✓, B=B✓, A=A✓, B=B✓ → length=4 (all remaining)
  best: length=4, offset=2 → Match(offset=2, length=4)
cursor=6: end of input.

Tokens: [Literal('A'), Literal('B'), Match(offset=2, length=4)]
```

3 tokens for 6 bytes. The match decodes as a **self-referential** (overlapping) copy:
copy 4 bytes starting 2 back in the output = ABAB.

### Overlapping Matches

LZSS inherits LZ77's ability to copy from an offset smaller than the match length.
This is how a single `Match(offset=1, length=6)` expands "A" into "AAAAAAA":

```
output = [A]
copy 6 bytes from offset 1 (= position 0):
  pos 0 → A  ; output = [A, A]
  pos 1 → A  ; output = [A, A, A]
  ...
```

The copy must proceed **byte-by-byte**, not as a bulk memcpy, because each new byte
may reference a position that was just written.

## Decoding Algorithm

```
function decode(tokens: list[Token], original_length: int) -> bytes:
    output ← []

    for token in tokens:
        match token:
            case Literal(byte):
                output.append(byte)

            case Match(offset, length):
                start ← len(output) - offset
                for i in range(length):
                    output.append(output[start + i])  # byte-by-byte for overlap

    return output[:original_length]
```

The `[:original_length]` slice handles any excess bytes from the last block's
alignment padding (rare in practice, since LZSS tokens map cleanly, but
`original_length` provides the authoritative byte count).

## Parameters

| Parameter   | Default | Meaning                                                     |
|-------------|---------|-------------------------------------------------------------|
| window_size | 4096    | Max lookback distance (offset range: 1..window_size).       |
| max_match   | 255     | Max match length (fits in uint8).                           |
| min_match   | 3       | Minimum match length to emit a Match token (break-even).    |

With `window_size=4096` and `max_match=255`, offset fits in uint16 and length in
uint8 — both fit cleanly in the 3-byte match record.

## Interface Contract

```
Literal(byte: int)          — byte ∈ [0, 255]
Match(offset: int, length: int)
  offset ∈ [1, window_size]
  length ∈ [min_match, max_match]

Token = Literal | Match

encode(data: bytes,
       window_size: int = 4096,
       max_match:   int = 255,
       min_match:   int = 3) -> list[Token]

decode(tokens: list[Token], original_length: int) -> bytes
  Invariant: decode(encode(x), len(x)) == x   for all x

compress(data: bytes, window_size: int = 4096, max_match: int = 255) -> bytes
  Wire format: see "Wire Format" above.
  First 8 bytes: original_length (BE uint32) + block_count (BE uint32).
  Then block_count blocks of flag-byte + symbol data.

decompress(data: bytes) -> bytes
  Invariant: decompress(compress(x)) == x   for all x
```

## Test Vectors

All vectors use `window_size=4096`, `max_match=255`, `min_match=3`.

### 1. Empty input

```
encode(b"") → []
compress(b"") → 8-byte header with original_length=0, block_count=0
decompress(compress(b"")) == b""
```

### 2. Single byte

```
encode(b"A") → [Literal(65)]
```

### 3. No repetition (all literals)

```
encode(b"ABCDE") → [Literal(65), Literal(66), Literal(67), Literal(68), Literal(69)]
```

All 5 tokens are Literals — the search buffer never contains a useful match.

### 4. "AABCBBABC" — mixed literal + match

```
encode(b"AABCBBABC") → [
    Literal(65),        # 'A'
    Literal(65),        # 'A'
    Literal(66),        # 'B'
    Literal(67),        # 'C'
    Literal(66),        # 'B'
    Literal(66),        # 'B'
    Match(offset=5, length=3),  # "ABC" from 5 bytes back
]
```

The Match decodes: output has 6 bytes so far ("AABCBB"); 6−5=1 → start at index 1.
Copy 3 bytes: output[1]='A', output[2]='B', output[3]='C' → appends "ABC". ✓

### 5. "ABABAB" — overlapping match

```
encode(b"ABABAB") → [
    Literal(65),            # 'A'
    Literal(66),            # 'B'
    Match(offset=2, length=4),   # copy 4 from 2 back → "ABAB"
]
```

Round-trip: `decompress(compress(b"ABABAB")) == b"ABABAB"` ✓

### 6. "AAAAAAA" — self-referential match

```
encode(b"AAAAAAA") → [
    Literal(65),            # 'A'
    Match(offset=1, length=6),   # copy 6 from 1 back → "AAAAAA"
]
```

Decoding: output=['A']; copy 6 bytes from offset 1 (= position 0), byte-by-byte:
each step appends the character just written → "AAAAAAA" ✓

### 7. Round-trip invariant

```python
for s in [b"", b"A", b"ABCDE", b"AAAAAAA", b"ABABAB", b"AABCBBABC"]:
    assert decompress(compress(s)) == s
```

### 8. Binary data with null bytes

```python
data = bytes([0, 0, 0, 255, 255])
assert decompress(compress(data)) == data
```

### 9. Long repetitive data compresses

```python
data = b"ABC" * 1000  # 3000 bytes
assert len(compress(data)) < len(data)
```

### 10. LZSS beats LZ77 on repetitive data

LZSS should produce strictly fewer bytes than LZ77 on any input with meaningful
repetition, because matches cost 3 bytes instead of 4 and literals cost 1 byte
instead of 4. (Absolute byte counts are implementation-specific.)

## Comparison with LZ77 and LZ78

| Property          | LZ77                         | LZSS                         | LZ78                        |
|-------------------|------------------------------|------------------------------|-----------------------------|
| Token kinds       | 1 (offset, length, next_char)| 2 (Literal or Match)         | 1 (dict_index, next_char)   |
| Literal cost      | 4 bytes                      | 1 byte                       | 4 bytes                     |
| Match cost        | 4 bytes                      | 3 bytes                      | 4 bytes                     |
| Flag overhead     | None                         | 1 byte per 8 symbols         | None                        |
| Overlapping copy  | Yes                          | Yes                          | No                          |
| Dictionary        | Implicit (sliding window)    | Implicit (sliding window)    | Explicit (growing trie)     |
| Successor         | LZSS                         | DEFLATE (+ Huffman)          | LZW                         |

## Package Matrix

| Language   | Package                                    | Build command           |
|------------|--------------------------------------------|-------------------------|
| Python     | `coding-adventures-lzss`                   | `pytest tests/ -v`      |
| Go         | `github.com/.../go/lzss`                   | `go test ./... -v`      |
| Ruby       | `coding_adventures_lzss`                   | `bundle exec rake test` |
| TypeScript | `@coding-adventures/lzss`                  | `npx vitest run`        |
| Rust       | `lzss`                                     | `cargo test`            |
| Elixir     | `coding_adventures_lzss`                   | `mix test`              |
| Lua        | `coding_adventures.lzss`                   | `busted .`              |
| Perl       | `CodingAdventures::LZSS`                   | `prove -l -v t/`        |
| Swift      | `LZSS`                                     | `swift test`            |

No external dependencies. Each package is standalone — the sliding-window
matching and flag-bit serialisation are self-contained.
