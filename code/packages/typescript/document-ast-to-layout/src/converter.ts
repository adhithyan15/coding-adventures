/**
 * Document AST → LayoutNode converter
 *
 * This module is the bridge between the Document AST (TE00) and the layout
 * pipeline. It walks a `DocumentNode` tree and emits a `LayoutNode` tree that
 * the `layout-block` algorithm can position, and `layout-to-paint` can render.
 *
 * === Architecture ===
 *
 * The Document AST distinguishes two node kinds:
 *
 *   BlockNode  — structural skeleton: headings, paragraphs, lists, tables …
 *   InlineNode — formatted spans inside blocks: text, bold, links, images …
 *
 * The Layout IR flattens this into:
 *
 *   Container node  — content=null, children=[…]
 *   Leaf text node  — content={ kind:"text" }
 *   Leaf image node — content={ kind:"image" }
 *
 * The `ext["block"]` namespace carries block-flow semantics:
 *
 *   { display: "block" }  — stacks vertically (default for all containers)
 *   { display: "inline" } — flows horizontally in line boxes
 *
 * The `ext["paint"]` namespace carries visual decoration.
 *
 * === Inline flattening ===
 *
 * Inline content (paragraph, heading, list item text) is "flattened" into a
 * sequence of leaf text/image nodes, each carrying a fully-resolved `FontSpec`
 * and `Color`. Nesting (e.g. bold inside a link) is resolved by inheriting the
 * parent font and applying the child modifier.
 *
 *   ParagraphNode
 *     TextNode { "Hello " }
 *     StrongNode
 *       TextNode { "world" }
 *     TextNode { "!" }
 *
 * becomes:
 *
 *   Container (block, paragraphSpacing)
 *     LeafText "Hello "   font=regular, display=inline
 *     LeafText "world"    font=bold,    display=inline
 *     LeafText "!"        font=regular, display=inline
 *
 * === List layout ===
 *
 * Lists are a block container with one block child per item. Each item is
 * itself a block container with two inline children: a bullet/number label
 * and the item content.
 *
 *   ListNode { ordered:false }
 *     ListItemNode
 *       ParagraphNode { "first" }
 *
 * becomes:
 *
 *   Container (block, listIndent padding-left)
 *     Container (block, row-ish)
 *       LeafText "•"   display=inline
 *       Container (block) ← item body
 *         LeafText "first"  display=inline
 *
 * === Table layout ===
 *
 * Tables use CSS Grid layout via `ext["grid"]`.
 *
 *   TableNode { align: ["left","right"], rows: [header, body…] }
 *
 * becomes:
 *
 *   Container (grid, templateColumns="1fr 1fr")
 *     Container (grid item, columnStart=1, paint={background:headerBg})
 *       LeafText (header cell content)
 *     Container (grid item, columnStart=2, …)
 *       …
 *
 * === Skipped node types ===
 *
 *   raw_block  / raw_inline — output-format-specific; skipped (format≠"layout")
 *   soft_break              — rendered as a single space
 *   hard_break              — rendered as a newline text node "\n"
 *   thematic_break          — rendered as a thin horizontal rule container
 *
 * @module converter
 */

import type {
  DocumentNode,
  BlockNode,
  InlineNode,
  ListChildNode,
} from "@coding-adventures/document-ast";
import type { LayoutNode, FontSpec, Color, Edges } from "@coding-adventures/layout-ir";
import {
  rgb,
  rgba,
  font_spec,
  font_bold,
  font_italic,
  size_fill,
  size_wrap,
  size_fixed,
  edges_all,
  edges_zero,
} from "@coding-adventures/layout-ir";

// ─── Theme ────────────────────────────────────────────────────────────────────

/**
 * Visual + typographic configuration injected by the caller.
 *
 * All measurements are in logical units (the same units used throughout the
 * layout IR). A "logical unit" corresponds to one CSS pixel at 1× DPR.
 *
 * ```
 * const theme = document_default_theme();
 * const layout = document_ast_to_layout(doc, theme);
 * ```
 */
export interface DocumentLayoutTheme {
  /** Base prose font — used for paragraph text, list items, table cells. */
  bodyFont: FontSpec;
  /** Monospace font — used for code blocks and inline code spans. */
  codeFont: FontSpec;
  /**
   * Heading scale factors relative to `bodyFont.size`.
   * Index 0 = h1, index 5 = h6.
   *
   * ```
   * headingScale[0] = 2.0  → h1 is 2× bodyFont.size
   * headingScale[5] = 0.85 → h6 is 0.85× bodyFont.size
   * ```
   */
  headingScale: [number, number, number, number, number, number];
  /** Vertical spacing between top-level block nodes (margin-bottom on each block). */
  paragraphSpacing: number;
  /** Left indent for blockquote content. */
  blockquoteIndent: number;
  /** Left padding for list items (bullet or number lives in this indent). */
  listIndent: number;
  /** Spacing between list items. */
  listItemSpacing: number;
  /** Padding inside code blocks. */
  codePadding: number;
  /** Padding inside table header cells. */
  tableCellPadding: number;
  /** Named colors. */
  colors: {
    /** Default prose text color. */
    text: Color;
    /** Heading text color. */
    heading: Color;
    /** Code/pre text color. */
    code: Color;
    /** Code block background. */
    codeBackground: Color;
    /** Link text color. */
    link: Color;
    /** Blockquote left-border color. */
    blockquoteBorder: Color;
    /** Blockquote background tint. */
    blockquoteBackground: Color;
    /** Table header cell background. */
    tableHeaderBackground: Color;
    /** Horizontal rule / thematic break color. */
    hrColor: Color;
  };
}

/**
 * Returns the default document layout theme — neutral grays, 16 px base,
 * 1.5 line-height, system-ui font stack.
 *
 * ```
 * const tree = document_ast_to_layout(doc, document_default_theme());
 * ```
 */
export function document_default_theme(): DocumentLayoutTheme {
  const body = font_spec("system-ui", 16);
  const code = font_spec("monospace", 14);
  return {
    bodyFont: body,
    codeFont: code,
    headingScale: [2.0, 1.5, 1.25, 1.1, 1.0, 0.85],
    paragraphSpacing: 16,
    blockquoteIndent: 24,
    listIndent: 32,
    listItemSpacing: 8,
    codePadding: 12,
    tableCellPadding: 8,
    colors: {
      text: rgb(30, 30, 30),
      heading: rgb(10, 10, 10),
      code: rgb(180, 60, 60),
      codeBackground: rgb(246, 246, 246),
      link: rgb(0, 86, 179),
      blockquoteBorder: rgb(180, 180, 180),
      blockquoteBackground: rgb(248, 248, 248),
      tableHeaderBackground: rgb(240, 240, 240),
      hrColor: rgb(200, 200, 200),
    },
  };
}

// ─── Public API ───────────────────────────────────────────────────────────────

/**
 * Convert a `DocumentNode` AST to a `LayoutNode` tree.
 *
 * The returned tree uses:
 *   - `ext["block"]` for block-flow semantics (pass to `layout-block`)
 *   - `ext["paint"]` for visual decoration (consumed by `layout-to-paint`)
 *
 * Usage:
 *
 * ```typescript
 * import { document_ast_to_layout, document_default_theme } from "…";
 * import { layout_block } from "@coding-adventures/layout-block";
 * import { createEstimatedMeasurer } from "@coding-adventures/layout-text-measure-estimated";
 *
 * const theme   = document_default_theme();
 * const tree    = document_ast_to_layout(doc, theme);
 * const measurer = createEstimatedMeasurer();
 * const positioned = layout_block(tree, { maxWidth: 800, maxHeight: Infinity, minWidth: 0, minHeight: 0 }, measurer);
 * ```
 *
 * @param doc   The root of the Document AST produced by any front-end parser.
 * @param theme Visual and typographic configuration.
 * @returns     A `LayoutNode` tree ready for `layout-block`.
 */
export function document_ast_to_layout(
  doc: DocumentNode,
  theme: DocumentLayoutTheme
): LayoutNode {
  const children = doc.children
    .flatMap((block) => convertBlock(block, theme, theme.bodyFont))
    .filter((n): n is LayoutNode => n !== null);
  return blockContainer(children, { width: size_fill() });
}

// ─── Block conversion ─────────────────────────────────────────────────────────

/**
 * Convert a single block node to zero or more `LayoutNode`s.
 *
 * Most block types produce exactly one node. A `raw_block` produces zero nodes
 * (it is skipped). Tables produce exactly one grid container.
 *
 * @param block        The block to convert.
 * @param theme        Current theme.
 * @param inheritFont  The inherited font context (body font at top level;
 *                     code font inside code blocks).
 */
function convertBlock(
  block: BlockNode,
  theme: DocumentLayoutTheme,
  inheritFont: FontSpec
): LayoutNode[] {
  switch (block.type) {
    case "document": {
      // A nested document shouldn't appear, but handle it gracefully.
      return block.children.flatMap((b) => convertBlock(b, theme, inheritFont));
    }

    case "heading": {
      // h1–h6: collect inline children as text leaves, set heading font.
      // font_spec only accepts (family, size) — apply bold separately.
      const scale = theme.headingScale[block.level - 1];
      const headingFont = font_bold(
        font_spec(inheritFont.family, Math.round(inheritFont.size * scale))
      );
      const inlines = convertInlines(block.children, headingFont, theme, "heading");
      const node = blockContainer(inlines, {
        width: size_fill(),
        marginBottom: theme.paragraphSpacing * 0.5,
      });
      return [node];
    }

    case "paragraph": {
      const inlines = convertInlines(block.children, inheritFont, theme, "body");
      const node = blockContainer(inlines, {
        width: size_fill(),
        marginBottom: theme.paragraphSpacing,
      });
      return [node];
    }

    case "code_block": {
      // Pre-formatted monospace block with code background.
      const text = block.value;
      const leaf: LayoutNode = {
        content: {
          kind: "text",
          value: text,
          font: theme.codeFont,
          color: theme.colors.code,
          maxLines: null,
          textAlign: "start",
        },
        children: [],
        width: size_fill(),
        height: null,
        padding: edges_all(theme.codePadding),
        margin: { top: 0, right: 0, bottom: theme.paragraphSpacing, left: 0 },
        ext: {
          block: { display: "block", whiteSpace: "pre" },
          paint: { backgroundColor: theme.colors.codeBackground },
        },
      };
      return [leaf];
    }

    case "blockquote": {
      // Indented container with a left border tint and background.
      const innerChildren = block.children.flatMap((b) =>
        convertBlock(b, theme, inheritFont)
      );
      // Remove margin-bottom from the last child to avoid double spacing
      const node: LayoutNode = {
        content: null,
        children: innerChildren,
        width: size_fill(),
        height: null,
        padding: {
          top: 8,
          right: 16,
          bottom: 8,
          left: theme.blockquoteIndent,
        },
        margin: { top: 0, right: 0, bottom: theme.paragraphSpacing, left: 0 },
        ext: {
          block: { display: "block" },
          paint: {
            backgroundColor: theme.colors.blockquoteBackground,
            borderColor: theme.colors.blockquoteBorder,
            borderWidth: 4,
          },
          blockquote: true,
        },
      };
      return [node];
    }

    case "list": {
      const items = block.children.flatMap((item, idx) =>
        convertListItem(item, idx, block.ordered, block.start, theme, inheritFont)
      );
      const node: LayoutNode = {
        content: null,
        children: items,
        width: size_fill(),
        height: null,
        padding: { top: 0, right: 0, bottom: 0, left: theme.listIndent },
        margin: { top: 0, right: 0, bottom: theme.paragraphSpacing, left: 0 },
        ext: { block: { display: "block" } },
      };
      return [node];
    }

    case "list_item":
    case "task_item": {
      // These are reached only when a list item appears outside a ListNode —
      // e.g. inside a blockquote or another list item. Delegate directly.
      return convertListItem(block, 0, false, null, theme, inheritFont);
    }

    case "thematic_break": {
      // A horizontal rule: a thin filled container.
      const node: LayoutNode = {
        content: null,
        children: [],
        width: size_fill(),
        height: size_fixed(1),
        padding: edges_zero(),
        margin: { top: theme.paragraphSpacing * 0.5, right: 0, bottom: theme.paragraphSpacing * 0.5, left: 0 },
        ext: {
          block: { display: "block" },
          paint: { backgroundColor: theme.colors.hrColor },
        },
      };
      return [node];
    }

    case "raw_block": {
      // Skip — layout back-end does not handle raw HTML/LaTeX/etc.
      return [];
    }

    case "table": {
      return [convertTable(block.align, block.children, theme, inheritFont)];
    }

    case "table_row":
    case "table_cell": {
      // These are reached only if rows/cells appear outside a table — skip.
      return [];
    }

    default: {
      // Exhaustiveness guard — TypeScript will warn if a new block type is added.
      return [];
    }
  }
}

// ─── List item conversion ─────────────────────────────────────────────────────

/**
 * Convert a `ListItemNode` or `TaskItemNode` into a row containing a bullet
 * label and the item body.
 *
 * The layout looks like:
 *
 * ```
 * Container (block, flex-like row via inline children)
 *   LeafText "•"    (bullet, display=inline, fixed width)
 *   Container (block, flex=1)  ← item body
 *     … block children …
 * ```
 *
 * Because `layout-block` supports inline siblings, the bullet and body will
 * flow side-by-side in the same line box. However, for tight layout where the
 * body has multiple lines, we need the body to be a block that grows.
 *
 * Practical approach: We put bullet and body into a flex-row container via
 * `ext["flex"]` so the bullet stays left and the body fills the rest.
 */
function convertListItem(
  item: ListChildNode,
  index: number,
  ordered: boolean,
  start: number | null,
  theme: DocumentLayoutTheme,
  inheritFont: FontSpec
): LayoutNode[] {
  // Determine bullet label
  let bulletText: string;
  if (item.type === "task_item") {
    bulletText = item.checked ? "☑ " : "☐ ";
  } else if (ordered) {
    const n = (start ?? 1) + index;
    bulletText = `${n}. `;
  } else {
    bulletText = "• ";
  }

  const bullet: LayoutNode = {
    content: {
      kind: "text",
      value: bulletText,
      font: inheritFont,
      color: theme.colors.text,
      maxLines: 1,
      textAlign: "start",
    },
    children: [],
    width: size_fixed(24),
    height: null,
    ext: {
      flex: { flexShrink: 0 },
    },
  };

  const bodyChildren = item.children.flatMap((b) =>
    convertBlock(b, theme, inheritFont)
  );
  const body: LayoutNode = {
    content: null,
    children: bodyChildren,
    width: null,
    height: null,
    ext: {
      flex: { flexGrow: 1 },
      block: { display: "block" },
    },
  };

  const row: LayoutNode = {
    content: null,
    children: [bullet, body],
    width: size_fill(),
    height: null,
    margin: { top: 0, right: 0, bottom: theme.listItemSpacing, left: 0 },
    ext: {
      flex: { direction: "row", gap: 0 },
    },
  };

  return [row];
}

// ─── Table conversion ─────────────────────────────────────────────────────────

import type { TableAlignment, TableRowNode } from "@coding-adventures/document-ast";

/**
 * Convert a GFM table to a CSS Grid layout node.
 *
 * The number of columns is inferred from the `align` array on the TableNode.
 * Each cell becomes a grid item with an explicit `columnStart` so the grid
 * algorithm places them correctly even if a row has fewer cells than columns.
 *
 * ```
 * TableNode { align: ["left","right"] }
 *
 * Grid: templateColumns = "1fr 1fr"
 *   Cell (row=1,col=1) "Header A"
 *   Cell (row=1,col=2) "Header B"
 *   Cell (row=2,col=1) "Body A"
 *   Cell (row=2,col=2) "Body B"
 * ```
 */
function convertTable(
  align: readonly TableAlignment[],
  rows: readonly TableRowNode[],
  theme: DocumentLayoutTheme,
  inheritFont: FontSpec
): LayoutNode {
  const colCount = align.length || 1;
  const templateColumns = Array(colCount).fill("1fr").join(" ");

  const cells: LayoutNode[] = [];
  rows.forEach((row, rowIdx) => {
    row.children.forEach((cell, colIdx) => {
      const isHeader = row.isHeader;
      const font = isHeader ? font_bold(inheritFont) : inheritFont;
      const inlines = convertInlines(cell.children, font, theme, isHeader ? "heading" : "body");
      const cellNode: LayoutNode = {
        content: null,
        children: inlines,
        width: null,
        height: null,
        padding: edges_all(theme.tableCellPadding),
        ext: {
          grid: {
            columnStart: colIdx + 1,
            rowStart: rowIdx + 1,
          },
          block: { display: "block" },
          paint: isHeader
            ? { backgroundColor: theme.colors.tableHeaderBackground }
            : undefined,
        },
      };
      cells.push(cellNode);
    });
  });

  return {
    content: null,
    children: cells,
    width: size_fill(),
    height: null,
    margin: { top: 0, right: 0, bottom: theme.paragraphSpacing, left: 0 },
    ext: {
      grid: { templateColumns, gap: 1 },
    },
  };
}

// ─── Inline conversion ────────────────────────────────────────────────────────

/**
 * Convert an array of inline nodes to an array of leaf `LayoutNode`s.
 *
 * Each inline node may produce zero or more leaves. The function is recursive
 * for wrapper nodes (emphasis, strong, strikethrough, link).
 *
 * @param inlines   The inline nodes to convert.
 * @param font      The inherited (currently active) font spec.
 * @param theme     Theme for colors.
 * @param colorRole "body" = `theme.colors.text`; "heading" = `theme.colors.heading`;
 *                  "link" = `theme.colors.link`; "code" = `theme.colors.code`.
 */
function convertInlines(
  inlines: readonly InlineNode[],
  font: FontSpec,
  theme: DocumentLayoutTheme,
  colorRole: "body" | "heading" | "link" | "code"
): LayoutNode[] {
  const color = roleColor(colorRole, theme);
  const result: LayoutNode[] = [];

  for (const inline of inlines) {
    switch (inline.type) {
      case "text": {
        if (inline.value === "") break;
        result.push(inlineText(inline.value, font, color));
        break;
      }

      case "soft_break": {
        // A soft break renders as a single space in flowing text.
        result.push(inlineText(" ", font, color));
        break;
      }

      case "hard_break": {
        // A hard break inserts a real newline (layout-block treats \n as a
        // line boundary in inline content when whiteSpace="pre").
        result.push(inlineText("\n", font, color));
        break;
      }

      case "emphasis": {
        const emphFont = font_italic(font);
        result.push(...convertInlines(inline.children, emphFont, theme, colorRole));
        break;
      }

      case "strong": {
        const boldFont = font_bold(font);
        result.push(...convertInlines(inline.children, boldFont, theme, colorRole));
        break;
      }

      case "strikethrough": {
        // No strikethrough support in the layout IR — render as regular text
        // tagged so back-ends that do support it can detect it.
        const children = convertInlines(inline.children, font, theme, colorRole);
        children.forEach((n) => {
          (n.ext as Record<string, unknown>)["strikethrough"] = true;
        });
        result.push(...children);
        break;
      }

      case "code_span": {
        result.push(inlineText(inline.value, theme.codeFont, theme.colors.code));
        break;
      }

      case "link": {
        // Render link children with link color; attach destination as ext.
        const linkChildren = convertInlines(inline.children, font, theme, "link");
        linkChildren.forEach((n) => {
          (n.ext as Record<string, unknown>)["link"] = inline.destination;
        });
        result.push(...linkChildren);
        break;
      }

      case "autolink": {
        const dest = inline.isEmail
          ? `mailto:${inline.destination}`
          : inline.destination;
        const leaf = inlineText(inline.destination, font, theme.colors.link);
        (leaf.ext as Record<string, unknown>)["link"] = dest;
        result.push(leaf);
        break;
      }

      case "image": {
        // Inline image — render as a leaf image node with display:inline.
        const img: LayoutNode = {
          content: {
            kind: "image",
            src: inline.destination,
            fit: "contain",
          },
          children: [],
          width: size_wrap(),
          height: size_wrap(),
          ext: {
            block: { display: "inline" },
            imageAlt: inline.alt,
          },
        };
        result.push(img);
        break;
      }

      case "raw_inline": {
        // Skip — layout back-end ignores format-specific raw spans.
        break;
      }
    }
  }

  return result;
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/**
 * Map a color role string to the appropriate theme color.
 *
 * Color roles allow inline converters to be called with the right color without
 * threading the full theme color struct everywhere.
 *
 *   "body"    → theme.colors.text     (default prose color)
 *   "heading" → theme.colors.heading  (heading text)
 *   "link"    → theme.colors.link     (hyperlink)
 *   "code"    → theme.colors.code     (code span / code block)
 */
function roleColor(
  role: "body" | "heading" | "link" | "code",
  theme: DocumentLayoutTheme
): Color {
  switch (role) {
    case "body":    return theme.colors.text;
    case "heading": return theme.colors.heading;
    case "link":    return theme.colors.link;
    case "code":    return theme.colors.code;
  }
}

/**
 * Create a leaf text node with `display: inline`.
 *
 * Every inline text node produced by this converter uses `display: inline`
 * so that `layout-block` flows them left-to-right within a line box.
 */
function inlineText(value: string, font: FontSpec, color: Color): LayoutNode {
  return {
    content: {
      kind: "text",
      value,
      font,
      color,
      maxLines: null,
      textAlign: "start",
    },
    children: [],
    width: null,
    height: null,
    ext: { block: { display: "inline" } },
  };
}

/**
 * Options for `blockContainer`.
 *
 * `marginBottom` is the most common property to set — it controls vertical
 * spacing between sibling blocks (analogous to `margin-bottom` in CSS).
 */
interface BlockContainerOpts {
  width?: LayoutNode["width"];
  marginBottom?: number;
  padding?: Edges;
}

/**
 * Create a block container node.
 *
 * A block container has `content = null` and `ext["block"] = { display:"block" }`.
 * It stacks its children vertically when processed by `layout-block`.
 */
function blockContainer(children: LayoutNode[], opts: BlockContainerOpts = {}): LayoutNode {
  return {
    content: null,
    children,
    width: opts.width ?? null,
    height: null,
    padding: opts.padding ?? null,
    margin: opts.marginBottom !== undefined
      ? { top: 0, right: 0, bottom: opts.marginBottom, left: 0 }
      : null,
    ext: { block: { display: "block" } },
  };
}
