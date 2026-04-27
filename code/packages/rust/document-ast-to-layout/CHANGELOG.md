# Changelog

## [0.1.0] — initial release

### Added
- `DocumentTheme` struct — a complete, explicit set of visual tokens (body / h1..h6 / code / blockquote fonts, text / heading / link / code / blockquote colors, paragraph / heading / list / blockquote / code-block spacing, page width + padding).
- `document_default_theme()` — a legible, system-default theme. Uses the empty string for font family so the renderer resolves the OS default UI font.
- `document_ast_to_layout(&DocumentNode, &DocumentTheme) -> LayoutNode` — the UI06 conversion entry point.
- Block mappings for: Document (root), Heading (1..=6, level→font size), Paragraph, CodeBlock (trailing newline stripped, monospace font, background rect via `ext["paint"]`), Blockquote (padded container with background + border), List (ordered/unordered with numeric/bullet markers, proper left indent), ListItem, TaskItem (ASCII `[ ]` marker), ThematicBreak.
- `flatten_inline_text(&[InlineNode]) -> String` — concatenates inline content with v1 styling-lost simplification. Handles: Text, Emphasis, Strong, Strikethrough, CodeSpan (value only), Link (inner text + URL when they differ), Image (`[image: alt]`), Autolink (URL), RawInline (stripped), HardBreak (newline), SoftBreak (space).
- Ext bag usage for `paint` namespace (backgroundColor, borderColor, borderWidth, cornerRadius) and `block` namespace (display: "block"). Each is a nested Map of `ExtValue` entries.
- 13 unit tests covering default theme, empty document, heading levels, paragraph font, inline flattening (plain / strong nesting / softbreak / hardbreak / emphasis nesting / link with and without URL annotation), blockquote carry-through, ordered/unordered list markers, and code-block newline trimming.

### Design
- Theme is the **single source of truth**. No cascade, no inheritance, no CSS shorthand. Every emitted `LayoutNode` carries a fully-resolved `FontSpec` and `Color`.
- Inline styling lost in v1 is **documented**, not ignored. Upgrading to per-run styled children is an isolated refactor localised to the paragraph/heading converters; the block structure does not change.
- Placeholder empty `LayoutNode`s are emitted for unsupported blocks (Table, RawBlock) so the downstream layout engine never encounters a missing node.
