/**
 * title.test.ts — three-step title fallback unit tests.
 */

import { describe, it, expect } from "vitest";
import { deriveTitle } from "../src/title.js";
import type { ContentNode } from "@coding-adventures/forme-types";

function makeNode(opts: {
  title?: string;
  documentChildren?: readonly unknown[];
}): ContentNode {
  const fm: Record<string, string> = {};
  if (opts.title !== undefined) fm.title = opts.title;
  return {
    identity: "00000000-0000-7000-8000-000000000001" as ContentNode["identity"],
    revision: ("blake2b:" + "0".repeat(64)) as ContentNode["revision"],
    document: { type: "document", children: opts.documentChildren ?? [] } as unknown as ContentNode["document"],
    frontmatter: fm,
    route: null,
    assetRefs: [],
    sourcePath: "posts/p.md",
  };
}

describe("deriveTitle", () => {
  it("prefers frontmatter.title when present", () => {
    const node = makeNode({
      title: "Hello from frontmatter",
      documentChildren: [
        { type: "heading", level: 1, children: [{ type: "text", value: "h1 title" }] },
      ],
    });
    expect(deriveTitle(node, "fallback-slug")).toBe("Hello from frontmatter");
  });

  it("falls back to the first H1 text when no frontmatter title", () => {
    const node = makeNode({
      documentChildren: [
        { type: "heading", level: 1, children: [{ type: "text", value: "Heading title" }] },
        { type: "paragraph", children: [{ type: "text", value: "body" }] },
      ],
    });
    expect(deriveTitle(node, "fallback-slug")).toBe("Heading title");
  });

  it("flattens inline children of the H1 (emphasis, strong)", () => {
    const node = makeNode({
      documentChildren: [
        {
          type: "heading", level: 1, children: [
            { type: "text", value: "Hello " },
            { type: "emphasis", children: [{ type: "text", value: "world" }] },
          ],
        },
      ],
    });
    expect(deriveTitle(node, "x")).toBe("Hello world");
  });

  it("turns soft/hard breaks into spaces", () => {
    const node = makeNode({
      documentChildren: [
        {
          type: "heading", level: 1, children: [
            { type: "text", value: "first" },
            { type: "soft_break" },
            { type: "text", value: "line" },
          ],
        },
      ],
    });
    expect(deriveTitle(node, "x")).toBe("first line");
  });

  it("preserves inline `code` value", () => {
    const node = makeNode({
      documentChildren: [
        {
          type: "heading", level: 1, children: [
            { type: "text", value: "About " },
            { type: "code", value: "useEffect" },
          ],
        },
      ],
    });
    expect(deriveTitle(node, "x")).toBe("About useEffect");
  });

  it("falls back to the slug when no h1 exists", () => {
    const node = makeNode({
      documentChildren: [
        { type: "heading", level: 2, children: [{ type: "text", value: "h2 only" }] },
        { type: "paragraph", children: [{ type: "text", value: "body" }] },
      ],
    });
    expect(deriveTitle(node, "post-slug")).toBe("post-slug");
  });

  it("falls back to the slug when the document is empty", () => {
    const node = makeNode({ documentChildren: [] });
    expect(deriveTitle(node, "post-slug")).toBe("post-slug");
  });

  it("falls back to the slug when an empty-text h1 is found", () => {
    const node = makeNode({
      documentChildren: [
        { type: "heading", level: 1, children: [{ type: "text", value: "" }] },
      ],
    });
    expect(deriveTitle(node, "fallback")).toBe("fallback");
  });

  it("ignores frontmatter.title that's an empty string", () => {
    const node = makeNode({
      title: "",
      documentChildren: [
        { type: "heading", level: 1, children: [{ type: "text", value: "From h1" }] },
      ],
    });
    expect(deriveTitle(node, "x")).toBe("From h1");
  });
});
