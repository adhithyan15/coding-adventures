/**
 * @coding-adventures/forme-render-static
 *
 * Forme render stage: `Stream<ContentNode>` → `Stream<RenderedPage>`.
 * Wraps `@coding-adventures/document-ast-to-html`, matches Style IR rules,
 * and inlines the existing AOT slicer's per-page CSS artefact.
 *
 *   consumes:    streamOf(Kinds.ContentNode)
 *   produces:    streamOf(Kinds.RenderedPage)
 *   capabilities: []                ← pure transform
 *   configSchema: { siteTitle?, siteUrl?, siteHomeRoute?,
 *                   rssRoute?, atomRoute? }
 *
 * === Why this is a stream-to-stream stage ===
 *
 * A router stage upstream owns canonical URL policy and records its
 * decision on `ContentNode.route`. The renderer consumes that route
 * directly and rejects unrouted nodes with an actionable diagnostic.
 *
 * === RenderedPage shape ===
 *
 * The kernel's `RenderedPage` (see `forme-types/shapes.ts`) is:
 *
 *     { route, html, usedStyle, usedIslands, usedAssets, meta, provenance }
 *
 * `usedStyle` records rules matched in source order, while `usedIslands: []`
 * (no interactivity) and
 * `usedAssets: []` (no asset-extraction stage yet). `provenance` records the
 * input node's logical and revision IDs; `source` remains as a temporary
 * compatibility hint for consumers of the v1.0 kind.
 *
 * `meta.title` is derived via the three-step fallback in `title.ts`:
 * `frontmatter.title` → first H1 → slug.
 *
 * === Spec adherence ===
 *
 * No deliberate divergences from FM00 / FM01. Current simplifications:
 *
 *   - Themes must be resolved before they reach this stage.
 *   - `usedIslands` / `usedAssets` remain empty.
 *   - OpenGraph and structured data remain empty; canonical URLs,
 *     descriptions, and feed discovery are emitted when `siteUrl`
 *     is configured.
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
import { createOutputProvenance } from "@coding-adventures/forme-identity";
import { toHtml } from "@coding-adventures/document-ast-to-html";
import { generateMetaLinkTags } from "@coding-adventures/forme-aot-meta-link-tags";
import { generateFeedDiscoveryLinks } from "@coding-adventures/forme-aot-rss-discovery-link";
import {
  emptyStyleDocument,
  validateStyleDocument,
  type StyleDocument,
} from "@coding-adventures/forme-style-ir";
import { slicePerPage } from "@coding-adventures/forme-aot-css-slicer";
import { slugify } from "./slug.js";
import { renderHtmlDocument } from "./theme.js";
import { deriveTitle } from "./title.js";
import { collectUsedStyle } from "./used-style.js";

/** Renderer configuration; every field is optional with deterministic defaults. */
export interface RenderStaticConfig {
  /** A validated, fully resolved StyleDocument. Omit for unstyled HTML. */
  readonly style?: StyleDocument;
  /** Style contexts to compile. Dark preference support is on by default. */
  readonly activeStyleContexts?: readonly string[];
  /** Site title for the page header.  Empty/undefined → no header. */
  readonly siteTitle?: string;
  /** Public deployment base, including a project-page prefix when present. */
  readonly siteUrl?: string;
  /** Canonical route used by the site-title link. */
  readonly siteHomeRoute?: string;
  /** Canonical RSS route advertised in article heads. */
  readonly rssRoute?: string;
  /** Canonical Atom route advertised in article heads. */
  readonly atomRoute?: string;
}

const DEFAULT_SITE_TITLE = "";

const renderStatic = defineStage({
  name: "@coding-adventures/forme-render-static",
  version: "0.2.0",
  apiVersion: 1,
  description: "Render ContentNode pages with matched Style IR and AOT-sliced CSS.",
  consumes: streamOf(Kinds.ContentNode),
  produces: streamOf(Kinds.RenderedPage),
  capabilities: [],
  configSchema: {
    type: "object",
    properties: {
      siteTitle:     { type: "string" },
      siteUrl:       { type: "string" },
      siteHomeRoute: { type: "string" },
      rssRoute:      { type: "string" },
      atomRoute:     { type: "string" },
      style:         { type: "object" },
      activeStyleContexts: {
        type: "array",
        items: { type: "string" },
      },
    },
  },
  async *run(rawInput, rawConfig, ctx) {
    const config = (rawConfig ?? {}) as RenderStaticConfig;
    const siteTitle     = config.siteTitle     ?? DEFAULT_SITE_TITLE;
    const validatedStyle = validateStyleDocument(config.style ?? emptyStyleDocument());
    const style = validatedStyle.document;
    if (style.theme !== null) {
      throw new Error(
        `forme-render-static: StyleDocument theme ${JSON.stringify(style.theme)} is unresolved; compose it before rendering`,
      );
    }
    const activeStyleContexts = config.activeStyleContexts ?? ["screen", "dark"];
    for (const warning of validatedStyle.warnings) {
      ctx.logger.warn(`forme-render-static: ${warning.message}`, { code: warning.code });
    }
    const stream = rawInput as AsyncIterable<ContentNode>;

    for await (const node of stream) {
      ctx.cancellation.throwIfCancelled();

      if (node.route === null) {
        throw new Error(
          `forme-render-static: ContentNode ${node.identity} (${node.sourcePath}) has no route; add forme-router upstream`,
        );
      }

      // Canonical routes are assigned once by forme-router. Slug
      // derivation remains solely for the final title fallback.
      const slug = slugify(node.sourcePath);
      const route = node.route;

      // Render the document body via the wrapped renderer.  Note we
      // do NOT pass `sanitize: true` — v0 trusts authored Markdown
      // (this is your own blog, you wrote the posts).  Real
      // multi-tenant systems should wire the sanitizer in between
      // parser and renderer; documented in README.
      const bodyHtml = toHtml(node.document);

      // Title derivation: frontmatter.title → first H1 → slug.
      const title = deriveTitle(node, slug);
      const usedStyle = collectUsedStyle(node.document, style, {
        siteHeader: siteTitle.length > 0,
        frontmatter: node.frontmatter,
      });
      const cssArtifact = slicePerPage(style, [{ id: route, usedRuleIds: usedStyle }], {
        activeContexts: activeStyleContexts,
        // CSS is inlined in a single page, so no cross-page selector scope is
        // needed. Going through slicePerPage still preserves the AOT contract.
        scopePrefix: () => "",
      }).artefacts.get(route)!;
      for (const warning of cssArtifact.warnings) {
        ctx.logger.warn(`forme-render-static: ${warning.message}`, {
          code: warning.code,
          ...(warning.ruleId === undefined ? {} : { ruleId: warning.ruleId }),
        });
      }

      const description = stringFromFrontmatter(node.frontmatter, "excerpt");
      const canonicalUrl = config.siteUrl === undefined
        ? null
        : publicUrl(config.siteUrl, route);
      const headParts: string[] = [];
      if (canonicalUrl !== null || description !== null) {
        headParts.push(generateMetaLinkTags({
          ...(canonicalUrl === null ? {} : { canonical: canonicalUrl }),
          ...(description === null ? {} : {
            meta: [{ name: "description", content: description }],
          }),
        }));
      }
      if (config.siteUrl !== undefined && (config.rssRoute !== undefined || config.atomRoute !== undefined)) {
        headParts.push(generateFeedDiscoveryLinks([
          ...(config.rssRoute === undefined ? [] : [{
            href: publicUrl(config.siteUrl, config.rssRoute),
            type: "application/rss+xml" as const,
            title: `${siteTitle || title} RSS`,
          }]),
          ...(config.atomRoute === undefined ? [] : [{
            href: publicUrl(config.siteUrl, config.atomRoute),
            type: "application/atom+xml" as const,
            title: `${siteTitle || title} Atom`,
          }]),
        ]));
      }

      // Wrap in the theme-agnostic HTML5 shell with only this page's CSS.
      const html = renderHtmlDocument({
        title,
        siteTitle,
        bodyHtml,
        siteHref: config.siteUrl === undefined
          ? "/"
          : publicUrl(config.siteUrl, config.siteHomeRoute ?? "/"),
        headHtml: headParts.filter(Boolean).join("\n"),
        styleCss: cssArtifact.css,
        supportsDarkMode: activeStyleContexts.includes("dark"),
      });

      const meta: PageMeta = {
        title,
        description,
        canonicalUrl,
        openGraph: {},
        structured: [],
        extra: {},
      };

      const page: RenderedPage = {
        route,
        html,
        usedStyle,
        usedIslands: [],
        usedAssets: [],
        meta,
        provenance: createOutputProvenance([node]),
        source: node.identity,
      };
      yield page as never;
    }

    ctx.logger.debug("forme-render-static: stream complete");
  },
});

function stringFromFrontmatter(
  frontmatter: ContentNode["frontmatter"],
  key: string,
): string | null {
  const value = frontmatter[key];
  return typeof value === "string" && value.length > 0 ? value : null;
}

/** Compose a portable route with its deployment base without URL resetting. */
export function publicUrl(siteUrl: string, route: string): string {
  const base = siteUrl.replace(/\/+$/, "");
  const path = route.startsWith("/") ? route : `/${route}`;
  return `${base}${path}`;
}

export default renderStatic;
export { renderStatic, slugify, deriveTitle, renderHtmlDocument, collectUsedStyle };
