# Changelog — @coding-adventures/forme-opengraph

## 0.1.0 — 2026-05-18

Initial release.  Second FM00 v0 stage package — OpenGraph + Twitter
Card + basic `<meta>`/`<title>`/`<link>` HTML tag generators.
Companion to `forme-feeds`.

### Added

- `generateOpenGraphTags(meta): string` — emits OpenGraph
  `<meta property="og:...">` tags per https://ogp.me/.
  Conventional tag order: title → type → image → url → description
  → siteName → locale → video.
- `generateTwitterCardTags(meta): string` — emits Twitter Card
  `<meta name="twitter:...">` tags per Twitter Cards spec.
- `generateBasicTags(meta): string` — emits `<title>`,
  `<meta name="description">`, `<link rel="canonical">` for
  search-engine snippet metadata.
- `generateMetaTags({ basic?, og?, twitter? }): string` —
  convenience wrapper combining all three in
  basic → og → twitter order.
- `OpenGraphMeta`, `TwitterCardMeta`, `BasicMeta`, `CombinedMeta`
  type definitions.
- `escapeHtmlAttr`, `escapeHtmlText`, `assertAbsoluteUrl`
  re-exported for callers wiring custom generators.

### Spec adherence

- OpenGraph per https://ogp.me/
- Twitter Cards per https://developer.twitter.com/en/docs/twitter-for-websites/cards/overview/markup
- HTML5 attribute syntax per WHATWG HTML

No spec divergences.

### Behavioural notes

- **URL validation FIRST.**  All URL-bearing fields (og:image,
  og:url, og:video, twitter:image, canonical) are validated against
  `/^https?:\/\//i` BEFORE any output is emitted.  Malformed URLs
  throw `TypeError` synchronously — no partial output.
- **HTML attribute escaping uniform across all generators.**
  Single-pass replace of the five entities (CodeQL-friendly form).
- **Control-byte stripping** (`U+0000-U+001F`, `U+007F`) before
  any escape pass.  Unicode `> U+007F` passes through (HTML5 UTF-8).
- **Tag order is deterministic** within each block.  Block-level
  order in `generateMetaTags` is basic → og → twitter.
- **Empty blocks emit empty strings** (filtered before joining)
  so the combined output never has spurious blank lines.
- **Twitter card type validation.**  Unknown card values throw
  with the allowed set in the error message.

### Security posture

Three concerns explicitly addressed (pre-push review):

- **HTML attribute injection.**  `escapeHtmlAttr` covers all five
  HTML entities in single-pass replacement.  Every interpolated
  string in `opengraph.ts` / `twitter.ts` / `basic.ts` routes
  through it (or `escapeHtmlText`, which is an alias).
- **URL scheme validation.**  `assertAbsoluteUrl` rejects
  `javascript:`, `data:`, `file:`, protocol-relative `//host`,
  relative paths, empty strings, and non-string inputs.  These
  are real injection vectors in social-card scrapers that render
  previews — XSS via `javascript:` in `og:image`, payload delivery
  via `data:`.  Tests pin every rejected form.
- **Control-character stripping** removes ASCII control bytes
  before escaping, so a meta-tag value containing NUL or DEL
  can't confuse downstream HTML parsers.

### Capabilities

`[]` — pure transform.  No I/O, no network, no shell, no env.

### Tests

68 tests across 5 files:

- `escape.test.ts` (19) — every HTML entity (single + composite),
  control-byte stripping for 0x00-0x1F and 0x7F, Unicode passthrough,
  URL scheme acceptance (http, https, port, query, fragment,
  case-insensitive) and rejection (relative, javascript:, data:,
  file:, protocol-relative, empty, non-string), field-name in
  error messages
- `opengraph.test.ts` (16) — required fields in conventional order,
  every optional field, URL validation rejects each forbidden form
  for image/url/video, HTML escaping, control-byte stripping,
  pre-output throw verification, reproducibility
- `twitter.test.ts` (18) — all four card types parameterised,
  unknown card rejection, optional fields with order check,
  emits-only-supplied-tags policy, URL validation for image,
  escaping, reproducibility
- `basic.test.ts` (9) — each tag individually, all-three order,
  empty input → empty string, canonical URL validation, escaping,
  control-byte stripping
- `combined.test.ts` (6) — basic → og → twitter ordering,
  skip-when-not-supplied, URL-error propagation, no spurious
  blank lines, reproducibility

Coverage: **100% line / 100% branch** across all source files
with logic (`types.ts` is type-only declarations).

### v0 simplifications (documented)

- No `og:image:width` / `og:image:height` / `og:image:alt`
  sub-tags.
- No `article:author` / `article:published_time` Open Graph
  Article extensions.
- No `twitter:player` extra metadata.
- No multi-image arrays.
