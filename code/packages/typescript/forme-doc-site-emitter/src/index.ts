/**
 * @coding-adventures/forme-doc-site-emitter
 *
 * Eleventh and FINAL DOC00 v0 package — the GLUE that turns
 * DOC00's per-stage outputs into a `PageBundleConfig` for the
 * FM00 deploy chain.
 *
 * Pure transform.  Capabilities: `[]`.  Per DOC00 spec section 8:
 * "Every DOC00 package has required_capabilities.json ->
 * capabilities: []. No exceptions in v0."  Site-emitter never
 * instantiates any I/O primitive — it returns a data structure.
 * The actual disk writes happen downstream in
 * `forme-aot-page-bundle-emitter` ->
 * `forme-aot-deploy-manifest-emitter` -> `forme-deploy-runner`,
 * which already own the relevant capabilities.
 *
 * ```ts
 * import { emitSite } from "@coding-adventures/forme-doc-site-emitter";
 * import { generatePageBundle } from "@coding-adventures/forme-aot-page-bundle-emitter";
 *
 * const bundle = emitSite({
 *   pages: [
 *     { route: "/",           html: indexHtml },
 *     { route: "/guide/setup", html: setupHtml, lastmod: "2026-05-22" },
 *   ],
 *   sidebar,                       // from forme-doc-sidebar-builder
 *   search: { manifest, shards },  // from forme-doc-search-index-builder
 *   baseUrl: "https://example.com",
 * });
 *
 * const manifestJson = generatePageBundle(bundle);
 * // → JSON manifest with sorted routes; deploy runner consumes
 * //   it and writes the files.
 * ```
 *
 * @module index
 */

export { emitSite } from "./emitter.js";
export {
  DEFAULT_SIDEBAR_PATH,
  DEFAULT_SEARCH_BASE_PATH,
  DEFAULT_MAX_PAGES,
  DEFAULT_MAX_SHARDS,
  DEFAULT_MAX_EXTRAS,
  CONTENT_TYPE_JSON,
  CONTENT_TYPE_JS,
} from "./emitter.js";
export type {
  DocPage,
  ExtraFile,
  SearchAssets,
  SiteEmitInput,
  SidebarTree,
  PageBundleConfig,
} from "./types.js";
