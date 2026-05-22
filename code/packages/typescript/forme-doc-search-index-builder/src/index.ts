/**
 * @coding-adventures/forme-doc-search-index-builder
 *
 * Build-time inverted-index builder for the documentation-site
 * search.  Takes per-page text + metadata, tokenises each body
 * via `@coding-adventures/forme-doc-search-tokenizer`, builds a
 * `token → postings` inverted index, shards by token-prefix for
 * incremental browser loading, and emits the shards plus a
 * small bootstrap manifest.
 *
 * Pure transform.  Capabilities: `[]`.  Depends only on
 * `@coding-adventures/forme-doc-search-tokenizer` (itself
 * `[]`-capability and zero-dep).
 *
 * ```ts
 * import { buildSearchIndex } from "@coding-adventures/forme-doc-search-index-builder";
 *
 * const { shards, manifest } = buildSearchIndex([
 *   { id: "/intro",       body: "Welcome to the docs", title: "Introduction" },
 *   { id: "/guide/setup", body: "Install via npm install foo", title: "Setup Guide" },
 * ]);
 *
 * // manifest.pages      = ["/guide/setup", "/intro"]            (sorted)
 * // manifest.shardKeys  = ["do", "fo", "gu", "in", "no", "se", "we"]  (etc., sorted)
 * // shards.get("se").postings.get("setup") = [{pageId:"/guide/setup", freq:1, titleHit:true}]
 * ```
 *
 * Ninth concrete DOC00 v0 package (after frontmatter,
 * heading-anchors, toc-extractor, code-block-decorator,
 * syntax-highlighter, sidebar-builder, page-shell,
 * search-tokenizer).
 *
 * @module index
 */

export { buildSearchIndex } from "./builder.js";
export type {
  IndexPageInput,
  BuildIndexOptions,
  BuildIndexOutput,
  IndexShard,
  IndexManifest,
  IndexStats,
  Posting,
} from "./types.js";
