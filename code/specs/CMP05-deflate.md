# CMP05 — DEFLATE

## Overview

DEFLATE (1996, RFC 1951) is the dominant general-purpose lossless compression algorithm.
It combines two complementary techniques from earlier in this series:

1. **LZSS-style tokenization** (CMP02) — eliminates repeated substrings by replacing them
   with back-references into a sliding window.
2. **Huffman coding** (CMP04) — entropy-codes the resulting token stream, squeezing the
   remaining statistical redundancy.

Together they achieve compression that neither technique can match alone: LZ removes
patterns; Huffman removes symbol-frequency bias in the remaining data. On typical text,
DEFLATE achieves 60–70% reduction. On binary data it varies widely (0–90%).

DEFLATE is not a single algorithm but a **composition**. Implementing it in this series
means wiring together the LZSS tokenizer (CMP02) and the Huffman coder (CMP04) with a
two-tree structure that handles the expanded token alphabet.

```
Series:
  CMP00 (LZ77,     1977) — Sliding-window backreferences.
  CMP01 (LZ78,     1978) — Explicit dictionary (trie).
  CMP02 (LZSS,     1982) — LZ77 + flag bits; no wasted literals.
  CMP03 (LZW,      1984) — LZ78 + pre-initialised alphabet; GIF.
  CMP04 (Huffman,  1952) — Entropy coding; prerequisite for DEFLATE.
  CMP05 (DEFLATE,  1996) — LZSS + dual Huffman; ZIP/gzip/PNG/zlib.   ← YOU ARE HERE
  CMP06 (Brotli,   2013) — DEFLATE + context modeling + static dict; HTTPS.
  CMP07 (Zstd,     2016) — ANS/FSE + LZ4 matching; modern universal codec.
```

## Historical Context

Phil Katz designed DEFLATE for PKZIP in 1989. The formal specification (RFC 1951) was
written by L. Peter Deutsch in 1996 to document the algorithm independently of any
implementation. The same year, Deutsch and Jean-Loup Gailly published `zlib` as a
portable reference implementation.

DEFLATE is the compression layer inside:

- **ZIP** (PKZIP, 1989) — PC file archiving standard.
- **gzip** (1992) — Unix file compression; the `Content-Encoding: gzip` header.
- **PNG** (1996) — Portable Network Graphics; DEFLATE per scanline row.
- **zlib** (1996) — The reference C library; used in countless systems.
- **HTTP/2 HPACK, TLS record compression** — both optionally use DEFLATE.

Phil Katz released the PKZIP specification publicly, which is why DEFLATE became a
standard rather than a proprietary format. He died in 2000 at age 37.

## Key Concepts

### Why Two Passes?

LZ tokenization and entropy coding address *different kinds* of redundancy:

```
Input: "ABCABCABC" (9 bytes)

Pass 1 — LZSS tokenization:
  → [Lit('A'), Lit('B'), Lit('C'), Match(offset=3, length=6)]
  → 4 tokens instead of 9 bytes; repetition eliminated.

Pass 2 — Huffman coding of token stream:
  → Variable-length codes; frequent tokens get shorter codes.
  → Exploits skewed frequency distribution of literals vs. match codes.
```

If you only run LZSS (CMP02) on "AAABABABABAB", you eliminate the AB repetition. But
the remaining literal 'A' still appears many times — Huffman coding can exploit that.
If you only run Huffman (CMP04), you exploit letter frequencies but cannot eliminate
the repeated "ABABABABAB" substring at all.

### The Expanded Token Alphabet

LZSS produces two token types: `Literal(byte)` and `Match(offset, length)`. To Huffman-
code this stream, we need a single alphabet that covers both token types. DEFLATE
solves this with a **combined Literal/Length (LL) alphabet**:

```
Symbols 0–255:    Literal byte values (same as in CMP04).
Symbol 256:       End-of-data marker (replaces original_length counting).
Symbols 257–284:  Length codes for match lengths 3–255.
                  Each symbol encodes a base length + optional extra bits.
```

This is the key insight: literals and length codes share **one Huffman tree**. Because
literals follow the same statistical distribution whether they appear as standalone bytes
or as residuals after LZ matching, sharing the tree is efficient.

### Length Codes with Extra Bits

Encoding each possible length (3–255) as a separate symbol would require 253 symbols in
the LL tree. Instead, DEFLATE groups similar lengths into **codes** and appends a few
raw (non-Huffman-coded) extra bits to select the exact length within the group:

```
LL Symbol  Extra bits  Base length  Max length in group
─────────  ──────────  ───────────  ───────────────────
257        0           3            3
258        0           4            4
259        0           5            5
260        0           6            6
261        0           7            7
262        0           8            8
263        0           9            9
264        0           10           10
265        1           11           12
266        1           13           14
267        1           15           16
268        1           17           18
269        2           19           22
270        2           23           26
271        2           27           30
272        2           31           34
273        3           35           42
274        3           43           50
275        3           51           58
276        3           59           66
277        4           67           82
278        4           83           98
279        4           99           114
280        4           115          130
281        5           131          162
282        5           163          194
283        5           195          226
284        5           227          255   (cap at max_match=255)
```

Encoding length L: find the symbol S where `base[S] ≤ L < base[S+1]`, then append
`extra_bits[S]` raw bits encoding `L − base[S]`.

Example: length 13 → symbol 266 (base=13, extra_bits=1, extra_value=13−13=0 → bit "0").
Example: length 14 → symbol 266 (base=13, extra_bits=1, extra_value=14−13=1 → bit "1").
Example: length 50 → symbol 274 (base=43, extra_bits=3, extra_value=50−43=7 → bits "111").

### Distance Codes with Extra Bits

The distance (back-reference offset) ranges from 1 to `window_size` (4096 by default).
A separate **distance alphabet** (24 symbols for window_size=4096) follows the same
code+extra-bits pattern:

```
Dist code  Extra bits  Base dist  Max dist in group
─────────  ──────────  ─────────  ─────────────────
0          0           1          1
1          0           2          2
2          0           3          3
3          0           4          4
4          1           5          6
5          1           7          8
6          2           9          12
7          2           13         16
8          3           17         24
9          3           25         32
10         4           33         48
11         4           49         64
12         5           65         96
13         5           97         128
14         6           129        192
15         6           193        256
16         7           257        384
17         7           385        512
18         8           513        768
19         8           769        1024
20         9           1025       1536
21         9           1537       2048
22         10          2049       3072
23         10          3073       4096
```

Encoding distance D: find code C where `base[C] ≤ D < base[C+1]`, then append
`extra_bits[C]` raw bits encoding `D − base[C]`.

Example: distance 5 → code 4 (base=5, extra_bits=1, extra_value=5−5=0 → bit "0").
Example: distance 4096 → code 23 (base=3073, extra_bits=10, extra_value=4096−3073=1023 → bits "1111111111").

### Two Huffman Trees

DEFLATE uses **two canonical Huffman trees** per compressed stream:

1. **LL tree**: encodes the combined literal/length alphabet (symbols 0–284).
2. **Distance tree**: encodes the distance codes (symbols 0–23 for window_size=4096).

These two trees are built separately from their respective frequency distributions.
Extra bits (for lengths and distances) are emitted raw, not Huffman-coded.

In the bit stream, a Match token is encoded as:
```
[LL code for length symbol] [extra_bits for exact length] [dist code] [extra_bits for exact distance]
```

### Comparison: CMP04 vs. CMP05 Token Stream

```
CMP04 (plain Huffman):
  Input:   b"AAABBC"
  Symbols: A, A, A, B, B, C  (raw bytes, no LZ preprocessing)
  LL tree: {A→"0", B→"10", C→"11"}   (3 symbols)

CMP05 (DEFLATE):
  Input:   b"AABCBBABC"
  LZSS:    Lit('A'), Lit('A'), Lit('B'), Lit('C'), Lit('B'), Lit('B'), Match(5,3)
  LL tree: {B→"0", A→"100", C→"101", 256→"110", 257→"111"}   (5 symbols)
  Dist tree: {code_4→"0"}   (1 symbol)
  Extra bits: length 3 → 0 bits, distance 5 → 1 bit (value=0)
```

### LSB-First Bit Packing

Same convention as CMP02/CMP03/CMP04: bits are packed into bytes LSB-first. Both
Huffman codes and raw extra bits are written sequentially into the same bit stream using
this convention.

For extra bits, the LEAST significant bit of the extra value is emitted first. So
extra_value=7 with 3 bits is emitted as "111" (7 in binary, same either way since it's
symmetric for all-ones). More subtly, extra_value=5 with 3 bits: 5 = 0b101, emitted as
bit0=1, bit1=0, bit2=1 (LSB first).

## Wire Format (CMP05)

```
Header (8 bytes):
  Bytes 0–3:   original_length   — big-endian uint32. Byte length of uncompressed data.
  Bytes 4–5:   ll_entry_count    — big-endian uint16. Entries in LL code-length table.
  Bytes 6–7:   dist_entry_count  — big-endian uint16. Entries in distance table (0 if
                                   no matches in the input).

LL code-length table (ll_entry_count × 3 bytes each):
  [2 bytes] symbol      — big-endian uint16. LL symbol value (0–284).
  [1 byte]  code_length — uint8. Huffman code length (1–16).
  Entries sorted by (code_length ASC, symbol ASC).

Distance code-length table (dist_entry_count × 3 bytes each):
  [2 bytes] symbol      — big-endian uint16. Distance code (0–23).
  [1 byte]  code_length — uint8. Huffman code length (1–16).
  Entries sorted by (code_length ASC, symbol ASC).
  Omitted entirely (dist_entry_count=0) when input has no matches.

Bit stream (remaining bytes):
  LSB-first packed bits. Encoding:
    For each LZSS token in sequence:
      Literal(byte):    LL Huffman code for symbol `byte`.
      Match(off, len):  LL Huffman code for length_symbol(len)
                      + extra_bits(len)   [raw, LSB-first]
                      + dist code for dist_code(off)
                      + extra_bits(off)   [raw, LSB-first]
    At end:             LL Huffman code for symbol 256 (end-of-data).
  Zero-padded to byte boundary.
```

### Key Differences from CMP04

| Feature              | CMP04              | CMP05                              |
|----------------------|--------------------|------------------------------------|
| LZ preprocessing     | None               | LZSS tokenization first            |
| LL alphabet size     | 0–255 (256 syms)   | 0–284 (285 syms, lengths included) |
| End-of-data          | original_length    | Explicit symbol 256 in LL tree     |
| Distance tree        | None               | Separate dist Huffman tree         |
| Extra bits           | None               | Raw bits after length/dist codes   |
| Table entry width    | 2 bytes (sym+len)  | 3 bytes (sym 2B + len 1B)          |

CMP05 uses an explicit end-of-data symbol (256) instead of original_length counting
because the number of tokens does not correspond 1:1 to output bytes (a single Match
token can expand to many bytes).

## Encoding Algorithm

```
function compress(data: bytes,
                  window_size: int = 4096,
                  max_match:   int = 255,
                  min_match:   int = 3) -> bytes:

    if len(data) == 0:
        # No symbols at all — empty LL tree with just the end-of-data symbol.
        return pack(">IHHHH", 0, 1, 0) + encode_single_symbol_tree(256)

    # ── Pass 1: LZSS tokenization (same algorithm as CMP02) ──────────────────
    tokens ← lzss_tokenize(data, window_size, max_match, min_match)

    # ── Pass 2a: Tally symbol frequencies ────────────────────────────────────
    ll_freq   ← Counter()   # literal/length symbols
    dist_freq ← Counter()   # distance codes

    for token in tokens:
        if token is Literal(byte):
            ll_freq[byte] += 1
        else:  # Match(offset, length)
            ll_freq[length_symbol(token.length)] += 1
            dist_freq[dist_code(token.offset)]   += 1
    ll_freq[256] += 1  # end-of-data marker

    # ── Pass 2b: Build canonical Huffman trees (via DT27) ────────────────────
    ll_tree   ← HuffmanTree.build(list(ll_freq.items()))
    ll_table  ← ll_tree.canonical_code_table()   # {symbol: bit_string}

    dist_table ← {}
    if dist_freq:
        dist_tree  ← HuffmanTree.build(list(dist_freq.items()))
        dist_table ← dist_tree.canonical_code_table()

    # ── Pass 2c: Encode token stream ─────────────────────────────────────────
    bits ← ""
    for token in tokens:
        if token is Literal(byte):
            bits += ll_table[byte]
        else:  # Match(offset, length)
            sym   = length_symbol(token.length)
            extra = token.length - LENGTH_BASE[sym]
            ebits = LENGTH_EXTRA[sym]
            bits += ll_table[sym]
            if ebits > 0:
                bits += format(extra, f"0{ebits}b")[::-1]  # LSB-first extra bits

            dc    = dist_code(token.offset)
            dextra = token.offset - DIST_BASE[dc]
            debits = DIST_EXTRA[dc]
            bits += dist_table[dc]
            if debits > 0:
                bits += format(dextra, f"0{debits}b")[::-1]  # LSB-first extra bits
    bits += ll_table[256]  # end-of-data

    bit_bytes ← pack_bits_lsb_first(bits)

    # ── Assemble wire format ──────────────────────────────────────────────────
    ll_lengths   ← sorted([(s, len(b)) for s,b in ll_table.items()],   key=lambda p: (p[1],p[0]))
    dist_lengths ← sorted([(s, len(b)) for s,b in dist_table.items()], key=lambda p: (p[1],p[0]))

    header   ← pack(">IHH", len(data), len(ll_lengths), len(dist_lengths))
    ll_bytes ← concat(pack(">HB", s, l) for s,l in ll_lengths)
    dt_bytes ← concat(pack(">HB", s, l) for s,l in dist_lengths)

    return header + ll_bytes + dt_bytes + bit_bytes


# ── Helper: length symbol lookup ─────────────────────────────────────────────
function length_symbol(length: int) -> int:
    # Find S such that LENGTH_BASE[S] <= length < LENGTH_BASE[S+1].
    for S in range(257, 285):
        if length <= LENGTH_MAX[S]:
            return S
    return 284  # max


# ── Helper: distance code lookup ─────────────────────────────────────────────
function dist_code(offset: int) -> int:
    # Find C such that DIST_BASE[C] <= offset < DIST_BASE[C+1].
    for C in range(0, 24):
        if offset <= DIST_MAX[C]:
            return C
    return 23  # max
```

**Note on extra-bits byte order:** Extra bits are emitted with the **least-significant
bit first** (same LSB-first convention as the rest of the bit stream). For a 3-bit
extra value of 5 (binary 101), the bits emitted are: 1 (lsb), 0, 1 (msb). The decoder
reads them back in the same order.

## Decoding Algorithm

```
function decompress(data: bytes) -> bytes:

    # ── Parse header ────────────────────────────────────────────────────────
    original_length ← unpack(">I", data[0:4])
    ll_entry_count  ← unpack(">H", data[4:6])
    dist_entry_count← unpack(">H", data[6:8])

    if original_length == 0: return b""

    offset ← 8

    # ── Parse LL code-length table ──────────────────────────────────────────
    ll_lengths ← []
    for i in range(ll_entry_count):
        symbol ← unpack(">H", data[offset:offset+2])
        length ← data[offset+2]
        ll_lengths.append((symbol, length))
        offset += 3

    # ── Parse distance code-length table ───────────────────────────────────
    dist_lengths ← []
    for i in range(dist_entry_count):
        symbol ← unpack(">H", data[offset:offset+2])
        length ← data[offset+2]
        dist_lengths.append((symbol, length))
        offset += 3

    # ── Reconstruct canonical Huffman codes ────────────────────────────────
    ll_code_table   ← reconstruct_canonical_codes(ll_lengths)
    dist_code_table ← reconstruct_canonical_codes(dist_lengths)

    # ── Unpack bit stream ──────────────────────────────────────────────────
    bits ← unpack_bits_lsb_first(data[offset:])
    bit_pos ← 0

    function read_bits(n: int) -> int:
        # Read n raw bits LSB-first, return as integer.
        val ← 0
        for i in range(n):
            val |= int(bits[bit_pos + i]) << i
        bit_pos += n
        return val

    function next_huffman_symbol(code_table: dict) -> int:
        # Read bits until we match a code in code_table.
        acc ← ""
        while True:
            acc += bits[bit_pos]; bit_pos += 1
            if acc in code_table: return code_table[acc]

    # ── Decode token stream ────────────────────────────────────────────────
    output ← []
    while True:
        ll_sym ← next_huffman_symbol(ll_code_table)

        if ll_sym == 256:
            break  # end-of-data

        elif ll_sym < 256:
            output.append(ll_sym)  # literal byte

        else:  # ll_sym is 257–284: length code
            extra_bits ← LENGTH_EXTRA[ll_sym]
            length ← LENGTH_BASE[ll_sym] + read_bits(extra_bits)

            dist_sym   ← next_huffman_symbol(dist_code_table)
            extra_bits ← DIST_EXTRA[dist_sym]
            offset     ← DIST_BASE[dist_sym] + read_bits(extra_bits)

            # Copy length bytes from output[-offset] byte-by-byte (supports overlap).
            start ← len(output) - offset
            for i in range(length):
                output.append(output[start + i])

    return bytes(output)
```

### Worked Example: "AABCBBABC"

LZSS tokenization (window=4096, min_match=3):
```
cursor=0: window empty → Literal('A')
cursor=1: window=[A],   A@1: len=1 < 3 → Literal('A')
cursor=2: window=[AA],  B → no match → Literal('B')
cursor=3: window=[AAB], C → no match → Literal('C')
cursor=4: window=[AABC], B@1: len=1 < 3 → Literal('B')
cursor=5: window=[AABCB], B@1: len=1 < 3 → Literal('B')
cursor=6: window=[AABCBB]:
  pos=2 (B): B=A?→No; pos=0 (A): A=A✓,B=B? wait…
  scanning: pos=0(A): A✓,then data[7]='B',window[1]='A'→No, len=1.
            pos=3(C): C≠A. pos=4(B): B≠A. pos=5(B): B≠A.
            best len=1 < 3 → Literal('A')  [NOTE: A at window[0] or [1] matches 1]

Hmm let me reconsider cursor=6 more carefully.
data = A A B C B B A B C  (indices 0-8)
cursor=6, data[6]=A, data[7]=B, data[8]=C
window = data[0..5] = A A B C B B
Check each position in window:
  pos=0 (A): data[6]='A'==window[0]='A' ✓
             data[7]='B'==window[1]='A' ✗ → len=1
  pos=1 (A): data[6]='A'==window[1]='A' ✓
             data[7]='B'==window[2]='B' ✓
             data[8]='C'==window[3]='C' ✓ → len=3, offset=cursor-pos=6-1=5

cursor=6: Match(offset=5, length=3), cursor→9.
cursor=9: end of input.

Tokens: [Lit(A), Lit(A), Lit(B), Lit(C), Lit(B), Lit(B), Match(offset=5, length=3)]
```

Build LL frequency table:
```
A(65) → 2,  B(66) → 3,  C(67) → 1
length=3 → symbol 257 (base=3, extra_bits=0) → freq 1
256(end) → 1
```

Build distance frequency table:
```
offset=5 → dist_code=4 (base=5, extra_bits=1, extra_value=0) → freq 1
```

Build canonical Huffman trees:

**LL tree** (frequencies: B=3, A=2, C=1, 256=1, 257=1):
```
Heap: [(1,C), (1,256), (1,257), (2,A), (3,B)]
Step 1: Pop C(1),256(1)→N1(2).   Heap: [(1,257),(2,A),(2,N1),(3,B)]
Step 2: Pop 257(1),A(2)→N2(3).   Heap: [(2,N1),(3,B),(3,N2)]
Step 3: Pop N1(2),B(3)→N3(5).    Heap: [(3,N2),(5,N3)]
Step 4: Pop N2(3),N3(5)→root(8).

Depths:  257→3, C→3, 256→3, A→3, B→2

Canonical codes (sorted by len, then symbol):
  (2, B=66):  code=0           → "00"
  (3, A=65):  code=0<<1+0=0→  wait, code advances:
    prev_len=2, code=0 → B gets "00"; code→1
    len jumps to 3: code = 1 << (3-2) = 2
  A(65):  code=2 → "010";  code→3
  C(67):  code=3 → "011";  code→4
  256:    code=4 → "100";  code→5
  257:    code=5 → "101";  code→6
```

**Distance tree** (only dist code 4, freq=1):
```
Single symbol → code length=1, code="0"
```

Bit stream assembly (tokens → bits):
```
Lit(A)=010, Lit(A)=010, Lit(B)=00, Lit(C)=011,
Lit(B)=00,  Lit(B)=00,
Match(5,3):
  LL for len 3 (symbol 257)="101", extra_bits=0 (no extra)
  Dist code 4 = "0",       extra_bits=1, extra_value=5−5=0 → "0"
End (256)="100"

Full bit string:
  0,1,0, 0,1,0, 0,0, 0,1,1, 0,0, 0,0, 1,0,1, 0, 0, 1,0,0
  [A  ] [A  ] [B] [C  ] [B] [B] [257 ] [d4][de][256]
  Total: 3+3+2+3+2+2+3+1+1+3 = 23 bits → 3 bytes (1 padding bit)

LSB-first byte packing:
  Bits 0–7 : 0,1,0,0,1,0,0,0 → 0×1+1×2+0×4+0×8+1×16+0×32+0×64+0×128 = 18 = 0x12
  Bits 8–15: 1,0,0,0,0,1,0,0 → 1+0+0+0+0+32+0+0 = 33 = 0x21
  Bits 16–23: 0,1,1,0,0,1,0,0 → 0+2+4+0+0+32+0+0 = 38 = 0x26
```

Wire bytes for "AABCBBABC":
```
original_length:  00 00 00 09
ll_entry_count:   00 05
dist_entry_count: 00 01

LL table (sorted by len ASC, symbol ASC):
  B  (66),  len=2: 00 42 02
  A  (65),  len=3: 00 41 03
  C  (67),  len=3: 00 43 03
  256,      len=3: 01 00 03
  257,      len=3: 01 01 03

Dist table:
  dist_code 4, len=1: 00 04 01

Bit stream: 12 21 26

Total wire bytes: 4 + 2 + 2 + 15 + 3 + 3 = 29 bytes
(expands 9→29; overhead dominates for tiny inputs)
```

Verification: Decode bit stream "010 010 00 011 00 00 101 0 0 100" against trees above:
- "010" → A ✓
- "010" → A ✓
- "00"  → B ✓
- "011" → C ✓
- "00"  → B ✓
- "00"  → B ✓
- "101" → 257 (length code): base=3, extra_bits=0 → length=3
- "0"   → dist code 4: base=5, extra_bits=1 → read 1 raw bit = "0" → offset=5+0=5
- Copy 3 bytes from position (6−5)=1: output[1]='A', output[2]='B', output[3]='C' → appends "ABC"
- "100" → 256 (end-of-data) → stop
- Output: "AABCBBABC" ✓

## Parameters

| Parameter   | Default | Meaning                                               |
|-------------|---------|-------------------------------------------------------|
| window_size | 4096    | Max lookback distance for LZSS matching.              |
| max_match   | 255     | Max match length (fits in our length code table).     |
| min_match   | 3       | Minimum match length to emit a Match token.           |

The window_size=4096 cap means distance codes 0–23 suffice (dist code 23 covers up to 4096).
The max_match=255 cap means length codes 257–284 suffice (symbol 284 covers up to 255).

## Interface Contract

```
compress(data: bytes,
         window_size: int = 4096,
         max_match:   int = 255,
         min_match:   int = 3) -> bytes
  Returns CMP05 wire-format bytes.
  compress(b"") → minimal header (original_length=0, single end-of-data symbol).

decompress(data: bytes) -> bytes
  Returns original bytes from CMP05 wire-format input.
  decompress(compress(b"")) → b"".

Round-trip invariant: decompress(compress(x)) == x   for all x: bytes
```

**Dependencies:**
- `coding-adventures-lzss` (CMP02) — LZSS tokenization and the `Literal`/`Match` token types.
- `coding-adventures-huffman-tree` (DT27) — Huffman tree construction and canonical codes.

## Length Code Table (constant)

```python
# (base_length, extra_bits) indexed by LL symbol 257–284.
LENGTH_CODES = {
    257: (3,  0),  258: (4,  0),  259: (5,  0),  260: (6,  0),
    261: (7,  0),  262: (8,  0),  263: (9,  0),  264: (10, 0),
    265: (11, 1),  266: (13, 1),  267: (15, 1),  268: (17, 1),
    269: (19, 2),  270: (23, 2),  271: (27, 2),  272: (31, 2),
    273: (35, 3),  274: (43, 3),  275: (51, 3),  276: (59, 3),
    277: (67, 4),  278: (83, 4),  279: (99, 4),  280: (115, 4),
    281: (131, 5), 282: (163, 5), 283: (195, 5), 284: (227, 5),
}
# Max length per symbol = base + (2**extra_bits - 1), capped at max_match=255.
```

## Distance Code Table (constant)

```python
# (base_distance, extra_bits) indexed by distance code 0–23.
DIST_CODES = [
    (1,    0), (2,    0), (3,    0), (4,    0),
    (5,    1), (7,    1), (9,    2), (13,   2),
    (17,   3), (25,   3), (33,   4), (49,   4),
    (65,   5), (97,   5), (129,  6), (193,  6),
    (257,  7), (385,  7), (513,  8), (769,  8),
    (1025, 9), (1537, 9), (2049, 10),(3073, 10),
]
# Max distance per code = base + (2**extra_bits - 1), capped at window_size=4096.
```

## Test Vectors

All vectors use `window_size=4096, max_match=255, min_match=3`.

### 1. Empty input

```
compress(b"") → header with original_length=0
decompress(compress(b"")) == b""
```

### 2. No matches (all literals) — "AAABBC"

```
LZSS tokens: all literals [Lit(A)×3, Lit(B)×2, Lit(C)]
LL symbols: A(65)=3, B(66)=2, C(67)=1, 256(end)=1
Dist symbols: none → dist_entry_count=0

LL Huffman tree (same construction as CMP04 reference vector):
  Code lengths: A=1, B=2, C=3, 256=3
  Canonical codes: A→"0", B→"10", C→"110", 256→"111"

Bit stream:
  A=0, A=0, A=0, B=10, B=10, C=110, 256=111
  = "000" + "10" + "10" + "110" + "111"
  = "000101011" + "0111"  (13 bits → 2 bytes)

LSB-first:
  Byte 0: bit0=0,1=0,2=0,3=1,4=0,5=1,6=0,7=1 → 8+32+128 = 168 = 0xA8
  Byte 1: bit0=1,1=0,2=1,3=1,4=1,5..7=0     → 1+4+8+16 = 29  = 0x1D

Wire bytes:
  00 00 00 06          (original_length = 6)
  00 04                (ll_entry_count = 4)
  00 00                (dist_entry_count = 0)
  00 41 01             (A=65, len=1)
  00 42 02             (B=66, len=2)
  00 43 03             (C=67, len=3)
  01 00 03             (256,  len=3)
  A8 1D                (bit stream)
```

### 3. Mixed literals + match — "AABCBBABC"

See the worked example above. Wire bytes:
```
  00 00 00 09          (original_length = 9)
  00 05                (ll_entry_count = 5)
  00 01                (dist_entry_count = 1)
  00 42 02             (B=66,  len=2)
  00 41 03             (A=65,  len=3)
  00 43 03             (C=67,  len=3)
  01 00 03             (256,   len=3)
  01 01 03             (257,   len=3)
  00 04 01             (dist_code_4, len=1)
  12 21 26             (bit stream)
```

### 4. Long repetition — "AAABBBAAABBB" (12 bytes)

```
LZSS tokens (min_match=3):
  cursor=0–2: Lit(A)×3
  cursor=3–5: Lit(B)×3
  cursor=6: window=[AAABBB], data[6..11]='AAABBB'
    pos=0: A=A✓, A=A✓, A=A✓, B=B✓, B=B✓, B=B✓ → len=6
    Match(offset=6, length=6)
  cursor=12: end.

Tokens: [Lit(A)×3, Lit(B)×3, Match(offset=6, length=6)]

Length=6 → symbol 260 (base=6, extra_bits=0)
Distance=6 → dist_code=4 (base=5, extra_bits=1, extra_value=6−5=1 → bit "1")

LL freqs: A=3, B=3, 260=1, 256=1
Dist freqs: code_4=1

Build LL Huffman (A=3, B=3, 256=1, 260=1):
  Pop 256(1),260(1) → N1(2).  Heap: [N1(2), A(3), B(3)]
  Pop N1(2), A(3)   → N2(5).  Heap: [B(3), N2(5)]
  Pop B(3), N2(5)   → root(8).
  Depths: B=1, N1=2→256=3, 260=3; A=2

  Canonical (sorted by len, sym):
    B(66)  len=1: "0"
    A(65)  len=2: code = 1<<1 = 2 → "10"
    256    len=3: code = (2+1)<<1 = 6 → "110"
    260    len=3: code = 7 → "111"

Bit stream:
  A=10, A=10, A=10, B=0, B=0, B=0, 260=111 (0 extra bits), dist_4="0" + "1" (extra), 256=110
  Bits: 10,10,10, 0,0,0, 111, 0,1, 110
  = 1,0,1,0,1,0,0,0,0,1,1,1,0,1,1,1,0   (17 bits → 3 bytes with 7 padding bits)

Wait, the extra bit for dist_code=4 is extra_value=6−5=1, and extra_bits=1, so we emit "1".
The extra_value=1 in binary is "1", LSB-first = "1". ✓

Byte 0 (bits 0-7): 1,0,1,0,1,0,0,0 → 1+4+16 = 21 = 0x15
Byte 1 (bits 8-15): 0,1,1,1,0,1,1,1 → 2+4+8+32+64+128 = 238 = 0xEE (wait)
  bit8=0→0, bit9=1→2, bit10=1→4, bit11=1→8, bit12=0→0, bit13=1→32, bit14=1→64, bit15=1→128
  = 0+2+4+8+0+32+64+128 = 238 = 0xEE
Byte 2 (bits 16): 0 + 7 padding zeros → 0x00

Wire bytes:
  00 00 00 0C          (original_length = 12)
  00 04                (ll_entry_count = 4)
  00 01                (dist_entry_count = 1)
  00 42 01             (B=66, len=1)
  00 41 02             (A=65, len=2)
  01 00 03             (256,  len=3)
  01 04 03             (260,  len=3)
  00 04 01             (dist_code_4, len=1)
  15 EE 00             (bit stream)
```

### 5. Overlapping match — "AAAAAAA" (7 bytes)

```
LZSS tokens: Lit(A), Match(offset=1, length=6)

Length=6 → symbol 260 (extra_bits=0)
Distance=1 → dist_code=0 (base=1, extra_bits=0, extra_value=0)

LL freqs: A(65)=1, 260=1, 256=1
Dist freqs: code_0=1

LL Huffman (3 symbols of freq 1 each):
  Pop any two, merge:
    Pop 256(1), 260(1) → N1(2). Heap: [A(1), N1(2)]  ← actually A has freq 1 too
    Wait: A=1, 256=1, 260=1.
    Pop 256(1), 260(1) → N1(2). Heap: [A(1), N1(2)]
    Pop A(1), N1(2) → root(3).
  Depths: A=1; 256=2, 260=2

  Canonical (sorted):
    A(65)  len=1: "0"
    256    len=2: code=1<<1=2 → "10"
    260    len=2: code=3 → "11"

Bit stream:
  A=0, 260=11(0 extra), dist_0="0"(0 extra), 256=10
  = 0, 1,1, 0, 1,0  (6 bits → 1 byte with 2 padding)
  Byte 0: bit0=0,1=1,2=1,3=0,4=1,5=0,6=0,7=0 → 2+4+16 = 22 = 0x16

decompress: output=[A]; copy 6 bytes from offset 1 (pos 0), byte-by-byte:
  → [A,A,A,A,A,A,A] ✓
```

### 6. Binary data

```python
data = bytes(range(256)) * 4   # 1024 bytes, all byte values
assert decompress(compress(data)) == data
```

### 7. Round-trip invariant

```python
for s in [b"", b"A", b"AAABBC", b"ABABAB", b"AABCBBABC",
          b"AAABBBAAABBB", b"AAAAAAA", bytes(range(256))]:
    assert decompress(compress(s)) == s
```

### 8. Compression ratio beats CMP02 (LZSS) and CMP04 (Huffman alone)

```python
import random
random.seed(42)
# Skewed distribution: many A's, fewer others
data = b"A" * 5000 + b"B" * 1000 + b"C" * 500 + b"D" * 200

lzss_size    = len(lzss_compress(data))
huffman_size = len(huffman_compress(data))
deflate_size = len(compress(data))

# DEFLATE should beat both on most realistic data
assert deflate_size <= min(lzss_size, huffman_size)
```

## Comparison with Prior Algorithms

| Property            | CMP02 LZSS         | CMP04 Huffman      | CMP05 DEFLATE              |
|---------------------|--------------------|--------------------|----------------------------|
| Exploits repetition | Yes (sliding win)  | No                 | Yes (via LZSS pass)        |
| Exploits statistics | No                 | Yes                | Yes (via dual Huffman)     |
| Dependencies        | None               | DT27 huffman-tree  | CMP02 lzss + DT27          |
| Header overhead     | 8 bytes            | 8 + 2N bytes       | 8 + 3(M+K) bytes           |
| Alphabet size       | N/A                | ≤ 256 symbols      | ≤ 285 LL + ≤ 24 dist syms  |
| End-of-data         | original_length    | original_length    | Symbol 256 in LL tree      |
| Extra bits          | None               | None               | Raw bits after each code   |
| Best on             | Repetitive data    | Skewed alphabets   | Most real-world data       |

## Package Matrix

| Language   | Package                               | Build command           | Depends on              |
|------------|---------------------------------------|-------------------------|-------------------------|
| Python     | `coding-adventures-deflate`           | `pytest tests/ -v`      | CMP02 lzss, DT27        |
| Go         | `github.com/.../go/deflate`           | `go test ./... -v`      | CMP02 lzss, DT27        |
| Ruby       | `coding_adventures_deflate`           | `bundle exec rake test` | CMP02 lzss, DT27        |
| TypeScript | `@coding-adventures/deflate`          | `npx vitest run`        | CMP02 lzss, DT27        |
| Rust       | `deflate`                             | `cargo test`            | CMP02 lzss, DT27        |
| Elixir     | `coding_adventures_deflate`           | `mix test`              | CMP02 lzss, DT27        |
| Lua        | `coding_adventures_deflate`           | `busted .`              | CMP02 lzss, DT27        |
| Perl       | `CodingAdventures::Deflate`           | `prove -l -v t/`        | CMP02 lzss, DT27        |
| Swift      | `Deflate`                             | `swift test`            | CMP02 lzss, DT27        |
