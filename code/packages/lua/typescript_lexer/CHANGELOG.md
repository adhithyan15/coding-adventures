# Changelog — coding-adventures-typescript-lexer (Lua)

All notable changes to this package are documented here.

## [0.2.0] — 2026-04-05

### Added

- `version` parameter added to `tokenize(source, version)`,
  `create_lexer(source, version)`, and `get_grammar(version)`.
- New `create_lexer(source, version)` function — returns an initialized
  `GrammarLexer` without immediately tokenizing.
- Version routing: when `version` is `"ts1.0"`, `"ts2.0"`, `"ts3.0"`,
  `"ts4.0"`, `"ts5.0"`, or `"ts5.8"`, the corresponding versioned grammar
  file is loaded from `code/grammars/typescript/<version>.tokens`.
- Generic fallback: passing `nil` or `""` loads the unified
  `code/grammars/typescript.tokens` as before (backward compatible).
- Per-version grammar cache: each version is loaded and parsed at most once
  per process; the cache is a table keyed by version string.
- Validation: unknown version strings raise a descriptive error immediately.
- Extended test suite: new `describe("version-aware tokenization")` block
  covering all 6 recognized versions, `create_lexer`, `get_grammar`, cache
  behavior, and error cases for invalid versions.

### Changed

- `M.VERSION` bumped from `"0.1.0"` to `"0.2.0"`.
- `_grammar_cache` changed from a single value to a table keyed by version
  (backward-compatible — no-version calls use key `""`).

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of `coding_adventures.typescript_lexer`.
- `tokenize(source)` — tokenizes a TypeScript string using the shared
  `typescript.tokens` grammar and the grammar-driven `GrammarLexer` from
  `coding-adventures-lexer`.
- `get_grammar()` — returns the cached `TokenGrammar` for direct use.
- Grammar is read from `code/grammars/typescript.tokens` once and cached.
- Path navigation uses `debug.getinfo` to locate the grammar file relative
  to the installed module, avoiding hardcoded absolute paths.
- Full token set: all JavaScript tokens (NAME, NUMBER literal, STRING
  literal, LET, CONST, VAR, IF, ELSE, WHILE, FOR, DO, FUNCTION, RETURN,
  CLASS, IMPORT, EXPORT, FROM, AS, NEW, THIS, TYPEOF, INSTANCEOF, TRUE,
  FALSE, NULL, UNDEFINED, and all operators and delimiters) plus
  TypeScript-specific keywords: INTERFACE, TYPE, ENUM, NAMESPACE, DECLARE,
  READONLY, PUBLIC, PRIVATE, PROTECTED, ABSTRACT, IMPLEMENTS, EXTENDS,
  KEYOF, INFER, NEVER, UNKNOWN, ANY, VOID, NUMBER (keyword), STRING
  (keyword), BOOLEAN, OBJECT, SYMBOL, BIGINT.
- Comprehensive busted test suite covering: inherited JavaScript keywords,
  TypeScript-specific keywords, access modifiers, primitive type keywords,
  TypeScript constructs (type annotations, generics, interfaces, enums,
  abstract classes, implements/extends, keyof, as), whitespace handling,
  position tracking, and error cases.
- `required_capabilities.json` declaring `filesystem:read` (reads grammar
  file at startup).
- `BUILD` and `BUILD_windows` scripts with transitive dependency
  installation in leaf-to-root order.
