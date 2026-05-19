/**
 * group.test.ts — grouping by category / year / month.
 */

import { describe, it, expect } from "vitest";
import { groupItems, type IndexItem } from "../src/index.js";

const ITEMS: IndexItem[] = [
  { id: "1", title: "T1", url: "/1", pubDate: "2026-01-15T00:00:00Z", category: "Code" },
  { id: "2", title: "T2", url: "/2", pubDate: "2026-02-20T00:00:00Z", category: "Life" },
  { id: "3", title: "T3", url: "/3", pubDate: "2025-12-01T00:00:00Z", category: "Code" },
  { id: "4", title: "T4", url: "/4" }, // no pubDate, no category
];

describe("groupItems — none", () => {
  it("returns a single group with empty heading", () => {
    const groups = groupItems(ITEMS, "none");
    expect(groups.length).toBe(1);
    expect(groups[0]!.heading).toBe("");
    expect(groups[0]!.items.length).toBe(ITEMS.length);
  });
});

describe("groupItems — category", () => {
  it("buckets by category", () => {
    const groups = groupItems(ITEMS, "category");
    const code = groups.find((g) => g.heading === "Code");
    const life = groups.find((g) => g.heading === "Life");
    expect(code?.items.map((i) => i.id).sort()).toEqual(["1", "3"]);
    expect(life?.items.map((i) => i.id)).toEqual(["2"]);
  });

  it("items without a category land in `Uncategorized`", () => {
    const groups = groupItems(ITEMS, "category");
    const u = groups.find((g) => g.heading === "Uncategorized");
    expect(u?.items.map((i) => i.id)).toEqual(["4"]);
  });

  it("category groups are alphabetical with Uncategorized last", () => {
    const headings = groupItems(ITEMS, "category").map((g) => g.heading);
    expect(headings).toEqual(["Code", "Life", "Uncategorized"]);
  });
});

describe("groupItems — year", () => {
  it("buckets by 4-digit UTC year", () => {
    const groups = groupItems(ITEMS, "year");
    expect(groups.find((g) => g.heading === "2026")?.items.map((i) => i.id).sort()).toEqual(["1", "2"]);
    expect(groups.find((g) => g.heading === "2025")?.items.map((i) => i.id)).toEqual(["3"]);
  });

  it("undated items land in Undated", () => {
    const groups = groupItems(ITEMS, "year");
    const u = groups.find((g) => g.heading === "Undated");
    expect(u?.items.map((i) => i.id)).toEqual(["4"]);
  });

  it("year groups are reverse-chronological with Undated last", () => {
    const headings = groupItems(ITEMS, "year").map((g) => g.heading);
    expect(headings).toEqual(["2026", "2025", "Undated"]);
  });

  it("malformed pubDate → Undated", () => {
    const bad: IndexItem[] = [{ id: "x", title: "X", url: "/x", pubDate: "trash" }];
    const groups = groupItems(bad, "year");
    expect(groups[0]!.heading).toBe("Undated");
  });
});

describe("groupItems — month", () => {
  it("buckets by YYYY-MM", () => {
    const groups = groupItems(ITEMS, "month");
    expect(groups.find((g) => g.heading === "2026-01")?.items.map((i) => i.id)).toEqual(["1"]);
    expect(groups.find((g) => g.heading === "2026-02")?.items.map((i) => i.id)).toEqual(["2"]);
    expect(groups.find((g) => g.heading === "2025-12")?.items.map((i) => i.id)).toEqual(["3"]);
  });

  it("month groups are reverse-chronological with Undated last", () => {
    const headings = groupItems(ITEMS, "month").map((g) => g.heading);
    expect(headings).toEqual(["2026-02", "2026-01", "2025-12", "Undated"]);
  });
});

describe("groupItems — preserves caller's item order within group", () => {
  it("two items in the same bucket keep input order", () => {
    const sameMonth: IndexItem[] = [
      { id: "c", title: "C", url: "/c", pubDate: "2026-01-15T00:00:00Z" },
      { id: "a", title: "A", url: "/a", pubDate: "2026-01-10T00:00:00Z" },
      { id: "b", title: "B", url: "/b", pubDate: "2026-01-20T00:00:00Z" },
    ];
    // groupItems doesn't sort within groups — preserves order.
    const groups = groupItems(sameMonth, "month");
    expect(groups[0]!.items.map((i) => i.id)).toEqual(["c", "a", "b"]);
  });
});

describe("groupItems — empty input", () => {
  it("none → one group, no items", () => {
    expect(groupItems([], "none")).toEqual([{ heading: "", items: [] }]);
  });

  it("category → no groups", () => {
    expect(groupItems([], "category")).toEqual([]);
  });
});
