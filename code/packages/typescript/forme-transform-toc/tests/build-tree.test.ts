/**
 * build-tree.test.ts — flat HeadingSlug[] → hierarchical TocNode[].
 */

import { describe, it, expect } from "vitest";
import type { HeadingSlug } from "@coding-adventures/forme-transform-autolink-headings";
import { buildTree } from "../src/index.js";

function s(level: 1|2|3|4|5|6, slug: string): HeadingSlug {
  return { level, text: slug, slug, anchorHref: `#${slug}` };
}

describe("buildTree — well-formed hierarchies", () => {
  it("empty input → empty roots", () => {
    expect(buildTree([])).toEqual([]);
  });

  it("single heading → single root with no children", () => {
    const t = buildTree([s(1, "title")]);
    expect(t).toEqual([{
      level: 1, text: "title", slug: "title", href: "#title", children: [],
    }]);
  });

  it("h1 + h2 → h1 root with one child", () => {
    const t = buildTree([s(1, "h1"), s(2, "h2a")]);
    expect(t.length).toBe(1);
    expect(t[0]!.children.length).toBe(1);
    expect(t[0]!.children[0]!.slug).toBe("h2a");
  });

  it("h1 + two h2 siblings", () => {
    const t = buildTree([s(1, "title"), s(2, "intro"), s(2, "main")]);
    expect(t.length).toBe(1);
    expect(t[0]!.children.length).toBe(2);
    expect(t[0]!.children.map((c) => c.slug)).toEqual(["intro", "main"]);
  });

  it("h1 > h2 > h3 chain (3 levels deep)", () => {
    const t = buildTree([s(1, "a"), s(2, "b"), s(3, "c")]);
    expect(t[0]!.slug).toBe("a");
    expect(t[0]!.children[0]!.slug).toBe("b");
    expect(t[0]!.children[0]!.children[0]!.slug).toBe("c");
  });

  it("realistic blog post: h1 > (h2 > h3, h3), h2 > h3", () => {
    const t = buildTree([
      s(1, "post"),
      s(2, "intro"),
      s(3, "background"),
      s(3, "motivation"),
      s(2, "details"),
      s(3, "step-1"),
    ]);
    expect(t.length).toBe(1);
    const post = t[0]!;
    expect(post.children.map((c) => c.slug)).toEqual(["intro", "details"]);
    expect(post.children[0]!.children.map((c) => c.slug)).toEqual(["background", "motivation"]);
    expect(post.children[1]!.children.map((c) => c.slug)).toEqual(["step-1"]);
  });
});

describe("buildTree — multiple roots", () => {
  it("two h1s → two roots", () => {
    const t = buildTree([s(1, "a"), s(1, "b")]);
    expect(t.length).toBe(2);
    expect(t.map((n) => n.slug)).toEqual(["a", "b"]);
  });

  it("starting at h2 (no h1) → h2 roots", () => {
    const t = buildTree([s(2, "a"), s(2, "b")]);
    expect(t.length).toBe(2);
    expect(t.every((n) => n.level === 2)).toBe(true);
  });

  it("outdent past root: h1 > h2 > h1 → two h1 roots", () => {
    const t = buildTree([s(1, "first"), s(2, "child"), s(1, "second")]);
    expect(t.length).toBe(2);
    expect(t.map((n) => n.slug)).toEqual(["first", "second"]);
    expect(t[0]!.children.length).toBe(1);
    expect(t[1]!.children.length).toBe(0);
  });
});

describe("buildTree — malformed sequences (skipped levels)", () => {
  it("h1 → h3 skip: h3 becomes direct child of h1", () => {
    const t = buildTree([s(1, "a"), s(3, "deep")]);
    expect(t[0]!.children.map((c) => c.slug)).toEqual(["deep"]);
    expect(t[0]!.children[0]!.level).toBe(3); // level preserved, no fake h2 inserted
  });

  it("h1 → h6 (skip 4 levels) just becomes direct child", () => {
    const t = buildTree([s(1, "a"), s(6, "deepest")]);
    expect(t[0]!.children[0]!.slug).toBe("deepest");
    expect(t[0]!.children[0]!.level).toBe(6);
  });

  it("first heading is h3 (no preceding h1/h2) → h3 root", () => {
    const t = buildTree([s(3, "orphan")]);
    expect(t.length).toBe(1);
    expect(t[0]!.level).toBe(3);
  });

  it("h2 > h4 > h3: h4 child of h2; h3 closes h4 and becomes sibling of h4", () => {
    const t = buildTree([s(2, "a"), s(4, "b"), s(3, "c")]);
    // Stack walk:
    //   h2 "a" → root, stack [a]
    //   h4 "b" → child of a, stack [a, b]
    //   h3 "c" → pop b (level 4 >= 3), stack [a]; child of a
    expect(t[0]!.children.map((c) => c.slug)).toEqual(["b", "c"]);
    expect(t[0]!.children[0]!.children).toEqual([]);
    expect(t[0]!.children[1]!.children).toEqual([]);
  });

  it("same-level repeats become siblings, not nested", () => {
    const t = buildTree([s(2, "a"), s(2, "b"), s(2, "c")]);
    expect(t.length).toBe(3);
    expect(t.every((n) => n.level === 2)).toBe(true);
  });
});

describe("buildTree — href preserved from HeadingSlug.anchorHref", () => {
  it("href === slug.anchorHref (not regenerated)", () => {
    const custom: HeadingSlug = { level: 1, text: "T", slug: "my-slug", anchorHref: "#my-slug" };
    const t = buildTree([custom]);
    expect(t[0]!.href).toBe("#my-slug");
  });
});

describe("buildTree — purity", () => {
  it("does not mutate input slugs array", () => {
    const input = [s(1, "a"), s(2, "b"), s(2, "c")];
    const before = JSON.stringify(input);
    buildTree(input);
    expect(JSON.stringify(input)).toBe(before);
  });

  it("does not mutate individual HeadingSlug entries", () => {
    const slug = s(1, "a");
    const before = JSON.stringify(slug);
    buildTree([slug]);
    expect(JSON.stringify(slug)).toBe(before);
  });

  it("same input → byte-identical output (FM03)", () => {
    const input = [s(1, "a"), s(2, "b"), s(3, "c"), s(2, "d")];
    expect(JSON.stringify(buildTree(input))).toBe(JSON.stringify(buildTree(input)));
  });

  it("returns a fresh tree each call (no shared substructure)", () => {
    const input = [s(1, "a"), s(2, "b")];
    const t1 = buildTree(input);
    const t2 = buildTree(input);
    expect(t1).not.toBe(t2);
    expect(t1[0]).not.toBe(t2[0]);
    expect(t1[0]!.children[0]).not.toBe(t2[0]!.children[0]);
  });
});

describe("buildTree — stress / large input", () => {
  it("handles 100 alternating h2/h3", () => {
    const slugs: HeadingSlug[] = [];
    for (let i = 0; i < 50; i++) {
      slugs.push(s(2, `a${i}`), s(3, `b${i}`));
    }
    const t = buildTree(slugs);
    expect(t.length).toBe(50);
    expect(t.every((n) => n.level === 2 && n.children.length === 1)).toBe(true);
  });
});
