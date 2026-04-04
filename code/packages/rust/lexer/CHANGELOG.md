# Changelog

All notable changes to this package will be documented in this file.

## [0.3.0] - 2026-04-04

### Added
- Token flag constants: `TOKEN_PRECEDED_BY_NEWLINE` (bit 0) and
  `TOKEN_CONTEXT_KEYWORD` (bit 1) for bitmask metadata on tokens.
- `Token.flags: Option<u32>` field — optional bitmask carrying metadata
  that is neither type nor value but affects downstream interpretation
  (e.g., automatic semicolon insertion, context-sensitive keywords).
- `BracketDepths` struct and `BracketKind` enum for per-type bracket
  nesting depth tracking (`()`, `[]`, `{}`). The lexer updates depths
  after each token emission; callbacks access them via `LexerContext`.
- `LexerContext` extensions:
  - `previous_token()` — lookbehind: the most recently emitted token.
  - `bracket_depth(kind)` / `total_bracket_depth()` — bracket nesting.
  - `preceded_by_newline()` — true if a line break appeared between the
    previous token and the current token.
- `GrammarLexer` extensions:
  - `last_emitted_token` field — tracks the most recently emitted token.
  - `bracket_depths` field — per-type nesting counters.
  - `context_keyword_set` field — set of context-sensitive keywords from
    the grammar's `context_keywords:` section. NAME tokens matching this
    set are emitted with the `TOKEN_CONTEXT_KEYWORD` flag.

## [0.2.0] - 2026-03-21

### Added
- `LexerContext` struct — callback interface for controlling the lexer during
  tokenization. Provides methods for group stack manipulation (`push_group`,
  `pop_group`, `active_group`, `group_stack_depth`), token injection (`emit`),
  token suppression (`suppress`), source peeking (`peek`, `peek_str`), and
  skip pattern toggling (`set_skip_enabled`).
- `ContextAction` enum — deferred mutation type (`Push`, `Pop`, `Emit`,
  `Suppress`, `SetSkipEnabled`) that satisfies the borrow checker by collecting
  actions during the callback and applying them afterward.
- `OnTokenCallback` type alias for `Box<dyn FnMut(&Token, &mut LexerContext)>`.
- `GrammarLexer::set_on_token()` — register an optional callback that fires
  after each token match (not for skip matches, emitted tokens, or EOF).
- Pattern group support in `GrammarLexer`:
  - `group_patterns` HashMap — compiled patterns per group ("default" + named groups).
  - `group_stack` — stackable group transitions, bottom is always "default".
  - `try_match_token_in_group()` — match against a specific group's patterns.
  - `skip_enabled` flag — togglable by callback for significant-whitespace groups.
  - Group stack and skip_enabled reset between `tokenize()` calls.
- 24 new tests covering LexerContext unit behavior, pattern group switching
  (push/pop, nested tags, attributes), token suppression, synthetic token
  emission, suppress+emit replacement, skip toggling, backward compatibility,
  callback clearing, and group stack reset.

## [0.1.0] - 2026-03-19

### Added
- `token` module with `TokenType` enum (23 variants), `Token` struct, and `LexerError` type.
- `tokenizer` module — hand-written character-by-character Python lexer with:
  - Configurable keyword set for keyword promotion (NAME -> KEYWORD).
  - String literal support with escape sequence processing (\n, \t, \\, \").
  - Lookahead for multi-character operators (= vs ==).
  - Single-character token lookup table for operators and delimiters.
  - Line and column position tracking for error messages.
  - Comprehensive error reporting for unexpected characters and unterminated strings.
- `grammar_lexer` module — grammar-driven universal lexer with:
  - Accepts a `TokenGrammar` from the `grammar-tools` crate.
  - Compiles grammar patterns into anchored regexes at construction time.
  - First-match-wins semantics matching the grammar's definition order.
  - Keyword promotion from NAME to KEYWORD using the grammar's keyword list.
  - String escape processing matching the hand-written lexer's behavior.
  - Consistency tests verifying identical output between both lexer implementations.
