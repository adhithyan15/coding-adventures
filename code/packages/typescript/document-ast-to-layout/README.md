# document-ast-to-layout

Converts a **Document AST** (TE00) into a **LayoutNode** tree suitable for
block-flow layout (`layout-block`).

This is the bridge between the document processing pipeline (Markdown → AST)
and the layout rendering pipeline (LayoutNode → PaintScene → pixels).

---

## What is the Document AST?

`document-ast` defines a format-agnostic IR for structured documents. Any
front-end parser (Markdown, RST, HTML, DOCX) produces a `DocumentNode` tree;
any back-end renderer (HTML, PDF, plain text, **layout**) consumes it.

## What is a LayoutNode?

`layout-ir` defines the data structure consumed by `layout-block` (block/inline
flow), `layout-flexbox`, and `layout-grid`. It models boxes with dimensions,
padding, margin, flex/grid extensions, and paint decorations.

---

## Position in the pipeline

```
.md / .rst / HTML source
         │
   front-end parser     — e.g. commonmark-parser
         │
   document-ast         — DocumentNode tree
         │
   document-ast-to-layout  ← YOU ARE HERE
         │
   layout-block         — position nodes (block/inline flow)
         │
   layout-to-paint      — emit paint instructions
         │
   renderer             — HTML canvas, WebGL, PDF, …
```

---

## Installation

```bash
npm install coding-adventures-document-ast-to-layout
```

---

## Usage

```typescript
import { parse } from "@coding-adventures/commonmark-parser";
import {
  document_ast_to_layout,
  document_default_theme,
} from "@coding-adventures/document-ast-to-layout";
import { layout_block } from "@coding-adventures/layout-block";
import { createEstimatedMeasurer } from "@coding-adventures/layout-text-measure-estimated";

const doc      = parse("# Hello\n\nWorld!\n");
const theme    = document_default_theme();
const tree     = document_ast_to_layout(doc, theme);
const measurer = createEstimatedMeasurer();

const positioned = layout_block(
  tree,
  { minWidth: 0, maxWidth: 800, minHeight: 0, maxHeight: Infinity },
  measurer
);
```

---

## API

### `document_ast_to_layout(doc, theme)`

| Parameter | Type                   | Description                           |
|-----------|------------------------|---------------------------------------|
| `doc`     | `DocumentNode`         | Root of any Document AST              |
| `theme`   | `DocumentLayoutTheme`  | Fonts, colors, spacing configuration  |

Returns a `LayoutNode` — the root of the layout tree.

### `document_default_theme()`

Returns a sensible default theme:

| Setting             | Value                        |
|---------------------|------------------------------|
| Body font           | system-ui 16 px, regular     |
| Code font           | monospace 14 px              |
| Heading scale       | 2.0 / 1.5 / 1.25 / 1.1 / 1.0 / 0.85 |
| Paragraph spacing   | 16 px                        |
| Blockquote indent   | 24 px                        |
| List indent         | 32 px                        |
| Text color          | rgb(30, 30, 30)              |
| Heading color       | rgb(10, 10, 10)              |
| Link color          | rgb(0, 86, 179)              |
| Code color          | rgb(180, 60, 60)             |

---

## Block node mapping

| Document AST node  | LayoutNode shape                                    |
|--------------------|-----------------------------------------------------|
| `heading` (h1–h6)  | Block container with scaled bold font inline leaves |
| `paragraph`        | Block container with inline text leaves             |
| `code_block`       | Leaf text, monospace, `whiteSpace:"pre"`, code bg   |
| `blockquote`       | Block container, left border + tinted background    |
| `list`             | Block container with left padding                   |
| `list_item`        | Flex row: bullet leaf + body block                  |
| `task_item`        | Flex row: ☐/☑ leaf + body block                     |
| `thematic_break`   | 1 px filled horizontal container                    |
| `raw_block`        | **Skipped** — format-specific, not layout           |
| `table`            | CSS Grid container with explicit cell placement     |

---

## Inline node mapping

| Inline node      | LayoutNode                                          |
|------------------|-----------------------------------------------------|
| `text`           | Leaf text, inherited font + color                   |
| `emphasis`       | Leaf text, italic font                              |
| `strong`         | Leaf text, bold font (weight 700)                   |
| `strikethrough`  | Leaf text + `ext["strikethrough"] = true`           |
| `code_span`      | Leaf text, monospace font, code color               |
| `link`           | Leaf text, link color + `ext["link"] = destination` |
| `autolink`       | Leaf text, link color + `ext["link"]`               |
| `image`          | Leaf image, `display: inline`, `ext["imageAlt"]`    |
| `soft_break`     | Leaf text `" "`                                     |
| `hard_break`     | Leaf text `"\n"`                                    |
| `raw_inline`     | **Skipped**                                         |

---

## Ext namespace reference

| Key              | Set on           | Value                                    |
|------------------|------------------|------------------------------------------|
| `block`          | every container  | `{ display: "block" \| "inline" }`       |
| `paint`          | styled nodes     | `{ backgroundColor, borderColor, … }`   |
| `flex`           | list item rows   | `{ direction: "row" }`                   |
| `grid`           | table cells      | `{ columnStart, rowStart }`              |
| `link`           | link text leaves | `string` — the href destination          |
| `imageAlt`       | image leaves     | `string` — the alt text                  |
| `strikethrough`  | struck text      | `true` — renderer hint                   |
| `blockquote`     | blockquote box   | `true` — semantic tag                    |

---

## Customising the theme

```typescript
import { document_default_theme, type DocumentLayoutTheme } from "…";
import { rgb, font_spec, font_bold } from "@coding-adventures/layout-ir";

const theme: DocumentLayoutTheme = {
  ...document_default_theme(),
  bodyFont: font_spec("Georgia", 18),
  codeFont: font_spec("Fira Code", 13),
  paragraphSpacing: 20,
  headingScale: [2.5, 1.8, 1.4, 1.2, 1.0, 0.9],
  colors: {
    ...document_default_theme().colors,
    text: rgb(20, 20, 20),
    link: rgb(0, 100, 200),
  },
};
```
