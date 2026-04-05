# QR Code

## Overview

This spec defines a **QR Code encoder** for the coding-adventures monorepo.

QR Code (Quick Response code) was invented by Masahiro Hara at Denso Wave in
1994 to track automotive parts on assembly lines. It was designed to be read
10× faster than a 1D barcode, and to survive physical damage to up to 30% of
the symbol's area. In 2023, the ISO/IEC 18004 standard governs it. It is now
the most widely deployed 2D barcode format on earth — on every product label,
restaurant menu, bus stop, and business card.

Understanding how to build a QR Code encoder from scratch teaches:

- how binary data is packed into field elements
- how Reed-Solomon erasure coding works in practice
- how a structured 2D layout is designed around invariant structural elements
- how masking defeats degenerate patterns that confuse scanners
- why error correction level and version selection matter for reliability

The encoder in this spec produces a **valid, scannable QR Code** for any input
string that fits within QR version 40. It does not implement decoding —
that is a separate, more complex problem involving image preprocessing and
perspective correction.

---

## Symbol Structure

A QR Code symbol is a square grid of **modules** — dark (1) or light (0) square
cells. The grid size is `(4V + 17) × (4V + 17)` where `V` is the **version**
from 1 to 40.

```
Version 1:  21×21  modules
Version 2:  25×25  modules
Version 10: 57×57  modules
Version 40: 177×177 modules
```

The modules are partitioned into several functional regions:

```
┌─────────────────────────────────────────────┐
│ quiet zone (4 modules on all sides)         │
│  ┌─────────────────────────────────────┐    │
│  │ ┌───────┐  timing  ┌───────┐       │    │
│  │ │finder │──────────│finder │       │    │
│  │ │pattern│          │pattern│       │    │
│  │ └───────┘          └───────┘       │    │
│  │    │   format info                 │    │
│  │ timing                    data     │    │
│  │    │                      and      │    │
│  │    ▼                      ECC      │    │
│  │ ┌───────┐   alignment   modules    │    │
│  │ │finder │   patterns               │    │
│  │ │pattern│   (v2+)                  │    │
│  │ └───────┘                          │    │
│  └─────────────────────────────────────┘    │
└─────────────────────────────────────────────┘
```

### Functional regions in detail

**Finder patterns** — Three identical 7×7 square-ring patterns placed at the
top-left, top-right, and bottom-left corners. Each finder pattern is:

```
1 1 1 1 1 1 1
1 0 0 0 0 0 1
1 0 1 1 1 0 1
1 0 1 1 1 0 1
1 0 1 1 1 0 1
1 0 0 0 0 0 1
1 1 1 1 1 1 1
```

A scanner locates these three distinctive 1:1:3:1:1 dark-light ratio sequences
to find and orient the symbol. The three-corner-only placement means a scanner
always knows which corner is which: bottom-right is the data corner.

**Separators** — A 1-module-wide row and column of light modules surrounding
each finder pattern. These isolate the finder patterns from the data area.

**Timing patterns** — Two strips of alternating dark/light modules running
between the finder patterns: one horizontal (row 6), one vertical (column 6).
They start and end dark. The timing patterns let scanners determine module size
and grid alignment, especially for larger symbols where grid distortion is more
likely.

**Alignment patterns** — Small 5×5 finder-like squares (with a dark center)
placed at predetermined positions in the data area. Version 1 has none;
version 2 has one; larger versions have more. Their positions are tabulated in
the ISO standard (reproduced below). They help scanners correct for perspective
distortion and rotation.

**Format information** — 15 bits encoded twice (in two L-shaped strips adjacent
to the top-left finder pattern and one strip each adjacent to the other two).
Contains the error correction level (2 bits) and mask pattern index (3 bits),
protected by a 10-bit BCH code.

**Version information** — 18 bits (version number in 6 bits + 12-bit BCH code),
present only in versions 7–40. Placed in two 6×3 blocks adjacent to the
top-right and bottom-left finder patterns.

**Dark module** — A single always-dark module at position (row=4V+9, col=8)
from the top-left. Always set to 1. Not part of data, not masked.

**Data and ECC modules** — All remaining modules. These carry the encoded
message codewords followed by Reed-Solomon error correction codewords.

---

## Versions and Capacity

The choice of version and error correction level determines how many characters
the symbol can hold. Higher versions = larger symbol = more data. Higher ECC
levels = more redundancy = less data but more damage tolerance.

Error correction levels:

| Level | Approx. recovery capacity |
|-------|--------------------------|
| L (Low) | ~7% of codewords |
| M (Medium) | ~15% of codewords |
| Q (Quartile) | ~25% of codewords |
| H (High) | ~30% of codewords |

Selected capacity figures (byte mode, ISO-8859-1):

| Version | Size | L | M | Q | H |
|---------|------|---|---|---|---|
| 1 | 21×21 | 17 | 14 | 11 | 7 |
| 2 | 25×25 | 32 | 26 | 20 | 14 |
| 3 | 29×29 | 53 | 42 | 32 | 24 |
| 4 | 33×33 | 78 | 62 | 46 | 34 |
| 5 | 37×37 | 106 | 84 | 60 | 44 |
| 7 | 45×45 | 154 | 122 | 86 | 58 |
| 10 | 57×57 | 271 | 213 | 151 | 106 |
| 15 | 77×77 | 543 | 435 | 311 | 217 |
| 20 | 97×97 | 858 | 666 | 482 | 342 |
| 25 | 117×117 | 1249 | 955 | 691 | 465 |
| 30 | 137×137 | 1817 | 1373 | 985 | 677 |
| 40 | 177×177 | 2953 | 2331 | 1663 | 1273 |

The encoder must select the **minimum version** that fits the input at the
chosen ECC level.

---

## Data Encoding Modes

QR Code supports four encoding modes. Each mode optimizes for a different
character set.

### Mode indicator bits

| Mode | Indicator (4 bits) |
|------|--------------------|
| Numeric | 0001 |
| Alphanumeric | 0010 |
| Byte | 0100 |
| Kanji | 1000 |
| ECI | 0111 |
| Terminator | 0000 |

### Numeric mode

Encodes only the digits 0–9. Groups of three digits are packed into 10 bits
(range 0–999). Remaining pairs into 7 bits (0–99). Remaining single digits
into 4 bits (0–9).

```
"01234567" → groups: "012", "345", "67"
              bits:    10,   10,    7 = 27 bits
```

Capacity in numeric mode is approximately 3× higher than byte mode.

### Alphanumeric mode

Encodes digits, uppercase A–Z, space, and the symbols: `$ % * + - . / :`.
That is 45 characters total, indexed 0–44. Pairs of characters are packed into
11 bits: `first_idx * 45 + second_idx`. Single trailing character into 6 bits.

Character-to-index mapping:
```
0–9 → 0–9
A–Z → 10–35
SP  → 36
$   → 37
%   → 38
*   → 39
+   → 40
-   → 41
.   → 42
/   → 43
:   → 44
```

### Byte mode

Encodes raw bytes (typically ISO-8859-1, though any byte values 0–255 are
valid). Each byte uses 8 bits. This is the universal fallback — any UTF-8
string can be encoded in byte mode by treating the UTF-8 bytes as raw bytes.
Scanners that support UTF-8 will decode the bytes as UTF-8; others will see
Latin-1.

v0.1.0 implements byte mode as the primary mode and adds numeric and
alphanumeric as optimizations. Kanji mode and ECI are future work.

### Character count indicator

Following the mode indicator, the data segment starts with a **character
count** field. The width of this field depends on the mode and the symbol
version:

| Mode | Versions 1–9 | Versions 10–26 | Versions 27–40 |
|------|-------------|----------------|----------------|
| Numeric | 10 bits | 12 bits | 14 bits |
| Alphanumeric | 9 bits | 11 bits | 13 bits |
| Byte | 8 bits | 16 bits | 16 bits |
| Kanji | 8 bits | 10 bits | 12 bits |

### Bit stream assembly

The encoder assembles a **bit stream**:

```
[4-bit mode indicator]
[character count indicator]
[encoded data bits for this segment]
[4-bit terminator 0000 if there is room]
[padding to fill to a byte boundary: 0 bits]
[padding bytes: alternate 0xEC and 0x11 to fill remaining data codewords]
```

The final bit stream must be exactly `data_codewords × 8` bits long, where
`data_codewords` is the number of data codewords for this version and ECC
level.

---

## Reed-Solomon Error Correction

QR uses Reed-Solomon over GF(256) with the **irreducible polynomial**
`x^8 + x^4 + x^3 + x^2 + 1` (decimal 285, hex 0x11D). This is the same
GF(256) field used by MA01/MA02.

The **generator polynomial** convention in QR is:

```
g(x) = (x + α^0)(x + α^1)(x + α^2)···(x + α^{n-1})
```

where `n` is the number of ECC codewords for this block. This is the
**b=0 convention** (the first root is α^0 = 1, not α^1 = 2).

This differs from MA02's b=1 convention by one shift. Concretely:

| EC codewords | QR generator polynomial (hex) |
|-------------|-------------------------------|
| 7 | 01 7c bc 7e 1c 05 0f |
| 10 | 01 f6 75 a8 d0 c3 e3 36 e1 3c 45 |
| 13 | 01 8a 4c 38 12 e5 83 2a 65 03 55 05 02 |
| 17 | 01 97 4d 89 18 77 98 bf db 93 c5 e4 1a 8a 9a 1b 71 |

These are correct QR RS generator polynomials. The qr-code package should
precompute these rather than calling MA02.

### Block structure

For most versions, the message codewords are split across multiple blocks,
each with its own RS computation. This improves damage resilience: a burst
error that destroys a contiguous region will only wipe out one or two blocks,
leaving the others intact.

The block structure is defined by two parameters per version/ECC combination:

```
(num_blocks_group_1, data_codewords_per_block_group_1,
 num_blocks_group_2, data_codewords_per_block_group_2)
```

ECC codewords per block is the same for both groups.

Example: Version 5, ECC level Q:

```
Total data codewords:   64
Total ECC codewords:    72
Block structure:        2 blocks of 15 data codewords
                        2 blocks of 16 data codewords
ECC per block:          18
```

The data stream is split first: blocks get their data codewords in sequence.
Each block's data bytes are fed to the RS encoder to produce its ECC bytes.

### Interleaving

After computing ECC, the codewords are **interleaved** before placement:

1. Take the first data codeword from block 1, then block 2, then block 3, ...
2. Take the second data codeword from block 1, then block 2, then block 3, ...
3. ... continue until all data codewords are interleaved ...
4. Take the first ECC codeword from block 1, then block 2, then block 3, ...
5. ... continue until all ECC codewords are interleaved ...
6. Append remainder bits (zero-padding, if any, to complete the grid)

The interleaved stream is what gets placed into the module grid.

---

## Module Placement

After interleaving, the final message stream is placed into the data modules
of the grid using a **two-column zigzag** scan, right to left, starting from
the bottom-right corner.

### Reserved modules

Before placing data, mark all structural modules as reserved so the placement
algorithm skips them:

1. Three finder patterns (7×7 each) at (0,0), (0, size-7), (size-7, 0)
2. Separators (1-module border around each finder)
3. Horizontal timing strip (row 6, col 8 to size-9)
4. Vertical timing strip (col 6, row 8 to size-9)
5. Alignment patterns (tabulated positions, version-specific)
6. Format information modules (15 bits × 2 copies)
7. Version information modules (18 bits × 2 copies, version 7+)
8. Dark module at (4V+9, 8)

### Alignment pattern positions

Alignment pattern centers are defined by the ISO standard. Selected values:

| Version | Centers |
|---------|---------|
| 1 | (none) |
| 2 | 6, 18 |
| 3 | 6, 22 |
| 4 | 6, 26 |
| 5 | 6, 30 |
| 6 | 6, 34 |
| 7 | 6, 22, 38 |
| 10 | 6, 28, 50 |
| 14 | 6, 26, 46, 66 |
| 20 | 6, 34, 62, 90 |
| 27 | 6, 34, 62, 90, 118 |
| 35 | 6, 30, 54, 78, 102, 126, 150 |
| 40 | 6, 34, 62, 90, 118, 146, 174 |

The full table (all 40 versions) is in Annex E of ISO/IEC 18004:2015.
Alignment pattern centers form a grid of all combinations of the tabulated
positions, **excluding** any that would overlap with a finder pattern.

### Zigzag data placement

```
current_col = size - 1      -- start from rightmost column
direction = -1              -- upward (-1) or downward (+1)
bit_index = 0               -- position in the interleaved message stream

loop:
  -- Process 2-column strip (columns current_col and current_col-1)
  for row in the current direction (top-to-bottom or bottom-to-top):
    for sub_col in [current_col, current_col - 1]:
      if sub_col == 6: skip (timing column)
      if module is reserved: skip
      place message_stream[bit_index] at (row, sub_col)
      bit_index += 1

  -- After finishing this 2-column strip, flip direction and move left
  direction = -direction
  current_col -= 2
  if current_col == 6: current_col -= 1   -- skip timing column
  if current_col < 0: stop
```

This zigzag pattern ensures the data streams diagonally up and then down
through the available modules, maximizing locality between adjacent codeword
bits.

---

## Masking

A mask is applied to all **data and ECC modules** (not structural modules) to
prevent degenerate patterns — large solid areas, finder-pattern look-alikes, or
long runs of the same color — that could confuse a scanner.

### The 8 mask patterns

Each mask pattern defines a condition on (row, col). If the condition is true
for a module, that module's bit is flipped:

| Pattern | Condition |
|---------|-----------|
| 0 | `(row + col) mod 2 == 0` |
| 1 | `row mod 2 == 0` |
| 2 | `col mod 3 == 0` |
| 3 | `(row + col) mod 3 == 0` |
| 4 | `(row / 2 + col / 3) mod 2 == 0` |
| 5 | `(row * col) mod 2 + (row * col) mod 3 == 0` |
| 6 | `((row * col) mod 2 + (row * col) mod 3) mod 2 == 0` |
| 7 | `((row + col) mod 2 + (row * col) mod 3) mod 2 == 0` |

### Penalty scoring

After applying each candidate mask, score the result with four penalty rules.
The mask with the **lowest total penalty** is selected.

**Rule 1 — Adjacent modules in row/column:**

For each row and column, scan for runs of ≥5 consecutive modules of the same
color. Score = `run_length - 2` for each qualifying run.

```
run of 5 → +3
run of 6 → +4
run of 7 → +5
...
```

**Rule 2 — 2×2 blocks of same color:**

For each 2×2 square with all four modules the same color: `score += 3`.

**Rule 3 — Finder-pattern-like patterns:**

Check for the pattern `1 0 1 1 1 0 1 0 0 0 0` or its reverse in rows and
columns (this resembles a finder pattern). Each occurrence adds 40 to the
score.

**Rule 4 — Proportion of dark modules:**

```
dark_ratio = dark_modules / total_modules * 100
prev5 = nearest lower multiple of 5 from dark_ratio
next5 = prev5 + 5
penalty = min(|prev5 - 50| / 5, |next5 - 50| / 5) * 10
```

The penalty is zero when dark_ratio is exactly 50%.

---

## Format Information

The format information string encodes the error correction level and mask
pattern. It is placed in two copies in the symbol:

- Copy 1: along the top and left edges of the top-left finder pattern
- Copy 2: one strip along the right edge of the top-left finder pattern (in
  row 8) and one strip along the bottom edge of the bottom-left finder pattern

The 15-bit format string is constructed as:

```
1. Start with a 5-bit data string:
   [ECC_level_indicator (2 bits)] [mask_pattern (3 bits)]

   ECC level indicators:
   L → 01,  M → 00,  Q → 11,  H → 10

2. Multiply by x^10 (left-shift 10 places).

3. Divide (polynomial long division over GF(2)) by the generator:
   G(x) = x^10 + x^8 + x^5 + x^4 + x^2 + x + 1  (decimal 10111010100)

4. The 10-bit remainder is appended to the 5-bit data.

5. XOR the entire 15 bits with the mask value: 101010000010010 (0x5412)

The XOR mask ensures the format info is never all-zero.
```

The resulting 15-bit string is placed into the format modules bit by bit.
The module positions for copy 1 and copy 2 are defined in Annex C of
ISO/IEC 18004.

---

## Version Information

For versions 7–40 only. The 18-bit version information string is constructed as:

```
1. Start with the 6-bit version number (e.g., version 7 → 000111).
2. Multiply by x^12 (left-shift 12 places).
3. Divide by the generator:
   G(x) = x^12 + x^11 + x^10 + x^9 + x^8 + x^5 + x^2 + 1  (0x1F25)
4. Append the 12-bit remainder.
```

The 18 bits are arranged in a 6×3 pattern. Two copies: one near the
top-right finder pattern, one near the bottom-left finder pattern.

---

## Complete Encoding Algorithm

The full encoding pipeline for a given input string and ECC level:

```
1. CHOOSE VERSION
   Try versions 1, 2, 3, ... in order.
   For each version, check if the input fits in the available data
   codewords at the chosen ECC level.
   Use the smallest version that fits.

2. ENCODE DATA
   a. Select encoding mode (numeric/alphanumeric/byte — choose the most
      compact mode that covers the entire input; byte is always valid).
   b. Build the bit stream:
      - mode indicator (4 bits)
      - character count (mode- and version-dependent width)
      - encoded data bits
      - terminator (4 zero bits, or fewer if at capacity)
      - zero-pad to byte boundary
      - pad with 0xEC 0x11 alternating to fill remaining data codewords

3. SPLIT INTO BLOCKS
   Split the data codeword sequence into blocks per the version/ECC table.

4. COMPUTE ECC
   For each block, compute the RS ECC codewords using the QR generator
   polynomial for this block's ECC count.

5. INTERLEAVE
   Interleave data codewords, then ECC codewords, across blocks.
   Append remainder bits if required.

6. INITIALIZE GRID
   Create a (4V+17) × (4V+17) grid with all modules unset.
   Place finder patterns, separators, timing strips, alignment patterns,
   dark module. Mark all structural modules as reserved.
   Place temporary format information (all zeros) to reserve those modules.

7. PLACE DATA
   Run the zigzag placement algorithm to fill non-reserved modules with
   the interleaved message stream.

8. EVALUATE ALL 8 MASKS
   For each mask pattern 0–7:
   a. Apply the mask (flip data/ECC modules matching the condition).
   b. Write the correct format information for this mask.
   c. Compute the penalty score.
   Record (penalty, mask_index) for each.

9. SELECT BEST MASK
   Choose the mask pattern with the lowest penalty score.

10. FINALIZE
    Apply the chosen mask to the grid.
    Write the final format information for the chosen mask.
    Write version information if version ≥ 7.

11. RENDER
    Convert the final ModuleGrid to a DrawScene via barcode-2d's
    `to_draw_scene`. Render to SVG, PNG, or native window.
```

---

## Implementation Notes

### Tables to embed

The following tables must be embedded in the implementation (not computed
at runtime):

1. **Capacity table**: for each version (1–40) × ECC level (L/M/Q/H):
   - total data codewords
   - total ECC codewords
   - block structure (group 1 blocks, data per block; group 2 blocks,
     data per block; ECC per block)

2. **Alignment pattern position table**: center coordinates per version

3. **Format information module positions**: two fixed lists of (row, col)
   pairs

4. **Version information module positions**: two fixed 6×3 grids

5. **QR Reed-Solomon generator polynomials**: one per possible ECC
   codeword count (7, 10, 13, 15, 16, 17, 18, 20, 22, 24, 26, 28, 30)

These are all constants from ISO/IEC 18004:2015. Embedding them as lookup
tables is far simpler and more reliable than computing them.

### RS encoder for QR

The QR RS encoder only needs polynomial remainder (encoding), not decoding.
The encoder computes:

```
Given: data polynomial D(x) of degree k-1 (k data codewords)
       generator polynomial G(x) of degree n (n ECC codewords)

Compute: remainder R(x) = D(x) * x^n mod G(x)
The ECC codewords are the coefficients of R(x).
```

This is a single polynomial division operation, simpler than full MA02
syndrome decoding. Implement as:

```
ecc = [0] * n_ecc
for byte in data_codewords:
    feedback = byte XOR ecc[0]
    ecc = ecc[1:] + [0]
    for i in range(n_ecc):
        ecc[i] = ecc[i] XOR gf_multiply(generator[n_ecc - i], feedback)
```

### GF(256) polynomial

QR uses the same primitive polynomial as MA01: `x^8 + x^4 + x^3 + x^2 + 1`
(0x11D). The qr-code package should depend on MA01 (gf256) for all field
arithmetic. The only difference from MA02 is the generator polynomial
construction.

### Mode selection heuristic

For v0.1.0, choose the most compact mode that covers the entire input:

1. If all characters are digits (0–9): use numeric mode.
2. Else if all characters are in the alphanumeric set (45 chars): use alphanumeric.
3. Otherwise: use byte mode.

Mixed-mode segments (e.g., a long numeric string in the middle of an ASCII
string) can improve capacity by 20–30% for typical URLs but add considerable
implementation complexity. This is a v0.2.0 enhancement.

### Byte mode and UTF-8

When using byte mode, encode the string as UTF-8 bytes. Most modern QR
scanners default to UTF-8. To signal UTF-8 explicitly, add an ECI segment
`\000026` before the data (ECI assignment 26 = UTF-8). For v0.1.0, omit
the ECI header and rely on scanner defaults.

---

## Visualization Annotations

The encoder should optionally produce an **annotated module grid** for the
visualizer. Each module records its role:

| Color (suggested) | Role |
|-------------------|------|
| Deep blue | finder pattern |
| Blue-grey | separator |
| Grey | timing |
| Teal | alignment |
| Purple | format information |
| Indigo | version information |
| Black/white | data module |
| Green/light green | ECC module |
| Orange | remainder bits |
| Always dark | dark module |

A visualizer built on this annotation can show exactly what each part of a
QR code does, turning any QR code into an interactive learning diagram.

---

## Error Types

```
QRCodeError::InputTooLong   -- input does not fit in any version/ECC combination
QRCodeError::InvalidInput   -- input contains characters not supported by the mode
```

---

## Public API

```
encode(input: string, ecc: EccLevel) → ModuleGrid
  -- Encodes input to a QR code module grid.
  -- Raises InputTooLong if input exceeds version 40 capacity.

render(input: string, ecc: EccLevel, config?: RenderConfig) → DrawScene
  -- Encode and translate to draw instructions.

render_svg(input: string, ecc: EccLevel, config?: RenderConfig) → string
  -- Encode, render, and return an SVG string.

explain(input: string, ecc: EccLevel) → AnnotatedModuleGrid
  -- Encode with full per-module role annotations.

EccLevel = L | M | Q | H
```

---

## Package Matrix

| Language | Directory | Depends on |
|----------|-----------|------------|
| Rust | `code/packages/rust/qr-code/` | barcode-2d, gf256 |
| TypeScript | `code/packages/typescript/qr-code/` | barcode-2d, gf256 |
| Python | `code/packages/python/qr-code/` | barcode-2d, gf256 |
| Go | `code/packages/go/qr-code/` | barcode-2d, gf256 |
| Ruby | `code/packages/ruby/qr_code/` | barcode-2d, gf256 |
| Elixir | `code/packages/elixir/qr_code/` | barcode_2d, gf256 |
| Lua | `code/packages/lua/qr-code/` | barcode-2d, gf256 |
| Perl | `code/packages/perl/qr-code/` | barcode-2d, gf256 |
| Swift | `code/packages/swift/qr-code/` | Barcode2D, GF256 |

---

## Test Strategy

### Unit tests

1. **RS encoder**: for each QR generator (7, 10, 13, 17, 26 ECC codewords),
   encode a known data sequence and verify the ECC bytes match the expected
   values from the ISO standard's worked example.

2. **Format information**: verify the 15-bit format string for known
   (ECC level, mask pattern) pairs.

3. **Mode selection**: byte mode, alphanumeric mode, numeric mode; verify
   correct mode indicator and encoding.

4. **Bit stream assembly**: verify correct padding with 0xEC/0x11 bytes.

### Integration tests

1. **Encode a known string**: encode `"https://example.com"` at ECC level M.
   Verify:
   - Version selected: V3 (29×29), 32 data codewords fit
   - Module grid is 29×29
   - Finder patterns are present at the three corners
   - Symbol passes a QR scanner (use a Python script calling `zxing` or
     `zbar` to decode the rendered PNG and compare to input)

2. **All ECC levels**: same input, all four levels → correct version selection.

3. **Numeric mode**: input `"01234567"` → verify numeric mode is selected
   and correct bit stream.

4. **Edge cases**: empty string, single character, exactly version 40 capacity.

### Cross-language verification

All 9 languages should produce **identical `ModuleGrid` outputs** for the same
input. The test corpus should be run against all implementations, comparing
module grids bit-by-bit.

Suggested test corpus:

```
"A"                    (minimal)
"HELLO WORLD"          (alphanumeric)
"https://example.com"  (URL, byte mode)
"01234567890"          (numeric)
"The quick brown fox jumps over the lazy dog"   (full byte mode)
```

---

## Dependency Stack

```
draw-instructions-metal  draw-instructions-png
        └──────┬────────────────┘
               │
       draw-instructions
               │
           barcode-2d           MA01 gf256
               │                    │
           qr-code ───────────────── ┘
```

`qr-code` depends on `barcode-2d` for the ModuleGrid type and
`to_draw_scene`, and on `gf256` for GF(256) multiplication in the RS
encoder. It does **not** depend on MA02 (the RS convention differs).

---

## Future Extensions

- **Decoder** (FNT-style — a separate, much more complex spec)
- **Mixed-mode encoding** — segment the input into numeric/alphanumeric/byte
  regions for maximum capacity
- **ECI mode** — Explicit UTF-8 signal for scanners that default to Latin-1
- **Structured Append** — split a large message across multiple QR symbols
- **Micro QR** — single finder pattern, versions M1–M4 (see micro-qr.md)
- **rMQR** — rectangular variant for very narrow spaces
- **Visualizer integration** — interactive drill-down showing exactly which
  codeword, block, and ECC polynomial produced each module
