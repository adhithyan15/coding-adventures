# Changelog — @coding-adventures/forme-index-renderer

## 0.1.0 — 2026-05-18

Initial release.  Third FM00 v0 stage package — index / archive page
renderer producing reproducible HTML `<ul>` (optionally grouped by
category / year / month) from `IndexItem[]` + `IndexOptions`.

Companion to `forme-feeds` and `forme-opengraph`; pairs with
`forme-aot-page-emitter` which writes the surrounding `.html`
wrapper.

### Added

- `renderIndexPage(items, options?): string` — emits an HTML `<ul
  class="forme-index">` (or a sequence of `<section
  class="forme-index-group"><h2>...</h2><ul>...</ul></section>`
  blocks when grouping is enabled).  Drop straight into an archive
  page body.
- `groupItems(items, groupBy): ItemGroup[]` — exposed sub-helper so
  callers can build custom navigation widgets, year-archive sidebars,
  or category clouds without re-rendering.
- `sortItems(items, sortBy): IndexItem[]` — exposed sub-helper for
  callers that want the deterministic sort order without rendering.
- `escapeHtmlAttr`, `escapeHtmlText`, `assertItemUrl` re-exported
  for callers wiring custom item renderers.
- `IndexItem`, `IndexOptions`, `ItemGroup` type definitions.

### Spec adherence

Implements FM00 v0 index renderer.  No spec divergences.

### Behavioural notes

- **URL validation FIRST.**  Every `item.url` is validated against
  the accept-list (absolute `http(s)://`, root-relative `/path`,
  bare `/`) BEFORE any output is emitted.  Mixed good/bad inputs
  throw synchronously with no partial output.
- **Stable sort with id tiebreaker.**  Two inputs that differ only
  in caller-array order produce byte-identical HTML (FM03
  reproducibility requirement).
- **Undated / Uncategorized buckets sort LAST.**  Missing `pubDate`
  → `"Undated"` group / sorted to end.  Missing `category` →
  `"Uncategorized"` group / sorted last.
- **Within-group order preserved.**  Sort happens before grouping,
  so `sortBy: "pubDate-desc"` + `groupBy: "year"` gives
  reverse-chrono within each year heading.
- **HTML attribute escaping uniform across all interpolations.**
  Single-pass replace of the five HTML entities (CodeQL-friendly
  form).  Control-byte stripping (`U+0000-U+001F`, `U+007F`) runs
  before the escape pass.
- **Display toggles are opt-in.**  `showDate` / `showSummary`
  default to `false`; items without `pubDate` / `summary` skip the
  respective markup rather than emitting empty tags.

### Security posture

Three concerns explicitly addressed (pre-push review):

- **HTML attribute injection.**  `escapeHtmlAttr` covers all five
  HTML entities in single-pass replacement.  Every interpolated
  string (title, summary, URL, category heading) routes through it
  (or `escapeHtmlText`, which is an alias).
- **URL scheme validation.**  `assertItemUrl` rejects
  `javascript:`, `data:`, `file:`, protocol-relative `//host`,
  bare relative paths (`about`, `./about`), empty strings, and
  non-string inputs.  These are real XSS / payload-delivery vectors
  in archive pages whose link list comes from user-authored
  frontmatter.  Tests pin every rejected form.
- **Attacker-controlled category heading.**  A hostile
  `category: "<script>alert(1)</script>"` lands as
  `<h2>&lt;script&gt;alert(1)&lt;/script&gt;</h2>` rather than
  executable HTML.  Pinned by `index-renderer.test.ts`.

### Capabilities

`[]` — pure transform.  No I/O, no network, no shell, no env.

### Tests

60 tests across 4 files:

- `escape.test.ts` (13) — every HTML entity (single + composite),
  control-byte stripping for `0x00-0x1F` and `0x7F`, URL
  acceptance (http, https, root-relative, bare `/`) and rejection
  (javascript:, data:, file:, protocol-relative, bare relative,
  empty, non-string)
- `sort.test.ts` (10) — every sort mode (`pubDate-desc`,
  `pubDate-asc`, `title-asc`), undated-items-last,
  ties-broken-by-id, input-not-mutated, malformed-pubDate handling
- `group.test.ts` (13) — every groupBy mode (`none` / `category` /
  `year` / `month`), Uncategorized / Undated buckets, deterministic
  group order, within-group order preservation, empty input
- `index-renderer.test.ts` (24) — flat list, sort dispatch, all
  three groupings, `showDate` / `showSummary` / `dateFormat`
  toggles, HTML escaping (title, summary, URL attribute, category
  heading), URL validation rejecting each forbidden form,
  reproducibility including reshuffled-input invariance

Coverage: **100% line / 92.45% branch** across all source files
with logic (`types.ts` is type-only declarations).

### v0 simplifications (documented)

- **Tags not displayed** — `tags` field is declared on `IndexItem`
  for forward compat but the v0 renderer does not emit them.
  Rendering would need taxonomy navigation (cloud / list / filter)
  which v0 defers.
- **No pagination.**  The full list renders in one pass; consumers
  wanting paginated archives slice their `items[]` before calling.
- **No per-group sort override.**  All groups share the same
  `sortBy` (since grouping happens after sorting).
