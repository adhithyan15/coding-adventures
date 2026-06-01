/**
 * search-bundle.ts — bundle `forme-doc-search-client-js` (plus
 * its only file: dep, `forme-doc-search-tokenizer`) into a
 * single browser-loadable JavaScript file, wrapped with the
 * tiny in-page UI glue (find the search input, render a
 * dropdown of results, navigate on click).
 *
 * # Why bundle in the demo program rather than ship a
 *   pre-built bundle alongside the library?
 *
 * The library deliberately stays as TypeScript source and
 * declares `capabilities: []` — it never instantiates `fetch()`
 * itself, the caller injects the shard-fetcher.  Bundling for
 * the browser is therefore a CALLER concern.  This demo is the
 * first caller; it picks esbuild (battle-tested, single binary,
 * no config), wraps the class with a small IIFE that wires
 * `window` + DOM, and hands the result back as a string to
 * `emitSite`'s `search.clientJs` option.
 *
 * # The Map-vs-Object shape adapter
 *
 * `forme-doc-site-emitter` serialises each shard's `postings`
 * Map as a plain JSON object (Maps don't survive JSON round-
 * trips).  The browser-side `SearchClient.isLikelyShard` check
 * requires `postings instanceof Map`.  The wrapper below
 * converts the parsed Object back into a Map inside its
 * injected `fetchShard` callback — so the shape contract is
 * preserved end-to-end without changing either package.
 *
 * # The UI bootstrap
 *
 * Conservative DOM scripting: find the `<input class="search">`
 * page-shell already renders, attach a debounced `input`
 * listener, render results as an `<ul>` directly below the
 * input.  No frameworks, no virtual DOM; just `createElement`
 * + `textContent` (defends against XSS — pageId / matched
 * tokens become text nodes, never HTML).
 *
 * # Capabilities
 *
 * `[]` — this file is a pure transform (markdown of strings
 * into a bundled string).  The build script in `main.ts` calls
 * it; the resulting string is handed to `emitSite`.  esbuild
 * is invoked synchronously in this process, but that's not an
 * I/O capability — it's a library call.
 */

import { build as esbuild } from "esbuild";
import * as path from "node:path";
import * as url from "node:url";

const HERE = path.dirname(url.fileURLToPath(import.meta.url));

/**
 * Bundle `SearchClient` + UI glue into one self-executing
 * browser script.  Returns the JS source as a string ready to
 * pass into `emitSite({ search: { clientJs: ... } })`.
 *
 * The bundle:
 *   - Is an IIFE (no globals leak; safe to load via
 *     `<script src="/search/client.js" defer>`).
 *   - Exposes `window.__formeDocSearch` ONLY for debugging
 *     (the actual wiring happens inside the IIFE).
 *   - Imports `SearchClient` from
 *     `forme-doc-search-client-js` via esbuild's
 *     standard resolution from the demo program's
 *     `node_modules` (where the library was installed as a
 *     `file:` dependency).
 *   - Targets ES2020 — fine for any browser shipped in the
 *     last ~5 years; matches the rest of the repo's TS target.
 *
 * Minification: ON by default for the demo (smaller artifact
 * + easier visual inspection that the bundle is "done"); pass
 * `{ minify: false }` to keep the source readable while
 * developing.
 */
export async function bundleSearchClient(options: { minify?: boolean } = {}): Promise<string> {
  const minify = options.minify ?? true;

  // The browser-side wrapper.  We hand it to esbuild via
  // `stdin` — that lets us co-locate the source here without a
  // physical entry file.  `resolveDir: HERE` makes the
  // `forme-doc-search-client-js` import resolve from THIS
  // package's `node_modules`, which is what we want.
  const result = await esbuild({
    stdin: {
      contents: BROWSER_ENTRY_SOURCE,
      resolveDir: HERE,
      sourcefile: "search-bootstrap.ts",
      loader: "ts",
    },
    bundle: true,
    format: "iife",
    target: "es2020",
    platform: "browser",
    minify,
    legalComments: "none",
    write: false,
    // Defensive: we don't expect any externals (search-client
    // and tokenizer are pure), so any unresolved import should
    // throw rather than silently produce a runtime ReferenceError
    // in the browser.
    external: [],
  });

  if (result.outputFiles.length !== 1) {
    throw new Error(`bundleSearchClient: expected 1 output file, got ${result.outputFiles.length}`);
  }
  return result.outputFiles[0]!.text;
}

// ─────────────────────────────────────────────────────────────────────
// The browser-side bootstrap — pure ES module source.  esbuild bundles
// THIS together with the imported `SearchClient` into a single IIFE.
//
// Kept inside a template string (rather than a separate `.ts` file)
// because:
//   (a) it's small and tightly coupled to the demo's exact CSS
//       selectors / route layout — no consumer would reuse it
//       verbatim;
//   (b) the only thing that varies across demo runs is the manifest
//       URL, which the bootstrap fetches at runtime — no per-build
//       string interpolation needed, so a static template is fine.
//
// Style notes for whoever extends this:
//   - Pure DOM APIs only (no jQuery, no framework).
//   - No `innerHTML` from user data — every text insertion uses
//     `textContent` to keep the dropdown XSS-safe even if a
//     malicious pageId or matched-token slipped through.
//   - Debounce on input — search isn't free; rapid typing should
//     coalesce.
//   - Graceful no-op when DOM doesn't have the search input
//     (some pages may opt out by not setting `searchPlaceholder`).
// ─────────────────────────────────────────────────────────────────────
const BROWSER_ENTRY_SOURCE = String.raw`
import { SearchClient } from "@coding-adventures/forme-doc-search-client-js";

const MANIFEST_URL = "/search/manifest.json";
const SHARD_URL_PREFIX = "/search/";
const DEBOUNCE_MS = 120;
const MAX_RESULTS = 10;
const INPUT_SELECTOR = "input.search";

// build.ts injects a tiny <script>window.__formeDocSearchBuildId=...</script>
// right before loading us, so we can append the same cache-bust
// query string to the JSON fetches.  Without it, Safari happily
// reuses cached manifest/shard JSON even after a rebuild.
function buildIdSuffix() {
  const id = window.__formeDocSearchBuildId;
  return typeof id === "string" && id.length > 0 ? "?v=" + encodeURIComponent(id) : "";
}

async function fetchShard(shardKey) {
  const r = await fetch(SHARD_URL_PREFIX + encodeURIComponent(shardKey) + ".json" + buildIdSuffix());
  if (!r.ok) throw new Error("shard fetch failed: " + r.status);
  const raw = await r.json();
  // Shape guard before the Object.entries call — without it, a
  // malformed shard (postings: null, a number, a string)
  // throws a confusing TypeError out of Object.entries.  With
  // it, the message in the caller's console.warn is actionable.
  if (raw === null || typeof raw !== "object" || raw.postings === null || typeof raw.postings !== "object") {
    throw new Error("malformed shard: missing or non-object postings");
  }
  // The server emitted postings as a plain JSON object (Maps
  // don't survive JSON.stringify).  Convert back to a Map so
  // SearchClient.isLikelyShard accepts the shape.
  return {
    shardKey: raw.shardKey,
    postings: new Map(Object.entries(raw.postings)),
  };
}

async function init() {
  const input = document.querySelector(INPUT_SELECTOR);
  if (!input) return; // No search input on this page; nothing to wire.

  // Render skeleton: a dropdown UL.  IMPORTANT — we append
  // to BODY and use position:fixed rather than dropping the
  // dropdown next to the input.  Two reasons:
  //   (a) the input lives inside a display:flex header;
  //       inserting the dropdown as a sibling would turn it
  //       into a flex item and disrupt the header layout
  //       (pushing the GitHub link off-screen, etc.).
  //   (b) position:absolute would need a positioned ancestor —
  //       fragile across themes.  position:fixed + body-level
  //       placement is independent of any ancestor styling.
  // We re-measure the input's bounding rect every time we
  // show the dropdown so it stays anchored under the input
  // even after window resize / sticky-header scrolling.
  const dropdown = document.createElement("ul");
  dropdown.className = "search-results";
  dropdown.setAttribute("role", "listbox");
  dropdown.hidden = true;
  Object.assign(dropdown.style, {
    position: "fixed", listStyle: "none", margin: "0", padding: "0",
    background: "var(--bg, #fff)", border: "1px solid var(--border, #e5e5e5)",
    borderRadius: "4px", boxShadow: "0 4px 12px rgba(0,0,0,.08)",
    maxHeight: "60vh", overflowY: "auto", zIndex: "1000",
    fontSize: "14px",
    // top/left/width set dynamically by positionDropdown() before show.
  });
  document.body.appendChild(dropdown);

  function positionDropdown() {
    const rect = input.getBoundingClientRect();
    dropdown.style.top = (rect.bottom + 4) + "px";
    dropdown.style.left = rect.left + "px";
    dropdown.style.width = rect.width + "px";
  }

  let manifestPromise = null;
  function getManifest() {
    if (manifestPromise === null) {
      manifestPromise = fetch(MANIFEST_URL + buildIdSuffix()).then(function (r) {
        if (!r.ok) throw new Error("manifest fetch failed: " + r.status);
        return r.json();
      });
    }
    return manifestPromise;
  }

  let client = null;
  async function getClient() {
    if (client !== null) return client;
    const manifest = await getManifest();
    client = new SearchClient({ manifest, fetchShard });
    return client;
  }

  function clearDropdown() {
    while (dropdown.firstChild) dropdown.removeChild(dropdown.firstChild);
    dropdown.hidden = true;
  }

  function renderResults(results) {
    clearDropdown();
    if (results.length === 0) {
      const li = document.createElement("li");
      li.style.padding = "0.6rem 0.9rem";
      li.style.color = "var(--muted, #666)";
      li.textContent = "No matches.";
      dropdown.appendChild(li);
      positionDropdown();
      dropdown.hidden = false;
      return;
    }
    for (const r of results.slice(0, MAX_RESULTS)) {
      const li = document.createElement("li");
      li.setAttribute("role", "option");
      const a = document.createElement("a");
      // Scheme guard — defence-in-depth.  pageId comes from
      // build-time routeFor() which always emits a leading "/",
      // but if a future code path ever fed an attacker-controlled
      // pageId, an href like "javascript:alert(1)" would execute
      // on click.  Refuse anything that doesn't look like an
      // app-relative path.
      a.href = (typeof r.pageId === "string" && r.pageId.startsWith("/")) ? r.pageId : "#";
      a.style.display = "block";
      a.style.padding = "0.5rem 0.9rem";
      a.style.color = "var(--fg, #222)";
      a.style.textDecoration = "none";
      a.addEventListener("mouseenter", function () {
        a.style.background = "var(--sidebar-bg, #fafafa)";
      });
      a.addEventListener("mouseleave", function () {
        a.style.background = "";
      });

      // textContent for both — defends against any user-controlled
      // pageId / token leaking HTML.  No innerHTML anywhere.
      const title = document.createElement("div");
      title.style.fontWeight = "500";
      title.textContent = r.pageId;
      a.appendChild(title);

      if (r.matchedTokens && r.matchedTokens.length > 0) {
        const meta = document.createElement("div");
        meta.style.fontSize = "12px";
        meta.style.color = "var(--muted, #666)";
        meta.textContent = "matched: " + r.matchedTokens.join(", ");
        a.appendChild(meta);
      }

      li.appendChild(a);
      dropdown.appendChild(li);
    }
    positionDropdown();
    dropdown.hidden = false;
  }

  let debounceTimer = null;
  let lastQueryId = 0;
  async function handleInput() {
    const q = input.value.trim();
    if (q.length === 0) {
      clearDropdown();
      return;
    }
    const myQueryId = ++lastQueryId;
    try {
      const c = await getClient();
      const results = await c.search(q);
      if (myQueryId !== lastQueryId) return; // Stale — a newer query won.
      renderResults(results);
    } catch (err) {
      // Defensive: never crash the page on a search failure.
      // Console only; no UI alert that would startle the user.
      console.warn("[forme-doc-search] query failed:", err);
    }
  }

  input.addEventListener("input", function () {
    if (debounceTimer !== null) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(handleInput, DEBOUNCE_MS);
  });

  // Click-outside dismissal.
  document.addEventListener("click", function (e) {
    if (e.target !== input && !dropdown.contains(e.target)) {
      clearDropdown();
    }
  });

  // ESC clears + closes the dropdown.
  input.addEventListener("keydown", function (e) {
    if (e.key === "Escape") {
      input.value = "";
      clearDropdown();
    }
  });

  // Re-show dropdown when re-focusing if there's already text.
  input.addEventListener("focus", function () {
    if (input.value.trim().length > 0) handleInput();
  });

  // Re-anchor on viewport changes while the dropdown is open.
  // Scrolling matters because the header is sticky; resize matters
  // because the input width comes from the header layout.
  window.addEventListener("scroll", function () { if (!dropdown.hidden) positionDropdown(); }, { passive: true });
  window.addEventListener("resize", function () { if (!dropdown.hidden) positionDropdown(); });

  // Expose for debugging only.  Tests / dev console can call
  // window.__formeDocSearch.search("query") to inspect.
  window.__formeDocSearch = { getClient };
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", init);
} else {
  init();
}
`;
