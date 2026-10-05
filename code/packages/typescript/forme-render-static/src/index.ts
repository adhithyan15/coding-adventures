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
 * `usedStyle` records rules matched in source order. Per-route validated FM05
 * documents resolve authored element IDs and populate exact `usedIslands`
 * plus reviewed, SHA-256-bound module-asset uses. Resolved assets become collision-free
 * emitter placeholders and populate `usedAssets`. `provenance` records the
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
 *   - Routes without configured Interactivity IR remain zero-JavaScript.
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
import { types as utilTypes } from "node:util";
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
import { rewriteAssetReferences } from "./asset-references.js";
import { slugify } from "./slug.js";
import { renderHtmlDocument } from "./theme.js";
import { deriveTitle } from "./title.js";
import { collectUsedStyle } from "./used-style.js";
import {
  prepareInteractivity,
  selectPageInteractivity,
  type ReviewedIslandModule,
  type RouteInteractivity,
} from "./interactivity.js";

/** Renderer configuration; every field is optional with deterministic defaults. */
export interface RenderStaticConfig {
  /** A validated, fully resolved StyleDocument. Omit for unstyled HTML. */
  readonly style?: StyleDocument;
  /** Style contexts to compile. Dark preference support is on by default. */
  readonly activeStyleContexts?: readonly string[];
  /** Validated per-route FM05 documents supplied by the product composition boundary. */
  readonly interactivity?: readonly RouteInteractivity[];
  /** Reviewed package/export to exact script-content mappings. */
  readonly islandModules?: readonly ReviewedIslandModule[];
  /** Explicit authority to emit reviewed executable assets. */
  readonly allowExecutableAssets?: boolean;
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
  version: "1.0.0",
  apiVersion: 2,
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
      interactivity: { type: "array", items: { type: "object" } },
      islandModules: { type: "array", items: { type: "object" } },
      allowExecutableAssets: { type: "boolean" },
    },
  },
  async *run(rawInput, rawConfig, ctx) {
    const config = snapshotRenderStaticConfig(rawConfig);
    const siteTitle     = config.siteTitle     ?? DEFAULT_SITE_TITLE;
    const validatedStyle = validateStyleDocument(config.style ?? emptyStyleDocument());
    const style = validatedStyle.document;
    if (style.theme !== null) {
      throw new Error(
        `forme-render-static: StyleDocument theme ${JSON.stringify(style.theme)} is unresolved; compose it before rendering`,
      );
    }
    const activeStyleContexts = config.activeStyleContexts ?? ["screen", "dark"];
    const preparedInteractivity = prepareInteractivity(config.interactivity, config.islandModules);
    if (preparedInteractivity.modules.size !== 0 && config.allowExecutableAssets !== true) {
      throw new Error(
        "forme-render-static: islandModules require allowExecutableAssets: true because they authorize executable output",
      );
    }
    for (const warning of validatedStyle.warnings) {
      ctx.logger.warn(`forme-render-static: ${warning.message}`, { code: warning.code });
    }
    const stream = rawInput as AsyncIterable<ContentNode>;
    const renderedInteractivityRoutes = new Set<string>();

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
      const renderDocument = rewriteAssetReferences(node.document, node.assetRefs);
      const bodyHtml = toHtml(renderDocument);
      const interactivity = selectPageInteractivity(route, bodyHtml, preparedInteractivity);
      if (preparedInteractivity.byRoute.has(route)) renderedInteractivityRoutes.add(route);

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
        usedIslands: interactivity.usedIslands,
        islandModules: interactivity.islandModules,
        usedAssets: [...new Set([
          ...node.assetRefs.filter(ref => ref.role === "image").map(ref => ref.id),
          ...interactivity.moduleAssets,
        ])],
        meta,
        provenance: createOutputProvenance([node]),
      };
      yield page as never;
    }

    for (const route of preparedInteractivity.byRoute.keys()) {
      if (!renderedInteractivityRoutes.has(route)) {
        throw new Error(
          `forme-render-static: configured interactivity route ${route.length > 64 ? `${JSON.stringify(route.slice(0, 64))}…` : JSON.stringify(route)} did not match a rendered page`,
        );
      }
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
export type { ReviewedIslandModule, RouteInteractivity } from "./interactivity.js";

const CONFIG_KEYS = [
  "style", "activeStyleContexts", "interactivity", "islandModules",
  "allowExecutableAssets", "siteTitle", "siteUrl", "siteHomeRoute", "rssRoute", "atomRoute",
] as const;

function snapshotRenderStaticConfig(rawConfig: unknown): RenderStaticConfig {
  if (rawConfig === undefined || rawConfig === null) return Object.freeze({});
  if (typeof rawConfig !== "object" || Array.isArray(rawConfig) || utilTypes.isProxy(rawConfig)) {
    throw new TypeError("forme-render-static: config must be a plain object");
  }
  const prototype = Object.getPrototypeOf(rawConfig);
  if (prototype !== Object.prototype && prototype !== null) {
    throw new TypeError("forme-render-static: config must be a plain object");
  }
  if (Object.getOwnPropertySymbols(rawConfig).length !== 0) {
    throw new TypeError("forme-render-static: config must not contain symbol keys");
  }
  const allowed = new Set<string>(CONFIG_KEYS);
  for (const key of Object.getOwnPropertyNames(rawConfig)) {
    if (!allowed.has(key)) throw new TypeError(`forme-render-static: unknown config key ${safeKey(key)}`);
  }
  const copy: Record<string, unknown> = Object.create(null);
  for (const key of CONFIG_KEYS) {
    const descriptor = Object.getOwnPropertyDescriptor(rawConfig, key);
    if (descriptor === undefined) continue;
    if (!("value" in descriptor)) {
      throw new TypeError(`forme-render-static: config.${key} must not be an accessor`);
    }
    copy[key] = descriptor.value;
  }
  for (const key of ["siteTitle", "siteUrl", "siteHomeRoute", "rssRoute", "atomRoute"] as const) {
    const value = copy[key];
    if (value !== undefined && (typeof value !== "string" || value.length > 8_192)) {
      throw new TypeError(`forme-render-static: config.${key} must be a string of at most 8192 characters`);
    }
  }
  if (copy.allowExecutableAssets !== undefined && typeof copy.allowExecutableAssets !== "boolean") {
    throw new TypeError("forme-render-static: config.allowExecutableAssets must be a boolean");
  }
  if (copy.activeStyleContexts !== undefined) {
    copy.activeStyleContexts = snapshotStringArray(copy.activeStyleContexts, "config.activeStyleContexts", 64);
  }
  return Object.freeze(copy) as RenderStaticConfig;
}

function snapshotStringArray(value: unknown, path: string, maximum: number): readonly string[] {
  if (!Array.isArray(value) || utilTypes.isProxy(value) || value.length > maximum) {
    throw new TypeError(`forme-render-static: ${path} must be an array of at most ${maximum} strings`);
  }
  if (Object.getOwnPropertySymbols(value).length !== 0) {
    throw new TypeError(`forme-render-static: ${path} must not contain symbol keys`);
  }
  const result: string[] = [];
  for (let index = 0; index < value.length; index++) {
    const descriptor = Object.getOwnPropertyDescriptor(value, String(index));
    if (descriptor === undefined || !("value" in descriptor) || typeof descriptor.value !== "string") {
      throw new TypeError(`forme-render-static: ${path}[${index}] must be a string data property`);
    }
    result.push(descriptor.value);
  }
  for (const key of Object.getOwnPropertyNames(value)) {
    if (key === "length") continue;
    const index = Number(key);
    if (!Number.isInteger(index) || index < 0 || index >= value.length || String(index) !== key) {
      throw new TypeError(`forme-render-static: ${path} contains unknown property ${safeKey(key)}`);
    }
  }
  return Object.freeze(result);
}

function safeKey(value: string): string {
  let result = '"';
  let count = 0;
  for (const point of value) {
    if (count++ === 64) return `${result}..."`;
    const code = point.codePointAt(0)!;
    result += code >= 0x20 && code <= 0x7e && point !== '"' && point !== "\\"
      ? point
      : code <= 0xffff ? `\\u${code.toString(16).padStart(4, "0")}` : `\\u{${code.toString(16)}}`;
  }
  return `${result}"`;
}
