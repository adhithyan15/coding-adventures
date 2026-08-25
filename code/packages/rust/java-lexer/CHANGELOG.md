# Changelog

All notable changes to the `coding-adventures-java-lexer` crate will be documented in this file.

## [0.1.1] - 2026-08-24

### Added
- Real construct-coverage tests (JV02 spec's M0 hardening pass, ahead of
  `java-to-semantic-ir`, the first consumer that needs to actually trust
  this crate): interface declarations with generic type parameters, the
  lambda `->` arrow tokenizing as one token (not `-` then `>`), the
  varargs `...` and method-reference `::` operators each tokenizing as
  one token, `@`/identifier separation for annotations, and `try`/
  `throw`/`catch`/`finally` keyword recognition. Previously this crate's
  only tests exercised a bare `class Hello { }` and a bare `42;`.
- A test (`nested_generic_closing_angle_brackets_merge_into_one_token`)
  pinning a load-bearing fact discovered while writing the above: this
  crate's shared `TokenType` enum has no dedicated variant for `<`/`>`/
  `->`/`@`/`&`/`::`/`...` (they all fall through to the generic `Name`
  type), and consecutive closing `>` characters from nested generics
  (`List<List<Integer>>`) merge into a single `">>"`-valued token — the
  same shape a real right-shift operator would produce. This is not a
  bug in this crate (a context-free lexer cannot know it shouldn't merge
  them without knowing it's inside a generic-argument list); it is the
  reason `java-parser` needs its own contextual token-splitting logic,
  tracked as a separate follow-up.

No public API change — version bump reflects the added test coverage
only.

### Added
- `create_java_lexer(source, version)` — factory function that loads the appropriate `java{version}.tokens` grammar and returns a configured `GrammarLexer`. The `version` parameter selects the Java edition: `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"` (default: `"21"`).
- `tokenize_java(source, version)` — convenience function that tokenizes Java source and returns `Vec<Token>`.
- `grammar_root()` helper that uses `PathBuf` navigation from `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Returns `Err(String)` for unrecognised version strings instead of panicking on a missing file.
- Test suite covering class declarations, keywords, operators, string literals, numbers, delimiters, whitespace, factory function, versioned grammar selection, and error cases for unknown versions.
