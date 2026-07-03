# forme-doc-demo

> End-to-end demo of the DOC00 v0 documentation-site cluster.
> Reads a small Markdown corpus, runs it through all 11
> `forme-doc-*` packages plus the FM00 page-bundle-emitter, and
> writes a fully-functional static site to `dist/`.

The point of this program is to **see the DOC00 v0 cluster
working** — every package wired together against a real corpus,
with browsable HTML on the other side.

## Quick start

```bash
cd code/programs/typescript/forme-doc-demo

# install + build
npm install
npm start
# → reads ./corpus, writes ./dist (6 pages + sidebar + search)

# serve dist/ with anything that speaks static files
npx serve dist
# or
python3 -m http.server --directory dist
```

Open the printed URL.  The demo includes:

- A two-column page shell (sidebar on the left, content in the
  middle, in-page TOC on the right).
- Heading anchors (every `<h*>` gets an `id` so deep-links work).
- Syntax-highlighted code blocks (bash, typescript, javascript,
  css).
- A working sidebar with one group (Guide) and three top-level
  pages.
- A pre-built search index (one manifest + ~130 alphabetised
  shards under `/search/`) PLUS a working browser search box
  in the header — try typing `install`, `widget`, `theme`,
  `deno`, `render`.  Results dropdown is debounced (120 ms);
  ESC clears; click outside dismisses.

## What it actually does

```
corpus/*.md                  (six handwritten markdown pages with
                              frontmatter)
   │
   │  fs:read (walks corpus/)
   ▼
┌──────────────── build() pipeline ────────────────────────────┐
│                                                              │
│  per page:                                                   │
│    extractFrontmatter  ── splits YAML/TOML from body         │
│    parseMarkdown       ── commonmark-parser → DocumentNode   │
│    generateHeadingAnchors                                    │
│    decorateCodeBlocks                                        │
│    highlightCodeBlocks ── TextMate grammars                  │
│    extractToc          ── for the in-page nav                │
│    toHtml              ── document-ast-to-html               │
│    injectHeadingIds    ── small local helper (see below)     │
│    renderPageShell     ── two-column chrome                  │
│    generateHtmlDocument── final <!doctype html>...           │
│                                                              │
│  cross-page:                                                 │
│    buildSidebar          ── tree from paths + frontmatter    │
│    normaliseSidebarPaths ── small local helper (see below)   │
│    buildSearchIndex      ── Porter-stemmed, prefix-sharded   │
│                                                              │
│  composition:                                                │
│    emitSite              ── PageBundleConfig                 │
│                                                              │
└──────────────────────────────────────────────────────────────┘
   │
   │  fs:write (iterates bundle.pages, writes each one)
   ▼
dist/
├── index.html
├── getting-started/index.html
├── faq/index.html
├── guide/installation/index.html
├── guide/configuration/index.html
├── api/reference/index.html
├── sidebar.json
└── search/
    ├── manifest.json
    └── <prefix>.json × N
```

## Capability discipline

| Package                              | Capability                |
|--------------------------------------|---------------------------|
| All 11 `forme-doc-*` libraries       | `[]` (pure transforms)    |
| `commonmark-parser`, `document-ast-to-html`, `forme-aot-html-doc-emitter`, `forme-aot-page-bundle-emitter` | `[]` |
| **`forme-doc-demo` (this program)**  | **`fs:read`, `fs:list`, `fs:write`, `fs:create`** |

The I/O capability lives at the leaf — exactly where the DOC00
spec puts it.  Every library in the chain remains testable in
memory.

Capability scope: `fs:read` is bounded to `./corpus/**`,
`fs:write` to `./dist/**` (with a `safeJoin` containment check
on every write target).  No network, no shell, no env.

## Two small local helpers

This driver does two things itself that don't fit cleanly into
any existing package, both `<60 lines`:

1. **`injectHeadingIds`** — walks the rendered HTML and injects
   `id="..."` attributes into each `<h*>` tag from the
   heading-anchors result.  Necessary because
   `document-ast-to-html`'s `renderHeading` ignores the
   `AnchoredHeadingNode.id` field.  Positional match (Nth
   `<h*>` in HTML order ↔ Nth anchor in document order);
   trivial ReDoS-free regex (`/<h([1-6])>/g`).
2. **`normaliseSidebarPaths`** — rewrites each sidebar entry's
   `path` field from a filename (`"guide/setup.md"`) to a URL
   route (`"/guide/setup"`).  Necessary because the
   sidebar-builder emits file paths and page-shell's
   `currentPath` comparison needs URL routes.

Both are recursive walks over small trees; both are exported
and unit-tested.

## The corpus

Six handwritten markdown pages:

```
corpus/
├── index.md                   (home)
├── getting-started.md
├── faq.md
├── guide/installation.md
├── guide/configuration.md
└── api/reference.md
```

They describe a fictional `@acme/widget` library — the content
is designed to exercise paragraph rendering, ordered and
unordered lists, fenced code blocks in multiple languages,
heading nesting, inline code, and internal cross-page links.

## Running the demo

```bash
npm start
# defaults: ./corpus → ./dist

# custom paths
npx tsx src/main.ts ./my-corpus ./public
```

## Tests

46 tests in `tests/build.test.ts`:

- **`routeFor`** — every input form (root, nested, `./`, `/`,
  `.md`, `.mdx`).
- **`titleOf`** — frontmatter precedence, basename humanising
  fallback, non-string frontmatter rejection.
- **`injectHeadingIds`** — in-order injection, extra `<h*>` tag
  handling, HTML-escape defence-in-depth.
- **`normaliseSidebarPaths`** — file → route rewrite, null path
  preservation.
- **`build` end-to-end** — the cluster correctly emits one route
  per page + sidebar + search manifest + search shards; the
  rendered HTML carries the expected fingerprints of every
  upstream package (heading anchors, sidebar highlight, fenced
  code blocks, theme CSS, site title).
- **`writeBundle`** — actual disk output with nested
  directories and `search/` shards.
- **`safeJoin`** — rejects `..`, absolute, and prefix-string
  false-match attacks.
- **`validateOutDir`** — rejects empty, non-string, and system
  directories.

Run:

```bash
npm test
# or
npm run test:coverage
```

## Why this lives in `programs/` not `packages/`

Programs are end-of-line consumers — runnables, not building
blocks.  They're allowed to own capabilities.  Libraries in
`packages/` stay pure so consumers can compose them safely.

Putting the demo here means:

- It can own `fs:read` + `fs:write` without leaking those
  capabilities into the upstream cluster.
- It can pull in a hand-curated corpus and a default theme
  without polluting the library packages.
- Killing or replacing the demo doesn't churn any of the 11
  shipped library packages.

## Search wire-up

The demo bundles `forme-doc-search-client-js` (plus its only
dep, `forme-doc-search-tokenizer`) via esbuild into a single
~9KB self-executing IIFE, written to `/search/client.js` and
loaded by every page with a `<script defer>` tag.  The
bootstrap inside the bundle:

- Finds the page-shell's existing `<input class="search">`.
- Lazily fetches `/search/manifest.json` on the first
  keystroke (one round-trip per session; cached via
  `SearchClient`'s built-in LRU).
- Renders results as a floating dropdown anchored under the
  input via `getBoundingClientRect()` + `position: fixed` —
  doesn't disturb the header flex layout, doesn't depend on
  any ancestor's positioning context.
- Every keystroke is debounced 120 ms; stale-query guard
  via a monotonic `queryId`.
- ESC clears, click-outside dismisses, focus re-opens if
  there's text.

### Cache busting

Every build computes a 12-hex-char SHA-256 of the bundle and
uses it as a `?v=<id>` query string on the script tag AND on
the manifest / shard fetches.  Same content → same hash →
browser cache reused; different content → new URL → browser
re-fetches.  Defeats Safari's aggressive same-URL caching,
which would otherwise pin a stale client.js across rebuilds.

### Shape adapter

`forme-doc-site-emitter` serialises shard `postings` as plain
JSON objects (Maps don't survive `JSON.stringify`).  The
browser-side `fetchShard` callback converts each object back
into a `Map<token, Posting[]>` so `SearchClient.isLikelyShard`
accepts the shape:

```ts
async function fetchShard(key) {
  const raw = await (await fetch(`/search/${key}.json?v=${buildId}`)).json();
  return { shardKey: raw.shardKey, postings: new Map(Object.entries(raw.postings)) };
}
```

This adapter lives at the consumer boundary, not in either
library — neither `forme-doc-search-client-js` (capability
`[]`) nor `forme-doc-site-emitter` (capability `[]`) had to
change to make search work in the browser.

## v0 simplifications

- No watch mode / live reload (run `npm start` again after
  editing).
- No multi-locale support.
- No custom theme system — one inlined stylesheet.
- No image / asset copying — corpus is text-only.
- Search bundle uses esbuild; v1 could swap for rollup / tsc.

The cluster composes correctly end-to-end and the search
loop closes in the browser.
