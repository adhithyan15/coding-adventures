/**
 * search-bundle.test.ts — tests for the browser-side search
 * bundle.
 *
 * Two layers:
 *
 *   1. UNIT tests for `bundleSearchClient` — runs esbuild,
 *      verifies the output is well-formed JS, includes the
 *      adapter that converts JSON-object postings back into a
 *      Map (the bug that caused "No matches" for every query
 *      in the first attempt).
 *
 *   2. END-TO-END test in JSDOM — loads the actual `dist/`
 *      output, stubs `fetch` to read from disk, types into the
 *      rendered search input, and asserts the dropdown
 *      populates with real results.  This is the regression
 *      test for the bug where `plainText` returned "" for
 *      every page (because it read `node.text` instead of
 *      `node.value`), leaving the search index with only
 *      title tokens and most queries returning "No matches".
 *
 * The JSDOM test BUILDS the site fresh (rather than relying
 * on a prior `npm start` to have produced `dist/`) so it
 * works in CI with no setup steps.  It runs in a few seconds
 * because the corpus is small.
 */

import { describe, it, expect, beforeAll } from "vitest";
import { JSDOM } from "jsdom";
import * as fs from "node:fs/promises";
import * as fsSync from "node:fs";
import * as path from "node:path";

import { bundleSearchClient } from "../src/search-bundle.js";
import { build } from "../src/build.js";
import { readCorpus, writeBundle } from "../src/main.js";

const REPO_ROOT = path.resolve(__dirname, "..");
const CORPUS = path.join(REPO_ROOT, "corpus");

// =====================================================================
// 1. bundleSearchClient — unit tests
// =====================================================================

describe("bundleSearchClient", () => {
  // esbuild is slow-ish to start (cold-cache ~300ms); cache the
  // bundle across tests so each `it` block doesn't repeat the work.
  let minified: string;
  let readable: string;

  beforeAll(async () => {
    [minified, readable] = await Promise.all([
      bundleSearchClient(),
      bundleSearchClient({ minify: false }),
    ]);
  });

  it("returns a non-empty string", () => {
    expect(typeof minified).toBe("string");
    expect(minified.length).toBeGreaterThan(1000);
  });

  it("wraps output as an IIFE (safe to load via <script>)", () => {
    expect(minified.startsWith("(()=>{") || minified.startsWith("(function(){")).toBe(true);
    expect(minified.trimEnd().endsWith(";")).toBe(true);
  });

  it("bundles SearchClient + tokenize (the only file: deps)", () => {
    // Symbol names survive even under minification because
    // they're class members / function arguments.  Search for
    // text that has to be present in any working bundle.
    expect(readable).toContain("SearchClient");
    expect(readable).toContain("tokenize");
  });

  it("includes the Map adapter so postings JSON-objects become Maps", () => {
    // The bug this protects against: SearchClient's isLikelyShard
    // requires `postings instanceof Map`, but JSON.parse returns
    // plain objects.  The adapter MUST be present.
    expect(readable).toContain("new Map(Object.entries");
  });

  it("targets the right DOM selector for the page-shell search input", () => {
    // page-shell renders `<input class="search" type="search">`
    // — the bundle MUST query that selector verbatim.
    expect(readable).toContain('"input.search"');
  });

  it("reads the build-id cache-bust from window.__formeDocSearchBuildId", () => {
    expect(readable).toContain("__formeDocSearchBuildId");
  });

  it("is small enough to ship in every page (< 20KB minified)", () => {
    expect(minified.length).toBeLessThan(20 * 1024);
  });
});

// =====================================================================
// 2. End-to-end in JSDOM — the regression test for "No matches"
// =====================================================================

describe("search bundle — end-to-end in JSDOM", () => {
  let dom: JSDOM;
  let window: JSDOM["window"];
  let tmpDist: string;

  beforeAll(async () => {
    // Build the site to a tmp dir.  We use the program's real
    // build pipeline rather than relying on a checked-in
    // `dist/` so the test fails if anything in the pipeline
    // breaks the search index.
    tmpDist = await fs.mkdtemp(path.join(REPO_ROOT, ".tmp-search-e2e-"));
    const files = await readCorpus(CORPUS);
    const searchClientJs = await bundleSearchClient();
    const bundle = build(files, {
      siteTitle: "Test Docs",
      searchClientJs,
    });
    await writeBundle(bundle, tmpDist);

    // Spin up JSDOM with the rendered index.html.
    const indexHtml = await fs.readFile(path.join(tmpDist, "index.html"), "utf8");
    dom = new JSDOM(indexHtml, {
      url: "http://test.local/",
      runScripts: "outside-only", // we'll eval the bundle manually
      pretendToBeVisual: true,
    });
    window = dom.window;

    // Stub fetch so the bundle's manifest + shard fetches
    // resolve from the tmp dist.  Strips the `?v=...` cache-
    // bust query string before resolving the file path.
    window.fetch = async function (url: unknown): Promise<unknown> {
      const u = String(url);
      const pathOnly = u.startsWith("http")
        ? new URL(u).pathname
        : u.split("?")[0]!;
      const filePath = path.join(tmpDist, pathOnly.replace(/^\//, ""));
      try {
        const data = fsSync.readFileSync(filePath, "utf8");
        return {
          ok: true, status: 200,
          json: async () => JSON.parse(data),
          text: async () => data,
        };
      } catch {
        return { ok: false, status: 404 };
      }
    } as unknown as typeof fetch;

    // Load the bundle into the JSDOM window.  evalRunsInWindow
    // ensures `document`, `window`, `fetch` all resolve to JSDOM.
    const bundleJs = await fs.readFile(path.join(tmpDist, "search/client.js"), "utf8");
    window.eval(bundleJs);

    // Give init() a tick to run (it's async via DOMContentLoaded).
    await waitFor(() => window.document.querySelector("ul.search-results") !== null);
  });

  it("renders the dropdown skeleton into the body", () => {
    const dropdown = window.document.querySelector("ul.search-results");
    expect(dropdown).not.toBeNull();
    // Per the alignment fix, dropdown lives at body level
    // (not inside the header).
    expect(dropdown!.parentElement).toBe(window.document.body);
  });

  it('returns real results for "install"', async () => {
    const results = await typeAndCollect("install");
    expect(results.length).toBeGreaterThan(0);
    // The installation page should be a hit (it's literally
    // titled "Installation").
    expect(results.some((r) => r.includes("/guide/installation"))).toBe(true);
  });

  it('returns real results for "widget" (body-text token)', async () => {
    // REGRESSION TEST: "widget" appears only in page BODIES
    // (not titles).  If plainText() returns "" for bodies —
    // the original bug — this query returns 0 results.
    const results = await typeAndCollect("widget");
    expect(results.length).toBeGreaterThan(0);
  });

  it('returns real results for "deno" (body-only, single page)', async () => {
    const results = await typeAndCollect("deno");
    expect(results.length).toBe(1);
    expect(results[0]).toContain("/guide/installation");
  });

  it('shows "No matches." for a query that hits nothing', async () => {
    const results = await typeAndCollect("zzzzzzzzzzzz");
    expect(results).toEqual(["No matches."]);
  });

  it("clears the dropdown when the input is emptied", async () => {
    const input = window.document.querySelector("input.search") as HTMLInputElement;
    input.value = "widget";
    input.dispatchEvent(new window.Event("input", { bubbles: true }));
    await waitFor(() => {
      const d = window.document.querySelector("ul.search-results") as HTMLElement;
      return d && !d.hidden;
    });
    // Now clear.
    input.value = "";
    input.dispatchEvent(new window.Event("input", { bubbles: true }));
    await waitFor(() => {
      const d = window.document.querySelector("ul.search-results") as HTMLElement;
      return d.hidden === true;
    });
    const dropdown = window.document.querySelector("ul.search-results") as HTMLElement;
    expect(dropdown.hidden).toBe(true);
  });

  // ---- helpers --------------------------------------------------

  /**
   * Type a query into the input, wait for the dropdown to
   * populate with the NEW result set, and return the per-row
   * text content.
   *
   * We snapshot the dropdown's current text BEFORE dispatching
   * the input event, then poll until the text differs.  This
   * defeats stale-content false-positives across consecutive
   * `typeAndCollect` calls (e.g. typing "deno" after "install"
   * — without the change check, the poll's `children.length > 0`
   * condition is satisfied immediately by the leftover "install"
   * results before the new query has run).
   */
  async function typeAndCollect(q: string): Promise<string[]> {
    const input = window.document.querySelector("input.search") as HTMLInputElement;
    const dropdown = window.document.querySelector("ul.search-results")!;
    const before = snapshotDropdown(dropdown);
    input.value = q;
    input.dispatchEvent(new window.Event("input", { bubbles: true }));
    await waitFor(() => {
      const d = window.document.querySelector("ul.search-results") as HTMLElement;
      if (!d || d.hidden) return false;
      if (d.children.length === 0) return false;
      return snapshotDropdown(d) !== before;
    });
    return snapshotDropdownRows(dropdown);
  }

  function snapshotDropdown(d: Element): string {
    return Array.from(d.children).map((c) => c.textContent ?? "").join("\n");
  }
  function snapshotDropdownRows(d: Element): string[] {
    return Array.from(d.children).map((c) => c.textContent ?? "");
  }
});

/**
 * Poll `cond` every 25ms up to `timeoutMs`.  Resolves when
 * `cond` returns truthy.  Rejects on timeout.
 *
 * We poll instead of using a fixed `setTimeout` because the
 * UI's debounce + microtask chain can vary by a few ms in
 * JSDOM; busy-waiting is cheap and avoids flake.
 */
async function waitFor(cond: () => boolean, timeoutMs = 2000): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    if (cond()) return;
    await new Promise((r) => setTimeout(r, 25));
  }
  throw new Error(`waitFor: condition not met within ${timeoutMs}ms`);
}
