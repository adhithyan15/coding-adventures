/**
 * cache.ts — `createIncrementalCache(io)` + `sliceWithCache(...)`
 * (FM06 §4 incremental rebuilds).
 *
 * Wraps `forme-aot-css-slicer.slicePerPage` with content-addressed
 * caching.  On each call:
 *
 *   1. Derive a deterministic cache key from
 *      `(canonicalStyleDocument(doc), sort(usedRuleIds), sort(activeContexts))`.
 *   2. Consult the caller-supplied `CacheIO.get(key)`.
 *   3. **Hit** → return the cached `CssArtifact` (with
 *      `cacheHit: true, cacheKey`).
 *   4. **Miss** → run the slicer, persist via `CacheIO.put(...)`,
 *      return the fresh artefact (with `cacheHit: false, cacheKey`).
 *
 * The IO surface is injected so this package can stay capability-
 * minimal (`["hash"]` only).  Production callers wire `CacheIO` to
 * `node:fs` or a network store in their own package — declaring
 * `"fs"` / `"net"` capability there, not here.
 *
 * ## Determinism of cache keys
 *
 * The key inputs go through a strict ordering before hashing so
 * permutations of the same input collapse to one key:
 *
 *   - `canonicalStyleDocument(doc)` — FM04 §12 byte-stable JSON
 *     (sorted keys at every depth).
 *   - `usedRuleIds` — sorted lexicographically.
 *   - `activeContexts` — sorted lexicographically.
 *
 * Together: `sha256(canonical + "\n" + sortedUsedJson + "\n" +
 * sortedCtxJson)`.  Separators chosen as raw `\n` so the boundary
 * bytes are unambiguous (none of the three components contain raw
 * newlines after canonicalisation).
 *
 * ## Why per-page granularity?
 *
 * Each page's `usedRuleIds` is independent; a doc change that only
 * touches rules NOT used by page A leaves page A's cache key
 * unchanged.  Pages B and C with overlapping `usedRuleIds` might
 * share a single cached artefact (same key → one entry).
 *
 * @module cache
 */

import { createHash } from "node:crypto";
import {
  slicePerPage, defaultScopePrefix,
  type CssArtifact, type PageSlice, type SliceOptions,
} from "@coding-adventures/forme-aot-css-slicer";
import { translateToCss } from "@coding-adventures/forme-style-to-css";
import {
  canonicalStyleDocument,
  type StyleDocument, type StyleRuleId,
} from "@coding-adventures/forme-style-ir";

// ─── Public types ────────────────────────────────────────────────────────

/**
 * Storage surface the cache delegates to.  All three methods are
 * async to let production callers wire to disk / network without
 * blocking.
 */
export interface CacheIO {
  /** Returns the stored value for `key`, or `null` if absent. */
  get(key: string): Promise<string | null>;
  /**
   * Persist `value` under `key`.  `meta` carries optional metadata
   * (page id, byte size, etc.) that file-backed IO implementations
   * may surface in directory listings; in-memory IO can ignore it.
   */
  put(key: string, value: string, meta: CachePutMeta): Promise<void>;
  /** Enumerate every key in the store (used by tests + admin tools). */
  list(): Promise<readonly string[]>;
}

/** Optional metadata passed to `CacheIO.put` for surfacing only. */
export interface CachePutMeta {
  readonly pageId: string;
  readonly byteSize: number;
  readonly sha256: string;
}

/** What `sliceWithCache` produces per page. */
export interface CacheArtifact extends CssArtifact {
  /** True if this page was served from cache; false if freshly computed. */
  readonly cacheHit: boolean;
  /** The cache key used (deterministic, exposed for debugging / dedup). */
  readonly cacheKey: string;
}

/** Top-level result.  Map preserves caller's page order. */
export interface SliceWithCacheResult {
  readonly artefacts: ReadonlyMap<string, CacheArtifact>;
}

/** The cache instance returned by `createIncrementalCache`. */
export interface IncrementalCache {
  /**
   * Slice the document into per-page CSS artefacts, consulting the
   * cache before recomputing.
   */
  sliceWithCache(
    doc: StyleDocument,
    pages: readonly PageSlice[],
    options: SliceOptions,
  ): Promise<SliceWithCacheResult>;

  /**
   * Compute the deterministic cache key for a single (doc, page,
   * activeContexts) triple — exposed so callers can pre-check
   * cache contents, prime entries, or build dependency graphs
   * without invoking the slicer.
   */
  cacheKey(
    doc: StyleDocument,
    usedRuleIds: readonly StyleRuleId[],
    activeContexts: readonly string[],
  ): string;
}

// ─── Public factory ──────────────────────────────────────────────────────

/**
 * Construct an `IncrementalCache` wired to the supplied `CacheIO`.
 * Each call returns an independent instance; instances do NOT share
 * state, so per-project / per-tenant isolation is a one-liner.
 */
export function createIncrementalCache(io: CacheIO): IncrementalCache {
  return {
    cacheKey: (doc, usedRuleIds, activeContexts) =>
      computeCacheKey(doc, usedRuleIds, activeContexts),

    sliceWithCache: async (doc, pages, options) => {
      const artefacts = new Map<string, CacheArtifact>();
      const scopeFn = options.scopePrefix ?? defaultScopePrefix;

      // We compute one cache key per page — pages with identical
      // usedRuleIds collapse to one key on lookup but still get
      // separate scoped CSS deliverables (per-page scope differs).
      //
      // The cache stores the UNSCOPED CSS bytes (so two pages that
      // share usedRuleIds also share one cache entry).  On serve
      // we just call translateToCss a SECOND time with the scope
      // applied — same cost as a fresh compute for the scoped
      // pass, no reverse-engineering of selector strings.

      for (const page of pages) {
        const key = computeCacheKey(doc, page.usedRuleIds, options.activeContexts);
        const cached = await io.get(key);

        const scope = scopeFn(page.id);

        if (cached !== null) {
          // CACHE HIT — re-decode the stored CacheEntry, then re-scope
          // via a fresh translateToCss call (same shape as the slicer's
          // scoped pass; avoids reverse-engineering CSS).
          const entry = parseCacheEntry(cached);
          if (entry !== null) {
            const scoped = translateToCss(doc, {
              activeContexts: options.activeContexts,
              usedRuleIds: page.usedRuleIds,
              scope,
            });
            artefacts.set(page.id, {
              pageId: page.id,
              css: scoped.output,
              emittedRules: scoped.emittedRules,
              warnings: scoped.warnings,
              byteSize: Buffer.byteLength(scoped.output, "utf8"),
              sha256: entry.sha256,
              cacheHit: true,
              cacheKey: key,
            });
            continue;
          }
          // Malformed cache entry — fall through to fresh compute.
        }

        // CACHE MISS — run the slicer for this one page.
        const fresh = slicePerPage(doc, [page], options);
        const art = fresh.artefacts.get(page.id)!;

        // To populate the cache we need the UNSCOPED CSS.  The slicer
        // computed it internally (for its sha256) but doesn't expose
        // it; recomputing here is the same constant-factor cost as
        // the slicer's own unscoped pass.
        const unscoped = translateToCss(doc, {
          activeContexts: options.activeContexts,
          usedRuleIds: page.usedRuleIds,
        });
        const entry: CacheEntry = {
          unscopedCss: unscoped.output,
          emittedRules: art.emittedRules,
          warnings: art.warnings,
          sha256: art.sha256,
        };
        await io.put(key, serializeCacheEntry(entry), {
          pageId: page.id,
          byteSize: art.byteSize,
          sha256: art.sha256,
        });

        artefacts.set(page.id, {
          ...art,
          cacheHit: false,
          cacheKey: key,
        });
      }

      return { artefacts };
    },
  };
}

// ─── Cache-key derivation ────────────────────────────────────────────────

function computeCacheKey(
  doc: StyleDocument,
  usedRuleIds: readonly StyleRuleId[],
  activeContexts: readonly string[],
): string {
  // Order-stable canonical inputs:
  //   - canonicalStyleDocument is FM04 §12 sorted-keys output.
  //   - sortStrings produces a fresh sorted copy (we don't mutate
  //     the caller's array).
  const canonical = canonicalStyleDocument(doc);
  const sortedUsed = JSON.stringify(sortStrings(usedRuleIds as readonly string[]));
  const sortedCtx  = JSON.stringify(sortStrings(activeContexts));

  // `\n` separator: canonical JSON never embeds a raw newline
  // (JSON encoding escapes them inside string literals); sortStrings
  // output is `["a","b","c"]` with no newlines.  So the separator
  // is unambiguous and the three components are uniquely recoverable
  // by splitting on `\n` — though we never need to.
  const blob = `${canonical}\n${sortedUsed}\n${sortedCtx}`;
  return createHash("sha256").update(blob, "utf8").digest("hex");
}

function sortStrings(xs: readonly string[]): readonly string[] {
  return [...xs].sort();
}

// ─── Cache entry serialisation ───────────────────────────────────────────

interface CacheEntry {
  readonly unscopedCss: string;
  readonly emittedRules: readonly StyleRuleId[];
  readonly warnings: readonly CssArtifact["warnings"][number][];
  readonly sha256: string;
}

function serializeCacheEntry(e: CacheEntry): string {
  // JSON is fine — entries are small (one page's CSS + metadata).
  // We don't compress; production CacheIO implementations can layer
  // compression at the storage boundary if they want.
  return JSON.stringify({
    unscopedCss: e.unscopedCss,
    emittedRules: e.emittedRules,
    warnings: e.warnings,
    sha256: e.sha256,
  });
}

function parseCacheEntry(s: string): CacheEntry | null {
  try {
    const v: unknown = JSON.parse(s);
    if (typeof v !== "object" || v === null) return null;
    const obj = v as Record<string, unknown>;
    if (typeof obj.unscopedCss !== "string") return null;
    if (typeof obj.sha256 !== "string") return null;
    if (!Array.isArray(obj.emittedRules)) return null;
    if (!Array.isArray(obj.warnings)) return null;
    return {
      unscopedCss: obj.unscopedCss,
      emittedRules: obj.emittedRules as readonly StyleRuleId[],
      warnings: obj.warnings as CacheEntry["warnings"],
      sha256: obj.sha256,
    };
  } catch {
    return null;
  }
}

// ─── In-memory IO (convenience) ──────────────────────────────────────────

/**
 * A trivial in-memory `CacheIO` for tests and dev-mode usage.  No
 * eviction, no concurrency control — the caller's responsibility.
 */
export function createMemoryCacheIO(): CacheIO & { clear(): void; size(): number } {
  const store = new Map<string, string>();
  return {
    get: async (key) => (store.has(key) ? store.get(key)! : null),
    put: async (key, value) => { store.set(key, value); },
    list: async () => Object.freeze([...store.keys()].sort()),
    clear: () => { store.clear(); },
    size: () => store.size,
  };
}
