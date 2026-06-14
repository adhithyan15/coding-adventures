# @coding-adventures/forme-transform-toc

Build a hierarchical Table-of-Contents tree from a
`DocumentNode` (or a pre-computed `HeadingSlug[]`).  FM00 v0 §5.3
transform.

Pure transform: heading sequence → nested `TocNode[]` tree that
renderers consume to emit

```html
<nav class="forme-toc">
  <ul>
    <li>
      <a href="#installation">Installation</a>
      <ul>
        <li><a href="#requirements">Requirements</a></li>
        <li><a href="#install-steps">Install steps</a></li>
      </ul>
    </li>
  </ul>
</nav>
```

Sixth FM00 v0 stage package — joins
[`forme-feeds`](../forme-feeds),
[`forme-opengraph`](../forme-opengraph),
[`forme-index-renderer`](../forme-index-renderer),
[`forme-transforms`](../forme-transforms), and
[`forme-transform-autolink-headings`](../forme-transform-autolink-headings).

## Quick start

```ts
import { buildToc } from "@coding-adventures/forme-transform-toc";

// Default: include every heading level 1-6.
const toc = buildToc(doc);

// Common idiom: skip the page title (h1), cap depth at h3.
const sidebarToc = buildToc(doc, { minLevel: 2, maxLevel: 3 });

// Walk the tree to emit HTML:
function render(nodes: readonly TocNode[]): string {
  if (nodes.length === 0) return "";
  return `<ul>${nodes.map((n) =>
    `<li><a href="${n.href}">${n.text}</a>${render(n.children)}</li>`
  ).join("")}</ul>`;
}
```

## Why this package exists

A flat `HeadingSlug[]` (from `forme-transform-autolink-headings`)
is the right shape for renderers walking the document body —
each heading consumed in document order.  But TOCs are nested:
`<h3>`s live inside their parent `<h2>`'s subtree.

This package is the bridge: stack-based one-pass tree
construction that handles:

- Skipped levels (`<h1>` directly to `<h3>`) without throwing.
- Outdent past root (`<h1>` after deep nesting) cleanly.
- Documents that start at a deep level (first heading is `<h3>`).
- Multiple top-level roots (docs with no `<h1>`).
- Repeated peer levels (sibling `<h2>`s under one `<h1>`).

All deterministic; same input → byte-identical tree.  Safe to
use as cache key input.

## API

### `buildToc(doc, options?): TocNode[]`

Build a TOC tree directly from a `DocumentNode`.  Internally
calls `autolinkHeadings(doc)` for the slug stream.

### `buildTocFromSlugs(slugs, options?): TocNode[]`

Build from a pre-computed `HeadingSlug[]`.  Use when the caller
already invoked `autolinkHeadings` for the renderer and doesn't
want to walk the AST twice.

### `buildTree(slugs): TocNode[]`

Sub-helper: pure flat → hierarchical tree construction, no
filtering.  Exposed for callers wiring custom pipelines.

### `filterByLevel(slugs, minLevel, maxLevel): HeadingSlug[]`

Sub-helper: drop slugs outside the level range.  Out-of-range
parameters clamp to `[1, 6]`; inverted ranges return `[]`.

### Types

```ts
interface TocNode {
  readonly level: 1 | 2 | 3 | 4 | 5 | 6;
  readonly text: string;
  readonly slug: string;
  readonly href: string;        // === "#" + slug
  readonly children: readonly TocNode[];
}

interface TocOptions {
  readonly minLevel?: 1 | 2 | 3 | 4 | 5 | 6;  // default 1
  readonly maxLevel?: 1 | 2 | 3 | 4 | 5 | 6;  // default 6
}
```

## Behavioural contract

| Aspect                              | Behaviour                            |
|-------------------------------------|--------------------------------------|
| Input AST / slugs                   | Never mutated                        |
| Output                              | Fresh tree each call (no shared refs)|
| Skipped levels (h1→h3)              | h3 becomes direct child of h1        |
| First heading at deep level         | Becomes a top-level root             |
| Outdent past root                   | Closes subtrees, starts new root     |
| Repeated peer levels                | Become siblings (not nested)         |
| `minLevel` filtered-out heading     | Does NOT preserve nesting; children promote to roots if no surviving parent |
| Source level                        | Preserved verbatim (no remapping)    |
| `href`                              | Always `"#" + slug`                  |
| Inverted range (minLevel > maxLevel)| Returns `[]`                         |

## Reproducibility (FM03)

Same `DocumentNode` → byte-identical `TocNode[]`.  The stack
algorithm is a pure function of input order; `autolinkHeadings`
already guarantees deterministic slug generation.  Safe to feed
into cache key derivation.

## Security posture

Three concerns explicitly addressed (pre-push review):

- **AST mutation.** Input arrays are never `.push`-ed into; the
  source AST is never modified.  Verified by JSON-snapshot tests.
- **Heading text passthrough.**  The `text` field on `TocNode`
  preserves the raw heading content verbatim — renderers must
  HTML-escape it when emitting markup.  Only the `slug` / `href`
  are sanitised (by `slugify` upstream); the `text` is data, not
  markup.
- **Bounded computation.**  Tree construction is O(N) — stack
  pushes once, pops at most once per heading.  Adversarial inputs
  like 10,000 `<h6>` headings under one `<h1>` produce a tree of
  10,000 children, not exponential nesting.

## Capabilities — `[]`

Pure transform.  No I/O, no network, no shell, no env, no fs.

## Tests

55 tests across 3 files:

- `filter.test.ts` (17) — defaults, minLevel / maxLevel
  filtering, combined, out-of-range clamping (negative, NaN,
  Infinity, fractional), inverted range, purity (no mutation,
  fresh array, empty input).
- `build-tree.test.ts` (20) — well-formed hierarchies (empty,
  single, parent-child, three-deep, realistic blog post),
  multiple roots (two h1s, starting at h2, outdent past root),
  malformed sequences (h1→h3 skip, h1→h6 skip, orphan h3,
  h2→h4→h3 stack-pop semantics, same-level repeats), href
  preservation, purity (no input mutation, byte-identical
  output, fresh tree per call), 100-heading stress test.
- `build-toc.test.ts` (18) — DocumentNode entry point with
  collision-suffix hrefs, empty/no-heading docs, options
  (minLevel, maxLevel, combo, defaults), `buildTocFromSlugs`
  parity, security (hostile heading text, control bytes),
  reproducibility, defaults match no-options call.

Coverage: **100% line / 100% branch** across all source files
with logic (`types.ts` is type-only).

## Spec adherence

Implements FM00 v0 §5.3 `transform-toc`.  No spec divergences.
Spec calls for "extract TOC, inject anchor points"; the anchor
points are produced by `forme-transform-autolink-headings`
upstream, and this package builds the extraction tree.

## v0 simplifications

- **No level remapping.**  An `<h2>` stays an `<h2>` in the
  output tree even with `minLevel: 2`.  Renderers that want
  "shift everything to start at h1" do that themselves.  v0
  preserves source levels for callers that need them.
- **No inclusion / exclusion by slug pattern.**  Only level-
  range filtering.  Headings can be skipped from a TOC via
  Markdown source convention (e.g. trailing `<!-- omit-toc -->`)
  but the heuristic is not in v0.
- **No max-depth cap (different from `maxLevel`).**  `maxLevel`
  caps absolute heading level; "stop after 2 levels of nesting
  regardless of source level" would be a separate option,
  deferred.
- **No counter / numbering.**  Renderers can apply CSS
  counters to produce "1.1.2" prefixes; this package emits the
  raw tree only.
