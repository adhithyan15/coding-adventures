# Changelog — twig-document-symbols

## [0.1.0] — 2026-04-30

Initial release.  LSP outline view for Twig — the third piece of
the Twig authoring-experience layer (alongside `twig-formatter`
and `twig-semantic-tokens`).

### Added

- `document_symbols(source) -> Result<Vec<DocumentSymbol>, TwigParseError>`
  — parses + walks.
- `symbols_for_program(&Program) -> Vec<DocumentSymbol>` —
  already-parsed AST → symbols.
- `DocumentSymbol { name, kind, detail, line, column }` — 1-based
  positions matching twig-parser.
- `SymbolKind` enum (`#[non_exhaustive]`, `Function` / `Variable`).
- `SymbolKind::mnemonic()` — stable lowercase string matching LSP's
  `SymbolKind` names where the meanings line up.
- `SymbolKind: Display` via the mnemonic.

### Behaviour

- `(define name (lambda params body))` → `Function` symbol with
  `detail = "(params)"`.
- `(define name expr)` (any other expr) → `Variable` symbol with
  `detail = None`.
- Bare top-level expressions are skipped.
- Symbols come back in document order (top to bottom).
- `(define (f x) body)` sugar is lowered to `(define f (lambda
  (x) body))` by the parser, so it surfaces as `Function`
  automatically.
- Nested defines aren't a concern — the Twig grammar only allows
  `(define …)` at the top level; the parser rejects them in
  expression position before they reach this crate (test
  `nested_defines_rejected_at_parse_layer` documents this).

### Notes

- Pure data → typed symbol list.  Single dep on `twig-parser`
  (also capability-empty).  No I/O, no FFI, no unsafe.  See
  `required_capabilities.json`.
- 20 unit tests covering empty programs, bare top-level
  expressions (no symbol), value bindings (`Variable`), lambda
  bindings (`Function`) including sugar form, nullary and
  multi-param signatures, multiple top-level defines, document-
  order sort, mixed top-level streams, correct multi-line
  positions, error path, direct `symbols_for_program` path, a
  realistic four-symbol module outline, and long-identifier
  passthrough.
- Filed as follow-ups in README roadmap: end positions (needs
  twig-parser threading for LSP `range` / `selectionRange`),
  workspace symbols aggregator, `twig-lsp` wire encoding, `twig
  outline` CLI binary.
