# document-ast-to-docx

Render a shared **Document AST** ([`document-ast`](../document-ast)) to a real
Word **`.docx`**, over the [`docx-writer`](../docx-writer) writer. The OOXML
sibling of [`document-ast-to-html`](../document-ast-to-html): where that renders a
`DocumentNode` to HTML, this renders it to WordprocessingML bytes.

Spec: [`code/specs/MD02-markdown-to-docx.md`](../../../specs/MD02-markdown-to-docx.md).

## Where it fits

```text
  document_ast::DocumentNode
       │  to_docx_document        (this crate — map blocks & inlines)
       ▼
  docx_writer::Document
       │  docx_writer::write_docx
       ▼
  .docx bytes
```

Markdown already parses to a `DocumentNode` (`commonmark-parser::parse`), so
composing that with `to_docx_bytes` is a native **Markdown → `.docx`** converter
(the `markdown-docx` crate does exactly that). Any frontend that targets the
Document AST — GFM, ASCIIDoc, HTML — gets `.docx` output for free.

## Usage

```rust
use document_ast_to_docx::to_docx_bytes;

// From any DocumentNode — e.g. one parsed from Markdown:
let ast = commonmark_parser::parse("# Title\n\nA **bold** word.\n\n- one\n- two\n");
let bytes = to_docx_bytes(&ast);
std::fs::write("out.docx", bytes).unwrap();
```

`to_docx_document(&DocumentNode) -> docx_writer::Document` is also public if you
want to add content before serializing.

## What maps to what

| Block | `.docx` |
|---|---|
| Heading (1–6) | `Heading N` styled paragraph |
| Paragraph | `Normal` paragraph, formatted runs |
| Code block | one monospace `Code` paragraph per line |
| Blockquote | child blocks in the `Quote` style |
| List | prefixed `ListParagraph`s (`• ` / `1. `, indented by nesting) |
| Task item | `☑ ` / `☐ ` prefix |
| Thematic break | a box-drawing rule paragraph |
| Table | a `<w:tbl>` of cell text |

Inlines flatten to runs, combining down the tree: `Strong` → **bold**,
`Emphasis` → *italic*, `CodeSpan` → `monospace`, so `**_x_**` is a single
bold+italic run.

## Fidelity (documented, MD02 §6)

- **Raw HTML** (`RawBlock`/`RawInline`) is **dropped**, never injected.
- **Links** render as `text (url)`; **images** as `[alt] (url)`.
- **Strikethrough** renders as plain text; **hard breaks** as a space (the writer
  has no strike flag / `<w:br/>` yet).
- **Lists** are prefixed paragraphs, not native Word numbering; **table cells**
  carry flattened text (header not bolded yet).

Each limit is lossless for the visible text and pinned by a test, so a future
enrichment is a deliberate change. `#![forbid(unsafe_code)]`, total and
panic-free.

## Testing

```bash
bash BUILD   # cargo test -p document-ast-to-docx
```

Tests assert the emitted `word/document.xml` shape (styles + run properties, via
the independent `opc` reader) and reopen the `.docx` with the `wordprocessingml`
reader to verify the visible text + block structure.
