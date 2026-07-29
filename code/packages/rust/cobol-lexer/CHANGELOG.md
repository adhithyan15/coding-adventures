# Changelog

## 0.10.0 — `CONVERTING` keyword

- Reserved the `CONVERTING` keyword for `INSPECT … CONVERTING from TO to`, the
  per-character translation-table form of `INSPECT`. `TO` was already reserved (it
  is the `ADD`/`SET`/`MOVE`/`SELECT … ASSIGN` preposition), so `CONVERTING`'s `TO`
  needs no new token.
- `_grammar.rs` regenerated from `cobol.tokens` via `grammar-tools compile-tokens`
  (never hand-edited).

## 0.9.0 — `INSPECT` verb keywords

- Added the reserved words the `INSPECT` statement needs: `INSPECT`, `TALLYING`,
  `REPLACING`, `LEADING`, `CHARACTERS`, `BEFORE`, `AFTER`, the new preposition
  `FOR`, and the hyphenated `END-INSPECT`. (`ALL` and `BY` were already reserved.)
  These let the parser accept the fuller `INSPECT` surface — `LEADING`/
  `CHARACTERS` tallies, `BEFORE`/`AFTER` regions, and every `REPLACING` form — so
  the reader/compiler reject the unimplemented ones with a friendly "later rung"
  error rather than a bare parse failure.
- `FIRST` and `INITIAL` are deliberately **not** reserved: they are only needed by
  `REPLACING FIRST` / `BEFORE INITIAL` (both later rungs), and reserving such
  common words would collide with existing data names (`FIRST` is a field name in
  the runtime's group-item test). `_grammar.rs` regenerated from `cobol.tokens`
  via `grammar-tools compile-tokens`.

## 0.8.0 — `UNSTRING` verb keywords

- Added the reserved words the `UNSTRING` statement needs: `UNSTRING` and the
  hyphenated `END-UNSTRING` (`DELIMITED`, `BY`, `INTO`, `WITH`, `POINTER`, `ON`,
  `OVERFLOW`, `NOT` were already reserved from the `STRING` cut). As with `STRING`,
  promoting the bare word `UNSTRING` to a KEYWORD does **not** disturb the
  string-literal token type (keyword promotion only rewrites bare `NAME` words).
  `END-UNSTRING` works like the other hyphenated keywords (`END-STRING`,
  `END-EVALUATE`). `_grammar.rs` regenerated from `cobol.tokens` via
  `grammar-tools compile-tokens`.

## 0.7.0 — `STRING` verb keywords

- Added the reserved words the `STRING` statement needs: `STRING`, `DELIMITED`,
  `WITH`, `POINTER`, `OVERFLOW`, and the hyphenated `END-STRING` (`BY`, `INTO`,
  `SIZE`, `ON`, `NOT` were already reserved). Promoting the bare word `STRING` to a
  KEYWORD does **not** disturb the *string-literal* token type (also named
  `STRING`, produced by the quoted `"…"` / `'…'` patterns): keyword promotion only
  rewrites bare `NAME` words, never the quoted-literal tokens. `END-STRING` works
  like the other hyphenated keywords (`END-EVALUATE`, `WORKING-STORAGE`).
  `_grammar.rs` regenerated from `cobol.tokens` via `grammar-tools compile-tokens`.

## 0.6.0 — `COLON` token for reference modification

- Added `COLON = ":"` to SECTION 2 (Punctuation), next to `LPAREN`/`RPAREN`. It
  separates the start position from the length in a reference modification
  (`WS-NAME(2:3)`). It is a single character with no maximal-munch conflict
  against any other punctuation. `_grammar.rs` regenerated from `cobol.tokens`
  via `grammar-tools compile-tokens`.

## 0.5.0 — `EVALUATE` / `OTHER` / `END-EVALUATE` keywords

- Added `EVALUATE`, `OTHER`, and the hyphenated `END-EVALUATE` to the reserved-word
  list so COBOL's case statement lexes as keywords (`WHEN` was already reserved).
  `END-EVALUATE` works like the existing hyphenated keywords (`WORKING-STORAGE`,
  `HIGH-VALUE`): the NAME pattern accepts internal hyphens and the keyword list
  promotes it to a KEYWORD. `_grammar.rs` regenerated from `cobol.tokens` via
  `grammar-tools compile-tokens`.

## 0.4.0 — symbolic relational operator tokens

- Added `GT` (`>`), `LT` (`<`), `GE` (`>=`), `LE` (`<=`), and `NE` (`<>`) so a
  `relation` can be written with symbols as well as the word forms. `EQ` (`=`) was
  already present (it doubles as the COMPUTE assignment). The two-character
  operators are listed before the one-character ones so the lexer's longest-match
  takes them whole (as `POW` `**` precedes `STAR` `*`). `_grammar.rs` regenerated
  from `cobol.tokens` via `grammar-tools compile-tokens`.

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
