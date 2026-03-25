# Changelog — coding-adventures-document-ast (Lua)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-03-24

### Added

- Initial release of the Lua Document AST package.
- Constructor functions for all CommonMark 0.31.2 block nodes:
  `document`, `heading`, `paragraph`, `code_block`, `blockquote`,
  `list`, `list_item`, `thematic_break`, `raw_block`.
- Constructor functions for all CommonMark 0.31.2 inline nodes:
  `text`, `emphasis`, `strong`, `code_span`, `link`, `image`,
  `autolink`, `raw_inline`, `hard_break`, `soft_break`.
- Every constructor validates its inputs and raises an error on misuse
  (e.g. heading level out of 1–6 range, non-boolean `ordered` flag).
- All nodes are plain Lua tables with a `type` field — no metatables,
  no hidden state, no mutation after construction.
- 33-test suite covering every node type and key validation rules;
  all tests pass with busted.
- LuaRocks rockspec (`coding-adventures-document-ast-0.1.0-1.rockspec`).
- BUILD and BUILD_windows files for the monorepo build tool.
