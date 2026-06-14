/**
 * builder.test.ts — search-index builder tests.
 */

import { describe, it, expect } from "vitest";
import { buildSearchIndex } from "../src/index.js";
import type { IndexPageInput } from "../src/index.js";

/** Convenience: page builder. */
function page(id: string, body: string, title?: string): IndexPageInput {
  return title === undefined ? { id, body } : { id, body, title };
}

// ─────────────────────────────────────────────────────────────────────
// Degenerate inputs
// ─────────────────────────────────────────────────────────────────────

describe("buildSearchIndex — degenerate", () => {
  it("empty input → empty index", () => {
    const out = buildSearchIndex([]);
    expect(out.shards.size).toBe(0);
    expect(out.manifest.pages).toEqual([]);
    expect(out.manifest.shardKeys).toEqual([]);
    expect(out.manifest.stats.uniqueTokens).toBe(0);
    expect(out.manifest.stats.totalTokens).toBe(0);
    expect(out.manifest.stats.pageCount).toBe(0);
    expect(out.manifest.stats.shardCount).toBe(0);
  });
  it("page with empty body produces no postings but appears in manifest", () => {
    const out = buildSearchIndex([page("/empty", "")]);
    expect(out.manifest.pages).toEqual(["/empty"]);
    expect(out.shards.size).toBe(0);
    expect(out.manifest.stats.pageCount).toBe(1);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Basic indexing
// ─────────────────────────────────────────────────────────────────────

describe("buildSearchIndex — basic indexing", () => {
  it("single page with single distinct token", () => {
    // "welcome" → no stop word, stems to "welcom"
    const out = buildSearchIndex([page("/p", "welcome")]);
    expect(out.manifest.pages).toEqual(["/p"]);
    expect(out.shards.size).toBe(1);
    const shard = out.shards.get("we")!;
    expect(shard).toBeDefined();
    expect(shard.postings.has("welcom")).toBe(true);
    const postings = shard.postings.get("welcom")!;
    expect(postings).toEqual([{ pageId: "/p", freq: 1, titleHit: false }]);
  });
  it("freq counts repeated occurrences", () => {
    const out = buildSearchIndex([page("/p", "install install install")]);
    const shard = out.shards.get("in")!;
    const postings = shard.postings.get("instal")!; // stem
    expect(postings[0].freq).toBe(3);
  });
  it("title hits set titleHit: true", () => {
    const out = buildSearchIndex([page("/p", "body text", "title")]);
    const shard = out.shards.get("ti")!;
    const postings = shard.postings.get("titl")!; // "title" stems to "titl"
    expect(postings[0].titleHit).toBe(true);
  });
  it("title hits AND body hits combine into one posting with titleHit=true", () => {
    const out = buildSearchIndex([page("/p", "install install", "install")]);
    const shard = out.shards.get("in")!;
    const postings = shard.postings.get("instal")!;
    expect(postings[0].freq).toBe(3); // 2 body + 1 title
    expect(postings[0].titleHit).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Multi-page indexing
// ─────────────────────────────────────────────────────────────────────

describe("buildSearchIndex — multi-page", () => {
  it("token appearing in two pages → posting list of length 2", () => {
    const out = buildSearchIndex([
      page("/a", "install"),
      page("/b", "install"),
    ]);
    const shard = out.shards.get("in")!;
    const postings = shard.postings.get("instal")!;
    expect(postings.length).toBe(2);
    expect(postings.map((p) => p.pageId).sort()).toEqual(["/a", "/b"]);
  });
  it("postings sorted by descending freq", () => {
    const out = buildSearchIndex([
      page("/a", "install"),
      page("/b", "install install install"),
      page("/c", "install install"),
    ]);
    const shard = out.shards.get("in")!;
    const postings = shard.postings.get("instal")!;
    expect(postings.map((p) => p.pageId)).toEqual(["/b", "/c", "/a"]);
    expect(postings.map((p) => p.freq)).toEqual([3, 2, 1]);
  });
  it("postings tie-break by ascending pageId", () => {
    const out = buildSearchIndex([
      page("/zebra", "install"),
      page("/apple", "install"),
    ]);
    const postings = out.shards.get("in")!.postings.get("instal")!;
    expect(postings.map((p) => p.pageId)).toEqual(["/apple", "/zebra"]);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Sharding
// ─────────────────────────────────────────────────────────────────────

describe("buildSearchIndex — sharding", () => {
  it("default shardPrefix=2 groups by 2-char prefix", () => {
    const out = buildSearchIndex([
      page("/p", "install welcome configure"),
    ]);
    // tokens after stemming: instal, welcom, configur
    // shard keys: "in", "we", "co"
    expect(out.manifest.shardKeys.sort()).toEqual(["co", "in", "we"]);
  });
  it("shardPrefix=1 groups by single char", () => {
    const out = buildSearchIndex(
      [page("/p", "install welcome configure")],
      { shardPrefix: 1 },
    );
    expect(out.manifest.shardKeys.sort()).toEqual(["c", "i", "w"]);
  });
  it("shardPrefix=3 produces 3-char keys", () => {
    const out = buildSearchIndex(
      [page("/p", "install welcome")],
      { shardPrefix: 3 },
    );
    expect(out.manifest.shardKeys.sort()).toEqual(["ins", "wel"]);
  });
  it("short tokens use the whole token as key", () => {
    const out = buildSearchIndex(
      [page("/p", "go to it")],
      { shardPrefix: 3, filterStopWords: false, stem: false },
    );
    // "go" (2 chars) → shard key "go" (whole token, < 3 chars)
    // "to" (2 chars) → shard key "to"
    // "it" (2 chars) → shard key "it"
    expect(out.manifest.shardKeys.sort()).toEqual(["go", "it", "to"]);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Options forwarding
// ─────────────────────────────────────────────────────────────────────

describe("buildSearchIndex — options forwarding", () => {
  it("filterStopWords=false keeps stop words", () => {
    const out = buildSearchIndex(
      [page("/p", "the cat")],
      { filterStopWords: false, stem: false },
    );
    expect(out.manifest.stats.uniqueTokens).toBe(2); // "the" and "cat"
  });
  it("filterStopWords=true (default) drops stop words", () => {
    const out = buildSearchIndex([page("/p", "the cat")]);
    expect(out.manifest.stats.uniqueTokens).toBe(1); // just "cat"
  });
  it("stem=false preserves morphological variants", () => {
    const out = buildSearchIndex(
      [page("/p", "running runs ran")],
      { filterStopWords: false, stem: false },
    );
    expect(out.manifest.stats.uniqueTokens).toBe(3);
  });
  it("stem=true (default) collapses variants", () => {
    // running → run, runs → run, ran → ran (Porter doesn't handle irregulars)
    const out = buildSearchIndex([page("/p", "running runs ran")]);
    expect(out.manifest.stats.uniqueTokens).toBe(2); // "run" + "ran"
  });
  it("customStopWords overrides built-in", () => {
    const out = buildSearchIndex(
      [page("/p", "custom built")],
      {
        filterStopWords: true,
        stem: false,
        customStopWords: new Set(["custom"]),
      },
    );
    expect(out.manifest.stats.uniqueTokens).toBe(1); // just "built"
  });
});

// ─────────────────────────────────────────────────────────────────────
// Memory bounds (caps)
// ─────────────────────────────────────────────────────────────────────

describe("buildSearchIndex — memory caps", () => {
  it("rejects pages.length > maxPages", () => {
    const pages = Array.from({ length: 5 }, (_, i) => page(`/p${i}`, "hi"));
    expect(() => buildSearchIndex(pages, { maxPages: 3 })).toThrow(/maxPages cap/);
  });
  it("default maxPages allows 100k", () => {
    // Smoke-test with a small but representative count.
    const pages = Array.from({ length: 50 }, (_, i) => page(`/p${i}`, `unique${i}`));
    expect(() => buildSearchIndex(pages)).not.toThrow();
  });
  it("maxTokensPerPage caps unique tokens from one page", () => {
    // Generate 100 unique tokens; cap at 10.
    const body = Array.from({ length: 100 }, (_, i) => `term${i}`).join(" ");
    const out = buildSearchIndex(
      [page("/p", body)],
      { filterStopWords: false, stem: false, maxTokensPerPage: 10 },
    );
    expect(out.manifest.stats.uniqueTokens).toBeLessThanOrEqual(10);
  });
  it("maxPostingsPerToken caps postings list length", () => {
    // 100 pages, all containing the same token.
    const pages = Array.from({ length: 100 }, (_, i) =>
      page(`/p${i}`, "common")
    );
    const out = buildSearchIndex(pages, { maxPostingsPerToken: 5 });
    const shard = out.shards.get("co")!;
    const postings = shard.postings.get("common")!;
    expect(postings.length).toBe(5);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Validation errors
// ─────────────────────────────────────────────────────────────────────

describe("buildSearchIndex — validation", () => {
  it("duplicate page id throws", () => {
    expect(() =>
      buildSearchIndex([page("/p", "a"), page("/p", "b")]),
    ).toThrow(/duplicate page id/);
  });
  it("shardPrefix < 1 throws", () => {
    expect(() => buildSearchIndex([page("/p", "a")], { shardPrefix: 0 })).toThrow(
      /shardPrefix/,
    );
  });
  it("shardPrefix NaN throws (no silent degraded-shard behaviour)", () => {
    expect(() => buildSearchIndex([page("/p", "a")], { shardPrefix: NaN })).toThrow(
      /shardPrefix/,
    );
  });
  it("shardPrefix Infinity throws", () => {
    expect(() => buildSearchIndex([page("/p", "a")], { shardPrefix: Infinity })).toThrow(
      /shardPrefix/,
    );
  });
  it("shardPrefix fractional throws", () => {
    expect(() => buildSearchIndex([page("/p", "a")], { shardPrefix: 1.5 })).toThrow(
      /shardPrefix/,
    );
  });
  it("maxPages NaN throws (would otherwise silently disable the cap)", () => {
    expect(() => buildSearchIndex([page("/p", "a")], { maxPages: NaN })).toThrow(
      /maxPages/,
    );
  });
  it("maxPages negative throws", () => {
    expect(() => buildSearchIndex([page("/p", "a")], { maxPages: -1 })).toThrow(
      /maxPages/,
    );
  });
  it("maxTokensPerPage NaN throws", () => {
    expect(() =>
      buildSearchIndex([page("/p", "a")], { maxTokensPerPage: NaN }),
    ).toThrow(/maxTokensPerPage/);
  });
  it("maxPostingsPerToken NaN throws", () => {
    expect(() =>
      buildSearchIndex([page("/p", "a")], { maxPostingsPerToken: NaN }),
    ).toThrow(/maxPostingsPerToken/);
  });
});

describe("buildSearchIndex — stats accuracy under caps", () => {
  it("totalTokens does NOT over-count postings dropped by maxPostingsPerToken", () => {
    // 5 pages each containing the SAME token.  Cap postings
    // per token to 2.  Pages 3-5 contribute postings that get
    // dropped — and those occurrences should NOT count toward
    // stats.totalTokens.
    const pages = Array.from({ length: 5 }, (_, i) => page(`/p${i}`, "common"));
    const out = buildSearchIndex(pages, { maxPostingsPerToken: 2 });
    // Only 2 postings accepted, so totalTokens should be 2,
    // NOT 5.
    expect(out.manifest.stats.totalTokens).toBe(2);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Manifest
// ─────────────────────────────────────────────────────────────────────

describe("buildSearchIndex — manifest", () => {
  it("pages list is sorted", () => {
    const out = buildSearchIndex([
      page("/z", "a"),
      page("/a", "a"),
      page("/m", "a"),
    ]);
    expect(out.manifest.pages).toEqual(["/a", "/m", "/z"]);
  });
  it("shardKeys list is sorted", () => {
    const out = buildSearchIndex([
      page("/p", "zebra apple mango"),
    ]);
    const keys = [...out.manifest.shardKeys];
    expect(keys).toEqual([...keys].sort());
  });
  it("stats accurate", () => {
    const out = buildSearchIndex([
      page("/p1", "alpha beta alpha"),
      page("/p2", "beta gamma"),
    ], { filterStopWords: false, stem: false });
    // tokens: alpha, beta, alpha, beta, gamma → 5 total
    // unique: alpha, beta, gamma → 3
    expect(out.manifest.stats.totalTokens).toBe(5);
    expect(out.manifest.stats.uniqueTokens).toBe(3);
    expect(out.manifest.stats.pageCount).toBe(2);
    expect(out.manifest.stats.shardCount).toBeGreaterThan(0);
  });
  it("records option flags in manifest", () => {
    const out = buildSearchIndex([page("/p", "test")], {
      filterStopWords: false,
      stem: false,
      shardPrefix: 3,
    });
    expect(out.manifest.filterStopWords).toBe(false);
    expect(out.manifest.stem).toBe(false);
    expect(out.manifest.shardPrefix).toBe(3);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Determinism + immutability
// ─────────────────────────────────────────────────────────────────────

describe("buildSearchIndex — determinism + immutability", () => {
  it("same input → identical output structure", () => {
    const pages: IndexPageInput[] = [
      page("/a", "alpha beta gamma"),
      page("/b", "beta gamma delta"),
    ];
    const a = buildSearchIndex(pages);
    const b = buildSearchIndex(pages);
    expect([...a.manifest.pages]).toEqual([...b.manifest.pages]);
    expect([...a.manifest.shardKeys]).toEqual([...b.manifest.shardKeys]);
    expect(a.manifest.stats).toEqual(b.manifest.stats);
  });
  it("does not mutate input pages array", () => {
    const pages = [page("/p", "hi")];
    const snapshot = JSON.stringify(pages);
    buildSearchIndex(pages);
    expect(JSON.stringify(pages)).toBe(snapshot);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Realistic
// ─────────────────────────────────────────────────────────────────────

describe("buildSearchIndex — realistic scenario", () => {
  it("typical 5-page docs site", () => {
    const out = buildSearchIndex([
      page("/intro", "Welcome to the documentation. Learn the basics.", "Introduction"),
      page("/guide/setup", "Install via npm install foo. Configure your environment.", "Setup Guide"),
      page("/guide/usage", "Common usage patterns and examples.", "Usage Guide"),
      page("/api/reference", "Full API reference for the foo library.", "API Reference"),
      page("/faq", "Frequently asked questions about foo.", "FAQ"),
    ]);
    expect(out.manifest.pages).toHaveLength(5);
    expect(out.manifest.stats.pageCount).toBe(5);
    expect(out.manifest.stats.uniqueTokens).toBeGreaterThan(10);
    expect(out.shards.size).toBeGreaterThan(0);
    // "foo" appears in three pages — should have a posting list of length 3.
    const shard = out.shards.get("fo")!;
    const postings = shard.postings.get("foo")!;
    expect(postings.length).toBe(3);
  });
});
