/**
 * Immutable top-level block operations for the default editor.
 *
 * The authoring core accepts complete document replacements. These helpers
 * make that large-grained persistence contract pleasant to use without ever
 * exposing mutable AST references to the UI.
 */

import type { BlockNode, DocumentNode } from "@coding-adventures/document-ast";

export type DefaultBlockKind =
  | "paragraph"
  | "heading"
  | "list"
  | "image"
  | "code_block"
  | "blockquote"
  | "table"
  | "link";

export class BlockEditError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "BlockEditError";
  }
}

function deepFreeze<T>(value: T): T {
  if (value !== null && typeof value === "object" && !Object.isFrozen(value)) {
    for (const child of Object.values(value)) deepFreeze(child);
    Object.freeze(value);
  }
  return value;
}

export function createDefaultBlock(kind: DefaultBlockKind): BlockNode {
  switch (kind) {
    case "paragraph":
      return deepFreeze({ type: "paragraph", children: [{ type: "text", value: "" }] });
    case "heading":
      return deepFreeze({ type: "heading", level: 2, children: [{ type: "text", value: "" }] });
    case "list":
      return deepFreeze({
        type: "list",
        ordered: false,
        start: null,
        tight: true,
        children: [{
          type: "list_item",
          children: [{ type: "paragraph", children: [{ type: "text", value: "" }] }],
        }],
      });
    case "image":
      return deepFreeze({
        type: "paragraph",
        children: [{ type: "image", destination: "/", title: null, alt: "" }],
      });
    case "code_block":
      return deepFreeze({ type: "code_block", language: null, value: "\n" });
    case "blockquote":
      return deepFreeze({
        type: "blockquote",
        children: [{ type: "paragraph", children: [{ type: "text", value: "" }] }],
      });
    case "table":
      return deepFreeze({
        type: "table",
        align: [null],
        children: [
          { type: "table_row", isHeader: true, children: [{ type: "table_cell", children: [{ type: "text", value: "" }] }] },
          { type: "table_row", isHeader: false, children: [{ type: "table_cell", children: [{ type: "text", value: "" }] }] },
        ],
      });
    case "link":
      return deepFreeze({
        type: "paragraph",
        children: [{ type: "link", destination: "/", title: null, children: [{ type: "text", value: "" }] }],
      });
  }
}

function insertionIndex(index: number, length: number): void {
  if (!Number.isSafeInteger(index) || index < 0 || index > length) {
    throw new BlockEditError("Block insertion index is outside the document.");
  }
}

function existingIndex(index: number, length: number): void {
  if (!Number.isSafeInteger(index) || index < 0 || index >= length) {
    throw new BlockEditError("Block index is outside the document.");
  }
}

function documentWith(children: readonly BlockNode[]): DocumentNode {
  return deepFreeze({ type: "document", children: [...children] });
}

export function insertBlock(body: DocumentNode, index: number, block: BlockNode): DocumentNode {
  insertionIndex(index, body.children.length);
  return documentWith([
    ...body.children.slice(0, index),
    block,
    ...body.children.slice(index),
  ]);
}

export function replaceBlock(body: DocumentNode, index: number, block: BlockNode): DocumentNode {
  existingIndex(index, body.children.length);
  return documentWith(body.children.map((item, itemIndex) => itemIndex === index ? block : item));
}

export function removeBlock(body: DocumentNode, index: number): DocumentNode {
  existingIndex(index, body.children.length);
  return documentWith(body.children.filter((_item, itemIndex) => itemIndex !== index));
}

export function moveBlock(body: DocumentNode, from: number, to: number): DocumentNode {
  existingIndex(from, body.children.length);
  existingIndex(to, body.children.length);
  if (from === to) return documentWith(body.children);
  const children = [...body.children];
  const [item] = children.splice(from, 1);
  children.splice(to, 0, item!);
  return documentWith(children);
}
