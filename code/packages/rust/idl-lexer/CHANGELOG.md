# Changelog

## [0.1.0] - 2026-07-23

### Added

- Initial grammar-driven Rust IDL tokenizer (MA12 §6, task MA-12b).
- `code/grammars/idl/idl.tokens`, written to IDL's own Algol/Fortran-family
  imperative grammar shape (MA12 §5) rather than forked from an array
  sibling. Covers the MA-12b-scoped surface: `;` line comments, `$` line
  continuation (a real, significant token — not stripped as trivia), `&`
  statement separator, single- and double-quoted strings (unified to one
  `STRING` token type, no escape mechanism — real IDL strings have none),
  the word operators `EQ NE LT LE GT GE` (comparison) and `AND OR NOT XOR`
  (logical), the two matrix-product operators `#`/`##` (longest-match-first,
  `##` before `#`), `[`/`]` subscript brackets, ordinary
  arithmetic/punctuation (`+ - * / ^ = , : ( )`), integer/float numeric
  literals (including leading-dot floats and exponents; IDL's typed numeric
  tower and literal type suffixes like `5L`/`1.5D` are explicitly deferred,
  MA12 §2/§4), identifiers (must start with a letter — leading-underscore
  names are reserved for the deferred `_EXTRA`/`_REF_EXTRA` mechanism), and
  the closed control-flow/definition keyword set `IF THEN ELSE ENDIF ENDELSE
  FOR DO ENDFOR WHILE ENDWHILE REPEAT UNTIL ENDREP BREAK CONTINUE BEGIN END
  PRO FUNCTION RETURN`.
- Case-insensitivity via `# @case_insensitive true` alone (no
  `case_sensitive: false`) — the same combination `dot.tokens`/
  `excel.tokens`/`spice/berkeley.tokens` already use in this repo. Only
  `keywords:`-block lookup folds case; ordinary `NAME` tokens and `STRING`
  contents keep the exact case the source used. This was confirmed against
  documented IDL case-insensitivity (multiple independent secondary IDL
  references, since MA12's own text does not state it explicitly) rather
  than assumed from a sibling grammar's convention.
- **Finding: the `/KEYWORD`-vs-division question (MA12 §3 item 3) is
  correctly a parser-level concern, not a lexer one, and this crate does
  not attempt to resolve it.** Q's (MA11) analogous `/`-comment-vs-REDUCE
  ambiguity is resolved in `q-lexer` via whitespace-adjacency pre/post-
  tokenize hooks. That strategy does not transfer here: `PLOT, x, /YLOG`
  (boolean shorthand) and `x = a/YLOG` (ordinary division) both have `/`
  glued to an identifier with zero intervening whitespace on either side —
  no character-adjacency test can tell them apart, because the actual
  distinguishing fact is whether the parser is currently inside a call's
  argument-list production, which only a parser (with a production stack)
  can know. `idl.tokens` therefore emits `SLASH` as one ordinary,
  unconditional division token in every position; the `/KEYWORD` production
  belongs entirely to `idl-parser` (MA-12c).
- **No pre/post-tokenize hooks at all** — a direct consequence of the
  finding above, plus the fact that IDL has no transpose operator to
  collide with `'`/`"` (unlike Scilab) and `;` has no other meaning
  anywhere in this cut's grammar (unlike Q's dual-use `/`). `idl.tokens` is
  entirely declarative; `create_idl_lexer` installs nothing beyond the
  compiled grammar — simpler than every sibling array-family `*-lexer`
  crate in this repo.
- `code/packages/rust/Cargo.toml` workspace registration alongside the
  other array-language lexer/parser/runtime/repl/to-semantic-ir crate
  groups (only `idl-lexer` itself is added; the sibling `idl-parser`/
  `idl-runtime`/`idl-repl`/`idl-to-semantic-ir` crates do not exist yet —
  they are MA-12c/d/e, separate follow-on tasks).
- No recursion-depth cap in this crate, by design: `idl-lexer` performs no
  recursive descent at all (that begins with `idl-parser`, MA-12c) — the
  same split every sibling `*-lexer`/`*-parser` pair in this repo already
  follows.
- 59 tests (5 lib smoke tests + 53 integration tests + 1 doc test) covering:
  every word operator and control-flow/definition keyword (including
  case-insensitive lookup and that plain identifiers preserve their exact
  case); `;` comments (including one containing characters outside the
  grammar's alphabet, and not swallowing the terminating newline); `$`
  continuation surviving as its own token; `&` statement separation; both
  quote styles unifying to `STRING` (including each quote style embedding
  the other verbatim, since there is no escape mechanism); the `##`-vs-`#`
  longest-match regression (including three- and four-hash chains); array
  literals and every in-scope subscript form (plain, negative-from-end,
  ranged, strided, `*`-wildcard, 2-D); a `PRO` definition, a `FUNCTION`
  definition with `RETURN`, an `IF...THEN...ENDIF` block, and a procedure
  call mixing positional arguments, a keyword argument, and the `/KEYWORD`
  boolean shorthand; the `SLASH`-is-always-division finding, demonstrated
  by comparing the call-argument and ordinary-division cases directly;
  numeric literals (integers, leading-dot floats, exponents, and the
  deferred-type-suffix honest-non-handling); identifier rules (leading
  letter required, underscore allowed as a continuation character but not
  as a leading character); and unrecognized-character errors for every
  deferred/out-of-scope construct exercised (`@`, `?`, struct-style `s.tag`,
  brace literals).
