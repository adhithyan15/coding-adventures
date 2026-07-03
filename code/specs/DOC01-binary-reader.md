# DOC01 — Legacy `.doc` (Word 97–2003 binary) text reader

**Layer:** B2 (readers). Sits on top of the merged **`cfb`** crate (CFB01).
**Crate:** `code/packages/rust/doc/`
**Status:** implemented, v0.1.0.

This spec is *literate*: read it top to bottom and you will understand exactly
how the main-document text is pulled out of a legacy Word binary file, and why
every step is bounds-checked. The implementation in `src/lib.rs` mirrors this
document section-for-section.

---

## 1. What a `.doc` file actually is

A Word 97–2003 `.doc` is **not** a single flat blob of text. It is an
**OLE2 Compound File** (a "filesystem in a file" — see [MS-CFB], handled by our
`cfb` crate). Inside that container are several named *streams*. The two we care
about:

| Stream          | Purpose                                                        |
|-----------------|----------------------------------------------------------------|
| `WordDocument`  | Starts with the **FIB** header; also holds the raw text bytes. |
| `0Table`/`1Table` | The **Table stream**; holds the **CLX** (piece table) and lots of other structures we ignore. |

Exactly one of `0Table` / `1Table` is present; a flag in the FIB tells us which.

The reason text lives in two places is *editing history*: as a user cut, pasted,
and retyped, Word appended new text runs to `WordDocument` and recorded, in a
**piece table**, the order in which those scattered runs must be glued back
together. To recover the logical document you must replay that piece table.

```
  .doc file (bytes)
        │  cfb::CompoundFile::open
        ▼
   ┌──────────────────────────────────────────┐
   │ CFB container                             │
   │   ├── "WordDocument"  ← FIB + text runs   │
   │   └── "1Table" (or "0Table") ← CLX/PlcPcd │
   └──────────────────────────────────────────┘
```

---

## 2. The FIB (File Information Block)

The `WordDocument` stream *begins* with the FIB. We only need four fields, at
fixed little-endian byte offsets (verified against [MS-DOC] and Apache POI):

| Offset | Type  | Name           | Meaning                                                     |
|-------:|-------|----------------|-------------------------------------------------------------|
| `0x000`| `u16` | `wIdent`       | Magic. MUST be `0xA5EC`. Otherwise not a Word doc.          |
| `0x00A`| `u16` | FibBase flags  | Bit `0x0200` = **fWhichTblStm**.                            |
| `0x1A2`| `u32` | `fcClx`        | Byte offset of the CLX **within the Table stream**.         |
| `0x1A6`| `u32` | `lcbClx`       | Byte length of the CLX.                                     |

**fWhichTblStm** (`flags & 0x0200`): if the bit is **set**, the table stream is
named `"1Table"`; if **clear**, it is `"0Table"`. Pick the right one — the other
may not exist at all.

> ⚠️ **Trailing zero padding.** CFB pads every stream up to a sector boundary
> with zero bytes. So `read_stream("WordDocument")` returns *more* bytes than the
> logical content, and `read_stream("1Table")` likewise. **Never** treat the
> returned `Vec` length as the logical length. Always drive reads from the FIB
> numbers (`fcClx`, `lcbClx`) and from the CP counts (below), and bounds-check
> every slice against the *actual* returned length.

---

## 3. The CLX and its parts (Prc / Pcdt)

`clx = table_stream[fcClx .. fcClx + lcbClx]` (bounds-checked!).

The CLX is a *sequence of parts*. Each part begins with a one-byte tag `clxt`:

- `clxt == 0x01` — **Prc**: followed by `i16 cbGrpprl` (2 bytes), then
  `cbGrpprl` bytes of property data we **skip**. (These describe formatting, not
  text.) Advance past it and read the next part.
- `clxt == 0x02` — **Pcdt**: followed by `u32 lcb`, then `lcb` bytes of
  **PlcPcd** — the piece table itself. **This is the payload we want.** Read it
  and stop.

A minimal file has just the Pcdt. Real files may have zero or more Prc parts
first. The parse loop **must always advance** (consume the tag byte + its
declared payload) or reject — a zero-length/short part must not spin forever.

---

## 4. The PlcPcd (the piece table)

A `Plc` ("plex") is a packed pair of parallel arrays:

```
  PlcPcd layout  (total length = lcb bytes)
  ┌───────────────────────────────┬───────────────────────────────┐
  │  CP array: (n+1) × u32         │  PCD array: n × 8 bytes        │
  │  cp[0] cp[1] ... cp[n]         │  pcd[0] pcd[1] ... pcd[n-1]    │
  └───────────────────────────────┴───────────────────────────────┘
        character positions              per-piece descriptors
```

There are `n` pieces, `n+1` character positions (CPs), and `n` piece descriptors
(PCDs). Each CP is 4 bytes, each PCD is 8 bytes, so:

```
  lcb = (n + 1) * 4  +  n * 8   ⇒   n = (lcb - 4) / 12
```

**Validation:** require `lcb >= 4` and `(lcb - 4) % 12 == 0`, else the piece
table is malformed → error. Also cap `n` so a lying table cannot drive huge
allocation.

- **CP array** `cp[0..=n]`: `cp[i]` is the character position where piece `i`
  begins. The number of characters in piece `i` is `cp[i+1] - cp[i]`.
- **PCD array** `pcd[0..n]`, each 8 bytes:
  - `u16` flags @0 — ignored.
  - `u32` **FcCompressed** @2 — the interesting field (below).
  - `u16` prm @6 — ignored.

---

## 5. FcCompressed — the bit trick that locates + decodes a piece

`FcCompressed` is a `u32` that packs a *flag* and an *offset*:

- `fCompressed = (fc & 0x4000_0000) != 0`  — **bit 30**.
- `real        =  fc & 0x3FFF_FFFF`         — the low 30 bits.

Two cases:

| `fCompressed` | Encoding      | Byte position in `WordDocument` | Bytes / char |
|:-------------:|---------------|---------------------------------|:------------:|
| **true**      | 8-bit (CP-1252 / Latin-1) | `real / 2`          | 1            |
| **false**     | 16-bit (UTF-16LE)         | `real`              | 2            |

So for a piece with `count = cp[i+1] - cp[i]` characters:

- **Compressed:** read `count` bytes starting at `real/2`; each byte `b` maps to
  the Unicode scalar `U+00bb` (Latin-1). (CP-1252 differs only in `0x80..=0x9F`;
  Latin-1 is faithful for the fixture and all ASCII.)
- **Uncompressed:** read `count * 2` bytes starting at `real`; decode as
  little-endian UTF-16 (surrogate-pair aware).

Concatenate the decoded pieces **in array order** (0..n) → the main text.

### Worked example (the fixture)

The fixture's one piece has `FcCompressed = 0x4000_0300` roughly (bit-30 set):

```
  fc          = 0100_0000 0000_0000 0000_0011 0000_0000   (binary)
  fCompressed = fc & 0x40000000  = nonzero → true (8-bit)
  real        = fc & 0x3FFFFFFF  = 0x00000300 = 768
  byte offset = real / 2         = 384          (0x180 into WordDocument)
```

At WordDocument offset 384 sit the bytes `48 65 6C 6C 6F 2C 20 44 4F 43 21`
= ASCII `"Hello, DOC!"` (11 chars). `cp = [0, 11]`, so `count = 11`. One
compressed piece → the text is exactly `"Hello, DOC!"`.

---

## 6. The algorithm (end to end)

1. `cfb::CompoundFile::open(bytes)` — any CFB error → `DocError::Cfb`.
2. `read_stream("WordDocument")` — absent → `NotWordDocument`.
3. Check `u16 @ 0 == 0xA5EC` — else `NotWordDocument`.
4. Read fWhichTblStm from `u16 @ 10`; choose `"1Table"` / `"0Table"`.
5. `read_stream(name)` — absent → `NoTableStream`.
6. `fcClx = u32 @ 0x1A2`, `lcbClx = u32 @ 0x1A6`.
7. Slice `clx` (bounds-checked) → `MalformedPieceTable`/`Truncated` on overrun.
8. Walk CLX parts, skipping Prc, until Pcdt → get `PlcPcd`.
9. Parse CPs + PCDs (`n = (lcb-4)/12`), decode each piece, concatenate.

Every offset/length derived from the file — `fcClx`, `lcbClx`, `real/2`,
`count`, `count*2`, and every array index — is checked against the *actual*
stream length with `checked_*` arithmetic and `get(..)` before use. A hostile
value yields a typed error, never a panic.

---

## 7. Security model

Input is **attacker-controlled**. Guarantees:

- `#![forbid(unsafe_code)]`. No `unwrap`/`expect`/`panic!`/panicking index.
- All arithmetic on untrusted values uses `checked_add`/`checked_mul`; all
  slicing uses `get(..)` / `get(..).ok_or(...)`.
- Total decoded text is capped (`MAX_TEXT_BYTES` = 64 MiB) and piece count is
  capped (`MAX_PIECES`) so a lying piece table cannot exhaust memory. Nothing is
  pre-allocated to an unvalidated declared count.
- The CLX-part loop is guaranteed to make progress on every iteration; a
  malformed/short part is rejected rather than looping.

## 8. Public API

```rust
pub fn open_doc(bytes: &[u8]) -> Result<Document, DocError>;
pub struct Document { /* reassembled main text */ }
impl Document { pub fn text(&self) -> &str; }
pub enum DocError { Cfb(cfb::CfbError), NotWordDocument, NoTableStream,
                    MalformedPieceTable, Truncated }
// Display + std::error::Error (with source()) + From<cfb::CfbError>.
```

An internal seam `extract_text(word_document, table, fc_clx, lcb_clx)` lets the
piece-table logic be unit-tested with synthetic byte arrays, without building a
whole CFB.
