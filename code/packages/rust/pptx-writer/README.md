# coding-adventures-pptx-writer

**Milestone C3 of the OOXML effort.** A zero-third-party-dependency Rust crate
that turns a simple in-memory slide-deck model into the bytes of a valid
`.pptx` (PresentationML) file.

## What it does

```rust
use coding_adventures_pptx_writer::{Presentation, write_pptx};

let mut p = Presentation::new();

let intro = p.add_slide();
intro.add_text("Welcome");
intro.add_text("A zero-dependency .pptx writer");

let outro = p.add_slide();
outro.add_text("Thanks!");

let bytes = write_pptx(&p);          // valid .pptx bytes
std::fs::write("deck.pptx", bytes).unwrap();
```

The result opens in PowerPoint, LibreOffice Impress, and python-pptx.

## Where it fits in the stack

`pptx-writer` knows the **PresentationML** vocabulary; it delegates all ZIP,
content-type, and relationship packaging to the format-agnostic
[`opc-writer`](../opc-writer) (which in turn builds on the [`zip`](../zip)
crate). It is the presentation-side sibling of
[`xlsx-writer`](../xlsx-writer) (SpreadsheetML) and
[`wordprocessingml`](../wordprocessingml).

```text
  Presentation / Slide          (this crate's model)
        │  write_pptx
        ▼
  PresentationML parts          (this crate)
        │  opc_writer::PackageWriter
        ▼
  OPC package                   (opc-writer: content-types, .rels, ZIP)
        ▼
  .pptx bytes
```

## Why a deck is more than "slides in a zip"

A conformant `.pptx` requires a whole scaffold wired by relationships —
`presentation → slide master → slide layout → theme` — because slides inherit
their placeholders, colour map, and fonts from that chain of parents. A package
with only a `presentation.xml` and a `slide1.xml` is **rejected** by strict
consumers. This writer always emits one shared master, one shared layout, and
one shared theme (constant boilerplate) plus one `slideN.xml` per slide. The
full part list and the relationship graph are documented in
[`code/specs/PPTXW01-pptx-writer.md`](../../../specs/PPTXW01-pptx-writer.md).

## The `a:` namespace gotcha

A slide is `<p:sld>` in the PresentationML namespace, but the *text* lives in
the **DrawingML** namespace (prefix `a:`):
`<a:p><a:r><a:t>your text</a:t></a:r></a:p>`. Each `Slide::add_text` call adds
one such paragraph. All user text is escaped via `opc_writer::xml_escape`.

## Public API

```rust
pub struct Presentation;
impl Presentation {
    pub fn new() -> Self;
    pub fn add_slide(&mut self) -> &mut Slide;
    pub fn slides(&self) -> &[Slide];
}

pub struct Slide;
impl Slide {
    pub fn add_text(&mut self, text: &str);
    pub fn paragraphs(&self) -> &[String];
}

pub fn write_pptx(p: &Presentation) -> Vec<u8>;
```

An empty deck (no slides) is still valid: it produces a `presentation.xml` with
an empty `<p:sldIdLst/>` and the full scaffold.

## Safety

`#![forbid(unsafe_code)]`. No panics on model-driven paths — empty decks, empty
slides, and arbitrary Unicode / XML-special text are all handled. Characters
illegal in XML 1.0 (NUL, other C0 controls) are dropped by `xml_escape` so
untrusted text can never make the package unparseable.

## Testing

Because the `.pptx` *reader* is not on this branch, tests verify the generated
bytes structurally: they unzip with this repo's `zip::ZipReader` to assert the
expected members exist, then parse slide parts with
`coding_adventures_xml_parser::parse_xml` to check the `<a:t>` text nodes carry
the right text (and only the right text, per slide). Run them with:

```sh
cargo test -p coding-adventures-pptx-writer
```
