# Coding Adventures Blog

The end-to-end demo of the [Forme](../../specs/fm00-vision.md) universal
authoring pipeline. Markdown lives in `data/`; HTML lands in
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
forme-source-fs           Void                 → Stream<ContentSource>
forme-parse-markdown      ContentSource        → ContentNode
forme-render-static       Stream<ContentNode>  → Stream<RenderedPage>
forme-emit-fs             Stream<RenderedPage> → DeployArtifact
```

`forme-collect-chronological` exists in the monorepo but is **not
wired in v0** — v0's renderer derives routes from `sourcePath`
locally, so there's nothing for the collector to feed yet. The v0.2
router stage will fold collection routes back onto `ContentNode.route`
and the collector will rejoin the pipeline.

## Local build

```bash
cd code/sites/blog
npm install
npx tsx build.ts        # or: npm run build
```

That runs the full pipeline and writes `dist/blog/*.html`.
Each file is a self-contained HTML5 document with a classless theme
inlined in `<style>` — no JS, no external CSS.

`tsx` is a devDependency — it strips TypeScript types at execution
time so we don't need a separate `tsc` step just to drive the
pipeline. The stage packages compile their own published types; the
site driver runs straight from source.

## Layout

- `data/` — Markdown posts. Frontmatter is `key: value` only (the v0
  parser is grammar-restricted; see
  `code/packages/typescript/forme-parse-markdown/README.md`).
- `forme.config.ts` — `PipelineConfig` literal wiring the five
  stages in order. Per FM03 §2.2, IDs are inferred from `stage.name`
  when unique.
- `build.ts` — ~50-line driver: load config → `createOrchestrator`
  → `buildPipeline` → `runOnce` → assert success.
- `BUILD` — chain-installs the file:dependency graph then runs the
  pipeline. Verifies `dist/blog/` is produced.
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

`.github/workflows/deploy-blog.yml` runs the build on every push to
`main` that touches this directory or any Forme package, then
publishes `dist/` to the `gh-pages` branch under `coding-adventures/blog/`.
The live URL is
[adhithyan15.github.io/coding-adventures/blog/](https://adhithyan15.github.io/coding-adventures/blog/).

## What's missing (intentional v0 scope)

- No index page. The collector exists but isn't wired in v0; a
  v0.2 index-page renderer will read its `Collection` output.
- No RSS feed. Same story — a `Stage<Collection, Feed>` will
  produce one when the collector lands.
- No asset extraction. Posts that reference images today will
  link to `data/`-relative paths; an asset stage will copy + hash
  them.
- No dark mode. The classless theme is light-only for v0.

Every one of these is a follow-up *stage*, not a rewrite of any
existing stage. That's the bet.
