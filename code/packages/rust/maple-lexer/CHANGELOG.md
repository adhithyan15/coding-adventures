# Changelog

## [0.1.0] - 2026-07-18

### Added

- Initial grammar-driven Rust Maple tokenizer (MA09 §2, task MP-2).
- Statically linked compiled token grammar
  (`code/grammars/maple/maple.tokens`), covering the MP-1-scoped surface
  (MA09 §3): ordinary-parenthesis function/procedure application
  (`f(x, y)`), the single `:=` assignment operator, the arrow/functional
  operator `->` (Maple's real general-purpose function-definition
  spelling, `f := (x, y) -> e` — a token neither `reduce-lexer` nor
  `derive-lexer` needs), `=` as the *equation* operator (never assignment
  — shared with REDUCE/Derive/Macsyma's convention), comparison
  (`< > <= >= <>` — `<>` is Maple's own not-equal spelling, not REDUCE's
  word-spelled `neq` and not Wolfram's `!=`), the word-spelled logical
  keywords `and`/`or`/`not` (lowercase reserved words, like REDUCE's —
  the mirror image of `derive-lexer`'s uppercase `AND`/`OR`/`NOT`), the
  `if`/`then`/`elif`/`else`/`end`/`fi` conditional keywords (`end`/`fi`
  are two distinct ways real Maple closes an `if` — `end if` or bare
  `fi` — both lexed as ordinary `KEYWORD` tokens, with `end`+`if`
  composition left to the parser), the `true`/`false` boolean literals,
  `[a, b, c]` square-bracket **list** literals (ordered, duplicates kept
  — the *opposite* bracket choice from Derive's `[a,b,c]` vector literal)
  and `{a, b, c}` curly-brace **set** literals (unordered, duplicates
  removed — the same bracket REDUCE uses for its own *list* literal,
  different meaning) as four distinct token types (`LBRACKET`/`RBRACKET`,
  `LBRACE`/`RBRACE`) — the first CAS-family lexer in this repo to need
  two aggregate-literal brackets instead of one (MA09 §1), both statement
  terminators `;` and `:` (kept as distinct `SEMI`/`COLON` tokens,
  mirroring `reduce-lexer`'s `SEMI`/`DOLLAR` split), and arithmetic
  `+ - * / ^` (`^` only — deliberately no `POW`/`**` token, since real
  Maple documents no `**` synonym, unlike REDUCE's `^`/`**` pair).
- No `NEWLINE` token and no post-tokenize hook: Maple's `;`/`:`-
  terminated statement model (Programming Guide §5.3) has no significant
  newlines, mirroring `reduce-lexer`/`macsyma-lexer` exactly rather than
  `derive-lexer`/`wolfram-lexer`'s worksheet-style significant-newline
  model.
- 28 tests covering function application, `:=` vs `=` vs bare `:`
  disambiguation, the `->` arrow operator winning over a split `-`/`>`,
  `<>`/`<=`/`>=` winning over their shorter prefixes, square-bracket list
  vs. curly-brace set literals lexing as distinct token types, `^`-only
  arithmetic (`**` is NOT a power operator — it lexes as two `TIMES`
  tokens), `;`/`:` terminator distinctness, `NUMBER`/`NAME` literals
  (including the no-leading-dot rule), every keyword (`and`/`or`/`not`/
  `if`/`then`/`elif`/`else`/`end`/`fi`/`true`/`false`) promoting to
  `KEYWORD` case-sensitively (and that uppercase spellings are NOT
  promoted), `end`+`if` lexing as two separate keyword tokens rather than
  one, newlines being plain whitespace, case-sensitive names, and two
  realistic end-to-end snippets from MA09 §3 (`f := (x, y) -> x + y;` and
  `if x > 0 then 1 elif x < 0 then -1 else 0 end if;`).
