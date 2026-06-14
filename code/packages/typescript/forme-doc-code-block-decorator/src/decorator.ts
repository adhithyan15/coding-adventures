/**
 * decorator.ts — DocumentNode → DocumentNode with annotated CodeBlockNodes.
 *
 * =============================================================================
 * WHY A NEW TREE, NOT MUTATION?
 * =============================================================================
 *
 * Same reasoning as `forme-doc-heading-anchors`: every node in
 * `@coding-adventures/document-ast` is `readonly`, callers may share
 * AST references across multiple transforms in a pipeline
 * (frontmatter → headings → TOC → code-decorate → syntax-highlight
 * → HTML), and any transform mutating in place breaks that contract.
 *
 * So we build a NEW `DocumentNode` whose children are:
 *   - The same reference as the input for blocks that don't need
 *     touching (paragraphs, headings, thematic breaks, tables,
 *     blockquotes/lists with no code blocks anywhere inside).
 *   - A freshly-allocated `DecoratedCodeBlockNode` for each code
 *     block.
 *   - A new container with rewritten children for any
 *     blockquote/list/list-item/task-item that transitively
 *     contains a code block.
 *
 * Memory cost: O(code blocks + ancestors-of-code-blocks).  Bounded
 * and tiny in practice — code blocks are leaves; the ancestor
 * rewrite chain is at most O(nesting depth).
 *
 * =============================================================================
 * WALK STRATEGY
 * =============================================================================
 *
 * Recursive top-down.  At each container we ask: "do any of my
 * descendants contain a code block we need to rewrite?"  If yes,
 * allocate a new container with rewritten children.  If no, return
 * the original by reference.
 *
 * The naive way is to *always* allocate; we'd lose the
 * pass-through-by-reference optimisation but the code is simpler.
 * For v0 we go simple: always allocate new containers when we
 * recurse.  Sharing happens at the leaf level (every non-code
 * leaf is shared by reference).  Profiling can revisit if it
 * matters; for typical doc sizes it never will.
 *
 * @module decorator
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

import { extractFilenameHint } from "./filename.js";
import { languageLabel } from "./language-labels.js";
import type { DecoratedCodeBlockNode, DecorateOptions } from "./types.js";

// ─────────────────────────────────────────────────────────────────────
// Public entry
// ─────────────────────────────────────────────────────────────────────

/**
 * Decorate every fenced code block in `doc` with presentation
 * metadata (copy-button hook, language label, filename badge,
 * line-numbers flag).
 *
 * @param doc - The input DocumentNode.
 * @param options - `{ lineNumbers?: boolean }`.  Default `lineNumbers: false`.
 * @returns A new DocumentNode with all `CodeBlockNode`s replaced by
 *          `DecoratedCodeBlockNode`s.  Non-code descendants pass by
 *          reference at the leaf level; containers on the path to a
 *          code block are freshly allocated.
 */
export function decorateCodeBlocks(
  doc: DocumentNode,
  options: DecorateOptions = {},
): DocumentNode {
  const lineNumbers = options.lineNumbers === true;
  return {
    type: "document",
    children: doc.children.map((child) => walkBlock(child, lineNumbers)),
  };
}

// ─────────────────────────────────────────────────────────────────────
// Recursive walk
// ─────────────────────────────────────────────────────────────────────

/**
 * Dispatch a block to the right rewriter based on `type`.
 * Containers (blockquote, list, list_item, task_item) recurse;
 * code blocks decorate; everything else passes through by reference.
 */
function walkBlock(block: BlockNode, lineNumbers: boolean): BlockNode {
  switch (block.type) {
    case "code_block":
      return decorate(block, lineNumbers);
    case "blockquote":
      return walkBlockquote(block, lineNumbers);
    case "list":
      return walkList(block, lineNumbers);
    case "list_item":
      return walkListItem(block, lineNumbers);
    case "task_item":
      return walkTaskItem(block, lineNumbers);
    default:
      // Headings, paragraphs, thematic breaks, raw blocks, tables,
      // table rows/cells, nested document nodes — none contain code
      // blocks in document-ast v0's type system.  Pass through.
      return block;
  }
}

function walkBlockquote(node: BlockquoteNode, lineNumbers: boolean): BlockquoteNode {
  return {
    type: "blockquote",
    children: node.children.map((c) => walkBlock(c, lineNumbers)),
  };
}

function walkList(node: ListNode, lineNumbers: boolean): ListNode {
  return {
    type: "list",
    ordered: node.ordered,
    start: node.start,
    tight: node.tight,
    children: node.children.map((c) => walkListChild(c, lineNumbers)),
  };
}

function walkListChild(child: ListChildNode, lineNumbers: boolean): ListChildNode {
  return child.type === "task_item"
    ? walkTaskItem(child, lineNumbers)
    : walkListItem(child, lineNumbers);
}

function walkListItem(node: ListItemNode, lineNumbers: boolean): ListItemNode {
  return {
    type: "list_item",
    children: node.children.map((c) => walkBlock(c, lineNumbers)),
  };
}

function walkTaskItem(node: TaskItemNode, lineNumbers: boolean): TaskItemNode {
  return {
    type: "task_item",
    checked: node.checked,
    children: node.children.map((c) => walkBlock(c, lineNumbers)),
  };
}

// ─────────────────────────────────────────────────────────────────────
// The decoration itself
// ─────────────────────────────────────────────────────────────────────

/**
 * Build a `DecoratedCodeBlockNode` from a plain `CodeBlockNode`.
 *
 * Order of operations:
 *   1. Extract a filename hint from the first line of `value`.
 *      If found, `value` is stripped to remove that line; otherwise
 *      `value` is untouched.
 *   2. Compute the display label from `language`.
 *   3. Combine into the decorated node, preserving `type`/`language`
 *      and using the (possibly stripped) `value`.
 */
function decorate(block: CodeBlockNode, lineNumbers: boolean): DecoratedCodeBlockNode {
  const { filename, strippedValue } = extractFilenameHint(block.value);
  return {
    type: "code_block",
    language: block.language,
    value: strippedValue,
    copyable: true,
    languageLabel: languageLabel(block.language),
    filename,
    lineNumbers,
  };
}
