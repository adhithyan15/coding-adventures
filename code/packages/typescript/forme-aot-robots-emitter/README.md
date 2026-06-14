# @coding-adventures/forme-aot-robots-emitter

Emit `robots.txt` from a structured `RobotsConfig` per
[robotstxt.org/orig.html](https://www.robotstxt.org/orig.html)
+ widely-recognised Google / Yandex extensions.

Pure transform — returns the plain-text string; caller writes
it wherever.  Validation runs BEFORE emission, so callers never
see a partial robots.txt.

Twelfth FM00 v0 stage package — joins the FM00 v0 cluster.

## Quick start

```ts
import { generateRobots } from "@coding-adventures/forme-aot-robots-emitter";

const txt = generateRobots({
  rules: [
    { userAgent: "*",         disallow: ["/admin", "/private"], crawlDelay: 10 },
    { userAgent: "Googlebot", allow: ["/"]                                     },
    { userAgent: ["Bingbot", "Slurp"], crawlDelay: 5 },
  ],
  sitemap: "https://example.com/sitemap.xml",
  host: "example.com",
});

fs.writeFileSync("dist/robots.txt", txt);
```

Output:

```
User-agent: *
Disallow: /admin
Disallow: /private
Crawl-delay: 10

User-agent: Googlebot
Allow: /

User-agent: Bingbot
User-agent: Slurp
Crawl-delay: 5

Sitemap: https://example.com/sitemap.xml
Host: example.com
```

## API

### `generateRobots(config): string`

Main entry.  Returns the complete robots.txt content as a string.

```ts
interface RobotsConfig {
  readonly rules: readonly RobotsRule[];
  readonly sitemap?: string | readonly string[];
  readonly host?: string;
}

interface RobotsRule {
  readonly userAgent: string | readonly string[];
  readonly allow?: readonly string[];
  readonly disallow?: readonly string[];
  readonly crawlDelay?: number;
}
```

Throws `TypeError` synchronously BEFORE any output if:

- Any directive value contains CR / LF / NUL / DEL / other C0
  control byte (line-format injection risk — see Security).
- `crawlDelay` isn't a non-negative finite integer.
- `sitemap` URL isn't `http(s)://`.
- `host` looks like a URL (scheme / path / query / fragment).
- Required fields missing or wrong type.

### Sub-helpers (exposed)

- `validateDirectiveValue(value, field)` — guards a User-agent /
  Allow / Disallow value against injection.
- `validateCrawlDelay(value)` — non-negative finite integer.
- `validateSitemapUrl(url)` — http(s):// accept-list.
- `validateHost(host)` — bare hostname.

## Header-injection defence

The robots.txt protocol is line-oriented: any unescaped newline
or CR in a value would split into extra (possibly attacker-
controlled) directive lines:

```
User-agent: Goodbot
Disallow: /admin    ← intended

User-agent: Goodbot
Disallow: /admin
User-agent: *       ← injected by hostile data!
Allow: /admin       ← injected by hostile data!
```

This is the line-format analogue of HTTP response splitting.
We **reject** (not strip) such inputs so the caller knows about
the bad data rather than silently shipping a half-formed
directive.

Forbidden characters in any directive value:

- CR (`\x0D`), LF (`\x0A`) — the injection vectors themselves
- NUL (`\x00`), DEL (`\x7F`) — parser confusion vectors
- Other C0 controls (`\x01-\x1F`) EXCEPT TAB (`\x09`) — kept for
  the rare case of multi-word user-agent strings

## URL validation (sitemap)

Sitemap URLs use the same accept-list as
[`forme-aot-sitemap-emitter`](../forme-aot-sitemap-emitter):

- Accept: `http(s)://...` (case-insensitive scheme).
- Reject: `javascript:`, `data:`, `file:`, `vbscript:`,
  protocol-relative `//host`, root-relative `/path` (absolute
  required for `Sitemap:` directive per the spec), empty,
  non-string.

## Host validation

The `Host:` directive (Yandex extension; widely-ignored but
harmless) takes a bare hostname.  We reject:

- URLs with scheme (`https://example.com`)
- URLs with path / query / fragment
- Strings containing spaces
- Strings with injection characters

## Behavioural contract

| Aspect                          | Behaviour                              |
|---------------------------------|----------------------------------------|
| Input config                    | Never mutated                          |
| Rule order                      | Preserved (caller decides order)       |
| Per-rule line order             | UA → Allow → Disallow → Crawl-delay    |
| Multiple UA per rule            | One `User-agent:` line per element     |
| Validation                      | All fields validated BEFORE emit       |
| Bad value                       | Throws `TypeError`; no partial output  |
| Same input                      | Byte-identical output                  |
| Empty config (`rules: []`)      | Empty string output                    |
| Output trailing newline         | Yes (single `\n`)                      |

## Reproducibility (FM03)

Same `config` → byte-identical robots.txt.

## Security posture

Five concerns explicitly addressed (pre-push review):

- **Line-format header injection.**  Newlines / CRs / NULs /
  DELs / other C0 in any directive value rejected via
  `validateDirectiveValue`.  Defends against an attacker
  passing `userAgent: "Goodbot\nUser-agent: Evilbot"` to
  inject sibling rules.
- **URL scheme injection on sitemap.**  Same accept-list as
  the sitemap emitter — `javascript:` / `data:` / etc.
  rejected with `TypeError`.
- **Host scheme confusion.**  `host: "https://x.com"` rejected
  before emission so attackers can't piggy-back a full URL.
- **Crawl-delay range / type confusion.**  Negative values,
  NaN, Infinity, non-numbers, non-integers all rejected.
- **Fail-fast.**  Validation pass completes for every field
  BEFORE any output is built.  An exception means the caller
  has nothing to write — no risk of a half-formed robots.txt
  reaching disk.

## Capabilities — `[]`

Pure transform.  No I/O, no network, no shell, no env, no fs.

## Tests

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
  long-URL truncation); `validateHost` accept (bare,
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
  array, after rules, javascript: reject, root-relative
  reject, LF reject, non-string reject); host (after sitemap,
  scheme reject, path reject); header-injection defence
  (userAgent / disallow / allow with newline/CR/NUL all
  reject, error identifies bad field); fail-fast (bad rule in
  mid-array, bad sitemap with valid rules); input shape
  (null rule, non-array allow/disallow); purity / determinism
  (no input mutation, byte-identical output, trailing
  newline); full real-world example (verbatim line check).

Coverage: **100% line / 98.11% branch** across all source
files with logic (`types.ts` is type-only).

## Spec adherence

Implements robotstxt.org protocol (`User-agent`, `Allow`,
`Disallow`) + Google / Bing / Yandex extensions
(`Crawl-delay`, `Sitemap`, `Host`).  No spec divergences.

## v0 simplifications

- **No comments.**  The `#`-prefixed comment syntax is part of
  robots.txt but rarely used in machine-generated output.  v1
  may add an optional `comment` field on rules.
- **No `Request-rate` extension.**  Less widely-supported than
  `Crawl-delay`; deferred.
- **No `Clean-param` extension.**  Yandex-specific; deferred.
- **No wildcard validation.**  `*` and `$` are accepted as path
  characters but their meaning isn't checked — the spec
  permits arbitrary text.
