# Changelog — document-html-sanitizer (Go)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.2.0] — 2026-04-02

### Changed
- Wrapped all public functions with the Operations system (`StartNew[T]`):
  `SanitizeHtml`.
- Every public call now has automatic timing, structured logging, and panic
  recovery via the capability-cage Operations infrastructure.
- Public API signatures are unchanged.

## [0.1.0] — 2026-03-24

### Added

- Initial implementation of the `document-html-sanitizer` Go package.
- `SanitizeHtml(html string, policy HtmlSanitizationPolicy) string` — pure
  string-to-string HTML sanitization using regexp-based pattern matching.
  Zero external dependencies.
- `HtmlSanitizationPolicy` struct controlling all sanitization behaviour.
- Three named presets:
  - `HTML_STRICT` — for untrusted HTML from external sources
  - `HTML_RELAXED` — for authenticated users and internal tools
  - `HTML_PASSTHROUGH` — no sanitization (identity transform)
- **Element dropping** — removes dangerous HTML elements and their entire
  content (including nested content), case-insensitively.
  Default dangerous elements: `script`, `style`, `iframe`, `object`, `embed`,
  `applet`, `form`, `input`, `button`, `select`, `textarea`, `noscript`,
  `meta`, `link`, `base`.
- **Attribute stripping** — strips all `on*` event handler attributes (for
  non-passthrough policies) plus explicitly listed attributes (`srcdoc`,
  `formaction` in `HTML_STRICT`).
- **URL sanitization** in `href` and `src` attributes:
  - Strips invisible characters (C0 controls, zero-width Unicode) before
    scheme extraction to defeat bypass attacks.
  - Case-insensitive scheme comparison.
  - Relative URLs always pass through.
- **CSS injection prevention** (`SanitizeStyleAttributes: true`):
  - Drops `style` attributes containing `expression(` (CSS expression injection).
  - Drops `style` attributes containing `url()` with non-http/https arguments
    (Go stdlib has no lookahead, so uses iterative match-and-check approach).
  - Preserves safe `style` attributes containing `url(https://...)`.
- **Comment stripping** (`DropComments: true`) removes `<!-- … -->` including
  multi-line and IE conditional comments.
- Iterative element dropping (up to 20 passes) to handle nested same-name
  elements.
- `isPassthroughPolicy()` helper so that `HTML_PASSTHROUGH` truly passes
  through everything, including `on*` attributes.
- 55 unit tests covering all XSS vectors from the spec:
  - Script element removal (lowercase, uppercase, mixed case)
  - All 14 `on*` event handler variants
  - URL scheme attacks (javascript:, data:, vbscript:, blob:)
  - Null-byte and zero-width-space URL bypass attempts
  - CSS expression() and url(javascript:) injection
  - HTML comments (including IE conditional comments)
  - All three preset smoke tests (STRICT, RELAXED, PASSTHROUGH)
  - Safe content preservation (https/http/mailto/ftp hrefs, relative URLs,
    safe CSS, link titles, img src/alt)
- 89% statement coverage.
- `BUILD` file for the monorepo build tool.
- `README.md` with usage examples, limitations, and security documentation.

### Implementation Notes

- Go's `regexp` package does not support negative lookahead (`(?!...)`).
  The CSS `url()` check uses `FindAllStringSubmatch` to capture each `url()`
  argument and inspect it individually instead of using a lookahead pattern.
- `isPassthroughPolicy()` determines full-passthrough mode by checking:
  `AllowAllUrlSchemes && len(DropElements)==0 && len(DropAttributes)==0 &&
  !SanitizeStyleAttributes && !DropComments`. This allows `HTML_PASSTHROUGH`
  to preserve `on*` attributes that would otherwise be hardcoded-stripped.
- The attribute parser handles double-quoted, single-quoted, and unquoted
  attribute values as well as boolean (valueless) attributes.
