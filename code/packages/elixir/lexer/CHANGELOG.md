# Changelog

## 0.3.0 — 2026-04-04

### Added
- `Token.preceded_by_newline/0` — flag constant (bit 0, value 1) for newline detection
- `Token.context_keyword/0` — flag constant (bit 1, value 2) for context-sensitive keywords
- `Token.flags` field — optional bitmask carrying metadata about the token
- `LexerContext.previous_token/1` — lookbehind access to the most recently emitted token
- `LexerContext.bracket_depth/2` — query per-type or total bracket nesting depth
- `LexerContext.preceded_by_newline/1` — detect line breaks between tokens
- Bracket depth tracking in lexer state — tracks `()`, `[]`, `{}` independently
- Last emitted token tracking — updated after each token push (including callback-emitted)
- Context keyword support — words in `context_keywords:` section are emitted as NAME
  with `TOKEN_CONTEXT_KEYWORD` flag set

### Changed
- `Token` struct now has an optional `:flags` field (nil when no flags set)
- `LexerContext` struct extended with `:previous_token`, `:bracket_depths`,
  and `:current_token_line` fields
- `State` struct extended with `:last_emitted_token`, `:bracket_depths`,
  and `:context_keyword_set` fields

## 0.2.0 — 2026-03-21

### Added
- `LexerContext` struct — read-only context passed to on-token callbacks with
  `active_group`, `group_stack_depth`, `source`, `pos_after_token`, and
  `available_groups` fields
- `LexerContext.peek/2` — peek at a source character past the current token
- `LexerContext.peek_str/2` — peek at next N characters past the current token
- Pattern group support — compile per-group patterns from grammar `group:` sections
- Group stack — stackable group transitions (push/pop) during tokenization
- On-token callback via `tokenize/3` `:on_token` option — functional style using
  action tuples instead of mutable context methods
- Action types: `{:push_group, name}`, `:pop_group`, `{:emit, token}`,
  `:suppress`, `{:set_skip_enabled, bool}`
- `skip_enabled` toggle — callbacks can disable skip pattern processing for
  groups where whitespace is significant (e.g., CDATA, raw text)
- 20 new tests covering LexerContext, pattern groups, push/pop, suppress,
  emit, token replacement, nested tags, skip toggling, and backward compat

### Changed
- `tokenize/2` now accepts an optional third argument (keyword opts) for
  passing the `:on_token` callback; existing 2-arity calls are unchanged
- State struct extended with `group_patterns`, `group_stack`, `on_token`,
  and `skip_enabled` fields
- Alias map now includes aliases from group definitions (not just top-level)
- Token matching uses active group's patterns instead of always using default

## 0.1.0 — 2026-03-20

### Added
- `GrammarLexer.tokenize/2` — grammar-driven tokenization engine
- `Token` struct with type, value, line, column fields
- Standard (non-indentation) tokenization mode
- Skip pattern support (grammar-defined whitespace/comment handling)
- Keyword detection and reclassification (NAME → KEYWORD)
- Reserved keyword checking (raises error on reserved identifiers)
- Type alias resolution (e.g., STRING_DQ → STRING)
- String escape processing: `\n`, `\t`, `\r`, `\b`, `\f`, `\\`, `\"`, `\/`, `\uXXXX`
- Position tracking (line and column numbers)
- First-match-wins priority ordering from `.tokens` file
- JSON grammar integration tests
