# Changelog — @coding-adventures/forme-aot-html-doc-emitter

## 0.1.0 — 2026-05-19

Initial release.  Eighteenth FM00 v0 stage package — final
assembly emitter that wraps pre-built `<head>` + `<body>`
string chunks into a complete `<!doctype html>...</html>`
document.

Pure transform: `HtmlDocConfig` → HTML string.  Two-pass
validate-then-emit.  Head/body are passthrough (trusted upstream
FM00 emitter output); attribute maps and lang/dir get full
validation.

### Added

- `generateHtmlDocument(config): string` — main entry.
- `validateLang(value)` — conservative BCP-47-shaped regex
  (primary alpha subtag + dash-separated alphanumeric
  subsequent subtags).
- `validateDir(value)` — `ltr | rtl | auto` allowlist.
- `validateAttrKey(key, field)` — lowercase ASCII identifier
  + dashes / colons shape check; rejects reserved (`lang`,
  `dir`, `xmlns`) AND any `on*` event-handler key.
- `validateAttrValue(value, field)` — string required, ASCII
  control bytes rejected.
- `escapeHtmlAttr` / `stripAsciiControl` — same single-pass
  character-class pattern as sibling emitters.
- Types: `HtmlDocConfig`, `DocDirection`.

### Spec adherence

Implements HTML Living Standard `<!doctype html>` + `<html>` /
`<head>` / `<body>` element semantics.  `lang` validation is a
conservative subset of BCP-47; `dir` follows the §3.2.6.4
allowlist.  No divergence.

### Behavioural notes

- **Output layout**: 9 newline-separated lines (doctype, html
  open, head open, head content, head close, body open, body
  content, body close, html close).
- **Attribute order** on `<html>`: `lang → dir → extras`
  (extras in `Object.keys` insertion order).
- **Attribute order** on `<body>`: extras in insertion order.
- **head/body NOT escaped** — passthrough.
- **Empty head/body** produce empty lines in the output (still
  valid HTML).
- **Reproducibility.**  Same input → byte-identical output.
  No input mutation.

### Security posture

Six concerns explicitly addressed (pre-push review):

- **HTML attribute injection.**  Every interpolated value
  passes through `escapeHtmlAttr`.
- **Event-handler injection.**  Entire `on*` namespace
  rejected at `validateAttrKey` — no way to ship
  `<body onload="...">` through `bodyAttrs`.  Rejection covers
  every `on*` prefix (not just the known names) so future-spec
  event handlers are pre-empted.
- **Reserved-key shadowing.**  `lang` / `dir` / `xmlns` cannot
  be supplied via `htmlAttrs` / `bodyAttrs` — they'd shadow the
  validator-checked dedicated config fields.
- **Attribute-key injection.**  Key shape regex
  `^[a-z][a-z0-9\-:]{0,63}$` prevents
  `x" onclick="alert(1)`-style escapes.  No whitespace, no
  quote, no `>`, no `=` — all rejected.
- **Prototype pollution.**  `Object.keys()` (own enumerable
  only).  Resolved attrs land in a `Map`, not a plain object.
- **Fail-fast.**  Full validation pass completes before any
  string concatenation.

### Capabilities

`[]` — pure transform.  No I/O, network, fs, shell, env.

### Tests

116 tests across 2 files:

- `validate.test.ts` (75) — `validateLang` accept (en, en-US,
  zh-Hant-HK, pt-BR, de-CH-1996, 3-letter, 8-letter cap) +
  reject (non-string, null, empty, 9-letter over cap,
  digit-leading, trailing dash, leading dash, double dash,
  underscore, space, non-ASCII, XSS attempt, attr-injection
  attempt); `validateDir` (3 accept + case-sensitive +
  empty + non-string); `validateAttrKey` accept (simple,
  class, data-*, aria-*, xml:base, alphanumeric, 64-char cap)
  + reject (non-string, null, empty, uppercase, leading
  digit / dash, space, quote, `>`, `=`, 65-char over cap,
  `__proto__` (starts with `_`), reserved `lang`/`dir`/`xmlns`,
  on* handlers — onload/onclick/onerror/on-thing, error
  contains field path); `validateAttrValue` (4 accept
  including embedded quotes / brackets — they get escaped
  later; reject non-string / null / NUL / tab / newline / DEL
  / error contains field path); `escapeHtmlAttr` (5 entities +
  composite + NUL strip + non-string coercion);
  `stripAsciiControl` (2 cases).  Note: `constructor` is
  explicitly tested as ACCEPTED — it matches the key shape and
  isn't reserved / on*; it's a harmless attribute (browsers
  ignore unknown attrs).  The attack we care about is event
  handlers / lang shadowing, both of which are rejected.
- `generate.test.ts` (41) — shape (null / string / missing
  head / body / non-string head); minimal (empty + populated
  head + body); lang + dir (each alone, both in order, bad
  lang, bad dir); htmlAttrs (data-*, multi-attr insertion
  order, attrs-after-lang-dir, reserved lang/dir/xmlns,
  onload/onmouseover handlers, attr-injection-attempt key,
  array/null/string/number rejected as attrs, non-string
  value, NUL in value, HTML-escapes value); bodyAttrs (class
  attr, multi-attr, onload/onunload rejected, escape
  XSS-attempt value); head/body passthrough (full structure
  preserved, raw HTML in body, multi-line head); purity /
  determinism (byte-identical, no input mutation); fail-fast
  (bad attr key, lang checked first); full real-world example
  with verbatim line check.

Coverage: **100% line / 100% branch** across all source files
with logic (`types.ts` is type-only).

### v0 simplifications (documented)

- **No XHTML / XML self-closing tags.**  HTML Living Standard
  syntax only.
- **No `<head>` / `<body>` content validation** — passthrough.
- **No `<noscript>` fallback shell** — caller adds via `body`.
- **No BCP-47 extensions / private-use subtags** in `lang`
  validation (conservative subset; v1 may extend).
