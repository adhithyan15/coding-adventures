/**
 * src/config.ts — pipeline configuration for the hello-world demo.
 *
 * `makePipelineConfig({ contentRoot, outDir })` returns the
 * `PipelineConfig` value the FM03 orchestrator consumes.  Keeping
 * this in a pure factory (rather than a top-level const) lets the
 * CLI and the test pass different roots / output dirs without
 * mutating shared state.
 *
 * ═══ Topology (v0) ════════════════════════════════════════════════
 *
 *   forme-source-fs        Void                    → Stream<ContentSource>
 *       │
 *       ▼
 *   forme-parse-markdown   ContentSource           → ContentNode  (per-item)
 *       │      [effective downstream: Stream<ContentNode>
 *       │       via the orchestrator's stream-iteration promotion —
 *       │       see forme-orchestrator@0.1.1 changelog]
 *       ▼
 *   forme-render-static    Stream<ContentNode>     → Stream<RenderedPage>
 *       │
 *       ▼
 *   forme-emit-fs          Stream<RenderedPage>    → DeployArtifact  (sink)
 *
 * The orchestrator infers every wire from declared kinds (FM03 §3.3
 * inference rule #2: "most recent declared producer of a compatible
 * kind").  Stages appear in declaration order; no `wires` block is
 * needed.
 *
 * ═══ Why `forme-collect-chronological` is NOT in this topology ════
 *
 * The collector produces `Kinds.Collection`, which is NOT in the
 * orchestrator's sink kind list (DeployArtifact / RequestHandler /
 * Feed / SearchIndex per FM03 §3.3 rule #5).  Wiring it in here
 * would create an orphan terminal producer; the v0 renderer derives
 * its own routes from `sourcePath` and never reads the collection.
 *
 * The fix lands when the v0.2 "router" stage exists and feeds
 * collection-side routes back onto `ContentNode.route`.  At that
 * point the renderer becomes a consumer of routed nodes and the
 * collector slots in cleanly between parse and render.  See
 * forme-render-static's README and the source-level comments in
 * `forme-render-static/src/slug.ts` for the cross-package note.
 *
 * ═══ Settings rationale ═══════════════════════════════════════════
 *
 *   storageRoot       — passed through to stages' `ctx.storage`.
 *                       Source-fs and emit-fs bypass it in v0 (they
 *                       ARE the storage adapters) so this is mostly
 *                       documentation.
 *   cacheDir          — null.  The orchestrator falls back to an
 *                       in-memory cache.  Persistent caching is
 *                       gated on FM03 §6 (incremental rebuild)
 *                       landing.
 *   reproducibleBuild — false.  Toggled on by the e2e test so two
 *                       consecutive builds produce identical HTML.
 *   maxConcurrency    — null → orchestrator default (hardware
 *                       concurrency).  Hello-world has exactly one
 *                       post so this is moot.
 *   logLevel          — info.  The CLI logger pretty-prints; the
 *                       test overrides with `silentLogger()`.
 *   bestEffort        — false.  A broken hello-world should fail
 *                       loudly, not partially.
 *   deadlineMs        — 30_000.  Generous ceiling for the demo;
 *                       prevents a runaway from hanging CI.
 *
 * @module config
 */

import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";
import sourceFs from "@coding-adventures/forme-source-fs";
import parseMarkdown from "@coding-adventures/forme-parse-markdown";
import renderStatic from "@coding-adventures/forme-render-static";
import emitFs from "@coding-adventures/forme-emit-fs";

/** Inputs to {@link makePipelineConfig}. */
export interface MakePipelineConfigOptions {
  /**
   * Directory the source stage walks for Markdown files.  Passed to
   * `forme-source-fs` as `config.root`.  Must be an absolute path so
   * the demo works regardless of `process.cwd()` at runtime.
   */
  readonly contentRoot: string;

  /**
   * Directory the emit stage writes HTML files under.  Passed to
   * `forme-emit-fs` as `config.outDir`.  Must be an absolute path
   * for the same reason as `contentRoot`.
   */
  readonly outDir: string;

  /**
   * When true, the pipeline runs in FM03 §8 reproducible-build mode
   * (frozen time, sorted iteration, deterministic randomness).  The
   * v0 orchestrator does not yet implement most of these — the flag
   * is plumbed through anyway so the demo is forward-compatible.
   * Default: false.
   */
  readonly reproducibleBuild?: boolean;

  /**
   * Override the pipeline-wide log level.  Defaults to `"info"` for
   * the CLI; the test overrides via `silentLogger()` on the
   * orchestrator side, not via this field.
   */
  readonly logLevel?: PipelineConfig["settings"]["logLevel"];
}

/**
 * Build a `PipelineConfig` for the hello-world blog.
 *
 * The returned value is plain data — pass it to
 * `orchestrator.buildPipeline()` and then `orchestrator.runOnce()`.
 */
export function makePipelineConfig(
  options: MakePipelineConfigOptions,
): PipelineConfig {
  const {
    contentRoot,
    outDir,
    reproducibleBuild = false,
    logLevel = "info",
  } = options;

  return {
    name: "forme-hello-world",
    settings: {
      storageRoot:       contentRoot,
      cacheDir:          null,
      reproducibleBuild,
      maxConcurrency:    null,
      logLevel,
      bestEffort:        false,
      deadlineMs:        30_000,
    },
    stages: [
      // 1. Walk the content directory; emit one ContentSource per .md.
      //    `glob: "**/*.md"` matches the v0 walker's "*\*/*.<ext>"
      //    constraint (forme-source-fs/walker.ts).
      {
        stage:  sourceFs,
        config: { glob: "**/*.md", root: contentRoot },
      },

      // 2. Parse frontmatter + body into a ContentNode.  gfm: true is
      //    the parser's only supported mode in v0 (the flag is
      //    accepted but currently ignored; declaring it documents
      //    intent for when toggling lands).
      {
        stage:  parseMarkdown,
        config: { gfm: true },
      },

      // 3. Render each ContentNode as a self-contained HTML5 page
      //    with the classless theme.  routeTemplate is the v0
      //    default (/blog/{slug}.html) — repeating it here makes the
      //    output path obvious to readers of this config.
      {
        stage:  renderStatic,
        config: {
          siteTitle:     "forme-hello-world",
          routeTemplate: "/blog/{slug}.html",
        },
      },

      // 4. Write each page to disk; emit a final DeployArtifact
      //    summarising the build.  outDir is REQUIRED by emit-fs's
      //    schema — undefined here would fail config validation.
      {
        stage:  emitFs,
        config: { outDir },
      },
    ],
  };
}
