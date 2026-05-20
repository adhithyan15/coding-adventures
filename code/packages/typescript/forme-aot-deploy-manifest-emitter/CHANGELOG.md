# Changelog — @coding-adventures/forme-aot-deploy-manifest-emitter

## 0.1.0 — 2026-05-20

Initial release.  Twentieth FM00 v0 stage package — the
**final compose stage** of the FM00 v0 pipeline.  Reads the
outputs of every other FM00 v0 emitter (page bundle JSON +
sitemap.xml + robots.txt + Web App Manifest JSON + caller-
supplied extra files) and produces a single byte-deterministic
deploy-record JSON.

Pure transform: `DeployManifestConfig` → JSON manifest string.
Two-pass fail-fast.  Uses Node's built-in `node:crypto` for
SHA-256 of synthesised / extra files; page-bundle hashes are
trusted verbatim.

### Added

- `generateDeployManifest(config): string` — main entry.
- `parsePageBundle(json)` — safe JSON parse + shape validation
  for the incoming page bundle.  Uses `JSON.parse` + named-
  field walk (no `for...in`, no prototype traversal).
- `routeToDeployEntry(route)` — page-bundle route → deploy
  file entry (sets `source: "page-bundle"`).
- `validateOutputPath(value, field)` — relative-file-path
  validator with path-traversal defence.
- `validateString(value, field)` — generic string check.
- `sha256Base64(s)` / `utf8ByteLength(s)` — same `node:crypto`
  + `TextEncoder` helpers as the page-bundle emitter.
- Types: `DeployManifestConfig`, `ExtraFile`, `DeployFileEntry`,
  `DeployManifest`.

### Spec adherence

Internal manifest format (versioned at `version: 1`).  No
external spec to adhere to.

### Behavioural notes

- **Output is byte-deterministic.**  Files sorted by
  `outputPath`; top-level + entry keys in fixed order.
- **Page-bundle hashes are trusted verbatim** — we don't
  re-hash; the page-bundle emitter already did that.
- **Synthesised entries** (sitemap / robots / manifest) get
  fresh `sha256Base64` hashes from the input string.
- **Extra-file content** hashed as a UTF-8 string.  Binary
  callers should base64-encode before passing.
- **Synthesised paths are fixed**: `sitemap.xml`, `robots.txt`,
  `manifest.webmanifest`.
- **Duplicate output paths throw** — within page bundle, across
  sources, in extraFiles.  The deploy must be unambiguous.
- **baseUrl propagates** from page bundle into the deploy
  manifest top-level (so deploy targets that need it — sitemap
  hosts, CDN origin configs — have it in one place).
- **No input mutation.**

### Security posture

Ten concerns explicitly addressed (pre-push review found three
hardening gaps — all fixed below):

- **Path traversal on extra files.**  Strict shape regex +
  per-segment `..` / `.` / empty / leading-`/` / leading-`~` /
  backslash / colon rejection.  Rejects every escape vector
  from the canonical OWASP list.
- **Cross-platform path safety.**  No `\` (Windows), no `:`
  (Windows drive separator + HFS+ historic).  No `/`-prefixed
  absolute, no `~`-prefixed home-dir.
- **Percent-encoding traversal defence.**  `%` is not in the
  segment charset.  `%2e%2e` / `%2f` / `%00` are rejected
  wholesale (not decoded), so a recheck-after-decode race
  can't exist.
- **Prototype pollution from parsed JSON.**  `JSON.parse`
  preserves `__proto__` keys as own properties (not on the
  prototype chain).  We walk fields by name via `Object.keys`
  (own enumerable only) — never `for...in`, never dynamic
  property assignment.  An end-to-end test verifies
  `Object.prototype` stays clean after parsing a sneaky
  bundle.
- **JSON-syntax injection.**  `JSON.stringify` is the only
  serialiser used — strings can't break out of the structure.
- **Hashing**: SHA-256 via Node's built-in `node:crypto`.  No
  external dep, no algorithm choice.
- **Determinism**: same input → byte-identical output.  Two
  builds diff cleanly.
- **Page-bundle outputPath revalidation.**  Page-bundle inputs
  are nominally trusted (produced by the upstream emitter),
  but the API accepts an arbitrary JSON string from the
  caller — there's no type-system handle that proves
  provenance.  `parsePageBundle` re-runs `validateOutputPath`
  on every route's `outputPath` so a malicious or buggy
  upstream can't smuggle `../etc/passwd` (or `__proto__`)
  through to the deploy target.
- **Windows reserved device names.**  Per-segment regex
  rejects `CON` / `PRN` / `AUX` / `NUL` / `COM1-9` / `LPT1-9`
  (with or without extension, case-insensitive).  Win32
  intercepts these names and writes to the device instead of
  creating a file — would silently break a cross-platform
  deploy.
- **Per-segment 255-byte filesystem cap.**  ext4 / APFS / NTFS
  all cap single filename components at 255 bytes; longer
  segments fail at deploy time with a cryptic error.  We
  surface it here.
- **Trailing dot / space in segment.**  Win32 silently strips
  these, producing a different file than the caller asked for.
  Rejected.
- **Prototype-pollution sink hardening.**  Output paths
  literally named `__proto__`, `constructor`, or `prototype`
  are rejected at validation.  The `filesObj` accumulator in
  `generateDeployManifest` additionally uses
  `Object.create(null)` so even if validation ever loosened,
  there's no prototype-chain mutation path.

### Capabilities

`[]` — pure transform.  Uses Node's built-in `node:crypto`
(no network, no fs).  No I/O, network, fs, shell, env.

### Tests

112 tests across 3 files:

- `validate.test.ts` (32) — `validateOutputPath` accept
  (simple filename, nested, .well-known/, hyphens/underscores/
  digits, deep nesting) + reject matrix (non-string, null,
  empty, over-cap 2048, leading /, leading ~, contains \, ..
  segment / .. only, . segment / . only, empty mid-segment,
  trailing /, Windows drive `C:/...`, URL-scheme-like
  `https:/...`, colon anywhere, whitespace, ?, #, NUL,
  percent-encoded `%2e%2e`, unicode, error contains field);
  `validateString` (4 cases).
- `parse-page-bundle.test.ts` (25) — accept (valid input,
  baseUrl omitted, empty routes, route with lastmod) + reject
  (invalid JSON, array/null/string root, wrong version,
  missing version, non-string baseUrl, routes is array/null,
  route entry is array, non-string route / outputPath /
  contentType / sha256 / lastmod, non-integer / negative /
  non-string sizeBytes); prototype-pollution defence test
  (after parsing a sneaky `__proto__` key, `Object.prototype`
  remains clean); `routeToDeployEntry` (maps fields + sets
  source; preserves lastmod when present).
- `generate.test.ts` (32) — shape (null config / non-string
  pageBundle / invalid JSON throws); minimal (page bundle
  only, baseUrl propagation, trailing newline); sitemap /
  robots / web-app-manifest synthesised entries + non-string
  inputs rejected; extraFiles (favicon, multiple, non-array
  rejected, null entry, path traversal rejected, absolute
  rejected, backslash rejected, lastmod preserved);
  duplicate detection (duplicate in page bundle, sitemap /
  robots / web-app-manifest path collisions, extra collides
  with page-bundle, extra collides with sitemap); output
  format (files sorted by outputPath, totalSizeBytes is sum,
  entry key order); determinism (byte-identical, no input
  mutation); full real-world example (page bundle + sitemap +
  robots + manifest + 2 extras, 7 total files, sources
  correctly tagged).

Coverage: **100% line / 98.31% branch** across all source
files with logic (`types.ts` is type-only).  The missing
branches are unreachable in practice (sort comparator equality
cases since duplicate paths throw at validation time).

### v0 simplifications (documented)

- **No deploy-target adapters** (S3, Netlify, Cloudflare
  Pages, etc.) — that's the next stage (a deploy-runner
  program).
- **No CDN purge directives** in the manifest.
- **No per-file caching headers / TTLs.**
- **No incremental diff vs. previous deploy** — callers can
  diff two manifest JSONs externally.
