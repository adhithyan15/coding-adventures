# Changelog — grammar-tools (Rust program)

## [2.3.0] - 2026-10-20

### Added — Perl joins the LANG84 release families (phase 4a)
- `generate-rust-compiled-grammars perl` uses the generic release-family path
  (`RustReleaseFamilySpec`) instead of a Perl-only generator. Perl's 117
  checked pairs become 5 token and 2 parser tables; its generated API matches
  Ruby's (`SUPPORTED_RELEASES`, `grammar_class`, the exact-ID selector, and
  `INVENTORY` in the parser table).
- `RustReleaseFamilySpec.legacy_token_grammar`: also embed the unversioned
  `<prefix>.tokens` as `unversioned_token_grammar()` (Perl on, Ruby off).
- `RustReleaseFamilySpec.stage_source`: `Column` (the LANG84 schema; the
  `grammar_stage` column is required) or `DerivedFromStatus` for an inventory
  that predates it (Perl until LANG84 2b), which derives the stage by the
  spec's migration rule (`pending` → `none`, otherwise `P`).

### Security
- The release-inventory reader now also refuses duplicate release IDs and
  unknown `grammar_status` / `grammar_stage` values, on top of quoted rows,
  field-count mismatches and unsafe IDs. A trailing `\r` is dropped so a
  CRLF checkout reads like an LF one. Ruby's generated tables are unchanged
  byte for byte.

## [2.2.0] - 2026-10-20

### Added — LANG84 release families
- `generate-rust-compiled-grammars ruby` now reads the release list from
  `code/grammars/ruby/releases.csv` instead of a hard-coded version array.
  Every row past stage `none` must have both `ruby<release>.tokens` and
  `ruby<release>.grammar`; a missing file is an error, not a skip.
- Pairs are deduplicated by content: each pair is parsed, its line numbers
  zeroed, compiled with a blank source name, and hashed with SHA-256. One
  module `g_<12 hex>` is emitted per distinct grammar, keeping the first
  release's real line numbers. A 48-bit prefix collision between different
  grammars is detected by comparing the full canonical text.
- Generated selectors take an exact release ID and return `None` otherwise
  (`token_grammar(release)` / `parser_grammar(release)`), plus
  `SUPPORTED_RELEASES`, `grammar_class(release)`, and for parser tables the
  full `INVENTORY` of `(release, grammar_stage)` rows. The unversioned
  `ruby.grammar` stays embedded as `unversioned_parser_grammar()` for legacy
  callers and is not reachable through the release selector.
- Each release pair is validated and cross-validated before generation
  (LANG84 stage G).
- New dependency: `coding_adventures_sha256` (module naming).
- 5 new tests, including one that fails when the committed Ruby tables are
  stale.

## [2.1.0] - 2026-03-26

### Added
- `--force` / `-f` flag — skip grammar validation and compile anyway. Used when patterns
  (e.g. lookahead `(?!...)` in `excel.tokens`, `xml.tokens`) are known to be handled at
  runtime by the consuming package. Without `--force`, any unsupported regex pattern causes
  the compile command to exit 1.

## [2.0.0] - 2026-03-26

### Added
- `compile-tokens <file.tokens> [-o out.rs]` — compiles a `.tokens` file to Rust source code.
- `compile-grammar <file.grammar> [-o out.rs]` — compiles a `.grammar` file to Rust source code.
- `--output` / `-o` flag — write generated code to a file instead of stdout.
- Status messages go to stderr; generated code goes to stdout.
- 8 new tests for compile commands and dispatch routing.

## [1.0.0] - 2026-03-26

### Added
- Initial release. Replaces `src/bin/grammar-tools.rs` in the library package.
- `validate`, `validate-tokens`, `validate-grammar` commands.
- Uses `cli-builder` for `--help`, `--version`, and parsing.
- Exit codes 0/1/2 identical to all other language implementations.
