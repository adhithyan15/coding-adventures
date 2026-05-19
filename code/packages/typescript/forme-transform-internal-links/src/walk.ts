/**
 * walk.ts — AST walker that rewrites internal `LinkNode`
 * destinations to resolved URLs.
 *
 * Per-`LinkNode` algorithm:
 *
 *   1. If destination is NOT an internal slug → pass-through
 *      (return a fresh LinkNode copy with the original URL).
 *   2. Else call `resolver(slug)`.
 *   3. If resolver returned a string:
 *        - `assertResolvedUrl(...)` (throws on `javascript:` etc.).
 *        - Return a new LinkNode with the resolved URL.
 *   4. If resolver returned `null` (or `undefined`):
 *        - Apply `unresolved` policy:
 *            - `"keep"`: return LinkNode with original `/slug`.
 *            - `"strip"`: return the LinkNode's children (drops
 *              the wrapper; caller's inline list expands inline).
 *            - `"throw"`: throw `Error` with slug in message.
 *
 * The "strip" return type is unusual — a single `LinkNode` input
 * becomes multiple `InlineNode` outputs.  Handled by the
 * `inline-list-rewrite` helper which flat-maps over inline
 * children.
 *
 * Image, autolink, raw_inline are pass-through — they may carry
 * URLs but `image-rewrite` is a separate spec transform and
 * `autolink` is the user's explicit external URL.  Same for
 * `RawBlockNode` / `RawInlineNode` values.
 *
 * The walker descends every block / inline container so
 * `LinkNode`s nested inside `BlockquoteNode` / list items / table
 * cells get rewritten.
 *
 * @module walk
 */

import type {
  BlockNode,
  CodeBlockNode,
  CodeSpanNode,
  DocumentNode,
  ImageNode,
  InlineNode,
  LinkNode,
  ListChildNode,
  RawBlockNode,
  RawInlineNode,
  TableNode,
  TableRowNode,
  TableCellNode,
  ThematicBreakNode,
} from "@coding-adventures/document-ast";
import { assertResolvedUrl, isInternalSlug } from "./url.js";
import type { InternalLinksOptions, SlugResolver, UnresolvedPolicy } from "./types.js";

/**
 * Rewrite internal links throughout `doc`.  Returns a fresh
 * `DocumentNode`; `doc` is never mutated.
 *
 * Defaults: `unresolved: "keep"`.
 *
 * Throws `TypeError` if the resolver ever returns a string
 * outside the `http(s)://` / root-relative accept-list.
 * Throws `Error` if `unresolved: "throw"` and the resolver
 * returns `null` for any internal link.
 */
export function rewriteInternalLinks(
  doc: DocumentNode,
  resolver: SlugResolver,
  options: InternalLinksOptions = {},
): DocumentNode {
  const unresolved: UnresolvedPolicy = options.unresolved ?? "keep";
  return {
    type: "document",
    children: doc.children.map((b) => transformBlock(b, resolver, unresolved)),
  };
}

function transformBlock(
  b: BlockNode,
  resolver: SlugResolver,
  unresolved: UnresolvedPolicy,
): BlockNode {
  switch (b.type) {
    case "document":
      return {
        type: "document",
        children: b.children.map((c) => transformBlock(c, resolver, unresolved)),
      };
    case "heading":
      return {
        type: "heading",
        level: b.level,
        children: transformInlines(b.children, resolver, unresolved),
      };
    case "paragraph":
      return {
        type: "paragraph",
        children: transformInlines(b.children, resolver, unresolved),
      };
    case "blockquote":
      return {
        type: "blockquote",
        children: b.children.map((c) => transformBlock(c, resolver, unresolved)),
      };
    case "list":
      return {
        type: "list",
        ordered: b.ordered,
        start: b.start,
        tight: b.tight,
        children: b.children.map((c) => transformListChild(c, resolver, unresolved)),
      };
    case "list_item":
      return {
        type: "list_item",
        children: b.children.map((c) => transformBlock(c, resolver, unresolved)),
      };
    case "task_item":
      return {
        type: "task_item",
        checked: b.checked,
        children: b.children.map((c) => transformBlock(c, resolver, unresolved)),
      };
    case "table":
      return transformTable(b, resolver, unresolved);
    case "table_row":
      return transformTableRow(b, resolver, unresolved);
    case "table_cell":
      return transformTableCell(b, resolver, unresolved);
    case "code_block":
      return passthroughCodeBlock(b);
    case "thematic_break":
      return passthroughThematicBreak(b);
    case "raw_block":
      return passthroughRawBlock(b);
    default: {
      const _exhaustive: never = b;
      void _exhaustive;
      return b;
    }
  }
}

/**
 * Walk an `InlineNode[]`, flat-mapping each one through the
 * single-node transform.  Flat-map is needed because the "strip"
 * policy turns one `LinkNode` into N children.
 */
function transformInlines(
  inlines: readonly InlineNode[],
  resolver: SlugResolver,
  unresolved: UnresolvedPolicy,
): InlineNode[] {
  const out: InlineNode[] = [];
  for (let i = 0; i < inlines.length; i++) {
    const result = transformInline(inlines[i]!, resolver, unresolved);
    if (Array.isArray(result)) {
      for (let j = 0; j < result.length; j++) out.push(result[j]!);
    } else {
      out.push(result);
    }
  }
  return out;
}

/**
 * Per-inline transform.  Returns one `InlineNode` for most
 * cases; returns `InlineNode[]` when an internal link is
 * stripped (the link's children replace the wrapper).
 */
function transformInline(
  n: InlineNode,
  resolver: SlugResolver,
  unresolved: UnresolvedPolicy,
): InlineNode | InlineNode[] {
  switch (n.type) {
    case "link":
      return transformLink(n, resolver, unresolved);
    case "emphasis":
      return {
        type: "emphasis",
        children: transformInlines(n.children, resolver, unresolved),
      };
    case "strong":
      return {
        type: "strong",
        children: transformInlines(n.children, resolver, unresolved),
      };
    case "strikethrough":
      return {
        type: "strikethrough",
        children: transformInlines(n.children, resolver, unresolved),
      };
    case "text":
      return { type: "text", value: n.value };
    case "code_span":
      return passthroughCodeSpan(n);
    case "image":
      return passthroughImage(n);
    case "autolink":
      return { type: "autolink", destination: n.destination, isEmail: n.isEmail };
    case "raw_inline":
      return passthroughRawInline(n);
    case "hard_break":
      return { type: "hard_break" };
    case "soft_break":
      return { type: "soft_break" };
    default: {
      const _exhaustive: never = n;
      void _exhaustive;
      return n;
    }
  }
}

function transformLink(
  link: LinkNode,
  resolver: SlugResolver,
  unresolved: UnresolvedPolicy,
): InlineNode | InlineNode[] {
  // Always recursively rewrite the link's children — they might
  // contain nested formatting (no nested links, but emphasis /
  // strong / text are legal).
  const children = transformInlines(link.children, resolver, unresolved);

  if (!isInternalSlug(link.destination)) {
    // External link → pass-through with children rewritten in
    // case they contained internal content (unlikely for links,
    // but defensive).
    return {
      type: "link",
      destination: link.destination,
      title: link.title,
      children,
    };
  }

  // Internal link → resolve.
  const resolved = resolver(link.destination);
  if (resolved !== null && resolved !== undefined) {
    assertResolvedUrl(resolved);
    return {
      type: "link",
      destination: resolved,
      title: link.title,
      children,
    };
  }

  // Unresolved.
  if (unresolved === "throw") {
    throw new Error(
      `forme-transform-internal-links: unresolved internal slug ${
        JSON.stringify(link.destination)
      }`,
    );
  }
  if (unresolved === "strip") {
    // Return the children flat; the wrapper disappears.
    return children;
  }
  // "keep" — preserve the original destination.
  return {
    type: "link",
    destination: link.destination,
    title: link.title,
    children,
  };
}

function transformListChild(
  c: ListChildNode,
  resolver: SlugResolver,
  unresolved: UnresolvedPolicy,
): ListChildNode {
  if (c.type === "task_item") {
    return {
      type: "task_item",
      checked: c.checked,
      children: c.children.map((b) => transformBlock(b, resolver, unresolved)),
    };
  }
  return {
    type: "list_item",
    children: c.children.map((b) => transformBlock(b, resolver, unresolved)),
  };
}

function transformTable(
  t: TableNode,
  resolver: SlugResolver,
  unresolved: UnresolvedPolicy,
): TableNode {
  return {
    type: "table",
    align: t.align,
    children: t.children.map((r) => transformTableRow(r, resolver, unresolved)),
  };
}

function transformTableRow(
  r: TableRowNode,
  resolver: SlugResolver,
  unresolved: UnresolvedPolicy,
): TableRowNode {
  return {
    type: "table_row",
    isHeader: r.isHeader,
    children: r.children.map((c) => transformTableCell(c, resolver, unresolved)),
  };
}

function transformTableCell(
  c: TableCellNode,
  resolver: SlugResolver,
  unresolved: UnresolvedPolicy,
): TableCellNode {
  return {
    type: "table_cell",
    children: transformInlines(c.children, resolver, unresolved),
  };
}

// ─── Pass-through node copies ──────────────────────────────────────

function passthroughCodeBlock(b: CodeBlockNode): CodeBlockNode {
  return { type: "code_block", language: b.language, value: b.value };
}

function passthroughThematicBreak(_b: ThematicBreakNode): ThematicBreakNode {
  return { type: "thematic_break" };
}

function passthroughRawBlock(b: RawBlockNode): RawBlockNode {
  return { type: "raw_block", format: b.format, value: b.value };
}

function passthroughImage(n: ImageNode): ImageNode {
  return { type: "image", destination: n.destination, title: n.title, alt: n.alt };
}

function passthroughCodeSpan(n: CodeSpanNode): CodeSpanNode {
  return { type: "code_span", value: n.value };
}

function passthroughRawInline(n: RawInlineNode): RawInlineNode {
  return { type: "raw_inline", format: n.format, value: n.value };
}
