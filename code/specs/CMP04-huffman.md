# CMP04 — Huffman Compression

## Overview

Huffman coding (David Huffman, 1952) is an **entropy coding** algorithm: it assigns
variable-length, prefix-free binary codes to symbols based on their frequency of
occurrence. Frequent symbols receive short codes; rare symbols receive long codes.
The resulting code is provably optimal — no other prefix-free code can achieve a
smaller expected bit-length for the same symbol distribution.

Unlike the LZ-family algorithms (CMP00–CMP03) which exploit **repetition** (duplicate
substrings), Huffman coding exploits **symbol statistics**. It works on individual
symbol frequencies, not on patterns of repetition. This makes it complementary to
LZ compression and explains why real-world compressors like DEFLATE (CMP05) combine
both: LZ to eliminate repeated substrings, then Huffman to optimally encode the
remaining symbol stream.

```
Series:
  CMP00 (LZ77,     1977) — Sliding-window backreferences.
  CMP01 (LZ78,     1978) — Explicit dictionary (trie).
  CMP02 (LZSS,     1982) — LZ77 + flag bits; no wasted literals.
  CMP03 (LZW,      1984) — LZ78 + pre-initialised alphabet; GIF.
  CMP04 (Huffman,  1952) — Entropy coding; prerequisite for DEFLATE.   ← YOU ARE HERE
  CMP05 (DEFLATE,  1996) — LZSS + dual Huffman; ZIP/gzip/PNG/zlib standard.
  CMP06 (Brotli,   2013) — DEFLATE + context modeling + static dict; HTTPS.
  CMP07 (Zstd,     2016) — ANS/FSE + LZ4 matching; modern universal codec.
```

## Historical Context

David Huffman was a graduate student at MIT in 1951 when his professor, Robert Fano,
offered students a choice: take a final exam, or write a term paper on the problem of
finding the most efficient binary code. Huffman chose the paper. His insight — that a
greedy bottom-up tree construction using a priority queue produces the optimal
prefix-free code — was simpler and more elegant than the top-down approach that Fano
and Claude Shannon had developed (the Shannon-Fano code). Huffman's paper "A Method
for the Construction of Minimum-Redundancy Codes" was published in the *Proceedings
of the IRE* in 1952.

Huffman coding is now ubiquitous:
- **DEFLATE** (used in ZIP, gzip, zlib, PNG): LZ77 backreferences followed by
  Huffman coding of the resulting tokens.
- **JPEG**: quantized DCT coefficients are Huffman coded.
- **MP3**: modified Huffman coding of quantized frequency bands.
- **HTTP/2 HPACK**: Huffman coding of header strings.

### Connection to Information Theory

Shannon's noiseless coding theorem states that the minimum average code length for
any lossless binary code over an alphabet with symbol probabilities `{p_s}` is
bounded below by the **Shannon entropy**:

```
H = -sum(p_s * log2(p_s))   bits per symbol
```

Huffman codes achieve this bound when all probabilities are exact powers of one half.
In general they are within 1 bit per symbol of the theoretical minimum. Arithmetic
coding can get closer still (CMP06 territory), but Huffman coding is simpler, faster,
and sufficient for most purposes.

```
Example: A(50%), B(25%), C(12.5%), D(12.5%)

Shannon entropy:
  H = -(0.5×log2(0.5) + 0.25×log2(0.25) + 0.125×log2(0.125) + 0.125×log2(0.125))
    = -(0.5×-1 + 0.25×-2 + 0.125×-3 + 0.125×-3)
    = 0.5 + 0.5 + 0.375 + 0.375 = 1.75 bits/symbol

Huffman codes: A→0 (1 bit), B→10 (2 bits), C→110 (3 bits), D→111 (3 bits)
Expected length = 0.5×1 + 0.25×2 + 0.125×3 + 0.125×3 = 1.75 bits/symbol

This alphabet hits the Shannon bound exactly because all probabilities are
powers of one half.
```

## Dependency on DT27 (Huffman Tree)

**CMP04 does not build its own Huffman tree.** It imports `huffman-tree` (DT27) and
delegates all tree construction and code derivation to that package. This is the same
pattern used by LZ78 (CMP01), which delegates trie operations to `trie` (DT13) via
the TrieCursor abstraction.

```
CMP01 (LZ78)         →  uses DT13 (Trie)       for dictionary management
CMP04 (Huffman)      →  uses DT27 (HuffmanTree) for code construction and decoding
```

The separation is intentional:
- DT27 is a pure data structure: given frequencies, build a tree and derive codes.
  It knows nothing about byte streams or wire formats.
- CMP04 is a codec: it counts frequencies, calls DT27, and handles bit I/O and
  the wire format. It treats DT27 as a black box.

### DT27 API Used by CMP04

CMP04 uses the following DT27 operations:

```
# Construction
tree = HuffmanTree.build([(symbol, frequency), ...])
  → HuffmanTree    — O(n log n)

# Encoding
table = tree.canonical_code_table()
  → {symbol: bit_string}   — canonical codes (e.g. {65: "0", 66: "10", 67: "11"})

# Transmitting the tree (wire format header)
# CMP04 reads code lengths from the canonical table:
lengths = [(symbol, len(bits)) for symbol, bits in table.items()]
  → [(symbol, code_length), ...]    sorted by (code_length, symbol)

# Decoding — reconstruct tree from transmitted lengths
# CMP04 calls HuffmanTree.build with synthesised frequencies that produce
# the correct code lengths (see "Decoder Tree Reconstruction" below)

# Symbol decoding (walk the tree bit-by-bit)
symbols = tree.decode_all(bit_string, count)
  → [symbol, ...]    — exactly `count` symbols decoded
```

The `canonical_code_table()` method is defined in DT27 §"Canonical Huffman Codes".
It sorts symbols by (code_length, symbol_value) and assigns codes numerically,
ensuring deterministic output across all language implementations.

Each language implementation of CMP04 lists `huffman-tree` (DT27) as a runtime
dependency in its build metadata:

| Language | Dependency declaration |
|---|---|
| Python | `coding-adventures-huffman-tree` in `pyproject.toml` dependencies |
| Go | `require .../code/packages/go/huffman-tree` in `go.mod` |
| Ruby | `gem "coding_adventures_huffman_tree"` in gemspec |
| TypeScript | `"@coding-adventures/huffman-tree": "file:../huffman-tree"` in package.json |
| Rust | `huffman-tree = { path = "../huffman-tree" }` in Cargo.toml |
| Elixir | `{:coding_adventures_huffman_tree, path: "../../elixir/huffman-tree"}` in mix.exs |

This mirrors how the LZ78 BUILD files listed `trie` as a dependency, and how the
LZW BUILD files avoided any such listing because LZW embeds its own dictionary.

## Key Concepts

### Why Huffman is Different from LZ Compression

Every CMP00–CMP03 algorithm finds repeated byte substrings and replaces them with
compact references. The savings come from avoiding re-sending data already seen.
Huffman works on a completely different principle:

```
LZ approach:  find repeated substrings → replace with back-references
Huffman:      count symbol frequencies → assign short codes to common symbols
```

A text that uses only three letters (e.g., "AAABBC") gains nothing from LZ (no
repeated sequences of length ≥ 3). But Huffman can compress it significantly: if
A appears 50% of the time, it gets a 1-bit code instead of 8 bits. If B is 33%,
it gets a 2-bit code. Savings are proportional to the skew of the distribution.

Conversely, a document with many repeated paragraphs gains enormously from LZ but
not necessarily from Huffman (if the alphabet is nearly uniform).

DEFLATE (CMP05) exploits both: LZ77 first eliminates repeated substrings, then
Huffman coding optimally encodes the resulting token stream.

### The Prefix-Free Property

Huffman codes are **prefix-free**: no valid codeword is a prefix of another. This
is guaranteed by the tree structure — codes are assigned only to leaves, and a leaf
cannot be an ancestor of another leaf.

```
"AAABBC" tree (tie-break: leaves before internal nodes):
       R(6)
      /    \
   A(3)   I(3)
          /   \
        C(1)  B(2)

Codes: A=0, C=10, B=11

Is "0" a prefix of "10"? No — they start with different bits.
Is "10" a prefix of "11"? No — second bit differs.
```

The prefix-free property enables unambiguous bit-stream decoding without separators:
just walk the tree, and emit a symbol whenever you reach a leaf.

### Canonical Huffman Codes

A Huffman tree is not unique — different tie-breaking rules produce different trees
with the same code lengths. Canonical Huffman coding resolves this by defining a
**unique mapping from code lengths to actual bit strings**:

1. Sort symbols by (code_length, symbol_value).
2. Assign the numerically smallest code to the first symbol at each length.
3. Increment the code for each subsequent symbol at the same length.
4. When the length increases, shift left by the length difference before assigning.

```
Example: symbols sorted by (length, symbol)
  A (length=1) → code = 0b0     = "0"
  B (length=2) → code = 0b10    = "10"   (shift: 0 → 0<<1=0, then next = 00+1 = 01... wait)

Correct canonical algorithm:
  prev_length = 1, code = 0
  A (1):  assign 0 → "0";     code = 1
  B (2):  length grew by 1 → shift: code = 1 << 1 = 2 → "10"; code = 3
  C (2):  same length       → assign 3 → "11"; code = 4
```

The critical property: given only the list of (symbol, code_length) pairs, the
decoder can reconstruct the exact canonical codes without transmitting the actual
bit patterns. This is how the CMP04 wire format works.

## Algorithm

### Encoding

```
ENCODE(input: bytes) → bytes:

  1. If input is empty:
       emit header: original_length=0, symbol_count=0
       return

  2. Count symbol frequencies:
       frequencies = {}
       for byte b in input:
         frequencies[b] += 1

  3. Build Huffman tree via DT27:
       tree = HuffmanTree.build([(s, f) for s, f in frequencies.items()])

  4. Get canonical code table via DT27:
       table = tree.canonical_code_table()
       # table: {symbol → bit_string}  e.g. {65: "0", 66: "10", 67: "11"}

  5. Build code-lengths list (for wire format header):
       lengths = sorted(
           [(symbol, len(bits)) for symbol, bits in table.items()],
           key=lambda p: (p[1], p[0])   # sort by (code_length, symbol)
       )

  6. Encode input as a bit string:
       bits = ""
       for byte b in input:
         bits += table[b]

  7. Serialise to wire format:
       emit BE uint32: original_length = len(input)
       emit BE uint32: symbol_count    = len(lengths)
       for (symbol, length) in lengths:
         emit uint8: symbol
         emit uint8: length
       pack bits into bytes LSB-first, zero-padding final byte

  return result bytes
```

### Decoding

```
DECODE(data: bytes) → bytes:

  1. Parse header:
       original_length = BE uint32 at bytes 0–3
       symbol_count    = BE uint32 at bytes 4–7

  2. If original_length == 0: return b""

  3. Parse code-lengths table (symbol_count × 2-byte entries at bytes 8+):
       lengths = []
       for i in range(symbol_count):
         symbol = uint8 at bytes 8 + 2*i
         length = uint8 at bytes 8 + 2*i + 1
         lengths.append((symbol, length))
       # lengths is already sorted by (code_length, symbol) per wire format spec

  4. Reconstruct canonical codes from lengths:
       table = canonical_codes_from_lengths(lengths)
       # table: {symbol → bit_string}

  5. Reconstruct tree from canonical code table via DT27:
       # Build a HuffmanTree whose canonical_code_table() matches `table`.
       # Strategy: assign synthetic weights that reproduce the correct code lengths.
       # See "Decoder Tree Reconstruction" below for the standard technique.
       tree = reconstruct_tree_from_lengths(lengths)

  6. Read bit stream (bytes starting at offset 8 + 2*symbol_count):
       bits = unpack_bits_lsb_first(data[8 + 2*symbol_count:])

  7. Decode exactly original_length symbols:
       symbols = tree.decode_all(bits, original_length)

  return bytes(symbols)
```

### Decoder Tree Reconstruction

The wire format transmits only (symbol, code_length) pairs, not the full tree.
The decoder reconstructs the canonical code table using the same algorithm as the
encoder, then either:

**(a) Direct canonical decoding (recommended):** reconstruct the canonical codes from
the lengths, build a lookup table `{bit_string → symbol}`, then walk the bit stream
character by character, accumulating bits until a match is found.

```
canonical_codes_from_lengths(lengths: [(symbol, length)]) -> {symbol: bit_string}:
  # lengths is already sorted by (code_length, symbol)
  code = 0
  prev_len = lengths[0][1]
  result = {}
  for (symbol, length) in lengths:
    if length > prev_len:
      code <<= (length - prev_len)
    result[symbol] = format(code, f"0{length}b")
    code += 1
    prev_len = length
  return result
```

**(b) Tree reconstruction via DT27:** assign synthetic "frequencies" that reproduce
the correct tree shape. The simplest correct approach: assign `freq(s) = 2^(L_max - L_s)`
where `L_s` is the code length for symbol `s` and `L_max` is the maximum code length
in the table. This produces a complete binary tree where the canonical left-to-right
order is respected, allowing `HuffmanTree.build()` to reconstruct the exact tree.

```
reconstruct_tree_from_lengths(lengths: [(symbol, length)]) -> HuffmanTree:
  L_max = max(length for _, length in lengths)
  weights = [(symbol, 2 ** (L_max - length)) for symbol, length in lengths]
  return HuffmanTree.build(weights)
```

Both approaches yield the same decoded output. Approach (a) is faster (no heap
operations); approach (b) reuses DT27 unchanged. Implementations should prefer
approach (a) for performance, falling back to (b) if a tree object is needed for
other reasons (e.g., streaming decode).

## Wire Format (CMP04)

The wire format transmits the original byte count, the code-lengths table (enough
for the decoder to reconstruct canonical codes), and the compressed bit stream.

```
Offset  Size  Field
──────  ────  ────────────────────────────────────────────────────────────────
0       4     original_length — BE uint32. Number of bytes in the original input.
4       4     symbol_count    — BE uint32. Number of distinct symbols (1–256).
8       2×N   code-lengths table — N = symbol_count entries, each 2 bytes:
                [0]  symbol value  (uint8, 0–255)
                [1]  code length   (uint8, 1–16)
              Entries are sorted by (code_length, symbol_value) ascending.
              This ordering is required for canonical code reconstruction.
8+2N    ⌈B/8⌉  bit stream — B total bits, packed LSB-first, zero-padded to byte boundary.
```

### Why Code Lengths Only?

A Huffman tree can be represented in three equivalent ways:
1. The full tree structure (nodes and edges)
2. The code table (symbol → bit string)
3. The code-lengths list (symbol → number of bits)

Option 3 is the most compact. Given a sorted (symbol, length) list, the canonical
codes can be reconstructed exactly — no ambiguity. DEFLATE uses the same trick:
it transmits Huffman code lengths (not the actual tree), and both encoder and decoder
agree on the canonical reconstruction rule.

```
Example: "AAABBC"
  Frequencies: A=3, B=2, C=1
  Tree (from DT27): A gets code length 1, B and C get code length 2
  Lengths sorted by (length, symbol): [(A,1), (B,2), (C,2)]

  Wire format:
    original_length = 6
    symbol_count    = 3
    Entry 0: symbol=65('A'), length=1
    Entry 1: symbol=66('B'), length=2
    Entry 2: symbol=67('C'), length=2

  Canonical reconstruction:
    code=0, prev_len=1
    A (len=1): assign "0";  code=1
    B (len=2): len grew → code=1<<1=2 → "10"; code=3
    C (len=2): same len    → "11"; code=4

  Code table: A→"0", B→"10", C→"11"
```

The decoder can verify its reconstruction by checking that the code table is
prefix-free and that all assigned codes have the correct lengths.

### Bit-Packing Convention (LSB-first)

Bits within each byte are filled from the **least significant bit** upward. This is
consistent with CMP03 (LZW) and the GIF specification.

```
pack_bits(bit_string: str) -> bytes:
  buffer  = 0     # accumulates bits
  bit_pos = 0     # how many valid bits are in buffer
  output  = []

  for b in bit_string:
    buffer |= int(b) << bit_pos
    bit_pos += 1
    if bit_pos == 8:
      output.append(buffer & 0xFF)
      buffer  = 0
      bit_pos = 0

  if bit_pos > 0:           # flush partial byte
    output.append(buffer & 0xFF)

  return bytes(output)

unpack_bits(data: bytes) -> str:
  bits = ""
  for byte_val in data:
    for i in range(8):
      bits += str((byte_val >> i) & 1)
  return bits
```

Reading: for each byte, bits are read from LSB upward, producing a bit string.
The decoder reads exactly `sum(freq(s) × codelen(s))` bits and ignores zero padding.

### Worked Wire Format Example: "AAABBC"

```
Input:      b"AAABBC"   (A=3, B=2, C=1; total 6 bytes)

Frequencies: {65('A'): 3, 66('B'): 2, 67('C'): 1}

DT27 tree construction (tie-break: leaves before internal nodes at equal weight):
  Initial heap: [C(1), B(2), A(3)]
  Step 1: pop C(1), pop B(2) → I(3) [left=C, right=B]; heap=[A(3), I(3)]
  Step 2: pop A(3) (leaf; wins tie), pop I(3) → Root(6) [left=A, right=I]

       Root(6)
       /      \
    A(3)      I(3)
              /   \
           C(1)  B(2)

Standard code table: A→"0", C→"10", B→"11"

Canonical code table (same lengths: A=1, B=2, C=2):
  Sorted by (length, symbol): [(A,1), (B,2), (C,2)]
  Assign: A→"0", B→"10", C→"11"
  (Same as standard codes in this case — the canonical form matches)

Encoding "AAABBC":
  A→"0", A→"0", A→"0", B→"10", B→"10", C→"11"
  Concatenated: "0" + "0" + "0" + "10" + "10" + "11" = "000101011"
  Length: 9 bits → 2 bytes (pad with 7 zero bits)

  Pack LSB-first: "000101011" padded to "000101011 0000000" (16 bits)
  Byte 0: bits 0–7 = "00010101" → 0xA8? Let's compute carefully:
    bit 0 (LSB) = 0  → byte 0, bit 0
    bit 1       = 0  → byte 0, bit 1
    bit 2       = 0  → byte 0, bit 2
    bit 3       = 1  → byte 0, bit 3
    bit 4       = 0  → byte 0, bit 4
    bit 5       = 1  → byte 0, bit 5
    bit 6       = 0  → byte 0, bit 6
    bit 7       = 1  → byte 0, bit 7
    → byte 0 = 0b10101000 = 0xA8
  Byte 1: bits 8–15 = "1" + 7 zeros
    bit 8 (LSB of byte 1) = 1 → byte 1 = 0b00000001 = 0x01

Bit stream bytes: [0xA8, 0x01]

Wire format bytes (hex):
  00 00 00 06   original_length = 6
  00 00 00 03   symbol_count = 3
  41 01         entry 0: symbol='A'(0x41), length=1
  42 02         entry 1: symbol='B'(0x42), length=2
  43 02         entry 2: symbol='C'(0x43), length=2
  A8 01         bit stream (2 bytes)

Total: 4 + 4 + 6 + 2 = 16 bytes to compress 6 bytes.
(Expansion — this is expected for small inputs with few symbols)
```

### Decoding the "AAABBC" Wire Format

```
Parse:
  original_length = 6
  symbol_count    = 3
  lengths = [(65, 1), (66, 2), (67, 2)]

Reconstruct canonical codes:
  code=0, prev_len=1
  symbol=65('A'), length=1: assign "0";  code=1
  symbol=66('B'), length=2: shifted → 1<<1=2 → "10"; code=3
  symbol=67('C'), length=2: "11"; code=4
  table = {65: "0", 66: "10", 67: "11"}

Reverse table (for decoding): {"0": 65, "10": 66, "11": 67}

Bit stream: [0xA8, 0x01]
Unpack LSB-first:
  0xA8 = 0b10101000 → bits: 0,0,0,1,0,1,0,1
  0x01 = 0b00000001 → bits: 1,0,0,0,0,0,0,0
  Full: "0 0 0 1 0 1 0 1 1 0 0 0 0 0 0 0"

Decode 6 symbols (tree walk):
  bits: 0,0,0,1,0,1,0,1,1,...
  Bit 0: "0" → A (leaf). Emit A. Reset. count=1.
  Bit 1: "0" → A (leaf). Emit A. Reset. count=2.
  Bit 2: "0" → A (leaf). Emit A. Reset. count=3.
  Bit 3: "1" → I (internal, go right).
  Bit 4: "0" → C (leaf, left of I). Emit C... wait.

  Wait — canonical table has A→"0", B→"10", C→"11". The tree is:
    Root: left=A, right=I; I: left=B, right=C
    (B gets "10": right from root, left in I; C gets "11": right, right)

  Re-decode:
  Bit 0: "0" → left → A (leaf). Emit 'A'. count=1.
  Bit 1: "0" → left → A. Emit 'A'. count=2.
  Bit 2: "0" → left → A. Emit 'A'. count=3.
  Bit 3: "1" → right → I.
  Bit 4: "0" → left  → B (leaf). Emit 'B'. count=4.
  Bit 5: "1" → right → I.
  Bit 6: "0" → left  → B (leaf). Emit 'B'. count=5.
  Bit 7: "1" → right → I.
  Bit 8: "1" → right → C (leaf). Emit 'C'. count=6. ← stop.

Output: [A, A, A, B, B, C] = b"AAABBC" ✓
```

Note: the canonical code table has B→"10" and C→"11". The tree reconstructed from
lengths `[(A,1),(B,2),(C,2)]` gives B the left child of the right subtree (code "10")
and C the right child (code "11"), consistent with the canonical assignment.

## Encoding Algorithm Pseudocode

```
function compress(data: bytes) -> bytes:
  if len(data) == 0:
    return encode_header(original_length=0, symbol_count=0) + b""

  # Step 1: Count frequencies
  freq = defaultdict(int)
  for b in data:
    freq[b] += 1

  # Step 2: Build Huffman tree via DT27
  tree = HuffmanTree.build(list(freq.items()))

  # Step 3: Canonical code table via DT27
  table = tree.canonical_code_table()
  # table: {symbol (int) -> bit_string}

  # Step 4: Build code-lengths list for header
  # DT27's canonical_code_table() returns codes already assigned canonically.
  # We need (symbol, length) pairs sorted by (length, symbol).
  lengths = sorted(
    [(sym, len(bits)) for sym, bits in table.items()],
    key=lambda p: (p[1], p[0])
  )

  # Step 5: Encode input data
  bit_string = "".join(table[b] for b in data)

  # Step 6: Pack bits LSB-first
  bit_bytes = pack_bits_lsb_first(bit_string)

  # Step 7: Assemble wire format
  header = (
    struct.pack(">I", len(data))         # original_length
    + struct.pack(">I", len(lengths))    # symbol_count
  )
  code_table_bytes = b"".join(
    bytes([sym, length]) for sym, length in lengths
  )
  return header + code_table_bytes + bit_bytes


function decompress(data: bytes) -> bytes:
  original_length = struct.unpack(">I", data[0:4])[0]
  symbol_count    = struct.unpack(">I", data[4:8])[0]

  if original_length == 0:
    return b""

  # Parse code-lengths table
  table_offset = 8
  lengths = []
  for i in range(symbol_count):
    off = table_offset + 2 * i
    symbol = data[off]
    length = data[off + 1]
    lengths.append((symbol, length))
  # lengths is sorted (wire format guarantees this)

  # Reconstruct canonical codes
  code_to_symbol = canonical_codes_from_lengths(lengths)
  # code_to_symbol: {bit_string -> symbol}

  # Unpack bit stream
  bits_offset = table_offset + 2 * symbol_count
  bit_string = unpack_bits_lsb_first(data[bits_offset:])

  # Decode original_length symbols
  output = []
  pos = 0
  accumulated = ""
  while len(output) < original_length:
    accumulated += bit_string[pos]
    pos += 1
    if accumulated in code_to_symbol:
      output.append(code_to_symbol[accumulated])
      accumulated = ""

  return bytes(output)
```

### Bit-Packing Helpers

```
function pack_bits_lsb_first(bits: str) -> bytes:
  output  = []
  buffer  = 0
  bit_pos = 0
  for b in bits:
    buffer  |= int(b) << bit_pos
    bit_pos += 1
    if bit_pos == 8:
      output.append(buffer)
      buffer  = 0
      bit_pos = 0
  if bit_pos > 0:
    output.append(buffer)    # partial byte, zero-padded
  return bytes(output)

function unpack_bits_lsb_first(data: bytes) -> str:
  bits = ""
  for byte_val in data:
    for i in range(8):
      bits += str((byte_val >> i) & 1)
  return bits
```

These helpers are private to each language package — they are not part of DT27
and are not shared across packages.

## Edge Cases

### Empty Input

```
compress(b"") → 8-byte header with original_length=0, symbol_count=0, no bit data.
decompress(compress(b"")) == b""
```

### Single Distinct Symbol

When the entire input is one repeated byte (e.g., `b"AAAAAAA"`), the Huffman tree
has exactly one leaf. DT27's `canonical_code_table()` returns `{A: "0"}` for this
case (by convention — see DT27 §"Edge case: single-symbol alphabet").

This means each `A` is encoded as 1 bit. The bit stream has `len(input)` bits:

```
compress(b"AAAAAAA"):
  Frequencies: {65: 7}
  DT27 tree: single leaf Leaf(65, 7)
  canonical_code_table: {65: "0"}
  Lengths header: [(65, 1)]
  Bit string: "0000000" (7 bits)
  Packed: [0b00000000] = [0x00] (1 byte, all zeros)

Wire format:
  00 00 00 07   original_length = 7
  00 00 00 01   symbol_count = 1
  41 01         entry: symbol='A', length=1
  00            bit stream (7 zero bits + 1 padding bit)

Decode:
  symbol_count=1 → only one symbol in code table → every "0" bit decodes to 'A'.
  Read 7 symbols → "AAAAAAA" ✓
```

The decoder must handle this case by treating every `"0"` bit as the single known
symbol, decoding exactly `original_length` symbols.

### 256 Distinct Symbols

Maximum code-lengths table: 256 entries × 2 bytes = 512 bytes.
Code lengths are bounded at 16 bits (the same limit used by DEFLATE). For 256
uniformly-weighted symbols, the tree is balanced and code lengths are all 8 bits,
giving no compression (1 bit per output byte overhead from the header).

DT27's tree constructor must not produce code lengths exceeding 16. In practice,
for 256 symbols, the worst-case code length is 9 bits (when one symbol has
frequency 1 and all others have frequency 2^k). Implementations should document
their behaviour if a pathological frequency distribution could theoretically produce
depths > 16, though for byte-level coding (256 symbols) this cannot happen in the
standard Huffman construction.

### Very Short Inputs (< 8 bytes)

For very short inputs, the wire format header (8 bytes) plus code-lengths table may
exceed the original input length. This is expected and correct — Huffman compression
is not designed for tiny inputs. The round-trip property still holds.

## Parameters

| Constant | Value | Meaning |
|---|---|---|
| `MAX_CODE_LENGTH` | 16 | Maximum code length in bits (DEFLATE-compatible). |
| `ALPHABET_SIZE` | 256 | Number of possible byte values (0–255). |
| Header size | 8 bytes | `original_length` (4) + `symbol_count` (4). |
| Max table size | 512 bytes | 256 symbols × 2 bytes per entry. |

Huffman compression has no "window size" or "dictionary size" parameters — the
only input is the frequency distribution derived from the data itself.

## Byte-Cost Analysis

```
Header:              8 bytes
Code-lengths table:  2 × symbol_count bytes   (at most 512 bytes for 256 symbols)
Bit stream:          ⌈sum(freq(s) × codelen(s)) / 8⌉ bytes

Total:  8 + 2×N + ⌈T/8⌉   where N = symbol count, T = total bits
```

### Best Case: Highly Skewed Distribution

If one symbol dominates (e.g., 99% of input is `A`), `A` gets a 1-bit code and
rare symbols get codes up to `~log2(1/0.01) ≈ 7` bits. Most bits are 1 bit each:

```
Input: 1,000,000 bytes, 99% 'A' (freq=990000), 1% uniformly spread over 99 others.
Each rare symbol: freq≈10100/99≈102; code length ≈ ceil(-log2(102/1000000)) ≈ 13 bits.
Expected bits ≈ 990000×1 + 10000×13 / 1000000 = 990000 + 130000 = 1,120,000 bits ≈ 140,000 bytes.
Compression ratio: ~14%. Header + table ≈ 206 bytes (negligible).
```

### Worst Case: Uniform Distribution

If all 256 symbols appear with equal frequency, all code lengths are 8 bits. The
compressed bit stream is the same size as the input. The header and code-lengths
table add 8 + 512 = 520 bytes overhead — about a 0.05% expansion for large files,
or significant expansion for small files.

### Incompressible Data

For data that is already entropy-coded (encrypted, or truly random bytes), Huffman
compression expands by at most 1 bit per symbol (a symbol with probability `p`
requires `⌈-log2(p)⌉` bits, versus `8` bits uncompressed; for uniform distribution
the ratio is 1:1 with header overhead added).

### Comparison with LZ Algorithms

```
Input: "AABCBBABC" (9 bytes, from CMP01–CMP03 examples)

LZ78:  4-byte header + 6 tokens × 4 bytes = 32 bytes
LZSS:  8-byte header + 1 flag block (9 bytes) = 17 bytes
LZW:   4-byte header + codes at 9 bits each ≈ 14 bytes

Huffman (frequencies: A=3, B=3, C=2 after counting "AABCBBABC"):
  freq: A=3, B=3, C=2, then also second 'B', second 'C'
  Wait: "AABCBBABC":
    A appears at positions 0,1,6  → freq 3
    B appears at positions 2,4,5,7 → freq 4
    C appears at positions 3,8    → freq 2
  Frequencies: {A:3, B:4, C:2}
  Build tree: B(4) has shortest code, A(3) and C(2) share the other subtree.
    Initial heap: [C(2), A(3), B(4)]
    Merge C(2)+A(3) → I(5); heap=[B(4), I(5)]
    Merge B(4)+I(5) → Root(9)
    Root: left=B(4), right=I(5): left=C(2), right=A(3)? 
    Wait, B pops first (lower weight wins) → Root[left=B, right=I]
    B→"0", C→"10", A→"11"
  canonical_code_table (sorted by (len, sym)): B→"0"(1), A→"11"(2), C→"10"(2)
    canonical assign: B(1)→"0"; A(2)→"10"; C(2)→"11"
    (canon sort: A(len=2,sym=65) before C(len=2,sym=67))
  Encode "AABCBBABC":
    A→"10", A→"10", B→"0", C→"11", B→"0", B→"0", A→"10", B→"0", C→"11"
    = "10 10 0 11 0 0 10 0 11" = "1010011000100 11" → wait, concatenate:
    "10" + "10" + "0" + "11" + "0" + "0" + "10" + "0" + "11"
    = "101001100010011" → 15 bits → 2 bytes

Wire format: 8 (header) + 6 (3 entries × 2 bytes) + 2 (bit stream) = 16 bytes
vs. input = 9 bytes → expansion for this tiny input.
```

For tiny inputs, Huffman always expands. Gains emerge on larger inputs with
genuinely skewed distributions. The larger the input and the more skewed the
distribution, the better Huffman performs.

## Test Vectors

All vectors are deterministic — DT27's canonical tie-breaking ensures the same
output across all language implementations.

### Vector 1 — Empty Input

```
compress(b""):
  Wire: 00 00 00 00  00 00 00 00   (8 bytes)

decompress(compress(b"")) == b""  ✓
```

### Vector 2 — Single Byte

```
compress(b"A"):
  Frequencies: {65: 1}
  Tree: single leaf Leaf(65, 1)
  canonical_code_table: {65: "0"}
  Lengths: [(65, 1)]
  Bit string: "0" → 1 bit → packed: [0x00]

Wire: 00 00 00 01  00 00 00 01  41 01  00
      (original_length=1, symbol_count=1, entry A/1, 1 bit zero-padded)

decompress(compress(b"A")) == b"A"  ✓
```

### Vector 3 — Single Symbol Repeated ("AAAAAAA")

```
compress(b"AAAAAAA"):
  Frequencies: {65: 7}
  canonical_code_table: {65: "0"}
  Bit string: "0000000" (7 bits) → 1 byte [0x00]

Wire: 00 00 00 07  00 00 00 01  41 01  00

decompress(compress(b"AAAAAAA")) == b"AAAAAAA"  ✓
```

### Vector 4 — "AAABBC" (canonical example from DT27)

```
Input: b"AAABBC"   (A=3, B=2, C=1)

Frequencies: {65:3, 66:2, 67:1}

DT27 tree construction:
  heap: [C(1), B(2), A(3)]
  Step 1: pop C(1), B(2) → I1(3); heap: [A(3), I1(3)]
  Step 2: pop A(3) (leaf wins tie), I1(3) → Root(6)
  Root: left=A(3), right=I1(3): left=C(1), right=B(2)

Standard codes: A→"0", C→"10", B→"11"
Code lengths:   A=1, B=2, C=2

Canonical sort by (length, symbol): [(A(65),1), (B(66),2), (C(67),2)]
Canonical codes: A→"0", B→"10", C→"11"

Encode "AAABBC":
  "0"+"0"+"0"+"10"+"10"+"11" = "000101011"  (9 bits)
  Pack LSB-first:
    bits: 0,0,0,1,0,1,0,1,1 + 7 padding zeros
    Byte 0 (bits 0-7): 0b10101000 = 0xA8
    Byte 1 (bits 8-15): 0b00000001 = 0x01

Wire format (hex):
  00 00 00 06   original_length = 6
  00 00 00 03   symbol_count = 3
  41 01         A, length=1
  42 02         B, length=2
  43 02         C, length=2
  A8 01         bit stream

Total: 16 bytes

decompress(compress(b"AAABBC")) == b"AAABBC"  ✓
```

### Vector 5 — Two Distinct Bytes, No Repetition ("AB")

```
compress(b"AB"):
  Frequencies: {65:1, 66:1}
  heap: [A(1), B(1)] — tie: lower symbol wins → A pops first
  Pop A(1), B(1) → Root(2): left=A, right=B
  Standard codes: A→"0", B→"1"
  Canonical sort: [(A(65),1), (B(66),1)]
  Canonical codes: A→"0", B→"1"  (both length 1)

Encode "AB": "0"+"1" = "01" → 2 bits → 1 byte [0b00000010] = 0x02
  Pack: bit0=0,bit1=1 → byte0 = 0b00000010 = 0x02

Wire: 00 00 00 02  00 00 00 02  41 01  42 01  02

decompress(...) == b"AB"  ✓
```

### Vector 6 — Classic textbook example, 6 symbols

```
Input frequency table: A=45, B=13, C=12, D=16, E=9, F=5

From DT27 worked example:
  Codes: A→"0"(1), B→"101"(3), C→"100"(3), D→"111"(3), E→"1101"(4), F→"1100"(4)

Canonical sort by (len, sym): [(A,1),(B,3),(C,3),(D,3),(E,4),(F,4)]
  Wait — B=66, C=67, D=68 in ASCII; sort: B before C before D.
  Canonical codes:
    A(1): "0";  code=1
    B(3): shift 1<<2=4 → "100"; code=5
    C(3): "101"; code=6
    D(3): "110"; code=7
    E(4): shift 7<<1=14 → "1110"; code=15
    F(4): "1111"; code=16

Round-trip: decompress(compress(input)) == input  ✓
```

### Vector 7 — Round-Trip Invariant

```python
test_cases = [
    b"",
    b"A",
    b"AB",
    b"AAABBC",
    b"AAAAAAA",
    b"ABCDE",
    b"ABABABABABABAB",
    bytes(range(256)),          # all byte values once each
    b"Hello, World!" * 100,
]
for data in test_cases:
    assert decompress(compress(data)) == data
```

### Vector 8 — Binary Data with Null Bytes

```python
data = bytes([0, 0, 0, 255, 255, 128, 128, 128])
assert decompress(compress(data)) == data
```

### Vector 9 — Compression on Skewed Data

```python
data = b"A" * 9000 + b"B" * 900 + b"C" * 90 + b"D" * 10
# A:9000, B:900, C:90, D:10 → entropy ≈ 0.53 bits/symbol
# Huffman: A→1bit, B→2bits, C→3bits, D→4bits (approx)
# Expected bits ≈ 9000+1800+270+40 = 11110 bits ≈ 1389 bytes + header/table
# Original: 10000 bytes → significant compression
assert len(compress(data)) < len(data)
assert decompress(compress(data)) == data
```

### Vector 10 — No Compression on Uniform Data

```python
import random
data = bytes([i % 256 for i in range(10000)])  # uniform distribution
# All 256 symbols appear ~39 times → nearly equal code lengths (~8 bits each)
# Compressed size ≈ original + 520 bytes header/table overhead
assert decompress(compress(data)) == data
# (No assertion on size — may expand slightly)
```

## Comparison Table

| Property | LZ77 (CMP00) | LZ78 (CMP01) | LZSS (CMP02) | LZW (CMP03) | Huffman (CMP04) |
|---|---|---|---|---|---|
| Exploits | Repetition | Repetition | Repetition | Repetition | Symbol frequency |
| Token type | (offset,len,char) | (dict_idx,char) | Literal or Match | Code (u16) | Variable-width code |
| Token size | Fixed 4 bytes | Fixed 4 bytes | 1 or 3 bytes | 9–16 bits | 1–16 bits |
| Literal cost | 4 bytes | 3 bytes | 1 byte | 9 bits | 1–8 bits |
| Dictionary | Sliding window | Trie (grows) | Sliding window | Trie (pre-seeded) | None — just a code table |
| External dep | None | DT13 (Trie) | None | None | DT27 (HuffmanTree) |
| Wire format | Fixed records | Fixed records | Flag blocks | Bit-packed codes | Header + lengths + bits |
| Transmit dict? | No | No | No | No | Yes (as code lengths) |
| Adaptive? | Yes (builds as it goes) | Yes | Yes | Yes | No (static, two-pass) |
| Optimal for | Long repetitions | Medium repetitions | Short repetitions | All repetitions | Skewed distributions |
| Worst case | ~12.5% expansion | 4× expansion | ~12.5% expansion | ~12.5% expansion | Header overhead only |
| Real usage | Base for DEFLATE | Base for LZW | Embedded systems | GIF, TIFF | JPEG, DEFLATE, MP3 |
| Successor | LZSS, DEFLATE | LZW | DEFLATE | DEFLATE | DEFLATE (+ LZ) |

### Adaptive vs. Static

One fundamental difference from LZ algorithms: **Huffman coding is static and
two-pass**. The encoder must scan the entire input to count frequencies before it
can build the tree and start emitting compressed data. LZ algorithms are online
(single-pass, adaptive) — the dictionary is built and used simultaneously.

This means Huffman compression:
- Cannot compress a stream of unknown length without buffering everything first
- Must transmit the code table in the header so the decoder can reconstruct it
- Produces the same output for the same input (no randomness)

Adaptive Huffman coding (FGK, Vitter) exists but is more complex and less used.
CMP04 implements static Huffman only.

## Implementation Notes

### Using DT27 from Each Language

**Python:**
```python
from coding_adventures.huffman_tree import HuffmanTree

def compress(data: bytes) -> bytes:
    freq = {}
    for b in data:
        freq[b] = freq.get(b, 0) + 1
    tree = HuffmanTree.build(list(freq.items()))
    table = tree.canonical_code_table()
    ...
```

The `BUILD` file must `uv pip install -e ../huffman-tree` before installing this
package — the same pattern used by any CMP package that depends on a DT package.

**Go:**
```go
import huffmantree "github.com/adhithyan15/coding-adventures/code/packages/go/huffman-tree"

func Compress(data []byte) ([]byte, error) {
    freq := make(map[byte]int)
    for _, b := range data {
        freq[b]++
    }
    weights := make([][2]int, 0, len(freq))
    for sym, f := range freq {
        weights = append(weights, [2]int{int(sym), f})
    }
    tree, err := huffmantree.Build(weights)
    ...
}
```

Run `go mod tidy` in both this package and all transitively dependent packages
after adding the `huffman-tree` module.

**Ruby:**
```ruby
require "coding_adventures/huffman_tree"

def compress(data)
  freq = Hash.new(0)
  data.bytes.each { |b| freq[b] += 1 }
  tree = CodingAdventures::HuffmanTree::Tree.build(freq.to_a)
  table = tree.canonical_code_table
  ...
end
```

The gemspec must declare `spec.add_dependency "coding_adventures_huffman_tree"`.
The `require` line must appear after any `require` lines for the huffman_tree gem.

**TypeScript:**
```typescript
import { HuffmanTree } from "@coding-adventures/huffman-tree";

export function compress(data: Uint8Array): Uint8Array {
  const freq = new Map<number, number>();
  for (const b of data) freq.set(b, (freq.get(b) ?? 0) + 1);
  const tree = HuffmanTree.build([...freq.entries()]);
  const table = tree.canonicalCodeTable();
  ...
}
```

The `package.json` must list `"@coding-adventures/huffman-tree": "file:../huffman-tree"`
and the `BUILD` file must chain-install it before this package.

**Rust:**
```rust
use huffman_tree::{HuffmanTree, HuffmanError};

pub fn compress(data: &[u8]) -> Result<Vec<u8>, HuffmanError> {
    let mut freq: std::collections::HashMap<u8, usize> = HashMap::new();
    for &b in data { *freq.entry(b).or_insert(0) += 1; }
    let weights: Vec<(u8, usize)> = freq.into_iter().collect();
    let tree = HuffmanTree::build(&weights)?;
    let table = tree.canonical_code_table();
    ...
}
```

Add `huffman-tree` to the workspace `Cargo.toml` members and to this package's
`[dependencies]`.

**Elixir:**
```elixir
alias CodingAdventures.HuffmanTree

def compress(data) when is_binary(data) do
  freq =
    data
    |> :binary.bin_to_list()
    |> Enum.frequencies()
  tree = HuffmanTree.build(Map.to_list(freq))
  table = HuffmanTree.canonical_code_table(tree)
  ...
end
```

Remember `import Bitwise` for bit manipulation in the bit-packing helpers.
The `mix.exs` must list the huffman_tree dep in the `deps` function.

### Bit I/O

The `BitWriter` and `BitReader` helpers in CMP04 are simpler than in CMP03 (LZW)
because CMP04 codes have variable widths determined by the code table, not by a
growing code_size counter. The packing rule is identical: LSB-first within each byte.

```
BitWriter state:
  buffer:  u64    (accumulates bits)
  bit_pos: u8     (count of valid bits)
  bytes:   Vec<u8>

write_bit_string(code: &str):
  for c in code.chars():
    buffer |= ((c == '1') as u64) << bit_pos
    bit_pos += 1
    if bit_pos == 8:
      bytes.push((buffer & 0xFF) as u8)
      buffer >>= 8
      bit_pos -= 8

flush():
  if bit_pos > 0:
    bytes.push((buffer & 0xFF) as u8)
```

The CMP04 bit helpers are private to each package implementation — they are not
exported and not shared with DT27.

### Security: Decoder Input Validation

The decoder must guard against malformed input:

1. **Truncated header:** If `len(data) < 8`, return an error.
2. **Truncated code-lengths table:** If `len(data) < 8 + 2 * symbol_count`, error.
3. **Symbol count = 0 with non-zero original_length:** Return an error — contradictory.
4. **Code length = 0:** A code length of 0 is invalid (every symbol in the table
   must have at least a 1-bit code). Return an error.
5. **Code length > 16:** Return an error — no code should exceed 16 bits.
6. **Duplicate symbol in code-lengths table:** Return an error.
7. **Bit stream exhausted:** If the decoder runs out of bits before decoding
   `original_length` symbols, return an error (do not access past the end of the
   input slice).
8. **Invalid prefix in bit stream:** If the accumulated bit string is longer than
   the maximum code length and still has no match, the stream is corrupt. Return
   an error.

### Language-Specific Notes

**Python:** Use `heapq` from the standard library inside DT27 for the priority queue.
CMP04's `BUILD` must `uv pip install -e ../huffman-tree`. The module path is
`coding_adventures.huffman` (not `coding_adventures_huffman`).

**Go:** Run `go mod tidy` in `code/packages/go/huffman` AND in every package that
transitively imports it after adding the huffman-tree dependency. The import path
for DT27 is `github.com/adhithyan15/coding-adventures/code/packages/go/huffman-tree`.

**Ruby:** `require "coding_adventures/huffman_tree"` before any `require` for this
package. `standardrb` must pass. Use `Data.define` for immutable value objects.

**TypeScript:** Chain-install transitive `file:` deps in the BUILD file:
`npm install ../huffman-tree && npm install .`. Use `Uint8Array` for raw bytes.
`npx vitest run` for tests.

**Rust:** Add `"huffman"` to the workspace `[members]` in `code/packages/rust/Cargo.toml`.
The crate name is `huffman` (short); the DT27 dependency is `huffman-tree`.
Run `cargo build --workspace` to catch any missing exports.

**Elixir:** Reserved words cannot be variable names — avoid `after`, `rescue`, `do`,
`end`, `when`. Use `import Bitwise` for `|||`, `&&&`, `<<<`. The module is
`CodingAdventures.Huffman`.

**Swift:** Create `.gitignore` with `.build/` before running any Swift commands.
The `BUILD_windows` target must use:
```
where swift >nul 2>nul && swift test || echo Swift not available on this runner — skipping
```

**Lua:** The module path is `coding_adventures.huffman`. DT27 is required via
`require("coding_adventures.huffman_tree")`.

**Perl:** Module path `CodingAdventures::Huffman`. Dependency on
`CodingAdventures::HuffmanTree` (DT27). Use `pack("N", $n)` for BE uint32.

### Two-Pass vs. Streaming

Because Huffman coding requires the full frequency table before a single bit can be
emitted, the `compress` function must buffer the entire input. There is no streaming
variant in CMP04. The `decompress` function is also bulk — it reads the header,
reconstructs the code table, then decodes the bit stream.

Contrast with LZ algorithms (CMP00–CMP03), which are online: the encoder processes
input byte-by-byte and emits output as it goes.

## Interface Contract

```
# compress: encode a byte string to the CMP04 wire format
compress(data: bytes) -> bytes
  Invariant: decompress(compress(data)) == data   for all byte strings data

# decompress: decode a CMP04 wire-format byte string
decompress(data: bytes) -> bytes
  Raises an error on malformed input (truncated header, invalid code lengths, etc.)

# Internal (not exported)
pack_bits_lsb_first(bit_string: str) -> bytes
unpack_bits_lsb_first(data: bytes) -> str
canonical_codes_from_lengths(lengths: [(symbol, length)]) -> {bit_string: symbol}
```

The `huffman-tree` (DT27) package provides:
```
HuffmanTree.build(weights: [(symbol, freq)]) -> HuffmanTree
tree.canonical_code_table() -> {symbol: bit_string}
tree.decode_all(bits: str, count: int) -> [symbol]
```

These DT27 functions are the only external calls CMP04 makes. The bit-packing
helpers and the wire format serialisation are entirely internal to CMP04.

## Package Matrix

| Language | Package name | Import |
|---|---|---|
| Python | `coding-adventures-huffman` | `coding_adventures.huffman` |
| Go | `coding-adventures-huffman` | `.../code/packages/go/huffman` |
| Ruby | `coding_adventures_huffman` | `coding_adventures/huffman` |
| TypeScript | `@coding-adventures/huffman` | `@coding-adventures/huffman` |
| Rust | `huffman` | `huffman` |
| Elixir | `:coding_adventures_huffman` | `CodingAdventures.Huffman` |
| Lua | `coding-adventures-huffman` | `coding_adventures.huffman` |
| Perl | `CodingAdventures-Huffman` | `CodingAdventures::Huffman` |
| Swift | `Huffman` | `import Huffman` |

Each package declares `huffman-tree` (DT27) as a runtime dependency and does NOT
embed its own Huffman tree construction logic.

## Relationship to CMP05 (DEFLATE)

CMP04 is a prerequisite for CMP05. DEFLATE applies LZ77 backreference finding
(from CMP00/CMP02) to the input, producing a stream of literals and (offset, length)
pairs. It then applies **two Huffman codes** to that token stream:

- **Literal/length code:** encodes the 256 possible literal byte values plus 29
  length codes (for match lengths 3–258). Up to 288 symbols total.
- **Distance code:** encodes 30 distance codes (for offsets 1–32768).

Both Huffman trees are transmitted in the DEFLATE block header as code-lengths tables,
using the same canonical Huffman format as CMP04. CMP05 also introduces a third-level
Huffman code (the "code length code") to compress the code-lengths tables themselves.

Understanding CMP04 is essential before implementing CMP05.
