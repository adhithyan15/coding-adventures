/**
 * forme.config.ts — pipeline config for the Coding Adventures blog.
 *
 * Five stages, linear, default IDs.  See `build.ts` for the driver
 * that loads this config and runs it.
 *
 * Roll call:
 *
 *   forme-source-fs              Void                → Stream<ContentSource>
 *   forme-parse-markdown         ContentSource       → ContentNode
 *   forme-collect-chronological  Stream<ContentNode> → Collection   (built but unused in v0; see note)
 *   forme-render-static          Stream<ContentNode> → Stream<RenderedPage>
 *   forme-emit-fs                Stream<RenderedPage>→ DeployArtifact
 *
 * Note on the collector: v0's renderer derives routes from sourcePath
 * locally (the collector emits them on Collection.entries, but no
 * router stage exists yet to fold them back onto ContentNode.route).
 * The collector is left out of this v0 wiring so the orchestrator's
 * single-terminal heuristic stays happy — it'll come back in once
 * an index-page renderer can consume the Collection.  See the
 * forme-render-static README for the v0.2 roadmap.
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
      stage: sourceFs,
      config: { glob: "**/*.md", root: "data" },
    },
    {
      stage: parseMarkdown,
      config: {},
    },
    {
      stage: renderStatic,
      config: {
        siteTitle: "Coding Adventures",
        routeTemplate: "/blog/{slug}.html",
      },
    },
    {
      stage: emitFs,
      config: { outDir: "dist" },
    },
  ],
};

export default config;
