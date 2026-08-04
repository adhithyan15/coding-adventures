# Changelog

## 0.1.0 — FLOW-MATIC lexer (PL06)

- Grammar-driven tokenizer over `code/grammars/flow_matic/flow_matic.tokens`,
  wrapping `lexer::GrammarLexer`. Public API: `tokenize_flow_matic` /
  `try_tokenize_flow_matic` / `create_flow_matic_lexer`.
- Tokens: unsigned integers (`NUMBER`), hyphenated data names (`NAME`), the
  English verbs and clause words as case-insensitive `KEYWORD`s (including the
  hyphenated verbs `WRITE-ITEM`, `READ-ITEM`, `CLOSE-OUT` via keyword
  promotion), and the `( ) . ;` punctuation.
- Case is insignificant for matching (`case_sensitive: false` +
  `@case_insensitive true`), matching the UNIVAC's uppercase-only hardware;
  keywords are normalized to uppercase, while NAME values preserve the case they
  were typed in (canonical all-caps source keeps `PRODUCT-NO` intact).
- No pre/post-tokenize hooks: FLOW-MATIC's free-form listings need no column
  stripping (contrast COBOL), and operation labels `(0)` vs field qualifiers
  `(A)` are disambiguated by the parser, not the lexer.
