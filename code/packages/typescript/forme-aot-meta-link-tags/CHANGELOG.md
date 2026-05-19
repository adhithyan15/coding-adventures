# Changelog — @coding-adventures/forme-aot-meta-link-tags

## 0.1.0 — 2026-05-19

Initial release.  Fifteenth FM00 v0 stage package — generic
`<meta>` + `<link>` head-tag emitter.  Pure transform:
`MetaLinkConfig` → HTML string.  Validation runs in a fail-fast
pre-pass BEFORE emission so callers never see partial output.

### Added

- `generateMetaLinkTags(config): string` — main entry.  Accepts
  the structured `MetaLinkConfig` shape; returns the concatenated
  tag string (one tag per line, no trailing newline).
- `validateUrl(url, field)` — http(s)://-or-root-relative
  accept-list; `field` parameter threads through error messages
  so callers can pinpoint which entry failed.
- `validateIconRel`, `validateHintRel`, `validateHintAs`,
  `validateCrossOrigin` — allowlist checks for the four
  attribute-allowlisted fields.
- `validateOptionalString` — generic optional-string check
  (returns `undefined` for `undefined`, throws for any other
  non-string).
- `escapeHtmlAttr` / `stripAsciiControl` — same single-pass
  character-class HTML-entity escape + control-byte strip as the
  sibling FM00 emitters.
- Types: `MetaLinkConfig`, `IconLink`, `IconRel`, `ResourceHint`,
  `ResourceHintRel`, `ResourceHintAs`, `CrossOrigin`, `MetaTag`.

### Spec adherence

Implements the de-facto HTML head-tag conventions per the HTML
Living Standard + Resource Hints + Preload specs.  No formal
divergence from upstream specs.

### Behavioural notes

- **Output order**: `meta → canonical → prev → next → icons →
  hints`.  Fixed across runs.
- **Attribute order** is fixed per tag (see source).
- **`<meta>` shape**: exactly one of `name` / `httpEquiv` is
  required.  Empty values throw.
- **`as` is required for `preload` and `modulepreload`**;
  silently dropped on other hint rels (where it's invalid HTML).
- **Empty config** → empty string output.
- **Same input → byte-identical output.**

### Security posture

Five concerns explicitly addressed (pre-push review):

- **HTML attribute injection.**  Every interpolated value
  passes through `escapeHtmlAttr`.  Single-pass character-
  class replacement covers all five HTML entities + strips
  ASCII control bytes.  Attacker-controlled `content` like
  `<script>alert(1)</script>` becomes inert text.
- **URL scheme injection.**  Every `href` field (canonical /
  prev / next / icons / hints) runs through `validateUrl`.
  `javascript:`, `data:`, `file:`, `vbscript:`,
  protocol-relative `//`, backslash-variant `/\`, bare relative,
  empty, non-string all rejected with `TypeError`.  Crawlers and
  link prefetchers follow these URLs aggressively — emitter is
  the last line of defence.
- **rel allowlist (icons + hints).**  Different categories
  restrict to the spec-defined `rel` set for that category
  (icons: 4 values; hints: 5 values).  Prevents caller from
  injecting arbitrary rel values that downstream parsers might
  treat permissively.
- **as allowlist.**  Defends against `as="iframe"`-style fetch-
  destination confusion attacks.
- **Fail-fast.**  Validation completes for every field BEFORE
  any tag is emitted.  An exception means the caller has nothing
  to write — no risk of a half-formed `<head>` reaching disk.

### Capabilities

`[]` — pure transform.  No I/O, network, fs, shell, env.

### Tests

132 tests across 2 files:

- `validate.test.ts` (73) — `validateUrl` full accept (https,
  http, scheme case-insensitive, root-relative, bare /,
  multi-segment) and reject matrix (javascript:, data:, file:,
  vbscript:, protocol-relative, backslash-variant, bare
  relative, empty, non-string, null, undefined, long URL
  truncation, error field-path threading); `validateIconRel`
  (4 accept + case-sensitive reject + non-string + null);
  `validateHintRel` (all 5 values parametrised + reject +
  case-sensitive + non-string); `validateHintAs` (all 10 values
  parametrised + reject + non-string); `validateCrossOrigin`
  (2 accept + reject true / empty / non-string);
  `validateOptionalString` (undefined / string / empty / non-
  string / null / error path); `escapeHtmlAttr` (5 HTML entities
  individually + composite + ampersand-first + control bytes +
  non-string coercion); `stripAsciiControl` (control byte
  matrix + printable preservation + tab/lf/cr handling).
- `generate.test.ts` (59) — empty / null / non-object config;
  canonical / prev / next individually and in fixed order;
  multiple meta tags with name + http-equiv; meta validation
  (both / neither name+httpEquiv, empty values, null entry,
  non-string content / name / httpEquiv, non-array meta, XSS in
  content + name); icons (default rel, type + sizes, apple-
  touch-icon variant, multi-icon order, bad rel, javascript:
  href, null entry, non-array, escape in sizes); resource hints
  (preload requires as, modulepreload requires as, preload
  script, preload font with type + crossorigin, preconnect (no
  as), preconnect with as gets dropped, dns-prefetch, prefetch,
  modulepreload, preconnect with use-credentials, bad rel, bad
  as, bad crossorigin, javascript: href, null entry, non-array,
  non-string type); output order (meta → canonical → prev →
  next → icons → hints); fail-fast (bad icon URL mid-array,
  bad meta before canonical); HTML escaping (ampersand, quotes,
  control bytes); purity / determinism (byte-identical, no
  input mutation); full real-world example with verbatim line
  check.

Coverage: **100% line / 100% branch** across all source files
with logic (`types.ts` is type-only).

### v0 simplifications (documented)

- **No `media="..."`** attribute (deferred).
- **No `integrity`** / SRI (lives in `forme-aot-script-tag-emitter`).
- **No `referrerpolicy`** attribute (v1 may add).
- **No HTTP Link header emission** — HTML only.
- **No `charset` support** — emit `<meta charset="utf-8">` via a
  separate head builder.
