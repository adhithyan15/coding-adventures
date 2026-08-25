# Changelog

All notable changes to the `coding-adventures-csharp-parser` crate will be documented in this file.

## [0.1.1] - 2026-08-25

### Fixed — multi-level nested generics (`Dictionary<string, List<int>>`) now parse

Same shared-engine gap and fix as `coding-adventures-java-parser` 0.1.2:
the lexer merges consecutive `>` characters into a single `RIGHT_SHIFT`/
`UNSIGNED_RIGHT_SHIFT`-typed token, and this parser had no contextual
token-splitting to recover two/three separate closers from it. Fixed at
the shared `parser` crate engine level (`parser` 0.4.4's new
`split_angle_bracket_run`) — no code change needed in this crate itself,
only new tests: `two_level_nested_generic_closes_from_a_merged_right_shift_token`,
`three_level_nested_generic_closes_from_a_merged_unsigned_right_shift_token`,
and a confirmation that a real `>>` shift expression still parses
correctly.

## [0.1.0] - 2026-04-11

### Added
- `create_csharp_parser(source, version)` — factory function that loads the appropriate `csharp{version}.grammar` and returns a configured `GrammarParser`. The `version` parameter selects the C# edition: `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`, `"5.0"`, `"6.0"`, `"7.0"`, `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, `"12.0"` (default: `"12.0"`).
- `parse_csharp(source, version)` — convenience function that parses C# source and returns a `GrammarASTNode`.
- `grammar_root()` helper that uses `PathBuf` navigation from `env!("CARGO_MANIFEST_DIR")`.
- Returns `Err(String)` for unrecognised version strings instead of panicking on a missing file.
- The lexer is called with the same version string so tokens and grammar are always from the same C# edition — critical for version-sensitive keywords like `record` (C# 9.0+), `async`/`await` (C# 5.0+), and `dynamic` (C# 4.0+).
- Test suite (12 tests) covering class declarations, arithmetic expressions, multiple statements, empty programs, the factory function, versioned grammar selection for C# 8.0, 5.0, 3.0, and 12.0 individually, the all-versions smoke test (all 12 versions), and error cases for unknown versions and empty version strings.
