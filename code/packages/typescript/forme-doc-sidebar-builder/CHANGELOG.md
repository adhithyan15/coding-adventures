# Changelog — @coding-adventures/forme-doc-sidebar-builder

## 0.1.0 — 2026-05-22

Initial release.  Sixth concrete DOC00 v0 package — take a
directory layout (file paths) + each file's frontmatter and
produce a hierarchical, JSON-able sidebar navigation tree the
page-shell can render to HTML.

Pure transform: `{ pages: PageInput[] }` → `SidebarEntry[]`.
Capabilities `[]`.  **Zero runtime dependencies.**

### Added

- `buildSidebar(pages, options?): readonly SidebarEntry[]` —
  main entry.  Filters drafts, normalises paths, builds a
  directory trie, recursively emits a sorted sidebar tree.
- `humanise(slug): string` — standalone helper exported for
  callers building related UI text (breadcrumbs, page titles).
  Acronym-aware: `api → "API"`, `http → "HTTP"`, etc.
- `normalisePath(rawPath): { parts, isIndex }` — standalone
  helper exported for advanced callers building custom sidebar
  layouts.
- `stripRoot(parts, root): readonly string[] | null` — companion
  to `normalisePath` for root-prefix matching.
- `PageInput`, `BuildSidebarOptions`, `SidebarEntry`,
  `SidebarPageEntry`, `SidebarGroupEntry` types.

### Spec adherence

Implements DOC00 v0's `forme-doc-sidebar-builder` per
`code/specs/DOC00-docs-vision.md`:

> Take a directory layout (file paths) + each file's frontmatter
> (sidebar position, title overrides, group metadata) → sidebar
> nav structure.  Output is a plain JSON-able tree the page
> shell renders to HTML.

Spec adherence is direct.  Spec also mentions "group metadata"
— v0 honours `title` / `sidebar_label` / `sidebar_position` /
`draft` on each page (and on `index.md` pages for groups).
Explicit per-directory `category.json` files (a Docusaurus
convention) are deferred to v1.

### Algorithm

Three-phase pipeline, O(N log N) overall (dominated by sort):

1. **Filter + Normalise** — drop `draft: true` pages; strip
   extensions, leading/trailing slashes, optional `root`
   prefix; detect index pages.
2. **Trie build** — insert each page into a directory trie
   keyed by normalised parts.  Index pages attach to their
   directory's node; non-index pages go at leaves.  Duplicate
   slugs / duplicate indexes throw `TypeError`.
3. **Emit** — recursive depth-first walk; at each level, sort
   by `(position ?? +Infinity, label)`.  Locale-independent
   string compare; positioned entries first, unpositioned
   alphabetical last.

### Behavioural notes

- **Pure transform.**  Input `pages` array and input
  `frontmatter` objects are never mutated.  Verified by JSON
  snapshot.
- **Deterministic.**  Same input bytes → identical output bytes.
  Locale-independent ordering (`toLowerCase` not
  `toLocaleLowerCase`), V8's stable `Array.prototype.sort`.
- **Output is JSON-safe.**  No AST references, no `Date`s, no
  symbols.  `JSON.parse(JSON.stringify(result))` round-trips.
- **Frontmatter is read defensively via `Object.hasOwn`** so
  hand-crafted `{ __proto__: { title: "Polluted" } }` literals
  don't leak inherited values into the sidebar.  In practice
  `forme-doc-frontmatter` returns null-prototype objects
  anyway; this is defence-in-depth for callers using arbitrary
  frontmatter sources.
- **Group destinations from index pages.**  A `<dir>/index.md`
  (case-insensitive) becomes the group's `path` /
  `label` / `position` rather than a separate child entry.
  Sidebar widgets make the group label clickable in that case.
- **All-draft groups disappear.**  A group whose children all
  end up filtered out is also omitted from the output.
- **Acronym-aware humanisation.**  ~40-entry alias table covers
  `api`, `sdk`, `url`, `http`, `https`, `json`, `yaml`, `html`,
  `css`, `cli`, `cpu`, `gpu`, `io` (renders as `I/O`), etc.
  Built via `Object.create(null)` for defence-in-depth.

### Security posture

- **No `eval` / `new Function` / `JSON.parse`-with-reviver** —
  pure data construction.
- **No mutation** of inputs (verified).
- **`Object.hasOwn` on every frontmatter read** — defends
  against prototype-pollution-style attacks where the caller
  hands us `{ __proto__: { … } }`.
- **`Object.create(null)` for the acronym table** — directory
  named `__proto__` falls through to default capitalisation.
- **64-level directory-depth cap** in `normalisePath` —
  prevents stack-overflow DoS from adversarial deeply-nested
  inputs (e.g. `a/b/c/…/z.md` with 10k segments would otherwise
  trigger `RangeError` during `emit`'s recursive walk).  Real
  docs sites essentially never exceed ~8 levels; 64 leaves a
  wide safety margin.
- **No I/O** — capabilities `[]`.  Zero runtime dependencies.
- **Total-ordering comparator** — `Array.prototype.sort` is
  stable in V8 (and the contract since ES2019), so equal
  entries preserve input order.

### Tests

97 tests across 3 files:

- `labels.test.ts` (23) — humanise() coverage: kebab-case /
  snake_case / mixed, acronyms (12+), case-insensitive acronym
  lookup, Unicode preservation, edge cases (empty,
  only-separators, only-whitespace, run collapsing,
  `__proto__` defence).
- `path-utils.test.ts` (31) — extension stripping (`.md` /
  `.mdx` / `.html` / `.htm`, case-insensitive, non-doc
  preserved), slash handling (leading / trailing / multiple /
  empty segments), index detection (root / nested /
  case-insensitive / index-like-but-not), root-prefix stripping
  (matching / non-matching / multi-part / leading slash /
  whitespace), empty-path errors, `"/"` → empty parts case.
- `builder.test.ts` (43) — degenerate inputs, ordering (position
  ascending, alphabetical fallback, positioned-before-
  unpositioned, tie-breaks), labels (title / sidebar_label /
  fallthrough / non-string ignored / acronyms), drafts (skipped
  / `draft: false` not skipped / string `"true"` not draft /
  group collapses when empty), grouping (single-level / mixed
  root pages and groups / deeply nested), index pages (group
  destination / null path / not listed in children / root index
  / duplicate throws), duplicate detection (`.md` vs `.mdx` /
  `.md` vs `.html` / slugless `"/"`), root-prefix option,
  frontmatter robustness (non-numeric / NaN / Infinity /
  negative / unknown keys / `__proto__` defence), determinism,
  immutability, JSON-safety, realistic-doc scenario.

Coverage: **100% line / 100% branch / 100% function** across
all source files with logic (`types.ts` is type-only).

### v0 simplifications (documented)

- **No multi-sidebar support.**  v0 produces ONE tree.  Sites
  that want separate sidebars per section wrap the function and
  pass filtered page subsets.
- **No `category.json`-style explicit group metadata.**  Group
  labels come from `index.md` frontmatter or from the humanised
  directory name.  v1 can add explicit metadata injection.
- **Root index page's metadata is unsurfaced.**  A root
  `index.md` contributes to the implicit root group's metadata,
  but since we return `root.children`, the root group itself
  isn't visible.  Use a top-level `intro.md` for a navigable
  root entry.
- **No collapsed/expanded state.**  Output is structural only.
  Collapse state is the page-shell's concern.
- **Strict-boolean `draft` check.**  `draft: "true"` (string),
  `draft: 1` (number), etc. are NOT treated as drafts.  Only
  the literal boolean `true` suppresses a page.  Authors who
  accidentally quote their `draft` value will see their page
  in the sidebar — surfaces the mistake.
