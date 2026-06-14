/**
 * @coding-adventures/forme-doc-search-client-js
 *
 * Browser-side search client logic for the documentation-site
 * search.  Consumes the manifest from
 * `forme-doc-search-index-builder` plus an INJECTED
 * shard-fetcher callback; loads shards on demand, tokenises
 * queries via `forme-doc-search-tokenizer` (using the flags
 * baked into the manifest so client and index agree), merges
 * postings, ranks results, returns the top N.
 *
 * The shard-fetcher is INJECTED — the caller provides the
 * actual `fetch()` (browser) or `fs.readFile` (Node test)
 * wrapper.  This package has capabilities `[]` — the
 * net:fetch / fs:read capability lives with the caller.
 *
 * ```ts
 * import { SearchClient } from "@coding-adventures/forme-doc-search-client-js";
 *
 * const client = new SearchClient({
 *   manifest,                                 // from build time
 *   fetchShard: async (key) => {              // browser caller injects fetch
 *     const r = await fetch(`/search/${key}.json`);
 *     return await r.json();
 *   },
 * });
 *
 * const results = await client.search("install");
 * // → [{ pageId, score, matchedTokens }, ...]
 * ```
 *
 * Tenth concrete DOC00 v0 package (after frontmatter,
 * heading-anchors, toc-extractor, code-block-decorator,
 * syntax-highlighter, sidebar-builder, page-shell,
 * search-tokenizer, search-index-builder).
 *
 * @module index
 */

export { SearchClient } from "./client.js";
export type {
  ShardFetcher,
  SearchClientOptions,
  SearchQueryOptions,
  SearchResult,
} from "./types.js";
