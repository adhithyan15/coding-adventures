/**
 * forme.config.ts — pipeline config for the Coding Adventures blog.
 *
 * Eight stages with explicit fan-out after routing and collection.
 * See `build.ts` for the driver that verifies both deploy sinks.
 *
 * Roll call:
 *
 *   forme-source-fs              Void                 → Stream<ContentSource>
 *   forme-parse-markdown         ContentSource        → ContentNode
 *   forme-router                 Stream<ContentNode>  → Stream<ContentNode>
 *                                         ├→ collect → blog-surface → emit-surface
 *                                         └→ render-pages → emit-articles
 *
 * The router is the sole routing-policy stage. Its materialized stream
 * fans out to the chronological collection and page renderer, so both
 * branches consume exactly the same canonical `ContentNode.route`.
 *
 * Note on the route template: `/blog/{slug}.html` does NOT include the
 * `/coding-adventures/` repo-name prefix that the live URL has.  That
 * prefix is a GitHub-Pages-project-page deployment detail — every
 * project page lives under https://<user>.github.io/<repo>/ — not a
 * routing concern.  Baking it into the route would make the build
 * non-portable (rename the repo, switch to a user/org page, point a
 * custom domain at it → all the routes would need rewriting).  The
 * deploy workflow publishes dist/blog/ to gh-pages:blog/, while
 * `siteUrl` composes the deployment prefix only for public links,
 * canonical metadata, feeds, and sitemap entries.
 */

import sourceFs       from "@coding-adventures/forme-source-fs";
import parseMarkdown  from "@coding-adventures/forme-parse-markdown";
import router         from "@coding-adventures/forme-router";
import collectChronological from "@coding-adventures/forme-collect-chronological";
import renderStatic   from "@coding-adventures/forme-render-static";
import emitFs         from "@coding-adventures/forme-emit-fs";
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
      },
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
      stage: emitFs,
      config: { outDir: "dist" },
    },
    {
      id: "emit-surface",
      stage: emitFs,
      config: { outDir: "dist" },
    },
  ],
  wires: [
    { from: { id: "source" },       to: { id: "parse" } },
    { from: { id: "parse" },        to: { id: "route" } },
    { from: { id: "route" },        to: { id: "collect-posts" } },
    { from: { id: "route" },        to: { id: "render-pages" } },
    { from: { id: "collect-posts" }, to: { id: "render-surface" } },
    { from: { id: "render-pages" }, to: { id: "emit-articles" } },
    { from: { id: "render-surface" }, to: { id: "emit-surface" } },
  ],
  outputs: [
    { fromInstance: "emit-articles", name: "articles" },
    { fromInstance: "emit-surface", name: "surface" },
  ],
};

export default config;
