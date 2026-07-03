# `doc` — legacy Word `.doc` (Word 97–2003 binary) text reader

A from-scratch, **zero-third-party-dependency** Rust crate that reads a
legacy `.doc` file (the binary format specified by **[MS-DOC]**) and extracts
its **main-document text**. It is layered on the sibling [`cfb`](../cfb) crate,
which opens the OLE2 compound-file container that a `.doc` lives inside.

`#![forbid(unsafe_code)]`. Pure in-memory parsing — no filesystem, network,
process, or environment access. Input is treated as attacker-controlled: every
file-derived offset and length is bounds-checked with checked arithmetic, and
decoded size / piece count are capped, so a hostile file yields a typed error,
never a panic.

## Where it fits in the stack

```
  bytes ──▶ cfb (CFB01) ──▶ doc (DOC01) ──▶ your text
            OLE2 container   FIB + piece table
```

This is milestone **B2** in the OOXML/legacy-Office reader ladder — the hardest
reader, because a `.doc`'s text is not stored contiguously.

## How it works (the piece table)

A `.doc` stores its text scattered across runs inside a `WordDocument` stream.
The order in which those runs reassemble is recorded in a **piece table**
(`CLX`) held in a second stream (`0Table` or `1Table`). The reader:

1. Opens the compound file with `cfb`.
2. Reads a few **FIB** header fields from `WordDocument`: the `wIdent` magic
   (`0xA5EC`), the `fWhichTblStm` flag (which table stream to use), and
   `fcClx`/`lcbClx` (where the CLX is and how big).
3. Slices the **CLX** out of the table stream, skips any property runs (`Prc`),
   and reads the piece table (`Pcdt` → `PlcPcd`).
4. For each piece, decodes its `FcCompressed` field — a `u32` whose bit 30 says
   8-bit vs 16-bit and whose low 30 bits locate the bytes in `WordDocument` —
   then decodes Latin-1 (8-bit) or UTF-16LE (16-bit) and concatenates.

See [`code/specs/DOC01-binary-reader.md`](../../../specs/DOC01-binary-reader.md)
for a fully literate walkthrough with diagrams and a worked `FcCompressed`
decode.

## Usage

```rust
use doc::open_doc;

let bytes: &[u8] = /* the raw .doc file */;
let document = doc::open_doc(bytes)?;
println!("{}", document.text());
# Ok::<(), doc::DocError>(())
```

### API

```rust
pub fn open_doc(bytes: &[u8]) -> Result<Document, DocError>;

pub struct Document { /* … */ }
impl Document { pub fn text(&self) -> &str; }

pub enum DocError {
    Cfb(cfb::CfbError),   // container could not be read (source() chains through)
    NotWordDocument,      // no WordDocument stream, or wIdent != 0xA5EC
    NoTableStream,        // the selected 0Table/1Table stream is absent
    MalformedPieceTable,  // inconsistent CLX / PlcPcd
    Truncated,            // an offset/length ran past the available bytes
}
```

`DocError` implements `Display` and `std::error::Error`; the `Cfb` variant
reports the underlying `cfb::CfbError` via `source()`. `From<cfb::CfbError>` is
provided.

## Building & testing

```sh
cargo test -p doc -- --nocapture
```

The suite includes an end-to-end read of a real CFB-wrapped fixture that decodes
to `"Hello, DOC!"`, plus unit tests covering compressed/uncompressed/multi-piece
decoding, the `n = (lcb-4)/12` piece-count math, `Prc` skipping, and every error
path (non-CFB input, missing streams, out-of-range offsets, malformed tables).

## Limitations

- Extracts the **main-document** text only (not headers/footers, footnotes,
  comments, or field results).
- 8-bit runs are decoded as Latin-1, which is faithful for ASCII and the
  fixture; genuine Windows-1252 differs only in the `0x80..=0x9F` range.
- Formatting, styles, images, and revision data are ignored by design.

## License

MIT.
