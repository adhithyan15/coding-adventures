# Changelog — CodingAdventures::LispLexer

## 0.02 — 2026-08-17

Eliminated runtime grammar-file disk reads.

- `_grammar()` now `require`s a checked-in generated module,
  `CodingAdventures::LispLexer::_Grammar` (compiled ahead of time from
  `code/grammars/lisp/lisp.tokens` via `grammar-tools.pl compile-tokens`),
  instead of opening and parsing `lisp.tokens` off disk on first use.
- The old path climbed out of the installed package's own directory to a
  monorepo-relative `code/grammars/lisp/lisp.tokens` path that a real CPAN
  install of this package does not ship, so first use after `cpanm install`
  would have died with "cannot open ... No such file or directory".
- `_grammars_dir()` is removed along with the now-unused
  `File::Basename`/`File::Spec` imports.
- Public API (`tokenize`), token types, and error messages are unchanged.

## 0.01 — 2026-03-29

Initial release.

- Grammar-driven Lisp/Scheme tokenizer using `lisp.tokens` and Perl's `\G` scanning.
- Emits: NUMBER, SYMBOL, STRING, LPAREN, RPAREN, QUOTE, DOT, EOF.
- Silently skips whitespace and `;` line comments.
- Accurate line/column tracking on all tokens.
- Full Test2::V0 test suite covering all token types, comments, whitespace
  skipping, position tracking, composite expressions, and error cases.
