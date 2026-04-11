# CMP01 — LZ78

## Overview

Published by Abraham Lempel and Jacob Ziv in **1978** (*IEEE Transactions on
Information Theory*, one year after LZ77), LZ78 takes a fundamentally different
approach to compression: instead of a sliding window over recent bytes, it builds
an **explicit dictionary** of sequences seen so far. Each time the encoder
encounters a sequence not yet in the dictionary, it records the sequence for
future reuse. The decoder mirrors this process exactly, so no dictionary is
transmitted — both sides rebuild it identically.

The payoff of a global dictionary over a local window: sequences discovered early
in the data are reusable no matter how far back they occurred, unlike LZ77 where
references must fall within `window_size` bytes. The trade-off: the dictionary
grows unboundedly, which LZW (CMP03) addresses with a fixed-size dictionary and a
reset strategy.

```
Series:
  CMP00 (LZ77, 1977) — Sliding-window backreferences.   ← predecessor
  CMP01 (LZ78, 1978) — Explicit dictionary (trie).      ← YOU ARE HERE
  CMP02 (LZSS, 1982) — LZ77 + flag bits; no wasted literals.
  CMP03 (LZW,  1984) — LZ78 + pre-initialised alphabet; used in GIF.
  CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
  CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
```

## Key Concepts

### The Explicit Dictionary vs. The Sliding Window

LZ77 finds matches inside a *recent window*. Every reference is relative to the
current cursor position and is forgotten once it falls outside the window:

```
LZ77 — sliding window (fixed lookback distance)
───────────────────────────────────────────────
  [  SEARCH BUFFER  ][ LOOKAHEAD ]
  ← window_size bytes →
    (older data forgotten beyond window)
```

LZ78 instead maintains a *dictionary* of sequences, indexed by ID:

```
LZ78 — explicit dictionary (grows as encoding proceeds)
───────────────────────────────────────────────────────
  ID │ Sequence
  ───┼──────────────────────────────────────────
   0 │ (empty — the implicit root)
   1 │ "A"       (first new sequence seen)
   2 │ "B"       (second)
   3 │ "AB"      (extends entry 1 by byte 'B')
   4 │ "ABA"     (extends entry 3 by byte 'A')
   …
```

An entry at index `k` is defined by a parent ID `p` and a byte `b`: it represents
the sequence obtained by appending `b` to the sequence at `p`. This is exactly a
trie: each node is a dictionary entry, each edge is labelled with the byte that
extends the parent sequence.

```
Trie representation of the dictionary above:

        root (id=0)
        |
        A (id=1)
        |
        B (id=3)
        |
        A (id=4)
```

### Token: (dict_index, next_char)

Each output token has **two fields**:

```
Token(dict_index: int, next_char: int)
```

- `dict_index`: ID of the longest dictionary entry that matches the current
  input. `0` means no dictionary match (a pure literal).
- `next_char`:  The byte immediately following the matched sequence. This byte
  is always emitted and seeds the next dictionary entry.

Compare with LZ77's three-field token `(offset, length, next_char)`: LZ78 has no
`length` because the length is implicit in the dictionary entry's depth. No
`offset` because references are by ID, not by position.

### Encoding: Building the Trie

The encoder maintains a trie (rooted at the dictionary root, id=0). At each step:

1. Read bytes one at a time, following trie edges from the current node.
2. When the current byte **has** a child edge in the trie, follow it (extend the
   current match).
3. When the current byte **has no** child edge, the match is complete:
   - Emit `Token(current_node.id, current_byte)`.
   - Add a new trie node for `current_node → current_byte` (new dict entry).
   - Reset to the trie root.

The new dictionary entry is `(current_node.id, current_byte)` — a compact pair
that encodes the entire sequence by pointing to its parent.

### Decoding: Mirroring the Trie

The decoder maintains a parallel dictionary (a plain list of `(parent_id, byte)`
pairs). For each token `(dict_index, next_char)`:

1. Reconstruct the sequence for `dict_index` by following the parent chain:
   decode until parent_id = 0 (root), then reverse.
2. Emit the reconstructed sequence.
3. Emit `next_char`.
4. Add a new dictionary entry for `(dict_index, next_char)` — identical to the
   entry the encoder added.

The decoder never needs to transmit the dictionary because both sides add entries
at the same time, in the same order.

### No Overlapping Matches

Unlike LZ77, LZ78 references always point to already-complete dictionary entries.
A reference `dict_index = k` refers to an entry added before the current token was
encoded. There is no overlapping-copy analogue. Decoding is bulk-safe.

### The "Dangerous Token" Problem

One edge case arises during decoding: the encoder may emit a token whose
`dict_index` refers to an entry that was **just added** in the same step. Can this
happen?

Yes — but only in one specific pattern: when the current input starts with a byte
sequence that equals the last dictionary entry followed by its own first byte.

```
Example: "ABAB..." when "AB" was just added as entry k.
  Encoder: at cursor on the second "AB", follows the trie to node k.
           Next byte is 'A' — no child. Emits Token(k, 'A').
           Adds entry k+1 = (k, 'A').
  Decoder: receives Token(k, 'A'). Looks up entry k. But k was just
           added in the previous step — it IS available (added before this token).
```

For LZ78 specifically, this case does not arise in its purest form because the
encoder adds a new entry AFTER emitting the token, so `dict_index` in the emitted
token always refers to a previously existing entry (not the one being added now).

The dangerous token IS a real concern in **LZW** (CMP03), where the pre-initialised
alphabet changes the timing. We call it out here so readers are aware when they
reach CMP03.

## Encoding Algorithm

```
function encode(data: bytes,
                max_dict_size: int = 65536) -> list[Token]:

    # The trie root represents the empty sequence (id = 0).
    trie_root  ← new TrieNode(id=0)
    next_id    ← 1
    current    ← trie_root
    tokens     ← []

    for byte in data:
        if byte in current.children:
            # The sequence so far can be extended — follow the edge.
            current ← current.children[byte]
        else:
            # No match for this extension — emit a token.
            tokens.append(Token(dict_index=current.id, next_char=byte))

            # Add the new sequence to the dictionary (if space allows).
            if next_id < max_dict_size:
                node          ← new TrieNode(id=next_id)
                next_id       ← next_id + 1
                current.children[byte] ← node

            # Reset to root — start a fresh match.
            current ← trie_root

    # End-of-stream: if we have a partial match, emit a flush token.
    # next_char = 0 is a sentinel meaning "no following byte".
    if current is not trie_root:
        tokens.append(Token(dict_index=current.id, next_char=0))

    return tokens
```

### Worked Example 1: "AABCBBABC"

Trace the encoder step by step. The dictionary starts empty (root = id 0).

```
Input: A  A  B  C  B  B  A  B  C
       ↑
Step  Byte  cur.id  child?  Action               Token emitted  Dict added
 1     A      0      No     emit(0, 'A'), add 1   (0, 'A')      1 → "A"
               ↑ reset to root
 2     A      0      Yes    follow to node 1
       B      1      No     emit(1, 'B'), add 2   (1, 'B')      2 → "AB"
               ↑ reset to root
 3     C      0      No     emit(0, 'C'), add 3   (0, 'C')      3 → "C"
               ↑ reset to root
 4     B      0      No     emit(0, 'B'), add 4   (0, 'B')      4 → "B"
               ↑ reset to root
 5     B      0      Yes    follow to node 4
       A      4      No     emit(4, 'A'), add 5   (4, 'A')      5 → "BA"
               ↑ reset to root
 6     B      0      Yes    follow to node 4
       C      4      No     emit(4, 'C'), add 6   (4, 'C')      6 → "BC"
               ↑ reset to root

 End-of-stream: cur = root → no flush needed.
```

Tokens: `[(0,'A'), (1,'B'), (0,'C'), (0,'B'), (4,'A'), (4,'C')]`
— 6 tokens for 9 input bytes. All `dict_index=0` tokens are pure literals.

### Worked Example 2: "ABABAB"

```
Input: A  B  A  B  A  B

Step  Byte  cur.id  child?  Action               Token emitted  Dict added
 1     A      0      No     emit(0, 'A'), add 1   (0, 'A')      1 → "A"
 2     B      0      No     emit(0, 'B'), add 2   (0, 'B')      2 → "B"
 3     A      0      Yes    follow to node 1
       B      1      No     emit(1, 'B'), add 3   (1, 'B')      3 → "AB"
 4     A      0      Yes    follow to node 1
       B      1      Yes    follow to node 3
 End-of-stream: cur = node 3 (≠ root) → emit flush token    (3, 0)

Tokens: [(0,'A'), (0,'B'), (1,'B'), (3, 0)]
4 tokens for 6 bytes. Entry 3 encodes "AB" (2 bytes) — real compression.
```

The flush token `(3, 0)` emits sequence "AB" (from dict entry 3) with a sentinel
`next_char=0`. During decoding the compressed format stores the original data
length so the sentinel byte is never returned to the caller.

## Decoding Algorithm

```
function decode(tokens: list[Token], original_length: int) -> bytes:

    # Build a parallel dictionary alongside the token stream.
    # Entry 0 is the implicit root (empty sequence).
    dict    ← [(parent_id=0, byte=0)]   # index 0 = root (sentinel)
    output  ← []

    for (dict_index, next_char) in tokens:

        # Reconstruct the sequence for dict_index.
        sequence ← reconstruct(dict, dict_index)
        output.extend(sequence)

        # Emit next_char (unless it's the flush sentinel — see below).
        if len(output) < original_length:
            output.append(next_char)

        # Add a new dictionary entry (same as encoder did).
        dict.append((dict_index, next_char))

    return output[:original_length]

function reconstruct(dict, index) -> bytes:
    if index == 0:
        return []
    # Walk parent chain, collecting bytes in reverse.
    sequence ← []
    while index != 0:
        (parent_id, byte) ← dict[index]
        sequence.append(byte)
        index ← parent_id
    sequence.reverse()
    return sequence
```

The `original_length` guard prevents the flush token's sentinel `next_char=0` from
being appended to the output when the input ends on a dictionary boundary.

## Parameters

| Parameter       | Default | Meaning                                              |
|-----------------|---------|------------------------------------------------------|
| max_dict_size   | 65536   | Maximum dictionary entries (IDs 0 to max-1). After this limit, new sequences are no longer added — the encoder emits repeated literals. |

Larger `max_dict_size` improves compression on long repetitive data but requires
more memory for the dictionary. LZW (CMP03) uses a fixed 4096 or 65536 entry
dictionary and resets it when full.

**Why no `window_size`?** LZ77 forgets sequences older than `window_size` bytes.
LZ78's dictionary never forgets unless explicitly cleared. The equivalent control
knob is `max_dict_size`.

## Interface Contract

```
Token: (dict_index: int, next_char: int)
  dict_index ∈ [0, max_dict_size)  — 0 = literal (no dict match)
  next_char  ∈ [0, 255]            — byte following the match;
                                     0 is also used as the flush sentinel

encode(data: bytes, max_dict_size: int = 65536) -> list[Token]
  Invariant: decode(encode(x), len(x)) == x   for all x

decode(tokens: list[Token], original_length: int) -> bytes

compress(data: bytes, max_dict_size: int = 65536) -> bytes
  Wire format: 4 bytes original_length (BE uint32)
             + 4 bytes token_count (BE uint32)
             + token_count × 4 bytes each:
               2 bytes dict_index (BE uint16) + 1 byte next_char + 1 byte reserved (0x00)

decompress(data: bytes) -> bytes
  Invariant: decompress(compress(x)) == x   for all x
```

## Test Vectors

All vectors assume `max_dict_size=65536`.

### 1. Empty input

```
encode(b"") → []
decode([], original_length=0) → b""
```

### 2. Single byte

```
encode(b"A") → [Token(0, 65)]
decode([Token(0, 65)], original_length=1) → b"A"
```

`dict_index=0` means "no prefix match"; the token is a pure literal.

### 3. No repeated sequences (all literals)

```
encode(b"ABCDE") → [
    Token(0, 65),   # 'A'
    Token(0, 66),   # 'B'
    Token(0, 67),   # 'C'
    Token(0, 68),   # 'D'
    Token(0, 69),   # 'E'
]
```

Every token has `dict_index=0` — no match was long enough to reuse.

### 4. Simple repetition: "AABCBBABC"

From the worked example above:

```
encode(b"AABCBBABC") → [
    Token(0, 65),   # 'A'
    Token(1, 66),   # 'A' from dict + 'B'
    Token(0, 67),   # 'C'
    Token(0, 66),   # 'B'
    Token(4, 65),   # 'B' from dict + 'A'
    Token(4, 67),   # 'B' from dict + 'C'
]
```

Dictionary after encoding:
```
ID │ Sequence   (parent_id, byte)
───┼──────────────────────────────
 1 │ "A"        (0, 'A')
 2 │ "AB"       (1, 'B')
 3 │ "C"        (0, 'C')
 4 │ "B"        (0, 'B')
 5 │ "BA"       (4, 'A')
 6 │ "BC"       (4, 'C')
```

### 5. Growing dictionary: "ABABAB"

From the worked example above:

```
encode(b"ABABAB") → [
    Token(0, 65),   # 'A'
    Token(0, 66),   # 'B'
    Token(1, 66),   # 'A' + 'B'
    Token(3,  0),   # flush: "AB" from dict (sentinel next_char)
]
```

Round-trip: `decompress(compress(b"ABABAB")) == b"ABABAB"` ✓
(The sentinel byte from the flush token is stripped by the stored original_length.)

### 6. All identical bytes: "AAAAAAA"

```
encode(b"AAAAAAA"):
  Step 1: 'A' → emit(0, 'A'), dict: {1:"A"}, reset
  Step 2: 'A' → follow to node 1
          'A' → emit(1, 'A'), dict: {1:"A", 2:"AA"}, reset
  Step 3: 'A' → follow to node 1
          'A' → follow to node 2
          'A' → emit(2, 'A'), dict: {…, 3:"AAA"}, reset
  Step 4: 'A' → follow to node 1 (only 1 byte left)
          End-of-stream → flush(1, 0)

Tokens: [Token(0,65), Token(1,65), Token(2,65), Token(1, 0)]
4 tokens for 7 bytes.
```

### 7. Round-trip invariant

```python
for s in [b"", b"A", b"ABCDE", b"AAAAAAA", b"ABABABAB", b"AABCBBABC"]:
    assert decompress(compress(s)) == s
```

### 8. Binary data with null bytes

```python
data = bytes([0, 0, 0, 255, 255])
assert decompress(compress(data)) == data
```

### 9. Long repetitive data compresses

```python
data = b"ABC" * 1000   # 3000 bytes
compressed = compress(data)
assert len(compressed) < len(data)
```

## Serialisation Format

The wire format produced by `compress` and consumed by `decompress`:

```
Offset  Size  Field
──────  ────  ────────────────────────────────────────────
0       4     original_length — BE uint32. Length of the uncompressed data.
4       4     token_count — BE uint32. Number of tokens that follow.
8       4×N   tokens — one 4-byte record per token:
                [0..1]  dict_index — BE uint16 (0..65535)
                [2]     next_char — uint8 (0..255)
                [3]     reserved — 0x00

Total: 8 + 4 × token_count bytes.
```

Storing `original_length` allows `decompress` to strip the flush sentinel byte
that may appear when input ends mid-dictionary-match.

## Comparison with LZ77

| Property            | LZ77                          | LZ78                              |
|---------------------|-------------------------------|-----------------------------------|
| Dictionary          | Implicit (sliding window)     | Explicit (growing trie)           |
| Reference type      | (offset, length, next_char)   | (dict_index, next_char)           |
| Memory              | Fixed (window_size bytes)     | Grows with distinct sequences     |
| Lookback distance   | Limited to window_size        | Unlimited (any prior sequence)    |
| Overlapping matches | Yes (must copy byte-by-byte)  | No (bulk copy safe)               |
| Successors          | LZSS, DEFLATE                 | LZW                               |

## Package Matrix

| Language   | Package                                    | Build command           |
|------------|--------------------------------------------|-------------------------|
| Python     | `coding-adventures-lz78`                   | `pytest tests/ -v`      |
| Go         | `github.com/.../go/lz78`                   | `go test ./... -v`      |
| Ruby       | `coding_adventures_lz78`                   | `bundle exec rake test` |
| TypeScript | `@coding-adventures/lz78`                  | `npx vitest run`        |
| Rust       | `lz78`                                     | `cargo test`            |
| Elixir     | `coding_adventures_lz78`                   | `mix test`              |
| Lua        | `coding_adventures.lz78`                   | `busted .`              |
| Perl       | `CodingAdventures::LZ78`                   | `prove -l -v t/`        |
| Swift      | `LZ78`                                     | `swift test`            |

No external dependencies in any language. Each package embeds its own
byte-indexed trie for the dictionary.
