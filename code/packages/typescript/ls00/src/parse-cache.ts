/**
 * parse-cache.ts -- ParseCache: avoid re-parsing unchanged documents
 *
 * # Why Cache Parse Results?
 *
 * Parsing is the most expensive operation in a language server. For a large
 * file, parsing on every keystroke would lag the editor noticeably.
 *
 * The LSP protocol helps by sending a version number with every change. If the
 * document hasn't changed (same URI, same version), the parse result from the
 * previous keystroke is still valid.
 *
 * # Cache Key Design
 *
 * The cache key is `${uri}::${version}`. Version is a monotonically increasing
 * integer that the editor increments with each change. Using version in the key:
 *
 *   - Same (uri, version) -> cache hit -> return cached result
 *   - Different version   -> cache miss -> re-parse and cache new result
 *
 * The old entry is evicted when a new version is cached for the same URI.
 * This keeps memory bounded at O(open_documents) entries.
 *
 * # Concurrency
 *
 * The ParseCache is NOT designed for concurrent access. This is intentional:
 * the LspServer processes one message at a time (single-threaded event loop
 * in Node.js), so no locking is needed.
 *
 * @module
 */

import type { Diagnostic } from "./types.js";
import type { LanguageBridge } from "./language-bridge.js";

// ---------------------------------------------------------------------------
// ParseResult -- the outcome of one parse
// ---------------------------------------------------------------------------

/**
 * ParseResult holds the outcome of parsing one version of a document.
 *
 * Even on parse error, we store the partial AST and diagnostics so that other
 * features (hover, folding, symbols) can still work on the valid portions.
 */
export interface ParseResult {
  ast: unknown;            // may be null if the parser couldn't produce any AST
  diagnostics: Diagnostic[];
}

// ---------------------------------------------------------------------------
// ParseCache
// ---------------------------------------------------------------------------

/**
 * ParseCache stores the most recent parse result for each open document.
 *
 * Create one with `new ParseCache()`. Call `getOrParse()` on every feature
 * request to get the current (possibly cached) parse result.
 */
export class ParseCache {
  /** The cache maps "${uri}::${version}" -> ParseResult. */
  private cache: Map<string, ParseResult> = new Map();

  /**
   * Build the cache key from URI and version.
   *
   * We concatenate them with "::" as a separator. Since URIs don't contain "::"
   * and versions are integers, this is collision-free.
   */
  private key(uri: string, version: number): string {
    return `${uri}::${version}`;
  }

  /**
   * GetOrParse returns the parse result for (uri, version).
   *
   * If the result is already cached, it is returned immediately without calling
   * the bridge again. Otherwise, bridge.parse(source) is called, the result is
   * stored, and the previous cache entry for this URI (if any) is evicted to
   * prevent unbounded growth.
   *
   * This function is the single point of truth for "what is the parsed state of
   * this document right now?" All feature handlers call it before operating on
   * the AST.
   */
  getOrParse(uri: string, version: number, source: string, bridge: LanguageBridge): ParseResult {
    const k = this.key(uri, version);

    // Cache hit: the document hasn't changed since last parse.
    const cached = this.cache.get(k);
    if (cached !== undefined) {
      return cached;
    }

    // Cache miss: parse and store. Evict any stale entry for this URI first.
    this.evictByUri(uri);

    const [ast, diagnostics] = bridge.parse(source);

    const result: ParseResult = {
      ast,
      diagnostics: diagnostics ?? [],
    };
    this.cache.set(k, result);
    return result;
  }

  /**
   * Evict removes all cached entries for a given URI.
   *
   * Called when a document is closed (didClose) so the cache entry is cleaned up.
   */
  evict(uri: string): void {
    this.evictByUri(uri);
  }

  /**
   * Internal eviction: removes all keys whose URI prefix matches.
   */
  private evictByUri(uri: string): void {
    const prefix = `${uri}::`;
    for (const k of this.cache.keys()) {
      if (k.startsWith(prefix)) {
        this.cache.delete(k);
      }
    }
  }
}
