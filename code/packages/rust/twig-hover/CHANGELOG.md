# Changelog — twig-hover

## [0.1.0] — 2026-05-01

Initial release.  LSP hover-info extraction for Twig — the fifth
piece of the Twig authoring-experience layer.

### Added

- `hover_at(source, line, column) -> Result<Option<Hover>, TwigParseError>`
  — parses + walks.
- `hover_for_program(&Program, line, column) -> Option<Hover>` —
  already-parsed AST + position → hover.
- `Hover { kind, name, signature, line, column, length }` — 1-based
  positions, monospace cell length.
- `HoverKind` enum (`#[non_exhaustive]`): `Function`, `Variable`,
  `UnresolvedVariable`, `Boolean`, `Nil`, `Number`, `Symbol`,
  `Keyword`.
- `HoverKind::mnemonic() -> &'static str` — stable lowercase
  strings.

### Behaviour

- VarRef resolution against top-level `(define …)` symbols via
  `twig-document-symbols`:
  - `Function` (with `signature: Some("(params)")`) for lambda
    bindings.
  - `Variable` for value bindings.
  - `UnresolvedVariable` if no matching define (parameter,
    let-binding, or typo).
- `(define name …)` keyword and name surface as separate hovers.
- Boundary rule: cursor at `col + len` still counts as "on" the
  token; one past does not.
- Zero-position sentinels (`line == 0` or `column == 0`) never
  match.

### Notes

- Pure data → typed hover info.  Two deps (`twig-parser`,
  `twig-document-symbols`), both capability-empty.  No I/O, no
  FFI, no unsafe.  See `required_capabilities.json`.
- 26 unit tests covering each atom kind, each keyword, VarRef
  resolution (function / variable / unresolved), define name
  surfacing, boundary cases, multi-line input, error path, and
  `hover_for_program` direct path.
- Filed as follow-ups in README roadmap: type info (needs Twig
  type-checker), scope-aware resolution for let / lambda
  parameters (needs parser threading), doc comments (needs lexer
  trivia channel), LSP wire encoding.
