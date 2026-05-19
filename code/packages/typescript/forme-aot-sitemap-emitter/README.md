# @coding-adventures/forme-aot-sitemap-emitter

Emit `sitemap.xml` from a `SitemapEntry[]` + `baseUrl` per
[sitemaps.org/protocol.html](https://www.sitemaps.org/protocol.html).

Pure transform — returns the XML string; caller decides where
to write it (filesystem, CDN upload, in-memory test).
Validation runs BEFORE emission, so callers never see a
partial sitemap.

Eleventh FM00 v0 stage package — joins the FM00 v0 cluster.

## Quick start

```ts
import { generateSitemap } from "@coding-adventures/forme-aot-sitemap-emitter";

const xml = generateSitemap([
  { url: "/",      lastmod: "2026-05-19", changefreq: "daily",   priority: 1.0 },
  { url: "/about", lastmod: "2026-05-15", changefreq: "monthly", priority: 0.8 },
  { url: "https://other.example/x" },  // absolute URL passes through verbatim
], "https://example.com");

fs.writeFileSync("dist/sitemap.xml", xml);
```

Output:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
<url>
  <loc>https://example.com/</loc>
  <lastmod>2026-05-19</lastmod>
  <changefreq>daily</changefreq>
  <priority>1.0</priority>
</url>
<url>
  <loc>https://example.com/about</loc>
  <lastmod>2026-05-15</lastmod>
  <changefreq>monthly</changefreq>
  <priority>0.8</priority>
</url>
<url>
  <loc>https://other.example/x</loc>
</url>
</urlset>
```

## API

### `generateSitemap(entries, baseUrl): string`

Main entry.  Returns the complete XML document as a string.

```ts
interface SitemapEntry {
  readonly url: string;                // http(s):// OR /root-relative
  readonly lastmod?: string;           // ISO-8601 date or datetime
  readonly changefreq?: ChangeFreq;    // allowlisted
  readonly priority?: number;          // clamped to [0.0, 1.0]
}

type ChangeFreq =
  | "always" | "hourly" | "daily" | "weekly"
  | "monthly" | "yearly" | "never";
```

Throws `TypeError` synchronously BEFORE any output is emitted
if:

- `baseUrl` is not `http(s)://`.
- Any entry `url` is not `http(s)://` or root-relative `/path`.
- Any entry `changefreq` is not in the allowlist.

Priority values outside `[0.0, 1.0]` are silently clamped
(they're a hint, and the spec allows the convention).

### Sub-helpers (exposed)

- `escapeXml(s)` / `stripInvalidXml(s)` — same single-pass
  entity-replacement pattern as
  [`forme-feeds`](../forme-feeds).
- `normaliseBaseUrl(baseUrl)` — validates http(s):// and strips
  trailing slash.
- `resolveEntryUrl(url, normalisedBase)` — validates and
  resolves a single entry URL.
- `validateChangefreq(value)` — allowlist check; returns
  lowercased value.
- `clampPriority(value)` — clamps to `[0.0, 1.0]`, formats with
  one decimal.

## URL validation (security-critical)

`entry.url` accept set:

- `http(s)://...` (case-insensitive scheme) — emitted verbatim.
- `/path` (root-relative) — emitted as `baseUrl + path`.
- `/` exactly — emitted as `baseUrl + "/"`.

Reject set (throws `TypeError`):

- `javascript:`, `data:`, `file:`, `vbscript:`, `mailto:`,
  `tel:`, etc.
- `//host/path` (protocol-relative — ambiguous scheme).
- `/\host` (backslash variant — some browsers normalise `\`
  to `/`).
- Bare relative (`about`, `./about`).
- Empty string, non-string.

Error message includes the offending URL (truncated to 200
chars) so debugging is straightforward.

## `changefreq` allowlist

Per [protocol §4](https://www.sitemaps.org/protocol.html#changefreqdef):

`always | hourly | daily | weekly | monthly | yearly | never`

Case-insensitive: `"Daily"` and `"DAILY"` both accepted and
normalised to `"daily"`.  Anything else throws.

## `priority` clamping

Per [protocol §4](https://www.sitemaps.org/protocol.html#prioritydef):
valid range is `[0.0, 1.0]`.  Out-of-range values clamp:

- `5 → 1.0`
- `-1 → 0.0`
- `NaN → 0.5` (the spec's documented default for unspecified)
- `0.75 → 0.8` (rounded to one decimal)

Output always matches `/^[0-1]\.\d$/` for byte-deterministic
diff-friendly sitemaps.

## Behavioural contract

| Aspect                          | Behaviour                              |
|---------------------------------|----------------------------------------|
| Input entries array             | Never mutated                          |
| Entry order                     | Preserved (caller decides sort)        |
| Validation                      | All entries validated BEFORE emit      |
| Bad URL / changefreq            | Throws `TypeError`; no partial output  |
| Bad baseUrl                     | Throws BEFORE any entry validation     |
| Same input + baseUrl            | Byte-identical output                  |
| XML escape                      | All five entities + C0 control strip   |

## Reproducibility (FM03)

Same `entries` + same `baseUrl` → byte-identical XML.  Safe to
feed into cache key derivation.

## Security posture

Five concerns explicitly addressed (pre-push review):

- **URL scheme injection.**  `javascript:`, `data:`, `file:`,
  `vbscript:`, protocol-relative `//host`, `/\backslash-variant`
  all rejected with `TypeError`.  Search-engine crawlers
  blindly follow `<loc>` URLs; the emitter is the line of
  defence.
- **XML attribute / text injection.**  Every interpolated
  value (`loc`, `lastmod`, `changefreq`, `priority`) routes
  through `escapeXml`.  Single-pass character-class
  replacement covers all five XML 1.0 entities (CodeQL-friendly
  pattern — no incomplete-string-escape warning).
- **Control-byte stripping.**  XML 1.0 §2.2 forbids C0
  controls except `\t \n \r`.  `stripInvalidXml` removes
  them before escape so callers can't smuggle NUL / DEL /
  ESC into the document.
- **changefreq allowlist.**  Caller-supplied values match the
  spec-defined set or throw.  Prevents downstream parsers from
  encountering surprise values.
- **Fail-fast (no partial output).**  Validation pass completes
  for every entry BEFORE any XML is emitted.  An exception
  means the caller has nothing to write — no risk of a half-
  formed sitemap reaching disk.

## Capabilities — `[]`

Pure transform.  No I/O, no network, no shell, no env, no fs.

## Tests

112 tests across 4 files:

- `escape.test.ts` (22) — every XML entity individually +
  composite, ampersand-first ordering (no double-escape),
  unicode passthrough, C0 control stripping (each
  individually) + preservation of `\t \n \r`, combined
  control + entity, non-string defensive coercion.
- `url.test.ts` (30) — `normaliseBaseUrl` accept (with/without
  trailing slash, http, https, port, case-insensitive scheme)
  + reject (non-string, null, empty, javascript:, file:,
  protocol-relative, bare relative, long URL truncation in
  message); `resolveEntryUrl` absolute pass-through, root-
  relative join, `/` join, full reject matrix (every forbidden
  scheme, protocol-relative, `/\backslash-variant`, bare
  relative, mailto:, empty, non-string, null, long URL
  truncation).
- `validate.test.ts` (30) — `validateChangefreq` allowlist
  (all seven values parametrised, case-insensitive), reject
  (not-in-allowlist, empty, non-string, null, error message
  contains bad value); `clampPriority` clamps to `[0.0, 1.0]`,
  rounds to single decimal, NaN → `"0.5"`, non-number → `"0.5"`,
  output matches `/^[0-1]\.\d$/`.
- `generate.test.ts` (30) — XML envelope (prelude, urlset
  namespace, empty entries), entry rendering (minimal,
  full, child order loc→lastmod→changefreq→priority, absolute
  pass-through, mixed, trailing-slash baseUrl normalisation),
  URL validation throws before emit (every forbidden scheme,
  protocol-relative, bad baseUrl), changefreq validation
  (reject + allowlist parametrised), priority clamping (verbatim
  / above-1 / negative / NaN), XML escaping (ampersand,
  quotes, control bytes), purity / determinism (no input
  mutation, byte-identical output, preserves input order),
  fail-fast (no partial XML on validation error in mid-array),
  100-entry stress test.

Coverage: **100% line / 100% branch** across all source files
with logic (`types.ts` is type-only).

## Spec adherence

Implements the sitemaps.org protocol (XML schema at
https://www.sitemaps.org/schemas/sitemap/0.9/sitemap.xsd).
No spec divergences.

## v0 simplifications

- **No sitemap index file.**  A single `<urlset>` is emitted;
  sites that exceed the 50,000-URL or 50 MB-per-file protocol
  limits should split externally.  Sitemap-index support is a
  future v1 feature.
- **No image / video / news extensions.**  Core URL set only;
  the protocol's optional namespaces are deferred.
- **No gzip output.**  Caller compresses the returned string
  before writing if desired (`gzip(xml)` in their pipeline).
- **No URL deduplication.**  Caller deduplicates before
  passing entries.  Different `<loc>` values share the same
  `<url>` block in the spec's strict reading, but we emit
  whatever the caller supplies.
- **No locale-aware `lastmod` validation.**  The spec mandates
  W3C Datetime; we pass through whatever string the caller
  supplies (after XML escape).  Validating ISO-8601 format is
  a future v1 feature.
