# Changelog

All notable changes to this package will be documented in this file.

## [0.4.0] - 2026-06-13

### Added

- **F10 declarative lexer mode transitions** (TypeScript port mirroring the
  Rust reference (#5662), Python (#5668), and Ruby (#5694) ports). When a
  `.tokens` grammar declares a `transitions:` table plus optional
  `start_mode:`, the lexer now consults the table after each emitted token
  to switch the active mode — no host-language callback required. The
  generic group stack is initialized to `start_mode` (or `"default"` when
  unset) and reset to it between `tokenize()` calls.
  - New `_transitions` / `_startMode` / `_inheritingModes` fields on
    `GrammarLexer` (read-only, computed at construction).
  - New private `_applyTransitions(token)` runs the first-matching rule
    after each token in standard and indentation tokenize loops; supports
    `set_mode` (flat toggle, mutates top-of-stack in place), `push`/`pop`
    (nested), and `enable_skip`/`disable_skip`.
  - `_tryMatchTokenInGroup` honors F10 flat-mode inheritance: targets
    reached via `set-mode` (and never via `push`) inherit the `default`
    group's patterns; their own patterns take priority, the default
    patterns fall through. `push` targets stay exclusive (F04 / XML
    semantics).
  - Empty `transitions:` table is a no-op — F04 behavior is preserved
    exactly. Backward compatible: all existing grammars and tests
    unchanged.
  - +9 unit tests covering start-mode init, flat-toggle depth invariance,
    flat-mode inheritance, `push`/`pop` depth changes, `in MODE` guards,
    `KEYWORD="value"` guards, empty-table no-op, unknown-start-mode
    fallback, and a custom non-`"default"` start mode.

  Existing test imports were retargeted from a stale sibling copy of
  `grammar-tools` to the published `@coding-adventures/grammar-tools`
  package so the F10 surface (`startMode`/`transitions`) is visible to
  parser tests.

## [0.3.0] - 2026-04-18

### Added

- Optional rich source preservation for `GrammarLexer` and `grammarTokenize()`
  via `{ preserveSourceInfo: true }`.
- `Trivia` type for preserved skip matches such as whitespace and comments.
- Optional token metadata fields:
  - `startOffset` / `endOffset`
  - `endLine` / `endColumn`
  - `tokenIndex`
  - `leadingTrivia`
- `typeName?: string` on `Token` so layout-mode virtual tokens and similar
  source-preserving callers can remain type-safe.

## [0.2.0] - 2026-03-21

### Added

- `GrammarLexer` class — class-based grammar-driven lexer with support for
  pattern groups and on-token callbacks. Replaces the internal implementation
  of `grammarTokenize` while maintaining backward compatibility.
- `LexerContext` class — callback interface for controlling the lexer during
  tokenization. Provides methods for:
  - `pushGroup(groupName)` / `popGroup()` — switch between pattern groups
  - `activeGroup()` / `groupStackDepth()` — inspect group stack state
  - `emit(token)` — inject synthetic tokens after the current one
  - `suppress()` — suppress the current token from output
  - `peek(offset)` / `peekStr(length)` — lookahead into source text
  - `setSkipEnabled(enabled)` — toggle skip pattern processing
- `OnTokenCallback` type — signature for on-token callback functions.
- `GrammarLexer.setOnToken(callback)` — register a callback that fires on
  every token match (except skip matches, emitted tokens, and EOF).
- Pattern group support in `GrammarLexer` — compiles and uses per-group
  patterns from the grammar's `groups` field. The group stack starts at
  "default" and resets between `tokenize()` calls.
- Comprehensive test suite for `LexerContext` (10 unit tests) and pattern
  group tokenization (13 integration tests) covering push/pop, suppress,
  emit, token replacement, skip toggling, nested structures, and backward
  compatibility.
- Exported `GrammarLexer`, `LexerContext`, and `OnTokenCallback` from
  package index.

### Changed

- `grammarTokenize()` is now a thin wrapper around `GrammarLexer.tokenize()`.
  All existing callers continue to work without changes.

## [0.1.0] - 2026-03-19

### Added

- Initial TypeScript port of the lexer package from Python.
- `Token` interface with `type`, `value`, `line`, `column` fields.
- `tokenize()` — hand-written character-by-character lexer supporting:
  - Integer literals (NUMBER)
  - Identifiers (NAME)
  - String literals with escape sequences (STRING)
  - Configurable keyword recognition (KEYWORD)
  - Operators: `+`, `-`, `*`, `/`, `=`, `==`
  - Delimiters: `(`, `)`, `,`, `:`
  - Newline tokens and EOF sentinel
  - Position tracking (line and column numbers)
  - Error reporting with `LexerError`
- `grammarTokenize()` — grammar-driven lexer that reads token definitions from a `TokenGrammar` object (parsed from `.tokens` files by `@coding-adventures/grammar-tools`).
  - Regex and literal pattern compilation
  - First-match-wins priority ordering
  - Keyword detection via grammar keyword lists
  - String escape sequence processing
  - Full interchangeability with `tokenize()`
- Comprehensive test suite for both lexer implementations.
- Comparison tests verifying both lexers produce identical output.
- Custom grammar tests for programmatically-built grammars.
- Ruby grammar integration tests.
