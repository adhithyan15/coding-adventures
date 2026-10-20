# Changelog

## Unreleased

- LANG84 phase 4a: release selection. `compile_source_at(source, release,
  module_name, &FrontendBudgets)` compiles against exactly one checked
  inventory release, or `DEFAULT_RELEASE` (still the unversioned LANG81 pair)
  for `None`. `resolve_release` (in `release.rs`, the only code that looks at
  release IDs) returns typed `UnknownRelease` (up to five longest-prefix
  suggestions; caller text truncated to 64 characters and control
  characters escaped) or
  `ReleaseNotAvailable` errors and never falls back. `FrontendError` adds
  `Budget`, `Lex` and `Parse` with line and column, and `Unsupported` with the
  CST rule and span; every variant after resolution names the release.
  `FrontendBudgets` bounds source bytes, parser depth and IIR instructions.
  `compile_source`, `compile_ast` and `run_source` delegate to the new entry
  points; `compile_ast_at` and `run_source_at` are new.
- `print` lowers to the LANG83 `write_i64` builtin instead of the Perl-only
  `pl_print_int`, so every lang-aot backend runs the output.
- `plvm` accepts `--release <inventory-id>`.
- Tests: every checked release lowers the corpus to IIR identical to the
  default's and runs it on vm-core; release-specific lexing (CR, 250-digit
  limit); C7 op validation; typed errors and budgets.

- Reject legacy leading-zero octal tokens instead of lowering them as decimal.
- Bound every directly supplied AST text field and the module name before
  traversing or lowering the tree.

## 0.1.0

- Initial native Perl source-to-IIR-to-vm-core arithmetic and print pilot.
