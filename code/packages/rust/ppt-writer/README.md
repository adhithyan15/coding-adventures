# ppt-writer

A from-scratch, **zero-third-party-dependency** writer for the legacy
**PowerPoint 97-2003 binary** format (`.ppt`, [MS-PPT]). You build a slide-deck
model in memory; it produces the bytes of a real `.ppt` file.

This is milestone **C4** of the OOXML/binary-Office effort — the `.ppt` sibling
to the `.xls` writer path. It sits directly on top of the
[`cfb-writer`](../cfb-writer) crate, which supplies the OLE2 Compound File
container.

## Where it fits in the stack

```text
  Presentation model  (this crate's public API)
        │  write_ppt
        ▼
  "PowerPoint Document" record stream   ← [MS-PPT] Slide containers + text atoms
        │  cfb-writer::write_cfb
        ▼
  OLE2 Compound File bytes  →  a real .ppt file
```

A `.ppt` file is an OLE2 compound file (the same container as `.xls`/`.doc`)
whose payload lives in a stream named exactly **"PowerPoint Document"**. That
stream is a **tree of records**: each opens with an 8-byte header, and its body
is either child records (a *container*) or opaque data (an *atom*). This crate
emits, per slide, a **Slide container** whose children are **text atoms** — one
per paragraph — and hands the concatenated stream to `cfb-writer`.

## Usage

```rust
use ppt_writer::{Presentation, write_ppt};

let mut deck = Presentation::new();

let s1 = deck.add_slide();
s1.add_text("Slide One Title");
s1.add_text("First slide body");

let s2 = deck.add_slide();
s2.add_text("Slide Two Title");
s2.add_text("Second slide body");

let bytes = write_ppt(&deck);
std::fs::write("deck.ppt", &bytes).unwrap();
```

## API

- `Presentation::new()` — an empty deck.
- `Presentation::add_slide() -> &mut Slide` — append a slide, get a handle.
- `Slide::add_text(&str)` — one paragraph → one text atom.
- `write_ppt(&Presentation) -> Vec<u8>` — serialise to `.ppt` bytes.

## Encoding details

- **RecordHeader** (8 bytes, little-endian): `recVerAndInstance` (low 4 bits =
  `recVer`, high 12 = `recInstance`), `recType`, then a `u32` `recLen` (body
  length only). **`recVer == 0xF` marks a container.**
- **Slide container** — `recType 0x03EE`, `recVer 0xF`.
- **TextBytesAtom** — `recType 0x0FA8`: one byte per char (Latin-1). Chosen when
  every char is ≤ U+00FF.
- **TextCharsAtom** — `recType 0x0FA0`: UTF-16LE. Chosen when any char is
  > U+00FF (e.g. `"你好"`), so nothing is lost.

The per-paragraph atom choice is a pure size optimisation; a reader decodes each
atom by its `recType`.

## Robustness

- `#![forbid(unsafe_code)]`; no `unwrap`/`expect`/`panic!` on the public path.
- Lengths are packed with `u32::try_from`; an atom or container whose body would
  overflow the `u32` `recLen` is **skipped** rather than wrapped into a
  corrupting length.
- Output is **deterministic**: identical models yield identical bytes.

## The proof: round-trip through the `cfb` reader

The test suite writes a two-slide deck, reopens the `.ppt` with the sibling
[`cfb`](../cfb) reader, extracts the "PowerPoint Document" stream, and walks the
record tree — asserting two Slide containers and their exact paragraph text.
Because a CFB stream is zero-padded up to a sector boundary, the walker stops on
the padding sentinel (`recType == 0 && recLen == 0`) or when fewer than 8 bytes
remain.

See `code/specs/PPTW01-ppt-writer.md` for the full literate walkthrough.

## Testing

```sh
cargo test -p ppt-writer -- --nocapture
```

[MS-PPT]: https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-ppt/
