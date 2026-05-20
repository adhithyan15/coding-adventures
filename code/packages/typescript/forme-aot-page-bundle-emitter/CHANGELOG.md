# Changelog — @coding-adventures/forme-aot-page-bundle-emitter

## 0.1.0 — 2026-05-19

Initial release.  Nineteenth FM00 v0 stage package — deploy-
stage page bundle emitter.  Pure transform:
`PageBundleConfig` → deterministic JSON manifest string.

The manifest is the deploy hand-off format: route → file path
+ content type + size + SHA-256 hash + optional last-modified.
A downstream deploy tool consumes the manifest and writes the
actual files.

### Added

- `generatePageBundle(config): string` — main entry.  Returns
  the manifest as a 2-space-indented JSON string with trailing
  newline.
- `validateRoute(value, field)` — strict route validation with
  path-traversal defence.
- `validateBaseUrl(value)` — http(s):// accept-list.
- `validateString(value, field)` — generic string check (for
  html / lastmod / contentType).
- `routeToOutputPath(route)` — deterministic route → output
  file path derivation.  `/` → `index.html`; `/about` →
  `about/index.html`; `/feed.xml` → `feed.xml` (extension
  preserved); `/file.tar.gz` → `file.tar.gz`; `/.hidden` →
  `.hidden/index.html` (dotfile doesn't count as extension).
- `sha256Base64(s)` — SHA-256 via Node's built-in `node:crypto`,
  base64-encoded.  Standard alphabet.
- `utf8ByteLength(s)` — UTF-8 byte count via `TextEncoder`.
- Types: `PageBundleConfig`, `PageEntry`, `RouteEntry`,
  `PageBundleManifest`.

### Spec adherence

Internal manifest format (versioned at `version: 1`).  No
external spec to adhere to.

### Behavioural notes

- **Output is byte-deterministic.**  Same input → byte-identical
  manifest string.  Reordering input pages doesn't change
  output (routes sorted alphabetically in the manifest).
- **Top-level key order**: `version → baseUrl (if present) →
  routes`.
- **Route entry key order**: `route → outputPath → contentType
  → sizeBytes → sha256 → lastmod (if present)`.
- **HTML body is passthrough** — hashed and measured but never
  escaped/validated.  Trusted upstream output.
- **Duplicate routes** (after validation) throw.
- **Empty `pages: []`** → minimal manifest (`{ version: 1,
  routes: {} }`).
- **No input mutation.**
- **Default contentType** = `text/html; charset=utf-8`.
- **`sizeBytes`** is UTF-8 byte count, not char count.

### Security posture

Six concerns explicitly addressed (pre-push review):

- **Path traversal.**  Routes validated against strict shape
  regex + segment-by-segment `..` / `.` / empty-segment
  checks before becoming file paths.  No `..\..\etc` can
  escape the deploy root.
- **Protocol confusion.**  Routes starting with `//` (treated
  as URL by some servers / CDNs) or `/\` (backslash variant)
  rejected.
- **Windows path separators.**  Any `\` in the route rejected
  — prevents `..\..\etc` on Windows-target deploys.
- **baseUrl scheme injection.**  `javascript:` / `data:` /
  `file:` / `ftp:` / scheme-less / non-http(s) rejected.
- **Hashing**: SHA-256 via Node's built-in `node:crypto`.  No
  network, no third-party dep, no weak algorithm choice.
- **Determinism**: same input → byte-identical output.  Two
  builds diff cleanly; no spurious churn from
  non-deterministic ordering.

### Capabilities

`[]` — pure transform.  Uses Node's built-in `node:crypto`
(no network, no fs).  No I/O, network, fs, shell, env.

### Tests

101 tests across 4 files:

- `validate.test.ts` (51) — `validateRoute` accept (bare /,
  /about, /posts/x, /p/x.html, /feed.xml, dashes/underscores,
  digits, mixed-case preserved, colon segment) + reject
  matrix (non-string, null, empty, over-cap 2048, no leading
  /, protocol-relative //, backslash variant /\, mid-path \,
  /.., /a/.., /a/../b, /., /a/./b, /a//b, trailing /a/, query
  /a?b=c, hash /a#b, whitespace, NUL, unicode, error contains
  field, long-route truncated); `validateBaseUrl` accept
  (https, http, HTTPS uppercase, with path) + reject
  (non-string, null, empty, javascript:, data:, file:, ftp:,
  /relative, over-cap); `validateString` (4 cases).
- `path.test.ts` (10) — `routeToOutputPath` all 10 documented
  cases (/, /about, /posts/x, /page.html, /feed.xml,
  /posts/x.html, /p/x.json, deep no-ext, dotfile, compound
  extension).
- `hash.test.ts` (10) — `sha256Base64` empty + known "abc"
  digest + determinism + different inputs differ + 44-char
  length; `utf8ByteLength` ASCII + empty + multi-byte é +
  emoji + mixed.
- `generate.test.ts` (30) — shape (null config / non-array
  pages / empty pages → minimal); single page (minimal /,
  custom contentType, lastmod present-only-when-provided);
  multiple pages (routes sorted alphabetically regardless
  of input order, duplicate throws); baseUrl (included,
  omitted, javascript: throws); route validation (traversal,
  protocol-relative, backslash, absolute https throws);
  page field validation (non-string html / contentType /
  lastmod / null page); output format (2-space indent,
  trailing newline, top-level key order, entry key order);
  determinism (byte-identical, reordering invariant, no
  input mutation); content hashing (different html differs,
  same html identical, UTF-8 byte count); full real-world
  example with 4 pages + baseUrl + feed.

Coverage: **100% line / 98.82% branch** across all source
files with logic (`types.ts` is type-only).  The single
missing branch is the sort-comparator equality case
(`a.route === b.route` → returns 0); duplicate routes throw at
validation time so the equality branch is unreachable in
practice.

### v0 simplifications (documented)

- **No gzip / brotli precompression metadata** — caller
  compresses at deploy time.
- **No per-page redirect / rewrite rules** — 1:1 route to
  file.
- **No multi-locale routing logic** — caller pre-flattens.
- **No content-addressed output paths** (e.g.
  `page.abc123.html`) — v1 may add as option.
