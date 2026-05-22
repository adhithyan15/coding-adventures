# Changelog — @coding-adventures/forme-doc-page-shell

## 0.1.0 — 2026-05-22

Initial release.  Seventh concrete DOC00 v0 package — RTD-minimal
two-column HTML page-shell.  Wraps content in a sidebar + main +
optional TOC layout with header (brand + search + GitHub) and
footer (version + edit link + copyright).

Output is `{ head, body }` HTML chunks suitable for
`forme-aot-html-doc-emitter` to wrap into a full `<!DOCTYPE html>`
document.  Pure transform; capabilities `[]`; **zero runtime
dependencies**.

### Added

- `renderPageShell(input): { head, body }` — main entry.  Takes
  `{ site, page, sidebar, options? }` and returns two HTML chunks.
- `renderSidebar(entries, currentPath?): string` — standalone
  sidebar `<nav>` renderer with `aria-current="page"` highlighting.
- `renderToc(entries): string` — standalone TOC `<aside>` renderer.
- `renderBreadcrumbs(items): string` — standalone breadcrumbs
  `<ol>` renderer.
- `escapeHtml(s): string` — five-character HTML escape (`&<>"'`).
- `escapeAttr(s): string` — alias for `escapeHtml`
  (attribute-context).
- `safeHref(raw): string` — URL-scheme allowlist (`http:`,
  `https:`, `mailto:`, relative, `#anchor`) + escape.  Rejected
  schemes (`javascript:`, `data:`, `vbscript:`, `file:`, etc.)
  become `"#"`.
- Types: `SiteConfig`, `PageInfo`, `Breadcrumb`, `TocEntry`,
  `SidebarEntry`, `PageShellOptions`, `PageShellInput`,
  `PageShellOutput`.

### Spec adherence

Implements DOC00 v0's `forme-doc-page-shell` per
`code/specs/DOC00-docs-vision.md`:

> Wrap content in the RTD-minimal two-column shell: sidebar on
> the left, content in the middle, optional in-page TOC on the
> right.  Header bar at the top with site title, GitHub link,
> search input.  Footer with version / "edit this page" link /
> copyright.  Outputs HTML chunks suitable for
> `forme-aot-html-doc-emitter`'s `head` + `body` fields.

All of the above is present in v0.  The search input is
non-interactive markup; `forme-doc-search-client-js` will wire
it up at runtime (per the spec's separation of build-time vs
client-side concerns).

### Behavioural notes

- **Pure transform.**  Input objects are never mutated.
  Verified by JSON snapshot.
- **Deterministic.**  Same input bytes → identical output bytes.
- **Output is JSON-safe.**  Both `head` and `body` are plain
  strings; no AST refs, no `Date`s, no symbols.
- **Conditional blocks.**  Optional fields (`description`,
  `headExtra`, `breadcrumbs`, `toc`, `githubUrl`,
  `searchPlaceholder`, `version`, `editUrl`, `copyright`) are
  rendered only when present and non-empty.  Absent fields
  produce no markup at all — themes don't need to handle
  empty-state special-cases.
- **`aria-current="page"`** added to the sidebar entry whose
  `path` matches `options.currentPath`.  Both leaf pages and
  group index pages participate.
- **Default `homeUrl`** is `"/"` when omitted.  Default
  `lang` is `"en"` (though v0 doesn't actually emit
  `<html lang="…">` — that's the AOT doc emitter's job).

### Security posture

XSS defence is this package's primary concern.

- **`escapeHtml` on every user-supplied string** — site title,
  page title, description, copyright, version, sidebar labels,
  breadcrumb labels, TOC text, search placeholder.
- **`safeHref` on every URL** — `home`, `github`, `edit`,
  `sidebar paths`, `breadcrumb hrefs`.  Allowlists `http:`,
  `https:`, `mailto:`, relative URLs, and `#anchor` fragments.
  Case-insensitive scheme check.  Rejected URLs become `"#"`
  (visually broken but inert).
- **Escape ordering** — `&` replaced first, then `<>"'`, so
  emitted entities don't get re-escaped.
- **Two TRUSTED fields documented**:
  - `page.body` — pre-rendered HTML from the upstream markdown
    pipeline; passed through verbatim.  Callers building this
    from untrusted CMS-style input MUST escape upstream.
  - `options.headExtra` — explicit escape hatch for analytics
    scripts, custom stylesheets.  Caller's responsibility.
- **No `eval` / `new Function` / `JSON.parse`-with-reviver**.
- **No I/O** — capabilities `[]`.  Zero runtime dependencies.
- **No regex DoS** — `safeHref`'s scheme-extraction regex is
  bounded (single non-greedy alternation on a small character
  class, anchored at `^`); the five-char `escapeHtml` regexes
  are single-character classes with linear behaviour.

### Tests

98 tests across 5 files:

- `escape.test.ts` (40) — every escape character, escape
  ordering (& first), XSS payloads (script tags, img onerror,
  attribute breakout both quote styles), non-string coercion;
  URL allowlist (safe schemes pass: relative / absolute path /
  `./` / `../` / http / https / mailto / `#fragment` /
  case-insensitive; rejected: javascript / data / vbscript /
  file / about / ftp / case variants); URL edge cases (empty,
  whitespace, query-string `&` escaped, double-quote escaped,
  malformed schemes).
- `render-sidebar.test.ts` (16) — empty / single page / multiple
  / null-path defensive span / groups with and without index /
  nested groups / active-page highlighting on leaves AND group
  index / no `currentPath` → no aria / XSS in label / XSS in
  query string / `javascript:` rejected / group label quotes
  escaped / undefined `children` defensive default.
- `render-toc.test.ts` (5) — empty → empty string / single
  entry / nested / XSS in text / XSS in id.
- `render-breadcrumbs.test.ts` (5) — empty → empty / single
  (current page) / multi-item ordering / XSS in label /
  `javascript:` href rejected.
- `render-shell.test.ts` (32) — head (charset, viewport, title,
  description present/absent, headExtra raw passthrough, XSS in
  title and description); body header (brand link with default
  and custom homeUrl, GitHub link presence, search input
  presence, XSS in site title, `javascript:` githubUrl
  rejected); body main (trusted body verbatim, h1 with escaped
  title, optional breadcrumbs, optional TOC); body sidebar
  (entries rendered, `currentPath` highlights); body footer
  (empty footer emitted, version, edit, copyright, all three,
  XSS in copyright and version); structure (head and body
  both non-empty, header → layout → footer order, layout
  contains sidebar AND main); determinism; immutability via
  JSON snapshot.

Coverage: **100% line / 100% branch / 100% function** across
all source files with logic (`types.ts` is type-only).

### v0 simplifications (documented)

- **No theme system** — emits semantic class names only.
- **No dark mode toggle** — client-side concern; themes can
  hook on `prefers-color-scheme`.
- **No collapsible sidebar** — markup is flat; client-side JS
  can toggle visibility.
- **No interactive search** — input is non-interactive markup;
  `forme-doc-search-client-js` will wire it up at runtime.
- **No i18n** — UI strings (`"On this page"`, `"Edit this page"`,
  `"GitHub"`, `"Breadcrumb"`, `"Site navigation"`) are
  hard-coded English.  v1 may add a `messages` option.
- **No `<!DOCTYPE>` / `<html>` / `<head>` / `<body>` tags** —
  these belong to `forme-aot-html-doc-emitter` which wraps our
  chunks.
