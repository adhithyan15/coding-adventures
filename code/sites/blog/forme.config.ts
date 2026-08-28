/**
 * forme.config.ts — pipeline config for the Coding Adventures blog.
 *
 * Ten stages with explicit fan-out after asset resolution, routing, and collection.
 * See `build.ts` for the driver that verifies both deploy sinks.
 *
 * Roll call:
 *
 *   forme-source-fs              Void                 → Stream<ContentSource>
 *   forme-parse-markdown         ContentSource        → ContentNode
 *   forme-resolve-asset-refs-fs  Stream<ContentNode>  → Stream<ContentNode>
 *   forme-router                 Stream<ContentNode>  → Stream<ContentNode>
 *                                         ├→ collect → blog-surface → emit-surface
 *                                         ├→ render-pages ──────────────┐
 *                                         └→ load-assets ──(assets)─────┴→ emit-articles
 *
 * The router is the sole routing-policy stage. Its materialized stream
 * fans out to the chronological collection, page renderer, and asset loader,
 * so all three branches consume the same canonical `ContentNode.route` and
 * resolved `AssetRef` identities.
 *
 * Note on the route template: `/blog/{slug}.html` does NOT include the
 * `/coding-adventures/` repo-name prefix that the live URL has.  That
 * prefix is a GitHub-Pages-project-page deployment detail — every
 * project page lives under https://<user>.github.io/<repo>/ — not a
 * routing concern.  Baking it into the route would make the build
 * non-portable (rename the repo, switch to a user/org page, point a
 * custom domain at it → all the routes would need rewriting).  The
 * deploy workflow publishes dist/blog/ to gh-pages:blog/, while
 * `siteUrl` composes the deployment prefix for canonical metadata,
 * feeds, and sitemap entries; the site emitter's `publicPathPrefix`
 * does the same at the asset-link boundary without changing artifact paths.
 */

import sourceFs       from "@coding-adventures/forme-source-fs";
import parseMarkdown  from "@coding-adventures/forme-parse-markdown";
import resolveAssetRefsFs from "@coding-adventures/forme-resolve-asset-refs-fs";
import router         from "@coding-adventures/forme-router";
import collectChronological from "@coding-adventures/forme-collect-chronological";
import renderStatic   from "@coding-adventures/forme-render-static";
import classlessTheme from "@coding-adventures/forme-theme-classless";
import loadAssetsFs   from "@coding-adventures/forme-load-assets-fs";
import emitFs         from "@coding-adventures/forme-emit-fs";
import emitSiteFs     from "@coding-adventures/forme-emit-site-fs";
import blogSurface    from "./surface-stage.ts";
import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";

const config: PipelineConfig = {
  name: "coding-adventures-blog",
  settings: {
    storageRoot: ".",
    cacheDir: null,
    reproducibleBuild: false,
    maxConcurrency: null,
    logLevel: "info",
    bestEffort: false,
    deadlineMs: null,
  },
  stages: [
    {
      id: "source",
      stage: sourceFs,
      config: { glob: "**/*.md", root: "data" },
    },
    {
      id: "parse",
      stage: parseMarkdown,
      config: {},
    },
    {
      id: "resolve-assets",
      stage: resolveAssetRefsFs,
      config: { root: "data", persistIdentities: true },
    },
    {
      id: "route",
      stage: router,
      config: { routeTemplate: "/blog/{slug}.html" },
    },
    {
      id: "collect-posts",
      stage: collectChronological,
      config: { name: "posts", dateField: "date" },
    },
    {
      id: "render-pages",
      stage: renderStatic,
      config: {
        siteTitle: "Coding Adventures",
        siteUrl: "https://adhithyan15.github.io/coding-adventures",
        siteHomeRoute: "/blog/index.html",
        rssRoute: "/blog/rss.xml",
        atomRoute: "/blog/atom.xml",
        style: classlessTheme,
        activeStyleContexts: ["dark", "narrow", "high-contrast"],
      },
    },
    {
      id: "load-assets",
      stage: loadAssetsFs,
      config: { root: "data" },
    },
    {
      id: "render-surface",
      stage: blogSurface,
      config: {
        siteTitle: "Coding Adventures",
        siteDescription: "Small systems, built from first principles.",
        siteUrl: "https://adhithyan15.github.io/coding-adventures",
        indexRoute: "/blog/index.html",
        rssRoute: "/blog/rss.xml",
        atomRoute: "/blog/atom.xml",
        sitemapRoute: "/blog/sitemap.xml",
      },
    },
    {
      id: "emit-articles",
      stage: emitSiteFs,
      config: {
        outDir: "dist",
        assetDir: "blog/assets",
        publicPathPrefix: "/coding-adventures",
      },
    },
    {
      id: "emit-surface",
      stage: emitFs,
      config: { outDir: "dist" },
    },
  ],
  wires: [
    { from: { id: "source" },       to: { id: "parse" } },
    { from: { id: "parse" },        to: { id: "resolve-assets" } },
    { from: { id: "resolve-assets" }, to: { id: "route" } },
    { from: { id: "route" },        to: { id: "collect-posts" } },
    { from: { id: "route" },        to: { id: "render-pages" } },
    { from: { id: "route" },        to: { id: "load-assets" } },
    { from: { id: "collect-posts" }, to: { id: "render-surface" } },
    { from: { id: "render-pages" }, to: { id: "emit-articles" } },
    { from: { id: "load-assets" },  to: { id: "emit-articles", port: "assets" } },
    { from: { id: "render-surface" }, to: { id: "emit-surface" } },
  ],
  outputs: [
    { fromInstance: "emit-articles", name: "articles" },
    { fromInstance: "emit-surface", name: "surface" },
  ],
};

export default config;
