# document-ast-to-layout

UI06 implementation. Converts a `DocumentNode` (from the CommonMark /
GFM parser) into a `LayoutNode` tree with a full `DocumentTheme`
applied. Zero runtime dependencies beyond the repo's own crates.

Spec: [code/specs/UI06-document-ast-to-layout.md](../../../specs/UI06-document-ast-to-layout.md).

## Exports

- `document_ast_to_layout(&DocumentNode, &DocumentTheme) -> LayoutNode`
- `DocumentTheme` struct (typography, colors, spacing, page config)
- `document_default_theme() -> DocumentTheme`
- `flatten_inline_text(&[InlineNode]) -> String` helper

## v1 scope

Implements the common block structures: **Document, Heading (1..=6),
Paragraph, CodeBlock, Blockquote, List (ordered + unordered), ListItem,
TaskItem, ThematicBreak**.

Inline formatting — `Emphasis`, `Strong`, `CodeSpan`, `Link` color — is
**flattened to plain text** for the MVP. The paragraph becomes a single
`TextContent` with the base font; bold/italic/link styling is lost in
v1. A future PR will turn each distinct inline run into a separate
layout child so the block layout engine can stitch styled spans on a
single line.

`SoftBreak` → space; `HardBreak` → newline; `Link` → inner text followed
by `(url)` when the URL differs from the text; `Autolink` → URL; `Image`
→ `[image: alt]`; `RawInline` → stripped.

Tables, raw blocks, and inline images render as empty placeholders in
v1. Explicit per-node handling is a v2 task.
