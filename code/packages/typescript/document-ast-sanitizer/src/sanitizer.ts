/**
 * Document AST Sanitizer — Core Transform
 *
 * Performs a policy-driven tree transformation of a DocumentNode, producing
 * a new sanitized DocumentNode. The transform is:
 *
 *   PURE       — the input document is never mutated; a fresh tree is returned
 *   COMPLETE   — every node type is handled explicitly (no silent pass-through)
 *   IMMUTABLE  — callers can safely pass the same document through multiple
 *                sanitizers with different policies
 *
 * The transformation logic is specified in TE02 §Transformation Rules as a
 * truth table. This implementation follows that table exactly, with one entry
 * per node type per condition.
 *
 * === Architecture ===
 *
 * The sanitizer is a recursive descent. Each `sanitize*` function takes a node
 * and the resolved policy, and returns either:
 *
 *   - A single node (the node is kept, possibly transformed)
 *   - null (the node is dropped)
 *   - An array of nodes (link children promoted after dropLinks: true)
 *
 * Container nodes collect their sanitized children and, if empty after
 * sanitization, are themselves dropped (except DocumentNode, which is never
 * dropped — an empty document is a valid document).
 *
 * === Link Promotion ===
 *
 * When dropLinks: true, a LinkNode is not dropped — its children are promoted
 * to the parent. For example:
 *
 *   Before: ParagraphNode { children: [
 *     TextNode("See "),
 *     LinkNode { children: [TextNode("click here")] },
 *     TextNode(" for more")
 *   ]}
 *
 *   After:  ParagraphNode { children: [
 *     TextNode("See "),
 *     TextNode("click here"),
 *     TextNode(" for more")
 *   ]}
 *
 * This is implemented by having sanitizeInline return InlineNode | InlineNode[]
 * and flattening the array at each container level.
 *
 * Spec: TE02 — Document Sanitization §Transformation Rules
 *
 * @module sanitizer
 */

import type {
  DocumentNode,
  BlockNode,
  InlineNode,
  HeadingNode,
  ParagraphNode,
  CodeBlockNode,
  BlockquoteNode,
  ListNode,
  ListItemNode,
  TaskItemNode,
  ThematicBreakNode,
  RawBlockNode,
  TableNode,
  TableRowNode,
  TableCellNode,
  TextNode,
  EmphasisNode,
  StrongNode,
  StrikethroughNode,
  CodeSpanNode,
  LinkNode,
  ImageNode,
  AutolinkNode,
  RawInlineNode,
  HardBreakNode,
  SoftBreakNode,
} from "@coding-adventures/document-ast";

import type { SanitizationPolicy } from "./policy.js";
import { isSchemeAllowed } from "./url-utils.js";

// ─── Resolved Policy ──────────────────────────────────────────────────────────
//
// The user-supplied SanitizationPolicy has all optional fields. To avoid
// repeated null-checks throughout the tree walk, we resolve defaults once
// at the top of `sanitize()` into a ResolvedPolicy with all fields present.

interface ResolvedPolicy {
  readonly allowRawBlockFormats: "drop-all" | "passthrough" | readonly string[];
  readonly allowRawInlineFormats: "drop-all" | "passthrough" | readonly string[];
  readonly allowedUrlSchemes: readonly string[] | null;
  readonly dropLinks: boolean;
  readonly dropImages: boolean;
  readonly transformImageToText: boolean;
  readonly maxHeadingLevel: 1 | 2 | 3 | 4 | 5 | 6 | "drop";
  readonly minHeadingLevel: 1 | 2 | 3 | 4 | 5 | 6;
  readonly dropBlockquotes: boolean;
  readonly dropCodeBlocks: boolean;
  readonly transformCodeSpanToText: boolean;
}

/**
 * Apply defaults to a partial SanitizationPolicy, producing a ResolvedPolicy
 * with all fields present.
 *
 * Default values are the PASSTHROUGH behaviour — omitting a field means
 * "keep everything", never "drop everything". This follows the principle of
 * least surprise: explicitly opt in to restrictive behaviour.
 */
function resolvePolicy(policy: SanitizationPolicy): ResolvedPolicy {
  return {
    allowRawBlockFormats: policy.allowRawBlockFormats ?? "passthrough",
    allowRawInlineFormats: policy.allowRawInlineFormats ?? "passthrough",
    // Note: null is a valid explicit value (allow all schemes) and must be
    // preserved. The `??` operator handles undefined-only, not null, which
    // is exactly what we want here.
    allowedUrlSchemes: policy.allowedUrlSchemes !== undefined
      ? policy.allowedUrlSchemes
      : ["http", "https", "mailto", "ftp"],
    dropLinks: policy.dropLinks ?? false,
    dropImages: policy.dropImages ?? false,
    transformImageToText: policy.transformImageToText ?? false,
    maxHeadingLevel: policy.maxHeadingLevel ?? 6,
    minHeadingLevel: policy.minHeadingLevel ?? 1,
    dropBlockquotes: policy.dropBlockquotes ?? false,
    dropCodeBlocks: policy.dropCodeBlocks ?? false,
    transformCodeSpanToText: policy.transformCodeSpanToText ?? false,
  };
}

// ─── Public API ───────────────────────────────────────────────────────────────

/**
 * Sanitize a DocumentNode by applying a SanitizationPolicy.
 *
 * Returns a new DocumentNode with all policy violations removed or neutralised.
 * The input is never mutated.
 *
 * The result is always a valid DocumentNode. If every block node is dropped by
 * the policy, the result is { type: "document", children: [] } — an empty
 * document is valid.
 *
 * @param document  The document to sanitize.
 * @param policy    The sanitization policy to apply.
 * @returns         A new, sanitized DocumentNode.
 *
 * @example
 * // User-generated content — strict policy
 * const safe = sanitize(parse(userMarkdown), STRICT);
 * const html = toHtml(safe);
 *
 * // Documentation — pass through everything
 * const doc = sanitize(parse(trustedMarkdown), PASSTHROUGH);
 *
 * // Custom policy — allow HTML blocks but restrict headings
 * const doc = sanitize(parse(editorMarkdown), {
 *   ...RELAXED,
 *   minHeadingLevel: 2,
 *   allowedUrlSchemes: ["http", "https"],
 * });
 */
export function sanitize(document: DocumentNode, policy: SanitizationPolicy): DocumentNode {
  const resolved = resolvePolicy(policy);
  const sanitizedChildren = sanitizeBlocks(document.children, resolved);
  return { type: "document", children: sanitizedChildren };
}

// ─── Block Sanitization ───────────────────────────────────────────────────────

/**
 * Sanitize an array of block nodes, returning a new array with dropped nodes
 * removed. Never returns null — the document root needs an array (possibly empty).
 */
function sanitizeBlocks(blocks: readonly BlockNode[], policy: ResolvedPolicy): readonly BlockNode[] {
  const result: BlockNode[] = [];
  for (const block of blocks) {
    const sanitized = sanitizeBlock(block, policy);
    if (sanitized !== null) {
      result.push(sanitized);
    }
  }
  return result;
}

/**
 * Sanitize a single block node.
 *
 * Returns the sanitized node, or null if the node is dropped.
 *
 * Truth table (from spec):
 *
 *   DocumentNode       → recurse into children (handled by sanitize() directly)
 *   HeadingNode        → drop / clamp level / recurse (see sanitizeHeading)
 *   ParagraphNode      → recurse; drop if empty after sanitization
 *   CodeBlockNode      → drop if dropCodeBlocks, else keep as-is
 *   BlockquoteNode     → drop if dropBlockquotes, else recurse; drop if empty
 *   ListNode           → recurse into children
 *   ListItemNode       → recurse into children
 *   ThematicBreakNode  → keep as-is (leaf)
 *   RawBlockNode       → drop / keep based on allowRawBlockFormats
 */
function sanitizeBlock(block: BlockNode, policy: ResolvedPolicy): BlockNode | null {
  switch (block.type) {
    case "document":
      // DocumentNode is handled at the top level by sanitize().
      // If encountered as a child (unusual), recurse.
      return { type: "document", children: sanitizeBlocks(block.children, policy) };

    case "heading":
      return sanitizeHeading(block, policy);

    case "paragraph":
      return sanitizeParagraph(block, policy);

    case "code_block":
      return sanitizeCodeBlock(block, policy);

    case "blockquote":
      return sanitizeBlockquote(block, policy);

    case "list":
      return sanitizeList(block, policy);

    case "list_item":
      return sanitizeListItem(block, policy);

    case "task_item":
      return sanitizeTaskItem(block, policy);

    case "thematic_break":
      // Leaf node — no children to sanitize, always kept.
      // A thematic break carries no content that could be dangerous.
      return block;

    case "raw_block":
      return sanitizeRawBlock(block, policy);

    case "table":
      return sanitizeTable(block, policy);

    case "table_row":
      return sanitizeTableRow(block, policy);

    case "table_cell":
      return sanitizeTableCell(block, policy);

    default: {
      // TypeScript's never type ensures this is unreachable if all cases are handled.
      // If a new node type is added to the AST, TypeScript will error here.
      const _never: never = block;
      return null;
    }
  }
}

/**
 * Sanitize a HeadingNode.
 *
 * Truth table:
 *   maxHeadingLevel === "drop"           → drop node
 *   level < minHeadingLevel              → clamp level up to minHeadingLevel
 *   level > maxHeadingLevel              → clamp level down to maxHeadingLevel
 *   otherwise                            → recurse into children
 *
 * "Clamping" means: a heading that is too shallow (h1 when min is h2) becomes
 * the minimum level. A heading that is too deep (h5 when max is h3) becomes
 * the maximum level. In both cases, the children are recursively sanitized.
 *
 * If all children are dropped by inline sanitization, the heading itself is
 * dropped (an empty heading is meaningless).
 */
function sanitizeHeading(node: HeadingNode, policy: ResolvedPolicy): BlockNode | null {
  // Check drop-all first
  if (policy.maxHeadingLevel === "drop") {
    return null;
  }

  // Clamp the level to [minHeadingLevel, maxHeadingLevel]
  let level: 1 | 2 | 3 | 4 | 5 | 6 = node.level;
  if (level < policy.minHeadingLevel) {
    level = policy.minHeadingLevel;
  }
  if (level > (policy.maxHeadingLevel as number)) {
    level = policy.maxHeadingLevel as 1 | 2 | 3 | 4 | 5 | 6;
  }

  // Recurse into children
  const sanitizedChildren = sanitizeInlines(node.children, policy);

  // Drop empty headings (nothing left after sanitizing children)
  if (sanitizedChildren.length === 0) {
    return null;
  }

  return { type: "heading", level, children: sanitizedChildren };
}

/**
 * Sanitize a ParagraphNode.
 *
 * A paragraph always recurses into its children. If all children are dropped
 * by inline sanitization, the paragraph itself is dropped (no empty <p> tags).
 */
function sanitizeParagraph(node: ParagraphNode, policy: ResolvedPolicy): BlockNode | null {
  const sanitizedChildren = sanitizeInlines(node.children, policy);
  if (sanitizedChildren.length === 0) {
    return null;
  }
  return { type: "paragraph", children: sanitizedChildren };
}

/**
 * Sanitize a CodeBlockNode.
 *
 * Truth table:
 *   dropCodeBlocks === true  → drop node
 *   otherwise               → keep as-is (leaf; no children to sanitize)
 *
 * Code blocks are leaf nodes — their `value` is raw text that is not
 * processed for Markdown or inline markup. There is no XSS risk in the
 * value itself (the HTML renderer escapes it). The risk, if any, is in
 * whether to show code blocks at all in a given context.
 */
function sanitizeCodeBlock(node: CodeBlockNode, policy: ResolvedPolicy): BlockNode | null {
  if (policy.dropCodeBlocks) {
    return null;
  }
  // Keep as-is: the node is a leaf and the renderer will escape its value
  return node;
}

/**
 * Sanitize a BlockquoteNode.
 *
 * Truth table:
 *   dropBlockquotes === true  → drop node (children are NOT promoted)
 *   otherwise                → recurse into children; drop if empty
 *
 * Note: unlike link children, blockquote children are NOT promoted when the
 * blockquote is dropped. The entire quote (including its content) disappears.
 * This is the spec's intentional design — there is no semantic equivalent of
 * "blockquote text without the blockquote structure" in most contexts.
 */
function sanitizeBlockquote(node: BlockquoteNode, policy: ResolvedPolicy): BlockNode | null {
  if (policy.dropBlockquotes) {
    return null;
  }
  const sanitizedChildren = sanitizeBlocks(node.children, policy);
  if (sanitizedChildren.length === 0) {
    return null;
  }
  return { type: "blockquote", children: sanitizedChildren };
}

/**
 * Sanitize a ListNode.
 *
 * Lists are always kept (no list-specific drop policy). The list's children
 * (ListItemNodes) are recursively sanitized. If all items are dropped, the
 * list itself is dropped.
 */
function sanitizeList(node: ListNode, policy: ResolvedPolicy): BlockNode | null {
  const sanitizedItems: Array<ListItemNode | TaskItemNode> = [];
  for (const item of node.children) {
    const sanitized = item.type === "task_item"
      ? sanitizeTaskItem(item, policy)
      : sanitizeListItem(item, policy);
    if (sanitized !== null) {
      sanitizedItems.push(sanitized);
    }
  }
  if (sanitizedItems.length === 0) {
    return null;
  }
  return {
    type: "list",
    ordered: node.ordered,
    start: node.start,
    tight: node.tight,
    children: sanitizedItems,
  };
}

/**
 * Sanitize a ListItemNode.
 *
 * List items always recurse. If all children are dropped, the item is dropped.
 */
function sanitizeListItem(node: ListItemNode, policy: ResolvedPolicy): ListItemNode | null {
  const sanitizedChildren = sanitizeBlocks(node.children, policy);
  if (sanitizedChildren.length === 0) {
    return null;
  }
  return { type: "list_item", children: sanitizedChildren };
}

function sanitizeTaskItem(node: TaskItemNode, policy: ResolvedPolicy): TaskItemNode | null {
  const sanitizedChildren = sanitizeBlocks(node.children, policy);
  if (sanitizedChildren.length === 0) {
    return null;
  }
  return { type: "task_item", checked: node.checked, children: sanitizedChildren };
}

/**
 * Sanitize a RawBlockNode.
 *
 * Truth table:
 *   allowRawBlockFormats === "drop-all"    → drop node
 *   allowRawBlockFormats === "passthrough" → keep as-is
 *   allowRawBlockFormats === string[]      → keep if format in list, else drop
 *
 * Raw blocks are the primary XSS vector in Markdown-to-HTML pipelines.
 * A raw block with format "html" will be emitted verbatim by the HTML
 * renderer, bypassing all HTML escaping. The STRICT policy drops all
 * raw blocks to prevent script injection.
 */
function sanitizeRawBlock(node: RawBlockNode, policy: ResolvedPolicy): BlockNode | null {
  const p = policy.allowRawBlockFormats;
  if (p === "drop-all") return null;
  if (p === "passthrough") return node;
  // It's a string[] — check if the format is in the allowlist
  if ((p as readonly string[]).includes(node.format)) return node;
  return null;
}

function sanitizeTable(node: TableNode, policy: ResolvedPolicy): BlockNode | null {
  const children = node.children
    .map((row) => sanitizeTableRow(row, policy))
    .filter((row): row is TableRowNode => row !== null);
  if (children.length === 0) {
    return null;
  }
  return { type: "table", align: [...node.align], children };
}

function sanitizeTableRow(node: TableRowNode, policy: ResolvedPolicy): TableRowNode | null {
  const children = node.children
    .map((cell) => sanitizeTableCell(cell, policy))
    .filter((cell): cell is TableCellNode => cell !== null);
  if (children.length === 0) {
    return null;
  }
  return { type: "table_row", isHeader: node.isHeader, children };
}

function sanitizeTableCell(node: TableCellNode, policy: ResolvedPolicy): TableCellNode | null {
  const children = sanitizeInlines(node.children, policy);
  if (children.length === 0) {
    return null;
  }
  return { type: "table_cell", children };
}

// ─── Inline Sanitization ──────────────────────────────────────────────────────

/**
 * Sanitize an array of inline nodes, flattening any promoted children.
 *
 * The "flatten" step is for link promotion: when dropLinks is true, a
 * LinkNode is replaced by its children (an InlineNode[]). Without flattening,
 * we would end up with nested arrays in the children list.
 *
 *   Input:  [TextNode("a"), LinkNode { children: [TextNode("b")] }, TextNode("c")]
 *   After:  [TextNode("a"), TextNode("b"), TextNode("c")]   (if dropLinks: true)
 */
function sanitizeInlines(
  nodes: readonly InlineNode[],
  policy: ResolvedPolicy,
): readonly InlineNode[] {
  const result: InlineNode[] = [];
  for (const node of nodes) {
    const sanitized = sanitizeInline(node, policy);
    if (sanitized === null) {
      // Node dropped — skip it
    } else if (Array.isArray(sanitized)) {
      // Promoted children (from dropLinks) — flatten into the result
      result.push(...sanitized);
    } else {
      result.push(sanitized);
    }
  }
  return result;
}

/**
 * Sanitize a single inline node.
 *
 * Returns:
 *   - InlineNode    — the node, possibly transformed
 *   - InlineNode[]  — promoted children (for dropLinks: true)
 *   - null          — the node is dropped
 *
 * Truth table (from spec):
 *
 *   TextNode          → keep as-is
 *   EmphasisNode      → recurse; drop if empty
 *   StrongNode        → recurse; drop if empty
 *   CodeSpanNode      → TextNode if transformCodeSpanToText, else keep as-is
 *   LinkNode          → promote children if dropLinks; sanitize URL; recurse
 *   ImageNode         → drop / TextNode / sanitize URL
 *   AutolinkNode      → drop if URL not allowed; else keep
 *   RawInlineNode     → drop/keep based on allowRawInlineFormats
 *   HardBreakNode     → keep as-is
 *   SoftBreakNode     → keep as-is
 */
function sanitizeInline(
  node: InlineNode,
  policy: ResolvedPolicy,
): InlineNode | InlineNode[] | null {
  switch (node.type) {
    case "text":
      return sanitizeText(node);

    case "emphasis":
      return sanitizeEmphasis(node, policy);

    case "strong":
      return sanitizeStrong(node, policy);

    case "strikethrough":
      return sanitizeStrikethrough(node, policy);

    case "code_span":
      return sanitizeCodeSpan(node, policy);

    case "link":
      return sanitizeLink(node, policy);

    case "image":
      return sanitizeImage(node, policy);

    case "autolink":
      return sanitizeAutolink(node, policy);

    case "raw_inline":
      return sanitizeRawInline(node, policy);

    case "hard_break":
      return node;

    case "soft_break":
      return node;

    default: {
      const _never: never = node;
      return null;
    }
  }
}

// ─── Inline Node Sanitizers ───────────────────────────────────────────────────

/**
 * TextNode — always keep as-is.
 *
 * Text nodes contain decoded Unicode strings. There is nothing to sanitize
 * in a text node — the HTML renderer will escape it before output.
 */
function sanitizeText(node: TextNode): InlineNode {
  return node;
}

/**
 * EmphasisNode — recurse into children, drop if empty.
 *
 * Emphasis has no policy options — it is always kept unless empty after
 * sanitizing its children.
 */
function sanitizeEmphasis(node: EmphasisNode, policy: ResolvedPolicy): InlineNode | null {
  const children = sanitizeInlines(node.children, policy);
  if (children.length === 0) return null;
  return { type: "emphasis", children };
}

/**
 * StrongNode — recurse into children, drop if empty.
 */
function sanitizeStrong(node: StrongNode, policy: ResolvedPolicy): InlineNode | null {
  const children = sanitizeInlines(node.children, policy);
  if (children.length === 0) return null;
  return { type: "strong", children };
}

function sanitizeStrikethrough(node: StrikethroughNode, policy: ResolvedPolicy): InlineNode | null {
  const children = sanitizeInlines(node.children, policy);
  if (children.length === 0) return null;
  return { type: "strikethrough", children };
}

/**
 * CodeSpanNode — convert to TextNode or keep.
 *
 * Truth table:
 *   transformCodeSpanToText === true  → TextNode { value: node.value }
 *   otherwise                        → keep as-is
 */
function sanitizeCodeSpan(node: CodeSpanNode, policy: ResolvedPolicy): InlineNode {
  if (policy.transformCodeSpanToText) {
    return { type: "text", value: node.value };
  }
  return node;
}

/**
 * LinkNode — promote children, sanitize URL, or keep.
 *
 * Truth table:
 *   dropLinks === true               → promote children to parent (return array)
 *   URL scheme not allowed           → keep node, set destination=""
 *   otherwise                       → sanitize URL, recurse into children
 *
 * When dropLinks is true, the function returns an InlineNode[] (the link's
 * children, recursively sanitized). The caller (sanitizeInlines) flattens this
 * array into the parent's children list.
 *
 * When the URL scheme is disallowed (e.g. javascript:alert(1)), the link node
 * is kept but its destination is replaced with "" — making the link inert
 * (<a href="">text</a>) rather than removing the link text entirely.
 */
function sanitizeLink(
  node: LinkNode,
  policy: ResolvedPolicy,
): InlineNode | InlineNode[] | null {
  // Case 1: drop links — promote children to parent
  if (policy.dropLinks) {
    const promotedChildren = sanitizeInlines(node.children, policy);
    // Return the promoted children as an array for the caller to flatten.
    // If all children were dropped, return an empty array (will be flattened
    // to nothing in the parent).
    return promotedChildren as InlineNode[];
  }

  // Case 2: URL scheme check
  const allowed = isSchemeAllowed(node.destination, policy.allowedUrlSchemes);
  const destination = allowed ? node.destination : "";

  // Case 3: recurse into children
  const children = sanitizeInlines(node.children, policy);
  if (children.length === 0) {
    return null;
  }

  return { type: "link", destination, title: node.title, children };
}

/**
 * ImageNode — drop, convert to text, sanitize URL, or keep.
 *
 * Truth table (precedence order — dropImages takes priority):
 *   dropImages === true             → drop node entirely
 *   transformImageToText === true   → TextNode { value: node.alt }
 *   URL scheme not allowed          → keep node, set destination=""
 *   otherwise                      → keep as-is
 *
 * Note: when transformImageToText is true and alt is empty (""), we still
 * return a TextNode with value: "" rather than null. The image existed and
 * had no alt text — that's the author's information, and we preserve it
 * (even if it produces an empty text node). An empty TextNode is harmless.
 */
function sanitizeImage(node: ImageNode, policy: ResolvedPolicy): InlineNode | null {
  // dropImages takes precedence over transformImageToText
  if (policy.dropImages) {
    return null;
  }

  if (policy.transformImageToText) {
    return { type: "text", value: node.alt };
  }

  // URL scheme check
  const allowed = isSchemeAllowed(node.destination, policy.allowedUrlSchemes);
  const destination = allowed ? node.destination : "";

  return { type: "image", destination, title: node.title, alt: node.alt };
}

/**
 * AutolinkNode — drop if URL scheme not allowed, else keep.
 *
 * Unlike LinkNode, an AutolinkNode has no children to promote when the URL
 * is dangerous — the only content is the URL itself (displayed as the link
 * text). So when the scheme is not allowed, the entire node is dropped.
 *
 * Truth table:
 *   URL scheme not allowed  → drop node
 *   otherwise              → keep as-is
 */
function sanitizeAutolink(node: AutolinkNode, policy: ResolvedPolicy): InlineNode | null {
  // Email autolinks use "mailto:" scheme implicitly. isEmail=true autolinks
  // have a destination like "user@example.com" without the mailto: prefix.
  // They are relative by scheme detection (no ":" in the address part before
  // "@"), so they always pass the allowlist check, which is correct.
  const allowed = isSchemeAllowed(node.destination, policy.allowedUrlSchemes);
  if (!allowed) {
    return null;
  }
  return node;
}

/**
 * RawInlineNode — drop/keep based on allowRawInlineFormats.
 *
 * Truth table:
 *   allowRawInlineFormats === "drop-all"    → drop node
 *   allowRawInlineFormats === "passthrough" → keep as-is
 *   allowRawInlineFormats === string[]      → keep if format in list, else drop
 */
function sanitizeRawInline(node: RawInlineNode, policy: ResolvedPolicy): InlineNode | null {
  const p = policy.allowRawInlineFormats;
  if (p === "drop-all") return null;
  if (p === "passthrough") return node;
  if ((p as readonly string[]).includes(node.format)) return node;
  return null;
}
