# Changelog — coding-adventures-document-html-sanitizer

All notable changes to this package are documented here.

## [0.1.0] — 2026-03-24

### Added

- Initial implementation of `sanitize_html(html, policy) → str`.
- `HtmlSanitizationPolicy` frozen dataclass with full type annotations.
- Named presets: `HTML_STRICT`, `HTML_RELAXED`, `HTML_PASSTHROUGH`.
- `url_utils.py` (independent copy — no shared dep with document-ast-sanitizer):
  - `strip_control_chars(url)` — strips C0 control chars and zero-width Unicode.
  - `is_url_allowed(url, allowed_schemes)` — complete URL safety check.
- `html_sanitizer.py` — regex-based HTML string sanitizer:
  - Step 1: Comment stripping (`<!-- ... -->`) via `re.DOTALL`.
  - Step 2: Element dropping including nested content (script, style, iframe, etc.)
    — uses paired-tag pattern (`<tag>...</tag>`) and self-closing pattern.
  - Step 3: Attribute stripping — all `on*` event handlers always dropped;
    named attributes (`srcdoc`, `formaction`) dropped per policy.
  - Step 4: URL scheme sanitization in `href` and `src` attributes.
  - Step 5: Style attribute CSS injection prevention (`expression()`, `url(non-https)`).
- Default element drop list: script, style, iframe, object, embed, applet, form,
  input, button, select, textarea, noscript, meta, link, base.
- Default attribute drop list: srcdoc, formaction (on* handled separately).
- 112 unit tests covering all XSS vectors from spec + all policy options.
- `BUILD` and `BUILD_windows` files for the monorepo build system.
- `py.typed` marker for PEP 561 type checking support.
- Zero runtime dependencies — standard library only (`re` module).

### Implementation notes

- Implemented in Python 3.12 with full type annotations.
- `HtmlSanitizationPolicy` uses `frozen=True` dataclass for safe sharing.
- Regex patterns compiled at module level for efficiency.
- Known limitation: HTML with `<` / `>` characters inside attribute values
  (e.g. `<div srcdoc="<b>text</b>">`) can confuse the attribute parser.
  For adversarial input with entity-encoded payloads, use the DOM adapter
  or pre-process with an HTML decoder. This is documented in the module docstring.
- Tests achieve 97%+ coverage (target: 95%+).

### Spec compliance

Implements TE02 — Document Sanitization, Stage 2 (HTML Sanitizer).
All transformation rules from the spec are implemented.
