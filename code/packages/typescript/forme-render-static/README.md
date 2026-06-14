# @coding-adventures/forme-render-static

Forme render stage: `Stream<ContentNode>` → `Stream<RenderedPage>`.
Wraps [`@coding-adventures/document-ast-to-html`](../document-ast-to-html)
and injects a minimal classless HTML5 theme.

Fourth Forme stage of the blog v0 effort. Sits between the parsers
and the `forme-emit-fs` writer.

## Stage shape

```ts
import render from "@coding-adventures/forme-render-static";

render.consumes      // streamOf(Kinds.ContentNode)
render.produces      // streamOf(Kinds.RenderedPage)
render.capabilities  // []  ← pure transform
render.configSchema  // { type: "object", properties: { siteTitle?, routeTemplate? } }
```

## What it does

For each input `ContentNode`:

1. **Derive route** from `sourcePath` via `slugify` + `formatRoute`
   (same rules as `forme-collect-chronological`, so both produce
   identical routes for the same input).
2. **Render body** by calling `toHtml(node.document)` from
   `document-ast-to-html`.
3. **Derive title** via the three-step fallback:
   `frontmatter.title` → first `<h1>` text → slug.
4. **Wrap** the body in a self-contained HTML5 document with the
   classless theme CSS inlined in `<style>`.
5. **Emit** a `RenderedPage` carrying the route, full HTML, derived
   title, and a `source` reference back to the input node's identity.

## Why routes are re-derived here

The "purest" topology has the collector emit a `Collection` whose
entries carry pre-assigned routes, and the renderer reads them.  But
that would force the renderer to consume a different `Kind` than the
parser emits, breaking the natural `parse → render → emit` shape.

For v0 the renderer derives routes locally from `sourcePath` using the
same `slugify` helper the collector uses (duplicated in `src/slug.ts`).
**v0.2 will introduce a router stage** that folds collection routes
back onto `ContentNode.route` so the renderer reads them directly; at
that point the duplicated slug helper goes away.

The duplication is called out explicitly in the source — do NOT
extract it into a shared package until a third stage needs it (FM02
plugin packaging will be the right excuse).

## v0 simplifications (documented)

- **Single hard-coded classless theme.** Replaced when FM04 (Style IR)
  lands and a `forme-render-themed` sibling stage starts consuming a
  `StyleDocument`.
- **Routes derived locally**, not from collection routing (see above).
- **`usedStyle` / `usedIslands` / `usedAssets` all empty.** Driving the
  AOT bundler's smallest-artifact decisions (FM06) needs all three;
  static rendering doesn't have the inputs yet.
- **`meta.description` / `meta.canonicalUrl` null**, `meta.openGraph` /
  `meta.structured` / `meta.extra` empty. Richer head metadata is a
  later concern; the existing fallback path already produces a usable
  `<title>`.
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
  routeTemplate?: string;   // default "/blog/{slug}.html"
}
```

## Dependencies

- `@coding-adventures/forme-types` — `Kinds`, `streamOf`,
  `ContentNode`, `RenderedPage`, `PageMeta`.
- `@coding-adventures/forme-stage` — `defineStage`, `StageContext`.
- `@coding-adventures/document-ast-to-html` — `toHtml`.
- `@coding-adventures/document-ast` — `DocumentNode` type only.
- `@coding-adventures/gfm-parser` — **test only** (round-trip
  fixtures use the real parser).

## Tests

```
npx vitest run --coverage
```

Coverage target 90%+ line. See `tests/` for the title fallback /
theme / stage suites.
