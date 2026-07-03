# DOCXW01 — `docx-writer`: turn a document model into a valid `.docx`

Milestone **C2** of the OOXML effort. This is the word-processing *write* sibling
of the spreadsheet-side `xlsx-writer`, and the mirror image of the read-side
[`wordprocessingml`](WML01-wordprocessingml.md) crate. Where `wordprocessingml`
*opens* the bytes of a `.docx` into a `Document → Block → Paragraph/Table`
model, `docx-writer` *assembles* such a model back into the bytes of a valid
`.docx`.

It knows nothing about ZIP, content types, or relationships — that packaging
work belongs to the generic [`opc-writer`](OPCW01-opc-writer.md) crate (milestone
C1), which `docx-writer` sits directly on top of:

```text
Document model → docx-writer (C2, HERE) → opc-writer (C1) → zip → bytes
```

`docx-writer`'s single job is to know the **WordprocessingML** vocabulary: how a
document body is spelled as `<w:body>` full of `<w:p>` paragraphs (each a
sequence of `<w:r>` runs wrapping `<w:t>` text) and `<w:tbl>` tables (rows of
cells). Everything below that — synthesizing `[Content_Types].xml`, wiring the
package-root relationship, DEFLATE-compressing each part into a ZIP — is
delegated to `opc-writer`.

## The part tree of a `.docx`

A `.docx` is an OPC package (a ZIP with three conventions layered on top). The
minimal word-processing package `docx-writer` emits has exactly three parts:

```text
example.docx  (a ZIP archive)
├── [Content_Types].xml     ← what media type is each part?
│     • Default  rels → application/vnd.openxmlformats-package.relationships+xml
│     • Default  xml  → application/xml
│     • Override /word/document.xml
│                    → …wordprocessingml.document.main+xml
├── _rels/
│    └── .rels              ← the package-root relationships
│          • rId1  …/officeDocument  →  word/document.xml
└── word/
     └── document.xml       ← the body: paragraphs, runs, tables
```

Three conventions, three responsibilities:

1. **Content types.** `[Content_Types].xml` states each part's media type, by
   file *extension* (`<Default>`) or by exact *part name* (`<Override>`, which
   wins). `.rels` files and stray `.xml` are typed by defaults; the main
   document part gets an explicit override so a reader knows it is *the*
   WordprocessingML document. `docx-writer` registers these with `opc-writer`'s
   `add_default` / `add_part` and lets `opc-writer` synthesize the file.

2. **Relationships.** `_rels/.rels` maps the short id `rId1` to the target
   `word/document.xml` under the type URI ending `/officeDocument`. The read-side
   `wordprocessingml` crate follows exactly this relationship (`main_document_part()`
   looks for a Type ending `/officeDocument`) to locate the body, so emitting it
   is what makes our output openable *by that reader*. Built with `opc-writer`'s
   `RelationshipsBuilder` and added as the `_rels/.rels` part (typed by the
   `rels` default — no override needed).

3. **The document body.** `word/document.xml` carries the actual prose. This is
   the only part with format-specific structure, described next.

## WordprocessingML body structure

The body is a flat, ordered list of **block-level** items: paragraphs and
tables. The vocabulary, prefix `w:` bound to
`http://schemas.openxmlformats.org/wordprocessingml/2006/main`:

| element   | meaning                                             |
|-----------|-----------------------------------------------------|
| `w:document` | root; declares the `w:` namespace                |
| `w:body`  | the block container                                 |
| `w:p`     | a **paragraph** — a sequence of runs                |
| `w:r`     | a **run** — a maximal span of uniform formatting    |
| `w:t`     | the **text** of a run (a leaf, holds characters)    |
| `w:tbl`   | a **table**                                         |
| `w:tr`    | a table **row**                                     |
| `w:tc`    | a table **cell** — itself contains paragraphs       |

### Paragraph → run → text

A word processor lets you bold one word mid-sentence. WordprocessingML records
that by splitting a paragraph's text into **runs** — maximal spans sharing
formatting. So "Second **paragraph**." is *two* runs, `"Second "` and
`"paragraph."`. The reader concatenates runs with no separator (run boundaries
are formatting boundaries, not word boundaries), so the two rejoin to exactly
`"Second paragraph."` — the trailing space survives.

That survival depends on **`xml:space="preserve"`** on every `<w:t>`. Without it,
an XML processor is free to collapse leading/trailing whitespace, and the space
after `"Second"` would vanish. We therefore always emit it on user-text runs.

### A sample `document.xml`

For a document of: a paragraph `"Hello, DOCX!"`; a two-run paragraph
`"Second "` + `"paragraph."`; and a one-row table with cells `"A1cell"`,
`"B1cell"`:

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t xml:space="preserve">Hello, DOCX!</w:t></w:r></w:p>
    <w:p><w:r><w:t xml:space="preserve">Second </w:t></w:r><w:r><w:t xml:space="preserve">paragraph.</w:t></w:r></w:p>
    <w:tbl>
      <w:tr>
        <w:tc><w:p><w:r><w:t xml:space="preserve">A1cell</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t xml:space="preserve">B1cell</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>
```

## Escaping and robustness

Five characters are special in XML (`& < > " '`) and every scrap of caller text
lands inside a `<w:t>` text node, so all of it goes through `opc-writer`'s
`xml_escape`. That function is *total*: it escapes the five specials and silently
**drops** XML-illegal control characters (a NUL in pasted text would otherwise
make the whole part unparseable). By routing all user text through it we inherit
that safety for free and never panic on hostile input.

The crate is `#![forbid(unsafe_code)]` and takes no `unwrap`/`expect`/`panic!` on
any model-driven path. Degenerate inputs are valid, not fatal: an empty document
produces a well-formed empty `<w:body/>`; an empty table produces an empty
`<w:tbl>`; Unicode passes through unchanged.

## Public API

```rust
pub struct Document { /* blocks in order */ }
impl Document {
    pub fn new() -> Self;
    pub fn add_paragraph(&mut self, text: &str);         // single-run paragraph
    pub fn add_paragraph_runs(&mut self, runs: &[&str]); // multi-run paragraph
    pub fn add_table(&mut self, rows: &[Vec<String>]);   // rows of cell text
}
pub fn write_docx(doc: &Document) -> Vec<u8>;
```

## The round-trip proof

The headline test builds the sample document above with the API, calls
`write_docx`, then reopens the bytes with
`coding_adventures_wordprocessingml::open_docx` and asserts:

* `doc.text()` contains `"Hello, DOCX!"`, `"Second paragraph."`, `"A1cell"`,
  `"B1cell"`.
* the first paragraph's `.text` == `"Hello, DOCX!"`; the second == `"Second
  paragraph."` (the two runs concatenated);
* the table's row-0 cells are `"A1cell"` / `"B1cell"`.

Passing this proves the whole write path — WordprocessingML serialization, OPC
packaging, content types, relationships, ZIP — against an independent reader.
Unit tests additionally cover escaping of `& < > "`, Unicode, an empty document,
and paragraph ordering.
