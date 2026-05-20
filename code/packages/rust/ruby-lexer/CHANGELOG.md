# Changelog

All notable changes to the `coding-adventures-ruby-lexer` crate will be documented in this file.

## [0.6.0] - 2026-05-20

### Added (Phase 3c — heredocs `<<TAG`)
- `<<` is now recognized as a single Op token in [`ruby-1.8.lexer.states.toml`](./ruby-1.8.lexer.states.toml) (new transition `after_lt + "<" → emit(Op)` with text `<<`).  In Phase 1/2/3a/3b the engine emitted two separate `<` Ops, never `<<`.
- `pending_heredocs: VecDeque<PendingHeredoc>` field on `RubyLexer`.  Each entry tracks the tag string, the indices of the `<<` Op and tag Name tokens in `self.tokens`, and the body buffer.  Multiple heredocs on one line are queued FIFO and finalized in source order.
- `heredoc_op_candidate: Option<usize>` field on `RubyLexer`.  Set when an `<<` Op is emitted at expression-start (`ExprBeg` / `ExprMid` per the Phase 2 lex-state machine); consumed on the very next emit — if it's a name-shaped Name or Keyword token, the heredoc is queued.
- `RubyLexer::push` rewritten to be heredoc-aware.  After every `\n` that closes a line with pending heredoc openers, control jumps to `capture_heredoc_bodies`, which slurps whole lines from the source cursor (bypassing the engine) until each pending terminator has been seen, then resumes normal lexing.
- `finalize_heredoc` splices each captured heredoc into the token stream — the `<<` Op becomes a `String` token carrying the verbatim `<<TAG\n<body>TAG` source-shape; the tag Name (or Keyword) token is removed.  Replacements are applied in reverse index order so earlier indices remain valid as later tokens are removed.

### Tests (+12 new, total 77)
- Simple, empty-body, single-line-body, and chained-method (`<<EOF.upcase`) cases.
- Multi-heredoc-per-line FIFO (`<<A; y = <<B\nA body\nA\nB body\nB`) — bodies arrive in opener order.
- Lowercase tag (`<<eof`) and keyword-shaped tag (`<<END`) are both accepted.
- `<<` after a value (`3 << 1`) is a left-shift operator, not a heredoc opener.
- `<<EOF` after `(` is recognized (paren is expression-start).
- Unterminated heredoc records an `unterminated-heredoc` diagnostic and still emits the partial body as a `String` token.
- Body content with `#{...}` interpolation syntax is preserved verbatim (no `#{}` expansion at the lexer level, matching the Phase 3b precedent).
- Heredoc tag shape predicate `is_heredoc_tag` accepts identifiers (letters/digits/underscore, non-digit first) and rejects empty / digit-leading / whitespace-containing inputs.

### Deferred (subsequent Phase 3 follow-ups)
- Phase 3d: indent-modifier heredoc forms — `<<-TAG` (terminator may be indented) and `<<~TAG` (terminator may be indented *and* common leading whitespace is stripped from every body line).
- Quoted-tag forms: `<<"TAG"` (interpolating, same as `<<TAG`) and `<<'TAG'` (non-interpolating, suppress `#{...}` even semantically).
- Recursive sub-lexing of `#{...}` expressions in heredoc bodies (currently captured verbatim, same as Phase 3b strings).

## [0.5.0] - 2026-05-20

### Added (Phase 3b — string interpolation `"#{...}"`)
- `string_d_hash` and `string_d_interp` states in [`ruby-1.8.lexer.states.toml`](./ruby-1.8.lexer.states.toml).  `"..."` body now branches on `#`: if followed by `{` it enters interpolation (`string_d_interp`), otherwise the `#` is treated as a literal character and the follower is re-dispatched to `string_d_body`.
- New `interp_brace_depth: usize` field on `RubyLexer`.  Tracks nesting of `{` / `}` inside an interpolation expression so that `"#{ {a: 1} }"` correctly closes only on the outermost matching `}`.
- The action interpreter intercepts `}` in `string_d_interp` — at depth 0 it appends the `}` to the string buffer and forces the engine back to `string_d_body` via `set_current_state`.  Same pattern as the Phase 2 `/` interceptor.
- v0 captures interpolation **verbatim** — the `#{expr}` substring is preserved inside the `TokenType::String` value.  Recursive sub-lexing of the embedded expression is a future refinement; the parser can dispatch on the `#{` substring when it needs to evaluate.

### Tests (+11 new, total 65)
- Simple, expression, at-start, at-end, multiple-interpolation cases.
- `#` without following `{` is literal (`"a # b"`, `"trailing #"`).
- Nested braces inside interpolation (`"#{ {a: 1} }"`) close correctly.
- Method call inside interpolation (`"#{arr.length}"`).
- Single-quoted strings do **not** interpolate (`'#{name}'` stays as `#{name}`).
- Embedded double-quotes inside `#{...}` are accepted (brace tracker is string-agnostic).

### Deferred (subsequent Phase 3 follow-ups)
- Phase 3c: heredocs (`<<X`, `<<-X`, `<<~X`) with deferred body capture and FIFO queue for multi-per-line heredocs.
- Future refinement: recursive sub-lexing of the interpolation expression (so the parser receives individual tokens for the embedded code instead of one verbatim string).

## [0.4.0] - 2026-05-20

### Added (Phase 3a — regex flags + `%w[]` / `%q{}` percent literals)
- Regex flag suffixes (`/foo/i`, `/foo/im`, `/foo/IM`, …) — both cases accepted, greedy slurp on `imxoeunsIMXOEUNS`.
- `%w[...]` string-array percent literal (canonical `[` delimiter only in v0).
- `%q{...}` non-interpolating string percent literal (canonical `{` delimiter only in v0).
- New `regex_flags`, `after_percent`, `percent_w_open`, `percent_w_body`, `percent_q_open`, `percent_q_body` states in [`ruby-1.8.lexer.states.toml`](./ruby-1.8.lexer.states.toml).
- New `PercentW` and `PercentQ` token kinds in the declared token vocabulary (action interpreter maps both to `TokenType::String` with the verbatim source-shape preserved as the value).
- 11 new unit tests; total now 54 (was 43 in Phase 2).

### Changed
- The `data → %` transition now routes through `after_percent` to peek for a type letter.  When the follower is not `w` or `q`, `%` falls back to the modulo operator (matches Phase 1 behaviour).
- The `regex_body → /` closing transition now goes to `regex_flags` (instead of `data`), appending the `/` to the text buffer.  When `regex_flags` exits, the emitted token's value has the source-shape `/body/` (no flags) or `/body/flags` (with flags).

### Deferred (subsequent Phase 3 follow-ups)
- Phase 3b: string interpolation `"a#{expr}b"` — recursive sub-lexer / sub-machine.
- Phase 3c: heredocs (`<<X`, `<<-X`, `<<~X`) with deferred body capture and FIFO queue for multi-per-line heredocs.
- Additional percent literals: `%W`, `%Q`, `%i`, `%I`, `%r`, `%s`, `%x` and the full set of delimiter pairs (`(...)`, `<...>`, `|...|`, any non-alphanumeric).

## [0.3.0] - 2026-05-20

### Added (Phase 2 — parser-feedback)
- `LexState` enum (`ExprBeg`, `ExprMid`, `ExprEnd`, `ExprArg`, `ExprFname`, `ExprDot`) — tracked by the lexer and updated after every emitted token.  See `code/specs/ruby-lexer-state-machine.md` §1.
- `ParserOracle` trait with default impls (`NoLocals`, `StaticLocals`).  Consulted by the lexer when `/` follows a name in `ExprArg` position — local-variable names get binary division, method-like names get a regex literal.  See `code/specs/ruby-lexer-state-machine.md` §3.
- `RubyLexer::with_oracle(version, oracle)` constructor and `tokenize_ruby_with_oracle(source, oracle)` convenience entry point.
- `regex_body` / `regex_escape` sub-states in `ruby-1.8.lexer.states.toml`.  Reached via `set_current_state` from the action interpreter once it has decided to open a regex literal; the engine then accumulates the body until the closing `/`.
- Regex literals emit as `TokenType::String` tokens with the value framed as `/.../` so the parser can dispatch by lexeme until a dedicated `TokenType::Regex` lands in a later phase.

### Changed
- `tokenize_ruby` default semantics for `/` after a name now follow the spec: with the `NoLocals` oracle every name is a method, so `f /x/` lexes as a method call with a regex argument.  Callers that want binary division on locals must pass a `ParserOracle` via `tokenize_ruby_with_oracle` (or `RubyLexer::with_oracle`).
- The Phase 1 `binary_operators_dispatch_to_dedicated_kinds` test was rewritten to pass an explicit `StaticLocals` oracle declaring its operands as locals.

### Notes
- Phase 2 leaves the `+` / `-` / `*` whitespace-sensitive disambiguation untouched — the spec defers it to a Phase 2b refinement.  Only `/` is interpreted via lex-state + oracle in this cut.
- Regex flags (`/foo/i`, `/foo/m`, …) remain unhandled — they arrive alongside heredoc / interpolation in Phase 3.

## [0.2.0] - 2026-05-19

### Changed (BREAKING)
- Replaced the regex-based `lexer::GrammarLexer` backend with a TOML-encoded state machine driven by `state_machine::EffectfulStateMachine`.  This is **Phase 1** of the multi-phase plan in [code/specs/ruby-parser.md](../../../specs/ruby-parser.md).
- Source of truth is [`ruby-1.8.lexer.states.toml`](./ruby-1.8.lexer.states.toml) at the crate root; the action interpreter in `src/lib.rs` turns its effect strings into `lexer::token::Token` values.
- `create_ruby_lexer(source)` is gone; replaced by `RubyLexer::new(version)` which constructs a versioned lexer (`"1.8"` is currently the only accepted value).
- `tokenize_ruby(source)` keeps its signature.  A new `tokenize_ruby_diag(source)` variant returns the diagnostic list alongside the tokens.

### Added
- `RubyLexer` struct with explicit `push` / `finish` / `drain_tokens` / `diagnostics` methods.
- `Diagnostic` struct for non-fatal lex errors — the lexer never panics on malformed input.
- Newline as a first-class token (`TokenType::Newline`); Ruby treats `\n` as a statement terminator.
- Method-name suffixes `?` and `!` are now part of the identifier token (`empty?`, `save!`).

### Phase 1 scope
- Identifiers (with `?` / `!` suffix), integers (with `_` separators), strings (`"..."` and `'...'`, no interpolation), line comments, common operators (`+ - * / % == != < > <= >= = ! && || => ** ::` …), and basic punctuation.
- **Heredocs, regex literals, string interpolation, parser-driven `f /x/` disambiguation, and the 1.9.1+ syntax additions are deferred to subsequent phases** (see [ruby-parser.md](../../../specs/ruby-parser.md) §"Phasing").

## [0.1.0] - 2026-03-21

### Added
- `create_ruby_lexer(source)` — factory function that loads `ruby.tokens` and returns a configured `GrammarLexer`.
- `tokenize_ruby(source)` — convenience function that tokenizes Ruby source and returns `Vec<Token>`.
- Loads grammar from `ruby.tokens` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering assignments, keywords, arithmetic operators, comparison operators, strings, numbers, comments, delimiters, whitespace, method definitions, symbols, and the factory function.
