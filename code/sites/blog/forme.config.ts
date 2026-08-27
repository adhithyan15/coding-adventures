/**
 * forme.config.ts — pipeline config for the Coding Adventures blog.
 *
 * Six stages with an explicit fan-out after routing. See `build.ts`
 * for the driver that loads this config and verifies both sinks.
 *
 * Roll call:
 *
 *   forme-source-fs              Void                 → Stream<ContentSource>
 *   forme-parse-markdown         ContentSource        → ContentNode
 *   forme-router                 Stream<ContentNode>  → Stream<ContentNode>
 *                                         ├─────────→ forme-collect-chronological → Collection
 *                                         └─────────→ forme-render-static → Stream<RenderedPage>
 *                                                                    └────→ forme-emit-fs → DeployArtifact
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
 * deploy workflow publishes dist/blog/ to gh-pages:blog/, and Pages
 * exposes it as <user>.github.io/<repo>/blog/<slug>.html — composition
 * lives at the deploy boundary, not in the content.
 */

import sourceFs       from "@coding-adventures/forme-source-fs";
import parseMarkdown  from "@coding-adventures/forme-parse-markdown";
import router         from "@coding-adventures/forme-router";
import collectChronological from "@coding-adventures/forme-collect-chronological";
import renderStatic   from "@coding-adventures/forme-render-static";
import emitFs         from "@coding-adventures/forme-emit-fs";
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
      config: { siteTitle: "Coding Adventures" },
    },
    {
      id: "emit-site",
      stage: emitFs,
      config: { outDir: "dist" },
    },
  ],
  wires: [
    { from: { id: "source" },       to: { id: "parse" } },
    { from: { id: "parse" },        to: { id: "route" } },
    { from: { id: "route" },        to: { id: "collect-posts" } },
    { from: { id: "route" },        to: { id: "render-pages" } },
    { from: { id: "render-pages" }, to: { id: "emit-site" } },
  ],
  outputs: [
    { fromInstance: "collect-posts", name: "posts" },
    { fromInstance: "emit-site", name: "site" },
  ],
};

export default config;
