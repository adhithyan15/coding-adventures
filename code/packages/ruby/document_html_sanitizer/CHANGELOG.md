# Changelog

## [0.1.0] — 2026-03-24

### Added

- Initial implementation of `CodingAdventures::DocumentHtmlSanitizer.sanitize_html(html, policy)`.
- `HtmlSanitizationPolicy` as a `Data.define` value object with `with()` support.
- Three named presets: `HTML_STRICT`, `HTML_RELAXED`, `HTML_PASSTHROUGH`.
- Three-pass sanitization architecture:
  - Pass 1: HTML comment stripping (`<!-- ... -->` including multi-line and
    IE conditional comments).
  - Pass 2: Dangerous element removal including all inner content (script,
    style, iframe, object, embed, applet, form, input, button, select,
    textarea, noscript, meta, link, base).
  - Pass 3: Tag-by-tag attribute filtering — strips `on*` event handlers,
    `srcdoc`, `formaction`; sanitizes `href`/`src` URL schemes; strips
    dangerous CSS `expression()` and `url()` patterns from style attributes.
- Custom regex tokeniser (no DOM dependency) for portability across
  environments without a native HTML parser.
- URL scheme sanitization with C0 control character and zero-width character
  stripping (closes `java\x00script:` and `\u200Bjavascript:` bypasses).
- CSS injection prevention: strips `style` attributes containing `expression(`
  or `url()` with non-http/https arguments.
- PASSTHROUGH mode: skips all sanitization when `allowed_url_schemes: nil` and
  no other sanitization is configured.
- 60 unit tests covering all XSS vectors from the TE02 spec.
- 100% test coverage via SimpleCov.
- Passes `standardrb` linting.
- No runtime dependencies — stdlib only.
