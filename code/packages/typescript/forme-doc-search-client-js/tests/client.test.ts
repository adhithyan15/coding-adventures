/**
 * client.test.ts — SearchClient integration tests.
 *
 * Builds a small in-memory index via forme-doc-search-index-builder
 * to get a realistic manifest + shards; wires that into the
 * SearchClient via an in-memory fetcher.
 */

import { describe, it, expect, vi } from "vitest";
import { SearchClient } from "../src/index.js";
import type { ShardFetcher } from "../src/index.js";
import { buildSearchIndex } from "@coding-adventures/forme-doc-search-index-builder";
import type {
  IndexShard,
  IndexPageInput,
  IndexManifest,
} from "@coding-adventures/forme-doc-search-index-builder";

// ─────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────

/** Build an index from a small fixture corpus. */
function buildFixture(pages: IndexPageInput[]) {
  const out = buildSearchIndex(pages);
  return {
    manifest: out.manifest,
    shards: out.shards,
    fetchShard: createMemFetcher(out.shards),
  };
}

/** In-memory fetcher: shard map → ShardFetcher. */
function createMemFetcher(
  shards: ReadonlyMap<string, IndexShard>,
): ShardFetcher {
  return async (shardKey: string) => {
    const shard = shards.get(shardKey);
    if (shard === undefined) {
      throw new Error(`shard not found: ${shardKey}`);
    }
    return shard;
  };
}

// ─────────────────────────────────────────────────────────────────────
// Construction + validation
// ─────────────────────────────────────────────────────────────────────

describe("SearchClient — construction", () => {
  it("constructs from a valid manifest + fetcher", () => {
    const f = buildFixture([{ id: "/p", body: "hello" }]);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    expect(client.cacheSize).toBe(0);
  });
  it("maxCachedShards < 1 throws", () => {
    const f = buildFixture([{ id: "/p", body: "hello" }]);
    expect(
      () => new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard, maxCachedShards: 0 }),
    ).toThrow(/maxCachedShards/);
  });
  it("maxCachedShards NaN throws", () => {
    const f = buildFixture([{ id: "/p", body: "hello" }]);
    expect(
      () => new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard, maxCachedShards: NaN }),
    ).toThrow(/maxCachedShards/);
  });
  it("titleBoost negative throws", () => {
    const f = buildFixture([{ id: "/p", body: "hello" }]);
    expect(
      () => new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard, titleBoost: -1 }),
    ).toThrow(/titleBoost/);
  });
  it("titleBoost NaN throws", () => {
    const f = buildFixture([{ id: "/p", body: "hello" }]);
    expect(
      () => new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard, titleBoost: NaN }),
    ).toThrow(/titleBoost/);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Basic search
// ─────────────────────────────────────────────────────────────────────

describe("SearchClient — basic search", () => {
  it("returns matching page", async () => {
    const f = buildFixture([
      { id: "/p1", body: "install foo" },
      { id: "/p2", body: "configure foo" },
    ]);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    const results = await client.search("install");
    expect(results.length).toBe(1);
    expect(results[0].pageId).toBe("/p1");
  });
  it("scores titleHit higher (titleBoost=2 default)", async () => {
    const f = buildFixture([
      { id: "/p1", body: "install install" }, // body-only, freq=2
      { id: "/p2", body: "install", title: "Install" }, // title hit, freq=2 * boost
    ]);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    const results = await client.search("install");
    // /p2: titleHit + freq=2 → score 4
    // /p1: body-only + freq=2 → score 2
    expect(results[0].pageId).toBe("/p2");
    expect(results[0].score).toBeGreaterThan(results[1].score);
  });
  it("multiple query tokens accumulate score", async () => {
    const f = buildFixture([
      { id: "/p1", body: "install configure" }, // matches both
      { id: "/p2", body: "install only" },       // matches one
    ]);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    const results = await client.search("install configure");
    expect(results[0].pageId).toBe("/p1");
    expect(results[0].matchedTokens.length).toBeGreaterThan(results[1].matchedTokens.length);
  });
  it("returns matchedTokens for each result", async () => {
    const f = buildFixture([{ id: "/p1", body: "install configure" }]);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    const results = await client.search("install configure");
    // After stemming: "instal", "configur"
    expect(results[0].matchedTokens.sort()).toEqual(["configur", "instal"]);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Empty / no-match cases
// ─────────────────────────────────────────────────────────────────────

describe("SearchClient — empty results", () => {
  it("empty query → []", async () => {
    const f = buildFixture([{ id: "/p", body: "hello" }]);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    expect(await client.search("")).toEqual([]);
  });
  it("all-stop-words query → []", async () => {
    const f = buildFixture([{ id: "/p", body: "hello" }]);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    // Default index has filterStopWords=true; "the and of" all filter out.
    expect(await client.search("the and of")).toEqual([]);
  });
  it("query matching no shards → []", async () => {
    const f = buildFixture([{ id: "/p", body: "alpha" }]);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    // "zzzzz" doesn't share shard with anything in the index
    expect(await client.search("zzzzz")).toEqual([]);
  });
  it("query matching shard but no token → []", async () => {
    const f = buildFixture([{ id: "/p", body: "install" }]);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    // "infinite" → "in" shard (matches), but "infinit" not in postings
    expect(await client.search("infinite")).toEqual([]);
  });
});

// ─────────────────────────────────────────────────────────────────────
// limit option
// ─────────────────────────────────────────────────────────────────────

describe("SearchClient — limit option", () => {
  it("default limit is 20", async () => {
    const pages = Array.from({ length: 30 }, (_, i) => ({
      id: `/p${i}`,
      body: "common",
    }));
    const f = buildFixture(pages);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    const results = await client.search("common");
    expect(results.length).toBe(20);
  });
  it("custom limit honoured", async () => {
    const pages = Array.from({ length: 30 }, (_, i) => ({
      id: `/p${i}`,
      body: "common",
    }));
    const f = buildFixture(pages);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    const results = await client.search("common", { limit: 5 });
    expect(results.length).toBe(5);
  });
  it("limit < 0 throws", async () => {
    const f = buildFixture([{ id: "/p", body: "x" }]);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    await expect(client.search("x", { limit: -1 })).rejects.toThrow(/limit/);
  });
  it("limit NaN throws", async () => {
    const f = buildFixture([{ id: "/p", body: "x" }]);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    await expect(client.search("x", { limit: NaN })).rejects.toThrow(/limit/);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Caching
// ─────────────────────────────────────────────────────────────────────

describe("SearchClient — shard caching", () => {
  it("caches fetched shards", async () => {
    const f = buildFixture([{ id: "/p", body: "install" }]);
    const fetchSpy = vi.fn(f.fetchShard);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: fetchSpy });
    await client.search("install");
    const beforeCount = fetchSpy.mock.calls.length;
    expect(beforeCount).toBe(1);
    // Second search for same token should NOT re-fetch.
    await client.search("install");
    expect(fetchSpy.mock.calls.length).toBe(beforeCount);
  });
  it("LRU evicts when cap exceeded", async () => {
    // Build an index with 5+ distinct shards (default prefix=2).
    const f = buildFixture([
      { id: "/p1", body: "alpha bravo charlie delta echo foxtrot" },
      { id: "/p2", body: "alpha bravo charlie delta echo foxtrot" },
    ]);
    const client = new SearchClient({
      manifest: f.manifest,
      fetchShard: f.fetchShard,
      maxCachedShards: 2,
    });
    await client.search("alpha bravo charlie delta echo foxtrot");
    expect(client.cacheSize).toBe(2);
  });
  it("clearCache empties the cache", async () => {
    const f = buildFixture([{ id: "/p", body: "install" }]);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    await client.search("install");
    expect(client.cacheSize).toBeGreaterThan(0);
    client.clearCache();
    expect(client.cacheSize).toBe(0);
  });
  it("prefetchShards warms the cache", async () => {
    const f = buildFixture([
      { id: "/p", body: "install configure deploy" },
    ]);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    await client.prefetchShards(f.manifest.shardKeys);
    expect(client.cacheSize).toBe(f.manifest.shardKeys.length);
  });
  it("prefetchShards silently skips unknown shard keys", async () => {
    const f = buildFixture([{ id: "/p", body: "install" }]);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    await client.prefetchShards(["zz", "xx", "yy"]);
    expect(client.cacheSize).toBe(0);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Inflight deduplication
// ─────────────────────────────────────────────────────────────────────

describe("SearchClient — inflight deduplication", () => {
  it("concurrent searches share in-flight shard fetches", async () => {
    const f = buildFixture([{ id: "/p", body: "install" }]);
    const fetchSpy = vi.fn(async (key: string) => {
      // Simulate latency.
      await new Promise((r) => setTimeout(r, 5));
      const shard = f.shards.get(key);
      if (shard === undefined) throw new Error(`shard not found: ${key}`);
      return shard;
    });
    const client = new SearchClient({ manifest: f.manifest, fetchShard: fetchSpy });
    // Fire two concurrent searches before the first resolves.
    const [r1, r2] = await Promise.all([
      client.search("install"),
      client.search("install"),
    ]);
    expect(fetchSpy.mock.calls.length).toBe(1); // dedupe worked
    expect(r1).toEqual(r2);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Failure handling
// ─────────────────────────────────────────────────────────────────────

describe("SearchClient — degrade gracefully on fetch failure", () => {
  it("rejecting fetcher does NOT crash search", async () => {
    const f = buildFixture([{ id: "/p", body: "install" }]);
    const failingFetcher: ShardFetcher = async () => {
      throw new Error("network fail");
    };
    const client = new SearchClient({ manifest: f.manifest, fetchShard: failingFetcher });
    const results = await client.search("install");
    expect(results).toEqual([]); // no results, but no exception
  });
  it("malformed shard from fetcher is treated as missing", async () => {
    const f = buildFixture([{ id: "/p", body: "install" }]);
    const badFetcher: ShardFetcher = (async () =>
      ({ wrong: "shape" } as unknown)) as ShardFetcher;
    const client = new SearchClient({ manifest: f.manifest, fetchShard: badFetcher });
    const results = await client.search("install");
    expect(results).toEqual([]);
  });
  it("partial failure: one shard fails, others succeed → partial results", async () => {
    const f = buildFixture([
      { id: "/p1", body: "install" },     // "in" shard
      { id: "/p2", body: "configure" },   // "co" shard
    ]);
    // Fail only the "in" shard.
    const partialFetcher: ShardFetcher = async (key) => {
      if (key === "in") throw new Error("fail");
      const shard = f.shards.get(key);
      if (shard === undefined) throw new Error(`not found: ${key}`);
      return shard;
    };
    const client = new SearchClient({ manifest: f.manifest, fetchShard: partialFetcher });
    const results = await client.search("install configure");
    // Only /p2 should match (the "co" shard succeeded).
    expect(results.length).toBe(1);
    expect(results[0].pageId).toBe("/p2");
  });
});

// ─────────────────────────────────────────────────────────────────────
// Ranking
// ─────────────────────────────────────────────────────────────────────

describe("SearchClient — ranking", () => {
  it("higher freq → higher score", async () => {
    const f = buildFixture([
      { id: "/p1", body: "install" },
      { id: "/p2", body: "install install install" },
    ]);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    const results = await client.search("install");
    expect(results[0].pageId).toBe("/p2");
    expect(results[0].score).toBeGreaterThan(results[1].score);
  });
  it("custom titleBoost overrides default", async () => {
    const f = buildFixture([
      { id: "/p1", body: "install", title: "Install" },
      { id: "/p2", body: "install install install install install" },
    ]);
    // With titleBoost=10, /p1's score = 2 * 10 = 20 (title freq=1, body freq=1)
    // /p2's score = 5
    const client = new SearchClient({
      manifest: f.manifest,
      fetchShard: f.fetchShard,
      titleBoost: 10,
    });
    const results = await client.search("install");
    expect(results[0].pageId).toBe("/p1");
  });
  it("tied scores sort alphabetically by pageId", async () => {
    const f = buildFixture([
      { id: "/zebra", body: "install" },
      { id: "/apple", body: "install" },
    ]);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    const results = await client.search("install");
    expect(results[0].pageId).toBe("/apple");
    expect(results[1].pageId).toBe("/zebra");
  });
});

// ─────────────────────────────────────────────────────────────────────
// Stop-word & stemmer consistency
// ─────────────────────────────────────────────────────────────────────

describe("SearchClient — index/query option consistency", () => {
  it("uses manifest's filterStopWords flag", async () => {
    // Build index without stop-word filter; "the" gets indexed.
    const out = buildSearchIndex([{ id: "/p", body: "the answer" }], {
      filterStopWords: false,
    });
    const client = new SearchClient({
      manifest: out.manifest,
      fetchShard: createMemFetcher(out.shards),
    });
    // Client should ALSO not filter stop words (per manifest).
    const results = await client.search("the");
    expect(results.length).toBe(1);
  });
  it("uses manifest's stem flag", async () => {
    // Build index without stemming.  "installing" stays as-is.
    const out = buildSearchIndex([{ id: "/p", body: "installing" }], {
      stem: false,
    });
    const client = new SearchClient({
      manifest: out.manifest,
      fetchShard: createMemFetcher(out.shards),
    });
    // Query "installing" should match (no stemming on either side).
    const r1 = await client.search("installing");
    expect(r1.length).toBe(1);
    // Query "install" should NOT match (different tokens).
    const r2 = await client.search("install");
    expect(r2.length).toBe(0);
  });
});

// ─────────────────────────────────────────────────────────────────────
// Realistic
// ─────────────────────────────────────────────────────────────────────

describe("SearchClient — realistic", () => {
  it("typical docs query", async () => {
    const f = buildFixture([
      { id: "/intro", body: "Welcome to the documentation. Learn the basics.", title: "Introduction" },
      { id: "/guide/setup", body: "Install via npm install foo. Configure your environment.", title: "Setup Guide" },
      { id: "/guide/usage", body: "Common usage patterns and examples.", title: "Usage Guide" },
      { id: "/api/reference", body: "Full API reference for the foo library.", title: "API Reference" },
      { id: "/faq", body: "Frequently asked questions about foo.", title: "FAQ" },
    ]);
    const client = new SearchClient({ manifest: f.manifest, fetchShard: f.fetchShard });
    const results = await client.search("install");
    // /guide/setup mentions "install" twice (body) — should win.
    expect(results[0].pageId).toBe("/guide/setup");
  });
});
