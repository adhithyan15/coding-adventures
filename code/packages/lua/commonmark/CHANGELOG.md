# Changelog — coding-adventures-commonmark (Lua)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-03-24

### Added

- Initial release of the Lua CommonMark 0.31.2 pipeline package.
- `to_html(source, opts)` — parses Markdown and renders to HTML in one call.
- Wires together `coding-adventures-commonmark-parser` and
  `coding-adventures-document-ast-to-html`; exposes their combined behaviour
  via a single, simple API.
- Forwards the `sanitize` option to the HTML renderer.
- 652/652 CommonMark 0.31.2 specification examples pass
  (test file `tests/test_commonmark_spec.lua`).
- LuaRocks rockspec (`coding-adventures-commonmark-0.1.0-1.rockspec`).
- BUILD and BUILD_windows files for the monorepo build tool.
