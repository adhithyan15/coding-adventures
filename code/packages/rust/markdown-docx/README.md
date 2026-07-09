# markdown-docx

Native **Markdown → `.docx`** — turn Markdown (or GitHub-Flavored Markdown) into
a real Word document, over the repo's own zero-dependency stack, by routing it
through the shared **Document AST**.

Spec: [`code/specs/MD02-markdown-to-docx.md`](../../../specs/MD02-markdown-to-docx.md).

## The pipeline

This crate is the terminal convenience wrapper; it just composes two already-
tested stages:

```text
  Markdown  ──commonmark-parser::parse──▶  document_ast::DocumentNode
            ──document-ast-to-docx::to_docx_bytes──▶  .docx bytes
```

All the block/inline mapping (headings → `Heading N`, `**bold**` → bold runs,
lists → prefixed paragraphs, GFM tables → `<w:tbl>`, raw HTML dropped, …) plus
the recursion-depth DoS guard live in
[`document-ast-to-docx`](../document-ast-to-docx); the Markdown parsing lives in
[`commonmark-parser`](../commonmark-parser) / [`gfm-parser`](../gfm-parser).

## Usage

```rust
use markdown_docx::{markdown_to_docx, gfm_to_docx};

let docx = markdown_to_docx("# Title\n\nA **bold** word.\n");
std::fs::write("out.docx", docx).unwrap();          // opens in Word

// GFM adds pipe tables, task lists, strikethrough, autolinks:
let gfm = gfm_to_docx("| A | B |\n| - | - |\n| 1 | 2 |\n");
```

- `markdown_to_docx(&str) -> Vec<u8>` — strict CommonMark.
- `gfm_to_docx(&str) -> Vec<u8>` — GitHub-Flavored Markdown.

Both are total and panic-free on any input (the depth guard bounds adversarial
nesting); an empty string yields a valid empty `.docx`. `#![forbid(unsafe_code)]`.

## Fidelity

Inherited from `document-ast-to-docx` (MD02 §6), each lossless for visible text:
raw HTML is **dropped** (never injected — XSS-safe), links render as `text (url)`,
images as `[alt] (url)`, strikethrough as plain text, lists as prefixed
paragraphs. See that crate for the full table.

## The CLI

The [`md2docx`](../../../programs/rust/md2docx) program wraps this crate for the
shell: `md2docx in.md out.docx` (or `--gfm`, `--demo`).

## Testing

```bash
bash BUILD   # cargo test -p markdown-docx
```

The tests are the headline end-to-end proof: a Markdown string in, a real `.docx`
out, reopened with the independent `wordprocessingml` reader to confirm the text
and block structure survived — plus a raw-HTML-not-injected check and an
adversarially-deep (20 000-level) input that must not crash.
