# Changelog

## [0.3.0] - 2026-04-04

### Added
- **Token flags**: `Flags int` field on `Token` struct with bitmask constants
  `TokenPrecededByNewline` (1) and `TokenContextKeyword` (2). Flags carry metadata
  that is neither type nor value but affects how downstream consumers interpret a token.
- **Per-type bracket depth tracking**: `GrammarLexer` now tracks `()`, `[]`, and `{}`
  depths independently (previously only a single counter for indentation mode).
  Exposed via `GrammarLexer.BracketDepth(kind)` and `LexerContext.BracketDepth(kind)`.
- **Token lookbehind**: `LexerContext.PreviousToken()` returns the most recently
  emitted token for context-sensitive decisions (e.g., regex vs division in JS).
- **Newline detection**: `LexerContext.PrecededByNewline()` returns true when a line
  break appeared between the previous and current token. Used for automatic semicolon
  insertion in JavaScript/Go.
- **Context keywords**: When the grammar defines `context_keywords:`, matching NAME
  tokens are emitted with `Flags = TokenContextKeyword`. This supports words like
  JavaScript's `async`, `yield`, `get` that are sometimes keywords and sometimes
  identifiers.
- `GrammarLexer` now resets `lastEmittedToken` and `bracketDepths` between
  `Tokenize()` calls for safe reuse.

## [0.2.2] - 2026-04-02

### Fixed

- **PanicOnUnexpected missing from intentional-panic operations**: `Lexer.Tokenize()`,
  `GrammarLexer.Tokenize()`, and `LexerContext.PushGroup()` all call `StartNew` but
  previously omitted `.PanicOnUnexpected()` before `.GetResult()`. The `StartNew`
  framework recovers panics by default, silently swallowing errors that are part of
  the documented contract (unterminated string literals, unexpected characters,
  reserved keyword violations, tab indentation, inconsistent dedent, unknown group
  names). Adding `.PanicOnUnexpected()` to these three chains restores the
  intended panic-propagation behavior so callers can use `defer/recover` to catch
  lex errors.

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
