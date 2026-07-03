# WML01 — WordprocessingML document reader

## Overview

This is milestone **W3** of the OOXML effort — the *word-processing* sibling of
the SpreadsheetML reader in [SML01](SML01-spreadsheetml.md). It builds a Rust
crate, `coding-adventures-wordprocessingml`, that reads the bytes of a `.docx`
file and hands back a **text-extractable document model**: a document → blocks →
paragraphs/tables model where every paragraph exposes its plain text (and its
individual runs), and every table exposes its rows × cells.

Where the layers below stop:

```text
raw bytes (.docx)
      |
      v
zip crate (M0)         → ZIP members: name → bytes
      |
      v
xml-parser (M1)        → XmlDocument (namespaces resolved, entities decoded)
      |
      v
opc crate (M2)         → Package  (parts, content types, relationships)
      |                     - main_document_part() → "/word/document.xml"
      |                     - read_part("/word/document.xml") → &[u8]
      v
wordprocessingml (W3)  → Document / Block / Paragraph / Run / Table / Cell (THIS)
```

OPC knows "here is a bag of named parts and how they link." It does **not** know
that `/word/document.xml` is a document with a body, that a `<w:p>` is a
*paragraph* made of `<w:r>` *runs*, or that a `<w:t>` holds the actual text.
Teaching the reader those WordprocessingML-specific meanings is exactly this
milestone's job.

Deliberately **out of scope**: styles, fonts, colours, numbering, headers and
footers, comments, images, and section geometry (`<w:sectPr>`). We extract the
*text content* and the *table structure* — the two things a downstream
text-extraction or search pipeline needs — and nothing else. Formatting is left
to a later milestone.

## The paragraph → run → text model a newcomer must understand

Unlike a spreadsheet — which is *normalized* with two levels of indirection
(`r:id` → part, and shared-string index → text) — a `.docx` body is refreshingly
*direct*: the text lives inline, right where you read it. There is no indirection
to resolve. The only thing that trips up a newcomer is the **three-level nesting**
between "a paragraph" and "the characters in it."

### Why runs exist

A word processor lets you bold *one word* in the middle of a sentence. To record
that, WordprocessingML splits a paragraph's text into **runs** — maximal spans of
text that share the same formatting. The sentence

> Second **paragraph**.

is stored as *two* runs, "Second " and "paragraph.", because the second run is
bold and the first is not:

```xml
<w:p>
  <w:r><w:t xml:space="preserve">Second </w:t></w:r>
  <w:r><w:rPr><w:b/></w:rPr><w:t>paragraph.</w:t></w:r>
</w:p>
```

So a paragraph is **paragraph → runs → text**. To get the paragraph's plain
text you walk its runs in order and concatenate each run's text. Split points
between runs are formatting boundaries, **not** word or space boundaries — which
is why "Second " keeps its trailing space (guarded by `xml:space="preserve"`, see
below) and joining the two runs yields exactly `"Second paragraph."`, no space
inserted or lost.

### `xml:space="preserve"` — why the trailing space survives

By XML's default rules, leading and trailing whitespace in element text *may* be
collapsed. A run whose text is `"Second "` (with a meaningful trailing space)
guards it with the standard attribute `xml:space="preserve"`. Our underlying
xml-parser preserves *trailing* text-node whitespace verbatim, so we honour the
meaningful space naturally by reading `<w:t>`'s text content directly — we never
trim run text ourselves. (The parser does not itself *interpret*
`xml:space="preserve"`; it trims a text node's leading whitespace but keeps the
trailing. WordprocessingML runs that need a word-separating space put it
trailing — as in `"Second "` — which is exactly the whitespace that survives, so
run joining is correct.)

### Tabs and breaks are text too

Two empty elements inside a run also contribute characters:

* `<w:tab/>` → a tab character (`\t`)
* `<w:br/>` → a line break (`\n`)

They appear as *siblings* of `<w:t>` inside the run, in document order, so a run
is really a small ordered sequence of "text | tab | break" pieces. We flatten
them into the run's text string in order.

### Why not just call `text_content()` on the paragraph?

The xml-parser gives every element a `text_content()` that concatenates **all**
descendant text. For a *simple* paragraph that happens to equal the run text. But
a paragraph can legally contain nested content — most importantly, a cell in a
table contains paragraphs, and `text_content()` on an outer element would grab
the inner table's text too, over-concatenating across structural boundaries. So
we walk `<w:r>` runs **explicitly** and, within each run, only `<w:t>` / `<w:tab>`
/ `<w:br>`. This keeps paragraph text exact and table text correctly scoped.

## Block content: paragraphs and tables

The body (`<w:body>`) is a flat, ordered list of **block-level** items. We model
the two that carry content:

* `<w:p>` — a paragraph (above).
* `<w:tbl>` — a table: `<w:tr>` rows → `<w:tc>` cells, and each cell contains its
  own paragraphs. A cell's text is the newline-join of its paragraphs' text.

The body's trailing `<w:sectPr>` (section properties: page size, margins) carries
no content and is ignored. Any other block-level element we don't recognise is
skipped rather than erroring — forward-compatibility with producers that emit
structured document tags, SDTs, etc.

## Public API

```rust
pub fn open_docx(bytes: &[u8]) -> Result<Document, DocxError>;

pub struct Document { pub blocks: Vec<Block> }
pub enum   Block    { Paragraph(Paragraph), Table(Table) }
pub struct Paragraph { pub text: String, pub runs: Vec<Run> }
pub struct Run       { pub text: String }
pub struct Table     { pub rows: Vec<Row> }      // Row = Vec<Cell>
pub struct Cell      { pub text: String, pub paragraphs: Vec<Paragraph> }
```

Convenience accessors:

* `Document::text()` — the whole document as plain text: every paragraph's text
  (including paragraphs inside table cells, in document order) joined with `\n`.
  This is the headline "text extraction" output.
* `Document::paragraphs()` — iterate only the top-level paragraphs.
* `Document::tables()` — iterate only the tables.

## Errors

`DocxError` wraps everything that can go wrong:

| Variant             | When                                                        |
|---------------------|-------------------------------------------------------------|
| `Opc(OpcError)`     | bytes were not a readable OPC package (not a ZIP, …)         |
| `MissingDocument`   | package opened but has no main document part (not a `.docx`) |
| `NotUtf8(String)`   | the document part was not valid UTF-8 (carries part name)   |
| `MalformedXml(String)` | the document part did not parse as XML                    |

## Namespaces

* **W** (WordprocessingML main): `http://schemas.openxmlformats.org/wordprocessingml/2006/main` — every structural element (`document`, `body`, `p`, `r`, `t`, `tab`, `br`, `tbl`, `tr`, `tc`).

The `xml:space` attribute lives in the reserved XML namespace
(`http://www.w3.org/XML/1998/namespace`), but we do not need to *read* it: our
xml-parser already preserves text verbatim, so honouring the space is automatic.

## Test plan

The end-to-end fixture (`MINIMAL_DOCX`) is a real DEFLATE-compressed `.docx`
whose body has, in order:

1. a paragraph `"Hello, DOCX!"`,
2. a paragraph `"Second paragraph."` stored as two runs (`"Second "` + `"paragraph."`),
3. a one-row table with cells `"A1cell"` and `"B1cell"`.

Assertions:

* `open_docx(MINIMAL_DOCX)` yields those blocks in order.
* the two-run paragraph joins to exactly `"Second paragraph."`.
* the table cells read `"A1cell"` / `"B1cell"`.
* `Document::text()` contains all of the above text.
* non-`.docx` bytes error (`Opc` / `MissingDocument`).
* empty / whitespace-only bodies are handled without panic.
