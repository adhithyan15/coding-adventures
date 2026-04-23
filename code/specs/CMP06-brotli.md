# CMP06 — Brotli

## Overview

Brotli (2013, RFC 7932) is a lossless compression algorithm developed at Google that
achieves significantly better compression ratios than DEFLATE (CMP05), particularly on
web content (HTML, CSS, JavaScript). It became the dominant algorithm for HTTP
`Content-Encoding: br` compression.

Brotli builds on DEFLATE's foundation but adds three major innovations:

1. **Context-dependent literal trees** — instead of one Huffman tree for all literals,
   Brotli assigns each literal to one of up to 64 *context buckets* based on the two
   preceding bytes. Each context bucket gets its own Huffman tree, letting the coder
   exploit the fact that the letter following a space is very different from the letter
   following another letter.

2. **Insert-and-copy commands** — instead of DEFLATE's flat stream of "literal" and
   "back-reference" tokens, Brotli uses *commands* that bundle an insert run (raw
   literals) with a copy operation (back-reference). The lengths of both halves are
   encoded together in a single Huffman symbol, reducing overhead.

3. **Larger sliding window** — up to 16 MiB instead of DEFLATE's 32 KiB, allowing
   matches across much longer distances in large documents.

(The real RFC 7932 also includes a 122,784-entry static dictionary of common English
word forms and morphological transforms. The CodingAdventures CMP06 implementation
omits the static dictionary to keep implementations tractable and cross-language
consistent.)

```
Series:
  CMP00 (LZ77,     1977) — Sliding-window backreferences.
  CMP01 (LZ78,     1978) — Explicit dictionary (trie).
  CMP02 (LZSS,     1982) — LZ77 + flag bits; no wasted literals.
  CMP03 (LZW,      1984) — LZ78 + pre-initialised alphabet; GIF.
  CMP04 (Huffman,  1952) — Entropy coding; prerequisite for DEFLATE.
  CMP05 (DEFLATE,  1996) — LZSS + dual Huffman; ZIP/gzip/PNG/zlib.
  CMP06 (Brotli,   2013) — Context modeling + insert-copy + large window. ← YOU ARE HERE
  CMP07 (Zstd,     2016) — ANS/FSE + LZ4 matching; modern universal codec.
```

## Historical Context

Jyrki Alakuijärvi and Zoltán Szabadka designed Brotli at Google in 2013, initially as a
web-font compression format (`.woff2`). In 2015 it was extended for general-purpose HTTP
response compression. RFC 7932 was published in July 2016.

Brotli is the standard compression algorithm for:

- **HTTP `Content-Encoding: br`** (2015) — supported by all major browsers and servers.
  Replaces gzip for new deployments; typically 15–25% smaller than gzip on HTML.
- **WOFF2 font format** (2014) — the dominant web font container format.
- **Chromium source distribution** — used internally for resource packing.

Why is Brotli better than DEFLATE? Three reasons at once:

```
DEFLATE weakness 1: One-size-fits-all literal tree.
  "the " (5 bytes) — 't' after space, 'h' after 't', etc.
  A single tree encodes ALL literals identically, even though
  'h' after 't' is far more probable than 'x' after 't'.

Brotli fix: Context modeling.
  Assign 't' followed by 'h' to context bucket #12 (e.g.).
  Context #12's tree gives 'h' a very short code.
  Different contexts → different trees → much better entropy coding.

DEFLATE weakness 2: Separate literal and match tokens waste header bits.
  Each DEFLATE token needs a Huffman code to say "literal" or "match".
  If you have 10 literals then a match, that's 10 + 1 = 11 Huffman symbols.

Brotli fix: Insert-and-copy commands.
  One command says "insert 10 literals, then copy 5 bytes from offset 42".
  The insert+copy lengths share a SINGLE Huffman symbol, saving overhead.

DEFLATE weakness 3: 32 KiB window.
  DEFLATE cannot match text more than 32,768 bytes earlier.
  A web page often repeats boilerplate (nav, footer, class names) beyond that.

Brotli fix: Window up to 16 MiB.
  Matches can reach back up to 16,777,216 bytes.
```

## Key Concepts

### Context Modeling

Context modeling is the most impactful innovation in Brotli. The idea:

```
English text example — predicting the next byte after "th":
  After "th" we almost always see: 'e', 'a', 'i', 'r', 's', 'o'
  After "qu" we almost always see: 'i', 'a', 'e'
  After "0x" we almost always see: '0'-'9', 'a'-'f'

If we use ONE Huffman tree for all bytes in all positions, the tree must
accommodate every byte value with probabilities averaged over all contexts.
That averaging wastes bits.

If we use a SEPARATE tree for each context ("what were the last two bytes?"),
each tree can be highly tuned to its specific context's distribution.
```

**Context buckets:**  Full RFC 7932 uses 64 buckets, derived from a 6-bit context
function over the last two bytes. CMP06 (CodingAdventures) uses **4 literal context
buckets**, derived from a 2-bit context function over the last byte:

```
Context function (last byte p1):
  bucket 0 — p1 is a space or punctuation (0x00–0x2F, 0x3A–0x40, 0x5B–0x60, 0x7B–0xFF)
  bucket 1 — p1 is a digit ('0'–'9')
  bucket 2 — p1 is an uppercase letter ('A'–'Z')
  bucket 3 — p1 is a lowercase letter ('a'–'z')

At the start of the stream (no previous byte), bucket 0 is used.
```

Four buckets captures the dominant structure (space/punct → alpha, digit → digit, case
transitions) without the implementation complexity of the full 64-bucket model.

Each literal context bucket has its own canonical Huffman tree, built from the
frequencies of literals that appear in that context across the input.

### Insert-and-Copy Commands

Every Brotli command has three parts:

```
Command {
  insert_length:  uint — number of raw literal bytes that follow.
  copy_length:    uint — number of bytes to copy from the history buffer.
  copy_distance:  uint — how far back to look (1 = immediately preceding byte).
}
```

The command stream is:

```
[Command 0]  [insert_length literals]  [Command 1]  [insert_length literals] ...
```

At the end, a final command with `copy_length = 0` acts as the end-of-data marker.
(CMP06 uses `insert_length = 0, copy_length = 0` as the sentinel.)

**Insert-copy length encoding:**  Instead of separate symbols for insert length and
copy length, they are encoded together as a single **insert-copy code (ICC)** Huffman
symbol. The ICC symbol implicitly encodes the *ranges* of both lengths; extra bits
then select the exact values within those ranges.

```
ICC table (CMP06 subset — 64 codes):

Code  Insert base  Insert extra  Copy base  Copy extra
   0            0             0          4           0
   1            0             0          5           0
   2            0             0          6           0
   3            0             0          8           1
   4            0             0         10           1
   5            0             0         14           2
   6            0             0         18           2
   7            0             0         26           3
   8            0             0         34           3
   9            0             0         50           4
  10            0             0         66           4
  11            0             0         98           5
  12            0             0        130           5
  13            0             0        194           6
  14            0             0        258           7
  15            0             0        514           8
  16            1             0          4           0
  17            1             0          5           0
  18            1             0          6           0
  19            1             0          8           1
  20            1             0         10           1
  21            1             0         14           2
  22            1             0         18           2
  23            1             0         26           3
  24            2             0          4           0
  25            2             0          5           0
  26            2             0          6           0
  27            2             0          8           1
  28            2             0         10           1
  29            2             0         14           2
  30            2             0         18           2
  31            2             0         26           3
  32            3             1          4           0
  33            3             1          5           0
  34            3             1          6           0
  35            3             1          8           1
  36            3             1         10           1
  37            3             1         14           2
  38            3             1         18           2
  39            3             1         26           3
  40            5             2          4           0
  41            5             2          5           0
  42            5             2          6           0
  43            5             2          8           1
  44            5             2         10           1
  45            5             2         14           2
  46            5             2         18           2
  47            5             2         26           3
  48            9             3          4           0
  49            9             3          5           0
  50            9             3          6           0
  51            9             3          8           1
  52            9             3         10           1
  53            9             3         14           2
  54            9             3         18           2
  55            9             3         26           3
  56           17             4          4           0
  57           17             4          5           0
  58           17             4          6           0
  59           17             4          8           1
  60           17             4         10           1
  61           17             4         14           2
  62           17             4         18           2
  63            0             0          0           0   ← end-of-data sentinel
```

Minimum copy length is 4 (codes 0–15, 16–63 with insert=0), except for the sentinel.
(Note: insert_length = 0 means the command has no literal bytes; copy_length = 0 means
end-of-data.)

**Distance encoding:**  Distances use the same table as CMP05 (codes 0–23, base + extra
bits), covering distances 1–4096. CMP06 extends the window to 65535:

```
Dist code  Base  Extra bits
        0     1           0
        1     2           0
        2     3           0
        3     4           0
        4     5           1
        5     7           1
        6     9           2
        7    13           2
        8    17           3
        9    25           3
       10    33           4
       11    49           4
       12    65           5
       13    97           5
       14   129           6
       15   193           6
       16   257           7
       17   385           7
       18   513           8
       19   769           8
       20  1025           9
       21  1537           9
       22  2049          10
       23  3073          10
       24  4097          11
       25  6145          11
       26  8193          12
       27 12289          12
       28 16385          13
       29 24577          13
       30 32769          14
       31 49153          14
```

(Codes 24–31 extend the window to 65535; these are absent in CMP05.)

### Sliding Window

CMP06 uses a **65535-byte sliding window** (vs CMP05's 4096). The LZSS-style matching
algorithm (from CMP02) still applies: scan backwards from the current position, find
the longest match of length ≥ 4, subject to the constraint that offset ≤ 65535.

Minimum match length is **4 bytes** (vs 3 in LZSS/DEFLATE). Any run shorter than 4
bytes is always emitted as literals.

## Algorithm

### Compression

```
function compress(data: bytes) → bytes:

  # ── Pass 1: LZ matching → raw commands ─────────────────────────────────────
  commands = []
  insert_buf = []
  pos = 0

  while pos < len(data):
    window_start = max(0, pos - 65535)
    (offset, length) = find_longest_match(data, pos, window_start, max_length=258)

    if length >= 4:
      commands.append(Command(
        insert_length = len(insert_buf),
        copy_length   = length,
        copy_distance = offset,
        literals      = insert_buf.copy()
      ))
      insert_buf = []
      pos += length
    else:
      insert_buf.append(data[pos])
      pos += 1

  # Flush remaining literals with a copy_length=0 final command.
  commands.append(Command(
    insert_length = len(insert_buf),
    copy_length   = 0,
    copy_distance = 0,
    literals      = insert_buf.copy()
  ))

  # Append end-of-data sentinel (ICC code 63: insert=0, copy=0).
  commands.append(Command(insert_length=0, copy_length=0, copy_distance=0, literals=[]))

  # ── Pass 2a: Tally frequencies ───────────────────────────────────────────────
  # Literal frequencies per context bucket (4 buckets).
  lit_freq[4] = [Hash.new(0)] × 4
  icc_freq    = Hash.new(0)
  dist_freq   = Hash.new(0)

  history = []   # reconstructed output bytes (for context tracking)
  for cmd in commands (excluding sentinel):
    for byte in cmd.literals:
      ctx = literal_context(history)
      lit_freq[ctx][byte] += 1
      history.append(byte)

    if cmd.copy_length > 0:
      icc = icc_code(cmd.insert_length, cmd.copy_length)
      icc_freq[icc] += 1
      dc = dist_code(cmd.copy_distance)
      dist_freq[dc] += 1
      # Simulate copy for context tracking.
      start = len(history) - cmd.copy_distance
      for i in 0..cmd.copy_length-1:
        history.append(history[start + i])
    elif cmd.insert_length > 0:
      # Final flush command: no ICC needed, just literals (already counted above).
      pass

  icc_freq[63] += 1   # end-of-data sentinel

  # ── Pass 2b: Build Huffman trees ─────────────────────────────────────────────
  lit_trees[4] = [HuffmanTree.build(lit_freq[ctx].to_a) for ctx in 0..3]
  icc_tree     = HuffmanTree.build(icc_freq.to_a)
  dist_tree    = HuffmanTree.build(dist_freq.to_a)   # nil if no copies

  # ── Pass 2c: Encode ──────────────────────────────────────────────────────────
  bits = ""
  history = []
  for cmd in commands:
    # Encode literals using per-context trees.
    for byte in cmd.literals:
      ctx  = literal_context(history)
      bits += lit_trees[ctx].code(byte)
      history.append(byte)

    if cmd.copy_length > 0:
      # Encode ICC symbol.
      icc              = icc_code(cmd.insert_length, cmd.copy_length)
      ins_extra_count  = ICC_TABLE[icc].insert_extra
      copy_extra_count = ICC_TABLE[icc].copy_extra
      ins_extra_val    = cmd.insert_length - ICC_TABLE[icc].insert_base
      copy_extra_val   = cmd.copy_length  - ICC_TABLE[icc].copy_base

      bits += icc_tree.code(icc)
      for i in 0..ins_extra_count-1:  bits += ((ins_extra_val  >> i) & 1).to_s
      for i in 0..copy_extra_count-1: bits += ((copy_extra_val >> i) & 1).to_s

      # Encode distance.
      dc               = dist_code(cmd.copy_distance)
      dist_extra_count = DIST_TABLE[dc].extra
      dist_extra_val   = cmd.copy_distance - DIST_TABLE[dc].base

      bits += dist_tree.code(dc)
      for i in 0..dist_extra_count-1: bits += ((dist_extra_val >> i) & 1).to_s

      # Simulate copy for context tracking.
      start = len(history) - cmd.copy_distance
      for i in 0..cmd.copy_length-1:
        history.append(history[start + i])

    elif cmd.insert_length > 0 and cmd.copy_length == 0:
      # Final flush: no ICC symbol (copy_length=0, no copy to encode).
      pass

  # Encode end-of-data sentinel: ICC code 63.
  bits += icc_tree.code(63)

  bit_stream = pack_bits_lsb_first(bits)
  return assemble_wire_format(original_length, lit_trees, icc_tree, dist_tree, bit_stream)
```

**Finding ICC code:**  Given `insert_length` and `copy_length`, find the smallest ICC
code whose insert range contains `insert_length` AND whose copy range contains
`copy_length`. When no single ICC code covers both, use the ICC code for the copy
length with `insert_length = 0` and emit the excess insert bytes as literals before
the command. (In practice, the encoder may split oversized commands freely.)

**`literal_context(history)`:**

```
function literal_context(history: bytes) → 0..3:
  if history is empty: return 0
  p1 = history[-1]
  if p1 >= 'a' and p1 <= 'z': return 3
  if p1 >= 'A' and p1 <= 'Z': return 2
  if p1 >= '0' and p1 <= '9': return 1
  return 0
```

### Decompression

```
function decompress(data: bytes) → bytes:

  # Parse wire format → trees, bit stream, original_length.
  (original_length, lit_trees, icc_tree, dist_tree, bit_stream) = parse_wire(data)

  bits    = unpack_bits_lsb_first(bit_stream)
  bit_pos = 0
  output  = []

  loop:
    # Decode ICC symbol.
    icc = next_huffman_symbol(icc_tree, bits, &bit_pos)
    if icc == 63: break   # end-of-data

    ins_extra_count  = ICC_TABLE[icc].insert_extra
    copy_extra_count = ICC_TABLE[icc].copy_extra
    insert_length    = ICC_TABLE[icc].insert_base + read_bits_lsb(bits, ins_extra_count,  &bit_pos)
    copy_length      = ICC_TABLE[icc].copy_base   + read_bits_lsb(bits, copy_extra_count, &bit_pos)

    # Decode and emit insert_length literals.
    for _ in 0..insert_length-1:
      ctx  = literal_context(output)
      byte = next_huffman_symbol(lit_trees[ctx], bits, &bit_pos)
      output.append(byte)

    # Decode and perform copy (if copy_length > 0).
    if copy_length > 0:
      dc           = next_huffman_symbol(dist_tree, bits, &bit_pos)
      dist_extra   = read_bits_lsb(bits, DIST_TABLE[dc].extra, &bit_pos)
      copy_distance = DIST_TABLE[dc].base + dist_extra
      start = len(output) - copy_distance
      for i in 0..copy_length-1:
        output.append(output[start + i])

  return output.pack("C*")
```

## Wire Format (CMP06)

```
Header (10 bytes):
  Bytes 0–3:   original_length     — big-endian uint32.
  Bytes 4–4:   icc_entry_count     — uint8. Entries in ICC code-length table (1–64).
  Bytes 5–5:   dist_entry_count    — uint8. Entries in dist code-length table (0–32).
  Bytes 6–6:   ctx0_entry_count    — uint8. Entries in literal tree 0.
  Bytes 7–7:   ctx1_entry_count    — uint8. Entries in literal tree 1.
  Bytes 8–8:   ctx2_entry_count    — uint8. Entries in literal tree 2.
  Bytes 9–9:   ctx3_entry_count    — uint8. Entries in literal tree 3.

ICC code-length table (icc_entry_count × 2 bytes each):
  [1 byte] symbol      — uint8. ICC code (0–63).
  [1 byte] code_length — uint8. Huffman code length (1–16).
  Entries sorted by (code_length ASC, symbol ASC).

Distance code-length table (dist_entry_count × 2 bytes each):
  [1 byte] symbol      — uint8. Distance code (0–31).
  [1 byte] code_length — uint8. Huffman code length (1–16).
  Entries sorted by (code_length ASC, symbol ASC).
  Omitted entirely (dist_entry_count=0) when no copy commands exist.

Literal tree 0 code-length table (ctx0_entry_count × 3 bytes each):
  [2 bytes] symbol      — big-endian uint16. Literal byte value (0–255).
  [1 byte]  code_length — uint8. Huffman code length (1–16).
  Entries sorted by (code_length ASC, symbol ASC).
  Omitted (ctx0_entry_count=0) if no literals appeared in context 0.

Literal tree 1 code-length table (ctx1_entry_count × 3 bytes each):
  Same structure as tree 0. For context 1 (last byte was a digit).

Literal tree 2 code-length table (ctx2_entry_count × 3 bytes each):
  Same structure. For context 2 (last byte was uppercase).

Literal tree 3 code-length table (ctx3_entry_count × 3 bytes each):
  Same structure. For context 3 (last byte was lowercase).

Bit stream (remaining bytes):
  LSB-first packed bits. See encoding algorithm above.
  Zero-padded to byte boundary.
```

### Single-symbol Huffman Trees

If exactly one ICC, distance, or literal symbol exists for a given tree, the canonical
code for that single symbol is the bit string `"0"` (code length = 1). During
decompression, reading one bit (which must be `0`) returns that symbol.

### Zero-entry Literal Trees

A literal tree with zero entries means no literals appeared in that context during
compression. During decompression, the tree is never consulted for that context
(because the encoder would never produce a literal in a context it never saw).

### Empty Input

Empty input (original_length = 0) is encoded as:
- Header: `[0x00000000][0x01][0x00][0x00][0x00][0x00][0x00]`
- ICC table: 1 entry — symbol=63 (sentinel), code_length=1.
- Bit stream: `\x00` (one zero byte: the single bit of ICC code 63 = "0", padded).

## Key Differences from CMP05

| Feature               | CMP05 (DEFLATE)          | CMP06 (Brotli)                         |
|-----------------------|--------------------------|----------------------------------------|
| LZ window             | 4096 bytes               | 65535 bytes                            |
| Minimum match length  | 3 bytes                  | 4 bytes                                |
| Token structure       | Flat literal + match     | Insert-and-copy commands               |
| Literal entropy coder | One Huffman tree         | Four context-dependent Huffman trees   |
| ICC encoding          | Separate LL + dist codes | Bundled insert+copy length (ICC)       |
| Distance codes        | 24 codes (up to 4096)    | 32 codes (up to 65535)                 |
| End-of-data           | LL symbol 256            | ICC code 63 (insert=0, copy=0)        |
| Static dictionary     | None                     | None (omitted from CodingAdventures)   |

## Dependencies

- `coding-adventures-huffman-tree` (DT27) — canonical Huffman tree builder, same as CMP05.
- No LZSS dependency — CMP06 performs its own LZ matching internally (the command
  structure differs from CMP02's flat token stream; reusing CMP02 directly would
  require adapting its output into insert-and-copy commands, which is more work than
  re-implementing the O(n²) scan).

## Expected Compression Performance

On typical English text (e.g., a 100 KB HTML page):

```
Original:         100,000 bytes
CMP02 (LZSS):      ~55,000 bytes   (45% reduction)
CMP05 (DEFLATE):   ~38,000 bytes   (62% reduction)
CMP06 (Brotli):    ~31,000 bytes   (69% reduction) — educational impl, no static dict
Real brotli:       ~28,000 bytes   (72% reduction) — includes static dictionary
```

The gap between CMP05 and CMP06 on random binary data is smaller; context modeling
primarily helps on structured text where adjacent byte probabilities are highly
non-uniform.

## Test Cases

Every implementation MUST pass all of the following:

1. **Round-trip: empty input** — `decompress(compress("")) == ""`
2. **Round-trip: single byte** — `decompress(compress("\x42")) == "\x42"`
3. **Round-trip: all literals, no matches** — 256 distinct bytes, one of each.
   Output must be larger than input (incompressible data), but round-trip must be exact.
4. **Round-trip: all copies, no leading literals** — `"AAAA...A"` (1024 × 'A').
   The command should be: insert "AAAA" (4 bytes, since window was empty), then
   one or more copy commands. Final decompressed output = 1024 × 'A'.
5. **Round-trip: English prose** — ASCII text ≥ 1024 bytes with varied vocabulary.
   Compressed size must be < 80% of input size.
6. **Round-trip: binary blob** — 512 random bytes. Round-trip exact. (No compression
   ratio requirement — random data is incompressible.)
7. **Cross-command literal context** — a string where context bucket changes mid-stream:
   `"abc123ABC"`. Literals 'b','c' appear in ctx3 (after lowercase); '2','3' in ctx1
   (after digit); 'B','C' in ctx2 (after uppercase). Verify round-trip and that all
   four context trees are populated correctly.
8. **Long-distance match** — input where a 10-byte sequence is repeated with offset
   > 4096 (to exercise the extended distance codes 24–31).
9. **Cross-language compatibility** — compress in language A, decompress in language B,
   for at least 3 pairs. Use the English prose test case.
10. **Wire format parsing** — manually construct a minimal valid CMP06 payload and verify
    it decompresses correctly without using the compressor.

## Packages

Implement as `coding-adventures-brotli` in each language, following the same package
structure as `coding-adventures-deflate` (CMP05):

| Language   | Package name                         | Module/namespace                      |
|------------|--------------------------------------|---------------------------------------|
| Python     | `coding-adventures-brotli`           | `coding_adventures.brotli`            |
| Go         | `coding-adventures-brotli`           | `codingadventures/brotli`             |
| Ruby       | `coding_adventures_brotli`           | `CodingAdventures::Brotli`            |
| TypeScript | `@coding-adventures/brotli`          | `CodingAdventures.Brotli`             |
| Rust       | `coding-adventures-brotli`           | `coding_adventures_brotli`            |
| Elixir     | `coding_adventures_brotli`           | `CodingAdventures.Brotli`             |
| Lua        | `coding-adventures-brotli`           | `CodingAdventures.Brotli`             |
| Perl       | `CodingAdventures::Brotli`           | `CodingAdventures::Brotli`            |
| Swift      | `CodingAdventuresBrotli`             | `CodingAdventures.Brotli`             |

Public API (same as CMP05):

```
compress(data: bytes) → bytes
decompress(data: bytes) → bytes
```
