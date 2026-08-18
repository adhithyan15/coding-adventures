# Changelog

## 0.4.0 — 2026-08-17

### Changed

- `lexer.rs`/`parser.rs` no longer read `brainfuck.tokens`/`brainfuck.grammar`
  off disk at runtime via `env!("CARGO_MANIFEST_DIR")` path-climbing. Added
  `build.rs`, which compiles both grammar files into Rust source (via
  `grammar_tools::compiler`) at build time, embedded with `include!` and
  cached in a `OnceLock` — the same pattern already used by `twig-lexer`/
  `twig-parser`. This was the last Rust crate in the repo with runtime
  grammar-file disk I/O; fixes a real gap (a published crate would not ship
  `code/grammars/`) and restores Miri-sandbox compatibility (Miri's default
  isolation mode blocks filesystem access).

## 0.3.0 — 2026-04-10

### Added

- `lexer.rs`: `Lexer` struct and `tokenize(source: &str) -> Vec<Token>` function. Grammar-driven tokenizer using `brainfuck.tokens`. Each `Token` carries `token_type`, `value`, `line`, and `column` fields. Comment characters are silently skipped.
- `parser.rs`: `Parser` struct and `parse(source: &str) -> Result<AstNode, ParseError>` function. Grammar-driven parser using `brainfuck.grammar`. Returns an AST rooted at a `Program` node with `Instruction`, `Loop`, and `Command` child variants. Returns a `ParseError` with precise source-location details for unmatched brackets.
- Grammar files: `brainfuck.tokens` and `brainfuck.grammar` embedded via `include_str!` (shared across all language implementations).
- New dependencies: `grammar-tools`, `lexer`, and `parser` crates from the coding-adventures workspace.
- Extensive tests for `tokenize` (command tokens, comment skipping, multi-line source, position tracking) and `parse` (simple programs, nested loops, unmatched bracket errors with line/column).

## 0.1.0 — 2026-03-20

### Added
- `translate()` — Brainfuck source-to-bytecode translator with bracket matching
- `BrainfuckVM` — specialized VM with 30,000-cell tape, data pointer, and input buffer
- All 8 Brainfuck opcodes: RIGHT, LEFT, INC, DEC, OUTPUT, INPUT, LOOP_START, LOOP_END
- Wrapping arithmetic (255+1=0, 0-1=255) matching standard Brainfuck semantics
- Tape pointer wrapping at boundaries
- EOF convention: input reads return 0 when exhausted
- `execute_brainfuck()` — one-shot high-level API returning `BrainfuckResult`
- `BrainfuckResult` — output, tape state, traces, and step count
- Comprehensive error handling for unmatched brackets and invalid opcodes
- Execution tracing with human-readable descriptions for every step
