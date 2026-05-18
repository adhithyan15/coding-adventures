/**
 * cache.test.ts — `createIncrementalCache` + `sliceWithCache`.
 *
 * Covers: cache miss/hit, change-driven invalidation (doc /
 * usedRuleIds / activeContexts), order independence of array
 * inputs, manual clear, concurrent slices, warning propagation
 * across hits.
 */

import { describe, it, expect } from "vitest";
import {
  emptyStyleDocument, styleRuleId, sel,
  type StyleDocument,
} from "@coding-adventures/forme-style-ir";
import {
  defaultScopePrefix,
  type PageSlice,
} from "@coding-adventures/forme-aot-css-slicer";
import {
  createIncrementalCache, createMemoryCacheIO,
} from "../src/index.js";

// ─── Fixture ─────────────────────────────────────────────────────────────

function fixture(): StyleDocument {
  return {
    ...emptyStyleDocument(),
    tokens: {
      ...emptyStyleDocument().tokens,
      colors: { text: { kind: "rgb", r: 31, g: 35, b: 40 } },
    },
    rules: [
      {
        id: styleRuleId("body"),
        selector: sel.type("paragraph"),
        properties: [
          { kind: "color", value: { kind: "token-ref", path: "colors.text" } },
        ],
      },
      {
        id: styleRuleId("headline"),
        selector: { kind: "node-type-level", level: 1 },
        properties: [{ kind: "color", value: { kind: "named", name: "black" } }],
      },
      {
        id: styleRuleId("nav"),
        selector: { kind: "tag", tag: "nav" },
        properties: [{ kind: "color", value: { kind: "named", name: "tomato" } }],
      },
    ],
  };
}

const page = (id: string, ids: string[]): PageSlice => ({
  id,
  usedRuleIds: ids.map((s) => styleRuleId(s)),
});

// ─── Tests ───────────────────────────────────────────────────────────────

describe("sliceWithCache — miss → hit", () => {
  it("first call is a cache miss (cacheHit: false)", async () => {
    const cache = createIncrementalCache(createMemoryCacheIO());
    const r = await cache.sliceWithCache(fixture(), [page("/a.html", ["body"])], { activeContexts: [] });
    expect(r.artefacts.get("/a.html")!.cacheHit).toBe(false);
  });

  it("identical second call is a cache hit (cacheHit: true)", async () => {
    const cache = createIncrementalCache(createMemoryCacheIO());
    const pages = [page("/a.html", ["body"])];
    const r1 = await cache.sliceWithCache(fixture(), pages, { activeContexts: [] });
    const r2 = await cache.sliceWithCache(fixture(), pages, { activeContexts: [] });
    expect(r1.artefacts.get("/a.html")!.cacheHit).toBe(false);
    expect(r2.artefacts.get("/a.html")!.cacheHit).toBe(true);
  });

  it("cache hit produces byte-identical CSS to a fresh compute", async () => {
    const cache = createIncrementalCache(createMemoryCacheIO());
    const pages = [page("/a.html", ["body"])];
    const r1 = await cache.sliceWithCache(fixture(), pages, { activeContexts: [] });
    const r2 = await cache.sliceWithCache(fixture(), pages, { activeContexts: [] });
    expect(r1.artefacts.get("/a.html")!.css).toBe(r2.artefacts.get("/a.html")!.css);
  });

  it("emittedRules and sha256 survive cache round-trip", async () => {
    const cache = createIncrementalCache(createMemoryCacheIO());
    const pages = [page("/a.html", ["body", "headline"])];
    const r1 = await cache.sliceWithCache(fixture(), pages, { activeContexts: [] });
    const r2 = await cache.sliceWithCache(fixture(), pages, { activeContexts: [] });
    const a1 = r1.artefacts.get("/a.html")!;
    const a2 = r2.artefacts.get("/a.html")!;
    expect([...a2.emittedRules].sort()).toEqual([...a1.emittedRules].sort());
    expect(a2.sha256).toBe(a1.sha256);
  });
});

describe("sliceWithCache — change-driven invalidation", () => {
  it("changing usedRuleIds invalidates cache (different cacheKey)", async () => {
    const cache = createIncrementalCache(createMemoryCacheIO());
    const r1 = await cache.sliceWithCache(fixture(), [page("/a.html", ["body"])], { activeContexts: [] });
    const r2 = await cache.sliceWithCache(fixture(), [page("/a.html", ["body", "headline"])], { activeContexts: [] });
    expect(r1.artefacts.get("/a.html")!.cacheKey)
      .not.toBe(r2.artefacts.get("/a.html")!.cacheKey);
    expect(r2.artefacts.get("/a.html")!.cacheHit).toBe(false);
  });

  it("changing activeContexts invalidates cache", async () => {
    const cache = createIncrementalCache(createMemoryCacheIO());
    const r1 = await cache.sliceWithCache(fixture(), [page("/a.html", ["body"])], { activeContexts: ["screen"] });
    const r2 = await cache.sliceWithCache(fixture(), [page("/a.html", ["body"])], { activeContexts: ["print"] });
    expect(r1.artefacts.get("/a.html")!.cacheKey)
      .not.toBe(r2.artefacts.get("/a.html")!.cacheKey);
  });

  it("changing the document invalidates cache (canonical-aware)", async () => {
    const cache = createIncrementalCache(createMemoryCacheIO());
    const docA = fixture();
    const docB: StyleDocument = {
      ...docA,
      tokens: {
        ...docA.tokens,
        colors: { text: { kind: "rgb", r: 99, g: 99, b: 99 } },
      },
    };
    const k1 = cache.cacheKey(docA, [styleRuleId("body")], []);
    const k2 = cache.cacheKey(docB, [styleRuleId("body")], []);
    expect(k1).not.toBe(k2);
  });

  it("doc with reshuffled token keys produces the SAME cache key (canonical = order-independent)", async () => {
    const cache = createIncrementalCache(createMemoryCacheIO());
    const docA: StyleDocument = {
      ...fixture(),
      tokens: {
        ...fixture().tokens,
        colors: {
          alpha: { kind: "named", name: "red" },
          beta:  { kind: "named", name: "blue" },
        },
      },
    };
    const docB: StyleDocument = {
      ...fixture(),
      tokens: {
        ...fixture().tokens,
        colors: {
          beta:  { kind: "named", name: "blue" },
          alpha: { kind: "named", name: "red" },
        },
      },
    };
    expect(cache.cacheKey(docA, [styleRuleId("body")], []))
      .toBe(cache.cacheKey(docB, [styleRuleId("body")], []));
  });
});

describe("sliceWithCache — order-independence of array inputs", () => {
  it("usedRuleIds in different order → same cache key", async () => {
    const cache = createIncrementalCache(createMemoryCacheIO());
    const k1 = cache.cacheKey(fixture(), [styleRuleId("body"), styleRuleId("headline")], []);
    const k2 = cache.cacheKey(fixture(), [styleRuleId("headline"), styleRuleId("body")], []);
    expect(k1).toBe(k2);
  });

  it("activeContexts in different order → same cache key", () => {
    const cache = createIncrementalCache(createMemoryCacheIO());
    const k1 = cache.cacheKey(fixture(), [], ["print", "screen", "dark"]);
    const k2 = cache.cacheKey(fixture(), [], ["dark", "screen", "print"]);
    expect(k1).toBe(k2);
  });

  it("end-to-end: reshuffled inputs still hit cache on second call", async () => {
    const cache = createIncrementalCache(createMemoryCacheIO());
    const r1 = await cache.sliceWithCache(
      fixture(),
      [page("/a.html", ["body", "headline"])],
      { activeContexts: ["print", "screen"] },
    );
    const r2 = await cache.sliceWithCache(
      fixture(),
      [page("/a.html", ["headline", "body"])],
      { activeContexts: ["screen", "print"] },
    );
    expect(r1.artefacts.get("/a.html")!.cacheHit).toBe(false);
    expect(r2.artefacts.get("/a.html")!.cacheHit).toBe(true);
  });
});

describe("sliceWithCache — same key → one cache entry across pages", () => {
  it("two pages with the same usedRuleIds share one cache entry but get different scopes", async () => {
    const io = createMemoryCacheIO();
    const cache = createIncrementalCache(io);
    const pages = [
      page("/a.html", ["body"]),
      page("/b.html", ["body"]),
    ];
    const r = await cache.sliceWithCache(fixture(), pages, { activeContexts: [] });
    // Only ONE cache key was needed.
    expect(io.size()).toBe(1);
    // But each page got its own scoped CSS deliverable.
    const aCss = r.artefacts.get("/a.html")!.css;
    const bCss = r.artefacts.get("/b.html")!.css;
    expect(aCss).toContain(defaultScopePrefix("/a.html"));
    expect(bCss).toContain(defaultScopePrefix("/b.html"));
    expect(aCss).not.toBe(bCss);
    // Page B was processed AFTER page A in the same call —
    // confirm it observed the cache hit set by A.
    expect(r.artefacts.get("/b.html")!.cacheHit).toBe(true);
    expect(r.artefacts.get("/a.html")!.cacheHit).toBe(false);
  });
});

describe("sliceWithCache — manual clear", () => {
  it("clearing the in-memory IO forces fresh recompute on next call", async () => {
    const io = createMemoryCacheIO();
    const cache = createIncrementalCache(io);
    const pages = [page("/a.html", ["body"])];
    await cache.sliceWithCache(fixture(), pages, { activeContexts: [] });
    expect(io.size()).toBe(1);
    io.clear();
    expect(io.size()).toBe(0);
    const r2 = await cache.sliceWithCache(fixture(), pages, { activeContexts: [] });
    expect(r2.artefacts.get("/a.html")!.cacheHit).toBe(false);
  });

  it("CacheIO.list returns every stored key, sorted, frozen", async () => {
    const io = createMemoryCacheIO();
    const cache = createIncrementalCache(io);
    await cache.sliceWithCache(
      fixture(),
      [page("/a.html", ["body"]), page("/b.html", ["headline"])],
      { activeContexts: [] },
    );
    const keys = await io.list();
    expect(keys.length).toBe(2);
    expect(Object.isFrozen(keys)).toBe(true);
    expect([...keys]).toEqual([...keys].sort());
  });
});

describe("sliceWithCache — concurrency", () => {
  it("concurrent slices for the same input share the cache (no double-compute on hits)", async () => {
    // Note: the cache implementation is NOT internally locked
    // (CacheIO is async; concurrent get/put can race for the same
    // key on the first call).  Behaviour: both calls may CACHE-MISS,
    // both will put — the second put overwrites with identical
    // content (idempotent).  Subsequent calls hit.
    const io = createMemoryCacheIO();
    const cache = createIncrementalCache(io);
    const pages = [page("/a.html", ["body"])];
    const opts = { activeContexts: [] };

    await Promise.all([
      cache.sliceWithCache(fixture(), pages, opts),
      cache.sliceWithCache(fixture(), pages, opts),
    ]);
    // After the dust settles there's still only one key.
    expect(io.size()).toBe(1);

    // And the third call is a hit.
    const r3 = await cache.sliceWithCache(fixture(), pages, opts);
    expect(r3.artefacts.get("/a.html")!.cacheHit).toBe(true);
  });
});

describe("sliceWithCache — warnings propagate identically on hit", () => {
  it("a rule that emits a warning on miss emits the same warning on hit", async () => {
    const docWithUnresolved: StyleDocument = {
      ...fixture(),
      rules: [
        ...fixture().rules,
        {
          id: styleRuleId("bad"),
          selector: sel.type("p"),
          properties: [
            { kind: "color", value: { kind: "token-ref", path: "colors.nope" } },
          ],
        },
      ],
    };
    const cache = createIncrementalCache(createMemoryCacheIO());
    const pages = [page("/x.html", ["bad"])];
    const r1 = await cache.sliceWithCache(docWithUnresolved, pages, { activeContexts: [] });
    const r2 = await cache.sliceWithCache(docWithUnresolved, pages, { activeContexts: [] });
    expect(r2.artefacts.get("/x.html")!.warnings.length).toBe(r1.artefacts.get("/x.html")!.warnings.length);
    expect(r2.artefacts.get("/x.html")!.warnings.length).toBeGreaterThan(0);
  });
});

describe("sliceWithCache — robustness", () => {
  it("malformed cache entry falls back to fresh compute (defensive)", async () => {
    // Pre-poison the cache with garbage at the key we're about to look up.
    const io = createMemoryCacheIO();
    const cache = createIncrementalCache(io);
    const pages = [page("/a.html", ["body"])];
    const key = cache.cacheKey(fixture(), pages[0].usedRuleIds, []);
    await io.put(key, "{ not valid cache entry }}", { pageId: "x", byteSize: 0, sha256: "x" });
    const r = await cache.sliceWithCache(fixture(), pages, { activeContexts: [] });
    // Should NOT throw — should fall through to fresh compute.
    expect(r.artefacts.get("/a.html")!.css.length).toBeGreaterThan(0);
    // The cacheHit should be `false` because the malformed entry was skipped.
    expect(r.artefacts.get("/a.html")!.cacheHit).toBe(false);
  });

  it("malformed JSON cache entry (not JSON at all) falls back to fresh compute", async () => {
    const io = createMemoryCacheIO();
    const cache = createIncrementalCache(io);
    const pages = [page("/a.html", ["body"])];
    const key = cache.cacheKey(fixture(), pages[0].usedRuleIds, []);
    await io.put(key, "not json at all", { pageId: "x", byteSize: 0, sha256: "x" });
    const r = await cache.sliceWithCache(fixture(), pages, { activeContexts: [] });
    expect(r.artefacts.get("/a.html")!.cacheHit).toBe(false);
  });

  it("byteSize on hit equals byteSize on miss (re-scoped CSS is identical)", async () => {
    const cache = createIncrementalCache(createMemoryCacheIO());
    const pages = [page("/a.html", ["body"])];
    const r1 = await cache.sliceWithCache(fixture(), pages, { activeContexts: [] });
    const r2 = await cache.sliceWithCache(fixture(), pages, { activeContexts: [] });
    expect(r2.artefacts.get("/a.html")!.byteSize).toBe(r1.artefacts.get("/a.html")!.byteSize);
  });
});

describe("sliceWithCache — page iteration order preserved", () => {
  it("artefacts Map iteration matches input pages order", async () => {
    const cache = createIncrementalCache(createMemoryCacheIO());
    const pages = [
      page("/c.html", ["body"]),
      page("/a.html", ["body"]),
      page("/b.html", ["body"]),
    ];
    const r = await cache.sliceWithCache(fixture(), pages, { activeContexts: [] });
    expect([...r.artefacts.keys()]).toEqual(["/c.html", "/a.html", "/b.html"]);
  });
});
