# UI06 — document-ast-to-layout: DocumentAST → LayoutNode Tree

## Overview

`document-ast-to-layout` converts a `DocumentAST` (produced by the CommonMark
or GFM parser) into a `LayoutNode` tree with `BlockExt` fields populated,
ready for `layout-block`.

```
DocumentAST + DocumentTheme
    ↓  document_ast_to_layout()
LayoutNode tree (with ext["block"] and ext["paint"] populated)
    ↓  layout-block (UI07)
PositionedNode tree
    ↓  layout-to-paint (UI04)
PaintScene
    ↓  paint-vm backend
pixels
```

This replaces the current HTML emission path for document rendering in
non-browser contexts (Canvas, Metal, Direct2D, PDF). The HTML path remains
the right choice when targeting a browser DOM.

---

## Package: `document-ast-to-layout`

**Depends on:** `layout-ir`, `document-ast`, `commonmark-parser` (or `gfm-parser`)

**Exports:** `document_ast_to_layout`, `DocumentTheme`, `document_default_theme`

---

## Function signature

```
document_ast_to_layout(
  ast:   DocumentAST,
  theme: DocumentTheme
) → LayoutNode
```

Returns a single root `LayoutNode` (a block container) wrapping the entire
document.

---

## `DocumentTheme`

The theme is a complete, explicit set of visual tokens. There is no cascade,
no inheritance, no CSS. Every node gets a fully resolved `FontSpec` and
`Color` from the theme.

```
DocumentTheme {
  // Typography
  bodyFont:         FontSpec     // body text
  h1Font:           FontSpec
  h2Font:           FontSpec
  h3Font:           FontSpec
  h4Font:           FontSpec
  h5Font:           FontSpec
  h6Font:           FontSpec
  codeFont:         FontSpec     // inline code and code blocks
  blockquoteFont:   FontSpec

  // Colors
  textColor:        Color
  headingColor:     Color
  linkColor:        Color
  codeColor:        Color
  codeBgColor:      Color
  blockquoteBgColor:  Color
  blockquoteBorderColor: Color
  hrColor:          Color

  // Spacing (in logical units)
  paragraphSpacing:   float    // vertical gap between block elements
  headingSpacing:     float    // vertical gap above/below headings
  listIndent:         float    // left indent for list items
  listItemSpacing:    float    // vertical gap between list items
  blockquotePadding:  float    // padding inside blockquote
  codeBlockPadding:   float    // padding inside fenced code blocks
  hrHeight:           float    // height of horizontal rule
  tableRowHeight:     float    // minimum height of a table row
  tablePadding:       float    // cell padding in tables

  // Page
  pageWidth:    float    // maximum content width
  pagePadding:  Edges    // outer page padding
}
```

`document_default_theme()` returns a legible theme based on system defaults.

---

## Node mapping

### `Document` (root)

```
LayoutNode {
  width:  size_fill()
  height: size_wrap()
  padding: theme.pagePadding
  ext["block"] = { display: "block" }
  children: [converted block children]
}
```

### `Heading` (h1–h6)

```
LayoutNode {
  width:  size_fill()
  height: size_wrap()
  margin: edges_xy(0, theme.headingSpacing)
  ext["block"] = { display: "block" }
  ext["paint"] = { /* no background */ }
  children: [inline run from heading content]
}
```

Inline content (text, code spans, links) is flattened into a single
`TextContent` leaf with the appropriate heading `FontSpec` from theme.

For mixed inline content (e.g. heading with a code span inside), inline
elements are concatenated into a single string for the text node. Full
inline-level mixed layout (separate span per run) is handled by
`layout-block`'s inline flow — the leaf node here carries the heading
`FontSpec` and the heading text value.

### `Paragraph`

```
LayoutNode {
  width:  size_fill()
  height: size_wrap()
  margin: edges_xy(0, theme.paragraphSpacing / 2)
  ext["block"] = { display: "block" }
  children: [inline run nodes]
}
```

Inline content produces a sequence of inline `LayoutNode` children. Each
distinct run (bold, italic, code span, plain text) produces its own
`TextContent` leaf with appropriate `FontSpec` derived from theme by applying
bold/italic modifiers.

### `BlockQuote`

```
LayoutNode {
  width:  size_fill()
  height: size_wrap()
  padding: edges_all(theme.blockquotePadding)
  margin:  edges_xy(0, theme.paragraphSpacing)
  ext["block"] = { display: "block" }
  ext["paint"] = {
    backgroundColor: theme.blockquoteBgColor,
    borderWidth: 3,
    borderColor: theme.blockquoteBorderColor,
    borderSide: "left"     // hint to paint backend; left border only
  }
  children: [converted inner blocks]
}
```

### `BulletList` / `OrderedList`

A block container. Children are `ListItem` nodes.

```
LayoutNode {
  width:  size_fill()
  height: size_wrap()
  margin: edges_xy(0, theme.paragraphSpacing)
  ext["block"] = { display: "block" }
  children: [converted ListItem nodes]
}
```

### `ListItem`

```
LayoutNode {
  width:  size_fill()
  height: size_wrap()
  padding: edges_xy(theme.listIndent, 0)
  margin: edges_xy(0, theme.listItemSpacing)
  ext["block"] = { display: "block" }
  children: [
    // bullet/number marker inline node,
    // then content nodes
  ]
}
```

The marker (bullet `•` or number `1.`) is a `TextContent` leaf with
`bodyFont` and positioned at the left of the item's padding area.

### `FencedCode` / `IndentedCode`

```
LayoutNode {
  width:  size_fill()
  height: size_wrap()
  padding: edges_all(theme.codeBlockPadding)
  margin:  edges_xy(0, theme.paragraphSpacing)
  ext["block"] = { display: "block" }
  ext["paint"] = {
    backgroundColor: theme.codeBgColor,
    cornerRadius: 4
  }
  children: [
    TextContent {
      value:    raw code string (no syntax highlighting at this layer)
      font:     theme.codeFont
      color:    theme.codeColor
      maxLines: null
    }
  ]
}
```

Syntax highlighting is a future extension — a post-processing pass that splits
the single `TextContent` into multiple inline runs with different colors.

### `ThematicBreak` (horizontal rule)

```
LayoutNode {
  width:  size_fill()
  height: size_fixed(theme.hrHeight)
  margin: edges_xy(0, theme.paragraphSpacing)
  ext["paint"] = {
    backgroundColor: theme.hrColor
  }
}
```

### `Table`

```
LayoutNode {
  width:  size_fill()
  height: size_wrap()
  margin: edges_xy(0, theme.paragraphSpacing)
  ext["grid"] = {
    templateColumns: "repeat(N, 1fr)",   // N = number of columns
    gap: 1
  }
  children: [TableRow nodes (header first, then body rows)]
}
```

Table nodes use `ext["grid"]` — they are the first use of the grid extension.
Table rows are grid-row containers; cells are grid-item leaves.

### `TableRow`

```
LayoutNode {
  width:  size_fill()
  height: size_fixed(theme.tableRowHeight)
  ext["grid"] = { rowStart: row_index }
  children: [TableCell nodes]
}
```

### `TableCell`

```
LayoutNode {
  padding: edges_all(theme.tablePadding)
  ext["grid"] = { columnStart: col_index }
  ext["paint"] = {
    borderWidth: 1,
    borderColor: theme.hrColor
  }
  children: [inline run nodes]
}
```

### Inline text nodes

For plain text spans, bold, italic, bold-italic, and code spans within
paragraphs and headings:

```
LayoutNode {
  content: TextContent {
    value: text string
    font:  derived font (bold/italic flags applied to base font)
    color: theme.textColor (or theme.linkColor for links, theme.codeColor for code spans)
  }
  ext["block"] = { display: "inline" }
}
```

---

## Inline content flattening

`layout-block` (UI07) handles inline flow. This converter produces a sequence
of `display: "inline"` `LayoutNode` children for mixed inline content. Each
inline run is a separate leaf with its own `TextContent` carrying the resolved
`FontSpec`.

For simple paragraphs with no styling variation, a single inline leaf is
sufficient. For paragraphs with mixed bold/italic/code/link runs, each run
becomes its own inline leaf.

---

## What this package does NOT do

- Does not run any layout algorithm
- Does not handle browser-specific rendering (use the existing HTML/DOM path
  for browser targets)
- Does not perform syntax highlighting on code blocks
- Does not load or embed images (leaves `src` as a URL string in `ImageContent`)
- Does not handle HTML blocks or raw HTML inline (those require a sanitizer
  pass before conversion)
- Does not handle footnotes, definition lists, or other extended Markdown
  features beyond CommonMark and GFM table extension
