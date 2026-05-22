/**
 * emitter.test.ts — tests for forme-doc-site-emitter.
 *
 * Coverage strategy:
 *   - Happy paths for every composable input slice (pages,
 *     sidebar, search, extras, baseUrl).
 *   - Every validation throw — one test per failure mode so a
 *     regression points at exactly what slipped.
 *   - Determinism — emit twice, assert byte-identical bundle JSON.
 *   - End-to-end realism — feed the result through the actual
 *     downstream `generatePageBundle` to confirm round-trip
 *     compatibility (the manifest the deploy runner will see is
 *     well-formed).
 */

import { describe, it, expect } from "vitest";
import { generatePageBundle } from "@coding-adventures/forme-aot-page-bundle-emitter";
import {
  emitSite,
  DEFAULT_SIDEBAR_PATH,
  DEFAULT_SEARCH_BASE_PATH,
  DEFAULT_MAX_PAGES,
  DEFAULT_MAX_SHARDS,
  DEFAULT_MAX_EXTRAS,
  CONTENT_TYPE_JSON,
  CONTENT_TYPE_JS,
} from "../src/index.js";
import type {
  DocPage,
  SearchAssets,
  ExtraFile,
  SiteEmitInput,
} from "../src/index.js";

// -- Test fixtures ------------------------------------------------

const PAGE_A: DocPage = { route: "/", html: "<html>A</html>" };
const PAGE_B: DocPage = { route: "/about", html: "<html>B</html>", lastmod: "2026-05-22" };

const SIDEBAR = [
  { kind: "page" as const, label: "Home", path: "index.md", position: 1 },
];

function makeShard(key: string, tokens: Record<string, Array<{ pageId: string; freq: number; titleHit: boolean }>>) {
  const postings = new Map<string, Array<{ pageId: string; freq: number; titleHit: boolean }>>();
  for (const [tok, posts] of Object.entries(tokens)) postings.set(tok, posts);
  return { shardKey: key, postings };
}

function makeSearch(): SearchAssets {
  return {
    manifest: {
      pages: ["/", "/about"],
      shardKeys: ["a", "b"],
      shardPrefix: 1,
      filterStopWords: true,
      stem: true,
      stats: { totalTokens: 4, uniqueTokens: 2, pageCount: 2, shardCount: 2 },
    },
    shards: new Map([
      ["a", makeShard("a", { apple: [{ pageId: "/", freq: 2, titleHit: false }] })],
      ["b", makeShard("b", { banana: [{ pageId: "/about", freq: 1, titleHit: true }] })],
    ]),
  };
}

// =====================================================================
// 1. Construction / input-shape validation
// =====================================================================

describe("emitSite — input shape", () => {
  it("throws on null/undefined input", () => {
    expect(() => emitSite(null as unknown as SiteEmitInput)).toThrow(TypeError);
    expect(() => emitSite("hi" as unknown as SiteEmitInput)).toThrow(TypeError);
  });

  it("throws when pages is not an array", () => {
    expect(() => emitSite({ pages: "x" } as unknown as SiteEmitInput)).toThrow(/pages must be an array/);
  });

  it("throws when a page is not an object", () => {
    expect(() => emitSite({ pages: [null] } as unknown as SiteEmitInput)).toThrow(/pages\[0\] must be an object/);
  });

  it("throws when page.html is not a string", () => {
    expect(() => emitSite({ pages: [{ route: "/", html: 5 } as unknown as DocPage] })).toThrow(/pages\[0\].html/);
  });

  it("throws when page.lastmod is not a string", () => {
    expect(() => emitSite({ pages: [{ route: "/", html: "h", lastmod: 5 } as unknown as DocPage] })).toThrow(/lastmod must be a string/);
  });

  it("throws when baseUrl is non-string", () => {
    expect(() =>
      emitSite({ pages: [PAGE_A], baseUrl: 5 } as unknown as SiteEmitInput),
    ).toThrow(/baseUrl must be a string/);
  });
});

// =====================================================================
// 2. Route validation
// =====================================================================

describe("emitSite — route shape", () => {
  it("rejects non-string route", () => {
    expect(() => emitSite({ pages: [{ route: 5, html: "x" } as unknown as DocPage] })).toThrow(/route must be a string/);
  });
  it("rejects empty route", () => {
    expect(() => emitSite({ pages: [{ route: "", html: "x" }] })).toThrow(/must start with "\/"/);
  });
  it("rejects route without leading slash", () => {
    expect(() => emitSite({ pages: [{ route: "about", html: "x" }] })).toThrow(/must start with "\/"/);
  });
  it("rejects backslash", () => {
    expect(() => emitSite({ pages: [{ route: "/foo\\bar", html: "x" }] })).toThrow(/must not contain "\\"/);
  });
  it("rejects // protocol-relative hint", () => {
    expect(() => emitSite({ pages: [{ route: "//evil.example.com/path", html: "x" }] })).toThrow(/must not contain "\/\/"/);
  });
  it("rejects .. segment", () => {
    expect(() => emitSite({ pages: [{ route: "/foo/../bar", html: "x" }] })).toThrow(/must not contain "\.\." segment/);
  });
  it("rejects control chars", () => {
    expect(() => emitSite({ pages: [{ route: "/foo\x01bar", html: "x" }] })).toThrow(/must not contain control chars/);
  });
  it("rejects DEL char (0x7f)", () => {
    expect(() => emitSite({ pages: [{ route: "/foo\x7fbar", html: "x" }] })).toThrow(/must not contain control chars/);
  });
  it("rejects routes longer than 8192 chars", () => {
    const longRoute = "/" + "a".repeat(8192);
    expect(() => emitSite({ pages: [{ route: longRoute, html: "x" }] })).toThrow(/exceeds 8192 chars/);
  });
  it("accepts a route with a dot in the middle (e.g. /file.html)", () => {
    const bundle = emitSite({ pages: [{ route: "/file.html", html: "x" }] });
    expect(bundle.pages[0]!.route).toBe("/file.html");
  });
  it("accepts a single-segment dot (not ..) like /. notation", () => {
    // ".foo" is a single segment that starts with a dot but isn't "..", should pass.
    const bundle = emitSite({ pages: [{ route: "/.dotted", html: "x" }] });
    expect(bundle.pages[0]!.route).toBe("/.dotted");
  });
});

// =====================================================================
// 3. Numeric option validation
// =====================================================================

describe("emitSite — numeric options", () => {
  it("rejects NaN maxPages", () => {
    expect(() => emitSite({ pages: [], maxPages: NaN })).toThrow(/maxPages/);
  });
  it("rejects Infinity maxPages", () => {
    expect(() => emitSite({ pages: [], maxPages: Infinity })).toThrow(/maxPages/);
  });
  it("rejects negative maxPages", () => {
    expect(() => emitSite({ pages: [], maxPages: -1 })).toThrow(/maxPages/);
  });
  it("rejects non-integer maxPages", () => {
    expect(() => emitSite({ pages: [], maxPages: 2.5 })).toThrow(/maxPages/);
  });
  it("rejects non-integer maxShards", () => {
    expect(() => emitSite({ pages: [], maxShards: 2.5 })).toThrow(/maxShards/);
  });
  it("rejects non-integer maxExtras", () => {
    expect(() => emitSite({ pages: [], maxExtras: 2.5 })).toThrow(/maxExtras/);
  });
  it("rejects when pages exceeds maxPages", () => {
    expect(() => emitSite({ pages: [PAGE_A, PAGE_B], maxPages: 1 })).toThrow(/exceeds maxPages=1/);
  });
  it("rejects when extras exceeds maxExtras", () => {
    const xs: ExtraFile[] = [
      { route: "/a.txt", content: "1", contentType: "text/plain" },
      { route: "/b.txt", content: "2", contentType: "text/plain" },
    ];
    expect(() => emitSite({ pages: [], extras: xs, maxExtras: 1 })).toThrow(/exceeds maxExtras=1/);
  });
  it("accepts maxPages: 0 with an empty pages list (legitimate assertion)", () => {
    const bundle = emitSite({ pages: [], maxPages: 0 });
    expect(bundle.pages).toEqual([]);
  });
});

// =====================================================================
// 4. Happy-path pages
// =====================================================================

describe("emitSite — pages", () => {
  it("emits a single page as a PageEntry", () => {
    const bundle = emitSite({ pages: [PAGE_A] });
    expect(bundle.pages).toEqual([{ route: "/", html: "<html>A</html>" }]);
    expect(bundle.baseUrl).toBeUndefined();
  });

  it("preserves lastmod when present", () => {
    const bundle = emitSite({ pages: [PAGE_B] });
    expect(bundle.pages[0]).toEqual({ route: "/about", html: "<html>B</html>", lastmod: "2026-05-22" });
  });

  it("emits multiple pages preserving input order", () => {
    const bundle = emitSite({ pages: [PAGE_B, PAGE_A] });
    expect(bundle.pages.map((p) => p.route)).toEqual(["/about", "/"]);
  });

  it("forwards baseUrl unchanged", () => {
    const bundle = emitSite({ pages: [PAGE_A], baseUrl: "https://docs.example.com" });
    expect(bundle.baseUrl).toBe("https://docs.example.com");
  });

  it("rejects duplicate routes among pages", () => {
    expect(() =>
      emitSite({ pages: [PAGE_A, { route: "/", html: "<html>A2</html>" }] }),
    ).toThrow(/duplicate route "\/"/);
  });
});

// =====================================================================
// 5. Sidebar
// =====================================================================

describe("emitSite — sidebar", () => {
  it("emits sidebar at the default path with JSON content-type", () => {
    const bundle = emitSite({ pages: [PAGE_A], sidebar: SIDEBAR });
    const sidebarEntry = bundle.pages.find((p) => p.route === DEFAULT_SIDEBAR_PATH)!;
    expect(sidebarEntry).toBeDefined();
    expect(sidebarEntry.contentType).toBe(CONTENT_TYPE_JSON);
    expect(JSON.parse(sidebarEntry.html)).toEqual(SIDEBAR);
  });

  it("honours sidebarPath override", () => {
    const bundle = emitSite({ pages: [], sidebar: SIDEBAR, sidebarPath: "/nav.json" });
    expect(bundle.pages.find((p) => p.route === "/nav.json")).toBeDefined();
    expect(bundle.pages.find((p) => p.route === DEFAULT_SIDEBAR_PATH)).toBeUndefined();
  });

  it("throws when sidebar is not an array", () => {
    expect(() => emitSite({ pages: [], sidebar: "x" } as unknown as SiteEmitInput)).toThrow(/sidebar must be an array/);
  });

  it("throws when sidebarPath is invalid", () => {
    expect(() => emitSite({ pages: [], sidebar: SIDEBAR, sidebarPath: "nav.json" })).toThrow(/sidebarPath must start with "\/"/);
  });

  it("throws when sidebar route collides with a page", () => {
    expect(() =>
      emitSite({ pages: [{ route: "/sidebar.json", html: "<html>X</html>" }], sidebar: SIDEBAR }),
    ).toThrow(/duplicate route "\/sidebar.json"/);
  });
});

// =====================================================================
// 6. Search
// =====================================================================

describe("emitSite — search", () => {
  it("emits manifest + shards + (no client by default)", () => {
    const bundle = emitSite({ pages: [PAGE_A], search: makeSearch() });
    const routes = bundle.pages.map((p) => p.route);
    expect(routes).toContain("/search/manifest.json");
    expect(routes).toContain("/search/a.json");
    expect(routes).toContain("/search/b.json");
    expect(routes).not.toContain("/search/client.js");
  });

  it("emits client.js when provided", () => {
    const search = { ...makeSearch(), clientJs: "console.log('ok');" };
    const bundle = emitSite({ pages: [], search });
    const client = bundle.pages.find((p) => p.route === "/search/client.js")!;
    expect(client.contentType).toBe(CONTENT_TYPE_JS);
    expect(client.html).toBe("console.log('ok');");
  });

  it("honours basePath override", () => {
    const search = { ...makeSearch(), basePath: "/idx" };
    const bundle = emitSite({ pages: [], search });
    expect(bundle.pages.find((p) => p.route === "/idx/manifest.json")).toBeDefined();
    expect(bundle.pages.find((p) => p.route === "/idx/a.json")).toBeDefined();
  });

  it("rejects basePath with trailing slash", () => {
    const search = { ...makeSearch(), basePath: "/search/" };
    expect(() => emitSite({ pages: [], search })).toThrow(/search.basePath must not end with "\/"/);
  });

  it("rejects clientJs that isn't a string", () => {
    const search = { ...makeSearch(), clientJs: 5 } as unknown as SearchAssets;
    expect(() => emitSite({ pages: [], search })).toThrow(/clientJs must be a string/);
  });

  it("rejects non-Map shards", () => {
    const search = {
      manifest: makeSearch().manifest,
      shards: { a: "x" } as unknown as Map<string, never>,
    };
    expect(() => emitSite({ pages: [], search })).toThrow(/shards must be a Map/);
  });

  it("rejects when shards is missing an entry the manifest lists", () => {
    const search: SearchAssets = {
      manifest: makeSearch().manifest, // lists "a" and "b"
      shards: new Map([["a", makeShard("a", { x: [] })]]), // missing "b"
    };
    expect(() => emitSite({ pages: [], search })).toThrow(/missing entry for shardKey "b"/);
  });

  it("rejects non-string shardKey in manifest", () => {
    const search: SearchAssets = {
      manifest: { ...makeSearch().manifest, shardKeys: [5 as unknown as string] },
      shards: new Map(),
    };
    expect(() => emitSite({ pages: [], search })).toThrow(/shardKeys\[0\] must be a string/);
  });

  it("rejects shardKey containing /", () => {
    const search: SearchAssets = {
      manifest: { ...makeSearch().manifest, shardKeys: ["a/b"] },
      shards: new Map([["a/b", makeShard("a/b", {})]]),
    };
    expect(() => emitSite({ pages: [], search })).toThrow(/forbidden char/);
  });

  it("rejects empty shardKey", () => {
    const search: SearchAssets = {
      manifest: { ...makeSearch().manifest, shardKeys: [""] },
      shards: new Map([["", makeShard("", {})]]),
    };
    expect(() => emitSite({ pages: [], search })).toThrow(/must not be empty/);
  });

  it("rejects shardKey longer than 256 chars", () => {
    const long = "x".repeat(257);
    const search: SearchAssets = {
      manifest: { ...makeSearch().manifest, shardKeys: [long] },
      shards: new Map([[long, makeShard(long, {})]]),
    };
    expect(() => emitSite({ pages: [], search })).toThrow(/exceeds 256 chars/);
  });

  it("rejects shards Map size > maxShards", () => {
    const shards = new Map<string, ReturnType<typeof makeShard>>();
    for (let i = 0; i < 3; i++) shards.set(String.fromCharCode(0x61 + i), makeShard(String.fromCharCode(0x61 + i), {}));
    const search: SearchAssets = {
      manifest: { ...makeSearch().manifest, shardKeys: ["a", "b", "c"] },
      shards,
    };
    expect(() => emitSite({ pages: [], search, maxShards: 2 })).toThrow(/exceeds maxShards=2/);
  });

  it("rejects null search", () => {
    expect(() => emitSite({ pages: [], search: null } as unknown as SiteEmitInput)).toThrow(/search must be an object/);
  });

  it("rejects null manifest", () => {
    const search = { manifest: null, shards: new Map() } as unknown as SearchAssets;
    expect(() => emitSite({ pages: [], search })).toThrow(/manifest must be an object/);
  });

  it("rejects shard with non-Map postings", () => {
    const search: SearchAssets = {
      manifest: { ...makeSearch().manifest, shardKeys: ["a"] },
      shards: new Map([["a", { shardKey: "a", postings: {} } as unknown as ReturnType<typeof makeShard>]]),
    };
    expect(() => emitSite({ pages: [], search })).toThrow(/shard.postings must be a Map/);
  });

  it("rejects shard with non-string shardKey", () => {
    const badShard = { shardKey: 5, postings: new Map() } as unknown as ReturnType<typeof makeShard>;
    const search: SearchAssets = {
      manifest: { ...makeSearch().manifest, shardKeys: ["a"] },
      shards: new Map([["a", badShard]]),
    };
    expect(() => emitSite({ pages: [], search })).toThrow(/shard.shardKey must be a string/);
  });

  it("rejects null shard", () => {
    const search: SearchAssets = {
      manifest: { ...makeSearch().manifest, shardKeys: ["a"] },
      shards: new Map([["a", null as unknown as ReturnType<typeof makeShard>]]),
    };
    expect(() => emitSite({ pages: [], search })).toThrow(/shard must be an object/);
  });

  it("rejects shard.postings with non-string token keys", () => {
    const postings = new Map<unknown, unknown>();
    postings.set(5, []);
    const search: SearchAssets = {
      manifest: { ...makeSearch().manifest, shardKeys: ["a"] },
      shards: new Map([["a", { shardKey: "a", postings } as unknown as ReturnType<typeof makeShard>]]),
    };
    expect(() => emitSite({ pages: [], search })).toThrow(/postings keys must be strings/);
  });

  it("tolerates a manifest whose shardKeys is not an array (emits no shards)", () => {
    // Defensive: if a caller hands us a partial/odd manifest, we
    // still emit manifest.json and skip shard emission rather than
    // throwing on the iterator.
    const search: SearchAssets = {
      manifest: { ...makeSearch().manifest, shardKeys: undefined as unknown as string[] },
      shards: new Map(),
    };
    const bundle = emitSite({ pages: [], search });
    expect(bundle.pages.find((p) => p.route === "/search/manifest.json")).toBeDefined();
    expect(bundle.pages.find((p) => p.route?.startsWith("/search/") && p.route !== "/search/manifest.json")).toBeUndefined();
  });

  it("serialises shard tokens in sorted order (determinism)", () => {
    const postings = new Map<string, Array<{ pageId: string; freq: number; titleHit: boolean }>>();
    // Insert in reverse alphabetical order.
    postings.set("zebra", [{ pageId: "/", freq: 1, titleHit: false }]);
    postings.set("apple", [{ pageId: "/", freq: 1, titleHit: false }]);
    postings.set("mango", [{ pageId: "/", freq: 1, titleHit: false }]);
    const search: SearchAssets = {
      manifest: { ...makeSearch().manifest, shardKeys: ["a"] },
      shards: new Map([["a", { shardKey: "a", postings }]]),
    };
    const bundle = emitSite({ pages: [], search });
    const shardEntry = bundle.pages.find((p) => p.route === "/search/a.json")!;
    const parsed = JSON.parse(shardEntry.html) as { postings: Record<string, unknown> };
    expect(Object.keys(parsed.postings)).toEqual(["apple", "mango", "zebra"]);
  });
});

// =====================================================================
// 7. Extras
// =====================================================================

describe("emitSite — extras", () => {
  it("emits extras with declared content-type and lastmod", () => {
    const extras: ExtraFile[] = [
      { route: "/robots.txt", content: "User-agent: *\nAllow: /\n", contentType: "text/plain", lastmod: "2026-05-22" },
    ];
    const bundle = emitSite({ pages: [PAGE_A], extras });
    const robots = bundle.pages.find((p) => p.route === "/robots.txt")!;
    expect(robots.contentType).toBe("text/plain");
    expect(robots.html).toContain("User-agent: *");
    expect(robots.lastmod).toBe("2026-05-22");
  });

  it("throws when extras is not an array", () => {
    expect(() => emitSite({ pages: [], extras: "x" } as unknown as SiteEmitInput)).toThrow(/extras must be an array/);
  });

  it("throws when an extra is not an object", () => {
    expect(() => emitSite({ pages: [], extras: [null] } as unknown as SiteEmitInput)).toThrow(/extras\[0\] must be an object/);
  });

  it("throws when extra.content is not a string", () => {
    const extras = [{ route: "/x", content: 5, contentType: "text/plain" }] as unknown as ExtraFile[];
    expect(() => emitSite({ pages: [], extras })).toThrow(/content must be a string/);
  });

  it("throws when extra.contentType is not a string", () => {
    const extras = [{ route: "/x", content: "y", contentType: 5 }] as unknown as ExtraFile[];
    expect(() => emitSite({ pages: [], extras })).toThrow(/contentType must be a string/);
  });

  it("throws when extra.lastmod is not a string", () => {
    const extras = [{ route: "/x", content: "y", contentType: "text/plain", lastmod: 5 }] as unknown as ExtraFile[];
    expect(() => emitSite({ pages: [], extras })).toThrow(/lastmod must be a string/);
  });

  it("rejects duplicate route between extra and page", () => {
    const extras: ExtraFile[] = [{ route: "/", content: "robot", contentType: "text/plain" }];
    expect(() => emitSite({ pages: [PAGE_A], extras })).toThrow(/duplicate route "\/"/);
  });
});

// =====================================================================
// 8. Determinism + downstream round-trip
// =====================================================================

describe("emitSite — determinism and round-trip", () => {
  it("produces byte-identical JSON when called twice with the same input", () => {
    const input: SiteEmitInput = {
      pages: [PAGE_B, PAGE_A],
      sidebar: SIDEBAR,
      search: makeSearch(),
      extras: [{ route: "/robots.txt", content: "User-agent: *", contentType: "text/plain" }],
      baseUrl: "https://docs.example.com",
    };
    const bundle1 = emitSite(input);
    const bundle2 = emitSite(input);
    expect(JSON.stringify(bundle1)).toBe(JSON.stringify(bundle2));
  });

  it("feeds cleanly into generatePageBundle (no late validation throws)", () => {
    const bundle = emitSite({
      pages: [PAGE_A, PAGE_B],
      sidebar: SIDEBAR,
      search: makeSearch(),
      baseUrl: "https://docs.example.com",
    });
    const manifestJson = generatePageBundle(bundle);
    const manifest = JSON.parse(manifestJson) as {
      version: 1;
      baseUrl?: string;
      routes: Record<string, { route: string; outputPath: string; contentType: string; sizeBytes: number; sha256: string }>;
    };
    expect(manifest.version).toBe(1);
    expect(manifest.baseUrl).toBe("https://docs.example.com");
    // Every PageEntry we emitted is present.
    for (const p of bundle.pages) {
      expect(manifest.routes[p.route]).toBeDefined();
    }
    // The downstream emitter derives outputPath deterministically;
    // sidebar.json keeps its extension.
    expect(manifest.routes["/sidebar.json"]!.outputPath).toBe("sidebar.json");
    expect(manifest.routes["/search/manifest.json"]!.outputPath).toBe("search/manifest.json");
    expect(manifest.routes["/search/a.json"]!.outputPath).toBe("search/a.json");
  });
});

// =====================================================================
// 9. Default constants — sanity check (also bumps line coverage)
// =====================================================================

describe("emitSite — default constants", () => {
  it("exports the documented defaults", () => {
    expect(DEFAULT_SIDEBAR_PATH).toBe("/sidebar.json");
    expect(DEFAULT_SEARCH_BASE_PATH).toBe("/search");
    expect(DEFAULT_MAX_PAGES).toBe(100_000);
    expect(DEFAULT_MAX_SHARDS).toBe(10_000);
    expect(DEFAULT_MAX_EXTRAS).toBe(10_000);
    expect(CONTENT_TYPE_JSON).toBe("application/json; charset=utf-8");
    expect(CONTENT_TYPE_JS).toBe("application/javascript; charset=utf-8");
  });
});

// =====================================================================
// 10. Realistic 3-page docs site
// =====================================================================

describe("emitSite — realistic", () => {
  it("composes a 3-page docs site with sidebar + search + favicon", () => {
    const pages: DocPage[] = [
      { route: "/", html: "<html><body>home</body></html>", lastmod: "2026-05-20" },
      { route: "/guide/setup", html: "<html><body>setup</body></html>", lastmod: "2026-05-21" },
      { route: "/api", html: "<html><body>api</body></html>", lastmod: "2026-05-22" },
    ];
    const sidebar = [
      { kind: "page" as const, label: "Home", path: "index.md", position: 1 },
      { kind: "group" as const, label: "Guide", path: "guide/index.md", position: 2, children: [
        { kind: "page" as const, label: "Setup", path: "guide/setup.md", position: 1 },
      ] },
      { kind: "page" as const, label: "API", path: "api.md", position: 3 },
    ];
    const search = makeSearch();
    const extras: ExtraFile[] = [
      { route: "/favicon.ico", content: "<binary>", contentType: "image/x-icon" },
      { route: "/robots.txt", content: "User-agent: *\nAllow: /\n", contentType: "text/plain" },
    ];
    const bundle = emitSite({
      pages,
      sidebar,
      search: { ...search, clientJs: "/* search client */" },
      extras,
      baseUrl: "https://docs.example.com",
    });

    // Expected route set.
    const routes = bundle.pages.map((p) => p.route).sort();
    expect(routes).toEqual([
      "/",
      "/api",
      "/favicon.ico",
      "/guide/setup",
      "/robots.txt",
      "/search/a.json",
      "/search/b.json",
      "/search/client.js",
      "/search/manifest.json",
      "/sidebar.json",
    ]);
    // Round-trip through generatePageBundle.
    const manifest = JSON.parse(generatePageBundle(bundle)) as { routes: Record<string, { outputPath: string }> };
    expect(Object.keys(manifest.routes).length).toBe(10);
  });
});
