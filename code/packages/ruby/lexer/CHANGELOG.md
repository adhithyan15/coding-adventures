# Changelog

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
