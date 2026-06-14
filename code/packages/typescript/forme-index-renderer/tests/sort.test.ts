/**
 * sort.test.ts — comparators.
 */

import { describe, it, expect } from "vitest";
import { sortItems, type IndexItem } from "../src/index.js";

const items: IndexItem[] = [
  { id: "b", title: "Bravo",  url: "/b", pubDate: "2026-01-02T00:00:00Z" },
  { id: "a", title: "Alpha",  url: "/a", pubDate: "2026-01-03T00:00:00Z" },
  { id: "c", title: "Charlie", url: "/c", pubDate: "2026-01-01T00:00:00Z" },
  { id: "d", title: "Delta",  url: "/d" },  // no pubDate
];

describe("sortItems — pubDate-desc (default)", () => {
  it("newest first", () => {
    const out = sortItems(items, "pubDate-desc").map((i) => i.id);
    expect(out).toEqual(["a", "b", "c", "d"]);
  });

  it("undated items sort to the end", () => {
    const out = sortItems([items[3]!, items[0]!], "pubDate-desc").map((i) => i.id);
    expect(out).toEqual(["b", "d"]);
  });

  it("ties broken by id ascending", () => {
    const same: IndexItem[] = [
      { id: "z", title: "Z", url: "/z", pubDate: "2026-01-01T00:00:00Z" },
      { id: "a", title: "A", url: "/a", pubDate: "2026-01-01T00:00:00Z" },
    ];
    const out = sortItems(same, "pubDate-desc").map((i) => i.id);
    expect(out).toEqual(["a", "z"]);
  });
});

describe("sortItems — pubDate-asc", () => {
  it("oldest first", () => {
    const out = sortItems(items, "pubDate-asc").map((i) => i.id);
    expect(out).toEqual(["c", "b", "a", "d"]);
  });

  it("undated items sort to the end", () => {
    const out = sortItems([items[3]!, items[0]!], "pubDate-asc").map((i) => i.id);
    expect(out).toEqual(["b", "d"]);
  });
});

describe("sortItems — title-asc", () => {
  it("alphabetical by title", () => {
    const out = sortItems(items, "title-asc").map((i) => i.id);
    expect(out).toEqual(["a", "b", "c", "d"]);
  });

  it("ties broken by id ascending", () => {
    const same: IndexItem[] = [
      { id: "z", title: "Same", url: "/z" },
      { id: "a", title: "Same", url: "/a" },
    ];
    const out = sortItems(same, "title-asc").map((i) => i.id);
    expect(out).toEqual(["a", "z"]);
  });
});

describe("sortItems — returns a copy (input not mutated)", () => {
  it("input array order is preserved after sort", () => {
    const original = [...items];
    sortItems(items, "pubDate-desc");
    expect(items).toEqual(original);
  });
});

describe("sortItems — malformed pubDate", () => {
  it("malformed pubDate treated as undated (sorts to end)", () => {
    const mixed: IndexItem[] = [
      { id: "good", title: "G", url: "/g", pubDate: "2026-01-01T00:00:00Z" },
      { id: "bad",  title: "B", url: "/b", pubDate: "not-a-date" },
    ];
    expect(sortItems(mixed, "pubDate-desc").map((i) => i.id)).toEqual(["good", "bad"]);
  });

  it("all malformed → ties broken by id", () => {
    const all: IndexItem[] = [
      { id: "z", title: "Z", url: "/z", pubDate: "garbage" },
      { id: "a", title: "A", url: "/a", pubDate: "trash" },
    ];
    expect(sortItems(all, "pubDate-desc").map((i) => i.id)).toEqual(["a", "z"]);
  });
});
