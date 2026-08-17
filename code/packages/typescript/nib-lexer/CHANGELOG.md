# Changelog

## 0.1.1 - 2026-08-17

- Eliminated runtime grammar loading: now imports a pre-compiled `_grammar.ts` instead of `readFileSync`-ing `nib.tokens` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published npm package does not ship, so `npm install` + first use would throw `ENOENT`.

## 0.1.0

- add a TypeScript Nib lexer wrapper over the shared grammar-driven lexer
- add an opt-in `preserveSourceInfo` path so formatter callers can retain
  offsets, token indices, and leading comment/whitespace trivia
