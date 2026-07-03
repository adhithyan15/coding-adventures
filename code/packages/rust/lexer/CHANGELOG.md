# Changelog

All notable changes to this package will be documented in this file.

## [0.7.0] — 2026-06-29 — `Token` carries an optional correlation-vector id (CLOC27 P1)

`Token` gains a `pub cv: Option<String>` field — the correlation-vector id of
the source token, defaulting to `None`. This is the first slice of CLOC27
(per-fold CV provenance): the CV-aware tokenizer will populate it so the id can
ride the token through the parser into the typed AST, where the bridge stamps it
onto leaf literals — letting an optimizer fold trace its output back to source
bytes.

No behaviour change: every construction site sets `cv: None`, so a token with no
id behaves exactly as before, and nothing reads the field yet. The id is a plain
`String` (not the `CvId` alias) so this low-level crate takes on no dependency on
`correlation-vector`. All 131 lexer tests pass unchanged.

## [0.6.0] — 2026-06-21 — `GrammarLexer` sets `TOKEN_PRECEDED_BY_NEWLINE`

`GrammarLexer` now sets the `TOKEN_PRECEDED_BY_NEWLINE` flag on a token when a
line terminator (`\n`/`\r`/U+2028/U+2029) was consumed **as trivia** before it.
Previously the flag constant existed but was never populated.

- Detection lives in `try_skip` (the trivia consumer): a new
  `newline_before_next` field is set when skipped text contains a line
  terminator, then read into the next token's flags and cleared.
- **Correct across multi-line tokens by construction:** a newline *inside* a
  string or template literal is consumed by token matching, not `try_skip`, so
  it never trips the flag — the token *after* a multi-line template is only
  flagged if there is a real line break between them. (This avoids the
  start-line-arithmetic pitfall, where a multi-line predecessor's lower start
  line would falsely imply an intervening newline.)
- **Purely additive:** the flag is OR-ed alongside the existing
  `TOKEN_CONTEXT_KEYWORD`; nothing in the tree read this flag before, so no
  existing parser behaviour changes. Enables automatic semicolon insertion
  (JavaScript) and is available to any ASI-bearing language (Go, …).

## [0.5.0] — 2026-06-15 — gap-044b: template literal depth tracking

### Fixed
- `GrammarLexer` now correctly lexes template literal substitutions containing
  non-identifier expressions such as `${obj.name}`, `${a + b}`, `${f()}`,
  `${{a:1}}`, and `${x ? y : z}`.  Previously, any F10 flat-mode transition
  firing inside the substitution (e.g. `on NAME -> set-mode div`) silently
  overwrote the active group to "div" or "default", losing the template context.
  A subsequent `}` was then consumed as RBRACE instead of TEMPLATE_TAIL,
  producing a spurious `LexerError: Unexpected sequence '}'`.

### Implementation
- Added `template_entry_depths: Vec<usize>` field to `GrammarLexer`.  Each
  `TEMPLATE_HEAD` or `TEMPLATE_MIDDLE` token pushes the current brace depth;
  `TEMPLATE_TAIL` pops it.  Across the main loop, whenever a `}` is about to
  be matched at a depth that equals the recorded template-entry depth, the
  active group is temporarily overridden: "div" → "template_div", "default" →
  "template".  This ensures TEMPLATE_TAIL / TEMPLATE_MIDDLE patterns take
  priority over RBRACE for the closing `}` of each substitution, regardless of
  any flat-mode transitions that fired inside the expression.
- Nested templates are handled correctly: each `${` adds one entry; the
  innermost closes first.  Nested braces inside the expression (object
  literals, arrow function bodies) are not misidentified because the depth
  guard fires only at the exact entry depth.

## [Unreleased] — F10 declarative lexer mode transitions

### Added
- `GrammarLexer` now interprets the declarative mode transition table on a
  `TokenGrammar` (F10). After each token is emitted it consults the table and
  may `set-mode` (flat toggle of the active group), `push`/`pop` (F04 nested
  regions), or toggle skip — enabling context-sensitive lexing (JavaScript
  regex-vs-division, template substitutions) without a hand-written `on-token`
  callback. New `apply_transitions` + `transition_key`; start mode from
  `grammar.start_mode`.
- **Flat-mode inheritance**: a group reached via `set-mode` inherits the
  default group's patterns (own patterns take priority), so a JS `div` mode can
  override `SLASH`/`SLASH_EQUALS` ahead of `REGEX` without duplicating the
  grammar. `push` targets stay exclusive (F04 region semantics). Derived
  automatically from the transition table.

### Notes
- Fully backward compatible: empty transition table ⇒ identical token stream
  (early-return). Verified by the existing suite + 7 new F10 tests.

## [0.4.0] — 2026-05-14 — LANG51 string escape improvements

### Changed

- `process_escapes` in `grammar_lexer.rs` now handles `\r` (carriage return)
  and `\'` (single-quote) escape sequences in addition to the existing
  `\n`, `\t`, `\\`, `\"` set.  Unknown sequences still pass through unchanged
  (the character after `\` is emitted as-is).

  This is a non-breaking change: no existing grammar or token type was relying
  on `\r` / `\'` being passed through literally; they were simply under-specified.

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
