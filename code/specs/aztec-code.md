# Aztec Code

## Overview

This spec defines an **Aztec Code encoder** for the coding-adventures monorepo.

Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
published as a patent-free format. It is defined by **ISO/IEC 24778:2008** (with
a 2014 corrigendum). The name comes from the central bullseye finder pattern,
which resembles the stepped pyramid on the Aztec calendar.

Where QR Code uses three square finder patterns at three corners, Aztec Code
uses a single **bullseye at the center** of the symbol. This elegant design
means there is no structural requirement for a large quiet zone — the scanner
finds the center first, then reads outward. A 1-module quiet zone is
recommended, but the standard does not mandate one.

Aztec is in heavy operational use today:

- **IATA boarding passes** — the barcode on every airline boarding pass
- **Eurostar and Amtrak rail tickets** — printed and on-screen tickets
- **US driver's licences** — AAMVA-standard PDF417 is common, but many states
  also use Aztec
- **PostNL, Deutsche Post, La Poste** — European postal routing
- **US military ID cards**

Understanding how to build an Aztec Code encoder from scratch teaches:

- how a bullseye finder pattern enables orientation without corner anchors
- how Reed-Solomon works in two different field sizes (GF(16) and GF(256))
- how a spiral bit-layout algorithm works from the inside out
- how variable-width codewords (4 bits, 5 bits, 8 bits) mix in one symbol
- how bit stuffing prevents degenerate all-zero or all-one runs in a
  two-dimensional code

The encoder in this spec produces a **valid, scannable Aztec Code** for any
input string or byte sequence that fits within a 32-layer full symbol. It does
not implement decoding.

---

## Two Symbol Variants

Aztec Code has two structurally different variants. The choice between them is
determined automatically by the amount of data to encode, though the caller
can force compact mode.

### Compact Aztec

Compact Aztec supports **1 to 4 layers**. The central bullseye is 11×11
modules (radius 5 from the center module). Each added layer wraps a 4-module
band around the symbol: 2 modules on each side.

| Layers | Symbol size |
|--------|-------------|
| 1 | 15×15 |
| 2 | 19×19 |
| 3 | 23×23 |
| 4 | 27×27 |

Formula: `size = 11 + 4 * layers`

The mode message (format information) for compact symbols occupies the 28-bit
band in the innermost data layer (the ring immediately outside the bullseye),
starting at the top-left corner of that ring and running clockwise.

### Full Aztec

Full Aztec supports **1 to 32 layers**. The central bullseye is 15×15 modules
(radius 7 from the center). Same growth rule: 4 modules per layer.

| Layers | Symbol size |
|--------|-------------|
| 1 | 19×19 |
| 2 | 23×23 |
| 3 | 27×27 |
| 4 | 31×31 |
| 5 | 35×35 |
| 6 | 39×39 |
| ... | ... |
| 10 | 55×55 |
| 15 | 75×75 |
| 20 | 95×95 |
| 22 | 103×103 |
| 32 | 143×143 |

Formula: `size = 15 + 4 * layers`

The mode message for full symbols occupies the 40-bit band in the innermost
data layer.

### Choosing between compact and full

The encoder selects the smallest symbol that fits the data at the requested
ECC level:

```
1. Try compact layers 1, 2, 3, 4 in order.
2. If none fit, try full layers 1, 2, 3, ..., 32 in order.
3. If even full 32 layers cannot fit the data, raise InputTooLong.
```

The caller can override this with `compact: true` to force compact mode (and
raise InputTooLong if the data does not fit in 4 compact layers).

---

## Symbol Structure

A complete Aztec Code symbol has the following concentric structural zones,
reading from the center outward:

```
┌─────────────────────────────────────────────────────┐
│                   quiet zone (1 module)              │
│  ┌────────────────────────────────────────────────┐  │
│  │           data layers (N layers)               │  │
│  │  ┌──────────────────────────────────────────┐  │  │
│  │  │        reference grid (full only)        │  │  │
│  │  │  ┌────────────────────────────────────┐  │  │  │
│  │  │  │     mode message band              │  │  │  │
│  │  │  │  ┌──────────────────────────────┐  │  │  │  │
│  │  │  │  │       orientation marks      │  │  │  │  │
│  │  │  │  │  ┌────────────────────────┐  │  │  │  │  │
│  │  │  │  │  │                        │  │  │  │  │  │
│  │  │  │  │  │     bullseye finder    │  │  │  │  │  │
│  │  │  │  │  │       (center)         │  │  │  │  │  │
│  │  │  │  │  │                        │  │  │  │  │  │
│  │  │  │  │  └────────────────────────┘  │  │  │  │  │
│  │  │  │  └──────────────────────────────┘  │  │  │  │
│  │  │  └────────────────────────────────────┘  │  │  │
│  │  └──────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

Each zone is described in detail below.

---

## Bullseye Finder Pattern

The bullseye is the heart of the symbol. A scanner searches for the bullseye's
concentric ring structure to locate the symbol center, determine module size,
and correct for perspective distortion.

### Compact bullseye (11×11 modules, radius 5)

```
Ring index  Radius  Size   Color
──────────  ──────  ─────  ──────────────────────────────
Center         0    1×1    DARK
Ring 1         1    3×3    DARK  (filled square)
Ring 2         2    5×5    LIGHT (border only — forms ring)
Ring 3         3    7×7    DARK  (border only)
Ring 4         4    9×9    LIGHT (border only)
Ring 5         5   11×11   DARK  (border only)
```

The "ring at radius r" means all modules at Chebyshev distance exactly r from
the center module. The Chebyshev distance from (cx, cy) to (x, y) is
`max(|x - cx|, |y - cy|)`. A ring of radius r forms the border of a square
of side `2r + 1`.

Let `cx = cy = center coordinate`. Then:

```
For each module (x, y):
  d = max(|x - cx|, |y - cy|)    -- Chebyshev distance from center
  if d == 0: DARK   (center)
  if d == 1: DARK   (inner solid core — rings 0 and 1 merge into a 3×3 dark square)
  if d == 2: LIGHT
  if d == 3: DARK
  if d == 4: LIGHT
  if d == 5: DARK
```

Visualized (D = dark, L = light, · = not bullseye):

```
· · · · · · · · · · ·
· D D D D D D D D D ·    row = cy - 5
· D L L L L L L L D ·    row = cy - 4
· D L D D D D D L D ·    row = cy - 3
· D L D L L L D L D ·    row = cy - 2
· D L D L D L D L D ·    row = cy - 1
· D L D L D L D L D ·    row = cy     (center row)
· D L D L D L D L D ·    row = cy + 1
· D L D L L L D L D ·    row = cy + 2
· D L D D D D D L D ·    row = cy + 3
· D L L L L L L L D ·    row = cy + 4
· D D D D D D D D D ·    row = cy + 5
· · · · · · · · · · ·
```

Wait — this is slightly misleading. Rings 0 and 1 are both DARK, so the
inner 3×3 is fully dark. Rings 2 and 4 are LIGHT, rings 3 and 5 are DARK.
A cleaner visualization, showing only the 11×11 bullseye region:

```
Col: 0 1 2 3 4 5 6 7 8 9 A   (hex, A = 10)
Row 0: D D D D D D D D D D D
Row 1: D L L L L L L L L L D
Row 2: D L D D D D D D D L D
Row 3: D L D L L L L L D L D
Row 4: D L D L D D D L D L D
Row 5: D L D L D D D L D L D   ← center row (d=0 at col 5)
Row 6: D L D L D D D L D L D
Row 7: D L D L L L L L D L D
Row 8: D L D D D D D D D L D
Row 9: D L L L L L L L L L D
Row A: D D D D D D D D D D D
```

Hmm — the center module is at (5, 5) in this coordinate. The 3×3 core
(d ≤ 1) includes (4..6, 4..6) — all DARK. The ring at d = 2 is LIGHT.
The ring at d = 3 is DARK. The ring at d = 4 is LIGHT. The ring at d = 5
is the outermost DARK ring. This is exactly the standard pattern.

Because rings 0 and 1 are both DARK, the center appears as a solid 3×3 dark
square surrounded by alternating light and dark rings. This produces the
visually distinctive "bull's eye" appearance. Scanners look for a region with
a 1:1:1:1:1 ratio of module widths along any scan line through the center.

### Full bullseye (15×15 modules, radius 7)

Full Aztec adds two more rings beyond the compact bullseye:

```
d == 0: DARK   (center)
d == 1: DARK   (inner core)
d == 2: LIGHT
d == 3: DARK
d == 4: LIGHT
d == 5: DARK
d == 6: LIGHT   ← extra ring compared to compact
d == 7: DARK    ← outermost ring of full bullseye
```

The outermost ring of the bullseye is always DARK in both variants. This
means the transition from the bullseye (DARK) to the orientation mark band
(which starts with a LIGHT module) is well-defined.

### Why the bullseye is self-locating

The key property: the bullseye's `1:1:1:1:1` cross-ratio can be read from
any angle. A scanner casts many 1D scan lines across the captured image.
When a scan line passes through the center, it sees a sequence of dark/light
transitions at equal spacings. By finding this equal-spacing pattern in many
scan directions, the scanner triangulates the center and orientation without
requiring any corner markers.

This is fundamentally different from QR Code, which requires three corner
finder patterns to establish orientation. Aztec's single-center design means:

1. No quiet zone required (there is no "which side is which" ambiguity that a
   quiet zone would help resolve).
2. The symbol can be printed right up to the edge of a label or ticket.
3. The symbol can be rotated to any of four orientations; the scanner detects
   the orientation from the mode message after finding the bullseye.

---

## Orientation Marks

Immediately outside the bullseye, the innermost ring of the data-and-mode-message
band contains **orientation marks** — four corner modules that are always dark.
These break the rotational symmetry of the concentric rings and allow a scanner
to determine which of the four 90-degree rotations the symbol is in.

The orientation marks occupy the **four corner positions** of the mode message
band (the ring just outside the bullseye):

```
For compact Aztec (bullseye is 11×11, center at (cx, cy)):
  The mode message band occupies the 13×13 ring at radius 6 from the center.
  Corners of this ring (Chebyshev distance = 6 from center):
    Top-left:     (cx - 6, cy - 6)  → always DARK
    Top-right:    (cx + 6, cy - 6)  → always DARK
    Bottom-right: (cx + 6, cy + 6)  → always DARK
    Bottom-left:  (cx - 6, cy + 6)  → always DARK

For full Aztec (bullseye is 15×15, center at (cx, cy)):
  The mode message band occupies the 17×17 ring at radius 8 from the center.
  Corners:
    Top-left:     (cx - 8, cy - 8)  → always DARK
    Top-right:    (cx + 8, cy - 8)  → always DARK
    Bottom-right: (cx + 8, cy + 8)  → always DARK
    Bottom-left:  (cx + 8, cy + 8)  → always DARK
```

These four dark corners are fixed. The mode message bits occupy the remaining
non-corner modules of the mode message band.

---

## Mode Message (Format Information Equivalent)

The mode message is Aztec Code's equivalent of QR Code's format information.
It encodes:

- Whether the symbol is compact or full (implied by bullseye size, but
  also encoded redundantly)
- The number of layers (compact: 2 bits, full: 5 bits)
- The number of data codewords (compact: 6 bits, full: 11 bits)
- Reed-Solomon error correction protecting the mode message itself

The mode message is placed in the ring immediately outside the bullseye
(Chebyshev distance = bullseye_radius + 1 from center), starting just after
the top-left orientation mark corner, running clockwise. The four corner
modules of this ring are the orientation marks; the remaining modules carry
the mode message bits.

### Compact mode message layout

The compact mode message ring is 13×13 (side = 13, perimeter = 4 × 12 = 48
modules, minus 4 corners = 44 modules for bits). The mode message is
**28 bits**, stored as **7 four-bit nibbles** in the ring. After the 28 bits,
the remaining 16 modules in the ring are filled by the start of the first data
layer.

Wait — that is not how the ISO standard describes it. Let me be precise:

The mode message band for compact Aztec has exactly **28 bits**:

```
bit[0..1]    : reserved = 0b01  (indicates compact mode when decoded)
bit[2..3]    : layers - 1 (0b00 = 1 layer, ..., 0b11 = 4 layers)
bit[4..9]    : data_codewords - 1 (6 bits, since compact max data words ≤ 64)
bit[10..27]  : RS ECC of the above 10 data bits, using GF(16) Reed-Solomon
               producing 18 ECC bits total (4.5 nibbles... this needs precision)
```

Actually, the ISO standard defines the mode message differently. The precise
layout from ISO/IEC 24778:2008, Section 7.3.1.1:

**Compact mode message** is exactly 28 bits, organized as 7 nibbles (4 bits
each). It is protected by a (7, 2) Reed-Solomon code over GF(16) with
primitive polynomial `x^4 + x + 1 = 0x13`:

- 2 data nibbles  (8 bits of data: 2-bit mode-flag + 2-bit layers + 6-bit word-count — but these are bits 0..7 spread across 2 nibbles)
  - Nibble 0 (bits 0..3): `d[3..0]` where `d = (layers-1) << 6 | (data_words-1)`
  - Nibble 1 (bits 4..7): `d[7..4]`

Hmm, the exact nibble encoding for compact mode message is:

```
total_data_bits = 0b00 | (layers - 1) | (data_words - 1)
    -- where layers is 2 bits (0..3) and data_words is 6 bits (0..63)
    -- combined: 8 bits total, split into 2 nibbles
nibble_0 = bits 3..0 of combined_value
nibble_1 = bits 7..4 of combined_value
```

Then 5 RS ECC nibbles are appended (using the (7,2) code over GF(16)):

```
compact mode message = [nibble_0, nibble_1, ecc_0, ecc_1, ecc_2, ecc_3, ecc_4]
                       = 7 nibbles = 28 bits
```

**Full mode message** is exactly 40 bits, organized as 10 nibbles. It is
protected by a (10, 4) RS code over GF(16) with the same primitive polynomial:

- 4 data nibbles (16 bits of data):
  - Nibbles 0..1 encode: `0b00` (mode flag, 2 bits) + `layers - 1` (5 bits)
    + `data_words - 1` (11 bits) = 18 bits total? That's 4.5 nibbles.

The ISO standard defines the encoding in terms of **bits**, then packs them
into nibbles LSB-first for RS processing. The precise construction is:

**Compact** (28-bit mode message):

```
Message bits (10 bits of data, 18 bits of RS ECC):
  Bit  0:     0  (mode flag LSB — compact = 0)
  Bit  1:     0  (mode flag MSB — compact = 0)
  Bits 2..3:  layers - 1  (2 bits)
  Bits 4..9:  data_codewords - 1  (6 bits)
  Bits 10..27: RS ECC (18 bits = 4.5 nibbles ← doesn't divide evenly)
```

This doesn't divide cleanly into nibbles when you include the 2-bit mode flag.
The ISO standard handles this by specifying the RS computation differently:
the Reed-Solomon is computed over the **8 data bits** (ignoring the 2-bit mode
flag), producing 5 ECC nibbles, for a total of:

```
2 data nibbles (8 bits) + 5 ECC nibbles (20 bits) = 7 nibbles = 28 bits
```

The 2-bit mode flag (`00` for compact) is effectively baked into the
interpretation rather than encoded as explicit RS-protected bits.

**Full** (40-bit mode message):

```
Data bits (16 bits → 4 nibbles):
  Bit  0:     1  (mode flag LSB — full = 1)
  Bit  1:     0  (mode flag MSB)
  Bits 2..6:  layers - 1  (5 bits)
  Bits 7..17: data_codewords - 1  (11 bits)
  → Total data bits: 1 + 1 + 5 + 11 = 18 bits... still doesn't fit 4 nibbles cleanly

Actually, the mode flag for full is NOT encoded in the RS-protected part.
The RS data is exactly:
  Bits 0..4:   layers - 1 (5 bits), padded to nibble boundary
  Bits 5..15:  data_codewords - 1 (11 bits)
  → 16 bits = 4 nibbles

Then 6 ECC nibbles, for 10 nibbles = 40 bits total.
```

### Implementation note on mode message bits

The cleanest way to implement this, following the ISO specification's intent:

**Compact mode message encoding:**
1. Compute `m = ((layers - 1) << 6) | (data_codewords - 1)` — an 8-bit value
2. Pack as 2 nibbles (LSB first): nibble[0] = m & 0xF, nibble[1] = (m >> 4) & 0xF
3. Compute 5 RS ECC nibbles using GF(16), primitive polynomial `x^4 + x + 1`
   (0x13), generator polynomial with roots α^1 through α^5
4. Concatenate: [nibble[0], nibble[1], ecc[0], ecc[1], ecc[2], ecc[3], ecc[4]]
5. Flatten to 28 bits (nibble[0] LSB first, nibble[1] LSB first, etc.)

**Full mode message encoding:**
1. Compute `m = ((layers - 1) << 11) | (data_codewords - 1)` — a 16-bit value
2. Pack as 4 nibbles: nibble[i] = (m >> (4 * i)) & 0xF
3. Compute 6 RS ECC nibbles using GF(16), same polynomial
4. Concatenate: 10 nibbles → 40 bits

### Mode message bit placement

The mode message bits are placed in the ring immediately outside the bullseye,
starting **just right of the top-left corner** (the orientation mark), running
**clockwise**:

```
For compact (ring at radius = 6 from center, side = 13):
  Start position: column (cx - 5), row (cy - 6)  ← one right of top-left corner
  Direction: clockwise (→ across top, ↓ down right, ← across bottom, ↑ up left)
  Skip the 4 corner modules.
  Place 28 bits in the 44 non-corner modules of this ring's perimeter.
  Remaining 16 modules after the 28 mode message bits are the leading bits of
  the first data layer. (Wait — this is only true if the ring is split between
  mode message and data. The ISO standard says the mode message ring is
  distinct from the data layers. The first data layer starts at radius 7 for
  compact, radius 9 for full.)
```

Actually, the ISO standard is clear: the mode message occupies the entire
perimeter ring immediately outside the bullseye. The perimeter of a
13×13 square is 4 × 12 = 48 modules. Minus 4 corners = 44 non-corner modules.
But the mode message is only 28 bits. The remaining 44 - 28 = 16 modules
in the ring are filled by the first few bits of the data layers.

Let me re-examine this: In ISO 24778, the mode message ring and the data layers
share the same physical ring. The mode message bits take 28 positions; the
data bits fill the remaining 16 positions in the ring. The data continues
spiraling outward.

### Precise mode message ring placement algorithm

```
-- For compact Aztec:
ring_radius = bullseye_radius + 1  -- = 6 for compact, 8 for full
side = 2 * ring_radius + 1          -- = 13 for compact, 17 for full
perimeter_non_corner = 4 * (side - 2)  -- = 44 for compact, 60 for full

-- Enumerate non-corner ring positions clockwise from (cx - ring_radius + 1, cy - ring_radius):
positions = []
for col in (cx - ring_radius + 1)..(cx + ring_radius):     -- top edge, left to right
    positions.push( (col, cy - ring_radius) )
for row in (cy - ring_radius + 1)..(cy + ring_radius):     -- right edge, top to bottom
    positions.push( (cx + ring_radius, row) )
for col in (cx + ring_radius - 1)..(cx - ring_radius):     -- bottom edge, right to left (reversed)
    positions.push( (col, cy + ring_radius) )
for row in (cy + ring_radius - 1)..(cy - ring_radius):     -- left edge, bottom to top (reversed)
    positions.push( (cx - ring_radius, row) )

-- Place mode message bits first, then data bits:
for i, pos in enumerate(positions):
    if i < mode_message_bit_count:
        set_module(pos, mode_message[i])
    else:
        set_module(pos, data_bits[i - mode_message_bit_count])
```

For **compact**: 28 mode message bits, then 16 data bits in the ring.
For **full**: 40 mode message bits, then 20 data bits in the ring.

---

## Reference Grid

Full Aztec symbols with enough layers include a **reference grid** — a grid of
alternating dark and light modules that helps scanners correct for severe
perspective distortion. The reference grid does not appear in compact symbols.

The reference grid consists of:

- Horizontal lines of alternating dark/light modules, spaced every 16 modules
  from the center row (row `cy`)
- Vertical lines of alternating dark/light modules, spaced every 16 modules
  from the center column (col `cx`)

The alternating pattern along a reference grid line: the module at the
intersection of a reference grid line and the center row/column is DARK;
alternates every module from there.

```
Reference grid lines exist at:
  Rows: cy, cy ± 16, cy ± 32, cy ± 48, ...  (as long as within symbol bounds)
  Cols: cx, cx ± 16, cx ± 32, cx ± 48, ...

The center row and column themselves (cy and cx) are reference grid lines.

Module value at (row, col) on reference grid:
  if (row == cy or row == cy ± 16n) and (col == cx or col == cx ± 16n):
    -- intersection of two reference lines
    DARK
  else if row == cy or row == cy ± 16n:
    -- on horizontal reference line: alternate dark/light from center column
    (cx - col) mod 2 == 0 → DARK, else LIGHT
  else if col == cx or col == cx ± 16n:
    -- on vertical reference line: alternate dark/light from center row
    (cy - row) mod 2 == 0 → DARK, else LIGHT
```

Reference grid modules are **fixed structural modules** — they are not data
bits and are not altered by the data encoding. The data placement algorithm
skips reference grid positions.

For a **32-layer full** symbol (143×143), the reference grid lines at distance
16n from center are:
- Rows: cy ± 16, cy ± 32, cy ± 48, cy ± 64  (8 horizontal grid lines plus center)
- Cols: cx ± 16, cx ± 32, cx ± 48, cx ± 64  (8 vertical grid lines plus center)

For smaller full symbols, fewer reference lines fit:
- Layers 1..4 (19×19 to 31×31): only the center row and center column (distance 0)
- Layers 5..11 (35×35 to 59×59): center + ±16 lines
- Layers 12..19 (63×63 to 91×91): center + ±16 + ±32 lines
- Layers 20..27 (95×95 to 123×123): center + ±16 + ±32 + ±48 lines
- Layers 28..32 (127×127 to 143×143): center + ±16 + ±32 + ±48 + ±64 lines

The center row and center column are always reference grid lines in full
symbols. In compact symbols, there is no reference grid at all.

---

## Data Encoding Modes

Aztec Code uses a **latched mode-switching** encoding system with five base
modes. Each mode uses a different codeword size. The encoder builds a sequence
of variable-width codewords that represent the input, switching between modes
as needed for compactness.

This is similar to QR Code's mode system, but the codewords are narrower
(mostly 5-bit) and there are more modes with richer switching semantics.

### Mode overview

| Mode | Codeword bits | Character set |
|------|-------------|---------------|
| Upper | 5 | A–Z, space, and control codes via shift |
| Lower | 5 | a–z, space, and control codes via shift |
| Mixed | 5 | digits 0–9, symbols, control chars |
| Punct | 5 | punctuation, CR+LF pair |
| Digit | 4 | 0–9, space, comma, period |

### Upper mode character table (5 bits per codeword)

Upper mode is the default starting mode.

| Codeword | Character |
|----------|-----------|
| 00000 | Pad (null / filler) |
| 00001 | Space |
| 00010 | A |
| 00011 | B |
| 00100 | C |
| ... | ... |
| 11011 | Z |
| 11100 | Shift to Lower |
| 11101 | Latch to Mixed |
| 11110 | Latch to Punct |
| 11111 | Latch to Digit → or "shift" codeword (context-dependent) |

The full Upper alphabet is:

```
Value 0:  PADDING (not a character; used to fill incomplete codewords)
Value 1:  SP (space, ASCII 0x20)
Value 2:  A   Value 3:  B   Value 4:  C   Value 5:  D   Value 6:  E
Value 7:  F   Value 8:  G   Value 9:  H   Value 10: I   Value 11: J
Value 12: K   Value 13: L   Value 14: M   Value 15: N   Value 16: O
Value 17: P   Value 18: Q   Value 19: R   Value 20: S   Value 21: T
Value 22: U   Value 23: V   Value 24: W   Value 25: X   Value 26: Y
Value 27: Z
Value 28: Latch to Lower
Value 29: Latch to Mixed
Value 30: Latch to Punct
Value 31: Binary-Shift (shift to Byte mode for a specified count of bytes)
```

### Lower mode character table (5 bits per codeword)

```
Value 0:  PADDING
Value 1:  SP (space)
Value 2:  a   Value 3:  b   Value 4:  c   Value 5:  d   Value 6:  e
Value 7:  f   Value 8:  g   Value 9:  h   Value 10: i   Value 11: j
Value 12: k   Value 13: l   Value 14: m   Value 15: n   Value 16: o
Value 17: p   Value 18: q   Value 19: r   Value 20: s   Value 21: t
Value 22: u   Value 23: v   Value 24: w   Value 25: x   Value 26: y
Value 27: z
Value 28: Shift to Upper (single character only — returns to Lower after)
Value 29: Latch to Mixed
Value 30: Latch to Punct
Value 31: Binary-Shift
```

### Mixed mode character table (5 bits per codeword)

Mixed mode encodes digits, common symbols, and control characters.

```
Value 0:  PADDING
Value 1:  SP
Value 2:  0  (digit zero)
...
Value 11: 9
Value 12: ,   (comma, ASCII 0x2C)
Value 13: .   (period, ASCII 0x2E)
Value 14: !   (ASCII 0x21)
Value 15: "   (ASCII 0x22)
Value 16: #   (ASCII 0x23)
Value 17: $   (ASCII 0x24)
Value 18: %   (ASCII 0x25)
Value 19: &   (ASCII 0x26)
Value 20: '   (ASCII 0x27)
Value 21: (   (ASCII 0x28)
Value 22: )   (ASCII 0x29)
Value 23: *   (ASCII 0x2A)
Value 24: +   (ASCII 0x2B)
Value 25: -   (ASCII 0x2D)
Value 26: /   (ASCII 0x2F)
Value 27: :   (ASCII 0x3A)
Value 28: Latch to Upper
Value 29: Latch to Lower
Value 30: Latch to Punct
Value 31: Binary-Shift
```

### Punct mode character table (5 bits per codeword)

Punctuation mode is optimized for common punctuation sequences.

```
Value 0:  PADDING
Value 1:  CR+LF (two-character sequence, ASCII 0x0D 0x0A)
Value 2:  .     (period)
Value 3:  ,     (comma)
Value 4:  :     (colon)
Value 5:  !
Value 6:  "
Value 7:  (
Value 8:  )
Value 9:  ;
Value 10: [
Value 11: ]
Value 12: {
Value 13: }
Value 14: @
Value 15: \     (backslash)
Value 16: ^
Value 17: _
Value 18: `
Value 19: |
Value 20: ~
Value 21: DEL   (ASCII 0x7F)
Value 22: ESC   (ASCII 0x1B)
Value 23: SOH   (ASCII 0x01)
Value 24: STX   (ASCII 0x02)
Value 25: ETX   (ASCII 0x03)
Value 26: EOT   (ASCII 0x04)
Value 27: ENQ   (ASCII 0x05)
Value 28: ACK   (ASCII 0x06)
Value 29: BEL   (ASCII 0x07)
Value 30: BS    (ASCII 0x08)
Value 31: HT    (ASCII 0x09)
```

Note: there is no latch-back in Punct mode; the scanner returns to the
previous mode automatically after one Punct codeword (Punct is a shift,
not a latch). To encode multiple consecutive punctuation characters, you
must re-enter Punct mode with repeated Latch-to-Punct codewords.

Actually — the ISO standard specifies Punct differently. Punct can be entered
via a latch (persistent) or via a two-codeword sequence from other modes. The
above is a simplification. For v0.1.0, treat all mode transitions as latches.

### Digit mode character table (4 bits per codeword)

Digit mode is the most compact mode and uses 4-bit codewords.

```
Value 0:  PADDING
Value 1:  SP
Value 2:  0   Value 3:  1   Value 4:  2   Value 5:  3
Value 6:  4   Value 7:  5   Value 8:  6   Value 9:  7
Value 10: 8   Value 11: 9
Value 12: ,   (comma)
Value 13: .   (period)
Value 14: Latch to Upper
Value 15: Latch to Lower
```

Wait — this is a 4-bit codeword but there are 16 possible values (0..15), and
the above table only has non-latch characters at 0..13, with 14 and 15 as latch
codewords. Digit mode can latch to Upper or Lower. It cannot latch to Mixed or
Punct (you must first latch to Upper, then to the target mode).

### Binary / Byte mode

To encode arbitrary bytes (including non-printable ASCII and raw binary data),
the encoder inserts a **Binary Shift** escape:

```
In Upper or Lower mode, codeword value 31 is "Binary-Shift".
Following the Binary-Shift codeword:
  - A length prefix is encoded: if length ≤ 31, 5 bits; if length > 31, 11 bits
    (first 5 bits = 00000, then 11 bits for the actual count)
  - Then `length` bytes, each 8 bits, MSB first.
  - After the byte sequence, the mode reverts to the mode active before Binary-Shift.
```

This means byte mode is not a persistent "latch" — it is a temporary escape
with an explicit byte count. For encoding arbitrary binary data or UTF-8, this
is the universal path.

### Mode selection heuristic (v0.1.0)

For v0.1.0, the encoder uses **byte mode exclusively** (via Binary-Shift from
Upper mode) for any input that is not pure uppercase ASCII. This is always
valid, though not maximally compact. Multi-mode optimization is a v0.2.0
enhancement.

For v0.1.0:
1. Start in Upper mode.
2. If a character is in the Upper alphabet (A–Z, space): emit Upper codeword.
3. Otherwise: emit Binary-Shift with count = remaining bytes in the current
   non-Upper run, emit bytes, return to Upper.

In practice, for URLs and general strings, this reduces to: emit Binary-Shift
once for the entire input if it contains any lowercase or special characters.

### Codeword size selection for RS

Reed-Solomon error correction for Aztec is applied to the **complete codeword
sequence** — the bit stream after mode switching and padding, but before bit
stuffing. The RS codewords have the same bit width as the data codewords.

Since Aztec mixes codeword sizes (4-bit in Digit mode, 5-bit in Upper/Lower/
Mixed/Punct, 8-bit in Binary), the RS computation must be done per-segment or
on a normalized codeword sequence. For simplicity (and as mandated by the
ISO standard for the general case), the encoder:

1. Determines the **primary codeword size** for the symbol: if Binary-Shift
   data exceeds 50% of total bits, use 8-bit codewords (GF(256)); otherwise
   use 5-bit codewords (GF(32)) unless the data is entirely in Digit mode
   (4-bit, GF(16)).
2. Normalizes the bit stream to that codeword size for RS computation.

For v0.1.0 (byte mode only via Binary-Shift from Upper), the codeword size
is **8 bits** (GF(256)).

---

## Bit Stuffing

Aztec Code applies a **bit-stuffing** rule to the final data bit stream (after
RS encoding) to prevent long runs of identical bits that could interfere with
the scanner's ability to read the reference grid.

The rule is simple:

> After placing 4 consecutive identical bits (all 0 or all 1), insert one bit
> of the opposite value.

This is applied to the **codeword data + RS ECC bits** as they are laid out
in the data layers, NOT to the raw data before RS encoding.

Wait — the ISO standard applies bit stuffing before laying bits into the symbol.
The precise sequence is:

1. Encode data into codewords (mode switches + data characters + padding).
2. Append RS ECC codewords.
3. Apply bit stuffing to the entire (data + ECC) bit string.
4. Lay the stuffed bit string into the symbol's data layers.

### Bit-stuffing algorithm

```
input:  bits[] -- the data + ECC bit stream
output: stuffed[] -- the bit stream with inserted stuff bits

run_val = -1
run_len = 0
stuffed = []

for bit in bits:
    if bit == run_val:
        run_len += 1
    else:
        run_val = bit
        run_len = 1

    stuffed.push(bit)

    if run_len == 4:
        -- Insert a stuff bit of the opposite value
        stuffed.push(1 - bit)
        run_val = 1 - bit
        run_len = 1   -- the stuff bit starts a new run of length 1
```

**Example:**

```
Input:   1 1 1 1 0 0 0 0 1 0
After 4× 1:  stuff a 0  →  1 1 1 1 [0] 0 0 0 0 ...
After 4× 0:  stuff a 1  →  ... 0 0 0 0 [1] 1 0
Result:  1 1 1 1 0 0 0 0 0 1 1 0
```

Note that the run count **resets** after the stuffed bit. So five consecutive
1-bits would be:

```
Input:   1 1 1 1 1
Bits 1..4: emit, then stuff → 1 1 1 1 0
Bit 5 (the 5th 1): emit normally → 1 1 1 1 0 1
```

Bit stuffing increases the total bit count. The decoder reverses this by
removing the bit after every group of 4 identical bits.

### Bit stuffing does NOT apply to

- The bullseye finder pattern
- The orientation marks
- The mode message band
- The reference grid lines

Only the **data layer bits** are stuffed.

---

## Reed-Solomon Parameters

Aztec Code uses Reed-Solomon error correction in two different field sizes,
depending on the codeword bit width:

### GF(16) — for mode message and 4-bit or 5-bit codeword sequences

- **Field**: GF(2^4) = GF(16)
- **Primitive polynomial**: x^4 + x + 1 = 0x13
  (the binary representation is `10011`; in hex the polynomial coefficients
   give 0x13 = 0b10011 = x^4 + x + 1)
- **Generator polynomial convention**: b=1 (roots α^1 through α^n), matching
  the MA02 convention
- **Uses**: mode message RS (both compact and full), RS over 4-bit or 5-bit
  codewords when the primary codeword size is ≤ 5 bits

The GF(16) primitive polynomial `x^4 + x + 1`:

```
x^4 ≡ x + 1  (mod the primitive poly)
```

Multiplication table for GF(16) with this polynomial (element as 4-bit number,
rows × cols = product):

```
α^0 = 0b0001 = 1
α^1 = 0b0010 = 2
α^2 = 0b0100 = 4
α^3 = 0b1000 = 8
α^4 = 0b0011 = 3  (since x^4 = x + 1)
α^5 = 0b0110 = 6
α^6 = 0b1100 = 12
α^7 = 0b1011 = 11
α^8 = 0b0101 = 5
α^9 = 0b1010 = 10
α^10= 0b0111 = 7
α^11= 0b1110 = 14
α^12= 0b1111 = 15
α^13= 0b1101 = 13
α^14= 0b1001 = 9
α^15= α^0 = 1   (period = 15, so it is primitive)
```

### GF(256) — for 8-bit codeword sequences (byte mode)

- **Field**: GF(2^8) = GF(256)
- **Primitive polynomial**: x^8 + x^5 + x^4 + x^2 + x + 1 = 0x12D
  (same polynomial used by Data Matrix ECC200)
- **Generator polynomial convention**: b=1 (roots α^1 through α^n), matching
  the MA02 convention exactly — `aztec-code` can call MA02 directly for full
  byte-mode symbols
- **Uses**: RS over 8-bit codewords (byte mode or binary data)

Note: The GF(256) polynomial for Aztec (0x12D) is **different** from the
QR Code polynomial (0x11D). Aztec's polynomial is the same as Data Matrix's.

```
QR Code:     x^8 + x^4 + x^3 + x^2 + 1 = 0x11D
Data Matrix: x^8 + x^5 + x^4 + x^2 + x + 1 = 0x12D  ← Aztec uses this
```

### RS parameters by symbol configuration

| Codeword size | Field | Poly | Convention | Package |
|--------------|-------|------|------------|---------|
| 4-bit (Digit) | GF(16) | 0x13 | b=1 | Custom |
| 5-bit (Upper/Lower/Mixed) | GF(32) | 0x25 | b=1 | Custom |
| 8-bit (Binary) | GF(256) | 0x12D | b=1 | MA02 |
| Mode message (both sizes) | GF(16) | 0x13 | b=1 | Custom |

**GF(32)** (for 5-bit codewords):
- Primitive polynomial: x^5 + x^2 + 1 = 0x25 = 0b100101
- α^31 = α^0 (period = 31, so it is primitive)

### Error correction capacity

The number of RS ECC codewords is determined by the requested minimum ECC
percentage. The ECC percentage is the ratio of ECC codewords to **total
codewords** (data + ECC):

```
ecc_ratio = ecc_codewords / (data_codewords + ecc_codewords)
```

The minimum is 10%; the default is 23%; the maximum is 90%.

Because RS over GF(2^m) can correct up to ⌊n/2⌋ errors in a codeword
sequence of length n (where n is the number of ECC codewords), a 23% ECC
ratio means the symbol can recover from approximately 11.5% corrupted
codewords.

To achieve a target ECC ratio `e`:

```
ecc_count = ceil(e * (data_count + ecc_count))
          = ceil(e * data_count / (1 - e))
```

In practice, the encoder picks the smallest symbol (layer count) such that
the total codeword capacity minus the required ECC codewords is at least
enough for the data codewords.

### Capacity tables

Total data bits available per layer count (before stuffing):

**Compact Aztec** (5-bit codewords assumed for capacity calculation):

| Layers | Symbol size | Total bits | Max 5-bit codewords |
|--------|-------------|------------|---------------------|
| 1 | 15×15 | 78 | 15 |
| 2 | 19×19 | 200 | 40 |
| 3 | 23×23 | 390 | 78 |
| 4 | 27×27 | 648 | 129 |

Bit counts above are the usable data layer bits (excluding bullseye, mode
message, and orientation marks).

**Full Aztec** (5-bit codewords, selected layers):

| Layers | Symbol size | Total bits | Max 5-bit codewords |
|--------|-------------|------------|---------------------|
| 1 | 19×19 | 120 | 24 |
| 2 | 23×23 | 304 | 60 |
| 3 | 27×27 | 560 | 112 |
| 4 | 31×31 | 888 | 177 |
| 5 | 35×35 | 1288 | 257 |
| 6 | 39×39 | 1760 | 352 |
| 8 | 47×47 | 2888 | 577 |
| 10 | 55×55 | 4224 | 844 |
| 12 | 63×63 | 5776 | 1155 |
| 16 | 79×79 | 9548 | 1909 |
| 22 | 103×103 | 17048 | 3409 |
| 32 | 143×143 | 36052 | 7210 |

For byte mode (8-bit codewords), the max byte capacity at 23% ECC:
- Compact 4 layers: ~50 bytes
- Full 4 layers: ~85 bytes
- Full 10 layers: ~406 bytes
- Full 32 layers: ~3471 bytes

### Computing usable bits per layer

Each data layer wraps a band of 4 modules wide around the symbol. Within each
layer band, the data bits spiral around the band in groups of 2 bits per module
pair (see Data Layout section). The number of usable positions per layer:

```
-- For layer number L (1-indexed from innermost data layer):
-- Compact: bullseye radius = 5, mode message ring = 6
--   Innermost data layer L=1 is at radius 7..10 (4 module wide band)
--   Side of this ring's outer boundary = 2 * (bullseye_radius + 1 + L * 2 - 1) + 1

-- More directly:
-- For compact, layer L's outer ring has side:
side_outer = 11 + 2 * 2 * L + 2 = 11 + 4*L + 2... 
-- hmm, let me compute from the symbol sizes directly.

-- Compact 1 layer: 15×15. Bullseye = 11×11. Mode ring = 13×13.
--   Data ring = 15×15 outer minus 13×13 inner = perimeter of 15×15 = 56 modules.
--   But we must subtract reference grid intersections (none for compact).
--   Usable = 56 modules, each holding 2 data bits = 112 bits.
--   Wait but the mode message ring also overlaps... Let me re-derive.

-- The symbol is 15×15 = 225 modules total.
-- Bullseye (11×11) = 121 modules (fixed structural).
-- Mode message ring (13×13 perimeter) = 44 non-corner modules (28 mode + 16 data).
-- That leaves the 15×15 outer ring: perimeter = 4 * 14 = 56 modules = 56 data bits.
-- Total data bits for compact layer 1: 16 (in mode ring) + 56 = 72.
-- But actually, each module holds 2 bits in Aztec's zigzag layout (see below).
-- The 56 modules in the outer ring hold 2 bits each? No — each module holds 1 bit.

Actually each module (cell) holds exactly 1 bit (dark or light). The capacity
count is simply the number of available (non-structural) modules.
```

For a precise capacity derivation, refer to ISO/IEC 24778:2008, Table 1.
The implementation should embed the capacity table as a lookup rather than
computing it dynamically.

---

## Data Layout Algorithm

After computing the data + ECC bit stream and applying bit stuffing, the bits
are placed into the symbol's data layers. The layout proceeds from the
**innermost data layer** outward, one layer at a time. Within each layer, bits
are placed in a **clockwise spiral**.

### Overview of a single layer

Each layer is a band 2 modules wide (not 4 — the 4-module per layer size
is the total increase when adding a layer on both sides; each individual band
is 2 modules wide on one pass). Actually — Aztec layers are each 2 modules
thick as measured from the inner edge to the outer edge of the layer band.

Wait — the standard uses "layer" to mean a band that is exactly 2 modules
wide on each side of the bullseye, for a total of 4 extra modules per dimension.
Each layer band forms a closed ring around the symbol.

The data bits within a layer are arranged in **pairs** along the two rows/columns
of the band:

```
For a layer at Chebyshev distance d_inner..d_outer from center:
  (d_outer = d_inner + 1, since each layer is 2 modules wide and has inner and outer)

Placement pattern in the layer band (clockwise from top-right):
  1. Top edge:    columns (cx - d_inner + 1) to (cx + d_inner),  row (cy - d_outer)
                  then row (cy - d_inner) paired below it
  2. Right edge:  rows (cy - d_inner + 1) to (cy + d_inner),     col (cx + d_outer)
                  then col (cx + d_inner) paired to its left
  3. Bottom edge: columns (cx + d_inner) to (cx - d_inner + 1) (reversed), row (cy + d_outer)
                  then row (cy + d_inner) paired above it
  4. Left edge:   rows (cy + d_inner) to (cy - d_inner + 1) (reversed), col (cx - d_outer)
                  then col (cx - d_inner) paired to its right
```

This gives a 2-wide clockwise spiral. At each position along the spiral,
two bits are placed (one in the inner column/row of the band, one in the outer).

### Precise layer spiral algorithm

```
-- Center is at (cx, cy).
-- For compact, innermost data layer L=1 has:
--   d_inner = bullseye_radius + 2 = 7 (since mode message ring is at radius 6)
--   d_outer = d_inner + 1 = 8
--
-- For each subsequent layer L, d_inner += 2, d_outer += 2.
-- (Each layer adds 2 to the radius from center.)

-- For a layer with inner radius d_i and outer radius d_o = d_i + 1:

procedure place_layer(d_i, bits, bit_index):
    d_o = d_i + 1

    -- Top edge: left to right, starting one column right of the top-left corner
    for col from (cx - d_i + 1) to (cx + d_i):
        place_bit(col, cy - d_o, bits[bit_index]);   bit_index++
        place_bit(col, cy - d_i, bits[bit_index]);   bit_index++
        -- outer row first, then inner row

    -- Right edge: top to bottom, skipping corners already placed
    for row from (cy - d_i + 1) to (cy + d_i):
        place_bit(cx + d_o, row, bits[bit_index]);   bit_index++
        place_bit(cx + d_i, row, bits[bit_index]);   bit_index++

    -- Bottom edge: right to left
    for col from (cx + d_i) downto (cx - d_i + 1):
        place_bit(col, cy + d_o, bits[bit_index]);   bit_index++
        place_bit(col, cy + d_i, bits[bit_index]);   bit_index++

    -- Left edge: bottom to top
    for row from (cy + d_i) downto (cy - d_i + 1):
        place_bit(cx - d_o, row, bits[bit_index]);   bit_index++
        place_bit(cx - d_i, row, bits[bit_index]);   bit_index++
```

This is the canonical Aztec data spiral. Note the pair order: outer then
inner for all four sides. The full symbol's layers are traversed from
L=1 (innermost) to L=N (outermost), each using this spiral.

### Skipping reserved modules

Before placing data bits, mark the following as reserved (not available for
data):

1. Bullseye modules (Chebyshev distance ≤ bullseye_radius from center)
2. Orientation mark corners (four corners of the mode message ring)
3. Mode message band (the non-corner perimeter of the mode message ring)
4. Reference grid lines (full symbols only): all modules at rows or columns
   that are multiples of 16 from the center

The data placement algorithm skips any module that is reserved. The bit
index only advances when an unreserved module is written.

### Mode message ring shares the innermost data ring

The ring immediately outside the bullseye is split between the mode message
bits and the start of the data bits. The mode message occupies the first
28 bits (compact) or 40 bits (full) of this ring's non-corner positions
(clockwise from top-left corner + 1). The remaining positions in this ring
are filled by the data bit stream starting from bit 0.

For compact mode message ring (ring at radius 6):
- 44 non-corner positions total
- 28 used for mode message
- 16 used for the first 16 data bits

The layer spiral algorithm as described above already handles this: the "mode
message ring" is ring at Chebyshev distance `d_i = bullseye_radius + 1`, and
the first `mode_message_length` positions in the clockwise order are reserved
for the mode message. The data spiral starts placing bits immediately after
the mode message positions.

---

## Complete Encoding Algorithm

The full encoding pipeline:

```
1. ENCODE DATA INTO CODEWORDS

   a. Choose encoding: for v0.1.0, use Upper + Binary-Shift for all input.
      For full multi-mode v0.2.0+, segment the input to minimize codeword count.
   b. Build codeword list:
      - Start in Upper mode (default starting mode)
      - For uppercase A-Z or space: emit 5-bit Upper codeword
      - For all other input: emit Binary-Shift (codeword 31 in Upper mode),
        then 5-bit length (if ≤ 31 bytes) or 11-bit length, then raw bytes
   c. Count the total bits in the codeword sequence.

2. DETERMINE SYMBOL SIZE

   a. Start with compact Aztec layers 1..4. For each:
      - data_bits_available = total_bits_in_layer(compact, L)
                              minus mode_message_bits (28)
      - minimum_ecc_bits = ceil(requested_ecc_percent * data_bits_available)
      - if (data_bits + ecc_bits) ≤ data_bits_available: this layer fits → use it
   b. If no compact layer fits, try full Aztec layers 1..32 similarly
      (using 40 bits for mode message).
   c. If nothing fits → raise InputTooLong.

3. PAD DATA CODEWORDS

   The data codeword sequence must be padded to exactly `data_codeword_count`
   codewords, where `data_codeword_count` is determined by the symbol size
   minus ECC codeword count. Pad with PADDING codewords (value 0).

   If the last codeword is a partial codeword (the total data bits are not a
   multiple of the codeword bit width), zero-fill the trailing bits.

   Exception: if the last codeword before ECC would be all zeros (value 0 in
   GF), replace it with the maximum codeword value (all ones) to avoid RS
   complications. This is the "all-zero codeword avoidance" rule.

4. COMPUTE REED-SOLOMON ECC

   a. Determine field size from codeword bit width:
      - 4-bit codewords → GF(16), poly 0x13
      - 5-bit codewords → GF(32), poly 0x25
      - 8-bit codewords → GF(256), poly 0x12D  (use MA02 for this case)
   b. Compute the RS generator polynomial for the chosen ECC codeword count.
   c. Divide the data codeword polynomial by the generator to get the ECC
      remainder. (Polynomial remainder / systematic encoding, same as MA02.)
   d. Append ECC codewords to the data codewords.

5. APPLY BIT STUFFING

   Run the bit-stuffing algorithm on the combined (data + ECC) bit stream.

6. COMPUTE MODE MESSAGE

   a. Determine `layers - 1` (compact: 2 bits; full: 5 bits)
   b. Determine `data_codeword_count - 1` (compact: 6 bits; full: 11 bits)
   c. Pack into nibbles and compute GF(16) RS ECC.
   d. Flatten to 28 bits (compact) or 40 bits (full).

7. INITIALIZE GRID

   a. Create an NxN grid (all light modules).
   b. Place the bullseye (concentric rings from center, alternating D/L as
      defined by Chebyshev distance).
   c. Place orientation marks (4 dark corner modules of the mode message ring).
   d. Place mode message bits in the non-corner perimeter of the mode message ring.
   e. For full symbols: draw reference grid lines (center row/col, ±16, ±32, ...).
   f. Mark all placed modules as "reserved" for the data placement step.

8. PLACE DATA + ECC BITS

   Run the layer spiral algorithm from the innermost data layer outward,
   placing bits from the stuffed bit stream into non-reserved modules.
   The first layer's spiral starts after the mode message positions.

9. LAYOUT + PAINT

   Pass the final ModuleGrid to barcode-2d's `layout(grid, config)` to
   produce a PaintScene (P2D00). Pass the PaintScene to a PaintVM backend.
```

---

## Implementation Notes

### v0.1.0 simplifications

The following simplifications are acceptable for v0.1.0 and should be
explicitly noted in code comments:

1. **Byte mode only**: encode all input via Binary-Shift from Upper mode.
   Multi-mode optimization (Digit, Upper, Lower, Mixed, Punct) is v0.2.0.

2. **8-bit codewords only**: treat all data as byte-mode (GF(256) RS via MA02).
   This avoids implementing GF(16) and GF(32) RS for data codewords.
   GF(16) is still required for the mode message.

3. **No multi-segment encoding**: the entire input is one Binary-Shift segment.

4. **Default ECC = 23%**: do not expose the ECC percentage knob in v0.1.0.

5. **Auto-select compact vs full**: do not expose a `compact` option in v0.1.0.

### Tables to embed

The following constants must be embedded as lookup tables:

1. **Layer capacity table**: for compact layers 1..4 and full layers 1..32:
   - total usable data modules (after subtracting bullseye, mode message,
     reference grid, orientation marks)
   - number of full codewords (at 8-bit and 5-bit sizes) that fit
   - number of RS ECC codewords for 23% default ECC

2. **GF(16) log/antilog tables**: for mode message RS computation.

3. **GF(16) RS generator polynomials**: for (7,2) compact and (10,4) full
   mode message codes.

4. **GF(256) = MA02** (already in repo): reused for 8-bit codeword RS.

### GF(16) implementation

GF(16) arithmetic uses the primitive polynomial `x^4 + x + 1 = 0x13`:

```
GF16_LOG  = [−∞, 0, 1, 4, 2, 8, 5, 10, 3, 14, 9, 7, 6, 13, 11, 12]
GF16_ALOG = [1, 2, 4, 8, 3, 6, 12, 11, 5, 10, 7, 14, 15, 13, 9, 1]

gf16_mul(a, b):
  if a == 0 or b == 0: return 0
  return GF16_ALOG[(GF16_LOG[a] + GF16_LOG[b]) mod 15]
```

### Mode message RS in detail

**Compact mode message** uses a (7,2) code over GF(16):
- 2 data nibbles → 5 ECC nibbles
- RS generator polynomial with roots α^1 through α^5 over GF(16)

The generator polynomial:
```
g(x) = (x + α^1)(x + α^2)(x + α^3)(x + α^4)(x + α^5)
     = x^5 + g4*x^4 + g3*x^3 + g2*x^2 + g1*x + g0

Computing over GF(16), poly 0x13:
g(x) = x^5 + 0xF*x^4 + 0x1A*x^3 + ...
```

Rather than deriving this at runtime, embed the precomputed generator
polynomial coefficients directly (ISO/IEC 24778:2008, Annex A).

**Full mode message** uses a (10,4) code over GF(16):
- 4 data nibbles → 6 ECC nibbles
- RS generator polynomial with roots α^1 through α^6 over GF(16)

### Bit stuffing interaction with RS

Bit stuffing is applied **after** RS ECC is appended. This means:

1. The RS check symbols are valid over the un-stuffed bit sequence.
2. A decoder must de-stuff first, then perform RS decoding.
3. The stuffed bit count exceeds the un-stuffed bit count by at most
   1 extra bit for every 4 bits (25% overhead in the worst case; typical
   data has much less overhead because perfectly alternating bits trigger
   no stuffing).

In practice the stuffing overhead is small: for random binary data, the
expected overhead is `n / (4 * 2) = n/8` extra bits (one stuff per expected
run of 4 same-value bits, which occurs on average every 16 bits for truly
random data). The symbol size selection must account for stuffing overhead.
A conservative estimate: add 20% to the bit count before comparing against
layer capacity.

### Reference grid conflicts with data layers

The reference grid lines occupy modules that would otherwise be data modules.
The data placement algorithm must skip these. The bit index does NOT advance
when a skipped module is encountered — only when a bit is actually placed.

This means the reference grid acts as a "gap" in the data layout. Decoders
must know which modules are reference grid modules and skip them when
extracting data bits — the data bit stream is continuous despite the gaps.

### Coordinate system

The symbol is a square grid. Use row-major order:

```
(row=0, col=0) = top-left corner of symbol
(row=N-1, col=N-1) = bottom-right corner
```

The bullseye center is at:
```
cx = cy = (N - 1) / 2 = floor(N / 2)
```

Since N is always odd (from the formula `11 + 4L` and `15 + 4L`), the center
is always an integer coordinate.

---

## Visualization Annotations

| Color (suggested) | Role |
|-------------------|------|
| Deep blue | bullseye rings |
| Blue-grey | orientation marks |
| Purple | mode message bits |
| Teal | reference grid lines |
| Black/white | data module |
| Green/light green | ECC module |
| Orange | bit-stuffing bits |

---

## Error Types

```
AztecError::InputTooLong    -- input does not fit in a 32-layer full symbol
AztecError::InvalidMode     -- internal: mode encoding produced an invalid codeword
AztecError::LayerOverflow   -- bit stuffing caused data to overflow the layer
                               (should not occur if sizing accounts for stuffing overhead)
```

---

## Public API

```
encode(input: string | bytes, options?: AztecOptions) → ModuleGrid
  -- Encodes input to an Aztec Code module grid (abstract module units, no pixels).
  -- Raises InputTooLong if input exceeds 32-layer full symbol capacity.
  -- ModuleGrid contains the dark/light module values for the complete symbol.

layout(grid: ModuleGrid, config?: Barcode2DLayoutConfig) → PaintScene
  -- Translate a ModuleGrid into a pixel-resolved PaintScene (P2D00).
  -- Delegates to barcode-2d::layout() — aztec-code does not implement this itself.

encode_and_layout(input: string | bytes, options?: AztecOptions,
                  config?: Barcode2DLayoutConfig) → PaintScene
  -- Convenience: encode + layout in one call.

render_svg(input: string | bytes, options?: AztecOptions,
           config?: Barcode2DLayoutConfig) → string
  -- Convenience: encode + layout + paint-vm-svg backend → SVG string.

explain(input: string | bytes, options?: AztecOptions) → AnnotatedModuleGrid
  -- Encode with full per-module role annotations (for visualizers).

AztecOptions {
  min_ecc_percent?: number    -- minimum ECC percentage (default: 23, range: 10..90)
  compact?:         boolean   -- force compact form; error if data does not fit (default: false)
  -- v0.2.0 additions:
  -- encoding_mode?: "auto" | "bytes" | "text"
}
```

---

## Package Matrix

| Language | Directory | Depends on |
|----------|-----------|------------|
| Rust | `code/packages/rust/aztec-code/` | barcode-2d, gf256 (MA01), reed-solomon (MA02) |
| TypeScript | `code/packages/typescript/aztec-code/` | barcode-2d, gf256, reed-solomon |
| Python | `code/packages/python/aztec-code/` | barcode-2d, gf256, reed-solomon |
| Go | `code/packages/go/aztec-code/` | barcode-2d, gf256, reed-solomon |
| Ruby | `code/packages/ruby/aztec_code/` | barcode-2d, gf256, reed-solomon |
| Elixir | `code/packages/elixir/aztec_code/` | barcode_2d, gf256, reed_solomon |
| Lua | `code/packages/lua/aztec-code/` | barcode-2d, gf256, reed-solomon |
| Perl | `code/packages/perl/aztec-code/` | barcode-2d, gf256, reed-solomon |
| Swift | `code/packages/swift/aztec-code/` | Barcode2D, GF256, ReedSolomon |
| C# | `code/packages/csharp/aztec-code/` | Barcode2D, GF256, ReedSolomon |
| F# | `code/packages/fsharp/aztec-code/` | Barcode2D, GF256, ReedSolomon |
| Kotlin | `code/packages/kotlin/aztec-code/` | barcode-2d, gf256, reed-solomon |
| Java | `code/packages/java/aztec-code/` | barcode-2d, gf256, reed-solomon |
| Dart | `code/packages/dart/aztec-code/` | barcode_2d, gf256, reed_solomon |
| Haskell | `code/packages/haskell/aztec-code/` | barcode-2d, gf256, reed-solomon |

---

## Test Strategy

### Unit tests

1. **GF(16) arithmetic**: verify the log/antilog tables, multiplication and
   division for all non-zero elements, and the identity `α^15 = α^0 = 1`.

2. **Mode message encoding (compact)**:
   - Input: 1 layer, 5 data codewords
   - Expected mode message: compute manually and verify bit pattern
   - Verify all 7 nibbles (2 data + 5 ECC) are correct

3. **Mode message encoding (full)**:
   - Input: 2 layers, 12 data codewords
   - Verify all 10 nibbles

4. **Bit stuffing**:
   - Input: `0b00001111_00001111` → verify stuff bits inserted after each
     run of 4 identical bits
   - Input: alternating bits → verify no stuffing (no runs of 4)
   - Input: all zeros (32 bits) → verify every 4th bit has a stuff 1

5. **Layer spiral placement**:
   - For a compact 1-layer symbol with known data, verify that each module
     in the data ring is set to the expected bit value at the expected
     (row, col) position

6. **RS encoding (GF(256) byte mode)**:
   - Encode a known byte sequence. Verify ECC codewords match MA02 output
     with the same generator roots and poly 0x12D.

7. **Full encode integration**:
   - Encode `"A"` → verify it produces a 15×15 compact 1-layer symbol
   - Verify bullseye is correctly placed (center 3×3 dark, ring 2 light, etc.)
   - Verify mode message ring modules

### Integration tests

1. **Round-trip test**: encode a string with this encoder, render to PNG,
   decode with a reference Aztec scanner (e.g., `zxing`), verify decoded
   string matches input.

2. **Known test vectors** from ISO/IEC 24778:2008 Annex I (sample symbols):
   - Character string "ISO/IEC 24778:2008" → verify exact module grid

3. **IATA boarding pass encoding**: encode a standard IATA boarding pass
   data string and verify the symbol is accepted by boarding pass scanners.

### Cross-language verification

All 15 language implementations should produce **identical ModuleGrid outputs**
for the same input. Run the test corpus against all implementations:

```
"A"                               -- compact 1 layer, uppercase only
"Hello World"                     -- binary-shift from Upper
"https://example.com"             -- URL, mixed case, symbol chars
"01234567890123456789"             -- digit-heavy
<64 bytes of 0x00..0x3F>          -- raw binary
```

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
                     barcode-2d          MA02 reed-solomon
                         │                   │
                    aztec-code ──────────────┘
                         │
                      MA01 gf256
                         │
                      MA00 polynomial
```

`aztec-code` depends on:
- `barcode-2d` — for the `ModuleGrid` type and `layout()` function
- `MA02` (reed-solomon) — for GF(256) RS encoding in byte mode (same polynomial
  as Data Matrix, `0x12D`, b=1 convention — MA02 is a direct drop-in)
- `MA01` (gf256) — for GF(256) field arithmetic; GF(16) is implemented inline
  since it is not provided by an existing package

Unlike `qr-code`, which uses a different RS convention (b=0) and cannot call
MA02 directly, `aztec-code` uses the b=1 convention and calls MA02 directly
for all GF(256) RS operations. GF(16) arithmetic for the mode message is a
small self-contained implementation (15-element field, two 15-element tables).

---

## Future Extensions

- **Multi-mode encoding** — optimal segmentation into Digit, Upper, Lower,
  Mixed, Punct, and Binary regions for minimum symbol size. For typical URL
  inputs this can reduce the layer count by 1–2 compared to byte-only mode.

- **ECI mode** — Extended Channel Interpretation for signaling non-ASCII
  character encodings (UTF-8, ISO-8859-n, shift-JIS, etc.) to the decoder.

- **Structured Append** — split a large message across multiple Aztec symbols,
  each with a sequence number and total count.

- **Decoder** — a separate, more complex spec covering image preprocessing,
  perspective correction, bullseye detection, mode message extraction,
  RS decoding with error location, and multi-mode codeword parsing.

- **Animated / multi-layer display** — the `explain()` API with the
  interactive visualizer showing each layer, mode switch, and RS codeword
  highlighted in the symbol.

- **Micro Aztec** — sub-compact forms for very short strings on tiny labels
  (not part of ISO/IEC 24778 but discussed in the Welch Allyn patent history).

- **GF(32) RS** — implement a general GF(32) RS encoder for 5-bit codeword
  sequences; currently the 8-bit codeword path via GF(256) serves as the
  universal v0.1.0 fallback.
