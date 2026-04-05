# Changelog

## 0.1.0 — Initial release

### Added
- `document_ast_to_layout(doc, theme)` — converts a `DocumentNode` AST to a
  `LayoutNode` tree using `ext["block"]` for block-flow semantics and
  `ext["paint"]` for visual decoration.
- `DocumentLayoutTheme` interface — configures fonts, heading scales, spacing,
  indents, and all named colors.
- `document_default_theme()` — sensible defaults (16 px body, monospace code,
  system-ui, neutral grays).
- Block node support:
  - `heading` (h1–h6) — scaled bold font, heading color, margin-bottom
  - `paragraph` — body font, text color, margin-bottom
  - `code_block` — monospace leaf with code background and `whiteSpace:"pre"`
  - `blockquote` — indented container with left border, tinted background
  - `list` (ordered + unordered) — block container with indent padding; each
    item is a flex row with bullet/number + body
  - `task_item` — bullet is ☐ / ☑ based on `checked` flag
  - `thematic_break` — 1 px horizontal rule with hr color
  - `raw_block` — skipped (format-specific, not handled by layout back-end)
  - `table` — CSS Grid container; header cells are bold with header background;
    every cell carries `{ columnStart, rowStart }` grid placement
- Inline node support:
  - `text` — leaf with current inherited font and color
  - `emphasis` — italic font modifier
  - `strong` — bold font modifier (weight 700)
  - `strikethrough` — regular font + `ext["strikethrough"] = true` tag
  - `code_span` — monospace font with code color
  - `link` — link color + `ext["link"] = destination`
  - `autolink` — link color + `ext["link"]` (email prepended with `mailto:`)
  - `image` (inline) — leaf image with `display:inline`, `ext["imageAlt"]`
  - `soft_break` — single space text node
  - `hard_break` — newline text node
  - `raw_inline` — skipped
- Nested document nodes are flattened (children hoisted up).
- Orphan `list_item`, `task_item` outside a `ListNode` are converted at index 0.
- Orphan `table_row`, `table_cell` are skipped.
- 70 unit tests, 99.49 % statement coverage, 92.85 % branch coverage.
