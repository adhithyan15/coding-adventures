# Changelog — @coding-adventures/forme-aot-style-tag-emitter

## 0.1.0 — 2026-05-19

Initial release.  Seventeenth FM00 v0 stage package — HTML
`<link rel="stylesheet">` + inline `<style>` tag emitter with
SRI integrity validation, media-query passthrough, crossorigin
allowlist, and `</style>` injection defence on inline CSS.

Pure transform: `StyleConfig` → HTML string.  Two-pass
validate-then-emit so callers never see partial output.

### Added

- `generateStyleTags(config): string` — main entry.
- `validateStyleHref(url, field)` — http(s)://-or-root-relative
  URL accept-list + control-byte rejection.
- `validateIntegrity(value, field)` — SRI string format
  validator (algo allowlist, base64 charset, per-algo length +
  padding).  Same Map-backed algo lookup pattern as
  `forme-aot-script-tag-emitter`.
- `validateCrossOrigin(value, field)` — `anonymous |
  use-credentials` allowlist.
- `validateInlineCss(css, field)` — non-string + literal
  `</style>` rejection (case-insensitive, matches HTML parser's
  close-tag recognition: `</style` followed by `[\s>/]`).
- `validateOptionalString` — generic optional-string check.
- `escapeHtmlAttr` / `stripAsciiControl` — single-pass HTML
  attribute escape + C0 control strip.
- Types: `StyleConfig`, `StylesheetLink`, `InlineStyle`,
  `CrossOrigin`.

### Spec adherence

Implements HTML Living Standard `<link rel="stylesheet">` +
`<style>` semantics + W3C Subresource Integrity.  No
divergence.

### Behavioural notes

- **Output order**: stylesheets first, then inline.
- **Attribute order** per `<link>`: `rel → href → media →
  integrity → crossorigin → disabled`.
- **`disabled`** is a boolean attribute (bare attribute name).
- **Inline CSS body is NOT escaped** — browsers parse `<style>`
  contents as raw text and escaping would corrupt CSS.
- **`media` query passes through HTML-attr-escaped.**  No
  CSS-syntax gatekeeping (no usable pure-JS parser).
- **Empty config** → empty string.
- **Empty CSS** → empty `<style></style>` block.
- **Reproducibility.**  Same input → byte-identical output.
  No input mutation.

### Security posture

Seven concerns explicitly addressed (pre-push review):

- **URL scheme injection.**  Every `href` runs through
  `validateStyleHref`.  Rejects `javascript:`, `data:`,
  `file:`, `vbscript:`, protocol-relative `//host`,
  backslash-variant `/\host`, bare relative, empty, non-string,
  AND ASCII control bytes anywhere in the URL.
- **HTML attribute injection.**  Every interpolated value
  passes through `escapeHtmlAttr`.
- **SRI integrity format validation** with per-algo length AND
  padding count checks.  Sha256 with `==` would pass total
  length but decode to 31 bytes → browsers silently disable
  SRI → MITM substitution.  Per-algo padding catch is the
  subtle one.
- **Object-prototype walk defence.**  Algo table is a `Map`
  not a plain object — `"__proto__"` / `"toString"` /
  `"hasOwnProperty"` can't return truthy values from
  `Object.prototype`.
- **`</style>` injection in inline CSS.**  This is the
  canonical XSS sink for `<style>` sinks: any literal
  `</style` followed by whitespace, `>`, or `/` would close
  the style block early and let attacker-controlled HTML
  follow.  We reject it with a clear error.  Case-insensitive
  match.  Callers who genuinely need `</style>` in a CSS
  string literal can use the CSS escape `\3C/style>`.
- **Control bytes in href** rejected at validator (not
  silently stripped by escape).
- **Fail-fast.**  Validation completes for every entry BEFORE
  any tag is emitted.

### Capabilities

`[]` — pure transform.  No I/O, network, fs, shell, env.

### Tests

122 tests across 2 files:

- `validate.test.ts` (84) — `validateStyleHref` accept (https,
  http, case-insensitive scheme, root-relative, bare /,
  multi-segment) + reject matrix (javascript:, data:, file:,
  vbscript:, protocol-relative, backslash-variant, bare
  relative, empty, non-string, null, undefined, NUL / tab /
  newline / DEL control bytes, error contains field path,
  long-URL truncation); `validateIntegrity` accept (sha256 /
  sha384 / sha512 single + multi, whitespace collapsing,
  trimming) + reject (non-string, empty, whitespace-only,
  md5 / sha1, no dash, dash at start / end, wrong length per
  algo, invalid base64 chars including URL-safe `_` and triple
  padding, wrong per-algo padding (sha256 `==`, sha384 padded,
  sha512 `=`), `__proto__` / `toString` / `hasOwnProperty`
  algos, second token bad, error contains field path);
  `validateCrossOrigin` (2 accept + reject true / empty /
  non-string); `validateInlineCss` (benign CSS, empty,
  `<`/`>` in selectors, `.style` substring, `</styles>`
  different tag — all pass; literal `</style>`, `</STYLE>`,
  mixed-case, with whitespace, with slash — all rejected;
  non-string + null rejected; field path in error);
  `validateOptionalString` (5 cases); `escapeHtmlAttr` (5
  entities + composite + NUL strip + non-string coercion);
  `stripAsciiControl` (2 cases).
- `generate.test.ts` (38) — empty / null / non-object configs;
  external stylesheets (minimal, media, media query, SRI +
  crossorigin, disabled bool, disabled=false omits,
  attribute-order, multi-sheet order, bad href, bad integrity,
  bad crossorigin, non-bool disabled, non-string media, null
  entry, non-array stylesheets); inline styles (minimal, empty
  CSS, with media, body NOT escaped (verbatim), multi-block
  order, `</style>` rejected, case-variant rejected, non-
  string, null, non-array, media gets escaped); output order
  (stylesheets → inline); fail-fast (bad mid-stylesheet, bad
  late inline); HTML escaping (ampersand in href, quote in
  media); purity / determinism (byte-identical, no input
  mutation); full real-world example with verbatim line check.

Coverage: **100% line / 100% branch** across all source files
with logic (`types.ts` is type-only).

### v0 simplifications (documented)

- **No CSS minification / autoprefixing** — caller
  pre-processes.
- **No CSS-syntax validation** — only the HTML-injection
  vector (`</style>`) is rejected.
- **No `<link rel="preload" as="style">`** — that's a resource
  hint, lives in `forme-aot-meta-link-tags`.
- **No `title` attribute** on `<link>` (deferred).
- **No `blocking="render"`** (low adoption).
