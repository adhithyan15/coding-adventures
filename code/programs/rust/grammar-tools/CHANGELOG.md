# Changelog — grammar-tools (Rust program)

## [2.3.0] - 2026-10-20

### Added — LANG84 phase 7a: Python joins the release families
- `generate-rust-compiled-grammars python` now reads its release list from
  `code/grammars/python/releases.csv`. The five phase-7a pairs (3.0a1,
  3.11.17, 3.12.3, 3.13.16, 3.16.0a1) compile to three distinct token tables
  and one parser table.
- A release family can keep pre-LANG84 **minor-line** pairs
  (`RustReleaseFamilySpec::legacy_versions`). Python's six (`python2.7` …
  `python3.12`) are embedded in a generated `pub mod legacy_versions` with its
  own `SUPPORTED_VERSIONS` and `token_grammar(version)` /
  `parser_grammar(version)`, which the release selector cannot reach. A
  legacy stem that is also a paired release, or a missing legacy file, is an
  error.
- Release pairs are now always validated and cross-validated (LANG84 stage G),
  even when the package needs `force` for its legacy grammars, as
  `python-parser` does. `force` applies only to the legacy modules.
- New tests: `release_family_embeds_legacy_minor_lines_out_of_the_release_selector`
  and `committed_python_release_tables_match_regeneration`.

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
