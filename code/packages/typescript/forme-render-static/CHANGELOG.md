# Changelog — @coding-adventures/forme-render-static

## 0.1.0 — 2026-05-15

Initial release. Fourth Forme stage of the blog v0 effort.

### Added

- `renderStatic` default-exported stage:
  - `consumes: streamOf(Kinds.ContentNode)`
  - `produces: streamOf(Kinds.RenderedPage)`
  - `capabilities: []` (pure transform)
  - `configSchema: { siteTitle?: string; routeTemplate?: string }`
- Per-input pipeline: derive route from `sourcePath` →
  `toHtml(document)` → resolve title → wrap in classless HTML5 doc.
- `deriveTitle(node, slug)` — three-step fallback
  (frontmatter.title → first H1 → slug); always returns non-empty.
- `slugify` + `formatRoute` duplicated from
  `forme-collect-chronological` (DRY violation called out in source).
- Classless theme (`theme.ts`) — system font stack, 38rem reading
  column, defaults for headings / lists / code / blockquote / hr /
  table / img.  Inlined in `<style>` (no external CSS request).
- `escapeHtml` — narrow set (`&`, `<`, `>`, `"`, `'`) for title /
  site-title contexts.
- Cancellation honoured between input nodes.

### Spec adherence

No deliberate divergences from FM00 / FM01.

### v0 simplifications (documented)

- **Single hard-coded classless theme.** Replaced when FM04 (Style IR)
  lands and a `forme-render-themed` sibling stage starts consuming a
  `StyleDocument`.
- **Routes derived locally** from `sourcePath` rather than from
  `Collection.entries[i].route`.  A v0.2 router stage will fold
  collection routes back onto `ContentNode.route`, at which point the
  duplicated `slugify` helper goes away.  Tracking note in `slug.ts`.
- **`usedStyle` / `usedIslands` / `usedAssets` all empty arrays.**
  These drive the AOT bundler's smallest-artifact decisions (FM06);
  static rendering doesn't have the inputs yet.
- **`meta.description` / `meta.canonicalUrl` null**, `meta.openGraph`
  / `meta.structured` / `meta.extra` empty.  Richer head metadata is a
  later concern; the title fallback already produces a usable `<title>`.
- **No sanitization.**  Renderer trusts the input Markdown — this is
  your own blog, you wrote the posts.  Multi-tenant pipelines must
  wire `@coding-adventures/document-ast-sanitizer` in between parser
  and renderer.

### Notes

- The duplicated slug logic (versus
  `forme-collect-chronological/src/slug.ts`) is intentional for v0.
  Standing up a shared utility package now would cost more in monorepo
  wiring (BUILD chains, lockfile drift, peer-dep graphs) than ~30
  lines saves.  Extract when a third stage needs it.
- Theme keeps `theme.ts` deterministic — no clock reads, no current-
  year footer.  The clock facility lives on `StageContext` if a future
  caller wants to inject a date.
- The renderer leaves a trailing newline on the HTML doc so emitted
  files match common Unix conventions.
