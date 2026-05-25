# IC04 — `image-codec-jpeg`: Baseline JPEG Encoder/Decoder

## Overview

JPEG (Joint Photographic Experts Group) is the world's most widely deployed
image format, introduced in 1992 and named after the committee that standardised
it (ISO/IEC 10918-1 / ITU-T T.81). Almost every photograph you have ever seen
on a website, in a text message, or on a digital camera memory card is a JPEG.

JPEG achieves its compression through a pipeline of mathematically elegant
transforms. Unlike BMP (raw pixels) or QOI (lossless delta coding), JPEG is
**lossy** — it throws away information that the human visual system is unlikely
to notice. The key insight is that the eye is far more sensitive to changes in
brightness (luminance) than to changes in colour (chrominance), and far more
sensitive to low-frequency spatial variation (gradual shading) than to
high-frequency detail (sharp edges at the pixel level).

This crate implements **baseline JPEG** (the most common and universally
compatible profile), wrapped in a **JFIF** (JPEG File Interchange Format)
container. This is what every major application produces when it "saves as JPEG".

### Key properties

| Property | Value |
|----------|-------|
| File extensions | `.jpg`, `.jpeg` |
| MIME type | `image/jpeg` |
| Compression | Lossy (DCT-based) |
| Colour model | YCbCr (BT.601 coefficients) |
| Chroma sampling | 4:4:4 (v1); 4:2:0 planned (v2) |
| Quality range | 1 (worst, smallest) to 100 (best, largest) |
| Alpha channel | Not supported — discarded on encode |
| Bit depth | 8 bits per component (24-bit colour) |
| Patent status | Royalty-free — all JPEG patents expired by ~2006 |
| Standard | ISO/IEC 10918-1 / ITU-T T.81 |

### Why JPEG instead of PNG or QOI?

Compared to **QOI** (IC03), JPEG works in the *frequency domain* instead of
the spatial domain. Rather than predicting "this pixel is similar to the
previous pixel", JPEG asks "which *frequencies* make up this 8×8 image patch?"
Most natural images have most of their energy in low frequencies — smooth
gradients of colour and brightness. JPEG exploits this by discarding the
high-frequency coefficients that carry fine detail but little perceptual weight.

Compared to **PNG** (lossless DEFLATE), JPEG achieves 10–50× smaller files
for photographs at quality ≥ 75, with visual difference that is usually
imperceptible. PNG remains the right choice for screenshots, icons, line art,
and any image that must survive round-tripping without pixel changes.

### Relationship to other specs

- **DSP02** (`dsp-dct`): provides the `dct` / `idct` primitives used in Steps 3
  and 8 of the encode/decode pipelines below.
- **CMP04** (`huffman`): the entropy coding step (Steps 6 / 6′) uses Huffman
  coding; the standard JPEG Huffman tables are embedded as constants in this
  crate (the `huffman-tree` crate is used at encode time to build code tables).
- **IC00** (`pixel-container`): input and output type for encode/decode.

---

## JPEG File Structure

JPEG files consist of a sequence of **markers**. Every marker begins with the
byte `0xFF` followed by a one-byte marker code. Most markers are followed by a
**segment** that starts with a 2-byte big-endian **length field**; the length
counts itself (2 bytes) but does not count the `FF XX` marker prefix.

Two markers have no segment at all: `SOI` (Start of Image, `FF D8`) and `EOI`
(End of Image, `FF D9`).

### Marker table

| Marker | Hex | Segment? | Purpose |
|--------|-----|----------|---------|
| SOI | `FF D8` | No | Start of Image — always first two bytes |
| APP0 | `FF E0` | Yes | JFIF application header (signature, version, density) |
| DQT | `FF DB` | Yes | Define Quantization Table — one table per quality setting |
| SOF0 | `FF C0` | Yes | Start of Frame (baseline DCT) — dimensions and components |
| DHT | `FF C4` | Yes | Define Huffman Table — up to four tables (luma/chroma × DC/AC) |
| SOS | `FF DA` | Yes | Start of Scan — scan header + entropy-coded bitstream |
| EOI | `FF D9` | No | End of Image — always last two bytes |

### ASCII file layout diagram

The numbers below are byte offsets for a typical minimal-image JFIF file.
Actual offsets depend on image dimensions and quality; only the marker order
is fixed.

```
Offset  Size    Contents
------  ------  -------------------------------------------------------
0       2       FF D8          SOI (Start of Image)
2       2       FF E0          APP0 marker
4       2       00 10          APP0 length = 16 (2 length bytes + 14 data)
6       5       4A 46 49 46 00 "JFIF\0" — JFIF identifier
11      2       01 01          JFIF version 1.1
13      1       00             density units: 0 = pixel aspect ratio
14      2       00 01          X density = 1
16      2       00 01          Y density = 1
18      2       00 00          thumbnail width/height = 0 (no thumbnail)
                               ─── APP0 segment ends ───
20      2       FF DB          DQT marker
22      2       00 43          DQT length = 67 (2 + 1 precision/ID byte + 64 table)
24      1       00             0 = luma table, precision 8-bit
25      64      [64 bytes]     Luma quantization table, zigzag order
89      2       FF DB          DQT marker (chroma table)
91      67      ...            Chroma quantization table
                               ─── DQT segments end ───
160     2       FF C0          SOF0 marker (Start of Frame, baseline DCT)
162     2       00 11          SOF0 length = 17 (2 + 1 + 2 + 2 + 1 + 3×3)
164     1       08             sample precision = 8 bits
165     2       [height]       image height, big-endian u16
167     2       [width]        image width, big-endian u16
169     1       03             number of components = 3 (Y, Cb, Cr)
170     3       01 11 00       component 1: ID=1, H/V sampling=1:1, quant table 0
173     3       02 11 01       component 2: ID=2, H/V sampling=1:1, quant table 1
176     3       03 11 01       component 3: ID=3, H/V sampling=1:1, quant table 1
                               ─── SOF0 ends ───
179     2       FF C4          DHT marker (luma DC Huffman table)
...     ...     [luma DC]      Huffman counts + code values
...     2       FF C4          DHT marker (luma AC Huffman table)
...     ...     [luma AC]
...     2       FF C4          DHT marker (chroma DC)
...     ...     [chroma DC]
...     2       FF C4          DHT marker (chroma AC)
...     ...     [chroma AC]
                               ─── DHT segments end ───
...     2       FF DA          SOS marker (Start of Scan)
...     2       [length]       SOS header length
...     1       03             number of components in scan = 3
...     2       01 00          component 1 (Y):  DC table 0, AC table 0
...     2       02 11          component 2 (Cb): DC table 1, AC table 1
...     2       03 11          component 3 (Cr): DC table 1, AC table 1
...     3       00 3F 00       Ss=0 Se=63 Ah/Al=0 (baseline scan)
...     [N]     [entropy data] bit-packed Huffman-coded MCU data
                               ─── Entropy stream ends ───
...     2       FF D9          EOI (End of Image)
```

The **entropy-coded segment** (between SOS header and EOI) is a raw bitstream —
not chunked — and any `FF` byte within it is byte-stuffed as `FF 00` (see
Step 7 in the encoding pipeline).

---

## APP0 Segment Detail

The APP0 segment carries the JFIF signature and basic metadata:

| Offset (from segment start) | Size | Field | Value |
|----|----|----|---|
| 0 | 2 | Length | `0x0010` = 16 |
| 2 | 5 | Identifier | `"JFIF\0"` (ASCII, null-terminated) |
| 7 | 1 | Version major | `0x01` (version 1) |
| 8 | 1 | Version minor | `0x01` (revision 1) |
| 9 | 1 | Density units | `0` = pixel aspect ratio only |
| 10 | 2 | X density | `0x0001` |
| 12 | 2 | Y density | `0x0001` |
| 14 | 1 | Thumbnail width | `0` (no embedded thumbnail) |
| 15 | 1 | Thumbnail height | `0` |

The decoder must verify the identifier string is exactly `"JFIF\0"`.

---

## Encoding Pipeline

The encoder transforms a `PixelContainer` (RGBA8, row-major) into a JPEG byte
stream in eight steps. Each step is described below with its purpose, the
mathematics involved, and intuitive explanations.

### Step 1 — Colour Conversion: RGBA → YCbCr

**Why YCbCr?** The human visual system devotes far more neural bandwidth to
processing *brightness* than *colour*. The YCbCr colour space separates
luminance (Y) from chrominance (Cb = blue difference, Cr = red difference).
This lets the codec represent colour channels with lower precision while keeping
brightness sharp, and the eye barely notices.

BT.601 forward transform (used in standard-definition video and JFIF):

```
Y  =  0.299  · R + 0.587  · G + 0.114  · B
Cb = -0.168736 · R - 0.331264 · G + 0.5    · B + 128
Cr =  0.5    · R - 0.418688 · G - 0.081312 · B + 128
```

All three output values are clamped to [0, 255] and stored as u8.

The constants add up intuitively:
- Y mixes RGB to match luminosity (0.299+0.587+0.114 = 1.0). Green gets the
  biggest weight because the eye is most sensitive to green light.
- Cb and Cr are centred at 128 so that a neutral grey (R=G=B) produces
  Cb=Cr=128, fitting the full [0,255] range without negative values.
- Alpha is discarded — JPEG has no alpha channel.

**Integer-friendly approximation** (avoids floating-point in tight loops):

```
// Scale factors × 2^16 (shift right 16 after multiply, then round)
Y  = ( 19595·R + 38470·G +  7471·B          ) >> 16
Cb = (-11056·R - 21712·G + 32768·B + 8388608) >> 16
Cr = ( 32768·R - 27440·G -  5328·B + 8388608) >> 16
```

The implementation may use either floating-point or integer arithmetic; results
must agree within 1 LSB of the exact floating-point values.

### Step 2 — 8×8 Block Extraction and Level Shifting

JPEG processes images in **8×8 pixel blocks** (called Minimum Coded Units, or
MCUs). This block size is the sweet spot for the DCT: large enough to capture
useful frequency information, small enough to compute cheaply.

**Padding**: if the image width or height is not a multiple of 8, the image is
padded by replicating the nearest edge pixels. For example, a 10×10 image is
padded to 16×16 by copying column 9 into column 10-15 and row 9 into rows
10-15. The decoder discards the padding.

**Level shifting**: DCT is defined for symmetric input centred around zero.
JPEG samples range [0, 255], so each sample is shifted by subtracting 128
before the DCT, producing values in [−128, 127].

```
shifted[y][x] = original[y][x] - 128
```

This matters because the DCT of zero-centred data places all the energy in
the AC (non-DC) coefficients for constant blocks, making quantization uniform.

### Step 3 — Forward 2-D DCT

The **Discrete Cosine Transform** (DCT-II, see DSP02) converts an 8×8 block of
spatial-domain pixel values into an 8×8 block of frequency-domain coefficients.

**How it works**: think of the 8×8 block as a superposition of 64 "basis
patterns" — cosine waves oscillating at different horizontal and vertical
frequencies. The DCT finds how much of each basis pattern is present. The top-
left coefficient F[0][0] is the **DC coefficient** (average brightness of the
block). The other 63 are **AC coefficients** (variations around that average).

```
F[u][v] = Σ_{x=0..7} Σ_{y=0..7}  f[x][y]
           · cos( π(2x+1)u / 16 )
           · cos( π(2y+1)v / 16 )

where f[x][y] is the level-shifted block sample and
F[u][v] is the (u,v) frequency coefficient.
```

The 2-D DCT is computed as a row-then-column 1-D DCT:

```rust
// Apply 1-D DCT-II to each of the 8 rows
for row in 0..8 {
    let out = dsp_dct::dct(&block[row], DctType::II, DctNorm::None)?;
    // write back
}
// Apply 1-D DCT-II to each of the 8 resulting columns
for col in 0..8 {
    let out = dsp_dct::dct(&column, DctType::II, DctNorm::None)?;
    // write back
}
```

**Why DCT compacts energy**: for smooth image regions, nearby pixels differ by
small amounts. When you take the DCT of a nearly-constant signal, the sum
(DC coefficient) is large, but the oscillating cosine terms nearly cancel out,
leaving AC coefficients close to zero. This is called "energy compaction" — the
useful information is packed into a few large coefficients. Quantization (Step 4)
then aggressively rounds the small AC coefficients to zero, achieving compression.

For a natural photo at quality 75, typically only 5–15 of the 64 DCT
coefficients per block survive quantization as non-zero values.

### Step 4 — Quantization

**This is the lossy step.** Quantization divides each DCT coefficient by a
value from a **quantization table** and rounds to the nearest integer:

```
Q[u][v] = round( F[u][v] / Qtable[u][v] )
```

Dividing by a larger number discards more precision — that is, throws away more
information. The standard quantization tables (from Annex K of ISO/IEC 10918-1)
are tuned to human perception: high-frequency entries (bottom-right of the
table) have large divisors (coarse quantization) because the eye cannot see
fine-grained high-frequency variation. Low-frequency entries (top-left) have
small divisors (fine quantization) because brightness gradients are visible.

**Quality scaling**: the `quality` parameter (1–100) scales the standard tables:

```
// Convert quality 1–100 to a scale factor
if quality < 50 {
    scale = 5000 / quality        // quality 1 → scale 5000 (very coarse)
} else {
    scale = 200 - 2 * quality     // quality 100 → scale 0; quality 50 → scale 100
}

// Scale each table entry; clamp to [1, 255]
qtable[i] = clamp((std_qtable[i] * scale + 50) / 100, 1, 255)
```

A scale of 100 means "use the standard tables unchanged" (quality 50).
A scale of 1 means "divide by almost nothing" — very little loss (quality ~99).
A scale of 5000 means "divide by 50× the standard" — extreme compression (quality 1).

**Standard luma quantization table** (Annex K, Table K.1), row-major, zig-zag
order explained in Step 5:

```
Standard luminance quantization table (8×8, u, v ∈ 0..7):

     u=0  u=1  u=2  u=3  u=4  u=5  u=6  u=7
v=0 [ 16   11   10   16   24   40   51   61 ]
v=1 [ 12   12   14   19   26   58   60   55 ]
v=2 [ 14   13   16   24   40   57   69   56 ]
v=3 [ 14   17   22   29   51   87   80   62 ]
v=4 [ 18   22   37   56   68  109  103   77 ]
v=5 [ 24   35   55   64   81  104  113   92 ]
v=6 [ 49   64   78   87  103  121  120  101 ]
v=7 [ 72   92   95   98  112  100  103   99 ]
```

Notice: the top-left entry (DC, u=v=0) is 16. The bottom-right entry
(highest frequency AC, u=v=7) is 99. Dividing by 99 instead of 16 means
high-frequency components lose 6× more precision.

**Standard chroma quantization table** (Annex K, Table K.2):

```
Standard chrominance quantization table (8×8):

     u=0  u=1  u=2  u=3  u=4  u=5  u=6  u=7
v=0 [ 17   18   24   47   99   99   99   99 ]
v=1 [ 18   21   26   66   99   99   99   99 ]
v=2 [ 24   26   56   99   99   99   99   99 ]
v=3 [ 47   66   99   99   99   99   99   99 ]
v=4 [ 99   99   99   99   99   99   99   99 ]
v=5 [ 99   99   99   99   99   99   99   99 ]
v=6 [ 99   99   99   99   99   99   99   99 ]
v=7 [ 99   99   99   99   99   99   99   99 ]
```

The chroma table is much coarser than luma (many entries are 99), because
colour information can be stored at lower precision without visible degradation.

### Step 5 — Zigzag Reordering

After quantization, the 8×8 block is serialised into a 1-D sequence of 64
values. A naive row-major scan would not work well because adjacent spatial
coefficients differ in meaningful ways, and run-length coding (Step 6) needs
the zeros grouped together.

The standard **zigzag scan** traverses the 8×8 coefficient matrix diagonally,
starting from the DC coefficient (top-left) and snaking down-right toward the
high-frequency corner (bottom-right). Because energy compacts toward the
top-left, this ensures that the non-zero coefficients come first and the zeros
(quantized away) form a long run at the end — ideal for run-length coding.

**Zigzag permutation table** (each entry is the linear index `v*8 + u` in the
8×8 block that appears at position `i` in the output stream):

```
Position in zigzag output stream → 2-D block index (row*8 + col)

 i=  0   1   5   6  14  15  27  28
 i=  2   4   7  13  16  26  29  42
 i=  3   8  12  17  25  30  41  43
 i=  9  11  18  24  31  40  44  53
 i= 10  19  23  32  39  45  52  54
 i= 20  22  33  38  46  51  55  60
 i= 21  34  37  47  50  56  59  61
 i= 35  36  48  49  57  58  62  63
```

As a flat lookup table (zigzag_index → block_index):

```
ZIGZAG: [usize; 64] = [
     0,  1,  8, 16,  9,  2,  3, 10,
    17, 24, 32, 25, 18, 11,  4,  5,
    12, 19, 26, 33, 40, 48, 41, 34,
    27, 20, 13,  6,  7, 14, 21, 28,
    35, 42, 49, 56, 57, 50, 43, 36,
    29, 22, 15, 23, 30, 37, 44, 51,
    58, 59, 52, 45, 38, 31, 39, 46,
    53, 60, 61, 54, 47, 55, 62, 63,
]
```

Reading `zigzag[i]` gives the position in the row-major flat block that should
appear at position `i` in the zigzag-ordered output.

### Step 6 — Entropy Coding (Huffman + RLE)

Entropy coding converts the quantized, zigzag-ordered 64-coefficient sequence
into a compact bitstream. JPEG uses a combination of **Huffman coding** (for
compact variable-length codes) and **run-length encoding** for AC coefficients.

All bits are packed MSB-first (most-significant bit first) into bytes.

#### DC coefficient encoding

The DC coefficient (zigzag position 0) is encoded **differentially**: instead
of the absolute value, the difference from the previous block's DC coefficient
for the same component is coded.

```
diff = DC[current_block] - DC[previous_block]    // initialise prev = 0
```

The difference is encoded as a **(category, magnitude)** pair:

- **Category** (0–11): the number of bits needed to represent `|diff|`.
  Category 0 means diff=0. Category k means `2^(k-1) ≤ |diff| < 2^k`.
- **Magnitude**: the actual bit representation of `diff` within its category.
  For positive diff: the binary representation. For negative diff: the binary
  representation of `diff + 2^category - 1` (one's complement).

The **Huffman code for the category** is looked up in the DC Huffman table,
then the magnitude bits are appended directly (not Huffman-coded).

Component Y uses the **luminance DC table**; Cb and Cr use the **chrominance
DC table**.

#### AC coefficient encoding (zigzag positions 1–63)

AC coefficients are run-length encoded as **(run, category, magnitude)** triples:
- **run**: number of consecutive zero coefficients preceding this non-zero value
  (0–15).
- **category** (1–10): bit-length of the non-zero value.
- The pair `(run, category)` is packed as a single byte `(run << 4) | category`
  and Huffman-coded from the AC table.
- **Magnitude bits** (category bits) are appended after the Huffman code.

Two special AC symbols:
- **EOB** (End of Block): the symbol `(run=0, category=0)` = byte `0x00`.
  Emitted when all remaining coefficients are zero. Terminates the AC stream
  for this block.
- **ZRL** (Zero Run Length): the symbol `(run=15, category=0)` = byte `0xF0`.
  Emitted when there are 16 or more consecutive zeros. A single ZRL codes 16
  zeros; subsequent zeros need another ZRL or a regular symbol.

Component Y uses **luminance AC table**; Cb and Cr use **chrominance AC table**.

#### Standard Huffman tables (Annex K of T.81)

Each Huffman table is stored in the DHT segment as a list of code-length counts
followed by the code values in increasing-length order.

**Luminance DC table** (12 categories):

```
Code lengths (number of codes of each bit-length, 1..16):
  0  1  5  1  1  1  1  1  1  0  0  0  0  0  0  0

Code values (in order of increasing length):
  00 01 02 03 04 05 06 07 08 09 0A 0B
```

**Chrominance DC table** (12 categories):

```
Code lengths:
  0  3  1  1  1  1  1  1  1  1  1  0  0  0  0  0

Code values:
  00 01 02 03 04 05 06 07 08 09 0A 0B
```

**Luminance AC table** (162 symbols):

```
Code lengths (codes of bit-length 1..16):
  0  2  1  3  3  2  4  3  5  5  4  4  0  0  1  125

Code values (162 bytes, hex):
  01 02 03 00 04 11 05 12 21 31 41 06 13 51 61
  07 22 71 14 32 81 91 A1 08 23 42 B1 C1 15 52
  D1 F0 24 33 62 72 82 09 0A 16 17 18 19 1A 25
  26 27 28 29 2A 34 35 36 37 38 39 3A 43 44 45
  46 47 48 49 4A 53 54 55 56 57 58 59 5A 63 64
  65 66 67 68 69 6A 73 74 75 76 77 78 79 7A 83
  84 85 86 87 88 89 8A 92 93 94 95 96 97 98 99
  9A A2 A3 A4 A5 A6 A7 A8 A9 AA B2 B3 B4 B5 B6
  B7 B8 B9 BA C2 C3 C4 C5 C6 C7 C8 C9 CA D2 D3
  D4 D5 D6 D7 D8 D9 DA E1 E2 E3 E4 E5 E6 E7 E8
  E9 EA F1 F2 F3 F4 F5 F6 F7 F8 F9 FA
```

**Chrominance AC table** (162 symbols):

```
Code lengths (codes of bit-length 1..16):
  0  2  1  2  4  4  3  4  7  5  4  4  0  1  2  119

Code values (162 bytes, hex):
  00 01 02 03 11 04 05 21 31 06 12 41 51 07 61
  71 13 22 32 81 08 14 42 91 A1 B1 C1 09 23 33
  52 F0 15 62 72 D1 0A 16 24 34 E1 25 F1 17 18
  19 1A 26 27 28 29 2A 35 36 37 38 39 3A 43 44
  45 46 47 48 49 4A 53 54 55 56 57 58 59 5A 63
  64 65 66 67 68 69 6A 73 74 75 76 77 78 79 7A
  82 83 84 85 86 87 88 89 8A 92 93 94 95 96 97
  98 99 9A A2 A3 A4 A5 A6 A7 A8 A9 AA B2 B3 B4
  B5 B6 B7 B8 B9 BA C2 C3 C4 C5 C6 C7 C8 C9 CA
  D2 D3 D4 D5 D6 D7 D8 D9 DA E2 E3 E4 E5 E6 E7
  E8 E9 EA F2 F3 F4 F5 F6 F7 F8 F9 FA
```

To build a Huffman code table from counts + values:

```
1. Start with code = 0, length = 1
2. For each bit-length L from 1 to 16:
     For each code value in values_of_length[L]:
       assign (value → code, L bits)
       code += 1
     code <<= 1    // next length: shift left
```

This produces canonical Huffman codes: the shortest codes go to the most-
frequent symbols, and the codes are numerically ordered within each length group.

### Step 7 — Byte Stuffing

The entropy-coded bitstream is written directly into the file between the SOS
header and the EOI marker, with no framing. Because the decoder searches the
stream for marker bytes (`FF`), any `0xFF` that appears in the entropy data would
be misinterpreted as a marker prefix.

**Byte stuffing** prevents this: after every `0xFF` byte in the entropy stream,
the encoder inserts a `0x00` byte. The decoder discards `0x00` bytes that
follow `0xFF` in the entropy region.

```
entropy byte 0xFF → output bytes 0xFF 0x00
any other byte b  → output byte b (unchanged)
```

Note that actual marker bytes in the entropy stream (e.g., restart markers
`FF D0`–`FF D7`) do appear and are valid; the decoder distinguishes them from
stuffed bytes by the fact that a stuffed byte always has `0x00` as its second
byte, while restart markers have `D0`–`D7`.

### Step 8 — JFIF Container Assembly

Write the segments in order:

```
1. SOI         (2 bytes, no length)
2. APP0        (18 bytes + 2-byte length field = 20 bytes total)
3. DQT luma    (2 marker + 2 length + 1 precision/ID + 64 table = 69 bytes)
4. DQT chroma  (69 bytes)
5. SOF0        (2 marker + 2 length + 1 precision + 2 height + 2 width +
                1 count + 3×3 component specs = 19 bytes)
6. DHT luma DC (2 marker + 2 length + 1 class/ID + 16 counts + N values)
7. DHT luma AC
8. DHT chroma DC
9. DHT chroma AC
10. SOS        (scan header + entropy-coded segment)
11. EOI        (2 bytes, no length)
```

**Segment length field** is always big-endian u16 and always includes the 2
length bytes themselves. For example, a DQT segment with 64 table bytes and 1
precision/ID byte has `length = 2 + 1 + 64 = 67 = 0x0043`.

---

## Decoding Pipeline

Decoding is the reverse of encoding. The decoder scans markers sequentially,
parses each segment, then uses the gathered tables to entropy-decode and
inverse-transform the image data.

### Step 1 — Find and Validate SOI

```
if bytes[0..2] != [0xFF, 0xD8]:
    return Err("JPEG decode: missing SOI marker")
pos = 2
```

### Step 2 — Parse APP0

Find the `FF E0` marker. Validate that the identifier string at offset 2 of the
segment body equals `"JFIF\0"` (5 bytes). Extract version and density fields
(informational; not needed for decode). Advance past the segment.

### Step 3 — Parse DQT Segments

For each `FF DB` marker:

```
length = big-endian u16 at pos+2   // includes itself
precision_and_id = byte at pos+4
precision = precision_and_id >> 4  // 0 = 8-bit table entries
table_id  = precision_and_id & 0xF // 0 = luma, 1 = chroma
```

Read 64 bytes (for 8-bit precision) or 128 bytes (for 16-bit precision, uncommon
in baseline) as the quantization table in zigzag order.

A JPEG file may have two DQT segments (luma + chroma) or one segment containing
both tables (distinguished by length). Store both tables indexed by `table_id`.

### Step 4 — Parse SOF0

Find `FF C0`. Extract:

```
precision = byte (must be 8 for baseline)
height    = big-endian u16
width     = big-endian u16
n_components = byte (1 = greyscale, 3 = YCbCr)
```

For each component:

```
component_id = byte (1, 2, or 3)
h_v_sampling = byte (high nibble = H, low nibble = V; both must be 1 for 4:4:4)
quant_table_id = byte (0 = luma table, 1 = chroma table)
```

### Step 5 — Parse DHT Segments

For each `FF C4` marker:

```
class_and_id = byte
class = class_and_id >> 4   // 0 = DC table, 1 = AC table
id    = class_and_id & 0xF  // 0 = luma, 1 = chroma
```

Read 16 bytes of code-length counts, then the code values in order.
Reconstruct canonical Huffman codes (same algorithm as encoding, Step 6).
Store four tables: (class=0,id=0), (class=0,id=1), (class=1,id=0), (class=1,id=1).

### Step 6 — Parse SOS and Decode Entropy Data

The SOS header specifies which Huffman tables each component uses. After the
SOS header, the entropy-coded segment begins.

**Bit reader**: read from the entropy stream MSB-first. Skip byte-stuffing:
whenever a `0xFF` byte is read from the stream, discard the following `0x00`
byte if present.

For each MCU (Minimum Coded Unit — one 8×8 block per component at 4:4:4):

For each component (Y, then Cb, then Cr):

```
// Decode DC coefficient
category = decode_huffman(dc_table[component])
diff = read_magnitude(category)     // category bits, MSB-first; negate if sign bit = 0
DC[component] = DC_prev[component] + diff
DC_prev[component] = DC[component]

// Decode AC coefficients (positions 1..63)
i = 1
while i < 64:
    symbol = decode_huffman(ac_table[component])  // one byte: (run, cat)
    run = symbol >> 4
    cat = symbol & 0xF
    if symbol == 0x00:   // EOB
        break            // remaining coefficients are zero
    if symbol == 0xF0:   // ZRL
        i += 16          // 16 zeros
        continue
    i += run             // skip `run` zero coefficients
    ac[i] = read_magnitude(cat)
    i += 1
```

### Step 7 — Dequantization

Multiply each zigzag-ordered coefficient by the corresponding quantization
table entry (using the table assigned to this component's channel):

```
F[zigzag_pos] = Q[zigzag_pos] * qtable[zigzag_pos]
```

Then reorder from zigzag back to 2-D block layout (inverse zigzag permutation).

### Step 8 — Inverse 2-D DCT

Apply the 2-D DCT-III (inverse DCT-II) to recover the spatial-domain block:

```rust
// Apply DCT-III to each column first, then each row
// (inverse of the encode order: encode was row then column)
for col in 0..8 {
    dsp_dct::dct(&column, DctType::III, DctNorm::None)
}
for row in 0..8 {
    dsp_dct::dct(&row, DctType::III, DctNorm::None)
}
```

Note: with `DctNorm::None`, the IDCT must be scaled by `1/(4N)` where N=8
(i.e., `1/32` per application of 1-D). The encode + decode combination
introduces a factor of `(2N)^2 = 256`; divide by 256 after the two IDCT passes
to recover the original scale, or use `DctNorm::Ortho` at both ends.

### Step 9 — Level Un-shifting and Clamp

Add 128 back to each sample (reversing the level shift from encoding Step 2):

```
pixel = clamp(idct_output + 128, 0, 255)
```

### Step 10 — YCbCr → RGB Conversion

BT.601 inverse transform:

```
R = Y                       + 1.402   · (Cr - 128)
G = Y - 0.344136 · (Cb - 128) - 0.714136 · (Cr - 128)
B = Y + 1.772   · (Cb - 128)
```

All results are clamped to [0, 255] and stored as u8.

**Integer-friendly approximation**:

```
// Scale factors × 2^16
R = clamp(Y_s + (91881 * (Cr - 128)) >> 16, 0, 255)
G = clamp(Y_s - (22554 * (Cb - 128) + 46802 * (Cr - 128)) >> 16, 0, 255)
B = clamp(Y_s + (116130 * (Cb - 128)) >> 16, 0, 255)
```

### Step 11 — Alpha Insertion and PixelContainer Assembly

JPEG has no alpha channel. The decoder sets A = 255 for every pixel and
constructs the output `PixelContainer`:

```rust
for each decoded (R, G, B):
    output.push(R);
    output.push(G);
    output.push(B);
    output.push(255);  // alpha = fully opaque

PixelContainer::from_data(width, height, output)
```

Discard padding rows/columns introduced during encoding (crop to the original
`width` and `height` declared in the SOF0 segment).

---

## Public API

```rust
/// Baseline JPEG encoder/decoder.
///
/// `quality` controls the compression ratio: 1 (worst quality, smallest file)
/// to 100 (best quality, largest file). Quality 75 is the industry default.
/// Quality 95 is suitable for archival purposes. Quality below 50 produces
/// visible artefacts (blockiness, colour banding).
pub struct JpegCodec {
    /// Compression quality, 1–100 inclusive.
    pub quality: u8,
}

impl JpegCodec {
    /// Create a new codec with the given quality.
    ///
    /// # Panics
    /// Does not panic; returns `Err` from `encode` / `decode` if quality is 0.
    pub fn new(quality: u8) -> Self {
        Self { quality }
    }
}

impl ImageCodec for JpegCodec {
    /// Always returns `"image/jpeg"`.
    fn mime_type(&self) -> &'static str {
        "image/jpeg"
    }

    /// Encode `pixels` as a JPEG byte stream.
    ///
    /// Returns `Err` only if `self.quality == 0` (quality must be 1–100).
    /// The output is a complete, self-contained JFIF file.
    fn encode(&self, pixels: &PixelContainer) -> Vec<u8>;

    /// Decode a JFIF/JPEG byte stream into a `PixelContainer`.
    ///
    /// Only baseline JPEG (SOF0) is supported. Returns `Err` with a
    /// descriptive message on any parse or unsupported-feature error.
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String>;
}

/// Encode with quality 75 (widely considered the perceptual-lossless default).
pub fn encode_jpeg(pixels: &PixelContainer) -> Vec<u8> {
    JpegCodec::new(75).encode(pixels)
}

/// Decode a JFIF/JPEG byte stream. Quality is not needed for decoding.
pub fn decode_jpeg(bytes: &[u8]) -> Result<PixelContainer, String> {
    JpegCodec::new(75).decode(bytes)
}

/// Library version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

---

## Error Cases

| Condition | Error message |
|-----------|---------------|
| First two bytes are not `FF D8` | `"JPEG decode: missing SOI marker"` |
| A segment extends past end of input | `"JPEG decode: truncated segment FF XX"` (XX = marker byte hex) |
| SOF marker is not SOF0 (e.g. SOF1/SOF2 = progressive) | `"JPEG decode: only baseline (SOF0) supported"` |
| Component count is not 1 or 3 | `"JPEG decode: only 1 or 3 components supported"` |
| Entropy decode references a DQT table not in the file | `"JPEG decode: quantization table N not found"` |
| Entropy decode references a DHT table not in the file | `"JPEG decode: Huffman table class C id N not found"` |
| SOF0 declares width or height of 0 | `"JPEG decode: zero width or height"` |
| `quality == 0` passed to `JpegCodec::new` | `"JPEG encode: quality must be 1–100"` |

---

## Round-Trip Properties

JPEG is **lossy**. The round-trip guarantee is intentionally weaker than for
BMP or QOI:

| Quality | Guarantee |
|---------|-----------|
| 100 | Every pixel channel is within ±2 of the original (rounding from float DCT) |
| ≥ 75 | PSNR ≥ 35 dB for natural images (industry "visually lossless" threshold) |
| Any | YCbCr ↔ RGB conversion is accurate within 1 LSB at quality=100 |
| Any | `decode(encode(img)).width == img.width` and same for height |

The ±2 error at quality=100 arises from floating-point rounding in the
DCT + IDCT pair and from the level shift, not from quantization (which is
nearly lossless at quality=100 since most qtable entries become 1).

---

## Teaching Notes

### JPEG vs QOI: spatial vs frequency domain

QOI (IC03) predicts "this pixel is close to the previous pixel" and codes the
small difference. JPEG asks a different question: "if I represent this 8×8
patch as a sum of cosine waves, which waves matter?" Both exploit spatial
redundancy, but JPEG's frequency-domain representation is much more powerful
for photographs with smooth gradients and complex textures.

| Codec | Domain | Lossless? | Good for |
|-------|--------|-----------|----------|
| BMP | Spatial (raw) | Yes | Simplicity, editing |
| QOI | Spatial (delta) | Yes | Fast encode/decode, exact pixels |
| PNG | Spatial (prediction + DEFLATE) | Yes | Screenshots, line art, icons |
| JPEG | Frequency (DCT) | No | Photographs, web images |

### Where the loss comes from

The encoding pipeline has exactly one lossy step: **quantization** (Step 4).
Every other step is reversible:
- Colour conversion: invertible within 1 LSB rounding
- DCT: perfectly invertible (given infinite precision)
- Zigzag: a pure permutation
- Huffman: lossless entropy coding

Increasing quality means dividing by smaller quantization table values, which
preserves more of the DCT coefficients at full precision. At quality=100, most
entries become 1 (divide-by-1 = no change), and only floating-point rounding
is lost.

### Why DCT, not wavelet or FFT?

- **Wavelet (JPEG 2000)**: wavelets handle the full image at once, avoiding
  visible 8×8 block boundaries ("blocking artefacts") at low quality. But
  baseline JPEG's 8×8 DCT is simpler and its hardware support is ubiquitous.
- **FFT**: the Discrete Fourier Transform produces complex outputs and assumes
  periodic signals (causing edge wrap-around). The DCT-II assumes an even
  reflection at the boundaries, which better matches the natural continuity of
  images. The DCT is also real-valued, halving storage.

### Blocking artefacts

At low quality (≤ 30), the block boundaries become visible as a grid of 8×8
tiles with discontinuous brightness. This "blocking" artefact is the signature
tell of aggressive JPEG compression. It happens because each block is quantized
independently: the encoder doesn't know that a coefficient in one block should
"agree" with its neighbour.

### Chroma subsampling (4:2:0, future v2)

In v1 we use **4:4:4** sampling: Y, Cb, Cr are all at full resolution. Most
real-world JPEG files use **4:2:0**: Cb and Cr are stored at half horizontal
and half vertical resolution. Because the eye is less sensitive to colour than
brightness, this halves chroma data with little visible impact. V2 of this
crate will add 4:2:0 support.

### Connecting to DSP02 and CMP04

- **DSP02** (DCT spec): Steps 3 and 8 above are a direct application of
  `dct_2d` / `idct_2d` from DSP02's Phase 4 API.
- **CMP04** (Huffman spec): the `huffman-tree` crate builds the canonical
  Huffman codes at encode time; the standard table constants embedded in this
  crate are the code tables from Annex K of T.81.

---

## Crate Layout

```
code/packages/rust/image-codec-jpeg/
├── Cargo.toml        (deps: dsp-dct, huffman-tree, pixel-container,
│                            paint-instructions)
├── BUILD
├── README.md
├── CHANGELOG.md
└── src/
    ├── lib.rs          (JpegCodec, VERSION, encode_jpeg, decode_jpeg,
    │                    ImageCodec impl)
    ├── color.rs        (rgb_to_ycbcr, ycbcr_to_rgb — BT.601 forward/inverse)
    ├── quantize.rs     (LUMA_QTABLE, CHROMA_QTABLE constants; quality_scale;
    │                    quantize_block, dequantize_block)
    ├── entropy.rs      (BitWriter, BitReader; zigzag permutation tables;
    │                    Huffman encode/decode; RLE-AC encode/decode;
    │                    standard Huffman table constants from Annex K)
    ├── encoder.rs      (encode_jpeg internals: colour-convert, block-extract,
    │                    DCT, quantize, entropy-code, assemble JFIF markers)
    └── decoder.rs      (JFIF marker scanner; SOI/APP0/DQT/SOF0/DHT/SOS parsers;
                         entropy-decode; IDCT; colour-convert; PixelContainer build)
```

### Module responsibilities

| Module | Responsibility |
|--------|---------------|
| `color.rs` | BT.601 RGB↔YCbCr conversions, integer and float paths |
| `quantize.rs` | Standard table constants, quality scaling, quantize/dequantize |
| `entropy.rs` | Bit I/O (MSB-first), zigzag table, Huffman build/encode/decode, RLE-AC |
| `encoder.rs` | Orchestrates Steps 1–8; writes JFIF markers |
| `decoder.rs` | Scans marker stream; dispatches to parsers; orchestrates decode Steps 1–11 |
| `lib.rs` | Public `JpegCodec` struct, `encode_jpeg`, `decode_jpeg`, `ImageCodec` impl |

---

## Dependencies

| Crate | Used for |
|-------|---------|
| `dsp-dct` | Forward DCT-II (encode Step 3) and inverse DCT-III (decode Step 8) |
| `huffman-tree` | Building canonical Huffman code tables from count arrays at encode time |
| `pixel-container` | `PixelContainer` input/output type and `ImageCodec` trait |
| `paint-instructions` | `ImageCodec` trait definition |

No external (crates.io) libraries. No `unsafe`. No `std::io`; all I/O through
`&[u8]` slices and `Vec<u8>`.
