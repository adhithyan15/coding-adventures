# Data Matrix ECC200

## Overview

This spec defines a **Data Matrix ECC200 encoder** for the coding-adventures
monorepo.

Data Matrix is a two-dimensional matrix barcode first developed by RVSI Acuity
CiMatrix (formerly Siemens) in 1989 under the name "DataCode." The ECC200
variant — introduced in the mid-1990s and standardized as ISO/IEC 16022:2006
— replaced the older ECC000–ECC140 lineage with Reed-Solomon error correction
over GF(256). It is now the dominant form worldwide and the only variant this
spec covers.

Data Matrix is used wherever small, high-density, damage-tolerant marks are
needed on physical objects:

- **Printed circuit boards**: every PCB carries a Data Matrix etched or printed
  on the substrate for traceability through automated assembly lines.
- **Pharmaceuticals**: the US FDA mandates Data Matrix 2D barcodes on unit-dose
  packaging for drug traceability (DSCSA).
- **Aerospace**: parts marking on aircraft components — rivets, shims, brackets —
  requires marks that survive decades of abrasion, heat, and cleaning chemicals.
  Data Matrix is used because it can be etched (dot-peen, laser, chemical) onto
  metal, surviving conditions that would destroy ink-printed labels.
- **US Postal Service**: the USPS 4-State Customer Barcode (Intelligent Mail)
  embeds routing information in a stacked linear format, but Data Matrix is used
  on registered mail and customs forms.
- **Medical devices**: DI (device identification) codes on surgical instruments
  and implants.

The format's defining characteristics:

1. **No masking** — data is placed directly without the XOR masking step QR
   requires. The diagonal placement algorithm distributes bits well enough
   without it.
2. **L-shaped finder + clock border** — instead of three separate finder
   patterns, the entire perimeter is used: a solid-dark L on two sides, and
   alternating dark/light on the other two. This single-pass border is faster
   to locate than three disjoint finder patterns.
3. **Diagonal "Utah" placement** — codeword bits spiral diagonally through the
   grid in a pattern that resembles the outline of the US state of Utah. The
   diagonal trajectory distributes bits evenly, gives every codeword physical
   separation from its neighbors, and avoids the need for masking.
4. **Uses MA02 (b=1) Reed-Solomon directly** — unlike QR, which uses a
   different generator root convention (b=0), Data Matrix uses exactly the
   same b=1 convention as the MA02 reed-solomon package in this repo.

Understanding how to build a Data Matrix encoder from scratch teaches:

- how structured borders serve as both finders and timing signals
- how a diagonal placement algorithm distributes bits without masking
- how large symbols are subdivided into data regions with interregion alignment
  borders
- how multiple interleaved RS blocks improve burst-error resilience
- how the same GF(256) math underlies different barcode standards

The encoder in this spec produces a **valid, scannable Data Matrix ECC200
symbol** for any input string that fits within the 144×144 maximum symbol size.
It does not implement decoding.

---

## Symbol Structure

A Data Matrix ECC200 symbol is a square or rectangular grid of **modules** —
dark (1) or light (0) square cells. Every module participates in the symbol:
there are no dedicated "alignment-only" interior patterns separate from the
data area (unlike QR's alignment patterns, which eat into data capacity).

### The "finder + clock" border

The outermost ring of every Data Matrix symbol forms a fixed perimeter pattern.
Reading the perimeter clockwise from the bottom-left corner:

```
  col 0    col 1   ...  col N-1   col N
row 0:  D   D    D   D   D   D   D   D
        ^                               ^
        |  timing (alternating D/L)     |
        L                               L
row 1:  D                               D
        |  ...       data modules ...   |
        |  (interior)                   |
        L                               L
row N:  D   L    D   L   D   L   D   L
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
        finder L-bar (all dark, bottom row)

Legend:
  D = dark module
  L = light module
  col 0 = left column (all dark = "L-bar" left leg)
  row N = bottom row  (all dark = "L-bar" bottom leg)
  row 0 = top row     (alternating D/L starting with D = timing clock)
  col N = right column (alternating D/L starting with D = timing clock)
```

More precisely for a symbol of size R rows × C columns:

```
Top row    (row 0):       module[0][c] = (c % 2 == 0) ? dark : light
                          -- alternating, starts dark at col 0
Right col  (col C-1):     module[r][C-1] = (r % 2 == 0) ? dark : light
                          -- alternating, starts dark at row 0
Bottom row (row R-1):     module[R-1][c] = dark  (all dark)
Left col   (col 0):       module[r][0]   = dark  (all dark)
```

The L-shaped solid-dark bar (left column + bottom row) is the **finder
pattern**. It tells a scanner where the symbol is and which way it is
oriented. The alternating dark/light clock on the top row and right column
is the **timing pattern**. It tells the scanner the module pitch so it can
recover from slight distortion.

Together the four sides form the "finder + clock" border:

```
D D D D D D D D D D   ← timing (col 0 is also the start of the L-finder)
D . . . . . . . . D   ← right column timing  |
D . data modules . D   |
D . (interior)   . D   |
D . . . . . . . . D   |
D . . . . . . . . L   ← timing (alternating)
D D D D D D D D D D   ← L-finder bottom row (all dark)
↑
L-finder left col (all dark)
```

The interior `(R-2) × (C-2)` module area after stripping the perimeter border
is the "data area." This is where data and ECC codewords are placed by the
Utah algorithm.

### Quiet zone

A quiet zone of **1 module** on all sides is required around the symbol.
This is narrower than QR's 4-module quiet zone because the L-finder is itself
high-contrast and self-delimiting. Many practical applications use 2 modules
for robustness. The `quiet_zone_modules` default for this package should be 1.

### Visualization of a 10×10 symbol

Below is an annotated map of the smallest Data Matrix symbol (10×10 modules).
The data area is 8×8 after stripping the border.

```
  col: 0  1  2  3  4  5  6  7  8  9
row 0: [D][D][L][D][L][D][L][D][L][D]   ← timing row (top)
row 1: [D][.][.][.][.][.][.][.][.][D]   ← right col timing
row 2: [D][.][.][.][.][.][.][.][.][L]
row 3: [D][.][.][.][.][.][.][.][.][D]
row 4: [D][.][  data + ECC modules .][D]
row 5: [D][.][  (placed by Utah   .][L]
row 6: [D][.][   algorithm)        .][D]
row 7: [D][.][.][.][.][.][.][.][.][L]
row 8: [D][.][.][.][.][.][.][.][.][D]
row 9: [D][D][D][D][D][D][D][D][D][D]   ← L-finder bottom row

Legend:
  [D] = dark module (fixed, structural)
  [L] = light module (fixed, structural)
  [.] = data/ECC module (set by Utah placement)
```

Note that (row 0, col 0) is dark — it is simultaneously the start of the
L-finder (left column dark) and the start of the timing row. Both converge
at the corner.

---

## Symbol Sizes

Data Matrix ECC200 comes in 30 square sizes and 6 rectangular sizes, for a
total of 36 standardized variants.

### Square symbol sizes

| Symbol | Data regions | Data codewords | ECC codewords | Max chars (ASCII) |
|--------|-------------|----------------|---------------|-------------------|
| 10×10  | 1×1         | 3              | 5             | 1                 |
| 12×12  | 1×1         | 5              | 7             | 3                 |
| 14×14  | 1×1         | 8              | 10            | 6                 |
| 16×16  | 1×1         | 12             | 12            | 10                |
| 18×18  | 1×1         | 18             | 14            | 16                |
| 20×20  | 1×1         | 22             | 18            | 20                |
| 22×22  | 1×1         | 30             | 20            | 28                |
| 24×24  | 1×1         | 36             | 24            | 34                |
| 26×26  | 1×1         | 44             | 28            | 42                |
| 32×32  | 2×2         | 62             | 36            | 60                |
| 36×36  | 2×2         | 86             | 42            | 84                |
| 40×40  | 2×2         | 114            | 48            | 112               |
| 44×44  | 2×2         | 144            | 56            | 142               |
| 48×48  | 2×2         | 174            | 68            | 172               |
| 52×52  | 2×2         | 204            | 84            | 202               |
| 64×64  | 4×4         | 280            | 112           | 278               |
| 72×72  | 4×4         | 368            | 144           | 366               |
| 80×80  | 4×4         | 456            | 192           | 454               |
| 88×88  | 4×4         | 576            | 224           | 574               |
| 96×96  | 4×4         | 696            | 272           | 694               |
| 104×104 | 4×4        | 816            | 336           | 814               |
| 120×120 | 6×6        | 1050           | 408           | 1048              |
| 132×132 | 6×6        | 1304           | 496           | 1302              |
| 144×144 | 6×6        | 1558           | 620           | 1556              |

Notes:

- "Data regions" = how many subregions the interior is divided into (see
  below). Small symbols (up to 26×26) have a single data region (1×1).
- "Max chars (ASCII)" is approximate; digit pairs take half the codeword
  budget because two-digit sequences are packed into one codeword.
- The 24 sizes above are the primary 24 square sizes. The ISO standard lists
  30 total square sizes; the 6 additional "extended" square sizes
  (52×52 is the first extended) involve row/column interleaving for
  particularly large blocks — the table above covers all values an
  implementer needs.

### Rectangular symbol sizes

| Symbol | Data regions | Data codewords | ECC codewords |
|--------|-------------|----------------|---------------|
| 8×18   | 1×1         | 5              | 7             |
| 8×32   | 1×2         | 10             | 11            |
| 12×26  | 1×1         | 16             | 14            |
| 12×36  | 1×2         | 22             | 18            |
| 16×36  | 1×2         | 32             | 24            |
| 16×48  | 1×2         | 49             | 28            |

Rectangular symbols follow the same encoding algorithm as square symbols.
Their data regions are always single row (1 row of regions, 1 or 2 region
columns). The L-finder and timing clock borders are on the same sides (left
column and bottom row dark; top row and right column alternating).

### Symbol size selection

The encoder must select the **smallest symbol** that can hold the encoded data.
The selection algorithm:

```
1. Compute the number of encoded codewords for the input
   (encoding details in the next section).
2. Iterate over the symbol sizes in ascending order (10×10, 12×12, 14×14, ...,
   then rectangular variants 8×18, 8×32, ..., if rectangular mode is enabled).
3. Select the first symbol whose data_codewords capacity ≥ encoded codeword count.
4. If none fits, return InputTooLong.
```

By default, the encoder selects from square symbols only. Rectangular symbols
are available when `DataMatrixOptions.shape = Rectangular`.

---

## Data Regions

For symbols larger than 26×26, the interior data area is subdivided into
**data regions** — rectangular subareas separated by "alignment borders."
Each alignment border is 2 modules wide and consists of a solid-dark bar
adjacent to an alternating dark/light bar (the same visual language as the
outer finder + timing border). The alignment borders between regions allow
a scanner to correct for perspective distortion within large symbols.

The structure is:

```
┌─────────────────────────────────────────────────────────────────┐
│  outer finder+timing border (all four sides)                    │
│  ┌──────────┬──┬──────────┬──┬──────────┐                      │
│  │ data     │AB│ data     │AB│ data     │                      │
│  │ region   │  │ region   │  │ region   │                      │
│  │ (10×10   │  │ (10×10   │  │ (10×10   │                      │
│  │ interior)│  │ interior)│  │ interior)│                      │
│  ├──────────┴──┴──────────┴──┴──────────┤                      │
│  │ alignment border (horizontal)        │                      │
│  ├──────────┬──┬──────────┬──┬──────────┤                      │
│  │ data     │AB│ data     │AB│ data     │                      │
│  │ region   │  │ region   │  │ region   │                      │
│  └──────────┴──┴──────────┴──┴──────────┘                      │
│                                                                  │
│  AB = alignment border column (2 modules wide)                   │
└─────────────────────────────────────────────────────────────────┘
```

### Alignment border pattern

Horizontal alignment borders (between rows of regions):

```
row AB+0: D D D D D D D D ...   (all dark)
row AB+1: D L D L D L D L ...   (alternating, starts dark)
```

Vertical alignment borders (between columns of regions):

```
col AB+0: D D D D D D D D ...   (all dark)
col AB+1: D L D L D L D L ...   (alternating, starts dark)
```

The alignment border has the same "solid + alternating" structure as the outer
finder + timing border but embedded within the interior.

### Data region layout table

Selected symbol sizes showing data region dimensions:

| Symbol  | # row × col regions | Region data size (interior) |
|---------|---------------------|-----------------------------|
| 10×10   | 1×1                 | 8×8                         |
| 12×12   | 1×1                 | 10×10                       |
| 14×14   | 1×1                 | 12×12                        |
| 16×16   | 1×1                 | 14×14                        |
| 18×18   | 1×1                 | 16×16                        |
| 20×20   | 1×1                 | 18×18                        |
| 22×22   | 1×1                 | 20×20                        |
| 24×24   | 1×1                 | 22×22                        |
| 26×26   | 1×1                 | 24×24                        |
| 32×32   | 2×2                 | 14×14 each                   |
| 40×40   | 2×2                 | 18×18 each                   |
| 44×44   | 2×2                 | 20×20 each                   |
| 48×48   | 2×2                 | 22×22 each                   |
| 64×64   | 4×4                 | 14×14 each                   |
| 88×88   | 4×4                 | 20×20 each                   |
| 104×104 | 4×4                 | 24×24 each                   |
| 120×120 | 6×6                 | 18×18 each                   |
| 132×132 | 6×6                 | 20×20 each                   |
| 144×144 | 6×6                 | 22×22 each                   |

Computing the region interior size from the symbol dimensions:

```
For a symbol of size R×C with rr×rc data region rows×cols:
  region_height = (R - 2) / rr - 2    -- subtract outer border, divide, subtract AB
  region_width  = (C - 2) / rc - 2

More precisely, the "matrix height" (interior excl. outer border) = R - 2,
and with rr regions and (rr - 1) horizontal alignment borders (each 2 wide):
  region_data_height = (R - 2 - 2*(rr-1)) / rr

Equivalently, the ISO standard defines the "number of data rows" and
"number of data columns" per region directly as tabulated constants. Embed
these as a lookup table rather than recomputing at runtime.
```

### Importance of data regions for the Utah algorithm

The Utah placement algorithm (described below) operates within the
**logical data matrix** — the concatenation of all data region interiors,
treated as a single flat grid. The algorithm knows nothing about the
structural borders. The implementation must map from logical (row, col) in
the flat data matrix to physical (row, col) in the full symbol, accounting
for outer border, alignment borders, and region layout. This mapping is
performed once when writing the final module grid.

---

## Data Encoding

Data Matrix ECC200 does not use distinct "mode indicators" like QR. Instead
it uses a **codeword vocabulary** of 256 values, where different ranges of
codeword values invoke different encoding schemes. The encoding state machine
starts in ASCII mode and can switch to other modes using latch/shift
codewords.

### ASCII mode (default)

ASCII mode is the default state. Each codeword encodes one character or two
digits.

| Input | Codeword value | Range |
|-------|----------------|-------|
| ASCII 0–127 (single char) | ASCII_value + 1 | 1–128 |
| Two consecutive ASCII digits (0x30–0x39) | 130 + (d1×10 + d2) | 130–229 |
| ASCII 128–255 (extended; uses UPPER_SHIFT first) | 235, then ASCII_value - 127 | — |
| Pad (fill unused capacity) | 129 | — |
| Latch to C40 mode | 230 | — |
| Latch to Text mode | 239 | — |
| Latch to X12 mode | 238 | — |
| Latch to EDIFACT mode | 240 | — |
| Latch to Base256 mode | 231 | — |
| Return to ASCII from C40/Text/X12 | 254 (Unlatch) | — |
| Return to ASCII from EDIFACT | 0x1f in EDIFACT stream | — |

The digit-pair optimization is critical for performance:

```
"12" → codeword 130 + (1×10 + 2) = 142       (1 codeword for 2 chars)
"1"  → codeword 1 + 0x31 = 50                 (1 codeword for 1 char)
```

Encoding "12345678":
```
"12" → 142   "34" → 174   "56" → 196   "78" → 208
Total: 4 codewords for 8 digit characters.
Without digit pairs: 8 codewords. Saving: 50%.
```

The ASCII encoder should consume digit pairs greedily: whenever the current
and next character are both ASCII digits (0x30–0x39), emit one codeword for
the pair and advance two positions. Otherwise, encode the current character
as a single codeword.

#### Extended ASCII (UPPER_SHIFT)

Characters with ASCII value 128–255 are encoded as two codewords:

```
UPPER_SHIFT (235), then  ASCII_value - 127
```

Example: ü (ASCII 252):
```
codeword 1: 235   (UPPER_SHIFT)
codeword 2: 252 - 127 = 125
```

### C40 mode

C40 is optimized for uppercase alphanumeric text mixed with numbers and
special characters. It was named for the encoding of 40 characters using
a base-40 packing scheme.

C40 encodes three character values per two codeword bytes:

```
Three C40 values c1, c2, c3 → two bytes:
  word = 1600 × c1 + 40 × c2 + c3 + 1
  byte_high = word >> 8
  byte_low  = word & 0xFF
```

The C40 character set (base values):

| Value | Characters |
|-------|-----------|
| 0     | Shift 1 (values 0–31: ASCII control chars) |
| 1     | Shift 2 (values 33–47, 58–64, 91–95) |
| 2     | Shift 3 (values 96–127) |
| 3     | Space (ASCII 32) |
| 4–13  | Digits 0–9 |
| 14–39 | Uppercase A–Z |

Shift 1, 2, and 3 are shift prefixes that extend the base-40 vocabulary to
cover all 128 ASCII characters. A character needing a shift prefix consumes
two C40 values (shift + character value) instead of one.

To return to ASCII mode from C40: emit the Unlatch codeword (254).

C40 mode is most efficient when the message is dominated by uppercase letters
and digits — common in alphanumeric part numbers, lot codes, and identifiers
used in manufacturing and healthcare.

### Text mode

Text mode is structurally identical to C40 but with lowercase letters in the
base set (values 14–39 map to a–z). Uppercase letters require Shift 3.
Text mode is efficient when the message is dominated by lowercase ASCII text.

| Value | Characters |
|-------|-----------|
| 0     | Shift 1 |
| 1     | Shift 2 |
| 2     | Shift 3 |
| 3     | Space |
| 4–13  | Digits 0–9 |
| 14–39 | Lowercase a–z |

### X12 mode

X12 mode encodes the 40-character subset used in ANSI X12 EDI transactions:

```
Carriage return (CR), asterisk (*), greater-than (>), space ( ),
digits 0–9, uppercase A–Z
```

Same packing formula as C40 (three values per two codewords). X12 is used
almost exclusively for electronic data interchange (EDI) transactions in
healthcare and supply chain.

### EDIFACT mode

EDIFACT mode packs four 6-bit values into three bytes. Each EDIFACT character
is an ASCII value in the range 32–94 (printable ASCII excluding DEL and
most control characters). The 6-bit value is `ASCII_value - 32`.

```
Four EDIFACT values v1, v2, v3, v4 → three bytes:
  three_bytes = (v1 << 18) | (v2 << 12) | (v3 << 6) | v4
  byte_1 = (three_bytes >> 16) & 0xFF
  byte_2 = (three_bytes >>  8) & 0xFF
  byte_3 =  three_bytes        & 0xFF
```

To return to ASCII from EDIFACT: emit the special 6-bit Unlatch value
`0x1F` (31) as the next EDIFACT character before the normal end-of-mode
handling.

EDIFACT is used in European and international EDI messaging formats.

### Base256 mode

Base256 mode encodes arbitrary binary data. The length is embedded using a
length indicator codeword that may be 1 or 2 bytes:

```
If length ≤ 249: one length codeword = length
If length > 249:
  length_byte_1 = (length / 250) + 249
  length_byte_2 = length mod 250
```

Data bytes are randomized using a 255-element pseudorandom sequence
(derived from the position within the symbol) to prevent long runs of the
same value from creating degenerate placement patterns.

The randomization function for byte at position `k` (1-indexed from start
of symbol's codeword sequence):

```
scrambled = (raw_byte + (((149 × k) mod 255) + 1)) mod 256
```

And to unscramble during decoding:

```
raw_byte = (scrambled - (((149 × k) mod 255) + 1) + 256) mod 256
```

### Mode selection (v0.1.0)

For the initial implementation, use ASCII mode for all input. This is correct
for all printable ASCII text and supports the digit-pair optimization. The
more efficient C40, Text, X12, EDIFACT, and Base256 modes are v0.2.0
enhancements.

ASCII mode selection heuristic:

```
If all characters in the input are ASCII digits (0x30–0x39):
    Use digit-pair packing exclusively. This halves codeword count.
Else:
    Encode each character as a single ASCII codeword (ASCII + 1).
    Apply digit-pair packing greedily wherever two consecutive digits appear.
```

### Pad codewords

After encoding the data, the codeword sequence must be padded to exactly
`data_codewords` bytes (the symbol's capacity). Padding rules:

```
1. The first pad byte is always 129 (the pad codeword).
2. Subsequent pad bytes are "scrambled" pads using the same 149×k formula
   as Base256, applied at the position k of the pad byte within the full
   codeword stream:

   pad_k = (129 + (((149 × k) mod 253) + 1)) mod 254
   -- Note: mod 253 and mod 254, not 255/256 — different formula for pad!

   More precisely (ISO/IEC 16022 §5.2.3):
   scrambled_pad = 129 + (149 × k mod 253) + 1
   if scrambled_pad > 254: scrambled_pad -= 254

3. Each position k is 1-indexed from the start of the codeword sequence
   (including data codewords before the pad region — k starts at
   (data_codewords_used + 1) for the first pad).
```

The scrambled pad prevents a sequence of "129 129 129 ..." from creating
a degenerate placement pattern in the unused area of a symbol.

---

## Reed-Solomon Error Correction

Data Matrix ECC200 uses Reed-Solomon over **GF(256)** with the primitive
polynomial:

```
x^8 + x^5 + x^4 + x^2 + x + 1   (decimal 301, hex 0x12D)
```

This is **different from QR Code's polynomial** (0x11D). Both are
degree-8 irreducible polynomials over GF(2) and therefore define a valid
GF(256) field, but the fields are non-isomorphic — the primitive element
α has different additive and multiplicative properties in each.

### Generator polynomial convention

Data Matrix uses the **b=1 convention** — the generator polynomial's roots
are α^1, α^2, ..., α^n:

```
g(x) = (x + α^1)(x + α^2)···(x + α^n)
```

This is exactly the convention used by **MA02 (reed-solomon)** in this repo.
Data Matrix implementations MUST use MA02 with the 0x12D field polynomial,
not QR's 0x11D polynomial.

The difference in convention between QR (b=0) and Data Matrix (b=1):

| Format       | Primitive poly | Generator roots | MA02 usable? |
|--------------|----------------|-----------------|--------------|
| QR Code      | 0x11D          | α^0 ... α^{n-1} | No (wrong roots) |
| Data Matrix  | 0x12D          | α^1 ... α^n     | Yes (exact match) |
| Aztec full   | 0x12D          | α^1 ... α^n     | Yes (exact match) |

### GF(256) with polynomial 0x12D

The GF(256) field for Data Matrix is generated with the reduction rule:
when a polynomial multiplication produces a degree ≥ 8 term, reduce modulo
0x12D:

```
0x12D = 0b100101101
       = x^8 + x^5 + x^4 + x^2 + x + 1

Reduction: if bit 8 is set in intermediate result r:
    r = r XOR 0x12D       (because x^8 ≡ x^5+x^4+x^2+x+1 in this field)
```

The alpha (primitive element) table for 0x12D:

```
α^0  = 1 (= 0x01)
α^1  = 2 (= 0x02)
α^2  = 4 (= 0x04)
α^3  = 8 (= 0x08)
α^4  = 16 (= 0x10)
α^5  = 32 (= 0x20)
α^6  = 64 (= 0x40)
α^7  = 128 (= 0x80)
α^8  = 0x2D  (= 45  — 0x80 << 1, reduced by 0x12D)
α^9  = 0x5A  (= 90)
α^10 = 0xB4  (= 180)
α^11 = 0x6D  (= 109)  -- reduction applied
α^12 = 0xDA  (= 218)
...
α^254 = ??? (compute from recurrence)
α^255 = 1    (field element order = 255)
```

Implementations must precompute the full 256-entry exp table and 256-entry
log table for the 0x12D field. Do not reuse QR's 0x11D tables.

### ECC block structure

For large symbols, the data codeword stream is split across multiple
interleaved RS blocks. Each block has its own independent RS computation.
Interleaving distributes burst errors across all blocks, so a physical
scratch or contamination that destroys a contiguous region of the symbol
affects at most a few codewords from each block — well within each block's
correction capacity.

Full ECC block table:

| Symbol  | Data CWs | ECC CWs | Blocks | Data/block     | ECC/block |
|---------|----------|---------|--------|----------------|-----------|
| 10×10   | 3        | 5       | 1      | 3              | 5         |
| 12×12   | 5        | 7       | 1      | 5              | 7         |
| 14×14   | 8        | 10      | 1      | 8              | 10        |
| 16×16   | 12       | 12      | 1      | 12             | 12        |
| 18×18   | 18       | 14      | 1      | 18             | 14        |
| 20×20   | 22       | 18      | 1      | 22             | 18        |
| 22×22   | 30       | 20      | 1      | 30             | 20        |
| 24×24   | 36       | 24      | 1      | 36             | 24        |
| 26×26   | 44       | 28      | 1      | 44             | 28        |
| 32×32   | 62       | 36      | 2      | 31             | 18        |
| 36×36   | 86       | 42      | 2      | 43             | 21        |
| 40×40   | 114      | 48      | 2      | 57             | 24        |
| 44×44   | 144      | 56      | 4      | 36             | 14        |
| 48×48   | 174      | 68      | 4      | 43 + 44 + ...  | 17        |
| 52×52   | 204      | 84      | 4      | 51             | 21        |
| 64×64   | 280      | 112     | 4      | 70             | 28        |
| 72×72   | 368      | 144     | 4      | 92             | 36        |
| 80×80   | 456      | 192     | 4      | 114            | 48        |
| 88×88   | 576      | 224     | 4      | 144            | 56        |
| 96×96   | 696      | 272     | 4      | 174            | 68        |
| 104×104 | 816      | 336     | 6      | 136            | 56        |
| 120×120 | 1050     | 408     | 6      | 175            | 68        |
| 132×132 | 1304     | 496     | 8      | 163            | 62        |
| 144×144 | 1558     | 620     | 10     | 155 + 156×4+.. | 62        |

For symbols with multiple blocks, split data codewords evenly:
- If `data_codewords` is divisible by `num_blocks`: each block has
  `data_codewords / num_blocks` data codewords.
- If not evenly divisible: the first `data_codewords mod num_blocks` blocks
  get one extra codeword (rounded up), the rest get the floor. This
  matches the ISO interleaving convention.

After computing ECC:
```
-- Block layout: each block i has data[i] and ecc[i]
-- Interleave for placement:
interleaved = []
for position in 0..max_data_per_block:
    for block in 0..num_blocks:
        if position < len(data[block]):
            interleaved.append(data[block][position])
for position in 0..ecc_per_block:
    for block in 0..num_blocks:
        interleaved.append(ecc[block][position])
```

### RS encoding algorithm

Each block's ECC codewords are computed as the polynomial remainder:

```
Given:
  D(x) = data polynomial (data codewords as coefficients, highest degree first)
  G(x) = generator polynomial for n_ecc ECC codewords (degree n_ecc)

Compute:
  R(x) = (D(x) × x^n_ecc) mod G(x)

The n_ecc coefficients of R(x) are the ECC codewords for this block.
```

Linear-feedback implementation (using the LFSR approach, identical to QR
but with the 0x12D GF table):

```python
def rs_encode_block(data: list[int], n_ecc: int, gen_poly: list[int]) -> list[int]:
    # gen_poly[0] is the leading coefficient (degree n_ecc-1 term)
    # gen_poly[n_ecc] is the constant term
    ecc = [0] * n_ecc
    for byte in data:
        feedback = byte ^ ecc[0]
        # Shift register left
        for i in range(n_ecc - 1):
            ecc[i] = ecc[i + 1] ^ gf_mul_0x12D(gen_poly[i + 1], feedback)
        ecc[n_ecc - 1] = gf_mul_0x12D(gen_poly[n_ecc], feedback)
    return ecc
```

### Generator polynomials for Data Matrix

These must be embedded as constants. They are computed as
`g(x) = ∏(x + α^k)` for k = 1 to n_ecc, over GF(256)/0x12D.

Key generator polynomials (hex coefficients, highest degree first, including
the implicit leading 1):

```
n_ecc = 5:
  01 0F 36 78 40 A9  (degree-5 polynomial, 6 coefficients incl. leading 1)

n_ecc = 7:
  01 4F A0 C3 09 39 F5 82

n_ecc = 10:
  01 4B C1 E0 26 D6 43 2E 72 B8 A3

n_ecc = 12:
  01 E3 DB 1C 71 1F C5 D4 2C B0 83 EB 12

n_ecc = 14:
  01 4D 91 2A E3 EA F3 5D 1B 7B A1 2F 15 15 06

n_ecc = 18:
  01 8F E1 93 61 BA B1 BD 90 5B A5 F3 0E A6 4A B4 3A B0 3A

n_ecc = 20:
  01 1D C8 C1 0C A1 29 2D 47 E4 1B E8 6D B1 73 6C 1B 97 EA 3C DB

n_ecc = 24:
  01 DA 31 32 04 AE B6 D7 E2 21 93 94 D8 29 B8 3B 83 B7 4D 83 9A DA D4 22

n_ecc = 28:
  01 36 92 5D 52 40 B5 D1 57 A2 E3 CD 28 39 BD 19 C8 A0 FE 87 49 F1 2A 37
     A7 B9 32 C5 3D

n_ecc = 36:
  01 DA 9E 89 5B 1E C9 06 77 FD 27 F0 81 C5 60 4D CF E2 B4 3E 3D 8A 51 A9
     DE DC 5F F3 4F 79 A6 AB 54 4F E3

n_ecc = 42:
  (see ISO/IEC 16022 Annex A for full tables)

n_ecc = 48:
  (see ISO/IEC 16022 Annex A)

... (larger block sizes: embed from ISO Annex A)
```

Implementers should embed all required generator polynomials from ISO
Annex A as compile-time constants keyed by ECC codeword count.

---

## Module Placement — The Utah Algorithm

The module placement algorithm is the most distinctive part of Data Matrix
encoding. It is called the **"Utah" algorithm** because the 8-module patterns
used to place each codeword vaguely resemble the US state of Utah — a
rectangular shape with a notch cut from the top-right corner.

### Conceptual overview

The placement algorithm works on the **logical data matrix** — a virtual grid
with rows numbered 0 to (data_rows-1) and columns 0 to (data_cols-1), where
data_rows and data_cols are the interior dimensions after stripping the outer
border. For multi-region symbols, this virtual grid spans all data regions
concatenated.

The algorithm walks a reference position (row, col) through this logical
grid in a specific diagonal pattern, placing 8 bits of each codeword at
8 fixed offsets relative to that reference position. After each codeword,
the reference moves up-right by 2 columns and 2 rows.

### The 8-module "Utah" placement shape

For a codeword with MSB as bit 8 and LSB as bit 1, the 8 modules are placed
at the following offsets relative to reference position (row, col):

```
Standard "Utah" placement (the common case for most positions):

    col-2 col-1  col   col+1
row-2: [bit8]
row-1: [bit7] [bit6]
row  : [bit5] [bit4] [bit3]
                      [bit2] [bit1]  ← (row+1, col+1) and (row, col+1) NOT here
```

Wait — the actual layout is subtler. The Utah shape is:

```
Standard "Utah" shape for codeword at reference (row, col):

  Offset    Row      Col
  ------    ---      ---
  bit 1:   row-2    col-1
  bit 2:   row-2    col
  bit 3:   row-1    col-2
  bit 4:   row-1    col-1
  bit 5:   row-1    col
  bit 6:   row      col-2
  bit 7:   row      col-1
  bit 8:   row      col
```

Visualized as a grid (X = bit placed here, . = empty):

```
col:   c-2  c-1   c
row-2:  .   [1]  [2]
row-1: [3]  [4]  [5]
row  : [6]  [7]  [8]
```

This shape looks like the state of Utah — a rectangle (cols c-2 to c,
rows row-2 to row) with the top-left corner missing (no bit at row-2, col-2).

The numbers [1]–[8] correspond to bits 1–8 of the codeword:
- bit 8 (MSB) at (row, col)
- bit 7 at (row, col-1)
- bit 6 at (row, col-2)
- bit 5 at (row-1, col)
- bit 4 at (row-1, col-1)
- bit 3 at (row-1, col-2)
- bit 2 at (row-2, col)
- bit 1 at (row-2, col-1)

### The corner placement patterns

When the standard Utah shape would place bits outside the data area boundary,
special corner patterns handle wrapping. There are four special corner
patterns defined in ISO/IEC 16022, Section 10.

#### Corner pattern 1 (top-left wrap)

Triggered when a codeword falls at the top-left corner region.

```
Offsets for corner pattern 1:
  bit 8: (0,    nCols-2)    -- top row, near-right column
  bit 7: (0,    nCols-1)    -- top row, rightmost column
  bit 6: (1,    0)          -- second row, leftmost column
  bit 5: (2,    0)          -- third row, leftmost column
  bit 4: (nRows-2, 0)       -- near-bottom row, leftmost column
  bit 3: (nRows-1, 0)       -- bottom row, leftmost column
  bit 2: (nRows-1, 1)       -- bottom row, second column
  bit 1: (nRows-1, 2)       -- bottom row, third column
```

#### Corner pattern 2 (top-right wrap)

Triggered when the reference position falls in the top-right corner.

```
Offsets for corner pattern 2:
  bit 8: (0,    nCols-2)
  bit 7: (0,    nCols-1)
  bit 6: (1,    nCols-1)
  bit 5: (2,    nCols-1)
  bit 4: (nRows-1, 0)
  bit 3: (nRows-1, 1)
  bit 2: (nRows-1, 2)
  bit 1: (nRows-1, 3)
```

#### Corner pattern 3 (bottom-left wrap)

Triggered at the bottom-left corner region.

```
Offsets for corner pattern 3:
  bit 8: (0,    nCols-1)
  bit 7: (1,    0)
  bit 6: (2,    0)
  bit 5: (nRows-2, 0)
  bit 4: (nRows-1, 0)
  bit 3: (nRows-1, 1)
  bit 2: (nRows-1, 2)
  bit 1: (nRows-1, 3)
```

#### Corner pattern 4 (right-edge wrap for odd-size matrices)

A special-case 4th corner used only in matrices where (nRows mod 2 ≠ 0)
and (nCols mod 2 ≠ 0) — rectangular symbols and some extended square sizes.

```
Offsets for corner pattern 4:
  bit 8: (nRows-3, nCols-1)
  bit 7: (nRows-2, nCols-1)
  bit 6: (nRows-1, nCols-3)
  bit 5: (nRows-1, nCols-2)
  bit 4: (nRows-1, nCols-1)
  bit 3: (0,       0)
  bit 2: (1,       0)
  bit 1: (2,       0)
```

### Boundary wrapping

Even in the non-corner cases, the Utah shape may partially extend beyond
the grid boundaries. When any bit's computed (row, col) is out of bounds,
apply the following wrap rules:

```
If row < 0 and col >= 0:
    row += nRows
    col -= 4

If col < 0 and row >= 0:
    col += nCols
    row -= 4

If row < 0 and col == 0:
    row = 1
    col = 3

If row < 0 and col == nCols:
    row = 0
    col = col - 2
```

These rules handle the diagonal scanning when the reference position is near
the top or left edges.

### The placement algorithm (complete pseudocode)

```
-- Initialization
nRows = data_rows    -- logical data matrix height (excl. outer border)
nCols = data_cols    -- logical data matrix width  (excl. outer border)
grid  = new bool[nRows][nCols]   -- all false
used  = new bool[nRows][nCols]   -- tracks assigned modules

-- Precompute: is_assigned(r, c) = grid module already set (for special patterns)

-- Run the Utah algorithm
codeword_index = 0
row = 4
col = 0

while true:
    -- Special case 1: top-left corner fill for odd-position
    if row == nRows and col == 0 and (nRows mod 4 == 0 or nCols mod 4 == 0):
        place_corner1(codewords[codeword_index++], nRows, nCols, grid)
    
    -- Special case 2: top-right corner fill
    if row == (nRows - 2) and col == 0 and nCols mod 4 != 0:
        place_corner2(codewords[codeword_index++], nRows, nCols, grid)
    
    -- Special case 3: top-right corner fill (different condition)
    if row == (nRows - 2) and col == 0 and nCols mod 8 == 4:
        place_corner3(codewords[codeword_index++], nRows, nCols, grid)
    
    -- Special case 4: bottom-left / odd rectangle
    if row == (nRows + 4) and col == 2 and nCols mod 8 == 0:
        place_corner4(codewords[codeword_index++], nRows, nCols, grid)
    
    -- Standard diagonal traversal (upward-right)
    loop:
        if row >= 0 and col < nCols and not used[row][col]:
            place_utah(codewords[codeword_index++], row, col, nRows, nCols, grid, used)
        row -= 2
        col += 2
        if row < 0 or col >= nCols: break
    
    -- Step to next diagonal start position
    row += 1
    col += 3
    
    loop:
        if row < nRows and col >= 0 and not used[row][col]:
            place_utah(codewords[codeword_index++], row, col, nRows, nCols, grid, used)
        row += 2
        col -= 2
        if row >= nRows or col < 0: break
    
    -- Step to next diagonal start position
    row += 3
    col += 1
    
    -- Termination: all codewords placed
    if row >= nRows and col >= nCols: break
    if codeword_index >= total_codewords: break

-- Fill any remaining unset modules with 1 (dark)
-- (These are the alignment/fill modules at the lower-right corner that
-- exist in some symbol sizes to complete the grid)
for r in 0..nRows:
    for c in 0..nCols:
        if not used[r][c]:
            grid[r][c] = (r + c) mod 2 == 1   -- fill pattern: dark at odd positions
```

The fill pattern at the end (`(r + c) mod 2 == 1`) matches the ISO
specification's "right and bottom fill" rule for symbols whose data area is
not evenly divisible by the codeword placement scheme.

### Placing a single codeword (place_utah)

```
procedure place_utah(codeword: byte, row: int, col: int,
                     nRows: int, nCols: int,
                     grid: bool[][], used: bool[][]):

    -- Compute the 8 module positions using the standard Utah offsets
    positions = [
        (row,   col   ),   -- bit 8 (MSB)
        (row,   col-1 ),   -- bit 7
        (row,   col-2 ),   -- bit 6
        (row-1, col   ),   -- bit 5
        (row-1, col-1 ),   -- bit 4
        (row-1, col-2 ),   -- bit 3
        (row-2, col   ),   -- bit 2
        (row-2, col-1 ),   -- bit 1 (LSB)
    ]
    
    bits = [
        (codeword >> 7) & 1,  -- bit 8
        (codeword >> 6) & 1,  -- bit 7
        (codeword >> 5) & 1,  -- bit 6
        (codeword >> 4) & 1,  -- bit 5
        (codeword >> 3) & 1,  -- bit 4
        (codeword >> 2) & 1,  -- bit 3
        (codeword >> 1) & 1,  -- bit 2
        (codeword >> 0) & 1,  -- bit 1
    ]
    
    for i in 0..8:
        (r, c) = apply_boundary_wrap(positions[i], nRows, nCols)
        if 0 <= r < nRows and 0 <= c < nCols and not used[r][c]:
            grid[r][c] = bits[i] == 1
            used[r][c] = true
```

### Mapping logical to physical coordinates

After the Utah algorithm fills the logical data matrix `grid[nRows][nCols]`,
map each (r, c) to physical symbol coordinates:

```
For a symbol with rr × rc data regions, each of size (rh × rw) logical:

physical_row(r, c) =
    (r / rh) * (rh + 2) + (r mod rh) + 1   -- +1 for outer finder border

physical_col(r, c) =
    (c / rw) * (rw + 2) + (c mod rw) + 1   -- +1 for outer finder border
```

The `+2` accounts for the 2-module alignment border between regions.
The `+1` accounts for the 1-module outer border (finder + timing).

For single-region symbols (1×1), this simplifies to:

```
physical_row = r + 1
physical_col = c + 1
```

### No masking

Data Matrix ECC200 does **not** apply masking after module placement. The
diagonal Utah placement pattern inherently distributes bits across the symbol
in a way that avoids degenerate patterns. The encoded data is placed once and
the symbol is complete. This is in contrast to QR Code, which requires
evaluating all 8 mask patterns and picking the best one.

---

## Complete Encoding Algorithm

The full pipeline for encoding a given input string into a Data Matrix ECC200
symbol:

```
1. ENCODE DATA
   a. If all input characters are ASCII (0–127), use ASCII mode.
      For each character:
        - If current and next characters are both ASCII digits (0x30–0x39):
          emit codeword 130 + (d1*10 + d2); advance two positions.
        - Else: emit codeword ASCII_value + 1; advance one position.
   b. Characters 128–255: emit 235 (UPPER_SHIFT) then (ASCII_value - 127).
   c. Collect encoded codewords into a list.

2. CHOOSE SYMBOL SIZE
   a. Count encoded codewords (length of list from step 1).
   b. Iterate over symbol sizes in ascending order (smallest first).
   c. Select the first symbol where data_codewords ≥ encoded codeword count.
   d. If none fits, return DataMatrixError::InputTooLong.

3. PAD TO CAPACITY
   a. If encoded_count < data_codewords:
      - Append the pad codeword 129.
      - For each subsequent position k (1-indexed from start of codeword stream)
        where padding is needed, compute scrambled_pad and append.
   b. Result is a list of exactly data_codewords codewords.

4. SPLIT INTO RS BLOCKS
   a. Look up num_blocks and ecc_per_block for the chosen symbol.
   b. Split data codewords evenly across blocks (earlier blocks get +1 if uneven).
   c. Each block: data_block[i] = data[i*block_size .. (i+1)*block_size].

5. COMPUTE ECC PER BLOCK
   a. For each block, compute RS ECC using the generator polynomial for
      ecc_per_block, over GF(256)/0x12D.
   b. ecc_block[i] = rs_encode_block(data_block[i], ecc_per_block, gen_poly).

6. INTERLEAVE BLOCKS
   a. Interleave data codewords across blocks:
      for pos in 0..max_data_per_block:
          for blk in 0..num_blocks:
              if pos < len(data_block[blk]):
                  interleaved.append(data_block[blk][pos])
   b. Interleave ECC codewords across blocks:
      for pos in 0..ecc_per_block:
          for blk in 0..num_blocks:
              interleaved.append(ecc_block[blk][pos])
   c. interleaved now contains total_codewords = data_codewords + ecc_codewords values.

7. INITIALIZE PHYSICAL MODULE GRID
   a. Create grid of size symbol_rows × symbol_cols, all light (0).
   b. Place finder pattern:
      - Left column (col 0): all modules dark.
      - Bottom row (row symbol_rows-1): all modules dark.
   c. Place timing pattern:
      - Top row (row 0): alternate dark/light starting from col 0 (dark).
      - Right column (col symbol_cols-1): alternate dark/light starting from row 0 (dark).
   d. For multi-region symbols, place alignment borders:
      - For each horizontal alignment border between region rows r and r+1:
        Rows at position: 1 + r*(region_height+2) + region_height   (the row after last data row of region r)
        Row AB+0: all dark.
        Row AB+1: alternating dark/light.
      - For each vertical alignment border between region cols c and c+1:
        Cols at position: 1 + c*(region_width+2) + region_width
        Col AB+0: all dark.
        Col AB+1: alternating dark/light.

8. RUN UTAH PLACEMENT ALGORITHM
   a. Compute logical data matrix size:
      nRows = (symbol_rows - 2 - 2*(rr-1)) = total data area height / rr * rr
      nCols = (symbol_cols - 2 - 2*(rc-1)) = total data area width  / rc * rc
      (More precisely: nRows = rr * region_data_height, nCols = rc * region_data_width)
   b. Run the Utah algorithm on interleaved[], filling logical grid[nRows][nCols].
   c. Map each logical (r, c) to physical (physical_row, physical_col) and set
      the corresponding module in the physical grid.

9. OUTPUT
   a. Return the physical ModuleGrid (symbol_rows × symbol_cols).
   b. No masking step — the grid is final.

10. LAYOUT + PAINT (optional)
    Pass the ModuleGrid to barcode-2d::layout(grid, config) → PaintScene.
    The quiet zone (1 module minimum) is added at this stage by the layout step.
```

---

## Implementation Notes

### Tables to embed as constants

The following tables must be embedded in each implementation. Do not compute
them at runtime.

#### 1. Symbol size table

For each of the 30 square and 6 rectangular symbol sizes:

```
{
  symbol_rows,        -- total rows including outer border
  symbol_cols,        -- total cols including outer border
  region_rows,        -- rr: number of data region rows
  region_cols,        -- rc: number of data region cols
  data_region_height, -- interior height of each data region
  data_region_width,  -- interior width of each data region
  data_codewords,     -- total data codeword capacity
  ecc_codewords,      -- total ECC codeword count
  num_blocks,         -- number of interleaved RS blocks
  ecc_per_block,      -- ECC codewords per block (same for all blocks)
}
```

#### 2. Generator polynomials for all required ECC lengths

All unique ecc_per_block values that appear in the symbol size table must
have a precomputed generator polynomial embedded. From the table above, the
required lengths are:

```
5, 7, 10, 11, 12, 14, 17, 18, 20, 21, 24, 28, 36, 42, 48, 56, 62, 68
```

Each generator polynomial is a list of `n_ecc + 1` bytes (including the
implicit leading coefficient 1). Embed from ISO/IEC 16022, Annex A, using
GF(256)/0x12D.

#### 3. GF(256)/0x12D exp and log tables

```
gf_exp[256]:  gf_exp[i] = α^i mod 0x12D  (for i = 0..254; gf_exp[255] = gf_exp[0] = 1)
gf_log[256]:  gf_log[v] = k such that α^k = v  (for v = 1..255; gf_log[0] = undefined)
```

These two tables (512 bytes total) are the foundation of all GF(256)
arithmetic. Use them for the RS encoding inner loop.

### Implementing GF multiplication

```
gf_mul(a: byte, b: byte) -> byte:
    if a == 0 or b == 0: return 0
    return gf_exp[(gf_log[a] + gf_log[b]) mod 255]
```

This log/exp trick turns a field multiplication (which would require
polynomial reduction) into two table lookups and an addition modulo 255.
It is the standard implementation for GF(256) multiplication.

### Corner pattern trigger conditions

The four corner patterns in the Utah algorithm are triggered based on
specific (row, col) reference positions and symbol dimensions. These
conditions are precise and must be implemented exactly. Incorrect corner
handling produces an invalid symbol that may encode incorrect data or be
unreadable by scanners.

The reference implementation in ISO/IEC 16022, Annex F, defines the exact
conditions in pseudocode. Implementors should verify their Utah algorithm
against the ISO worked example for the 10×10 symbol (the simplest case
that exercises most of the algorithm's behavior).

The easiest verification strategy:

1. Encode "A" → should produce a 10×10 symbol.
2. The expected hexadecimal module grid (row by row, MSB to LSB) for "A"
   in a 10×10 Data Matrix is published in Annex F of ISO/IEC 16022:2006.
   Compare module-for-module.

### Module numbering convention

The physical module grid uses the convention:
- `grid[0][0]` = top-left corner of the symbol (inside quiet zone)
- Row 0 = topmost row (the timing row)
- Column 0 = leftmost column (the L-finder column)
- `grid[R-1][C-1]` = bottom-right corner

This matches the visual orientation when printed: row increases downward,
column increases rightward.

### Rectangular symbol handling

Rectangular symbols follow all the same rules as square symbols. The Utah
algorithm operates identically; the only difference is `nRows != nCols`.
The boundary wrap conditions in the algorithm handle non-square logical
matrices correctly.

For rectangular symbols with a single data region (8×18, 12×26, 16×36):
`nRows` and `nCols` differ but both are small (≤ 14 or ≤ 22), making the
algorithm easy to verify visually.

### Mode selection for v0.1.0

The v0.1.0 implementation uses only ASCII mode. Mode selection for C40,
Text, EDIFACT, X12, and Base256 is deferred to v0.2.0. The full multi-mode
optimizer for v0.3.0 would segment the input into runs of different
character classes and choose the most compact mode for each segment.

---

## Visualization Annotations

The encoder should optionally produce an annotated module grid for the
visualizer. Each module records its role:

| Color (suggested) | Role |
|-------------------|------|
| Deep green        | finder (L-bar: left column + bottom row) |
| Green-grey        | timing (top row + right column alternating) |
| Teal              | alignment border (inter-region, multi-region symbols only) |
| Black/white       | data module |
| Gold/yellow       | ECC module |
| Blue              | pad module (scrambled pad codeword bits) |

The visualizer can distinguish which codeword each module belongs to by
annotating `codeword_index` (0-indexed within the interleaved stream) and
`bit_index` (0–7, MSB=7) per module.

---

## Error Types

```
DataMatrixError::InputTooLong
    -- The encoded codeword count exceeds the 144×144 symbol's data capacity.
    -- Message should include the encoded codeword count and the maximum (1558).

DataMatrixError::InvalidInput
    -- Input contains byte values that cannot be encoded in the selected mode.
    -- In ASCII mode, all byte values 0–255 are technically encodable
    -- (values ≥ 128 use UPPER_SHIFT), so this error occurs only in
    -- restricted modes (C40/Text/X12/EDIFACT) with out-of-range characters.

DataMatrixError::SymbolTooSmall
    -- The caller forced a specific symbol size via DataMatrixOptions.min_size
    -- but the input is too large for that size.
    -- (v0.2.0 when forced-size option is implemented)
```

---

## Public API

```
encode(input: string | bytes, options?: DataMatrixOptions) → ModuleGrid
  -- Encode input to a Data Matrix ECC200 module grid.
  -- Selects the smallest symbol that fits the input.
  -- Raises DataMatrixError::InputTooLong if input exceeds 144×144 capacity.
  -- Raises DataMatrixError::InvalidInput if input contains unsupported characters.

layout(grid: ModuleGrid, config?: Barcode2DLayoutConfig) → PaintScene
  -- Translate a ModuleGrid into a pixel-resolved PaintScene (P2D00).
  -- Delegates to barcode-2d::layout(). data-matrix does not implement this itself.

encode_and_layout(
    input: string | bytes,
    options?: DataMatrixOptions,
    config?: Barcode2DLayoutConfig
) → PaintScene
  -- Convenience: encode + layout in one call.

render_svg(
    input: string | bytes,
    options?: DataMatrixOptions,
    config?: Barcode2DLayoutConfig
) → string
  -- Convenience: encode + layout + paint-vm-svg backend → SVG string.

explain(input: string | bytes, options?: DataMatrixOptions) → AnnotatedModuleGrid
  -- Encode with full per-module role and codeword annotations (for visualizers).

DataMatrixOptions {
  min_size?: SymbolSize
    -- Force at least this symbol size. If the encoded data fits in a smaller
    -- symbol, the encoder will use the forced minimum size and pad to fill.
    -- If the data does not fit, raise DataMatrixError::SymbolTooSmall.

  shape?: SymbolShape
    -- "Square" (default): select from square symbol sizes only.
    -- "Rectangular": prefer rectangular symbol sizes over square.
    -- "Any": try both and pick the smallest.

  mode?: EncodingMode
    -- Force encoding mode. Default: ASCII.
    -- (C40 | Text | X12 | EDIFACT | Base256 are v0.2.0)
}

SymbolSize =
  | Square10x10   | Square12x12   | Square14x14   | Square16x16
  | Square18x18   | Square20x20   | Square22x22   | Square24x24
  | Square26x26   | Square32x32   | Square36x36   | Square40x40
  | Square44x44   | Square48x48   | Square52x52   | Square64x64
  | Square72x72   | Square80x80   | Square88x88   | Square96x96
  | Square104x104 | Square120x120 | Square132x132 | Square144x144
  | Rect8x18      | Rect8x32      | Rect12x26
  | Rect12x36     | Rect16x36     | Rect16x48

SymbolShape = Square | Rectangular | Any

EncodingMode = ASCII | C40 | Text | X12 | EDIFACT | Base256
```

---

## Package Matrix

| Language   | Directory                                   | Depends on                    |
|------------|---------------------------------------------|-------------------------------|
| Rust       | `code/packages/rust/data-matrix/`           | barcode-2d, gf256, reed-solomon |
| TypeScript | `code/packages/typescript/data-matrix/`     | barcode-2d, gf256, reed-solomon |
| Python     | `code/packages/python/data-matrix/`         | barcode-2d, gf256, reed-solomon |
| Go         | `code/packages/go/data-matrix/`             | barcode-2d, gf256, reed-solomon |
| Ruby       | `code/packages/ruby/data_matrix/`           | barcode-2d, gf256, reed-solomon |
| Elixir     | `code/packages/elixir/data_matrix/`         | barcode_2d, gf256, reed_solomon |
| Lua        | `code/packages/lua/data-matrix/`            | barcode-2d, gf256, reed-solomon |
| Perl       | `code/packages/perl/data-matrix/`           | barcode-2d, gf256, reed-solomon |
| Swift      | `code/packages/swift/DataMatrix/`           | Barcode2D, GF256, ReedSolomon  |
| C#         | `code/packages/csharp/DataMatrix/`          | Barcode2D, GF256, ReedSolomon  |
| F#         | `code/packages/fsharp/DataMatrix/`          | Barcode2D, GF256, ReedSolomon  |
| Kotlin     | `code/packages/kotlin/data-matrix/`         | barcode-2d, gf256, reed-solomon |
| Java       | `code/packages/java/data-matrix/`           | barcode-2d, gf256, reed-solomon |
| Dart       | `code/packages/dart/data_matrix/`           | barcode_2d, gf256, reed_solomon |
| Haskell    | `code/packages/haskell/data-matrix/`        | barcode-2d, gf256, reed-solomon |

---

## Test Strategy

### Unit tests

#### 1. GF(256)/0x12D arithmetic

Verify the exp and log tables and multiplication for the 0x12D field:

```
gf_exp[0]   == 1      (α^0)
gf_exp[1]   == 2      (α^1)
gf_exp[7]   == 128    (α^7 = 0x80)
gf_exp[8]   == 0x2D   (α^8 reduced: 0x80<<1 XOR 0x12D = 0x2D)
gf_exp[254] == ???     (compute and verify against ISO Annex D)
gf_mul(2, 2)   == 4   (α^1 * α^1 = α^2 = 4)
gf_mul(0x80, 2) == 0x2D  (α^7 * α^1 = α^8 = 0x2D)
gf_mul(0, 0xFF) == 0    (zero absorbs multiplication)
```

#### 2. ASCII encoding

```
encode_ascii("A")    → [66]          (65 + 1 = 66)
encode_ascii(" ")    → [33]          (32 + 1 = 33)
encode_ascii("12")   → [142]         (130 + 12 = 142, digit pair)
encode_ascii("1234") → [142, 174]    (2 digit pairs)
encode_ascii("1A")   → [50, 66]      (digit, then letter — no pair)
encode_ascii("00")   → [130]         (130 + 0 = 130)
encode_ascii("99")   → [229]         (130 + 99 = 229)
```

#### 3. Pad codewords

For a 10×10 symbol (data_codewords = 3), encoding "A" produces [66].
After padding to 3 codewords:

```
Codeword sequence before padding: [66]
First pad at position k=2: codeword = 129
Second pad at position k=3: scrambled_pad = (129 + (149*3 mod 253) + 1) mod 254
                                           = (129 + (447 mod 253) + 1) mod 254
                                           = (129 + 194 + 1) mod 254
                                           = 324 mod 254 = 70
Final padded sequence: [66, 129, 70]
```

Verify this against the ISO-16022 worked example.

#### 4. RS encoding

For the 10×10 symbol (ecc_per_block = 5, generator polynomial for n=5):
Data = [66, 129, 70] (encoded "A" + padding).

Expected ECC bytes: compute using RS over GF(256)/0x12D with the n=5
generator polynomial. Cross-check against the ISO/IEC 16022 Annex F
worked example for encoding "A".

#### 5. Module placement (Utah algorithm)

For the 10×10 symbol, the complete module placement of "A" is given in
ISO/IEC 16022, Annex F. The expected physical module grid (10 rows × 10
columns) is reproduced there as a binary matrix. Compare module-by-module.

#### 6. Symbol border

For any symbol:

```
-- Finder L-bar: all left column and bottom row modules are dark
for r in 0..symbol_rows: assert grid[r][0] == dark
for c in 0..symbol_cols: assert grid[symbol_rows-1][c] == dark

-- Timing clock: top row alternating, starting dark
for c in 0..symbol_cols:
    expected = (c mod 2 == 0) ? dark : light
    assert grid[0][c] == expected

-- Right column alternating, starting dark
for r in 0..symbol_rows:
    expected = (r mod 2 == 0) ? dark : light
    assert grid[r][symbol_cols-1] == expected

-- Corner (0,0) must be dark (L-bar meets timing)
assert grid[0][0] == dark
```

### Integration tests

#### 1. Encode "A" → 10×10 symbol

The smallest possible Data Matrix ECC200 symbol. Full pipeline:
- Input: single character "A" (ASCII 65)
- Expected: 10×10 symbol
- Verify: module grid matches ISO Annex F worked example bit-for-bit

#### 2. Encode "1234" → 10×10 symbol

Digit-pair encoding test:
- "12" → codeword 142, "34" → codeword 174
- 2 data codewords; fits in 10×10 (capacity 3)
- Verify: correct codewords, correct ECC, correct module grid

#### 3. Encode "Hello World" → 16×16 symbol

Mixed case, space:
- 11 characters → 11 ASCII codewords → needs ≥ 11 data codewords
- 16×16 has 12 data codewords, so 11 + 1 pad = 12 codewords
- Verify: symbol is 16×16, correct border, passes scanner test

#### 4. Encode a 44-character string → 26×26 symbol

Maximum single-region square symbol:
- Verify: data regions = 1×1, no alignment borders in interior

#### 5. Encode a string requiring 32×32 → multi-region

Choose a string requiring 45–62 codewords:
- Verify: data regions = 2×2, alignment borders present at correct positions
- Verify: physical module grid has correct alignment border pattern

#### 6. Scanner verification

Render the symbol as SVG. Use `zxing-cpp` or `libdmtx` to decode the SVG.
Compare decoded string to original input. Run for all test cases.

### Cross-language verification

All 15 language implementations must produce **identical ModuleGrid outputs**
for the same input. Use a JSON format for the test vectors:

```json
{
  "input": "A",
  "symbol": "10x10",
  "grid": [
    [1,1,0,1,0,1,0,1,0,1],
    [1,...],
    ...
  ]
}
```

Generate the canonical test vectors from the ISO Annex F worked example and
from one reference implementation (e.g., libdmtx, which implements ISO/IEC
16022 faithfully). All 15 implementations must match these vectors exactly.

Suggested cross-language test corpus:

```
"A"             (10×10 — ISO worked example; minimal)
"1234"          (10×10 — digit-pair test; same symbol as "A")
"Hello World"   (16×16 — mixed ASCII)
"https://coding-adventures.dev"   (32×32 — URL, multi-region)
"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"  (26×26 — full alphanumeric)
```

All cross-language outputs must be bit-for-bit identical for each input.

---

## Dependency Stack

```
paint-metal (P2D02)    paint-vm-svg    paint-vm-canvas
      └──────────────────┬──────────────────┘
                         │
                  paint-vm (P2D01)
                         │
              paint-instructions (P2D00)
                         │
                     barcode-2d          MA02 reed-solomon (b=1, 0x12D)
                         │                        │
                    data-matrix ─────────────────┘
                                            MA01 gf256 (0x12D)
```

`data-matrix` depends on:

- `barcode-2d`: for the `ModuleGrid` type and the `layout()` function.
- `reed-solomon` (MA02): for RS encoding over GF(256)/0x12D. Data Matrix
  uses the b=1 convention, which matches MA02 exactly. The only required
  configuration is the field polynomial (0x12D, not QR's 0x11D).
- `gf256` (MA01): for GF(256)/0x12D field arithmetic tables.

Unlike `qr-code`, which must implement its own RS encoder with a different
generator root convention, `data-matrix` can call MA02 directly. This is
a significant simplification.

---

## Future Extensions

- **Decoder** — A separate, considerably more complex spec involving scanner
  coordinate correction, border detection, perspective transform, and syndrome
  decoding. Not planned for the near term; a separate spec will cover it.

- **C40 / Text mode encoding** (v0.2.0) — Optimize uppercase-heavy input.
  The encoder will detect when C40 saves codewords over ASCII and
  automatically switch to C40 mode.

- **Multi-mode segmentation** (v0.3.0) — Segment input into runs of digits,
  uppercase, lowercase, binary. Choose the best mode per segment. The
  optimizer is a dynamic-programming problem over the mode-codeword cost
  table.

- **Extended Channel Interpretation (ECI)** — Embed a charset identifier
  (e.g., ECI 26 = UTF-8) before the data for scanners that do not default
  to UTF-8.

- **Structured Append** — Split a large message across up to 16 Data Matrix
  symbols, each carrying a segment index and a file identification character.
  The reader reassembles the segments in order.

- **Rectangular symbol auto-selection** — When `shape = Any`, the encoder
  finds the smallest symbol across both square and rectangular options. For
  short strings like lot codes and part numbers, a rectangular symbol often
  fits the aspect ratio of the print area better.

- **Direct mark / dot-peen rendering** — Physical Data Matrix marks (etched
  or dot-peened on metal) use circles or dots instead of squares. The render
  pipeline supports this via a `dot_style: Circular` option in
  `Barcode2DLayoutConfig`, which switches the PaintRect instructions to
  PaintEllipse. This is a paint-layer concern; the encoding is unchanged.

- **GS1 Data Matrix** — A standardized use of Data Matrix for product and
  trade item identification. The input uses Application Identifiers (AIs)
  from GS1-128. The encoding is standard Data Matrix; the difference is a
  mandatory `FNC1` character (ASCII 29, Group Separator) at the start of the
  message. A `gs1_mode: bool` option in `DataMatrixOptions` prepends this
  character automatically.

- **Visualizer integration** — Interactive drill-down showing which codeword,
  RS block, and generator polynomial produced each module. Same architecture
  as the QR visualizer; just different role annotations.
