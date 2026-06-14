# Changelog — format-doc

## [0.1.0] — 2026-04-30

Initial release.  Rust port of P2D03 — the TypeScript
`@coding-adventures/format-doc` document algebra.  The semantic
IR every formatter builds: language-specific formatters compile
AST → `Doc`, this crate realises `Doc` → `DocLayoutTree`.

### Added

- `Doc` enum (`#[non_exhaustive]`, 8 variants): `Nil`, `Text`,
  `Concat`, `Group`, `Indent`, `Line(LineMode)`, `IfBreak`,
  `Annotate`.  Internal `Arc` sharing in compound variants —
  cheap to clone.
- `LineMode` (`Soft` / `Normal` / `Hard`) — three flavours of
  conditional line break.
- `DocAnnotation` (`#[non_exhaustive]`, `Str` / `Int` / `Bool`
  / `Null`) — open-ended metadata for spans.
- Builders: `nil()`, `text(s)`, `concat(parts)`, `join(sep, parts)`,
  `group(d)`, `indent(d, levels)`, `line()`, `softline()`,
  `hardline()`, `if_break(broken, flat)`, `annotate(ann, d)`.
- `LayoutOptions { print_width, indent_width, line_height }` (Default:
  80 / 2 / 1).  `print_width` must be `> 0` (asserted).
- `DocLayoutSpan { column, text, annotations }`,
  `DocLayoutLine { row, indent_columns, width, spans }`,
  `DocLayoutTree { print_width, indent_width, line_height,
  width, height, lines }`.
- `layout_doc(doc, options) -> DocLayoutTree` — the realisation
  interpreter.  Iterative dispatch over an explicit Command stack;
  groups make flat-vs-broken decisions via look-ahead `fits()`.
- `render_text(layout) -> String` — flatten a layout tree to a
  plain text dump.  Useful for formatters that just want a String
  without going through the paint pipeline.

### Hardening (security review)

- **HIGH: `fits()` stack-clone DoS** — original implementation
  cloned the entire pending stack at every group via `to_vec()`,
  giving O(N²) memory for N nested groups.  Fix: borrow the
  parent stack and only clone the few descended children.  Two
  regression tests added (1000 nested groups, 500-deep nested
  with siblings).
- **HIGH: `text("a\nb")` not validated** — the doc claimed "no
  newlines inside text" but the implementation didn't enforce
  it; literal `\n` flowed into spans, breaking downstream
  backends (paint-vm, canvas, SVG) that assume monospace
  single-line cells.  Fix: `text()` auto-splits on `\n` and
  normalises `\r` / `\r\n`.  Three regression tests added.

### Notes

- Pure data + algorithms.  **Zero dependencies.**  No I/O, no
  FFI, no unsafe.  See `required_capabilities.json`.
- 40 unit tests + 1 doctest covering all builders, all line
  modes, group flat/broken decisions, `if_break`, annotations
  (single / nested / layout-neutral / span coalescing), layout
  tree shape, `render_text`, look-ahead `fits()` corner cases,
  idempotency-of-layout, and the security hardening above.
- Filed as follow-ups in the README roadmap: `format-doc-to-paint`
  (PaintScene bridge), `format-doc-std` (delimited_list / call_like
  / block_like / infix_chain templates), `twig-formatter` (the
  authoring-experience consumer), richer combinators (`fill`,
  `align`, `line_suffix`, `break_parent`), `unicode-width`
  integration for CJK / emoji.
