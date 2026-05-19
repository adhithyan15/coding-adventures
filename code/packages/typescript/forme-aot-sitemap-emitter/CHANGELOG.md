# Changelog — @coding-adventures/forme-aot-sitemap-emitter

## 0.1.0 — 2026-05-19

Initial release.  Eleventh FM00 v0 stage package — sitemap.xml
emitter per https://www.sitemaps.org/protocol.html.

Pure transform: `SitemapEntry[]` + `baseUrl` → XML string.
Validation pass runs BEFORE emission so callers never see a
partial sitemap on error.

### Added

- `generateSitemap(entries, baseUrl): string` — main entry.
  Returns the complete XML document.  Throws `TypeError`
  synchronously if `baseUrl` isn't `http(s)://`, any entry
  `url` isn't `http(s)://` / root-relative, or any entry
  `changefreq` isn't in the allowlist.
- `escapeXml(s)` / `stripInvalidXml(s)` — XML 1.0 entity
  escape + C0 control strip, same single-pass character-class
  pattern as `forme-feeds`.
- `normaliseBaseUrl(baseUrl)` — validates http(s)://, trims
  trailing slash.
- `resolveEntryUrl(url, normalisedBase)` — per-entry URL
  validator + joiner.
- `validateChangefreq(value)` — allowlist check; returns
  lowercased value.
- `clampPriority(value)` — clamps to `[0.0, 1.0]`, formats
  with one decimal place.  NaN → `"0.5"` (spec default).
- `SitemapEntry`, `ChangeFreq` types.

### Spec adherence

Implements the sitemaps.org XML protocol.  No spec
divergences.

### Behavioural notes

- **URL accept set.**  `http(s)://...` (case-insensitive) OR
  `/path` (root-relative, NOT `//host` and NOT `/\host`).
  Everything else throws.
- **`changefreq` allowlist.**  `always | hourly | daily |
  weekly | monthly | yearly | never`.  Case-insensitive
  comparison; output lowercased.
- **`priority` clamping.**  `[0.0, 1.0]`.  Above 1 → 1.0;
  below 0 → 0.0; NaN → 0.5 (spec-documented default for
  unspecified); non-number → 0.5.  Formatted with
  `toFixed(1)` for byte-deterministic output.
- **Validation BEFORE emission.**  All entries validated
  in a single pass; if any throws, no XML reaches the caller.
- **Child order per `<url>`.**  loc → lastmod → changefreq →
  priority.  Matches the spec's documented presentation order.
- **Entry order preserved.**  Caller decides the sort; the
  emitter doesn't re-order.
- **Reproducibility.**  Same input + same baseUrl →
  byte-identical XML.

### Security posture

Five concerns explicitly addressed (pre-push review):

- **URL scheme injection.**  `javascript:`, `data:`, `file:`,
  `vbscript:`, protocol-relative `//host`,
  `/\backslash-variant` all rejected with `TypeError`.
  Crawlers blindly follow `<loc>` URLs; the emitter is the
  line of defence.  Tests pin every forbidden form.
- **XML entity injection.**  Every interpolated value routes
  through `escapeXml`.  Single-pass character-class replace
  covers all five XML 1.0 entities (`& < > " '`) — CodeQL-
  friendly form (no incomplete-string-escape warning).
- **Control-byte stripping.**  XML 1.0 §2.2 forbids C0
  controls except `\t \n \r`.  `stripInvalidXml` removes
  them before escape so callers can't smuggle NUL / DEL /
  ESC through into the document.
- **`changefreq` allowlist.**  Caller-supplied values match
  the spec-defined set or throw.  Defends downstream parsers
  against surprise values.
- **Fail-fast.**  Validation completes for every entry BEFORE
  any XML is emitted.  An exception means the caller has
  nothing to write — no risk of a half-formed sitemap
  reaching disk.

### Capabilities

`[]` — pure transform.  No I/O, no network, no shell, no env,
no fs.

### Tests

112 tests across 4 files:

- `escape.test.ts` (22) — every XML 1.0 entity individually +
  composite, ampersand-first ordering, unicode passthrough,
  C0 control stripping (NUL, backspace, vertical tab, form
  feed, SO, ESC each parameterised), preservation of
  `\t \n \r`, combined control + entity, non-string
  defensive coercion.
- `url.test.ts` (30) — `normaliseBaseUrl` accept (https,
  http, with / without trailing slash, port, case-insensitive
  scheme, preserves path) + reject (non-string, null, empty,
  javascript:, file:, protocol-relative, bare relative, long
  URL truncation in error message); `resolveEntryUrl`
  absolute pass-through (https, http, case-insensitive
  scheme), root-relative join (`/about`, `/`, multi-segment),
  full reject matrix (javascript:, data:, file:, protocol-
  relative, `/\backslash-variant`, bare relative, mailto:,
  empty, non-string, null, long URL truncation).
- `validate.test.ts` (30) — `validateChangefreq` allowlist
  (all seven values individually parametrised, case-
  insensitive), reject (not-in-allowlist, empty, non-string,
  null, error contains bad value); `clampPriority` clamps to
  `[0.0, 1.0]`, single-decimal rounding (0/0.5/0.75/0.25/0.01),
  out-of-range (negative, very negative, above 1, very large,
  ±Infinity), NaN → `"0.5"`, non-number → `"0.5"`, output
  regex `/^[0-1]\.\d$/`.
- `generate.test.ts` (30) — XML envelope (prelude, urlset
  namespace, empty entries), entry rendering (minimal full,
  child order loc→lastmod→changefreq→priority, absolute
  pass-through, mixed absolute + relative, baseUrl trailing
  slash normalisation), URL validation throws before any
  emit (every forbidden scheme, protocol-relative, bad
  baseUrl), changefreq validation (reject, allowlist
  parametrised), priority clamping (verbatim, above-1,
  negative, NaN), XML escaping (ampersand, quotes, control
  bytes), purity / determinism (no input mutation,
  byte-identical output, preserves caller's entry order),
  fail-fast (no partial XML on mid-array validation error),
  100-entry stress test.

Coverage: **100% line / 100% branch** across all source files
with logic (`types.ts` is type-only declarations).

### v0 simplifications (documented)

- **No sitemap index file.**  Single `<urlset>` only; sites
  exceeding 50,000-URL / 50 MB-per-file protocol limits
  split externally.  Sitemap-index support deferred.
- **No image / video / news extensions.**  Core URL set
  only; optional namespaces deferred.
- **No gzip output.**  Caller compresses the returned string
  before writing if desired.
- **No URL deduplication.**  Caller dedupes before passing
  entries.
- **No locale-aware `lastmod` validation.**  Pass through
  verbatim (after XML escape); ISO-8601 format validation
  deferred.
