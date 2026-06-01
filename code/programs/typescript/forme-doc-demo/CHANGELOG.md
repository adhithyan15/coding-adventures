# Changelog — forme-doc-demo

## 0.2.0 — 2026-06-01

Wire the browser-side search client into the demo so the
header search box actually searches.  Closes the search loop
end-to-end:

```
  user types in <input class="search">
    → debounced 120ms
    → SearchClient.search(q) in the browser
    → fetches /search/manifest.json + needed shards
    → renders dropdown of pageId + matched-token results
    → click navigates to /pageId
```

### Added

- `src/search-bundle.ts` — bundles
  `forme-doc-search-client-js` + UI bootstrap into a single
  self-executing IIFE script.  Uses esbuild (added as devDep).
  Output is ~9KB minified, targets ES2020, no globals leaked.
- `build.ts.buildIdFor(bundleSrc)` — content-addressed
  12-hex-char hash used as a cache-bust query string on the
  script tag + manifest + shard fetches.  Defeats Safari's
  aggressive caching when we rebuild.
- `BROWSER_ENTRY_SOURCE` template in `search-bundle.ts`:
  - Finds `<input class="search">` (the page-shell's existing
    rendered element — no markup changes needed).
  - Builds a debounced `input` handler (120 ms debounce; stale-
    query guard via monotonic queryId).
  - Renders results as a `<ul role="listbox">` floated at
    `position: fixed` and anchored to the input via
    `getBoundingClientRect()`.  Repositioned on scroll/resize.
  - Click-outside + ESC dismissal.
  - XSS-safe — every text insertion is `textContent`, never
    `innerHTML`.
- Map adapter inside the bootstrap's `fetchShard` callback —
  converts the server's JSON-object `postings` (Maps don't
  survive `JSON.stringify`) back into a `Map<string, Posting[]>`
  so SearchClient's `isLikelyShard` shape check passes.
- 13 new tests in `tests/search-bundle.test.ts`:
  - **Unit** (7): bundle is non-empty, IIFE-wrapped, contains
    SearchClient + tokenize + Map-adapter + correct DOM
    selector + buildId reader; size ≤ 20KB minified.
  - **End-to-end in JSDOM** (6): builds the site fresh,
    stubs fetch, types queries into the rendered input, and
    asserts the dropdown populates with real results.
    Regression suite for the two bugs we caught in dev:
    (a) "No matches" for every query — caused by `plainText`
        reading `node.text` instead of `node.value`; the
        search index ended up with only title tokens.
    (b) "No matches" after Safari cache pinning — caused by
        a stable client.js URL; fixed via the content-
        addressed cache-bust above.

### Fixed

- **`plainText` was returning empty string for every page** —
  it read `node.text`, but `commonmark-parser`'s `TextNode`
  uses `node.value`.  Fixed to accept BOTH field names
  defensively.  Index goes from 10 → 345 unique tokens on the
  6-page corpus.  Regression test in `tests/build.test.ts`.
- **CSS selector typo** — the theme styled `header .search
  input` (descendant input of `.search`) but the rendered
  element is `input.search` (input WITH class).  Search input
  was using browser defaults instead of the themed style.
- **Dropdown messed up the header layout** — original
  placement put the dropdown as a sibling of the input inside
  the `display: flex` header, making it a flex item.  Now
  appended to `<body>` at `position: fixed` with coords
  computed from `input.getBoundingClientRect()` — independent
  of any ancestor styling, no layout disruption.
- **Safari cache pinning** — every build now produces a unique
  `?v=<sha256-12chars>` cache-bust on script + JSON URLs.

### Added — UI styling

- `header input.search` properly styled (border, padding,
  focus ring), `min-width: 280px`.

### Tests

74 total (was 56):

- 61 in `tests/build.test.ts` (added 3 for `buildIdFor` + 2
  for `plainText`'s new `value`-field support)
- 13 in `tests/search-bundle.test.ts` (new file)

Coverage: **85.4% line / 92.3% branch / 90.5% function**.
Per-file: `build.ts` 100% / 97.8% / 100%; `plain-text.ts`
100% / 100% / 100%; `search-bundle.ts` 93.1% / 66.7% / 100%
(uncovered lines are the defensive throw paths in
`bundleSearchClient` for esbuild's unexpected-output case);
`main.ts` 65% (CLI bootstrap, integration-tested via
`npm start`).

### Dependencies

- Added `@coding-adventures/forme-doc-search-client-js` (was
  only consumed transitively via emitter types; now imported
  directly for the browser bundle).
- Added `esbuild` as devDep (for the browser bundle step).
- Added `jsdom@22` as devDep (for the end-to-end test).
- BUILD chains the new `forme-doc-search-client-js` install.

## 0.1.0 — 2026-05-22

Initial release.  End-to-end demo of the DOC00 v0 cluster —
the first runnable that exercises all 11 `forme-doc-*` packages
plus the FM00 page-bundle-emitter against a real markdown
corpus, producing a browsable static site.

### Added

- Six-page hand-curated markdown corpus (`corpus/`) describing
  a fictional `@acme/widget` library.
- `src/build.ts` — pure transform that wires the cluster:
  per-page parse → AST decoration → HTML render → page-shell
  wrap; cross-page sidebar + search index build; final
  composition through `emitSite`.  No I/O.
- `src/main.ts` — CLI entrypoint and the only file in the
  program that touches the filesystem.  Reads `corpus/`,
  writes `dist/`.  Refuses symlinks during corpus walk;
  validates output directory; `safeJoin` containment check on
  every write target.
- `src/plain-text.ts` — small AST walker that extracts plain
  text from a `DocumentNode` for search-index input (so the
  index is built over content, not markdown syntax).
- 46 tests covering the entire pipeline + helpers + disk I/O
  + path-escape defences.

### Two small in-program helpers

- **`injectHeadingIds`** — after `document-ast-to-html` renders
  each `<h*>` tag, this walks the HTML and adds the
  heading-anchors result's `id` attribute (positional match;
  in-document order).  Necessary because the upstream renderer
  doesn't read `AnchoredHeadingNode.id`.  ReDoS-free regex
  (`/<h([1-6])>/g`).
- **`normaliseSidebarPaths`** — recursively rewrites each
  sidebar entry's `path` field from a filename
  (`"guide/setup.md"`) to a URL route (`"/guide/setup"`) so
  page-shell's `currentPath` highlight works.

Both are unit-tested and clearly scoped; if they grow they
should be promoted to their own packages.

### Capability declaration

- `fs:read` on `./corpus/**` — read the bundled markdown.
- `fs:list` on `./corpus/**` — walk the corpus directory.
- `fs:write` on `./dist/**` — write the rendered site.
- `fs:create` on `./dist/**` — create output directories.

No net, no proc, no env, no shell.  Capabilities scoped to the
program's own subdirectories.  Per-write `safeJoin`
containment check prevents any path-escape attack.

### Security posture

- **Symlinks refused during corpus walk** — defends against
  traversal-via-symlink to `/etc/passwd` etc.
- **`safeJoin` containment check** — every write target is
  resolved and asserted to live under `outDir`; rejects `..`,
  absolute paths, and prefix-string false matches
  (`outD` vs `outDir`).
- **`validateOutDir`** — rejects empty/non-string/system
  directories (`/`, `/etc`, `/usr`, ...).
- **No regex on user input** — `injectHeadingIds` uses a fixed
  6-char character class with no `+` quantifier; `routeFor`
  uses `String.startsWith`/`endsWith` not regex on the path.
- **HTML-escape defence-in-depth** — `injectHeadingIds`
  escapes anchor IDs even though heading-anchors already
  produces URL-safe slugs.
- **Pure-transform `build()`** — `build.ts` has zero
  side-effects; testable in memory, unaffected by environment.
- **No `eval` / `new Function` / `JSON.parse` with reviver.**

### Build

A 5KB-per-page static site is built in under a second:

```
[forme-doc-demo] read   = 6 markdown files
[forme-doc-demo] wrote  = 18 files to ./dist
```

The 18 files = 6 HTML pages + `sidebar.json` + `search/manifest.json`
+ 10 search shards (one per token-prefix).

### Tests

46 tests in `tests/build.test.ts`:

- `routeFor` (8 cases — every input form)
- `titleOf` (6 cases — frontmatter precedence + fallback)
- `injectHeadingIds` (3 cases — ordering, surplus tags, escaping)
- `normaliseSidebarPaths` (2 cases — file→route, null preserved)
- `build` end-to-end (12 cases against the bundled corpus)
- `writeBundle` (4 cases — actual disk output)
- `safeJoin` (4 cases — including prefix-string false-match)
- `validateOutDir` (5 cases — non-string, empty, system dirs)

### v0 simplifications

- No watch mode (re-run `npm start` after editing).
- No multi-locale.
- No theme system — one inlined CSS stylesheet.
- No image / asset copying.
- No search-client JS embedded (manifest + shards emitted;
  bundling the client is consumer-side per the
  `forme-doc-search-client-js` README).
