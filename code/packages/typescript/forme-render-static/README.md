# @coding-adventures/forme-render-static

Forme render stage: `Stream<ContentNode>` → `Stream<RenderedPage>`.
Wraps [`@coding-adventures/document-ast-to-html`](../document-ast-to-html)
and injects a minimal classless HTML5 theme.

Page-rendering branch of the Forme blog DAG. It consumes routed nodes
and feeds the `forme-emit-fs` writer.

## Stage shape

```ts
import render from "@coding-adventures/forme-render-static";

render.consumes      // streamOf(Kinds.ContentNode)
render.produces      // streamOf(Kinds.RenderedPage)
render.capabilities  // []  ← pure transform
render.configSchema  // siteTitle, siteUrl, siteHomeRoute, rssRoute, atomRoute
```

## What it does

For each input `ContentNode`:

1. **Use the canonical route** from `ContentNode.route`, set upstream by
   `forme-router`. An unrouted node fails with a diagnostic that names the
   source and tells the pipeline author to add the router.
2. **Render body** by calling `toHtml(node.document)` from
   `document-ast-to-html`.
3. **Derive title** via the three-step fallback:
   `frontmatter.title` → first `<h1>` text → slug.
4. **Compose public metadata** when `siteUrl` is configured: description,
   canonical URL, project-page-safe site navigation, and RSS/Atom discovery.
5. **Wrap** the body in a self-contained HTML5 document with the
   classless theme CSS inlined in `<style>`.
6. **Emit** a `RenderedPage` carrying the route, full HTML, derived title,
   and revision-aware provenance for the input node. The legacy `source`
   identity is retained temporarily for v1.0 consumers.

## Routing contract

The renderer does not own URL policy. A routed product pipeline uses:

```text
parse → router ┬→ render → emit
               └→ collector
```

Both branches read the same `ContentNode.route`; URL templates belong only
to `forme-router`.

## v0 simplifications (documented)

- **Single hard-coded classless theme.** Replaced when FM04 (Style IR)
  lands and a `forme-render-themed` sibling stage starts consuming a
  `StyleDocument`.
- **Routed input is required.** This keeps one canonical URL decision across
  all product branches.
- **`usedStyle` / `usedIslands` / `usedAssets` all empty.** Driving the
  AOT bundler's smallest-artifact decisions (FM06) needs all three;
  static rendering doesn't have the inputs yet.
- **OpenGraph / structured metadata still empty.** Description and canonical
  URL are populated when configured; social cards and structured data remain
  later work.
- **No sanitization.** The renderer trusts the input Markdown — this
  is your own blog, you wrote the posts. Multi-tenant pipelines must
  wire `@coding-adventures/document-ast-sanitizer` in between parser
  and renderer (see the document-ast-to-html `RenderOptions` docs).

## Title resolution

```
frontmatter.title (non-empty string)
  ↳ if missing: first <h1> in the document, flattened to text
    ↳ if missing/empty: the URL slug (always non-empty)
```

`flattenInline()` handles common inline children (`text`, inline
`code`, `emphasis`, `strong`, soft/hard breaks → space). Less-common
inline kinds fall through to recursion or are ignored.

## Theme

The classless CSS is a hand-rolled minimum:

- System font stack (no web-font request)
- `max-width: 38rem` reading column (60–75 character measure)
- Defaults for headings, lists, code (inline + block), blockquote,
  hr, table, images
- Light-mode only (dark mode is a v0.2 concern)

Inlined in `<style>` so a single-file build needs no external CSS
request — important for first-paint on cold caches.

## Config

```ts
interface RenderStaticConfig {
  siteTitle?:     string;   // header anchor text; empty → no header
  siteUrl?:       string;   // public deployment base, including project prefix
  siteHomeRoute?: string;   // route used by the site-title link
  rssRoute?:      string;   // RSS auto-discovery route
  atomRoute?:     string;   // Atom auto-discovery route
}
```

## Dependencies

- `@coding-adventures/forme-types` — `Kinds`, `streamOf`,
  `ContentNode`, `RenderedPage`, `PageMeta`.
- `@coding-adventures/forme-stage` — `defineStage`, `StageContext`.
- `@coding-adventures/forme-identity` — deterministic output provenance.
- `@coding-adventures/document-ast-to-html` — `toHtml`.
- `@coding-adventures/document-ast` — `DocumentNode` type only.
- `@coding-adventures/forme-aot-meta-link-tags` — description and canonical
  head tags.
- `@coding-adventures/forme-aot-rss-discovery-link` — RSS/Atom discovery tags.
- `@coding-adventures/gfm-parser` — **test only** (round-trip
  fixtures use the real parser).

## Tests

```
npx vitest run --coverage
```

Coverage target 90%+ line. See `tests/` for the title fallback /
theme / stage suites.
