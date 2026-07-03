# PPTW01 — Legacy PowerPoint (`.ppt`) writer ([MS-PPT])

> A **writer** for the legacy **PowerPoint 97-2003 binary** format (`.ppt`,
> [MS-PPT]). You hand it a slide-deck model — presentations made of slides made
> of paragraphs of text — and it produces the bytes of a real `.ppt` file. Under
> the hood it emits [MS-PPT] *records* into a "PowerPoint Document" stream and
> wraps that stream (plus a tiny "Current User" stream) in an OLE2 Compound File
> using the sibling [`cfb-writer`](CFBW01-cfb-writer.md) crate. This is milestone
> **C4** of the OOXML/binary-Office effort: the `.ppt` sibling to the `.xls`
> path.

Implemented by `code/packages/rust/ppt-writer` (crate `ppt-writer`).

This spec assumes you have read [CFBW01](CFBW01-cfb-writer.md), which explains
the container ("a FAT filesystem crammed into a single file"). Here we focus on
the *contents* of one stream inside that container: the tree of [MS-PPT]
records.

---

## 1. The one-paragraph mental model

A `.ppt` file is an OLE2 compound file (like `.xls` and `.doc`). The interesting
payload lives in a stream named exactly **"PowerPoint Document"**. That stream
is not flat bytes — it is a **tree of records**. Every record starts with an
8-byte **RecordHeader**, followed by a body. A record is either:

- a **container**, whose body is *more records* (children), or
- an **atom**, whose body is raw data (text, numbers, …).

To write a deck we emit, for each slide, a **Slide container** (`recType`
`0x03EE`) whose children are **text atoms** — one atom per paragraph. To read
one back, you walk the tree: read a header; if it is a container, recurse into
its body; if it is a text atom, decode the bytes. That walker is exactly what
our round-trip test does, and it is the whole proof that the file is correct.

---

## 2. The RecordHeader — 8 bytes, bit-packed

Every [MS-PPT] record opens with this fixed 8-byte header:

```text
 offset  size  field         meaning
 ------  ----  ------------  --------------------------------------------------
   0      2    recVerAndI    low  4 bits = recVer, high 12 bits = recInstance
   2      2    recType       which record this is (e.g. 0x03EE = Slide)
   4      4    recLen        length of the BODY that follows, in bytes
```

All fields are **little-endian**. The first `u16` packs two numbers:

```text
 recVerAndInstance (u16)
 ┌───────────────────────────────┬───────┐
 │ recInstance (12 bits)         │ recVer│   recVer  = value & 0x000F
 │ bits 4..15                    │ 4 bits│   recInst = (value >> 4) & 0x0FFF
 └───────────────────────────────┴───────┘
```

So to build the field: `(recVer & 0xF) | ((recInstance & 0xFFF) << 4)`.

The single most important convention: **`recVer == 0xF` means "this record is a
container"** (its body is child records). Any other `recVer` means "atom" (its
body is opaque data). We use `recVer = 0xF` for the Slide container and
`recVer = 0` for the text atoms.

`recLen` counts only the **body**, never the 8 header bytes. For a container,
`recLen` is the total size of all its children (each child being header + body).

---

## 3. The records we emit

We deliberately emit the **minimum** set of records that round-trips the slide
text through a record walker. A production `.ppt` has a `DocumentContainer`, a
persist-object directory, master slides, and much more; none of that is needed
to prove text placement, and adding it would be gold-plating for milestone C4.

### 3.1 Slide container — `recType 0x03EE`, `recVer 0xF`

One per slide. Its body is the slide's text atoms, concatenated. Empty slides
are legal: a Slide container with a zero-length body.

### 3.2 TextBytesAtom — `recType 0x0FA8`, `recVer 0`

The compact text encoding: **one byte per character** (Latin-1). Character *c*
is written as the single byte `c as u8`, valid exactly when every character's
Unicode scalar value is ≤ `0x00FF`. We choose this atom whenever the paragraph
is all-Latin-1, because it halves the size of ASCII text.

### 3.3 TextCharsAtom — `recType 0x0FA0`, `recVer 0`

The full text encoding: **UTF-16LE**, two bytes per code unit. We fall back to
this whenever any character exceeds `0x00FF` (e.g. `"你好"`), so no information
is lost. (Characters above the Basic Multilingual Plane become surrogate pairs,
which is exactly what UTF-16 is for.)

The per-paragraph choice is purely an encoding optimization; a reader decodes
each atom by its `recType`, so mixing the two across paragraphs is fine.

---

## 4. The record tree we produce

For a two-slide deck (slide 1 has two paragraphs, slide 2 has two paragraphs):

```text
PowerPoint Document stream
├─ Slide container            recType=0x03EE recVer=0xF  recLen = Σ children
│  ├─ TextBytesAtom           recType=0x0FA8 recVer=0    "Slide One Title"
│  └─ TextBytesAtom           recType=0x0FA8 recVer=0    "First slide body"
└─ Slide container            recType=0x03EE recVer=0xF  recLen = Σ children
   ├─ TextBytesAtom           recType=0x0FA8 recVer=0    "Slide Two Title"
   └─ TextBytesAtom           recType=0x0FA8 recVer=0    "Second slide body"
```

The concatenation of all Slide containers **is** the "PowerPoint Document"
stream. There is no outer container in this minimal profile.

---

## 5. Wrapping in the CFB container

We hand [`cfb-writer`](CFBW01-cfb-writer.md) two streams:

- **"PowerPoint Document"** — the record tree above.
- **"Current User"** — a tiny 4-byte stream (`0x14 0x00 0x00 0x00`). Real `.ppt`
  files carry a `CurrentUserAtom` here; ours is a stub for authenticity. It is
  harmless: our reader never consults it, and neither does the round-trip test.

`cfb-writer` decides sector vs mini-stream placement, builds the FAT, and emits
the OLE2 header. We do not care which store our stream lands in — the reader
gives it back to us by name.

### 5.1 The padding-stop subtlety

A CFB stream is stored in fixed-size sectors (or mini-sectors), so the reader
returns bytes **padded up to a sector boundary with zeros** past our logical
records. Our *logical* content ends at the last record we wrote, but the reader
hands back a slightly longer, zero-padded buffer.

A record walker must therefore stop cleanly on the padding. A zero byte region
decodes as a header with `recType == 0` **and** `recLen == 0` — a degenerate
"empty atom" that can only be padding. So the walker's termination rule is:

> Stop when fewer than 8 bytes remain, **or** when a header has
> `recType == 0 && recLen == 0`.

This is exactly the rule the round-trip test's walker uses. (Our writer never
emits a `recType == 0` record, so this can only ever be padding.)

---

## 6. Robustness — a total writer over trusted input

The model is trusted (the caller built it in-process), but the writer is still
**total**: it never panics on the public path.

- `#![forbid(unsafe_code)]`.
- The only place a length could overflow is `recLen` (a `u32`) or the packed
  container length. A single paragraph would need to exceed 4 GiB to overflow;
  we use `u32::try_from` and **skip** (drop) any atom whose encoded body does
  not fit, rather than silently wrapping the length into a wrong, corrupting
  value. A container whose total child length overflows likewise drops the
  overflowing tail. Dropping is the safe choice: a shorter valid file beats a
  longer corrupt one.
- Output is **deterministic**: identical models produce identical bytes.

---

## 7. Public API

```rust
pub struct Presentation { /* slides */ }
impl Presentation {
    pub fn new() -> Self;
    pub fn add_slide(&mut self) -> &mut Slide;
}

pub struct Slide { /* paragraphs */ }
impl Slide {
    pub fn add_text(&mut self, text: &str); // one paragraph -> one text atom
}

pub fn write_ppt(p: &Presentation) -> Vec<u8>;
```

Each `add_text` becomes one text atom; the TextBytes-vs-TextChars choice is made
per atom from the text's contents.

---

## 8. The proof — round-trip through the `cfb` reader

The canonical test builds two slides, writes the `.ppt`, reopens it with the
[`cfb`](CFB01-compound-file.md) reader, extracts "PowerPoint Document", and walks
the record tree in-test:

1. Read the 8-byte header.
2. If `recVer == 0xF`, recurse into the body (a container).
3. Else if `recType` is TextBytes/TextChars, decode the atom's text.
4. Stop on the padding sentinel (§5.1).

It asserts: two Slide containers; slide 1's atoms decode to
`"Slide One Title"` + `"First slide body"`; slide 2's to `"Slide Two Title"` +
`"Second slide body"`; and that the walk stops cleanly on the zero-padding.
Passing this test means the bytes we wrote are a real, readable `.ppt`.

[MS-PPT]: https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-ppt/
