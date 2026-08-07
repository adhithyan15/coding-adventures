# coding-adventures-docx-writer

Turn a simple document model into a valid `.docx` file. This is milestone **C2**
of the OOXML effort — the word-processing *write* sibling of the spreadsheet
`xlsx-writer`, and the mirror image of the read-side
[`coding-adventures-wordprocessingml`](../wordprocessingml) reader.

## Where it fits

```text
Document model → docx-writer (C2) → opc-writer (C1) → zip → bytes
```

`docx-writer` knows exactly one thing: the **WordprocessingML** vocabulary — how
a document body is spelled as `<w:body>` full of `<w:p>` paragraphs (each a
sequence of `<w:r>` runs wrapping `<w:t>` text) and `<w:tbl>` tables. Everything
below — synthesizing `[Content_Types].xml`, wiring the package-root
relationship, DEFLATE-compressing each part into a ZIP — is delegated to the
generic [`coding-adventures-opc-writer`](../opc-writer) packaging layer. It has
**no third-party dependencies**.

## What it produces

A minimal three-part OPC package:

```text
example.docx  (a ZIP archive)
├── [Content_Types].xml     ← media type of each part (synthesized by opc-writer)
├── _rels/.rels             ← rId1 …/officeDocument → word/document.xml
└── word/document.xml       ← the body: paragraphs, runs, tables
```

## Usage

```rust
use coding_adventures_docx_writer::{Document, write_docx};

let mut doc = Document::new();
doc.add_paragraph("Hello, DOCX!");                       // single-run paragraph
doc.add_paragraph_runs(&["Second ", "paragraph."]);      // multi-run paragraph
doc.add_table(&[
    vec!["A1cell".to_string(), "B1cell".to_string()],    // rows of cell text
]);

let bytes = write_docx(&doc);
std::fs::write("example.docx", bytes).unwrap();
```

### Formatting (v0.2)

Beyond plain text, a paragraph can carry a **style** and its runs can carry
**direct formatting** — the minimum a rich document (e.g. one produced from
Markdown) needs:

```rust
use coding_adventures_docx_writer::{Document, ParagraphStyle, Run, write_docx};

let mut doc = Document::new();
doc.add_styled_paragraph(ParagraphStyle::Heading(1), vec![Run::plain("Title")]);
doc.add_styled_paragraph(
    ParagraphStyle::Normal,
    vec![Run::plain("A "), Run::plain("bold").bold(), Run::plain(" word, and "),
         Run::plain("code").mono(), Run::plain(".")],
);
let _bytes = write_docx(&doc);
```

`Run::plain(..).bold()/.italic()/.mono()` emit `<w:b/>` / `<w:i/>` / a monospace
`<w:rFonts>` (all *direct* formatting, no styles part needed).
`ParagraphStyle::{Heading(1..=6), Code, Quote, List}` emit a `<w:pStyle>` and pull
in a minimal `word/styles.xml` that defines those styles (headings carry an
outline level, so Word's navigation pane works). **An all-`Normal`, unformatted
document has no `styles.xml` and serializes byte-for-byte as it did in v0.1.**

`write_docx` returns the complete `.docx` as a `Vec<u8>` — open it in Word, or
reopen it with the sibling `wordprocessingml` reader.

## Notes on fidelity

* **Runs and whitespace.** A multi-run paragraph becomes one `<w:r>` per run;
  the reader rejoins them with no separator, so `["Second ", "paragraph."]`
  reads back as `"Second paragraph."`. Every `<w:t>` carries
  `xml:space="preserve"`, so leading/trailing spaces survive.
* **Escaping and safety.** All user text passes through `opc-writer`'s
  `xml_escape`, which escapes the five XML specials and drops XML-illegal control
  characters. The crate is `#![forbid(unsafe_code)]` and never panics on model
  input — empty documents, empty tables, and arbitrary Unicode are all valid.

## Testing

```sh
cargo test -p coding-adventures-docx-writer
```

The headline test builds a document, writes it, and **round-trips** it through
`coding-adventures-wordprocessingml`, asserting the model comes back intact —
proving the whole write path against an independent reader.

See [`code/specs/DOCXW01-docx-writer.md`](../../../specs/DOCXW01-docx-writer.md)
for the full literate write-up.
