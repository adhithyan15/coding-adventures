# MD02 — Markdown → `.docx` through the Document AST

Convert Markdown into a real Word `.docx` file **natively**, over the repo's own
zero-dependency stack, by routing it through the shared **Document AST**
([`document-ast`](TE00-document-ast.md)) — the same format-agnostic IR that
already backs Markdown → HTML.

This is the word-processing analogue of the spreadsheet unification: just as
`spreadsheet-core` became the one hub every tabular format converts to and from,
`document-ast` is the one hub every *prose* format should converge on. MD02 is
the first bridge from that hub to the OOXML world.

## Where it sits

The pipeline reuses two shipped stages (Markdown → AST, and OPC packaging) and
adds two new bridges plus a formatting enrichment of `docx-writer`:

```text
  Markdown text
      │  commonmark-parser::parse          (SHIPPED — TE01)
      ▼
  document_ast::DocumentNode               (SHIPPED — TE00, the hub)
      │  document-ast-to-docx::to_docx_*    (NEW — this spec, §4)
      ▼
  docx_writer::Document                     (ENRICHED — this spec, §3; DOCXW01)
      │  docx_writer::write_docx
      ▼
  opc-writer → zip → .docx bytes            (SHIPPED — C1/C2)
```

Nothing here is format-specific until the AST → `docx-writer` step: the same
`DocumentNode` a Markdown file parses to is what an ASCIIDoc file, an HTML page,
or (eventually) a `.docx` reader would produce. Every future *AST → format*
writer (LaTeX, ODT, …) plugs in at the same seam.

## 1. Scope

**In scope (the CommonMark + GFM block/inline vocabulary the AST models):**
headings (levels 1–6), paragraphs, hard/soft breaks, ordered & unordered lists
(nested), GFM task lists, blockquotes, fenced/indented code blocks, thematic
breaks, GFM pipe tables, and the inline set — text, **strong**, *emphasis*,
~~strikethrough~~, `code spans`, links, images (as their alt text + a trailing
URL), and autolinks.

**Out of scope (documented fidelity limits, §6):** embedded binary images
(alt-text only), footnotes, raw-HTML blocks/inlines (dropped, not injected),
nested-block table cells (cell inlines only), and heading/list *numbering*
restart semantics. These degrade gracefully; none is an error.

## 2. Design principles

- **The AST is authoritative.** `document-ast-to-docx` reads a `DocumentNode`
  and never re-parses Markdown; anything the AST can't express, it can't emit.
- **Semantic, not notational.** A heading becomes a Word *Heading N* paragraph
  style (so Word's outline/navigation works), not literal `#` characters. Bold
  becomes a run property, not `**`.
- **Additive & back-compatible.** `docx-writer`'s existing text-only API
  (`add_paragraph`/`add_paragraph_runs`/`add_table`) keeps working byte-for-byte;
  formatting is layered on top.
- **Total & panic-free.** Every stage is pure in-memory computation returning a
  value; malformed/degenerate input yields a valid (possibly empty) `.docx`,
  never a panic. Untrusted Markdown is a first-class input.

## 3. `docx-writer` enrichment (DOCXW01 addition)

`docx-writer` today keeps only run *text* (DOCXW01 §"paragraph → run → text").
MD02 needs it to carry the minimum formatting Markdown implies. Additions, all
back-compatible:

### 3.1 Styled runs — `<w:rPr>`

A run gains optional **bold**, **italic**, and **monospace** flags, emitted as a
run-properties element before the text:

```xml
<w:r><w:rPr><w:b/><w:i/><w:rFonts w:ascii="Consolas" w:hAnsi="Consolas"/></w:rPr>
     <w:t xml:space="preserve">…</w:t></w:r>
```

Bold (`<w:b/>`) and italic (`<w:i/>`) are **direct** formatting — they render in
Word with no styles part. Monospace is a direct font override (used for inline
`code`). A run with no flags emits no `<w:rPr>`, so existing output is unchanged.

### 3.2 Paragraph styles — `<w:pPr><w:pStyle>` + `styles.xml`

A paragraph gains an optional **style id**, emitted as paragraph properties:

```xml
<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr>…runs…</w:p>
```

Unlike direct run formatting, a `pStyle` reference only *renders* as a heading if
the package defines that style. So MD02 adds a minimal **`word/styles.xml`** part
(wired as a relationship off `word/document.xml`) defining the built-in-linked
styles MD02 uses: `Heading1`…`Heading6`, `Code` (monospace block), and `Quote`.
A document that uses no styles omits `styles.xml` entirely — again keeping the
minimal-case output byte-identical.

### 3.3 New `docx-writer` API (additive)

```rust
pub struct Run { pub text: String, pub bold: bool, pub italic: bool, pub mono: bool }
impl Run { pub fn plain(text: &str) -> Run; /* + bold()/italic()/mono() builders */ }

pub enum ParagraphStyle { Normal, Heading(u8 /*1..=6*/), Code, Quote, List }

impl Document {
    // existing: new / add_paragraph / add_paragraph_runs / add_table  (unchanged)
    pub fn add_styled_paragraph(&mut self, style: ParagraphStyle, runs: Vec<Run>);
    pub fn add_table_runs(&mut self, rows: Vec<Vec<Vec<Run>>>); // styled cells (optional)
}
```

## 4. `document-ast-to-docx` (new crate)

Maps a `DocumentNode` onto the enriched `docx-writer::Document`.

```rust
pub fn to_docx_document(doc: &document_ast::DocumentNode) -> docx_writer::Document;
pub fn to_docx_bytes(doc: &document_ast::DocumentNode) -> Vec<u8>; // = write_docx(&to_docx_document(..))
```

### 4.1 Block mapping

| `BlockNode` | `.docx` |
|---|---|
| `Heading{level, children}` | styled paragraph, `ParagraphStyle::Heading(level)`, inline runs |
| `Paragraph{children}` | `ParagraphStyle::Normal` paragraph, inline runs |
| `CodeBlock{value}` | one `ParagraphStyle::Code` paragraph per source line (blank line ⇒ empty paragraph), text verbatim, mono runs |
| `Blockquote{children}` | child blocks rendered with `ParagraphStyle::Quote` |
| `List{ordered, start, children}` | one paragraph per item, prefixed with `"1. "`/`"2. "…` (ordered, honouring `start`) or `"• "` (unordered); nested lists indented with leading spaces in the prefix |
| `TaskItem{checked, children}` | list item prefixed `"☑ "` / `"☐ "` |
| `ThematicBreak` | an empty paragraph carrying a bottom border (horizontal rule) |
| `Table{align, children}` | `docx-writer` table; header row cells rendered bold; cell inlines flattened to runs |
| `RawBlock` | **dropped** (not injected) — see §6 |

Lists render as *prefixed paragraphs*, not native Word numbering
(`numbering.xml`), in v1: it reads correctly, round-trips as text, and avoids the
heavier numbering-definitions part. Native numbering is a documented follow-up.

### 4.2 Inline mapping (→ `Vec<Run>`)

Inlines flatten to a run list, carrying formatting down the tree (nested
`Strong`/`Emphasis` combine, e.g. `**_x_**` ⇒ a bold+italic run):

| `InlineNode` | run(s) |
|---|---|
| `Text{value}` | one plain run |
| `Strong{children}` | children with `bold` set |
| `Emphasis{children}` | children with `italic` set |
| `Strikethrough{children}` | children (strike not yet a run flag — v1 renders as plain; documented) |
| `CodeSpan{value}` | one `mono` run |
| `Link{destination, children}` | the child runs, then `" (<destination>)"` appended as a plain run (a real `w:hyperlink` is a follow-up) |
| `Image{alt, destination}` | `"[<alt>]"` + `" (<destination>)"` as plain runs |
| `Autolink{destination}` | one plain run of the URL |
| `HardBreak` | a run boundary + `<w:br/>` (paragraph split not needed — same paragraph) |
| `SoftBreak` | a single space |
| `RawInline` | **dropped** — see §6 |

## 5. `markdown-docx` (new crate) + `md2docx` CLI

The end-to-end convenience wrapper and the user-facing deliverable:

```rust
pub fn markdown_to_docx(markdown: &str) -> Vec<u8>;      // parse + to_docx_bytes
pub fn gfm_to_docx(markdown: &str) -> Vec<u8>;           // via gfm-parser (tables/tasks/strike)
```

A tiny `md2docx` program reads a `.md` path and writes a `.docx` path, so the
whole pipeline is exercisable from the shell.

## 6. Fidelity limits (documented + pinned in tests)

- **Raw HTML** (`RawBlock`/`RawInline`, `format == "html"`) is **dropped**, never
  injected — a `.docx` can't host arbitrary HTML and we won't smuggle markup.
- **Images** carry alt text + URL only (no binary embedding / media part in v1).
- **Links** render as `text (url)` (no clickable `w:hyperlink` in v1).
- **Strikethrough** renders as plain text in v1 (`<w:strike/>` is a trivial
  follow-up once needed).
- **Lists** are prefixed paragraphs, not native Word numbering.
- **Table cells** carry inline content only (no nested block cells).

Each limit is *lossless for text* — the round-trip reader recovers every visible
character — and pinned by a test so a future enrichment is a deliberate change.

## 7. Verification

- **`docx-writer`** (unit): styled runs emit `<w:b/>`/`<w:i/>`/mono `w:rFonts`;
  styled paragraphs emit `<w:pStyle>`; `styles.xml` present iff a style is used;
  existing round-trip (open with [`wordprocessingml`](WML01-wordprocessingml.md),
  compare text) still green.
- **`document-ast-to-docx`** (unit): hand-built `DocumentNode`s map to the
  expected `docx-writer::Document` / XML; every `BlockNode`/`InlineNode` variant
  covered; the raw-HTML-dropped and image/link fidelity limits pinned.
- **`markdown-docx`** (integration, the headline proof): a Markdown document with
  headings, bold/italic, lists, a code block, and a table → `.docx` bytes →
  reopened with `wordprocessingml::open_docx` → assert the recovered `.text()`
  contains the expected strings **and** the block structure (heading/paragraph/
  table counts) matches. Proves the Markdown ↔ AST ↔ `.docx` path end to end.

## 8. Why native (not `pandoc`)

Same rationale as the rest of the repo: pandoc is a large Haskell binary and an
external dependency; the OOXML + Markdown stack here is already zero-dependency
Rust that reads and writes real Office files. Routing through `document-ast`
means every prose format the repo learns to read or write reuses one IR and one
set of bridges — not an N×N matrix of point converters.
