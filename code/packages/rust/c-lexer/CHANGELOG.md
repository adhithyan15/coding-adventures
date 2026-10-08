# Changelog

## Unreleased — PREP01 paste keyword classification

- Expose a cached keyword lookup derived from the compiled C token grammar so
  macro paste can reject keyword spellings without recompiling lexer patterns
  for every expansion.

## Unreleased — PREP01 C directive handoff

- Surface `#`, `##`, and `.` tokens for the C preprocessor dialect; continue
  skipping whitespace and comments.

## 0.1.0 — C integer-core lexer (SIR27)

- Grammar-driven tokenizer over `code/grammars/c/c.tokens`, wrapping
  `lexer::GrammarLexer`.  `tokenize_c` / `try_tokenize_c` / `create_c_lexer`.
- Tokens: integer literals (hex/decimal + `u`/`l` suffixes), char and string
  literals, the C operators (multi-char `== != <= >= << >> && ||` ordered before
  the single-char ones), and the type + control-flow keywords — the `<stdint.h>`
  names (`int8_t`…`uint64_t`, `size_t`) are keywords so a type is never an
  identifier.
- Skips whitespace, line/block comments, and whole preprocessor lines (`#…`),
  so no pre-tokenize hook is required for the v1 subset.
