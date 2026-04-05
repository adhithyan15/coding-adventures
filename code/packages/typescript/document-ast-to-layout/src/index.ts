/**
 * @coding-adventures/document-ast-to-layout
 *
 * Converts a Document AST (TE00) into a LayoutNode tree for the block-flow
 * layout algorithm (`layout-block`).
 *
 * This is one back-end in the Document AST ecosystem. It accepts any
 * `DocumentNode` produced by any front-end parser (Markdown, RST, HTML, …)
 * and emits a `LayoutNode` tree ready for `layout-block` to position.
 *
 * === Quick Start ===
 *
 * ```typescript
 * import { parse } from "@coding-adventures/commonmark-parser";
 * import {
 *   document_ast_to_layout,
 *   document_default_theme,
 * } from "@coding-adventures/document-ast-to-layout";
 * import { layout_block } from "@coding-adventures/layout-block";
 * import { createEstimatedMeasurer } from "@coding-adventures/layout-text-measure-estimated";
 *
 * const doc      = parse("# Hello\n\nWorld!\n");
 * const theme    = document_default_theme();
 * const tree     = document_ast_to_layout(doc, theme);
 * const measurer = createEstimatedMeasurer();
 * const result   = layout_block(tree, { maxWidth: 800, maxHeight: Infinity, minWidth: 0, minHeight: 0 }, measurer);
 * ```
 *
 * === Ext namespace summary ===
 *
 * The returned `LayoutNode` tree uses the following `ext` namespaces:
 *
 * | Key            | Set on           | Purpose                                 |
 * |----------------|------------------|-----------------------------------------|
 * | `block`        | every container  | `{ display: "block" | "inline" }`       |
 * | `paint`        | styled nodes     | `backgroundColor`, `borderColor`, …     |
 * | `flex`         | list item rows   | `{ direction: "row" }` bullet+body      |
 * | `grid`         | table cells      | `{ columnStart, rowStart }`             |
 * | `link`         | link text leaves | `string` — the href destination         |
 * | `imageAlt`     | image leaves     | `string` — the alt text                 |
 * | `strikethrough`| struck text      | `true` — visual hint for renderers      |
 * | `blockquote`   | blockquote box   | `true` — semantic tag                   |
 *
 * Spec: TE00 — Document AST
 *
 * @module index
 */

export { document_ast_to_layout, document_default_theme } from "./converter.js";
export type { DocumentLayoutTheme } from "./converter.js";
