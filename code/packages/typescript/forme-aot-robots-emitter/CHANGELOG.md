# Changelog — @coding-adventures/forme-aot-robots-emitter

## 0.1.0 — 2026-05-19

Initial release.  Twelfth FM00 v0 stage package — robots.txt
emitter per https://www.robotstxt.org/orig.html + Google /
Yandex extensions (Crawl-delay, Sitemap, Host).

Pure transform: `RobotsConfig` → plain-text string.  Validation
runs in a fail-fast pre-pass BEFORE emission so callers never
see a partial robots.txt.

### Added

- `generateRobots(config): string` — main entry.  Returns the
  complete robots.txt content.  Throws `TypeError`
  synchronously on any validation failure.
- `validateDirectiveValue(value, field)` — guards User-agent /
  Allow / Disallow values against line-format injection.
- `validateCrawlDelay(value)` — non-negative finite integer
  check.
- `validateSitemapUrl(url)` — http(s):// accept-list (same as
  forme-aot-sitemap-emitter).
- `validateHost(host)` — bare hostname check (no scheme, no
  path).
- `RobotsConfig`, `RobotsRule` types.

### Spec adherence

Implements the de-facto robots.txt protocol plus widely-
recognised extensions.  No spec divergences.

### Behavioural notes

- **Rule order preserved.**  Caller decides the order of rule
  blocks; emitter never reorders.
- **Within-rule line order.**  `User-agent:` → `Allow:` →
  `Disallow:` → `Crawl-delay:`.  Allow before Disallow because
  the most-specific-rule-wins convention makes Allow
  exceptions easier to read when listed first.
- **Multiple userAgents per rule.**  Array input emits one
  `User-agent:` line per element; the subsequent Allow /
  Disallow / Crawl-delay lines apply to the union.
- **Sitemap / Host after all rules.**  Convention; defers
  these to the end of the file separated from the last rule
  block by a blank line.
- **Output trailing newline.**  Single `\n` at the end per
  Unix convention.
- **Empty config (`rules: []`, no sitemap, no host)** →
  empty string (no trailing newline).

### Security posture

Five concerns explicitly addressed (pre-push review):

- **Line-format header injection.**  Newlines (LF / CR / CRLF),
  NUL, DEL, and all C0 controls EXCEPT TAB rejected in any
  directive value.  Robots.txt is line-oriented; an unescaped
  newline would split into extra (attacker-controlled)
  directives.  This is the line-format analogue of HTTP
  response splitting.  Pinned by tests for every vector.
- **URL scheme injection on sitemap.**  Same accept-list as
  the sitemap emitter — `javascript:`, `data:`, `file:`,
  `vbscript:`, protocol-relative, root-relative all rejected
  with `TypeError`.
- **Host scheme confusion.**  `host: "https://x.com"`
  rejected; the directive takes a bare hostname per Yandex's
  documentation.  Path / query / fragment / space also
  rejected.
- **Crawl-delay type / range confusion.**  Negative values,
  NaN, ±Infinity, non-numbers, non-integers all rejected with
  `TypeError`.  Zero is permitted (means "no throttling").
- **Fail-fast.**  Validation pass completes for every field
  BEFORE any output is built.  An exception means the caller
  has nothing to write — no risk of a half-formed robots.txt
  reaching disk.

### Capabilities

`[]` — pure transform.  No I/O, no network, no shell, no env,
no fs.

### Tests

93 tests across 2 files:

- `validate.test.ts` (52) — `validateDirectiveValue` accept
  (plain string, path, wildcard, `$` end-of-URL marker,
  %-encoded path, embedded TAB) + reject (LF, CR, CRLF, NUL,
  DEL, ESC, parameterised loop over every C0 except TAB,
  empty, non-string, null, error contains field name);
  `validateCrawlDelay` accept (zero, positive integer, large)
  + reject (negative, NaN, ±Infinity, fractional, non-number);
  `validateSitemapUrl` accept (https, http, case-insensitive,
  port + query) + reject (root-relative, javascript:, file:,
  data:, protocol-relative, empty, non-string, LF injection,
  long-URL truncation in error); `validateHost` accept (bare,
  hostname:port, subdomain, IP) + reject (URL with scheme,
  path, query, fragment, space, LF injection, empty,
  non-string, null).
- `generate.test.ts` (41) — minimal (empty config, single
  allow / disallow); multiple rule blocks (separated by blank
  line, preserved order); userAgent array (one line per
  element, empty array throws, non-string/array throws);
  Allow before Disallow ordering, multiple paths each get
  line; crawlDelay (after disallow, zero permitted, undefined
  omits, negative/fractional/NaN reject); sitemap (single,
  array, after rules, javascript:/root-relative/LF/non-string
  rejects); host (after sitemap, scheme/path reject); header-
  injection defence (userAgent / disallow / allow newline /
  CR / NUL all reject, error identifies bad field); fail-
  fast (bad rule in mid-array, bad sitemap with valid rules);
  input shape validation (null rule, non-array allow/disallow);
  purity / determinism (no input mutation, byte-identical
  output, trailing newline); full real-world example
  (verbatim line check).

Coverage: **100% line / 98.11% branch** across all source
files with logic (`types.ts` is type-only declarations).

### v0 simplifications (documented)

- **No comments.**  `#`-prefixed syntax part of robots.txt but
  rarely used in machine-generated output.  v1 may add an
  optional `comment` field on rules.
- **No `Request-rate` extension.**  Less widely-supported than
  `Crawl-delay`; deferred.
- **No `Clean-param` extension.**  Yandex-specific; deferred.
- **No wildcard validation.**  `*` and `$` accepted as path
  characters but their meaning isn't checked.
