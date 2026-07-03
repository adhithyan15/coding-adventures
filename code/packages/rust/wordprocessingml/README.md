# coding-adventures-wordprocessingml

Read a `.docx` (WordprocessingML) file into a **text-extractable document
model** — the word-processing sibling of
[`coding-adventures-spreadsheetml`](../spreadsheetml). Given the raw bytes of a
`.docx`, it hands back a `Document` → `Block` → `Paragraph`/`Table` tree where
every paragraph exposes its plain text (and its individual runs) and every table
exposes its rows × cells.

## Where it sits in the stack

```text
bytes → zip (M0) → xml-parser (M1) → opc (M2) → wordprocessingml (W3, here)
```

* **opc** opens the ZIP, exposes named parts, and points us at the main document
  part (`/word/document.xml` for a `.docx`).
* **xml-parser** parses that part's UTF-8 XML into a namespaced element tree.

This crate teaches the reader the WordprocessingML-specific meanings on top:
`<w:p>` is a paragraph made of `<w:r>` runs, `<w:t>` holds text, `<w:tbl>` is a
table of `<w:tr>` rows and `<w:tc>` cells.

## The paragraph → run → text model

A `.docx` body is *direct* — the text lives inline where you read it, with no
indirection to resolve (unlike a spreadsheet's shared-string table). The one
thing to learn is the three-level nesting:

**paragraph → runs → text.** A word processor splits a paragraph into *runs* —
maximal spans of text sharing the same formatting — so it can bold one word
mid-sentence. To get the paragraph's plain text, walk its runs in order and
concatenate. Run boundaries are *formatting* boundaries, not word boundaries, so
joining `"Second "` + `"paragraph."` yields exactly `"Second paragraph."`.

Two empty elements inside a run also add characters: `<w:tab/>` → `\t`,
`<w:br/>` → `\n`. `xml:space="preserve"` on a `<w:t>` is honoured automatically
because the xml-parser preserves text verbatim.

## Usage

```rust
use coding_adventures_wordprocessingml::{open_docx, Block};

let doc = open_docx(&docx_bytes)?;

// Whole-document plain-text extraction:
println!("{}", doc.text());

// Walk the structure:
for block in &doc.blocks {
    match block {
        Block::Paragraph(p) => println!("¶ {}", p.text),
        Block::Table(t) => {
            for row in &t.rows {
                let cells: Vec<_> = row.iter().map(|c| c.text.as_str()).collect();
                println!("| {}", cells.join(" | "));
            }
        }
    }
}
# Ok::<(), coding_adventures_wordprocessingml::DocxError>(())
```

Convenience accessors:

* `Document::text()` — the whole document as plain text (paragraphs, including
  those inside table cells, `\n`-joined).
* `Document::paragraphs()` — iterate only top-level paragraphs.
* `Document::tables()` — iterate only tables.

## Out of scope

Styles, fonts, colours, numbering, headers/footers, comments, images, and
section geometry (`<w:sectPr>`). We extract *text* and *table structure*;
formatting is left to a later milestone.

## Errors

`DocxError` covers a non-OPC package (`Opc`), a package with no main document
part (`MissingDocument`), a non-UTF-8 document part (`NotUtf8`), and a document
part that fails to parse (`MalformedXml`).

## Testing

```sh
cargo test -p coding-adventures-wordprocessingml
```

The suite includes an end-to-end test over a real DEFLATE-compressed `.docx`
fixture plus focused unit tests for run-joining, tabs/breaks, table structure,
`text()`, error paths, and empty/whitespace bodies.
