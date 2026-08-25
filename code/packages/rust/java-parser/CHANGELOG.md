# Changelog

All notable changes to the `coding-adventures-java-parser` crate will be documented in this file.

## [0.1.2] - 2026-08-25

### Fixed — multi-level nested generics (`Map<String, List<Integer>>`) now parse

Previously a KNOWN, tracked gap (see this crate's own `README.md`/test
comments before this version): the lexer merges consecutive `>`
characters into a single `RIGHT_SHIFT`/`UNSIGNED_RIGHT_SHIFT`-typed token,
and this parser had no contextual token-splitting to recover two/three
separate closers from it. Fixed at the shared `parser` crate engine level
(`parser` 0.4.4's new `split_angle_bracket_run`, a shared fix that also
resolves the identical gap in `coding-adventures-csharp-parser`) — no
code change needed in this crate itself, only new tests proving the fix:
`two_level_nested_generic_closes_from_a_merged_right_shift_token`,
`three_level_nested_generic_closes_from_a_merged_unsigned_right_shift_token`,
and two tests confirming a real `>>`/`>>>` shift expression still parses
correctly, including alongside a nested generic in the same file.

## [0.1.1] - 2026-08-24

### Added
- Real construct-coverage tests (JV02 spec's M0 hardening pass): interface
  declarations with a generic type parameter and method signature, class
  declarations with `extends`/`implements`, a generic class with a field,
  a lambda expression, `try`/`catch`/`finally` with a `throws` clause, an
  `@Override` annotation on a method, an enum declaration, and varargs +
  a method reference. Previously this crate's only construct-level test
  (`parses_basic_class`) checked nothing beyond `ast.rule_name ==
  "program"` on a bare `class Hello { }`. Uses the engine's own
  `parser::grammar_parser::find_nodes`/`collect_tokens` walkers to assert
  named rules genuinely appear in the parsed tree, rather than only that
  parsing didn't error.

### Known gap (not fixed here, tracked separately)
- Multi-level generic nesting (`Map<String, List<Integer>>`) fails to
  parse: the lexer merges the two closing `>` characters into a single
  `">>"`-valued token (the same shape a real right-shift operator would
  produce — see `coding-adventures-java-lexer`'s own CHANGELOG for the
  same finding), and this crate has no contextual token-splitting logic
  to re-derive two separate closers from it. Confirmed to be a shared
  `parser` crate (`GrammarParser`) engine gap, not Java-specific
  (identically reproducible in `csharp-parser`) — not fixed in this PR,
  which is scoped to test coverage; tracked as its own follow-up.

No public API change — version bump reflects the added test coverage
only.

### Added
- `create_java_parser(source, version)` — factory function that loads the appropriate `java{version}.grammar` and returns a configured `GrammarParser`. The `version` parameter selects the Java edition: `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"` (default: `"21"`).
- `parse_java(source, version)` — convenience function that parses Java source and returns a `GrammarASTNode`.
- `grammar_root()` helper that uses `PathBuf` navigation from `env!("CARGO_MANIFEST_DIR")`.
- Returns `Err(String)` for unrecognised version strings instead of panicking on a missing file.
- The lexer is called with the same version string so tokens and grammar are always from the same Java edition.
- Test suite covering class declarations, expressions, multiple statements, empty programs, factory function, versioned grammar selection, and error cases for unknown versions.
