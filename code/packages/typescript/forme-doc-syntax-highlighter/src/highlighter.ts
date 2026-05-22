/**
 * highlighter.ts — DocumentNode → DocumentNode with `highlighted` spans
 * attached to every code block.
 *
 * =============================================================================
 * WHY A STUB IN v0?
 * =============================================================================
 *
 * A real TextMate-style syntax highlighter — the kind VS Code, Atom,
 * and Sublime ship — is THOUSANDS of lines of code that wouldn't get
 * meaningfully reviewed inside a single PR cycle.  It needs:
 *
 *   - A non-trivial grammar interpreter (TextMate uses Oniguruma
 *     regex with begin/end/while/captures/patterns; the spec runs
 *     to dozens of pages).
 *   - A grammar bundle per language (TypeScript's is ~3000 lines
 *     of JSON).
 *   - A theme system (TextMate themes map scope selectors to
 *     colours via a `tmTheme` plist file).
 *   - A scope-stack tokeniser that handles nested constructs
 *     correctly (a comment inside a string inside a template
 *     literal).
 *
 * That's a ~v1 effort.  What v0 ships is the TYPE-LEVEL CONTRACT
 * downstream consumers (HTML renderer, page-shell) can write against
 * RIGHT NOW, so v1 can swap in the real engine without rippling
 * type changes across the rest of the pipeline.
 *
 * =============================================================================
 * THE STUB's BEHAVIOUR
 * =============================================================================
 *
 * For every `CodeBlockNode` in the input document:
 *
 *   1. Recurse into blockquote / list / list_item / task_item
 *      containers (same as `forme-doc-code-block-decorator`).
 *   2. Replace the code block with a `HighlightedCodeBlockNode`
 *      that:
 *        - Preserves every existing field (object spread, so a
 *          `DecoratedCodeBlockNode` stays decorated).
 *        - Adds `highlighted: [{ type: "plain", value: block.value }]`.
 *
 * Edge case: an empty `value` (`""`) gets `highlighted: []` rather
 * than a span with an empty value.  Spans must always have non-empty
 * `value` to satisfy the v1 tiling invariant cleanly.
 *
 * Non-code, non-container blocks pass through by reference.
 *
 * @module highlighter
 */

import type {
  DocumentNode,
  BlockNode,
  CodeBlockNode,
  BlockquoteNode,
  ListNode,
  ListItemNode,
  TaskItemNode,
  ListChildNode,
} from "@coding-adventures/document-ast";

import type {
  HighlightedCodeBlockNode,
  HighlightSpan,
  HighlightOptions,
} from "./types.js";

// ─────────────────────────────────────────────────────────────────────
// Public entry
// ─────────────────────────────────────────────────────────────────────

/**
 * Walk a `DocumentNode` and attach a `highlighted` span sequence to
 * every code block.
 *
 * v0 stub: emits a single `plain` span per block (or an empty array
 * for empty blocks).  v1 will swap in a real TextMate-grammar engine
 * without changing this signature.
 *
 * @param doc - The input DocumentNode.
 * @param options - Reserved for v1; v0 accepts but ignores.
 * @returns A new DocumentNode with every `CodeBlockNode` replaced by
 *          a `HighlightedCodeBlockNode`.
 */
export function highlightCodeBlocks(
  doc: DocumentNode,
  options: HighlightOptions = {},
): DocumentNode {
  // Touch `options` so future evolutions can lint for unused params
  // without a dead-code warning blocking this call.  v0 has no
  // option-driven behaviour.
  void options;
  return {
    type: "document",
    children: doc.children.map((child) => walkBlock(child)),
  };
}

/**
 * Stand-alone helper for callers who already have the raw code
 * string and just want the span array (no AST involved).  Useful
 * for unit tests, CLI tools, or other transforms that want to embed
 * v0's stub behaviour without wrapping/unwrapping nodes.
 *
 * @param value - The raw code text.
 * @param language - Reserved for v1; v0 ignores.
 * @returns A single-element span array `[{ type: "plain", value }]`,
 *          or `[]` if `value` is empty.
 */
export function highlight(value: string, language: string | null = null): HighlightSpan[] {
  void language;
  if (value.length === 0) return [];
  return [{ type: "plain", value }];
}

// ─────────────────────────────────────────────────────────────────────
// Recursive walk — mirrors forme-doc-code-block-decorator's structure
// so the pipeline stays consistent.
// ─────────────────────────────────────────────────────────────────────

function walkBlock(block: BlockNode): BlockNode {
  switch (block.type) {
    case "code_block":
      return decorate(block);
    case "blockquote":
      return walkBlockquote(block);
    case "list":
      return walkList(block);
    case "list_item":
      return walkListItem(block);
    case "task_item":
      return walkTaskItem(block);
    default:
      return block;
  }
}

function walkBlockquote(node: BlockquoteNode): BlockquoteNode {
  return { type: "blockquote", children: node.children.map(walkBlock) };
}

function walkList(node: ListNode): ListNode {
  return {
    type: "list",
    ordered: node.ordered,
    start: node.start,
    tight: node.tight,
    children: node.children.map(walkListChild),
  };
}

function walkListChild(child: ListChildNode): ListChildNode {
  return child.type === "task_item" ? walkTaskItem(child) : walkListItem(child);
}

function walkListItem(node: ListItemNode): ListItemNode {
  return { type: "list_item", children: node.children.map(walkBlock) };
}

function walkTaskItem(node: TaskItemNode): TaskItemNode {
  return {
    type: "task_item",
    checked: node.checked,
    children: node.children.map(walkBlock),
  };
}

// ─────────────────────────────────────────────────────────────────────
// The stub's core
// ─────────────────────────────────────────────────────────────────────

/**
 * Build a `HighlightedCodeBlockNode` from any `CodeBlockNode`
 * (including subtypes like `DecoratedCodeBlockNode` — the spread
 * preserves their added fields).
 */
function decorate(block: CodeBlockNode): HighlightedCodeBlockNode {
  return {
    // Preserve every field on the input node (incl. decorator fields
    // like `copyable`, `languageLabel`, `filename`, `lineNumbers`).
    ...block,
    // Add (or overwrite) the highlighted span sequence.
    highlighted: highlight(block.value, block.language),
  };
}
