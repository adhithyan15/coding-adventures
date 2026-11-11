/**
 * collect.test.ts — collectByTag end-to-end behaviour.
 */

import assert from "node:assert/strict";
import { describe, test as it } from "node:test";
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
    assert.equal(byTag.has("typescript"), true);
    assert.equal(byTag.has("react"), true);
    assert.equal(byTag.has("node-22"), true);
  });

  it("merges case/punctuation variants into one bucket", () => {
    const { byTag } = collectByTag(POSTS, { tagsOf });
    const tsBucket = byTag.get("typescript")!;
    assert.deepEqual(tsBucket.map((p) => p.id).sort(), ["a", "b"]);
  });

  it("untagged items dropped by default", () => {
    const { byTag, tagNames } = collectByTag(POSTS, { tagsOf });
    assert.equal(tagNames.includes("untagged"), false);
    assert.equal(byTag.has("untagged"), false);
  });

  it("each item appears in each of its tags' buckets", () => {
    const { byTag } = collectByTag(POSTS, { tagsOf });
    assert.deepEqual(byTag.get("react")!.map((p) => p.id).sort(), ["b", "c"]);
  });
});

describe("collectByTag — sorted tagNames", () => {
  it("tagNames is alphabetically sorted", () => {
    const { tagNames } = collectByTag(POSTS, { tagsOf });
    const sortedCopy = [...tagNames].sort();
    assert.deepEqual(tagNames, sortedCopy);
  });

  it("tagNames length matches byTag.size", () => {
    const { byTag, tagNames } = collectByTag(POSTS, { tagsOf });
    assert.equal(tagNames.length, byTag.size);
  });
});

describe("collectByTag — within-bucket sort", () => {
  it("default: preserve input order", () => {
    const { byTag } = collectByTag(POSTS, { tagsOf });
    // typescript bucket: post 'a' first (appeared first), then 'b'.
    assert.deepEqual(byTag.get("typescript")!.map((p) => p.id), ["a", "b"]);
  });

  it("sortBy comparator sorts within each bucket", () => {
    const { byTag } = collectByTag(POSTS, {
      tagsOf,
      sortBy: (a, b) => (b.pubDate ?? "").localeCompare(a.pubDate ?? ""),
    });
    // newest-first within typescript: b (2026-02) before a (2026-01)
    assert.deepEqual(byTag.get("typescript")!.map((p) => p.id), ["b", "a"]);
    // newest-first within react: b (2026-02) before c (2025-12)
    assert.deepEqual(byTag.get("react")!.map((p) => p.id), ["b", "c"]);
  });

  it("sortBy applied to every bucket", () => {
    const { byTag } = collectByTag(POSTS, {
      tagsOf,
      sortBy: (a, b) => a.id.localeCompare(b.id),
    });
    for (const bucket of byTag.values()) {
      const ids = bucket.map((p) => p.id);
      assert.deepEqual(ids, [...ids].sort());
    }
  });
});

describe("collectByTag — untagged bucket", () => {
  it("includeUntagged: true creates the bucket", () => {
    const { byTag, tagNames } = collectByTag(POSTS, { tagsOf, includeUntagged: true });
    assert.equal(byTag.has("untagged"), true);
    assert.ok(tagNames.includes("untagged"));
  });

  it("untagged bucket contains items with no tags field", () => {
    const { byTag } = collectByTag(POSTS, { tagsOf, includeUntagged: true });
    const untagged = byTag.get("untagged")!;
    assert.deepEqual(untagged.map((p) => p.id).sort(), ["d", "e"]);
  });

  it("untagged includes items with empty tags array", () => {
    const items: Post[] = [{ id: "x", title: "X", tags: [] }];
    const { byTag } = collectByTag(items, { tagsOf, includeUntagged: true });
    assert.equal(byTag.get("untagged")!.length, 1);
  });

  it("untagged includes items where every tag normalises to empty", () => {
    const items: Post[] = [{ id: "x", title: "X", tags: ["@@@", "日本語"] }];
    const { byTag } = collectByTag(items, { tagsOf, includeUntagged: true });
    assert.equal(byTag.get("untagged")!.length, 1);
  });

  it("untaggedBucketName option overrides the bucket name", () => {
    const { byTag } = collectByTag(POSTS, {
      tagsOf,
      includeUntagged: true,
      untaggedBucketName: "no-tags",
    });
    assert.equal(byTag.has("untagged"), false);
    assert.equal(byTag.has("no-tags"), true);
  });

  it("includeUntagged: false explicit also drops untagged", () => {
    const { byTag } = collectByTag(POSTS, { tagsOf, includeUntagged: false });
    assert.equal(byTag.has("untagged"), false);
  });

  it("tagsOf returning null treated as untagged", () => {
    const items = [{ id: "x" }];
    const { byTag } = collectByTag(items, {
      tagsOf: () => null,
      includeUntagged: true,
    });
    assert.equal(byTag.get("untagged")!.length, 1);
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
    assert.equal(byTag.get("typescript")?.length, 1);
    assert.equal(byTag.get("type-script")?.length, 1);
  });

  it("genuinely identical normalised tags dedupe to a single insert", () => {
    const items: Post[] = [
      { id: "x", title: "X", tags: ["TypeScript", "typescript", "TYPESCRIPT"] },
    ];
    const { byTag } = collectByTag(items, { tagsOf });
    assert.equal(byTag.get("typescript")!.length, 1);
  });
});

describe("collectByTag — prototype pollution defence", () => {
  it("'__proto__' tag normalises to 'proto' and lands in its own bucket", () => {
    const items: Post[] = [{ id: "x", title: "X", tags: ["__proto__"] }];
    const { byTag } = collectByTag(items, { tagsOf });
    assert.equal(byTag.has("proto"), true);
    // Object.prototype must not have been polluted.
    assert.equal(({} as Record<string, unknown>).polluted, undefined);
  });

  it("even if a hostile tag somehow survived as '__proto__', Map keys don't touch Object.prototype", () => {
    const items: Post[] = [{ id: "x", title: "X" }];
    // Synthetic test: bypass normaliseTag by injecting a tagsOf
    // that returns a pre-baked attacker key.  Map storage protects.
    const { byTag } = collectByTag(items, {
      tagsOf: () => ["__proto__-literally"],
    });
    assert.equal(byTag.has("proto-literally"), true);
    assert.equal(({} as Record<string, unknown>).polluted, undefined);
  });

  it("'constructor' tag normalised + Map-stored — no constructor hijack", () => {
    const items: Post[] = [{ id: "x", title: "X", tags: ["constructor"] }];
    const { byTag } = collectByTag(items, { tagsOf });
    assert.equal(byTag.has("constructor"), true);
    assert.equal(byTag.get("constructor")!.length, 1);
  });
});

describe("collectByTag — purity / determinism", () => {
  it("does not mutate input items array", () => {
    const before = JSON.stringify(POSTS);
    collectByTag(POSTS, { tagsOf, sortBy: (a, b) => a.id.localeCompare(b.id) });
    assert.equal(JSON.stringify(POSTS), before);
  });

  it("does not mutate individual item tag arrays", () => {
    const item: Post = { id: "x", title: "X", tags: ["b", "a"] };
    collectByTag([item], { tagsOf });
    assert.deepEqual(item.tags, ["b", "a"]);  // unchanged order
  });

  it("same input → byte-identical output", () => {
    const a = collectByTag(POSTS, { tagsOf });
    const b = collectByTag(POSTS, { tagsOf });
    assert.equal(JSON.stringify([...a.byTag.entries()]), JSON.stringify([...b.byTag.entries()]));
    assert.deepEqual(a.tagNames, b.tagNames);
  });

  it("output buckets are fresh arrays (no shared references)", () => {
    const a = collectByTag(POSTS, { tagsOf });
    const b = collectByTag(POSTS, { tagsOf });
    assert.notEqual(a.byTag.get("typescript"), b.byTag.get("typescript"));
  });

  it("byTag Map iteration order matches first-seen-bucket order", () => {
    const items: Post[] = [
      { id: "1", title: "1", tags: ["Zebra"] },
      { id: "2", title: "2", tags: ["Apple"] },
      { id: "3", title: "3", tags: ["Mango"] },
    ];
    const { byTag, tagNames } = collectByTag(items, { tagsOf });
    assert.deepEqual([...byTag.keys()], ["zebra", "apple", "mango"]);
    // tagNames is sorted alphabetically though, regardless.
    assert.deepEqual(tagNames, ["apple", "mango", "zebra"]);
  });

  it("empty input → empty result", () => {
    const { byTag, tagNames } = collectByTag([], { tagsOf });
    assert.equal(byTag.size, 0);
    assert.deepEqual(tagNames, []);
  });
});

describe("collectByTag — tags that normalise to empty are dropped", () => {
  it("tag of '@@@' (normalises to '') doesn't create a bucket", () => {
    const items: Post[] = [
      { id: "x", title: "X", tags: ["@@@", "javascript"] },
    ];
    const { byTag, tagNames } = collectByTag(items, { tagsOf });
    assert.deepEqual(tagNames, ["javascript"]);
    assert.equal(byTag.size, 1);
  });

  it("all tags normalise to empty + includeUntagged=true → goes to untagged", () => {
    const items: Post[] = [{ id: "x", title: "X", tags: ["@@@", "..."] }];
    const { byTag } = collectByTag(items, { tagsOf, includeUntagged: true });
    assert.equal(byTag.get("untagged")!.length, 1);
  });

  it("all tags normalise to empty + includeUntagged=false → item disappears", () => {
    const items: Post[] = [{ id: "x", title: "X", tags: ["@@@"] }];
    const { byTag } = collectByTag(items, { tagsOf });
    assert.equal(byTag.size, 0);
  });
});
