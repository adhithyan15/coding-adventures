/**
 * @coding-adventures/forme-render-static
 *
 * Forme render stage: `Stream<ContentNode>` → `Stream<RenderedPage>`.
 * Wraps `@coding-adventures/document-ast-to-html` and injects a
 * minimal classless HTML5 theme (see `theme.ts`).
 *
 *   consumes:    streamOf(Kinds.ContentNode)
 *   produces:    streamOf(Kinds.RenderedPage)
 *   capabilities: []                ← pure transform
 *   configSchema: { siteTitle?: string; routeTemplate?: string }
 *
 * === Why this is a stream-to-stream stage ===
 *
 * The "purest" topology would have the collector emit a single
 * `Collection` and the renderer consume it (so routes derived by the
 * collector flow into the renderer directly).  For v0 we take the
 * simpler shape: render each `ContentNode` as it arrives, derive the
 * route locally from `sourcePath` using the same `slugify` +
 * `formatRoute` helpers the collector uses.
 *
 * Trade-off: the renderer duplicates ~30 lines of slug logic.  Win:
 * the rendering stage doesn't have to consume a different Kind from
 * what the parser emits — every Markdown-handling pipeline today
 * looks like `source → parse → render → emit` (no collector in the
 * critical path for individual pages).  v0.2 will introduce a router
 * stage that folds collection-side routes back onto
 * `ContentNode.route` so the renderer can read it directly.
 *
 * The duplication is called out explicitly in `slug.ts` and in the
 * README; do NOT extract a shared package for it until a *third*
 * stage needs the same helper (FM02 plugin packaging will give us
 * that excuse cleanly).
 *
 * === RenderedPage shape ===
 *
 * The kernel's `RenderedPage` (see `forme-types/shapes.ts`) is:
 *
 *     { route, html, usedStyle, usedIslands, usedAssets, meta, source }
 *
 * v0 emits `usedStyle: []` (no Style IR yet — the theme CSS is
 * inlined as one string), `usedIslands: []` (no interactivity), and
 * `usedAssets: []` (no asset-extraction stage yet).  `source` holds
 * the input `ContentNode.identity` so downstream stages can trace
 * back to the source.
 *
 * `meta.title` is derived via the three-step fallback in `title.ts`:
 * `frontmatter.title` → first H1 → slug.
 *
 * === Spec adherence ===
 *
 * No deliberate divergences from FM00 / FM01.  v0 simplifications:
 *
 *   - Single hard-coded theme (no Style IR).
 *   - Routes derived locally from `sourcePath` (no router stage).
 *   - `usedStyle` / `usedIslands` / `usedAssets` empty.
 *   - `meta.description`, `meta.openGraph`, `meta.structured`,
 *     `meta.canonicalUrl` left empty/null (richer head metadata is a
 *     later concern).
 *
 * @module index
 */

import {
  Kinds,
  streamOf,
  type ContentNode,
  type RenderedPage,
  type PageMeta,
} from "@coding-adventures/forme-types";
import { defineStage } from "@coding-adventures/forme-stage";
import { toHtml } from "@coding-adventures/document-ast-to-html";
import { slugify, formatRoute } from "./slug.js";
import { renderHtmlDocument } from "./theme.js";
import { deriveTitle } from "./title.js";

/** v0 config surface — every field optional with sensible defaults. */
export interface RenderStaticConfig {
  /** Site title for the page header.  Empty/undefined → no header. */
  readonly siteTitle?: string;
  /** Route template — must contain `{slug}`.  Default `/blog/{slug}.html`. */
  readonly routeTemplate?: string;
}

const DEFAULT_ROUTE_TEMPLATE = "/blog/{slug}.html";
const DEFAULT_SITE_TITLE = "";

const renderStatic = defineStage({
  name: "@coding-adventures/forme-render-static",
  version: "0.1.0",
  apiVersion: 1,
  description: "Render each ContentNode as a self-contained HTML page with a classless theme.",
  consumes: streamOf(Kinds.ContentNode),
  produces: streamOf(Kinds.RenderedPage),
  capabilities: [],
  configSchema: {
    type: "object",
    properties: {
      siteTitle:     { type: "string" },
      routeTemplate: { type: "string" },
    },
  },
  async *run(rawInput, rawConfig, ctx) {
    const config = (rawConfig ?? {}) as RenderStaticConfig;
    const siteTitle     = config.siteTitle     ?? DEFAULT_SITE_TITLE;
    const routeTemplate = config.routeTemplate ?? DEFAULT_ROUTE_TEMPLATE;
    const stream = rawInput as AsyncIterable<ContentNode>;

    for await (const node of stream) {
      ctx.cancellation.throwIfCancelled();

      // Derive the route from sourcePath using the same rules the
      // collector uses, so both produce identical routes for the
      // same input (until the v0.2 router stage lands).
      const slug = slugify(node.sourcePath);
      const route = formatRoute(routeTemplate, slug);

      // Render the document body via the wrapped renderer.  Note we
      // do NOT pass `sanitize: true` — v0 trusts authored Markdown
      // (this is your own blog, you wrote the posts).  Real
      // multi-tenant systems should wire the sanitizer in between
      // parser and renderer; documented in README.
      const bodyHtml = toHtml(node.document);

      // Title derivation: frontmatter.title → first H1 → slug.
      const title = deriveTitle(node, slug);

      // Wrap in the full HTML5 document with the classless theme.
      const html = renderHtmlDocument({ title, siteTitle, bodyHtml });

      const meta: PageMeta = {
        title,
        description: null,
        canonicalUrl: null,
        openGraph: {},
        structured: [],
        extra: {},
      };

      const page: RenderedPage = {
        route,
        html,
        usedStyle: [],
        usedIslands: [],
        usedAssets: [],
        meta,
        source: node.identity,
      };
      yield page as never;
    }

    ctx.logger.debug("forme-render-static: stream complete");
  },
});

export default renderStatic;
export { renderStatic, slugify, formatRoute, deriveTitle, renderHtmlDocument };
