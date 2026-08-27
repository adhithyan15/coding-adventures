# Coding Adventures Blog

The end-to-end demo of the [Forme](../../specs/FM00-forme-vision.md) universal
authoring pipeline. Markdown lives in `data/`; a complete static blog lands in
`dist/blog/` and is deployed to
[adhithyan15.github.io/coding-adventures/blog/](https://adhithyan15.github.io/coding-adventures/blog/)
by `.github/workflows/deploy-blog.yml`.

### Why the route says `/blog/...` but the URL says `/coding-adventures/blog/...`

This repo is a GitHub Pages **project** page. Project pages are served
from `https://<user>.github.io/<repo>/`, so the `coding-adventures/`
segment of the live URL comes from the repo name (project-page prefix),
not from anything in the build. The route template
(`/blog/{slug}.html`) and the publish destination (`gh-pages:blog/`)
both stay clean of that prefix; Pages composes it at the edge. If the
repo ever gets a custom domain or moves to a user/org page, the route
template and the destination don't need to change.

## Pipeline

```
forme-source-fs           Void                → Stream<ContentSource>
forme-parse-markdown      ContentSource       → ContentNode
forme-router              Stream<ContentNode> → Stream<ContentNode>
                         ├→ forme-render-static → forme-emit-fs (`articles`)
                         └→ forme-collect-chronological
                              → blog-surface → forme-emit-fs (`surface`)
```

`forme-router` is the single owner of URL policy. The orchestrator
materializes its routed-node stream once and fans it out: the article renderer
uses each canonical route, while the chronological collector feeds an aggregate
surface stage built from the existing Forme index, RSS, Atom, sitemap, metadata,
and feed-discovery generators. Two named deploy sinks write disjoint route sets
to the same `dist/` tree.

## Local build

```bash
cd code/sites/blog
npm run build:clean
```

That one command discovers every `file:`-linked package, installs the local
dependency graph from leaves to the site, clears stale output, and runs the
full pipeline. It writes article pages plus `index.html`, `rss.xml`, `atom.xml`,
and `sitemap.xml` under `dist/blog/`. Subsequent builds can use `npm run build`;
run `npm test` to exercise both the dependency planner and collection-derived
surface. The build verifies both named deploy manifests and the exact aggregate
route set. HTML files are self-contained documents with a classless theme
inlined in `<style>` — no JS, no external CSS.

`tsx` is a devDependency — it strips TypeScript types at execution
time so we don't need a separate `tsc` step just to drive the
pipeline. The stage packages compile their own published types; the
site driver runs straight from source.

## Layout

- `data/` — Markdown posts. Frontmatter is `key: value` only (the v0
  parser is grammar-restricted; see
  `code/packages/typescript/forme-parse-markdown/README.md`).
- `forme.config.ts` — `PipelineConfig` literal wiring eight named stage
  instances, including two fan-outs and two deploy outputs.
- `surface-stage.ts` — collection adapter that composes the reusable Forme
  index/feed/sitemap/head generators into deployable pages.
- `build.ts` — driver: load config → `createOrchestrator`
  → `buildPipeline` → `runOnce` → assert success.
- `scripts/bootstrap.mjs` — discovers and installs the complete local
  `file:` dependency graph in dependency order. This is the same bootstrap
  path used by local builds and pull-request CI.
- `BUILD` — monorepo build-tool entry point. It installs the same dependency
  graph, runs the pipeline, and verifies `dist/blog/` is produced.
- `dist/` — build output (git-ignored).

## Adding a post

1. Drop `YYYY-MM-DD-<slug>.md` in `data/` with frontmatter:
   ```
   ---
   title: My post title
   date: 2026-05-15
   excerpt: One-line summary.
   ---

   # My post title

   …body in GFM…
   ```
2. Re-run `npm run build`.
3. Open `dist/blog/<slug>.html`.

The slug is derived from the filename (strip `.md`, lowercase,
replace whitespace/`_` with `-`). To override, set `slug:` in
frontmatter.

## Deploy

`.github/workflows/deploy-blog.yml` runs `npm run build:clean` for relevant pull
requests and for every push to `main` that touches this directory or a runtime
dependency. Main-branch builds then publish `dist/blog/` to the `gh-pages`
branch under `blog/`.
The live URL is
[adhithyan15.github.io/coding-adventures/blog/](https://adhithyan15.github.io/coding-adventures/blog/).

## What's missing (intentional v0 scope)

- No asset extraction. Posts that reference images today will
  link to `data/`-relative paths; an asset stage will copy + hash
  them.
- No dark mode. The classless theme is light-only for v0.
- Aggregate artifacts currently carry a synthetic single source ID because
  `RenderedPage.source` cannot yet represent collection provenance.

These are tracked in the
[Forme completion roadmap](../../specs/FM00-forme-completion-roadmap.md).
