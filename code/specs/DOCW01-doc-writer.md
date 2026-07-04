# DOCW01 — Legacy `.doc` (Word 97-2003) writer

> A **writer** for the legacy **`.doc`** binary format ([MS-DOC]) — the
> Word 97-2003 document. You give it a simple document model (paragraphs of
> text); it produces a valid `.doc` byte buffer by emitting the **FIB** (File
> Information Block), a **piece table** (CLX), and the text, all wrapped in an
> **OLE2 / Compound File Binary** container via the [CFBW01](CFBW01-cfb-writer.md)
> writer. This is milestone **C4** of the OOXML/legacy-Office stack.

This spec assumes you have read [CFBW01](CFBW01-cfb-writer.md) and its reader
sibling [CFB01](CFB01-compound-file.md). Those explain the CFB container ("a FAT
filesystem crammed into a single file"). Here we focus one level up: what
*streams* a `.doc` puts inside that container, and how Word finds the document
text through a **piece table**.

Implemented by `code/packages/rust/doc-writer` (crate `doc-writer`).

---

## 1. The one-paragraph mental model

A `.doc` file is a CFB container holding (among others) two streams:

- **`WordDocument`** — starts with the **FIB**, a big fixed-layout header of
  offsets and lengths. The document's *characters* also live in this stream, but
  **not** at a fixed place: you must follow a pointer chain to find them.
- **`1Table`** (or `0Table`) — holds the **CLX**, which contains the **piece
  table** (`PlcPcd`). The piece table is the crucial indirection: it maps
  *character positions* (CPs) in the logical document to *byte offsets* (FCs) in
  the `WordDocument` stream, and records whether each run of text is stored as
  **8-bit** (one Latin-1 byte per char) or **16-bit** (UTF-16LE) bytes.

So the retrieval algorithm — the thing every `.doc` reader does, and the thing
our round-trip test re-implements — is:

```text
FIB (in WordDocument)  ── fcClx/lcbClx ──▶  CLX (in 1Table)
                                                │
                                          PlcPcd (piece table)
                                                │
                                   for each piece: FcCompressed
                                                │
                        ┌───────────────────────┴───────────────────────┐
                        │ bit 30 set → 8-bit text at (fc & 0x3FFFFFFF)/2 │
                        │ bit 30 clear → 16-bit text at fc               │
                        └───────────────────────┬───────────────────────┘
                                                ▼
                                    bytes in WordDocument → characters
```

Why the indirection? Historically Word edited documents *in place* by keeping
the original text and appending edits, then rewriting only the piece table to
splice pieces into the new logical order — an early persistent-rope. We don't
need that power (we always emit **one** piece covering the whole document), but
we must still emit a well-formed piece table because that is the only way a
reader locates the text.

---

## 2. The output we commit to

A **single-piece** `.doc`. The whole document is one run of text stored once,
either 8-bit compressed (the common, compact case) or 16-bit (the faithful
fallback when any character is outside Latin-1, i.e. `> U+00FF`).

Two streams go into the CFB:

| Stream         | Contents                                               |
|----------------|--------------------------------------------------------|
| `WordDocument` | FIB (fixed header) + the text bytes at a fixed offset  |
| `1Table`       | the CLX (a `Pcdt` wrapping the `PlcPcd` piece table)   |

The FIB's `fWhichTblStm` flag says whether the table stream is named `0Table` or
`1Table`. We always set it to **1** → `1Table`.

---

## 3. The `WordDocument` stream — FIB + text

The FIB is a large structure; a reader only needs a handful of its fields to
find the text. We build a buffer of length `TEXT_OFFSET + text_bytes` where
`TEXT_OFFSET = 2048` — a fixed location safely past the FIB — and set exactly
the fields below. Every other byte is zero.

| Offset (dec / hex) | Type  | Value    | Field / meaning                                        |
|--------------------|-------|----------|--------------------------------------------------------|
| 0                  | u16   | `0xA5EC` | `wIdent` — the magic that identifies a Word FIB        |
| 2                  | u16   | `0x00C1` | `nFib` — FIB version (Word 97 = 193 = `0x00C1`)        |
| 10  / 0x0A         | u16   | `0x0200` | FibBase flags; bit 9 = `fWhichTblStm` = 1 → `1Table`   |
| 32  / 0x20         | u16   | `0x000E` | `csw` = 14 (count of 16-bit values in `fibRgW`)        |
| 62  / 0x3E         | u16   | `0x0016` | `cslw` = 22 (count of 32-bit values in `fibRgLw`)      |
| 76  / 0x4C         | u32   | ccpText  | character count of the main document (informational)   |
| 152 / 0x98         | u16   | `0x005D` | `cbRgFcLcb` = 93 (count of FC/LCB pairs in `fibRgFcLcbBlob`) |
| 418 / 0x1A2        | u32   | `0`      | `fcClx` — byte offset of the CLX in the table stream    |
| 422 / 0x1A6        | u32   | lcbClx   | `lcbClx` — byte length of the CLX                       |

The `fWhichTblStm` bit is bit **9** of the u16 at offset 10, i.e. `0x0200`.

### 3.1 The text bytes

We store the document text **8-bit compressed** when every character is
`<= U+00FF`: one Latin-1 byte per character, written at byte offset
`TEXT_OFFSET`. When any character exceeds `U+00FF`, we fall back to **16-bit**:
two UTF-16LE bytes per character, still at `TEXT_OFFSET`. The piece table's
`FcCompressed` (below) tells the reader which encoding to expect.

Paragraphs are joined with a single **carriage return** `\r` (`0x0D`), Word's
paragraph mark, into one text run.

---

## 4. The `1Table` stream — the CLX

The CLX we emit is the simplest legal one: a single **`Pcdt`** (piece table
container). Its bytes:

```text
  u8   clxt   = 0x02        ── this CLX component is a Pcdt
  u32  lcb                  ── byte length of the PlcPcd that follows
  ── PlcPcd (lcb bytes) ──
  CP array : u32 0, u32 n   ── (m+1) CPs for m pieces; here 2 CPs, 1 piece
  PCD array: one 8-byte PCD ── u16 fFlags, u32 FcCompressed, u16 prm
```

A **`PLC`** (**P**osition-**L**ength-**C**ombined, "plex") is a packed pair of
parallel arrays: first `m+1` **CP** (character position) `u32`s, then `m`
fixed-size data records. For the piece table the records are 8-byte **PCD**s and
`m` is the number of pieces. The CP array is *cumulative*: CP[i] is the first
character of piece i, and CP[m] is one past the last character — so a single
piece covering `n` characters has CP array `{0, n}`.

For our single piece:

- CP array = `u32 0`, `u32 n_chars` → 8 bytes.
- PCD array = one 8-byte PCD: `u16 0` (flags), `u32 FcCompressed`, `u16 0` (prm).
- So `lcb = 8 (CPs) + 8 (PCD) = 16`, and the whole CLX is `1 + 4 + 16 = 21` bytes.

A reader recovers the piece count as `n = (lcb - 4) / 12` — because a `PlcPcd`
with `n` pieces is `4*(n+1)` CP bytes + `8*n` PCD bytes = `12*n + 4`.

### 4.1 `FcCompressed` — the bit-30 trick

The middle `u32` of a PCD is an **`FcCompressed`**. Bit 30 (`0x40000000`) is the
`fCompressed` flag. It overloads one field to carry both a byte offset *and* the
encoding:

- **8-bit (compressed):** set bit 30, and store the offset **doubled**. The
  reader computes the real byte offset as `(fc & 0x3FFFFFFF) / 2`. So for
  `TEXT_OFFSET = 2048`: `fc = (2048 * 2) | 0x40000000 = 4096 | 0x40000000 =`
  **`0x40001000`**. Text is one byte per char at the real offset.
- **16-bit (uncompressed):** clear bit 30, and store the offset **as-is** (not
  doubled). The reader uses `fc & 0x3FFFFFFF` directly. Text is two UTF-16LE
  bytes per char.

Why doubled in the 8-bit case? The historical intent is that `fc` addresses a
16-bit code-unit stream; halving it recovers the byte offset, and the low bit
being consumed by the `/2` is why the offset is stored doubled. We simply mirror
what every reader does.

---

## 5. Wrapping in CFB

We hand the two streams to the CFB writer in this order:

```rust
cfb_writer::write_cfb(&[("WordDocument", &wd), ("1Table", &table)])
```

The result is the finished `.doc` byte buffer, opening with the CFB signature
`D0 CF 11 E0 A1 B1 1A E1`.

---

## 6. The round-trip proof

The correctness proof (and the required test) does not trust our own reader-side
helpers — it **re-implements** the retrieval from first principles:

1. Open the bytes with the `cfb` reader; read `WordDocument` and `1Table`.
2. From `WordDocument`: check `wIdent == 0xA5EC`; read the `fWhichTblStm` bit;
   read `fcClx` (@418) and `lcbClx` (@422).
3. Slice the CLX out of `1Table` at `[fcClx .. fcClx + lcbClx]`.
4. Parse `clxt == 0x02`, then `lcb`, then the `PlcPcd`. Piece count
   `n = (lcb - 4) / 12`.
5. For each PCD: decode `FcCompressed` → (real offset, compressed flag);
   read `CP[i+1] - CP[i]` characters from `WordDocument` at that offset, in the
   indicated encoding.
6. Concatenate the pieces → the reassembled document text; assert it equals the
   input paragraphs joined by `\r`.

---

## 7. Robustness

The writer takes **trusted** input (a `Document` the caller built), but it is
still **total**: `#![forbid(unsafe_code)]`, no `unwrap`/`expect`/`panic!` on the
public path, and `checked_*` arithmetic wherever a colossal text could overflow
an offset or length. If the text is so large that `TEXT_OFFSET + len` (8-bit) or
`TEXT_OFFSET + 2*len` (16-bit) would overflow `usize`, or the doubled 8-bit
`FcCompressed` offset would collide with bit 30, we fall back to emitting an
empty document rather than producing corrupt bytes. Output is **deterministic**.

---

## 8. What we deliberately leave out (future extensions)

- **Multiple pieces.** Real Word splices many pieces; we always emit one. The
  piece-table code is already general enough to describe more; the writer just
  never produces them.
- **Mixed 8-bit/16-bit pieces.** We pick one encoding for the whole document.
- **Formatting** (character/paragraph properties, `Plcfbte*`, style sheet,
  `Prm`s): none. A reader that only wants the text sees a clean single run.

These are intentional simplifications for milestone C4; the format leaves room
for all of them.
