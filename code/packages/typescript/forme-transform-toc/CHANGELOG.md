# Changelog — @coding-adventures/forme-transform-toc

## 0.1.0 — 2026-05-18

Initial release.  Sixth FM00 v0 stage package — extracts a
hierarchical Table-of-Contents tree from a `DocumentNode` (or
pre-computed `HeadingSlug[]`).

Sits alongside `forme-feeds`, `forme-opengraph`,
`forme-index-renderer`, `forme-transforms`, and
`forme-transform-autolink-headings`.

### Added

- `buildToc(doc, options?): TocNode[]` — top-level entry.  Walks
  `doc` via `autolinkHeadings` to get the slug stream, filters
  by level range, builds the hierarchical tree.
- `buildTocFromSlugs(slugs, options?): TocNode[]` — same, but
  takes a pre-computed `HeadingSlug[]`.  Use when caller already
  invoked `autolinkHeadings` for the renderer and doesn't want
  to walk the AST twice.
- `buildTree(slugs): TocNode[]` — sub-helper: pure flat →
  hierarchical tree construction without filtering.
- `filterByLevel(slugs, minLevel, maxLevel): HeadingSlug[]` —
  sub-helper: drop slugs outside the level range.
- `TocNode` and `TocOptions` types.

### Spec adherence

Implements FM00 v0 §5.3 `transform-toc`.  No spec divergences.
Spec calls for "extract TOC, inject anchor points"; the anchor
points are produced by `forme-transform-autolink-headings`
upstream, and this package builds the extraction tree.

### Behavioural notes

- **Stack-based tree construction.**  Single forward pass over
  the flat heading list, maintaining a stack of currently-open
  ancestor nodes.  For each incoming heading: pop ancestors at
  the same or deeper level (their subtrees close), append as
  child of stack-top (or as root if stack empty), push the new
  node.  O(N) — each heading pushed once, popped at most once.
- **Malformed sequences handled gracefully.**  Skipped levels
  (h1 → h3) just nest the deeper heading as a child of the
  shallower one — no interpolated empty heading, no throw.
  Documents starting at a deep level (first heading is h3)
  treat that h3 as a root.  Outdent past root (h1 after deep
  nesting) closes all open subtrees and starts a new root.
- **Source level preserved.**  Filtering by `minLevel: 2` doesn't
  remap surviving h2 headings down to h1; the `level` field
  stays at its source value for callers that want level-
  specific styling.
- **Filter then build, not build then prune.**  Filtering
  happens before tree construction.  A filtered-out heading
  does not preserve its nesting — its children promote to roots
  if no surviving ancestor exists.
- **Out-of-range options clamp.**  `minLevel: 0` clamps to 1;
  `maxLevel: 99` clamps to 6.  NaN clamps to 1.  Inverted
  ranges (`minLevel > maxLevel`) return an empty array.
- **`href` is always `"#" + slug`.**  Preserved from
  `HeadingSlug.anchorHref` directly; not re-concatenated to
  avoid accidental encoding bugs.

### Security posture

Three concerns explicitly addressed (pre-push review):

- **No AST mutation.**  Input arrays are never `.push`-ed into;
  the source `DocumentNode` is never modified.  JSON-snapshot
  tests confirm.
- **Heading text passthrough.**  The `text` field on `TocNode`
  preserves raw heading content verbatim — renderers must
  HTML-escape it when emitting markup.  Only the `slug` /
  `href` are sanitised (by `slugify` upstream).  This is the
  same contract every other FM00 stage observes: text is data,
  not markup, and the renderer owns the escape boundary.
- **Bounded computation.**  Tree construction is O(N) — stack
  pushes once, pops at most once per heading.  Adversarial
  inputs like 10,000 h6 headings under one h1 produce a tree of
  10,000 children, not exponential nesting.  No regex
  backtracking surface (no regex used).

### Capabilities

`[]` — pure transform.  No I/O, network, fs, shell, env.

### Tests

55 tests across 3 files:

- `filter.test.ts` (17) — defaults [1, 6] keep everything,
  minLevel drops shallow / maxLevel drops deep, combined,
  out-of-range clamping (negative, NaN, Infinity, fractional),
  inverted range → empty, purity (no mutation, fresh array,
  empty input).
- `build-tree.test.ts` (20) — well-formed (empty, single
  heading, h1+h2, h1+two siblings, h1>h2>h3, realistic blog
  post with mixed nesting), multiple roots (two h1s, doc
  starting at h2, outdent past root), malformed (h1→h3 skip,
  h1→h6 skip, orphan h3 root, h2→h4→h3 stack-pop semantics,
  same-level repeats become siblings), href preservation,
  purity (no input mutation, byte-identical output across
  calls, fresh tree per call with no shared substructure),
  100-heading stress test.
- `build-toc.test.ts` (18) — DocumentNode entry point with
  collision-suffix hrefs (`#setup`, `#setup-2`), empty doc /
  no-heading doc, options (minLevel, maxLevel, combo,
  defaults), `buildTocFromSlugs` parity with `buildToc` given
  the same slug stream, security (hostile heading text
  preserves raw text but sanitises slug, control bytes
  stripped from slug), reproducibility (FM03 byte-identical,
  no mutation of input doc), defaults match no-options call.

Coverage: **100% line / 100% branch** across all source files
with logic (`types.ts` is type-only declarations).

### v0 simplifications (documented)

- **No level remapping.**  An h2 stays an h2 in the output
  even with `minLevel: 2`.  Renderers can shift levels
  themselves.
- **No inclusion / exclusion by slug pattern.**  Only level-
  range filtering.  Per-heading omit hints (e.g. trailing
  `<!-- omit-toc -->`) are not in v0.
- **No max-nesting-depth cap.**  `maxLevel` caps absolute
  source level, not nesting depth from the first surviving
  ancestor.  Different option, deferred.
- **No counter / numbering.**  Renderers apply CSS counters to
  produce "1.1.2" prefixes; this package emits the raw tree.
