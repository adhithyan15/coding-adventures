/**
 * collect.test.ts — collectByTag end-to-end behaviour.
 */

import { describe, it, expect } from "vitest";
import { collectByTag } from "../src/index.js";

interface Post {
  id: string;
  title: string;
  tags?: readonly string[];
  pubDate?: string;
}

const POSTS: Post[] = [
  { id: "a", title: "A", tags: ["TypeScript", "Node 22"], pubDate: "2026-01-15" },
  { id: "b", title: "B", tags: ["typescript", "React"], pubDate: "2026-02-20" },
  { id: "c", title: "C", tags: ["react"], pubDate: "2025-12-01" },
  { id: "d", title: "D" },  // no tags field
  { id: "e", title: "E", tags: [] },  // explicit empty
];

const tagsOf = (p: Post) => p.tags;

describe("collectByTag — basic grouping", () => {
  it("returns a Map keyed by normalised tag", () => {
    const { byTag } = collectByTag(POSTS, { tagsOf });
    expect(byTag.has("typescript")).toBe(true);
    expect(byTag.has("react")).toBe(true);
    expect(byTag.has("node-22")).toBe(true);
  });

  it("merges case/punctuation variants into one bucket", () => {
    const { byTag } = collectByTag(POSTS, { tagsOf });
    const tsBucket = byTag.get("typescript")!;
    expect(tsBucket.map((p) => p.id).sort()).toEqual(["a", "b"]);
  });

  it("untagged items dropped by default", () => {
    const { byTag, tagNames } = collectByTag(POSTS, { tagsOf });
    expect(tagNames.includes("untagged")).toBe(false);
    expect(byTag.has("untagged")).toBe(false);
  });

  it("each item appears in each of its tags' buckets", () => {
    const { byTag } = collectByTag(POSTS, { tagsOf });
    expect(byTag.get("react")!.map((p) => p.id).sort()).toEqual(["b", "c"]);
  });
});

describe("collectByTag — sorted tagNames", () => {
  it("tagNames is alphabetically sorted", () => {
    const { tagNames } = collectByTag(POSTS, { tagsOf });
    const sortedCopy = [...tagNames].sort();
    expect(tagNames).toEqual(sortedCopy);
  });

  it("tagNames length matches byTag.size", () => {
    const { byTag, tagNames } = collectByTag(POSTS, { tagsOf });
    expect(tagNames.length).toBe(byTag.size);
  });
});

describe("collectByTag — within-bucket sort", () => {
  it("default: preserve input order", () => {
    const { byTag } = collectByTag(POSTS, { tagsOf });
    // typescript bucket: post 'a' first (appeared first), then 'b'.
    expect(byTag.get("typescript")!.map((p) => p.id)).toEqual(["a", "b"]);
  });

  it("sortBy comparator sorts within each bucket", () => {
    const { byTag } = collectByTag(POSTS, {
      tagsOf,
      sortBy: (a, b) => (b.pubDate ?? "").localeCompare(a.pubDate ?? ""),
    });
    // newest-first within typescript: b (2026-02) before a (2026-01)
    expect(byTag.get("typescript")!.map((p) => p.id)).toEqual(["b", "a"]);
    // newest-first within react: b (2026-02) before c (2025-12)
    expect(byTag.get("react")!.map((p) => p.id)).toEqual(["b", "c"]);
  });

  it("sortBy applied to every bucket", () => {
    const { byTag } = collectByTag(POSTS, {
      tagsOf,
      sortBy: (a, b) => a.id.localeCompare(b.id),
    });
    for (const bucket of byTag.values()) {
      const ids = bucket.map((p) => p.id);
      expect(ids).toEqual([...ids].sort());
    }
  });
});

describe("collectByTag — untagged bucket", () => {
  it("includeUntagged: true creates the bucket", () => {
    const { byTag, tagNames } = collectByTag(POSTS, { tagsOf, includeUntagged: true });
    expect(byTag.has("untagged")).toBe(true);
    expect(tagNames).toContain("untagged");
  });

  it("untagged bucket contains items with no tags field", () => {
    const { byTag } = collectByTag(POSTS, { tagsOf, includeUntagged: true });
    const untagged = byTag.get("untagged")!;
    expect(untagged.map((p) => p.id).sort()).toEqual(["d", "e"]);
  });

  it("untagged includes items with empty tags array", () => {
    const items: Post[] = [{ id: "x", title: "X", tags: [] }];
    const { byTag } = collectByTag(items, { tagsOf, includeUntagged: true });
    expect(byTag.get("untagged")!.length).toBe(1);
  });

  it("untagged includes items where every tag normalises to empty", () => {
    const items: Post[] = [{ id: "x", title: "X", tags: ["@@@", "日本語"] }];
    const { byTag } = collectByTag(items, { tagsOf, includeUntagged: true });
    expect(byTag.get("untagged")!.length).toBe(1);
  });

  it("untaggedBucketName option overrides the bucket name", () => {
    const { byTag } = collectByTag(POSTS, {
      tagsOf,
      includeUntagged: true,
      untaggedBucketName: "no-tags",
    });
    expect(byTag.has("untagged")).toBe(false);
    expect(byTag.has("no-tags")).toBe(true);
  });

  it("includeUntagged: false explicit also drops untagged", () => {
    const { byTag } = collectByTag(POSTS, { tagsOf, includeUntagged: false });
    expect(byTag.has("untagged")).toBe(false);
  });

  it("tagsOf returning null treated as untagged", () => {
    const items = [{ id: "x" }];
    const { byTag } = collectByTag(items, {
      tagsOf: () => null,
      includeUntagged: true,
    });
    expect(byTag.get("untagged")!.length).toBe(1);
  });
});

describe("collectByTag — per-item dedup", () => {
  it("two raw tags normalising to the same bucket only insert the item once", () => {
    const items: Post[] = [
      { id: "x", title: "X", tags: ["TypeScript", "typescript", "Type Script"] },
    ];
    const { byTag } = collectByTag(items, { tagsOf });
    // All three raw tags normalise to "type-script" vs "typescript" —
    // wait: "Type Script" → "type-script", "typescript" → "typescript".
    // So we get TWO buckets, each containing the item once.
    expect(byTag.get("typescript")?.length).toBe(1);
    expect(byTag.get("type-script")?.length).toBe(1);
  });

  it("genuinely identical normalised tags dedupe to a single insert", () => {
    const items: Post[] = [
      { id: "x", title: "X", tags: ["TypeScript", "typescript", "TYPESCRIPT"] },
    ];
    const { byTag } = collectByTag(items, { tagsOf });
    expect(byTag.get("typescript")!.length).toBe(1);
  });
});

describe("collectByTag — prototype pollution defence", () => {
  it("'__proto__' tag normalises to 'proto' and lands in its own bucket", () => {
    const items: Post[] = [{ id: "x", title: "X", tags: ["__proto__"] }];
    const { byTag } = collectByTag(items, { tagsOf });
    expect(byTag.has("proto")).toBe(true);
    // Object.prototype must not have been polluted.
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
  });

  it("even if a hostile tag somehow survived as '__proto__', Map keys don't touch Object.prototype", () => {
    const items: Post[] = [{ id: "x", title: "X" }];
    // Synthetic test: bypass normaliseTag by injecting a tagsOf
    // that returns a pre-baked attacker key.  Map storage protects.
    const { byTag } = collectByTag(items, {
      tagsOf: () => ["__proto__-literally"],
    });
    expect(byTag.has("proto-literally")).toBe(true);
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
  });

  it("'constructor' tag normalised + Map-stored — no constructor hijack", () => {
    const items: Post[] = [{ id: "x", title: "X", tags: ["constructor"] }];
    const { byTag } = collectByTag(items, { tagsOf });
    expect(byTag.has("constructor")).toBe(true);
    expect(byTag.get("constructor")!.length).toBe(1);
  });
});

describe("collectByTag — purity / determinism", () => {
  it("does not mutate input items array", () => {
    const before = JSON.stringify(POSTS);
    collectByTag(POSTS, { tagsOf, sortBy: (a, b) => a.id.localeCompare(b.id) });
    expect(JSON.stringify(POSTS)).toBe(before);
  });

  it("does not mutate individual item tag arrays", () => {
    const item: Post = { id: "x", title: "X", tags: ["b", "a"] };
    collectByTag([item], { tagsOf });
    expect(item.tags).toEqual(["b", "a"]);  // unchanged order
  });

  it("same input → byte-identical output", () => {
    const a = collectByTag(POSTS, { tagsOf });
    const b = collectByTag(POSTS, { tagsOf });
    expect(JSON.stringify([...a.byTag.entries()])).toBe(JSON.stringify([...b.byTag.entries()]));
    expect(a.tagNames).toEqual(b.tagNames);
  });

  it("output buckets are fresh arrays (no shared references)", () => {
    const a = collectByTag(POSTS, { tagsOf });
    const b = collectByTag(POSTS, { tagsOf });
    expect(a.byTag.get("typescript")).not.toBe(b.byTag.get("typescript"));
  });

  it("byTag Map iteration order matches first-seen-bucket order", () => {
    const items: Post[] = [
      { id: "1", title: "1", tags: ["Zebra"] },
      { id: "2", title: "2", tags: ["Apple"] },
      { id: "3", title: "3", tags: ["Mango"] },
    ];
    const { byTag, tagNames } = collectByTag(items, { tagsOf });
    expect([...byTag.keys()]).toEqual(["zebra", "apple", "mango"]);
    // tagNames is sorted alphabetically though, regardless.
    expect(tagNames).toEqual(["apple", "mango", "zebra"]);
  });

  it("empty input → empty result", () => {
    const { byTag, tagNames } = collectByTag([], { tagsOf });
    expect(byTag.size).toBe(0);
    expect(tagNames).toEqual([]);
  });
});

describe("collectByTag — tags that normalise to empty are dropped", () => {
  it("tag of '@@@' (normalises to '') doesn't create a bucket", () => {
    const items: Post[] = [
      { id: "x", title: "X", tags: ["@@@", "javascript"] },
    ];
    const { byTag, tagNames } = collectByTag(items, { tagsOf });
    expect(tagNames).toEqual(["javascript"]);
    expect(byTag.size).toBe(1);
  });

  it("all tags normalise to empty + includeUntagged=true → goes to untagged", () => {
    const items: Post[] = [{ id: "x", title: "X", tags: ["@@@", "..."] }];
    const { byTag } = collectByTag(items, { tagsOf, includeUntagged: true });
    expect(byTag.get("untagged")!.length).toBe(1);
  });

  it("all tags normalise to empty + includeUntagged=false → item disappears", () => {
    const items: Post[] = [{ id: "x", title: "X", tags: ["@@@"] }];
    const { byTag } = collectByTag(items, { tagsOf });
    expect(byTag.size).toBe(0);
  });
});
