# Changelog

All notable changes to the `coding-adventures-ruby-lexer` crate will be documented in this file.

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
