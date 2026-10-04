import { describe, expect, it } from "vitest";
import { validateAuthoringProject } from "@coding-adventures/forme-authoring-core";
import type { AuthoringProject } from "@coding-adventures/forme-authoring-core";

import {
  BlockEditError,
  createDefaultBlock,
  insertBlock,
  moveBlock,
  removeBlock,
  replaceBlock,
} from "../src/blocks.js";

const projectBase: Omit<AuthoringProject, "documents" | "activeDocumentId"> = {
  schemaVersion: 1,
  projectId: "018f47a0-9b6c-7def-9234-56789abcdef0",
  title: "Demo",
  site: { baseUrl: null, themeId: "forme-classless" },
};

describe("immutable default block operations", () => {
  it.each([
    "paragraph", "heading", "list", "image", "code_block", "blockquote", "table", "link",
  ] as const)("creates a core-valid %s block", (kind) => {
    const body = insertBlock({ type: "document", children: [] }, 0, createDefaultBlock(kind));
    const result = validateAuthoringProject({
      ...projectBase,
      documents: [{
        id: "018f47a0-9b6c-7def-9234-56789abcdef1",
        slug: "demo",
        title: "Demo",
        status: "draft",
        body,
      }],
      activeDocumentId: "018f47a0-9b6c-7def-9234-56789abcdef1",
    });
    expect(result.documents[0]!.body.children).toHaveLength(1);
  });

  it("inserts, replaces, moves, and removes without mutating the input", () => {
    const initial = { type: "document" as const, children: [
      createDefaultBlock("paragraph"),
      createDefaultBlock("heading"),
    ] };
    const inserted = insertBlock(initial, 1, createDefaultBlock("code_block"));
    const replaced = replaceBlock(inserted, 0, createDefaultBlock("blockquote"));
    const moved = moveBlock(replaced, 2, 0);
    const removed = removeBlock(moved, 1);

    expect(initial.children.map((block) => block.type)).toEqual(["paragraph", "heading"]);
    expect(inserted.children.map((block) => block.type)).toEqual(["paragraph", "code_block", "heading"]);
    expect(replaced.children.map((block) => block.type)).toEqual(["blockquote", "code_block", "heading"]);
    expect(moved.children.map((block) => block.type)).toEqual(["heading", "blockquote", "code_block"]);
    expect(removed.children.map((block) => block.type)).toEqual(["heading", "code_block"]);
    expect(Object.isFrozen(removed)).toBe(true);
    expect(Object.isFrozen(removed.children)).toBe(true);
  });

  it.each([
    () => insertBlock({ type: "document", children: [] }, 1, createDefaultBlock("paragraph")),
    () => replaceBlock({ type: "document", children: [] }, 0, createDefaultBlock("paragraph")),
    () => removeBlock({ type: "document", children: [] }, 0),
    () => moveBlock({ type: "document", children: [createDefaultBlock("paragraph")] }, 0, 1),
  ])("rejects an out-of-range structural edit", (operation) => {
    expect(operation).toThrow(BlockEditError);
  });
});
