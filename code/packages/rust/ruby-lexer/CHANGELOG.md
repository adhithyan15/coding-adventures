# Changelog

All notable changes to the `coding-adventures-ruby-lexer` crate will be documented in this file.

## [0.12.0] - 2026-05-20

### Added (Phase 4f — 2.1 numeric suffixes `r` / `i`)
- New `fuse_numeric_suffixes` post-pass under era ≥ 2.1 — folds a `Number` token followed (no whitespace) by `Name("r")` or `Name("i")` into a single fused `Number` token (e.g. `2` + `r` → `2r`).  Ruby 2.1's rational and complex literal forms.
- Pre-2.1 eras leave them split — the era gate is precise.

### Tests (+5 new, total 108)
- 2.1 fuses `2r + 3r` to two Number tokens with values "2r", "3r".
- 2.1 fuses `4i`.
- 2.0 does NOT fuse — the `r` stays as a Name.
- `2 r` (whitespace) does NOT fuse even under 2.1.
- `2x` (`x` isn't a recognised suffix) stays split.

## [0.11.0] - 2026-05-20

### Added (Phase 4e — range fusion + 2.6 endless ranges)
- New unconditional `fuse_range_ops` post-pass — folds adjacent `Dot` tokens into `Op("..")` (inclusive range) or `Op("...")` (exclusive range).  Ruby has had range literals since 1.0, so this fires for every era.
- New `ENDLESS_RANGE_FLAG: u32 = 1 << 1` — set on `..` / `...` range tokens under era ≥ 2.6 when they're followed by a *closer* (`)`, `]`, `}`, `,`, `;`, `\n`, or EOF).  Pre-2.6 these positions were parse errors; 2.6 made them legal endless ranges (`(1..)`, `arr[2..]`, etc.).
- New `mark_endless_ranges` post-pass runs only under era ≥ 2.6.

### Tests (+6 new, total 103)
- `..` fuses unconditionally across eras 1.0/1.8/2.0/3.3.
- `...` fuses unconditionally.
- 2.6 flags `(1..)` as endless range.
- 2.3 does NOT flag — era gate is precise.
- 2.6 leaves normal ranges (`1..5`) unflagged.
- 2.6 flags endless ranges before newline and before comma.

## [0.10.0] - 2026-05-20

### Added (Phase 4d — 2.7 numbered block params `_1`..`_9`)
- New public constant `NUMBERED_BLOCK_PARAM_FLAG: u32 = 1 << 0` — flag bit set on `Token.flags` for Name tokens whose lexeme matches the `_<digit>` (1–9) pattern under era ≥ 2.7.
- New `mark_numbered_block_params` post-pass — runs under era ≥ 2.7 and tags every `Name` token whose value is `_1`..`_9` with the flag bit.  Pre-2.7 eras leave the flag clear so callers treat them as ordinary locals.
- The lexer can't tell whether a given `_1` is actually *inside a block* (that's parser-level context), but it can flag every `_N` lexeme as a *candidate* numbered-param so downstream consumers can apply the era-aware semantics without re-scanning the token stream.
- `is_numbered_block_param` helper exactly matches `_1`..`_9` and explicitly excludes `_0`, `_10`, `_`, `_foo`, etc.

### Tests (+5 new, total 97)
- Era 2.7 flags `_1` and `_2`.
- Era 2.6 does NOT flag them — the era gate is precise.
- Era 2.7 leaves `_foo`, `_`, `_0`, `_10` unflagged (they're not numbered params).
- Eras 2.7, 3.0, 3.3 all flag `_1` consistently.
- `is_numbered_block_param` classifier table-test covering the +/- boundaries.

## [0.9.0] - 2026-05-20

### Added (Phase 4c — 2.3 safe-nav `&.` token fusion)
- New `fuse_safe_nav` post-pass — under era ≥ 2.3, adjacent `Op("&")` + `Dot(".")` tokens fuse into a single `Op("&.")` (the safe-navigation operator).  Pre-2.3 eras keep them split so the lexing is faithful to what each Ruby version originally accepted.
- **Whitespace-aware adjacency**: a new parallel `whitespace_before_token: Vec<bool>` field on `RubyLexer` records whether any whitespace (`' '` / `'\t'`) was consumed between the previous emit and this one.  The fusion post-pass consults this array directly — peek-state operators (`>`, `&`, `=`, `!`, `<`, `|`) report the column of the *follower* character, so the per-token column field alone can't always distinguish `&.` from `& .`.  Tracking whitespace explicitly is robust.
- Both Phase 4b's `fuse_lambda_arrow` and the new `fuse_safe_nav` now share the same adjacency criterion: *same line, no whitespace between*.  The fragile "column distance ≤ 2" heuristic from Phase 4b is replaced.

### Tests (+5 new, total 92)
- 2.3 fuses `a&.b`; 1.8 does NOT; `a & .b` (with whitespace) does NOT fuse even under 2.3.
- Eras 2.5, 2.7, 3.0, 3.3 all inherit the fusion from 2.3.
- Era 2.1 (one notch before 2.3) does NOT fuse — the era gate is precise.

## [0.8.0] - 2026-05-20

### Added (Phase 4b — 1.9.1 lambda `->` token fusion)
- `RubyLexer` now records its target era (`era: String`) and applies era-gated token-stream rewrites in `finish()` after the engine reaches its final state.
- New post-pass `apply_era_token_fusions` — extensible hook for every era's "compose multiple 1.8 tokens into a single newer-era operator" rewrite.  Phase 4b ships the first one; later phases (`&.` in 2.3, etc.) plug in here without touching the TOML state machine.
- **Lambda arrow `->` (1.9.1+)** — adjacent `Op("-")` + `Op(">")` tokens, with no source whitespace between them, fuse into a single `Op("->")` token under era ≥ 1.9.1.  The 1.8 baseline keeps them as two tokens (Ruby 1.8 doesn't know about lambda literals).
- New `era_at_least` helper — total ordering of era strings against `machine::ERA_VERSIONS` (chronological).  Unknown era strings fold to the `1.8` baseline so misconfigured callers get the conservative behaviour.

### Adjacency heuristic
- The 1.8 state machine emits single-char operators like `>` on a *follower* character (it peeks one char ahead to disambiguate `>` vs `>=`), so the `>` token's recorded column is the *follower's* column, 1–2 ahead of the `>` source position.  The fusion check allows a "virtual gap" of up to 2 columns — strictly less than the ≥ 3 a real whitespace-separated `-` `>` would produce.

### Tests (+5 new, total 87)
- 1.9.1 fuses `->(a)` to a single token.
- 1.8 does NOT fuse `->(a)` — the era gate works.
- `1 - > 2` (with a space) does NOT fuse even under 1.9.1 — the adjacency check is strict enough.
- Eras 2.0, 2.3, 2.7, 3.0, 3.3 all inherit the lambda fusion from 1.9.1.
- `era_at_least` is total + chronological, with sensible fallback for unknown / empty era strings.

## [0.7.0] - 2026-05-20

### Added (Phase 4a — 15-era version dispatch)
- `ERA_VERSIONS: &[&str]` constant listing every era version modelled by [code/specs/ruby-version-evolution.md](../../../specs/ruby-version-evolution.md): `"1.0"`, `"1.6"`, `"1.8"`, `"1.9.1"`, `"1.9.3"`, `"2.0"`, `"2.1"`, `"2.3"`, `"2.5"`, `"2.6"`, `"2.7"`, `"3.0"`, `"3.1"`, `"3.2"`, `"3.3"` (chronological order).  Re-exported from the crate root.
- `tokenize_ruby_for_version(source, version)` convenience entry point.  Validates `version` against `ERA_VERSIONS` and returns a `Result<Vec<Token>, String>`; the existing `RubyLexer::new(version)` constructor accepts the same set of strings.
- `definition_for_version` now accepts any era string (was: only `"1.8"`) and tags the returned `StateMachineDefinition`'s `name` field with the requested era (e.g. `ruby-2.3-lexer`) so downstream tooling can identify which grammar produced the tokens.
- Error message on unknown versions now points callers at the spec (`see code/specs/ruby-version-evolution.md`) so they know where the canonical era list lives.

### Notes
- v0 inheritance model: every era currently shares the **1.8 baseline TOML** — only the machine name differs.  This is deliberate: physically duplicating ~1100 lines of TOML 14 times would be massive churn for zero behaviour change in this PR.  Phase 4b+ will fork the era TOMLs as real syntactic deltas land (lambda `->` in 1.9.1, `%i[]` in 2.0, `&.` and `<<~` in 2.3, endless ranges in 2.6, numbered block params in 2.7, …).
- The version-string surface is the load-bearing piece of this phase: callers that need version-gated tooling can already plumb the era string through; the underlying grammar will diverge incrementally as later phases land.

### Tests (+5 new, total 82)
- All 15 era versions parse cleanly and produce a uniquely-named `StateMachineDefinition`.
- `ERA_VERSIONS` has no duplicates and is chronological (1.8 present, 3.3 last).
- `tokenize_ruby_for_version` produces the expected token stream for the baseline source `"x = 1 + 2\n"` under every era.
- Unknown version strings produce a helpful error pointing at the spec.

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
