# Changelog

## [0.1.0] - 2026-07-20

### Added

- Initial grammar-driven Rust Scilab tokenizer (MA10 §6, task MA-10b).
- `code/grammars/scilab/scilab.tokens`, forked from
  `code/grammars/matlab/matlab.tokens` at the grammar-source level (copied,
  then diverged) — this crate does not depend on `matlab-lexer` (MA10 §5).
  Covers the MA10 §4-scoped surface: dense-matrix arithmetic, elementwise
  operators, transpose, matrix literals, ranges, comparisons (including both
  not-equal spellings), logical operators, `$`-based indexing, assignment,
  `if`/`select`/`while`/`for` control flow with the optional `then`/`do`
  linker keywords, `break`/`continue`, `function ... endfunction`, comments,
  the eight `%`-prefixed special constants, and single-/double-quoted
  strings.
- Five lexer-level divergences from `matlab.tokens` (MA10 §3):
  1. `//` line comments and `/* ... */` block comments, genuinely simpler
     than MATLAB's `%{`/`%}` — Scilab's block comment may sit inline,
     sharing a line with code on both sides, so no dedicated
     alone-on-its-line stripping pre-pass is needed at all (`BLOCK_COMMENT`
     is an ordinary `skip:` regex).
  2. `'...'` and `"..."` unify to a single `STRING` token type (unlike
     MATLAB's CHARARRAY-vs-STRING split) — the `'`/`.'`
     transpose-vs-string-open ambiguity still needs MA01 §3's context-hook
     *strategy*, reimplemented independently in this crate's own
     `protect_quotes` (not shared code with `matlab-lexer`).
  3. `PERCENT_CONST` — a new, closed eight-word token class (`%pi %e %i
     %inf %nan %eps %t %f`) with no MATLAB analogue. The alternation is
     ordered longest-spelling-first (`inf`/`eps` before `i`/`e`) since this
     repo's `regex` crate uses leftmost-first (not leftmost-longest)
     alternation semantics and has no lookaround.
  4. `$` — a single, always-unambiguous last-index token, unlike MATLAB's
     own context-sensitive `end`.
  5. `<>` — a second not-equal spelling alongside `~=`, kept as its own
     distinct token (`NE_ALT`) rather than aliased, mirroring
     `maple.tokens`'s own `;`/`:` deferral discipline ("the parser, not
     this lexer, collapses the two spellings onto one production").
- One deliberate omission vs. `matlab.tokens`: no `AT` (`@`) token — neither
  MATLAB's function-handle meaning nor Scilab's own deprecated legacy
  `@`-for-`~` spelling is in this cut's scope, so `@` is simply absent
  (falls through to an honest lex error) rather than silently inheriting
  either meaning.
- `endfunction` kept as its own `KEYWORD` value, distinct from the generic
  block-closer `end` that `if`/`while`/`for`/`select` all reduce to (MA10 §1
  finding 7) — `scilab-parser` (MA-10c) needs that distinction as a separate
  production. `switch`/`otherwise` are deliberately NOT keywords (Scilab has
  neither spelling at all — its own construct is `select`/`case`/`else`,
  MA10 §1 finding 4); `return`/`global`/`persistent`/`try`/`catch` are
  likewise excluded as outside this cut's in-scope surface.
- Deliberately absent (MA10 §4's deferred list, simple omission rather than
  special-cased rejection): the Kronecker trigraphs `.*.`/`./.`/`.\.`; the
  `end`-as-last-index convergence; the deprecated legacy `**` spelling for
  `^` (so `a ** b` lexes as two bare `STAR` tokens); the general `%name`
  sigil-dispatch mechanism beyond the fixed eight-word vocabulary.
- `DQ_STRING` is deliberately left un-aliased to `STRING` at the grammar
  level (unlike MATLAB's own identical pattern, which does alias directly):
  the shared `GrammarLexer` engine strips a `DQ_STRING` token's outer quotes
  automatically but does not collapse the doubled `""` escape itself, so
  this crate adds its own `collapse_dq_string_escapes` post-tokenize step
  (mirroring how `STRING_PLACEHOLDER`'s `''` escape is already collapsed by
  this crate's own decode function) to actually honor the doubled-quote
  escaping MA10 §4 promises, rather than leaving `"a""b"` half-decoded to
  `a""b`.
- Known, accepted edge case (documented, not "fixed" further): a `%`-word
  that is not one of the eight constants but happens to start with one of
  the single-letter ones (e.g. `%foo`, which starts with the real constant
  `f`) lexes as `PERCENT_CONST("%f")` + `NAME("oo")` rather than failing
  outright, since the `regex` crate has no lookaround to assert "not
  followed by more identifier characters." This is not valid Scilab either
  way and is expected to be rejected at parse time (MA-10c), so no extra
  lexer-level guard was added for it.
- 33 tests covering the transpose-vs-string `'` ambiguity (including the
  classic `A' * B'` two-transposes trap and `$`-as-a-value before a
  transpose), single- and double-quoted strings resolving to the same
  `STRING` type (including doubled-quote escaping and the
  placeholder-collision regression), `//`/`/* */` comments (including a
  block comment appearing inline mid-line with code on both sides, and one
  spanning multiple lines), every one of the eight `PERCENT_CONST`
  constants (plus a regression guarding `%inf`/`%eps` against being split
  by their own single-letter prefixes), `$` last-index indexing, `<>`
  alongside bare `<`/`>` and alongside `~=`, inherited MATLAB constructs
  (elementwise operators, the `3.*4` trailing-dot trap, matrix literals,
  ranges), every keyword (including `endfunction` staying distinct from
  `end`, and `switch`/`otherwise` staying ordinary `NAME`s), the deferred
  `**`/`@` omissions failing honestly, and unrecognized `%`-words falling
  through to an honest lex error rather than a silent guess.
