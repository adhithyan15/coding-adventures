# Micro QR Code

## Overview

This spec defines a **Micro QR Code encoder** for the coding-adventures monorepo.

Micro QR Code is a compact variant of QR Code, standardized in ISO/IEC 18004:2015
Annex E. It was designed for applications where space is extremely limited —
think surface-mount electronic components, tiny labels on circuit boards, and
miniature product markings — where even the smallest regular QR Code (21×21 at
version 1) is too large.

The defining characteristic of Micro QR is the **single finder pattern**: where
regular QR Code uses three identical corner squares to establish orientation,
Micro QR uses only one, in the top-left. This saves a dramatic amount of space at
the cost of some scanner robustness: Micro QR scanners must determine orientation
by other means (the single-corner placement is unambiguous). The tradeoff is
deliberate — Micro QR targets controlled scanning environments (factory floors,
industrial equipment) rather than consumer apps.

Micro QR has four symbol versions: M1 through M4. Each is a square, and the sizes
are much smaller than regular QR:

```
M1: 11×11 modules
M2: 13×13 modules
M3: 15×15 modules
M4: 17×17 modules
```

The formula is `size = 2 × version_number + 9`. So M1 = 2(1)+9 = 11, M4 = 2(4)+9 = 17.

Understanding how to build a Micro QR encoder from scratch teaches:

- how a constrained 2D format packs data into a tiny fixed-size grid
- how error correction levels trade symbol capacity for damage tolerance
- how format information encoding differs when you only have a single finder
- why the quiet zone can be halved (from 4 to 2 modules) without breaking
  scanner detection
- how the same Reed-Solomon math used in QR Code scales down to tiny data payloads

The encoder in this spec produces a **valid, scannable Micro QR Code** for any
input string that fits within M4. It does not implement decoding — that is a
separate, more complex problem.

---

## Symbol Structure

A Micro QR Code symbol is a square grid of **modules** — dark (1) or light (0)
square cells. Unlike regular QR where size is `4V+17`, Micro QR uses:

```
size = 2 × version_number + 9
```

| Symbol | Version Number | Size |
|--------|---------------|------|
| M1     | 1             | 11×11 |
| M2     | 2             | 13×13 |
| M3     | 3             | 15×15 |
| M4     | 4             | 17×17 |

Here is a schematic view of an M4 (17×17) symbol, showing all functional regions:

```
col:  0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16
     ┌──────────────────────────────────────────────────────
row0 │ T  T  T  T  T  T  T  T  .  .  .  .  .  .  .  .  .
row1 │ T  ■  ■  ■  ■  ■  T  S  F  .  .  .  .  .  .  .  .
row2 │ T  ■  □  □  □  ■  T  S  F  .  .  .  .  .  .  .  .
row3 │ T  ■  □  ■  □  ■  T  S  F  .  .  .  .  .  .  .  .
row4 │ T  ■  □  □  □  ■  T  S  F  .  .  .  .  .  .  .  .
row5 │ T  ■  ■  ■  ■  ■  T  S  F  .  .  .  .  .  .  .  .
row6 │ T  T  T  T  T  T  T  S  F  .  .  .  .  .  .  .  .
row7 │ T  S  S  S  S  S  S  S  F  .  .  .  .  .  .  .  .
row8 │ T  F  F  F  F  F  F  F  .  .  .  .  .  .  .  .  .
row9 │ .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .
...  │ .  .  .  .  .  .  .  .  .  (data and ECC modules)  .
row16│ .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .
     └──────────────────────────────────────────────────────

Legend:
  T = timing pattern module (alternating dark/light)
  ■ = dark module of finder pattern
  □ = light module of finder pattern
  S = separator module (always light)
  F = format information module
  . = data or ECC module
```

Note how the top row (row 0) and left column (col 0) are entirely timing pattern
modules. This is the opposite of regular QR Code, where timing patterns are on
row 6 and column 6.

### Finder Pattern

The same 7×7 finder pattern as regular QR Code, but placed in the **top-left
corner only** (rows 0–6, cols 0–6):

```
1 1 1 1 1 1 1
1 0 0 0 0 0 1
1 0 1 1 1 0 1
1 0 1 1 1 0 1
1 0 1 1 1 0 1
1 0 0 0 0 0 1
1 1 1 1 1 1 1
```

This is precisely the same 1:1:3:1:1 dark-light-dark pattern that scanners use
for detection. Because there is only one finder pattern, a scanner immediately
knows the top-left corner — so orientation is unambiguous with one pattern (the
data area is always to the bottom-right of the finder).

### Separators

The finder pattern is bordered on its **bottom and right sides** by a 1-module-wide
strip of light modules (value 0). The top and left sides of the finder are the
symbol edge — there is nothing to separate there.

```
Separator modules:
  Row 7, cols 0–7  (bottom of finder)
  Col 7, rows 0–7  (right of finder)
```

This forms an L-shaped separator (not a full surrounding border as in regular QR,
which has separators on all four sides of each finder — Micro QR's top-left
and top-right edges of the finder are literally the edge of the symbol).

### Timing Patterns

Unlike regular QR Code where timing patterns run along row 6 and column 6, Micro
QR places its timing patterns along the **outer edges** of the finder pattern —
along row 0 and column 0 — and extends them to the opposite edge of the symbol:

```
Horizontal timing: row 0, cols 0 through (size-1)
Vertical timing:   col 0, rows 0 through (size-1)
```

The timing pattern alternates dark/light starting with dark at (0,0). For an
11×11 symbol (M1):

```
Row 0: 1 1 1 1 1 1 1 1 1 1 1   ← NO! They're not all dark.
```

Wait — let's be precise. The timing pattern rule is: module at position `k` is
dark if `k` is even, light if `k` is odd. But in Micro QR, the top-left 7×7
region is the finder pattern. The finder pattern's top row (row 0, cols 0–6) and
left column (col 0, rows 0–6) are already determined by the finder pattern
definition, and those modules *are* the timing pattern for those positions. The
timing pattern extends outward:

```
Row 0 pattern:
  Cols 0–6: finder pattern row 0 = [1,1,1,1,1,1,1]  (all dark)

  Hmm — but timing should alternate. Let's look at the standard more carefully.
```

Actually, the ISO standard places the timing patterns consistently as alternating
dark/light, and the finder pattern's outer ring happens to satisfy the timing
pattern at those positions (outer ring of finder is all dark). The timing
pattern modules that extend beyond the finder (from col 8 onward on row 0, and
from row 8 onward on col 0) alternate dark/light in the usual way.

The correct specification is:

```
Timing pattern value at (row=0, col=c): dark if c is even, light if c is odd
Timing pattern value at (row=r, col=0): dark if r is even, light if r is odd
```

These definitions are consistent with the finder pattern: the finder's outer
ring entries on row 0 (cols 0–6) are all dark, and col indices 0,2,4,6 are even
(dark) while 1,3,5 are odd — but the finder is 1,1,1,1,1,1,1, not alternating.

The resolution is that in Micro QR the **finder pattern overrides** the timing
pattern for the overlapping modules. The timing pattern is placed first (or the
overlap is treated as reserved by the finder). In practice: set the finder
pattern, then place timing starting at col 8 (row 0) and row 8 (col 0).
The timing value at col 8 (even-numbered if you count from 0) is dark.

Concretely for M4 (17×17):

```
Row 0 (timing, starting after separator at col 8):
  col 8: dark (even), col 9: light, col 10: dark, ..., col 16: dark

Col 0 (timing, starting after separator at row 8):
  row 8: dark (even), row 9: light, row 10: dark, ..., row 16: dark
```

The timing row and column modules at positions 0–6 are part of the finder
pattern. Position 7 is part of the separator (always light). Position 8 onward
is the extended timing pattern.

### No Alignment Patterns

Micro QR has **no alignment patterns**. The symbol is too small to suffer from
the perspective distortion that alignment patterns correct in larger QR codes.
Removing them frees up data capacity.

### Quiet Zone

Micro QR requires only a **2-module quiet zone** on all sides (compared to 4
modules for regular QR). This is safe because the single-finder-corner detection
is more spatially distinctive and the smaller symbol size means distortion is
less severe.

```
Quiet zone layout for M4 (17 + 2 + 2 = 21 total rendering width):

  2 light modules │ 17-module symbol │ 2 light modules
                  ↕
              2 light modules
```

### Format Information

15 bits reserved in a specific location. See the [Format Information](#format-information)
section for detailed placement.

### Dark Module

Micro QR does **not** have a separate always-dark "dark module" independent of
the finder pattern (unlike regular QR Code's dark module at position `(4V+9, 8)`).
All forced-dark modules in Micro QR are part of the finder pattern or timing
strips.

---

## Versions and Capacity

Micro QR has four symbol versions. The version determines maximum capacity and
which encoding modes and error correction levels are available.

### ECC Levels by Version

| Symbol | Available ECC Levels |
|--------|---------------------|
| M1     | Detection only (no correction) |
| M2     | L, M |
| M3     | L, M |
| M4     | L, M, Q |

No symbol supports level H (High). This is a deliberate tradeoff: the symbols
are so small that adding 30% redundancy would leave almost no room for data.
M1 does not even support error *correction* — only error *detection* (one error
detection codeword, essentially a simple checksum).

Error correction level meanings (same as regular QR):

| Level | Approximate recovery capacity |
|-------|-------------------------------|
| L (Low) | ~7% of codewords |
| M (Medium) | ~15% of codewords |
| Q (Quartile) | ~25% of codewords |

### Capacity Table

The maximum number of characters per symbol/ECC level combination:

| Symbol | ECC | Numeric | Alphanumeric | Byte (ISO-8859-1) | Kanji |
|--------|-----|---------|--------------|-------------------|-------|
| M1     | Det | 5       | —            | —                 | —     |
| M2     | L   | 10      | 6            | 4                 | —     |
| M2     | M   | 8       | 5            | 3                 | —     |
| M3     | L   | 23      | 14           | 9                 | —     |
| M3     | M   | 18      | 11           | 7                 | —     |
| M4     | L   | 35      | 21           | 15                | 9     |
| M4     | M   | 30      | 18           | 13                | 8     |
| M4     | Q   | 21      | 13           | 9                 | 6     |

These numbers come from the ISO standard. Dashes indicate mode is not available
for that symbol version.

### Codeword Counts

The exact codeword structure (total codewords, data codewords, ECC codewords) per
version and ECC level:

| Symbol | ECC | Total CWs | Data CWs | ECC CWs | Blocks |
|--------|-----|-----------|----------|---------|--------|
| M1     | Det | 5         | 3        | 2       | 1      |
| M2     | L   | 10        | 5        | 5       | 1      |
| M2     | M   | 10        | 4        | 6       | 1      |
| M3     | L   | 17        | 11       | 6       | 1      |
| M3     | M   | 17        | 9        | 8       | 1      |
| M4     | L   | 24        | 16       | 8       | 1      |
| M4     | M   | 24        | 14       | 10      | 1      |
| M4     | Q   | 24        | 10       | 14      | 1      |

All Micro QR symbols use a **single block** — there is no interleaving needed.
The data stream is a single byte sequence; the ECC codewords follow directly.

Note on M1: it uses 3 data codewords but only 3.5 bytes of data (the last
codeword is 4 bits, not 8). See the special M1 encoding rules in the
[Data Encoding Modes](#data-encoding-modes) section.

### Module Count and Remainder

| Symbol | Grid modules | Function modules | Data+ECC modules |
|--------|-------------|-----------------|-----------------|
| M1     | 121         | 81              | 36 (+ 4 remainder = 40) |
| M2     | 169         | 97              | 80              |
| M3     | 225         | 113             | 136 (+ 0 remainder) |
| M4     | 289         | 129             | 160 (+ 0 remainder) |

The function modules are: finder pattern (49) + separators (row 7 cols 0–7 = 8,
plus col 7 rows 0–7 = 8, minus corner overlap = 15 total) + timing extensions +
format information (15).

In practice, just reserve all known functional positions and fill the rest with
data and ECC bits.

---

## Data Encoding Modes

Micro QR supports the same four encoding modes as regular QR, but availability
depends on the symbol version. Each mode encodes a specific character set in a
compact binary format.

### Mode Availability

| Mode | M1 | M2 | M3 | M4 |
|------|----|----|----|----|
| Numeric | YES | YES | YES | YES |
| Alphanumeric | NO | YES | YES | YES |
| Byte (ISO-8859-1) | NO | NO | YES | YES |
| Kanji | NO | NO | NO | YES |

### Mode Indicators (Narrower Than Regular QR)

Regular QR uses a 4-bit mode indicator. Micro QR uses **fewer bits**, saving
precious capacity. The mode indicator is **prefixed** to the character count:

| Symbol | Mode Indicator Width | Mode Codes |
|--------|---------------------|------------|
| M1 | 0 bits (implicit numeric) | N/A — only numeric, no indicator |
| M2 | 1 bit | `0` = numeric, `1` = alphanumeric |
| M3 | 2 bits | `00` = numeric, `01` = alphanumeric, `10` = byte |
| M4 | 3 bits | `000` = numeric, `001` = alphanumeric, `010` = byte, `011` = kanji |

M1 has no mode indicator because it only supports one mode. This is analogous to
a function that has no arguments because it has no choices to make.

### Character Count Indicator Widths

After the mode indicator, the number of characters (not bytes) is encoded in a
fixed-width field. The width varies by mode and symbol version:

| Mode         | M1 | M2 | M3 | M4 |
|-------------|----|----|----|----|
| Numeric     | 3  | 4  | 5  | 6  |
| Alphanumeric| —  | 3  | 4  | 5  |
| Byte        | —  | —  | 4  | 5  |
| Kanji       | —  | —  | —  | 4  |

Compare these to regular QR's character count widths (10–14 bits for numeric,
9–13 for alphanumeric, 8–16 for byte). Micro QR's counts are dramatically
smaller because the symbols hold far fewer characters.

### Numeric Mode

Identical to regular QR numeric mode encoding:

- Groups of 3 digits → 10 bits (decimal value 000–999)
- Remaining pair of 2 digits → 7 bits (decimal value 00–99)
- Single remaining digit → 4 bits (decimal value 0–9)

Example: `"12345"` → groups `"123"`, `"45"` → bits `0001111011` (123 in 10 bits),
`0101101` (45 in 7 bits) = 17 bits.

```
Digit grouping (greedy from left):
  "12345" → "123" + "45" → 10 bits + 7 bits = 17 bits
  "1234"  → "123" + "4"  → 10 bits + 4 bits = 14 bits
  "123"   → "123"        → 10 bits           = 10 bits
  "12"    → "12"         → 7 bits            = 7 bits
  "1"     → "1"          → 4 bits            = 4 bits
```

### Alphanumeric Mode

Identical to regular QR alphanumeric encoding. The 45-character set:

```
0–9 → indices 0–9
A–Z → indices 10–35
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

Pairs of characters are packed into 11 bits: `first_index * 45 + second_index`.
A trailing single character uses 6 bits.

Example: `"AC-3"` → pair `"AC"` = (10)(12) = 10×45+12 = 462 = `00111001110`
(11 bits), pair `"-3"` = (41)(3) = 41×45+3 = 1848 = `11100111000` (11 bits).

### Byte Mode

Raw byte encoding. Each character is encoded as its ISO-8859-1 byte value in 8
bits. UTF-8 strings can be encoded in byte mode by treating each UTF-8 byte as
a raw byte (scanners supporting UTF-8 will decode correctly; legacy scanners
will see Latin-1 or garbled text for non-ASCII).

Example: `"Hi!"` → `0x48 0x69 0x21` → `01001000 01101001 00100001` (24 bits).

### Kanji Mode (M4 Only)

Encodes Japanese characters from the Shift-JIS encoding. Characters in the ranges
0x8140–0x9FFC and 0xE040–0xEBBF are supported. Each character is encoded in 13
bits using the following transformation:

```
1. Look up the 2-byte Shift-JIS code point.
2. Subtract 0x8140 (for range 0x8140–0x9FFC) or 0xC140 (for range 0xE040–0xEBBF).
3. Multiply the high byte by 0xC0 and add the low byte.
4. Encode the result in 13 bits.
```

This packs a character that normally needs 16 bits into 13 bits. Kanji mode is
only available in M4 — the other symbols are too small to hold meaningful kanji
content.

### Terminator

After all data segments, a **terminator** is appended (all zero bits). The
terminator width depends on the symbol version:

| Symbol | Terminator Width |
|--------|-----------------|
| M1     | 3 bits (000) |
| M2     | 5 bits (00000) |
| M3     | 7 bits (0000000) |
| M4     | 9 bits (000000000) |

Regular QR uses a 4-bit terminator for all versions. Micro QR uses longer
terminators to fill more of the final partial codeword, and shorter for M1 to
save space.

The terminator is truncated if the remaining capacity is less than the full
terminator width.

### Bit Stream Assembly

The encoder builds a **bit stream** as follows:

```
[mode indicator (0–3 bits, version-dependent)]
[character count (width from table above)]
[encoded data bits]
[terminator (3/5/7/9 zero bits, or truncated)]
[zero bits to reach next byte boundary]
[padding codewords: alternate 0xEC and 0x11 to fill remaining data codewords]
```

For M1, the bit stream has a special structure. M1 uses 3 data codewords, but
the last "codeword" is only **4 bits** (not 8). The M1 bit stream format is:

```
M1 total data bits = 3 × 8 − 4 = 20 bits  (NO — this is wrong)
M1 actual data capacity = 20 bits total (not 24), because the final nibble
  is a half-codeword.
```

Concretely: M1 has 3 data codewords. The first two are full 8-bit codewords
(16 bits); the third is 4 bits. So total usable data bits = 20. The standard
treats this as a 3-byte stream where the last byte is only 4 bits wide —
terminators and padding account for this.

The final bit stream is placed into the module grid after Reed-Solomon encoding
(see next section).

---

## Reed-Solomon Error Correction

Micro QR uses the same Galois field as regular QR Code:

- **Field**: GF(256)
- **Primitive polynomial**: `x^8 + x^4 + x^3 + x^2 + 1` (hex 0x11D, decimal 285)
- **Generator convention**: b=0 (first root is α^0 = 1)

This is identical to regular QR. The `gf256` package (MA01) provides all field
arithmetic needed.

The generator polynomial of degree `n` is:

```
g(x) = (x + α^0)(x + α^1)···(x + α^{n-1})
```

where `n` is the number of ECC codewords.

### Generator Polynomials

Micro QR needs only four distinct ECC codeword counts: 2, 5, 6, 8, 10, 14.
The generator polynomials (in coefficient form, highest degree first, over GF(256)):

```
2 ECC codewords (M1 detection):
  g(x) = x^2 + α^0·x + α^1·x^0... wait, let me derive properly.

  g(x) = (x + α^0)(x + α^1)
       = x^2 + (α^0 + α^1)x + α^0·α^1
       = x^2 + (1 + 2)x + 2
       = x^2 + 3x + 2
  Coefficients: [01, 03, 02]   (n+1 coefficients for degree n)
```

Full table of generator polynomials (coefficients in hex, leading 1 omitted,
highest power first):

| ECC CWs | Generator polynomial coefficients (hex) |
|---------|----------------------------------------|
| 2       | 03 02 |
| 5       | 1f f6 44 d9 68 |
| 6       | 3f 4e 17 9b 05 37 |
| 8       | 63 0d 60 6d 5b 10 a2 a3 |
| 10      | f6 75 a8 d0 c3 e3 36 e1 3c 45 |
| 14      | f6 9a 60 97 8a f1 a4 a1 8e fc 7a 52 ad ac |

These are the same polynomials used in regular QR for blocks with matching ECC
codeword counts. Implement as compile-time constants; do not compute at runtime.

### Block Structure

All Micro QR symbols use **a single block** — there are no groups, no interleaving.
The entire data stream is one block, and the ECC codewords for that one block
are appended to form the final symbol codeword stream.

This greatly simplifies the encoder: there is no block splitting, no
per-block RS computation, and no interleaving step.

```
final_codewords = data_codewords ++ rs_ecc(data_codewords, n_ecc)
```

### RS Encoding Algorithm

For a single block, RS encoding computes the remainder of polynomial division:

```
Given: data bytes D[0..k-1] (k = number of data codewords)
       generator poly G[0..n] (n = number of ECC codewords)

Compute ECC bytes E[0..n-1] by polynomial remainder:

ecc = [0] × n
for each byte b in data_codewords:
    feedback = b XOR ecc[0]
    shift ecc left by one (drop ecc[0], append 0)
    for i in 0..n-1:
        ecc[i] = ecc[i] XOR gf_multiply(G[n - i], feedback)

Result: E = ecc
```

This is identical to the QR Code RS encoder. The `gf_multiply` function is
GF(256) multiplication using the 0x11D primitive polynomial.

---

## Module Placement

After computing ECC, the final codeword stream is placed into the module grid
using a **two-column zigzag** scan.

### Reserved Modules

Before placing data, mark all structural modules as reserved:

```
1. Finder pattern: rows 0–6, cols 0–6  (49 modules)
2. Separators: row 7 cols 0–7, col 7 rows 0–7
   (15 unique modules — corner at row 7/col 7 is shared)
3. Timing row: row 0, cols 8 through (size-1)
4. Timing col: col 0, rows 8 through (size-1)
5. Format information strip (see Format Information section)
```

Note: there are no alignment patterns and no version information in Micro QR.

### Format Information Module Positions

The format information occupies 15 specific modules. These are the only modules
that need to be reserved before placement but filled in after mask selection:

```
Row 8, cols 1 through 8  →  8 modules  (format bits f14 down to f7)
Col 8, rows 1 through 7  →  7 modules  (format bits f6 down to f0)
```

Total: 15 modules, matching the 15-bit format information word exactly.

Illustrated for M4 (17×17):

```
  c0 c1 c2 c3 c4 c5 c6 c7 c8 ...
r0  T  T  T  T  T  T  T  T  T ...   (timing)
r1  T  ■  ■  ■  ■  ■  T  S [f0]...  (finder + sep + fmt)
r2  T  ■  □  □  □  ■  T  S [f1]...
r3  T  ■  □  ■  □  ■  T  S [f2]...
r4  T  ■  □  □  □  ■  T  S [f3]...
r5  T  ■  ■  ■  ■  ■  T  S [f4]...
r6  T  T  T  T  T  T  T  S [f5]...
r7  T  S  S  S  S  S  S  S [f6]...
r8  T [f14][f13][f12][f11][f10][f9][f8]  .  ...  (format row)
r9  T  .  .  .  .  .  .  .  .  ...
```

Reading order:

- f14 (MSB) at row 8, col 1
- f13 at row 8, col 2
- f12 at row 8, col 3
- f11 at row 8, col 4
- f10 at row 8, col 5
- f9  at row 8, col 6
- f8  at row 8, col 7
- f7  at row 8, col 8
- f6  at col 8, row 7
- f5  at col 8, row 6
- f4  at col 8, row 5
- f3  at col 8, row 4
- f2  at col 8, row 3
- f1  at col 8, row 2
- f0 (LSB) at col 8, row 1

The format information strip forms an L-shape: 8 bits going rightward along row
8, then 7 bits going upward along col 8. (Note the "upward" direction: row 7
holds f6, row 1 holds f0 — the LSB is nearest the finder corner.)

### Zigzag Data Placement

The data placement algorithm is a **two-column zigzag**, starting from the
**bottom-right** corner of the symbol, scanning upward, then downward alternately,
moving left two columns at a time:

```
size  = symbol side length (11, 13, 15, or 17)
col   = size - 1        -- start at rightmost column
dir   = -1              -- -1 = upward, +1 = downward
bit_index = 0           -- index into final_codewords bit stream

while col >= 1:
    -- scan this 2-column strip in the current direction
    if dir == -1:   -- upward
        rows = range(size-1, -1, -1)    -- size-1 down to 0
    else:           -- downward
        rows = range(0, size)           -- 0 up to size-1

    for row in rows:
        for sub_col in [col, col-1]:
            if module(row, sub_col) is reserved:
                skip
            place bit_stream[bit_index] at module(row, sub_col)
            bit_index += 1

    -- move left and flip direction
    dir = -dir
    col -= 2
```

Note that unlike regular QR Code, there is **no timing column at col 6** to skip
around — the timing in Micro QR is at col 0, which is always reserved and thus
naturally skipped by the reserved-module check.

The zigzag must stop at `col >= 1` (not `col >= 0`) because the leftmost two
columns (0 and 1) start at col 1 — col 0 is entirely reserved for timing.

Wait — col 0 is the timing column, so when `col = 1`, the two-column strip is
`col = 1` and `col - 1 = 0`. The col-0 modules are reserved (timing), so the
bit-placement skips them automatically. This is correct behavior.

After all data+ECC bits are placed, the remaining unreserved modules (if any)
receive **remainder bits** (zeros). M1 has 4 remainder bits; all others have 0.

---

## Masking

Masking flips data and ECC module values to avoid patterns that could confuse
scanners. Unlike regular QR which evaluates 8 masks, **Micro QR only uses 4
mask patterns**.

### The 4 Mask Patterns

| Pattern | Condition (flip module if true) |
|---------|---------------------------------|
| 0       | `(row + col) mod 2 == 0` |
| 1       | `row mod 2 == 0` |
| 2       | `col mod 3 == 0` |
| 3       | `(row + col) mod 3 == 0` |

These are the first four of regular QR's eight patterns. The more complex
patterns (4–7) are absent from Micro QR — the smaller symbol size means the
simpler patterns are sufficient to break up degenerate sequences.

Masking is applied **only to data and ECC modules**, never to finder pattern,
separator, timing, or format information modules.

### Penalty Scoring

All four candidate masks are evaluated, and the one with the **lowest penalty
score** is selected. The same four penalty rules as regular QR Code apply:

**Rule 1 — Adjacent run penalty:**

Scan each row and each column for runs of ≥5 consecutive modules of the same
color. Add `run_length − 2` to the penalty for each qualifying run.

```
Run of 5 → +3
Run of 6 → +4
Run of 7 → +5
...etc.
```

**Rule 2 — 2×2 block penalty:**

For each 2×2 square with all four modules the same color (all dark or all
light), add 3 to the penalty.

**Rule 3 — Finder-pattern-like sequences:**

Scan all rows and columns for the 11-module sequence `1 0 1 1 1 0 1 0 0 0 0`
or its reverse `0 0 0 0 1 0 1 1 1 0 1`. Each occurrence adds 40 to the penalty.

These sequences look like a finder pattern to a scanner and must be avoided.

**Rule 4 — Dark-module proportion:**

```
dark_count = number of dark modules in the entire symbol
total      = size × size
dark_pct   = dark_count × 100 / total   (integer percent)
prev5      = largest multiple of 5 ≤ dark_pct
next5      = prev5 + 5
penalty    = min(|prev5 - 50|, |next5 - 50|) / 5 × 10
```

The penalty is 0 when dark_pct is exactly 50%, increasing as the balance shifts.
A heavily dark or heavily light symbol is harder for scanners to process.

### Mask Selection

Evaluate all four masks. For each:

1. Apply mask to data/ECC modules.
2. Compute format information for this mask.
3. Write format information into the grid.
4. Compute penalty score.
5. Record (penalty, mask_index).

Select the mask with the **lowest total penalty**. Break ties by preferring the
lower-numbered mask pattern.

---

## Format Information

The format information tells a scanner which symbol version+ECC combination it
is reading, and which mask was applied. Micro QR encodes this differently from
regular QR.

### Format Information Bits

The 15-bit format string is constructed from:

```
[symbol_indicator (3 bits)][mask_pattern (2 bits)][BCH remainder (10 bits)]
```

Total: 3 + 2 + 10 = 15 bits.

**Symbol indicator** encodes both the version (M1–M4) and ECC level in 3 bits:

| Symbol + ECC    | Symbol Indicator |
|----------------|-----------------|
| M1 (det. only) | 000 |
| M2-L           | 001 |
| M2-M           | 010 |
| M3-L           | 011 |
| M3-M           | 100 |
| M4-L           | 101 |
| M4-M           | 110 |
| M4-Q           | 111 |

Eight possible combinations, covered by 3 bits. The symbol indicator is enough
to completely identify the symbol type — version and ECC level are bound together
here, unlike regular QR where ECC level and version are independent.

**Mask pattern** is a 2-bit value (00–11), indicating which of the four Micro QR
mask patterns was selected.

### BCH Code Generation

The 10-bit BCH remainder is computed using the same generator polynomial as
regular QR format information:

```
G(x) = x^10 + x^8 + x^5 + x^4 + x^2 + x + 1
     = 10100110111 in binary
     = 0x537 (but as a BCH generator: 0b10100110111)
```

Procedure:

```
1. Form the 5-bit data string:
      D = [symbol_indicator (3 bits)] [mask_pattern (2 bits)]

2. Left-shift by 10 positions (multiply by x^10):
      D_shifted = D << 10   (now a 15-bit integer)

3. Compute remainder:
      remainder = D_shifted mod G(x)   (polynomial division over GF(2))
      The result is a 10-bit integer.

4. Append remainder to D:
      format_15 = D_shifted | remainder   (15-bit integer)

5. XOR with the Micro QR mask value 0x4445:
      format_final = format_15 XOR 0x4445
```

Note: Micro QR uses **0x4445** as the XOR mask, **not** regular QR's 0x5412.
This is a deliberate difference to prevent a Micro QR symbol from being
misread as a regular QR symbol.

The XOR mask in binary: `100 0100 0100 0101` = 0x4445.

### Example: M2-L, mask pattern 1

```
symbol_indicator = 001  (M2-L)
mask_pattern     = 01   (pattern 1)
5-bit data       = 001 01 = 00101 = 0x05

D_shifted = 0x05 << 10 = 0x1400 = 0001 0100 0000 0000

Divide by G(x) = 10100110111:
  Polynomial division of 1 0100 0000 0000 0 by 10100110111
  (work through binary long division)
  ... remainder R

format_15 = D_shifted | R

format_final = format_15 XOR 0x4445
```

Implementors should generate a lookup table of all 32 possible format values
(8 symbol_indicator values × 4 mask patterns) at build time or embed them as a
constant table.

### Pre-computed Format Information Table

For convenience, here are all 32 format information values (after XOR with
0x4445), expressed as 15-bit integers. Bit 14 is MSB, bit 0 is LSB:

| Symbol+ECC | Mask 0 | Mask 1 | Mask 2 | Mask 3 |
|-----------|--------|--------|--------|--------|
| M1 (000)  | 0x4445 | 0x4172 | 0x4E2B | 0x4B1C |
| M2-L (001)| 0x5528 | 0x501F | 0x5F46 | 0x5A71 |
| M2-M (010)| 0x6649 | 0x637E | 0x6C27 | 0x6910 |
| M3-L (011)| 0x7764 | 0x7253 | 0x7D0A | 0x783D |
| M3-M (100)| 0x06DE | 0x03E9 | 0x0CB0 | 0x0987 |
| M4-L (101)| 0x17F3 | 0x12C4 | 0x1D9D | 0x18AA |
| M4-M (110)| 0x24B2 | 0x2185 | 0x2EDC | 0x2BEB |
| M4-Q (111)| 0x359F | 0x30A8 | 0x3FF1 | 0x3AC6 |

Note: these values are computed using the procedure above. Verify them against
the ISO standard worked examples before relying on them in production. Embed the
final table as a constant to avoid any computation errors.

### Format Information Placement

The 15-bit format value is placed into the symbol with **f14 as the MSB**:

```
Row 8, col 1  ← f14  (MSB)
Row 8, col 2  ← f13
Row 8, col 3  ← f12
Row 8, col 4  ← f11
Row 8, col 5  ← f10
Row 8, col 6  ← f9
Row 8, col 7  ← f8
Row 8, col 8  ← f7
Col 8, row 7  ← f6
Col 8, row 6  ← f5
Col 8, row 5  ← f4
Col 8, row 4  ← f3
Col 8, row 3  ← f2
Col 8, row 2  ← f1
Col 8, row 1  ← f0  (LSB)
```

There is **only one copy** of the format information in Micro QR (unlike regular
QR, which places it in two locations). This means Micro QR format information
offers no redundancy — if those modules are damaged, the symbol cannot be decoded.

---

## Complete Encoding Algorithm

The full encoding pipeline for a given input string, desired symbol version (or
auto-selection), and ECC level:

```
1. CHOOSE VERSION
   If version is not specified, auto-select:
   For each symbol M1, M2, M3, M4 in order:
     For the chosen ECC level (or all levels if not specified):
       Check if the input fits in the data capacity (data_codewords available).
       Use the smallest symbol+ECC combination that fits.
   If no symbol fits, raise InputTooLong.
   Validate that the chosen ECC level is available for the selected symbol
   (M1 only supports detection; M2/M3 support L and M; M4 supports L, M, Q).

2. ENCODE DATA
   a. Select encoding mode:
      - If all characters are digits 0–9 AND the symbol supports numeric: numeric
      - Else if all chars are in the 45-char alphanumeric set AND symbol supports
        alphanumeric: alphanumeric
      - Else if all bytes are in ISO-8859-1 AND symbol supports byte: byte
      - Else if all characters are valid Shift-JIS kanji AND symbol supports
        kanji: kanji
      - Raise InvalidMode if no mode works for this symbol.
      
   b. Build the bit stream:
      - mode indicator (0/1/2/3 bits depending on symbol)
      - character count (width from the table above)
      - encoded data bits (mode-specific encoding)
      - terminator (3/5/7/9 zero bits, truncated if capacity exhausted)
      - zero bits to reach next byte boundary
      - pad bytes: alternate 0xEC and 0x11 to fill remaining data codewords
      
   Special M1 case: the data capacity is 20 bits (not 24). After encoding,
   pad with zeros to 20 bits. No 0xEC/0x11 padding for M1 since it uses
   a non-integer number of full codewords.

3. COMPUTE RS ECC
   a. Extract the data codeword bytes from the bit stream.
   b. For M1: the "codewords" are the first 2 full bytes plus 4 bits;
      use the 3 data bytes (the last nibble sits in the high bits of byte 2
      with 4 zero low-bits). Feed all 3 bytes to the RS encoder.
   c. For all others: straightforward byte sequence.
   d. Compute ECC using the RS algorithm above.
   e. Append ECC bytes to data bytes → final_codewords.

4. BUILD BIT STREAM FOR PLACEMENT
   Flatten final_codewords to a bit string (MSB-first per codeword).
   For M1: the last data codeword contributes only 4 bits (the MSB nibble).

5. INITIALIZE GRID
   Create a size×size grid of unset modules.
   Place finder pattern at rows 0–6, cols 0–6.
   Place separators at row 7 cols 0–7 and col 7 rows 0–7 (all light).
   Place timing row 0 (cols 8 to size-1): alternating dark/light, dark at
     col 8 (even index).
   Place timing col 0 (rows 8 to size-1): alternating dark/light, dark at
     row 8 (even index).
   Mark format information modules (row 8 cols 1–8 and col 8 rows 1–7) as
     reserved (temporarily set to light = 0).

6. PLACE DATA
   Run the two-column zigzag from bottom-right, placing bits from the
   flat bit stream into unreserved modules.
   Set any remaining unreserved modules to remainder bits (0).

7. EVALUATE ALL 4 MASKS
   For each mask pattern 0–3:
   a. For each data/ECC module at (row, col) (non-reserved):
      if mask_condition(row, col): flip the module value.
   b. Compute format information for this mask.
   c. Place format information into reserved format modules.
   d. Compute penalty score (rules 1–4).
   e. Undo the mask (or work on a copy) and record (penalty, mask_index).

8. SELECT BEST MASK
   Choose the mask with the lowest penalty score (ties → lower index wins).

9. FINALIZE
   Apply the selected mask to all data/ECC modules in the grid.
   Write the final format information into the format modules.

10. LAYOUT + PAINT
    Pass the final ModuleGrid to barcode-2d's layout(grid, config), which
    resolves module positions to pixel coordinates and returns a PaintScene
    (P2D00). Pass to a PaintVM backend: paint-vm-svg for SVG output, or
    paint-metal for GPU-rendered PNG or native window.
    Include the 2-module quiet zone in the layout config.
```

---

## Implementation Notes

### Tables to Embed

The following tables must be embedded as compile-time constants:

1. **Capacity table**: for each of the 8 symbol+ECC combinations:
   - data codewords
   - ECC codewords
   - capacity per mode (numeric, alphanumeric, byte, kanji)

2. **RS generator polynomials**: for ECC codeword counts 2, 5, 6, 8, 10, 14.

3. **Format information lookup table**: all 32 pre-computed format words
   (8 symbol+ECC × 4 masks), stored as 15-bit integers.

4. **Format module positions**: the 15 (row, col) pairs for format placement.
   These are fixed and identical regardless of symbol size (always row 8 and
   col 8 relative to the finder).

### Quiet Zone

The quiet zone (2 modules on all sides) is **not part of the module grid**.
The grid is exactly `size × size`. The quiet zone is added at the layout/render
stage. The `layout()` function in barcode-2d accepts a `quiet_zone` parameter;
pass 2 (not 4) for Micro QR.

### Mode Selection Heuristic

For auto-selection, prefer the most compact mode that covers the full input:

```
1. All digits 0–9 and symbol supports numeric? → numeric mode
2. All chars in the 45-char alphanumeric set and symbol supports alphanumeric?
   → alphanumeric mode
3. All bytes ≤ 255 and symbol supports byte? → byte mode
4. All chars are valid Shift-JIS kanji and symbol is M4? → kanji mode
```

This is a greedy single-mode selection. Multi-mode segments (like QR Code's
"numeric-then-byte" for a URL with a code) are future work.

### Symbol vs. Version Terminology

The ISO standard refers to the four sizes as "M1", "M2", "M3", "M4" (symbol
designators), not "versions". In code, use the type name `MicroQRVersion` with
values `M1 | M2 | M3 | M4`. This avoids confusion with regular QR's integer
versions.

### Relationship to Regular QR

Micro QR shares:
- GF(256) field and primitive polynomial (0x11D)
- RS generator polynomial convention (b=0)
- Same 4 encoding modes (subset of regular QR's modes)
- Same finder pattern design
- Same penalty scoring rules
- Same format information BCH generator polynomial

Micro QR differs from regular QR in:
- Single finder pattern instead of three
- No alignment patterns
- No version information blocks
- No dark module
- Timing patterns at row 0 / col 0 instead of row 6 / col 6
- Only 4 masks instead of 8
- Narrower mode indicators (0–3 bits instead of 4)
- Narrower character count fields
- Longer terminators (3–9 bits instead of fixed 4)
- 2-module quiet zone instead of 4
- Format XOR mask: 0x4445 instead of 0x5412
- Format info encodes symbol_indicator (version+ECC) instead of ECC-only
- Format info is placed once (not twice)
- Single-block RS (no interleaving)

### UTF-8 and Byte Mode

When encoding to byte mode, use UTF-8 byte values for characters outside
ISO-8859-1. Multi-byte UTF-8 sequences are each treated as individual byte
values in the character count (so a 3-byte UTF-8 character counts as 3 in the
character count field and contributes 3 bytes of data, not 1 character).

### M1 Half-Codeword

M1's final codeword is a 4-bit "nibble", not a full byte. This means:
- The character count field for numeric is only 3 bits (max value 7)
- After encoding `"12345"` (the maximum), the bit stream must fit in 20 bits
- Terminate at the 3-bit boundary
- The RS encoder receives 3 bytes (with the third byte having the data in its
  upper 4 bits and zeros in the lower 4 bits)

Some implementations treat M1 as having 2 full data bytes plus 4 data bits,
while others treat it as 3 bytes where the last byte's low nibble is forced to
zero. Both approaches yield identical symbols.

### Error Detection vs. Error Correction

M1 provides error **detection** only, via its 2-byte "ECC". Strictly speaking,
these are used only to detect errors, not correct them. A scanner that finds
the RS check fails on an M1 symbol will report an unreadable symbol rather than
attempting correction. For M2–M4, the ECC bytes provide genuine error correction.

---

## Visualization Annotations

The encoder should optionally produce an **annotated module grid** where each
module records its role. This supports interactive learning tools and visualizers.

| Color (suggested) | Role |
|-------------------|------|
| Deep blue  | finder pattern module |
| Blue-grey  | separator module |
| Grey       | timing pattern module |
| Purple     | format information module |
| Black/white | data module |
| Green/light green | ECC module |
| Orange     | remainder bits |

There are no alignment, version information, or dark-module roles in Micro QR.
A visualizer can annotate each module with its codeword index and bit position,
making it possible to see exactly which character's data occupies which region
of the symbol.

---

## Error Types

```
MicroQRError::InputTooLong
  -- Input does not fit in any M1–M4 symbol at any ECC level.
  -- Suggest using regular QR Code instead.

MicroQRError::InputTooLongForECC(requested_ecc)
  -- Input fits at a lower ECC level but not the requested one.
  -- Include message: "fits at L but not M" or similar.

MicroQRError::InvalidMode(mode, symbol)
  -- Requested encoding mode not available for this symbol.
  -- E.g., byte mode requested but only M1 available.

MicroQRError::InvalidCharacter(char, mode)
  -- Character not encodable in the selected mode.
  -- E.g., lowercase letter in alphanumeric mode.

MicroQRError::ECCNotAvailable(ecc_level, symbol)
  -- E.g., ECC level H requested (not supported by any Micro QR symbol).
  -- E.g., ECC level Q requested for M2 (only L and M available).
```

---

## Public API

```
encode(input: string, ecc?: EccLevel) → ModuleGrid
  -- Encodes input to a Micro QR Code module grid (abstract module units, no
  -- pixels). Auto-selects the smallest symbol that fits the input at the
  -- requested ECC level. If ecc is omitted, defaults to M (medium).
  -- Raises InputTooLong if input exceeds M4 capacity.
  -- Raises InvalidCharacter if input contains unencodable characters.

encode_at(input: string, version: MicroQRVersion, ecc: EccLevel) → ModuleGrid
  -- Encodes to a specific symbol version. Raises InputTooLong if the input
  -- does not fit in the requested version/ECC combination.

layout(grid: ModuleGrid, config?: Barcode2DLayoutConfig) → PaintScene
  -- Translates a ModuleGrid into a pixel-resolved PaintScene (P2D00).
  -- Sets quiet_zone=2 automatically unless overridden in config.
  -- Delegates to barcode-2d::layout() — micro-qr does not implement this.

encode_and_layout(input: string, ecc?: EccLevel, config?: Barcode2DLayoutConfig)
  → PaintScene
  -- Convenience: encode + layout in one call.

render_svg(input: string, ecc?: EccLevel, config?: Barcode2DLayoutConfig) → string
  -- Convenience: encode + layout + paint-vm-svg backend → SVG string.

explain(input: string, ecc?: EccLevel) → AnnotatedModuleGrid
  -- Encode with full per-module role annotations. Used by visualizers.
  -- Each module includes its role, codeword index, and bit position.

MicroQRVersion = M1 | M2 | M3 | M4

EccLevel = Detection | L | M | Q
  -- Detection is M1 only. L/M available for M2–M4. Q only for M4.
  -- H is not available in Micro QR — raise ECCNotAvailable if requested.
```

---

## Package Matrix

| Language   | Directory                              | Depends on            |
|------------|----------------------------------------|-----------------------|
| Rust       | `code/packages/rust/micro-qr/`         | barcode-2d, gf256     |
| TypeScript | `code/packages/typescript/micro-qr/`   | barcode-2d, gf256     |
| Python     | `code/packages/python/micro-qr/`       | barcode-2d, gf256     |
| Go         | `code/packages/go/micro-qr/`           | barcode-2d, gf256     |
| Ruby       | `code/packages/ruby/micro_qr/`         | barcode-2d, gf256     |
| Elixir     | `code/packages/elixir/micro_qr/`       | barcode_2d, gf256     |
| Lua        | `code/packages/lua/micro-qr/`          | barcode-2d, gf256     |
| Perl       | `code/packages/perl/micro-qr/`         | barcode-2d, gf256     |
| Swift      | `code/packages/swift/micro-qr/`        | Barcode2D, GF256      |
| C#         | `code/packages/csharp/micro-qr/`       | Barcode2D, GF256      |
| F#         | `code/packages/fsharp/micro-qr/`       | Barcode2D, GF256      |
| Kotlin     | `code/packages/kotlin/micro-qr/`       | barcode-2d, gf256     |
| Java       | `code/packages/java/micro-qr/`         | barcode-2d, gf256     |
| Dart       | `code/packages/dart/micro-qr/`         | barcode-2d, gf256     |
| Haskell    | `code/packages/haskell/micro-qr/`      | barcode-2d, gf256     |

Each package follows the standard repo layout:

```
micro-qr/
  src/           (or lib/ per language conventions)
  tests/
  BUILD
  README.md
  CHANGELOG.md
  <build-manifest>  (Cargo.toml / package.json / pyproject.toml / etc.)
```

The `gf256` dependency is the MA01 package in the same language. The `barcode-2d`
dependency provides `ModuleGrid`, `Barcode2DLayoutConfig`, and `layout()`.

Note on language-specific naming:
- Ruby and Elixir conventionally use snake_case module/package names: `micro_qr`
- Rust, TypeScript, Python, Go, Lua, Perl: `micro-qr` (kebab-case directories)
- Swift, C#, F#: `MicroQR` or `Micro-QR` depending on toolchain conventions
- Kotlin, Java: `micro-qr` directory, `com.codingadventures.microqr` Java package

---

## Test Strategy

### Unit Tests

**1. RS Encoder**

For each of the five ECC codeword counts used in Micro QR (2, 5, 6, 8, 10, 14),
encode a known data sequence and verify the output ECC bytes match hand-computed
values. At minimum, verify the 2-ECC-codeword case:

```
data = [0x10, 0x20, 0x0C]   (example M1 data codewords for "123")
ecc  = rs_encode(data, 2)
-- expected: compute by hand or derive from standard's example
```

**2. Format Information**

Verify all 32 format words in the lookup table. Pick several arbitrary entries,
recompute from the BCH formula, and check against the table:

```
format_word(symbol_indicator=0b001, mask=0b01)   -- M2-L, mask 1
format_word(symbol_indicator=0b111, mask=0b11)   -- M4-Q, mask 3
```

**3. Mode Selection**

- `"12345"` → numeric, M1 (fits in 5 numeric chars)
- `"HELLO"` → alphanumeric, M2-L
- `"hello"` → byte, M3-L (lowercase not in alphanumeric set)
- `"https://a.b"` → byte (colon and slashes are in alphanumeric, but lowercase
  isn't — must use byte)
- `"123456"` → numeric, M2-L (6 digits exceeds M1's 5-char limit)

**4. Bit Stream Assembly**

- Verify correct mode indicators for each symbol version
- Verify correct character count widths
- Verify terminator length (3/5/7/9 bits)
- Verify 0xEC/0x11 padding fills to exact data codeword count
- Verify M1 20-bit data structure

**5. Penalty Scoring**

Create module grids with known degenerate patterns (all-dark row, checkerboard,
etc.) and verify the four penalty rules produce expected scores.

**6. Masking**

Apply each of the 4 mask patterns to a known grid, verify the XOR is applied
only to data/ECC modules (not structural modules), and verify the format
information changes to reflect the new mask.

### Integration Tests

**1. Encode a known M1 symbol: `"1"`**

```
Input: "1" at M1 (detection only)
Expected:
  - Symbol is 11×11
  - Grid passes scanner validation
  - Decode via external library confirms output == "1"
```

**2. Encode at the limit of M1: `"12345"`**

```
Input: "12345" at M1
Expected:
  - Fits exactly (5 numeric chars is M1 capacity)
  - "123456" should fall through to M2
```

**3. Alphanumeric in M2: `"HELLO"`**

```
Input: "HELLO" at M2-L
Expected:
  - Symbol is 13×13
  - Decode confirms "HELLO"
```

**4. Byte mode URL: `"https://a.b"`**

```
Input: "https://a.b" at M4-L
Expected:
  - Symbol is 17×17
  - Byte mode selected
  - Decode confirms "https://a.b"
```

**5. All ECC levels for M4**

```
Input: "MICRO QR" at each of M4-L, M4-M, M4-Q
Expected:
  - Each produces a valid 17×17 symbol
  - Different format information bits for each level
  - All decode correctly
```

**6. Scanner round-trip**

Use a Python script calling `zxing-cpp` or `zbar` to decode the rendered PNG
from each language's implementation and compare to the original input. All 15
implementations should produce scannable symbols.

### Cross-Language Verification

All 15 implementations must produce **bit-for-bit identical `ModuleGrid` outputs**
for the same input. The test corpus:

```
"1"              (M1, minimal numeric)
"12345"          (M1, maximum numeric capacity)
"HELLO"          (M2-L, alphanumeric)
"01234567"       (M2-L, numeric, 8 digits)
"https://a.b"    (M4-L, byte mode URL)
"MICRO QR TEST"  (M3-L, alphanumeric, typical use)
```

For each test input, serialize the ModuleGrid to a plain text format (e.g.,
`"0" and "1"` characters, row by row) and compare across all 15 language outputs.
A CI job should run this comparison automatically.

---

## Dependency Stack

```
paint-metal (P2D02)    paint-vm-svg    paint-vm-canvas
      └──────────────────┬─────────────────┘
                         │
                  paint-vm (P2D01)
                         │
              paint-instructions (P2D00)
                         │
                     barcode-2d          MA01 gf256
                         │                   │
                    micro-qr ───────────────┘
```

`micro-qr` depends on:
- `barcode-2d` — for `ModuleGrid` type and `layout()` function
- `gf256` (MA01) — for GF(256) multiplication in the RS encoder

`micro-qr` does **not** depend on:
- `qr-code` — Micro QR is a sibling package, not a superset or subset
- MA02 (reed-solomon) — Micro QR uses the same b=0 RS convention as QR Code
  and handles RS internally (the computation is simple enough)
- Any alignment table package — Micro QR has no alignment patterns

The relationship between `micro-qr` and `qr-code`:
- Both encode barcode symbols to `ModuleGrid`
- Both use the same GF(256) field
- Neither depends on the other
- A future `barcode-factory` package might wrap both with a unified API

---

## Future Extensions

- **Decoder** — Micro QR decoding is simpler than regular QR (no perspective
  correction for three finder patterns) but still requires image preprocessing
  and RS syndrome decoding. A separate spec.

- **Mixed-mode encoding** — Allow a single symbol to contain both a numeric
  segment and a byte segment for maximum capacity. For example, a barcode like
  `"PART-12345"` could encode `"PART-"` in alphanumeric and `"12345"` in
  numeric. Significant implementation complexity, modest capacity improvement.

- **rMQR (Rectangular Micro QR)** — ISO/IEC 23941:2022 defines a rectangular
  variant for extremely space-constrained applications. Similar structure to
  Micro QR but in a rectangular rather than square symbol. A separate spec.

- **QR Code integration** — A `barcode-factory` package that accepts an input
  string and desired ECC level, and automatically chooses between Micro QR and
  regular QR based on capacity requirements and size constraints.

- **Visualizer integration** — The `explain()` API returns an `AnnotatedModuleGrid`
  that a browser-based visualizer can render with per-module color coding,
  showing exactly which bits encode which characters, which modules are ECC vs.
  data, and how the format information identifies the symbol type.

- **Japanese Industrial Standard validation** — Verify symbols against the JIS
  X 0510 standard (the Japanese national standard equivalent of ISO/IEC 18004),
  which includes additional test vectors.
