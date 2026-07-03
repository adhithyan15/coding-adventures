# Changelog — `nib-lsp-bridge`

## 0.2.0 — 2026-06-01 (wires nib-formatter as the format_fn)

nib-formatter 0.1.0 (NIB-FMT01) is now wired in as the
`textDocument/formatting` provider.  Editors driving the Nib
language server will get "Format Document" + "Format Selection"
that produce canonical Nib (2-space indent inside `{}`, single
space around operators, etc.).

### Changed

- `nib_language_spec().format_fn` flipped from `None` to
  `Some(nib_format_wrapper)`.
- Added `nib_format_wrapper(&str) -> Result<String, String>`
  adapter.
- Two new tests (`format_fn_is_set`, `format_fn_produces_canonical_output`).

### Dependencies

- Added `nib-formatter` 0.1.0 as a path dependency.

## 0.1.0 — 2026-06-01 (NIB-LSP01 — initial Nib LSP bridge)

Initial release.  Sibling of `twig-lsp-bridge` 0.2.0 and
`basic-lsp-bridge` 0.1.0.  Part of the LSP phase (task #41).

### What's wired

- Embeds `code/grammars/nib.tokens` and `code/grammars/nib.grammar`
  via `include_str!`.
- `nib_language_spec()` returns a static `LanguageSpec` with:
  - Token kind map covering KEYWORD / INT_LIT / HEX_LIT / NAME /
    operators.
  - `declaration_rules: &["fn_decl"]` — Nib function declarations
    become document symbols.
  - All 12 Nib reserved keywords (`fn`, `let`, `static`, `const`,
    `return`, `for`, `while`, `in`, `if`, `else`, `true`, `false`)
    for completion + hover.
  - `format_fn: None` — formatter integration follows in
    nib-formatter (a separate PR).
- `nib-lsp-server` binary wires the spec into
  `grammar_lsp_bridge::GrammarLanguageBridge` and runs over stdio.

### Tests

- 8 unit tests covering spec sanity, file-extension detection,
  declaration-rule presence, keyword coverage, bridge
  construction, tokenize/parse, and the format_fn default.
