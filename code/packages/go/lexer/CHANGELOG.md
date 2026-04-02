# Changelog

## [0.2.1] - 2026-03-31

### Fixed

- **STRING case preservation in case-insensitive grammars**: When a grammar uses
  `case_sensitive: false` (e.g. SQL, VHDL), the lexer lowercases the working
  source copy for pattern matching. Previously this also lowercased STRING token
  values — `'Alice'` would tokenize as `STRING("alice")` instead of `STRING("Alice")`.
  The fix adds an `originalSource` field to `GrammarLexer` that stores the raw
  (unmodified) input. When extracting the body of STRING tokens, the lexer now
  reads from `originalSource` at the same byte offset, so string literal case is
  always preserved regardless of the grammar's case-sensitivity setting.

## [0.2.0] - Unreleased

### Added
- `LexerContext` struct with methods for controlling the lexer from callbacks:
  `PushGroup`, `PopGroup`, `ActiveGroup`, `GroupStackDepth`, `Emit`,
  `Suppress`, `Peek`, `PeekStr`, `SetSkipEnabled`.
- `OnTokenCallback` function type for on-token callbacks.
- `GrammarLexer.SetOnToken()` method to register/clear callbacks.
- Pattern group support: `groupPatterns` map compiles per-group patterns from
  grammar `Groups`; `groupStack` enables stackable group transitions.
- Skip-enabled toggle: callbacks can disable skip pattern processing for groups
  where whitespace is significant (e.g., CDATA, raw text blocks).
- `tryMatchTokenInGroup()` method for matching against a specific group's patterns.
- Group stack and skip state reset between `Tokenize()` calls for safe reuse.
- Standard tokenizer now respects `hasSkipPatterns` flag to choose between
  grammar-defined skip patterns and hardcoded whitespace skipping (matching
  Python lexer behavior).

## [0.1.0] - Unreleased

### Added
- Configurable Lexer implementation generating structural constants converting directly over standard UTF-8 parsing loops precisely tracking states independently resolving escapes iteratively internally directly executing safely natively inside `lexer`.
