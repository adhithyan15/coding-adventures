/**
 * types.ts — public signatures for the documentation-site emitter.
 *
 * Site-emitter is the GLUE PACKAGE for the DOC00 cluster: it takes
 * the per-stage outputs (rendered HTML pages from page-shell, the
 * sidebar tree from sidebar-builder, the search manifest + shards
 * from search-index-builder, optionally a pre-bundled browser
 * client.js for forme-doc-search-client-js) and composes a single
 * `PageBundleConfig` that the FM00 deploy chain consumes unchanged.
 *
 * Why a separate package?  Because each upstream package solves one
 * problem; tying them together with consistent route conventions,
 * deterministic JSON shapes, and validation belongs in one place,
 * not scattered across the caller.
 *
 * Pure transform.  No I/O primitive instantiated — the disk-write
 * capability lives with the deploy runner downstream.
 *
 * @module types
 */

import type { SidebarEntry } from "@coding-adventures/forme-doc-sidebar-builder";
import type {
  IndexManifest,
  IndexShard,
} from "@coding-adventures/forme-doc-search-index-builder";

/**
 * The sidebar tree shape — `SidebarEntry[]` from the upstream
 * `forme-doc-sidebar-builder` (re-aliased for clarity at the
 * site-emitter API boundary).
 */
export type SidebarTree = readonly SidebarEntry[];

/**
 * One rendered documentation page — what the caller has after
 * running the content pipeline through `forme-doc-page-shell` (and
 * before that, parsing markdown, applying heading anchors, the TOC
 * extractor, code-block decorator, and the syntax highlighter).
 *
 *   - `route`    — root-relative URL path (e.g. `"/"`, `"/guide/setup"`).
 *                  MUST start with `/`; MUST NOT contain `..`, `//`,
 *                  or `\`.  This is the route the FM00 deploy chain
 *                  installs the page at.
 *   - `html`     — the complete HTML document for the page (output
 *                  of `forme-doc-page-shell` wrapped by
 *                  `forme-aot-html-doc-emitter`, typically).
 *                  Passed through as a string — site-emitter never
 *                  parses or modifies the HTML.
 *   - `lastmod`  — optional ISO 8601 timestamp; forwarded into the
 *                  `PageEntry.lastmod` field on the bundle.
 */
export interface DocPage {
  readonly route: string;
  readonly html: string;
  readonly lastmod?: string;
}

/**
 * Inputs to the search engine slice.
 *
 *   - `manifest` — the small bootstrap manifest the browser loads
 *                  first.  Serialised to `<basePath>/manifest.json`.
 *   - `shards`   — the inverted-index shards.  Each shard becomes
 *                  `<basePath>/<shardKey>.json`.  Order in the
 *                  output bundle is by sorted `shardKey` for
 *                  determinism.
 *   - `clientJs` — optional pre-bundled browser JS for the
 *                  search client.  Caller-provided because the
 *                  bundling step lives outside the DOC00 cluster.
 *                  Emitted as `<basePath>/client.js` when present.
 *   - `basePath` — optional URL path prefix; defaults to
 *                  `"/search"`.  Leading slash required, trailing
 *                  slash forbidden.  This MUST line up with the
 *                  client-side `fetch("/search/${key}.json")` glue
 *                  the caller writes — the path is part of the
 *                  contract.
 */
export interface SearchAssets {
  readonly manifest: IndexManifest;
  readonly shards: ReadonlyMap<string, IndexShard>;
  readonly clientJs?: string;
  readonly basePath?: string;
}

/**
 * One arbitrary additional file the caller wants in the bundle —
 * for favicons, `robots.txt`, copied images, `CNAME`, etc.
 *
 *   - `route`        — root-relative URL.  Same rules as `DocPage.route`.
 *   - `content`      — body, as a string.  Binary files are not
 *                      supported in v0 (the upstream
 *                      `PageBundleConfig.PageEntry` schema takes
 *                      strings); for v1 we may add a `bytes` variant.
 *   - `contentType`  — MIME type.  No allowlist — pure pass-through
 *                      string (the page-bundle-emitter accepts any
 *                      string for content-type).
 *   - `lastmod`      — optional ISO 8601 timestamp.
 */
export interface ExtraFile {
  readonly route: string;
  readonly content: string;
  readonly contentType: string;
  readonly lastmod?: string;
}

/**
 * The complete site-emit input.
 *
 *   - `pages`    — the documentation pages.  May be empty (will
 *                  produce a bundle with only the sidebar / search
 *                  / extras, which is unusual but legal).
 *   - `sidebar`  — optional sidebar tree.  When provided, emitted
 *                  as `<sidebarPath>` (default `/sidebar.json`).
 *                  Same shape as `forme-doc-sidebar-builder`
 *                  returns.
 *   - `sidebarPath` — override default sidebar route.  Leading `/`
 *                  required; pathname only (no `?` / `#`).
 *   - `search`   — optional search-engine assets.  Omitted entirely
 *                  if the site doesn't ship search.
 *   - `extras`   — optional additional files (favicons, robots, ...).
 *   - `baseUrl`  — optional canonical site URL.  Validated only as
 *                  "starts with `http://` or `https://`" — same rule
 *                  page-bundle-emitter applies.  Forwarded into the
 *                  output `PageBundleConfig.baseUrl`.
 *
 *   - `maxPages`     — optional cap (default 100_000).  Inputs
 *                      beyond this throw.  Defends against
 *                      pathological / runaway inputs.
 *   - `maxShards`    — optional cap (default 10_000).
 *   - `maxExtras`    — optional cap (default 10_000).
 */
export interface SiteEmitInput {
  readonly pages: readonly DocPage[];
  readonly sidebar?: SidebarTree;
  readonly sidebarPath?: string;
  readonly search?: SearchAssets;
  readonly extras?: readonly ExtraFile[];
  readonly baseUrl?: string;
  readonly maxPages?: number;
  readonly maxShards?: number;
  readonly maxExtras?: number;
}

/**
 * Re-export the upstream type so callers don't need a separate
 * `import { PageBundleConfig }` from page-bundle-emitter.
 */
export type { PageBundleConfig } from "@coding-adventures/forme-aot-page-bundle-emitter";
