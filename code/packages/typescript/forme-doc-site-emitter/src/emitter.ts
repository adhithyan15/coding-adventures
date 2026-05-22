/**
 * emitter.ts — the `emitSite` transform.
 *
 * Composes DOC00 per-stage outputs into a `PageBundleConfig` for
 * `forme-aot-page-bundle-emitter`.  Pure data manipulation;
 * deterministic; capability `[]`.
 *
 * # The composition step in pictures
 *
 * ```
 *    pages: [{route, html}, ...]   ── as-is, one PageEntry each
 *                ▼
 *    sidebar (tree)                ── one PageEntry: /sidebar.json
 *                ▼
 *    search.manifest               ── one PageEntry: /search/manifest.json
 *    search.shards                 ── one PageEntry per shard:
 *                                       /search/<key>.json
 *    search.clientJs               ── one PageEntry: /search/client.js
 *                ▼
 *    extras: [{route, content}]    ── one PageEntry each
 *                ▼
 *           PageBundleConfig  →  forme-aot-page-bundle-emitter
 *                              →  forme-aot-deploy-manifest-emitter
 *                              →  forme-deploy-runner  (writes to disk)
 * ```
 *
 * Routes are deduplicated upfront — any collision throws a
 * `TypeError` rather than silently letting one entry win.  This
 * catches author mistakes like accidentally putting two pages at
 * the same URL, or a sidebar at `/search/sidebar.json` colliding
 * with a search shard.
 *
 * # JSON serialisation choices (deterministic on purpose)
 *
 *   - **No `indent` argument** to `JSON.stringify` — compact JSON
 *     ships smaller and is also easier to diff in content-hashed
 *     deploys (an indented file's hash changes whenever any nested
 *     value moves around).
 *
 *   - **Sorted keys** for manifest objects with arbitrary key sets
 *     (the shards-by-key emission iterates `manifest.shardKeys`
 *     which is already sorted by the upstream builder; we don't
 *     trust shard insertion order).
 *
 *   - **Maps → plain objects via `Object.create(null)`** — defends
 *     downstream consumers against unexpected prototype-chain
 *     properties when they `obj[key]` the result.  Then
 *     `JSON.stringify` happens to handle null-prototype objects
 *     identically to plain objects.
 *
 * @module emitter
 */

import type {
  PageBundleConfig,
  PageEntry,
} from "@coding-adventures/forme-aot-page-bundle-emitter";
import type {
  DocPage,
  ExtraFile,
  SearchAssets,
  SiteEmitInput,
} from "./types.js";

/**
 * Default sentinel values — exported for callers that want to
 * reference them or override consistently.
 */
export const DEFAULT_SIDEBAR_PATH = "/sidebar.json";
export const DEFAULT_SEARCH_BASE_PATH = "/search";
export const DEFAULT_MAX_PAGES = 100_000;
export const DEFAULT_MAX_SHARDS = 10_000;
export const DEFAULT_MAX_EXTRAS = 10_000;
export const CONTENT_TYPE_JSON = "application/json; charset=utf-8";
export const CONTENT_TYPE_JS = "application/javascript; charset=utf-8";

/**
 * Compose the input into a `PageBundleConfig`.
 *
 * Validates inputs up-front; deterministic in output order.
 *
 * @throws `TypeError` for invalid routes, duplicate routes,
 *         out-of-range counts, or malformed inputs.
 */
export function emitSite(input: SiteEmitInput): PageBundleConfig {
  if (input === null || typeof input !== "object") {
    throw new TypeError("emitSite: input must be an object");
  }

  // -------- 1. Numeric option validation -------------------------
  // Always `Number.isFinite` — `>`/`>=` silently pass NaN.
  const maxPages = resolveLimit(input.maxPages, DEFAULT_MAX_PAGES, "maxPages");
  const maxShards = resolveLimit(input.maxShards, DEFAULT_MAX_SHARDS, "maxShards");
  const maxExtras = resolveLimit(input.maxExtras, DEFAULT_MAX_EXTRAS, "maxExtras");

  // -------- 2. Pages validation ---------------------------------
  if (!Array.isArray(input.pages)) {
    throw new TypeError("emitSite: input.pages must be an array");
  }
  if (input.pages.length > maxPages) {
    throw new TypeError(
      `emitSite: ${input.pages.length} pages exceeds maxPages=${maxPages}`,
    );
  }

  // Output accumulator + the seen-route set for duplicate detection.
  const out: PageEntry[] = [];
  const seenRoutes = new Set<string>();

  for (let i = 0; i < input.pages.length; i++) {
    const page = input.pages[i]!;
    if (page === null || typeof page !== "object") {
      throw new TypeError(`emitSite: pages[${i}] must be an object`);
    }
    validateRouteShape(page.route, `pages[${i}].route`);
    if (typeof page.html !== "string") {
      throw new TypeError(`emitSite: pages[${i}].html must be a string`);
    }
    if (page.lastmod !== undefined && typeof page.lastmod !== "string") {
      throw new TypeError(`emitSite: pages[${i}].lastmod must be a string`);
    }
    pushUnique(out, seenRoutes, makePageEntry(page));
  }

  // -------- 3. Sidebar ------------------------------------------
  if (input.sidebar !== undefined) {
    const sidebarPath = input.sidebarPath ?? DEFAULT_SIDEBAR_PATH;
    validateRouteShape(sidebarPath, "sidebarPath");
    if (!Array.isArray(input.sidebar)) {
      throw new TypeError("emitSite: input.sidebar must be an array");
    }
    pushUnique(out, seenRoutes, {
      route: sidebarPath,
      html: stableJsonStringify(input.sidebar),
      contentType: CONTENT_TYPE_JSON,
    });
  }

  // -------- 4. Search ------------------------------------------
  if (input.search !== undefined) {
    appendSearchEntries(out, seenRoutes, input.search, maxShards);
  }

  // -------- 5. Extras ------------------------------------------
  if (input.extras !== undefined) {
    if (!Array.isArray(input.extras)) {
      throw new TypeError("emitSite: input.extras must be an array");
    }
    if (input.extras.length > maxExtras) {
      throw new TypeError(
        `emitSite: ${input.extras.length} extras exceeds maxExtras=${maxExtras}`,
      );
    }
    for (let i = 0; i < input.extras.length; i++) {
      const extra = input.extras[i]!;
      validateExtra(extra, i);
      pushUnique(out, seenRoutes, makeExtraEntry(extra));
    }
  }

  // -------- 6. Build the final PageBundleConfig ----------------
  const cfg: { pages: PageEntry[]; baseUrl?: string } = { pages: out };
  if (input.baseUrl !== undefined) {
    if (typeof input.baseUrl !== "string") {
      throw new TypeError("emitSite: input.baseUrl must be a string");
    }
    cfg.baseUrl = input.baseUrl;
  }
  return cfg;
}

// =====================================================================
// HELPERS
// =====================================================================

/**
 * Validate a numeric limit option.  `undefined` returns the
 * default; anything else must be a finite integer ≥ 0.  NaN,
 * Infinity, negatives, and non-integers all throw.
 *
 * The `>= 0` check (not `>= 1`) is intentional: a caller setting
 * `maxPages: 0` is asserting "I want this build to refuse any
 * pages" — that's a meaningful assertion, not a bug.
 */
function resolveLimit(
  value: number | undefined,
  fallback: number,
  name: string,
): number {
  if (value === undefined) return fallback;
  if (!Number.isFinite(value) || !Number.isInteger(value) || value < 0) {
    throw new TypeError(
      `emitSite: ${name} must be a non-negative integer (got ${String(value)})`,
    );
  }
  return value;
}

/**
 * Reject obviously dangerous / wrong-shaped routes BEFORE the
 * downstream emitter sees them — fail-fast at the boundary
 * closest to the user's input.  These checks mirror the rules
 * `forme-aot-page-bundle-emitter.validateRoute` enforces, with
 * the same rejection criteria, so feeding our output into the
 * downstream emitter never surprises with a late throw.
 *
 *   - Must be a string.
 *   - Must start with `/`.
 *   - Must not contain `\`        — Windows path separator.
 *   - Must not contain `//`       — protocol-relative URL hint
 *                                   (`//evil.example.com/page`
 *                                   parses as cross-origin).
 *   - Must not contain `..`       — path-traversal segment.
 *   - Must not contain control chars (< 0x20 or 0x7f).
 *   - Length cap (8192) — same as `forme-doc-sidebar-builder`'s
 *                         path cap; defends against pathological
 *                         long inputs.
 */
function validateRouteShape(route: unknown, what: string): asserts route is string {
  if (typeof route !== "string") {
    throw new TypeError(`emitSite: ${what} must be a string`);
  }
  if (route.length === 0 || route.charCodeAt(0) !== 0x2f /* "/" */) {
    throw new TypeError(`emitSite: ${what} must start with "/" (got ${JSON.stringify(route)})`);
  }
  if (route.length > 8192) {
    throw new TypeError(`emitSite: ${what} exceeds 8192 chars`);
  }
  // Explicit char-by-char loop (no regex) — keeps this trivially
  // ReDoS-free and matches the project-wide convention established
  // by sidebar-builder and page-shell.
  for (let i = 0; i < route.length; i++) {
    const c = route.charCodeAt(i);
    if (c === 0x5c /* "\" */) {
      throw new TypeError(`emitSite: ${what} must not contain "\\"`);
    }
    if (c < 0x20 || c === 0x7f) {
      throw new TypeError(`emitSite: ${what} must not contain control chars`);
    }
    // `//`  — adjacent slash check (other than allowed leading "/")
    if (i > 0 && c === 0x2f && route.charCodeAt(i - 1) === 0x2f) {
      throw new TypeError(`emitSite: ${what} must not contain "//"`);
    }
  }
  // `..` segment check — explicit segment walk (no regex).
  let segStart = 1; // skip leading "/"
  for (let i = 1; i <= route.length; i++) {
    if (i === route.length || route.charCodeAt(i) === 0x2f) {
      const segLen = i - segStart;
      if (segLen === 2
          && route.charCodeAt(segStart) === 0x2e
          && route.charCodeAt(segStart + 1) === 0x2e) {
        throw new TypeError(`emitSite: ${what} must not contain ".." segment`);
      }
      segStart = i + 1;
    }
  }
}

/**
 * Push an entry only after confirming the route hasn't already
 * been claimed.  Duplicates throw — the caller almost certainly
 * has a bug, and silently dropping data would surface as
 * mysterious missing pages at deploy time.
 */
function pushUnique(
  out: PageEntry[],
  seen: Set<string>,
  entry: PageEntry,
): void {
  if (seen.has(entry.route)) {
    throw new TypeError(
      `emitSite: duplicate route ${JSON.stringify(entry.route)}`,
    );
  }
  seen.add(entry.route);
  out.push(entry);
}

/**
 * Convert a `DocPage` into a `PageEntry`.  Pure data shape
 * rearrangement; HTML is passed through verbatim (the upstream
 * page-shell + html-doc-emitter own escaping correctness).
 */
function makePageEntry(page: DocPage): PageEntry {
  const entry: { route: string; html: string; lastmod?: string } = {
    route: page.route,
    html: page.html,
  };
  if (page.lastmod !== undefined) entry.lastmod = page.lastmod;
  return entry;
}

/**
 * Convert an `ExtraFile` into a `PageEntry`.  The
 * `PageBundleConfig.PageEntry` schema uses `html` as the body
 * field for any content type — naming is a holdover from the
 * common-case, not a content-type assertion.
 */
function makeExtraEntry(extra: ExtraFile): PageEntry {
  const entry: { route: string; html: string; contentType: string; lastmod?: string } = {
    route: extra.route,
    html: extra.content,
    contentType: extra.contentType,
  };
  if (extra.lastmod !== undefined) entry.lastmod = extra.lastmod;
  return entry;
}

/**
 * Validate an `ExtraFile` before we trust its fields.
 */
function validateExtra(extra: unknown, i: number): asserts extra is ExtraFile {
  if (extra === null || typeof extra !== "object") {
    throw new TypeError(`emitSite: extras[${i}] must be an object`);
  }
  const e = extra as { route?: unknown; content?: unknown; contentType?: unknown; lastmod?: unknown };
  validateRouteShape(e.route, `extras[${i}].route`);
  if (typeof e.content !== "string") {
    throw new TypeError(`emitSite: extras[${i}].content must be a string`);
  }
  if (typeof e.contentType !== "string") {
    throw new TypeError(`emitSite: extras[${i}].contentType must be a string`);
  }
  if (e.lastmod !== undefined && typeof e.lastmod !== "string") {
    throw new TypeError(`emitSite: extras[${i}].lastmod must be a string`);
  }
}

/**
 * Append the search-related `PageEntry`s into `out`.  Order:
 *
 *   1. `<basePath>/manifest.json`
 *   2. `<basePath>/<shardKey>.json` — one per shard, iterated in
 *      sorted `manifest.shardKeys` order for determinism.
 *   3. `<basePath>/client.js` (only if `clientJs` is provided).
 */
function appendSearchEntries(
  out: PageEntry[],
  seen: Set<string>,
  search: SearchAssets,
  maxShards: number,
): void {
  if (search === null || typeof search !== "object") {
    throw new TypeError("emitSite: search must be an object");
  }
  if (search.manifest === null || typeof search.manifest !== "object") {
    throw new TypeError("emitSite: search.manifest must be an object");
  }
  if (!(search.shards instanceof Map)) {
    throw new TypeError("emitSite: search.shards must be a Map");
  }
  if (search.shards.size > maxShards) {
    throw new TypeError(
      `emitSite: ${search.shards.size} shards exceeds maxShards=${maxShards}`,
    );
  }
  const basePath = search.basePath ?? DEFAULT_SEARCH_BASE_PATH;
  validateBasePath(basePath);

  // 1. manifest.json
  pushUnique(out, seen, {
    route: `${basePath}/manifest.json`,
    html: stableJsonStringify(search.manifest),
    contentType: CONTENT_TYPE_JSON,
  });

  // 2. shards/*.json — iterate by manifest's sorted shardKeys for
  //    determinism (we don't trust the Map's insertion order).
  const manifestShardKeys = Array.isArray(search.manifest.shardKeys)
    ? search.manifest.shardKeys
    : [];
  for (let i = 0; i < manifestShardKeys.length; i++) {
    const key = manifestShardKeys[i]!;
    if (typeof key !== "string") {
      throw new TypeError(
        `emitSite: search.manifest.shardKeys[${i}] must be a string`,
      );
    }
    validateShardKey(key, i);
    const shard = search.shards.get(key);
    if (shard === undefined) {
      throw new TypeError(
        `emitSite: search.shards missing entry for shardKey ${JSON.stringify(key)}`,
      );
    }
    pushUnique(out, seen, {
      route: `${basePath}/${key}.json`,
      html: stableJsonStringify(serialiseShard(shard)),
      contentType: CONTENT_TYPE_JSON,
    });
  }

  // 3. client.js (optional)
  if (search.clientJs !== undefined) {
    if (typeof search.clientJs !== "string") {
      throw new TypeError("emitSite: search.clientJs must be a string");
    }
    pushUnique(out, seen, {
      route: `${basePath}/client.js`,
      html: search.clientJs,
      contentType: CONTENT_TYPE_JS,
    });
  }
}

/**
 * Validate `search.basePath`:
 *   - Same route-shape rules (leading `/`, no `..`, etc.).
 *   - PLUS: no trailing `/`.  We build paths as
 *     `${basePath}/${key}.json`; a trailing slash would yield
 *     `//`.  Forbid it up front rather than papering over with a
 *     normalisation.
 */
function validateBasePath(basePath: string): void {
  validateRouteShape(basePath, "search.basePath");
  if (basePath.length > 1
      && basePath.charCodeAt(basePath.length - 1) === 0x2f /* "/" */) {
    throw new TypeError(
      `emitSite: search.basePath must not end with "/" (got ${JSON.stringify(basePath)})`,
    );
  }
}

/**
 * Shard keys are URL path segments — they must NOT contain `/`
 * (would change which directory the shard ends up in) or any of
 * the forbidden chars from route validation.  Empty shard keys
 * are also rejected (would produce `/search/.json`).
 *
 * Explicit char-by-char loop — same pattern as
 * `validateRouteShape`.
 */
function validateShardKey(key: string, i: number): void {
  if (key.length === 0) {
    throw new TypeError(`emitSite: search.manifest.shardKeys[${i}] must not be empty`);
  }
  if (key.length > 256) {
    throw new TypeError(`emitSite: search.manifest.shardKeys[${i}] exceeds 256 chars`);
  }
  for (let p = 0; p < key.length; p++) {
    const c = key.charCodeAt(p);
    if (c === 0x2f /* "/" */
        || c === 0x5c /* "\" */
        || c < 0x20
        || c === 0x7f) {
      throw new TypeError(
        `emitSite: search.manifest.shardKeys[${i}] contains forbidden char (${JSON.stringify(key)})`,
      );
    }
  }
}

/**
 * Convert an `IndexShard` (whose `postings` is a `Map`) into a
 * JSON-friendly plain shape.  Token iteration order is the
 * sorted token list — so the output bytes are stable between
 * builds with the same input even if the Map's internal
 * insertion order differs.
 */
function serialiseShard(shard: unknown): { shardKey: string; postings: Record<string, unknown> } {
  if (shard === null || typeof shard !== "object") {
    throw new TypeError("emitSite: shard must be an object");
  }
  const s = shard as { shardKey?: unknown; postings?: unknown };
  if (typeof s.shardKey !== "string") {
    throw new TypeError("emitSite: shard.shardKey must be a string");
  }
  if (!(s.postings instanceof Map)) {
    throw new TypeError("emitSite: shard.postings must be a Map");
  }
  // Pull tokens out, sort, then iterate — deterministic order.
  const tokens: string[] = [];
  for (const tok of s.postings.keys()) {
    if (typeof tok !== "string") {
      throw new TypeError("emitSite: shard.postings keys must be strings");
    }
    tokens.push(tok);
  }
  tokens.sort();
  // `Object.create(null)` — no inherited prototype, so a
  // downstream consumer doing `obj[token]` can't accidentally
  // pick up `toString` / `hasOwnProperty` etc.  JSON.stringify
  // handles null-prototype objects identically to plain ones.
  const postingsOut = Object.create(null) as Record<string, unknown>;
  for (const tok of tokens) {
    postingsOut[tok] = s.postings.get(tok);
  }
  return { shardKey: s.shardKey, postings: postingsOut };
}

/**
 * Wrapper around `JSON.stringify` that:
 *   - Uses no `indent` (compact output → smaller bundles, stable
 *     hashes when nested values change shape).
 *   - Uses no `replacer` (avoids the "replacer can mutate" footgun
 *     and the equally-real "replacer can throw on circular" one).
 *
 * Centralised so any future tweaks (e.g. enforcing sorted keys
 * across the board) land in one place.
 */
function stableJsonStringify(value: unknown): string {
  return JSON.stringify(value);
}
