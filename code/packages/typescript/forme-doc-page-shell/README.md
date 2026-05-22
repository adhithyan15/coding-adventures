# @coding-adventures/forme-doc-page-shell

> Seventh DOC00 v0 package — RTD-minimal two-column HTML
> page-shell. Wraps content in a sidebar + main + optional TOC
> layout with header (brand + search + GitHub) and footer
> (version + edit link + copyright).

Pure transform. Capabilities: `[]`. **Zero runtime dependencies.**

## What it does

```ts
import { renderPageShell } from "@coding-adventures/forme-doc-page-shell";

const { head, body } = renderPageShell({
  site: {
    title: "My Docs",
    homeUrl: "/",
    githubUrl: "https://github.com/me/repo",
    version: "1.2.3",
    copyright: "© 2026 Me",
  },
  page: {
    title: "Getting Started",
    body: "<p>Welcome to the docs.</p>",  // TRUSTED: pre-rendered HTML
    description: "Quick-start guide",
    breadcrumbs: [
      { label: "Home", href: "/" },
      { label: "Guide", href: "/guide" },
      { label: "Getting Started", href: "/guide/start" },
    ],
    toc: [
      { text: "Install", id: "install", level: 2, children: [] },
      { text: "Run", id: "run", level: 2, children: [] },
    ],
    editUrl: "https://github.com/me/repo/edit/main/guide/start.md",
  },
  sidebar: [
    { kind: "page", label: "Intro", path: "/intro" },
    { kind: "group", label: "Guide", path: "/guide", children: [
      { kind: "page", label: "Getting Started", path: "/guide/start" },
    ]},
  ],
  options: {
    currentPath: "/guide/start",
    searchPlaceholder: "Search docs…",
  },
});
// head: <meta charset>… <title>Getting Started | My Docs</title> …
// body: <header>…</header><div class="layout">…sidebar…main…</div><footer>…</footer>
```

The output is `{ head, body }` — two HTML chunks suitable for
`forme-aot-html-doc-emitter`'s `head` + `body` fields. The emitter
wraps these into a full `<!DOCTYPE html><html>…</html>` document.
This package never emits the outer `<html>` / `<head>` / `<body>`
tags themselves.

## Shell structure

```
head:
  <meta charset="utf-8">
  <meta name="viewport" …>
  <title>{page.title} | {site.title}</title>
  [<meta name="description" content="{page.description}">]
  [{options.headExtra raw}]

body:
  <header class="site-header">
    <a class="brand" href="{site.homeUrl ?? "/"}">{site.title}</a>
    [<input class="search" placeholder="{searchPlaceholder}">]
    [<a class="github" href="{site.githubUrl}">GitHub</a>]
  </header>
  <div class="layout">
    <nav class="sidebar">{sidebar tree as nested <ul>}</nav>
    <main>
      [<ol class="breadcrumbs">…</ol>]
      <article>
        <h1>{page.title}</h1>
        {page.body raw}
      </article>
      [<aside class="toc">…</aside>]
    </main>
  </div>
  <footer class="site-footer">
    [<span class="version">v{site.version}</span>]
    [<a class="edit" href="{page.editUrl}">Edit this page</a>]
    [<span class="copyright">{site.copyright}</span>]
  </footer>
```

Square brackets mark optional blocks (rendered only when the
underlying field is present and non-empty).

## Security (XSS defence)

This is the page-shell's primary concern. **Every user-supplied
string is escaped** before interpolation:

| Field                           | Treatment                             |
|---------------------------------|---------------------------------------|
| `site.title`, `site.copyright`, `site.version` | `escapeHtml`               |
| `site.homeUrl`, `site.githubUrl`, `page.editUrl` | `safeHref` (allowlist + escape) |
| `page.title`, `page.description` | `escapeHtml`                          |
| `page.breadcrumbs[].label`      | `escapeHtml`                          |
| `page.breadcrumbs[].href`       | `safeHref`                            |
| `page.toc[].text`               | `escapeHtml`                          |
| `page.toc[].id`                 | `escapeAttr` (defensive — slugs are URL-safe anyway) |
| `sidebar[].label`               | `escapeHtml`                          |
| `sidebar[].path`                | `safeHref`                            |
| `options.searchPlaceholder`     | `escapeAttr` (used in `placeholder` and `aria-label`) |

The **URL-scheme allowlist** in `safeHref` accepts `http:`,
`https:`, `mailto:`, relative URLs, and `#anchor` fragments.
Anything else — `javascript:`, `data:`, `vbscript:`, `file:`,
`ftp:`, etc. — becomes `"#"` (visually broken but inert). The
scheme check is case-insensitive.

**Two TRUSTED fields** are passed through verbatim and clearly
documented:

- **`page.body`** — pre-rendered HTML from the upstream markdown
  pipeline (`commonmark-parser` → `heading-anchors` →
  `code-block-decorator` → `syntax-highlighter` → HTML
  renderer). Those packages enforce their own structural
  invariants; passing the body through unchanged is the whole
  point of having them.
- **`options.headExtra`** — explicit escape hatch for analytics
  scripts, custom stylesheets, etc. Caller's responsibility to
  ensure safety.

If you're building `page.body` from untrusted user input
directly (e.g. a CMS), you MUST escape it first. That's outside
this package's contract.

## Public API

| Export                  | Purpose                                                                       |
|-------------------------|-------------------------------------------------------------------------------|
| `renderPageShell(input)` | Main entry. Returns `{ head, body }`.                                        |
| `renderSidebar(entries, currentPath?)` | Standalone sidebar `<nav>` renderer.                          |
| `renderToc(entries)`     | Standalone TOC `<aside>` renderer.                                           |
| `renderBreadcrumbs(items)` | Standalone breadcrumbs `<ol>` renderer.                                    |
| `escapeHtml(s)`          | Five-char HTML escape (`&<>"'`).                                              |
| `escapeAttr(s)`          | Alias for `escapeHtml` (attribute-context).                                   |
| `safeHref(raw)`          | URL allowlist + escape. Returns `"#"` for rejected schemes.                   |
| Types                    | `SiteConfig`, `PageInfo`, `Breadcrumb`, `TocEntry`, `SidebarEntry`, `PageShellOptions`, `PageShellInput`, `PageShellOutput`. |

## Active-page highlighting

When `options.currentPath` matches a sidebar entry's `path`,
that entry's `<a>` gets `aria-current="page"`. Themes hook on
this attribute for the "you are here" visual. Both page entries
and group entries (with index pages) participate.

## Tests

98 tests across five files:

- `escape.test.ts` (40) — every escape character, escape ordering,
  XSS payloads (`<script>`, `onerror`, attribute breakout in
  both quote styles), non-string coercion, URL allowlist (safe
  schemes pass, `javascript:`/`data:`/`vbscript:`/`file:`/`about:`/`ftp:`
  all rejected, case-insensitive), URL edge cases (empty, whitespace,
  query strings, malformed).
- `render-sidebar.test.ts` (16) — degenerate / pages / groups
  (with and without index) / nested / active-page highlighting /
  XSS in labels and paths / `javascript:` rejected / defensive
  defaults (undefined children).
- `render-toc.test.ts` (5) — degenerate / basic / nested /
  XSS in text and id.
- `render-breadcrumbs.test.ts` (5) — degenerate / single (current page)
  / multi-item / XSS in label / `javascript:` href rejected.
- `render-shell.test.ts` (32) — head (charset, viewport, title,
  description, headExtra, XSS in title / description), body
  header (brand link / homeUrl / GitHub / search / XSS in title
  / `javascript:` in GitHub), body main (trusted body / h1 with
  escaped title / optional breadcrumbs / optional TOC), body
  sidebar (entries / currentPath highlighting), body footer
  (empty / version / edit / copyright / all three / XSS in
  copyright and version), structure (head and body both emitted /
  order is header → layout → footer / layout contains sidebar
  and main), determinism, immutability.

Coverage: **100% line / 100% branch / 100% function** on every
source file with logic (`types.ts` is type-only).

## How it fits in the stack

Seventh concrete DOC00 v0 package. Sits at the very end of the
per-page content pipeline, just before the AOT HTML doc emitter
that produces the final `<!DOCTYPE html>` files:

```
.md → frontmatter → commonmark-parser → heading-anchors → toc-extractor
                                                              ↓
                                          code-block-decorator
                                                              ↓
                                          syntax-highlighter
                                                              ↓
sidebar-builder ───────────────────────►  page-shell (this package)
                                                              ↓
                                                    { head, body }
                                                              ↓
                                          forme-aot-html-doc-emitter
                                                              ↓
                                                     `<!DOCTYPE html>…</html>`
```

Remaining DOC00 v0 packages: `forme-doc-search-tokenizer`,
`forme-doc-search-index-builder`, `forme-doc-search-client-js`,
`forme-doc-site-emitter`.

## v0 simplifications (documented)

- **No theme system.** v0 emits semantic class names (e.g.
  `site-header`, `sidebar`, `toc`); CSS lives elsewhere.
  v1 may add a theme parameter that influences markup.
- **No dark mode toggle.** A purely client-side concern;
  themes hook on `prefers-color-scheme` directly.
- **No collapsible sidebar sections.** Markup is flat
  `<ul>` / `<li>`; client-side JS (in
  `forme-doc-search-client-js` or a sibling) can toggle
  visibility if desired.
- **No search-result rendering.** The search input is
  non-interactive in v0 markup; `forme-doc-search-client-js`
  wires it up at runtime.
- **No locale/i18n.** UI strings (`"On this page"`, `"Edit
  this page"`, `"GitHub"`, etc.) are hard-coded English.
  v1 may add a `messages` option.
