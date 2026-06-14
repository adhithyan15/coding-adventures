/**
 * collect.test.ts — collectByAuthor end-to-end behaviour.
 */

import { describe, it, expect } from "vitest";
import { collectByAuthor } from "../src/index.js";

interface Post {
  id: string;
  title: string;
  author?: string;
  authors?: readonly string[];
  pubDate?: string;
}

const POSTS: Post[] = [
  { id: "a", title: "A", author: "Ada Lovelace", pubDate: "2026-01-15" },
  { id: "b", title: "B", author: "ada lovelace", pubDate: "2026-02-20" },
  { id: "c", title: "C", authors: ["Ada Lovelace", "Charles Babbage"], pubDate: "2025-12-01" },
  { id: "d", title: "D" },                       // no author
  { id: "e", title: "E", author: "" },           // empty author
  { id: "f", title: "F", authors: [] },          // empty co-author array
];

const authorOf = (p: Post): string | readonly string[] | null | undefined =>
  p.author ?? p.authors;

describe("collectByAuthor — basic grouping", () => {
  it("groups by normalised author", () => {
    const { byAuthor } = collectByAuthor(POSTS, { authorOf });
    expect(byAuthor.has("ada-lovelace")).toBe(true);
    expect(byAuthor.has("charles-babbage")).toBe(true);
  });

  it("merges case/punctuation variants into one bucket", () => {
    const { byAuthor } = collectByAuthor(POSTS, { authorOf });
    const ada = byAuthor.get("ada-lovelace")!;
    expect(ada.map((p) => p.id).sort()).toEqual(["a", "b", "c"]);
  });

  it("anonymous items dropped by default", () => {
    const { byAuthor, authorNames } = collectByAuthor(POSTS, { authorOf });
    expect(authorNames.includes("anonymous")).toBe(false);
    expect(byAuthor.has("anonymous")).toBe(false);
  });

  it("co-authored item appears in every co-author's bucket", () => {
    const { byAuthor } = collectByAuthor(POSTS, { authorOf });
    // Post 'c' has Ada + Charles → in both buckets.
    expect(byAuthor.get("ada-lovelace")!.some((p) => p.id === "c")).toBe(true);
    expect(byAuthor.get("charles-babbage")!.some((p) => p.id === "c")).toBe(true);
  });
});

describe("collectByAuthor — string vs array accessor", () => {
  it("single-string author handled", () => {
    const items = [{ id: "x", title: "X", author: "Solo" }];
    const { byAuthor } = collectByAuthor(items, { authorOf: (p) => p.author });
    expect(byAuthor.get("solo")!.length).toBe(1);
  });

  it("array author handled (single element)", () => {
    const items = [{ id: "x", title: "X", authors: ["Solo"] }];
    const { byAuthor } = collectByAuthor(items, { authorOf: (p) => p.authors });
    expect(byAuthor.get("solo")!.length).toBe(1);
  });

  it("co-author array splits into multiple buckets", () => {
    const items = [{ id: "x", title: "X", authors: ["A", "B", "C"] }];
    const { byAuthor } = collectByAuthor(items, { authorOf: (p) => p.authors });
    expect(byAuthor.size).toBe(3);
    expect(byAuthor.get("a")!.length).toBe(1);
    expect(byAuthor.get("b")!.length).toBe(1);
    expect(byAuthor.get("c")!.length).toBe(1);
  });

  it("non-string array elements dropped defensively", () => {
    // Some loose-typed manifest might mix in nulls; we shouldn't
    // crash and we shouldn't normalise them to "null".
    const items = [{ id: "x", title: "X" }];
    const { byAuthor } = collectByAuthor(items, {
      authorOf: () => ["Ada", null, undefined, 42, "Charles"] as unknown as string[],
    });
    expect(byAuthor.size).toBe(2);
    expect(byAuthor.has("ada")).toBe(true);
    expect(byAuthor.has("charles")).toBe(true);
    expect(byAuthor.has("null")).toBe(false);
    expect(byAuthor.has("undefined")).toBe(false);
    expect(byAuthor.has("42")).toBe(false);
  });

  it("empty string in array dropped", () => {
    const items = [{ id: "x", title: "X" }];
    const { byAuthor } = collectByAuthor(items, {
      authorOf: () => ["", "Ada"],
    });
    expect(byAuthor.size).toBe(1);
    expect(byAuthor.has("ada")).toBe(true);
  });
});

describe("collectByAuthor — sorted authorNames", () => {
  it("authorNames is alphabetically sorted", () => {
    const { authorNames } = collectByAuthor(POSTS, { authorOf });
    expect(authorNames).toEqual([...authorNames].sort());
  });

  it("authorNames length matches byAuthor.size", () => {
    const { byAuthor, authorNames } = collectByAuthor(POSTS, { authorOf });
    expect(authorNames.length).toBe(byAuthor.size);
  });
});

describe("collectByAuthor — within-bucket sort", () => {
  it("default: preserve input order", () => {
    const { byAuthor } = collectByAuthor(POSTS, { authorOf });
    // ada-lovelace bucket: a (input idx 0) → b (idx 1) → c (idx 2)
    expect(byAuthor.get("ada-lovelace")!.map((p) => p.id)).toEqual(["a", "b", "c"]);
  });

  it("sortBy comparator sorts within each bucket", () => {
    const { byAuthor } = collectByAuthor(POSTS, {
      authorOf,
      sortBy: (a, b) => (b.pubDate ?? "").localeCompare(a.pubDate ?? ""),
    });
    // newest-first within ada-lovelace: b (2026-02) → a (2026-01) → c (2025-12)
    expect(byAuthor.get("ada-lovelace")!.map((p) => p.id)).toEqual(["b", "a", "c"]);
  });

  it("sortBy applied to every bucket", () => {
    const { byAuthor } = collectByAuthor(POSTS, {
      authorOf,
      sortBy: (a, b) => a.id.localeCompare(b.id),
    });
    for (const bucket of byAuthor.values()) {
      const ids = bucket.map((p) => p.id);
      expect(ids).toEqual([...ids].sort());
    }
  });
});

describe("collectByAuthor — anonymous bucket", () => {
  it("includeAnonymous: true creates the bucket", () => {
    const { byAuthor, authorNames } = collectByAuthor(POSTS, { authorOf, includeAnonymous: true });
    expect(byAuthor.has("anonymous")).toBe(true);
    expect(authorNames).toContain("anonymous");
  });

  it("anonymous bucket contains items with no author field", () => {
    const { byAuthor } = collectByAuthor(POSTS, { authorOf, includeAnonymous: true });
    const anon = byAuthor.get("anonymous")!;
    expect(anon.map((p) => p.id).sort()).toEqual(["d", "e", "f"]);
  });

  it("anonymousBucketName option overrides", () => {
    const { byAuthor } = collectByAuthor(POSTS, {
      authorOf,
      includeAnonymous: true,
      anonymousBucketName: "no-byline",
    });
    expect(byAuthor.has("anonymous")).toBe(false);
    expect(byAuthor.has("no-byline")).toBe(true);
  });

  it("authorOf returning null treated as anonymous", () => {
    const items = [{ id: "x" }];
    const { byAuthor } = collectByAuthor(items, {
      authorOf: () => null,
      includeAnonymous: true,
    });
    expect(byAuthor.get("anonymous")!.length).toBe(1);
  });

  it("authorOf returning undefined treated as anonymous", () => {
    const items = [{ id: "x" }];
    const { byAuthor } = collectByAuthor(items, {
      authorOf: () => undefined,
      includeAnonymous: true,
    });
    expect(byAuthor.get("anonymous")!.length).toBe(1);
  });

  it("includeAnonymous: false explicit also drops", () => {
    const { byAuthor } = collectByAuthor(POSTS, { authorOf, includeAnonymous: false });
    expect(byAuthor.has("anonymous")).toBe(false);
  });

  it("all-stripped names treated as anonymous when includeAnonymous=true", () => {
    const items = [{ id: "x", title: "X", author: "夏目漱石" }];
    const { byAuthor } = collectByAuthor(items, { authorOf: (p) => p.author, includeAnonymous: true });
    expect(byAuthor.get("anonymous")!.length).toBe(1);
  });
});

describe("collectByAuthor — per-item dedup", () => {
  it("same author multiple times in co-author array dedups", () => {
    const items = [{ id: "x", title: "X", authors: ["Ada", "ada", "ADA"] }];
    const { byAuthor } = collectByAuthor(items, { authorOf: (p) => p.authors });
    expect(byAuthor.get("ada")!.length).toBe(1);
  });

  it("different-normalised co-authors get separate buckets", () => {
    const items = [{ id: "x", title: "X", authors: ["Ada Lovelace", "Charles Babbage"] }];
    const { byAuthor } = collectByAuthor(items, { authorOf: (p) => p.authors });
    expect(byAuthor.get("ada-lovelace")!.length).toBe(1);
    expect(byAuthor.get("charles-babbage")!.length).toBe(1);
  });
});

describe("collectByAuthor — prototype pollution defence", () => {
  it("'__proto__' author normalises to 'proto' and lands in its own bucket", () => {
    const items = [{ id: "x", title: "X", author: "__proto__" }];
    const { byAuthor } = collectByAuthor(items, { authorOf: (p) => p.author });
    expect(byAuthor.has("proto")).toBe(true);
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
  });

  it("attacker-key bypass also safe (Map storage)", () => {
    const items = [{ id: "x", title: "X" }];
    const { byAuthor } = collectByAuthor(items, {
      authorOf: () => "__proto__-literally",
    });
    expect(byAuthor.has("proto-literally")).toBe(true);
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
  });

  it("'constructor' author normalised + Map-stored", () => {
    const items = [{ id: "x", title: "X", author: "constructor" }];
    const { byAuthor } = collectByAuthor(items, { authorOf: (p) => p.author });
    expect(byAuthor.has("constructor")).toBe(true);
  });
});

describe("collectByAuthor — purity / determinism", () => {
  it("does not mutate input items array", () => {
    const before = JSON.stringify(POSTS);
    collectByAuthor(POSTS, { authorOf, sortBy: (a, b) => a.id.localeCompare(b.id) });
    expect(JSON.stringify(POSTS)).toBe(before);
  });

  it("does not mutate co-author arrays", () => {
    const item: Post = { id: "x", title: "X", authors: ["B", "A"] };
    collectByAuthor([item], { authorOf: (p) => p.authors });
    expect(item.authors).toEqual(["B", "A"]);  // unchanged order
  });

  it("same input → byte-identical output", () => {
    const a = collectByAuthor(POSTS, { authorOf });
    const b = collectByAuthor(POSTS, { authorOf });
    expect(JSON.stringify([...a.byAuthor.entries()])).toBe(JSON.stringify([...b.byAuthor.entries()]));
    expect(a.authorNames).toEqual(b.authorNames);
  });

  it("output buckets are fresh arrays", () => {
    const a = collectByAuthor(POSTS, { authorOf });
    const b = collectByAuthor(POSTS, { authorOf });
    expect(a.byAuthor.get("ada-lovelace")).not.toBe(b.byAuthor.get("ada-lovelace"));
  });

  it("byAuthor Map iteration order matches first-seen-bucket order", () => {
    const items: Post[] = [
      { id: "1", title: "1", author: "Zelda" },
      { id: "2", title: "2", author: "Apollo" },
      { id: "3", title: "3", author: "Mercury" },
    ];
    const { byAuthor, authorNames } = collectByAuthor(items, { authorOf });
    expect([...byAuthor.keys()]).toEqual(["zelda", "apollo", "mercury"]);
    expect(authorNames).toEqual(["apollo", "mercury", "zelda"]);
  });

  it("empty input → empty result", () => {
    const { byAuthor, authorNames } = collectByAuthor([], { authorOf });
    expect(byAuthor.size).toBe(0);
    expect(authorNames).toEqual([]);
  });
});

describe("collectByAuthor — authors that normalise to empty are dropped", () => {
  it("author of '@@@' doesn't create a bucket", () => {
    const items = [{ id: "x", title: "X", author: "@@@" }];
    const { byAuthor } = collectByAuthor(items, { authorOf: (p) => p.author });
    expect(byAuthor.size).toBe(0);
  });

  it("mix of valid + empty author names: only valid one creates bucket", () => {
    const items = [{ id: "x", title: "X", authors: ["@@@", "Ada"] }];
    const { byAuthor } = collectByAuthor(items, { authorOf: (p) => p.authors });
    expect(byAuthor.size).toBe(1);
    expect(byAuthor.get("ada")!.length).toBe(1);
  });
});
