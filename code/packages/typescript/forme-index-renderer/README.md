# @coding-adventures/forme-index-renderer

Index / archive page renderer for the Forme pipeline (FM00 v0).

Pure transform: `IndexItem[]` + `IndexOptions` → reproducible HTML
`<ul>` (optionally grouped by category/year/month) suitable for
blog archives and index pages.  Pairs with
[`forme-aot-page-emitter`](../forme-aot-page-emitter): the emitter
writes the `.html` wrapper, this renderer fills the body.

Third FM00 v0 stage package — sits alongside
[`forme-feeds`](../forme-feeds) and
[`forme-opengraph`](../forme-opengraph).

## Quick start

```ts
import { renderIndexPage } from "@coding-adventures/forme-index-renderer";

const html = renderIndexPage(posts, {
  groupBy: "year",
  sortBy: "pubDate-desc",
  showDate: true,
  showSummary: true,
  dateFormat: (iso) => new Date(iso).toLocaleDateString("en-US"),
});
```

Output (with grouping):

```html
<section class="forme-index-group">
  <h2>2026</h2>
  <ul class="forme-index">
    <li>
      <a href="/posts/second">Second Post</a>
      <time datetime="2026-02-20T00:00:00Z">2/20/2026</time>
      <p class="summary">World</p>
    </li>
    ...
  </ul>
</section>
```

## API

### `renderIndexPage(items, options?): string`

Renders an HTML `<ul>` (or sequence of `<section>` blocks when
grouped) — drop straight into the body of an archive page.

### Types

```ts
interface IndexItem {
  id: string;                      // mandatory, used as sort tiebreaker
  title: string;
  url: string;                     // absolute http(s) OR root-relative "/path"
  pubDate?: string;                // ISO-8601
  summary?: string;
  category?: string;
  tags?: readonly string[];        // declared for forward compat; v0 doesn't render
}

interface IndexOptions {
  groupBy?: "none" | "category" | "year" | "month";   // default "none"
  sortBy?: "pubDate-desc" | "pubDate-asc" | "title-asc";   // default "pubDate-desc"
  showSummary?: boolean;           // default false
  showDate?: boolean;              // default false
  dateFormat?: (iso: string) => string;   // default: passthrough
}
```

### `groupItems(items, groupBy): ItemGroup[]`

Exposed sub-helper for callers who want the grouping logic without
rendering — e.g. building custom navigation widgets, year-archive
sidebars, or category clouds.

### `sortItems(items, sortBy): IndexItem[]`

Exposed sub-helper for callers who want deterministic sort without
rendering.

## Sort + group rules

| sortBy            | Order                                                  |
|-------------------|--------------------------------------------------------|
| `pubDate-desc` (default) | newest first; undated items last; ties → id asc |
| `pubDate-asc`     | oldest first; undated items last; ties → id asc        |
| `title-asc`       | alphabetical by title; ties → id asc                   |

| groupBy      | Headings                                                    |
|--------------|-------------------------------------------------------------|
| `none` (default) | single flat `<ul>`, no headings                         |
| `category`   | item.category (missing → `"Uncategorized"`); alphabetical; Uncategorized last |
| `year`       | 4-digit UTC year; missing pubDate → `"Undated"`; reverse-chronological; Undated last |
| `month`      | `YYYY-MM`; missing pubDate → `"Undated"`; reverse-chronological; Undated last |

Within each group, item order is preserved (so the sort happens
first, then grouping respects that order — `sortBy: "pubDate-desc"`
+ `groupBy: "year"` gives reverse-chrono inside each year).

## URL validation (security-critical)

`item.url` is validated:

- **Accepted:** absolute `http(s)://...` OR root-relative `/path` OR bare `/`
- **Rejected** (`TypeError` thrown BEFORE any output is emitted):
  - `javascript:alert(1)` → XSS injection vector
  - `data:text/html,...` → payload delivery
  - `file:///etc/passwd` → local file leak
  - Protocol-relative `//host/path` (ambiguous scheme)
  - Bare relative (`about`, `./about`) — caller should normalise to `/about`

## HTML escaping

All string fields (title, summary, category, URL) route through
`escapeHtmlAttr` / `escapeHtmlText` — both wrap a single-pass
replacement of the five HTML entities (`& < > " '`) over the input
*after* ASCII control bytes are stripped.

This handles the **attacker-controlled-category** case: a hostile
`category: "<script>alert(1)</script>"` lands as
`<h2>&lt;script&gt;alert(1)&lt;/script&gt;</h2>` rather than
executable HTML.

## Reproducibility (FM03)

Same inputs → byte-identical output.  Stable sort uses `id` as
tiebreaker, so two inputs that differ only in caller-array order
produce identical HTML.

## Capabilities — `[]`

Pure transform.  No I/O, no network, no shell.

## Tests

60 tests across 4 files:

- `escape.test.ts` (13) — entity escaping, control-byte stripping,
  URL acceptance (http/https/root-relative/bare-`/`) and rejection
  (javascript:, data:, file:, protocol-relative, bare relative,
  empty/non-string)
- `sort.test.ts` (10) — every sort mode, undated-items-last,
  ties-broken-by-id, input-not-mutated, malformed-pubDate handling
- `group.test.ts` (13) — every groupBy mode (none / category / year /
  month), Uncategorized / Undated buckets, deterministic group order,
  within-group order preservation, empty input
- `index-renderer.test.ts` (24) — flat list, sort dispatch, all three
  groupings, showDate / showSummary / dateFormat toggles, HTML
  escaping (title, summary, URL attribute, category heading),
  URL validation rejecting each forbidden form, reproducibility
  including reshuffled-input invariance

Coverage: **100% line / 92.45% branch** across all source files
with logic.  `types.ts` is type-only.

## Spec adherence

Implements FM00 v0 index renderer.  No spec divergences.

## v0 simplifications

- **Tags not displayed** — type slot reserved for forward compat;
  rendering would need taxonomy navigation (cloud / list / filter)
  which v0 defers.
- **No pagination.**  Render the full list; consumers wanting
  paginated archives slice their `items[]` before calling.
- **No per-group sort override.**  All groups share the same
  `sortBy` (since grouping happens after sorting).
- **`tags` field is declared but unused in v0 renderer.**
