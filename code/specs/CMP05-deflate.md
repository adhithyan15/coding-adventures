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

#### Encoder subset vs. full-standard decoder

The table above (24 distance codes, 0–23) is the subset our **encoder** emits: it
uses a 4096-byte window, so it never needs a distance beyond 4096. RFC 1951 itself,
however, defines a **32768-byte window** with distance codes **0–29** (code 29 reaches
32768) and one extra length code — **LL symbol 285** for the maximum match length of
258 with no extra bits. Every mainstream producer (zlib, gzip, and Microsoft Office
when it writes OOXML) uses the full range.

Because a decoder must read *anyone's* output — not just ours — `inflate` implements
the **complete** RFC 1951 alphabet: distance codes 0–29 and LL length symbol 285.
This is the asymmetry at the heart of interoperability: **encode conservatively (a
small window keeps the implementation simple), decode liberally (the full standard so
real files open).** Omitting 285 / codes 24–29 makes a decoder that can only read its
own output — which is exactly why reading a real `.xlsx` used to fail.

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

## Wire Format — standard RFC 1951

`compress` emits a **standard RFC 1951 raw DEFLATE stream** — the exact bytes a
ZIP entry or gzip body carries, with no envelope and no private header. This is a
deliberate choice: an educational codec is only convincing if a real tool
(`zlib`, `gzip`, `unzip`, a browser) can read its output. It emits a **single
final block**, choosing per input between a **fixed-Huffman block** (BTYPE=01,
pre-agreed §3.2.6 tables, nothing transmitted) and a **dynamic-Huffman block**
(BTYPE=10, code lengths adapted to the data and transmitted inline) — whichever
is smaller in exact emitted bits:

```
Block header (3 bits, LSB-first):
  bit 0     BFINAL = 1   (this is the only, final block)
  bits 1–2  BTYPE  = 01  (fixed Huffman)  OR  10  (dynamic Huffman)

(dynamic only) Header:
  HLIT  = read 5 bits = (#LL codes − 257)
  HDIST = read 5 bits = (#dist codes − 1)
  HCLEN = read 4 bits = (#CL lengths − 4)
  CL code lengths in CL_PERMUTATION order          [3 bits each, LSB-first]
  RLE'd (LL ++ dist) code lengths via CL symbols    [CL code MSB-first + extra LSB]

Token stream (LSB-first packed bits):
  For each LZSS token in sequence:
    Literal(byte):   LL Huffman code for symbol `byte`         [MSB-first code]
    Match(off, len): LL code for length_symbol(len)            [MSB-first code]
                   + extra_bits(len)                           [raw, LSB-first]
                   + distance code dist_code(off)              [MSB-first code]
                   + extra_bits(off)                           [raw, LSB-first]
  At end:            LL code for symbol 256 (end-of-block)

Zero-padded to the next byte boundary.
```

For a **fixed** block the literal/length codes are the canonical assignment of
RFC 1951 §3.2.6 (8-bit codes for symbols 0–143, 9-bit for 144–255, 7-bit for
256–279, 8-bit for 280–287) and distance symbols are 5-bit codes equal to the
symbol number, so no table is transmitted. For a **dynamic** block the LL and
distance code lengths are computed from the token frequencies and transmitted (as
above), giving better ratios on skewed data (text, repetitive input) where the
fixed tables waste 8–9 bits per literal.

**Length-limiting is mandatory.** RFC 1951 caps codes at 15 bits (LL and
distance) and 7 bits (the code-length alphabet), but an optimal Huffman tree over
up to 286 symbols can exceed 15 bits on skewed frequencies. `compress` therefore
builds its dynamic trees with the **package-merge** algorithm (Larmore–Hirschberg
1990), which produces the *optimal* code subject to a maximum length and provably
always yields a valid prefix code (Kraft sum ≤ 1) whenever the alphabet fits in
`2^max_len` symbols — which all three alphabets do (286 ≤ 2¹⁵, 30 ≤ 2¹⁵,
19 ≤ 2⁷). A plain (unlimited) Huffman tree could emit a >15-bit code and produce
an *invalid* stream, so this step is a correctness requirement, not merely an
optimisation. The implementation asserts `len ≤ max_len` and Kraft ≤ 1 so a
malformed tree can never reach the wire.

The choice between fixed and dynamic is made by computing the **exact bit length**
of each encoding of the same LZSS token stream and picking the minimum. Two
consequences: `compress` never emits a stream *larger* than the old fixed-only
encoder, and on tiny or near-incompressible inputs — where the dynamic
code-length header costs more than it saves — it transparently falls back to
fixed. Edge cases follow RFC 1951 §3.2.7: a block with no matches still emits a
valid `HDIST` with one dummy distance code of length 1, and single-symbol
alphabets receive a valid 1-bit code.

The **decoder** (`inflate`) reads all three block types — stored, fixed, and
dynamic — so it decodes `zlib`/`gzip`/Office streams as well as `compress`'s own
output.

### Key Differences from CMP04

| Feature              | CMP04              | CMP05                              |
|----------------------|--------------------|------------------------------------|
| LZ preprocessing     | None               | LZSS tokenization first            |
| LL alphabet size     | 0–255 (256 syms)   | 0–285 (lengths + max-match code)   |
| End-of-data          | original_length    | Explicit symbol 256 (end-of-block) |
| Distance codes       | None               | Separate distance alphabet (0–29)  |
| Extra bits           | None               | Raw bits after length/dist codes   |
| Output format        | Custom             | Standard RFC 1951 raw DEFLATE      |

CMP05 uses an explicit end-of-block symbol (256) instead of an original-length
count because the number of tokens does not correspond 1:1 to output bytes (a
single Match token can expand to many bytes).

## Encoding Algorithm

```
function compress(data: bytes,
                  window_size: int = 32768,
                  max_match:   int = 255,
                  min_match:   int = 3) -> bytes:
    # Tokenize once, then emit ONE final block — fixed (BTYPE=01) or dynamic
    # (BTYPE=10), whichever is smaller in exact bits.

    # ── LZSS tokenization (same algorithm as CMP02) ─────────────────────────
    tokens ← lzss_tokenize(data, window_size, max_match, min_match)

    # ── Cost both encodings of the SAME token stream, pick the smaller ──────
    fixed_bits   ← fixed_block_bits(tokens)          # 3 + Σ code widths + EOB
    plan         ← plan_dynamic(tokens)              # builds length-limited trees
    if plan.total_bits < fixed_bits:
        return emit_dynamic_block(tokens, plan)      # BFINAL=1, BTYPE=10
    else:
        return emit_fixed_block(tokens)              # BFINAL=1, BTYPE=01


# ── Building a dynamic block (RFC 1951 §3.2.7) ──────────────────────────────
function plan_dynamic(tokens) -> DynamicPlan:
    # 1. Count LL (286) and distance (30) symbol frequencies; EOB counts once.
    ll_freq, dist_freq ← count_frequencies(tokens)

    # 2. Length-limited Huffman: LL/dist ≤ 15 bits, via package-merge.
    ll_len   ← length_limited_huffman(ll_freq,   max_len = 15)
    dist_len ← length_limited_huffman(dist_freq, max_len = 15)
    if no distance code present: dist_len[0] ← 1   # RFC needs ≥1 dist code (dummy)

    # 3. Trim to HLIT (≥257) and HDIST (≥1); RLE-encode (LL ++ dist) lengths
    #    with CL symbols 0–18 (16=repeat 3–6, 17=zeros 3–10, 18=zeros 11–138).
    rle ← rle_code_lengths(ll_len[:HLIT] ++ dist_len[:HDIST])

    # 4. Length-limited CL Huffman ≤ 7 bits.
    cl_len ← length_limited_huffman(count(rle.symbols), max_len = 7)

    # 5. total_bits = header + CL lengths (3 bits each, permutation order)
    #              + Σ (CL code + extra) + Σ token code widths + EOB
    return DynamicPlan{ ll_len, dist_len, cl_len, rle, total_bits }


# ── Length-limited Huffman: package-merge (Larmore–Hirschberg 1990) ─────────
# Produces the OPTIMAL prefix code with every length ≤ max_len.  Correct because:
#   • Kraft's inequality: a code is valid iff Σ 2^(−ℓ_i) ≤ 1.  Package-merge
#     solves the equivalent "coin-collector" problem and provably attains the
#     minimum-cost length-limited code (Larmore–Hirschberg, JACM 1990).
#   • It always yields a VALID code whenever n ≤ 2^max_len — true for all our
#     alphabets (286 ≤ 2¹⁵, 30 ≤ 2¹⁵, 19 ≤ 2⁷) — so no length ever exceeds the cap.
function length_limited_huffman(freqs, max_len) -> lengths:
    present ← symbols with freqs > 0
    if |present| == 1: return length 1 for that symbol   # valid 1-bit code
    list ← originals (one coin per present symbol, weight = freq), sorted asc.
    repeat (max_len − 1) times:
        packages ← pair adjacent items of `list` (sum weights), drop odd tail
        list     ← merge(originals, packages)  sorted by weight
    select the 2·|present| − 2 lowest-weight items of `list`
    lengths[s] ← number of selected items covering symbol s
    assert every length ∈ [1, max_len]  and  Σ 2^(max_len−ℓ) ≤ 2^max_len

# ── Fixed literal/length codes (RFC 1951 §3.2.6), returned MSB-first ─────────
function fixed_ll_code(sym: int) -> bitstring:
    if   0   <= sym <= 143: return bin(0b0011_0000  + sym,        width=8)
    elif 144 <= sym <= 255: return bin(0b1_1001_0000 + sym - 144, width=9)
    elif 256 <= sym <= 279: return bin(sym - 256,                 width=7)
    else:                   return bin(0b1100_0000  + sym - 280,  width=8)  # 280–287

# Fixed distance codes are 5-bit values equal to the symbol number (MSB-first).
function fixed_dist_code(code: int) -> bitstring:  return bin(code, width=5)
```

**Note on bit order:** Huffman codes are emitted **MSB-first** (the canonical
convention), while extra bits and the block header are emitted **LSB-first**. The
decoder reverses each Huffman code as it accumulates bits and reads extra bits
directly — see `inflate`.

## Decoding Algorithm

`decompress` is an alias for `inflate`, the standard RFC 1951 decoder. Because
`compress` emits standard raw DEFLATE (see *Wire Format* above), decoding is
exactly standard inflate — there is no private format to parse. `inflate`:

1. Reads each block header (`BFINAL`, `BTYPE`).
2. Dispatches on `BTYPE`:
   - **00 stored** — byte-aligned `LEN`/`NLEN`, then `LEN` verbatim bytes.
   - **01 fixed** — decode with the pre-defined §3.2.6 code tables.
   - **10 dynamic** — first read the transmitted code-length trees (the
     code-length alphabet, run-length-encoded in the `CL_PERMUTATION` order),
     then decode the literal/length and distance trees with them.
3. In fixed/dynamic blocks it loops: decode an LL symbol; `<256` is a literal,
   `256` ends the block, `257–285` is a length code followed by a distance code
   and a back-reference copy (byte-by-byte, to honour overlapping matches).
4. Repeats until the `BFINAL` block, enforcing a 256 MB output cap
   (`MAX_INFLATE_OUTPUT`) against decompression bombs.

Huffman codes are decoded by accumulating bits MSB-first and matching against the
canonical `(code, length)` table; extra bits are read LSB-first. This is the same
decoder that reads `zlib`, `gzip`, and Microsoft Office (OOXML) streams.
## Parameters

| Parameter   | Default | Meaning                                               |
|-------------|---------|-------------------------------------------------------|
| window_size | 32768   | Max lookback distance for LZSS matching (full window).|
| max_match   | 255     | Max match length (fits in our length code table).     |
| min_match   | 3       | Minimum match length to emit a Match token.           |

window_size=32768 makes every distance code 0–29 reachable (code 29 covers up to
32768). max_match=255 means the encoder only needs length codes 257–284 (symbol
284 covers up to 258); the decoder additionally recognises symbol 285 (length
258, no extra bits) from other producers.

## Interface Contract

```
compress(data: bytes) -> bytes
  Returns a standard RFC 1951 raw DEFLATE stream: one final block, fixed
  (BTYPE=01) or dynamic (BTYPE=10) Huffman, whichever is smaller in exact bits.
  Never larger than a fixed-only encoding; usually much smaller on text.
  Decodable by any conforming inflater: this crate's `inflate`, zlib, gzip, unzip.
  compress(b"") -> the 2-byte empty fixed-Huffman block `03 00`.

decompress(data: bytes) -> bytes          # alias for `inflate`
  Decodes any RFC 1951 stream (stored / fixed / dynamic Huffman).
  decompress(compress(b"")) -> b"".

Round-trip invariant: decompress(compress(x)) == x                 for all x
Standard invariant:   python_zlib.decompress(compress(x), -15) == x for all x
```

**Dependencies:**
- `coding-adventures-lzss` (CMP02) — LZSS tokenization and the `Literal`/`Match` token types.

(No `huffman-tree` dependency: fixed Huffman uses the pre-defined RFC 1951 code
tables, so no tree is constructed.)

## Length Code Table (constant)

```python
# (base_length, extra_bits) indexed by LL symbol 257-285.
LENGTH_CODES = {
    257: (3,  0),  258: (4,  0),  259: (5,  0),  260: (6,  0),
    261: (7,  0),  262: (8,  0),  263: (9,  0),  264: (10, 0),
    265: (11, 1),  266: (13, 1),  267: (15, 1),  268: (17, 1),
    269: (19, 2),  270: (23, 2),  271: (27, 2),  272: (31, 2),
    273: (35, 3),  274: (43, 3),  275: (51, 3),  276: (59, 3),
    277: (67, 4),  278: (83, 4),  279: (99, 4),  280: (115, 4),
    281: (131, 5), 282: (163, 5), 283: (195, 5), 284: (227, 5),
    285: (258, 0),  # maximum-match code: length 258, no extra bits
}
```

## Distance Code Table (constant)

```python
# (base_distance, extra_bits) indexed by distance code 0-29.
DIST_CODES = [
    (1,     0), (2,     0), (3,     0), (4,     0),
    (5,     1), (7,     1), (9,     2), (13,    2),
    (17,    3), (25,    3), (33,    4), (49,    4),
    (65,    5), (97,    5), (129,   6), (193,   6),
    (257,   7), (385,   7), (513,   8), (769,   8),
    (1025,  9), (1537,  9), (2049, 10), (3073, 10),
    (4097, 11), (6145, 11), (8193, 12), (12289,12),
    (16385,13), (24577,13),
]
# Covers the full RFC 1951 32 KB window (code 29 reaches 32768).
```

## Test Vectors

Outputs are **standard RFC 1951 raw DEFLATE** and were verified in both
directions: this crate's `inflate` decodes them, and Python's
`zlib.decompress(bytes, wbits=-15)` decodes them to the original input.

### 1. Empty input
```
compress(b"")  -> 03 00      (BFINAL=1, BTYPE=01, then the 7-bit EOB symbol 256)
inflate(03 00) -> b""
```

### 2. Literals only — "AAABBC"
```
compress(b"AAABBC") -> 73 74 74 74 72 72 06 00
  First byte 0x73 -> low 3 bits 0b011 = BFINAL=1, BTYPE=01 (fixed Huffman).
  Body: the six literals as fixed 8-bit LL codes, then EOB (256), LSB-packed.
inflate(...) == b"AAABBC"                    # and python zlib agrees
```

### 3. With matches — "AABCBBABC", "AAAAAAA" (overlap)
```
These contain repeats, so LZSS emits (length, distance) matches — exercising the
fixed length codes + extra bits and the fixed 5-bit distance codes, including the
overlapping-copy case for "AAAAAAA" (offset=1, length=6). Verified by roundtrip
and python-zlib decode in the crate's tests.
```

### 4. Round-trip and standard-decode invariants
```
for x in [b"", b"A", bytes(0..=255), repetitive text, binary]:
    assert decompress(compress(x)) == x      # our decoder (alias of inflate)
    assert inflate(compress(x))    == x       # standard RFC 1951 decoder
```

### 5. Compression
```
Repetitive input compresses (LZSS matches + fixed entropy coding): for large n,
compress(b"ABCABCABC..." * n) is shorter than the input. Fixed Huffman is not
optimal (dynamic Huffman would do better) but is standard and correct for every
input.
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
