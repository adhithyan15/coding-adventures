# Changelog

All notable changes to this project will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: now imports pre-compiled `_token_grammar.ts`/`_parser_grammar.ts` instead of `readFileSync`-ing `dot.tokens`/`dot.grammar` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published npm package does not ship, so `npm install` + first use would throw `ENOENT`.

## [0.1.0] - 2026-04-23

### Added

- Tokenizer and parser for a focused DOT subset
- AST types for node, edge, attribute, and assignment statements
- Lowering from DOT AST into the shared graph diagram IR
- Support for `rankdir`, `label`, `shape`, `color`, `fillcolor`, `fontcolor`, and `style=rounded`
