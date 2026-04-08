# CMP00 — LZ77

## Overview

LZ77 is the foundational lossless compression algorithm, published in 1977 by Abraham Lempel and
Jacob Ziv in *"A Universal Algorithm for Sequential Data Compression"* (IEEE Transactions on
Information Theory, Vol. 23, No. 3). It was the first provably universal, dictionary-free
compressor — meaning it achieves the theoretical entropy limit for any stationary source without
needing prior knowledge of the data's statistics.

Every major practical compressor in use today descends directly from LZ77:

- **LZSS** (CMP02) removes LZ77's wasted literal byte when a match is found, cutting overhead
- **LZW** (CMP03) flips the model to an explicit dictionary, powering GIF and early modems
- **DEFLATE** (CMP05) pairs LZ77 backreferences with Huffman entropy coding, powering ZIP,
  gzip, PNG, and zlib
- Modern compressors like zstd, LZ4, Snappy, and Brotli are all LZ77 descendants

Starting here builds the mental model that everything else extends. If you understand LZ77, you
understand the core idea of every compressor listed above.

### Why Compression at All?

A computer stores bytes. If you have a file with the text `ABABABAB`, you could store 8 bytes.
But notice that after the first `AB`, the rest is just that pair repeated. A clever encoder
could instead store: "here's AB, then repeat that pair 3 more times." That is compression —
representing repetition cheaply rather than spelling it out.

LZ77's insight is that *almost all repetition in real data is local* — a word used once in a
document is likely used again nearby, an instruction sequence in a binary reappears in loops,
a colour run in an image persists across adjacent rows. By keeping a window of recently seen
bytes and referencing matches in that window, LZ77 exploits this locality universally.

### The CMP Series

This is the first package in the **CMP** (Compression) series. The series builds from the
1977 original through to DEFLATE, each package increasing in sophistication while building
directly on concepts from the previous one.

| Spec  | Algorithm      | Year | Key Idea                                      |
|-------|----------------|------|-----------------------------------------------|
| CMP00 | LZ77           | 1977 | Sliding-window backreferences                 |
| CMP01 | LZ78           | 1978 | Explicit dictionary (trie), no sliding window |
| CMP02 | LZSS           | 1982 | LZ77 + flag bits; no wasted literal byte      |
| CMP03 | LZW            | 1984 | Pre-initialized dictionary; used in GIF       |
| CMP04 | Huffman Coding | 1952 | Entropy coding; prerequisite for DEFLATE      |
| CMP05 | DEFLATE        | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib             |

---

## Key Concepts

### The Sliding Window

LZ77 processes input as a stream, maintaining two conceptual regions:

```
┌─────────────────────────────────┬──────────────────┐
│         SEARCH BUFFER           │ LOOKAHEAD BUFFER  │
│  (already compressed — up to    │  (not yet seen — │
│   window_size bytes back)       │  up to max_match) │
└─────────────────────────────────┴──────────────────┘
                                  ↑
                              cursor (current position)
```

- **Search buffer** (also called the *dictionary* or *history*): the bytes already emitted to
  the output. The encoder can reference any substring within the last `window_size` bytes. This
  is the "window" that slides forward as we process more input.
- **Lookahead buffer**: the next `max_match` bytes of unprocessed input. The encoder tries to
  find the longest prefix of the lookahead that also appears somewhere in the search buffer.

### Tokens

The encoder's output is a sequence of **tokens**. Each token is a triple:

```
(offset, length, next_char)
```

| Field       | Type | Meaning                                                              |
|-------------|------|----------------------------------------------------------------------|
| `offset`    | int  | How many bytes *back* from the cursor the match begins (1 = the byte immediately before cursor) |
| `length`    | int  | How many bytes the match covers (0 = no match found)                |
| `next_char` | byte | The literal byte immediately following the match                     |

**When `length == 0`:** `offset` is irrelevant (conventionally 0); the token encodes a single
literal byte. This handles characters that appear for the first time.

**When `length > 0`:** the decoder should copy `length` bytes from position
`(current_output_length - offset)` in the already-decoded output, then append `next_char`. This
always advances the cursor by `length + 1` bytes.

### The Overlap (Self-Referential) Match

A match is allowed to *overlap* the cursor. If `offset < length`, the match extends into bytes
that haven't been written yet — but will be written *as part of this very copy*. The decoder
must copy byte by byte (not `memcpy` in bulk) to handle this correctly.

**Example:** if the output so far is `[A, B]` and the token is `(2, 5, 'B')`:

```
Step 1: copy output[0] = A  → output is now [A, B, A]
Step 2: copy output[1] = B  → output is now [A, B, A, B]
Step 3: copy output[2] = A  → output is now [A, B, A, B, A]   (just-written byte)
Step 4: copy output[3] = B  → output is now [A, B, A, B, A, B]
Step 5: copy output[4] = A  → output is now [A, B, A, B, A, B, A]
Then append next_char B     → output is now [A, B, A, B, A, B, A, B]
```

This is effectively run-length encoding of the `AB` pattern, achieved automatically by the
sliding-window model.

### Parameters

| Parameter     | Default | Meaning                                                     |
|---------------|---------|-------------------------------------------------------------|
| `window_size` | 4096    | Maximum offset. Larger = better compression, more memory.  |
| `max_match`   | 255     | Maximum match length. Bounded by how many bits store length.|
| `min_match`   | 3       | Minimum length before a backreference is worth using. A token `(offset, length, next_char)` costs the same whether `length` is 0 or 1; a length-2 match saves nothing over two literals (each also emits a next_char); length ≥ 3 is the break-even point. |

---

## Encoding Algorithm

### Pseudocode

```
function encode(data, window_size=4096, max_match=255, min_match=3):
    tokens = []
    cursor = 0
    while cursor < len(data):
        # The lookahead cannot extend past the end of data.
        # Reserve 1 byte for next_char, so effective match limit is:
        lookahead_end  = min(cursor + max_match, len(data) - 1)
        search_start   = max(0, cursor - window_size)

        best_offset = 0
        best_length = 0

        # Try every possible match start in the search buffer.
        for start in range(search_start, cursor):
            length = 0
            # Match byte by byte (overlap is allowed — indices advance past cursor).
            while (cursor + length < lookahead_end and
                   data[start + length] == data[cursor + length]):
                length += 1
            if length > best_length:
                best_length = length
                best_offset = cursor - start   # how many bytes back

        if best_length >= min_match:
            next_char = data[cursor + best_length]
            tokens.append((best_offset, best_length, next_char))
            cursor += best_length + 1
        else:
            tokens.append((0, 0, data[cursor]))
            cursor += 1

    return tokens
```

> **Complexity note:** the naïve O(n · window) search above is correct for a spec; production
> implementations use hash chains or suffix arrays to achieve O(n) or O(n log n) encoding.

### Step-by-Step: Encoding `"AABCBBABC"`

Input bytes (0-indexed): `A(0) A(1) B(2) C(3) B(4) B(5) A(6) B(7) C(8)`

Parameters: `window_size=4096, max_match=255, min_match=3`

---

**Step 1 — cursor=0**

```
Search buffer : (empty)
Lookahead     : A A B C B B A B C
```

No search buffer to scan. No match possible.

→ Token `(0, 0, 'A')` — emit literal A. Advance 1.

---

**Step 2 — cursor=1**

```
Search buffer : A
Lookahead     : A B C B B A B C
```

Scan search buffer:
- `offset=1` (position 0): `A` matches `A` in lookahead. Extend: position 1 would be `A` but
  lookahead[1]=`B` — no. **Length=1**, which is < `min_match=3`.

No usable match.

→ Token `(0, 0, 'A')` — emit literal A. Advance 1.

---

**Step 3 — cursor=2**

```
Search buffer : A A
Lookahead     : B C B B A B C
```

Neither `A` in the search buffer matches `B` at the cursor.

→ Token `(0, 0, 'B')` — emit literal B. Advance 1.

---

**Step 4 — cursor=3**

```
Search buffer : A A B
Lookahead     : C B B A B C
```

`C` does not appear in the search buffer.

→ Token `(0, 0, 'C')` — emit literal C. Advance 1.

---

**Step 5 — cursor=4**

```
Search buffer : A A B C
Lookahead     : B B A B C
```

Scan:
- `offset=2` (position 2): `B` matches `B`. Extend: position 3=`C` vs lookahead[1]=`B` — no.
  Length=1, too short.

→ Token `(0, 0, 'B')` — emit literal B. Advance 1.

---

**Step 6 — cursor=5**

```
Search buffer : A A B C B
Lookahead     : B A B C
```

Scan:
- `offset=3` (position 2): `B` matches `B`. Extend: position 3=`C` vs lookahead[1]=`A` — no.
  Length=1.
- `offset=1` (position 4): `B` matches `B`. Extend: position 5=cursor, lookahead[1]=`A`;
  search_buffer at position 5 is `B` (just before cursor) — `B` vs `A` — no. Length=1.

All matches < `min_match`.

→ Token `(0, 0, 'B')` — emit literal B. Advance 1.

---

**Step 7 — cursor=6**

```
Search buffer : A A B C B B
Lookahead     : A B C
              (lookahead_end = min(6+255, 9-1) = 8, so max 2 bytes of match)
```

Scan (searching positions 0..5):
- `offset=6` (position 0): `A` matches `A`. Extend: position 1=`A` vs lookahead[1]=`B` — no.
  Length=1.
- `offset=5` (position 1): `A` matches `A`. Extend: position 2=`B` vs lookahead[1]=`B` — yes!
  Extend: position 3=`C` vs lookahead[2]=`C` — yes! **Length=3 but lookahead_end=8**, meaning
  we can only match up to cursor+2=8 (exclusive), so effective max=2. **Length=2**.
  `best_offset=5, best_length=2`.

`best_length=2 < min_match=3` — no usable match.

→ Token `(0, 0, 'A')` — emit literal A. Advance 1.

---

**Step 8 — cursor=7**

```
Search buffer : A A B C B B A
Lookahead     : B C
              (lookahead_end = min(7+255, 9-1) = 8, max 1 byte of match)
```

With only 1 possible match byte (we need to reserve 1 for next_char), no length ≥ 3 is
achievable.

→ Token `(0, 0, 'B')` — emit literal B. Advance 1.

---

**Step 9 — cursor=8**

```
Search buffer : A A B C B B A B
Lookahead     : C
              (lookahead_end = min(8+255, 9-1) = 8, max 0 bytes of match — end of input)
```

No lookahead remaining for a match. Emit the final byte as a literal.

→ Token `(0, 0, 'C')` — emit literal C. Advance 1.

---

**Final token stream for `"AABCBBABC"`:**

```
(0,0,'A')  (0,0,'A')  (0,0,'B')  (0,0,'C')
(0,0,'B')  (0,0,'B')  (0,0,'A')  (0,0,'B')  (0,0,'C')
```

9 tokens for 9 bytes. This input has no long repeated substrings, so LZ77 achieves no
compression. The next example shows where LZ77 shines.

---

### Step-by-Step: Encoding `"ABABABAB"` (Highly Repetitive)

Input bytes: `A(0) B(1) A(2) B(3) A(4) B(5) A(6) B(7)`

---

**Step 1 — cursor=0:** No buffer. Token `(0, 0, 'A')`. Advance 1.

**Step 2 — cursor=1:**
```
Search buffer : A
Lookahead     : B A B A B A B
```
`B` not in buffer. Token `(0, 0, 'B')`. Advance 1.

**Step 3 — cursor=2:**
```
Search buffer : A B
Lookahead     : A B A B A B
              (lookahead_end = min(2+255, 8-1) = 7, max 5 bytes of match)
```

Scan:
- `offset=2` (position 0): `A` matches `A`. Extend:
  - position 1=`B` vs lookahead[1]=`B` ✓ length=2
  - position 2=cursor (overlap!): copy byte-by-byte. output[2]=`A` (just written) vs
    lookahead[2]=`A` ✓ length=3
  - output[3]=`B` vs lookahead[3]=`B` ✓ length=4
  - output[4]=`A` vs lookahead[4]=`A` ✓ length=5
  - lookahead_end=7, cursor+5=7 — reached limit. **Length=5**.
  - `best_offset=2, best_length=5`.

`best_length=5 ≥ min_match=3`. `next_char = data[2+5] = data[7] = 'B'`.

→ Token `(2, 5, 'B')`. Advance 6. Cursor=8 = end.

---

**Final token stream for `"ABABABAB"`:**

```
(0,0,'A')  (0,0,'B')  (2,5,'B')
```

**3 tokens for 8 bytes.** The single backreference token `(2, 5, 'B')` encodes 6 bytes of
repeating pattern. This is the power of the sliding window.

---

## Decoding Algorithm

### Pseudocode

```
function decode(tokens, initial_buffer=b""):
    output = list(initial_buffer)
    for (offset, length, next_char) in tokens:
        if length > 0:
            start = len(output) - offset
            # Copy byte by byte — DO NOT use bulk copy.
            # The match may overlap the region being written.
            for i in range(length):
                output.append(output[start + i])
        output.append(next_char)
    return bytes(output)
```

### Decoding `(2, 5, 'B')` with `output = [A, B]`

```
start = len([A,B]) - 2 = 0

i=0: append output[0] = A  → [A, B, A]
i=1: append output[1] = B  → [A, B, A, B]
i=2: append output[2] = A  → [A, B, A, B, A]   (just written)
i=3: append output[3] = B  → [A, B, A, B, A, B]
i=4: append output[4] = A  → [A, B, A, B, A, B, A]
Then append next_char 'B'  → [A, B, A, B, A, B, A, B]
```

Result: `ABABABAB` ✓

---

## Serialisation Format

The `encode` / `decode` functions above operate on a list of `Token` structs in memory. For
`compress` / `decompress`, tokens must be serialised to bytes.

**Simple fixed-width format (used in this package):**

Each token is 6 bytes:

```
┌──────────────┬──────────────┬──────────────┐
│   offset     │   length     │  next_char   │
│  (2 bytes,   │  (1 byte,    │  (1 byte)    │
│  big-endian) │  uint8)      │              │
└──────────────┴──────────────┴──────────────┘
```

Wait — that is 4 bytes. Prefixed with a 4-byte big-endian token count:

```
[4 bytes: token count][N × 4 bytes: tokens]
```

> **Note:** This is a teaching format, not an industry format. Production LZ77 descendants
> use variable-width bit-packing (see LZSS, CMP02) to avoid spending 2 bytes on an offset
> even when `length=0`. The fixed-width format here prioritises readability over efficiency.

---

## Interface Contract

### Types

| Type    | Definition                                    |
|---------|-----------------------------------------------|
| `Token` | Named tuple / struct: `(offset: int, length: int, next_char: int)` where `next_char` is a raw byte value (0–255) |

### Functions

| Function      | Signature                                                                                          | Description |
|---------------|----------------------------------------------------------------------------------------------------|-------------|
| `encode`      | `(data: bytes, window_size: int = 4096, max_match: int = 255, min_match: int = 3) -> list[Token]` | Tokenise input into LZ77 token stream. |
| `decode`      | `(tokens: list[Token], initial_buffer: bytes = b"") -> bytes`                                      | Reconstruct bytes from token stream. `initial_buffer` seeds the search buffer (useful for streaming). |
| `compress`    | `(data: bytes, window_size: int = 4096, max_match: int = 255, min_match: int = 3) -> bytes`       | Encode then serialise tokens to bytes. |
| `decompress`  | `(data: bytes) -> bytes`                                                                           | Deserialise and decode back to original bytes. |

### Invariants

1. `decode(encode(data)) == data` for all `data`.
2. `decompress(compress(data)) == data` for all `data`.
3. All offsets in `encode` output satisfy `1 ≤ offset ≤ window_size`.
4. All lengths satisfy `0 ≤ length ≤ max_match`.
5. All `next_char` values are valid byte values (0–255).
6. `encode(b"") == []`, `decode([]) == b""`.

---

## Test Vectors

All test vectors use default parameters: `window_size=4096, max_match=255, min_match=3`.

### Vector 1 — No Repetition (`"ABCDE"`)

```
Input  : b"ABCDE"
Tokens : [(0,0,65), (0,0,66), (0,0,67), (0,0,68), (0,0,69)]
         i.e.  (0,0,'A') (0,0,'B') (0,0,'C') (0,0,'D') (0,0,'E')
Output : b"ABCDE"
```

No matches — purely literal encoding. Token count equals input length. No expansion, no
compression.

### Vector 2 — All Identical Bytes (`"AAAAAAA"`, 7 × A)

```
Input  : b"AAAAAAA"
Tokens : [(0,0,65), (1,5,65)]
         i.e.  (0,0,'A')  (1,5,'A')
Output : b"AAAAAAA"
```

Step 1: cursor=0, emit literal A.
Step 2: cursor=1, `A` at offset=1, match extends (overlap) to length=5
  (lookahead_end = min(1+255,7-1)=6, max match=5). next_char = data[6] = 'A'.
  Advance 6. End.

Decode of `(1,5,'A')` with output=[A]:
```
start = 1 - 1 = 0
copy output[0]=A → [A,A]
copy output[1]=A → [A,A,A]
copy output[2]=A → [A,A,A,A]
copy output[3]=A → [A,A,A,A,A]
copy output[4]=A → [A,A,A,A,A,A]
append 'A'       → [A,A,A,A,A,A,A]
```

Result: `AAAAAAA` ✓

### Vector 3 — Repeated Pair (`"ABABABAB"`)

```
Input  : b"ABABABAB"
Tokens : [(0,0,65), (0,0,66), (2,5,66)]
         i.e.  (0,0,'A')  (0,0,'B')  (2,5,'B')
Output : b"ABABABAB"
```

(Full walkthrough in Encoding Algorithm section above.)

### Vector 4 — Substring Reuse (`"AABCBBABC"`)

```
Input  : b"AABCBBABC"
Tokens : [(0,0,'A'), (0,0,'A'), (0,0,'B'), (0,0,'C'),
          (0,0,'B'), (0,0,'B'), (0,0,'A'), (0,0,'B'), (0,0,'C')]
Output : b"AABCBBABC"
```

(Full walkthrough in Encoding Algorithm section above. No long-enough repetitions with default
`min_match=3`.)

### Vector 5 — Substring Reuse with Lower `min_match`

```
Input     : b"AABCBBABC"  with  min_match=2
Tokens    : [(0,0,'A'), (1,1,'B'), (0,0,'C'), (2,1,'B'),
             (1,1,'A'), (5,2,'C')]
Output    : b"AABCBBABC"
```

With `min_match=2`, matches of length ≥ 2 are used. The last token `(5,2,'C')` encodes `AB`
starting at offset=5 from cursor=6 (position 1 in the output buffer), then appends `C`.

### Vector 6 — Edge Cases

```
encode(b"")        == []
encode(b"\x00")    == [(0, 0, 0)]
encode(b"\xff")    == [(0, 0, 255)]
decode([])         == b""
```

### Round-Trip Property

For all of the above vectors:
```
decode(encode(input)) == input
decompress(compress(input)) == input
```

---

## Limitations and What Comes Next

LZ77 as specified here has two weaknesses that later algorithms address:

1. **Wasted literal byte.** Even when we find a match of length 5, we still emit a `next_char`.
   That next character could itself be the start of another match — we threw away an opportunity.
   **LZSS (CMP02)** uses a 1-bit flag per token to separate "literal" tokens from "match"
   tokens, eliminating this waste.

2. **Fixed token cost.** Whether `length=0` (a literal) or `length=5` (a match), every token
   costs the same bytes in our serialisation. A literal wastes the `offset` and `length` fields.
   **DEFLATE (CMP05)** uses variable-length Huffman codes so common tokens cost fewer bits.

Understanding these two weaknesses is the bridge from CMP00 to the rest of the series.

---

## Package Matrix

| Language   | Package Directory                              | Module / Namespace                        |
|------------|------------------------------------------------|-------------------------------------------|
| Python     | `code/packages/python/lz77/`                  | `lz77`                                    |
| Go         | `code/packages/go/lz77/`                      | `lz77`                                    |
| Ruby       | `code/packages/ruby/lz77/`                    | `CodingAdventures::LZ77`                  |
| TypeScript | `code/packages/typescript/lz77/`              | `@coding-adventures/lz77`                 |
| Rust       | `code/packages/rust/lz77/`                    | `lz77`                                    |
| Elixir     | `code/packages/elixir/lz77/`                  | `CodingAdventures.LZ77`                   |
| Lua        | `code/packages/lua/lz77/`                     | `coding_adventures.lz77`                  |
| Perl       | `code/packages/perl/lz77/`                    | `CodingAdventures::LZ77`                  |
| Swift      | `code/packages/swift/lz77/`                   | `LZ77`                                    |

**Dependencies:** None. Standalone foundation package with no external dependencies.
