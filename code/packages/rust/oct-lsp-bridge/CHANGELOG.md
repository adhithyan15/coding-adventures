# Changelog — `oct-lsp-bridge`

## 0.2.0 — 2026-06-01 (wires oct-formatter as the format_fn)

oct-formatter 0.1.0 (OCT-FMT01) is now wired in as the
`textDocument/formatting` provider.  Editors driving the Oct
language server will get "Format Document" + "Format Selection"
that produce canonical Oct (2-space indent inside `{}`, single
space around operators, etc.).

### Changed

- `oct_language_spec().format_fn` flipped from `None` to
  `Some(oct_format_wrapper)`.
- Added `oct_format_wrapper(&str) -> Result<String, String>` adapter.
- Two new tests (`format_fn_is_set`, `format_fn_produces_canonical_output`).

### Dependencies

- Added `oct-formatter` 0.1.0 as a path dependency.

## 0.1.0 — 2026-06-01 (OCT-LSP01 — initial Oct LSP bridge)

Initial release.  Sibling of `twig-lsp-bridge` 0.2.0,
`basic-lsp-bridge` 0.1.0, and `nib-lsp-bridge` 0.1.0.

### What's wired

- Embeds `code/grammars/oct.tokens` and `code/grammars/oct.grammar`
  via `include_str!`.
- `oct_language_spec()` returns a static `LanguageSpec` with:
  - Token kind map covering KEYWORD / INT_LIT / HEX_LIT / BIN_LIT /
    NAME / operators / LINE_COMMENT.
  - `declaration_rules: &["fn_decl"]` — Oct function declarations
    become document symbols.
  - All 21 Oct reserved keywords (control-flow + 8008 hardware
    intrinsics) for completion + hover.
  - `format_fn: None` — formatter integration follows in
    oct-formatter.
- `oct-lsp-server` binary wires the spec into
  `grammar_lsp_bridge::GrammarLanguageBridge`.

### Tests

- 8 unit tests covering spec sanity, declaration-rule presence,
  keyword coverage (control flow AND intrinsics), bridge
  construction, tokenize, parse, and the format_fn default.
