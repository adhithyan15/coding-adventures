# Changelog — @coding-adventures/forme-aot-rss-discovery-link

## 0.1.0 — 2026-05-19

Initial release.  Fourteenth FM00 v0 stage package — emits HTML
`<link rel="alternate" type="application/rss+xml">` tags for
feed auto-discovery (de-facto convention browsers / feed
readers have used since the early 2000s).

Pure transform: `FeedDiscoveryLink | FeedDiscoveryLink[]` →
HTML string.  Validation runs in a fail-fast pre-pass BEFORE
emission so callers never see partial output.

### Added

- `generateFeedDiscoveryLinks(input): string` — main entry.
  Accepts a single object or array.  Returns one tag per
  line; no trailing newline.
- `validateFeedHref(url)` — URL accept-list (http(s):// OR
  root-relative).
- `validateFeedType(value)` — feed MIME allowlist (rss+xml,
  atom+xml, json).  Case-sensitive.
- `escapeHtmlAttr` / `stripAsciiControl` — single-pass HTML
  attribute escape + C0 control strip, same pattern as
  `forme-feeds` / `forme-opengraph` / `forme-index-renderer`.
- `FeedDiscoveryLink`, `FeedType` types.

### Spec adherence

Implements the de-facto feed auto-discovery convention.  No
formal spec divergences.

### Behavioural notes

- **Attribute order**: `rel="alternate"` → `type` → `title` →
  `href`.  Matches WordPress, Hugo, Jekyll conventions.
- **Default `type`**: `"application/rss+xml"` when omitted.
- **`title` is optional**: when omitted, no `title` attribute
  is emitted.
- **Empty array input** → empty string output.
- **Single object** treated as a one-element array (same
  output as `[{...}]`).

### Security posture

Three concerns explicitly addressed (pre-push review):

- **HTML attribute injection.**  Every interpolated value
  passes through `escapeHtmlAttr`.  Single-pass character-
  class replacement covers all five HTML entities + strips
  ASCII control bytes.  Attacker-controlled `title` like
  `<script>alert(1)</script>` becomes inert text.
- **URL scheme injection.**  Browsers follow `<link
  rel="alternate" href>` URLs when feed readers auto-discover.
  `javascript:`, `data:`, `file:`, `vbscript:`, protocol-
  relative, backslash-variant, bare relative all rejected
  with `TypeError`.
- **Type allowlist.**  Restricts to the three mainstream feed
  formats; prevents callers from injecting arbitrary MIME
  strings that downstream feed-reader implementations might
  parse permissively.

### Capabilities

`[]` — pure transform.  No I/O, network, fs, shell, env.

### Tests

64 tests across 2 files:

- `validate.test.ts` (33) — `validateFeedHref` full accept
  (https, http, case-insensitive, root-relative, bare /) and
  reject matrix (javascript:, data:, file:, protocol-relative,
  backslash-variant, bare relative, empty, non-string, null,
  long-URL truncation in error); `validateFeedType` allowlist
  (rss+xml, atom+xml, json) and reject (rdf+xml deprecated,
  text/xml, case-sensitive uppercase, empty, non-string);
  `escapeHtmlAttr` all five HTML entities + composite + control
  byte stripping (NUL, DEL, ESC) + defensive non-string
  coercion.
- `generate.test.ts` (31) — single link (minimal / full / each
  type / absolute href); array (multiple feeds, order
  preserved, single-element same as object, empty);
  attribute order (`rel → type → title → href`, no title);
  HTML escaping (ampersand in href, quotes in title, XSS
  attempt with script tag, control bytes in title); URL
  validation (each forbidden scheme); type validation
  (rdf+xml, text/xml, case-sensitive); input shape (null
  link, non-string title, error identifies bad index);
  fail-fast (mid-array bad type); purity / determinism / no
  input mutation; full real-world example with verbatim line
  check.

Coverage: **100% line / 100% branch** across all source files
with logic (`types.ts` is type-only).

### v0 simplifications (documented)

- **No `media="..."`** attribute (rarely used for feed
  discovery; deferred).
- **No `application/feed+json`** MIME alias (v1 may add).
- **No comment-attribute support** — single-line tags only.
