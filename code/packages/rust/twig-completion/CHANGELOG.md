# Changelog — twig-completion

## [0.1.0] — 2026-05-01

Initial release.  LSP code-completion items for Twig — the sixth
piece of the Twig authoring-experience layer.

### Added

- `completions(source, prefix) -> Result<Vec<CompletionItem>, TwigParseError>`
  — parses + walks.
- `completions_for_program(&Program, prefix) -> Vec<CompletionItem>`
  — already-parsed AST + prefix → items.
- `CompletionItem { label, kind, detail }`.
- `CompletionKind` enum (`#[non_exhaustive]`): `Function`,
  `Variable`, `Keyword`, `Constant`.
- `CompletionKind::mnemonic() -> &'static str`.

### Behaviour

- Items sorted: keywords (declaration order) → constants → user
  symbols (alphabetical).
- Six built-in keywords: `define`, `if`, `let`, `lambda`, `begin`,
  `quote`.
- Three built-in constants: `#t`, `#f`, `nil`.
- User-defined symbols sourced from `twig-document-symbols`:
  Function items carry the lambda parameter signature in `detail`;
  Variable items have `detail = None`.
- Prefix filter (when `Some`) is exact-prefix and case-sensitive.
- `prefix = None` returns the full menu (editors that prefer
  client-side fuzzy can pass `None`).
- Output is deterministic across calls for the same input.

### Notes

- Pure data → typed completion-item list.  Two deps
  (`twig-parser`, `twig-document-symbols`), both capability-empty.
  No I/O, no FFI, no unsafe.  See `required_capabilities.json`.
- 21 unit tests covering kind mnemonics, built-in always-present,
  kind classification, sort order (keywords → constants →
  symbols), keyword declaration order, alphabetical user-symbol
  order, function/variable detail, prefix filtering (empty /
  matching / keyword / constant / no-match / case-sensitive),
  error path, `completions_for_program` direct path, realistic
  four-symbol menu, and determinism.
- Filed as follow-ups in README roadmap: snippets (`insert_text`
  with placeholders), scope-aware suggestions (needs parser
  threading), doc comments (needs trivia channel), LSP wire
  encoding.
