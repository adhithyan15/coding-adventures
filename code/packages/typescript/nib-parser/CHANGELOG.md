# Changelog

## 0.1.1 - 2026-08-17

- Eliminated runtime grammar loading: now imports a pre-compiled `_grammar.ts` instead of `readFileSync`-ing `nib.grammar` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published npm package does not ship, so `npm install` + first use would throw `ENOENT`.

## 0.1.0

- add a TypeScript Nib parser wrapper over the shared grammar-driven parser
- add opt-in `preserveSourceInfo` support to propagate trivia-rich source data
  onto grammar AST nodes
- add `parseNibDocument()` for formatter-style callers that need both the AST
  and the original token stream, including EOF trivia
