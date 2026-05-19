/**
 * build-toc.test.ts — end-to-end: DocumentNode (or slugs) → TocNode[].
 */

import { describe, it, expect } from "vitest";
import type { BlockNode, DocumentNode, InlineNode } from "@coding-adventures/document-ast";
import type { HeadingSlug } from "@coding-adventures/forme-transform-autolink-headings";
import { buildToc, buildTocFromSlugs } from "../src/index.js";

function txt(value: string): InlineNode { return { type: "text", value }; }
function h(level: 1|2|3|4|5|6, text: string): BlockNode {
  return { type: "heading", level, children: [txt(text)] };
}
function doc(...children: BlockNode[]): DocumentNode {
  return { type: "document", children };
}
function s(level: 1|2|3|4|5|6, slug: string): HeadingSlug {
  return { level, text: slug, slug, anchorHref: `#${slug}` };
}

describe("buildToc — DocumentNode entry point", () => {
  it("calls autolinkHeadings internally", () => {
    const t = buildToc(doc(
      h(1, "Title"),
      h(2, "Section A"),
      h(2, "Section B"),
    ));
    expect(t.length).toBe(1);
    expect(t[0]!.slug).toBe("title");
    expect(t[0]!.children.map((c) => c.slug)).toEqual(["section-a", "section-b"]);
  });

  it("uses slug-resolved hrefs with collision suffixes", () => {
    const t = buildToc(doc(
      h(1, "Title"),
      h(2, "Setup"),
      h(2, "Setup"),  // duplicate
    ));
    expect(t[0]!.children.map((c) => c.href)).toEqual(["#setup", "#setup-2"]);
  });

  it("empty doc → empty array", () => {
    expect(buildToc(doc())).toEqual([]);
  });

  it("doc with no headings → empty array", () => {
    const t = buildToc(doc({
      type: "paragraph",
      children: [txt("just prose")],
    }));
    expect(t).toEqual([]);
  });
});

describe("buildToc — options", () => {
  it("minLevel: 2 drops the h1", () => {
    const t = buildToc(doc(
      h(1, "Title"),
      h(2, "Section A"),
      h(3, "Sub one"),
    ), { minLevel: 2 });
    expect(t.length).toBe(1);
    expect(t[0]!.slug).toBe("section-a");
    expect(t[0]!.children[0]!.slug).toBe("sub-one");
  });

  it("maxLevel: 2 drops h3+", () => {
    const t = buildToc(doc(
      h(1, "Title"),
      h(2, "Section"),
      h(3, "Skipped"),
    ), { maxLevel: 2 });
    expect(t[0]!.children.map((c) => c.slug)).toEqual(["section"]);
    expect(t[0]!.children[0]!.children).toEqual([]);
  });

  it("minLevel + maxLevel combo", () => {
    const t = buildToc(doc(
      h(1, "Title"),
      h(2, "Alpha"),
      h(3, "Sub"),
      h(4, "Deep"),
    ), { minLevel: 2, maxLevel: 3 });
    expect(t.length).toBe(1);
    expect(t[0]!.slug).toBe("alpha");
    expect(t[0]!.children[0]!.slug).toBe("sub");
    expect(t[0]!.children[0]!.children).toEqual([]); // h4 filtered out
  });

  it("default options keep everything (minLevel 1, maxLevel 6)", () => {
    const t = buildToc(doc(h(1, "A"), h(6, "Z")));
    expect(t[0]!.children[0]!.slug).toBe("z");
  });
});

describe("buildTocFromSlugs — pre-computed slug entry point", () => {
  it("builds tree from caller-supplied slug array", () => {
    const t = buildTocFromSlugs([
      s(1, "title"),
      s(2, "a"),
      s(2, "b"),
    ]);
    expect(t.length).toBe(1);
    expect(t[0]!.children.length).toBe(2);
  });

  it("same options as buildToc", () => {
    const slugs = [s(1, "title"), s(2, "a"), s(3, "a-1")];
    const t = buildTocFromSlugs(slugs, { minLevel: 2 });
    expect(t.length).toBe(1);
    expect(t[0]!.slug).toBe("a");
  });

  it("does not mutate input slugs array", () => {
    const slugs = [s(1, "a"), s(2, "b")];
    const before = JSON.stringify(slugs);
    buildTocFromSlugs(slugs);
    expect(JSON.stringify(slugs)).toBe(before);
  });

  it("equivalent to buildToc with the same slug stream", () => {
    const d = doc(h(1, "T"), h(2, "A"), h(3, "B"));
    const viaDoc = buildToc(d);
    // Derive slugs the same way buildToc does — text is preserved
    // verbatim (uppercase "T", "A", "B"), slugs are lowercased.
    const slugs: HeadingSlug[] = [
      { level: 1, text: "T", slug: "t", anchorHref: "#t" },
      { level: 2, text: "A", slug: "a", anchorHref: "#a" },
      { level: 3, text: "B", slug: "b", anchorHref: "#b" },
    ];
    const viaSlugs = buildTocFromSlugs(slugs);
    expect(JSON.stringify(viaDoc)).toBe(JSON.stringify(viaSlugs));
  });
});

describe("buildToc — security / hostile heading text", () => {
  it("attacker-controlled heading produces safe slug + href", () => {
    const t = buildToc(doc(h(1, `<script>alert("xss")</script>`)));
    expect(t[0]!.slug).toMatch(/^[a-z0-9-]+$/);
    expect(t[0]!.href).toMatch(/^#[a-z0-9-]+$/);
    // The TEXT preserves the raw heading content (renderers
    // escape it when emitting); only the slug/href are sanitised.
    expect(t[0]!.text).toBe(`<script>alert("xss")</script>`);
  });

  it("control bytes stripped from slug even though text preserves them", () => {
    const t = buildToc(doc(h(1, "hel\x00lo")));
    expect(t[0]!.slug).toBe("hello");
  });
});

describe("buildToc — reproducibility", () => {
  it("same DocumentNode → byte-identical tree", () => {
    const d = doc(h(1, "T"), h(2, "A"), h(2, "B"));
    expect(JSON.stringify(buildToc(d))).toBe(JSON.stringify(buildToc(d)));
  });

  it("does not mutate input document", () => {
    const d = doc(h(1, "T"), h(2, "A"));
    const before = JSON.stringify(d);
    buildToc(d);
    expect(JSON.stringify(d)).toBe(before);
  });
});

describe("buildToc — defaults match no-options call", () => {
  it("buildToc(doc) === buildToc(doc, {})", () => {
    const d = doc(h(1, "A"), h(2, "B"), h(3, "C"));
    expect(JSON.stringify(buildToc(d))).toBe(JSON.stringify(buildToc(d, {})));
  });

  it("buildTocFromSlugs(slugs) === buildTocFromSlugs(slugs, {})", () => {
    const slugs = [s(1, "a"), s(2, "b")];
    expect(JSON.stringify(buildTocFromSlugs(slugs))).toBe(JSON.stringify(buildTocFromSlugs(slugs, {})));
  });
});
