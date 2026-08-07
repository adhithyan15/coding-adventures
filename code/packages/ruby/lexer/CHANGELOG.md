# Changelog

## [Unreleased] — F10 declarative lexer mode transitions

### Fixed
- Indentation-sensitive token streams now emit the final logical `NEWLINE`
  before any remaining `DEDENT` tokens and the terminal `EOF`, matching the
  grammar contract for canonical multiline Starlark blocks.

### Added
- `GrammarLexer` now interprets the declarative mode-transition table on a
  `TokenGrammar` (F10), the Ruby port of the Rust `grammar_lexer.rs` and
  Python `grammar_lexer.py` interpreters. After each token is emitted it
  consults the table and may `set-mode` (flat toggle of the active group),
  `push`/`pop` (F04 nested regions), or toggle skip — enabling
  context-sensitive lexing (JavaScript regex-vs-division, template
  substitutions) without a hand-written on-token callback. New
  `apply_transitions` + `transition_key`; start mode from
  `grammar.start_mode`.
- **Flat-mode inheritance**: a group reached via `set-mode` inherits the
  default group's patterns (own patterns take priority), so a JS `div` mode
  can override `SLASH`/`SLASH_EQUALS` ahead of `REGEX` without duplicating
  the grammar. `push` targets stay exclusive (F04 region semantics).
  Derived automatically from the transition table (`@inheriting_modes`).
- Because every `TokenType` constant IS its UPPER_SNAKE string
  (`TokenType::NAME == "NAME"`) and custom token types are already strings,
  the transition key is the token's type directly — no inverse mapping.

### Notes
- Fully backward compatible: a grammar with no `transitions:` table yields
  an identical token stream (every F10 helper early-returns). Verified by
  the existing suite + 7 new F10 tests. Consumes the grammar_tools F10 data
  model added in the Ruby grammar_tools port.

## [0.3.0] - 2026-04-04

### Added
- **Token flags**: `Token` now accepts an optional `flags:` keyword argument
  (Integer or nil). Two bitmask constants are defined:
  - `TOKEN_PRECEDED_BY_NEWLINE` (1) -- set when a line break appeared before the token
  - `TOKEN_CONTEXT_KEYWORD` (2) -- set for context-sensitive keywords (e.g., `async`, `yield`)
- **LexerContext extensions**:
  - `previous_token` -- lookbehind: returns the most recently emitted token
  - `bracket_depth(kind = nil)` -- query nesting depth for `()`, `[]`, `{}`
  - `preceded_by_newline?` -- true if a newline appeared before the current token
- **GrammarLexer extensions**:
  - `bracket_depth(kind = nil)` -- public API for bracket depth tracking
  - Automatic bracket depth tracking via `update_bracket_depth` on every token
  - `@last_emitted_token` tracking for lookbehind support
  - Context keyword support: NAME tokens matching `context_keywords` from the
    grammar receive the `TOKEN_CONTEXT_KEYWORD` flag

## [0.2.1] - 2026-03-31

### Fixed

- **STRING case preservation in case-insensitive grammars**: When a grammar uses
  `case_sensitive: false` (e.g. SQL, VHDL), the lexer lowercases the working
  source copy for pattern matching. Previously this also lowercased STRING token
  values — `'Alice'` would tokenize as `STRING("alice")` instead of `STRING("Alice")`.
  The fix stores the original (unmodified) source in `@original_source` and uses
  it when extracting the body of STRING tokens, so string literal case is always
  preserved regardless of the grammar's case-sensitivity setting.

## [0.2.0] - 2026-03-21

### Added
- `LexerContext` class -- callback interface for controlling the lexer during tokenization
  - `push_group(name)` / `pop_group` -- push/pop pattern groups on the group stack
  - `active_group` / `group_stack_depth` -- inspect the current group stack
  - `emit(token)` -- inject synthetic tokens after the current one
  - `suppress` -- suppress the current token from output
  - `peek(offset)` / `peek_str(length)` -- peek ahead in the source text
  - `set_skip_enabled(bool)` -- toggle skip pattern processing
- `GrammarLexer#set_on_token(callback)` -- register an on-token callback
- Pattern group support in `GrammarLexer`:
  - `@group_patterns` dict -- compiled patterns per group ("default" + named groups)
  - `@group_stack` -- stackable group transitions (bottom is always "default")
  - `@skip_enabled` flag -- togglable by callback for significant whitespace
  - `try_match_token_in_group(group_name)` -- match against specific group's patterns
  - Group stack and skip flag reset between `tokenize` calls
- Standard tokenization now uses active group and invokes callback

## [0.1.0] - 2026-03-18

### Added
- `Tokenizer` class -- hand-written lexer (NAME, NUMBER, STRING, KEYWORD, operators, delimiters)
- `GrammarLexer` class -- grammar-driven lexer that reads `.tokens` files via grammar_tools
- `Token` immutable data type with type, value, line, column
- `TokenType` module with 16 token type constants
- `LexerError` exception with line and column information
- Keyword support via configurable keyword list
- Escape sequence handling in string literals (\\n, \\t, \\\\, \\")
