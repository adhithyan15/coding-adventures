# Changelog

## 0.4.0 — 2026-06-14

### Added

- **F10 declarative lexer mode transitions**: The lexer now reads `transitions:`
  and `start_mode:` from the `TokenGrammar` (parsed by the F10 grammar-tools
  upgrade) and fires mode-switch actions automatically after every emitted token
  — no `on_token` callback code required for common mode-switching patterns.

  New `State` fields:
  - `transitions` — list of `mode_transition` maps from `TokenGrammar.transitions`
  - `inheriting_modes` — MapSet of group names that inherit default patterns
  - `start_mode` — the mode the lexer starts in (defaults to `"default"`)

  Supported actions per transition rule (first-match-wins):
  - `{:set_mode, name}` — flat toggle: replace the active group in-place.
  - `{:push, name}` — nested region: push an exclusive group onto the stack.
  - `:pop` — close a nested region, restoring the previous group.
  - `:enable_skip` — resume skip-pattern processing.
  - `:disable_skip` — suspend skip-pattern processing.

  Guard semantics (all must match for a rule to fire):
  1. Token type must be in the rule's `on_tokens` list (required).
  2. If `in_mode` is set, the current active group must match.
  3. If `on_value` is set, the token's value must match.

- **Flat-mode inheritance** (`compute_inheriting_modes/1`): Groups targeted by
  `set_mode` (but not `push`) automatically inherit the default group's patterns
  as a fallthrough. Push targets remain exclusive — only their own patterns apply.
  This lets a subsidiary mode (e.g., `div_mode`) recognise its own tokens first
  and fall back to identifiers, numbers, etc. from the default group without
  duplicating patterns.

- **`start_mode` field** on `State`: when `TokenGrammar.start_mode` is set,
  the lexer begins in that group (previously always began in `"default"`).

- **F10 transitions coexist with `on_token` callbacks**: transitions fire before
  the callback, so the callback's `ctx.active_group` already reflects the
  post-transition mode. Both mechanisms can be active simultaneously.

- **13 new F10 tests** covering: backward compatibility, `set-mode` switching,
  flat-mode inheritance (set_mode target sees default patterns; push target does
  not), `in_mode` guard, push/pop nesting, `disable-skip` action, `start_mode`
  directive, and callback/transition coexistence.

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
