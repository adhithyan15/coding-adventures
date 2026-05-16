# Changelog — @coding-adventures/forme-parse-markdown

## 0.1.0 — 2026-05-15

Initial release.  Second Forme stage of the blog v0 effort — sits
between `forme-source-fs` and the collector stages.

### Added

- `parseMarkdown` default-exported stage:
  - `consumes: Kinds.ContentSource`
  - `produces: Kinds.ContentNode`
  - `capabilities: []` (pure transform)
  - `configSchema: { gfm?: boolean }`
- Pipeline: UTF-8 decode → BOM strip → `splitFrontmatter` →
  `gfm-parser.parse` → assemble `ContentNode`.
- `splitFrontmatter(source)` — hand-rolled YAML-subset parser.
  Returns `{ data: Record<string,string>, body: string }`.  Total
  function; malformed input degrades to `{ data: {}, body: source }`.
- Identity pass-through; revision recomputed from
  `{ documentJson, frontmatter, sourcePath }`.

### Spec adherence

No deliberate divergences from FM00 / FM01.

### v0 simplifications (documented)

- **Frontmatter grammar is tiny**: opening + closing `---` fences,
  `<key>: <value>` lines, **strings only** (no quoted strings, no
  numbers, no booleans, no arrays, no nested maps).  Hand-rolled — no
  `js-yaml` dependency.  Richer grammars are deferred to a future
  sibling stage.
- **`route` is always `null`** — assignment is the collector's job
  (`forme-collect-chronological`).
- **`assetRefs` is always `[]`** — asset extraction will be a separate
  stage that walks the document AST.
- **`gfm: false` is accepted but ignored.**  `gfm-parser` is currently
  GFM-only; the config flag is reserved for forward compatibility.
- **BOM is stripped** before frontmatter detection.  Without this,
  BOM-prefixed UTF-8 (common from Windows tooling) would never be
  recognised as having frontmatter.

### Notes

- Frontmatter parser intentionally normalises body line endings to LF
  (the source may be CRLF; output is always LF).  The AST is line-ending
  agnostic anyway, so downstream stages don't notice.
- Malformed frontmatter is silently preserved into the body verbatim
  — same behaviour as Jekyll.  The parser then sees `---\n...` at
  the top, which it renders as a thematic break or paragraph depending
  on what follows.  This is by design: a broken header shouldn't
  silently disappear from the rendered output.
