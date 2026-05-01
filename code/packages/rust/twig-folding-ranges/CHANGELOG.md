# Changelog — twig-folding-ranges

## [0.1.0] — 2026-04-30

Initial release.  LSP folding-range extraction for Twig — the
fourth piece of the Twig authoring-experience layer (alongside
`twig-formatter`, `twig-semantic-tokens`, `twig-document-symbols`).

### Added

- `folding_ranges(source) -> Result<Vec<FoldingRange>, TwigParseError>`
  — parses + walks.
- `ranges_for_program(&Program) -> Vec<FoldingRange>` — already-
  parsed AST → ranges.
- `FoldingRange { start_line, end_line, kind }` — 1-based lines.
- `FoldingRangeKind` enum (`#[non_exhaustive]`, `Region` only in
  v1) with `Default = Region`.
- `FoldingRangeKind::mnemonic()` — stable lowercase string
  matching LSP's `FoldingRangeKind` values.

### Behaviour

- Any compound form (`define`, `let`, `lambda`, `begin`, `if`,
  apply) that spans more than one source line emits a fold.
- Single-line forms produce no range (nothing to collapse).
- End lines are derived from the maximum line of any position in
  the form's subtree (approximate but tracks the visible region).
- Ranges come back sorted in document order (start line, then end
  line).

### Notes

- Pure data → typed range list.  Single dep on `twig-parser`
  (also capability-empty).  No I/O, no FFI, no unsafe.  See
  `required_capabilities.json`.
- 20 unit tests covering empty programs, single-line non-folding
  for each form, multi-line folding for each form, nested forms,
  document-order sort, mixed streams, deeply-nested apply,
  single-line filtering, error path, `ranges_for_program` direct
  path, and a realistic module example.
- Filed as follow-ups in README roadmap: end columns (needs
  twig-parser threading), comment regions (needs lexer trivia
  channel), `twig-lsp` wire encoding, configurable fold-size
  thresholds.
