# Changelog

All notable changes to `document-ast-to-docx` are documented here. The format
follows [Keep a Changelog](https://keepachangelog.com/), and this project adheres
to [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-07-08

### Added

- Initial release — the **Document AST → `.docx`** bridge (spec
  `code/specs/MD02-markdown-to-docx.md`).
- `to_docx_document(&DocumentNode) -> docx_writer::Document` and
  `to_docx_bytes(&DocumentNode) -> Vec<u8>` — map the shared Document AST onto the
  enriched `docx-writer` model (headings → `Heading N`, code → `Code`, quotes →
  `Quote`, lists → prefixed `ListParagraph`s, tables → `<w:tbl>`), flattening
  inlines to formatted runs (`Strong`→bold, `Emphasis`→italic, `CodeSpan`→mono,
  combining down the tree).
- Composing `commonmark-parser::parse` with `to_docx_bytes` yields a native
  Markdown → `.docx` conversion; any Document-AST frontend gets `.docx` for free.
- Documented, text-lossless fidelity limits (MD02 §6): raw HTML dropped, links as
  `text (url)`, images as `[alt] (url)`, strikethrough/hard-break degrade to
  plain text/space, lists as prefixed paragraphs, table cells flattened.
- **Recursion-depth DoS guard (CWE-674).** The AST comes from untrusted Markdown,
  which the upstream parser doesn't cap (`>>>>…>` nests blockquotes, `***…*`
  emphasis, arbitrarily deep); a naive recursive walker would overflow the native
  stack (an uncatchable SIGSEGV). Both the block and inline walkers are bounded to
  `MAX_DEPTH` (256) — past that they drop the over-deep subtree instead of
  recursing. Verified adversarially: a 50 000-deep AST converts without
  overflowing, over-deep content is dropped, and neutering the guard makes the
  boundary test render the buried content (confirming it's load-bearing).
- 14 tests: `word/document.xml` shape assertions (styles + run properties, via
  the independent `opc` reader), a round-trip through the `wordprocessingml`
  reader (visible text + block structure), and the depth-guard tests above.
  `#![forbid(unsafe_code)]`, clippy-clean, total and panic-free on any AST.
