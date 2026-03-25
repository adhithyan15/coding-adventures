# Changelog — @coding-adventures/document-html-sanitizer

## 0.1.0 — 2026-03-24

Initial release.

### Added

- `sanitizeHtml(html, policy)` — pattern-based HTML string sanitizer
- `HtmlSanitizationPolicy` interface with all optional fields
- `HtmlSanitizerDomAdapter` and `DomVisitor` interfaces for DOM-mode path
- Three named presets: `HTML_STRICT`, `HTML_RELAXED`, `HTML_PASSTHROUGH`
- Regex/string-based sanitizer (`html-sanitizer.ts`):
  - Step 1: Comment stripping (default: on)
  - Step 2: Element dropping (removes element + all inner content)
  - Step 3: Attribute sanitization:
    - `on*` event handlers always stripped
    - `srcdoc` and `formaction` always stripped
    - Additional attributes from `dropAttributes` stripped
    - `style` attributes with `expression()` or `url(non-https)` stripped
    - `href` and `src` URL scheme checked against allowlist
- DOM-based sanitizer (`dom-sanitizer.ts`):
  - `DomVisitor` implementation applying the same policy
  - Used when `policy.domAdapter` is supplied
- URL scheme sanitization (independent copy — no `document-ast` dependency):
  - C0 control character stripping
  - Zero-width character stripping
  - Case-insensitive scheme detection
  - `null` allowedUrlSchemes correctly passes all schemes through
- URL utilities exported: `stripControlChars`, `extractScheme`, `isSchemeAllowed`
- 103 unit tests, 100% statement/function/line coverage

### Design Decisions

- No dependency on `@coding-adventures/document-ast` — string in, string out
- `on*` event handler stripping is always active (even with `HTML_PASSTHROUGH`)
  as minimum defense-in-depth
- `allowedUrlSchemes: null` (explicitly set) allows all schemes;
  `allowedUrlSchemes: undefined` (omitted) uses default safe list
- CSS injection: full `style` attribute dropped rather than attempting partial CSS parse
