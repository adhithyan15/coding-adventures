# Changelog

## 0.3.0 — `SET` and `TRUE` keywords

- Added `SET` and `TRUE` to the reserved-word list so `SET cond-name TO TRUE`
  (assigning a level-88 condition-name) lexes as keywords rather than data-names.
  `TO` was already reserved. `_grammar.rs` regenerated from `cobol.tokens` via
  `grammar-tools compile-tokens`.

## 0.2.0 — arithmetic operator tokens (COMPUTE)

- Added the arithmetic operator tokens used by `COMPUTE` expressions: `POW`
  (`**`), `PLUS` (`+`), `MINUS` (`-`), `STAR` (`*`), `SLASH` (`/`), and `EQ`
  (`=`). `**` is ordered before `*` so maximal-munch reads exponentiation as one
  token. COBOL's mandatory spacing around binary operators keeps `-` unambiguous
  against a negative `NUMBER` literal (`-3`) and against a hyphenated `NAME`
  (`A-B`); documented inline in `cobol.tokens`.
- New reserved words: `ROUNDED`, `SIZE`, `ERROR` (for `COMPUTE … ROUNDED` and the
  `ON SIZE ERROR` clause).

## 0.1.0 — COBOL-60 lexer (PL07)

- Grammar-driven tokenizer over `code/grammars/cobol/cobol.tokens`, wrapping
  `lexer::GrammarLexer`. Public API: `tokenize_cobol` / `try_tokenize_cobol` /
  `create_cobol_lexer`, plus the exported `strip_cobol_columns` hook.
- **`strip_cobol_columns` pre-tokenize hook**: turns 80-column card images into
  free-form text — drops the sequence (1–6) and identification (73–80) areas,
  removes `*`/`/` comment lines, splices `-` continuations, keeps the code area
  (8–72). Registered via `GrammarLexer::add_pre_tokenize`; unit-tested on its
  own.
- **PICTURE strings** via declarative mode transitions (F10): `PIC`/`PICTURE`
  switches into a `picture` group matching one `PIC_STRING` (core symbols
  `9 X A V S P` + repetition), then switches back. `PIC X(20).` → `PIC_STRING`
  then `DOT`.
- Reuses FLOW-MATIC machinery: hyphenated `NAME`s, English reserved words as
  case-insensitive `KEYWORD`s (including hyphenated ones like `PROGRAM-ID`,
  `WORKING-STORAGE`, `HIGH-VALUE`), numeric/quoted literals, `. ( )` punctuation,
  and `,`/`;` skipped as optional separators. Level numbers lex as `NUMBER`.
- Scope: a focused reserved-word subset and core PICTUREs — enough to lex a
  complete four-division program. Editing PICTUREs, the full reserved list, and
  Area A/B enforcement are future work.
