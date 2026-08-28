# @coding-adventures/forme-render-static

Forme render stage: `Stream<ContentNode>` → `Stream<RenderedPage>`.
It wraps [`@coding-adventures/document-ast-to-html`](../document-ast-to-html),
matches a resolved Style IR document, and compiles only the matched rules
through Forme's AOT CSS slicer.

Page-rendering branch of the Forme blog DAG. It consumes routed nodes
and feeds the `forme-emit-fs` writer.

## Stage shape

```ts
import render from "@coding-adventures/forme-render-static";

render.consumes      // streamOf(Kinds.ContentNode)
render.produces      // streamOf(Kinds.RenderedPage)
render.capabilities  // []  ← pure transform
render.configSchema  // style, activeStyleContexts, site metadata
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
4. **Match Style IR** against the HTML element tree and record source-ordered
   rule IDs in `RenderedPage.usedStyle`.
5. **Slice CSS** through `forme-aot-css-slicer`, with only matched rules and
   active contexts compiled for this page.
6. **Compose public metadata** when `siteUrl` is configured: description,
   canonical URL, project-page-safe site navigation, and RSS/Atom discovery.
7. **Wrap** the body in a theme-agnostic HTML5 shell with sliced CSS inlined.
8. **Emit** a `RenderedPage` carrying the route, full HTML, derived title,
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

## Current boundaries

- **Themes must be resolved.** Compose named theme overlays before rendering;
  a `StyleDocument` with non-null `theme` is rejected rather than guessed.
- **Routed input is required.** This keeps one canonical URL decision across
  all product branches.
- **`usedIslands` remains empty.** Resolved `AssetRef` images render as
  collision-free `forme-asset:` placeholders and populate `usedAssets`; the
  asset-aware emitter replaces those placeholders with fingerprinted paths.
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

## Style IR and themes

The renderer owns no visual defaults. Pass any resolved `StyleDocument`, or
use [`forme-theme-classless`](../forme-theme-classless) for the reusable
Coding Adventures prose theme. Matching is deterministic and supports Style
IR identity, structural, position, and composition selectors. Trusted raw
HTML conservatively retains the full theme because its elements are opaque to
the Document AST.

The default active contexts are `screen` and `dark`. Configure
`activeStyleContexts` to add `narrow`, `high-contrast`, or other declared
contexts. Dark rules compile to `prefers-color-scheme: dark` media queries.

## Config

```ts
interface RenderStaticConfig {
  style?:               StyleDocument; // resolved (`theme: null`)
  activeStyleContexts?: string[];      // defaults to screen + dark
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
- `@coding-adventures/forme-style-ir` — validation and selector contracts.
- `@coding-adventures/forme-aot-css-slicer` — per-page CSS compilation.
- `@coding-adventures/gfm-parser` — **test only** (round-trip
  fixtures use the real parser).

## Tests

```
npx vitest run --coverage
```

Coverage target 90%+ line. See `tests/` for the title fallback /
theme / stage suites.
