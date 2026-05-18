# Changelog — @coding-adventures/forme-aot-page-emitter

## 0.1.0 — 2026-05-17

Initial release.  Fourth FM06 AOT compiler family package.  Turns
in-memory per-page CSS artefacts into on-disk files a static-site
host can serve.

### Added

- `emitPages(distDir, artefacts, options?, io?)` — writes per-page
  CSS (and optional HTML wrapper) for each entry in
  `Map<pageId, CssArtifact>`.  Route → file path mapping handles
  `/`, trailing-slash directories, missing `.html` extension, and
  nested paths.  Sub-dirs created on demand (`mkdir recursive`).
- `EmitOptions` — `writeHtml?: boolean` (default `false`) +
  `htmlBody?: (pageId: string) => string` (default returns empty).
- `EmitIO` — injectable side-effect surface
  (`mkdir` + `writeFile`).  Defaults to `node:fs/promises`.
- `EmitResult { written, totalBytes }` — per-page
  `{ cssPath, htmlPath?, byteSize }` plus total bytes written.

### Spec adherence

Implements FM06 §5 (per-page artefact emission).  No spec
divergences.

### Behavioural notes

- **Route → file mapping.**  pageId interpreted as a URL-shaped
  route; leading `/` stripped, trailing `/` → `index.html`,
  missing `.html` appended.  CSS file sits next to HTML with
  `.css` extension.
- **HTML wrapper is opt-in via `writeHtml: true`.**  Default
  emits CSS only — consumers that already have an HTML pipeline
  (e.g. a static site generator) don't pay for unwanted output.
- **`htmlBody` callback is caller-trusted** — we do NOT sanitise
  the body content (legitimate use cases need literal `<` / `&` in
  markup).  Caller owns HTML escaping per standard XSS prevention.
- **`totalBytes` accounts for both CSS and HTML** when both are
  written.
- **Page iteration order preserved.**  The returned `written` Map
  iterates in caller's input order.
- **Pre-existing files are overwritten** without warning (build-
  artefact convention).

### Security posture

Pre-push focused review:

- **Path traversal via pageId.**  `assertValidPageId` rejects
  empty / `..` / `.` segments, NUL bytes, ASCII control chars,
  backslashes (Windows ambiguity), and absolute-path forms
  (`//...`, `\\...`, `<letter>:...`).  Tests pin 9 distinct
  rejection cases.
- **Defence in depth.**  After `path.join`, the resolved CSS path
  is verified to still live under `path.resolve(distDir)` —
  catches anything `assertValidPageId` might have missed.
- **HTML attribute escaping.**  The CSS href in the generated
  `<link>` tag is escaped (`&` → `&amp;`, etc.) — so a pageId like
  `r&d` lands as `href="r&amp;d.css"`.  One test pins.
- **`htmlBody` is documented caller-trusted.**  Body content lands
  raw in the wrapper; caller owns escaping.
- **No `child_process`, no `eval`, no `require()` of user paths,
  no symlink resolution.**

### Capabilities

`["fs"]` — writes per-page CSS + HTML under `distDir`.  Reads
nothing.  The fs IO is also injectable via `EmitIO`.

### Tests

32 tests in `page-emitter.test.ts`:

- Basic CSS write (4)
- Route → file path mapping (7 parameterised)
- HTML wrapper (5)
- pageId validation (9)
- Pre-existing files overwritten (1)
- IO injection (2)
- Page iteration order preserved (1)
- HTML attribute escaping (1)
- 50-page stress (1)
- pageId without leading slash (1)

Coverage: **98.07% line / 93.02% branch** — above the FM04 §14.4
≥95% line target.  Uncovered: the defensive "resolved outside
distDir" throw (unreachable given pageId validation) and the
production default-IO callbacks (tests inject in-memory IO).

### v0 simplifications (documented)

- Minimal HTML wrapper — just doctype, charset, CSS link, body
  callback output.  Richer HTML (title, OpenGraph, hreflang) is
  the consumer's job via `htmlBody`.
- Single CSS link per page.
- No cache-busting hash suffixes in filenames.
