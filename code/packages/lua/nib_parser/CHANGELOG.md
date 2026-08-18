# Changelog — coding-adventures-nib-parser

## Unreleased

### Fixed

- Eliminated runtime grammar loading: `M.get_grammar` now requires a
  pre-compiled `_grammar` module instead of reading and parsing the
  `nib.grammar` file from `code/grammars/` on every call. The old code
  walked out of the installed package's own directory to a
  monorepo-relative path that a published LuaRocks package does not ship.
- Corrected this file, which previously contained an unrelated package's
  (`coding-adventures-starlark-parser`) changelog content verbatim — a
  pre-existing copy-paste error unrelated to this fix.

## 0.1.0 — initial release

- Implement `nib_parser.parse(source)` — tokenize with `nib_lexer` and
  grammar-parse with `parser.GrammarParser`, returning the root ASTNode.
- Implement `nib_parser.get_grammar()` — returns the cached `ParserGrammar`.
