# IC03 — `image-codec-qoi`: QOI "Quite OK Image" Encoder/Decoder

## Overview

QOI (Quite OK Image Format) was designed by Dominic Szablewski in 2021 as a
replacement for PNG in use cases where simplicity and speed matter more than
maximum compression ratio. The full specification fits in a few pages; a
working implementation is typically 200–300 lines.

QOI introduces three classic compression primitives that also appear in more
complex codecs like JPEG and WebP lossless:

1. **Run-length encoding** — repeat the previous pixel N times with one byte
2. **Hash back-references** — if the pixel appeared recently (in a 64-slot
   hash table), reference it by index
3. **Delta coding** — store small RGB differences instead of full values

Understanding QOI is an excellent stepping stone toward JPEG (delta coding,
Huffman coding), WebP lossless (LZ77 back-references, Brotli), and AV1
intra-prediction.

The canonical reference is the QOI specification at qoiformat.org.

---

## File Structure

```
┌───────────────────────────────────┐
│  Header       (14 bytes)          │
├───────────────────────────────────┤
│  Encoded chunks (variable length) │
├───────────────────────────────────┤
│  End marker   (8 bytes)           │
└───────────────────────────────────┘
```

### Header (14 bytes, big-endian)

| Offset | Size | Field        | Value / Notes                         |
|--------|------|--------------|---------------------------------------|
| 0      | 4    | `magic`      | `b"qoif"` — ASCII signature           |
| 4      | 4    | `width`      | `u32` big-endian                      |
| 8      | 4    | `height`     | `u32` big-endian                      |
| 12     | 1    | `channels`   | `3` = RGB, `4` = RGBA                 |
| 13     | 1    | `colorspace` | `0` = sRGB with linear alpha, `1` = all channels linear |

`channels` tells the decoder how many channels to fill in the `PixelContainer`.
For RGB images, the alpha channel is always treated as 255 during decode.

All multi-byte integers in the QOI header are **big-endian** (most-significant
byte first), opposite to BMP.

### End Marker (8 bytes)

The encoded chunk stream ends with a fixed 8-byte sentinel:

```
[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]
```

A decoder reads chunks until it has decoded `width * height` pixels, then
verifies the next 8 bytes match this sentinel (or simply stops when the pixel
count is reached).

---

## Six Encoding Operations

Every pixel is encoded by exactly one chunk. The encoder chooses the most
compact applicable operation in priority order.

### State Carried Between Pixels

The encoder and decoder maintain identical state:

```
previous_pixel: (r, g, b, a) = (0, 0, 0, 255)   // starts fully opaque black
hash_table:     [(r, g, b, a); 64] = all zeros
run_length:     u8 = 0
```

The hash table is indexed by:

```
index = (r as u32 * 3 + g as u32 * 5 + b as u32 * 7 + a as u32 * 11) % 64
```

This is not a cryptographic hash — it is a fast mixing function that spreads
pixel values across 64 buckets. After emitting any non-RUN chunk, the
encoder/decoder writes the current pixel into `hash_table[index]`.

### Operation 1 — `QOI_OP_RGB` (0xFE, 4 bytes total)

```
Byte 0: 0xFE
Byte 1: r
Byte 2: g
Byte 3: b
```

Emit a full RGB value. Alpha is unchanged from the previous pixel.

Used when: no other operation applies and the alpha channel did not change.

### Operation 2 — `QOI_OP_RGBA` (0xFF, 5 bytes total)

```
Byte 0: 0xFF
Byte 1: r
Byte 2: g
Byte 3: b
Byte 4: a
```

Emit a full RGBA value. Used when: alpha changed and no other operation applies.

### Operation 3 — `QOI_OP_INDEX` (2-bit tag 00, 1 byte total)

```
Bits 7–6: 0b00
Bits 5–0: index (0–63)
```

```
Byte 0: 0b00_iiiiii   where iiiiii = hash_table_index
```

Reference a pixel from the hash table. Used when: `hash_table[hash(pixel)] == pixel`.

This is a back-reference. If the same pixel appeared recently and landed in the
same hash slot, the encoder can replace the full RGBA value with a single byte.

### Operation 4 — `QOI_OP_DIFF` (2-bit tag 01, 1 byte total)

```
Bits 7–6: 0b01
Bits 5–4: dr + 2   (delta red,   biased by +2 so range [-2, 1] maps to [0, 3])
Bits 3–2: dg + 2   (delta green, same bias)
Bits 1–0: db + 2   (delta blue,  same bias)
```

```
Byte 0: 0b01_drdgdb   where dr, dg, db ∈ [−2, 1] after subtracting bias
```

Store small per-channel deltas. Alpha must be unchanged. Each delta is biased
by +2 so it fits in 2 bits without sign extension:

```
dr = (current.r as i16) - (previous.r as i16)
dg = (current.g as i16) - (previous.g as i16)
db = (current.b as i16) - (previous.b as i16)
```

Used when: alpha unchanged AND all three deltas are in `[−2, 1]`.

Deltas wrap modulo 256 (treating r, g, b as u8 arithmetic during decode).

### Operation 5 — `QOI_OP_LUMA` (2-bit tag 10, 2 bytes total)

```
Byte 0: 0b10_dddddd   where dddddd = dg + 32  (green delta, range [-32, 31])
Byte 1: 0b_(dr-dg+8)_(db-dg+8)
         bits 7–4: dr - dg + 8
         bits 3–0: db - dg + 8
```

LUMA stores the green delta in 6 bits and the red/blue deltas relative to
green in 4 bits each. This exploits the fact that in natural images, colour
channels tend to move together — a pixel that gets brighter usually increases
all three channels by similar amounts.

```
dg    ∈ [−32, 31]    stored as dg + 32 in 6 bits
dr−dg ∈ [−8, 7]     stored as (dr−dg) + 8 in 4 bits
db−dg ∈ [−8, 7]     stored as (db−dg) + 8 in 4 bits
```

Used when: alpha unchanged AND `QOI_OP_DIFF` does not apply AND the above
delta ranges are satisfied.

### Operation 6 — `QOI_OP_RUN` (2-bit tag 11, 1 byte total)

```
Bits 7–6: 0b11
Bits 5–0: run_length − 1   (bias of −1 so run of 1 maps to 0)
```

```
Byte 0: 0b11_rrrrrr   where rrrrrr = run_length − 1
```

Repeat the previous pixel `run_length` times. Run length is in `[1, 62]`
(values 63 and 64 are reserved; the biased range 62–63 would be 0b111110 and
0b111111, which are reserved to avoid conflict with the end marker bytes).

The run counter increments while the current pixel equals the previous pixel.
When the pixel changes or the run reaches 62, the RUN chunk is emitted.

---

## Encoder Algorithm

```
1. Write header (14 bytes, big-endian)

2. Initialise state:
     prev = (0, 0, 0, 255)
     hash_table = [(0, 0, 0, 0); 64]
     run = 0

3. For each pixel p (row-major):
     if p == prev:
       run += 1
       if run == 62:
         emit QOI_OP_RUN(62); run = 0
       continue

     // flush pending run
     if run > 0:
       emit QOI_OP_RUN(run); run = 0

     idx = hash(p)
     if hash_table[idx] == p:
       emit QOI_OP_INDEX(idx)
     else:
       hash_table[idx] = p
       dr = p.r − prev.r   (wrapping i8 arithmetic)
       dg = p.g − prev.g
       db = p.b − prev.b
       if p.a == prev.a:
         if dr ∈ [−2,1] && dg ∈ [−2,1] && db ∈ [−2,1]:
           emit QOI_OP_DIFF(dr, dg, db)
         elif dg ∈ [−32,31] && (dr−dg) ∈ [−8,7] && (db−dg) ∈ [−8,7]:
           emit QOI_OP_LUMA(dg, dr−dg, db−dg)
         else:
           emit QOI_OP_RGB(p.r, p.g, p.b)
       else:
         emit QOI_OP_RGBA(p.r, p.g, p.b, p.a)

     prev = p

4. Flush any remaining run:
     if run > 0: emit QOI_OP_RUN(run)

5. Write end marker: [0,0,0,0, 0,0,0,1]
```

---

## Decoder Algorithm

```
1. Verify magic: bytes[0..4] == b"qoif"
2. Read width, height (u32 BE)
3. Read channels (u8), colorspace (u8)
4. Initialise state:
     prev = (0, 0, 0, 255)
     hash_table = [(0, 0, 0, 0); 64]
     pixels_remaining = width * height

5. While pixels_remaining > 0:
     read byte tag

     if tag == 0xFE:             // QOI_OP_RGB
       r, g, b = next 3 bytes
       p = (r, g, b, prev.a)
     elif tag == 0xFF:           // QOI_OP_RGBA
       r, g, b, a = next 4 bytes
       p = (r, g, b, a)
     elif tag >> 6 == 0b00:      // QOI_OP_INDEX
       p = hash_table[tag & 0x3F]
     elif tag >> 6 == 0b01:      // QOI_OP_DIFF
       dr = ((tag >> 4) & 0x3) - 2
       dg = ((tag >> 2) & 0x3) - 2
       db = ((tag >> 0) & 0x3) - 2
       p = (prev.r + dr, prev.g + dg, prev.b + db, prev.a)  // wrapping u8
     elif tag >> 6 == 0b10:      // QOI_OP_LUMA
       next_byte = read 1 byte
       dg     = (tag & 0x3F) - 32
       dr_dg  = ((next_byte >> 4) & 0xF) - 8
       db_dg  = ((next_byte >> 0) & 0xF) - 8
       dr = dr_dg + dg
       db = db_dg + dg
       p = (prev.r + dr, prev.g + dg, prev.b + db, prev.a)  // wrapping u8
     elif tag >> 6 == 0b11:      // QOI_OP_RUN
       run = (tag & 0x3F) + 1
       emit prev pixel `run` times into output
       pixels_remaining -= run
       continue (don't update hash or prev)

     hash_table[hash(p)] = p
     emit p into output
     pixels_remaining -= 1
     prev = p

6. Verify end marker: next 8 bytes == [0,0,0,0, 0,0,0,1]
7. Return Ok(PixelContainer::from_data(width, height, data))
```

---

## API

```rust
pub struct QoiCodec;

impl ImageCodec for QoiCodec {
    fn mime_type(&self) -> &'static str {
        "image/qoi"
    }

    fn encode(&self, container: &PixelContainer) -> Vec<u8>;
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String>;
}

/// Convenience wrapper: encode a PixelContainer to QOI bytes.
pub fn encode_qoi(container: &PixelContainer) -> Vec<u8> {
    QoiCodec.encode(container)
}

/// Convenience wrapper: decode QOI bytes into a PixelContainer.
pub fn decode_qoi(bytes: &[u8]) -> Result<PixelContainer, String> {
    QoiCodec.decode(bytes)
}
```

---

## Error Cases

| Condition | Error message |
|-----------|---------------|
| File shorter than 22 bytes (14 header + 8 end) | `"QOI: file too short"` |
| Magic bytes != `b"qoif"` | `"QOI: invalid magic"` |
| Width or height is zero | `"QOI: invalid dimensions"` |
| Chunk data truncated before end marker | `"QOI: unexpected end of data"` |
| End marker missing or incorrect | `"QOI: missing end marker"` |

---

## Round-Trip Property

For any valid `PixelContainer` `p`:

```rust
let encoded = QoiCodec.encode(&p);
let decoded  = QoiCodec.decode(&encoded).unwrap();
assert_eq!(p.width,  decoded.width);
assert_eq!(p.height, decoded.height);
assert_eq!(p.data,   decoded.data);
```

QOI is lossless; every RGBA value is preserved exactly.

---

## Teaching Notes

QOI is ideal for teaching compression fundamentals because all three major
codec primitives appear in a single, readable implementation:

| Technique | QOI operation | Also appears in... |
|-----------|---------------|--------------------|
| Run-length encoding | `QOI_OP_RUN` | PackBits, TIFF, fax formats |
| Hash back-references | `QOI_OP_INDEX` | LZ77/LZ78 (ZIP, gzip, PNG deflate), WebP lossless |
| Delta coding | `QOI_OP_DIFF`, `QOI_OP_LUMA` | JPEG DC coefficients, PNG filters, FLAC audio |

The LUMA operation specifically mirrors the YCbCr colour space transform in
JPEG: encode luminance (green) at full resolution, and store chrominance
(red/blue) as differences from luminance. This is why JPEG compression works
so well on natural images — human vision is much more sensitive to luminance
than colour.

---

## Crate Layout

```
code/packages/rust/image-codec-qoi/
├── Cargo.toml    # depends on pixel-container only
├── src/
│   └── lib.rs
├── BUILD
├── README.md
└── CHANGELOG.md
```

Dependencies: `pixel-container` only. No external libraries.
