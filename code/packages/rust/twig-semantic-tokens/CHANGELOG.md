# Changelog — twig-semantic-tokens

## [0.1.0] — 2026-04-30

Initial release.  Semantic-token extraction for Twig — the second
piece of the Twig authoring-experience layer (alongside
`twig-formatter`).

### Added

- `semantic_tokens(source) -> Result<Vec<SemanticToken>, TwigParseError>`
  — parses + walks.
- `tokens_for_program(&Program) -> Vec<SemanticToken>` — already-
  parsed AST → tokens.
- `SemanticToken { line, column, length, kind }` — 1-based
  positions in monospace cell units.
- `TokenKind` enum (`#[non_exhaustive]`): `Keyword`, `Boolean`,
  `Nil`, `Number`, `Symbol`, `Function`, `Variable`, `Parameter`.
- `TokenKind::mnemonic()` — stable lowercase string matching LSP
  semantic-token type names where the meanings line up.
- `TokenKind: Display` via the mnemonic.

### Behaviour

- Emits **Function** for the head of `(fn args)` when it's a
  `VarRef` — distinguishes callees from variable references in
  themes.
- Emits **Symbol** for `'foo`, with `length` including the
  apostrophe.
- Emits **Number** for integer literals, with `length` including
  the sign for negatives.
- Emits **Parameter** for the `define` name (the only binding
  whose position the parser preserves accurately today).
- Compound-form keywords (`if`, `let`, `lambda`, `begin`,
  `define`) are emitted at form-column + 1 (inside the opening
  paren).
- Tokens come back sorted in document order (line then column);
  sort is stable.

### Hardening

- All `usize → u32` conversions go through `u32_of` / `len_u32`
  (saturating to `u32::MAX`) — no truncation, no debug-mode panic.
- All column arithmetic uses `saturating_add`.
- `(line | column | length) == 0` sentinel drops AST-derived
  positions the parser couldn't fix (binding-name placeholders).

### Notes

- Pure data → typed token list.  Single dep on `twig-parser` (also
  capability-empty).  No I/O, no FFI, no unsafe.  See
  `required_capabilities.json`.
- Recursion bounded by `twig-parser`'s `MAX_AST_DEPTH`.
- 24 unit tests covering each atom, each compound form, function-
  position re-classification, document order, multi-line input,
  keyword position correctness, error path, `tokens_for_program`
  direct path, and a realistic factorial example.
- Security review: clean, no findings.
- Filed as follow-ups in README roadmap: position-preserving
  binding names (needs twig-parser threading), LSP wire encoding
  (separate `twig-lsp` crate), comment tokens (needs lexer trivia
  channel), token modifiers, operator tokens.
