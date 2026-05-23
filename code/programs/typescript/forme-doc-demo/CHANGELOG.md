# Changelog — forme-doc-demo

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
