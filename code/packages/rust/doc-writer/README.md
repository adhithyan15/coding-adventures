# doc-writer

A from-scratch, **zero-third-party-dependency** writer for the legacy **`.doc`**
format ([MS-DOC]) — the Word 97-2003 binary document. You give it a simple
document model (paragraphs of text); it produces a valid `.doc` byte buffer by
emitting the **FIB** (File Information Block), a **piece table** (CLX), and the
text, all wrapped in an **OLE2 / Compound File Binary** container via the
sibling [`cfb-writer`](../cfb-writer) crate.

This is milestone **C4** of the legacy-Office stack. The only dependency is our
own `cfb-writer`; the dev-dependency `cfb` (the reader) is used solely to prove
the output round-trips.

## Where it fits

```text
  Document (paragraphs)
        │  write_doc
        ▼
  FIB + text  ┐
  CLX (pieces)┘── cfb-writer::write_cfb ──▶  .doc bytes (a CFB container)
```

A `.doc` is a CFB holding two streams:

- **`WordDocument`** — the FIB (a fixed header of offsets/lengths) followed by
  the document text at a fixed offset.
- **`1Table`** — the **CLX**, whose **piece table** (`PlcPcd`) maps character
  positions to byte offsets in `WordDocument` and records the text encoding
  (8-bit Latin-1 or 16-bit UTF-16LE) via the `FcCompressed` bit-30 trick.

We always emit **one piece** covering the whole document.

## Usage

```rust
use doc_writer::{Document, write_doc};

let mut doc = Document::new();
doc.add_paragraph("Hello, DOC!");
doc.add_paragraph("Second paragraph.");

let bytes = write_doc(&doc);
std::fs::write("hello.doc", &bytes).unwrap();
```

Paragraphs are joined by the Word paragraph mark `\r` (`0x0D`) into a single run
of text. The retrieved text is therefore the paragraphs joined by `\r`.

### Encoding

- If **every** character is `<= U+00FF` (Latin-1), the text is stored **8-bit
  compressed** — one byte per character (compact).
- If **any** character is `> U+00FF`, the whole document falls back to **16-bit**
  UTF-16LE — the faithful choice that never mangles a character. Astral
  characters (surrogate pairs) round-trip correctly.

## The round-trip proof

The test suite writes a document, opens it with the independent `cfb` reader,
and then **re-implements** the [MS-DOC] retrieval from first principles (FIB →
`fcClx`/`lcbClx` → CLX → `PlcPcd` → PCD → `FcCompressed` → text). It asserts the
reassembled text equals the input. This proves the bytes are a genuinely
readable `.doc`, not merely something our own encoder can read back.

## Robustness

- `#![forbid(unsafe_code)]`.
- No `unwrap`/`expect`/`panic!` on the public path.
- `checked_*` arithmetic guards every offset/length a colossal document could
  overflow; on overflow the writer degrades to a valid **empty** document rather
  than emitting corrupt bytes.
- Output is **deterministic** — the same document always yields the same bytes.

## Limitations (future extensions)

- Always a **single piece**; no in-place edit history.
- One encoding for the whole document (no mixed 8-bit/16-bit pieces).
- No formatting (character/paragraph properties, styles).

See `code/specs/DOCW01-doc-writer.md` for the full literate specification,
including every FIB field offset, the CLX/`Pcdt`/`PlcPcd`/PCD layout, the
`FcCompressed` bit trick, and a diagram of the retrieval path.

[MS-DOC]: https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-doc/
